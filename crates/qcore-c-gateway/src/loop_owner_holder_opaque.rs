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

//! P-HOLDER-OPAQUE derived (card **W47-PREMISES-HOLDER-REGION**, gateway
//! rule 10p's amendment).
//!
//! The fixed test installs its plugin copy inside a holder and stores the
//! holder's address in the instance's user pointer (`ins->data = &test`). The
//! route decision rests on the copy's bytes (its flags and callbacks) and on
//! the instance's plugin field, coroutine flag and collectors; the last three
//! have derived writers (the constructor, the installation, the collector
//! constructor). W47-LOOP-OWNER-INSTANCE-FLOW stated the rest as P-HOLDER-OPAQUE:
//! code that does not name the holder's type reads or writes no holder byte
//! through `ins->data`. This module derives it for the program read:
//!
//! * every read of `struct flb_input_instance::data` the typed reading records
//!   (a function it did not read is read lexically; a file the reader cannot
//!   structure that names the member, or a read spelled outside its
//!   function's file, is not classified) starts a **carrier**: the value is
//!   compared, stored in a local (a carrier), or passed whole to a call — a
//!   read function's parameter, or, through a field dispatch, the parameter of
//!   every registrant the dispatch may reach (typed-keyed where the typed
//!   reading keys the site, else every registrant of the field) — each a
//!   carrier in turn;
//! * a carrier may be compared, passed on, copied into a local, or read
//!   through (`v->m`, a member read, a dispatch through it); in the holder's
//!   own file (the only one that names its type), a member other than the copy
//!   may also be written or have its address taken;
//! * anything else refuses: a write, an address or a dereference through a
//!   carrier outside the holder's file, the copy written through one, a
//!   carrier returned, stored in a field or passed to a function this reading
//!   does not read (a library function such as `memcpy`), or a use this
//!   reading does not classify.
//!
//! When every carrier is classified, no code outside the holder's file writes
//! a holder byte, and the holder's file writes none of the copy's but in the
//! installing function (checked by the holder escape), so the facts the route
//! decision reads through the holder are the ones Step B derived.
//!
//! Premises stated, not proved (trust boundary 28), narrowed from
//! P-HOLDER-OPAQUE: C's effective-type rule (as the typed reading states it)
//! and P-INSTANCE-FIELDS-BY-MEMBER extended to the user pointer (an
//! instance's `data` is read only through member accesses of its record: no
//! whole-instance copy or byte access carries it).

use std::collections::{BTreeMap, BTreeSet};

use super::{Definition, Keyed, StepB};
use crate::legacy_c::is_identifier;
use crate::typed_dispatch_census::member_key;

/// Where a value goes.
enum Flow {
    Compared,
    Local(String),
    Direct(String, usize),
    Dispatch(String, usize, usize),
    Refused(String),
}

const TYPE_WORDS: &[&str] = &[
    "struct",
    "union",
    "enum",
    "const",
    "volatile",
    "unsigned",
    "signed",
    "char",
    "int",
    "long",
    "short",
    "void",
    "float",
    "double",
    "restrict",
    "__restrict",
];

/// The start of a cast group `( TYPE * )` ending just before `at`, if any.
fn cast_start(texts: &[&str], at: usize) -> Option<usize> {
    if at == 0 || texts[at - 1] != ")" {
        return None;
    }
    let mut j = at - 1;
    loop {
        j = j.checked_sub(1)?;
        match texts[j] {
            "(" => break,
            "*" => {}
            word if is_identifier(word) || TYPE_WORDS.contains(&word) => {}
            _ => return None,
        }
    }
    // `f ( x ) value` is not a cast: the group follows an operator.
    if j > 0 && is_identifier(texts[j - 1]) && !matches!(texts[j - 1], "return" | "sizeof") {
        return None;
    }
    Some(j)
}

fn is_assignment(texts: &[&str], at: usize) -> bool {
    match (texts.get(at).copied(), texts.get(at + 1).copied()) {
        (Some("="), Some(next)) => next != "=",
        (Some("+" | "-"), Some(next)) => next == "=" || Some(next) == texts.get(at).copied(),
        (Some("|" | "&" | "^" | "*" | "/" | "%"), Some("=")) => true,
        (Some("<" | ">"), Some(next)) => {
            Some(next) == texts.get(at).copied() && texts.get(at + 2) == Some(&"=")
        }
        _ => false,
    }
}

/// Where the value spanning `start..=end` goes.
fn flow_of(texts: &[&str], start: usize, end: usize) -> Flow {
    let mut s = start;
    while let Some(cast) = cast_start(texts, s) {
        s = cast;
    }
    let before = |i: usize| s.checked_sub(i).map_or("", |at| texts[at]);
    let after = |i: usize| texts.get(end + i).copied().unwrap_or("");
    // Compared or tested.
    let compare_after = (after(1) == "=" || after(1) == "!") && after(2) == "=";
    let compare_before = before(1) == "=" && matches!(before(2), "=" | "!");
    let logical_after = matches!(after(1), "&" | "|") && after(2) == after(1);
    let logical_before = matches!(before(1), "&" | "|") && before(2) == before(1);
    let tested = before(1) == "(" && matches!(before(2), "if" | "while") && after(1) == ")";
    let negated = before(1) == "!" && !matches!(after(1), "=" | "-" | "." | "[");
    if compare_after
        || compare_before
        || logical_after
        || logical_before
        || tested
        || negated
        || after(1) == "?"
    {
        return Flow::Compared;
    }
    // A discarded value (`(void) data;`).
    if after(1) == ";" && matches!(before(1), ";" | "{" | "}") {
        return Flow::Compared;
    }
    if before(1) == "return" {
        return Flow::Refused("returned".to_owned());
    }
    // `v = VALUE ;` / `T *v = VALUE ;`
    if before(1) == "=" && !matches!(before(2), "=" | "!" | "<" | ">") && after(1) == ";" {
        let target = before(2);
        if is_identifier(target) && !matches!(before(3), "." | ">") {
            return Flow::Local(target.to_owned());
        }
        return Flow::Refused("stored in a field or an element".to_owned());
    }
    // A whole argument.
    if matches!(before(1), "(" | ",") && matches!(after(1), ")" | ",") {
        let mut depth = 0usize;
        let mut index = 0usize;
        let mut j = s;
        let open = loop {
            let Some(previous) = j.checked_sub(1) else {
                return Flow::Refused("an argument this reading cannot place".to_owned());
            };
            j = previous;
            match texts[j] {
                ")" | "]" | "}" => depth += 1,
                "[" | "{" => depth = depth.saturating_sub(1),
                "(" if depth == 0 => break j,
                "(" => depth -= 1,
                "," if depth == 0 => index += 1,
                ";" => return Flow::Refused("an argument this reading cannot place".to_owned()),
                _ => {}
            }
        };
        let Some(callee) = open.checked_sub(1).map(|at| texts[at]) else {
            return Flow::Refused("an argument this reading cannot place".to_owned());
        };
        if !is_identifier(callee) || super::loop_owner_instance::NOT_CALLS.contains(&callee) {
            return Flow::Refused("passed to a call this reading does not read".to_owned());
        }
        let member = open >= 2 && matches!(texts[open - 2], "." | ">");
        return if member {
            Flow::Dispatch(callee.to_owned(), open - 1, index)
        } else {
            Flow::Direct(callee.to_owned(), index)
        };
    }
    Flow::Refused(format!(
        "used as `{} {} … {} {}`",
        before(2),
        before(1),
        after(1),
        after(2)
    ))
}

impl StepB<'_> {
    /// The names a `typedef` gives the instance's record (or a pointer to it)
    /// in the read files.
    fn instance_aliases(&self) -> BTreeSet<String> {
        let tag = self
            .flow
            .instance_record
            .strip_prefix("struct ")
            .unwrap_or(self.flow.instance_record);
        let mut aliases = BTreeSet::new();
        for file in 0..self.files.len() {
            let texts = self.texts(file);
            for k in 0..texts.len().saturating_sub(3) {
                if texts[k] == "typedef" && texts[k + 1] == "struct" && texts[k + 2] == tag {
                    let mut j = k + 3;
                    while texts.get(j) == Some(&"*") {
                        j += 1;
                    }
                    if let Some(name) = texts.get(j).filter(|t| is_identifier(t)) {
                        aliases.insert((*name).to_owned());
                    }
                }
            }
        }
        aliases
    }

    /// Whether the `data` member at `k` (a function without a typed reading)
    /// belongs to an object `R` of `R -> data` / `R . data` that the function
    /// declares with a record type other than the instance's.
    fn declared_other_record(
        &self,
        definition: &Definition,
        k: usize,
        aliases: &BTreeSet<String>,
    ) -> bool {
        let texts = self.texts(definition.file);
        let tag = self
            .flow
            .instance_record
            .strip_prefix("struct ")
            .unwrap_or(self.flow.instance_record);
        let object = if texts[k - 1] == "." { k - 2 } else { k - 3 };
        let root = texts[object];
        if !is_identifier(root) || matches!(texts[object - 1], "." | ">") {
            return false;
        }
        for j in definition.header..definition.body.1 {
            if texts[j] != root || !matches!(texts[j + 1], "," | ")" | ";" | "=" | "[") {
                continue;
            }
            let mut t = j - 1;
            while matches!(
                texts[t],
                "*" | "const" | "volatile" | "restrict" | "__restrict"
            ) {
                t -= 1;
            }
            if t == j - 1 && texts[t] != ")" && !is_identifier(texts[t]) {
                continue;
            }
            let word = texts[t];
            if !is_identifier(word) || matches!(word, "return" | "sizeof" | "case" | "goto") {
                continue;
            }
            if matches!(texts[t - 1], "struct" | "union") {
                return word != tag;
            }
            if texts[t - 1] == "enum" {
                return true;
            }
            // A typedef name: another record unless it names the instance's.
            return !aliases.contains(word) && word != "void";
        }
        false
    }

    /// The carriers a value passed at `index` to function `id` starts.
    fn parameter_carriers(
        &self,
        id: usize,
        index: usize,
        site: &str,
    ) -> Result<Vec<(usize, String)>, String> {
        let mut out = Vec::new();
        for d in self.definitions_of(id) {
            if self
                .typed
                .is_some_and(|typed| self.in_build(typed, d) == Some(false))
            {
                continue;
            }
            let parameters =
                super::loop_owner_instance::ordered_parameters(&self.texts(d.file), d.header);
            let Some(name) = parameters.get(index) else {
                return Err(format!(
                    "P-HOLDER-OPAQUE is not derived: {site}: passed to `{}` beyond its named \
                     parameters",
                    self.functions[id]
                ));
            };
            out.push((id, name.clone()));
        }
        Ok(out)
    }

    /// The functions the holder's copy holds in `callback`, when the dispatch
    /// at `position` of `from` calls the callback of instance `x` itself
    /// (`x->p->F(…)`, or `Q->F(…)` after `Q = x->p`, `x` not reassigned
    /// between): whenever `x->data` is the holder, `x->p` is the copy (the
    /// two are installed together and each has no other writer but the
    /// constructor), so the dispatch runs the copy's `F`. That is the function
    /// the installing function stores in the copy's `F`, or the `F` the
    /// creating descriptor's initializer designates (the whole copy's source).
    /// `None` when the shape or a source is not read.
    fn copy_callbacks(
        &self,
        (g, root, member): (&Definition, &str, &str),
        from: &Definition,
        x: &str,
        callback: &str,
        position: usize,
    ) -> Option<BTreeSet<usize>> {
        let texts = self.texts(from.file);
        let plugin_field = self.hops.instance_plugin_field;
        let k = position;
        let direct = k >= 6
            && texts[k - 1] == ">"
            && texts[k - 2] == "-"
            && texts[k - 3] == plugin_field
            && texts[k - 4] == ">"
            && texts[k - 5] == "-"
            && texts[k - 6] == x
            && !matches!(texts[k - 7], "." | ">");
        let through_local = k >= 3
            && texts[k - 1] == ">"
            && texts[k - 2] == "-"
            && is_identifier(texts[k - 3])
            && !matches!(texts[k - 4], "." | ">")
            && {
                let q = texts[k - 3];
                let assigned = super::loop_owner_instance::assignments(&texts, from.body, q);
                let declared = (from.body.0..k).filter(|&i| {
                    texts[i] == q
                        && texts.get(i + 1) == Some(&"=")
                        && texts.get(i + 2) != Some(&"=")
                        && texts[i - 1] == "*"
                });
                let sites = assigned
                    .iter()
                    .copied()
                    .chain(declared)
                    .collect::<BTreeSet<_>>();
                matches!(sites.iter().copied().collect::<Vec<_>>()[..], [at] if at < k
                    && texts.get(at + 2) == Some(&x)
                    && texts.get(at + 3) == Some(&"-")
                    && texts.get(at + 4) == Some(&">")
                    && texts.get(at + 5) == Some(&plugin_field)
                    && texts.get(at + 6) == Some(&";")
                    && !super::loop_owner_instance::address_taken(&texts, from.body, q)
                    && super::loop_owner_instance::assignments(&texts, (at, k), x).is_empty())
            };
        if !(direct || through_local) {
            return None;
        }
        let mut targets = BTreeSet::new();
        // The installing function's stores: `root . member . F = f ;`.
        let g_texts = self.texts(g.file);
        for i in g.body.0..g.body.1 {
            if g_texts[i] == root
                && g_texts.get(i + 1) == Some(&".")
                && g_texts.get(i + 2) == Some(&member)
                && g_texts.get(i + 3) == Some(&".")
                && g_texts.get(i + 4) == Some(&callback)
                && g_texts.get(i + 5) == Some(&"=")
                && g_texts.get(i + 6) != Some(&"=")
            {
                let value = g_texts.get(i + 6)?;
                if g_texts.get(i + 7) != Some(&";") || !is_identifier(value) {
                    return None;
                }
                targets.extend(self.resolve_name(g.file, value));
            }
        }
        // The creating descriptors' designated initializers.
        let literal = self.copy_literal.borrow().clone()?;
        let typed = self.typed?;
        let prefix = format!(
            "struct {}|{}|",
            self.hops.plugin_descriptor, self.hops.plugin_name_field
        );
        let objects = typed
            .named_objects
            .iter()
            .filter_map(|(key, objects)| {
                let name = key.strip_prefix(&prefix)?;
                name.eq_ignore_ascii_case(&literal).then_some(objects)
            })
            .flatten()
            .collect::<BTreeSet<_>>();
        if objects.is_empty() {
            return None;
        }
        for object in objects {
            let mut found = false;
            for file in 0..self.files.len() {
                let t = self.texts(file);
                for i in 1..t.len().saturating_sub(2) {
                    if t[i] != object.as_str() || t[i + 1] != "=" || t[i + 2] != "{" {
                        continue;
                    }
                    found = true;
                    let mut depth = 0usize;
                    let mut j = i + 2;
                    loop {
                        // Every element is designated: a positional one leaves
                        // the slots unread.
                        if depth == 1
                            && matches!(t.get(j - 1).copied(), Some("{" | ","))
                            && !matches!(t.get(j).copied(), Some("." | "}"))
                        {
                            return None;
                        }
                        match t.get(j).copied()? {
                            "{" => depth += 1,
                            "}" => {
                                depth -= 1;
                                if depth == 0 {
                                    break;
                                }
                            }
                            "." if depth == 1
                                && t.get(j + 1) == Some(&callback)
                                && t.get(j + 2) == Some(&"=") =>
                            {
                                let value = t.get(j + 3)?;
                                if !is_identifier(value)
                                    || !matches!(t.get(j + 4), Some(&"," | &"}"))
                                {
                                    return None;
                                }
                                targets.extend(self.resolve_name(file, value));
                            }
                            _ => {}
                        }
                        j += 1;
                    }
                }
            }
            if !found {
                return None;
            }
        }
        Some(targets)
    }

    fn definitions_of(&self, id: usize) -> Vec<&Definition> {
        self.definitions
            .iter()
            .filter(|d| self.index[&d.census_name] == id)
            .collect()
    }

    /// Every carrier a value flowing as `flow` from `from` starts, or why it
    /// is refused.
    fn carriers_of(
        &self,
        from: &Definition,
        flow: Flow,
        site: &str,
    ) -> Result<Vec<(usize, String)>, String> {
        let texts = self.texts(from.file);
        let from_id = self.index[&from.census_name];
        let place = |id: usize, index: usize| -> Result<Vec<(usize, String)>, String> {
            let mut out = Vec::new();
            for d in self.definitions_of(id) {
                // A definition the build does not compile (a `.c` file no
                // command names) is not part of the program.
                if self
                    .typed
                    .is_some_and(|typed| self.in_build(typed, d) == Some(false))
                {
                    continue;
                }
                let parameters =
                    super::loop_owner_instance::ordered_parameters(&self.texts(d.file), d.header);
                let Some(name) = parameters.get(index) else {
                    return Err(format!(
                        "{site}: passed to `{}` beyond its named parameters",
                        self.functions[id]
                    ));
                };
                out.push((id, name.clone()));
            }
            Ok(out)
        };
        match flow {
            Flow::Compared => Ok(Vec::new()),
            Flow::Local(name) => Ok(vec![(from_id, name)]),
            Flow::Direct(callee, index) => {
                let ids = self.resolve_name(from.file, &callee);
                if ids.is_empty() {
                    return Err(format!(
                        "{site}: passed to `{callee}`, which this reading does not read"
                    ));
                }
                let mut out = Vec::new();
                for id in ids {
                    out.extend(place(id, index)?);
                }
                Ok(out)
            }
            Flow::Dispatch(field, position, index) => {
                let records = self
                    .dispatch_sites
                    .iter()
                    .find(|d| d.file == from.file && d.position == position)
                    .and_then(|d| d.records.clone());
                let mut out = Vec::new();
                for (id, keys) in self.registrants.get(&field).into_iter().flatten() {
                    let reached = records.as_ref().is_none_or(|records| {
                        keys.iter().any(|key| match key {
                            Keyed::Any => true,
                            Keyed::Record(record) => records.contains(record),
                        })
                    });
                    if reached {
                        out.extend(place(*id, index)?);
                    }
                }
                let _ = texts;
                Ok(out)
            }
            Flow::Refused(why) => Err(format!("{site}: {why}")),
        }
    }

    /// Classify every use of carrier `name` in `r`; the carriers it starts.
    fn carrier_uses(
        &self,
        r: &Definition,
        name: &str,
        holder_file: usize,
        copy_member: &str,
    ) -> Result<Vec<(usize, String)>, String> {
        let texts = self.texts(r.file);
        let path = &self.files[r.file].path;
        let mut out = Vec::new();
        for k in r.body.0..=r.body.1 {
            if texts[k] != name
                || matches!(texts[k - 1], ".")
                || (texts[k - 1] == ">" && texts[k - 2] == "-")
            {
                continue;
            }
            let line = self.scans[r.file].tokens[k].line;
            let site = format!(
                "the holder's pointer in `{}` (`{name}` at {path}:{line})",
                self.functions[self.index[&r.census_name]]
            );
            // A member chain through the carrier.
            let arrow = texts.get(k + 1) == Some(&"-") && texts.get(k + 2) == Some(&">");
            if arrow || texts.get(k + 1) == Some(&".") {
                let mut j = k + 1;
                let mut first = None;
                loop {
                    if texts.get(j) == Some(&"-")
                        && texts.get(j + 1) == Some(&">")
                        && texts.get(j + 2).is_some_and(|t| is_identifier(t))
                    {
                        first.get_or_insert(texts[j + 2]);
                        j += 3;
                    } else if texts.get(j) == Some(&".")
                        && texts.get(j + 1).is_some_and(|t| is_identifier(t))
                    {
                        first.get_or_insert(texts[j + 1]);
                        j += 2;
                    } else if texts.get(j) == Some(&"[") {
                        let mut depth = 0usize;
                        while j < texts.len() {
                            match texts[j] {
                                "[" => depth += 1,
                                "]" => {
                                    depth -= 1;
                                    if depth == 0 {
                                        break;
                                    }
                                }
                                _ => {}
                            }
                            j += 1;
                        }
                        j += 1;
                    } else {
                        break;
                    }
                }
                let written = is_assignment(&texts, j)
                    || (texts[k - 1] == "&" && texts[k - 2] != "&")
                    || (matches!(texts[k - 1], "+" | "-") && texts[k - 2] == texts[k - 1]);
                if written {
                    let member = first.unwrap_or("");
                    if r.file != holder_file {
                        return Err(format!(
                            "{site}: a holder byte is written or its address taken through it"
                        ));
                    }
                    if member == copy_member {
                        return Err(format!("{site}: the copy is written through it"));
                    }
                }
                continue;
            }
            if texts.get(k + 1) == Some(&"=") && texts.get(k + 2) != Some(&"=") {
                // Assigned: not a use of its value.
                continue;
            }
            if texts[k - 1] == "*" {
                if is_identifier(texts[k - 2]) && texts[k - 2] != "return" {
                    // A declaration `T *v`.
                    continue;
                }
                return Err(format!("{site}: dereferenced"));
            }
            if texts.get(k + 1) == Some(&";")
                && (is_identifier(texts[k - 1]) && texts[k - 1] != "return")
            {
                continue;
            }
            if texts.get(k + 1) == Some(&"[") {
                return Err(format!("{site}: indexed"));
            }
            out.extend(self.carriers_of(r, flow_of(&texts, k, k), &site)?);
        }
        Ok(out)
    }

    /// Derive P-HOLDER-OPAQUE for the holder of type `struct TAG`, declared in
    /// the test's file (that of `g`), whose member `member` is the copy.
    pub(super) fn holder_opaque(
        &self,
        g: &Definition,
        root: &str,
        member: &str,
    ) -> Result<String, String> {
        let tables = self.flow;
        let typed = self
            .typed
            .ok_or("P-HOLDER-OPAQUE is not derived: no typed reading")?;
        let field = tables.instance_data_field;
        let underived = |why: String| Err(format!("P-HOLDER-OPAQUE is not derived: {why}"));
        if !typed
            .member_fields_read
            .contains(&member_key(tables.instance_record, field))
        {
            return underived(format!(
                "the typed reading did not record `{}::{field}`'s readers",
                tables.instance_record
            ));
        }
        // The read sites: typed reads, and every `->data` of a function the
        // typed reading did not read.
        let mut sites: Vec<(&Definition, usize)> = Vec::new();
        let mut readers = BTreeSet::new();
        for key in typed
            .member_accesses
            .get(&member_key(tables.instance_record, field))
            .into_iter()
            .flatten()
        {
            let mut parts = key.split('|');
            let (Some(file), Some(line), Some(function), Some(use_label)) =
                (parts.next(), parts.next(), parts.next(), parts.next())
            else {
                return underived(format!("an access this reading cannot read: {key}"));
            };
            if use_label == "write" {
                continue;
            }
            let line: usize = line.parse().unwrap_or(0);
            let Some(definition) = self.definitions.iter().find(|d| {
                d.name == function
                    && self.files[d.file].path == file
                    && self.scans[d.file].tokens[d.header].line <= line
                    && line <= self.scans[d.file].tokens[d.body.1].line
            }) else {
                return underived(format!(
                    "the read at {file}:{line} in `{function}` is not in a function this reading \
                     reads there (a macro's replacement list, or an unread arm)"
                ));
            };
            let texts = self.texts(definition.file);
            let mut found = false;
            for k in definition.body.0..=definition.body.1 {
                if texts[k] == field
                    && self.scans[definition.file].tokens[k].line == line
                    && (texts[k - 1] == "." || (texts[k - 1] == ">" && texts[k - 2] == "-"))
                {
                    found = true;
                    sites.push((definition, k));
                }
            }
            if !found {
                return underived(format!(
                    "the read at {file}:{line} in `{function}` is not spelled there"
                ));
            }
            readers.insert(function.to_owned());
        }
        let aliases = self.instance_aliases();
        let mut other_records = 0usize;
        for definition in self.definitions {
            let id = self.index[&definition.census_name];
            if self.in_build(typed, definition).is_some() {
                continue;
            }
            let texts = self.texts(definition.file);
            for k in definition.body.0..=definition.body.1 {
                if texts[k] == field
                    && (texts[k - 1] == "." || (texts[k - 1] == ">" && texts[k - 2] == "-"))
                {
                    // A member of an object declared with another record type
                    // is not the instance's (C's effective-type rule).
                    if self.declared_other_record(definition, k, &aliases) {
                        other_records += 1;
                        continue;
                    }
                    if self.reach.unstructured.contains(&id) {
                        return underived(format!(
                            "`{}` (a file this reader cannot structure) names `{field}` and has \
                             no typed reading",
                            self.functions[id]
                        ));
                    }
                    sites.push((definition, k));
                    readers.insert(self.functions[id].clone());
                }
            }
        }
        // Each site's value, then every carrier to a fixed point.
        let mut work: Vec<(usize, String)> = Vec::new();
        let mut narrowed = 0usize;
        for (definition, k) in &sites {
            let texts = self.texts(definition.file);
            // The chain `X (-> F | . F)* -> data`.
            let mut start = *k;
            loop {
                let arrow = start >= 3 && texts[start - 1] == ">" && texts[start - 2] == "-";
                let dot = start >= 2 && texts[start - 1] == ".";
                let object = if arrow {
                    start - 3
                } else if dot {
                    start - 2
                } else {
                    break;
                };
                if !is_identifier(texts[object]) {
                    return underived(format!(
                        "the read at {}:{} has an object this reading does not follow",
                        self.files[definition.file].path,
                        self.scans[definition.file].tokens[*k].line
                    ));
                }
                start = object;
            }
            let site = format!(
                "the read of `{field}` at {}:{} in `{}`",
                self.files[definition.file].path,
                self.scans[definition.file].tokens[*k].line,
                self.functions[self.index[&definition.census_name]]
            );
            let flow = flow_of(&texts, start, *k);
            // The instance's own callback: the holder's instance runs its copy.
            if let Flow::Dispatch(callback, position, index) = &flow
                && start + 3 == *k
                && let Some(targets) = self.copy_callbacks(
                    (g, root, member),
                    definition,
                    texts[start],
                    callback,
                    *position,
                )
            {
                narrowed += 1;
                for id in targets {
                    work.extend(self.parameter_carriers(id, *index, &site)?);
                }
                continue;
            }
            match self.carriers_of(definition, flow, &site) {
                Ok(carriers) => work.extend(carriers),
                Err(why) => return underived(why),
            }
        }
        let mut seen: BTreeSet<(usize, String)> = BTreeSet::new();
        let mut functions: BTreeMap<usize, BTreeSet<String>> = BTreeMap::new();
        while let Some((id, name)) = work.pop() {
            if !seen.insert((id, name.clone())) {
                continue;
            }
            functions.entry(id).or_default().insert(name.clone());
            for definition in self.definitions_of(id) {
                match self.carrier_uses(definition, &name, g.file, member) {
                    Ok(more) => work.extend(more),
                    Err(why) => return underived(why),
                }
            }
        }
        let summary = format!(
            "P-HOLDER-OPAQUE derived: {} read(s) of `{field}` by {}; the holder's pointer reaches \
             {} carrier(s) in {} function(s) ({}), none writing through it outside {} ({} read(s) \
             passed to the instance's own callback, the copy's; {} untyped `{field}` member(s) \
             of objects declared with another record type)",
            sites.len(),
            readers.into_iter().collect::<Vec<_>>().join(", "),
            seen.len(),
            functions.len(),
            functions
                .keys()
                .map(|id| self.functions[*id].as_str())
                .collect::<Vec<_>>()
                .join(", "),
            self.files[g.file].path,
            narrowed,
            other_records
        );
        self.notes.borrow_mut().insert(summary.clone());
        Ok(summary)
    }
}
