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

//! Card **CG22-CALLBACK-CANDIDATE-CONTRACTS** (gateway rule 4a amended, rule
//! 13u): under the main-source scope, a `static` callback a header defines and
//! a selected body hands to a synchronous invocation or a table dispatch is
//! offered to the W4 derivation (as `rd_kafka_ApiVersion_key_cmp`, handed to
//! `bsearch` and `qsort`), so its validated summary is the candidate's
//! contract in force.
//!
//! * **accepting:** an identity header comparator (one that reads its formals
//!   only) decides the invocation, and the caller is `Safe` and re-certifies;
//! * **refusing:** a header callback whose body is not validated (an
//!   uncontracted call) keeps `callback-uncontracted`; one whose summary
//!   releases the context keeps `callback-effectful`; a table holding such a
//!   handler beside an identity one stays refused; a header static named in a
//!   body that calls no callback-invoking declaration is not reached;
//! * **not read benign:** the header callbacks are never reported rows.
//!
//! The callees are the synthetic `c_*` declarations of
//! `fixtures/contracts/callbacks.qv` (trusted test assumptions).

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyFunctionScope,
    LegacyProgramSummary, ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent CG22-CALLBACK-CANDIDATE-CONTRACTS test: {reason}");
            false
        }
    }
}

const PRELUDE: &str = "\
struct entry { const char *name; int (*handler)(void *, unsigned long, void *); };
void c_free(void *p);
void unknown_effect(void *p);
int c_each(void *reader, int (*cb)(void *, unsigned long, void *), void *context);
int c_dispatch(void *reader, struct entry *table, void *context);
";

/// The header callbacks: `static` definitions the main-source selection does
/// not report and does not call directly.
const HEADER: &str = "\
static int key_cmp(void *reader, unsigned long index, void *context) { return *(int *)context > (int)index; }
static int consume(void *reader, unsigned long index, void *context) { c_free(context); return 0; }
static int opaque_effect(void *reader, unsigned long index, void *context) { unknown_effect(context); return 0; }
static int unused_cb(void *reader, unsigned long index, void *context) { return 0; }
";

fn environment() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "callbacks.qv".to_owned(),
        include_str!("fixtures/contracts/callbacks.qv").to_owned(),
    )]))
    .expect("the fixture contracts load")
}

#[derive(Debug)]
struct Run {
    summary: LegacyProgramSummary,
    safe_certificate: bool,
}

/// Analyse `entry` of `main` with the header included, under the main-source
/// scope; a `Safe` result's certificate is re-checked.
fn run(tag: &str, main: &str, entry: &str) -> Run {
    let directory = std::env::temp_dir().join(format!(
        "qcore-cg22-cand-{tag}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::create_dir_all(&directory).expect("a temporary directory");
    std::fs::write(directory.join("cb.h"), format!("{PRELUDE}{HEADER}")).expect("the header");
    let source = format!("#include \"cb.h\"\n{main}\n");
    let unit = C11TranslationUnit {
        source: &source,
        include_paths: vec![directory.to_string_lossy().into_owned()],
        function_scope: LegacyFunctionScope::MainSourceDefinitions,
        ..C11TranslationUnit::default()
    };
    let result = analyze_unrestricted_c11(&unit, entry, &environment())
        .unwrap_or_else(|error| panic!("fixture rejected: {error:?}\n{source}"));
    std::fs::remove_dir_all(&directory).expect("cleanup");
    match result {
        UnrestrictedLegacyResult::Safe(certificate) => {
            check_legacy_safety_certificate(&certificate).expect("a Safe result rechecks");
            Run {
                summary: certificate.summary,
                safe_certificate: true,
            }
        }
        UnrestrictedLegacyResult::Unsafe(summary) => Run {
            summary,
            safe_certificate: false,
        },
        UnrestrictedLegacyResult::Unknown {
            partial: Some(summary),
            ..
        } => Run {
            summary,
            safe_certificate: false,
        },
        other => panic!("expected an analysable unit: {other:?}"),
    }
}

fn unsupported(run: &Run, function: &str) -> Vec<String> {
    run.summary.functions[function]
        .diagnostics
        .iter()
        .filter_map(|diagnostic| match &diagnostic.kind {
            LegacyDiagnosticKind::Unsupported { description } => Some(description.clone()),
            _ => None,
        })
        .collect()
}

fn refused_with(run: &Run, function: &str, key: &str) -> bool {
    run.summary.functions[function].status != AnalysisStatus::Safe
        && unsupported(run, function)
            .iter()
            .any(|description| description.contains(key))
}

#[test]
fn a_header_comparator_handed_to_an_invocation_is_in_force() {
    if !clang_available() {
        return;
    }
    let run = run(
        "identity",
        "int user(void *reader, int *context) { return c_each(reader, key_cmp, context); }",
        "user",
    );
    // Only the selection is reported; the header comparator is not a row.
    assert_eq!(
        run.summary.functions.keys().cloned().collect::<Vec<_>>(),
        vec!["user".to_owned()]
    );
    assert_eq!(
        run.summary.functions["user"].status,
        AnalysisStatus::Safe,
        "{:?}",
        unsupported(&run, "user")
    );
    assert!(run.safe_certificate);
}

#[test]
fn an_unvalidated_header_callback_keeps_the_invocation_refused() {
    if !clang_available() {
        return;
    }
    let run = run(
        "opaque",
        "int user(void *reader, void *context) { return c_each(reader, opaque_effect, context); }",
        "user",
    );
    assert!(
        refused_with(&run, "user", "callback-uncontracted"),
        "{:?}",
        unsupported(&run, "user")
    );
}

#[test]
fn an_effectful_header_callback_keeps_the_invocation_refused() {
    if !clang_available() {
        return;
    }
    let run = run(
        "consume",
        "int user(void *reader, void *context) { return c_each(reader, consume, context); }",
        "user",
    );
    assert!(
        refused_with(&run, "user", "callback-effectful"),
        "{:?}",
        unsupported(&run, "user")
    );
}

#[test]
fn a_table_with_an_unvalidated_header_handler_stays_refused() {
    if !clang_available() {
        return;
    }
    let mixed = run(
        "table-mixed",
        "int user(void *reader, int *context) {\n\
             struct entry table[] = { { \"a\", key_cmp }, { \"b\", opaque_effect }, { 0, 0 } };\n\
             return c_dispatch(reader, table, context);\n\
         }",
        "user",
    );
    assert_ne!(
        mixed.summary.functions["user"].status,
        AnalysisStatus::Safe,
        "{:?}",
        unsupported(&mixed, "user")
    );
    assert!(
        refused_with(&mixed, "user", "callback-uncontracted"),
        "{:?}",
        unsupported(&mixed, "user")
    );
    // Every member in force: the table decides.
    let closed = run(
        "table-closed",
        "int user(void *reader, int *context) {\n\
             struct entry table[] = { { \"a\", key_cmp }, { 0, 0 } };\n\
             return c_dispatch(reader, table, context);\n\
         }",
        "user",
    );
    assert_eq!(
        closed.summary.functions["user"].status,
        AnalysisStatus::Safe,
        "{:?}",
        unsupported(&closed, "user")
    );
}

#[test]
fn a_designator_beside_no_invocation_reaches_nothing() {
    if !clang_available() {
        return;
    }
    // `unused_cb` is only stored, beside no callback-invoking call: it is not
    // offered to the derivation, and the stored pointer is called through an
    // unresolved object, so nothing here may be Safe on its account.
    let run = run(
        "stored",
        "int (*slot)(void *, unsigned long, void *);\n\
         int user(void *reader, void *context) { slot = unused_cb; return slot(reader, 0, context); }",
        "user",
    );
    assert_ne!(
        run.summary.functions["user"].status,
        AnalysisStatus::Safe,
        "{:?}",
        unsupported(&run, "user")
    );
}

// --- A candidate defined in another unit (rule 4c) --------------------------

/// A comparator defined (external linkage) in another unit, handed to a
/// synchronous invocation: with no import it has no contract in force and the
/// invocation refuses; with rule 4c's import (the stated link premise, the
/// home exported and replayed) its validated home summary is in force, as at
/// a direct call, and the caller's certificate replays both bodies.
#[test]
fn a_candidate_from_another_unit_is_in_force_only_under_its_import() {
    use std::collections::BTreeSet;

    use qcore_c_gateway::legacy_c::{
        analyze_prepared_translation_unit_with_cross_unit, export_cross_unit_homes,
        prepare_translation_unit,
    };

    if !clang_available() {
        return;
    }
    let home = prepare_translation_unit(&C11TranslationUnit {
        source: "int broker_cmp(void *reader, unsigned long index, void *context) \
                 { return *(int *)context > (int)index; }\n",
        function_scope: LegacyFunctionScope::MainSourceDefinitions,
        ..Default::default()
    });
    let caller_source = format!(
        "{PRELUDE}int broker_cmp(void *reader, unsigned long index, void *context);\n\
         int user(void *reader, int *context) {{ return c_each(reader, broker_cmp, context); }}\n"
    );
    let caller = prepare_translation_unit(&C11TranslationUnit {
        source: &caller_source,
        function_scope: LegacyFunctionScope::MainSourceDefinitions,
        ..Default::default()
    });
    let digest = "a".repeat(64);
    let exports = export_cross_unit_homes(
        &home,
        &environment(),
        &BTreeSet::from(["broker_cmp".to_owned()]),
        &digest,
    )
    .expect("home export");
    assert_eq!(exports.len(), 1);

    // No import: no contract in force.
    let without =
        analyze_prepared_translation_unit_with_cross_unit(&caller, "user", &environment(), &[])
            .expect("caller analysis");
    let summary = match &without {
        UnrestrictedLegacyResult::Unknown {
            partial: Some(summary),
            ..
        }
        | UnrestrictedLegacyResult::Unsafe(summary) => summary,
        other => panic!("expected a refused caller without the import: {other:?}"),
    };
    assert!(
        summary.functions["user"]
            .diagnostics
            .iter()
            .any(|diagnostic| matches!(
                &diagnostic.kind,
                LegacyDiagnosticKind::Unsupported { description }
                    if description.contains("callback-uncontracted")
            )),
        "{summary:?}"
    );

    // Under the import: in force, Safe, both bodies replayed.
    let mut imported = environment();
    imported.set_inferred_imports(BTreeMap::from([(
        "broker_cmp".to_owned(),
        exports[0].contract.clone(),
    )]));
    imported.set_cross_unit_import_identities(BTreeMap::from([(
        "broker_cmp".to_owned(),
        exports[0].home.import_identity(),
    )]));
    imported.set_link_resolution_sha256(Some(digest));
    let with = analyze_prepared_translation_unit_with_cross_unit(
        &caller,
        "user",
        &imported,
        &[exports[0].home.clone()],
    )
    .expect("two-sided analysis");
    let UnrestrictedLegacyResult::Safe(certificate) = with else {
        panic!("expected a conditional Safe caller under the import: {with:?}");
    };
    check_legacy_safety_certificate(&certificate).expect("home and caller replay");
}
