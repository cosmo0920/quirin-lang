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

//! Card W44-INDIRECT-CHAIN-ACCESS — typed record-pointer member chains in
//! statements and conditions.
//!
//! A statement or controlling expression the typed scalar route admits once a
//! chain `R->L1->…->F`, `(*R).L1->…->F` or `S.L1->…->F` (every link exactly
//! `struct POINTEE *`, a scalar final member, at most four selections) is an
//! access is custody-neutral. What it keeps is the obligation of every pointer
//! it reads through: the root is a never-bound name (untracked) or exactly one
//! known live identity (tracked); a link of an untracked owner is untracked; a
//! link of a tracked owner must be the member place an admitted store bound to
//! one live child, or refuses with `UnknownAlias`.
//!
//! Every test asserts the **complete** diagnostic vector of the entry with its
//! status. The fixtures use a generic `c_*` surface: `Safe` here is `Safe`
//! over synthetic trusted test assumptions and promotes nothing.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11, check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const CONTRACTS: &str = "\
module contract;\n\
extern fn c_make(n: i32) -> ptr<mut, u8>\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(n);\n\
           ensures normal resource_return_owned(nullable); };\n\
extern fn c_drop(b: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_released(b, nonnull);\n\
           ensures normal resource_return_none(); };\n\
extern fn c_take(n: i32) -> ptr<mut, u8>\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(n);\n\
           ensures normal resource_return_owned(nullable); };\n\
extern fn c_release(i: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_released(i, nonnull);\n\
           ensures normal resource_return_none(); };";

const DECLARATIONS: &str = "\
struct c_leaf { int weight; unsigned long total; char *name; void *any; }; \
struct c_item { int count; struct c_leaf *leaf; struct c_leaf inline_leaf; }; \
struct c_box { int flag; struct c_item *item; struct c_box *next; }; \
union c_union { int x; }; \
struct c_odd { union c_union *u; }; \
struct c_box *c_make(int n); \
void c_drop(struct c_box *b); \
struct c_item *c_take(int n); \
void c_release(struct c_item *i);";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "indirect_chain_access.qcore".to_owned(),
        CONTRACTS.to_owned(),
    )]))
    .expect("the synthetic contracts load")
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent indirect-chain-access test: {reason}");
            false
        }
    }
}

fn analyze(body: &str) -> UnrestrictedLegacyResult {
    let source = format!("{DECLARATIONS}\n{body}");
    analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        "w",
        &contracts(),
    )
    .unwrap_or_else(|error| panic!("fixture rejected by Clang: {error:?}\n{source}"))
}

fn program(result: &UnrestrictedLegacyResult) -> &LegacyProgramSummary {
    match result {
        UnrestrictedLegacyResult::Safe(certificate) => &certificate.summary,
        UnrestrictedLegacyResult::Unsafe(summary) => summary,
        UnrestrictedLegacyResult::Unknown {
            partial: Some(summary),
            ..
        } => summary,
        other => panic!("no projection at all: {other:?}"),
    }
}

/// The status and the complete, sorted, de-duplicated diagnostic vector of `w`.
fn outcome(body: &str) -> (&'static str, Vec<String>) {
    let result = analyze(body);
    let status = match &result {
        UnrestrictedLegacyResult::Safe(certificate) => {
            check_legacy_safety_certificate(certificate)
                .expect("a Safe result carries a certificate that rechecks");
            "Safe"
        }
        UnrestrictedLegacyResult::Unsafe(_) => "Unsafe",
        UnrestrictedLegacyResult::Unknown { .. } => "Unknown",
    };
    let mut keys = program(&result)
        .functions
        .get("w")
        .expect("missing function `w`")
        .diagnostics
        .iter()
        .map(|diagnostic| match &diagnostic.kind {
            LegacyDiagnosticKind::UnknownAlias { variable } => format!("unknown_alias:{variable}"),
            LegacyDiagnosticKind::Leak { .. } => "leak".to_owned(),
            LegacyDiagnosticKind::DoubleRelease { .. } => "double_release".to_owned(),
            LegacyDiagnosticKind::UseAfterRelease { .. } => "use_after_release".to_owned(),
            LegacyDiagnosticKind::Unsupported { description } => {
                format!("unsupported:{description}")
            }
            other => format!("{other:?}"),
        })
        .collect::<Vec<_>>();
    keys.sort();
    keys.dedup();
    (status, keys)
}

fn safe() -> (&'static str, Vec<String>) {
    ("Safe", Vec::new())
}

#[test]
fn a_chain_store_through_a_formal_certifies() {
    if !clang_available() {
        return;
    }
    for body in [
        "void w(struct c_box *b) { b->item->count = 1; }",
        "void w(struct c_box *b) { b->item->leaf->weight = 2; }",
        "void w(struct c_box *b, int n) { b->item->leaf->total += n; }",
        "void w(struct c_box *b) { b->item->count++; b->item->leaf->weight--; }",
        "void w(struct c_box *b) { b->next->next->next->flag = 0; }",
    ] {
        assert_eq!(outcome(body), safe(), "{body}");
    }
}

/// C02-P premise 4's syntactic over-approximation reads `-- b -> …` as a
/// decrement of `b`, so the root is not never-bound; unbound in the flow, it
/// refuses. Refusing too many roots only narrows the rule.
#[test]
fn a_prefix_decrement_marks_its_root_as_bound_and_refuses() {
    if !clang_available() {
        return;
    }
    assert_eq!(
        outcome("void w(struct c_box *b) { --b->item->leaf->weight; }"),
        ("Unknown", vec!["unknown_alias:b".to_owned()])
    );
    // A root written `( * b ) .` names the formal's slot `*b`, which the
    // protected-formal reading counts as a use of the formal's identity, so
    // an unbound formal is never never-bound there: the same answer, for a
    // store and for a read.
    for body in [
        "void w(struct c_box *b) { (*b).item->count = 3; }",
        "int w(struct c_box *b) { int y; y = (*b).item->count + 1; return y; }",
    ] {
        assert_eq!(
            outcome(body),
            ("Unknown", vec!["unknown_alias:b".to_owned()]),
            "{body}"
        );
    }
}

#[test]
fn a_chain_read_beside_other_operands_certifies() {
    if !clang_available() {
        return;
    }
    for body in [
        "int w(struct c_box *b) { int y; y = b->item->count + b->flag; return y; }",
        "int w(struct c_box *b, int *p) { int y; y = b->item->leaf->weight * *p; return y; }",
        "void w(struct c_box *b, struct c_box *c) { b->item->count = c->item->count; }",
    ] {
        assert_eq!(outcome(body), safe(), "{body}");
    }
}

#[test]
fn a_chain_in_a_condition_certifies() {
    if !clang_available() {
        return;
    }
    for body in [
        "int w(struct c_box *b, int *p) { if (b->item->count > *p) { return 1; } return 0; }",
        "int w(struct c_box *b, int *p) { int n = 0; while (*p && b->item->leaf->weight) { n = 1; *p = 0; } return n; }",
    ] {
        assert_eq!(outcome(body), safe(), "{body}");
    }
}

#[test]
fn a_record_object_root_certifies_and_an_addressed_one_refuses() {
    if !clang_available() {
        return;
    }
    assert_eq!(
        outcome("void w(struct c_box s) { s.item->count = 1; }"),
        safe()
    );
    // Taking the object's address makes it a name the body may bind
    // (premise 4), so its chain refuses.
    let (status, keys) =
        outcome("void w(struct c_box s, struct c_box **o) { *o = &s; s.item->count = 1; }");
    assert_eq!(status, "Unknown");
    assert!(keys.contains(&"unknown_alias:s".to_owned()), "{keys:?}");
}

#[test]
fn typed_extended_paths_preserve_unlisted_refusals() {
    if !clang_available() {
        return;
    }
    for (body, expected) in [
        // A pointer-valued final member: a member-link question.
        (
            "void w(struct c_box *b, char *s) { b->item->leaf->name = s; }",
            "Unknown",
        ),
        // A scalar nested aggregate path under a statically untracked root.
        (
            "void w(struct c_box *b) { b->item->inline_leaf.weight = 1; }",
            "Safe",
        ),
        // A `void *` link. CG12 flip (rule 11j): under a never-bound root the
        // loaded `void *` is outside the tracked graph and its view is a view
        // step. BEFORE: Unknown.
        (
            "void w(struct c_box *b) { ((struct c_leaf *)b->item->leaf->any)->weight = 1; }",
            "Safe",
        ),
        // A union pointee: card CG13 (rule 10w) reads the scalar store
        // through the union step. BEFORE: Unknown.
        ("void w(struct c_odd *o) { o->u->x = 1; }", "Safe"),
        // The new typed path is not limited to four selections.
        (
            "void w(struct c_box *b) { b->next->next->next->next->flag = 0; }",
            "Safe",
        ),
    ] {
        let (status, keys) = outcome(body);
        assert_eq!(status, expected, "{body}: {keys:?}");
        if expected == "Safe" {
            assert!(keys.is_empty(), "{body}: {keys:?}");
        } else {
            assert!(
                keys.iter().all(|key| key.starts_with("unsupported:")),
                "{body}: {keys:?}"
            );
        }
    }
}

/// Deliberate pin update, card CG10-COMMON-TYPED-PLACE (rule 10t). Before it
/// a mixed path under a tracked root refused ("no place proof"). The common
/// typed place now reads `b->item->inline_leaf.weight` as the chain
/// `b -> item` (a direct member link) followed by an embedded selection, with
/// the chain's own obligations: `b` one live identity and `b->item` the live
/// child the admitted store linked. Both hold, so the store certifies; the
/// same access after the child's release refuses.
#[test]
fn a_mixed_path_under_a_tracked_root_carries_the_link_evidence() {
    if !clang_available() {
        return;
    }
    let body = "void w(int n) { \
        struct c_item *it; struct c_box *b; \
        it = c_take(n); if (!it) { return; } \
        b = c_make(n); if (!b) { c_release(it); return; } \
        b->item = it; b->item->inline_leaf.weight = 1; \
        c_release(it); c_drop(b); }";
    assert_eq!(outcome(body), safe());
    let released = "void w(int n) { \
        struct c_item *it; struct c_box *b; \
        it = c_take(n); if (!it) { return; } \
        b = c_make(n); if (!b) { c_release(it); return; } \
        b->item = it; c_release(it); b->item->inline_leaf.weight = 1; \
        c_drop(b); }";
    let (status, keys) = outcome(released);
    assert_ne!(status, "Safe", "{keys:?}");
    assert!(!keys.is_empty(), "a released child has no place proof");
    let unlinked = "void w(int n) { struct c_box *b; \
        b = c_make(n); if (!b) { return; } \
        b->item->inline_leaf.weight = 1; c_drop(b); }";
    let (status, keys) = outcome(unlinked);
    assert_eq!(status, "Unknown", "{keys:?}");
    assert!(
        keys.contains(&"unknown_alias:b->item".to_owned()),
        "{keys:?}"
    );
}

#[test]
fn a_chain_through_a_stored_live_child_certifies() {
    if !clang_available() {
        return;
    }
    let body = "void w(int n) { \
        struct c_item *it; struct c_box *b; \
        it = c_take(n); if (!it) { return; } \
        b = c_make(n); if (!b) { c_release(it); return; } \
        b->item = it; \
        b->item->count = n + 1; \
        if (b->item->count > 2) { b->flag = 1; } \
        c_release(it); c_drop(b); }";
    assert_eq!(outcome(body), safe());
}

#[test]
fn a_chain_through_a_released_child_refuses() {
    if !clang_available() {
        return;
    }
    let body = "void w(int n) { \
        struct c_item *it; struct c_box *b; \
        it = c_take(n); if (!it) { return; } \
        b = c_make(n); if (!b) { c_release(it); return; } \
        b->item = it; \
        c_release(it); \
        b->item->count = 1; \
        c_drop(b); }";
    assert_eq!(
        outcome(body),
        ("Unknown", vec!["unknown_alias:b->item".to_owned()])
    );
}

#[test]
fn a_chain_through_an_unstored_member_of_a_tracked_root_refuses() {
    if !clang_available() {
        return;
    }
    let body = "void w(int n) { \
        struct c_box *b; \
        b = c_make(n); if (!b) { return; } \
        b->item->count = 1; \
        c_drop(b); }";
    assert_eq!(
        outcome(body),
        ("Unknown", vec!["unknown_alias:b->item".to_owned()])
    );
}

#[test]
fn a_tracked_owner_past_the_first_link_refuses() {
    if !clang_available() {
        return;
    }
    let body = "void w(int n) { \
        struct c_item *it; struct c_box *b; \
        it = c_take(n); if (!it) { return; } \
        b = c_make(n); if (!b) { c_release(it); return; } \
        b->item = it; \
        b->item->leaf->weight = 1; \
        c_release(it); c_drop(b); }";
    assert_eq!(
        outcome(body),
        ("Unknown", vec!["unknown_alias:b->item->leaf".to_owned()])
    );
}

#[test]
fn a_released_root_refuses_and_is_accused() {
    if !clang_available() {
        return;
    }
    let body = "void w(int n) { \
        struct c_box *b; \
        b = c_make(n); if (!b) { return; } \
        c_drop(b); \
        b->item->count = 1; }";
    assert_eq!(
        outcome(body),
        (
            "Unsafe",
            vec!["unknown_alias:b".to_owned(), "use_after_release".to_owned()]
        )
    );
}

#[test]
fn an_unguarded_nullable_root_refuses() {
    if !clang_available() {
        return;
    }
    let body = "void w(int n) { \
        struct c_box *b; \
        b = c_make(n); \
        b->item->count = 1; \
        if (b) { c_drop(b); } }";
    assert_eq!(
        outcome(body),
        ("Unknown", vec!["unknown_alias:b".to_owned()])
    );
}

#[test]
fn a_leak_beside_an_admitted_chain_is_still_reported() {
    if !clang_available() {
        return;
    }
    let body = "void w(struct c_box *q, int n) { \
        struct c_box *b; \
        b = c_make(n); if (!b) { return; } \
        q->item->count = n; }";
    assert_eq!(outcome(body), ("Unsafe", vec!["leak".to_owned()]));
}

/// Composition with card W43-DESCRIPTOR-RESOURCE-TRACKING: a value that may
/// carry a tracked descriptor is custody, never data. The chain route adds no
/// instruction variant; its store is W43's `IndirectScalarAccess`, which
/// `descriptor_custody::admits_descriptor_read` refuses by name, exactly as for
/// a single selection. The same store of an ordinary integer is admitted.
#[test]
fn a_tracked_descriptor_stored_through_a_chain_is_refused_by_name() {
    if !clang_available() {
        return;
    }
    let contracts = load_resource_contract_sources(&BTreeMap::from([(
        "descriptor_generic.qcore".to_owned(),
        include_str!("fixtures/contracts/descriptor_generic.qv").to_owned(),
    )]))
    .expect("the descriptor fixture loads");
    let declarations = "int k_open(const char *path); int k_close(int fd); \
                        struct d_conn { int fd; }; struct d_pool { struct d_conn *conn; };";
    let run = |body: &str| {
        let source = format!("{declarations}\n{body}");
        let result = analyze_unrestricted_c11(
            &C11TranslationUnit {
                source: &source,
                ..Default::default()
            },
            "w",
            &contracts,
        )
        .unwrap_or_else(|error| panic!("fixture rejected by Clang: {error:?}\n{source}"));
        if let UnrestrictedLegacyResult::Safe(certificate) = &result {
            check_legacy_safety_certificate(certificate).expect("the certificate rechecks");
        }
        let status = match &result {
            UnrestrictedLegacyResult::Safe(_) => "Safe",
            UnrestrictedLegacyResult::Unsafe(_) => "Unsafe",
            UnrestrictedLegacyResult::Unknown { .. } => "Unknown",
        };
        let unsupported = program(&result).functions["w"]
            .diagnostics
            .iter()
            .filter_map(|diagnostic| match &diagnostic.kind {
                LegacyDiagnosticKind::Unsupported { description } => Some(description.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        (status, unsupported)
    };
    let (status, unsupported) = run(
        "void w(struct d_pool *p) { int fd = k_open(\"p\"); if (fd < 0) return; \
         p->conn->fd = fd; k_close(fd); }",
    );
    assert_eq!(status, "Unknown");
    assert!(
        unsupported.contains(
            &"tracked descriptor `fd` read by a custody-neutral scalar route is outside the \
              resource projection"
                .to_owned()
        ),
        "{unsupported:?}"
    );
    assert_eq!(
        run("void w(struct d_pool *p, int n) { p->conn->fd = n; }"),
        ("Safe", Vec::new())
    );
}
