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

//! The keys a variadic property setter carries (card
//! **W47-SERVICE-PROPERTY-KEYS**, gateway rule 10p's amendment).
//!
//! Step B's list-head region (card W47-LOOP-OWNER-INSTANCE-FLOW) counts every
//! call whose closure reaches a hazard. A setter such as
//! `flb_service_set(ctx, "Flush", "1", …, NULL)` hands each key to a property
//! function that dispatches on the key's text, and only some keys reach a
//! hazard (`Plugins_File` registers plugins into `config->in_plugins`). This
//! module reads such a call as reaching only its keys' effects, when every
//! part of that reading is derived from the source:
//!
//! 1. **The call's arguments** (typed reading): one argument list recorded at
//!    the call's line, NULL-terminated (the integer constant `0` converted to
//!    a pointer), an even number of key/value arguments before it, no other
//!    null argument, every key a string literal of printable ASCII without a
//!    backslash or quote. Values are not read: no key's reach depends on its
//!    value, because every branch is read whole (a file-path handler's whole
//!    closure, whatever file the value names).
//! 2. **The setter's body**: `va_start(VA, LAST)` on its last named
//!    parameter; exactly two `va_arg(VA, char *)`, the first the loop
//!    condition `while ((KEY = va_arg(…)))`, the second the loop body's first
//!    statement `VALUE = va_arg(…);`; `KEY` and `VALUE` locals assigned there
//!    only, their addresses never taken; `KEY` passed, as one whole argument,
//!    to exactly one call, the property function `P`, inside that loop; no
//!    jump. So `P` sees exactly the keys at even variadic positions, in order,
//!    up to the first null key or value.
//! 3. **The key table** (both readings, cross-checked): `P` walks one table
//!    (`char *KV = TABLE[0].KF; while (KV != NULL) { if (CMP(KV, K, LEN) ==
//!    0) { MATCHED } KV = TABLE[++I].KF; }`, `LEN = strnlen(K, N)`, `KV`,
//!    `I`, `LEN` assigned nowhere else). The typed reading gives the table's
//!    elements under the build (keys after macro expansion, the type
//!    enumerator); the configured lexical reading gives each element's
//!    `offsetof(struct R, FIELD)`; the two must agree element for element
//!    (count, enumerator), the last key being the only null one.
//! 4. **The comparison**, from `CMP`'s body: exactly `size_t L; L =
//!    strnlen(A, N1); if (strncasecmp(A, B, C) == 0 && L == C) { return 0; }
//!    return -1;`. A table key `e` matches a key `k` exactly when, with `n =
//!    min(|k|, N)`, the first `n` bytes agree ignoring ASCII case and
//!    `min(|e|, N1) = n`. Every key matched by no entry keeps every branch.
//! 5. **The branch chain** in `MATCHED`: one `if (!strncasecmp(KV, C, M))
//!    {…} else if (…) {…} … else {…}` chain, every arm a block, each
//!    constant `C` read by the typed reading. An entry takes the first arm
//!    whose comparison its key satisfies, else the final `else`. Everything
//!    in `P` outside the chain's arms is common to every key.
//! 6. **The scalar store** (the final `else`): `switch (TABLE[I].TYPEF)`, each
//!    case's stores `*(T *)((char *)CONFIG + TABLE[I].OFFF)`. An entry that
//!    reaches it is admitted only when its offset names a field of the
//!    property function's own config record other than a listed list member,
//!    declared once in that record with exactly the store's type (`unsigned`
//!    and `signed` ignored), so the store stays inside that field; and no
//!    such pointer arithmetic stands anywhere else in `P`.
//!
//! The call then reaches the functions named (called, taken as a value, or
//! named by an invoked macro's replacement list) in the setter's body and in
//! `P`'s body outside the arms no key selects; the setter and `P` are read in
//! place (their own hazards, and their unmodelled calls, count). Anything
//! else — no typed list, a non-literal key, an odd or unterminated list, an
//! unknown key, an entry this reading cannot classify, a shape that is not
//! the one read, a conditional directive left undecided inside a read body,
//! a field dispatch — keeps the whole call, every branch.
//!
//! Premises stated, not proved (trust boundary 28, beside rule 10p's): the
//! typed reading's argument and initializer extraction; the C library's
//! `strncasecmp` folds the printable ASCII bytes of such keys no more
//! permissively than ASCII case folding (P-KEY-FOLD); `strnlen` as C
//! specifies it; and `va_arg` reads the variadic arguments in order.

use std::collections::{BTreeMap, BTreeSet};

use super::StepB;
use super::loop_owner_instance::{
    NOT_CALLS, address_taken, assignments, call_arguments, declared_local, ordered_parameters,
};
use crate::legacy_c::is_identifier;
use crate::typed_dispatch_census::{ArgumentCall, literal_call_key};

/// Which calls this reading applies to, and what the typed reading records
/// for it. Nothing here states an effect: the setters' bodies, the property
/// function, its table and its comparison are read from the source.
#[derive(Clone, Copy, Debug)]
pub struct PropertyKeyTables {
    /// `(setter, index of its first variadic argument)`.
    pub setters: &'static [(&'static str, usize)],
    /// Property functions whose keyed-branch comparisons the typed reading
    /// records (a setter's property function must be one of them).
    pub property_functions: &'static [&'static str],
    /// The comparison a keyed branch and the comparison function call.
    pub comparison: &'static str,
    /// The bound a key's length is read with.
    pub bound: &'static str,
    /// Key tables whose initializers the typed reading records.
    pub key_tables: &'static [&'static str],
}

/// Fluent Bit at the `90978ac6` export: `src/flb_lib.c` (`flb_service_set`
/// :674, `flb_input_set`, `flb_output_set`, `flb_filter_set`),
/// `src/flb_config.c` (`service_configs[]` :56, `prop_key_check` :782,
/// `flb_config_set_property` :855).
pub const FLUENT_BIT_PROPERTY_KEYS: PropertyKeyTables = PropertyKeyTables {
    setters: &[
        ("flb_service_set", 1),
        ("flb_input_set", 2),
        ("flb_output_set", 2),
        ("flb_filter_set", 2),
    ],
    property_functions: &["flb_config_set_property"],
    comparison: "strncasecmp",
    bound: "strnlen",
    key_tables: &["service_configs"],
};

impl PropertyKeyTables {
    /// The calls whose arguments the typed reading records.
    #[must_use]
    pub fn argument_calls(&self) -> Vec<ArgumentCall> {
        self.setters
            .iter()
            .map(|(setter, first)| ArgumentCall {
                callee: (*setter).to_owned(),
                first: *first,
                caller: None,
            })
            .chain(self.property_functions.iter().map(|function| ArgumentCall {
                callee: self.comparison.to_owned(),
                first: 0,
                caller: Some((*function).to_owned()),
            }))
            .collect()
    }
}

/// What a keyed setter call reaches, when its keys are read.
#[derive(Debug)]
pub(super) struct KeyedCall {
    /// The functions the call's keys reach directly.
    pub roots: BTreeSet<usize>,
    /// The setter and its property function, read in place.
    pub in_place: BTreeSet<usize>,
    /// Per key: what it matched and reaches.
    pub keys: Vec<String>,
    /// The derived table, one line per entry.
    pub table: Vec<String>,
    /// Card W47-PREMISES-HOLDER-REGION: the two bodies as scanned (file,
    /// body, the ranges skipped), for the region's macro invocations.
    pub parts: Vec<super::region_macros::RegionPart>,
}

/// One table entry.
#[derive(Clone, Debug)]
struct Entry {
    key: Option<String>,
    kind: String,
    field: Option<String>,
}

/// One arm of the keyed branch chain.
#[derive(Clone, Debug)]
struct Arm {
    constant: Option<(String, usize)>,
    block: (usize, usize),
}

/// A property function's derived shape.
struct Property {
    /// The chain's arms; the last has no constant when the chain ends in a
    /// plain `else`.
    arms: Vec<Arm>,
    entries: Vec<Entry>,
    /// The comparison's bounds: the table key's and the argument key's.
    bounds: (usize, usize),
    /// Per entry: the arm it takes (`None`: none) and, for the scalar arm,
    /// what its stores write, or why the entry cannot be classified.
    effects: Vec<Result<(Option<usize>, String), String>>,
}

type Texts<'a> = Vec<&'a str>;

/// `strncasecmp(a, b, n) == 0` on NUL-free byte strings.
fn fold_equal_prefix(a: &str, b: &str, n: usize) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    for index in 0..n {
        let (x, y) = (a.get(index), b.get(index));
        match (x, y) {
            (None, None) => return true,
            (Some(x), Some(y)) if x.eq_ignore_ascii_case(y) => {}
            _ => return false,
        }
    }
    true
}

/// `CMP(e, k, min(|k|, key_bound)) == 0` as the comparison function reads it.
fn key_matches(entry: &str, key: &str, bounds: (usize, usize)) -> bool {
    let n = key.len().min(bounds.1);
    fold_equal_prefix(entry, key, n) && entry.len().min(bounds.0) == n
}

/// Whether `range` has a `goto` or a statement label (`ident :` that is not a
/// `case` value, a `default` label or a `?:` arm): a jump this reading does
/// not follow.
fn jumps(texts: &[&str], range: (usize, usize)) -> bool {
    (range.0..=range.1).any(|index| match texts[index] {
        "goto" => true,
        text => {
            is_identifier(text)
                && text != "default"
                && texts.get(index + 1) == Some(&":")
                && texts.get(index + 2) != Some(&":")
                && !matches!(texts.get(index.wrapping_sub(1)), Some(&"?" | &"case"))
        }
    })
}

fn number(text: &str) -> Option<usize> {
    text.parse().ok()
}

fn matching(texts: &[&str], open: usize, left: &str, right: &str) -> Option<usize> {
    let mut depth = 0usize;
    for (index, text) in texts.iter().enumerate().skip(open) {
        if *text == left {
            depth += 1;
        } else if *text == right {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some(index);
            }
        }
    }
    None
}

/// The number of named parameters of a variadic definition (its list ends
/// in `...`).
fn variadic_fixed(texts: &[&str], header: usize) -> Option<usize> {
    let (parts, _) = call_arguments(texts, header + 1)?;
    let (last, named) = parts.split_last()?;
    (texts[last.0..last.1] == [".", ".", "."]).then_some(named.len())
}

/// Whether `pattern` (with `$` slots) matches `texts` at `at`, filling the
/// slots in order.
fn shape<'a>(texts: &[&'a str], at: usize, pattern: &[&str]) -> Option<Vec<&'a str>> {
    let mut slots = Vec::new();
    for (offset, want) in pattern.iter().enumerate() {
        let got = *texts.get(at + offset)?;
        if *want == "$" {
            slots.push(got);
        } else if got != *want {
            return None;
        }
    }
    Some(slots)
}

impl StepB<'_> {
    fn definition_of(&self, id: usize) -> Option<&super::Definition> {
        self.definitions
            .iter()
            .find(|d| self.index[&d.census_name] == id)
    }

    /// A body this reading may read: its file structured, no undecided
    /// conditional directive inside it.
    fn readable(&self, id: usize, range: (usize, usize)) -> Result<(), String> {
        let definition = self
            .definition_of(id)
            .ok_or_else(|| format!("`{}` is not read", self.functions[id]))?;
        if self.reach.unstructured.contains(&id) {
            return Err(format!(
                "`{}`'s file has a conditional group this reader cannot structure",
                self.functions[id]
            ));
        }
        if self.scans[definition.file]
            .directives
            .iter()
            .any(|at| range.0 < *at && *at <= range.1)
        {
            return Err(format!(
                "`{}` has an undecided conditional group",
                self.functions[id]
            ));
        }
        Ok(())
    }

    /// The census ids a macro invoked from `file` names, through nested
    /// macros (card W47-SERVICE-PROPERTY-KEYS).
    pub(super) fn macro_callees(&self, file: usize, name: &str) -> BTreeSet<usize> {
        let mut found = BTreeSet::new();
        let mut seen = BTreeSet::new();
        let mut stack = vec![name.to_owned()];
        while let Some(macro_name) = stack.pop() {
            if !seen.insert(macro_name.clone()) {
                continue;
            }
            for word in self.macro_bodies.get(&macro_name).into_iter().flatten() {
                let ids = self.resolve_name(file, word);
                if ids.is_empty() {
                    stack.push(word.clone());
                }
                found.extend(ids);
            }
        }
        found
    }

    /// The keyed reading of a setter call whose name is at token `k` of
    /// `file`, when the name is a listed setter.
    pub(super) fn keyed_setter_call(
        &self,
        file: usize,
        k: usize,
    ) -> Option<Result<KeyedCall, String>> {
        let texts = self.texts(file);
        let name = texts[k];
        let (_, first) = *self.keys.setters.iter().find(|(s, _)| *s == name)?;
        if texts.get(k + 1) != Some(&"(") || matches!(texts[k - 1], "." | ">") {
            return None;
        }
        Some(self.keyed_call(file, k, name, first))
    }

    fn keyed_call(
        &self,
        file: usize,
        k: usize,
        name: &str,
        first: usize,
    ) -> Result<KeyedCall, String> {
        let typed = self.typed.ok_or("no typed reading")?;
        let [setter] = self.resolve_name(file, name)[..] else {
            return Err(format!("`{name}` is not defined once"));
        };
        let s = self.definition_of(setter).ok_or("the setter is not read")?;
        self.readable(setter, s.body)?;
        let (property, property_call) = self.setter_shape(s, first)?;
        let keys = self.literal_keys(typed, file, k, name)?;
        if !self
            .keys
            .property_functions
            .contains(&self.plain_name(property))
        {
            return Err(format!(
                "`{name}` passes its keys to `{}`, whose keyed branches the typed reading does not \
                 record",
                self.functions[property]
            ));
        }
        let p = self
            .definition_of(property)
            .ok_or("the property function is not read")?;
        self.readable(property, p.body)?;
        let key_argument = property_call.1;
        let shape = self.property_shape(p, key_argument)?;
        let mut selected = BTreeSet::new();
        let mut described = Vec::new();
        for key in &keys {
            if key.len() >= shape.bounds.0.min(shape.bounds.1) {
                return Err(format!(
                    "the key \"{key}\" is not shorter than the comparison's bound"
                ));
            }
            let matched = shape
                .entries
                .iter()
                .enumerate()
                .filter(|(_, e)| {
                    e.key
                        .as_deref()
                        .is_some_and(|e| key_matches(e, key, shape.bounds))
                })
                .collect::<Vec<_>>();
            if matched.is_empty() {
                return Err(format!(
                    "the key \"{key}\" matches no entry of the derived table"
                ));
            }
            let mut reach = Vec::new();
            for (index, entry) in matched {
                let (arm, what) = shape.effects[index].clone().map_err(|why| {
                    format!(
                        "the key \"{key}\" matches an entry this reading cannot classify: {why}"
                    )
                })?;
                selected.extend(arm);
                reach.push(format!(
                    "`{}` ({}): {what}",
                    entry.key.as_deref().unwrap_or(""),
                    entry.kind
                ));
            }
            described.push(format!("\"{key}\" → {}", reach.join(", ")));
        }
        // The scan: the setter's body but its call of the property function,
        // and the property function's body but the arms no key selects.
        let mut skipped = shape
            .arms
            .iter()
            .enumerate()
            .filter(|(index, _)| !selected.contains(index))
            .map(|(_, arm)| arm.block)
            .collect::<Vec<_>>();
        let mut roots = BTreeSet::new();
        self.scan_named(
            s.file,
            s.body,
            &[(property_call.0, property_call.0)],
            &mut roots,
        )?;
        skipped.sort_unstable();
        self.scan_named(p.file, p.body, &skipped, &mut roots)?;
        let parts = vec![
            (s.file, s.body, vec![(property_call.0, property_call.0)]),
            (p.file, p.body, skipped.clone()),
        ];
        roots.remove(&setter);
        roots.remove(&property);
        let table = shape
            .entries
            .iter()
            .zip(&shape.effects)
            .map(|(entry, effect)| {
                format!(
                    "{} | {} | {} | {}",
                    entry.key.as_deref().unwrap_or("NULL"),
                    entry.kind,
                    entry.field.as_deref().unwrap_or("-"),
                    match effect {
                        Ok((_, what)) => what.clone(),
                        Err(why) => format!("unclassified: {why}"),
                    }
                )
            })
            .collect();
        Ok(KeyedCall {
            roots,
            in_place: BTreeSet::from([setter, property]),
            keys: described,
            table,
            parts,
        })
    }

    fn plain_name(&self, id: usize) -> &str {
        let name = &self.functions[id];
        name.rsplit_once("::")
            .map_or(name.as_str(), |(_, plain)| plain)
    }

    /// Every function named in `range` of `file` outside `skipped`: called,
    /// taken as a value, or named by an invoked macro. A field dispatch, or a
    /// call only an unstructured file may define, refuses.
    fn scan_named(
        &self,
        file: usize,
        range: (usize, usize),
        skipped: &[(usize, usize)],
        roots: &mut BTreeSet<usize>,
    ) -> Result<(), String> {
        let texts = self.texts(file);
        let hidden = self.hidden_definitions();
        let path = &self.files[file].path;
        for at in range.0..=range.1 {
            if skipped.iter().any(|(a, b)| *a <= at && at <= *b) {
                continue;
            }
            let text = texts[at];
            if !is_identifier(text) || NOT_CALLS.contains(&text) {
                continue;
            }
            let call = texts.get(at + 1) == Some(&"(");
            if matches!(texts[at - 1], "." | ">") {
                if call {
                    return Err(format!(
                        "a field dispatch at {path}:{} is in a read body",
                        self.scans[file].tokens[at].line
                    ));
                }
                continue;
            }
            let callees = self.resolve_name(file, text);
            if callees.is_empty() {
                if call && let Some(defined_in) = hidden.get(text) {
                    return Err(format!(
                        "`{text}` may be defined only in {defined_in}, a file this reader \
                         cannot structure"
                    ));
                }
                roots.extend(self.macro_callees(file, text));
                continue;
            }
            roots.extend(callees);
        }
        Ok(())
    }

    /// The setter's derived shape: its property function, and the position
    /// of its call's name with the index of the key argument.
    fn setter_shape(
        &self,
        s: &super::Definition,
        first: usize,
    ) -> Result<(usize, (usize, usize)), String> {
        let texts = self.texts(s.file);
        let name = &s.name;
        let body = s.body;
        let fail = |why: &str| Err(format!("`{name}`'s body is not the read shape: {why}"));
        if variadic_fixed(&texts, s.header) != Some(first) {
            return fail("its named parameters are not the listed ones before `...`");
        }
        let parameters = ordered_parameters(&texts, s.header);
        let Some(last) = parameters.get(first - 1) else {
            return fail("no last named parameter");
        };
        if jumps(&texts, body) {
            return fail("a jump");
        }
        let at = |word: &str| {
            (body.0..=body.1)
                .filter(|&i| texts[i] == word && texts.get(i + 1) == Some(&"("))
                .collect::<Vec<_>>()
        };
        if !at("va_copy").is_empty() {
            return fail("`va_copy`");
        }
        let [start] = at("va_start")[..] else {
            return fail("not one `va_start`");
        };
        let [va_arg_key, va_arg_value] = at("va_arg")[..] else {
            return fail("not two `va_arg`");
        };
        let Some(start_slots) = shape(&texts, start, &["va_start", "(", "$", ",", "$", ")", ";"])
        else {
            return fail("`va_start` is not `va_start(VA, LAST);`");
        };
        let (list, started) = (start_slots[0], start_slots[1]);
        if started != last {
            return fail("`va_start` does not name the last named parameter");
        }
        let Some(key_slots) = va_arg_key.checked_sub(5).and_then(|from| {
            shape(
                &texts,
                from,
                &[
                    "while", "(", "(", "$", "=", "va_arg", "(", "$", ",", "char", "*", ")", ")",
                    ")", "{", "$", "=", "va_arg", "(", "$", ",", "char", "*", ")", ";",
                ],
            )
        }) else {
            return fail(
                "the loop is not `while ((KEY = va_arg(VA, char *))) { VALUE = va_arg(VA, char *); …`",
            );
        };
        let (key, value) = (key_slots[0], key_slots[2]);
        if key_slots[1] != list || key_slots[3] != list || va_arg_value != va_arg_key + 12 {
            return fail("the two `va_arg` read another list");
        }
        let open = va_arg_key + 9;
        let Some(close) = matching(&texts, open, "{", "}") else {
            return fail("the loop's block is not read");
        };
        for (local, at) in [(key, va_arg_key - 2), (value, va_arg_key + 10)] {
            if key == value
                || !is_identifier(local)
                || !declared_local(&texts, s, local)
                || assignments(&texts, body, local) != [at]
                || address_taken(&texts, body, local)
            {
                return fail("the key or value is not a local assigned only by its `va_arg`");
            }
        }
        // Every other use of the key: its declaration, or one whole argument
        // of one call inside the loop.
        let mut calls = Vec::new();
        for i in body.0..=body.1 {
            if texts[i] != key || matches!(texts[i - 1], "." | ">") || i == va_arg_key - 2 {
                continue;
            }
            let declaration = i < open
                && matches!(texts[i + 1], ";" | ",")
                && (texts[i - 1] == "*" || is_identifier(texts[i - 1]));
            if declaration {
                continue;
            }
            let whole = matches!(texts[i - 1], "(" | ",") && matches!(texts[i + 1], ")" | ",");
            if !(whole && open < i && i < close) {
                return fail("the key is used other than as one argument of one call");
            }
            // The call whose argument list holds it.
            let mut depth = 0usize;
            let mut paren = None;
            for j in (open..i).rev() {
                match texts[j] {
                    ")" => depth += 1,
                    "(" if depth == 0 => {
                        paren = Some(j);
                        break;
                    }
                    "(" => depth -= 1,
                    _ => {}
                }
            }
            let Some(paren) = paren else {
                return fail("the key's call is not read");
            };
            let callee = paren - 1;
            if !is_identifier(texts[callee]) || matches!(texts[callee - 1], "." | ">") {
                return fail("the key is passed to a call that is not by name");
            }
            let Some((arguments, _)) = call_arguments(&texts, paren) else {
                return fail("the key's call is not read");
            };
            let Some(index) = arguments.iter().position(|(a, b)| *a == i && *b == i + 1) else {
                return fail("the key is not one whole argument");
            };
            calls.push((callee, index));
        }
        let [(callee, index)] = calls[..] else {
            return fail("the key is not passed to exactly one call");
        };
        let [property] = self.resolve_name(s.file, texts[callee])[..] else {
            return fail("the key's callee is not defined once");
        };
        Ok((property, (callee, index)))
    }

    /// The call's key sequence, from the typed reading.
    fn literal_keys(
        &self,
        typed: &crate::typed_dispatch_census::TypedFacts,
        file: usize,
        k: usize,
        name: &str,
    ) -> Result<Vec<String>, String> {
        let texts = self.texts(file);
        let path = &self.files[file].path;
        let line = self.scans[file].tokens[k].line;
        let here = (0..texts.len())
            .filter(|&i| {
                texts[i] == name
                    && texts.get(i + 1) == Some(&"(")
                    && !matches!(texts[i - 1], "." | ">")
                    && self.scans[file].tokens[i].line == line
            })
            .count();
        let lists = typed
            .argument_lists
            .get(&literal_call_key(path, line, name))
            .map(|lists| lists.iter().collect::<Vec<_>>())
            .unwrap_or_default();
        let [list] = lists[..] else {
            return Err(format!(
                "the typed reading records {} argument lists for `{name}` at {path}:{line}",
                lists.len()
            ));
        };
        if here != 1 {
            return Err(format!("{here} calls of `{name}` stand at {path}:{line}"));
        }
        let Some((last, pairs)) = list.split_last() else {
            return Err("the argument list is empty".to_owned());
        };
        if last != "null" {
            return Err("the argument list is not NULL-terminated".to_owned());
        }
        if pairs.len() % 2 == 1 {
            return Err("the argument list has an odd number of key/value arguments".to_owned());
        }
        let mut keys = Vec::new();
        for (index, pair) in pairs.chunks(2).enumerate() {
            let Some(key) = pair[0].strip_prefix("s:") else {
                return Err(format!(
                    "the key of pair {} is not a string literal (`{}`)",
                    index + 1,
                    pair[0]
                ));
            };
            if pair[1] == "null" {
                return Err(format!("the value of pair {} is a null pointer", index + 1));
            }
            if key.is_empty()
                || !key
                    .bytes()
                    .all(|b| (0x20..0x7f).contains(&b) && b != b'\\' && b != b'"')
            {
                return Err(format!(
                    "the key \"{key}\" is not printable ASCII without a backslash or quote"
                ));
            }
            keys.push(key.to_owned());
        }
        Ok(keys)
    }

    /// The property function's table, comparison, branch chain and scalar
    /// stores.
    #[allow(clippy::too_many_lines)]
    fn property_shape(
        &self,
        p: &super::Definition,
        key_argument: usize,
    ) -> Result<Property, String> {
        let typed = self.typed.ok_or("no typed reading")?;
        let texts: Texts<'_> = self.texts(p.file);
        let body = p.body;
        let path = &self.files[p.file].path;
        let name = &p.name;
        let fail = |why: String| Err(format!("`{name}`'s body is not the read shape: {why}"));
        let parameters = ordered_parameters(&texts, p.header);
        let Some(k_param) = parameters.get(key_argument).map(String::as_str) else {
            return fail("no key parameter".to_owned());
        };
        let config = parameters.first().map(String::as_str).unwrap_or_default();
        if jumps(&texts, body) {
            return fail("a jump".to_owned());
        }
        // `while (KV != NULL) {`.
        let whiles = (body.0..=body.1)
            .filter(|&i| texts[i] == "while")
            .collect::<Vec<_>>();
        let [w] = whiles[..] else {
            return fail("not one loop".to_owned());
        };
        let Some(w_slots) = shape(&texts, w, &["while", "(", "$", "!", "=", "NULL", ")", "{"])
        else {
            return fail("the loop is not `while (KV != NULL) {`".to_owned());
        };
        let kv = w_slots[0];
        let lo = w + 7;
        let lc = matching(&texts, lo, "{", "}").ok_or("the loop is not read")?;
        // `if (CMP(KV, K, LEN) == 0) { MATCHED }` then `KV = TABLE[++I].KF;`.
        let Some(if_slots) = shape(
            &texts,
            lo + 1,
            &[
                "if", "(", "$", "(", "$", ",", "$", ",", "$", ")", "=", "=", "0", ")", "{",
            ],
        ) else {
            return fail("the loop does not open with `if (CMP(KV, K, LEN) == 0) {`".to_owned());
        };
        let (cmp, len) = (if_slots[0], if_slots[3]);
        if if_slots[1] != kv || if_slots[2] != k_param {
            return fail("the comparison does not compare the table key with the key".to_owned());
        }
        let mo = lo + 15;
        let mc = matching(&texts, mo, "{", "}").ok_or("the matched block is not read")?;
        let Some(inc) = shape(
            &texts,
            mc + 1,
            &["$", "=", "$", "[", "+", "+", "$", "]", ".", "$", ";", "}"],
        ) else {
            return fail("the loop does not end in `KV = TABLE[++I].KF;`".to_owned());
        };
        if inc[0] != kv || mc + 12 != lc {
            return fail("the loop's increment is not the key's".to_owned());
        }
        let (table, index, key_field) = (inc[1], inc[2], inc[3]);
        let kv_assigned = assignments(&texts, body, kv);
        let [init, step] = kv_assigned[..] else {
            return fail("the table key is not assigned exactly twice".to_owned());
        };
        if step != mc + 1
            || init >= w
            || shape(
                &texts,
                init,
                &[kv, "=", table, "[", "0", "]", ".", key_field, ";"],
            )
            .is_none()
            || address_taken(&texts, body, kv)
        {
            return fail("the table key does not start at the table's first element".to_owned());
        }
        let index_assigned = assignments(&texts, body, index);
        let increments = (body.0..=body.1)
            .filter(|&i| {
                texts[i] == index
                    && ((matches!(texts[i - 1], "+" | "-") && texts[i - 2] == texts[i - 1])
                        || (matches!(texts[i + 1], "+" | "-") && texts[i + 2] == texts[i + 1]))
            })
            .collect::<Vec<_>>();
        if !matches!(index_assigned[..], [at] if at < w && shape(&texts, at, &[index, "=", "0", ";"]).is_some())
            || increments != [step + 6]
            || address_taken(&texts, body, index)
        {
            return fail(
                "the table index is not zero then incremented once per element".to_owned(),
            );
        }
        let len_assigned = assignments(&texts, body, len);
        let [len_at] = len_assigned[..] else {
            return fail("the key's length is not assigned once".to_owned());
        };
        let Some(len_slots) = shape(
            &texts,
            len_at,
            &[len, "=", self.keys.bound, "(", k_param, ",", "$", ")", ";"],
        ) else {
            return fail("the key's length is not `strnlen(K, N)`".to_owned());
        };
        let key_bound = number(len_slots[0]).ok_or("the key's bound is not a number")?;
        if len_at >= w || address_taken(&texts, body, len) {
            return fail("the key's length is not read before the loop".to_owned());
        }
        let table_bound = self.comparison_shape(p.file, cmp)?;
        let entries = self.table_entries(typed, table, key_field)?;
        // The keyed chain in the matched block.
        let comparison = self.keys.comparison;
        let mut chains = Vec::new();
        let mut at = mo + 1;
        while at < mc {
            if texts[at] == "if"
                && texts.get(at + 2) == Some(&"!")
                && texts.get(at + 3) == Some(&comparison)
            {
                let (arms, end) = self.chain(p.file, &texts, at, kv)?;
                chains.push(arms);
                at = end + 1;
                continue;
            }
            if texts[at] == comparison && texts.get(at + 2) == Some(&kv) {
                return fail(format!(
                    "a comparison of the table key outside the keyed chain at {path}:{}",
                    self.scans[p.file].tokens[at].line
                ));
            }
            at += 1;
        }
        let [arms] = &chains[..] else {
            return fail("not one keyed branch chain".to_owned());
        };
        // The scalar stores, all in the final `else`.
        let stores = self.scalar_stores(&texts, body, arms, config, table, index)?;
        let config_record = self.flow.config_record;
        let mut effects = Vec::new();
        for entry in &entries {
            let Some(key) = &entry.key else {
                effects.push(Ok((None, "the table's end".to_owned())));
                continue;
            };
            let arm = arms.iter().position(|arm| match &arm.constant {
                Some((constant, bound)) => fold_equal_prefix(key, constant, *bound),
                None => true,
            });
            let effect = match arm {
                None => Ok((None, "no arm (the chain has no final else)".to_owned())),
                Some(j) if arms[j].constant.is_some() => {
                    let (constant, bound) = arms[j].constant.clone().unwrap_or_default();
                    Ok((
                        Some(j),
                        format!(
                            "keyed arm {} (`{constant}`, {bound}) at {path}:{}",
                            j + 1,
                            self.scans[p.file].tokens[arms[j].block.0].line
                        ),
                    ))
                }
                Some(j) => match stores.get(&entry.kind).or_else(|| stores.get("")) {
                    None => Err(format!(
                        "the scalar arm's switch reads no case for `{}`",
                        entry.kind
                    )),
                    Some(casts) if casts.is_empty() => {
                        Ok((Some(j), "the scalar arm, no store".to_owned()))
                    }
                    Some(casts) => {
                        let Some(field) = entry.field.as_deref() else {
                            effects.push(Err("its offset names no field".to_owned()));
                            continue;
                        };
                        let Some((record, member)) = field.split_once("::") else {
                            effects.push(Err("its offset names no field".to_owned()));
                            continue;
                        };
                        if record != config_record
                            || member == self.flow.list_field
                            || member == self.flow.registrations_field
                        {
                            effects.push(Err(format!("its offset names `{field}`")));
                            continue;
                        }
                        match self.field_type(record, member) {
                            Err(why) => Err(why),
                            Ok(declared) => {
                                if casts.iter().all(|cast| *cast == declared) {
                                    Ok((
                                        Some(j),
                                        format!("scalar store into `{field}` ({declared})"),
                                    ))
                                } else {
                                    Err(format!(
                                        "its store ({}) is not `{field}`'s type ({declared})",
                                        casts.join(", ")
                                    ))
                                }
                            }
                        }
                    }
                },
            };
            effects.push(effect);
        }
        Ok(Property {
            arms: arms.clone(),
            entries,
            bounds: (table_bound, key_bound),
            effects,
        })
    }

    /// `CMP`'s body, exactly the read comparison; its table-key bound.
    fn comparison_shape(&self, file: usize, cmp: &str) -> Result<usize, String> {
        let [id] = self.resolve_name(file, cmp)[..] else {
            return Err(format!("the comparison `{cmp}` is not defined once"));
        };
        let definition = self.definition_of(id).ok_or("the comparison is not read")?;
        self.readable(id, definition.body)?;
        let texts = self.texts(definition.file);
        let parameters = ordered_parameters(&texts, definition.header);
        let [a, b, c] = &parameters[..] else {
            return Err(format!("`{cmp}` does not take three parameters"));
        };
        let body = &texts[definition.body.0..=definition.body.1];
        let comparison = self.keys.comparison;
        let bound = self.keys.bound;
        let Some(slots) = shape(
            body,
            0,
            &[
                "{", "size_t", "$", ";", "$", "=", bound, "(", a, ",", "$", ")", ";", "if", "(",
                comparison, "(", a, ",", b, ",", c, ")", "=", "=", "0", "&", "&", "$", "=", "=", c,
                ")", "{", "return", "0", ";", "}", "return", "-", "1", ";", "}",
            ],
        ) else {
            return Err(format!("`{cmp}`'s body is not the read comparison"));
        };
        if slots[0] != slots[1] || slots[0] != slots[3] || body.len() != 43 {
            return Err(format!("`{cmp}`'s body is not the read comparison"));
        }
        number(slots[2]).ok_or_else(|| format!("`{cmp}`'s bound is not a number"))
    }

    /// One keyed branch chain starting at `if`: its arms and its last token.
    fn chain(
        &self,
        file: usize,
        texts: &[&str],
        start: usize,
        kv: &str,
    ) -> Result<(Vec<Arm>, usize), String> {
        let typed = self.typed.ok_or("no typed reading")?;
        let path = &self.files[file].path;
        let comparison = self.keys.comparison;
        let mut arms = Vec::new();
        let mut at = start;
        loop {
            let line = self.scans[file].tokens[at].line;
            let Some(slots) = shape(
                texts,
                at,
                &[
                    "if", "(", "!", comparison, "(", kv, ",", "$", ",", "$", ")", ")", "{",
                ],
            ) else {
                return Err(format!(
                    "the keyed branch at {path}:{line} is not `if (!{comparison}(KV, C, N)) {{`"
                ));
            };
            let lexical_bound = number(slots[1]).ok_or("a keyed branch's bound is not a number")?;
            let calls_here = (0..texts.len())
                .filter(|&i| {
                    texts[i] == comparison
                        && texts.get(i + 1) == Some(&"(")
                        && self.scans[file].tokens[i].line == line
                })
                .count();
            let lists = typed
                .argument_lists
                .get(&literal_call_key(path, line, comparison))
                .map(|lists| lists.iter().collect::<Vec<_>>())
                .unwrap_or_default();
            let constant = match (&lists[..], calls_here) {
                ([list], 1) => match &list[..] {
                    [v, s, i]
                        if v.strip_prefix("v:") == Some(kv)
                            && i.strip_prefix("i:").and_then(number) == Some(lexical_bound) =>
                    {
                        s.strip_prefix("s:")
                            .filter(|s| !s.contains('\\'))
                            .map(str::to_owned)
                    }
                    _ => None,
                },
                _ => None,
            };
            let Some(constant) = constant else {
                return Err(format!(
                    "the typed reading does not read the keyed branch's constant at {path}:{line}"
                ));
            };
            let open = at + 12;
            let close = matching(texts, open, "{", "}").ok_or("a keyed arm is not read")?;
            arms.push(Arm {
                constant: Some((constant, lexical_bound)),
                block: (open, close),
            });
            if texts.get(close + 1) != Some(&"else") {
                return Ok((arms, close));
            }
            match texts.get(close + 2) {
                Some(&"if") => at = close + 2,
                Some(&"{") => {
                    let end =
                        matching(texts, close + 2, "{", "}").ok_or("the final arm is not read")?;
                    arms.push(Arm {
                        constant: None,
                        block: (close + 2, end),
                    });
                    return Ok((arms, end));
                }
                _ => return Err(format!("an `else` at {path}:{line} is not a block")),
            }
        }
    }

    /// The final `else`'s `switch (TABLE[I].TYPEF)`: per case label, the
    /// store types its statements cast to. Every `(char *)CONFIG +` in the
    /// body must be one of these stores.
    fn scalar_stores(
        &self,
        texts: &[&str],
        body: (usize, usize),
        arms: &[Arm],
        config: &str,
        table: &str,
        index: &str,
    ) -> Result<BTreeMap<String, Vec<String>>, String> {
        let arithmetic = (body.0..=body.1)
            .filter(|&i| shape(texts, i, &["(", "char", "*", ")", config, "+"]).is_some())
            .collect::<Vec<_>>();
        let Some(last) = arms.last().filter(|arm| arm.constant.is_none()) else {
            return if arithmetic.is_empty() {
                Ok(BTreeMap::new())
            } else {
                Err("pointer arithmetic on the config outside a final else".to_owned())
            };
        };
        let switches = (last.block.0..=last.block.1)
            .filter(|&i| texts[i] == "switch")
            .collect::<Vec<_>>();
        let [sw] = switches[..] else {
            return Err("the final else has not one switch".to_owned());
        };
        let Some(slots) = shape(
            texts,
            sw,
            &["switch", "(", table, "[", index, "]", ".", "$", ")", "{"],
        ) else {
            return Err("the final else's switch is not on the element's type".to_owned());
        };
        let _type_field = slots[0];
        let open = sw + 9;
        let close = matching(texts, open, "{", "}").ok_or("the switch is not read")?;
        // Labels at the switch's own depth.
        let mut labels: Vec<(Option<String>, usize)> = Vec::new();
        let mut depth = 0usize;
        for i in open + 1..close {
            match texts[i] {
                "{" | "(" => depth += 1,
                "}" | ")" => depth -= 1,
                "case" if depth == 0 && texts.get(i + 2) == Some(&":") => {
                    labels.push((Some(texts[i + 1].to_owned()), i));
                }
                "default" if depth == 0 && texts.get(i + 1) == Some(&":") => {
                    labels.push((None, i));
                }
                _ => {}
            }
        }
        let mut stores: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut used = BTreeSet::new();
        let mut default = None;
        for (label, from) in &labels {
            // From the label to the first `break;` at the switch's depth.
            let mut depth = 0usize;
            let mut end = close;
            for (i, text) in texts.iter().enumerate().take(close).skip(*from) {
                match *text {
                    "{" | "(" => depth += 1,
                    "}" | ")" => depth = depth.saturating_sub(1),
                    "break" if depth == 0 => {
                        end = i;
                        break;
                    }
                    "return" | "goto" | "continue" if depth == 0 => {
                        return Err("a case leaves the switch other than by break".to_owned());
                    }
                    _ => {}
                }
            }
            let mut casts = Vec::new();
            for &a in arithmetic.iter().filter(|a| *from < **a && **a < end) {
                // `( T * ) ( ( char * ) CONFIG + TABLE [ I ] . OFF )`.
                if texts.get(a - 1) != Some(&"(")
                    || texts.get(a + 6..a + 13).is_none_or(|t| {
                        t[0] != table
                            || t[1] != "["
                            || t[2] != index
                            || t[3] != "]"
                            || t[4] != "."
                            || t[6] != ")"
                    })
                {
                    return Err(
                        "a store's pointer arithmetic is not `(char *)CONFIG + TABLE[I].OFF`"
                            .to_owned(),
                    );
                }
                // The cast before it: `( T * )`.
                if texts.get(a - 2) != Some(&")") || texts.get(a - 3) != Some(&"*") {
                    return Err("a store is not cast to a pointer".to_owned());
                }
                let mut b = a - 3;
                while b > 0 && texts[b - 1] != "(" {
                    b -= 1;
                }
                casts.push(texts[b..a - 3].join(" "));
                used.insert(a);
            }
            match label {
                Some(label) => {
                    stores.insert(label.clone(), casts);
                }
                None => default = Some(casts),
            }
        }
        if used.len() != arithmetic.len() {
            return Err("pointer arithmetic on the config outside the switch's stores".to_owned());
        }
        if let Some(casts) = default {
            stores.insert(String::new(), casts);
        }
        Ok(stores)
    }

    /// A field's declared type in a record read once: the tokens before its
    /// name, `unsigned`, `signed` and `const` dropped.
    fn field_type(&self, record: &str, field: &str) -> Result<String, String> {
        let tag = record
            .strip_prefix("struct ")
            .ok_or_else(|| format!("`{record}` is not a struct"))?;
        let mut found = Vec::new();
        for file in 0..self.scans.len() {
            let texts = self.texts(file);
            for i in 1..texts.len() {
                if texts[i] == tag && texts[i - 1] == "struct" && texts.get(i + 1) == Some(&"{") {
                    found.push((file, i + 1));
                }
            }
        }
        let [(file, open)] = found[..] else {
            return Err(format!("`{record}` is not defined once"));
        };
        let texts = self.texts(file);
        let close = matching(&texts, open, "{", "}").ok_or("the record is not read")?;
        // An undecided group inside the record may declare a field in both
        // arms: each declaration is counted, and two refuse.
        let mut declarations = Vec::new();
        let mut depth = 0usize;
        let mut statement = open + 1;
        for i in open + 1..close {
            match texts[i] {
                "{" | "(" | "[" => depth += 1,
                "}" | ")" | "]" => depth -= 1,
                ";" if depth == 0 => {
                    if i > statement && texts[i - 1] == field {
                        declarations.push(
                            texts[statement..i - 1]
                                .iter()
                                .filter(|t| !matches!(**t, "unsigned" | "signed" | "const"))
                                .copied()
                                .collect::<Vec<_>>()
                                .join(" "),
                        );
                    }
                    statement = i + 1;
                }
                _ => {}
            }
        }
        let [declared] = &declarations[..] else {
            return Err(format!("`{record}::{field}` is not declared once"));
        };
        Ok(declared.clone())
    }

    /// The table's elements: keys and kinds from the typed reading, each
    /// element's `offsetof` field from the configured lexical reading.
    fn table_entries(
        &self,
        typed: &crate::typed_dispatch_census::TypedFacts,
        table: &str,
        key_field: &str,
    ) -> Result<Vec<Entry>, String> {
        if !self.keys.key_tables.contains(&table) {
            return Err(format!(
                "the typed reading does not record the table `{table}`"
            ));
        }
        let readings = typed
            .key_tables
            .iter()
            .filter(|(key, _)| key.rsplit_once('|').is_some_and(|(_, name)| name == table))
            .flat_map(|(_, readings)| readings.iter())
            .collect::<Vec<_>>();
        let [typed_elements] = readings[..] else {
            return Err(format!(
                "the typed reading records {} readings of `{table}`",
                readings.len()
            ));
        };
        // `struct R TABLE [ ] = { … } ;`, once.
        let mut found = Vec::new();
        for file in 0..self.scans.len() {
            let texts = self.texts(file);
            for i in 2..texts.len() {
                if texts[i] == table
                    && texts[i - 2] == "struct"
                    && shape(&texts, i, &[table, "[", "]", "=", "{"]).is_some()
                {
                    found.push((file, i, texts[i - 1].to_owned()));
                }
            }
        }
        let [(file, at, ref tag)] = found[..] else {
            return Err(format!("`{table}` is not defined once"));
        };
        let texts = self.texts(file);
        let open = at + 4;
        let close = matching(&texts, open, "{", "}").ok_or("the table is not read")?;
        if self.scans[file]
            .directives
            .iter()
            .any(|d| open < *d && *d <= close)
        {
            return Err(format!("`{table}` has an undecided conditional group"));
        }
        let slots = self.record_slots(tag)?;
        let key_slot = slots
            .iter()
            .position(|s| s == key_field)
            .ok_or("the key field is not a slot")?;
        let mut lexical = Vec::new();
        let mut i = open + 1;
        while i < close {
            if texts[i] == "," {
                i += 1;
                continue;
            }
            if texts[i] != "{" {
                return Err(format!(
                    "`{table}` has an element that is not a braced list"
                ));
            }
            let end = matching(&texts, i, "{", "}").ok_or("an element is not read")?;
            let (parts, _) = call_arguments(&texts, i).ok_or("an element is not read")?;
            lexical.push(
                parts
                    .iter()
                    .map(|(a, b)| texts[*a..*b].to_vec())
                    .collect::<Vec<_>>(),
            );
            i = end + 1;
        }
        if lexical.len() != typed_elements.len() {
            return Err(format!(
                "`{table}` has {} elements lexically and {} typed",
                lexical.len(),
                typed_elements.len()
            ));
        }
        let mut entries = Vec::new();
        let count = lexical.len();
        for (n, (words, typed_slots)) in lexical.iter().zip(typed_elements).enumerate() {
            if words.len() != slots.len() || typed_slots.len() != slots.len() {
                return Err(format!(
                    "element {} of `{table}` does not fill every slot",
                    n + 1
                ));
            }
            let key = match typed_slots[key_slot].as_str() {
                "null" if n + 1 == count => None,
                text => match text.strip_prefix("s:") {
                    Some(key) if n + 1 < count && !key.contains('\\') => Some(key.to_owned()),
                    _ => return Err(format!("element {} of `{table}` has no literal key", n + 1)),
                },
            };
            let mut kind = String::new();
            let mut field = None;
            for (slot, (word, typed_slot)) in words.iter().zip(typed_slots).enumerate() {
                if slot == key_slot {
                    continue;
                }
                match (typed_slot.as_str(), &word[..]) {
                    (enumerator, [lexical]) if enumerator.strip_prefix("v:") == Some(*lexical) => {
                        kind = (*lexical).to_owned();
                    }
                    ("offsetof", ["offsetof", "(", "struct", record, ",", member, ")"]) => {
                        field = Some(format!("struct {record}::{member}"));
                    }
                    ("i:0", ["0"]) => {}
                    _ => {
                        return Err(format!(
                            "element {} of `{table}`: the typed and lexical readings disagree",
                            n + 1
                        ));
                    }
                }
            }
            entries.push(Entry { key, kind, field });
        }
        Ok(entries)
    }

    /// A record's slot names in order (one declarator per declaration).
    fn record_slots(&self, tag: &str) -> Result<Vec<String>, String> {
        let mut found = Vec::new();
        for file in 0..self.scans.len() {
            let texts = self.texts(file);
            for i in 1..texts.len() {
                if texts[i] == tag && texts[i - 1] == "struct" && texts.get(i + 1) == Some(&"{") {
                    found.push((file, i + 1));
                }
            }
        }
        let [(file, open)] = found[..] else {
            return Err(format!("`struct {tag}` is not defined once"));
        };
        let texts = self.texts(file);
        let close = matching(&texts, open, "{", "}").ok_or("the record is not read")?;
        let mut slots = Vec::new();
        let mut statement = open + 1;
        for i in open + 1..close {
            if texts[i] == ";" {
                let words = &texts[statement..i];
                if words.contains(&",") || words.contains(&"(") || words.contains(&"[") {
                    return Err(format!(
                        "`struct {tag}` has a declaration this reading does not read"
                    ));
                }
                let Some(name) = words.last().filter(|w| is_identifier(w)) else {
                    return Err(format!("`struct {tag}` has an unnamed slot"));
                };
                slots.push((*name).to_owned());
                statement = i + 1;
            }
        }
        Ok(slots)
    }
}

#[cfg(test)]
mod tests {
    use super::{fold_equal_prefix, key_matches};

    #[test]
    fn the_comparison_folds_ascii_case_over_the_whole_key() {
        assert!(key_matches("Plugins_File", "plugins_file", (256, 256)));
        assert!(key_matches("Plugins_File", "PLUGINS_FILE", (256, 256)));
        assert!(!key_matches("Plugins_File", "Plugins_Files", (256, 256)));
        assert!(!key_matches("Plugins_File", "Plugins_Fil", (256, 256)));
        assert!(fold_equal_prefix("parsers_file", "Parsers_File", 32));
        assert!(!fold_equal_prefix("parsers_file", "Plugins_File", 32));
    }
}
