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
    pub(super) fn holder_opaque(&self, g: &Definition, member: &str) -> Result<String, String> {
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
            match self.carriers_of(definition, flow_of(&texts, start, *k), &site) {
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
        Ok(format!(
            "P-HOLDER-OPAQUE derived: {} read(s) of `{field}` by {}; the holder's pointer reaches \
             {} carrier(s) in {} function(s), none writing through it outside {}",
            sites.len(),
            readers.into_iter().collect::<Vec<_>>().join(", "),
            seen.len(),
            functions.len(),
            self.files[g.file].path
        ))
    }
}
