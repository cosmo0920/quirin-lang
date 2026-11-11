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

//! Card CG22-INDIRECT-CALLS-AND-CALLBACKS (rule 11p): the decision core,
//! bound to `proof/QCore/IndirectCalls.lean` by its `joinVectors` and
//! `callbackVectors`: each Lean row is the Rust table's row, in order, and
//! the Rust decision reproduces it.

use super::*;

const LEAN: &str = include_str!("../../../../proof/QCore/IndirectCalls.lean");

/// The rows of the Lean list that starts with `header`, one per line.
fn lean_rows(header: &str) -> Vec<String> {
    let start = LEAN.find(header).expect("the Lean vector list") + header.len();
    let body = &LEAN[start..];
    let end = body.find("\n]").expect("the list's end");
    body[..end]
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| line.trim_end_matches(',').to_owned())
        .collect()
}

fn lean_verdict(text: &str) -> Verdict {
    match text.trim() {
        ".safe" => Verdict::Safe,
        ".pending" => Verdict::Pending,
        ".unknown" => Verdict::Unknown,
        ".unsafe_" => Verdict::Unsafe,
        other => panic!("unknown Lean verdict {other:?}"),
    }
}

fn lean_event(text: &str) -> CallbackEvent {
    match text.trim() {
        ".register" => CallbackEvent::Register,
        ".invoke" => CallbackEvent::Invoke,
        ".unregister" => CallbackEvent::Unregister,
        ".release" => CallbackEvent::Release,
        other => panic!("unknown Lean event {other:?}"),
    }
}

/// `[a, b]` as its items.
fn items(list: &str) -> Vec<&str> {
    let inner = list.trim().trim_start_matches('[').trim_end_matches(']');
    if inner.trim().is_empty() {
        Vec::new()
    } else {
        inner.split(',').map(str::trim).collect()
    }
}

#[test]
fn join_vectors_hold() {
    for (covers, verdicts, expected) in JOIN_VECTORS {
        assert_eq!(
            join_over(*covers, verdicts),
            *expected,
            "{covers} {verdicts:?}"
        );
    }
}

#[test]
fn join_vectors_are_the_lean_vectors() {
    let rows = lean_rows("def joinVectors : List (Bool × List Verdict × Verdict) := [");
    assert_eq!(rows.len(), JOIN_VECTORS.len());
    for (row, (covers, verdicts, expected)) in rows.iter().zip(JOIN_VECTORS) {
        // `(true, [.safe, .safe], .safe)`
        let row = row.trim_start_matches('(').trim_end_matches(')');
        let open = row.find('[').expect("a list");
        let close = row.find(']').expect("a list");
        let lean_covers = row[..open].trim().trim_end_matches(',').trim() == "true";
        let lean_verdicts = items(&row[open..=close])
            .into_iter()
            .map(lean_verdict)
            .collect::<Vec<_>>();
        let lean_expected = lean_verdict(row[close + 1..].trim().trim_start_matches(','));
        assert_eq!(lean_covers, *covers, "{row}");
        assert_eq!(lean_verdicts, *verdicts, "{row}");
        assert_eq!(lean_expected, *expected, "{row}");
        assert_eq!(
            join_over(lean_covers, &lean_verdicts),
            lean_expected,
            "{row}"
        );
    }
}

#[test]
fn callback_vectors_hold() {
    for (events, expected) in CALLBACK_VECTORS {
        assert_eq!(callback_trace(events), *expected, "{events:?}");
    }
}

#[test]
fn callback_vectors_are_the_lean_vectors() {
    let rows = lean_rows("def callbackVectors : List (List Ev × Verdict) := [");
    assert_eq!(rows.len(), CALLBACK_VECTORS.len());
    for (row, (events, expected)) in rows.iter().zip(CALLBACK_VECTORS) {
        // `([.register, .invoke], .unknown)`
        let row = row.trim_start_matches('(').trim_end_matches(')');
        let close = row.find(']').expect("a list");
        let lean_events = items(&row[..=close])
            .into_iter()
            .map(lean_event)
            .collect::<Vec<_>>();
        let lean_expected = lean_verdict(row[close + 1..].trim().trim_start_matches(','));
        assert_eq!(lean_events, *events, "{row}");
        assert_eq!(lean_expected, *expected, "{row}");
        assert_eq!(callback_trace(&lean_events), lean_expected, "{row}");
    }
}

#[test]
fn the_verdict_order_is_the_lean_rank() {
    let order = [
        Verdict::Safe,
        Verdict::Pending,
        Verdict::Unknown,
        Verdict::Unsafe,
    ];
    assert!(order.windows(2).all(|pair| pair[0] < pair[1]));
    let lean = LEAN.replace("\r\n", "\n");
    assert!(lean.contains("| .safe => 0\n  | .pending => 1\n  | .unknown => 2\n  | .unsafe_ => 3"));
}
