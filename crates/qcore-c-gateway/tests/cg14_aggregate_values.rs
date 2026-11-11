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

//! Card CG14-AGGREGATE-VALUES-AND-INITIALIZERS (gateway rule 11f): record
//! copies, braced initializers, compound literals and string initializers on
//! route G, with the refusal controls the card names. Conventions as in
//! `clang_gap_phase2.rs`: the fixture contracts are trusted test
//! assumptions, and a `Safe` here promotes nothing.

#[path = "support/clang_gap.rs"]
mod clang_gap;

use clang_gap::*;

const S: &str = "struct S { int a; void *p; struct I { void *p; int x; } in; };\n\
struct P { int x; int y; struct Q { long z; char c[4]; } q; };\n\
union N { int i; double d; };\n\
union W { int i; void *p; };\n\
struct T { union N n; unsigned bf : 3; enum E { E0, E1 } e; };\n";

fn gs(source: &str) -> G {
    g(&format!("{S}{source}"))
}

// ---------------------------------------------------------------------------
// M08: copies
// ---------------------------------------------------------------------------

#[test]
fn a_custody_free_copy_is_its_typed_read_and_store() {
    if !clang_available() {
        return;
    }
    for source in [
        "void subject(struct P *a, struct P *b) { *a = *b; }",
        "void subject(struct P *a, struct P *b) { a->q = b->q; }",
        "int subject(struct P *b) { struct P l = *b; return 0; }",
        // A custody-free union, a bit-field and an enumeration member.
        "void subject(struct T *a, struct T *b) { *a = *b; a->n = b->n; }",
        "typedef struct { int x; } A; struct H { A value; }; void subject(struct H *h) { h->value = h->value; }",
    ] {
        let run = gs(source);
        assert!(run.safe(), "{source}: {run:?}");
    }
}

#[test]
fn a_copy_carries_the_places_obligations() {
    if !clang_available() {
        return;
    }
    // A copy through a released pointer is a use after release (the place's
    // own per-hop obligation), never Safe.
    let run = gs("void subject(void) { struct P *a = q_alloc(); q_free(a); struct P l = *a; }");
    assert!(!run.safe(), "{run:?}");
    // A possibly-null root is not read until a test refines it.
    let run = gs(
        "void subject(struct P *b) { struct P *a = q_maybe_alloc(); *a = *b; q_nullable_free(a); }",
    );
    assert!(!run.safe(), "{run:?}");
    // A live tracked root: the store is its typed store.
    assert!(
        gs("void subject(struct P *b) { struct P *a = q_alloc(); *a = *b; q_free(a); }").safe()
    );
}

#[test]
fn a_pointer_bearing_copy_is_refused() {
    if !clang_available() {
        return;
    }
    for source in [
        "void subject(struct S *a, struct S *b) { *a = *b; }",
        "void subject(struct S *s) { s->in = s->in; }",
        "void subject(union W *a, union W *b) { *a = *b; }",
        "int subject(struct S *b) { struct S l = *b; return l.a; }",
    ] {
        let run = gs(source);
        assert!(!run.safe(), "{source}: {run:?}");
        assert!(run.refused(IND), "{source}: {run:?}");
    }
}

/// The brief's first refusal control: a record holding an owned field is
/// copied, then released through both copies. The copy is refused
/// (`pointer-members`), so the double release is never certified.
#[test]
fn copying_a_record_with_an_owned_field_then_releasing_both_is_never_safe() {
    if !clang_available() {
        return;
    }
    for source in [
        "void subject(void) { struct S *s = q_alloc(); s->p = q_alloc(); struct S t = *s; q_free(s->p); q_free(t.p); q_free(s); }",
        "void subject(struct S *b) { struct S t = *b; q_free(b->p); q_free(t.p); }",
        "void subject(struct S *a, struct S *b) { *a = *b; q_free(a->p); q_free(b->p); }",
    ] {
        let run = gs(source);
        assert!(!run.safe(), "{source}: {run:?}");
        assert!(run.refused(IND), "{source}: {run:?}");
    }
}

// ---------------------------------------------------------------------------
// M09: initializers
// ---------------------------------------------------------------------------

#[test]
fn an_identity_free_initializer_is_admitted() {
    if !clang_available() {
        return;
    }
    for source in [
        "int subject(void) { struct P s = { 0 }; return s.x; }",
        "int subject(void) { struct P s = {}; return s.x; }",
        "int subject(void) { struct P s = { .q = { .z = 2 } }; return s.x; }",
        "int subject(void) { int a[] = { 1, 2 }; return a[1]; }",
        "int subject(void) { struct P a[2] = { [1] = { .x = 3 } }; return 0; }",
        "int subject(struct P *b) { struct P l = { .x = b->x, .q = b->q }; return l.x; }",
        "void subject(void) { struct S s = { .p = 0, .a = 2 }; q_observe(&s); }",
        "void subject(void) { struct S s = { .p = \"x\" }; q_observe(&s); }",
        "void subject(void) { void (*table[])(void) = { cg_noop, 0 }; }",
        "int subject(void) { const union N one = { 1 }; return one.i; }",
        // `LZ4_isLittleEndian`'s shape: an unnamed union defined inline.
        "int subject(void) { const union { unsigned u; unsigned char c[4]; } one = { 1 }; return one.c[0]; }",
    ] {
        let run = g(&format!("void cg_noop(void);\n{S}{source}"));
        assert!(run.safe(), "{source}: {run:?}");
    }
}

#[test]
fn an_initial_alias_is_refused() {
    if !clang_available() {
        return;
    }
    for source in [
        // A local's pointer, a formal's pointer, a pointer read, an address, a
        // call, a nested compound literal.
        "void subject(void) { void *q = q_alloc(); struct S s = { .p = q }; q_free(s.p); }",
        "void subject(void *q) { struct S s = { .p = q }; q_observe(&s); }",
        "void subject(struct S *b) { struct S s = { .p = b->p }; q_observe(&s); }",
        "void subject(void) { int x; struct S s = { .p = &x }; q_observe(&s); }",
        "void subject(void) { struct S s = { .p = q_alloc() }; q_observe(&s); }",
        "void subject(void) { struct S s = { .a = opaque() }; q_observe(&s); }",
        // An unnamed record that can hold a pointer, with an initial alias.
        "void subject(void *q) { struct { void *p; } s = { q }; q_observe(&s); }",
    ] {
        let run = gs(source);
        assert!(!run.safe(), "{source}: {run:?}");
        assert!(run.refused(INITIALIZER), "{source}: {run:?}");
    }
}

/// Integration of CG15-STORAGE-DURATION-AND-VOLATILE (rule 11h): the static
/// local this card left to CG15 is decided there. A block-scope static
/// declaration with a constant initializer is no event, and the static
/// object's address is a borrowed actual with no identity, so the body is
/// Safe. It was this card's refused row until rule 11h landed.
#[test]
fn a_static_local_initializer_is_decided_by_rule_11h() {
    if !clang_available() {
        return;
    }
    let source = "void subject(void) { static struct S s = { 0 }; q_observe(&s); }";
    let run = gs(source);
    assert!(run.safe(), "{source}: {run:?}");
}

// ---------------------------------------------------------------------------
// M10: compound literals
// ---------------------------------------------------------------------------

#[test]
fn a_literal_is_admitted_only_as_a_borrowed_actual() {
    if !clang_available() {
        return;
    }
    let run = gs("void subject(void) { q_observe(&(struct S){ .p = 0 }); }");
    assert!(run.safe(), "{run:?}");
    let run = gs("void subject(void) { q_observe((int[]){ 1, 2 }); }");
    assert!(run.safe(), "{run:?}");
    for source in [
        // A custody-taking formal.
        "void subject(void) { q_free(&(struct S){ .p = 0 }); }",
        // An identity-carrying initializer.
        "void subject(void) { void *q = q_alloc(); q_observe(&(struct S){ .p = q }); q_free(q); }",
        // Bound to a pointer.
        "void subject(void) { struct S *p = &(struct S){ .p = 0 }; q_observe(p); }",
        // Inside an actual.
        "void subject(void) { q_observe((char *)&(struct S){ .p = 0 } + 1); }",
    ] {
        let run = gs(source);
        assert!(!run.safe(), "{source}: {run:?}");
    }
}

/// The brief's second refusal control: a block compound literal's address
/// returned past its block is never Safe.
#[test]
fn a_returned_block_literal_address_is_never_safe() {
    if !clang_available() {
        return;
    }
    for source in [
        "struct S *subject(void) { return &(struct S){ .p = 0 }; }",
        "int *subject(void) { return (int[]){ 1, 2 }; }",
        "struct S *subject(int n) { struct S *p = 0; if (n) { p = &(struct S){ .a = 1 }; } return p; }",
    ] {
        let run = gs(source);
        assert!(!run.safe(), "{source}: {run:?}");
        assert!(run.unknown(), "{source}: {run:?}");
    }
}

// ---------------------------------------------------------------------------
// M11: string initializers
// ---------------------------------------------------------------------------

#[test]
fn a_string_initializer_is_a_declaration_with_no_bound_claimed() {
    if !clang_available() {
        return;
    }
    for source in [
        "void subject(void) { char buf[] = \"abc\"; }",
        "void subject(void) { char buf[8] = \"abc\"; q_observe(buf); }",
        "void subject(void) { unsigned char buf[3] = \"abc\"; }",
    ] {
        let run = gs(source);
        assert!(run.safe(), "{source}: {run:?}");
    }
    // A pointer to a literal is unchanged: its identity is unresolved.
    let run = gs("int subject(void) { const char *s = \"abc\"; return s[3]; }");
    assert!(run.unknown() && run.has("__qcore_literal__"), "{run:?}");
}
