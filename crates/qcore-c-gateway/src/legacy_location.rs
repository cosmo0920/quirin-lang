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

//! Original (pre-preprocessing) coordinates for gateway diagnostics
//! (card **GATEWAY-C-LOCATION**).
//!
//! Every line number the existing-C analyzer carries is a line of the unit's
//! `-E -P` **preprocessed** text. That number is the analyzer's own identity —
//! a CFG node's position, an acquisition site, the key a pinned leaf is
//! compared by — and it stays exactly what it was. What it is not is a
//! coordinate anyone can act on: nobody reads the preprocessed translation
//! unit, so `preprocessed line 19399` names a place no editor can open.
//!
//! This module derives, for each line of that preprocessed text, the original
//! `file:line` it came from, so a *presented* diagnostic names the C a person
//! wrote. The preprocessed number is retained beside it, unchanged, wherever a
//! leaf identity keys on it.
//!
//! # Mechanism
//!
//! Two preprocessings of the same unit are compared:
//!
//! * the unit's own `-E -P` text — the authoritative one, handed in by the
//!   caller, never re-derived here, so the map is built against exactly the
//!   bytes the analyzer read;
//! * a second, presentation-only `-E` run **with** line markers — the same
//!   command [`crate::legacy_c::enumeration`] already makes for the include
//!   depth attribution, and the same reader.
//!
//! Clang's markers say where each line of its output came from. Walking the
//! marked output gives every *content* line an `(include depth, file, line)`
//! reading; the two outputs' content lines are then paired **in order**. The
//! pairing is checked rather than assumed: the sequences must have the same
//! length and the same text, line for line, or the whole map is refused
//! ([`LocationRefusal::ContentDisagreement`]).
//!
//! # The anti-forgery property (standing constraint 12)
//!
//! Constraint 12 says a `#line` directive in analysed source must not be able
//! to move the audited scope. That rule is untouched: the scope is decided by
//! [`crate::legacy_c::enumeration`]'s include-depth flags, which this module
//! does not read, does not write and cannot influence.
//!
//! The presentation is held to the same standard, by construction:
//!
//! * **No presented file spelling is ever read from a flagless marker.** A
//!   marker's file name is adopted only when it carries flag `1` (Clang
//!   *entering* a file). Clang re-emits a `#line` directive as a **flagless**
//!   marker, so a `#line` can never name a file in a presented coordinate.
//! * **Depth 0 is not named by a marker at all.** The primary input's
//!   presented name is the one the caller declares ([`OriginFile::Primary`],
//!   resolved against the audit's own configured file), not the spelling the
//!   opening marker writes — which under the pinned Cycle 19 transport is the
//!   fixed, guessable `<stdin>` (`CSourceInput::HistoricalStdin`).
//! * **A disagreeing flagless marker withdraws the coordinate.** When a
//!   flagless marker names a file other than the one its frame was entered as,
//!   the frame is *relabelled*: every content line it governs carries **no**
//!   presented coordinate at all, and the diagnostic falls back to its internal
//!   preprocessed position. Fail-closed: an unattributed diagnostic asserts
//!   nothing, where a relabelled one would assert something false.
//!
//! What a `#line` *can* still do is renumber lines inside the file it is
//! written in — `#line 500` with no file spelling, whose re-emitted marker
//! agrees with the frame and is indistinguishable from one of Clang's own
//! resynchronisations. That is exactly the directive's C11 §6.10.4 meaning, it
//! is the file author's statement about the author's own file, and it moves no
//! file identity, no include depth and no audited scope. Recorded as
//! `GATEWAY-LOCATION-LINE-ONLY-DIRECTIVE`.
//!
//! # Macro expansions
//!
//! A definition or statement produced by expanding a macro carries the
//! **expansion site's** coordinate, because that is where Clang's markers put
//! the expanded text: the expansion is written out at the line of the
//! invocation, and no marker intervenes. A diagnostic inside an expansion
//! therefore names the line a reader can actually see the call on.
//!
//! # Fail-closed
//!
//! Nothing here is guessed. A unit whose marked preprocessing does not run,
//! does not open with a marker, does not balance its include stack, or whose
//! content does not line up with the authoritative text, gets **no**
//! coordinates — not approximate ones. Every refusal ground is an
//! [`ReasonCategory::EnvironmentUnavailable`] fact about this analysis
//! machinery and says nothing whatever about the analysed source (§L21.1
//! rule 1), so it can never contribute to a provisional `Unsafe`.
//!
//! This module publishes no verdict, changes no status, reads no contract and
//! is never consulted by the analyzer. It is a presentation index.

use qcore_schemas::reason::ReasonCategory;

use super::C11TranslationUnit;
use super::enumeration::{line_marker, preprocess_with_line_markers};

/// Which file a presented coordinate names.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum OriginFile {
    /// Include depth 0: the unit's own primary input. Deliberately *not* a
    /// marker spelling — the caller supplies the name, so the pinned route's
    /// `<stdin>` transport spelling never reaches a report.
    Primary,
    /// A file Clang entered, named by that **flag `1`** marker's own spelling.
    /// A flagless marker can never produce one of these.
    Included(String),
}

/// One preprocessed line's original coordinate.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct OriginalLocation {
    /// Include depth: `0` is the primary input's own text.
    pub depth: usize,
    pub file: OriginFile,
    /// The line within that file, as the marker stream tracks it.
    pub line: usize,
}

impl OriginalLocation {
    /// The presented `file:line`, with `primary` naming the unit's own input.
    #[must_use]
    pub fn present(&self, primary: &str) -> String {
        match &self.file {
            OriginFile::Primary => format!("{primary}:{}", self.line),
            OriginFile::Included(file) => format!("{file}:{}", self.line),
        }
    }
}

/// Why a unit carries no original coordinates. Every ground is mechanical and
/// every one is about this analysis machinery, never about the analysed source.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LocationRefusal {
    /// The presentation-only line-marked preprocessing did not run.
    ExpansionUnavailable,
    /// The line-marked output does not open with a line marker, so no content
    /// line has a file to belong to.
    PrimaryInputUnidentified,
    /// The marked output's include stack does not balance: a return with
    /// nothing to return to, or a file still open at end of output. The same
    /// ground [`crate::legacy_c::enumeration`] refuses on, for the same reason.
    UnbalancedIncludeMarkers,
    /// The marked output's content lines do not correspond, one for one, with
    /// the authoritative `-E -P` text's. The pairing is the map, so a pairing
    /// that cannot be checked is no map.
    ContentDisagreement,
}

impl LocationRefusal {
    /// Stable report spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ExpansionUnavailable => "expansion-unavailable",
            Self::PrimaryInputUnidentified => "primary-input-unidentified",
            Self::UnbalancedIncludeMarkers => "unbalanced-include-markers",
            Self::ContentDisagreement => "content-disagreement",
        }
    }

    /// The free-text reason carried by the refusal row.
    #[must_use]
    pub const fn reason(self) -> &'static str {
        match self {
            Self::ExpansionUnavailable => {
                "original-coordinate derivation refused: the line-marked preprocessing for this unit is unavailable, so no diagnostic can name its original file and line"
            }
            Self::PrimaryInputUnidentified => {
                "original-coordinate derivation refused: the line-marked preprocessing does not open with a line marker, so no content line has an originating file"
            }
            Self::UnbalancedIncludeMarkers => {
                "original-coordinate derivation refused: the line-marked preprocessing's include stack does not balance, so no originating file can be established"
            }
            Self::ContentDisagreement => {
                "original-coordinate derivation refused: the line-marked preprocessing's content lines do not correspond one for one with the analysed preprocessed text"
            }
        }
    }

    /// The closed §L21.1 category.
    ///
    /// All four grounds are failures of the *analysis environment* — a tool did
    /// not run, or produced output this reader cannot pair with the text the
    /// analyzer read. None of them is a statement about the analysed source, so
    /// none can ever contribute to a provisional `Unsafe`.
    #[must_use]
    pub const fn category(self) -> ReasonCategory {
        ReasonCategory::EnvironmentUnavailable
    }
}

/// One unit's preprocessed-line → original-coordinate index.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SourceLineMap {
    /// Indexed by `preprocessed line - 1`. `None` where the line is blank in
    /// the preprocessed text, or where its frame was relabelled by a
    /// disagreeing flagless marker.
    lines: Vec<Option<OriginalLocation>>,
    refusals: Vec<LocationRefusal>,
    /// Content lines withdrawn because a flagless marker renamed their frame.
    /// Published so a withdrawal is visible rather than silent.
    relabelled_lines: usize,
}

impl SourceLineMap {
    pub(crate) fn expansion_unavailable() -> Self {
        Self::refused(LocationRefusal::ExpansionUnavailable)
    }

    /// The original coordinate of one preprocessed line, or `None` when this
    /// unit has no established coordinate for it.
    #[must_use]
    pub fn original(&self, preprocessed_line: usize) -> Option<&OriginalLocation> {
        preprocessed_line
            .checked_sub(1)
            .and_then(|index| self.lines.get(index))
            .and_then(Option::as_ref)
    }

    /// The presented `file:line` of one preprocessed line, with `primary`
    /// naming the unit's own input, or `None` when there is no coordinate.
    #[must_use]
    pub fn present(&self, preprocessed_line: usize, primary: &str) -> Option<String> {
        self.original(preprocessed_line)
            .map(|location| location.present(primary))
    }

    /// The presented coordinate, falling back to the internal preprocessed
    /// position when none is established. This is the one place a gateway
    /// diagnostic's site is rendered, so every consumer says it the same way.
    #[must_use]
    pub fn site(&self, preprocessed_line: usize, primary: &str) -> String {
        self.present(preprocessed_line, primary)
            .unwrap_or_else(|| unlocated_site(preprocessed_line))
    }

    /// The grounds on which this unit carries no coordinates. Empty means the
    /// map stands.
    #[must_use]
    pub fn refusals(&self) -> &[LocationRefusal] {
        &self.refusals
    }

    /// Whether this map claims to carry original coordinates at all.
    #[must_use]
    pub fn is_established(&self) -> bool {
        self.refusals.is_empty()
    }

    /// Content lines whose coordinate was withdrawn because a flagless marker
    /// renamed their frame.
    #[must_use]
    pub const fn relabelled_lines(&self) -> usize {
        self.relabelled_lines
    }

    fn refused(refusal: LocationRefusal) -> Self {
        Self {
            lines: Vec::new(),
            refusals: vec![refusal],
            relabelled_lines: 0,
        }
    }
}

/// How a site reads when no original coordinate is established. The internal
/// preprocessed position is named as such, so it can never be mistaken for a
/// coordinate in the analysed file.
#[must_use]
pub fn unlocated_site(preprocessed_line: usize) -> String {
    format!("preprocessed line {preprocessed_line}")
}

/// Derive one unit's preprocessed-line → original-coordinate index.
///
/// `preprocessed_source` must be the unit's authoritative `-E -P` text — the
/// bytes the analyzer's line numbers index. It is never re-derived here, so a
/// map can only ever describe the text it was built against.
///
/// Never fails: a unit whose coordinates cannot be established returns a map
/// that answers `None` everywhere, with the ground named.
#[must_use]
pub fn map_preprocessed_lines(
    unit: &C11TranslationUnit<'_>,
    preprocessed_source: &str,
) -> SourceLineMap {
    match preprocess_with_line_markers(unit) {
        Ok(marked) => map_marked_preprocessing(&marked, preprocessed_source),
        Err(_) => SourceLineMap::refused(LocationRefusal::ExpansionUnavailable),
    }
}

/// One open file of the marked output's include stack.
struct Frame {
    /// The spelling this frame was **entered** as (flag `1`), or the opening
    /// marker's spelling for the primary input. Never written by a flagless
    /// marker.
    file: String,
    /// The line the next content line of this frame sits on.
    line: usize,
    /// Set when a flagless marker named a file other than [`Self::file`]. While
    /// set, this frame's content lines carry no coordinate.
    relabelled: bool,
}

/// A marked output walked into per-content-line readings.
struct Walk<'a> {
    /// `(text, reading)` for every non-marker line, in output order.
    lines: Vec<(&'a str, Option<OriginalLocation>)>,
    opened: bool,
    unbalanced: bool,
    relabelled_lines: usize,
}

/// Walk the line-marked output, giving each content line the reading its
/// markers describe.
fn walk_marked(marked: &str) -> Walk<'_> {
    let mut stack: Vec<Frame> = Vec::new();
    let mut walk = Walk {
        lines: Vec::new(),
        opened: false,
        unbalanced: false,
        relabelled_lines: 0,
    };
    for text in marked.split('\n') {
        // Only a line-initial `#` can be a directive, and only a directive
        // whose first word is a digit run followed by a quoted name is a line
        // marker. Comments are already gone from preprocessed output and no C
        // string literal spans a line, so a physical line start is a logical
        // one here.
        if text.starts_with('#')
            && let Some(marker) = line_marker(text)
        {
            if marker.enters {
                stack.push(Frame {
                    file: marker.file,
                    line: marker.number,
                    relabelled: false,
                });
            } else if marker.returns {
                if stack.len() < 2 {
                    // Nothing to return to: the stack is unreadable.
                    walk.unbalanced = true;
                } else {
                    stack.pop();
                    let frame = stack.last_mut().expect("stack is non-empty");
                    frame.line = marker.number;
                    frame.relabelled = marker.file != frame.file;
                }
            } else if let Some(frame) = stack.last_mut() {
                frame.line = marker.number;
                // A flagless marker renaming its frame is a `#line` writing
                // another file's name. The file it names is never adopted.
                frame.relabelled = marker.file != frame.file;
            } else {
                // The opening marker: it establishes the primary frame and is
                // the environment check that Clang emitted markers at all.
                walk.opened = true;
                stack.push(Frame {
                    file: marker.file,
                    line: marker.number,
                    relabelled: false,
                });
            }
            continue;
        }
        let depth = stack.len().saturating_sub(1);
        let reading = match stack.last_mut() {
            None => None,
            Some(frame) => {
                let reading = if frame.relabelled {
                    // A blank line carries no diagnostic and is never paired,
                    // so only content lines are counted as withdrawn.
                    if !text.trim().is_empty() {
                        walk.relabelled_lines += 1;
                    }
                    None
                } else if depth == 0 {
                    Some(OriginalLocation {
                        depth,
                        file: OriginFile::Primary,
                        line: frame.line,
                    })
                } else {
                    Some(OriginalLocation {
                        depth,
                        file: OriginFile::Included(frame.file.clone()),
                        line: frame.line,
                    })
                };
                frame.line += 1;
                reading
            }
        };
        walk.lines.push((text, reading));
    }
    if stack.len() != 1 {
        walk.unbalanced = true;
    }
    walk
}

/// Pair one already-produced line-marked preprocessing with the authoritative
/// `-E -P` text of the same unit.
///
/// Split out from [`map_preprocessed_lines`] so the pairing rule can be driven
/// directly — by a test, or by a caller that already holds both outputs — with
/// no Clang run in between. It is the whole of the derivation: everything else
/// is producing the marked text.
#[must_use]
pub fn map_marked_preprocessing(marked: &str, preprocessed: &str) -> SourceLineMap {
    let walk = walk_marked(marked);
    if !walk.opened {
        return SourceLineMap::refused(LocationRefusal::PrimaryInputUnidentified);
    }
    if walk.unbalanced {
        return SourceLineMap::refused(LocationRefusal::UnbalancedIncludeMarkers);
    }
    // Both outputs carry the same content in the same order; only blank-line
    // filler and leading whitespace differ, because `-P` neither emits markers
    // nor pads to keep line numbers in step. Pair the non-blank lines.
    let marked_content = walk
        .lines
        .iter()
        .filter(|(text, _)| !text.trim().is_empty())
        .collect::<Vec<_>>();
    let plain_lines = preprocessed.split('\n').collect::<Vec<_>>();
    let plain_content = plain_lines
        .iter()
        .enumerate()
        .filter(|(_, text)| !text.trim().is_empty())
        .collect::<Vec<_>>();
    if marked_content.len() != plain_content.len() {
        return SourceLineMap::refused(LocationRefusal::ContentDisagreement);
    }
    if marked_content
        .iter()
        .zip(&plain_content)
        .any(|((marked, _), (_, plain))| marked.trim() != plain.trim())
    {
        return SourceLineMap::refused(LocationRefusal::ContentDisagreement);
    }
    let mut lines = vec![None; plain_lines.len()];
    for ((_, reading), (index, _)) in marked_content.iter().zip(&plain_content) {
        lines[*index] = reading.clone();
    }
    SourceLineMap {
        lines,
        refusals: Vec::new(),
        relabelled_lines: walk.relabelled_lines,
    }
}

#[cfg(test)]
#[path = "../tests/unit/legacy_location.rs"]
mod tests;
