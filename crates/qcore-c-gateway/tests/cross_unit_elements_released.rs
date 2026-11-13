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

//! Card **LERI-CROSS-UNIT-ELEMENTS-RELEASED**: a released-element summary
//! (gateway rule 43) carried to a caller in another unit by rule 4c's
//! two-sided transport (the channel rule 40's derived link resolution feeds).
//!
//! The home unit defines an external `drain` whose body hands every element
//! of its formal's list to an element destroyer; its derived summary states
//! `ElementsReleased` at that formal. A caller in another unit imports it:
//! the clause crosses with the contract (the certificate compares the whole
//! contract), the caller applies it (it forgets its published objects, so a
//! later unlink of one refuses), carries it on through a wrapper rooted at
//! its own formal, and withholds a summary for a local actual. An import that
//! dropped or added the clause is refused, and a consumer that cannot apply a
//! container effect (the extern-call bridge) refuses the imported contract.
//! `CROSS_UNIT_VECTORS` are `QCore.CrossUnitElementRelease.vectors`.

use std::collections::{BTreeMap, BTreeSet};
use std::process::Command;

use qcore_c_gateway::gateway_rules;
use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, CCompilerDialect, CrossUnitExport, LegacyDiagnosticKind,
    LegacyFunctionScope, ResourceContracts, UnrestrictedLegacyResult,
    analyze_prepared_translation_unit_with_cross_unit, check_legacy_safety_certificate,
    export_cross_unit_homes, prepare_translation_unit,
};
use qcore_contract::contract_loader::load_resource_contract_sources;
use qcore_contract::extern_bridge::{DeclaredExternSignature, declared_contract};
use qcore_schemas::contract::{ContainerEffect, ResourceFunctionContract};

const PRELUDE: &str = r#"
#include <stddef.h>
#define container_of(ptr, type, member) ({                      \
      const typeof( ((type *)0)->member ) *__mptr = (ptr);      \
      (type *)( (char *)__mptr - offsetof(type,member) );})
struct mk_list { struct mk_list *prev, *next; };
#define mk_list_foreach_safe(curr, n, head) \
    for (curr = (head)->next, n = curr->next; curr != (head); curr = n, n = curr->next)
#define mk_list_entry( ptr, type, member ) container_of( ptr, type, member )
void mk_list_add(struct mk_list *entry, struct mk_list *head);
void mk_list_del(struct mk_list *entry);
void *q_alloc(void); void q_free(void *); void q_use(void *);
struct item { int close; struct mk_list _head; void *payload; };
struct queue { struct mk_list items; struct mk_list done; int total; };
int item_destroy(struct item *e);
int drain(struct queue *q);
void touch(struct queue *q);
void drain_static(void);
"#;

/// The home unit: `drain` (rule 34 over rule 10f(2)'s element, rule 43's
/// summary at its formal), `touch` (a plain borrowed formal) and
/// `drain_static` (a static root: rule 43 withholds its summary).
const HOME: &str = "static struct mk_list registry;
int drain(struct queue *q) { int c = 0; struct mk_list *it, *tmp;
     struct item *e; mk_list_foreach_safe(it, tmp, &q->items) {
     e = mk_list_entry(it, struct item, _head); item_destroy(e); c++; } return c; }
void touch(struct queue *q) { q_use(q); }
void drain_static(void) { struct mk_list *it, *tmp;
     struct item *e; mk_list_foreach_safe(it, tmp, &registry) {
     e = mk_list_entry(it, struct item, _head); item_destroy(e); } }";

const LIST: &str = "module contract;
extern fn mk_list_add(entry: ptr<mut, u8>, head: ptr<mut, u8>) -> unit
contract { requires resource_contract_v1; ensures normal resource_borrowed(entry); ensures normal resource_borrowed(head); ensures normal resource_published(entry, head); ensures normal resource_return_none(); };
extern fn mk_list_del(entry: ptr<mut, u8>) -> unit
contract { requires resource_contract_v1; ensures normal resource_borrowed(entry); ensures normal resource_unpublished(entry); ensures normal resource_return_none(); };
extern fn q_use(p: ptr<mut, u8>) -> unit
contract { requires resource_contract_v1; ensures normal resource_borrowed(p); ensures normal resource_return_none(); };
extern fn item_destroy(e: ptr<mut, u8>) -> i32
contract { requires resource_contract_v1; ensures normal resource_unpublished(e); ensures normal resource_released(e, nonnull); ensures normal resource_return_none(); };
";

const LINK: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn clang_available() -> bool {
    qcore_c_gateway::clang_command().is_ok()
}

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(
        &[
            (
                "q_base.qv".to_owned(),
                include_str!("fixtures/contracts/q_base.qv").to_owned(),
            ),
            ("list.qv".to_owned(), LIST.to_owned()),
        ]
        .into_iter()
        .collect(),
    )
    .unwrap()
}

fn unit(source: &str) -> qcore_c_gateway::legacy_c::PreparedTranslationUnit {
    prepare_translation_unit(&C11TranslationUnit {
        source,
        dialect: CCompilerDialect::Gnu17,
        function_scope: LegacyFunctionScope::MainSourceDefinitions,
        ..Default::default()
    })
}

/// The home's exports, as rule 4c's first phase derives them.
fn exports() -> BTreeMap<String, CrossUnitExport> {
    let home = unit(&format!("{PRELUDE}\n{HOME}\n"));
    export_cross_unit_homes(
        &home,
        &contracts(),
        &BTreeSet::from(["drain".into(), "touch".into(), "drain_static".into()]),
        LINK,
    )
    .expect("home export")
    .into_iter()
    .map(|export| (export.home.function.clone(), export))
    .collect()
}

/// What a caller analysis gave: its row, or the refusal of its imports.
enum Caller {
    Row(AnalysisStatus, Vec<LegacyDiagnosticKind>),
    Refused(String),
}

/// Analyse `body` in a caller unit holding `export`'s home, with `contract`
/// as the imported contract (the export's own, or a changed one).
fn caller(body: &str, export: &CrossUnitExport, contract: ResourceFunctionContract) -> Caller {
    let mut environment = contracts();
    let name = export.home.function.clone();
    environment.set_inferred_imports(BTreeMap::from([(name.clone(), contract)]));
    environment
        .set_cross_unit_import_identities(BTreeMap::from([(name, export.home.import_identity())]));
    environment.set_link_resolution_sha256(Some(LINK.to_owned()));
    let prepared = unit(&format!("{PRELUDE}\n{body}\n"));
    match analyze_prepared_translation_unit_with_cross_unit(
        &prepared,
        "subject",
        &environment,
        std::slice::from_ref(&export.home),
    ) {
        Err(error) => Caller::Refused(error.to_string()),
        Ok(UnrestrictedLegacyResult::Safe(certificate)) => {
            check_legacy_safety_certificate(&certificate).expect("caller and home replay");
            Caller::Row(AnalysisStatus::Safe, Vec::new())
        }
        Ok(UnrestrictedLegacyResult::Unsafe(summary))
        | Ok(UnrestrictedLegacyResult::Unknown {
            partial: Some(summary),
            ..
        }) => {
            let row = &summary.functions["subject"];
            (
                row.status,
                row.diagnostics.iter().map(|d| d.kind.clone()).collect(),
            )
                .into()
        }
        Ok(UnrestrictedLegacyResult::Unknown { reason, .. }) => Caller::Refused(reason),
    }
}

impl From<(AnalysisStatus, Vec<LegacyDiagnosticKind>)> for Caller {
    fn from((status, diagnostics): (AnalysisStatus, Vec<LegacyDiagnosticKind>)) -> Self {
        Self::Row(status, diagnostics)
    }
}

fn row(result: Caller) -> (AnalysisStatus, Vec<LegacyDiagnosticKind>) {
    match result {
        Caller::Row(status, diagnostics) => (status, diagnostics),
        Caller::Refused(reason) => panic!("caller refused: {reason}"),
    }
}

fn uncontracted(diagnostics: &[LegacyDiagnosticKind], callee: &str) -> bool {
    let wanted = format!("call to uncontracted function `{callee}`");
    diagnostics.iter().any(|kind| {
        matches!(kind, LegacyDiagnosticKind::Unsupported { description } if *description == wanted)
    })
}

fn accusations(diagnostics: &[LegacyDiagnosticKind]) -> usize {
    diagnostics
        .iter()
        .filter(|kind| {
            matches!(
                kind,
                LegacyDiagnosticKind::Leak { .. } | LegacyDiagnosticKind::DoubleRelease { .. }
            )
        })
        .count()
}

/// The extern-call bridge: a consumer with no container-effect model.
fn bridge(contract: &ResourceFunctionContract) -> Result<(), String> {
    declared_contract(
        "drain",
        &DeclaredExternSignature {
            parameter_names: vec!["q".into()],
            parameter_types: vec!["ptr<mut,u8>".into()],
            result_type: "unit".into(),
            module: "fixture".into(),
        },
        contract,
    )
    .map(|_| ())
}

/// A consumer of a call's contract.
#[derive(Clone, Copy, Debug)]
enum Consumer {
    /// The gateway's call reading, which applies container effects.
    Applies,
    /// The extern-call bridge, which has no container-effect model.
    BorrowOnly,
}

/// How the call was read (`QCore.CrossUnitElementRelease.Reading`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Reading {
    Borrow,
    Forget,
    Refused,
}

/// `QCore.CrossUnitElementRelease.vectors`: the import's `ElementsReleased`
/// positions, the replayed home's, the consumer and the outcome (`None`: the
/// import is refused).
type Vector = (
    &'static [usize],
    &'static [usize],
    Consumer,
    Option<Reading>,
);

const CROSS_UNIT_VECTORS: &[Vector] = &[
    (&[0], &[0], Consumer::Applies, Some(Reading::Forget)),
    (&[0], &[0], Consumer::BorrowOnly, Some(Reading::Refused)),
    (&[], &[0], Consumer::Applies, None),
    (&[0, 1], &[0], Consumer::Applies, None),
    (&[], &[], Consumer::Applies, Some(Reading::Borrow)),
    (&[], &[], Consumer::BorrowOnly, Some(Reading::Borrow)),
];

/// A caller that published `x` into the drained list and unlinks it after
/// the call: a forgetting reading refuses the unlink and accuses nothing; a
/// borrow reading would admit it.
const PUBLISHED_THEN_UNLINKED: &str = "void subject(struct queue *q) { struct item *x = q_alloc();
     if (!x) return; mk_list_add(&x->_head, &q->items); CALL; mk_list_del(&x->_head); q_free(x); }";

#[test]
fn the_vectors_hold_on_the_fixtures() {
    if std::env::var_os(CHILD).is_some() || !clang_available() {
        return;
    }
    let exports = exports();
    assert_eq!(CROSS_UNIT_VECTORS.len(), 6);
    for &(imported, replayed, consumer, expected) in CROSS_UNIT_VECTORS {
        let export = if replayed.is_empty() {
            &exports["touch"]
        } else {
            &exports["drain"]
        };
        let stated = export
            .contract
            .container_effects
            .keys()
            .copied()
            .collect::<Vec<_>>();
        assert_eq!(stated, replayed, "the home states its clauses");
        let mut contract = export.contract.clone();
        contract.container_effects = imported
            .iter()
            .map(|&index| (index, ContainerEffect::ElementsReleased))
            .collect();
        let call = format!("{}(q)", export.home.function);
        let got = match consumer {
            Consumer::BorrowOnly => Some(if bridge(&contract).is_ok() {
                Reading::Borrow
            } else {
                Reading::Refused
            }),
            Consumer::Applies => {
                match caller(
                    &PUBLISHED_THEN_UNLINKED.replace("CALL", &call),
                    export,
                    contract,
                ) {
                    Caller::Refused(reason) => {
                        assert!(
                            reason.contains("cross-unit imported contracts differ"),
                            "{reason}"
                        );
                        None
                    }
                    Caller::Row(AnalysisStatus::Safe, _) => Some(Reading::Borrow),
                    Caller::Row(status, diagnostics) => {
                        assert_eq!(status, AnalysisStatus::Unknown, "{diagnostics:?}");
                        assert_eq!(accusations(&diagnostics), 0, "{diagnostics:?}");
                        Some(Reading::Forget)
                    }
                }
            }
        };
        assert_eq!(got, expected, "{imported:?} {replayed:?} {consumer:?}");
    }
}

#[test]
fn the_exported_summary_states_the_clause_and_a_static_root_exports_none() {
    if std::env::var_os(CHILD).is_some() || !clang_available() {
        return;
    }
    let exports = exports();
    assert_eq!(
        exports["drain"].contract.container_effects,
        BTreeMap::from([(0, ContainerEffect::ElementsReleased)])
    );
    assert!(exports["touch"].contract.container_effects.is_empty());
    assert!(
        !exports.contains_key("drain_static"),
        "{:?}",
        exports.keys()
    );
}

#[test]
fn an_imported_clause_reaches_the_caller_and_both_bodies_replay() {
    if std::env::var_os(CHILD).is_some() || !clang_available() {
        return;
    }
    let exports = exports();
    let drain = &exports["drain"];
    for body in [
        "void subject(struct queue *q) { drain(q); }",
        "void subject(void *data) { struct queue *q = data; if (drain(q) > 0) q_use(q); }",
        // Published, then drained: the drain may release it, nothing leaks.
        "void subject(struct queue *q) { struct item *x = q_alloc(); if (!x) return;
         mk_list_add(&x->_head, &q->items); drain(q); }",
        // The clause carried on through a wrapper rooted at its own formal.
        "static void wrapper(struct queue *q) { (void) drain(q); }
         void subject(struct queue *q) { wrapper(q); }",
    ] {
        let (status, diagnostics) = row(caller(body, drain, drain.contract.clone()));
        assert_eq!(status, AnalysisStatus::Safe, "{body}\n{diagnostics:?}");
    }
}

#[test]
fn a_local_actual_withholds_the_callers_summary() {
    if std::env::var_os(CHILD).is_some() || !clang_available() {
        return;
    }
    let exports = exports();
    let drain = &exports["drain"];
    let (status, diagnostics) = row(caller(
        "static void local(void) { struct queue l; l.items.next = &l.items;
         l.items.prev = &l.items; drain(&l); }
         void subject(void) { local(); }",
        drain,
        drain.contract.clone(),
    ));
    assert_eq!(status, AnalysisStatus::Unknown);
    assert!(uncontracted(&diagnostics, "local"), "{diagnostics:?}");
}

#[test]
fn a_missing_import_leaves_the_call_uncontracted() {
    if std::env::var_os(CHILD).is_some() || !clang_available() {
        return;
    }
    let prepared = unit(&format!(
        "{PRELUDE}\nvoid subject(struct queue *q) {{ drain(q); }}\n"
    ));
    let outcome =
        analyze_prepared_translation_unit_with_cross_unit(&prepared, "subject", &contracts(), &[])
            .expect("caller analysis");
    let UnrestrictedLegacyResult::Unknown {
        partial: Some(summary),
        ..
    } = outcome
    else {
        panic!("an uncontracted call is not Safe");
    };
    let diagnostics = summary.functions["subject"]
        .diagnostics
        .iter()
        .map(|d| d.kind.clone())
        .collect::<Vec<_>>();
    assert!(uncontracted(&diagnostics, "drain"), "{diagnostics:?}");
}

// ---- the switch ------------------------------------------------------------
//
// With rule 43 off the home publishes no summary for `drain`, so nothing is
// exported and the caller's call stays uncontracted (`child_*` tests do
// nothing unless `CROSS_UNIT_ELEMENTS_RELEASED_CHILD` names them).

const CHILD: &str = "CROSS_UNIT_ELEMENTS_RELEASED_CHILD";

#[test]
fn switching_rule_43_off_exports_no_released_element_summary() {
    if std::env::var_os(CHILD).is_some() || !clang_available() {
        return;
    }
    let test = "child_rule_43_off";
    let status = Command::new(std::env::current_exe().expect("the test binary"))
        .args([test, "--exact", "--nocapture", "--test-threads=1"])
        .env(CHILD, test)
        .env(gateway_rules::RULES_OFF_VARIABLE, "43")
        .status()
        .expect("the child test runs");
    assert!(status.success(), "{test}: {status}");
}

#[test]
fn child_rule_43_off() {
    if std::env::var(CHILD).is_ok_and(|named| named != "child_rule_43_off")
        || std::env::var_os(CHILD).is_none()
    {
        return;
    }
    assert_eq!(
        gateway_rules::effective_off()
            .into_iter()
            .collect::<Vec<_>>(),
        ["43"]
    );
    let exports = exports();
    assert!(!exports.contains_key("drain"), "{:?}", exports.keys());
    assert!(exports.contains_key("touch"));
}
