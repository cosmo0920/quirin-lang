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

//! The instance a runtime plugin copy is installed on (card
//! **W47-LOOP-OWNER-INSTANCE-FLOW**, gateway rule 10p's amendment).
//!
//! Step B of the thread-entry census (card W47-LOOP-OWNER-TYPED-DISPATCH)
//! drops the coroutine route of a collector registered from a runtime plugin
//! copy only when the instance the copy is installed on is derived to run no
//! coroutine. The route switch reads `collector->instance->runs_in_coroutine`,
//! which only the instance constructor writes, from the plugin it creates the
//! instance from. This module derives, from the census's own reading and the
//! typed member facts, three things Step B took as underived or as premises:
//!
//! 1. **The list-head instance.** An instance read as the head of a context's
//!    input list (`X = mk_list_entry(H, struct flb_input_instance, _head)`,
//!    `H` the list's `next`) is the one instance created from a literal plugin
//!    name when, on every path from the test's entry to the read: the list is
//!    the one of a context that path created (the context's constructor
//!    assigned once into a local returned by a function of the same file, and
//!    nothing re-pointing a context's config); the calls whose closure over
//!    the census graph reaches a hazard (the instance constructor, a mutation
//!    of an input list or of the registered-plugin list, a context re-point,
//!    an unstructured file, an untyped function mentioning those members) are
//!    exactly one creator call, outside every loop, with a literal plugin
//!    name; and no function an unmodelled call may reach (an address taken, a
//!    name in a macro, a registrant of a function-pointer field read as a
//!    value through a matching record) reaches a hazard. Every descriptor
//!    whose name matches the literal (ignoring ASCII case, as
//!    `check_protocol` compares) must derive no coroutine.
//! 2. **The holder escape.** The object holding the copy may escape only into
//!    the installed instance's `data` field, when its record type is named in
//!    the test's file alone, no function writes or takes the address of the
//!    copy member except the one Step B models, and the instance's plugin
//!    field is written only by the constructor and the one installation.
//! 3. **P-CALLBACK-INSTANCE.** Every dispatch of the copy's callback field
//!    that can reach the registering callback passes, as its first argument,
//!    the instance whose plugin field it dispatches through (`X->p->F(X, …)`,
//!    or `Q->F(X, …)` after `Q = X->p`), or is the callback itself passing its
//!    own first parameter; and the collector's `instance` is written only by
//!    the collector constructor, from its instance parameter, which each
//!    registration function passes its own first parameter to.
//!
//! Anything else leaves the flag underived, so the coroutine route is kept.
//! Premises stated, not proved (trust boundary 28), beside rule 10p's:
//! P-HEAD-INSTANCE (the list the test reads is not empty: its one creation
//! succeeded; an empty list's head is not an instance and its read is outside
//! the census), P-FRESH-CONTEXT (the context constructor returns a new
//! context whose config's input list is empty), P-REGISTERED-DESCRIPTORS (the
//! registered plugins a creation matches are the build's descriptors, one
//! registration each, before any runtime registration the path does not
//! reach), P-HOLDER-OPAQUE (code that does not name the holder's type does
//! not read or write the holder's bytes through the instance's `data`), and
//! P-INSTANCE-FIELDS-BY-MEMBER (an instance's plugin field and coroutine
//! flag, and a collector's instance, are written only through member
//! accesses).

use std::collections::{BTreeMap, BTreeSet};

use super::{Definition, Derivation, StepB};
use crate::legacy_c::is_identifier;
use crate::typed_dispatch_census::{TypedFacts, member_key};

/// Fluent Bit's names for this derivation, as read at the `90978ac6` export.
#[derive(Clone, Copy, Debug)]
pub struct InstanceFlowTables {
    /// `mk_list_entry(H, struct flb_input_instance, _head)`.
    pub list_entry_macro: &'static str,
    pub instance_record: &'static str,
    pub instance_link_field: &'static str,
    /// `config->inputs.next`.
    pub config_record: &'static str,
    pub list_field: &'static str,
    pub list_next_field: &'static str,
    /// The registered-plugin list (`config->in_plugins`).
    pub registrations_field: &'static str,
    /// `ctx->config` of `struct flb_lib_ctx`, set by `flb_create`.
    pub context_record: &'static str,
    pub context_config_field: &'static str,
    pub context_constructor: &'static str,
    /// Calls that only read a list head passed by address.
    pub list_readers: &'static [&'static str],
    /// `(function, list field, call)`: the list mutations that are the
    /// creation itself or a fresh config's initialisation.
    pub list_initialisations: &'static [(&'static str, &'static str, &'static str)],
    /// `(creator wrapper, name argument)`: a call whose literal the typed
    /// reading records (the derivation still reads the wrapper's body).
    pub creator_wrappers: &'static [(&'static str, usize)],
    /// The instance's opaque user pointer (`ins->data`).
    pub instance_data_field: &'static str,
    /// `coll->instance`, written by `collector_create`.
    pub collector_instance_field: &'static str,
    pub collector_constructor: &'static str,
    /// The allocator's release, the only call a returned context's holder
    /// may be passed to whole on a failure path.
    pub free_function: &'static str,
}

/// Fluent Bit at the `90978ac6` export: `include/fluent-bit/flb_input.h`
/// (`struct flb_input_instance`, `_head`, `p`, `data`), `flb_config.h`
/// (`inputs`, `in_plugins`), `flb_lib.h` (`struct flb_lib_ctx`, `config`),
/// `src/flb_lib.c:145` (`flb_create`), `:268` (`flb_input`),
/// `src/flb_config.c:502`, `:507` (the fresh config's two lists),
/// `src/flb_input.c:677` (the creation's insertion), `:2222`, `:2246`
/// (`collector_create`, `coll->instance = ins`).
pub const FLUENT_BIT_INSTANCE_FLOW: InstanceFlowTables = InstanceFlowTables {
    list_entry_macro: "mk_list_entry",
    instance_record: "struct flb_input_instance",
    instance_link_field: "_head",
    config_record: "struct flb_config",
    list_field: "inputs",
    list_next_field: "next",
    registrations_field: "in_plugins",
    context_record: "struct flb_lib_ctx",
    context_config_field: "config",
    context_constructor: "flb_create",
    list_readers: &["mk_list_size", "mk_list_is_empty"],
    list_initialisations: &[
        ("flb_config_init", "inputs", "mk_list_init"),
        ("flb_config_init", "in_plugins", "mk_list_init"),
        ("flb_input_new", "inputs", "mk_list_add"),
    ],
    creator_wrappers: &[("flb_input", 1)],
    instance_data_field: "data",
    collector_instance_field: "instance",
    collector_constructor: "collector_create",
    free_function: "flb_free",
};

impl InstanceFlowTables {
    /// The members whose accesses the typed reading records.
    #[must_use]
    pub fn member_fields(&self, hops: &super::HopTables) -> Vec<(String, String)> {
        vec![
            (
                self.instance_record.to_owned(),
                hops.instance_plugin_field.to_owned(),
            ),
            (
                self.instance_record.to_owned(),
                hops.instance_flag_field.to_owned(),
            ),
            (
                self.instance_record.to_owned(),
                self.instance_data_field.to_owned(),
            ),
            (self.config_record.to_owned(), self.list_field.to_owned()),
            (
                self.config_record.to_owned(),
                self.registrations_field.to_owned(),
            ),
            (
                self.context_record.to_owned(),
                self.context_config_field.to_owned(),
            ),
            (
                hops.collector_record.to_owned(),
                self.collector_instance_field.to_owned(),
            ),
        ]
    }
}

/// The census graph as reachability, over-approximated for this derivation:
/// direct calls (and the raw caller of a call hop C re-places), every
/// registrant a field dispatch may reach whatever the coroutine route, spawns
/// and coroutine entries.
#[derive(Debug, Default)]
pub(super) struct Reach {
    pub next: BTreeMap<usize, BTreeSet<usize>>,
    /// Functions an unmodelled call may reach, with why.
    pub wild: BTreeMap<usize, String>,
    /// Functions in a file whose conditional groups the reader cannot
    /// structure (their calls may be missing).
    pub unstructured: BTreeSet<usize>,
}

impl Reach {
    /// Every function reachable from `roots` (the roots included).
    fn closure(&self, roots: impl IntoIterator<Item = usize>) -> BTreeSet<usize> {
        let mut seen = BTreeSet::new();
        let mut stack = roots.into_iter().collect::<Vec<_>>();
        while let Some(at) = stack.pop() {
            if seen.insert(at) {
                stack.extend(self.next.get(&at).into_iter().flatten().copied());
            }
        }
        seen
    }

    /// A shortest call path from `from` to one of `targets`.
    fn path_to(&self, from: usize, targets: &BTreeMap<usize, String>) -> Vec<usize> {
        let mut parent: BTreeMap<usize, usize> = BTreeMap::new();
        let mut queue = std::collections::VecDeque::from([from]);
        let mut seen = BTreeSet::from([from]);
        while let Some(at) = queue.pop_front() {
            if targets.contains_key(&at) {
                let mut path = vec![at];
                let mut cursor = at;
                while let Some(previous) = parent.get(&cursor) {
                    cursor = *previous;
                    path.push(cursor);
                }
                path.reverse();
                return path;
            }
            for next in self.next.get(&at).into_iter().flatten() {
                if seen.insert(*next) {
                    parent.insert(*next, at);
                    queue.push_back(*next);
                }
            }
        }
        Vec::new()
    }

    /// Every function from which one of `targets` is reachable.
    fn reaching(&self, targets: &BTreeSet<usize>) -> BTreeSet<usize> {
        let mut back: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        for (from, tos) in &self.next {
            for to in tos {
                back.entry(*to).or_default().push(*from);
            }
        }
        let mut seen = targets.clone();
        let mut stack = targets.iter().copied().collect::<Vec<_>>();
        while let Some(at) = stack.pop() {
            for from in back.get(&at).into_iter().flatten() {
                if seen.insert(*from) {
                    stack.push(*from);
                }
            }
        }
        seen
    }
}

/// One field dispatch the census read, for P-CALLBACK-INSTANCE: its caller,
/// field, file, the position of the field token, and the records the typed
/// reading keys it by (`None`: untyped or unmatched, so it may reach every
/// registrant of the field).
#[derive(Clone, Debug)]
pub(super) struct DispatchSite {
    pub caller: usize,
    pub field: String,
    pub file: usize,
    pub position: usize,
    pub records: Option<BTreeSet<String>>,
}

/// A typed member access: file, line, function, use.
struct Access<'a> {
    file: &'a str,
    line: usize,
    function: &'a str,
    use_label: &'a str,
}

fn accesses<'a>(typed: &'a TypedFacts, record: &str, field: &str) -> Vec<Access<'a>> {
    typed
        .member_accesses
        .get(&member_key(record, field))
        .into_iter()
        .flatten()
        .filter_map(|site| {
            let mut parts = site.splitn(4, '|');
            let file = parts.next()?;
            let line = parts.next()?.parse().ok()?;
            let function = parts.next()?;
            let use_label = parts.next()?;
            Some(Access {
                file,
                line,
                function,
                use_label,
            })
        })
        .collect()
}

/// Whether an access only reads.
fn reads_only(use_label: &str, readers: &[&str]) -> bool {
    matches!(use_label, "read" | "address-read")
        || use_label
            .strip_prefix("arg:")
            .is_some_and(|callee| readers.contains(&callee))
}

/// The loop statements' token extents inside `range` (inclusive), and whether
/// the range has a `goto` or a label-like `ident :`.
pub(super) fn loops(texts: &[&str], range: (usize, usize)) -> (Vec<(usize, usize)>, bool) {
    let mut extents = Vec::new();
    let mut jumps = false;
    let block_end = |from: usize| -> usize {
        if texts.get(from) == Some(&"{") {
            let mut depth = 0usize;
            for (index, text) in texts.iter().enumerate().take(range.1 + 1).skip(from) {
                match *text {
                    "{" => depth += 1,
                    "}" => {
                        depth -= 1;
                        if depth == 0 {
                            return index;
                        }
                    }
                    _ => {}
                }
            }
            return range.1;
        }
        // A statement that is not a block: to the end of the enclosing block,
        // an over-approximation.
        let mut depth = 0isize;
        for (index, text) in texts.iter().enumerate().take(range.1 + 1).skip(from) {
            match *text {
                "{" | "(" => depth += 1,
                "}" | ")" => {
                    depth -= 1;
                    if depth < 0 {
                        return index;
                    }
                }
                _ => {}
            }
        }
        range.1
    };
    let close_paren = |open: usize| -> Option<usize> {
        let mut depth = 0usize;
        for (index, text) in texts.iter().enumerate().take(range.1 + 1).skip(open) {
            match *text {
                "(" => depth += 1,
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
    };
    for index in range.0..=range.1 {
        match texts[index] {
            "for" | "while" if texts.get(index + 1) == Some(&"(") => {
                let Some(close) = close_paren(index + 1) else {
                    extents.push((index, range.1));
                    continue;
                };
                extents.push((index, block_end(close + 1)));
            }
            "do" => extents.push((index, block_end(index + 1))),
            "goto" => jumps = true,
            text if is_identifier(text)
                && texts.get(index + 1) == Some(&":")
                && texts.get(index + 2) != Some(&":")
                && !matches!(texts.get(index.wrapping_sub(1)), Some(&"?" | &"case")) =>
            {
                jumps = true;
            }
            _ => {}
        }
    }
    (extents, jumps)
}

/// The brace blocks inside `range` that end in `return NULL;` (or `return
/// 0;`) and have no `break`, `continue`, `goto` or other `return`: every path
/// through them returns a null pointer.
fn null_return_blocks(texts: &[&str], range: (usize, usize)) -> Vec<(usize, usize)> {
    let mut blocks = Vec::new();
    for open in range.0 + 1..range.1 {
        if texts[open] != "{" {
            continue;
        }
        let mut depth = 0usize;
        let mut close = None;
        for (index, text) in texts.iter().enumerate().take(range.1).skip(open) {
            match *text {
                "{" => depth += 1,
                "}" => {
                    depth -= 1;
                    if depth == 0 {
                        close = Some(index);
                        break;
                    }
                }
                _ => {}
            }
        }
        let Some(close) = close else {
            continue;
        };
        let ends_null = close >= open + 4
            && texts[close - 1] == ";"
            && matches!(texts[close - 2], "NULL" | "0")
            && texts[close - 3] == "return";
        let returns = (open..close).filter(|k| texts[*k] == "return").count();
        let jumps = (open..close).any(|k| matches!(texts[k], "break" | "continue" | "goto"));
        if ends_null && returns == 1 && !jumps {
            blocks.push((open, close));
        }
    }
    blocks
}

/// Whether the call whose name is at `k` is `LHS = G(…);` and the next
/// statement is `if (!LHS)` or `if (LHS == NULL)` whose block is one of the
/// null-returning `blocks`: the enclosing function returns null whenever `G`
/// does.
fn null_checked_call(
    texts: &[&str],
    k: usize,
    range: (usize, usize),
    blocks: &[(usize, usize)],
) -> bool {
    if texts.get(k.wrapping_sub(1)) != Some(&"=") || texts.get(k.wrapping_sub(2)) == Some(&"=") {
        return false;
    }
    let mut start = k - 1;
    while start > range.0 && !matches!(texts[start - 1], ";" | "{" | "}") {
        start -= 1;
    }
    let lhs = &texts[start..k - 1];
    if lhs.is_empty()
        || !lhs
            .iter()
            .all(|t| is_identifier(t) || matches!(*t, "-" | ">" | "."))
    {
        return false;
    }
    let Some((_, close)) = call_arguments(texts, k + 1) else {
        return false;
    };
    if texts.get(close + 1) != Some(&";") {
        return false;
    }
    let next = close + 2;
    let condition: Vec<&str> = if texts.get(next) == Some(&"if")
        && texts.get(next + 1) == Some(&"(")
        && texts.get(next + 2) == Some(&"!")
    {
        let end = next + 3 + lhs.len();
        if texts.get(next + 3..end) != Some(lhs) || texts.get(end) != Some(&")") {
            return false;
        }
        texts[next..=end].to_vec()
    } else if texts.get(next) == Some(&"if") && texts.get(next + 1) == Some(&"(") {
        let end = next + 2 + lhs.len();
        if texts.get(next + 2..end) != Some(lhs)
            || texts.get(end..end + 4) != Some(&["=", "=", "NULL", ")"][..])
        {
            return false;
        }
        texts[next..end + 4].to_vec()
    } else {
        return false;
    };
    let open = next + condition.len();
    blocks.iter().any(|(start, _)| *start == open)
}

/// Whether the statement after `r = F(…);` (at `from`) returns from the test
/// when `r` is a null pointer: `if (!TEST_CHECK(r != NULL))`, `if (!r)`,
/// `if (r == NULL)`, followed by a block ending in `return;` or an `exit`
/// with no other jump.
fn returns_on_null(texts: &[&str], range: (usize, usize), r: &str) -> bool {
    let Some(end) = (range.0..range.1).find(|k| texts[*k] == ";") else {
        return false;
    };
    let rest = &texts[end + 1..=range.1.min(texts.len() - 1)];
    let condition_len = match rest {
        [
            "if",
            "(",
            "!",
            "TEST_CHECK",
            "(",
            x,
            "!",
            "=",
            "NULL",
            ")",
            ")",
            ..,
        ] if *x == r => 11,
        ["if", "(", "!", x, ")", ..] if *x == r => 5,
        ["if", "(", x, "=", "=", "NULL", ")", ..] if *x == r => 7,
        _ => return false,
    };
    let body = &rest[condition_len..];
    if body.first() != Some(&"{") {
        return matches!(body, ["return", ";", ..]);
    }
    let mut depth = 0usize;
    for (index, text) in body.iter().enumerate() {
        match *text {
            "{" => depth += 1,
            "}" => {
                depth -= 1;
                if depth == 0 {
                    let inner = &body[1..index];
                    let terminal = matches!(inner, [.., "return", ";"])
                        || matches!(inner, [.., "exit", "(", _, ")", ";"]);
                    let jumps = inner
                        .iter()
                        .any(|t| matches!(*t, "break" | "continue" | "goto"));
                    return terminal && !jumps;
                }
            }
            _ => {}
        }
    }
    false
}

pub(super) fn within(extents: &[(usize, usize)], position: usize) -> bool {
    extents
        .iter()
        .any(|(start, end)| *start <= position && position <= *end)
}

/// The positions `name` is assigned at in `range` (`name = …`, not `==`, not
/// a member).
pub(super) fn assignments(texts: &[&str], range: (usize, usize), name: &str) -> Vec<usize> {
    (range.0..=range.1)
        .filter(|&k| {
            texts[k] == name
                && !matches!(texts[k - 1], "." | ">")
                && texts.get(k + 1) == Some(&"=")
                && texts.get(k + 2) != Some(&"=")
        })
        .collect()
}

/// Whether `&name` (not `&&`) occurs in `range`.
pub(super) fn address_taken(texts: &[&str], range: (usize, usize), name: &str) -> bool {
    (range.0..=range.1).any(|k| {
        texts[k] == name
            && texts[k - 1] == "&"
            && texts[k - 2] != "&"
            && !matches!(texts.get(k + 1), Some(&"." | &"-"))
    })
}

/// Whether `name` is declared as a local in `definition`'s body (`T *name`,
/// `T name`, followed by `;`, `=` or `,`).
pub(super) fn declared_local(texts: &[&str], definition: &Definition, name: &str) -> bool {
    (definition.body.0..=definition.body.1).any(|k| {
        texts[k] == name
            && matches!(texts.get(k + 1), Some(&";" | &"=" | &","))
            && (texts[k - 1] == "*" || is_identifier(texts[k - 1]))
            && !matches!(texts[k - 1], "return" | "else")
    }) && !definition.parameters.contains(name)
}

/// The end of the statement starting at `from` (its `;`).
pub(super) fn semicolon(texts: &[&str], from: usize, limit: usize) -> usize {
    let mut depth = 0isize;
    for (index, text) in texts.iter().enumerate().take(limit + 1).skip(from) {
        match *text {
            "(" | "[" | "{" => depth += 1,
            ")" | "]" | "}" => depth -= 1,
            ";" if depth <= 0 => return index,
            _ => {}
        }
    }
    limit
}

/// Top-level `,`-separated argument ranges of the call whose `(` is at `open`.
pub(super) fn call_arguments(texts: &[&str], open: usize) -> Option<(Vec<(usize, usize)>, usize)> {
    let mut depth = 0usize;
    let mut ranges = Vec::new();
    let mut start = open + 1;
    for (index, text) in texts.iter().enumerate().skip(open) {
        match *text {
            "(" | "[" | "{" => depth += 1,
            ")" | "]" | "}" => {
                depth -= 1;
                if depth == 0 {
                    if start < index || !ranges.is_empty() {
                        ranges.push((start, index));
                    }
                    return Some((ranges, index));
                }
            }
            "," if depth == 1 => {
                ranges.push((start, index));
                start = index + 1;
            }
            _ => {}
        }
    }
    None
}

/// A definition's parameter names in order: the last identifier of each
/// top-level `,`-separated part of the list after its name token.
pub(super) fn ordered_parameters(texts: &[&str], header: usize) -> Vec<String> {
    let Some((ranges, _)) = call_arguments(texts, header + 1) else {
        return Vec::new();
    };
    ranges
        .iter()
        .filter_map(|(start, end)| {
            texts[*start..*end]
                .iter()
                .rev()
                .find(|t| is_identifier(t))
                .map(|t| (*t).to_owned())
        })
        .collect()
}

pub(super) const NOT_CALLS: &[&str] = &[
    "if",
    "while",
    "for",
    "switch",
    "return",
    "sizeof",
    "do",
    "else",
    "case",
    "goto",
    "typeof",
    "__typeof__",
    "_Alignof",
    "__attribute__",
    "defined",
];

impl StepB<'_> {
    fn plain(&self, id: usize) -> &str {
        let name = &self.functions[id];
        name.rsplit_once("::")
            .map_or(name.as_str(), |(_, plain)| plain)
    }

    /// The census ids `name` resolves to from `file` (a static of the same
    /// file first, else the external or header definitions).
    pub(super) fn resolve_name(&self, file: usize, name: &str) -> Vec<usize> {
        let candidates = self
            .definitions
            .iter()
            .filter(|d| d.name == name)
            .collect::<Vec<_>>();
        let own = candidates
            .iter()
            .filter(|d| d.file == file && d.census_name.contains("::"))
            .map(|d| self.index[&d.census_name])
            .collect::<BTreeSet<_>>();
        if !own.is_empty() {
            return own.into_iter().collect();
        }
        candidates
            .iter()
            .filter(|d| !d.census_name.contains("::") || self.files[d.file].path.ends_with(".h"))
            .map(|d| self.index[&d.census_name])
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    /// Whether a definition is part of the build the typed reading read:
    /// `Some(true)` when Clang read it, `Some(false)` only for a `.c` file no
    /// compile command names and no read unit includes, `None` otherwise (a
    /// header function no unit defined, a definition Clang did not see in a
    /// file it read, or a unit Clang could not read).
    pub(super) fn in_build(&self, typed: &TypedFacts, definition: &Definition) -> Option<bool> {
        let path = &self.files[definition.file].path;
        if typed
            .functions
            .contains(&format!("{path}|{}", definition.name))
        {
            return Some(true);
        }
        let prefix = format!("{path}|");
        let read = typed.units.contains(path)
            || typed.failures.contains_key(path)
            || typed
                .functions
                .range(prefix.clone()..)
                .next()
                .is_some_and(|key| key.starts_with(&prefix));
        (path.ends_with(".c") && !read).then_some(false)
    }

    /// What makes a function a hazard for the single-creation derivation.
    /// Names a file whose conditional groups the reader cannot structure may
    /// define (`name (…) {` anywhere in its tokens) that the census has no
    /// definition of: a call to one is a call the census graph does not see.
    pub(super) fn hidden_definitions(&self) -> BTreeMap<String, String> {
        let defined = self
            .definitions
            .iter()
            .map(|d| d.name.as_str())
            .collect::<BTreeSet<_>>();
        let mut hidden = BTreeMap::new();
        for (file, scan) in self.scans.iter().enumerate() {
            if !scan.unstructured {
                continue;
            }
            let texts = self.texts(file);
            for k in 0..texts.len() {
                if !is_identifier(texts[k])
                    || NOT_CALLS.contains(&texts[k])
                    || texts.get(k + 1) != Some(&"(")
                    || defined.contains(texts[k])
                {
                    continue;
                }
                let Some((_, close)) = call_arguments(&texts, k + 1) else {
                    continue;
                };
                if texts.get(close + 1) == Some(&"{") {
                    hidden
                        .entry(texts[k].to_owned())
                        .or_insert_with(|| self.files[file].path.clone());
                }
            }
        }
        hidden
    }

    pub(super) fn hazards(&self, typed: &TypedFacts) -> BTreeMap<usize, String> {
        let tables = self.flow;
        let mut hazards: BTreeMap<usize, String> = BTreeMap::new();
        // A call to a name only an unstructured file may define.
        let hidden = self.hidden_definitions();
        if !hidden.is_empty() {
            for definition in self.definitions {
                let texts = self.texts(definition.file);
                let found = (definition.body.0..=definition.body.1).find(|&k| {
                    texts.get(k + 1) == Some(&"(")
                        && !matches!(texts[k - 1], "." | ">")
                        && hidden.contains_key(texts[k])
                });
                if let Some(k) = found {
                    hazards
                        .entry(self.index[&definition.census_name])
                        .or_insert_with(|| {
                            format!(
                                "calls `{}`, which only {} (a file this reader cannot \
                                 structure) may define",
                                texts[k], hidden[texts[k]]
                            )
                        });
                }
            }
        }
        let by_plain = |file: &str, function: &str| -> Vec<usize> {
            self.definitions
                .iter()
                .filter(|d| d.name == function && self.files[d.file].path == file)
                .map(|d| self.index[&d.census_name])
                .collect()
        };
        for definition in self.definitions {
            if definition.name == self.hops.instance_constructor.0 {
                hazards
                    .entry(self.index[&definition.census_name])
                    .or_insert_with(|| "creates an input instance".to_owned());
            }
        }
        for field in [tables.list_field, tables.registrations_field] {
            for access in accesses(typed, tables.config_record, field) {
                if reads_only(access.use_label, tables.list_readers) {
                    continue;
                }
                let allowed = tables.list_initialisations.iter().any(|(f, l, call)| {
                    *f == access.function
                        && *l == field
                        && access.use_label == format!("arg:{call}")
                });
                if allowed {
                    continue;
                }
                for id in by_plain(access.file, access.function) {
                    hazards.entry(id).or_insert_with(|| {
                        format!(
                            "mutates a `{field}` list at {}:{} ({})",
                            access.file, access.line, access.use_label
                        )
                    });
                }
            }
            for access in accesses(typed, "?", field) {
                if reads_only(access.use_label, tables.list_readers) {
                    continue;
                }
                for id in by_plain(access.file, access.function) {
                    hazards.entry(id).or_insert_with(|| {
                        format!(
                            "writes a `{field}` member of an unmatched record at {}:{}",
                            access.file, access.line
                        )
                    });
                }
            }
        }
        for record in [tables.context_record, "?"] {
            for access in accesses(typed, record, tables.context_config_field) {
                if access.use_label != "write" && access.use_label != "address" {
                    continue;
                }
                if record != "?" && access.function == tables.context_constructor {
                    continue;
                }
                if record == "?" {
                    // `config` is a common member name: an unmatched write is
                    // a hazard only through a context-typed reading we lack.
                    continue;
                }
                for id in by_plain(access.file, access.function) {
                    hazards.entry(id).or_insert_with(|| {
                        format!(
                            "re-points a context's `{}` at {}:{}",
                            tables.context_config_field, access.file, access.line
                        )
                    });
                }
            }
        }
        // Functions the typed reading did not read: a member they name is not
        // classified.
        for definition in self.definitions {
            let id = self.index[&definition.census_name];
            if self.reach.unstructured.contains(&id) {
                continue;
            }
            if self.in_build(typed, definition).is_some() {
                continue;
            }
            let texts = self.texts(definition.file);
            let names_member = (definition.body.0..=definition.body.1).any(|k| {
                matches!(texts[k - 1], "." | ">")
                    && (texts[k] == tables.list_field
                        || texts[k] == tables.registrations_field
                        || (texts[k] == tables.context_config_field
                            && texts.get(k + 1) == Some(&"=")
                            && texts.get(k + 2) != Some(&"=")))
            });
            if names_member {
                hazards
                    .entry(id)
                    .or_insert_with(|| "names a listed member and has no typed reading".to_owned());
            }
        }
        // Card W47-PREMISES-INSTANCE: the premised census's hazards.
        self.premise_hazards(typed, &mut hazards);
        // A file whose conditional groups the reader cannot structure may hide
        // calls: its functions are hazards when its raw text names a listed
        // member, the constructor, or a function that reaches a hazard (to a
        // fixed point).
        let unstructured_files = self
            .reach
            .unstructured
            .iter()
            .filter_map(|id| {
                self.definitions
                    .iter()
                    .find(|d| self.index[&d.census_name] == *id)
                    .map(|d| d.file)
            })
            .collect::<BTreeSet<_>>();
        let words = unstructured_files
            .iter()
            .map(|file| {
                let words = self.files[*file]
                    .text
                    .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                    .filter(|w| !w.is_empty())
                    .map(str::to_owned)
                    .collect::<BTreeSet<_>>();
                (*file, words)
            })
            .collect::<BTreeMap<_, _>>();
        loop {
            let targets = hazards.keys().copied().collect::<BTreeSet<_>>();
            let reaching = self.reach.reaching(&targets);
            let names = reaching
                .iter()
                .map(|id| self.plain(*id).to_owned())
                .chain(
                    [
                        tables.list_field,
                        tables.registrations_field,
                        self.hops.instance_constructor.0,
                    ]
                    .map(str::to_owned),
                )
                .collect::<BTreeSet<_>>();
            let mut added = false;
            for (file, words) in &words {
                let Some(word) = words.intersection(&names).next() else {
                    continue;
                };
                for definition in self.definitions.iter().filter(|d| d.file == *file) {
                    let id = self.index[&definition.census_name];
                    if let std::collections::btree_map::Entry::Vacant(slot) = hazards.entry(id) {
                        slot.insert(format!(
                            "its file ({}) has a conditional group this reader cannot \
                             structure and names `{word}`",
                            self.files[*file].path
                        ));
                        added = true;
                    }
                }
            }
            if !added {
                break;
            }
        }
        hazards
    }

    /// A shortest path from `from` to a hazard, and the hazard.
    fn witness(&self, from: usize, hazards: &BTreeMap<usize, String>) -> String {
        let path = self.reach.path_to(from, hazards);
        let Some(last) = path.last() else {
            return "no path".to_owned();
        };
        format!(
            "{}: {}",
            path.iter()
                .map(|id| self.functions[*id].as_str())
                .collect::<Vec<_>>()
                .join(" > "),
            hazards[last]
        )
    }

    /// The literal a creator call at `file:line` passes, when the typed
    /// reading reads exactly one.
    fn literal_at(
        &self,
        typed: &TypedFacts,
        path: &str,
        line: usize,
        callee: &str,
    ) -> Option<String> {
        let literals = typed
            .literal_calls
            .get(&crate::typed_dispatch_census::literal_call_key(
                path, line, callee,
            ))?;
        let [literal] = literals.iter().collect::<Vec<_>>()[..] else {
            return None;
        };
        Some(literal.clone())
    }

    /// The coroutine flag of the descriptors a creation from `literal`
    /// matches: every `struct flb_input_plugin` object whose name equals it
    /// ignoring ASCII case (`check_protocol`, `src/flb_input.c:244`).
    pub(super) fn descriptors_flag(&self, literal: &str) -> Derivation {
        let Some(typed) = self.typed else {
            return Err("no typed reading names the descriptors".to_owned());
        };
        let prefix = format!(
            "struct {}|{}|",
            self.hops.plugin_descriptor, self.hops.plugin_name_field
        );
        let objects = typed
            .named_objects
            .iter()
            .filter_map(|(key, objects)| {
                let name = key.strip_prefix(&prefix)?;
                name.eq_ignore_ascii_case(literal).then_some(objects)
            })
            .flatten()
            .collect::<BTreeSet<_>>();
        if objects.is_empty() {
            return Err(format!(
                "no descriptor declares the plugin name \"{literal}\""
            ));
        }
        // Card W47-PREMISES-INSTANCE: the name matches a registered descriptor.
        if self.premises.is_some() {
            let note = self.registered_match(literal)?;
            self.notes.borrow_mut().insert(note);
        }
        let mut value = false;
        for object in objects {
            match self.descriptor_objects.get(object.as_str()) {
                Some(Some(flag)) => value |= *flag,
                _ => return Err(format!("the descriptor `{object}`'s flags are not derived")),
            }
        }
        Ok(value)
    }

    /// A function that creates at most one instance per call from its
    /// `argument`-th parameter's plugin name: one call of the constructor,
    /// outside every loop, passing that parameter unassigned; no other call of
    /// it reaches a hazard.
    fn creator_wrapper(
        &self,
        wrapper: usize,
        hazardous: &BTreeSet<usize>,
    ) -> Result<usize, String> {
        let name = self.functions[wrapper].clone();
        let definitions = self
            .definitions
            .iter()
            .filter(|d| self.index[&d.census_name] == wrapper)
            .collect::<Vec<_>>();
        let [definition] = definitions[..] else {
            return Err(format!("`{name}` is not defined once"));
        };
        let texts = self.texts(definition.file);
        let body = definition.body;
        let (extents, jumps) = loops(&texts, body);
        if jumps {
            return Err(format!("`{name}` has a jump this reading does not model"));
        }
        let constructor = self.hops.instance_constructor.0;
        let mut creation = None;
        for k in body.0..=body.1 {
            if !is_identifier(texts[k])
                || texts.get(k + 1) != Some(&"(")
                || matches!(texts[k - 1], "." | ">")
                || NOT_CALLS.contains(&texts[k])
            {
                continue;
            }
            if texts[k] == constructor {
                if creation.is_some() || within(&extents, k) {
                    return Err(format!("`{name}` may call `{constructor}` more than once"));
                }
                let Some((arguments, _)) = call_arguments(&texts, k + 1) else {
                    return Err(format!("`{name}`'s call of `{constructor}` is not read"));
                };
                let Some(&(start, end)) = arguments.get(self.hops.instance_constructor.1) else {
                    return Err(format!("`{name}`'s call of `{constructor}` has no name"));
                };
                creation = Some((start, end));
                continue;
            }
            for callee in self.resolve_name(definition.file, texts[k]) {
                if hazardous.contains(&callee) {
                    return Err(format!(
                        "`{name}` also calls `{}`, which reaches a hazard",
                        self.functions[callee]
                    ));
                }
            }
        }
        let Some((start, end)) = creation else {
            return Err(format!("`{name}` does not call `{constructor}`"));
        };
        let parameter = texts[start];
        let position = ordered_parameters(&texts, definition.header)
            .iter()
            .position(|p| p == parameter);
        match position {
            Some(index)
                if end == start + 1
                    && assignments(&texts, body, parameter).is_empty()
                    && !address_taken(&texts, body, parameter) =>
            {
                Ok(index)
            }
            _ => Err(format!(
                "`{name}` passes `{constructor}` a plugin name other than an unassigned parameter"
            )),
        }
    }

    /// Step B's list-head instance: `instance` is assigned once in `g`, at
    /// `start`, from the list-entry macro.
    pub(super) fn list_head_instance(
        &self,
        g: &Definition,
        instance: &str,
        start: usize,
    ) -> Derivation {
        let tables = self.flow;
        let Some(typed) = self.typed else {
            return Err("the list-head instance needs the typed reading".to_owned());
        };
        for (record, field) in [
            (tables.config_record, tables.list_field),
            (tables.config_record, tables.registrations_field),
            (tables.context_record, tables.context_config_field),
        ] {
            if !typed
                .member_fields_read
                .contains(&member_key(record, field))
            {
                return Err(format!(
                    "the typed reading did not record the accesses of `{record}::{field}`"
                ));
            }
        }
        let texts = self.texts(g.file);
        let path = self.files[g.file].path.clone();
        let line = self.scans[g.file].tokens[start].line;
        let not_tied = |why: String| -> Derivation {
            Err(format!(
                "the instance `{instance}` read as a list head at {path}:{line} is not \
                 derived to be the one creation's: {why}"
            ))
        };
        // `X = mk_list_entry(H, struct flb_input_instance, _head);`
        let Some((arguments, close)) = call_arguments(&texts, start + 1) else {
            return not_tied("the list entry is not read".to_owned());
        };
        let record = tables
            .instance_record
            .split_whitespace()
            .collect::<Vec<_>>();
        let [head, (rs, re), (fs, fe)] = arguments[..] else {
            return not_tied("the list entry does not have three arguments".to_owned());
        };
        if texts[rs..re] != record[..]
            || texts[fs..fe] != [tables.instance_link_field]
            || texts.get(close + 1) != Some(&";")
        {
            return not_tied("the list entry is not an input instance's link".to_owned());
        }
        // The head: `P -> config -> inputs . next`, inline or in a local.
        let list_path = |range: (usize, usize)| -> Option<(String, String)> {
            let tokens = &texts[range.0..range.1];
            match tokens {
                [
                    root,
                    "-",
                    ">",
                    field,
                    "-",
                    ">",
                    config,
                    "-",
                    ">",
                    list,
                    ".",
                    next,
                ] if is_identifier(root)
                    && is_identifier(field)
                    && *config == tables.context_config_field
                    && *list == tables.list_field
                    && *next == tables.list_next_field =>
                {
                    Some(((*root).to_owned(), (*field).to_owned()))
                }
                _ => None,
            }
        };
        let body = (g.body.0, g.body.1);
        let (read_at, root, field) = if head.1 == head.0 + 1 && is_identifier(texts[head.0]) {
            let local = texts[head.0];
            let assigned = assignments(&texts, body, local);
            let [at] = assigned[..] else {
                return not_tied(format!("the head `{local}` is not assigned once"));
            };
            if at > start || address_taken(&texts, body, local) || !declared_local(&texts, g, local)
            {
                return not_tied(format!("the head `{local}` is not a local read before"));
            }
            let end = semicolon(&texts, at + 2, body.1);
            let Some((root, field)) = list_path((at + 2, end)) else {
                return not_tied(format!(
                    "the head `{local}` is not a context's `{}.{}`",
                    tables.list_field, tables.list_next_field
                ));
            };
            (start, root, field)
        } else if let Some((root, field)) = list_path(head) {
            (start, root, field)
        } else {
            return not_tied("the list head is not a context's input list".to_owned());
        };
        // The context: `root = F(…)` once, F of this file returning a local
        // whose `field` is assigned once from the context constructor.
        if g.parameters.contains(&root) || !declared_local(&texts, g, &root) {
            return not_tied(format!("`{root}` is not a local of the test"));
        }
        let assigned = assignments(&texts, body, &root);
        let [at] = assigned[..] else {
            return not_tied(format!("`{root}` is not assigned once"));
        };
        if at > read_at || address_taken(&texts, body, &root) {
            return not_tied(format!("`{root}` is not assigned once before the read"));
        }
        let maker = texts[at + 2];
        if texts.get(at + 3) != Some(&"(") {
            return not_tied(format!("`{root}` is not assigned from a call"));
        }
        let makers = self
            .definitions
            .iter()
            .filter(|d| d.name == maker && d.file == g.file)
            .collect::<Vec<_>>();
        let [f] = makers[..] else {
            return not_tied(format!("`{maker}` is not one function of the test's file"));
        };
        // In the test, `root` is passed whole to no call and `root->field` is
        // never written, up to the read.
        for k in g.body.0..read_at {
            if texts[k] != root || matches!(texts[k - 1], "." | ">") {
                continue;
            }
            let whole = matches!(texts[k - 1], "(" | ",") && matches!(texts[k + 1], ")" | ",");
            let rewritten = texts.get(k + 1) == Some(&"-")
                && texts.get(k + 3) == Some(&field.as_str())
                && texts.get(k + 4) == Some(&"=")
                && texts.get(k + 5) != Some(&"=");
            if whole || rewritten {
                return not_tied(format!(
                    "`{root}` is passed on or its `{field}` rewritten before the read"
                ));
            }
        }
        if let Err(why) = self.context_maker(f, &field) {
            return not_tied(why);
        }
        // The region: the test up to the read, and the maker's body.
        let (g_loops, g_jumps) = loops(&texts, (g.body.0, read_at));
        if g_jumps {
            return not_tied("the test has a jump before the read".to_owned());
        }
        if within(&g_loops, at) {
            return not_tied(format!("`{maker}` is called inside a loop"));
        }
        let f_texts = self.texts(f.file);
        let (f_loops, f_jumps) = loops(&f_texts, f.body);
        if f_jumps {
            return not_tied(format!("`{maker}` has a jump"));
        }
        let hazards = self.hazards(typed);
        let hidden = self.hidden_definitions();
        let hazardous_set = hazards.keys().copied().collect::<BTreeSet<_>>();
        let reaching = self.reach.reaching(&hazardous_set);
        let mut creations = Vec::new();
        let mut roots = BTreeSet::new();
        // Paths on which the maker returns a null pointer end the test before
        // the read when the test checks it (acutest's `TEST_CHECK` is its
        // argument's truth); paths on which the context constructor returns
        // one never read a list (P-HEAD-INSTANCE).
        let f_pruned = if returns_on_null(&texts, (at, read_at), &root) {
            null_return_blocks(&f_texts, f.body)
        } else {
            Vec::new()
        };
        // Each part: file, token range, loop extents, the maker's call to
        // skip, the pruned null-returning blocks, and whether a callee whose
        // null result makes this part return null is read in place too.
        let mut region = vec![
            (
                g.file,
                (g.body.0, read_at),
                g_loops.clone(),
                Some(at + 2),
                Vec::new(),
                false,
            ),
            (f.file, f.body, f_loops.clone(), None, f_pruned, false),
        ];
        let mut inlined = BTreeSet::new();
        // Card W47-SERVICE-PROPERTY-KEYS: setters and property functions read
        // in place for a keyed call.
        let mut keyed_in_place = BTreeSet::new();
        // Card W47-PREMISES-HOLDER-REGION: the keyed bodies as scanned.
        let mut keyed_parts = Vec::new();
        let mut part = 0;
        while part < region.len() {
            let (file, range, extents, skip, pruned, propagate) = region[part].clone();
            part += 1;
            let texts = self.texts(file);
            let path = &self.files[file].path;
            for k in range.0..=range.1.min(texts.len() - 1) {
                if Some(k) == skip || within(&pruned, k) {
                    continue;
                }
                let text = texts[k];
                if !is_identifier(text) || NOT_CALLS.contains(&text) {
                    continue;
                }
                let member = matches!(texts[k - 1], "." | ">");
                let call = texts.get(k + 1) == Some(&"(");
                if member {
                    if call {
                        return not_tied(format!(
                            "a field dispatch at {path}:{} is in the region",
                            self.scans[file].tokens[k].line
                        ));
                    }
                    continue;
                }
                if text == tables.list_entry_macro {
                    continue;
                }
                // Card W47-SERVICE-PROPERTY-KEYS: a setter call whose keys are
                // read reaches only its keys' effects.
                if call && let Some(reading) = self.keyed_setter_call(file, k) {
                    let line = self.scans[file].tokens[k].line;
                    let site = format!("{path}:{line} {text}");
                    match reading {
                        Ok(keyed) => {
                            self.property_readings.borrow_mut().insert(
                                site.clone(),
                                keyed
                                    .keys
                                    .iter()
                                    .map(|key| format!("key {key}"))
                                    .chain(keyed.table.iter().map(|row| format!("table {row}")))
                                    .collect(),
                            );
                            self.notes.borrow_mut().insert(format!(
                                "property keys: `{text}` at {path}:{line} reaches only its keys' \
                                 effects ({})",
                                keyed.keys.join("; ")
                            ));
                            keyed_in_place.extend(keyed.in_place.iter().copied());
                            keyed_parts.extend(keyed.parts.iter().cloned());
                            for root in keyed.roots {
                                roots.insert(root);
                                if !reaching.contains(&root) {
                                    continue;
                                }
                                if within(&extents, k) {
                                    return not_tied(format!(
                                        "`{text}`'s keys at {path}:{line} reach a hazard inside a \
                                         loop ({})",
                                        self.witness(root, &hazards)
                                    ));
                                }
                                creations.push((root, file, k, line));
                            }
                            continue;
                        }
                        Err(why) => {
                            self.property_readings
                                .borrow_mut()
                                .insert(site, vec![format!("keys not read: {why}")]);
                        }
                    }
                }
                let callees = self.resolve_name(file, text);
                if callees.is_empty() {
                    if call && let Some(defined_in) = hidden.get(text) {
                        return not_tied(format!(
                            "`{text}` at {path}:{} may be defined only in {defined_in}, a \
                             file this reader cannot structure",
                            self.scans[file].tokens[k].line
                        ));
                    }
                    continue;
                }
                // The context constructor is read in place, its null-returning
                // paths pruned; so is, inside it, a callee whose null result it
                // returns as null.
                if call
                    && (text == tables.context_constructor
                        || (propagate && null_checked_call(&texts, k, range, &pruned)))
                {
                    let [constructor] = callees[..] else {
                        return not_tied(format!("`{text}` is not defined once"));
                    };
                    if within(&extents, k) {
                        return not_tied(format!("`{text}` is called inside a loop"));
                    }
                    if inlined.insert(constructor) {
                        let Some(definition) = self
                            .definitions
                            .iter()
                            .find(|d| self.index[&d.census_name] == constructor)
                        else {
                            return not_tied(format!("`{text}` is not read"));
                        };
                        let c_texts = self.texts(definition.file);
                        let (c_loops, c_jumps) = loops(&c_texts, definition.body);
                        if c_jumps {
                            return not_tied(format!("`{text}` has a jump"));
                        }
                        region.push((
                            definition.file,
                            definition.body,
                            c_loops,
                            None,
                            null_return_blocks(&c_texts, definition.body),
                            true,
                        ));
                    }
                    continue;
                }
                roots.extend(callees.iter().copied());
                for callee in callees {
                    if !reaching.contains(&callee) {
                        continue;
                    }
                    let line = self.scans[file].tokens[k].line;
                    if !call {
                        return not_tied(format!(
                            "`{text}` at {path}:{line} is named as a value and reaches a hazard"
                        ));
                    }
                    if within(&extents, k) {
                        return not_tied(format!(
                            "`{text}` at {path}:{line} reaches a hazard inside a loop ({})",
                            self.witness(callee, &hazards)
                        ));
                    }
                    creations.push((callee, file, k, line));
                }
            }
        }
        // A function read in place (the test, its maker, the context
        // constructor and what it returns null through) is itself on the path:
        // its own accesses count as a root's closure would.
        let mut blockers = Vec::new();
        let own = [self.index[&g.census_name], self.index[&f.census_name]]
            .into_iter()
            .chain(inlined.iter().copied())
            .chain(keyed_in_place.iter().copied());
        let own = own.collect::<BTreeSet<_>>();
        for id in &own {
            if let Some(why) = hazards.get(id) {
                blockers.push(format!("`{}`, read in place, {why}", self.functions[*id]));
            }
        }
        // Card W47-PREMISES-HOLDER-REGION: every macro invocation on the
        // region, read from its unit's own expansion.
        // With the facts, the functions the roots reach and those that reach a
        // hazard over the census graph and its macro-made calls.
        let mut macro_closure = BTreeSet::new();
        let mut macro_reaching = BTreeSet::new();
        let macro_unmodelled = match self.region_macros {
            None => Vec::new(),
            Some(view) => {
                let parts = region
                    .iter()
                    .map(|(file, range, _, _, pruned, _)| (*file, *range, pruned.clone()))
                    .chain(keyed_parts.iter().cloned())
                    .collect::<Vec<_>>();
                let witness = |id: usize| self.witness(id, &hazards);
                let read = match self.region_macro_calls(view, &parts, &reaching, &witness) {
                    Ok(read) => read,
                    Err(why) => return not_tied(why),
                };
                roots.extend(read.roots.iter().copied());
                // The census graph the region reaches over: the macro-made
                // calls of every function a root reaches.
                match self.closure_macro_calls(view, &roots, &reaching) {
                    Ok(widened) => {
                        self.notes.borrow_mut().insert(widened.note);
                        macro_closure = widened.closure;
                        macro_reaching = widened.reaching;
                    }
                    Err(why) => return not_tied(why),
                }
                let mut unmodelled = read.unmodelled.clone();
                unmodelled.extend(self.own_indirect_calls(typed, &own));
                self.notes.borrow_mut().insert(format!(
                    "region macros: {} invocation(s) of {} macro(s) on the list-head region read \
                     from their units' expansions, none naming or carrying a function that \
                     reaches a hazard (P-RECORDED-COMPILER, P-DUMP-TOKENS-COMPLETE)",
                    read.invocations,
                    read.names.len()
                ));
                unmodelled
            }
        };
        // An unmodelled call (through a function pointer the census does not
        // read) reaches only an escaped function; it matters only where the
        // region's closure makes one, or has a function the typed reading
        // could not read. A definition the build does not compile (a `.c` file
        // no compile command names, or a definition Clang did not see in a
        // unit it read) is not part of the program.
        let mut closure = self.reach.closure(roots.iter().copied());
        closure.extend(keyed_in_place.iter().copied());
        closure.extend(macro_closure.iter().copied());
        let unmodelled = closure
            .iter()
            .copied()
            .filter(|id| {
                let Some(definition) = self
                    .definitions
                    .iter()
                    .find(|d| self.index[&d.census_name] == *id)
                else {
                    return true;
                };
                match self.in_build(typed, definition) {
                    Some(true) => {
                        let path = &self.files[definition.file].path;
                        let key = format!("{path}|{}|", definition.name);
                        typed
                            .indirect_calls
                            .range(key.clone()..)
                            .next()
                            .is_some_and(|site| site.starts_with(&key))
                    }
                    Some(false) => false,
                    None => true,
                }
            })
            .collect::<Vec<_>>();
        if let Some(first) = unmodelled.first() {
            let wild = self
                .reach
                .wild
                .iter()
                .filter(|(id, _)| reaching.contains(id) || macro_reaching.contains(id))
                .collect::<Vec<_>>();
            if let Some((id, why)) = wild.first() {
                let targets = BTreeMap::from([(*first, String::new())]);
                let witness = roots
                    .iter()
                    .map(|root| self.reach.path_to(*root, &targets))
                    .filter(|path| !path.is_empty())
                    .min_by_key(Vec::len)
                    .unwrap_or_default()
                    .iter()
                    .map(|id| self.functions[*id].as_str())
                    .collect::<Vec<_>>()
                    .join(" > ");
                blockers.push(format!(
                    "`{}` ({why}) reaches a hazard ({}) and may be called through a pointer \
                     from the path: {} function(s) of its closure make an unmodelled call or \
                     have no typed reading ({witness}, first; all: {})",
                    self.functions[**id],
                    self.witness(**id, &hazards),
                    unmodelled.len(),
                    unmodelled
                        .iter()
                        .map(|id| self.functions[*id].as_str())
                        .collect::<Vec<_>>()
                        .join(", "),
                ));
            }
        }
        if !macro_unmodelled.is_empty() {
            let wild = self
                .reach
                .wild
                .iter()
                .filter(|(id, _)| reaching.contains(id) || macro_reaching.contains(id))
                .collect::<Vec<_>>();
            if let Some((id, why)) = wild.first() {
                blockers.push(format!(
                    "`{}` ({why}) reaches a hazard ({}) and may be called through a pointer \
                     from the region: {}",
                    self.functions[**id],
                    self.witness(**id, &hazards),
                    macro_unmodelled.join("; ")
                ));
            }
        }
        if creations.len() != 1 {
            let named = creations
                .iter()
                .map(|(id, file, _, line)| {
                    format!(
                        "`{}` at {}:{line} ({})",
                        self.functions[*id],
                        self.files[*file].path,
                        self.witness(*id, &hazards)
                    )
                })
                .collect::<Vec<_>>();
            blockers.push(if named.is_empty() {
                "no creation on the path".to_owned()
            } else {
                format!("more than one call reaches a hazard: {}", named.join(", "))
            });
        }
        if !blockers.is_empty() {
            return not_tied(blockers.join("; "));
        }
        let [(creator, file, _, line)] = creations[..] else {
            return not_tied("no single creation".to_owned());
        };
        let creator_name = self.plain(creator).to_owned();
        let argument = if creator_name == self.hops.instance_constructor.0 {
            self.hops.instance_constructor.1
        } else {
            match self.creator_wrapper(creator, &reaching) {
                Ok(argument) => argument,
                Err(why) => return not_tied(why),
            }
        };
        if !tables
            .creator_wrappers
            .iter()
            .any(|(w, a)| *w == creator_name && *a == argument)
            && creator_name != self.hops.instance_constructor.0
        {
            return not_tied(format!(
                "`{creator_name}`'s plugin name is not a literal the typed reading records"
            ));
        }
        // The constructor itself creates nothing else and mutates no list
        // but its own insertion.
        let constructor_ids = self
            .definitions
            .iter()
            .filter(|d| d.name == self.hops.instance_constructor.0)
            .map(|d| self.index[&d.census_name])
            .collect::<BTreeSet<_>>();
        for constructor in &constructor_ids {
            let callees = self
                .reach
                .next
                .get(constructor)
                .cloned()
                .unwrap_or_default();
            if let Some(bad) = callees.iter().find(|c| reaching.contains(c)) {
                return not_tied(format!(
                    "the constructor calls `{}`, which reaches a hazard",
                    self.functions[*bad]
                ));
            }
            if let Some(why) = hazards.get(constructor)
                && why != "creates an input instance"
            {
                return not_tied(format!("the constructor {why}"));
            }
        }
        let path = &self.files[file].path;
        let Some(literal) = self.literal_at(typed, path, line, &creator_name) else {
            return not_tied(format!(
                "the plugin name `{creator_name}` is called with at {path}:{line} is not one \
                 literal the typed reading reads"
            ));
        };
        let flag = self.descriptors_flag(&literal)?;
        // Card W47-PREMISES-HOLDER-REGION: the literal the copy's source was
        // created from.
        self.copy_literal.replace(Some(literal.clone()));
        // Card W47-PREMISES-INSTANCE: in the premised census the premises the
        // note names are derived, or the head is not tied.
        if self.premises.is_some() {
            let notes = match self.head_premises() {
                Ok(notes) => notes,
                Err(why) => return not_tied(why),
            };
            self.notes.borrow_mut().insert(format!(
                "list-head instance: the one creation `{creator_name}(…, \"{literal}\", …)` at \
                 {path}:{line} on the context `{maker}` returns"
            ));
            self.notes.borrow_mut().extend(notes);
            return Ok(flag);
        }
        self.notes.borrow_mut().insert(format!(
            "list-head instance: the one creation `{creator_name}(…, \"{literal}\", …)` at \
             {path}:{line} on the context `{maker}` returns (P-HEAD-INSTANCE, \
             P-FRESH-CONTEXT, P-REGISTERED-DESCRIPTORS)"
        ));
        Ok(flag)
    }

    /// The function a test's context comes from: every `return` returns a
    /// null pointer or one local `l`, `l->field` is assigned exactly once,
    /// from the context constructor called with no argument, and `l` is
    /// otherwise passed whole only to the release function.
    fn context_maker(&self, f: &Definition, field: &str) -> Result<(), String> {
        let tables = self.flow;
        let texts = self.texts(f.file);
        let body = f.body;
        let mut local = None;
        for k in body.0..=body.1 {
            if texts[k] != "return" {
                continue;
            }
            let end = semicolon(&texts, k + 1, body.1);
            match &texts[k + 1..end] {
                ["NULL"] | ["0"] => {}
                [name] if is_identifier(name) => {
                    if local.is_some_and(|l| l != *name) {
                        return Err(format!("`{}` returns two locals", f.name));
                    }
                    local = Some(*name);
                }
                other => {
                    return Err(format!(
                        "`{}` returns `{}`, not a null pointer or a local",
                        f.name,
                        other.join(" ")
                    ));
                }
            }
        }
        let Some(local) = local else {
            return Err(format!("`{}` returns no context holder", f.name));
        };
        if !declared_local(&texts, f, local) || address_taken(&texts, body, local) {
            return Err(format!("`{local}` is not a local of `{}`", f.name));
        }
        let mut made = 0;
        for k in body.0..=body.1 {
            if texts[k] != local || matches!(texts[k - 1], "." | ">") {
                continue;
            }
            if texts.get(k + 1) == Some(&"-")
                && texts.get(k + 2) == Some(&">")
                && texts.get(k + 3) == Some(&field)
                && texts.get(k + 4) == Some(&"=")
                && texts.get(k + 5) != Some(&"=")
            {
                if texts.get(k + 5) == Some(&tables.context_constructor)
                    && texts.get(k + 6) == Some(&"(")
                    && texts.get(k + 7) == Some(&")")
                    && texts.get(k + 8) == Some(&";")
                {
                    made += 1;
                    continue;
                }
                return Err(format!(
                    "`{local}->{field}` is assigned other than by `{}`",
                    tables.context_constructor
                ));
            }
            let whole = matches!(texts[k - 1], "(" | ",") && matches!(texts[k + 1], ")" | ",");
            if whole && !(texts[k - 1] == "(" && texts[k - 2] == tables.free_function) {
                return Err(format!(
                    "`{local}` is passed whole to a call in `{}`",
                    f.name
                ));
            }
        }
        if made != 1 {
            return Err(format!(
                "`{local}->{field}` is not assigned exactly once from `{}`",
                tables.context_constructor
            ));
        }
        Ok(())
    }

    /// The holder of a plugin copy (`root`, whose member `member` is the copy)
    /// escapes into the installed instance's `data` field: admitted only when
    /// nothing the escape can reach derives the route.
    pub(super) fn holder_escape(
        &self,
        g: &Definition,
        root: &str,
        member: &str,
    ) -> Result<(), String> {
        let tables = self.flow;
        let Some(typed) = self.typed else {
            return Err("the holder escape needs the typed reading".to_owned());
        };
        let texts = self.texts(g.file);
        // The holder's record type, from its declaration.
        let tag = (g.body.0..=g.body.1).find_map(|k| {
            (texts[k] == root
                && texts[k - 2] == "struct"
                && is_identifier(texts[k - 1])
                && matches!(texts.get(k + 1), Some(&";" | &"=")))
            .then(|| texts[k - 1])
        });
        let Some(tag) = tag else {
            return Err(format!(
                "the object `{root}` holding the copy escapes (its address is taken) and is \
                 not a named struct"
            ));
        };
        for (file, scan) in self.scans.iter().enumerate() {
            if file == g.file {
                continue;
            }
            if scan
                .tokens
                .windows(2)
                .any(|w| w[0].text == "struct" && w[1].text == tag)
            {
                return Err(format!(
                    "the holder's type `struct {tag}` is also named in {}",
                    self.files[file].path
                ));
            }
        }
        // In the test's file, the copy member is written or its address taken
        // only in `g` (which Step B models).
        let assignment = |k: usize| -> bool {
            match (texts.get(k).copied(), texts.get(k + 1).copied()) {
                (Some("="), Some(next)) => next != "=",
                (Some("+" | "-"), Some(next)) => next == "=" || next == texts[k],
                (Some("|" | "&" | "^" | "*" | "/" | "%"), Some("=")) => true,
                (Some("<" | ">"), Some(next)) => next == texts[k] && texts.get(k + 2) == Some(&"="),
                _ => false,
            }
        };
        for k in 1..texts.len() {
            if texts[k] != member || !matches!(texts[k - 1], "." | ">") {
                continue;
            }
            if g.body.0 <= k && k <= g.body.1 {
                continue;
            }
            // The member chain `member(.f)*`.
            let mut end = k + 1;
            while texts.get(end) == Some(&".")
                && texts.get(end + 1).is_some_and(|t| is_identifier(t))
            {
                end += 2;
            }
            // The chain's start: `x.member` or `x->member`.
            let mut begin = k - 1;
            if texts[begin] == ">" {
                begin -= 1;
            }
            begin = begin.saturating_sub(1);
            let addressed = begin > 0
                && texts[begin - 1] == "&"
                && texts.get(begin.wrapping_sub(2)) != Some(&"&");
            let incremented = matches!(
                (
                    texts.get(begin.wrapping_sub(1)),
                    texts.get(begin.wrapping_sub(2))
                ),
                (Some(&"+"), Some(&"+")) | (Some(&"-"), Some(&"-"))
            );
            if assignment(end) || addressed || incremented {
                return Err(format!(
                    "the copy member `{member}` is written or its address taken at {}:{}",
                    self.files[g.file].path, self.scans[g.file].tokens[k].line
                ));
            }
        }
        // The instance's plugin field is written only by the constructor and
        // the installation in `g`.
        let plugin_field = self.hops.instance_plugin_field;
        if !typed
            .member_fields_read
            .contains(&member_key(tables.instance_record, plugin_field))
        {
            return Err(format!(
                "the typed reading did not record `{plugin_field}`'s writers"
            ));
        }
        let path = &self.files[g.file].path;
        for access in accesses(typed, tables.instance_record, plugin_field) {
            if reads_only(access.use_label, &[]) {
                continue;
            }
            let constructor =
                access.function == self.hops.instance_constructor.0 && access.use_label == "write";
            let installation =
                access.function == g.name && access.file == path && access.use_label == "write";
            if !(constructor || installation) {
                return Err(format!(
                    "an instance's `{plugin_field}` is written at {}:{} in `{}` ({})",
                    access.file, access.line, access.function, access.use_label
                ));
            }
        }
        for access in accesses(typed, "?", plugin_field) {
            if !reads_only(access.use_label, &[]) {
                return Err(format!(
                    "a `{plugin_field}` member of an unmatched record is written at {}:{}",
                    access.file, access.line
                ));
            }
        }
        // Card W47-PREMISES-HOLDER-REGION: P-HOLDER-OPAQUE is derived once
        // the instance's creation is (`holder_opaque`, from `copy_flag`).
        self.notes.borrow_mut().insert(format!(
            "holder escape: `{root}` (`struct {tag}`, named only in {path}) escapes into the \
             installed instance's `{}`",
            tables.instance_data_field,
        ));
        Ok(())
    }

    /// P-CALLBACK-INSTANCE for a callback `p` stored in plugin copies' fields
    /// `fields` (keyed by `keys`): every dispatch that can reach it passes the
    /// instance whose plugin it dispatches through, and a collector's
    /// instance is its registration's.
    pub(super) fn callback_instance(
        &self,
        p: usize,
        fields: &BTreeSet<String>,
        keys: &BTreeSet<String>,
        any_key: bool,
    ) -> Result<(), String> {
        let plugin_field = self.hops.instance_plugin_field;
        let p_definition = self
            .definitions
            .iter()
            .find(|d| self.index[&d.census_name] == p)
            .ok_or("the callback is not defined")?;
        let underived = |why: String| Err(format!("P-CALLBACK-INSTANCE is not derived: {why}"));
        for site in self.dispatch_sites {
            if !fields.contains(&site.field) {
                continue;
            }
            let reaches = match &site.records {
                None => true,
                Some(records) => any_key || !records.is_disjoint(keys),
            };
            if !reaches {
                continue;
            }
            let texts = self.texts(site.file);
            let k = site.position;
            let path = &self.files[site.file].path;
            let line = self.scans[site.file].tokens[k].line;
            let Some((arguments, _)) = call_arguments(&texts, k + 1) else {
                return underived(format!("the dispatch at {path}:{line} is not read"));
            };
            let first = arguments
                .first()
                .filter(|(s, e)| e - s == 1 && is_identifier(texts[*s]))
                .map(|(s, _)| texts[*s]);
            let Some(first) = first else {
                return underived(format!(
                    "the dispatch at {path}:{line} does not pass an instance variable first"
                ));
            };
            let caller = self
                .definitions
                .iter()
                .find(|d| self.index[&d.census_name] == site.caller && d.file == site.file)
                .ok_or("the dispatch's caller is not read")?;
            let body = caller.body;
            // `X -> p -> F (X, …)`.
            let direct = k >= 6
                && texts[k - 1] == ">"
                && texts[k - 2] == "-"
                && texts[k - 3] == plugin_field
                && texts[k - 4] == ">"
                && texts[k - 5] == "-"
                && texts[k - 6] == first
                && !matches!(texts[k - 7], "." | ">");
            // `Q -> F (X, …)` after `Q = X -> p ;`, Q assigned once.
            let through_local = k >= 3
                && texts[k - 1] == ">"
                && texts[k - 2] == "-"
                && is_identifier(texts[k - 3])
                && !matches!(texts[k - 4], "." | ">")
                && {
                    let q = texts[k - 3];
                    let assigned = assignments(&texts, body, q);
                    matches!(assigned[..], [at] if at < k
                        && texts.get(at + 2) == Some(&first)
                        && texts.get(at + 3) == Some(&"-")
                        && texts.get(at + 4) == Some(&">")
                        && texts.get(at + 5) == Some(&plugin_field)
                        && texts.get(at + 6) == Some(&";")
                        && !address_taken(&texts, body, q)
                        && assignments(&texts, (at, k), first).is_empty())
                };
            // The callback itself, on its own unassigned first parameter.
            let own = site.caller == p
                && p_definition.first_parameter.as_deref() == Some(first)
                && assignments(&texts, body, first).is_empty();
            let stable = assignments(&texts, body, first).len() <= 1;
            if !((direct && stable) || through_local || own) {
                return underived(format!(
                    "the dispatch of `{}` at {path}:{line} does not pass the instance whose \
                     `{plugin_field}` it goes through",
                    site.field
                ));
            }
        }
        // The collector's instance.
        let Some(typed) = self.typed else {
            return underived("no typed reading".to_owned());
        };
        let tables = self.flow;
        let collector = self.hops.collector_record;
        let field = tables.collector_instance_field;
        if !typed
            .member_fields_read
            .contains(&member_key(collector, field))
        {
            return underived(format!(
                "the typed reading did not record `{field}`'s writers"
            ));
        }
        for access in accesses(typed, collector, field).into_iter().chain(
            accesses(typed, "?", field)
                .into_iter()
                .filter(|a| a.use_label == "write"),
        ) {
            if reads_only(access.use_label, &[]) {
                continue;
            }
            if access.function != tables.collector_constructor || access.use_label != "write" {
                return underived(format!(
                    "a collector's `{field}` is written at {}:{} in `{}`",
                    access.file, access.line, access.function
                ));
            }
        }
        let stored = typed.parameter_stores.iter().any(|store| {
            store.function == tables.collector_constructor
                && store.record == collector
                && store.field == field
        });
        if !stored {
            return underived(format!(
                "`{}` does not store a parameter in `{field}`",
                tables.collector_constructor
            ));
        }
        // Each registration the census reads passes its first parameter to
        // the collector constructor.
        for (registration, _) in self.hops.collector_registrations {
            for definition in self.definitions.iter().filter(|d| d.name == *registration) {
                let texts = self.texts(definition.file);
                let Some(first) = definition.first_parameter.as_deref() else {
                    return underived(format!("`{registration}` has no first parameter"));
                };
                let calls = (definition.body.0..=definition.body.1)
                    .filter(|&k| {
                        texts[k] == tables.collector_constructor && texts.get(k + 1) == Some(&"(")
                    })
                    .collect::<Vec<_>>();
                let [k] = calls[..] else {
                    return underived(format!(
                        "`{registration}` does not call `{}` once",
                        tables.collector_constructor
                    ));
                };
                let passes = call_arguments(&texts, k + 1).is_some_and(|(arguments, _)| {
                    arguments
                        .iter()
                        .any(|(s, e)| e - s == 1 && texts[*s] == first)
                });
                if !passes || !assignments(&texts, definition.body, first).is_empty() {
                    return underived(format!(
                        "`{registration}` does not pass its unassigned first parameter to `{}`",
                        tables.collector_constructor
                    ));
                }
            }
        }
        self.notes.borrow_mut().insert(
            "P-CALLBACK-INSTANCE derived: every dispatch of the copy's callback field passes the \
             instance whose plugin it goes through, and a collector's instance is its \
             registration's"
                .to_owned(),
        );
        Ok(())
    }
}
