// SPDX-License-Identifier: Apache-2.0
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! A schemas-only proposal producer. This process evaluates the restricted
//! statement input independently; the receiver checks the paths from its import
//! against its own pinned contract before admitting a result.
use qcore_schemas::SemanticWitness;
use qcore_schemas::chunk_summary::{
    BooleanFieldContractWitness, BooleanFieldExpression, BooleanFieldResult,
    BooleanFieldSourceInput, BooleanFieldStatement, ChunkSourcePath, ChunkSourceSummaryRecord,
};
use qcore_schemas::plugin::*;

// These limits also apply to direct Rust callers, independently of transport
// JSON limits. Preflight visits unreachable branches and statements as well.
const MAX_DEPTH: usize = 128;
const MAX_PROGRAM_OPERATIONS: usize = 16_384;
const MAX_EXECUTION_OPERATIONS: usize = 32_768;

fn charge(remaining: &mut usize, amount: usize) -> Option<()> {
    *remaining = remaining.checked_sub(amount)?;
    Some(())
}

fn validate_expression(
    expression: &BooleanFieldExpression,
    depth: usize,
    remaining: &mut usize,
) -> Option<()> {
    if depth > MAX_DEPTH {
        return None;
    }
    charge(remaining, 1)?;
    match expression {
        BooleanFieldExpression::Integer(_) | BooleanFieldExpression::Field => {}
        BooleanFieldExpression::Not(value) | BooleanFieldExpression::Negate(value) => {
            validate_expression(value, depth + 1, remaining)?;
        }
        BooleanFieldExpression::Equal(left, right)
        | BooleanFieldExpression::NotEqual(left, right) => {
            validate_expression(left, depth + 1, remaining)?;
            validate_expression(right, depth + 1, remaining)?;
        }
    }
    Some(())
}

fn validate_statement(
    statement: &BooleanFieldStatement,
    result_kind: BooleanFieldResult,
    depth: usize,
    remaining: &mut usize,
) -> Option<()> {
    if depth > MAX_DEPTH {
        return None;
    }
    charge(remaining, 1)?;
    match statement {
        BooleanFieldStatement::Block(statements) => {
            // Refuse a huge flat block before traversing it.
            if statements.len() > *remaining {
                return None;
            }
            for statement in statements {
                validate_statement(statement, result_kind, depth + 1, remaining)?;
            }
        }
        BooleanFieldStatement::If(condition, yes, no) => {
            validate_expression(condition, depth + 1, remaining)?;
            validate_statement(yes, result_kind, depth + 1, remaining)?;
            if let Some(no) = no {
                validate_statement(no, result_kind, depth + 1, remaining)?;
            }
        }
        BooleanFieldStatement::Return(value) => {
            if result_kind != BooleanFieldResult::I32 {
                return None;
            }
            validate_expression(value, depth + 1, remaining)?;
        }
        BooleanFieldStatement::ReturnUnit => {
            if result_kind != BooleanFieldResult::Unit {
                return None;
            }
        }
        BooleanFieldStatement::Assign(value) => {
            validate_expression(value, depth + 1, remaining)?;
        }
        BooleanFieldStatement::FrameWrites(writes) => {
            charge(remaining, writes.len())?;
            if writes.iter().any(|(name, _)| name.is_empty()) {
                return None;
            }
        }
    }
    Some(())
}

fn evaluate(
    expression: &BooleanFieldExpression,
    field: bool,
    remaining: &mut usize,
) -> Option<i32> {
    charge(remaining, 1)?;
    Some(match expression {
        BooleanFieldExpression::Integer(value) => *value,
        BooleanFieldExpression::Field => i32::from(field),
        BooleanFieldExpression::Not(value) => i32::from(evaluate(value, field, remaining)? == 0),
        BooleanFieldExpression::Negate(value) => {
            evaluate(value, field, remaining)?.checked_neg()?
        }
        BooleanFieldExpression::Equal(left, right) => {
            i32::from(evaluate(left, field, remaining)? == evaluate(right, field, remaining)?)
        }
        BooleanFieldExpression::NotEqual(left, right) => {
            i32::from(evaluate(left, field, remaining)? != evaluate(right, field, remaining)?)
        }
    })
}

// The outer Option represents execution failure. The inner Option distinguishes
// a return from a successful statement that continues with its successor.
fn execute(
    statement: &BooleanFieldStatement,
    path: &mut ChunkSourcePath,
    remaining: &mut usize,
) -> Option<Option<i32>> {
    charge(remaining, 1)?;
    match statement {
        BooleanFieldStatement::Block(statements) => {
            for statement in statements {
                if let Some(value) = execute(statement, path, remaining)? {
                    return Some(Some(value));
                }
            }
            Some(None)
        }
        BooleanFieldStatement::If(condition, yes, no) => {
            let taken = evaluate(condition, path.final_value, remaining)? != 0;
            path.branches.push(taken);
            if taken {
                execute(yes, path, remaining)
            } else if let Some(no) = no {
                execute(no, path, remaining)
            } else {
                Some(None)
            }
        }
        BooleanFieldStatement::Assign(value) => {
            let value = evaluate(value, path.final_value, remaining)?;
            if !matches!(value, 0 | 1) {
                return None;
            }
            path.final_value = value != 0;
            path.writes.push(path.final_value);
            Some(None)
        }
        BooleanFieldStatement::Return(value) => {
            Some(Some(evaluate(value, path.final_value, remaining)?))
        }
        BooleanFieldStatement::ReturnUnit => Some(Some(0)),
        BooleanFieldStatement::FrameWrites(writes) => {
            charge(remaining, writes.len())?;
            path.frame_writes.extend(writes.iter().cloned());
            Some(None)
        }
    }
}

fn derive_summary(source: &BooleanFieldSourceInput) -> Option<ChunkSourceSummaryRecord> {
    if !source.version_matches_result()
        || source.function.is_empty()
        || source.source_revision.is_empty()
    {
        return None;
    }
    let mut program_remaining = MAX_PROGRAM_OPERATIONS;
    validate_statement(
        &source.program,
        source.result_kind,
        0,
        &mut program_remaining,
    )?;
    let mut remaining = MAX_EXECUTION_OPERATIONS;
    let mut run = |initial_value| {
        let mut path = ChunkSourcePath {
            initial_value,
            final_value: initial_value,
            return_value: 0,
            branches: Vec::new(),
            writes: Vec::new(),
            frame_writes: Vec::new(),
        };
        let result = execute(&source.program, &mut path, &mut remaining)?;
        path.return_value = match source.result_kind {
            BooleanFieldResult::I32 => result?,
            BooleanFieldResult::Unit => result.unwrap_or(0),
        };
        Some(path)
    };
    Some(ChunkSourceSummaryRecord {
        result_kind: source.result_kind,
        function: source.function.clone(),
        source_revision: source.source_revision.clone(),
        field: source.field,
        paths: [run(false)?, run(true)?],
        dependencies: source.dependencies.clone(),
        profile_assumptions: source.profile_assumptions.clone(),
    })
}

pub fn propose(
    request: &VerificationRequestV2,
    manifest: &PluginManifest,
) -> Option<VerificationResponseV2> {
    if request.capability != VerifierCapability::ResourceContracts
        || request.target.profile
            != match request.source_effect.as_ref()?.result_kind {
                BooleanFieldResult::I32 => "qcore.c_boolean_field_contract.v2",
                BooleanFieldResult::Unit => "qcore.c_boolean_field_contract.v3",
            }
        || !matches!(
            request.target.dialect.as_str(),
            "c11" | "gnu99" | "gnu11" | "gnu17"
        )
    {
        return None;
    }
    let summary = derive_summary(request.source_effect.as_ref()?)?;
    Some(VerificationResponseV2 {
        protocol_version: 1,
        request_id: request.request_id.clone(),
        plugin_id: manifest.plugin_id.clone(),
        verdict: ProposedVerdict::Safe,
        diagnostics: Vec::new(),
        evidence: Some(ProposedEvidenceV2 {
            evidence_version: 2,
            unit: request.unit.clone(),
            capability: request.capability,
            coverage: request.coverage(),
            assumptions_used: request.assumptions.clone(),
            reproducibility: Reproducibility {
                provider_version: manifest.package_version.clone(),
                package_digest: manifest.package_digest.clone(),
                deterministic: true,
            },
            witness: SemanticWitness::BooleanFieldContract(Box::new(BooleanFieldContractWitness {
                task_digest: request.task_digest.clone(),
                summary,
            })),
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use BooleanFieldExpression as E;
    use BooleanFieldStatement as S;
    use qcore_schemas::chunk_summary::{BOOLEAN_FIELD_SOURCE_INPUT_VERSION, ChunkSourceField};

    fn input(program: S) -> BooleanFieldSourceInput {
        BooleanFieldSourceInput {
            result_kind: qcore_schemas::chunk_summary::BooleanFieldResult::I32,
            schema_version: BOOLEAN_FIELD_SOURCE_INPUT_VERSION,
            function: "toggle".into(),
            source_revision: "pinned-source".into(),
            field: ChunkSourceField::Locked,
            program,
            dependencies: [("helper".into(), "source-digest".into())].into(),
            profile_assumptions: [("field_is_boolean".into(), "true".into())].into(),
        }
    }

    #[test]
    fn computes_both_prestates_with_ordered_writes_and_updated_field_reads() {
        let source = input(S::Block(vec![
            S::Assign(E::Field),
            S::Assign(E::Not(Box::new(E::Field))),
            S::Return(E::Field),
        ]));
        let summary = derive_summary(&source).unwrap();
        for (path, initial) in summary.paths.iter().zip([false, true]) {
            assert_eq!(path.initial_value, initial);
            assert_eq!(path.final_value, !initial);
            assert_eq!(path.return_value, i32::from(!initial));
            assert_eq!(path.writes, [initial, !initial]);
            assert!(path.branches.is_empty());
        }
        assert_eq!(summary.dependencies, source.dependencies);
        assert_eq!(summary.profile_assumptions, source.profile_assumptions);
        assert_eq!(summary.field, ChunkSourceField::Locked);
    }

    #[test]
    fn branches_use_c_truthiness_and_return_stops_later_effects() {
        let source = input(S::Block(vec![
            S::If(
                E::Field,
                Box::new(S::If(
                    E::Integer(-7),
                    Box::new(S::Return(E::Integer(9))),
                    None,
                )),
                Some(Box::new(S::FrameWrites(vec![
                    ("count".into(), 3),
                    ("count".into(), 4),
                ]))),
            ),
            S::Return(E::Integer(-1)),
            // Valid syntax, but these effects and arithmetic are unreachable.
            S::Assign(E::Integer(1)),
            S::Return(E::Negate(Box::new(E::Integer(i32::MIN)))),
        ]));
        let summary = derive_summary(&source).unwrap();
        assert_eq!(summary.paths[0].branches, [false]);
        assert_eq!(summary.paths[0].return_value, -1);
        assert_eq!(
            summary.paths[0].frame_writes,
            [("count".into(), 3), ("count".into(), 4)]
        );
        assert_eq!(summary.paths[1].branches, [true, true]);
        assert_eq!(summary.paths[1].return_value, 9);
        assert!(summary.paths[1].frame_writes.is_empty());
        assert!(summary.paths.iter().all(|path| path.writes.is_empty()));
    }

    #[test]
    fn comparisons_negation_and_nested_blocks_preserve_integer_results() {
        let source = input(S::Block(vec![S::Block(vec![
            S::If(
                E::Equal(Box::new(E::Field), Box::new(E::Integer(0))),
                Box::new(S::Assign(E::NotEqual(
                    Box::new(E::Integer(2)),
                    Box::new(E::Integer(3)),
                ))),
                None,
            ),
            S::Return(E::Negate(Box::new(E::Field))),
        ])]));
        let summary = derive_summary(&source).unwrap();
        assert_eq!(summary.paths[0].branches, [true]);
        assert_eq!(summary.paths[1].branches, [false]);
        assert_eq!(summary.paths[0].writes, [true]);
        assert!(summary.paths[1].writes.is_empty());
        assert!(
            summary
                .paths
                .iter()
                .all(|path| path.final_value && path.return_value == -1)
        );
    }

    #[test]
    fn rejects_failure_on_either_prestate_and_never_emits_partial_paths() {
        for program in [
            S::Block(vec![]),
            S::If(E::Field, Box::new(S::Return(E::Integer(0))), None),
            S::If(
                E::Not(Box::new(E::Field)),
                Box::new(S::Return(E::Integer(0))),
                None,
            ),
            S::Block(vec![S::Assign(E::Integer(2)), S::Return(E::Field)]),
            S::Return(E::Negate(Box::new(E::Integer(i32::MIN)))),
            S::Block(vec![
                S::FrameWrites(vec![(String::new(), 1)]),
                S::Return(E::Field),
            ]),
        ] {
            assert!(derive_summary(&input(program)).is_none());
        }
        let mut source = input(S::Return(E::Field));
        source.schema_version = 1;
        assert!(derive_summary(&source).is_none());
        source.schema_version = BOOLEAN_FIELD_SOURCE_INPUT_VERSION;
        source.function.clear();
        assert!(derive_summary(&source).is_none());
    }

    #[test]
    fn limits_cover_deep_expressions_and_unreachable_oversized_programs() {
        let mut expression = E::Field;
        for _ in 0..=MAX_DEPTH {
            expression = E::Not(Box::new(expression));
        }
        assert!(derive_summary(&input(S::Return(expression))).is_none());
        let mut statement = S::Return(E::Field);
        for _ in 0..=MAX_DEPTH {
            statement = S::Block(vec![statement]);
        }
        assert!(derive_summary(&input(statement)).is_none());
        assert!(
            derive_summary(&input(S::Block(vec![
                S::Return(E::Field),
                S::Block(vec![S::Assign(E::Field); MAX_PROGRAM_OPERATIONS]),
            ])))
            .is_none()
        );
        assert!(
            derive_summary(&input(S::Block(vec![
                S::Return(E::Field),
                S::FrameWrites(vec![("field".into(), 0); MAX_PROGRAM_OPERATIONS]),
            ])))
            .is_none()
        );
        let source = input(S::Return(E::Field));
        let mut budget = 1;
        let mut path = ChunkSourcePath {
            initial_value: false,
            final_value: false,
            return_value: 0,
            branches: vec![],
            writes: vec![],
            frame_writes: vec![],
        };
        assert!(execute(&source.program, &mut path, &mut budget).is_none());
    }
    #[test]
    fn void_fallthrough_and_early_return_keep_unit_abi() {
        use qcore_schemas::chunk_summary::{
            BOOLEAN_FIELD_UNIT_SOURCE_INPUT_VERSION, BooleanFieldResult,
        };
        let mut source = input(S::Block(vec![S::Assign(E::Integer(1))]));
        source.result_kind = BooleanFieldResult::Unit;
        source.schema_version = BOOLEAN_FIELD_UNIT_SOURCE_INPUT_VERSION;
        let summary = derive_summary(&source).unwrap();
        assert_eq!(summary.result_kind, BooleanFieldResult::Unit);
        assert!(
            summary
                .paths
                .iter()
                .all(|path| path.final_value && path.return_value == 0)
        );
        source.program = S::Block(vec![S::ReturnUnit, S::Assign(E::Integer(1))]);
        let summary = derive_summary(&source).unwrap();
        assert!(
            summary
                .paths
                .iter()
                .all(|path| path.initial_value == path.final_value && path.writes.is_empty())
        );
    }

    #[test]
    fn wrong_result_forms_reject_even_when_unreachable() {
        use qcore_schemas::chunk_summary::{
            BOOLEAN_FIELD_UNIT_SOURCE_INPUT_VERSION, BooleanFieldResult,
        };
        let mut source = input(S::Block(vec![S::Return(E::Integer(0)), S::ReturnUnit]));
        assert!(derive_summary(&source).is_none());
        source.result_kind = BooleanFieldResult::Unit;
        source.schema_version = BOOLEAN_FIELD_UNIT_SOURCE_INPUT_VERSION;
        source.program = S::Block(vec![S::ReturnUnit, S::Return(E::Integer(0))]);
        assert!(derive_summary(&source).is_none());
        source.program = S::ReturnUnit;
        source.schema_version = BOOLEAN_FIELD_SOURCE_INPUT_VERSION;
        assert!(derive_summary(&source).is_none());
    }
}
