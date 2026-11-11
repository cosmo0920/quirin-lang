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

//! Card RKO-OP-CB-RUN-SET-OPEN-THROUGH-UNION (gateway rule 12n, amended in
//! place): a union's places are separated by Clang's record layout.
//!
//! `rd_kafka_op_s.rko_op_cb` is stored from a formal whose actual reads
//! `rko->rko_u.leaders.cb`; Q3 keyed every member of `rko_u` together, so a
//! store into any arm (or into any record an arm holds) joined it. A union's
//! place now keeps its arm path (`op_s.u.*@late.cb`), and the pre-pass's
//! `read_unit` reads Clang's layout of the union: two places of one union
//! alias only on overlapping bytes, a record an arm holds only where one of
//! its placements meets the place, and anything the layout does not give is
//! read as overlapping. The five cases the brief names, in the layout's
//! terms, and the fail-closed edges.

use std::collections::BTreeSet;

use qcore_c_gateway::callback_member_closure::{MemberFacts, closure, read_unit};
use qcore_c_gateway::legacy_c::C11TranslationUnit;

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent union layout test: {reason}");
            false
        }
    }
}

/// An op with a callback member and a union of arms, as `rd_kafka_op_s`:
/// `early.cb` and `admin.args.free_cb` at union offset 0 and `early.pos` at 8.
/// The late arm follows four C `long`s: offset 32 on LP64 and 16 on LLP64.
const RECORDS: &str = "\
typedef __SIZE_TYPE__ size_t;
void *memcpy(void *, const void *, size_t);
void *memset(void *, int, size_t);
void keep(void *);
typedef int cb_t(void *);
int fa(void *); int fb(void *); int fc(void *); int fd(void *);
struct pos_s { long off; int epoch; };
struct list_s { void (*free_cb)(void *); int cnt; };
struct op_s {
  int type;
  cb_t *op_cb;
  union {
    struct { cb_t *cb; struct pos_s pos; } early;
    struct { struct list_s args; } admin;
    struct { long pad[4]; cb_t *cb; void *opaque; } late;
    struct { long pad[4]; cb_t *cb2; } same;
    struct { long pad[4]; struct pos_s pos; } lp;
  } u;
};
struct other_s { struct pos_s pos; };
struct pos_s make_pos(void);
cb_t *some_callback;
void list_init(struct list_s *l, void (*free_cb)(void *)) { l->free_cb = free_cb; }
struct op_s *new_cb(int type, cb_t *cb) { static struct op_s o; o.type = type; o.op_cb = cb; return &o; }
void reply(struct op_s *w) { new_cb(2, w->u.late.cb); }
void start(struct op_s *w) { w->u.late.cb = fa; }
";

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

const MEMBER: &str = "op_s.op_cb";

/// The layout is read, and the read of `late.cb` keeps its arm path.
#[test]
fn the_union_layout_is_read_and_the_read_keeps_its_arm() {
    if !clang_available() {
        return;
    }
    let facts = unit("");
    let layout = &facts.union_layouts["op_s.u.*"];
    let late_offset = if cfg!(windows) { 16 } else { 32 };
    let pos_size = if cfg!(windows) { 8 } else { 16 };
    assert_eq!(layout.paths["late.cb"], (late_offset, 8));
    assert_eq!(layout.paths["early.cb"], (0, 8));
    assert_eq!(layout.paths["same.cb2"], (late_offset, 8));
    assert_eq!(
        layout.records["pos_s"],
        Some(BTreeSet::from([(8, pos_size), (late_offset, pos_size)]))
    );
    assert_eq!(
        layout.members["list_s.free_cb"],
        Some(BTreeSet::from([(0, 8)]))
    );
    assert!(
        facts.calls["new_cb"][&1]
            .iter()
            .any(|flow| format!("{flow:?}").contains("op_s.u.*@late.cb"))
    );
    assert_eq!(closure(&[&facts], MEMBER), Ok(names(&["fa"])));
}

/// Case 1: a sibling arm's function-pointer store on other bytes, a store
/// into a record an arm holds elsewhere (a cast free callback), and an opaque
/// write of a record placed elsewhere do not join or open the read.
#[test]
fn a_sibling_arm_on_other_bytes_does_not_join() {
    if !clang_available() {
        return;
    }
    let facts = unit(
        "void early(struct op_s *w) { w->u.early.cb = fb; }\n\
         void admin(struct op_s *w) { list_init(&w->u.admin.args, (void (*)(void *))fc); }\n",
    );
    assert_eq!(closure(&[&facts], MEMBER), Ok(names(&["fa"])));
}

/// Case 2: a sibling arm's function pointer on the same bytes joins the read
/// (type punning through the union is seen), and a `memcpy` into the union
/// opens it.
#[test]
fn a_sibling_arm_on_the_same_bytes_joins_and_a_copy_opens() {
    if !clang_available() {
        return;
    }
    let same = unit("void same(struct op_s *w) { w->u.same.cb2 = fb; }\n");
    assert_eq!(closure(&[&same], MEMBER), Ok(names(&["fa", "fb"])));
    let copied =
        unit("void copy(struct op_s *w, const void *p) { memcpy(&w->u.early, p, sizeof w->u); }\n");
    assert!(closure(&[&copied], MEMBER).is_err());
}

/// Case 3: a type-punning cast into the union opens it: an address of an arm
/// converted to another pointer type and written through, and a cast value
/// stored into the read's own arm.
#[test]
fn a_type_punning_cast_into_the_union_opens() {
    if !clang_available() {
        return;
    }
    let through = unit("void pun(struct op_s *w) { *(cb_t **)&w->u.early = fd; }\n");
    let open = closure(&[&through], MEMBER).expect_err("open");
    assert!(
        open.iter()
            .any(|why| why.contains("converted to another pointer type")),
        "{open:?}"
    );
    let value = unit("void pun(struct op_s *w, void *p) { w->u.late.cb = (cb_t *)p; }\n");
    assert!(closure(&[&value], MEMBER).is_err());
    // An address of an arm handed as `void *` to an unknown callee escapes;
    // a filler (`memset`) writes one byte value only and does not.
    let escaped = unit("void hand(struct op_s *w) { keep(&w->u.early); }\n");
    assert!(closure(&[&escaped], MEMBER).is_err());
    let filled = unit("void zero(struct op_s *w) { memset(&w->u.early, 0, sizeof w->u.early); }\n");
    assert_eq!(closure(&[&filled], MEMBER), Ok(names(&["fa"])));
}

/// Case 4: a whole-object write of a separate object of a type an arm holds
/// does not key into the union where the layout puts the record off the
/// read's bytes; card UNION-OPAQUE-RECORD-WRITES-BY-TYPE: nor where a
/// placement meets them, since the write is keyed by its target.
#[test]
fn a_separate_object_of_an_arm_type_does_not_key_into_the_union() {
    if !clang_available() {
        return;
    }
    let apart = unit("void other(struct other_s *x) { x->pos = make_pos(); }\n");
    // `pos_s` is placed at 8 (early) and 32 (lp): the second meets `late.cb`,
    // but this write lies in no union (card
    // UNION-OPAQUE-RECORD-WRITES-BY-TYPE: keyed by its target, `x->pos`
    // below a record no union holds). Read by type (facts without targets)
    // it still opens.
    assert_eq!(closure(&[&apart], MEMBER), Ok(names(&["fa"])));
    let mut by_type = apart.clone();
    by_type.opened_targets.clear();
    let open = closure(&[&by_type], MEMBER).expect_err("lp.pos meets late.cb");
    assert!(
        open.iter().any(|why| why.contains("struct pos_s")),
        "{open:?}"
    );
    // The same write read against `early.pos`'s neighbour only: a union
    // whose `pos_s` lies off the read's bytes.
    let text = "\
typedef int cb_t(void *);
int fa(void *);
struct pos_s { long off; int epoch; };
struct op_s { cb_t *op_cb; union { struct { struct pos_s pos; } reset; struct { long pad[4]; cb_t *cb; } late; } u; };
struct other_s { struct pos_s pos; };
struct pos_s make_pos(void);
void new_cb(struct op_s *o, cb_t *cb) { o->op_cb = cb; }
void reply(struct op_s *o, struct op_s *w) { new_cb(o, w->u.late.cb); }
void start(struct op_s *w) { w->u.late.cb = fa; }
void other(struct other_s *x) { x->pos = make_pos(); }
";
    let facts = read_unit(
        text,
        &C11TranslationUnit {
            source: text,
            ..Default::default()
        },
    )
    .expect("read");
    assert_eq!(closure(&[&facts], "op_s.op_cb"), Ok(names(&["fa"])));
}

/// Case 5: an unclassified store into the read's arm opens it.
#[test]
fn an_unclassified_store_opens() {
    if !clang_available() {
        return;
    }
    let facts = unit("void set(struct op_s *w) { w->u.late.cb = some_callback; }\n");
    assert!(closure(&[&facts], MEMBER).is_err());
}

/// The keying correction: Clang spells an anonymous member's type
/// `struct T::(unnamed at ...)`, which was read as the record `T`, so a store
/// below an anonymous member was keyed `T.f` with its path dropped and missed
/// the member's normal key `T.anon.f` (a closed empty set over a member that
/// has a store). It now keeps its path; an anonymous union is keyed by its
/// path (`T.u.*`), not as the holder's whole record (`T.*`).
#[test]
fn a_store_below_an_anonymous_member_keeps_its_path() {
    if !clang_available() {
        return;
    }
    let text = "\
typedef int cb_t(void *);
int fa(void *);
struct conf_s { int level; struct { struct { cb_t *refresh; } oauth; } sasl; };
void set(struct conf_s *c) { c->sasl.oauth.refresh = fa; }
";
    let facts = read_unit(
        text,
        &C11TranslationUnit {
            source: text,
            ..Default::default()
        },
    )
    .expect("read");
    assert!(
        facts.stores.contains_key("conf_s.sasl.oauth.refresh"),
        "{:?}",
        facts.stores
    );
    assert_eq!(
        closure(&[&facts], "conf_s.sasl.oauth.refresh"),
        Ok(names(&["fa"]))
    );
    let union = unit("");
    assert!(union.stores.keys().any(|key| key == "op_s.u.*@late.cb"));
    assert!(!union.stores.keys().any(|key| key.starts_with("op_s.*")));
}

/// Fail-closed edges: without a layout the union is shared as before (the
/// sibling and the cast free callback join and open the read); two units
/// whose layouts disagree leave it unplaced.
#[test]
fn an_unplaced_or_disagreeing_union_stays_shared() {
    if !clang_available() {
        return;
    }
    let facts = unit(
        "void early(struct op_s *w) { w->u.early.cb = fb; }\n\
         void admin(struct op_s *w) { list_init(&w->u.admin.args, (void (*)(void *))fc); }\n",
    );
    let mut unplaced = facts.clone();
    unplaced.union_layouts.clear();
    assert!(closure(&[&unplaced], MEMBER).is_err());
    let mut other = MemberFacts::default();
    let mut moved = facts.union_layouts["op_s.u.*"].clone();
    moved.paths.insert("late.cb".into(), (0, 8));
    other.union_layouts.insert("op_s.u.*".into(), moved);
    assert!(closure(&[&facts, &other], MEMBER).is_err());
    // The agreeing run keeps the separation.
    let again = facts.clone();
    assert_eq!(closure(&[&facts, &again], MEMBER), Ok(names(&["fa"])));
    // A record placement two units give differently unplaces it too.
    let mut shifted = MemberFacts::default();
    let mut layout = facts.union_layouts["op_s.u.*"].clone();
    layout
        .members
        .insert("list_s.free_cb".into(), Some(BTreeSet::from([(32, 8)])));
    shifted.union_layouts.insert("op_s.u.*".into(), layout);
    assert!(closure(&[&facts, &shifted], MEMBER).is_err());
}
