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

//! Card CG24-ALIAS-EXTENT-FRAME-VOCABULARY (gateway rule 11e): frame effects
//! and derived relations of the bounds property through the route-G source
//! consumer. Frames are not custody modes: a callee that only borrows custody
//! may still write or move what it reaches, and a read-only callee keeps the
//! caller's facts. Every custody verdict asserted here is the custody
//! gateway's own, unchanged by this card.

#[path = "support/clang_gap.rs"]
mod clang_gap;

use std::collections::BTreeMap;

use clang_gap::*;
use qcore_c_gateway::index_extent::{
    BoundsFunctionRow, BoundsUnitReport, analyze_bounds_c11, row_verdict,
};
use qcore_c_gateway::legacy_c::C11TranslationUnit;
use qcore_contract::bounds_contract::{BoundsContracts, load_bounds_contract_sources};
use qcore_core::index_extent::BoundsVerdict;

use BoundsVerdict::{Pending, Safe, Unknown};

fn contracts(text: &str) -> BoundsContracts {
    load_bounds_contract_sources(&BTreeMap::from([(
        "cg24.qv".to_owned(),
        format!("module contract;\n{text}"),
    )]))
    .expect("bounds fixture loads")
}

fn text(source: &str) -> String {
    format!("{DECLARATIONS}\n{source}\n")
}

fn report(source: &str, functions: &[&str], bounds: &BoundsContracts) -> BoundsUnitReport {
    let text = text(source);
    analyze_bounds_c11(
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
        &functions
            .iter()
            .map(|f| (*f).to_owned())
            .collect::<Vec<_>>(),
        bounds,
    )
    .unwrap_or_else(|error| panic!("bounds consumer refused the unit: {error}\n{source}"))
}

fn row(source: &str, bounds: &BoundsContracts) -> BoundsFunctionRow {
    report(source, &["subject"], bounds).functions.remove(0)
}

fn verdict(source: &str, bounds: &BoundsContracts) -> BoundsVerdict {
    row_verdict(&row(source, bounds))
}

fn has_reason(row: &BoundsFunctionRow, fragment: &str) -> bool {
    row.reasons.iter().any(|r| r.contains(fragment))
}

/// The subject's own precondition: `p` points to 8 elements (resolved).
const OWN: &str = "extern fn subject(p: ptr<mut, u8>) -> i32
contract {
    requires bounds_contract_v1;
    requires bounds_extent(p, 8, elements);
};
";

/// A callee over one pointer with the given frame clause (or none).
fn callee(name: &str, frame: &str) -> String {
    format!(
        "extern fn {name}(q: ptr<mut, u8>) -> unit
contract {{
    requires bounds_contract_v1;
{frame}}};
"
    )
}

// ---------------------------------------------------------------------------
// Frames: read, write, move
// ---------------------------------------------------------------------------

/// The card's first two controls on a declared object: a read-only callee
/// keeps the extent fact, a callee stating no frame drops it (CG16's rule),
/// a writing callee keeps it (writing contents resizes nothing), a moving
/// callee drops it.
#[test]
fn cg24_a_read_only_callee_keeps_a_bounds_fact_and_a_moving_one_drops_it() {
    if !clang_available() {
        return;
    }
    let source = "void touch_it(int *q); int subject(int *p) { touch_it(p); return p[7]; }";
    let cases = [
        ("    ensures normal bounds_frame_reads(q);\n", Safe),
        ("    ensures normal bounds_frame_writes(q);\n", Safe),
        ("    ensures normal bounds_frame_moves(q);\n", Unknown),
        ("", Unknown),
    ];
    for (frame, expected) in cases {
        let c = contracts(&format!("{OWN}{}", callee("touch_it", frame)));
        let r = row(source, &c);
        assert_eq!(row_verdict(&r), expected, "{frame:?}: {r:#?}");
        if expected == Unknown {
            assert!(has_reason(&r, "bounds-missing-extent"), "{r:#?}");
        }
    }
    // The custody gateway reads none of it: `touch_it` is uncontracted there.
    let run = g(&format!(
        "void touch_it(int *q); {}",
        "int subject(int *p) { touch_it(p); return p[7]; }"
    ));
    assert!(
        run.refused("call to uncontracted function `touch_it`"),
        "{run:?}"
    );
}

/// A local whose address is passed only to closed frame positions carries
/// facts: a read position keeps its value, a write position withdraws it.
/// Without a frame (CG16) the address-taken local carries no fact at all.
#[test]
fn cg24_a_writing_callee_drops_a_local_value_fact() {
    if !clang_available() {
        return;
    }
    let source = "void index_of(int *out); int subject(void) { int a[4]; int i = 2; index_of(&i); return a[i]; }";
    let reads = contracts(&callee(
        "index_of",
        "    ensures normal bounds_frame_reads(q);\n",
    ));
    let writes = contracts(&callee(
        "index_of",
        "    ensures normal bounds_frame_writes(q);\n",
    ));
    let unstated = contracts(&callee("index_of", ""));
    assert_eq!(verdict(source, &reads), Safe);
    let r = row(source, &writes);
    assert_eq!(row_verdict(&r), Unknown, "{r:#?}");
    assert!(has_reason(&r, "bounds-unsupported-range"), "{r:#?}");
    assert_eq!(verdict(source, &unstated), Unknown);
    // A pointer local: its offset fact survives a read and dies with a write.
    let pointer = "void slot_of(int **out); int subject(void) { int a[4]; int *p = a + 1; slot_of(&p); return p[2]; }";
    let slot = |frame: &str| {
        contracts(&format!(
            "extern fn slot_of(s: ptr<mut, ptr<mut, u8>>) -> unit
contract {{
    requires bounds_contract_v1;
    ensures normal bounds_frame_{frame}(s);
}};
"
        ))
    };
    assert_eq!(verdict(pointer, &slot("reads")), Safe);
    assert_eq!(verdict(pointer, &slot("writes")), Unknown);
    // An address passed anywhere else keeps the local untracked.
    let escaped = "void index_of(int *out); int subject(void) { int a[4]; int i = 2; int *e = &i; index_of(&i); return a[i] + *e; }";
    assert_eq!(verdict(escaped, &reads), Unknown);
    // A held frame does not track the local (Unknown, never Pending).
    let held = callee(
        "index_of",
        "    requires bounds_review_pending(\"does it only read?\");\n    ensures normal bounds_frame_reads(q);\n",
    );
    assert_eq!(verdict(source, &contracts(&held)), Unknown);
}

/// A loop withdraws what its frame positions write, as it withdraws what it
/// assigns.
#[test]
fn cg24_a_loop_withdraws_what_its_calls_write() {
    if !clang_available() {
        return;
    }
    let source = "void index_of(int *out); int subject(int c) { int a[4]; int i = 2; while (c--) { index_of(&i); } return a[i]; }";
    let reads = contracts(&callee(
        "index_of",
        "    ensures normal bounds_frame_reads(q);\n",
    ));
    let writes = contracts(&callee(
        "index_of",
        "    ensures normal bounds_frame_writes(q);\n",
    ));
    assert_eq!(verdict(source, &reads), Safe);
    assert_eq!(verdict(source, &writes), Unknown);
}

// ---------------------------------------------------------------------------
// The sds in-place reallocation (W46-SDS-CAT-SAFE-IN-PLACE-REALLOC, bounds half)
// ---------------------------------------------------------------------------

const MAKE: &str = "extern fn make(n: usize) -> ptr<mut, u8>
contract {
    requires bounds_contract_v1;
    ensures normal bounds_return_extent(n, bytes);
};
";

fn sds(frame: &str) -> String {
    format!(
        "extern fn cfl_sds_cat_safe(buf: ptr<mut, ptr<mut, u8>>, str: ptr<const, u8>, len: i32) -> unit
contract {{
    requires bounds_contract_v1;
    requires bounds_extent(str, len, bytes);
    ensures normal bounds_frame_{frame}(buf);
    ensures normal bounds_frame_reads(str);
}};
"
    )
}

/// `cfl_sds_cat_safe(&buf, …)` may reallocate the string behind the slot:
/// declared `bounds_frame_moves`, it withdraws the fact of the slot's local
/// and of every alias of the old base. A read declaration keeps both; a write
/// declaration (the slot rewritten, the old object kept) keeps only the alias.
#[test]
fn cg24_the_sds_in_place_realloc_control() {
    if !clang_available() {
        return;
    }
    let decl = "char *make(unsigned long n); void cfl_sds_cat_safe(char **buf, const char *str, int len); ";
    let alias = format!(
        "{decl}int subject(void) {{ char *buf = make(16); char *old = buf; cfl_sds_cat_safe(&buf, \"x\", 1); return old[15]; }}"
    );
    let slot = format!(
        "{decl}int subject(void) {{ char *buf = make(16); cfl_sds_cat_safe(&buf, \"x\", 1); return buf[15]; }}"
    );
    for (frame, alias_verdict, slot_verdict) in [
        ("reads", Safe, Safe),
        ("writes", Safe, Unknown),
        ("moves", Unknown, Unknown),
    ] {
        let c = contracts(&format!("{MAKE}{}", sds(frame)));
        assert_eq!(
            verdict(&alias, &c),
            alias_verdict,
            "{frame}: alias of the old base"
        );
        assert_eq!(
            verdict(&slot, &c),
            slot_verdict,
            "{frame}: the slot's local"
        );
    }
    // The call's own precondition on `str` is still checked: two bytes of
    // "x" do not cover a length of 3.
    let c = contracts(&format!("{MAKE}{}", sds("moves")));
    let short = format!(
        "{decl}int subject(void) {{ char *buf = make(16); cfl_sds_cat_safe(&buf, \"x\", 3); return 0; }}"
    );
    let r = row(&short, &c);
    assert_ne!(row_verdict(&r), Safe, "{r:#?}");
    // Custody is unchanged: resource v1 cannot state the reallocation, so the
    // custody gateway still refuses the call (the custody half stays open).
    let run = g(
        "void cfl_sds_cat_safe(char **buf, const char *str, int len); \
         void subject(void) { char *buf = q_alloc(); cfl_sds_cat_safe(&buf, \"x\", 1); q_free(buf); }",
    );
    assert!(
        run.refused("call to uncontracted function `cfl_sds_cat_safe`"),
        "{run:?}"
    );
}

// ---------------------------------------------------------------------------
// Derived relations
// ---------------------------------------------------------------------------

const SKIP: &str = "extern fn skip(p: ptr<mut, u8>, off: usize) -> ptr<mut, u8>
contract {
    requires bounds_contract_v1;
    ensures normal bounds_frame_reads(p);
    ensures normal bounds_return_derived(p, off, 4, elements);
};
";

const SKIP_C: &str = "int *skip(int *p, unsigned long off); ";

/// A result declared as `p + off` with 4 elements: inside a known base it is
/// that base (its whole extent applies), a base too small contradicts the
/// declaration, and an unknown base leaves the declared view.
#[test]
fn cg24_a_derived_result_stays_within_its_base() {
    if !clang_available() {
        return;
    }
    let c = contracts(SKIP);
    assert_eq!(
        verdict(
            &format!("{SKIP_C}int subject(void) {{ int a[8]; int *q = skip(a, 2); return q[5]; }}"),
            &c
        ),
        Safe,
        "q is a + 2 in an 8-element base"
    );
    assert_ne!(
        verdict(
            &format!("{SKIP_C}int subject(void) {{ int a[8]; int *q = skip(a, 2); return q[6]; }}"),
            &c
        ),
        Safe
    );
    // Same object: `q - a` is a subtraction within one object.
    assert_eq!(
        verdict(
            &format!(
                "{SKIP_C}long subject(void) {{ int a[8]; int *q = skip(a, 2); return q - a; }}"
            ),
            &c
        ),
        Safe
    );
    let r = row(
        &format!("{SKIP_C}int subject(void) {{ int a[4]; int *q = skip(a, 2); return q[0]; }}"),
        &c,
    );
    assert_eq!(row_verdict(&r), Unknown, "{r:#?}");
    assert!(has_reason(&r, "bounds-declaration-contradicted"), "{r:#?}");
    // An unknown base: the declared view of 4 elements.
    let r = row(
        &format!("{SKIP_C}int subject(int *p) {{ int *q = skip(p, 2); return q[3]; }}"),
        &c,
    );
    assert_eq!(row_verdict(&r), Safe, "{r:#?}");
    assert!(
        r.assumptions
            .iter()
            .any(|a| a.contains("bounds_return_derived"))
    );
    assert_ne!(
        verdict(
            &format!("{SKIP_C}int subject(int *p) {{ int *q = skip(p, 2); return q[4]; }}"),
            &c
        ),
        Safe
    );
}

/// Several bases: the result is one of them, so it receives only the common
/// declared view, and one contradicted base refuses it.
#[test]
fn cg24_a_multi_base_relation_gives_only_the_common_view() {
    if !clang_available() {
        return;
    }
    let pick = "extern fn pick(a: ptr<mut, u8>, b: ptr<mut, u8>) -> ptr<mut, u8>
contract {
    requires bounds_contract_v1;
    ensures normal bounds_frame_reads(a);
    ensures normal bounds_frame_reads(b);
    ensures normal bounds_return_derived(a, 0, 4, elements);
    ensures normal bounds_return_derived(b, 0, 4, elements);
};
";
    let c = contracts(pick);
    let decl = "int *pick(int *a, int *b); ";
    assert_eq!(
        verdict(
            &format!(
                "{decl}int subject(void) {{ int x[4]; int y[8]; int *r = pick(x, y); return r[3]; }}"
            ),
            &c
        ),
        Safe
    );
    assert_ne!(
        verdict(
            &format!(
                "{decl}int subject(void) {{ int x[4]; int y[8]; int *r = pick(x, y); return r[5]; }}"
            ),
            &c
        ),
        Safe,
        "the view is 4 elements whichever base it is"
    );
    let r = row(
        &format!(
            "{decl}int subject(void) {{ int x[2]; int y[8]; int *r = pick(x, y); return r[0]; }}"
        ),
        &c,
    );
    assert_eq!(row_verdict(&r), Unknown, "{r:#?}");
    assert!(has_reason(&r, "bounds-declaration-contradicted"), "{r:#?}");
}

// ---------------------------------------------------------------------------
// Refutation by the body
// ---------------------------------------------------------------------------

/// A body that writes through a parameter it declares read, moves one it
/// declares written, or returns something other than its declared base
/// refutes its declaration: callers then see an uncontracted callee.
#[test]
fn cg24_a_relation_or_frame_contradicted_by_its_body_is_refused() {
    if !clang_available() {
        return;
    }
    // A read frame written through.
    let c = contracts(&format!(
        "{OWN}{}",
        callee("touch_it", "    ensures normal bounds_frame_reads(q);\n")
    ));
    let source =
        "void touch_it(int *q) { q[0] = 1; } int subject(int *p) { touch_it(p); return p[7]; }";
    let unit = report(source, &["subject", "touch_it"], &c);
    let body = unit.row("touch_it").unwrap();
    assert!(body.refutes_own_contract, "{body:#?}");
    assert!(
        body.reasons
            .iter()
            .any(|r| r.contains("bounds-contract-refuted-by-body"))
    );
    assert_eq!(unit.row("subject").unwrap().verdict, "Unknown");
    // The same body declared `writes` is within its frame.
    let c = contracts(&format!(
        "{OWN}{}",
        callee("touch_it", "    ensures normal bounds_frame_writes(q);\n")
    ));
    let unit = report(source, &["subject", "touch_it"], &c);
    assert!(!unit.row("touch_it").unwrap().refutes_own_contract);
    assert_eq!(unit.row("subject").unwrap().verdict, "Safe");
    // A write frame that hands its parameter to a moving callee.
    let c = contracts(&format!(
        "{OWN}{}{}",
        callee("touch_it", "    ensures normal bounds_frame_writes(q);\n"),
        callee("grow", "    ensures normal bounds_frame_moves(q);\n")
    ));
    let moved = "void grow(int *q); void touch_it(int *q) { grow(q); } int subject(int *p) { touch_it(p); return p[7]; }";
    let unit = report(moved, &["subject", "touch_it"], &c);
    assert!(unit.row("touch_it").unwrap().refutes_own_contract);
    assert_eq!(unit.row("subject").unwrap().verdict, "Unknown");
    // A derived relation: `p + 2` declared, `p + 1` returned.
    let fixed = "extern fn skip(p: ptr<mut, u8>) -> ptr<mut, u8>
contract {
    requires bounds_contract_v1;
    ensures normal bounds_frame_reads(p);
    ensures normal bounds_return_derived(p, 2, 4, elements);
};
";
    let c = contracts(fixed);
    for (body, refuted) in [
        ("return p + 2;", false),
        ("return &p[2];", false),
        ("return p + 1;", true),
        ("return store;", true),
    ] {
        let source = format!(
            "static int store[8]; int *skip(int *p) {{ {body} }} \
             int subject(void) {{ int a[8]; int *q = skip(a); return q[5]; }}"
        );
        let unit = report(&source, &["subject", "skip"], &c);
        let getter = unit.row("skip").unwrap();
        assert_eq!(getter.refutes_own_contract, refuted, "{body}: {getter:#?}");
        assert_eq!(
            unit.row("subject").unwrap().verdict,
            if refuted { "Unknown" } else { "Safe" },
            "{body}"
        );
    }
}

// ---------------------------------------------------------------------------
// Review holds
// ---------------------------------------------------------------------------

/// A fact kept across a held frame rests on it: Pending as the sole obstacle,
/// Safe once the marker is edited away.
#[test]
fn cg24_a_held_frame_keeps_facts_only_as_held_premises() {
    if !clang_available() {
        return;
    }
    let source = "void touch_it(int *q); int subject(int *p) { touch_it(p); return p[7]; }";
    let held = callee(
        "touch_it",
        "    requires bounds_review_pending(\"does it only read?\", \"cg24-owner\");\n    ensures normal bounds_frame_reads(q);\n",
    );
    let r = row(source, &contracts(&format!("{OWN}{held}")));
    assert_eq!(row_verdict(&r), Pending, "{r:#?}");
    assert!(has_reason(&r, "does it only read?"), "{r:#?}");
    // A held derived relation: the view rests on it.
    let held_skip = SKIP.replace(
        "requires bounds_contract_v1;",
        "requires bounds_contract_v1;\n    requires bounds_review_pending(\"is it p + off?\");",
    );
    assert_eq!(
        verdict(
            &format!("{SKIP_C}int subject(int *p) {{ int *q = skip(p, 2); return q[3]; }}"),
            &contracts(&held_skip)
        ),
        Pending
    );
}
