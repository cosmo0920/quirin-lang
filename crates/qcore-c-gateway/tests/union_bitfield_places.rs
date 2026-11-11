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

//! Card CG13-UNION-AND-BITFIELD-PLACES (gateway rule 10w): union-member and
//! bit-field place steps through the custody gateway (route G), with the
//! brief's refusal controls. The decision table, the storage-unit reading and
//! the core-model naming are unit-tested in `tests/unit/union_bitfield.rs`.

#[path = "support/clang_gap.rs"]
mod clang_gap;

use clang_gap::*;

const RECORDS: &str = "union U { int i; void *p; struct { int a; void *q; } s; }; \
union N { int i; long l; }; \
struct R { int k; union U u; union U *pu; unsigned bf : 3; unsigned bg : 5; int x; unsigned bh : 2; union N n; };\n";

fn run(body: &str) -> G {
    g(&format!("{RECORDS}{body}"))
}

#[test]
fn union_member_reads_and_scalar_stores_are_admitted() {
    if !clang_available() {
        return;
    }
    for body in [
        // A scalar store through a union member, on a formal and on a tracked root.
        "void subject(struct R *r) { r->u.i = 1; }",
        "void subject(void) { struct R *r = q_alloc(); r->u.i = 1; q_free(r); }",
        "void subject(struct R *r) { r->u.i += 2; }",
        // Scalar reads in a statement, a condition, an actual and a return.
        "void subject(void) { struct R *r = q_alloc(); int y = r->u.i; (void)y; q_free(r); }",
        "void subject(struct R *r) { if (r->u.s.a) cg_sink(1); }",
        "void subject(struct R *r) { cg_sink(r->u.i); }",
        "int subject(struct R *r) { return r->pu->i; }",
        // An unpunned pointer read is a target-free borrowed alias.
        "void subject(union U *u) { void *x = u->p; (void)x; }",
        "void subject(union U *u) { q_observe(u->p); }",
    ] {
        assert!(run(body).safe(), "{body}: {:?}", run(body));
    }
}

/// Refusal control: a pointer read through a union after a store through
/// another member of it (type punning, M22).
#[test]
fn a_punned_union_pointer_read_is_refused() {
    if !clang_available() {
        return;
    }
    for body in [
        "void subject(union U *u) { void *x; u->i = 1; x = u->p; (void)x; }",
        "void subject(union U *u) { u->i = 1; q_observe(u->p); }",
        "void subject(struct R *r) { void *x; r->u.s.a = 1; x = r->u.p; (void)x; }",
    ] {
        let result = run(body);
        assert!(!result.safe(), "{body}");
        assert!(result.refused(IND), "{body}: {result:?}");
    }
}

/// Refusal control: a custody pointer stored through a union member, and a
/// release of a pointer read through one (a borrow, never the owner).
#[test]
fn a_custody_pointer_through_a_union_member_is_refused() {
    if !clang_available() {
        return;
    }
    for body in [
        "void subject(union U *u) { u->p = q_alloc(); }",
        "void subject(union U *u) { void *x = q_alloc(); u->p = x; }",
        "void subject(struct R *r) { void *x = q_alloc(); r->u.s.q = x; }",
    ] {
        let result = run(body);
        assert!(!result.safe(), "{body}");
        assert!(result.refused(IND), "{body}: {result:?}");
    }
    // Composition with CG11 (rule 10v): a value CG11 classifies neutral (the
    // null constant) is admitted into an ordinary member but still refused
    // through a union member, where no contract fixes the view.
    assert!(run("struct O { void *p; }; void subject(struct O *o) { o->p = 0; }").safe());
    for body in [
        "void subject(union U *u) { u->p = 0; }",
        "void subject(struct R *r) { r->u.s.q = 0; }",
    ] {
        let result = run(body);
        assert!(!result.safe(), "{body}: {result:?}");
        assert!(result.refused(IND), "{body}: {result:?}");
    }
    let released = run("void subject(union U *u) { void *x = u->p; q_free(x); }");
    assert!(!released.safe());
    assert!(released.has("UnknownAlias { variable: \"x\" }"));
}

/// A store through a union that can hold a pointer carries W43's
/// refused-write frame (asserted on the plan in `typed_access_path`'s unit
/// tests); the frame spares a local the store cannot write, so a tracked root
/// read through keeps its identity and its release still discharges it.
#[test]
fn a_framed_union_store_keeps_the_tracked_root() {
    if !clang_available() {
        return;
    }
    for store in ["r->u.i = 1;", "r->u.s.a = 1;", "r->n.i = 1;", "r->bf = 1;"] {
        let body = format!(
            "void subject(void) {{ struct R *r = q_alloc(); void *x = q_alloc(); {store} q_free(x); q_free(r); }}"
        );
        assert!(run(&body).safe(), "{store}: {:?}", run(&body));
    }
    // After the release the root is not live: the store is not Safe.
    let after = run("void subject(void) { struct R *r = q_alloc(); q_free(r); r->u.i = 1; }");
    assert!(!after.safe(), "{after:?}");
}

#[test]
fn bitfield_reads_and_stores_are_unit_accesses() {
    if !clang_available() {
        return;
    }
    for body in [
        "void subject(struct R *r) { r->bf = 1; }",
        "void subject(void) { struct R *r = q_alloc(); r->bf = 1; q_free(r); }",
        "void subject(struct R *r) { r->bf++; }",
        "int subject(struct R *r) { return r->bf + r->bg; }",
        // Two stores in sequence (a statement boundary orders them).
        "void subject(struct R *r) { r->bf = 1; r->bg = 2; }",
    ] {
        assert!(run(body).safe(), "{body}: {:?}", run(body));
    }
}

/// Refusal control: concurrent (unsequenced) stores to two bit-fields of one
/// storage unit are refused; their bit ranges are disjoint, their memory
/// location is not. Two units were refused too while CG13 admitted no field
/// pair; card CG18-FOOTPRINT-CONNECTION (rule 11k) admits them, the core
/// model deciding the two units disjoint from Clang's layout (a deliberate
/// flip of this control).
#[test]
fn unsequenced_stores_to_two_bitfields_are_refused() {
    if !clang_available() {
        return;
    }
    for body in [
        "void subject(struct R *r) { int y; y = (r->bf = 1) + (r->bg = 2); (void)y; }",
        "void subject(struct R *r) { int y; y = (r->bf = 1) + r->bg; (void)y; }",
    ] {
        let result = run(body);
        assert!(!result.safe(), "{body}");
        assert!(result.unknown(), "{body}: {result:?}");
    }
    let two_units = "void subject(struct R *r) { int y; y = (r->bf = 1) + (r->bh = 2); (void)y; }";
    assert!(run(two_units).safe(), "{two_units}: {:?}", run(two_units));
}
