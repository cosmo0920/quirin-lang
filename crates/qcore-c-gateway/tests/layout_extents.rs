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

//! Card CG17-LAYOUT-DERIVED-EXTENTS (gateway rule 11u): route G's VLA and
//! `container_of` lowerings with their refusal controls, and the published
//! layout report (`layout.alignment.v1`, flexible-array extents, VLA facts).
//! Contracts are the CG00 fixture assumptions (`support/clang_gap.rs`).

#[path = "support/clang_gap.rs"]
mod clang_gap;

use clang_gap::*;
use qcore_c_gateway::layout_extents::{
    Alignment, BoundClass, FlexExtent, FlexRefusal, LayoutReport, Offset, PropertyVerdict,
    VlaDecision, VlaUse, layout_report,
};
use qcore_c_gateway::legacy_c::C11TranslationUnit;

const S: &str = "struct S { int a; int b; unsigned bf : 3; void *p; void *v; \
struct I { void *p; int x; } in; int arr[4]; void *slots[4]; void (*cb)(void); };\n";

fn gs(source: &str) -> G {
    g(&format!("{S}{source}"))
}

fn report(source: &str, allocation: Option<u128>) -> LayoutReport {
    let text = format!("{S}{source}");
    layout_report(
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
        "subject",
        &|_| allocation,
    )
    .expect("the fixture parses")
}

// ---------------------------------------------------------------------------
// M12: variable-length arrays
// ---------------------------------------------------------------------------

#[test]
fn an_admitted_vla_declaration_carries_no_custody_event() {
    if !clang_available() {
        return;
    }
    for body in [
        "void subject(int n) { int a[n]; }",
        "void subject(int n) { int a[n]; a[0] = 1; }",
        "int subject(int n) { int a[n]; a[0] = 1; return a[0]; }",
        "void subject(void) { int m = 3; int a[m]; a[0] = 1; }",
        "void subject(int n) { unsigned long a[n * 2 + 1]; a[0] = 1; }",
        // The custody of a resource around the array is unchanged.
        "void subject(int n) { void *p = q_alloc(); int a[n]; a[0] = 1; q_free(p); }",
    ] {
        assert!(gs(body).safe(), "{body}: {:?}", gs(body));
    }
    // The leak around it is still reported.
    let leak = gs("void subject(int n) { void *p = q_alloc(); int a[n]; a[0] = 1; }");
    assert!(leak.unsafe_(), "{leak:?}");
}

#[test]
fn a_vla_whose_address_may_leave_its_block_is_refused() {
    if !clang_available() {
        return;
    }
    for body in [
        // A VLA address returned.
        "int *subject(int n) { int a[n]; return a; }",
        "void subject(int n) { int a[n]; q_observe(a); }",
        "int subject(int n) { int a[n]; int *p = &a[1]; return *p; }",
    ] {
        let run = gs(body);
        assert!(
            run.refused("local declaration may shadow a formal resource identity"),
            "{body}: {run:?}"
        );
    }
}

#[test]
fn a_vla_bound_with_an_effect_or_an_identity_element_is_refused() {
    if !clang_available() {
        return;
    }
    // A call in the bound: evaluated once, but not by this rule.
    let call = gs("int subject(int n) { int a[opaque()]; return 0; }");
    assert!(call.refused(IND), "{call:?}");
    // Elements that may hold an identity.
    let pointers = gs("void subject(int n) { void *a[n]; a[0] = 0; }");
    assert!(
        pointers.refused("local declaration may shadow a formal resource identity"),
        "{pointers:?}"
    );
}

#[test]
fn the_report_states_each_vla_with_its_numeric_obligations() {
    if !clang_available() {
        return;
    }
    let report = report(
        "int subject(int n) { int a[n]; a[0] = 1; int b[n]; int *q = b; return a[0] + (int) sizeof a; }",
        None,
    );
    let [a, b] = report.vlas.as_slice() else {
        panic!("two VLAs: {report:?}");
    };
    assert_eq!((a.name.as_str(), a.bound.as_str()), ("a", "n"));
    assert_eq!(a.bound_class, BoundClass::ReadsOnly);
    assert_eq!(
        a.uses,
        [VlaUse::Element, VlaUse::Element, VlaUse::SizeQuery]
    );
    assert_eq!(a.decision, VlaDecision::Admitted);
    assert_eq!(b.decision, VlaDecision::AddressEscapes);
    assert_eq!(
        qcore_c_gateway::layout_extents::obligations_of(a.decision)
            .iter()
            .map(|o| o.code())
            .collect::<Vec<_>>(),
        ["bound-defined", "bound-positive", "size-representable"]
    );
}

// ---------------------------------------------------------------------------
// E18: container_of
// ---------------------------------------------------------------------------

const CONTAINER: &str = "(struct S *)((char *)&x->b - __builtin_offsetof(struct S, b))";

#[test]
fn container_of_an_established_member_is_the_enclosing_object() {
    if !clang_available() {
        return;
    }
    // The round trip of a tracked identity: released once, through the result.
    let tracked = format!(
        "void subject(void) {{ struct S *x = q_alloc(); struct S *y = {CONTAINER}; q_free(y); }}"
    );
    assert!(gs(&tracked).safe(), "{:?}", gs(&tracked));
    // And a second release through the original is the double release it is.
    let twice = format!(
        "void subject(void) {{ struct S *x = q_alloc(); struct S *y = {CONTAINER}; q_free(y); q_free(x); }}"
    );
    assert!(gs(&twice).unsafe_(), "{:?}", gs(&twice));
    // Leaked through neither: the leak is reported.
    let leak =
        format!("void subject(void) {{ struct S *x = q_alloc(); struct S *y = {CONTAINER}; }}");
    assert!(gs(&leak).unsafe_(), "{:?}", gs(&leak));
    // A returned enclosing object, and the null-pointer offset form.
    assert!(
        gs(&format!(
            "struct S *subject(struct S *x) {{ return {CONTAINER}; }}"
        ))
        .safe()
    );
    assert!(
        gs(
            "void subject(void) { struct S *x = q_alloc(); struct S *y = \
         (struct S *)((char *)&x->b - (unsigned long)&((struct S *)0)->b); q_free(y); }"
        )
        .safe()
    );
}

#[test]
fn container_of_an_unrelated_pointer_is_refused() {
    if !clang_available() {
        return;
    }
    // A formal of the member's type: nothing says it points into a `struct S`.
    let unrelated = gs(
        "struct S *subject(int *m) { return (struct S *)((char *)m - __builtin_offsetof(struct S, b)); }",
    );
    assert!(unrelated.refused(RETURN), "{unrelated:?}");
    // The offset names another member than the pointer's.
    let other = gs(
        "struct S *subject(struct S *x) { return (struct S *)((char *)&x->b - __builtin_offsetof(struct S, a)); }",
    );
    assert!(other.refused(RETURN), "{other:?}");
    // The record differs.
    let record = gs(
        "struct T { int z; int b; }; struct T *subject(struct S *x) { \
         return (struct T *)((char *)&x->b - __builtin_offsetof(struct T, b)); }",
    );
    assert!(record.refused(RETURN), "{record:?}");
    // A list lookup (W45-FB-OR6's shape): the element's identity is the
    // container premise's, never member provenance.
    let lookup = gs(
        "struct L { struct L *next; }; struct E { int v; struct L link; }; \
         struct E *subject(struct L *head) { \
         return (struct E *)((char *)head->next - __builtin_offsetof(struct E, link)); }",
    );
    assert!(lookup.unknown(), "{lookup:?}");
}

// ---------------------------------------------------------------------------
// M13 and M19: the published layout report
// ---------------------------------------------------------------------------

#[test]
fn a_flexible_extent_needs_an_established_allocation_size() {
    if !clang_available() {
        return;
    }
    let source =
        "struct F { int n; int data[]; }; int subject(struct F *f, int i) { return f->data[i]; }";
    // Custody is unchanged: a scalar read through an untracked root.
    assert!(gs(source).safe());
    let unestablished = report(source, None);
    let [fact] = unestablished.flexible.as_slice() else {
        panic!("{unestablished:?}");
    };
    assert_eq!(
        (fact.record.as_str(), fact.member.as_str()),
        ("struct F", "data")
    );
    assert_eq!((fact.offset, fact.width), (Some(4), Some(4)));
    assert_eq!(
        fact.extent,
        FlexExtent::Refused(FlexRefusal::SizeNotEstablished)
    );
    // With a size a contract would establish (CG16's vocabulary), Clang's
    // offset and width give the remaining extent.
    let established = report(source, Some(22));
    assert_eq!(established.flexible[0].extent, FlexExtent::Elements(4));
    let below = report(source, Some(2));
    assert_eq!(
        below.flexible[0].extent,
        FlexExtent::Refused(FlexRefusal::BelowMember)
    );
}

#[test]
fn alignment_obligations_are_decided_from_clangs_numbers() {
    if !clang_available() {
        return;
    }
    // M19's control: a `char *` base guarantees one byte, so the obligation is
    // undecided and the property Unknown, while custody stays Safe.
    let source = "int subject(char *c) { return *(int *)(c + 1); }";
    assert!(gs(source).safe());
    let undecided = report(source, None);
    assert_eq!(undecided.alignment.len(), 1);
    assert_eq!(undecided.alignment[0].required, Some(4));
    assert_eq!(undecided.alignment[0].decision, Alignment::Undecided);
    assert_eq!(undecided.verdict, PropertyVerdict::Unknown);
    // An object's own alignment and a constant offset: definitely misaligned,
    // a provisional candidate that joins to Unknown, never Unsafe.
    let misaligned = report(
        "int subject(void) { int x = 0; return *(int *)((char *)&x + 1); }",
        None,
    );
    assert_eq!(misaligned.alignment[0].offset, Offset::Const(1));
    assert_eq!(misaligned.alignment[0].decision, Alignment::Misaligned);
    assert_eq!(misaligned.verdict, PropertyVerdict::Unknown);
    // A pointer value's own alignment (P-CG17-POINTER-ALIGNED) and a constant
    // offset of whole elements: aligned, Safe under the named premise.
    let aligned = report(
        "int subject(int *p) { return *(int *)((char *)p + 8); }",
        None,
    );
    assert_eq!(aligned.alignment[0].decision, Alignment::Aligned);
    assert_eq!(aligned.verdict, PropertyVerdict::Safe);
    assert_eq!(aligned.assumptions, ["P-CG17-POINTER-ALIGNED"]);
    assert_eq!(aligned.property, "layout.alignment.v1");
    // A scaled index over a wider element.
    let scaled = report(
        "__INT64_TYPE__ subject(__INT64_TYPE__ *p, int i) { return *(__INT64_TYPE__ *)(void *)(p + i); }",
        None,
    );
    assert_eq!(scaled.alignment.len(), 1, "{scaled:?}");
    assert_eq!(scaled.alignment[0].offset, Offset::Scaled(8));
    assert_eq!(scaled.alignment[0].decision, Alignment::Aligned);
    // No derived address: nothing to oblige.
    let none = report("int subject(int *p) { return *p; }", None);
    assert!(none.alignment.is_empty());
    assert_eq!(none.verdict, PropertyVerdict::Safe);
    assert!(none.assumptions.is_empty());
}
