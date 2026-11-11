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

//! Publication destinations — objects with **static storage duration** that a
//! function body may name as an assignment destination (card `CTWIN-PUBLISH`).
//!
//! Card `CT-CTWIN` measured, and recorded as the open finding
//! **`CT-CTWIN-PUBLISH`** (`docs/reports/wave9-ctwin/README.md` §1.1), that a
//! store of an owned pointer into a file-scope object is invisible to the
//! resource projection:
//!
//! ```c
//! struct b_handle *g_published;
//! void w(int n) { struct b_handle *h = b_open(0, 0, n);
//!                 if (!h) { return; } g_published = h; b_close(h); }
//! ```
//!
//! was **`Safe`, no diagnostic**. The importer lowered `g_published = h` to an
//! ordinary [`crate::legacy_c::LegacyInstruction::Alias`], so the file-scope
//! object was modeled as one more local name: custody stayed inside the
//! function, the release discharged it, and the dangling published reference
//! had no representation at all.
//!
//! This module supplies the **destination side** of the repair: the set of
//! names a body may assign to whose object outlives the call. Two disjoint
//! sources, both read from the same preprocessed token stream every other typed
//! rule reads, so W4 derivation, W4 validation and certificate rederivation
//! repeat the classification from the certificate's own content:
//!
//! * [`file_scope_objects`] — objects declared at file scope, from the whole
//!   translation unit;
//! * [`body_declarations`] — the names one body declares, split into *all* of
//!   them (which shadow a file-scope object of the same name) and the
//!   **static-duration** subset (`static` / `_Thread_local` locals), which
//!   outlive the call exactly as a file-scope object does.
//!
//! The decision itself lives in [`crate::legacy_c::transfer`] and is
//! flow-sensitive: naming such a destination is not by itself a refusal, only
//! *storing a tracked identity this body holds* into one is. See
//! `docs/reports/remaining-ctwin-publish/README.md` for the two-direction
//! argument, and in particular for why the destination is **refused** rather
//! than modeled as an escape.

use std::collections::{BTreeMap, BTreeSet};

use crate::typed_field_access::{is_declaration_candidate, is_name, statement_end};

/// Storage-class keywords that give a *block-scope* declaration static storage
/// duration. `extern` is deliberately absent from this list: a block-scope
/// `extern` declaration refers to a file-scope object, which
/// [`file_scope_objects`] already answers for, and recording it here would let
/// a body's own re-declaration of a global count twice.
const STATIC_DURATION_SPECIFIERS: &[&str] = &["static", "_Thread_local"];

/// Keywords that introduce a *tag*, never an object, when they immediately
/// precede the trailing identifier of a declarator.
const TAG_KEYWORDS: &[&str] = &["struct", "union", "enum"];

/// The C11 type specifiers whose spelling is `KEYWORD ( type-name )`.
///
/// Card `CTWIN-RESIDUALS`. `_Atomic` is the only one C11 defines (§6.7.2.4),
/// and the parenthesis it carries belongs to the *specifier*, never to a
/// declarator — which is what makes dropping it monotone: it can only reveal a
/// declarator the `(`-carrying skip in [`collect_declared_objects`] hid, never
/// hide one it recognised. Measured at the wave-16 base, `_Atomic(void *) g;`
/// was **not** a recognised destination, so `g = h; free(h);` through it was
/// `Safe` with no diagnostic — CT40's literal atomic-pointer spelling.
const PARENTHESISED_SPECIFIERS: &[&str] = &["_Atomic"];

/// Token-scan bound. A translation unit longer than this is read in full; the
/// bound applies per declaration run, mirroring the declaration reader's own.
const RUN_LIMIT: usize = 1 << 20;

/// The objects `unit` declares at **file scope**.
///
/// The scan is deliberately narrow and positive: a name enters the set only
/// when this reader recognised a file-scope declarator for it. A declaration
/// shape it cannot read contributes nothing, which leaves the pre-existing
/// modeling in place for that name rather than inventing a refusal — the
/// residual is recorded as `CTWIN-PUBLISH-SCAN`.
///
/// Function definitions and function declarators contribute nothing: a run
/// carrying `(` at declarator level is skipped whole, so neither a prototype's
/// parameter names nor a K&R definition's parameter list can enter the set.
pub(crate) fn file_scope_objects(unit: &[&str]) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    let mut saw_assignment = false;
    for (index, text) in unit.iter().enumerate() {
        match *text {
            "{" => depth += 1,
            "}" => {
                depth = depth.saturating_sub(1);
                // A brace group that closes back to file scope ends a
                // *definition* — a function body, or a record or enumerator
                // list — unless it is an initializer, which an `=` at file
                // scope already announced. Only in the definition case does the
                // run restart here; otherwise `struct S { … } g;` and
                // `int g[] = { … };` would both lose their declarator.
                if depth == 0 && !saw_assignment {
                    start = index + 1;
                }
            }
            "=" if depth == 0 => saw_assignment = true,
            ";" if depth == 0 => {
                if index > start && index - start <= RUN_LIMIT {
                    collect_declared_objects(&unit[start..index], &mut names);
                }
                start = index + 1;
                saw_assignment = false;
            }
            _ => {}
        }
    }
    names
}

/// The names `body` declares: every one of them, and the static-duration ones.
///
/// The first set is the **shadowing** answer — a body that declares `g` of its
/// own never publishes through that name, whatever file scope says — and the
/// second is a publication source in its own right, because a `static` local
/// outlives the call exactly as a file-scope object does.
///
/// Both over-approximate in the direction that *reduces* refusal: a statement
/// this reader mistakes for a declaration (`a * b;`) only adds a shadowing
/// name. That direction is deliberate; it keeps the rule from firing on
/// anything but a destination it positively recognised.
pub(crate) fn body_declarations(body: &[&str]) -> (BTreeSet<String>, BTreeSet<String>) {
    let mut declared = BTreeSet::new();
    let mut static_duration = BTreeSet::new();
    for start in 0..body.len() {
        if start > 0 && !matches!(body[start - 1], "{" | "}" | ";" | "(" | ":") {
            continue;
        }
        if !is_declaration_candidate(body, start) {
            continue;
        }
        let Some(end) = statement_end(body, start) else {
            continue;
        };
        let run = &body[start..end];
        let mut names = BTreeSet::new();
        collect_declared_objects(run, &mut names);
        if run
            .iter()
            .any(|text| STATIC_DURATION_SPECIFIERS.contains(text))
        {
            static_duration.extend(names.iter().cloned());
        }
        declared.extend(names);
    }
    (declared, static_duration)
}

/// The object names one declaration run declares.
fn collect_declared_objects(run: &[&str], names: &mut BTreeSet<String>) {
    if run.first() == Some(&"typedef") {
        return;
    }
    for group in split_top_level_commas(run) {
        let group = truncate_at_initializer(group);
        // Card CTWIN-RESIDUALS: drop a `_Atomic ( type-name )` *specifier*
        // before the `(`-carrying test below, so the declarator it prefixes is
        // read exactly as the unparenthesised spelling's is.
        let group = strip_parenthesised_specifiers(group);
        // A function declarator, a function-pointer declarator and a K&R
        // parameter list all carry `(`. None of them declares an object this
        // rule may name, and skipping the whole group is what keeps a
        // prototype's parameter names out of the set.
        if group.contains(&"(") {
            continue;
        }
        if let Some(name) = declarator_name(&group) {
            names.insert(name.to_owned());
        }
    }
}

/// The declared name of one declarator group, or `None` when the group declares
/// no object (a bare tag, a record or enumerator definition, an unreadable
/// shape).
fn declarator_name<'a>(group: &[&'a str]) -> Option<&'a str> {
    let mut end = group.len();
    // Skip trailing array declarators: `g_slots [ 4 ]`, `g_table [ ] [ 2 ]`.
    loop {
        if group.get(end.checked_sub(1)?) != Some(&"]") {
            break;
        }
        let mut depth = 0usize;
        let mut cursor = end;
        loop {
            cursor = cursor.checked_sub(1)?;
            match group[cursor] {
                "]" => depth += 1,
                "[" => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
        }
        end = cursor;
    }
    // `is_name` already excludes every reserved word, so a run ending in a
    // keyword (`struct S { … }`, `void`) yields no name.
    let name = *group.get(end.checked_sub(1)?)?;
    if !is_name(name) {
        return None;
    }
    // `struct foo;` declares a tag, not an object.
    if end >= 2 && TAG_KEYWORDS.contains(&group[end - 2]) {
        return None;
    }
    Some(name)
}

/// Drop every `KEYWORD ( type-name )` specifier from one declarator group
/// (card `CTWIN-RESIDUALS`).
///
/// Only a keyword in [`PARENTHESISED_SPECIFIERS`] *immediately* followed by
/// `(` is dropped, together with its balanced parenthesis run. Nothing else is
/// touched, and an unbalanced run leaves the group exactly as it was, so this
/// is monotone in the sense [`file_scope_objects`] promises: it can only reveal
/// a declarator, never hide one.
fn strip_parenthesised_specifiers<'a>(group: &[&'a str]) -> Vec<&'a str> {
    let mut kept = Vec::with_capacity(group.len());
    let mut index = 0usize;
    while index < group.len() {
        if PARENTHESISED_SPECIFIERS.contains(&group[index])
            && group.get(index + 1) == Some(&"(")
            && let Some(close) = balanced_close(group, index + 1)
        {
            index = close + 1;
            continue;
        }
        kept.push(group[index]);
        index += 1;
    }
    kept
}

/// The index of the `)` closing the `(` at `open`, when the run is balanced.
fn balanced_close(group: &[&str], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (index, text) in group.iter().enumerate().skip(open) {
        match *text {
            "(" => depth += 1,
            // The subtraction cannot underflow: `open` always points at a `(`,
            // which the first iteration counts, and the scan returns the moment
            // the depth falls back to zero.
            ")" => {
                depth -= 1;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

/// Split a declaration run at its top-level commas.
fn split_top_level_commas<'a, 'b>(run: &'b [&'a str]) -> Vec<&'b [&'a str]> {
    let mut groups = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    for (index, text) in run.iter().enumerate() {
        match *text {
            "(" | "[" | "{" => depth += 1,
            ")" | "]" | "}" => depth = depth.saturating_sub(1),
            "," if depth == 0 => {
                groups.push(&run[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    groups.push(&run[start..]);
    groups
}

/// Drop a declarator group's initializer, which says nothing about the name.
fn truncate_at_initializer<'a, 'b>(group: &'b [&'a str]) -> &'b [&'a str] {
    let mut depth = 0usize;
    for (index, text) in group.iter().enumerate() {
        match *text {
            "(" | "[" | "{" => depth += 1,
            ")" | "]" | "}" => depth = depth.saturating_sub(1),
            "=" if depth == 0 => return &group[..index],
            _ => {}
        }
    }
    group
}

/// One store whose destination is a *continued access* on a publication
/// destination — `g.f = v`, `g->f = v`, `g[i] = v` — recorded by line
/// (card `CTWIN-RESIDUALS`, shape `CTWIN-PUBLISH-INDIRECT`).
///
/// Every such statement is **already** refused: the lexical indirect-access
/// gate, or the unsanctioned-store gate, turns it into an unmodeled
/// instruction, so it was never a `Safe` answer and this record can never make
/// one. What it changes is the *residual*: an unmodeled statement poisons the
/// name bindings but leaves the site's leak claim standing, so the same body
/// that answers `Unknown` when it publishes directly answered provisional
/// `Unsafe` with a `Leak` when it published through a member. The identity may
/// well be released through the published object, so that `Leak` is an
/// overclaim in exactly the sense card `CTWIN-PUBLISH` §3 withdrew for the
/// direct shape.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct IndirectPublication {
    /// The static-storage object the store reaches into, named in the
    /// diagnostic.
    pub place: String,
    /// The identifiers on the stored side. Only sites these names denote are
    /// marked, so an unrelated resource's residual is untouched.
    pub sources: BTreeSet<String>,
}

/// The continued-access stores into a publication destination, keyed by line.
///
/// `body` and `lines` are the same slice of the token stream, texts and lines;
/// `destinations` is [`destinations`]'s answer for the same body. The scan is
/// positive in the same sense the destination scan is: a statement shape it
/// cannot read contributes nothing and keeps its pre-existing answer.
pub(crate) fn indirect_publications(
    body: &[&str],
    lines: &[usize],
    destinations: &BTreeSet<String>,
) -> BTreeMap<usize, IndirectPublication> {
    let mut found = BTreeMap::new();
    if destinations.is_empty() {
        return found;
    }
    // `depth` counts only `(` and `[`: a `;` inside a `for` header or an
    // initializer list does not end a statement, while a brace *always* does,
    // whatever block nesting it sits at.
    let mut depth = 0usize;
    let mut start = 0usize;
    for index in 0..body.len() {
        let boundary = match body[index] {
            "(" | "[" => {
                depth += 1;
                false
            }
            ")" | "]" => {
                depth = depth.saturating_sub(1);
                false
            }
            "{" | "}" => true,
            ";" => depth == 0,
            _ => false,
        };
        if boundary {
            if let Some(publication) = read_indirect_store(&body[start..index], destinations)
                && let Some(line) = lines.get(start)
            {
                found.insert(*line, publication);
            }
            start = index + 1;
        }
    }
    found
}

/// Read one statement as a continued-access store into `destinations`.
fn read_indirect_store(
    statement: &[&str],
    destinations: &BTreeSet<String>,
) -> Option<IndirectPublication> {
    let place = *statement.first()?;
    if !destinations.contains(place) {
        return None;
    }
    // A *bare* name is the direct shape, which `legacy_c::transfer` decides
    // flow-sensitively; only a continued access is this scan's business.
    match (statement.get(1), statement.get(2)) {
        (Some(&"."), _) | (Some(&"["), _) | (Some(&"-"), Some(&">")) => {}
        _ => return None,
    }
    let assign = simple_assignment(statement)?;
    let sources = statement[assign + 1..]
        .iter()
        .filter(|text| is_name(text))
        .map(|text| (*text).to_owned())
        .collect::<BTreeSet<_>>();
    Some(IndirectPublication {
        place: place.to_owned(),
        sources,
    })
}

/// The index of a statement's top-level **simple** assignment `=`.
///
/// Every punctuator is one token here, so `==`, `!=`, `<=`, `>=` and every
/// compound assignment are spelled as two adjacent tokens; each is excluded by
/// looking at the neighbours. A compound assignment is deliberately not a
/// publication: it reads the destination and stores an arithmetic value, never
/// a pointer this projection tracks.
fn simple_assignment(statement: &[&str]) -> Option<usize> {
    const COMPOUND_HEADS: &[&str] = &["=", "!", "<", ">", "+", "-", "*", "/", "%", "&", "|", "^"];
    let mut depth = 0usize;
    for (index, text) in statement.iter().enumerate() {
        match *text {
            "(" | "[" | "{" => depth += 1,
            ")" | "]" | "}" => depth = depth.saturating_sub(1),
            "=" if depth == 0
                && statement.get(index + 1) != Some(&"=")
                && index > 0
                && !COMPOUND_HEADS.contains(&statement[index - 1]) =>
            {
                return Some(index);
            }
            _ => {}
        }
    }
    None
}

/// The publication facts of one function body: the destinations, the
/// continued-access stores into them, and the source names no publication may
/// fire on (card `CTWIN-RESIDUALS`).
///
/// Grouped into one record because all three are read from the same token
/// stream at the same moment and travel together onto
/// [`crate::legacy_c::LegacyCfg`].
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PublicationFacts {
    /// The assignment destinations whose object outlives the call.
    pub destinations: BTreeSet<String>,
    /// Continued-access stores into one of those objects, keyed by line.
    pub indirect: BTreeMap<usize, IndirectPublication>,
    /// Names this function declares as **scalars**.
    ///
    /// A candidate entry custody binds every declared formal to a resource
    /// site, scalar parameters included, so without this exclusion
    /// `g_count = n;` — measured `Safe` before and after by card
    /// `CTWIN-PUBLISH` — would refuse on a modelling artefact rather than on a
    /// published identity. The exclusion is positive and fails closed: a
    /// function whose declarations the type reader could not read contributes
    /// no name here, so nothing is excluded there.
    pub scalar_sources: BTreeSet<String>,
    /// Card **W44-LEAK-PIN-AUDIT**: the places each preprocessed line's
    /// statements hand on, and how, for the refused-escape rule
    /// (`legacy_c::refused_operands`). Read from the same token stream at the
    /// same moment as the three facts above, for the same reason.
    pub line_reads: crate::legacy_c::refused_operands::LineReads,
}

impl PublicationFacts {
    /// Union two contexts' facts, for the contextual-recursion combine.
    ///
    /// Both directions are unions: a destination or a scalar recognised in
    /// either context is recognised in the combination, and a line recorded in
    /// either keeps its record. Refusals therefore only ever accumulate.
    pub fn absorb(&mut self, mut other: Self) {
        self.destinations.append(&mut other.destinations);
        self.scalar_sources.append(&mut other.scalar_sources);
        self.indirect.extend(other.indirect);
        // Card W44-LEAK-PIN-AUDIT: a union per line, the stronger hand-on
        // winning, so a hand-on recognised in either context withdraws in the
        // combination.
        for (line, places) in other.line_reads {
            self.line_reads.entry(line).or_default().absorb(places);
        }
    }
}

/// The publication destinations of one function body.
///
/// `unit` is the whole translation unit's token texts and `body` the tokens
/// between the definition's braces; `formals` are its parameter names.
pub(crate) fn destinations(
    unit: &[&str],
    body: &[&str],
    formals: &BTreeSet<String>,
) -> BTreeSet<String> {
    let (declared, static_duration) = body_declarations(body);
    let mut destinations = static_duration;
    for name in file_scope_objects(unit) {
        if formals.contains(&name) || declared.contains(&name) {
            continue;
        }
        destinations.insert(name);
    }
    destinations
}

/// Every publication fact of one function body, read in one pass.
pub(crate) fn facts(
    unit: &[&str],
    body: &[&str],
    lines: &[usize],
    formals: &BTreeSet<String>,
    types: &crate::typed_field_access::FunctionTypes,
) -> PublicationFacts {
    let destinations = destinations(unit, body, formals);
    let indirect = indirect_publications(body, lines, &destinations);
    // Every identifier this body mentions, plus its formals — whose only
    // occurrence may be in the parameter list `body` excludes.
    let scalar_sources = body
        .iter()
        .map(|text| (*text).to_owned())
        .chain(formals.iter().cloned())
        .filter(|text| is_name(text) && types.declared_is_scalar(text))
        .collect();
    PublicationFacts {
        destinations,
        indirect,
        scalar_sources,
        line_reads: crate::legacy_c::refused_operands::line_reads(body, lines),
    }
}

#[cfg(test)]
#[path = "../tests/unit/publication.rs"]
mod tests;
