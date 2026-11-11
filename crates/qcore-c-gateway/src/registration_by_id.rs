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

//! Card **FB-EVENT-LOOP-RETENTION-AND-REGISTRATION** (gateway rule 12n,
//! contract specification §C04.19.3; USER DECISION 2026-10-16, "Result +
//! list"): the body check of a registration forwarded through an id,
//!
//! ```text
//! ensures normal resource_callback_member_registered(#id, TAG.FIELD, F) where #id = #result in P.LIST;
//! ```
//!
//! read in rule 15w's discipline. In a unit that defines the declared
//! function, the clause stands only when the body
//!
//! * **creates exactly one record**: one local `struct TAG *r`, declared
//!   without an initialiser and written once, from a callee whose declaration
//!   in force returns an owned object, never named whole except in that
//!   write, a null test and (in a helper) `return r;`;
//! * **links it into that list**: exactly one call `ADD(&r->H, &P->LIST)` of
//!   a declaration in force that publishes its first actual into its second;
//! * **stores the callback into the member**: exactly one `r->FIELD = F;`,
//!   with `F` named nowhere else and no other access to `r->FIELD`;
//! * **returns that record's id**: every `return` is `return r->ID;` (one
//!   member, the same at every return, at least once) or a negative decimal
//!   constant (the failure value, with no record).
//!
//! The record may instead be created by a helper the unit defines: the body
//! names `F` once, as the whole actual of a direct call `x = G(…);` that
//! receives `P` whole exactly once; `G`'s body satisfies the first three
//! bullets with only `return r;` and `return NULL;`; and the body names `x`
//! only as its declaration, that assignment, a null test, member accesses
//! off `FIELD` (no address taken) and `return x->ID;`, every other return a
//! negative constant. Anything else withdraws the declaration in that unit
//! (every call there refuses, `callback-withdrawn`). A view `&r->H` handed
//! to a call reaches that member only (premise `P-PATH-MEMBER-VIEW`, rule 30's
//! reading). The Lean model is `QCore.RegistrationById`; [`VECTORS`] are its
//! controls.

use qcore_schemas::callback::CallbackMember;

/// A defined function of the unit: its parameter names and its body words
/// (braces included).
pub(crate) type Defined = (Vec<String>, Vec<String>);

/// What the checker may ask of the unit and its environment.
pub(crate) struct Unit<'u> {
    /// A function the unit defines, by name.
    pub defined: &'u dyn Fn(&str) -> Option<Defined>,
    /// Whether a callee's declaration in force returns an owned object.
    pub allocates: &'u dyn Fn(&str) -> bool,
    /// Whether a callee's declaration in force publishes its first actual
    /// into its second (`resource_published(entry, container)`).
    pub links: &'u dyn Fn(&str) -> bool,
}

fn is_identifier(text: &str) -> bool {
    let mut chars = text.chars();
    chars
        .next()
        .is_some_and(|first| first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

/// `words` with the lexer's byte-wise two-byte punctuators merged.
pub(crate) fn merged<S: AsRef<str>>(words: &[S]) -> Vec<String> {
    const PAIRS: [&str; 15] = [
        "->", "==", "!=", "<=", ">=", "++", "--", "+=", "-=", "*=", "/=", "|=", "&=", "^=", "%=",
    ];
    let mut out = Vec::with_capacity(words.len());
    let mut at = 0;
    while at < words.len() {
        let word = words[at].as_ref();
        if let Some(next) = words.get(at + 1).map(AsRef::as_ref) {
            let pair = format!("{word}{next}");
            if word.len() == 1 && next.len() == 1 && PAIRS.contains(&pair.as_str()) {
                out.push(pair);
                at += 2;
                continue;
            }
        }
        out.push(word.to_owned());
        at += 1;
    }
    out
}

fn word(words: &[String], at: usize) -> Option<&str> {
    words.get(at).map(String::as_str)
}

/// The member chain `-> a -> b` (or `.`) starting at `at`, and the index
/// after it.
fn chain(words: &[String], mut at: usize) -> (Vec<String>, usize) {
    let mut out = Vec::new();
    while matches!(word(words, at), Some("->" | "."))
        && words.get(at + 1).is_some_and(|name| is_identifier(name))
    {
        out.push(words[at + 1].clone());
        at += 2;
    }
    (out, at)
}

/// The index of the bracket matching the opening one at `open`.
fn matching(words: &[String], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (index, word) in words.iter().enumerate().skip(open) {
        match word.as_str() {
            "(" | "[" | "{" => depth += 1,
            ")" | "]" | "}" => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

/// The direct call whose argument list holds position `at`: its callee, the
/// index of its `(` and the position of the actual holding `at`.
fn enclosing_call(words: &[String], at: usize) -> Option<(String, usize, usize)> {
    let mut depth = 0usize;
    let mut commas = 0usize;
    for index in (0..at).rev() {
        match words[index].as_str() {
            ")" | "]" | "}" => depth += 1,
            "(" | "[" | "{" if depth > 0 => depth -= 1,
            "(" => {
                let callee = index.checked_sub(1).map(|c| words[c].clone())?;
                return is_identifier(&callee).then_some((callee, index, commas));
            }
            "[" | "{" | ";" => return None,
            "," if depth == 0 => commas += 1,
            _ => {}
        }
    }
    None
}

/// The actuals of the call whose `(` is at `open`, each as its words.
fn actuals(words: &[String], open: usize) -> Option<Vec<Vec<String>>> {
    let close = matching(words, open)?;
    let mut out = vec![Vec::new()];
    let mut depth = 0usize;
    for word in &words[open + 1..close] {
        match word.as_str() {
            "(" | "[" | "{" => depth += 1,
            ")" | "]" | "}" => depth = depth.saturating_sub(1),
            "," if depth == 0 => {
                out.push(Vec::new());
                continue;
            }
            _ => {}
        }
        out.last_mut()?.push(word.clone());
    }
    Some(out)
}

/// Every plain mention of `name` (not a member name after `->` or `.`).
fn mentions(words: &[String], name: &str) -> Vec<usize> {
    (0..words.len())
        .filter(|&at| {
            words[at] == name
                && !matches!(
                    at.checked_sub(1).and_then(|p| word(words, p)),
                    Some("->" | ".")
                )
        })
        .collect()
}

/// `- DIGITS ;` at `at`.
fn negative_constant(words: &[String], at: usize) -> bool {
    word(words, at) == Some("-")
        && words
            .get(at + 1)
            .is_some_and(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
        && word(words, at + 2) == Some(";")
}

/// The index after a null pointer constant starting at `at` (`NULL`, `0`,
/// `(void *) 0`, any of them parenthesised, as the preprocessor leaves
/// `NULL`), or `None`.
fn null_constant(words: &[String], at: usize) -> Option<usize> {
    match word(words, at)? {
        "NULL" | "0" => Some(at + 1),
        "(" => {
            if word(words, at + 1) == Some("void")
                && word(words, at + 2) == Some("*")
                && word(words, at + 3) == Some(")")
            {
                return null_constant(words, at + 4);
            }
            let inner = null_constant(words, at + 1)?;
            (word(words, inner) == Some(")")).then_some(inner + 1)
        }
        _ => None,
    }
}

/// A null test of the name at `at`: `!x )`, `( x )`, `x == NULL`, `x != NULL`.
fn null_test(words: &[String], at: usize) -> bool {
    let previous = at.checked_sub(1).and_then(|p| word(words, p));
    let next = word(words, at + 1);
    (matches!(previous, Some("!" | "(")) && next == Some(")"))
        || (matches!(next, Some("==" | "!=")) && null_constant(words, at + 2).is_some())
}

const STORES: [&str; 8] = ["=", "+=", "-=", "++", "--", "|=", "&=", "^="];

/// The creator check of the module docs over one body. `helper`: its
/// returns are `return r;` / `return NULL;`; otherwise `return r->ID;` or a
/// negative constant.
fn creator(
    words: &[String],
    callback: &str,
    list_formal: &str,
    list: &[String],
    member: &CallbackMember,
    helper: bool,
    unit: &Unit<'_>,
) -> Result<(), String> {
    let field = member
        .path
        .split('.')
        .map(str::to_owned)
        .collect::<Vec<_>>();
    // The callback: exactly one mention, `r -> FIELD = F ;`.
    let [at] = mentions(words, callback)[..] else {
        return Err(format!("`{callback}` is named other than once"));
    };
    if word(words, at + 1) != Some(";") || at < 2 || word(words, at - 1) != Some("=") {
        return Err(format!("`{callback}` is not the value of one member store"));
    }
    let mut start = at - 1;
    while start >= 2
        && is_identifier(&words[start - 1])
        && matches!(word(words, start - 2), Some("->" | "."))
    {
        start -= 2;
    }
    let record = start
        .checked_sub(1)
        .and_then(|r| words.get(r))
        .filter(|name| is_identifier(name))
        .cloned()
        .ok_or("the callback store has no record")?;
    if chain(words, start) != (field.clone(), at - 1) {
        return Err(format!(
            "`{callback}` is stored other than into `{record}->{}`",
            member.path
        ));
    }
    // The record's declarator: `struct TAG * r ;`.
    let declarators = (0..words.len())
        .filter(|&at| {
            word(words, at) == Some("struct")
                && words.get(at + 1) == Some(&member.tag)
                && word(words, at + 2) == Some("*")
                && words.get(at + 3) == Some(&record)
        })
        .collect::<Vec<_>>();
    let [declarator] = declarators[..] else {
        return Err(format!(
            "`{record}` is not one local `struct {} *`",
            member.tag
        ));
    };
    if word(words, declarator + 4) != Some(";") {
        return Err(format!("`{record}` is initialised at its declaration"));
    }
    let mut written = 0usize;
    let mut linked = 0usize;
    let mut returned = 0usize;
    let mut ids = std::collections::BTreeSet::new();
    for at in mentions(words, &record) {
        let previous = at.checked_sub(1).and_then(|p| word(words, p));
        let next = word(words, at + 1);
        if at == declarator + 3 {
            continue;
        }
        if next == Some("=") {
            let callee = words.get(at + 2).cloned().unwrap_or_default();
            if word(words, at + 3) != Some("(") || !(unit.allocates)(&callee) {
                return Err(format!(
                    "`{record}` is written other than from an allocator"
                ));
            }
            written += 1;
            continue;
        }
        if matches!(next, Some("->" | ".")) {
            let (path, after) = chain(words, at + 1);
            let touches = path.starts_with(&field) || field.starts_with(&path);
            let the_store = path == field
                && word(words, after) == Some("=")
                && word(words, after + 1) == Some(callback)
                && word(words, after + 2) == Some(";");
            if touches && !the_store {
                return Err(format!(
                    "`{record}->{}` is touched beyond its one store",
                    member.path
                ));
            }
            if previous == Some("&") {
                if touches {
                    return Err(format!(
                        "the address of `{record}->{}` is taken",
                        member.path
                    ));
                }
                if let Some((callee, open, 0)) = enclosing_call(words, at)
                    && let Some(arguments) = actuals(words, open)
                    && (unit.links)(&callee)
                    && arguments.len() == 2
                    && arguments[0].len() == after - at + 1
                    && arguments[1].first().map(String::as_str) == Some("&")
                    && arguments[1].get(1).map(String::as_str) == Some(list_formal)
                    && chain(&arguments[1], 2) == (list.to_vec(), arguments[1].len())
                {
                    linked += 1;
                }
                continue;
            }
            if previous == Some("return")
                && !helper
                && path.len() == 1
                && word(words, after) == Some(";")
            {
                ids.insert(path[0].clone());
                returned += 1;
                continue;
            }
            continue;
        }
        if null_test(words, at) {
            continue;
        }
        if helper && previous == Some("return") && next == Some(";") {
            returned += 1;
            continue;
        }
        return Err(format!(
            "`{record}` is used whole other than as the created record"
        ));
    }
    if written != 1 {
        return Err(format!("`{record}` is created {written} times, not once"));
    }
    if linked != 1 {
        return Err(format!(
            "`{record}` is linked into `{list_formal}->{}` {linked} times, not once",
            list.join("->")
        ));
    }
    if mentions(words, list_formal)
        .iter()
        .any(|&at| STORES.contains(&word(words, at + 1).unwrap_or_default()))
    {
        return Err(format!("`{list_formal}` is rebound"));
    }
    for at in (0..words.len()).filter(|&at| words[at] == "return") {
        let next = word(words, at + 1);
        let closes = word(words, at + 2) == Some(";");
        let fine = if helper {
            (next == Some(record.as_str()) && closes)
                || null_constant(words, at + 1).is_some_and(|end| word(words, end) == Some(";"))
        } else {
            (next == Some(record.as_str()) && word(words, at + 2) == Some("->"))
                || negative_constant(words, at + 1)
        };
        if !fine {
            return Err("a return is neither the record, its id nor the failure value".into());
        }
    }
    if returned == 0 || ids.len() > 1 {
        return Err("the body does not return the record (or one id member of it)".into());
    }
    Ok(())
}

/// Rule 12n's forwarding check (see the module docs). `body` and
/// `parameters` are the declared function's, `callback` and `list_root` its
/// clause's positions.
pub(crate) fn validates(
    body: &[String],
    parameters: &[String],
    callback: usize,
    list_root: usize,
    list: &[String],
    member: &CallbackMember,
    unit: &Unit<'_>,
) -> Result<(), String> {
    let words = merged(body);
    let callback_name = parameters
        .get(callback)
        .filter(|name| !name.is_empty())
        .ok_or("the callback parameter is not read")?;
    let list_formal = parameters
        .get(list_root)
        .filter(|name| !name.is_empty())
        .ok_or("the list's formal is not read")?;
    // The direct form.
    let direct = creator(
        &words,
        callback_name,
        list_formal,
        list,
        member,
        false,
        unit,
    );
    if direct.is_ok() {
        return Ok(());
    }
    // The helper form.
    let [at] = mentions(&words, callback_name)[..] else {
        return direct;
    };
    if !matches!(
        at.checked_sub(1).and_then(|p| word(&words, p)),
        Some("(" | ",")
    ) || !matches!(word(&words, at + 1), Some(")" | ","))
    {
        return direct;
    }
    let (helper, open, position) =
        enclosing_call(&words, at).ok_or("the callback is not a whole call actual")?;
    let arguments = actuals(&words, open).ok_or("an unreadable call")?;
    let list_positions = arguments
        .iter()
        .enumerate()
        .filter(|(_, argument)| argument.iter().any(|w| w == list_formal))
        .map(|(index, argument)| (index, argument.len()))
        .collect::<Vec<_>>();
    let [(list_position, 1)] = list_positions[..] else {
        return Err("the list's formal is not handed whole, once, to the helper".into());
    };
    let close = matching(&words, open).ok_or("an unreadable call")?;
    if open < 3
        || word(&words, open - 2) != Some("=")
        || !is_identifier(&words[open - 3])
        || word(&words, close + 1) != Some(";")
    {
        return Err("the helper's record is not assigned to a local".into());
    }
    let local = words[open - 3].clone();
    let (helper_parameters, helper_body) =
        (unit.defined)(&helper).ok_or("the helper is not defined in the unit")?;
    let helper_words = merged(&helper_body);
    let helper_callback = helper_parameters
        .get(position)
        .filter(|name| !name.is_empty())
        .ok_or("the helper's callback parameter is not read")?;
    let helper_list = helper_parameters
        .get(list_position)
        .filter(|name| !name.is_empty())
        .ok_or("the helper's list parameter is not read")?;
    creator(
        &helper_words,
        helper_callback,
        helper_list,
        list,
        member,
        true,
        unit,
    )?;
    let field_root = member.path.split('.').next().unwrap_or_default().to_owned();
    let mut ids = std::collections::BTreeSet::new();
    for at in mentions(&words, &local) {
        let previous = at.checked_sub(1).and_then(|p| word(&words, p));
        let next = word(&words, at + 1);
        if previous == Some("*") && next == Some(";") {
            continue;
        }
        if at + 3 == open && next == Some("=") {
            continue;
        }
        if null_test(&words, at) {
            continue;
        }
        if next == Some("->") {
            let (path, after) = chain(&words, at + 1);
            if path.first() == Some(&field_root) || previous == Some("&") {
                return Err(format!(
                    "`{local}->{}` is touched or viewed",
                    path.join("->")
                ));
            }
            if previous == Some("return") {
                if path.len() != 1 || word(&words, after) != Some(";") {
                    return Err("a return is not the record's id".into());
                }
                ids.insert(path[0].clone());
            }
            continue;
        }
        return Err(format!("`{local}` is used whole"));
    }
    for at in (0..words.len()).filter(|&at| words[at] == "return") {
        if word(&words, at + 1) != Some(local.as_str()) && !negative_constant(&words, at + 1) {
            return Err("a return is neither the record's id nor the failure value".into());
        }
    }
    if ids.len() != 1 {
        return Err("the body does not return one id member of the record".into());
    }
    Ok(())
}

/// The controls shared with `QCore.RegistrationById`: a body shape and
/// whether the forwarded registration stands.
pub const VECTORS: &[(&str, bool)] = &[
    ("helper-creates-links-stores-returns-id", true),
    ("direct-creates-links-stores-returns-id", true),
    ("two-records", false),
    ("not-linked", false),
    ("callback-stored-twice", false),
    ("returns-another-value", false),
    ("record-escapes-whole", false),
];

/// The C body each [`VECTORS`] row names: `(setter, helper)` sources.
#[must_use]
pub fn vector_source(name: &str) -> (&'static str, Option<&'static str>) {
    const HELPER: &str = "static struct coll_s *create(int type, struct ins_s *ins, int (*cb)(void), struct cfg_s *config) { struct coll_s *coll; coll = alloc_z(1, 8); if (!coll) { return NULL; } coll->type = type; coll->cb_collect = cb; mk_add(&coll->_head, &ins->collectors); return coll; }";
    match name {
        "helper-creates-links-stores-returns-id" => (
            "int set(struct ins_s *ins, int (*cb_collect)(void), long s, struct cfg_s *config) { struct coll_s *coll; coll = create(1, ins, cb_collect, config); if (!coll) { return -1; } coll->seconds = s; return coll->id; }",
            Some(HELPER),
        ),
        "direct-creates-links-stores-returns-id" => (
            "int set(struct ins_s *ins, int (*cb_collect)(void), long s, struct cfg_s *config) { struct coll_s *coll; coll = alloc_z(1, 8); if (!coll) { return -1; } coll->cb_collect = cb_collect; mk_add(&coll->_head, &ins->collectors); return coll->id; }",
            None,
        ),
        "two-records" => (
            "int set(struct ins_s *ins, int (*cb_collect)(void), long s, struct cfg_s *config) { struct coll_s *coll; coll = alloc_z(1, 8); coll = alloc_z(1, 8); if (!coll) { return -1; } coll->cb_collect = cb_collect; mk_add(&coll->_head, &ins->collectors); return coll->id; }",
            None,
        ),
        "not-linked" => (
            "int set(struct ins_s *ins, int (*cb_collect)(void), long s, struct cfg_s *config) { struct coll_s *coll; coll = alloc_z(1, 8); if (!coll) { return -1; } coll->cb_collect = cb_collect; return coll->id; }",
            None,
        ),
        "callback-stored-twice" => (
            "int set(struct ins_s *ins, int (*cb_collect)(void), long s, struct cfg_s *config) { struct coll_s *coll; coll = alloc_z(1, 8); if (!coll) { return -1; } coll->cb_collect = cb_collect; ins->last = cb_collect; mk_add(&coll->_head, &ins->collectors); return coll->id; }",
            None,
        ),
        "returns-another-value" => (
            "int set(struct ins_s *ins, int (*cb_collect)(void), long s, struct cfg_s *config) { struct coll_s *coll; coll = create(1, ins, cb_collect, config); if (!coll) { return -1; } return 0; }",
            Some(HELPER),
        ),
        "record-escapes-whole" => (
            "int set(struct ins_s *ins, int (*cb_collect)(void), long s, struct cfg_s *config) { struct coll_s *coll; coll = create(1, ins, cb_collect, config); if (!coll) { return -1; } ins->latest = coll; return coll->id; }",
            Some(HELPER),
        ),
        _ => ("", None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Split C text into the gateway lexer's shape: identifiers and numbers
    /// whole, every punctuator byte its own token.
    fn words(text: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut current = String::new();
        for ch in text.chars() {
            if ch == '_' || ch.is_ascii_alphanumeric() {
                current.push(ch);
                continue;
            }
            if !current.is_empty() {
                out.push(std::mem::take(&mut current));
            }
            if !ch.is_whitespace() {
                out.push(ch.to_string());
            }
        }
        if !current.is_empty() {
            out.push(current);
        }
        out
    }

    fn parameters(words: &[String]) -> Vec<String> {
        // `RET name ( … ) {`: the names in the first parenthesised list.
        let open = words.iter().position(|w| w == "(").unwrap();
        let close = matching(words, open).unwrap();
        let mut names = Vec::new();
        let mut group: Vec<&str> = Vec::new();
        let mut depth = 0usize;
        for w in &words[open + 1..close] {
            match w.as_str() {
                "(" => depth += 1,
                ")" => depth -= 1,
                "," if depth == 0 => {
                    names.push(group_name(&group));
                    group.clear();
                    continue;
                }
                _ => {}
            }
            group.push(w);
        }
        names.push(group_name(&group));
        names
    }

    fn group_name(group: &[&str]) -> String {
        if let Some(at) = group.iter().position(|w| *w == "*")
            && group.get(at.wrapping_sub(1)) == Some(&"(")
        {
            return group[at + 1].to_owned();
        }
        group.last().map(|w| (*w).to_owned()).unwrap_or_default()
    }

    fn body(words: &[String]) -> Vec<String> {
        let open = words.iter().position(|w| w == "{").unwrap();
        words[open..].to_vec()
    }

    #[test]
    fn vectors_hold() {
        let member = CallbackMember {
            tag: "coll_s".into(),
            path: "cb_collect".into(),
        };
        for (name, expected) in VECTORS {
            let (setter, helper) = vector_source(name);
            let setter = words(setter);
            let helper = helper.map(words);
            let defined = |callee: &str| {
                helper
                    .as_ref()
                    .filter(|_| callee == "create")
                    .map(|h| (parameters(h), body(h)))
            };
            let unit = Unit {
                defined: &defined,
                allocates: &|callee| callee == "alloc_z",
                links: &|callee| callee == "mk_add",
            };
            let outcome = validates(
                &body(&setter),
                &parameters(&setter),
                1,
                0,
                &["collectors".to_owned()],
                &member,
                &unit,
            );
            assert_eq!(outcome.is_ok(), *expected, "{name}: {outcome:?}");
        }
    }
}
