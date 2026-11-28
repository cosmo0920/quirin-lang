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

//! Card SETTLED-LEAK-MARK-LEAFLESS (gateway rule 84, amended; a module of
//! `cross_unit_elements_released.rs`, adding no test target): a leak whose
//! settled exit marks the identity in a function with no non-`Safe` leaf at
//! the settle point.
//!
//! The mark is rule 43's: `drain`'s imported `ElementsReleased` clause
//! withdraws every object the caller published (`forget_published`), a
//! modelled effect and no refusal. A caller that publishes `x` and drains on
//! one arm and leaves `x` open on the other has `x` live and marked at the
//! settled exit, beside no `Unknown` or `Pending` leaf. Each shape comes in two
//! schedules of the same program shape: the arm that leaves `x` open reaches
//! the exit first (an iterate claims the leak) or the drained arm does (its
//! mark joins first and nothing is claimed).
//!
//! * `ONE_ARM`: the open arm is refusal-free, so rule 84's witness carries it.
//! * `RETAINED`: the open arm passes a retained loop exit edge (`while (!k)`
//!   over a modelled `k = 0`), which is `unmodeled` with no diagnostic until
//!   the settled-state report after the settle point, so no witness exists.
//! * `JOINED`: the publication on one arm, the drain after the join. The
//!   published set is joined by intersection and the mark by union, so the
//!   settled states differ between the two programs (published-first leaves a
//!   stale mark at the exit; unpublished-first has none).
//!
//! Every `Unsafe` is provisional, over the synthetic trusted fixture
//! contracts.

use super::*;

const ONE_ARM: [(&str, &str); 2] = [
    (
        "open arm first",
        "void subject(struct queue *q, int flag) { struct item *x = q_alloc(); if (!x) return;
         if (flag) { flag = flag + 1; mk_list_add(&x->_head, &q->items); drain(q); } }",
    ),
    (
        "drained arm first",
        "void subject(struct queue *q, int flag) { struct item *x = q_alloc(); if (!x) return;
         if (flag) { mk_list_add(&x->_head, &q->items); drain(q); }
         else { flag = flag + 1; flag = flag + 2; flag = flag + 3; flag = flag + 4; } }",
    ),
];

const RETAINED: [(&str, &str); 2] = [
    (
        "open arm first",
        "void subject(struct queue *q, int flag) { struct item *x = q_alloc(); if (!x) return;
         if (flag) { flag = flag + 1; flag = flag + 2; flag = flag + 3;
         mk_list_add(&x->_head, &q->items); drain(q); }
         else { int k = 0; (void)k; while (!k) { } } }",
    ),
    (
        "drained arm first",
        "void subject(struct queue *q, int flag) { struct item *x = q_alloc(); if (!x) return;
         if (flag) { mk_list_add(&x->_head, &q->items); drain(q); }
         else { int k = 0; (void)k; flag = flag + 1; flag = flag + 2; while (!k) { } } }",
    ),
];

const JOINED: [(&str, &str); 2] = [
    (
        "unpublished arm first",
        "void subject(struct queue *q, int flag) { struct item *x = q_alloc(); if (!x) return;
         if (flag) { flag = flag + 1; flag = flag + 2; mk_list_add(&x->_head, &q->items); }
         drain(q); }",
    ),
    (
        "published arm first",
        "void subject(struct queue *q, int flag) { struct item *x = q_alloc(); if (!x) return;
         if (flag) { mk_list_add(&x->_head, &q->items); }
         else { flag = flag + 1; flag = flag + 2; flag = flag + 3; } drain(q); }",
    ),
];

const EXIT_WITHHELD: &str =
    "exit withholds a possibly-live obligation whose effect is unresolved and unreported";
const RETAINED_EDGE: &str = "loop exit edge retained: a modeled value cannot remove it";
const REASON: &str = "the identity acquired at line 21 may be live at this exit under a modelled \
                      effect's mark, and no refusal explains the mark (mark-without-refusal)";

type Row = (AnalysisStatus, usize, Vec<String>);

/// The caller's row: its status, its leak count and its `Unsupported`
/// descriptions.
fn analysed(body: &str) -> Row {
    let exports = exports();
    let drain = &exports["drain"];
    let (status, diagnostics) = row(caller(body, drain, drain.contract.clone()));
    let leaks = diagnostics
        .iter()
        .filter(|kind| matches!(kind, LegacyDiagnosticKind::Leak { .. }))
        .count();
    let refusals = diagnostics
        .iter()
        .filter_map(|kind| match kind {
            LegacyDiagnosticKind::Unsupported { description } => Some(description.clone()),
            _ => None,
        })
        .collect();
    (status, leaks, refusals)
}

fn expect(status: AnalysisStatus, leaks: usize, refusals: &[&str]) -> Row {
    (
        status,
        leaks,
        refusals.iter().map(|text| (*text).to_owned()).collect(),
    )
}

fn check(shapes: &[(&str, &str); 2], rows: [Row; 2]) {
    for ((order, body), row) in shapes.iter().zip(rows) {
        assert_eq!(analysed(body), row, "{order}: {body}");
    }
}

/// At base `287dce8ed` the verdict read the schedule (rule 6's hazard): the
/// open arm first was claimed and published (`Unsafe`, one leak), the drained
/// arm first was not (`Unknown`, no leak), in all three shapes (probe
/// `8ba33c206`). Now both orders of each shape decide the same: the witnessed
/// open arm is published, the arm through the retained exit is withdrawn with
/// the reason, and the published-first `JOINED` program's stale mark is
/// witnessed by its open arm.
#[test]
fn both_orders_of_a_leafless_mark_decide_the_same() {
    if std::env::var_os(CHILD).is_some() || !clang_available() {
        return;
    }
    check(
        &ONE_ARM,
        [
            expect(AnalysisStatus::Unsafe, 1, &[EXIT_WITHHELD]),
            expect(AnalysisStatus::Unsafe, 1, &[EXIT_WITHHELD]),
        ],
    );
    check(
        &RETAINED,
        [
            expect(AnalysisStatus::Unknown, 0, &[REASON, RETAINED_EDGE]),
            expect(AnalysisStatus::Unknown, 0, &[REASON, RETAINED_EDGE]),
        ],
    );
    check(
        &JOINED,
        [
            expect(AnalysisStatus::Unsafe, 1, &[]),
            expect(AnalysisStatus::Unsafe, 1, &[EXIT_WITHHELD]),
        ],
    );
}

/// The nine rule 43 marks the suite census found beside no leaf at base are
/// all this body's (in `an_imported_clause_reaches_the_caller_and_both_bodies_replay`):
/// `x` is published, so `ESCAPED` and never `LIVE`, when the drain marks it.
/// No candidate, no reason, still `Safe`.
#[test]
fn a_published_mark_records_no_reason() {
    if std::env::var_os(CHILD).is_some() || !clang_available() {
        return;
    }
    let body = "void subject(struct queue *q) { struct item *x = q_alloc(); if (!x) return;
         mk_list_add(&x->_head, &q->items); drain(q); }";
    assert_eq!(
        analysed(body),
        expect(AnalysisStatus::Safe, 0, &[]),
        "{body}"
    );
}
