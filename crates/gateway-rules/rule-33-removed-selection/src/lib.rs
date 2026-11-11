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

//! Gateway rule 33 (card DETACH-RESULT-WRAPPER-COMPOSITION; package card
//! **RULE33-PILOT**): the checked removed-selection composition as a rule
//! package of the gateway rule interface.
//!
//! A *selection detach* `F` is a function whose complete body is exactly
//! (after renaming and the detach and lookup readers' NULL normalization)
//! `{ if (W < 0) { return NULL; } return D(P, L(P, (size_t)W)); }`; a
//! *selection release* `G` is exactly `{ X(F(P, W)); }`. A selection
//! wrapper's selected child is `sel(P, W) = (W < 0) ? NULL : L(P,
//! (size_t)W)`, the value its exactly checked frame computes. The child is a
//! **prerequisite** the caller transports: membership is never derived from
//! borrowed provenance or from the detach representation premise.
//!
//! What lives here, none of it reading the gateway's vocabulary:
//!
//! * the frame bodies' word shapes ([`detach_body`], [`release_body`]), which
//!   the host asks before it checks the frame's contracts, record and
//!   linkage, and the [`Frame`] it then builds;
//! * the caller expansion ([`RemovedSelection`] as a
//!   [`StatementExpansion`]): a frame's own body, and every caller statement
//!   `d = F(q, K);`, `R *d = F(q, K);`, `struct T *d = F(q, K);`, `F(q, K);`
//!   or `G(q, K);` with a plain parent name and a decimal constant `K` in
//!   0–255, become the frame's constituent calls;
//! * the selection entry's decisions ([`materialize`], [`accept_exit`],
//!   [`validates`]) and the frame row's admission ([`RemovedSelection`] as an
//!   [`EntryRow`]).
//!
//! The host (`legacy_selection.rs` in `qcore-c-gateway`) reads the frame,
//! builds the selection entry, analyses the row and asks this package under
//! `QCORE_GATEWAY_RULES_OFF`. Switched off, nothing is expanded and no frame
//! is installed, which is the gateway before rule 33. `QCore.RemovedSelection`
//! proves the finite composition and binds the decision tables below by
//! shared vectors; `QCore.RemovedSelectionSwitch` states the switch. Source
//! extraction and Rust/Lean correspondence remain boundary-28 premises.

pub mod selection_route;

use std::collections::{BTreeMap, BTreeSet};

use qcore_gateway_rule_api::{
    Answer, BodyQuery, EntryRow, Expanded, StatementExpansion, traits::RulePackage,
};

/// The registry id.
pub const ID: &str = "33";
/// The reserved prefix of every place an expansion introduces.
pub const RESERVED: &str = "__qcore_selection_";
/// The frame's own lookup destination, read by the selection entry.
pub const ITEM_PLACE: &str = "__qcore_selection_item";
/// The frame's own detach destination.
pub const RESULT_PLACE: &str = "__qcore_selection_result";
/// A frame body is short: anything longer is refused before any other read.
pub const FRAME_SPAN: usize = 64;

/// A checked selection frame, built by the host after its own checks (the
/// detach's same-item declaration and premise, the lookup's borrowed summary
/// and complete indexed frame, the release's nullable mode, the owner's
/// record and `size_t`, and the definition's linkage).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    pub disposition: Disposition,
    pub owner: String,
    pub index: String,
    /// The owner's declared pointee spelling (`R` of `R *P`).
    pub pointee: Vec<String>,
    pub record: String,
    pub lookup: String,
    pub detach: String,
    /// `G` only: the release and the selection detach it composes.
    pub release: Option<String>,
    pub inner: Option<String>,
}

/// The gateway's lexical identifier shape (`legacy_token_shapes::is_identifier`):
/// the first byte is a letter or `_`.
fn identifier(text: &str) -> bool {
    text.as_bytes()
        .first()
        .is_some_and(|byte| byte.is_ascii_alphabetic() || *byte == b'_')
}

/// The NULL spellings the detach and lookup readers already normalize.
#[must_use]
pub fn nulls(mut words: Vec<String>) -> Vec<String> {
    loop {
        let mut change = None;
        for at in 0..words.len() {
            let tail = &words[at..];
            if tail
                .iter()
                .map(String::as_str)
                .take(5)
                .eq(["(", "void", "*", ")", "0"])
            {
                change = Some((at, 5));
                break;
            }
            if tail.iter().map(String::as_str).take(3).eq(["(", "0", ")"]) {
                change = Some((at, 3));
                break;
            }
        }
        let Some((at, n)) = change else {
            return words;
        };
        words.splice(at..at + n, ["0".to_owned()]);
    }
}

fn normalized(body: &[&str]) -> Vec<String> {
    nulls(body.iter().map(|word| (*word).to_owned()).collect())
}

/// `F`'s body shape, `{` through `}`, for the formals `owner` and `index`:
/// the detach and the lookup it names.
#[must_use]
pub fn detach_body(
    body: &[&str],
    owner: &str,
    index: &str,
    function: &str,
) -> Option<(String, String)> {
    let body = normalized(body);
    if body.len() != 29 {
        return None;
    }
    let detach = body.get(13)?.clone();
    let lookup = body.get(17)?.clone();
    let expected = [
        "{", "if", "(", index, "<", "0", ")", "{", "return", "0", ";", "}", "return", &detach, "(",
        owner, ",", &lookup, "(", owner, ",", "(", "size_t", ")", index, ")", ")", ";", "}",
    ];
    (body.iter().map(String::as_str).eq(expected)
        && identifier(&detach)
        && identifier(&lookup)
        && ![owner, index, function].contains(&detach.as_str())
        && ![owner, index, function, detach.as_str()].contains(&lookup.as_str()))
    .then_some((detach, lookup))
}

/// `G`'s body shape, `{` through `}`, for the formals `owner` and `index`:
/// the release and the selection detach it composes.
#[must_use]
pub fn release_body(
    body: &[&str],
    owner: &str,
    index: &str,
    function: &str,
) -> Option<(String, String)> {
    let body = normalized(body);
    if body.len() != 12 {
        return None;
    }
    let release = body.get(1)?.clone();
    let inner = body.get(3)?.clone();
    let expected = [
        "{", &release, "(", &inner, "(", owner, ",", index, ")", ")", ";", "}",
    ];
    (body.iter().map(String::as_str).eq(expected)
        && identifier(&release)
        && identifier(&inner)
        && release != inner
        && ![owner, index, function].contains(&release.as_str())
        && ![owner, index, function].contains(&inner.as_str()))
    .then_some((release, inner))
}

/// A frame's own body (`{` through `}`) as its constituent calls, each word
/// anchored to the body word whose line it takes. `F` keeps everything before
/// its second `return` verbatim (the guard); `G` reads the guard as a branch.
#[must_use]
pub fn own_body(body: &[&str], frame: &Frame) -> Option<Vec<(String, usize)>> {
    let last = body.len().checked_sub(1)?;
    let pointee = frame.pointee.iter().map(String::as_str).collect::<Vec<_>>();
    let (owner, index) = (frame.owner.as_str(), frame.index.as_str());
    let (lookup, detach) = (frame.lookup.as_str(), frame.detach.as_str());
    let mut texts: Vec<&str> = Vec::new();
    match frame.disposition {
        Disposition::Returned => {
            let second = body[..last]
                .iter()
                .enumerate()
                .filter(|(_, word)| **word == "return")
                .nth(1)
                .map(|(at, _)| at)?;
            let mut out = body[..second]
                .iter()
                .enumerate()
                .map(|(at, word)| ((*word).to_owned(), at))
                .collect::<Vec<_>>();
            texts.extend(pointee.iter().copied());
            texts.extend([
                "*", ITEM_PLACE, "=", lookup, "(", owner, ",", "(", "size_t", ")",
            ]);
            texts.extend([index, ")", ";"]);
            texts.extend(pointee.iter().copied());
            texts.extend([
                "*",
                RESULT_PLACE,
                "=",
                detach,
                "(",
                owner,
                ",",
                ITEM_PLACE,
                ")",
                ";",
            ]);
            texts.extend(["return", RESULT_PLACE, ";", "}"]);
            out.extend(texts.iter().map(|text| ((*text).to_owned(), second)));
            Some(out)
        }
        Disposition::Released => {
            body.get(1)?;
            let release = frame.release.as_deref()?;
            texts.push("{");
            for place in [RESULT_PLACE, ITEM_PLACE] {
                texts.extend(pointee.iter().copied());
                texts.extend(["*", place, "=", "(", "(", "void", "*", ")", "0", ")", ";"]);
            }
            texts.extend(["if", "(", index, "<", "0", ")", "{", "}", "else", "{"]);
            texts.extend([
                ITEM_PLACE, "=", lookup, "(", owner, ",", "(", "size_t", ")", index,
            ]);
            texts.extend([
                ")",
                ";",
                RESULT_PLACE,
                "=",
                detach,
                "(",
                owner,
                ",",
                ITEM_PLACE,
                ")",
            ]);
            texts.extend([";", "}", release, "(", RESULT_PLACE, ")", ";", "}"]);
            Some(texts.iter().map(|text| ((*text).to_owned(), 1)).collect())
        }
    }
}

/// A caller statement's destination part: nothing (`F(q, K);`), an
/// assignment `d =`, or a one-declarator pointer declaration `R *d =` /
/// `struct T *d =`. Answers its length, the destination and the declaration
/// spelling kept in the enclosing scope.
fn statement_prefix<'a>(rest: &[&'a str]) -> (usize, Option<&'a str>, Vec<&'a str>) {
    match rest {
        [name, "=", ..] if identifier(name) => (2, Some(*name), Vec::new()),
        [ty, "*", name, "=", ..] if identifier(ty) && identifier(name) => {
            (4, Some(*name), vec![*ty, "*", *name, ";"])
        }
        ["struct", tag, "*", name, "=", ..] if identifier(tag) && identifier(name) => {
            (5, Some(*name), vec!["struct", *tag, "*", *name, ";"])
        }
        _ => (0, None, Vec::new()),
    }
}

/// A decimal constant in 0–255 with no leading zero.
#[must_use]
pub fn decimal(text: &str) -> bool {
    !text.is_empty()
        && text.bytes().all(|b| b.is_ascii_digit())
        && (text == "0" || !text.starts_with('0'))
        && text.parse::<usize>().is_ok_and(|value| value <= 255)
}

/// The callee of a caller statement shape at `at` of the words between a
/// body's braces, before any frame read.
#[must_use]
pub fn statement_callee<'a>(body: &[&'a str], at: usize) -> Option<&'a str> {
    // The statement starts after `;`, `{` or `}`.
    if at > 0 && !matches!(body[at - 1], ";" | "{" | "}") {
        return None;
    }
    let rest = &body[at..];
    let (prefix, ..) = statement_prefix(rest);
    match &rest[prefix..] {
        [callee, "(", parent, ",", constant, ")", ";", ..]
            if identifier(callee) && identifier(parent) && decimal(constant) =>
        {
            Some(*callee)
        }
        _ => None,
    }
}

/// One caller statement, `[R *] d = F(q, K);`, `F(q, K);` or `G(q, K);`: the
/// words it spans and its constituent calls.
fn caller_statement(
    body: &[&str],
    at: usize,
    frames: &BTreeMap<String, Frame>,
    serial: usize,
) -> Option<(usize, Vec<String>)> {
    let callee = statement_callee(body, at)?;
    let frame = frames.get(callee)?;
    let rest = &body[at..];
    let (prefix, destination, declaration) = statement_prefix(rest);
    let [_, "(", parent, ",", constant, ")", ";", ..] = &rest[prefix..] else {
        return None;
    };
    if destination.is_some() && frame.disposition == Disposition::Released {
        return None;
    }
    let item = format!("{ITEM_PLACE}_{serial}");
    let result = format!("{RESULT_PLACE}_{serial}");
    let pointee = frame.pointee.iter().map(String::as_str).collect::<Vec<_>>();
    // A declaration keeps its name in the enclosing scope.
    let mut texts: Vec<&str> = declaration;
    texts.push("{");
    texts.extend(pointee.iter().copied());
    texts.extend([
        "*",
        &item,
        "=",
        &frame.lookup,
        "(",
        parent,
        ",",
        constant,
        ")",
        ";",
    ]);
    let destination = match destination {
        Some(destination) => destination,
        None => {
            texts.extend(pointee.iter().copied());
            texts.push("*");
            &result
        }
    };
    texts.extend([
        destination,
        "=",
        &frame.detach,
        "(",
        parent,
        ",",
        &item,
        ")",
        ";",
    ]);
    if let Some(release) = frame.release.as_deref() {
        texts.extend([release, "(", &result, ")", ";"]);
    }
    texts.push("}");
    Some((
        prefix + 7,
        texts.iter().map(|text| (*text).to_owned()).collect(),
    ))
}

/// The caller expansion of a body (`{` through `}`) under the checked
/// `frames`, or `None` when no statement expands.
#[must_use]
pub fn expand_callers(
    body: &[&str],
    frames: &BTreeMap<String, Frame>,
) -> Option<Vec<(String, usize)>> {
    let last = body.len().checked_sub(1)?;
    if last == 0 {
        return None;
    }
    let inner = &body[1..last];
    let mut out = vec![(body[0].to_owned(), 0)];
    let mut at = 0;
    let mut serial = 0;
    let mut changed = false;
    while at < inner.len() {
        if let Some((length, replacement)) = caller_statement(inner, at, frames, serial) {
            out.extend(replacement.into_iter().map(|word| (word, 1 + at)));
            at += length;
            serial += 1;
            changed = true;
        } else {
            out.push((inner[at].to_owned(), 1 + at));
            at += 1;
        }
    }
    out.push((body[last].to_owned(), last));
    changed.then_some(out)
}

/// What the host observed of a frame row's run under its selection entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidationRun {
    pub disposition: Disposition,
    /// The run is `Safe` and diagnostic-free.
    pub clean: bool,
    /// Every reachable exit.
    pub exits: Vec<ExitFacts>,
}

/// Rule 33's package.
#[derive(Clone, Copy, Debug, Default)]
pub struct RemovedSelection;

impl RulePackage for RemovedSelection {
    fn id(&self) -> &'static str {
        ID
    }
}

impl StatementExpansion for RemovedSelection {
    type Frame = Frame;

    fn reserved_prefix(&self) -> &'static str {
        RESERVED
    }

    fn candidates<'b>(&self, function: &str, body: &[&'b str]) -> BTreeSet<&'b str> {
        let Some(last) = body.len().checked_sub(1).filter(|last| *last > 0) else {
            return BTreeSet::new();
        };
        let inner = &body[1..last];
        (0..inner.len())
            .filter_map(|at| statement_callee(inner, at))
            .filter(|callee| *callee != function)
            .collect()
    }

    /// A frame's own body becomes its constituent calls; otherwise every
    /// decimal-constant caller statement of a checked frame does. Refuses when
    /// nothing changes.
    fn ask(&self, query: &BodyQuery<'_, Frame>) -> Answer<Expanded> {
        let words = match query.own {
            Some(frame) => own_body(query.body, frame),
            None => expand_callers(query.body, query.callees),
        };
        Answer::from_option(words.map(|words| Expanded { words }))
    }
}

impl EntryRow for RemovedSelection {
    type Run = ValidationRun;

    /// The run validates: clean, selecting on some path and accepting every
    /// exit.
    fn ask(&self, run: &ValidationRun) -> Answer<()> {
        let selected_somewhere = run
            .exits
            .iter()
            .any(|exit| exit.selected != Selected::Absent);
        Answer::from_option(
            validates(run.disposition, run.clean, &run.exits, selected_somewhere).then_some(()),
        )
    }
}

/// Facts at the one checked selection lookup point of a wrapper analysed
/// under its selection entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MaterializeFacts {
    /// The node is the frame's own lookup call into the reserved item place,
    /// whose parent actual is the entry's owner formal.
    pub selection_node: bool,
    /// The owner binding is one settled, live, non-NULL identity.
    pub parent_live_identity: bool,
    /// No call or write precedes the lookup on any path from entry.
    pub pristine: bool,
    /// The selected child has not already appeared on this path.
    pub fresh: bool,
    /// The ordinary call left its borrowed-from result binding in place and
    /// the path is modeled.
    pub borrowed_result: bool,
}

/// The selected child appears only when every fact holds.
#[must_use]
pub const fn materialize(f: MaterializeFacts) -> bool {
    f.selection_node && f.parent_live_identity && f.pristine && f.fresh && f.borrowed_result
}

/// What the wrapper hands back to its caller.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Disposition {
    /// `F`: the selected child is returned, detached.
    Returned,
    /// `G`: the selected child is released, detached.
    Released,
}

/// The selected child's custody at one exit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Selected {
    /// Never selected on any path reaching this exit.
    Absent,
    Contained,
    Live,
    Escaped,
    Released,
    Mixed,
}

/// What one exit returns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Returned {
    /// No resource place, or a null constant.
    Nothing,
    /// Exactly the selected child (possibly NULL).
    Selected,
    /// Anything else.
    Other,
}

/// One reachable exit of a validation run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExitFacts {
    /// The owner formal is still live and no other formal custody moved.
    pub parent_live: bool,
    /// No caller-held containment, out slot or pending replacement remains.
    pub frame_clear: bool,
    pub selected: Selected,
    pub returned: Returned,
}

/// An exit agrees with the summary. The absent-selection exit relies on the
/// exact frame: it is reached only by the `W < 0` arm, where `sel` is NULL.
#[must_use]
pub const fn accept_exit(disposition: Disposition, f: ExitFacts) -> bool {
    if !f.parent_live || !f.frame_clear {
        return false;
    }
    match disposition {
        Disposition::Returned => matches!(
            (f.selected, f.returned),
            (Selected::Absent, Returned::Nothing) | (Selected::Escaped, Returned::Selected)
        ),
        Disposition::Released => {
            matches!(f.selected, Selected::Absent | Selected::Released)
                && matches!(f.returned, Returned::Nothing)
        }
    }
}

/// A run validates only when it is fully modeled, diagnostic-free, reaches
/// an exit, selects on some path and accepts every exit.
#[must_use]
pub fn validates(
    disposition: Disposition,
    clean: bool,
    exits: &[ExitFacts],
    selected_somewhere: bool,
) -> bool {
    clean
        && selected_somewhere
        && !exits.is_empty()
        && exits.iter().all(|exit| accept_exit(disposition, *exit))
}

/// The selection value the frame computes, over a finite child list.
#[must_use]
pub fn selection(children: &[usize], which: i64) -> Option<usize> {
    if which < 0 {
        return None;
    }
    usize::try_from(which)
        .ok()
        .and_then(|index| children.get(index).copied())
}

/// Shared with `QCore.RemovedSelection.materializeVectors`.
pub const MATERIALIZE_VECTORS: [(MaterializeFacts, bool); 6] = [
    (
        MaterializeFacts {
            selection_node: true,
            parent_live_identity: true,
            pristine: true,
            fresh: true,
            borrowed_result: true,
        },
        true,
    ),
    (
        MaterializeFacts {
            selection_node: false,
            parent_live_identity: true,
            pristine: true,
            fresh: true,
            borrowed_result: true,
        },
        false,
    ),
    (
        MaterializeFacts {
            selection_node: true,
            parent_live_identity: false,
            pristine: true,
            fresh: true,
            borrowed_result: true,
        },
        false,
    ),
    (
        MaterializeFacts {
            selection_node: true,
            parent_live_identity: true,
            pristine: false,
            fresh: true,
            borrowed_result: true,
        },
        false,
    ),
    (
        MaterializeFacts {
            selection_node: true,
            parent_live_identity: true,
            pristine: true,
            fresh: false,
            borrowed_result: true,
        },
        false,
    ),
    (
        MaterializeFacts {
            selection_node: true,
            parent_live_identity: true,
            pristine: true,
            fresh: true,
            borrowed_result: false,
        },
        false,
    ),
];

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::*;

    fn lean() -> &'static str {
        include_str!("../../../../proof/QCore/RemovedSelection.lean")
    }

    fn table<'a>(source: &'a str, head: &str) -> Vec<&'a str> {
        source
            .split(head)
            .nth(1)
            .expect("vector table")
            .split("\n]")
            .next()
            .unwrap()
            .lines()
            .map(str::trim)
            .filter(|line| line.starts_with('('))
            .collect()
    }

    fn bools(text: &str) -> Vec<bool> {
        text.split(',')
            .map(|value| match value.trim() {
                "true" => true,
                "false" => false,
                other => panic!("{other}"),
            })
            .collect()
    }

    #[test]
    fn lean_materialize_vectors_match_the_rust_decision() {
        let rows = table(
            lean(),
            "def materializeVectors : List (MaterializeFacts × Bool) := [",
        );
        assert_eq!(rows.len(), MATERIALIZE_VECTORS.len());
        for (row, (facts, expected)) in rows.iter().zip(MATERIALIZE_VECTORS) {
            let (inside, result) = row.trim_start_matches("(⟨").split_once("⟩,").unwrap();
            let values = bools(inside);
            assert_eq!(
                values,
                [
                    facts.selection_node,
                    facts.parent_live_identity,
                    facts.pristine,
                    facts.fresh,
                    facts.borrowed_result
                ]
            );
            let result = result.trim().trim_end_matches(',').trim_end_matches(')');
            assert_eq!(result == "true", expected);
            assert_eq!(materialize(facts), expected);
        }
    }

    #[test]
    fn lean_exit_vectors_match_the_rust_decision() {
        let rows = table(
            lean(),
            "def exitVectors : List (Disposition × ExitFacts × Bool) := [",
        );
        assert_eq!(rows.len(), 12);
        for row in rows {
            let body = row.trim_start_matches('(');
            let (disposition, rest) = body.split_once(", ⟨").unwrap();
            let (inside, result) = rest.split_once("⟩,").unwrap();
            let disposition = match disposition.trim() {
                ".returned" => Disposition::Returned,
                ".released" => Disposition::Released,
                other => panic!("{other}"),
            };
            let parts = inside.split(',').map(str::trim).collect::<Vec<_>>();
            let facts = ExitFacts {
                parent_live: parts[0] == "true",
                frame_clear: parts[1] == "true",
                selected: match parts[2] {
                    ".absent" => Selected::Absent,
                    ".contained" => Selected::Contained,
                    ".live" => Selected::Live,
                    ".escaped" => Selected::Escaped,
                    ".released" => Selected::Released,
                    ".mixed" => Selected::Mixed,
                    other => panic!("{other}"),
                },
                returned: match parts[3] {
                    ".nothing" => Returned::Nothing,
                    ".selected" => Returned::Selected,
                    ".other" => Returned::Other,
                    other => panic!("{other}"),
                },
            };
            let result = result.trim().trim_end_matches(',').trim_end_matches(')');
            assert_eq!(accept_exit(disposition, facts), result == "true", "{row}");
        }
    }

    #[test]
    fn lean_selection_vectors_match_the_rust_model() {
        let rows = table(
            lean(),
            "def selectionVectors : List (List Nat × Int × Option Nat) := [",
        );
        assert_eq!(rows.len(), 8);
        for row in rows {
            let (list, rest) = row.trim_start_matches('(').split_once("],").unwrap();
            let children = list
                .trim_start_matches('[')
                .split(',')
                .map(str::trim)
                .filter(|x| !x.is_empty())
                .map(|x| x.parse::<usize>().unwrap())
                .collect::<Vec<_>>();
            let (which, result) = rest.split_once(',').unwrap();
            let which = which.trim().trim_matches(|c| c == '(' || c == ')');
            let which = which.parse::<i64>().unwrap();
            let result = result
                .trim()
                .trim_end_matches(',')
                .trim_end_matches(')')
                .trim();
            let expected = (result != "none").then(|| {
                result
                    .strip_prefix("some ")
                    .unwrap()
                    .parse::<usize>()
                    .unwrap()
            });
            assert_eq!(selection(&children, which), expected, "{row}");
        }
    }

    #[test]
    fn every_selected_child_is_an_existing_member_and_negative_selects_nothing() {
        for len in 0..7 {
            let children = (20..20 + len).collect::<Vec<_>>();
            for which in -3..9 {
                match selection(&children, which) {
                    Some(child) => {
                        assert!(which >= 0 && children.contains(&child));
                        assert_eq!(Some(child), children.get(which as usize).copied());
                    }
                    None => assert!(which < 0 || which as usize >= children.len()),
                }
            }
        }
    }

    #[test]
    fn validation_needs_a_clean_selecting_run_and_every_exit() {
        let good = ExitFacts {
            parent_live: true,
            frame_clear: true,
            selected: Selected::Escaped,
            returned: Returned::Selected,
        };
        let early = ExitFacts {
            selected: Selected::Absent,
            returned: Returned::Nothing,
            ..good
        };
        assert!(validates(Disposition::Returned, true, &[early, good], true));
        assert!(!validates(
            Disposition::Returned,
            false,
            &[early, good],
            true
        ));
        assert!(!validates(Disposition::Returned, true, &[early], false));
        assert!(!validates(Disposition::Returned, true, &[], true));
        let kept = ExitFacts {
            selected: Selected::Contained,
            returned: Returned::Nothing,
            ..good
        };
        assert!(!validates(Disposition::Returned, true, &[kept, good], true));
        assert!(!validates(Disposition::Released, true, &[good], true));
    }

    fn frame(disposition: Disposition) -> Frame {
        Frame {
            disposition,
            owner: "array".into(),
            index: "which".into(),
            pointee: vec!["cJSON".into()],
            record: "cJSON".into(),
            lookup: "get_array_item".into(),
            detach: "cJSON_DetachItemViaPointer".into(),
            release: (disposition == Disposition::Released).then(|| "cJSON_Delete".into()),
            inner: (disposition == Disposition::Released)
                .then(|| "cJSON_DetachItemFromArray".into()),
        }
    }

    fn split(text: &str) -> Vec<&str> {
        text.split_whitespace().collect()
    }

    #[test]
    fn the_frame_shapes_are_exact() {
        let detach = split(
            "{ if ( which < 0 ) { return ( ( void * ) 0 ) ; } return \
             cJSON_DetachItemViaPointer ( array , get_array_item ( array , ( size_t ) which ) ) ; }",
        );
        assert_eq!(
            detach_body(&detach, "array", "which", "F"),
            Some((
                "cJSON_DetachItemViaPointer".to_owned(),
                "get_array_item".to_owned()
            ))
        );
        let mut changed = detach.clone();
        changed[5] = "1";
        assert_eq!(detach_body(&changed, "array", "which", "F"), None);
        assert_eq!(
            detach_body(&detach, "array", "which", "get_array_item"),
            None
        );
        let release = split("{ cJSON_Delete ( cJSON_DetachItemFromArray ( array , which ) ) ; }");
        assert_eq!(
            release_body(&release, "array", "which", "G"),
            Some((
                "cJSON_Delete".to_owned(),
                "cJSON_DetachItemFromArray".to_owned()
            ))
        );
        let same = split("{ f ( f ( array , which ) ) ; }");
        assert_eq!(release_body(&same, "array", "which", "G"), None);
    }

    #[test]
    fn only_decimal_constant_statements_of_checked_frames_expand() {
        let frames = BTreeMap::from([
            (
                "cJSON_DetachItemFromArray".to_owned(),
                frame(Disposition::Returned),
            ),
            (
                "cJSON_DeleteItemFromArray".to_owned(),
                frame(Disposition::Released),
            ),
        ]);
        let body = split(
            "{ x ( ) ; cJSON * r = cJSON_DetachItemFromArray ( p , 0 ) ; \
             cJSON_DeleteItemFromArray ( p , 255 ) ; }",
        );
        let expanded = expand_callers(&body, &frames).expect("expands");
        let words = expanded.iter().map(|(w, _)| w.as_str()).collect::<Vec<_>>();
        let expected = "{ x ( ) ; cJSON * r ; { cJSON * __qcore_selection_item_0 = \
                        get_array_item ( p , 0 ) ; r = cJSON_DetachItemViaPointer ( p , \
                        __qcore_selection_item_0 ) ; } { cJSON * __qcore_selection_item_1 = \
                        get_array_item ( p , 255 ) ; cJSON * __qcore_selection_result_1 = \
                        cJSON_DetachItemViaPointer ( p , __qcore_selection_item_1 ) ; \
                        cJSON_Delete ( __qcore_selection_result_1 ) ; } }";
        assert_eq!(words, split(expected));
        assert!(expanded.iter().all(|(_, anchor)| *anchor < body.len()));
        // The verbatim words keep their own lines; a statement's expansion
        // takes the line of its first word.
        assert_eq!(expanded[0], ("{".to_owned(), 0));
        assert_eq!(expanded[5], ("cJSON".to_owned(), 5));
        assert_eq!(expanded.last(), Some(&("}".to_owned(), body.len() - 1)));
        for refused in [
            "{ cJSON_DeleteItemFromArray ( p , k ) ; }",
            "{ cJSON_DeleteItemFromArray ( p , 256 ) ; }",
            "{ cJSON_DeleteItemFromArray ( p , 00 ) ; }",
            "{ r = cJSON_DeleteItemFromArray ( p , 0 ) ; }",
            "{ f ( cJSON_DetachItemFromArray ( p , 0 ) ) ; }",
            "{ unchecked ( p , 0 ) ; }",
        ] {
            assert_eq!(expand_callers(&split(refused), &frames), None, "{refused}");
        }
        assert_eq!(
            RemovedSelection.candidates(
                "cJSON_DetachItemFromArray",
                &split("{ cJSON_DetachItemFromArray ( p , 0 ) ; g ( p , 1 ) ; }")
            ),
            BTreeSet::from(["g"])
        );
    }

    #[test]
    fn the_package_expands_own_bodies_and_admits_validated_runs_only() {
        let package = RemovedSelection;
        assert_eq!(package.id(), "33");
        let body = split(
            "{ if ( which < 0 ) { return 0 ; } return cJSON_DetachItemViaPointer ( array , \
             get_array_item ( array , ( size_t ) which ) ) ; }",
        );
        let own = frame(Disposition::Returned);
        let none = BTreeMap::new();
        let query = BodyQuery {
            function: "cJSON_DetachItemFromArray",
            body: &body,
            own: Some(&own),
            callees: &none,
        };
        let Answer::Admit(admission) = StatementExpansion::ask(&package, &query) else {
            panic!("own body expands");
        };
        let words = admission.lowering.words;
        let guard = body[..12]
            .iter()
            .enumerate()
            .map(|(at, word)| ((*word).to_owned(), at))
            .collect::<Vec<_>>();
        assert_eq!(&words[..12], &guard[..]);
        assert!(words[12..].iter().all(|(_, anchor)| *anchor == 12));
        assert_eq!(words.last().map(|(w, _)| w.as_str()), Some("}"));
        let caller = BodyQuery {
            function: "subject",
            own: None,
            ..query
        };
        assert_eq!(StatementExpansion::ask(&package, &caller), Answer::Refuse);
        let good = ExitFacts {
            parent_live: true,
            frame_clear: true,
            selected: Selected::Escaped,
            returned: Returned::Selected,
        };
        let run = |clean, exits: Vec<ExitFacts>| ValidationRun {
            disposition: Disposition::Returned,
            clean,
            exits,
        };
        assert_eq!(
            EntryRow::ask(&package, &run(true, vec![good])),
            Answer::admit(())
        );
        assert_eq!(
            EntryRow::ask(&package, &run(false, vec![good])),
            Answer::Refuse
        );
        assert_eq!(EntryRow::ask(&package, &run(true, vec![])), Answer::Refuse);
        let absent = ExitFacts {
            selected: Selected::Absent,
            returned: Returned::Nothing,
            ..good
        };
        assert_eq!(
            EntryRow::ask(&package, &run(true, vec![absent])),
            Answer::Refuse,
            "a run that never selects does not validate"
        );
    }

    /// Plan seam B: a rule package depends on the rule interface only.
    #[test]
    fn the_package_depends_on_the_rule_interface_only() {
        let manifest = include_str!("../Cargo.toml");
        let mut section = "";
        let mut dependencies = Vec::new();
        for line in manifest.lines() {
            let line = line.trim();
            if line.starts_with('[') {
                section = line;
                continue;
            }
            if section.contains("dependencies") && !line.is_empty() && !line.starts_with('#') {
                dependencies.push(line.split('=').next().unwrap_or("").trim().to_owned());
            }
        }
        assert_eq!(dependencies, ["qcore-gateway-rule-api"]);
    }
}
