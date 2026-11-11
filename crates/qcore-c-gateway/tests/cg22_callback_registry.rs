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

//! Card CG22-CALLBACK-REGISTRY-CONTRACT (gateway rules 13u-13x): the brief's
//! positive and negative controls per callback shape, on route G, over the
//! synthetic `c_*` callees of `fixtures/contracts/callbacks.qv` (trusted test
//! assumptions: a `Safe` here is `Safe` over them and promotes nothing).

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary,
    ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const PRELUDE: &str = "\
#include <stddef.h>
struct list { void **elems; int cnt; void (*free_cb)(void *); };
struct entry { const char *name; int (*handler)(void *, unsigned long, void *); };
void *c_alloc(__SIZE_TYPE__ n); void c_free(void *p); void c_opaque(void *p);
void unknown_effect(void *p);
int c_each(void *reader, int (*cb)(void *, unsigned long, void *), void *context);
int c_dispatch(void *reader, struct entry *table, void *context);
void c_list_init(struct list *rl, int size, void (*free_cb)(void *));
struct list *c_list_new(int size, void (*free_cb)(void *));
void c_list_add(struct list *rl, void *elem);
void c_list_clear(struct list *rl);
void c_list_destroy(struct list *rl);
";

fn environment() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "callbacks.qv".to_owned(),
        include_str!("fixtures/contracts/callbacks.qv").to_owned(),
    )]))
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
            eprintln!("skipping Clang-dependent CG22-CALLBACK-REGISTRY-CONTRACT test: {reason}");
            false
        }
    }
}

#[derive(Debug)]
struct Row {
    status: AnalysisStatus,
    unsupported: Vec<String>,
    kinds: Vec<String>,
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
                    kinds: row
                        .diagnostics
                        .iter()
                        .map(|d| format!("{:?}", d.kind))
                        .collect(),
                },
            )
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Rule 13u: synchronous invocation
// ---------------------------------------------------------------------------

const INVOCATIONS: &str = "\
static int visit(void *reader, unsigned long index, void *context) { return index > 3; }
static int consume(void *reader, unsigned long index, void *context) { c_free(context); return 0; }
static int opaque_effect(void *reader, unsigned long index, void *context) { unknown_effect(context); return 0; }
int run_identity(void *reader, void *context) { return c_each(reader, visit, context); }
int run_effectful(void *reader, void *context) { return c_each(reader, consume, context); }
int run_uncontracted(void *reader, void *context) { return c_each(reader, opaque_effect, context); }
int run_formal(void *reader, int (*cb)(void *, unsigned long, void *), void *context) { return c_each(reader, cb, context); }
int run_null(void *reader, void *context) { return c_each(reader, NULL, context); }
int run_choice(void *reader, void *context, int which) { return c_each(reader, which ? visit : consume, context); }
static int reads(void *reader, unsigned long index, void *context) { return *(int *)context + (int)index; }
int run_requires(void *reader, void *context) { return c_each(reader, reads, context); }
int direct_requires(void *reader, void *context) { return reads(reader, 0, context); }
int run_requires_local(void *reader) { int local = 0; return c_each(reader, reads, &local); }
#define each_generic(r, c) _Generic (0 ? (r) : (void *) 1, const void *: (int) (c_each (r, visit, c)), default: c_each (r, visit, c))
int run_generic(void *reader, void *context) { return each_generic(reader, context); }
";

#[test]
fn an_identity_callback_reads_as_its_custody_contract() {
    if !clang_available() {
        return;
    }
    let rows = rows(INVOCATIONS);
    assert!(rows["visit"].safe(), "{:?}", rows["visit"]);
    assert!(rows["run_identity"].safe(), "{:?}", rows["run_identity"]);
    // A callback that reads the caller's context (`resource_owned` in its
    // summary) puts that requirement on the call: the caller is decided as a
    // direct call of the callback with the same actual is.
    assert_eq!(
        rows["run_requires"].status, rows["direct_requires"].status,
        "{:?} {:?}",
        rows["run_requires"], rows["direct_requires"]
    );
    assert_eq!(
        rows["run_requires"].unsupported.is_empty(),
        rows["direct_requires"].unsupported.is_empty()
    );
    // glibc's `__glibc_const_generic` shape: two Clang calls on one line, one
    // per association, with one resolution.
    assert!(rows["run_generic"].safe(), "{:?}", rows["run_generic"]);
}

#[test]
fn no_callback_is_assumed_benign() {
    if !clang_available() {
        return;
    }
    let rows = rows(INVOCATIONS);
    // A callback that releases the caller's context is not loop-safe.
    assert!(
        rows["run_effectful"].refused_with("callback-effectful"),
        "{:?}",
        rows["run_effectful"]
    );
    // A callback with no contract in force.
    assert!(
        rows["run_uncontracted"].refused_with("callback-uncontracted"),
        "{:?}",
        rows["run_uncontracted"]
    );
    // A formal, a null pointer: open sets.
    for name in ["run_formal", "run_null"] {
        assert!(
            rows[name].refused_with("callback-undecided"),
            "{name}: {:?}",
            rows[name]
        );
    }
    // A choice of an identity and an effectful candidate: the join refuses.
    assert!(
        rows["run_choice"].refused_with("callback-effectful"),
        "{:?}",
        rows["run_choice"]
    );
}

// ---------------------------------------------------------------------------
// Rule 13v: table dispatch
// ---------------------------------------------------------------------------

const TABLES: &str = "\
static int on_a(void *reader, unsigned long index, void *context) { return 0; }
static int on_b(void *reader, unsigned long index, void *context) { return 1; }
static int on_bad(void *reader, unsigned long index, void *context) { c_free(context); return 0; }
int table_closed(void *reader, void *context) {
    struct entry table[] = { {\"a\", on_a}, {\"b\", on_b}, {NULL, NULL} };
    return c_dispatch(reader, table, context);
}
int table_effectful(void *reader, void *context) {
    struct entry table[] = { {\"a\", on_a}, {\"bad\", on_bad}, {NULL, NULL} };
    return c_dispatch(reader, table, context);
}
int table_written(void *reader, void *context, int (*h)(void *, unsigned long, void *)) {
    struct entry table[] = { {\"a\", on_a}, {NULL, NULL} };
    table[0].handler = h;
    return c_dispatch(reader, table, context);
}
int table_formal(void *reader, struct entry *table, void *context) { return c_dispatch(reader, table, context); }
";

#[test]
fn a_closed_table_of_identity_handlers_is_safe() {
    if !clang_available() {
        return;
    }
    let rows = rows(TABLES);
    assert!(rows["table_closed"].safe(), "{:?}", rows["table_closed"]);
}

#[test]
fn an_open_or_effectful_table_refuses() {
    if !clang_available() {
        return;
    }
    let rows = rows(TABLES);
    assert!(
        rows["table_effectful"].refused_with("callback-effectful"),
        "{:?}",
        rows["table_effectful"]
    );
    for name in ["table_written", "table_formal"] {
        assert!(
            rows[name].refused_with("callback-undecided"),
            "{name}: {:?}",
            rows[name]
        );
    }
}

// ---------------------------------------------------------------------------
// Rules 13w, 13x: registries and element custody
// ---------------------------------------------------------------------------

const REGISTRIES: &str = "\
void frame_none(void) { struct list l; c_list_init(&l, 0, NULL); c_list_add(&l, \"x\"); c_list_destroy(&l); }
void frame_releasing(void) { struct list l; char *p = c_alloc(8); c_list_init(&l, 0, c_free); if (!p) { c_list_destroy(&l); return; } c_list_add(&l, p); c_list_destroy(&l); }
void site_ok(void) { struct list *l = c_list_new(0, c_free); char *p = c_alloc(8); if (!p) { c_list_destroy(l); return; } c_list_add(l, p); c_list_destroy(l); }
void site_cleared(void) { struct list *l = c_list_new(0, c_free); char *p = c_alloc(8); if (!p) { c_list_destroy(l); return; } c_list_add(l, p); c_list_clear(l); c_list_destroy(l); }
void site_kept_element(void) { struct list *l = c_list_new(0, c_free); char *p = c_alloc(8); if (!p) { c_list_destroy(l); return; } c_list_add(l, p); c_list_destroy(l); c_free(p); }
void site_plain_release(void) { struct list *l = c_list_new(0, c_free); char *p = c_alloc(8); if (!p) { c_list_destroy(l); return; } c_list_add(l, p); c_free(l); }
void formal_holder(struct list *l) { c_list_destroy(l); }
void formal_add(struct list *l, void *p) { c_list_add(l, p); }
void cast_callback(void) { struct list l; c_list_init(&l, 0, (void (*)(void *))c_opaque); c_list_destroy(&l); }
void unregistered_twice(void) { struct list l; c_list_init(&l, 0, NULL); c_list_destroy(&l); c_list_destroy(&l); }
void one_path(int c) { struct list l; if (c) c_list_init(&l, 0, NULL); c_list_destroy(&l); }
";

#[test]
fn a_registry_in_force_decides_its_invocations_and_elements() {
    if !clang_available() {
        return;
    }
    let rows = rows(REGISTRIES);
    // No callback: a literal element is retained by a frame holder.
    assert!(rows["frame_none"].safe(), "{:?}", rows["frame_none"]);
    // A constructed registry takes the element's custody and releases it,
    // then itself.
    assert!(rows["site_ok"].safe(), "{:?}", rows["site_ok"]);
    assert!(rows["site_cleared"].safe(), "{:?}", rows["site_cleared"]);
}

#[test]
fn a_registry_not_in_force_or_not_decided_refuses() {
    if !clang_available() {
        return;
    }
    let rows = rows(REGISTRIES);
    // A frame registry has no identity to own the element.
    assert!(
        rows["frame_releasing"].refused_with("callback-element-custody-undecided"),
        "{:?}",
        rows["frame_releasing"]
    );
    // A caller's holder: no registration is in force in this frame.
    for name in ["formal_holder", "formal_add"] {
        assert!(
            rows[name].refused_with("callback-registry-undecided"),
            "{name}: {:?}",
            rows[name]
        );
    }
    // A cast callback: the cast actual is refused where it stands (the
    // access gate reads a cast actual first), so no registration is in force.
    assert!(
        rows["cast_callback"].refused_with("callback-undecided")
            || rows["cast_callback"].refused_with("callback-registry-undecided"),
        "{:?}",
        rows["cast_callback"]
    );
    // Used after its unregistration, or registered on one path only.
    for name in ["unregistered_twice", "one_path"] {
        assert!(
            rows[name].refused_with("callback-registry-undecided"),
            "{name}: {:?}",
            rows[name]
        );
    }
    // Released by a plain release while its registry owns an element.
    assert!(
        rows["site_plain_release"].refused_with("callback-element-custody-undecided"),
        "{:?}",
        rows["site_plain_release"]
    );
}

#[test]
fn an_element_the_registry_released_is_not_released_again() {
    if !clang_available() {
        return;
    }
    let rows = rows(REGISTRIES);
    let row = &rows["site_kept_element"];
    assert_eq!(row.status, AnalysisStatus::Unsafe, "{row:?}");
    assert!(
        row.kinds
            .iter()
            .any(|kind| kind.starts_with("DoubleRelease")),
        "{row:?}"
    );
}
