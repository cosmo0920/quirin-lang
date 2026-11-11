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

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyInstruction, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11, check_legacy_safety_certificate,
};

fn run(source: &str) -> UnrestrictedLegacyResult {
    analyze_unrestricted_c11(
        &C11TranslationUnit {
            source,
            ..Default::default()
        },
        "subject",
        &ResourceContracts::default(),
    )
    .expect("valid C")
}

#[test]
fn multiplication_is_not_a_pointer_access() {
    let source = "int subject(void) { int x = 2; x = 3 * 4; return 0; }";
    let result = run(source);
    let UnrestrictedLegacyResult::Safe(certificate) = result else {
        panic!("typed non-access should decide: {result:?}");
    };
    assert_eq!(certificate.summary.status, AnalysisStatus::Safe);
    check_legacy_safety_certificate(&certificate).expect("independent rederivation");
}

#[test]
fn untracked_multi_pointer_declaration_is_not_an_access() {
    let source = "struct S { int x; }; int subject(void *a, void *b) { const struct S *x = a, *y = b; return 0; }";
    let result = run(source);
    let UnrestrictedLegacyResult::Safe(certificate) = result else {
        panic!("untracked pointer aliases should decide: {result:?}");
    };
    check_legacy_safety_certificate(&certificate).expect("independent rederivation");
}

#[test]
fn sizeof_vla_with_call_is_not_treated_as_unevaluated() {
    let source = "int close(int); int subject(int n, int fd) { int x = sizeof(int[(close(fd), n)]); return x; }";
    assert!(matches!(
        run(source),
        UnrestrictedLegacyResult::Unknown { .. }
    ));
}

#[test]
fn embedded_and_indexed_paths_have_untracked_scope() {
    let source = "struct I { int value; }; struct S { struct I inner; struct I *items; }; int subject(struct S *p, int i) { int x = p->inner.value + p->items[i].value; return 0; }";
    let result = run(source);
    let UnrestrictedLegacyResult::Safe(certificate) = result else {
        panic!("untracked mixed path should decide: {result:?}");
    };
    check_legacy_safety_certificate(&certificate).expect("independent rederivation");
}

#[test]
fn pointer_store_and_union_still_refuse() {
    // Card CG13-UNION-AND-BITFIELD-PLACES flips the three union rows (rule
    // 10w): a scalar read through a union member, direct, typedef-hidden or
    // through a member pointer to a union, is custody-neutral and certifies.
    // BEFORE: each stayed Unknown with the indirect-access diagnostic.
    for source in [
        "union U { int x; }; int subject(union U *u) { int x = u->x; return 0; }",
        "typedef union U { int x; int y; } U; int subject(U *u) { int x = u->x; return 0; }",
        "typedef union U { int x; int y; } U; typedef struct S { U *u; } S; int subject(S *s) { int x = s->u->x; return 0; }",
    ] {
        assert!(
            matches!(run(source), UnrestrictedLegacyResult::Safe(_)),
            "{source}"
        );
    }
    // Card CG14-AGGREGATE-VALUES-AND-INITIALIZERS flip (rule 11f): the
    // `h->value = h->value` fixture copies a custody-free record (`A` holds
    // one `int`), which is the source's typed read and the destination's
    // typed store and certifies. BEFORE: Unknown with the indirect-access
    // diagnostic. A record with a pointer member keeps the refusal.
    let result = run(
        "typedef struct { int x; } A; struct H { A value; }; int subject(struct H *h) { h->value = h->value; return 0; }",
    );
    let UnrestrictedLegacyResult::Safe(certificate) = result else {
        panic!("a custody-free record copy certifies: {result:?}");
    };
    check_legacy_safety_certificate(&certificate).expect("independent rederivation");
    // Card MEMBER-EFFECT-SUMMARY (rule 38i) re-pins this deliberately: the pointer store into a formal's
    // member names its premise; still `Unknown`.
    for (source, description) in [
        (
            "struct S { int *p; }; int subject(struct S *s, int *q) { s->p = q; return 0; }",
            "store of a custody-carrying value into `s->p`, a member of the formal `s` \
             (member-store-through-formal)",
        ),
        (
            "typedef struct { int *x; } A; struct H { A value; }; int subject(struct H *h) { h->value = h->value; return 0; }",
            "indirect memory access is outside the resource projection",
        ),
    ] {
        let result = run(source);
        let UnrestrictedLegacyResult::Unknown {
            partial: Some(summary),
            ..
        } = result
        else {
            panic!("unsupported path must stay Unknown: {result:?}");
        };
        assert!(
            summary.functions["subject"]
                .diagnostics
                .iter()
                .any(|diagnostic| {
                    matches!(&diagnostic.kind, LegacyDiagnosticKind::Unsupported { description: found }
                if found == description)
                })
        );
    }
}

#[test]
fn extended_typed_member_chain_uses_untracked_scope() {
    let source = "struct S { struct S *next; int value; }; int subject(struct S *p) { int x = p->next->next->next->next->value; return 0; }";
    let result = run(source);
    let UnrestrictedLegacyResult::Safe(certificate) = result else {
        panic!("typed path should decide the chain: {result:?}");
    };
    check_legacy_safety_certificate(&certificate).expect("independent rederivation");
}

#[test]
fn pointer_value_read_binds_only_a_borrowed_alias() {
    let source = "struct S { struct S *next; int *child; }; int subject(struct S *s) { int *q = s->next->next->next->next->child; return 0; }";
    let result = run(source);
    let UnrestrictedLegacyResult::Safe(certificate) = result else {
        panic!("pointer read should bind a borrowed alias: {result:?}");
    };
    assert!(
        certificate.summary.functions["subject"]
            .cfg
            .nodes
            .iter()
            .any(|node| {
                matches!(&node.instruction, LegacyInstruction::IndirectScalarAccess {
            borrowed_destination: true, destination: Some(name), ..
        } if name == "q")
            })
    );
    check_legacy_safety_certificate(&certificate).expect("independent rederivation");
}
