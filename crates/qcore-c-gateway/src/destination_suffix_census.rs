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

//! Card **W44-ASSIGNMENT-DESTINATION-SUFFIX-PLACE**: the instrumented census
//! of assignment destinations named by a strict suffix of the left-hand side.
//!
//! A measurement, not a rule, built exactly like card W44-INDIRECT-CHAIN-ACCESS's
//! member-chain stream: the whole module and its one call site are behind the
//! off-by-default `indirect-census` feature, every record is written **after**
//! the classifier has decided, and nothing reads the stream but the card's
//! scripts. With the feature on, the pinned corpus audit must still reproduce
//! its artifacts byte for byte.
//!
//! One JSON line, to the path named by [`DESTINATION_SUFFIX_CENSUS_PATH`], per
//! statement or `for` clause whose left-hand side (the tokens before its first
//! `=`, exactly the slice the classifier hands `direct_assignment_destination`)
//! is not headed by `*` and whose first place-spelling suffix
//! ([`crate::legacy_c::first_suffix_place`]) starts **after** index 0. Each
//! record carries:
//!
//! * the prefix before that suffix and the answer of
//!   [`crate::legacy_c::is_declarator_prefix`], the predicate the repair uses;
//! * the classifier's outcome for the statement (the instruction's variant, a
//!   refusal's description) and whether it was admitted;
//! * for a prefix that is **not** a declarator prefix — the misnamed stores —
//!   what the named suffix's root is in this function (a formal, a local, a
//!   file-scope object, or nothing declared), how often a later line of the
//!   body mentions it as a whole name, and whether a later line passes it as a
//!   whole call argument; and the answers of the typed routes the repaired
//!   classifier consults after its refusal, so the census forecasts the rule.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::sync::Mutex;

/// The environment variable naming the destination-suffix census stream.
pub const DESTINATION_SUFFIX_CENSUS_PATH: &str = "QCORE_DESTINATION_SUFFIX_CENSUS";

static SINK: Mutex<Option<Option<BufWriter<File>>>> = Mutex::new(None);

/// What a misnamed suffix's root is in the enclosing function.
#[derive(Clone, Debug, serde::Serialize)]
pub struct NamedRoot {
    pub root: String,
    /// `formal`, `local` (a declaration of this body), `file-scope`, or
    /// `undeclared`.
    pub kind: &'static str,
    /// Whole-name mentions (not after `.` or `->`) on strictly later lines of
    /// the body.
    pub later_mentions: usize,
    /// Whether one of those mentions is a whole call argument
    /// (`f ( name )`, `f ( a , name )`).
    pub later_call_argument: bool,
}

/// The routes that can still admit the statement once the reader stops naming
/// it: the two typed routes the repaired classifier consults before the lexical
/// gate, which read no destination (`scalar_field_noop`, `scalar_member_store`),
/// and the typed routes it consults after the gate refusal.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Forecast {
    pub contains_call: bool,
    pub scalar_field_noop: bool,
    pub scalar_member_store: bool,
    pub typed_scalar: bool,
    pub chain: bool,
    pub stepped: bool,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct SuffixRecord {
    pub unit_seq: u64,
    pub function: String,
    pub line: usize,
    /// `statement`, `for-init` or `for-increment`.
    pub clause: &'static str,
    pub texts: Vec<String>,
    /// Index of the first `=`.
    pub equal: usize,
    /// Index the named suffix starts at (always > 0 here).
    pub start: usize,
    pub place: String,
    pub prefix: Vec<String>,
    pub declarator_prefix: bool,
    /// The instruction's variant name.
    pub outcome: String,
    /// A refusal's description.
    pub description: Option<String>,
    pub admitted: bool,
    /// Present only when the prefix is not a declarator prefix.
    pub named_root: Option<NamedRoot>,
    /// Present only when the prefix is not a declarator prefix.
    pub forecast: Option<Forecast>,
}

fn emit(record: &SuffixRecord) {
    let Ok(mut guard) = SINK.lock() else {
        return;
    };
    let opened = guard.get_or_insert_with(|| {
        let path = std::env::var_os(DESTINATION_SUFFIX_CENSUS_PATH)?;
        File::create(path).ok().map(BufWriter::new)
    });
    let Some(writer) = opened.as_mut() else {
        return;
    };
    if let Ok(line) = serde_json::to_string(record) {
        let _ = writeln!(writer, "{line}");
        let _ = writer.flush();
    }
}

/// Whether the stream is being collected.
pub(crate) fn collecting() -> bool {
    std::env::var_os(DESTINATION_SUFFIX_CENSUS_PATH).is_some()
}

/// The strict-suffix place of `texts`' left-hand side, if it has one:
/// `(equal, start, place)`.
pub(crate) fn strict_suffix(texts: &[&str]) -> Option<(usize, usize, String)> {
    let equal = texts.iter().position(|text| *text == "=")?;
    let left = &texts[..equal];
    if left.first() == Some(&"*") {
        return None;
    }
    let (start, place) = crate::legacy_c::first_suffix_place(left)?;
    (start > 0).then_some((equal, start, place))
}

/// The root identifier of a canonical place (`name`, `name.f`, `name->f`,
/// `name[0]`).
pub(crate) fn place_root(place: &str) -> &str {
    let end = place.find(['.', '-', '[']).unwrap_or(place.len());
    &place[..end]
}

/// Whole-name mentions of `root` in `body` on lines after `line`, and whether
/// one of them is a whole call argument.
pub(crate) fn later_mentions(body: &[(&str, usize)], root: &str, line: usize) -> (usize, bool) {
    let mut mentions = 0usize;
    let mut argument = false;
    for (index, (text, at)) in body.iter().enumerate() {
        if *text != root || *at <= line {
            continue;
        }
        let previous = index.checked_sub(1).map(|at| body[at].0);
        let member = previous == Some(".")
            || (previous == Some(">") && index >= 2 && body[index - 2].0 == "-");
        if member {
            continue;
        }
        mentions += 1;
        let next = body.get(index + 1).map(|(text, _)| *text);
        if matches!(previous, Some("(") | Some(",")) && matches!(next, Some(")") | Some(",")) {
            argument = true;
        }
    }
    (mentions, argument)
}

/// Record one classified statement or `for` clause.
pub(crate) fn record(record: SuffixRecord) {
    emit(&record);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_member_store_has_a_strict_suffix_and_a_declaration_is_read_by_the_prefix() {
        let store = ["recs", "[", "i", "]", ".", "name", "=", "x"];
        assert_eq!(strict_suffix(&store), Some((6, 5, "name".to_owned())));
        assert!(!crate::legacy_c::is_declarator_prefix(&store[..5]));
        let declaration = ["struct", "t", "*", "p", "=", "x"];
        assert_eq!(strict_suffix(&declaration), Some((4, 3, "p".to_owned())));
        assert!(crate::legacy_c::is_declarator_prefix(&declaration[..3]));
        assert_eq!(strict_suffix(&["p", "-", ">", "f", "=", "x"]), None);
        assert_eq!(strict_suffix(&["*", "out", "=", "x"]), None);
    }

    #[test]
    fn roots_and_later_mentions_are_read_structurally() {
        assert_eq!(place_root("name"), "name");
        assert_eq!(place_root("a->b"), "a");
        assert_eq!(place_root("a.b"), "a");
        assert_eq!(place_root("a[0]"), "a");
        let body = [
            ("x_free", 3),
            ("(", 3),
            ("name", 3),
            (")", 3),
            ("r", 4),
            (".", 4),
            ("name", 4),
            ("x_free", 5),
            ("(", 5),
            ("name", 5),
            (")", 5),
        ];
        assert_eq!(later_mentions(&body, "name", 3), (1, true));
        assert_eq!(later_mentions(&body, "name", 5), (0, false));
    }
}
