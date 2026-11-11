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

//! Card SERVE-FORWARDED-CALLBACK-FORMALS (gateway rule 15w): a callback
//! formal handed on unchanged to the same clause position of a callee whose
//! declaration is in force satisfies the forwarder's own declaration; every
//! deviation keeps rule 13y's withdrawal. Route G, over the synthetic `fw_*`
//! declarations of `fixtures/contracts/forwarded_callbacks.qv` (trusted test
//! assumptions: a `Safe` here is `Safe` over them and promotes nothing).

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary,
    ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const PRELUDE: &str = "\
typedef int (*fw_cb)(void *, unsigned long, void *);
void c_free(void *p);
int fw_each(void *reader, fw_cb cb, void *context);
int fw_pair(void *reader, fw_cb cb, fw_cb alt, void *context);
int fw_other(void *reader, fw_cb cb, void *context);
static int visit(void *reader, unsigned long index, void *context) { return index > 3; }
";

const C_FREE: &str = "
extern fn c_free(p: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_released(p, nullable);
    ensures normal resource_return_none();
};
";

fn environment() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([
        (
            "forwarded_callbacks.qv".to_owned(),
            include_str!("fixtures/contracts/forwarded_callbacks.qv").to_owned(),
        ),
        (
            "c_free.qv".to_owned(),
            format!("module contract;\n{C_FREE}"),
        ),
    ]))
    .expect("the fixture contracts load")
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent SERVE-FORWARDED-CALLBACK-FORMALS test: {reason}");
            false
        }
    }
}

#[derive(Debug)]
struct Row {
    status: AnalysisStatus,
    unsupported: Vec<String>,
}

impl Row {
    fn safe(&self) -> bool {
        self.status == AnalysisStatus::Safe
    }
    fn refused_with(&self, key: &str) -> bool {
        self.status != AnalysisStatus::Safe && self.unsupported.iter().any(|d| d.contains(key))
    }
}

fn summary_of(outcome: &UnrestrictedLegacyResult) -> Option<&LegacyProgramSummary> {
    match outcome {
        UnrestrictedLegacyResult::Safe(certificate) => Some(&certificate.summary),
        UnrestrictedLegacyResult::Unsafe(summary) => Some(summary),
        UnrestrictedLegacyResult::Unknown { partial, .. } => partial.as_ref(),
    }
}

/// Every row of the unit `PRELUDE + source`, keyed by function.
fn rows(source: &str) -> BTreeMap<String, Row> {
    let text = format!("{PRELUDE}\n{source}\n");
    let entry = source
        .lines()
        .rev()
        .find(|line| line.contains('('))
        .and_then(|line| line.split('(').next())
        .and_then(|head| head.split_whitespace().last())
        .expect("a definition")
        .trim_start_matches('*')
        .to_owned();
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
        &entry,
        &environment(),
    )
    .unwrap_or_else(|error| panic!("fixture rejected by Clang: {error:?}\n{text}"));
    if let UnrestrictedLegacyResult::Safe(certificate) = &outcome {
        check_legacy_safety_certificate(certificate).expect("a Safe result rechecks");
    }
    let summary = summary_of(&outcome).unwrap_or_else(|| panic!("no summary: {outcome:?}"));
    summary
        .functions
        .iter()
        .map(|(name, row)| {
            (
                name.clone(),
                Row {
                    status: row.status,
                    unsupported: row
                        .diagnostics
                        .iter()
                        .filter_map(|d| match &d.kind {
                            LegacyDiagnosticKind::Unsupported { description } => {
                                Some(description.clone())
                            }
                            _ => None,
                        })
                        .collect(),
                },
            )
        })
        .collect()
}

/// A forwarder `NAME` with `BODY`, and its caller `use_NAME` passing the
/// identity callback `visit`.
fn forwarder(name: &str, body: &str) -> String {
    format!(
        "int {name}(void *reader, fw_cb cb, void *context) {{ {body} }}\n\
         int use_{name}(void *r, void *c) {{ return {name}(r, visit, c); }}\n"
    )
}

/// The forwarder stands: its caller decides over the caller's own
/// candidate, and the forwarder's own row reads the call as the callee's
/// custody contract.
fn assert_stands(rows: &BTreeMap<String, Row>, name: &str) {
    let caller = &rows[&format!("use_{name}")];
    assert!(caller.safe(), "use_{name} decides over `visit`: {caller:?}");
    let own = &rows[name];
    assert!(
        own.safe(),
        "{name}'s forward is its declared invocation: {own:?}"
    );
}

/// The forwarder is withdrawn exactly as before the card.
fn assert_withdrawn(rows: &BTreeMap<String, Row>, name: &str) {
    let caller = &rows[&format!("use_{name}")];
    assert!(
        caller.refused_with("callback-withdrawn"),
        "use_{name} refuses on the withdrawn declaration: {caller:?}"
    );
}

#[test]
fn an_exact_forward_is_accepted() {
    if !clang_available() {
        return;
    }
    let rows = rows(&forwarder(
        "fw_exact",
        "return fw_each(reader, cb, context);",
    ));
    assert_stands(&rows, "fw_exact");
}

#[test]
fn the_forwarded_clause_is_decided_at_the_forwarders_callers() {
    if !clang_available() {
        return;
    }
    let source = format!(
        "{}static int consume(void *reader, unsigned long index, void *context) {{ c_free(context); return 0; }}\n\
         int use_consume(void *r, void *c) {{ return fw_exact(r, consume, c); }}\n\
         int use_formal(void *r, fw_cb f, void *c) {{ return fw_exact(r, f, c); }}\n",
        forwarder("fw_exact", "return fw_each(reader, cb, context);")
    );
    let rows = rows(&source);
    assert_stands(&rows, "fw_exact");
    assert!(
        rows["use_consume"].refused_with("callback-effectful"),
        "an effectful candidate is still refused at the outer call: {:?}",
        rows["use_consume"]
    );
    assert!(
        rows["use_formal"].refused_with("callback-undecided"),
        "an undeclared function's formal is still an open set: {:?}",
        rows["use_formal"]
    );
}

#[test]
fn a_formal_reassigned_before_the_forward_is_refused() {
    if !clang_available() {
        return;
    }
    let rows = rows(&forwarder(
        "fw_reassigned",
        "cb = visit; return fw_each(reader, cb, context);",
    ));
    assert_withdrawn(&rows, "fw_reassigned");
}

#[test]
fn a_forward_to_a_different_position_is_refused() {
    if !clang_available() {
        return;
    }
    let rows = rows(&forwarder(
        "fw_position",
        "return fw_pair(reader, visit, cb, context);",
    ));
    assert_withdrawn(&rows, "fw_position");
}

#[test]
fn a_forward_with_other_actuals_is_refused() {
    if !clang_available() {
        return;
    }
    let rows = rows(&forwarder(
        "fw_swapped",
        "return fw_each(context, cb, reader);",
    ));
    assert_withdrawn(&rows, "fw_swapped");
}

#[test]
fn a_forward_to_an_uncontracted_callee_is_refused() {
    if !clang_available() {
        return;
    }
    let rows = rows(&forwarder(
        "fw_uncontracted",
        "return fw_other(reader, cb, context);",
    ));
    assert_withdrawn(&rows, "fw_uncontracted");
}

#[test]
fn a_formal_both_forwarded_and_invoked_locally_is_refused() {
    if !clang_available() {
        return;
    }
    let rows = rows(&forwarder(
        "fw_both",
        "cb(reader, 0, context); return fw_each(reader, cb, context);",
    ));
    assert_withdrawn(&rows, "fw_both");
}

#[test]
fn a_chain_of_exact_forwards_stands_and_a_withdrawn_callee_refuses() {
    if !clang_available() {
        return;
    }
    let source = format!(
        "{}{}{}{}",
        forwarder("fw_exact", "return fw_each(reader, cb, context);"),
        forwarder("fw_chain", "return fw_exact(reader, cb, context);"),
        forwarder(
            "fw_both",
            "cb(reader, 0, context); return fw_each(reader, cb, context);"
        ),
        forwarder("fw_on_withdrawn", "return fw_both(reader, cb, context);"),
    );
    let rows = rows(&source);
    assert_stands(&rows, "fw_exact");
    assert_stands(&rows, "fw_chain");
    assert_withdrawn(&rows, "fw_both");
    assert_withdrawn(&rows, "fw_on_withdrawn");
}

#[test]
fn a_recursive_forward_alone_never_stands() {
    if !clang_available() {
        return;
    }
    let rows = rows(&forwarder(
        "fw_self",
        "if (!reader) return 0; return fw_self(reader, cb, context);",
    ));
    assert_withdrawn(&rows, "fw_self");
}
