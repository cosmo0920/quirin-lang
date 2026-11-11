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

use super::*;
use crate::cleanup_lowering::SourceOp;
use crate::frontend::{elaborate_source, parse_source_program};
use crate::ownership::Footprint;
use crate::resource_flow::Quantity;

fn zero_summary(parameters: Vec<ParameterOwnership>) -> FunctionSummary {
    FunctionSummary {
        effects: EffectRow::default(),
        flow: Flow::zero(),
        parameters,
    }
}

fn lowering(name: &str, resources: Vec<ResourceId>) -> LoweringProgram {
    LoweringProgram::new(
        name,
        resources,
        ["Failure".into()],
        vec![SourceOp::Return(0)],
    )
    .unwrap()
}

fn semantics(
    symbol: SymbolId,
    name: &str,
    resources: Vec<ResourceId>,
    calls: Vec<CallSite>,
) -> FunctionSemantics {
    FunctionSemantics {
        symbol,
        usage_declarations: BTreeMap::new(),
        usage_expression: UsageExpr::Unit,
        ownership_operations: Vec::new(),
        resource_segments: vec![Flow::zero(); calls.len() + 1],
        resource_bounds: ResourceBounds::new(Quantity::zero(), Quantity::zero()),
        allowed_effects: EffectRow::default(),
        initial_error_ledger: ErrorLedger::default(),
        error_operations: Vec::new(),
        region_operations: Vec::new(),
        lowering: lowering(name, resources),
        calls,
        identity_entry: Vec::new(),
        identity_exit: Vec::new(),
        parameters: Vec::new(),
        recursive_contract: None,
    }
}

fn typed(source: &str) -> TypedSourceProgram {
    elaborate_source(&parse_source_program(source).unwrap()).unwrap()
}

fn call_span(program: &TypedSourceProgram, caller: &str) -> Span {
    program
        .declarations
        .iter()
        .find(|declaration| declaration.name == caller)
        .unwrap()
        .statements[0]
        .span
}

#[test]
fn checks_an_acyclic_whole_program_and_preserves_cfg_observation() {
    let source = typed("fn leaf() -> unit { return; } fn root() -> unit { leaf(); return; }");
    let leaf = source.globals["leaf"];
    let root = source.globals["root"];
    let checked = check_whole_program(
        &source,
        &ErrorDeclarations::new(["Failure".into()]),
        vec![
            semantics(leaf, "leaf", Vec::new(), Vec::new()),
            semantics(
                root,
                "root",
                Vec::new(),
                vec![CallSite {
                    callee: leaf,
                    arguments: Vec::new(),
                    result: None,
                    span: call_span(&source, "root"),
                    conditional: false,
                }],
            ),
        ],
    )
    .unwrap();
    assert_eq!(checked.functions.len(), 2);
    assert!(checked.recursive_functions.is_empty());
    assert_eq!(
        interpret_cfg(&checked.functions[&root].cfg).unwrap(),
        interpret_source(&lowering("root", Vec::new())).unwrap()
    );
}

#[test]
fn supplied_usage_contract_cannot_override_the_source_annotation() {
    let source = typed("fn discard(x: i32 use one) -> i32 { return 0; }");
    let symbol = source.globals["discard"];
    for declared in [None, Some(Usage::MANY), Some(Usage::ONE)] {
        let mut body = semantics(symbol, "discard", Vec::new(), Vec::new());
        if let Some(grade) = declared {
            body.usage_declarations.insert("x".into(), grade);
        }
        let errors = check_whole_program(
            &source,
            &ErrorDeclarations::new(["Failure".into()]),
            vec![body],
        )
        .unwrap_err();
        assert_eq!(errors[0].pass, SemanticPass::Usage);
    }
}

#[test]
fn recursive_scc_requires_an_explicit_contract() {
    let source = typed("fn looped() -> unit { looped(); return; }");
    let symbol = source.globals["looped"];
    let error = check_whole_program(
        &source,
        &ErrorDeclarations::new(["Failure".into()]),
        vec![semantics(
            symbol,
            "looped",
            Vec::new(),
            vec![CallSite {
                callee: symbol,
                arguments: Vec::new(),
                result: None,
                span: call_span(&source, "looped"),
                conditional: false,
            }],
        )],
    )
    .unwrap_err();
    assert!(error[0].message.contains("explicit summary"));
}

#[test]
fn recursive_post_fixed_summary_is_accepted() {
    let source = typed("fn looped() -> unit { looped(); return; }");
    let symbol = source.globals["looped"];
    let mut body = semantics(
        symbol,
        "looped",
        Vec::new(),
        vec![CallSite {
            callee: symbol,
            arguments: Vec::new(),
            result: None,
            span: call_span(&source, "looped"),
            conditional: false,
        }],
    );
    body.recursive_contract = Some(zero_summary(Vec::new()));
    let checked = check_whole_program(
        &source,
        &ErrorDeclarations::new(["Failure".into()]),
        vec![body],
    )
    .unwrap();
    assert!(checked.recursive_functions.contains(&symbol));
    assert_eq!(checked.functions[&symbol].summary, zero_summary(Vec::new()));
}

#[test]
fn recursive_summary_must_cover_one_unfolding() {
    let source = typed("fn looped() -> unit { looped(); return; }");
    let symbol = source.globals["looped"];
    let mut body = semantics(
        symbol,
        "looped",
        Vec::new(),
        vec![CallSite {
            callee: symbol,
            arguments: Vec::new(),
            result: None,
            span: call_span(&source, "looped"),
            conditional: false,
        }],
    );
    body.resource_segments[0] = Flow::acquire(Quantity::single("memory", 1), Quantity::zero());
    body.recursive_contract = Some(zero_summary(Vec::new()));
    let errors = check_whole_program(
        &source,
        &ErrorDeclarations::new(["Failure".into()]),
        vec![body],
    )
    .unwrap_err();
    assert!(
        errors
            .iter()
            .any(|error| error.message.contains("post-fixed"))
    );
}

#[test]
fn consumed_parameter_transfers_the_same_identity_across_a_call() {
    let source = typed(
        "fn sink(x: own<File>) -> unit { return; } fn root(x: own<File>) -> unit { sink(move x); return; }",
    );
    let sink = source.globals["sink"];
    let root = source.globals["root"];
    let mut sink_body = semantics(sink, "sink", vec![7], Vec::new());
    sink_body.parameters = vec![ParameterOwnership::Consumed];
    let mut root_body = semantics(
        root,
        "root",
        vec![7],
        vec![CallSite {
            callee: sink,
            arguments: vec![7],
            result: None,
            span: call_span(&source, "root"),
            conditional: false,
        }],
    );
    root_body.identity_entry = vec![7];
    root_body.parameters = vec![ParameterOwnership::Consumed];
    let checked = check_whole_program(
        &source,
        &ErrorDeclarations::new(["Failure".into()]),
        vec![sink_body, root_body],
    )
    .unwrap();
    assert_eq!(
        checked.functions[&sink].summary.parameters,
        vec![ParameterOwnership::Consumed]
    );
}

#[test]
fn returned_parameter_preserves_identity_for_the_caller() {
    let source = typed(
        "fn inspect(x: own<File>) -> unit { return; } fn root(x: own<File>) -> unit { inspect(move x); return; }",
    );
    let inspect = source.globals["inspect"];
    let root = source.globals["root"];
    let mut inspect_body = semantics(inspect, "inspect", vec![9], Vec::new());
    inspect_body.parameters = vec![ParameterOwnership::Returned];
    let mut root_body = semantics(
        root,
        "root",
        vec![9],
        vec![CallSite {
            callee: inspect,
            arguments: vec![9],
            result: None,
            span: call_span(&source, "root"),
            conditional: false,
        }],
    );
    root_body.identity_entry = vec![9];
    root_body.identity_exit = vec![9];
    root_body.parameters = vec![ParameterOwnership::Returned];
    check_whole_program(
        &source,
        &ErrorDeclarations::new(["Failure".into()]),
        vec![inspect_body, root_body],
    )
    .unwrap();
}

#[test]
fn owned_call_parameters_cannot_alias_one_identity() {
    let source = typed(
        "fn pair(a: own<File>, b: own<File>) -> unit { return; } fn root(x: own<File>) -> unit { pair(move x, move x); return; }",
    );
    let pair = source.globals["pair"];
    let root = source.globals["root"];
    let mut pair_body = semantics(pair, "pair", vec![3], Vec::new());
    pair_body.parameters = vec![ParameterOwnership::Consumed, ParameterOwnership::Returned];
    let mut root_body = semantics(
        root,
        "root",
        vec![3],
        vec![CallSite {
            callee: pair,
            arguments: vec![3, 3],
            result: None,
            span: call_span(&source, "root"),
            conditional: false,
        }],
    );
    root_body.identity_entry = vec![3];
    root_body.parameters = vec![ParameterOwnership::Consumed];
    let errors = check_whole_program(
        &source,
        &ErrorDeclarations::new(["Failure".into()]),
        vec![pair_body, root_body],
    )
    .unwrap_err();
    assert!(errors.iter().any(|error| error.message.contains("aliases")));
}

#[test]
fn local_ownership_failure_is_reported_at_the_function_span() {
    let source = typed("fn bad() -> unit { return; }");
    let symbol = source.globals["bad"];
    let mut body = semantics(symbol, "bad", vec![1], Vec::new());
    body.ownership_operations = vec![OwnershipOp::Acquire {
        id: 1,
        owner: "leak".into(),
        resource_type: "File".into(),
        footprint: Footprint::single("files", 1),
    }];
    let errors = check_whole_program(
        &source,
        &ErrorDeclarations::new(["Failure".into()]),
        vec![body],
    )
    .unwrap_err();
    assert_eq!(errors[0].pass, SemanticPass::Ownership);
    assert_eq!(errors[0].span, source.declarations[0].span);
}

#[test]
fn callee_effects_must_fit_the_callers_declared_row() {
    let source = typed("fn io() -> unit { return; } fn root() -> unit { io(); return; }");
    let io = source.globals["io"];
    let root = source.globals["root"];
    let mut io_body = semantics(io, "io", Vec::new(), Vec::new());
    io_body.allowed_effects = EffectRow::new([crate::Effect::Io]);
    io_body.error_operations = vec![ErrorOp::Perform(crate::Effect::Io)];
    let root_body = semantics(
        root,
        "root",
        Vec::new(),
        vec![CallSite {
            callee: io,
            arguments: Vec::new(),
            result: None,
            span: call_span(&source, "root"),
            conditional: false,
        }],
    );
    let errors = check_whole_program(
        &source,
        &ErrorDeclarations::new(["Failure".into()]),
        vec![io_body, root_body],
    )
    .unwrap_err();
    assert!(
        errors
            .iter()
            .any(|error| error.pass == SemanticPass::TypedErrors
                && error.message.contains("callee effects"))
    );
}

#[test]
fn source_contract_clauses_cannot_be_bypassed_by_supplied_semantics() {
    for clause in [
        "requires true;",
        "ensures normal true;",
        "quant normal wall_clock <= 5;",
        "quant registered #coll: collector.event |-> collector.evl;",
        "quant #threaded = (#ins.is_threaded != 0);",
        "quant #threaded in { requires true; }: [[local]];",
    ] {
        let text = format!("fn main() -> i32 contract {{ {clause} }} {{ return 0; }}");
        let source = typed(&text);
        let errors = check_whole_program(
            &source,
            &ErrorDeclarations::new(["Failure".into()]),
            vec![semantics(
                source.globals["main"],
                "main",
                Vec::new(),
                Vec::new(),
            )],
        )
        .unwrap_err();
        assert!(errors.iter().any(|error| error.code == "Q021"
            && error.message.contains("refusing unchecked contract")
            && text[error.span.start..error.span.end].contains(clause.trim_end_matches(';'))));
    }
    for contract in ["", "contract {}"] {
        let source = typed(&format!("fn main() -> i32 {contract} {{ return 0; }}"));
        assert!(
            check_whole_program(
                &source,
                &ErrorDeclarations::new(["Failure".into()]),
                vec![semantics(
                    source.globals["main"],
                    "main",
                    Vec::new(),
                    Vec::new()
                )],
            )
            .is_ok()
        );
    }
}
