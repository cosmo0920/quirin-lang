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

//! Card SETTLED-LEAK-MARK-WITHOUT-UNKNOWN-LEAF (gateway rule 84, amended; a
//! module of `settled_leak_other_refusal.rs`, adding no test target): a
//! leak whose settled exit marks the identity in a function with no `Unknown`
//! leaf (a call held for review marks it) is decided from the settled states.
//!
//! The held call is `k_close` under a contract awaiting review (rule 10i,
//! card W45-CONTRACT-DEFERRED-REVIEW): its statement refuses with the
//! `Pending` held reason and hands the handle on, so the handle is marked on
//! that arm. Each fixture comes in two schedules of the same shape: the arm
//! that avoids the held call reaches the exit first (an iterate claims the
//! leak), or the held arm does (its mark joins first and nothing is claimed).
//! At base `07833c86a` the one-arm shape was `Unsafe` in the first order and
//! `Unknown` in the second (rule 6's hazard); both orders now decide the same.
//! The `k_*` family is declared inline and held out of every production
//! library; each `Unsafe` is provisional, over synthetic trusted test
//! assumptions.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, LegacyDiagnosticKind, LegacyFunctionSummary, ResourceContracts,
    analyze_legacy_c_translation_unit, held_call,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const BARE: &str = "module contract;

extern fn k_open() -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_return_owned(nullable);
};

extern fn k_close(h: ptr<mut, u8>, n: i32) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_released(h, nonnull);
    ensures normal resource_borrowed(n);
    ensures normal resource_return_none();
};

extern fn k_note(n: i32) -> unit
contract {
    requires resource_contract_v1;
    requires resource_review_pending(\"does the note keep anything?\");
    ensures normal resource_borrowed(n);
    ensures normal resource_return_none();
};
";

const PRELUDE: &str = "void *k_open(void);\nvoid k_close(void *h, int n);\nvoid k_note(int n);\n";

fn deferred() -> ResourceContracts {
    let text = BARE.replace(
        "    ensures normal resource_released(h, nonnull);",
        "    requires resource_review_pending(\"is the handle freed on every path?\");\n    \
         ensures normal resource_released(h, nonnull);",
    );
    load_resource_contract_sources(&BTreeMap::from([("k.qcore".to_owned(), text)]))
        .expect("the fixture module loads")
}

fn analyse(body: &str) -> LegacyFunctionSummary {
    let summary = analyze_legacy_c_translation_unit(&format!("{PRELUDE}{body}"), "w", &deferred())
        .expect("the fixture tokenizes");
    summary
        .functions
        .get("w")
        .expect("the function is analysed")
        .clone()
}

fn leaks(summary: &LegacyFunctionSummary) -> usize {
    summary
        .diagnostics
        .iter()
        .filter(|diagnostic| matches!(diagnostic.kind, LegacyDiagnosticKind::Leak { .. }))
        .count()
}

fn held(summary: &LegacyFunctionSummary) -> Vec<String> {
    summary
        .diagnostics
        .iter()
        .filter_map(|diagnostic| match &diagnostic.kind {
            LegacyDiagnosticKind::Unsupported { description } => {
                held_call(description).map(str::to_owned)
            }
            _ => None,
        })
        .collect()
}

/// The held arm and an arm that leaves the handle open, in two schedules.
const ONE_ARM: [(&str, &str); 2] = [
    (
        "avoiding arm first",
        "void w(int flag) { void *h; h = k_open(); if (!h) { return; } \
         if (flag) { flag = flag + 1; k_close(h, 0); } }",
    ),
    (
        "held arm first",
        "void w(int flag) { void *h; h = k_open(); if (!h) { return; } \
         if (flag) { k_close(h, 0); } else { flag = flag + 1; flag = flag + 2; } }",
    ),
];

/// The held call on both arms, in two schedules.
const BOTH_ARMS: [(&str, &str); 2] = [
    (
        "longer arm first",
        "void w(int flag) { void *h; h = k_open(); if (!h) { return; } \
         if (flag) { flag = flag + 1; k_close(h, 0); } else { k_close(h, 1); } }",
    ),
    (
        "shorter arm first",
        "void w(int flag) { void *h; h = k_open(); if (!h) { return; } \
         if (flag) { k_close(h, 0); } else { flag = flag + 1; flag = flag + 2; k_close(h, 1); } }",
    ),
];

/// The held close on one arm; the other arm leaves the handle open past an
/// unrelated held call.
const OTHER_HOLD: [(&str, &str); 2] = [
    (
        "unrelated hold arm first",
        "void w(int flag) { void *h; h = k_open(); if (!h) { return; } \
         if (flag) { flag = flag + 1; k_close(h, 0); } else { k_note(flag); } }",
    ),
    (
        "held close arm first",
        "void w(int flag) { void *h; h = k_open(); if (!h) { return; } \
         if (flag) { k_close(h, 0); } else { flag = flag + 1; flag = flag + 2; k_note(flag); } }",
    ),
];

const HOLD: &str =
    "call to `k_close` whose declared contract awaits human review (awaiting-human-review)";

fn keeps_the_hold(w: &LegacyFunctionSummary) -> bool {
    w.diagnostics.iter().any(|diagnostic| {
        matches!(&diagnostic.kind, LegacyDiagnosticKind::Unsupported { description } if description == HOLD)
    })
}

/// The avoiding arm's leak is published in both schedules, beside the held
/// reason. At base `07833c86a` the held arm first was `Unknown` with no leak
/// (the probe's `the_verdict_reads_the_schedule_at_base`): rule 84 now
/// decides the marked identity beside the `Pending` leaf, and the arm that
/// passes no mark and no refusal witnesses it.
#[test]
fn an_avoiding_arm_is_published_in_both_schedules() {
    for (order, body) in ONE_ARM {
        let w = analyse(body);
        assert_eq!(w.status, AnalysisStatus::Unsafe, "{order}: {w:?}");
        assert_eq!(leaks(&w), 1, "{order}: {w:?}");
        assert!(keeps_the_hold(&w), "{order}: {w:?}");
    }
}

/// The held call on every path: nothing is witnessed, nothing is claimed,
/// and the row stays `Pending` on the hold's own reason in both schedules,
/// never `Safe`.
#[test]
fn a_hold_on_every_path_stays_pending_in_both_schedules() {
    for (order, body) in BOTH_ARMS {
        let w = analyse(body);
        assert_eq!(w.status, AnalysisStatus::Pending, "{order}: {w:?}");
        assert_eq!(leaks(&w), 0, "{order}: {w:?}");
        assert_eq!(held(&w), ["k_close"], "{order}: {w:?}");
        assert!(keeps_the_hold(&w), "{order}: {w:?}");
    }
}

/// The open arm passes an unrelated held call: it is not a refusal-free path,
/// so nothing is witnessed and the leak is withdrawn. Both holds keep their
/// reasons, and rule 10i's standing analysis records the leak candidate the
/// accepted declarations would leave, so the row is `Unknown`, never `Safe`.
#[test]
fn an_open_arm_past_another_hold_is_withdrawn_and_stays_unknown() {
    for (order, body) in OTHER_HOLD {
        let w = analyse(body);
        assert_eq!(w.status, AnalysisStatus::Unknown, "{order}: {w:?}");
        assert_eq!(leaks(&w), 0, "{order}: {w:?}");
        assert_eq!(held(&w), ["k_close", "k_note"], "{order}: {w:?}");
        assert!(keeps_the_hold(&w), "{order}: {w:?}");
    }
}
