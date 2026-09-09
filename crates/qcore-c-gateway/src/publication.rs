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

use std::collections::BTreeSet;

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
        // A function declarator, a function-pointer declarator and a K&R
        // parameter list all carry `(`. None of them declares an object this
        // rule may name, and skipping the whole group is what keeps a
        // prototype's parameter names out of the set.
        if group.contains(&"(") {
            continue;
        }
        if let Some(name) = declarator_name(group) {
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

#[cfg(test)]
#[path = "../tests/unit/publication.rs"]
mod tests;
