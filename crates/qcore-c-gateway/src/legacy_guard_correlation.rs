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

//! Card **TAIL-FILE-DEFINITION-CENSUS-INCOMPLETE** (rule 49): deciding a raw
//! scan whose conditional groups disagree, by guard correlation.
//!
//! Unit **E01-RAW-COND**'s raw scan refuses the definition census when one
//! `#if … #endif` group's arms leave different file-scope brace depths, because
//! which depth survives is decided by evaluating the group's controlling
//! expression. Fluent Bit's `plugins/in_tail/tail_file.c` (`tag_compose`) has
//! the commonest shape of that kind:
//!
//! ```c
//! #ifdef FLB_HAVE_REGEX
//!     if (tag_regex) { … }
//!     else {
//! #endif
//!         …
//! #ifdef FLB_HAVE_REGEX
//!     }
//! #endif
//! ```
//!
//! Each group alone disagrees (+1 against 0, then −1 against 0), but the two
//! groups test the **same guard** with **no directive between them** that
//! could define or undefine it, so in every preprocessing of the file they
//! are both kept or both skipped, and the brace balances either way.
//!
//! This scan reads exactly that, and nothing more. It never evaluates a
//! controlling expression. It runs one raw scan per *world*: a world is a
//! scan state with a set of assumptions `NAME is (not) defined`. A group whose
//! opening line is exactly `#ifdef NAME` or `#ifndef NAME`, with at most an
//! `#else` and no `#elif` (a *guard group*), is read under the world's
//! assumption about `NAME`, and splits the world in two (one per value) when
//! it has none; every other group is read as E01-RAW-COND reads it, every arm
//! from the entry depth, and a world whose arms of such a group disagree
//! refuses the whole scan. Any other directive (`#define`, `#undef`,
//! `#include`, `#pragma`, …) and any `_Pragma`/`__pragma` operator in the
//! world's kept text clears all of the world's assumptions (a later guard
//! group splits again), because each can change which macros are defined.
//! Worlds whose scan states are equal are merged, keeping only the
//! assumptions they share; more than [`WORLD_BUDGET`] worlds refuse. A
//! directive that spells `_Pragma`, `__pragma`, `push_macro` or `pop_macro`
//! refuses outright: a macro defined with one changes the macro table where it
//! is *invoked*, which reads as an ordinary identifier here.
//!
//! The premise is E01-RAW-COND's, unchanged: the raw half reads written text,
//! not macro expansions, so a macro defined outside this file that expands to a
//! brace or to a `push_macro`/`pop_macro` pragma is outside what it reads (the
//! expansion half stays authoritative for the configuration actually
//! compiled).
//!
//! Soundness, stated once: the worlds always cover every assignment of
//! definedness to guard names, each world follows exactly the configurations
//! its assumptions admit, and a definition is collected whenever a world
//! reaches it at file scope. So every definition any preprocessing of the
//! file keeps at file scope — and every definition a guarded-out arm would
//! have at file scope in some configuration — is collected; a merged world may
//! also follow a configuration no preprocessing reaches, which can only add a
//! name. `QCore.GuardCorrelatedCensus` proves it over a bounded exhaustive
//! domain and the measured shape.

use std::collections::{BTreeMap, BTreeSet};

use super::{
    ConditionalDirective, ConditionalMark, Token, is_identifier, matching, tokenize_with_directives,
};

/// The most worlds one scan keeps at once; more refuses.
pub const WORLD_BUDGET: usize = 64;

/// Why the guard-correlation scan could not decide a source.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CorrelationRefusal {
    /// A conditional group is not well formed (as E01-RAW-COND's
    /// `UnstructuredGroup`).
    UnstructuredGroup,
    /// In some world, the arms of a group that is not a guard group leave
    /// different depths: deciding it would be conditional evaluation.
    ArmsDisagree,
    /// More than [`WORLD_BUDGET`] worlds would be needed.
    WorldBudget,
    /// A directive other than a conditional one spells `_Pragma`, `__pragma`,
    /// `push_macro` or `pop_macro`: a macro defined with one could change a
    /// guard's definedness wherever it is invoked, which this scan cannot see.
    MacroTableOperator,
}

impl CorrelationRefusal {
    /// Stable report spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UnstructuredGroup => "unstructured-conditional-group",
            Self::ArmsDisagree => "uncorrelated-arm-depth-disagreement",
            Self::WorldBudget => "guard-world-budget-exhausted",
            Self::MacroTableOperator => "macro-table-operator-in-a-directive",
        }
    }
}

/// The text of the directive starting at `start`, to the end of its logical
/// line.
fn directive_text(source: &str, start: usize) -> &str {
    let bytes = source.as_bytes();
    let mut cursor = start;
    while cursor < bytes.len() && bytes[cursor] != b'\n' {
        cursor += super::line_splice_width(bytes, cursor).unwrap_or(1);
    }
    &source[start..cursor.min(bytes.len())]
}

/// Whether a directive's text spells an operator that changes the macro table
/// when a macro carrying it is invoked.
fn writes_a_macro_table_operator(text: &str) -> bool {
    ["_Pragma", "__pragma", "push_macro", "pop_macro"]
        .iter()
        .any(|word| text.contains(word))
}

/// The guard of a guard-group opening line: `#ifdef NAME` is `(NAME, true)`,
/// `#ifndef NAME` is `(NAME, false)`; any other spelling is `None`.
///
/// Deliberately narrow: horizontal white space and line splices only, the
/// name, then nothing but white space or one trailing comment. A block comment
/// before the directive word, an `#if defined(NAME)` or anything else is not a
/// guard group and is read as an ordinary group.
#[must_use]
pub fn guard_of(line: &str) -> Option<(String, bool)> {
    let logical = line.replace("\\\r\n", "").replace("\\\n", "");
    let rest = logical.strip_prefix('#')?;
    let rest = rest.trim_start_matches([' ', '\t']);
    let (word, rest) = split_identifier(rest);
    let defined = match word {
        "ifdef" => true,
        "ifndef" => false,
        _ => return None,
    };
    let after = rest.trim_start_matches([' ', '\t']);
    if after.len() == rest.len() {
        return None;
    }
    let (name, rest) = split_identifier(after);
    if name.is_empty() || name.as_bytes()[0].is_ascii_digit() {
        return None;
    }
    let tail = rest.trim_matches([' ', '\t', '\r']);
    let clean = tail.is_empty()
        || tail.starts_with("//")
        || (tail.starts_with("/*")
            && tail[2..].find("*/").is_some_and(|end| {
                tail[2 + end + 2..]
                    .trim_matches([' ', '\t', '\r'])
                    .is_empty()
            }));
    clean.then(|| (name.to_owned(), defined))
}

fn split_identifier(text: &str) -> (&str, &str) {
    let end = text
        .bytes()
        .position(|byte| !(byte.is_ascii_alphanumeric() || byte == b'_'))
        .unwrap_or(text.len());
    text.split_at(end)
}

/// One conditional group's shape, read before the scan.
struct Group {
    /// The guard, when the group is a guard group.
    guard: Option<(String, bool)>,
    has_else: bool,
}

/// The group structure: each mark's group and the next mark of the same group.
struct Structure {
    groups: Vec<Group>,
    group_of: Vec<usize>,
    next_of: Vec<usize>,
}

fn structure(source: &str, marks: &[ConditionalMark]) -> Result<Structure, CorrelationRefusal> {
    let mut groups: Vec<Group> = Vec::new();
    let mut has_alternative: Vec<bool> = Vec::new();
    let mut group_of = vec![0usize; marks.len()];
    let mut next_of = vec![usize::MAX; marks.len()];
    // (group, the group's last mark so far)
    let mut open: Vec<(usize, usize)> = Vec::new();
    for (index, mark) in marks.iter().enumerate() {
        match mark.directive() {
            ConditionalDirective::Open => {
                let (start, end) = mark.span();
                groups.push(Group {
                    guard: guard_of(&source[start..end]),
                    has_else: false,
                });
                has_alternative.push(false);
                group_of[index] = groups.len() - 1;
                open.push((groups.len() - 1, index));
            }
            directive => {
                let Some((group, last)) = open.last().copied() else {
                    return Err(CorrelationRefusal::UnstructuredGroup);
                };
                if groups[group].has_else && directive != ConditionalDirective::Close {
                    return Err(CorrelationRefusal::UnstructuredGroup);
                }
                group_of[index] = group;
                next_of[last] = index;
                match directive {
                    ConditionalDirective::Alternative => {
                        has_alternative[group] = true;
                        open.last_mut().expect("open group").1 = index;
                    }
                    ConditionalDirective::Otherwise => {
                        groups[group].has_else = true;
                        open.last_mut().expect("open group").1 = index;
                    }
                    _ => {
                        open.pop();
                    }
                }
            }
        }
    }
    if !open.is_empty() {
        return Err(CorrelationRefusal::UnstructuredGroup);
    }
    for (group, alternative) in groups.iter_mut().zip(has_alternative) {
        if alternative {
            group.guard = None;
        }
    }
    Ok(Structure {
        groups,
        group_of,
        next_of,
    })
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Frame {
    /// A guard group: whether this world keeps its first arm.
    Guard { took_first: bool },
    /// Any other group: entry depth and the depths its finished arms reached.
    Plain { entry: usize, arms: Vec<usize> },
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Shape {
    depth: usize,
    frames: Vec<Frame>,
    /// The mark this world skips to (the next mark of the guard group whose
    /// current arm it does not keep).
    skip_until: Option<usize>,
    linkage_closes: BTreeSet<usize>,
}

#[derive(Clone, Debug)]
struct World {
    shape: Shape,
    assumed: BTreeMap<String, bool>,
}

enum Event {
    Conditional(usize),
    Other,
}

/// Scan `source` for its file-scope definitions under guard correlation.
///
/// `Ok` carries every definition some world reaches at file scope, sorted and
/// deduplicated; it is a superset of the definitions every preprocessing of
/// the source keeps at file scope, and of every arm's definitions under the
/// configurations that keep that arm. `Err` names why no such claim is made.
pub fn correlated_source_scan(source: &str) -> Result<Vec<String>, CorrelationRefusal> {
    let (tokens, marks, others) = tokenize_with_directives(source);
    let structure = structure(source, &marks)?;
    if others
        .iter()
        .any(|&(_, start)| writes_a_macro_table_operator(directive_text(source, start)))
    {
        return Err(CorrelationRefusal::MacroTableOperator);
    }
    let mut events: Vec<(usize, usize, Event)> = marks
        .iter()
        .enumerate()
        .map(|(index, mark)| (mark.token_index(), mark.span().0, Event::Conditional(index)))
        .chain(
            others
                .iter()
                .map(|&(token_index, start)| (token_index, start, Event::Other)),
        )
        .collect();
    events.sort_by_key(|(token_index, start, _)| (*token_index, *start));

    let mut names = BTreeSet::new();
    let mut worlds = vec![World {
        shape: Shape {
            depth: 0,
            frames: Vec::new(),
            skip_until: None,
            linkage_closes: BTreeSet::new(),
        },
        assumed: BTreeMap::new(),
    }];
    let mut next_event = 0usize;
    for index in 0..=tokens.len() {
        while next_event < events.len() && (index == tokens.len() || events[next_event].0 <= index)
        {
            worlds = apply_event(&events[next_event].2, &marks, &structure, worlds)?;
            next_event += 1;
        }
        if index == tokens.len() {
            break;
        }
        for world in &mut worlds {
            if world.shape.skip_until.is_none() {
                step_token(&tokens, index, world, &mut names);
            }
        }
    }
    Ok(names.into_iter().collect())
}

fn apply_event(
    event: &Event,
    marks: &[ConditionalMark],
    structure: &Structure,
    worlds: Vec<World>,
) -> Result<Vec<World>, CorrelationRefusal> {
    let mut out = Vec::with_capacity(worlds.len() + 1);
    for mut world in worlds {
        let Event::Conditional(mark) = *event else {
            if world.shape.skip_until.is_none() {
                world.assumed.clear();
            }
            out.push(world);
            continue;
        };
        let directive = marks[mark].directive();
        match world.shape.skip_until {
            Some(target) if target == mark => {
                // Arrival at the arm boundary this world skipped to.
                world.shape.skip_until = None;
                if directive == ConditionalDirective::Close {
                    world.shape.frames.pop();
                }
                out.push(world);
                continue;
            }
            Some(_) => {
                out.push(world);
                continue;
            }
            None => {}
        }
        let group = &structure.groups[structure.group_of[mark]];
        match directive {
            ConditionalDirective::Open => match &group.guard {
                Some((name, ifdef)) => {
                    let values = match world.assumed.get(name) {
                        Some(value) => vec![*value],
                        None => vec![true, false],
                    };
                    for value in values {
                        let mut split = world.clone();
                        split.assumed.insert(name.clone(), value);
                        let took_first = value == *ifdef;
                        split.shape.frames.push(Frame::Guard { took_first });
                        if !took_first {
                            split.shape.skip_until = Some(structure.next_of[mark]);
                        }
                        out.push(split);
                    }
                }
                None => {
                    let entry = world.shape.depth;
                    world.shape.frames.push(Frame::Plain {
                        entry,
                        arms: Vec::new(),
                    });
                    out.push(world);
                }
            },
            ConditionalDirective::Alternative | ConditionalDirective::Otherwise => {
                match world.shape.frames.last_mut() {
                    Some(Frame::Guard { took_first: true }) => {
                        world.shape.skip_until = Some(structure.next_of[mark]);
                    }
                    Some(Frame::Plain { entry, arms }) => {
                        arms.push(world.shape.depth);
                        world.shape.depth = *entry;
                    }
                    _ => return Err(CorrelationRefusal::UnstructuredGroup),
                }
                out.push(world);
            }
            ConditionalDirective::Close => {
                match world.shape.frames.pop() {
                    Some(Frame::Guard { .. }) => {}
                    Some(Frame::Plain { entry, mut arms }) => {
                        arms.push(world.shape.depth);
                        if !group.has_else {
                            arms.push(entry);
                        }
                        if arms.iter().any(|depth| *depth != arms[0]) {
                            return Err(CorrelationRefusal::ArmsDisagree);
                        }
                        world.shape.depth = arms[0];
                    }
                    None => return Err(CorrelationRefusal::UnstructuredGroup),
                }
                out.push(world);
            }
        }
    }
    let merged = merge(out);
    if merged.len() > WORLD_BUDGET {
        return Err(CorrelationRefusal::WorldBudget);
    }
    Ok(merged)
}

/// Merge worlds with equal scan states, keeping only shared assumptions.
fn merge(worlds: Vec<World>) -> Vec<World> {
    let mut by_shape: BTreeMap<Shape, BTreeMap<String, bool>> = BTreeMap::new();
    for world in worlds {
        match by_shape.get_mut(&world.shape) {
            Some(assumed) => assumed.retain(|name, value| world.assumed.get(name) == Some(value)),
            None => {
                by_shape.insert(world.shape, world.assumed);
            }
        }
    }
    by_shape
        .into_iter()
        .map(|(shape, assumed)| World { shape, assumed })
        .collect()
}

/// One kept token, exactly as E01-RAW-COND's `function_names` reads it.
fn step_token(tokens: &[Token], index: usize, world: &mut World, names: &mut BTreeSet<String>) {
    let token = &tokens[index];
    let shape = &mut world.shape;
    if token.text == "{" {
        if index >= 2
            && tokens[index - 2].text == "extern"
            && tokens[index - 1].text == "__qcore_literal__"
            && let Some(close) = matching(tokens, index, "{", "}")
        {
            shape.linkage_closes.insert(close);
            return;
        }
        shape.depth += 1;
        return;
    }
    if token.text == "}" {
        if shape.linkage_closes.remove(&index) {
            return;
        }
        shape.depth = shape.depth.saturating_sub(1);
        return;
    }
    if matches!(token.text.as_str(), "_Pragma" | "__pragma") {
        world.assumed.clear();
    }
    if shape.depth != 0 {
        return;
    }
    if !is_identifier(&token.text)
        || matches!(token.text.as_str(), "if" | "while" | "for" | "switch")
        || tokens.get(index + 1).map(|next| next.text.as_str()) != Some("(")
    {
        return;
    }
    if let Some(close) = matching(tokens, index + 1, "(", ")")
        && tokens.get(close + 1).map(|next| next.text.as_str()) == Some("{")
    {
        names.insert(token.text.clone());
    }
}
