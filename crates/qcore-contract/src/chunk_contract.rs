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

//! Interpret the bounded lifecycle predicate vocabulary in native QCore
//! contract files and check it against independently imported C field paths.
//! Library function names and postconditions belong in the canonical contract corpus, not here.
//!
//! Inputs are declared schema records. Successful evaluation means the supplied
//! finite paths satisfy the selected policy, not that they came from source.
//! Source-bound callers must independently establish their import/provenance;
//! no schema record can construct the compiler's source-checked certificate.

use std::collections::{BTreeMap, BTreeSet};

use qcore_frontend::frontend::{
    DeclarationKind, SourceContractClauseKind, SourceContractExitSelector, parse_source_program,
};
use qcore_frontend::lexer::{Symbol, TokenKind, lex};
use qcore_schemas::chunk_summary::{
    BooleanFieldResult, ChunkFrameSummaryRecord, ChunkSourceField, ChunkSourceSummaryRecord,
};

/// Evaluate a metadata contract against declared helper paths. Live storage,
/// exclusive access and complete source import are caller premises. Unknown
/// predicates and unmentioned writes fail closed; this is not a source proof.
pub fn evaluate_chunk_frame_contract(
    summary: &ChunkFrameSummaryRecord,
    source: &str,
) -> Result<usize, String> {
    evaluate_frame_contract(
        summary,
        source,
        "Chunk",
        "chunk_is_live",
        "exclusive_chunk_access",
        "lock_is_preserved",
        &BTreeSet::from(["lock".into()]),
    )
}

/// Evaluate a declared frame policy over a selected nominal record and protected
/// field set. These arguments are premises; this does not establish source origin.
/// Source-bound consumers must derive them from an opaque checked import.
pub fn evaluate_boolean_frame_contract(
    summary: &ChunkFrameSummaryRecord,
    source: &str,
    record: &str,
    protected_fields: &BTreeSet<String>,
) -> Result<usize, String> {
    if !matches!(tokens(record)?.as_slice(), [TokenKind::Word(_)]) {
        return Err("Boolean frame record must be one nominal identifier".into());
    }
    if protected_fields.is_empty() {
        return Err("Boolean frame contract requires a nonempty modeled-field inventory".into());
    }
    if summary.writes.is_empty()
        || summary
            .writes
            .iter()
            .any(|(field, _)| protected_fields.contains(field))
    {
        return Err(
            "Boolean frame helper must write nonempty metadata disjoint from modeled fields".into(),
        );
    }
    evaluate_frame_contract(
        summary,
        source,
        record,
        "object_is_live",
        "exclusive_object_access",
        "modeled_fields_are_preserved",
        protected_fields,
    )
}

#[allow(clippy::too_many_arguments)]
fn evaluate_frame_contract(
    summary: &ChunkFrameSummaryRecord,
    source: &str,
    record: &str,
    live: &str,
    exclusive: &str,
    preserved: &str,
    protected_fields: &BTreeSet<String>,
) -> Result<usize, String> {
    let parsed = parse_source_program(source).map_err(|error| error.to_string())?;
    let declarations: Vec<_> = parsed
        .declarations
        .iter()
        .filter(|decl| decl.value.name == summary.function.as_str())
        .collect();
    let [decl] = declarations.as_slice() else {
        return Err("expected one native frame-helper declaration".into());
    };
    let decl = &decl.value;
    if decl.kind != DeclarationKind::ExternFunction || decl.parameters.len() != 1 {
        return Err("frame helper requires one borrowed external parameter".into());
    }
    if decl.parameters[0].usage.as_ref().map(|usage| usage.value)
        != Some(qcore_core::usage::Usage::ONE)
    {
        return Err("frame helper requires explicit use one".into());
    }
    let parameter = tokens(&decl.parameters[0].type_source)?;
    let expected = tokens(&format!("borrow<'call, mut, {record}>"))?;
    // Lifetimes are binders; their spelling does not change the borrow mode.
    if parameter.len() != expected.len()
        || parameter
            .iter()
            .zip(&expected)
            .enumerate()
            .any(|(index, (actual, expected))| index != 3 && actual != expected)
        || !matches!(parameter.get(3), Some(TokenKind::Word(_)))
    {
        return Err("frame helper requires a mutable Chunk borrow".into());
    }
    let arrow = decl
        .tokens
        .iter()
        .position(|token| *token == TokenKind::Symbol(Symbol::Arrow))
        .ok_or("missing frame helper result")?;
    if decl.tokens.get(arrow + 1) != Some(&TokenKind::Word("unit".into()))
        || decl.tokens.get(arrow + 2) != Some(&TokenKind::Word("contract".into()))
    {
        return Err("frame helper requires unit result without additional effects".into());
    }
    let contract = decl
        .function_contract
        .as_ref()
        .ok_or("missing frame contract")?;
    let mut requirements = BTreeSet::new();
    let mut fields = BTreeSet::new();
    let mut ghost_names = BTreeSet::new();
    for clause in &contract.clauses {
        let clause_tokens = tokens(&clause.value.expression_source)?;
        match clause.value.kind {
            SourceContractClauseKind::Requires => {
                let [TokenKind::Word(name)] = clause_tokens.as_slice() else {
                    return Err("unsupported frame precondition".into());
                };
                if !requirements.insert(name.clone()) {
                    return Err("duplicate frame precondition".into());
                }
            }
            SourceContractClauseKind::Ensures(SourceContractExitSelector::Normal) => {
                for token in clause_tokens {
                    if let TokenKind::Word(name) = token {
                        if let Some(field) = name.strip_prefix("field_") {
                            fields.insert(field.to_owned());
                        }
                        ghost_names.insert(name);
                    }
                }
            }
            _ => {
                return Err(
                    "frame contract supports requires and normal postconditions only".into(),
                );
            }
        }
    }
    if requirements != BTreeSet::from([live.into(), exclusive.into()]) {
        return Err("frame contract must require live storage and exclusive access".into());
    }
    if !ghost_names.contains("writes_only_declared_fields") || !ghost_names.contains(preserved) {
        return Err("frame contract must constrain the write footprint and lock frame".into());
    }
    if summary
        .writes
        .iter()
        .any(|(field, _)| !fields.contains(field))
    {
        return Err("frame contract leaves a written field unconstrained".into());
    }
    let mut facts = BTreeMap::from([
        (live.into(), 1),
        (exclusive.into(), 1),
        (
            "writes_only_declared_fields".into(),
            i128::from(
                summary
                    .writes
                    .as_slice()
                    .iter()
                    .all(|(field, _)| fields.contains(field)),
            ),
        ),
        (
            preserved.into(),
            i128::from(
                summary
                    .writes
                    .as_slice()
                    .iter()
                    .all(|(field, _)| !protected_fields.contains(field)),
            ),
        ),
    ]);
    for (field, value) in summary.writes.as_slice() {
        facts.insert(format!("field_{field}"), i128::from(*value));
    }
    match crate::contract_observation::check_contract_observation(
        source,
        summary.function.as_str(),
        &facts,
    )? {
        crate::contract_observation::ObservationVerdict::Checked { postconditions } => {
            Ok(postconditions)
        }
        crate::contract_observation::ObservationVerdict::OutsidePreconditions { predicate } => {
            Err(format!("unmet frame precondition {predicate}"))
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChunkContractEvaluation {
    function: String,
    contract_source: String,
    checked_states: usize,
    dependency_sources: BTreeMap<String, String>,
}

impl ChunkContractEvaluation {
    pub fn function(&self) -> &str {
        &self.function
    }

    pub fn contract_source(&self) -> &str {
        &self.contract_source
    }

    pub fn checked_states(&self) -> usize {
        self.checked_states
    }

    pub fn dependency_sources(&self) -> &BTreeMap<String, String> {
        &self.dependency_sources
    }
}

/// Discharge all postconditions over both values of each Boolean field. The
/// four preconditions are explicit assumptions, not established facts about
/// arbitrary callers. Unsupported vocabulary or weakened/missing domain
/// requirements fails closed. This is a function-contract result, not a
/// whole-program lifecycle or memory-safety certificate.
pub fn evaluate_chunk_source_contract(
    summary: &ChunkSourceSummaryRecord,
    contract_source: &str,
) -> Result<ChunkContractEvaluation, String> {
    for (initial, path) in summary.paths.iter().enumerate() {
        if path.initial_value != (initial == 1)
            || path.final_value != path.writes.last().copied().unwrap_or(path.initial_value)
            || path
                .frame_writes
                .iter()
                .any(|(field, _)| matches!(field.as_str(), "busy" | "lock"))
        {
            return Err("inconsistent or incomplete Boolean path record".into());
        }
    }
    let parsed = parse_source_program(contract_source).map_err(|error| error.to_string())?;
    let declarations = parsed
        .declarations
        .iter()
        .filter(|declaration| declaration.value.name == summary.function.as_str())
        .collect::<Vec<_>>();
    let [declaration] = declarations.as_slice() else {
        return Err(format!(
            "expected exactly one QCore declaration for {}",
            summary.function.as_str()
        ));
    };
    let declaration = &declaration.value;
    if declaration.kind != DeclarationKind::ExternFunction || declaration.parameters.len() != 1 {
        return Err("bounded contract requires one borrowed external parameter".into());
    }
    let parameter = &declaration.parameters[0];
    let parameter_tokens = tokens(&parameter.type_source)?;
    let [
        TokenKind::Word(borrow),
        TokenKind::Symbol(Symbol::Less),
        TokenKind::Symbol(Symbol::Apostrophe),
        TokenKind::Word(_lifetime),
        TokenKind::Symbol(Symbol::Comma),
        TokenKind::Word(mode),
        TokenKind::Symbol(Symbol::Comma),
        TokenKind::Word(resource),
        TokenKind::Symbol(Symbol::Greater),
    ] = parameter_tokens.as_slice()
    else {
        return Err("unsupported borrowed chunk parameter type".into());
    };
    let expected_resource = if summary.result_kind == BooleanFieldResult::Unit {
        summary
            .profile_assumptions
            .get("boolean_field_record")
            .ok_or("unit source lacks record identity")?
            .as_str()
    } else {
        match summary.field {
            ChunkSourceField::Busy => "InputChunk",
            ChunkSourceField::Locked => "Chunk",
        }
    };
    if borrow != "borrow"
        || resource != expected_resource
        || !matches!(mode.as_str(), "mut" | "const")
    {
        return Err("QCore parameter does not match the C chunk field profile".into());
    }
    if mode == "const"
        && summary
            .paths
            .as_slice()
            .iter()
            .any(|path| !path.writes.is_empty() || !path.frame_writes.is_empty())
    {
        return Err("C source writes through a const QCore borrow".into());
    }
    let arrow = declaration
        .tokens
        .iter()
        .position(|token| *token == TokenKind::Symbol(Symbol::Arrow))
        .ok_or("missing QCore result type")?;
    let result_type = match summary.result_kind {
        BooleanFieldResult::I32 => "i32",
        BooleanFieldResult::Unit => "unit",
    };
    if declaration.tokens.get(arrow + 1) != Some(&TokenKind::Word(result_type.into()))
        || declaration.tokens.get(arrow + 2) != Some(&TokenKind::Word("contract".into()))
    {
        return Err(
            "bounded contract requires an ordinary i32 result without extra effects".into(),
        );
    }
    let contract = declaration
        .function_contract
        .as_ref()
        .ok_or("missing QCore contract")?;
    crate::quant_condition::declaration(declaration)?;
    let mut requirements = BTreeSet::new();
    let mut postconditions = Vec::new();
    let mut variables = BTreeSet::new();
    for clause in &contract.clauses {
        match &clause.value.kind {
            SourceContractClauseKind::Requires => {
                let clause_tokens = tokens(&clause.value.expression_source)?;
                let [TokenKind::Word(name)] = clause_tokens.as_slice() else {
                    return Err("unsupported bounded source precondition".into());
                };
                if !requirements.insert(name.clone()) {
                    return Err(format!("duplicate precondition {name}"));
                }
            }
            SourceContractClauseKind::QuantCondition(selector) => {
                use qcore_schemas::quant_condition::Selector as C;
                let condition = qcore_frontend::quant_condition::parse(
                    selector.clone(),
                    &clause.value.expression_source,
                )?;
                match selector {
                    C::Entry => {
                        let role = crate::boolean_condition_profile::entry_requirement(
                            &condition,
                            summary.result_kind == BooleanFieldResult::Unit,
                        )?;
                        if !requirements.insert(role.into()) {
                            return Err("duplicate Boolean source entry role".into());
                        }
                    }
                    C::Normal => {
                        let predicate = Predicate::from_condition(
                            &condition.proposition,
                            &mut variables,
                            summary.result_kind == BooleanFieldResult::Unit,
                        )?;
                        postconditions.push((clause.value.expression_source.clone(), predicate));
                    }
                    _ => {
                        return Err(
                            "Boolean source profile supports entry/normal conditions only".into(),
                        );
                    }
                }
            }
            SourceContractClauseKind::Ensures(SourceContractExitSelector::Normal) => {
                let clause_tokens = tokens(&clause.value.expression_source)?;
                let mut parser = PredicateParser {
                    tokens: &clause_tokens,
                    cursor: 0,
                    variables: &mut variables,
                };
                let predicate = parser.predicate(0)?;
                if parser.cursor != clause_tokens.len() {
                    return Err(format!(
                        "unsupported trailing predicate syntax: {}",
                        clause.value.expression_source
                    ));
                }
                postconditions.push((clause.value.expression_source.clone(), predicate));
            }
            _ => {
                return Err(
                    "bounded source profile supports requires and ensures normal only".into(),
                );
            }
        }
    }
    let required = if summary.result_kind == BooleanFieldResult::Unit {
        vec![
            "object_is_live",
            "exclusive_object_access",
            "field_is_boolean",
        ]
    } else {
        vec![
            "chunk_is_live",
            "exclusive_chunk_access",
            "busy_is_boolean",
            "lock_is_boolean",
        ]
    }
    .into_iter()
    .map(str::to_owned)
    .collect();
    if requirements != required {
        return Err(
            "contract must state live chunk, exclusive access, and both Boolean field domains"
                .into(),
        );
    }
    if summary.result_kind == BooleanFieldResult::I32
        && (variables.contains("field") || variables.contains("old_field"))
    {
        return Err("generic field predicates require the unit-result profile".into());
    }
    if summary.result_kind == BooleanFieldResult::Unit
        && (variables.iter().any(|name| {
            matches!(
                name.as_str(),
                "result" | "busy" | "lock" | "old_busy" | "old_lock"
            )
        }) || summary.paths.iter().any(|path| path.return_value != 0))
    {
        return Err("void source contracts cannot observe a scalar result".into());
    }
    if variables.iter().any(|name| {
        !matches!(
            name.as_str(),
            "field" | "old_field" | "busy" | "old_busy" | "lock" | "old_lock" | "result"
        ) && !name.starts_with("field_")
    }) {
        return Err("unsupported Boolean source condition parameter".into());
    }
    let constrained = if summary.result_kind == BooleanFieldResult::Unit {
        vec!["field"]
    } else {
        vec!["result", "busy", "lock"]
    };
    if !constrained.iter().all(|name| variables.contains(*name)) {
        return Err("contract must constrain result and both exit fields".into());
    }
    for path in summary.paths.as_slice() {
        let mut frame_values = BTreeMap::new();
        for (field, number) in &path.frame_writes {
            let name = format!("field_{field}");
            if !variables.contains(&name) {
                return Err(format!(
                    "contract must constrain helper-written field {name}"
                ));
            }
            frame_values.insert(name, i64::from(*number));
        }
        let companions: &[bool] = if summary.result_kind == BooleanFieldResult::Unit {
            &[false]
        } else {
            &[false, true]
        };
        for &other in companions {
            let (old_busy, old_lock, busy, lock) = match summary.field {
                ChunkSourceField::Busy => (path.initial_value, other, path.final_value, other),
                ChunkSourceField::Locked => (other, path.initial_value, other, path.final_value),
            };
            let mut values = BTreeMap::from([
                ("old_busy", i64::from(old_busy)),
                ("old_lock", i64::from(old_lock)),
                ("busy", i64::from(busy)),
                ("lock", i64::from(lock)),
                ("result", i64::from(path.return_value)),
                ("old_field", i64::from(path.initial_value)),
                ("field", i64::from(path.final_value)),
            ]);
            values.extend(
                frame_values
                    .iter()
                    .map(|(name, number)| (name.as_str(), *number)),
            );
            for (source, predicate) in &postconditions {
                if predicate.evaluate(&values)? == 0 {
                    return Err(format!(
                        "contract mismatch in {}: `{source}`; old_busy={}, old_lock={}, busy={}, lock={}, result={}",
                        summary.function.as_str(),
                        i32::from(old_busy),
                        i32::from(old_lock),
                        i32::from(busy),
                        i32::from(lock),
                        path.return_value
                    ));
                }
            }
        }
    }
    Ok(ChunkContractEvaluation {
        function: summary.function.as_str().to_owned(),
        contract_source: contract_source.to_owned(),
        checked_states: if summary.result_kind == BooleanFieldResult::Unit {
            2
        } else {
            4
        },
        dependency_sources: summary.dependencies.clone(),
    })
}

fn tokens(source: &str) -> Result<Vec<TokenKind>, String> {
    Ok(lex(source)
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|token| token.kind)
        .filter(|token| *token != TokenKind::Eof)
        .collect())
}

enum Predicate {
    Integer(i64),
    Variable(String),
    Add(Box<Self>, Box<Self>),
    Subtract(Box<Self>, Box<Self>),
    Equal(Box<Self>, Box<Self>),
    NotEqual(Box<Self>, Box<Self>),
    BoundParameters(Vec<String>, Box<Self>),
}

impl Predicate {
    fn from_condition(
        definition: &qcore_schemas::quant_proposition::Definition,
        variables: &mut BTreeSet<String>,
        unit: bool,
    ) -> Result<Self, String> {
        use qcore_schemas::quant_proposition::{Formula, Sort, Term};
        if definition.parameters.iter().any(|p| {
            p.sort
                != (Sort::Value {
                    name: "integer".into(),
                })
                || (!matches!(
                    p.name.as_str(),
                    "field" | "old_field" | "busy" | "old_busy" | "lock" | "old_lock" | "result"
                ) && !p.name.starts_with("field_"))
                || (unit
                    && matches!(
                        p.name.as_str(),
                        "busy" | "old_busy" | "lock" | "old_lock" | "result"
                    ))
                || (!unit && matches!(p.name.as_str(), "field" | "old_field"))
        }) {
            return Err("unsupported Boolean source condition parameter or sort".into());
        }
        fn term(value: &Term, variables: &mut BTreeSet<String>) -> Result<Predicate, String> {
            Ok(match value {
                Term::Variable { name } => {
                    variables.insert(name.clone());
                    Predicate::Variable(name.clone())
                }
                Term::Integer { decimal } => Predicate::Integer(
                    decimal
                        .parse()
                        .map_err(|_| "Boolean predicate integer is outside i64")?,
                ),
                Term::Add { left, right } => Predicate::Add(
                    Box::new(term(left, variables)?),
                    Box::new(term(right, variables)?),
                ),
                Term::Sub { left, right } => Predicate::Subtract(
                    Box::new(term(left, variables)?),
                    Box::new(term(right, variables)?),
                ),
                Term::Neg { term: body } => Predicate::Subtract(
                    Box::new(Predicate::Integer(0)),
                    Box::new(term(body, variables)?),
                ),
                _ => return Err("unsupported Boolean source integer term".into()),
            })
        }
        let body = match &definition.formula {
            Formula::Equal { left, right } => Self::Equal(
                Box::new(term(left, variables)?),
                Box::new(term(right, variables)?),
            ),
            Formula::Not { body } => match body.as_ref() {
                Formula::Equal { left, right } => Self::NotEqual(
                    Box::new(term(left, variables)?),
                    Box::new(term(right, variables)?),
                ),
                _ => return Err("unsupported Boolean source negated assertion".into()),
            },
            _ => {
                return Err("Boolean source exit assertion requires equality or inequality".into());
            }
        };
        // Declaring an unused output cannot satisfy the existing must-constrain
        // rule. Every declared input still needs an actual profile valuation.
        Ok(Self::BoundParameters(
            definition
                .parameters
                .iter()
                .map(|p| p.name.clone())
                .collect(),
            Box::new(body),
        ))
    }
    fn evaluate(&self, values: &BTreeMap<&str, i64>) -> Result<i64, String> {
        Ok(match self {
            Self::BoundParameters(parameters, body) => {
                if parameters
                    .iter()
                    .any(|name| !values.contains_key(name.as_str()))
                {
                    return Err("missing Boolean source condition parameter valuation".into());
                }
                body.evaluate(values)?
            }
            Self::Integer(value) => *value,
            Self::Variable(name) => *values.get(name.as_str()).ok_or("unknown ghost variable")?,
            Self::Add(left, right) => left
                .evaluate(values)?
                .checked_add(right.evaluate(values)?)
                .ok_or("contract integer overflow")?,
            Self::Subtract(left, right) => left
                .evaluate(values)?
                .checked_sub(right.evaluate(values)?)
                .ok_or("contract integer overflow")?,
            Self::Equal(left, right) => {
                i64::from(left.evaluate(values)? == right.evaluate(values)?)
            }
            Self::NotEqual(left, right) => {
                i64::from(left.evaluate(values)? != right.evaluate(values)?)
            }
        })
    }
}

struct PredicateParser<'a> {
    tokens: &'a [TokenKind],
    cursor: usize,
    variables: &'a mut BTreeSet<String>,
}

impl PredicateParser<'_> {
    fn take(&mut self, symbol: Symbol) -> bool {
        if self.tokens.get(self.cursor) == Some(&TokenKind::Symbol(symbol)) {
            self.cursor += 1;
            true
        } else {
            false
        }
    }

    fn predicate(&mut self, depth: usize) -> Result<Predicate, String> {
        let left = self.sum(depth)?;
        if self.take(Symbol::EqualEqual) {
            Ok(Predicate::Equal(Box::new(left), Box::new(self.sum(depth)?)))
        } else if self.take(Symbol::NotEqual) {
            Ok(Predicate::NotEqual(
                Box::new(left),
                Box::new(self.sum(depth)?),
            ))
        } else {
            Err("postcondition requires an equality or inequality".into())
        }
    }

    fn sum(&mut self, depth: usize) -> Result<Predicate, String> {
        let mut value = self.atom(depth)?;
        let mut terms = 0;
        loop {
            if self.take(Symbol::Plus) {
                value = Predicate::Add(Box::new(value), Box::new(self.atom(depth)?));
            } else if self.take(Symbol::Minus) {
                value = Predicate::Subtract(Box::new(value), Box::new(self.atom(depth)?));
            } else {
                return Ok(value);
            }
            terms += 1;
            if terms > 64 {
                return Err("contract expression exceeds term limit".into());
            }
        }
    }

    fn atom(&mut self, depth: usize) -> Result<Predicate, String> {
        if depth > 32 {
            return Err("contract expression exceeds depth limit".into());
        }
        if self.take(Symbol::Minus) {
            return Ok(Predicate::Subtract(
                Box::new(Predicate::Integer(0)),
                Box::new(self.atom(depth + 1)?),
            ));
        }
        if self.take(Symbol::LeftParen) {
            let value = self.sum(depth + 1)?;
            if !self.take(Symbol::RightParen) {
                return Err("unclosed contract expression".into());
            }
            return Ok(value);
        }
        let token = self
            .tokens
            .get(self.cursor)
            .ok_or("missing contract operand")?;
        self.cursor += 1;
        match token {
            TokenKind::Integer(value) => value
                .parse()
                .map(Predicate::Integer)
                .map_err(|_| "unsupported contract integer".into()),
            TokenKind::Word(name)
                if matches!(
                    name.as_str(),
                    "old_busy" | "old_lock" | "busy" | "lock" | "result" | "old_field" | "field"
                ) || name.starts_with("field_") =>
            {
                self.variables.insert(name.clone());
                Ok(Predicate::Variable(name.clone()))
            }
            _ => Err(format!("unsupported contract operand {token:?}")),
        }
    }
}
