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

//! Card **NAME-KEYED-HOLDER-SHADOWING** (gateway rule 86): rule 12t's
//! formal-keyed readings and a block-scope local that shadows the formal.
//!
//! Rule 12t reads a retained actual `NAME` (or `&NAME->f`) as a caller's
//! object, a path holder `NAME->f` as storage the caller supplied, and keys
//! a removal on either, from the formal's spelling: `NAME` is an unrebound
//! formal (`CallerFormals::contains`, a token census). A local that
//! shadows the formal and is named by the same spelling would be read as
//! the formal: a local array "outliving" a frame holder, a removal through
//! the shadow dropping the formal's record, or the shadow's record removed
//! through the formal.
//!
//! At base every probe already refuses. The shadow's declaration is refused
//! by the classifier's guard (`local declaration may shadow a formal
//! resource identity`, `legacy_classify.rs`) wherever the guard recognizes
//! a declaration. Where it does not (a declaration opening with
//! `__attribute__`, `_Alignas` or `__typeof__`, or a parenthesized
//! declarator), the declaration is refused by another route: an
//! uncontracted call, an indirect access, an unmodelled initializer. Those
//! refusals are incidental to the shape. Rule 86 makes the readings
//! themselves decide from Clang's declaration ids, so they no longer rest
//! on either refusal: a formal is read as one only where the name has one
//! declaration in the function, its `ParmVarDecl`, and every `DeclRefExpr`
//! of the name designates it (`legacy_formal_declaration.rs`, unit tests
//! there). The controls keep their readings.
//!
//! The `r_*` and `ev_*` declarations are synthetic trusted test assumptions
//! (`fixtures/contracts/retained_by.qv`, `fixtures/contracts/event_loop.qv`):
//! a `Safe` here is `Safe` over them and promotes nothing.
//! Lean: `QCore.NameKeyedHolderShadowing`.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11, check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const RETAINED: &str = "\
struct st { const char *base; int n; };
void *r_alloc(__SIZE_TYPE__ n); void r_free(void *p);
int r_keep(struct st *state, const char *src); void r_reset(struct st *state);
void r_global(const char *value);
";

const LOOP: &str = "\
struct ev_loop; struct obj { int x; };
struct coll { int id; struct ev_loop *evl; struct obj event; struct obj timer; struct coll *next; };
void *ev_alloc(__SIZE_TYPE__ n); void ev_free(void *p);
struct ev_loop *ev_loop_create(void); void ev_loop_destroy(struct ev_loop *loop);
int ev_add(struct ev_loop *loop, void *data); int ev_del(struct ev_loop *loop, void *data);
";

fn load(name: &str, text: &str) -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(name.to_owned(), text.to_owned())]))
        .expect("the fixture contracts load")
}

fn retained() -> ResourceContracts {
    load(
        "retained_by.qv",
        include_str!("fixtures/contracts/retained_by.qv"),
    )
}

fn event_loop() -> ResourceContracts {
    load(
        "event_loop.qv",
        include_str!("fixtures/contracts/event_loop.qv"),
    )
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent NAME-KEYED-HOLDER-SHADOWING test: {reason}");
            false
        }
    }
}

#[derive(Debug)]
struct Run {
    status: AnalysisStatus,
    unsupported: Vec<String>,
}

fn run(prelude: &str, source: &str, environment: &ResourceContracts) -> Run {
    let text = format!("{prelude}\n{source}\n");
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
        "subject",
        environment,
    )
    .unwrap_or_else(|error| panic!("fixture rejected by Clang: {error:?}\n{text}"));
    if let UnrestrictedLegacyResult::Safe(certificate) = &outcome {
        check_legacy_safety_certificate(certificate).expect("a Safe result rechecks");
    }
    let summary = match &outcome {
        UnrestrictedLegacyResult::Safe(certificate) => Some(&certificate.summary),
        UnrestrictedLegacyResult::Unsafe(summary) => Some(summary),
        UnrestrictedLegacyResult::Unknown { partial, .. } => partial.as_ref(),
    };
    let row = summary
        .and_then(|summary| summary.functions.get("subject"))
        .unwrap_or_else(|| panic!("`subject` projected: {outcome:?}"));
    Run {
        status: row.status,
        unsupported: row
            .diagnostics
            .iter()
            .map(|d| match &d.kind {
                LegacyDiagnosticKind::Unsupported { description } => description.clone(),
                other => format!("{other:?}"),
            })
            .collect(),
    }
}

const GUARD: &str = "local declaration may shadow a formal resource identity";
const UNCONTRACTED: &str = "call to uncontracted function";
const INDIRECT: &str = "indirect memory access";

type Shape<'a> = (&'a str, &'a [&'a str], &'a str);

/// Each shape must not be `Safe`, and must carry one of its refusal
/// fragments.
fn refused(prelude: &str, environment: &ResourceContracts, shapes: &[Shape<'_>]) {
    let mut admitted = Vec::new();
    for (name, fragments, source) in shapes {
        let run = run(prelude, source, environment);
        if run.status == AnalysisStatus::Safe
            || !fragments
                .iter()
                .any(|fragment| run.unsupported.iter().any(|d| d.contains(fragment)))
        {
            admitted.push(format!("{name}: {source}: {run:?}"));
        }
    }
    assert!(admitted.is_empty(), "{admitted:#?}");
}

/// Controls: an unshadowed formal keeps every formal-keyed reading (a
/// caller's object in a frame or tracked holder, a path holder and its
/// removal, an interior of a formal). `Safe` at base and after.
#[test]
fn an_unshadowed_formal_keeps_its_readings() {
    if !clang_available() {
        return;
    }
    let mut refused = Vec::new();
    for source in [
        "int subject(const char *src) { struct st s; return r_keep(&s, src); }",
        "void subject(const char *src) { struct st *h = r_alloc(sizeof(struct st)); if (!h) return; r_keep(h, src); r_free(h); }",
    ] {
        let run = run(RETAINED, source, &retained());
        if run.status != AnalysisStatus::Safe {
            refused.push(format!("{source}: {run:?}"));
        }
    }
    for source in [
        "void subject(struct coll *coll, struct obj *o) { ev_add(coll->evl, o); ev_del(coll->evl, o); }",
        "void subject(struct obj *o) { struct ev_loop *l = ev_loop_create(); if (!l) return; ev_add(l, o); ev_loop_destroy(l); }",
        "void subject(struct coll *coll) { struct ev_loop *l = ev_loop_create(); if (!l) return; ev_add(l, &coll->event); ev_loop_destroy(l); }",
        "void subject(struct coll *coll, struct coll *c) { ev_add(coll->evl, &c->event); ev_del(coll->evl, &c->event); }",
    ] {
        let run = run(LOOP, source, &event_loop());
        if run.status != AnalysisStatus::Safe {
            refused.push(format!("{source}: {run:?}"));
        }
    }
    assert!(refused.is_empty(), "{refused:#?}");
}

/// Retained actual (a caller's object): a local array shadowing the formal
/// in an inner block (S1, S2), a for-init (S3), a pointer local (S4), a
/// `__auto_type` local (S5), a typedef'd array (S6), each retained by a
/// frame holder, and a tracked holder (S7). The classifier's guard refuses
/// each declaration.
#[test]
fn a_shadowed_retained_formal_refuses_at_the_guard() {
    if !clang_available() {
        return;
    }
    refused(
        RETAINED,
        &retained(),
        &[
            (
                "S1",
                &[GUARD],
                "int subject(const char *src) { struct st s; { char src[4] = \"abc\"; r_keep(&s, src); } return 0; }",
            ),
            (
                "S2",
                &[GUARD],
                "int subject(const char *src) { struct st s; { char src[4] = \"abc\"; struct st t; r_keep(&t, src); } return r_keep(&s, src); }",
            ),
            (
                "S3",
                &[GUARD],
                "int subject(const char *src) { struct st s; int r = 0; for (char src[2] = \"a\"; src[0]; src[0] = 0) r = r_keep(&s, src); return r; }",
            ),
            (
                "S4",
                &[GUARD],
                "int subject(const char *src) { struct st s; { const char *src; src = \"x\"; r_keep(&s, src); } return 0; }",
            ),
            (
                "S5",
                &[GUARD],
                "int subject(const char *src) { struct st s; { char buf[4] = \"abc\"; __auto_type src = buf; r_keep(&s, src); } return 0; }",
            ),
            (
                "S6",
                &[GUARD],
                "typedef char buf4[4]; int subject(const char *src) { struct st s; { buf4 src; r_keep(&s, src); } return 0; }",
            ),
            (
                "S7",
                &[GUARD],
                "void subject(const char *src) { struct st *h = r_alloc(sizeof(struct st)); if (!h) return; { char src[4] = \"abc\"; r_keep(h, src); } r_free(h); }",
            ),
        ],
    );
}

/// Removal and interior: the shadow's removal dropping the formal's record
/// (M1), the shadow's record removed through the formal (M2), a shadow
/// retained by a tracked loop (M3), an interior of a shadowing array (M4),
/// an interior removal through a shadow (M5). The guard refuses each.
#[test]
fn a_shadowed_formal_never_keys_a_removal() {
    if !clang_available() {
        return;
    }
    refused(
        LOOP,
        &event_loop(),
        &[
            (
                "M1",
                &[GUARD],
                "void subject(struct coll *coll, struct obj *o) { ev_add(coll->evl, o); { struct obj o[1]; o[0].x = 0; ev_del(coll->evl, o); } }",
            ),
            (
                "M2",
                &[GUARD],
                "void subject(struct coll *coll, struct obj *o) { { struct obj o[1]; o[0].x = 0; ev_add(coll->evl, o); } ev_del(coll->evl, o); }",
            ),
            (
                "M3",
                &[GUARD],
                "void subject(struct obj *o) { struct ev_loop *l = ev_loop_create(); if (!l) return; { struct obj o[1]; o[0].x = 0; ev_add(l, o); } ev_loop_destroy(l); }",
            ),
            (
                "M4",
                &[GUARD],
                "void subject(struct coll *coll) { struct ev_loop *l = ev_loop_create(); if (!l) return; { struct coll coll[1]; coll->id = 0; ev_add(l, &coll->event); } ev_loop_destroy(l); }",
            ),
            (
                "M5",
                &[GUARD],
                "void subject(struct coll *coll, struct coll *c) { ev_add(coll->evl, &c->event); { struct coll c[1]; c->id = 0; ev_del(coll->evl, &c->event); } }",
            ),
        ],
    );
}

/// Path holder: the formal root of a path shadowed in an inner block (H1,
/// H2), behind an alias (H3), or in a for-init (H4). Besides the guard,
/// the shadow's declarator is a bare mention of the root, so the path is
/// not stable (P-HOLDER-PATH-STABLE's census) and is no path holder.
#[test]
fn a_shadowed_path_root_is_no_path_holder() {
    if !clang_available() {
        return;
    }
    refused(
        LOOP,
        &event_loop(),
        &[
            (
                "H1",
                &[GUARD],
                "void subject(struct coll *coll, struct obj *o) { ev_add(coll->evl, o); { struct coll coll[1]; coll->id = 0; ev_del(coll->evl, o); } }",
            ),
            (
                "H2",
                &[GUARD],
                "void subject(struct coll *coll) { { struct coll coll[1]; coll->evl = 0; ev_add(coll->evl, &coll->event); } ev_del(coll->evl, &coll->event); }",
            ),
            (
                "H3",
                &[GUARD],
                "void subject(struct coll *coll, struct coll *other) { struct coll *c = coll; { struct coll *c = other; ev_add(c->evl, &c->event); } ev_del(coll->evl, &coll->event); }",
            ),
            (
                "H4",
                &["braced initializer", INDIRECT],
                "void subject(struct coll *coll, struct obj *o) { ev_add(coll->evl, o); for (struct coll coll[1] = {{0}}; coll->id; coll->id = 0) ev_del(coll->evl, o); }",
            ),
        ],
    );
}

/// Declarations the guard does not recognize (an opening `__attribute__`,
/// `_Alignas` or `__typeof__`, a parenthesized declarator, a block-scope
/// enumeration constant). Each refuses by another route, recorded here, and
/// none is `Safe`. Rule 86's declaration reading refuses the formal-keyed
/// readings independently of those routes.
#[test]
fn an_unguarded_shadow_is_refused_and_never_read_as_the_formal() {
    if !clang_available() {
        return;
    }
    refused(
        RETAINED,
        &retained(),
        &[
            (
                "U1",
                &[UNCONTRACTED],
                "typedef char ch; int subject(const char *src) { struct st s; { ch (src)[4] = \"abc\"; r_keep(&s, src); } return 0; }",
            ),
            (
                "U2",
                &[INDIRECT],
                "int subject(const char *src) { struct st s; { __typeof__(char) src[4] = \"abc\"; r_keep(&s, src); } return 0; }",
            ),
            (
                "U3",
                &[INDIRECT],
                "int subject(const char *src) { struct st s; { _Alignas(8) char src[4] = \"abc\"; r_keep(&s, src); } return 0; }",
            ),
            (
                "U4",
                &[INDIRECT],
                "int subject(const char *src) { struct st s; { __attribute__((unused)) char src[4] = \"abc\"; r_keep(&s, src); } return 0; }",
            ),
            (
                "U5",
                &[UNCONTRACTED],
                "typedef char buf4[4]; int subject(const char *src) { struct st s; { __attribute__((unused)) buf4 src; r_keep(&s, src); } return 0; }",
            ),
            (
                "U6",
                &[INDIRECT],
                "int subject(const char *src) { struct st s; { __typeof__(char [4]) src; r_keep(&s, src); } return 0; }",
            ),
            (
                "U7",
                &[UNCONTRACTED],
                "typedef char buf4[4]; int subject(const char *src) { struct st s; { _Alignas(8) buf4 src; r_keep(&s, src); } return 0; }",
            ),
            (
                "U8",
                &[UNCONTRACTED],
                "typedef char buf4[4]; int subject(const char *src) { struct st s; { __attribute__((unused)) buf4 src; struct st t; r_keep(&t, src); } return r_keep(&s, src); }",
            ),
            (
                "U9",
                &["braced initializer"],
                "int subject(const char *src) { struct st s; { enum { src = 0 }; r_keep(&s, src); } return 0; }",
            ),
        ],
    );
    refused(
        LOOP,
        &event_loop(),
        &[
            (
                "U10",
                &[UNCONTRACTED],
                "typedef struct obj objarr[1]; void subject(struct coll *coll, struct obj *o) { ev_add(coll->evl, o); { __attribute__((unused)) objarr o; ev_del(coll->evl, o); } }",
            ),
            (
                "U11",
                &[INDIRECT],
                "void subject(struct coll *coll, struct obj *o) { { __typeof__(struct obj [1]) o; ev_add(coll->evl, o); } ev_del(coll->evl, o); }",
            ),
            (
                "U12",
                &[UNCONTRACTED],
                "typedef struct obj objarr[1]; void subject(struct obj *o) { struct ev_loop *l = ev_loop_create(); if (!l) return; { __attribute__((unused)) objarr o; ev_add(l, o); } ev_loop_destroy(l); }",
            ),
            (
                "U13",
                &[UNCONTRACTED],
                "typedef struct coll collarr[1]; void subject(struct coll *coll) { struct ev_loop *l = ev_loop_create(); if (!l) return; { __attribute__((unused)) collarr coll; ev_add(l, &coll->event); } ev_loop_destroy(l); }",
            ),
            (
                "U14",
                &[UNCONTRACTED],
                "typedef struct coll collarr[1]; void subject(struct coll *coll, struct obj *o) { ev_add(coll->evl, o); { __attribute__((unused)) collarr coll; ev_del(coll->evl, o); } }",
            ),
            (
                "U15",
                &[INDIRECT],
                "void subject(struct coll *coll, struct obj *o) { ev_add(coll->evl, o); { __typeof__(struct obj) o[1]; o[0].x = 0; ev_del(coll->evl, o); } }",
            ),
            (
                "U16",
                &[INDIRECT],
                "typedef struct obj objt; void subject(struct coll *coll, struct obj *o) { ev_add(coll->evl, o); { objt (o)[1]; o[0].x = 0; ev_del(coll->evl, o); } }",
            ),
        ],
    );
}
