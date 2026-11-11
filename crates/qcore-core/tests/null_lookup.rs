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

//! Card NULL-WITNESS-GUARD-CORRELATION: the Rust decisions of
//! `qcore_core::null_lookup` against the shared vectors of
//! `proof/QCore/NullLookupEvent.lean` (`missVectors`, `stateVectors`), read
//! from the Lean text so neither side can change alone, plus the Rust-only
//! controls.

use qcore_core::null_lookup::{
    MissEvent, Root, State, Tie, argument_state, lookup_state, miss_event,
};

const LEAN: &str = include_str!("../../../proof/QCore/NullLookupEvent.lean");

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

const MISS_VECTORS: [(&str, &str, &str); 6] = [
    ("none", "free", "named"),
    ("none", "correlated", "refused"),
    ("decided-null", "free", "named"),
    ("decided-null", "correlated", "named"),
    ("other", "free", "refused"),
    ("other", "correlated", "refused"),
];

const STATE_VECTORS: [(&str, bool, bool, &str); 9] = [
    ("constant", true, true, "free"),
    ("formal", false, false, "free"),
    ("formal", false, true, "free"),
    ("formal", true, false, "correlated"),
    ("global", false, false, "free"),
    ("global", false, true, "correlated"),
    ("global", true, false, "correlated"),
    ("local", false, false, "correlated"),
    ("unrooted", false, false, "correlated"),
];

fn tie(name: &str) -> Tie {
    match name {
        "none" => Tie::None,
        "decided-null" => Tie::DecidedNull,
        _ => Tie::Other,
    }
}

fn state(name: &str) -> State {
    match name {
        "free" => State::Free,
        _ => State::Correlated,
    }
}

fn state_name(state: State) -> &'static str {
    match state {
        State::Free => "free",
        State::Correlated => "correlated",
    }
}

fn root(name: &str) -> Root {
    match name {
        "constant" => Root::Constant,
        "formal" => Root::Formal,
        "global" => Root::Global,
        "local" => Root::Local,
        _ => Root::Unrooted,
    }
}

#[test]
fn the_vectors_are_the_leans() {
    let rows = MISS_VECTORS
        .iter()
        .map(|(t, s, o)| format!("(\"{t}\", \"{s}\", \"{o}\")"))
        .collect::<Vec<_>>();
    assert_eq!(
        lean_rows("def missVectors : List (String × String × String) := ["),
        rows
    );
    let rows = STATE_VECTORS
        .iter()
        .map(|(r, t, c, o)| format!("(\"{r}\", {t}, {c}, \"{o}\")"))
        .collect::<Vec<_>>();
    assert_eq!(
        lean_rows("def stateVectors : List (String × Bool × Bool × String) := ["),
        rows
    );
}

#[test]
fn the_rust_decisions_agree_with_the_vectors() {
    for (t, s, o) in MISS_VECTORS {
        assert_eq!(miss_event(tie(t), state(s)).as_str(), o, "{t} {s}");
    }
    for (r, t, c, o) in STATE_VECTORS {
        assert_eq!(state_name(argument_state(root(r), t, c)), o, "{r} {t} {c}");
    }
}

#[test]
fn one_correlated_argument_correlates_the_lookup() {
    assert_eq!(lookup_state(&[]), State::Free);
    assert_eq!(lookup_state(&[State::Free, State::Free]), State::Free);
    assert_eq!(
        lookup_state(&[State::Free, State::Correlated]),
        State::Correlated
    );
}

#[test]
fn a_named_miss_is_decided_or_free() {
    for t in [Tie::None, Tie::DecidedNull, Tie::Other] {
        for s in [State::Free, State::Correlated] {
            if miss_event(t, s) == MissEvent::Named {
                assert!(t == Tie::DecidedNull || (t == Tie::None && s == State::Free));
            }
        }
    }
}
