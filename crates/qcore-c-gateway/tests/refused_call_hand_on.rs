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

//! Card **W45-FB-BACKLOG-CUSTODY**, integration review: a contracted call the
//! projection refuses *before its effects* (a member-address or bound-read
//! actual whose aggregate does not resolve, cards O04-B09B10 / O04-READ-BOUND)
//! hands its actuals on exactly as any other refused statement does (card
//! W44-LEAK-PIN-AUDIT's refused hand-on rule).
//!
//! The corpus shapes are Monkey's `mk_rconf_section_entry_add` and
//! `mk_vhost_fdt_worker_init`: an unchecked, possibly-null allocation linked by
//! `mk_list_add(&obj->_head, …)`. The call is refused on the nullable base with
//! an unresolved identity, its publication never applies, and before the fix
//! the object was still accused of a leak at the exit.

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, CCompilerDialect, LegacyDiagnosticKind, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const PRELUDE: &str = r#"
struct mk_list { struct mk_list *prev, *next; };
void mk_list_add(struct mk_list *entry, struct mk_list *head);
void mk_list_del(struct mk_list *entry);
void *q_maybe_alloc(void); void *q_alloc(void); void q_free(void *);
char *q_dup(const char *);
struct entry { char *key; struct mk_list _head; };
struct section { struct mk_list entries; };
"#;

const LIST: &str = "module contract;
extern fn mk_list_add(entry: ptr<mut, u8>, head: ptr<mut, u8>) -> unit
contract { requires resource_contract_v1; ensures normal resource_borrowed(entry); ensures normal resource_borrowed(head); ensures normal resource_published(entry, head); ensures normal resource_return_none(); };
extern fn mk_list_del(entry: ptr<mut, u8>) -> unit
contract { requires resource_contract_v1; ensures normal resource_borrowed(entry); ensures normal resource_unpublished(entry); ensures normal resource_return_none(); };
";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(
        &[
            (
                "q_base.qcore".to_owned(),
                include_str!("fixtures/contracts/q_base.qv").to_owned(),
            ),
            (
                "q_nullable.qcore".to_owned(),
                include_str!("fixtures/contracts/q_nullable.qv").to_owned(),
            ),
            ("list.qcore".to_owned(), LIST.to_owned()),
        ]
        .into_iter()
        .collect(),
    )
    .unwrap()
}

fn analyze(body: &str) -> (AnalysisStatus, Vec<LegacyDiagnosticKind>) {
    let source = format!("{PRELUDE}\n{body}");
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            dialect: CCompilerDialect::Gnu17,
            ..Default::default()
        },
        "subject",
        &contracts(),
    )
    .unwrap_or_else(|error| panic!("valid fixture rejected: {error:?}\n{source}"));
    let summary = match outcome {
        UnrestrictedLegacyResult::Safe(certificate) => certificate.summary,
        UnrestrictedLegacyResult::Unsafe(summary) => summary,
        UnrestrictedLegacyResult::Unknown { partial, .. } => partial.unwrap(),
    };
    let result = &summary.functions["subject"];
    (
        result.status,
        result
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.kind.clone())
            .collect(),
    )
}

fn clang_available() -> bool {
    qcore_c_gateway::clang_command().is_ok()
}

fn leaks(diagnostics: &[LegacyDiagnosticKind]) -> usize {
    diagnostics
        .iter()
        .filter(|kind| matches!(kind, LegacyDiagnosticKind::Leak { .. }))
        .count()
}

#[test]
fn a_refused_publication_of_a_nullable_object_hands_it_on() {
    if !clang_available() {
        return;
    }
    // `mk_rconf_section_entry_add`'s shape: no null check, then the link.
    let (status, diagnostics) = analyze(
        "void subject(struct section *s) { struct entry *e = q_maybe_alloc();
         mk_list_add(&e->_head, &s->entries); }",
    );
    assert_eq!(status, AnalysisStatus::Unknown, "{diagnostics:?}");
    assert_eq!(leaks(&diagnostics), 0, "{diagnostics:?}");
    assert!(diagnostics.iter().any(|kind| matches!(
        kind,
        LegacyDiagnosticKind::UnknownAlias { variable } if variable == "e"
    )));
}

#[test]
fn a_refused_publication_into_a_local_container_hands_it_on() {
    if !clang_available() {
        return;
    }
    // `mk_vhost_fdt_worker_init`'s shape: the container is a local object.
    let (status, diagnostics) = analyze(
        "void subject(struct section *s) { struct section *local = s; struct entry *e = q_maybe_alloc();
         mk_list_add(&e->_head, &local->entries); }",
    );
    assert_ne!(status, AnalysisStatus::Unsafe, "{diagnostics:?}");
    assert_eq!(leaks(&diagnostics), 0, "{diagnostics:?}");
}

#[test]
fn an_object_never_published_and_never_freed_still_leaks() {
    if !clang_available() {
        return;
    }
    let (status, diagnostics) =
        analyze("void subject(struct section *s) { struct entry *e = q_alloc(); e->key = 0; }");
    assert_eq!(status, AnalysisStatus::Unsafe, "{diagnostics:?}");
    assert_eq!(leaks(&diagnostics), 1, "{diagnostics:?}");
}

#[test]
fn a_nullable_object_the_refused_call_does_not_name_still_leaks() {
    if !clang_available() {
        return;
    }
    // The refused call hands on `e`, never `other`.
    let (status, diagnostics) = analyze(
        "void subject(struct section *s) { struct entry *other = q_alloc(); struct entry *e = q_maybe_alloc();
         mk_list_add(&e->_head, &s->entries); }",
    );
    assert_eq!(status, AnalysisStatus::Unsafe, "{diagnostics:?}");
    assert_eq!(leaks(&diagnostics), 1, "{diagnostics:?}");
}

#[test]
fn an_object_published_accepted_then_unlinked_and_leaked_still_leaks() {
    if !clang_available() {
        return;
    }
    let (status, diagnostics) = analyze(
        "void subject(struct section *s) { struct entry *e = q_alloc();
         mk_list_add(&e->_head, &s->entries); mk_list_del(&e->_head); }",
    );
    assert_eq!(status, AnalysisStatus::Unsafe, "{diagnostics:?}");
    assert_eq!(leaks(&diagnostics), 1, "{diagnostics:?}");
}

#[test]
fn an_object_published_into_an_accepted_container_is_discharged() {
    if !clang_available() {
        return;
    }
    let (status, diagnostics) = analyze(
        "void subject(struct section *s) { struct entry *e = q_alloc();
         mk_list_add(&e->_head, &s->entries); }",
    );
    assert_eq!(status, AnalysisStatus::Safe, "{diagnostics:?}");
}
