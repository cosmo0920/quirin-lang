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

//! Card UNION-OPAQUE-RECORD-WRITES-BY-TYPE (gateway rule 12n, amended in
//! place): an opaque write of a record (a whole-object assignment from a call
//! or another type, a `memcpy`-family call into it) is keyed by its target
//! expression, as an assignment store is, not by the record's type alone.
//!
//! The read is `op_s.op_cb`, stored from a formal whose actual reads
//! `w->u.late.cb` (union bytes 32..40); `pos_s` is placed in the union at 8
//! (`early.pos`) and at 32 (`lp.pos`, and inside `boxed.box`). Before the
//! card every opaque write of a `pos_s`, wherever it landed, opened the read
//! because one placement meets it. The brief's four cases, the fail-closed
//! edges, and the declared spelling's arm path.

use std::collections::BTreeSet;

use qcore_c_gateway::callback_member_closure::{MemberFacts, Tables, closure, read_unit};
use qcore_c_gateway::legacy_c::C11TranslationUnit;

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent opaque write test: {reason}");
            false
        }
    }
}

// The pinned byte offsets rest on 8-byte integers: `__INT64_TYPE__` is `long int`
// on LP64 and `long long` on LLP64 (Windows), where a plain `long` is 4 bytes.
const RECORDS: &str = "\
typedef __SIZE_TYPE__ size_t;
void *memcpy(void *, const void *, size_t);
typedef int cb_t(void *);
int fa(void *); int fb(void *);
struct pos_s { __INT64_TYPE__ off; int epoch; };
struct box_s { struct pos_s pos; };
struct other_s { struct pos_s pos; };
struct op_s {
  cb_t *op_cb;
  union {
    struct { cb_t *cb; struct pos_s pos; } early;
    struct { __INT64_TYPE__ pad[4]; cb_t *cb; } late;
    struct { __INT64_TYPE__ pad[4]; struct pos_s pos; } lp;
    struct { __INT64_TYPE__ pad[4]; struct box_s box; } boxed;
  } u;
};
struct pos_s make_pos(void);
struct other_s global_other;
void new_cb(struct op_s *o, cb_t *cb) { o->op_cb = cb; }
void reply(struct op_s *o, struct op_s *w) { new_cb(o, w->u.late.cb); }
void start(struct op_s *w) { w->u.late.cb = fa; }
";

const MEMBER: &str = "op_s.op_cb";

fn unit(body: &str) -> MemberFacts {
    let text = format!("{RECORDS}{body}");
    read_unit(
        &text,
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
    )
    .unwrap_or_else(|error| panic!("read: {error}\n{text}"))
}

fn names(list: &[&str]) -> BTreeSet<String> {
    list.iter().map(|name| (*name).to_owned()).collect()
}

/// The layout the cases rest on.
#[test]
fn the_layout_places_the_record_on_and_off_the_read() {
    if !clang_available() {
        return;
    }
    let facts = unit("");
    let layout = &facts.union_layouts["op_s.u.*"];
    assert_eq!(layout.paths["late.cb"], (32, 8));
    assert_eq!(
        layout.records["pos_s"],
        Some(BTreeSet::from([(8, 16), (32, 16)]))
    );
    assert_eq!(layout.records["box_s"], Some(BTreeSet::from([(32, 16)])));
    assert_eq!(closure(&[&facts], MEMBER), Ok(names(&["fa"])));
}

/// Case 1: a write into a separate object of a type the union holds does
/// not key into the union: a member path below a record no union holds, a
/// local, a by-value formal, a global's member, and a copy into one. With
/// the targets dropped (facts written before the card) the same writes are
/// read by type and open the read, as before.
#[test]
fn a_separate_object_write_does_not_key_into_the_union() {
    if !clang_available() {
        return;
    }
    let facts = unit(
        "void a(struct other_s *x) { x->pos = make_pos(); }\n\
         void b(void) { struct pos_s p; p = make_pos(); (void)p; }\n\
         void c(struct pos_s q) { q = make_pos(); (void)q; }\n\
         void d(void) { global_other.pos = make_pos(); }\n\
         void e(struct other_s *x, const void *s) { memcpy(&x->pos, s, sizeof x->pos); }\n",
    );
    assert!(facts.opened.contains_key("pos_s"), "{:?}", facts.opened);
    assert_eq!(closure(&[&facts], MEMBER), Ok(names(&["fa"])));
    let mut untargeted = facts.clone();
    untargeted.opened_targets.clear();
    let open = closure(&[&untargeted], MEMBER).expect_err("read by type");
    assert!(
        open.iter().any(|why| why.contains("struct pos_s")),
        "{open:?}"
    );
}

/// Case 2: a write into a union arm whose bytes do not meet the read does
/// not open it.
#[test]
fn a_union_arm_write_that_does_not_overlap_does_not_open() {
    if !clang_available() {
        return;
    }
    let facts = unit("void f(struct op_s *w) { w->u.early.pos = make_pos(); }\n");
    assert!(facts.stores.contains_key("op_s.u.*@early.pos"));
    assert_eq!(facts.union_layouts["op_s.u.*"].paths["early.pos"], (8, 16));
    assert_eq!(closure(&[&facts], MEMBER), Ok(names(&["fa"])));
}

/// Case 3: a write into a union arm whose bytes meet the read opens it, and
/// so does a write below a record the union holds where it meets the read.
#[test]
fn an_overlapping_write_opens() {
    if !clang_available() {
        return;
    }
    let arm = unit("void g(struct op_s *w) { w->u.lp.pos = make_pos(); }\n");
    let open = closure(&[&arm], MEMBER).expect_err("lp.pos meets late.cb");
    assert!(
        open.iter().any(|why| why.contains("into the union")),
        "{open:?}"
    );
    let held = unit("void h(struct box_s *b) { b->pos = make_pos(); }\n");
    let open = closure(&[&held], MEMBER).expect_err("box_s lies on late.cb");
    assert!(
        open.iter().any(|why| why.contains("struct pos_s")),
        "{open:?}"
    );
}

/// Case 4: an unknown range opens: a union place the layout does not give,
/// an untargeted write through a pointer, and a copy through a pointer.
#[test]
fn an_unknown_range_opens() {
    if !clang_available() {
        return;
    }
    let mut facts = unit("void f(struct op_s *w) { w->u.early.pos = make_pos(); }\n");
    facts
        .union_layouts
        .get_mut("op_s.u.*")
        .expect("layout")
        .paths
        .remove("early.pos");
    assert!(closure(&[&facts], MEMBER).is_err());
    let through = unit("void i(struct pos_s *p) { *p = make_pos(); }\n");
    assert!(closure(&[&through], MEMBER).is_err());
    let copied = unit("void j(struct pos_s *p, const void *s) { memcpy(p, s, sizeof *p); }\n");
    assert!(closure(&[&copied], MEMBER).is_err());
}

/// The declared spelling of a member inside a union keeps its arm path, so
/// the layout places it (it was the whole union): a sibling's store on
/// other bytes does not join it, one on the same bytes does.
#[test]
fn a_declared_member_in_a_union_keeps_its_arm() {
    if !clang_available() {
        return;
    }
    let facts = unit("void s(struct op_s *w) { w->u.early.cb = fb; }\n");
    assert_eq!(
        Tables::of(&[&facts]).normalise("op_s.u.late.cb"),
        Ok(names(&["op_s.u.*@late.cb"]))
    );
    assert_eq!(closure(&[&facts], "op_s.u.late.cb"), Ok(names(&["fa"])));
    assert_eq!(closure(&[&facts], "op_s.u.early.cb"), Ok(names(&["fb"])));
    let mut unplaced = facts.clone();
    unplaced.union_layouts.clear();
    assert_eq!(
        closure(&[&unplaced], "op_s.u.late.cb"),
        Ok(names(&["fa", "fb"]))
    );
}
