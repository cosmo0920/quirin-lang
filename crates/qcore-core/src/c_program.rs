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

//! The one place an emitted QCore translation unit is assembled, and the one
//! place the program entry point is written (card `C-EMIT-MAIN-UNIFY`).
//!
//! # What this module is
//!
//! Before this card, ten backends each built a whole translation unit by
//! pushing header text and then their own `int main(` spelling into a `String`.
//! Every backend was correct on its own — the driver's route dispatch is
//! mutually exclusive, so exactly one of them ever ran for a given program, and
//! a census of every published example confirmed exactly one `main` per emitted
//! unit. What the old shape could not do is *compose*: the way to put two
//! routes in one program was to write an eleventh whole-program backend with an
//! eleventh `main`, which is how `composed_std_io_c_emit` came to exist.
//!
//! This module replaces the whole-program string with two pieces a backend
//! contributes independently:
//!
//! * a list of [`Fragment`]s — the file-scope text this route needs, each under
//!   a **key** that names the thing rather than the route that asked for it; and
//! * one [`Entry`] — the statements that run, plus the exact trivia around them.
//!
//! [`assemble`] takes a fragment list and **exactly one** `Entry`. That is the
//! whole single-`main` guarantee, and it is a type, not a check: there is no
//! way to hand `assemble` two entry points, and [`ENTRY_SIGNATURE`] is the only
//! `int main(` spelling in the crate. A composition of two routes therefore
//! cannot produce two `main`s by construction — it merges two fragment lists
//! (which is where deduplication happens) and interleaves two bodies into the
//! one entry, exactly as the composed standard-IO route already interleaves
//! `emit_extern_statements` with its own line statements.
//!
//! # Why order is insertion order, not section order
//!
//! [`Section`] is a **declared position label**, and [`assemble`] does not sort
//! by it. That is deliberate and was decided by measurement rather than taste.
//!
//! The emitted C of every existing program shape is pinned — by the two F02
//! emission digests, by `examples/` transcripts, by exact-count assertions on
//! `fprintf` and `fwrite`, and by `emitted_c_contains` fixtures — and those pins
//! bind bytes, not structure. Sorting by section would have moved one program's
//! bytes: the task backend places its `#include <stdio.h>` *after*
//! `DIAGNOSTIC_LOG_PRELUDE` rather than beside its other includes, so a stable
//! sort would have lifted that line above the logger prelude. The move is inert
//! in C — the logger prelude uses no stdio — but it is still a byte move in a
//! digest-pinned program, and this card's premise is that no such move happens
//! silently. Insertion order reproduces every backend's existing bytes exactly.
//!
//! The inversion is reported rather than hidden: [`out_of_order_fragments`] is a
//! decision procedure that names every position where a fragment's section
//! precedes its predecessor's, so the one real case is a measured finding a test
//! pins rather than a fact buried in a backend.
//!
//! Composition is where section order earns its keep. [`merge`] — the path a
//! multi-route program takes — concatenates each route's contributions,
//! deduplicates, and *then* orders stably by section, because two independently
//! written routes have no insertion order relative to each other and C does
//! require compatibility headers before includes and includes before the
//! definitions that use them.
//!
//! # What deduplication is, and what it is not
//!
//! Two routes that both need `io_emit::IO_PRELUDE` contribute it under the same
//! key, and [`deduplicate`] keeps the first. That is what makes "two routes in
//! one program" incapable of defining a helper twice.
//!
//! It is first-wins on the key alone, across sections: a key names a thing, and
//! a thing lives in one place. Two fragments sharing a key but disagreeing on
//! text would therefore be a latent defect — the second route's text silently
//! discarded — so [`conflicting_keys`] decides that question separately, in the
//! style of `io_emit::direct_stdio_calls`: the backends do not call it, the
//! tests and the Lean model do.
//!
//! # What is not claimed
//!
//! That the assembled text is valid C. This module concatenates bytes in a
//! declared order; every fragment's own correctness is its contributor's, and
//! `assemble` neither parses nor validates what it is handed.
//!
//! That `ENTRY_SIGNATURE` being the crate's only `int main(` spelling makes an
//! emitted program's `main` unique — a *fragment* could contain one, which is
//! why [`entry_definitions`] scans emitted text and the tests assert one per
//! program. Like the spelling table in `io_emit`, that scan is lexical and its
//! boundary is stated on both sides: a program could name an entry point some
//! way this scan does not spell.

use std::collections::BTreeMap;

/// The signature of an emitted program's entry point.
///
/// The only `int main(` spelling in this crate: every backend reaches its entry
/// point through [`assemble`], which writes this constant exactly once.
pub const ENTRY_SIGNATURE: &str = "int main(void)";

/// Where a fragment sits in an assembled translation unit.
///
/// A declared position label. [`assemble`] preserves the order a single backend
/// contributed its fragments in and does not consult this; [`merge`] orders by
/// it, because two routes composed into one program have no relative insertion
/// order of their own.
///
/// The order of the variants is the order C requires: a compatibility header
/// defines the feature-test macros every libc include reads, includes precede
/// the declarations that name their types, and a definition follows its
/// declaration.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Section {
    /// The generated-by comment a route puts at the top of its own file. First
    /// because it is what a reader sees first; it carries no C meaning.
    Banner,
    /// Macro configuration a later header reads — `#define`s that must precede
    /// even the compatibility header. The task runtime's capacity constants.
    Config,
    /// An embedded compatibility header. Its arms define `_POSIX_C_SOURCE` and
    /// friends, so it precedes every libc include in the unit.
    Compat,
    /// `#include` lines.
    Includes,
    /// Forward declarations — a name introduced before the definition that uses
    /// it, such as the hosted diagnostic adapter's prototype.
    Declarations,
    /// Macro, type and helper definitions: the preludes.
    Prelude,
    /// File-scope data and functions the entry point uses — static literal
    /// arrays, extern prototypes, emitted function bodies.
    FileScope,
}

/// One named piece of file-scope text.
///
/// `key` names the *thing*, not the route that asked for it: two routes needing
/// one prelude spell one key, which is what lets [`deduplicate`] give a composed
/// program a single copy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fragment {
    /// This fragment's declared position.
    pub section: Section,
    /// The deduplication identity.
    pub key: String,
    /// The exact bytes, emitted verbatim. `assemble` adds no separator of its
    /// own, so a fragment carries its own leading and trailing newlines.
    pub text: String,
}

impl Fragment {
    /// One fragment, from parts.
    #[must_use]
    pub fn new(section: Section, key: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            section,
            key: key.into(),
            text: text.into(),
        }
    }
}

/// The brace style of an emitted entry point.
///
/// Both spellings exist in emitted programs today and both are pinned, so the
/// injector reproduces each rather than choosing one. Normalizing them would be
/// a byte move in a digest-pinned program for no stated gain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BraceStyle {
    /// `int main(void)\n{\n` — the seven source-route backends and the
    /// sequential backend.
    Allman,
    /// `int main(void) {\n` — the task and mutex backends.
    KAndR,
}

impl BraceStyle {
    /// The bytes this style writes after [`ENTRY_SIGNATURE`].
    #[must_use]
    pub fn bytes(self) -> &'static str {
        match self {
            Self::Allman => "\n{\n",
            Self::KAndR => " {\n",
        }
    }
}

/// The program's single entry point.
///
/// `lead` is the trivia before the signature — a blank line for most routes,
/// nothing for the sequential, task and mutex routes, which follow text that
/// already ends in one. `prologue` is what the entry declares before the
/// program's own statements; `body` is those statements; `epilogue` closes the
/// function, including its `return` and its `}`.
///
/// The split is what a composition needs: two routes' `body` text interleaves,
/// their `prologue` text concatenates, and the trivia and the closing brace stay
/// the assembling route's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// Bytes before the signature.
    pub lead: String,
    /// The brace style.
    pub brace: BraceStyle,
    /// Declarations the entry opens with.
    pub prologue: String,
    /// The program's statements.
    pub body: String,
    /// The closing text, including the `return` and the `}`.
    pub epilogue: String,
}

impl Entry {
    /// The shape seven of the eight source-route backends emit: a blank line,
    /// an Allman brace, no prologue and a plain `return 0;`.
    #[must_use]
    pub fn source_route(body: impl Into<String>) -> Self {
        Self {
            lead: "\n".to_owned(),
            brace: BraceStyle::Allman,
            prologue: String::new(),
            body: body.into(),
            epilogue: "    return 0;\n}\n".to_owned(),
        }
    }
}

/// The fragments of `fragments` that survive deduplication, in the order they
/// were contributed.
///
/// First-wins on the key alone: a key names a thing, and a thing lives in one
/// place, so a key repeated in a second section is still the same thing.
#[must_use]
pub fn deduplicate(fragments: &[Fragment]) -> Vec<&Fragment> {
    let mut seen: Vec<&str> = Vec::new();
    let mut kept = Vec::new();
    for fragment in fragments {
        if seen.contains(&fragment.key.as_str()) {
            continue;
        }
        seen.push(&fragment.key);
        kept.push(fragment);
    }
    kept
}

/// Every key carried by two fragments whose text disagrees, in sorted order.
///
/// A decision procedure rather than a gate, in the style of
/// `io_emit::direct_stdio_calls`: the backends do not call it, and the tests and
/// the Lean model do. An empty result is the statement that deduplication
/// discarded nothing a contributor meant differently — without it, first-wins
/// would silently drop a second route's variant of a shared helper.
#[must_use]
pub fn conflicting_keys(fragments: &[Fragment]) -> Vec<String> {
    let mut first: BTreeMap<&str, &str> = BTreeMap::new();
    let mut conflicting: Vec<String> = Vec::new();
    for fragment in fragments {
        match first.get(fragment.key.as_str()) {
            None => {
                first.insert(&fragment.key, &fragment.text);
            }
            Some(text) if *text == fragment.text.as_str() => {}
            Some(_) => {
                if !conflicting.contains(&fragment.key) {
                    conflicting.push(fragment.key.clone());
                }
            }
        }
    }
    conflicting.sort();
    conflicting
}

/// The positions, zero-based, at which a fragment's declared section precedes
/// its predecessor's.
///
/// An empty result means this contribution is already in section order. The
/// procedure exists because [`assemble`] deliberately does not sort: the one
/// inversion in the repository — the task backend's late `#include <stdio.h>` —
/// is then a measured fact a test pins rather than a silent one, and any new
/// inversion shows up the same way.
#[must_use]
pub fn out_of_order_fragments(fragments: &[Fragment]) -> Vec<usize> {
    let mut inversions = Vec::new();
    for index in 1..fragments.len() {
        if fragments[index].section < fragments[index - 1].section {
            inversions.push(index);
        }
    }
    inversions
}

/// Merge several routes' contributions into one fragment list: concatenate,
/// deduplicate, then order stably by section.
///
/// This is the composition path. A single route's own emission goes through
/// [`assemble`] directly and keeps its insertion order; two routes have no
/// insertion order relative to each other, so their merged list is ordered by
/// the declared positions, which is the ordering C actually requires.
///
/// Within one section the contributors' relative order is preserved, so the
/// result is a function of the contribution order and nothing else.
#[must_use]
pub fn merge(contributions: &[&[Fragment]]) -> Vec<Fragment> {
    let flattened: Vec<Fragment> = contributions
        .iter()
        .flat_map(|route| route.iter().cloned())
        .collect();
    let mut merged: Vec<Fragment> = deduplicate(&flattened).into_iter().cloned().collect();
    // Stable, so the contribution order decides ties inside a section.
    merged.sort_by_key(|fragment| fragment.section);
    merged
}

/// The deduplicated fragments' text, concatenated in order — everything
/// [`assemble`] writes before the entry point.
///
/// Exposed because a backend that publishes a source map needs to know how many
/// generated lines precede the text it mapped: the task backend builds its
/// mappings against its own function bodies and shifts them by this prefix's
/// line count, which is exactly the number it used to get for free when the
/// preamble was spliced into the same `String`.
#[must_use]
pub fn preamble_text(fragments: &[Fragment]) -> String {
    let mut text = String::new();
    for fragment in deduplicate(fragments) {
        text.push_str(&fragment.text);
    }
    text
}

/// Assemble one translation unit: the deduplicated fragments in order, then the
/// program's single entry point.
///
/// No separator is inserted anywhere — every byte of the result comes from a
/// fragment, from `entry`, or from [`ENTRY_SIGNATURE`] and the brace style.
///
/// Taking `entry` by reference rather than as a list is the single-`main`
/// guarantee: a caller cannot supply two, so a composed program has one entry
/// whose body is an interleaving, never two entries.
#[must_use]
pub fn assemble(fragments: &[Fragment], entry: &Entry) -> String {
    let mut source = preamble_text(fragments);
    source.push_str(&entry.lead);
    source.push_str(ENTRY_SIGNATURE);
    source.push_str(entry.brace.bytes());
    source.push_str(&entry.prologue);
    source.push_str(&entry.body);
    source.push_str(&entry.epilogue);
    source
}

/// Every line of an emitted program that defines the entry point, as one-based
/// line numbers.
///
/// The single-`main` analogue of `io_emit::direct_stdio_calls`, and the same
/// kind of statement: for a program [`assemble`] built from fragments that
/// define no entry point of their own, the result has exactly one element.
///
/// Deliberately a lexical scan, because what is being decided is whether a
/// spelling occurs. Its boundary is the same as the spelling table's and is
/// stated on both sides: a translation unit could name an entry point in a way
/// this scan does not spell — a differently declared `main`, or one reached
/// through a macro — and this procedure would not see it.
#[must_use]
pub fn entry_definitions(source: &str) -> Vec<usize> {
    source
        .lines()
        .enumerate()
        .filter(|(_, line)| line.contains("int main("))
        .map(|(index, _)| index + 1)
        .collect()
}

#[cfg(test)]
#[path = "../tests/unit/c_program.rs"]
mod tests;
