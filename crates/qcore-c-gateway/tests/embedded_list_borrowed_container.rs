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

//! Card CG10-EMBEDDED-LIST-BORROWED-CONTAINER (gateway rule 69): a list
//! helper handed a formal's embedded list (`&c->list`, `&c->in.on_new`,
//! `&k->conf.in.on_new`) is a borrowed container operation; the gate's
//! refusal names the container custody that decides it (rule 13w's registry,
//! rule 12t's retention) and **no verdict moves**: a releasing helper is
//! never a no-op, an added element never passes to a container whose
//! registration the body does not hold.
//!
//! The fixture contracts are `callbacks.qv`'s `c_list_*` and
//! `embedded_list.qv`'s `el_*`: trusted test assumptions, not library facts.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const PRELUDE: &str = "\
struct list { void **elems; int cnt; void (*free_cb)(void *); };
struct ics { struct list on_new; struct list config; };
struct conf { int flags; struct ics in; struct list waiters; };
struct client { struct conf conf; struct list observers; struct conf *confp; };
void c_list_add(struct list *rl, void *elem);
void c_list_clear(struct list *rl);
void c_list_destroy(struct list *rl);
int el_count(struct list *head);
int el_method_add(struct list *list, const char *name, void *func, void *opaque);
void el_take(struct list *list, void *elem);
void el_free_all(struct list *list);
void *c_alloc(__SIZE_TYPE__ n);
";

fn contracts() -> ResourceContracts {
    let file = |name: &str, text: &str| (name.to_owned(), text.to_owned());
    load_resource_contract_sources(&BTreeMap::from([
        file(
            "callbacks.qv",
            include_str!("fixtures/contracts/callbacks.qv"),
        ),
        file(
            "embedded_list.qv",
            include_str!("fixtures/contracts/embedded_list.qv"),
        ),
    ]))
    .expect("the fixture contracts load")
}

fn run(source: &str) -> (AnalysisStatus, Vec<String>) {
    let text = format!("{PRELUDE}\n{source}\n");
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
        "subject",
        &contracts(),
    )
    .unwrap_or_else(|error| panic!("fixture rejected by Clang: {error:?}\n{text}"));
    let summary = match &outcome {
        UnrestrictedLegacyResult::Safe(certificate) => Some(&certificate.summary),
        UnrestrictedLegacyResult::Unsafe(summary) => Some(summary),
        UnrestrictedLegacyResult::Unknown { partial, .. } => partial.as_ref(),
    };
    let result = summary
        .and_then(|summary| summary.functions.get("subject"))
        .unwrap_or_else(|| panic!("`subject` projected: {outcome:?}"));
    (
        result.status,
        result
            .diagnostics
            .iter()
            .map(|diagnostic| match &diagnostic.kind {
                LegacyDiagnosticKind::Unsupported { description } => description.clone(),
                other => format!("{other:?}"),
            })
            .collect(),
    )
}

const INDIRECT: &str = "indirect memory access is outside the resource projection";

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent rule 69 test: {reason}");
            false
        }
    }
}

/// `Unknown`, exactly one leaf, and that leaf is `expected`.
fn assert_named(source: &str, expected: &str) {
    let (status, descriptions) = run(source);
    assert_eq!(
        status,
        AnalysisStatus::Unknown,
        "{source}\n{descriptions:?}"
    );
    assert_eq!(descriptions, [expected.to_owned()], "{source}");
}

#[test]
fn a_releasing_helper_on_a_borrowed_container_names_the_registry() {
    if !clang_available() {
        return;
    }
    assert_named(
        "void subject(struct conf *c) { c_list_destroy(&c->waiters); }",
        "call to `c_list_destroy`: the registry holder `& c - > waiters` is not one this body \
         names (callback-registry-undecided)",
    );
    assert_named(
        "void subject(struct conf *c) { c_list_clear(&c->in.on_new); }",
        "call to `c_list_clear`: the registry holder `& c - > in . on_new` is not one this body \
         names (callback-registry-undecided)",
    );
    assert_named(
        "void subject(struct client *k) { c_list_destroy(&k->conf.in.on_new); }",
        "call to `c_list_destroy`: the registry holder `& k - > conf . in . on_new` is not one \
         this body names (callback-registry-undecided)",
    );
}

#[test]
fn an_added_element_never_passes_to_a_borrowed_container() {
    if !clang_available() {
        return;
    }
    assert_named(
        "void subject(struct conf *c, void *e) { c_list_add(&c->waiters, e); }",
        "call to `c_list_add`: the registry holder `& c - > waiters` is not one this body names \
         (callback-registry-undecided)",
    );
}

#[test]
fn a_retention_in_the_callers_container_stays_undecided() {
    if !clang_available() {
        return;
    }
    let retained = "call to `el_method_add` retains `f` in `& c - > in . on_new`, whose lifetime \
                    this body does not decide against it (retention-undecided)";
    assert_named(
        "int subject(struct conf *c, const char *n, void *f, void *o) \
         { return el_method_add(&c->in.on_new, n, f, o); }",
        retained,
    );
    assert_named(
        "int subject(struct conf *c, const char *n, void *f, void *o) \
         { int r = el_method_add(&c->in.on_new, n, f, o); return r; }",
        retained,
    );
    assert_named(
        "int subject(struct client *k, const char *n, void *f, void *o) \
         { return el_method_add(&k->conf.in.on_new, n, f, o); }",
        "call to `el_method_add` retains `f` in `& k - > conf . in . on_new`, whose lifetime \
         this body does not decide against it (retention-undecided)",
    );
}

#[test]
fn a_borrowed_only_helper_was_already_safe() {
    if !clang_available() {
        return;
    }
    for source in [
        "int subject(struct conf *c) { return el_count(&c->in.on_new); }",
        "void subject(struct conf *c) { el_count(&c->in.on_new); }",
        "void subject(struct client *k) { el_count(&k->confp->waiters); }",
    ] {
        let (status, descriptions) = run(source);
        assert_eq!(status, AnalysisStatus::Safe, "{source}\n{descriptions:?}");
    }
}

#[test]
fn outside_the_rule_the_gate_or_another_route_decides() {
    if !clang_available() {
        return;
    }
    // A local record: not a formal's container; the gate's refusal stands.
    let (status, descriptions) = run(
        "void subject(void) { struct conf c; struct conf *p = &c; c_list_destroy(&p->waiters); }",
    );
    assert_ne!(status, AnalysisStatus::Safe, "{descriptions:?}");
    assert!(
        descriptions.iter().any(|d| d == INDIRECT),
        "{descriptions:?}"
    );
    // A cast container actual: not an embedded container spelling.
    let (status, descriptions) =
        run("int subject(struct conf *c) { return el_count((struct list *)(void *)&c->waiters); }");
    assert_ne!(status, AnalysisStatus::Safe, "{descriptions:?}");
    assert_eq!(descriptions, [INDIRECT.to_owned()]);
    // A released container (a custody position): rule 38i names its premise.
    let (status, descriptions) =
        run("void subject(struct conf *c) { el_free_all(&c->in.on_new); }");
    assert_eq!(status, AnalysisStatus::Unknown);
    assert_eq!(
        descriptions,
        [
            "release of `&c->in.on_new`, an address inside the formal `c` \
          (member-address-release-through-formal)"
                .to_owned()
        ]
    );
    // An owned element handed to a consuming position: the decision changes
    // nothing for it (no `INDIRECT` leaf at the container either).
    let (status, descriptions) =
        run("void subject(struct conf *c) { void *e = c_alloc(4); el_take(&c->in.on_new, e); }");
    assert_ne!(status, AnalysisStatus::Safe, "{descriptions:?}");
    assert!(
        !descriptions.iter().any(|d| d == INDIRECT),
        "{descriptions:?}"
    );
}
