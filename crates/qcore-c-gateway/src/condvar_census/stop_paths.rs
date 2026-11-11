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

//! Stop-path readings for the C5 census (card **W47-CONDVAR-JOIN-STOP-PATHS**,
//! gateway rule 10q as amended): the lexical facts the path reader needs to
//! exclude an infeasible null branch, to tell a first join from a repeated
//! one, and to decide the POSIX error cases of an unchecked join.
//!
//! Everything here is a *reading*; the kernel
//! (`qcore_core::condvar_join::{null_branch_excluded, join_succeeds}`)
//! decides, through the adapter in `quirin_source_analysis`. A fact the reader
//! cannot establish is reported as not established, never assumed.
//!
//! * **Null tests** ([`NullTestReading`]): a test `!X->F`, `X->F == NULL`,
//!   `X->F == 0`, `X->F != NULL` or `X->F` in a function the path reader
//!   reads, with the earlier dereference of `X->F` on every path to it in the
//!   same function (a top-level `X->F->…`, `X->F[…]` or `*X->F` in a
//!   statement that precedes it, outside every call's argument list, `sizeof`
//!   and every conditional operator), what the function does to `X` and
//!   `X->F` (an assignment or `&X` defeats it), `X`'s declared type, and every
//!   store of `F` into an object of that type in the file set, each classified
//!   as a **pre-publication** store or not.
//! * **Thread-identifier sites** ([`ThreadIdSite`]): every `pthread_join` and
//!   `pthread_detach` with the entries its identifier may name, read as the
//!   path reader reads a join, program-wide.
//! * **`pthread_self` sites** ([`SelfSite`]), and whether the value is only
//!   compared or signalled (`pthread_equal`, `pthread_kill`) or may be kept.
//! * **Spawn wrappers** ([`WrapperReading`]): whether a wrapper's body
//!   creates its thread joinable.
//!
//! Premises stated, not proved (trust boundary 28): the tokenizer's reading;
//! that a declaration the reader finds is the one in scope (a shadowing
//! declaration of another type makes the name unread); C's effective-type
//! rule (a store through another type does not write this field).

use std::collections::{BTreeMap, BTreeSet};

use super::{
    Definition, FileScan, KEYWORDS, Stmt, SyncTables, arguments, calls_in, matching, parse_body,
    texts,
};
use crate::legacy_c::{Token, is_identifier};

/// One null test of a field in a function the path reader reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NullTestReading {
    pub file: String,
    pub line: usize,
    pub function: String,
    /// `X` and `F` of `X->F`.
    pub base: String,
    pub field: String,
    /// Whether the condition holds exactly when the field is null (`!X->F`,
    /// `X->F == NULL`); otherwise it holds exactly when it is not null.
    pub null_when_true: bool,
    /// The earlier statement dereferencing `X->F` on every path to the test.
    pub dominating_dereference: Option<String>,
    /// Why the function-local part fails, if it does.
    pub local_defeat: Option<String>,
    /// `X`'s declared type (`struct TAG` or a type name), when read.
    pub base_type: Option<String>,
    /// Every store of `F` into an object of that type in the file set: its
    /// site and whether it is a pre-publication store (`Ok`) or why not.
    pub stores: Vec<(String, Result<(), String>)>,
    /// Writes the reader cannot rule out.
    pub unresolved: Vec<String>,
    /// The kernel's decision, filled in by the adapter's decider.
    pub excluded: Result<(), String>,
    /// The token index of the condition's first token (the key the path
    /// reader looks the decision up by).
    pub(super) cond_start: usize,
    pub(super) file_index: usize,
}

/// A `pthread_join` or `pthread_detach` call, program-wide.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThreadIdSite {
    pub file: String,
    pub line: usize,
    /// The enclosing function's census name.
    pub function: String,
    /// `join` or `detach`.
    pub call: &'static str,
    /// The entries the identifier may name, or why it is not read.
    pub identity: Result<BTreeSet<String>, String>,
}

/// A `pthread_self()` call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelfSite {
    pub file: String,
    pub line: usize,
    pub function: String,
    /// The value may be kept (anything but a direct argument of
    /// `pthread_equal` or `pthread_kill`).
    pub published: bool,
}

/// Whether a spawn wrapper creates its thread joinable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WrapperReading {
    pub wrapper: String,
    /// `Ok` with the site of the creation, or why not derived.
    pub joinable: Result<String, String>,
}

/// The name a spawn through `pthread_create` with a null attribute object
/// records as its wrapper: POSIX's default detach state is joinable.
pub const DEFAULT_ATTRIBUTE_CREATE: &str = "pthread_create(NULL attributes)";

const ASSIGNMENTS: &[&str] = &[
    "=", "+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=", "<<=", ">>=",
];
const QUALIFIERS: &[&str] = &[
    "const",
    "volatile",
    "restrict",
    "__restrict",
    "static",
    "register",
    "extern",
    "inline",
    "unsigned",
    "signed",
];

/// Stores of a field, each with whether it is a pre-publication store, and
/// the writes the reader cannot rule out.
type FieldWrites = (Vec<(String, Result<(), String>)>, Vec<String>);

/// A condition and the statements executed before it on every path.
type Preceded = ((usize, usize), Vec<(usize, usize)>);

fn site(scan: &FileScan, index: usize) -> String {
    format!("{}:{}", scan.path, scan.tokens[index].line)
}

// ---------------------------------------------------------------- types

/// `typedef struct TAG NAME;` and `typedef struct TAG { … } NAME;`, all files.
pub(super) fn typedefs(scans: &[FileScan]) -> BTreeMap<String, String> {
    let mut out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for scan in scans {
        let tokens = &scan.tokens;
        for index in 0..tokens.len() {
            if tokens[index].text != "typedef" {
                continue;
            }
            let Some(kind) = tokens.get(index + 1) else {
                continue;
            };
            if !matches!(kind.text.as_str(), "struct" | "union") {
                continue;
            }
            let Some(tag) = tokens.get(index + 2).filter(|t| is_identifier(&t.text)) else {
                continue;
            };
            let mut cursor = index + 3;
            if tokens.get(cursor).is_some_and(|t| t.text == "{") {
                let Some(close) = matching(tokens, cursor, "{", "}") else {
                    continue;
                };
                cursor = close + 1;
            }
            if let (Some(name), Some(end)) = (tokens.get(cursor), tokens.get(cursor + 1))
                && is_identifier(&name.text)
                && end.text == ";"
            {
                out.entry(name.text.clone())
                    .or_default()
                    .insert(format!("{} {}", kind.text, tag.text));
            }
        }
    }
    // A name typedef'd to two different tags is not resolved.
    out.into_iter()
        .filter_map(|(name, tags)| {
            (tags.len() == 1).then(|| (name, tags.into_iter().next().unwrap_or_default()))
        })
        .collect()
}

/// The type facts the declared-type reader uses: `typedef`s and file-scope
/// declarations, per file and program-wide.
#[derive(Default)]
pub(super) struct Types {
    pub(super) typedefs: BTreeMap<String, String>,
    /// File-scope declarations: `(file, name)` → types.
    by_file: BTreeMap<(usize, String), BTreeSet<(String, usize)>>,
    /// Every file's file-scope declarations of a name.
    anywhere: BTreeMap<String, BTreeSet<(String, usize)>>,
}

/// The canonical type and pointer depth of a declaration's words before its
/// declarator name, when the reader can split them.
fn declaration_type(
    before: &[&str],
    typedefs: &BTreeMap<String, String>,
) -> Option<(String, usize)> {
    let stars = before.iter().rev().take_while(|w| **w == "*").count();
    let core: Vec<&str> = before[..before.len() - stars]
        .iter()
        .copied()
        .filter(|w| !QUALIFIERS.contains(w))
        .collect();
    let ty = match core.as_slice() {
        [kind @ ("struct" | "union"), tag] if is_identifier(tag) => format!("{kind} {tag}"),
        [word] if is_identifier(word) && !KEYWORDS.contains(word) => typedefs
            .get(*word)
            .cloned()
            .unwrap_or_else(|| (*word).to_owned()),
        _ => return None,
    };
    Some((ty, stars))
}

pub(super) fn types(scans: &[FileScan]) -> Types {
    let typedefs = typedefs(scans);
    let mut by_file: BTreeMap<(usize, String), BTreeSet<(String, usize)>> = BTreeMap::new();
    for (file, scan) in scans.iter().enumerate() {
        let tokens = &scan.tokens;
        let mut depth = 0usize;
        let mut start = 0usize;
        for index in 0..tokens.len() {
            match tokens[index].text.as_str() {
                "{" => depth += 1,
                "}" => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        start = index + 1;
                    }
                }
                ";" if depth == 0 => start = index + 1,
                name if depth == 0 && is_identifier(name) && !KEYWORDS.contains(&name) => {
                    let next = tokens.get(index + 1).map_or("", |t| t.text.as_str());
                    if !matches!(next, ";" | "=" | "[") || start > index {
                        continue;
                    }
                    let words = texts(tokens, (start, index));
                    if words.first() == Some(&"typedef") {
                        continue;
                    }
                    let words: Vec<&str> = words
                        .into_iter()
                        .filter(|w| !matches!(*w, "extern" | "static"))
                        .collect();
                    if let Some(found) = declaration_type(&words, &typedefs) {
                        by_file
                            .entry((file, name.to_owned()))
                            .or_default()
                            .insert(found);
                    }
                }
                _ => {}
            }
        }
    }
    let mut anywhere: BTreeMap<String, BTreeSet<(String, usize)>> = BTreeMap::new();
    for ((_, name), found) in &by_file {
        anywhere
            .entry(name.clone())
            .or_default()
            .extend(found.iter().cloned());
    }
    Types {
        typedefs,
        by_file,
        anywhere,
    }
}

fn one(found: BTreeSet<(String, usize)>) -> Option<(String, usize)> {
    let mut found = found.into_iter();
    let first = found.next()?;
    found.next().is_none().then_some(first)
}

/// The declared type of `name` in a definition: its parameters and body
/// declarations, else a file-scope declaration in its file, else one in the
/// program; canonical (`struct TAG`), with the number of `*`s. `None` when
/// no declaration is found or two at the same level disagree.
pub(super) fn declared_type(
    scan: &FileScan,
    definition: &Definition,
    name: &str,
    types: &Types,
) -> Option<(String, usize)> {
    let local = local_type(scan, definition, name, &types.typedefs);
    if local.is_some() {
        return local.and_then(one);
    }
    if let Some(found) = types.by_file.get(&(definition.file, name.to_owned())) {
        return one(found.clone());
    }
    types.anywhere.get(name).cloned().and_then(one)
}

/// The local declarations of `name` (parameters and body), `None` when there
/// is none.
fn local_type(
    scan: &FileScan,
    definition: &Definition,
    name: &str,
    typedefs: &BTreeMap<String, String>,
) -> Option<BTreeSet<(String, usize)>> {
    let tokens = &scan.tokens;
    let mut found: BTreeSet<(String, usize)> = BTreeSet::new();
    // A local the reader sees declared but cannot type still shadows every
    // file-scope declaration: it answers `Some` with nothing in it.
    let mut declared = false;
    // The parameter list: the `( … )` before the body; unread, nothing is
    // typed and nothing falls back.
    let Some(close) = definition.body.0.checked_sub(1) else {
        return Some(found);
    };
    let Some(open) = (0..close)
        .rev()
        .find(|i| tokens[*i].text == "(" && matching(tokens, *i, "(", ")") == Some(close))
    else {
        return Some(found);
    };
    let mut declarations: Vec<(usize, usize)> = Vec::new();
    for (start, end) in arguments(tokens, open, close) {
        declarations.push((start, end));
    }
    // Body declarations: a type word, stars, then the name and `;`, `=`, `,`
    // or `[`.
    let (body_open, body_close) = definition.body;
    for index in body_open + 1..body_close {
        if tokens[index].text != name {
            continue;
        }
        let next = tokens.get(index + 1).map_or("", |t| t.text.as_str());
        if !matches!(next, ";" | "=" | "," | "[") {
            continue;
        }
        let mut cursor = index;
        while cursor > body_open + 1 && tokens[cursor - 1].text == "*" {
            cursor -= 1;
        }
        let Some(type_word) = cursor.checked_sub(1).map(|i| &tokens[i]) else {
            continue;
        };
        // A further declarator of a declaration (`T a, *NAME;`) declares the
        // name with a type this reader does not split.
        if type_word.text == ","
            && cursor < index + 1
            && tokens[cursor..index].iter().all(|t| t.text == "*")
        {
            declared = true;
            continue;
        }
        if !is_identifier(&type_word.text) || KEYWORDS.contains(&type_word.text.as_str()) {
            continue;
        }
        // The start of the declaration: back to `;`, `{`, `}` or `(`.
        let start = (body_open..cursor)
            .rev()
            .find(|i| matches!(tokens[*i].text.as_str(), ";" | "{" | "}" | "(" | ","))
            .map_or(cursor - 1, |i| i + 1);
        declarations.push((start, index + 1));
    }
    for (start, end) in declarations {
        let words = texts(tokens, (start, end));
        let Some(position) = words.iter().rposition(|w| *w == name) else {
            continue;
        };
        declared = true;
        // A parameter's or a declaration's words the reader cannot split (a
        // function pointer, several words) name no type here.
        if let Some(ty) = declaration_type(&words[..position], typedefs) {
            found.insert(ty);
        }
    }
    declared.then_some(found)
}

// ----------------------------------------------------------- null tests

/// The shape of a null test on `X->F`: `(base, field, null_when_true)`.
fn null_test(
    tokens: &[Token],
    cond: (usize, usize),
    tables: &SyncTables,
) -> Option<(String, String, bool)> {
    let (negated, inner) = super::condition_atom(tokens, cond, tables);
    let words = texts(tokens, inner);
    let (base, field, null_when_true) = match words.as_slice() {
        [base, "->", field] => (*base, *field, false),
        [base, "->", field, "==", "NULL" | "0"] | ["NULL" | "0", "==", base, "->", field] => {
            (*base, *field, true)
        }
        [base, "->", field, "!=", "NULL" | "0"] | ["NULL" | "0", "!=", base, "->", field] => {
            (*base, *field, false)
        }
        _ => return None,
    };
    (is_identifier(base) && is_identifier(field))
        .then(|| (base.to_owned(), field.to_owned(), null_when_true != negated))
}

/// Whether a statement's range dereferences `X->F` at its top level: not in
/// a call's arguments (a macro may drop them), not under `sizeof`, and not in
/// a statement with a conditional operator (`&&`, `||`, `?`).
fn dereferences(tokens: &[Token], range: (usize, usize), base: &str, field: &str) -> bool {
    let words = texts(tokens, range);
    if words.iter().any(|w| {
        matches!(
            *w,
            "&&" | "||" | "?" | "sizeof" | "typeof" | "__typeof__" | "_Alignof"
        )
    }) {
        return false;
    }
    let calls = calls_in(tokens, range.0, range.1);
    let in_call = |index: usize| {
        calls
            .iter()
            .any(|(_, open, close)| *open < index && index < *close)
    };
    for index in range.0..range.1.saturating_sub(2) {
        if tokens[index].text == base
            && tokens[index + 1].text == "->"
            && tokens[index + 2].text == field
            && (index == range.0 || !matches!(tokens[index - 1].text.as_str(), "." | "->" | "&"))
            && !in_call(index)
        {
            let after = tokens.get(index + 3).map_or("", |t| t.text.as_str());
            let star_before = index > range.0 && tokens[index - 1].text == "*";
            if matches!(after, "->" | "[") || star_before {
                return true;
            }
        }
    }
    false
}

fn walk_null_tests(list: &[Stmt], preceding: &mut Vec<(usize, usize)>, found: &mut Vec<Preceded>) {
    for stmt in list {
        match stmt {
            Stmt::Expr(s, e) => preceding.push((*s, *e)),
            Stmt::If { cond, then, els } => {
                found.push((*cond, preceding.clone()));
                let mut inner = preceding.clone();
                inner.push(*cond);
                walk_null_tests(std::slice::from_ref(then), &mut inner.clone(), found);
                if let Some(els) = els {
                    walk_null_tests(std::slice::from_ref(els), &mut inner, found);
                }
                preceding.push(*cond);
            }
            Stmt::Block(inner) => {
                let mut nested = preceding.clone();
                walk_null_tests(inner, &mut nested, found);
            }
            Stmt::Loop { init, body, .. } => {
                if let Some(init) = init {
                    preceding.push(*init);
                }
                let mut nested = preceding.clone();
                walk_null_tests(std::slice::from_ref(body), &mut nested, found);
            }
            Stmt::Maybe(inner) => {
                let mut nested = preceding.clone();
                walk_null_tests(std::slice::from_ref(inner), &mut nested, found);
            }
            Stmt::Return(..) | Stmt::Break | Stmt::Continue | Stmt::Empty => {}
        }
    }
}

/// What a function does to `X` and `X->F` that defeats the derivation.
fn local_defeat(tokens: &[Token], body: (usize, usize), base: &str, field: &str) -> Option<String> {
    for index in body.0 + 1..body.1 {
        if tokens[index].text != base {
            continue;
        }
        let before = tokens[index - 1].text.as_str();
        if matches!(before, "." | "->") {
            continue;
        }
        let next = tokens.get(index + 1).map_or("", |t| t.text.as_str());
        let line = tokens[index].line;
        if before == "&" {
            return Some(format!("the address of `{base}` is taken (line {line})"));
        }
        if ASSIGNMENTS.contains(&next)
            || matches!(next, "++" | "--")
            || matches!(before, "++" | "--")
        {
            return Some(format!("`{base}` is assigned (line {line})"));
        }
        if next == "->" && tokens.get(index + 2).is_some_and(|t| t.text == field) {
            let after = tokens.get(index + 3).map_or("", |t| t.text.as_str());
            if ASSIGNMENTS.contains(&after) || matches!(after, "++" | "--") {
                return Some(format!("`{base}->{field}` is assigned (line {line})"));
            }
        }
    }
    None
}

/// Whether a statement mentions `name` as an identifier.
fn mentions(tokens: &[Token], range: (usize, usize), name: &str) -> bool {
    (range.0..range.1).any(|i| {
        tokens[i].text == name && (i == 0 || !matches!(tokens[i - 1].text.as_str(), "." | "->"))
    })
}

fn stmt_mentions(tokens: &[Token], stmt: &Stmt, name: &str) -> bool {
    let mut found = false;
    super::visit_exprs(stmt, &mut |range| {
        if mentions(tokens, range, name) {
            found = true;
        }
    });
    found
}

/// Whether every path through a statement ends in `return`.
fn always_returns(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Return(..) => true,
        Stmt::Block(list) => list.last().is_some_and(always_returns),
        Stmt::If {
            then,
            els: Some(els),
            ..
        } => always_returns(then) && always_returns(els),
        _ => false,
    }
}

/// Whether a condition only tests `name` for null (`name`, `!name`,
/// `name == NULL`, `name != NULL`), receiving it nowhere.
fn bare_null_test(tokens: &[Token], cond: (usize, usize), name: &str, tables: &SyncTables) -> bool {
    let (_, inner) = super::condition_atom(tokens, cond, tables);
    matches!(
        texts(tokens, inner).as_slice(),
        [n] | [n, "==" | "!=", "NULL" | "0"] | ["NULL" | "0", "==" | "!=", n] if *n == name
    )
}

/// Classify a store `B->F = …` at token `store` of `definition`: a
/// pre-publication store is a statement of the function's top-level block,
/// every earlier top-level statement on a path to it mentions `B` only as
/// its allocation from a table allocator or a bare null test, and a branch
/// that mentions `B` otherwise always returns.
fn pre_publication(
    scan: &FileScan,
    definition: &Definition,
    base: &str,
    store: usize,
    tables: &SyncTables,
) -> Result<(), String> {
    let tokens = &scan.tokens;
    let tree =
        parse_body(scan, definition).map_err(|why| format!("the body is not read: {why}"))?;
    let mut allocated = false;
    for stmt in &tree {
        let contains = |stmt: &Stmt| {
            let mut hit = false;
            super::visit_exprs(stmt, &mut |(s, e)| {
                if s <= store && store < e {
                    hit = true;
                }
            });
            hit
        };
        if contains(stmt) {
            return match stmt {
                Stmt::Expr(..) if allocated => Ok(()),
                Stmt::Expr(..) => Err(format!("`{base}` holds no allocator's result before it")),
                _ => Err("the store is not a statement of the function's top level".to_owned()),
            };
        }
        match stmt {
            Stmt::Expr(s, e) => {
                if !mentions(tokens, (*s, *e), base) {
                    continue;
                }
                let words = texts(tokens, (*s, *e));
                // A declaration of `B` without an initialiser, or with a null
                // one, publishes nothing.
                if let Some(position) = words.iter().position(|w| *w == base)
                    && declaration_type(&words[..position], &BTreeMap::new()).is_some()
                    && matches!(&words[position + 1..], [] | ["=", "NULL" | "0"])
                {
                    continue;
                }
                let allocation = matches!(words.as_slice(), [b, "=", call, "(", ..]
                    if *b == base && tables.allocators.contains(call))
                    && calls_in(tokens, *s, *e).iter().all(|(n, open, close)| {
                        !mentions(tokens, (*open, *close), base) || tokens[*n].text == "sizeof"
                    });
                if allocation {
                    allocated = true;
                } else {
                    return Err(format!(
                        "`{base}` is used before the store ({})",
                        site(scan, *s)
                    ));
                }
            }
            Stmt::If { cond, then, els } => {
                if mentions(tokens, *cond, base) && !bare_null_test(tokens, *cond, base, tables) {
                    return Err(format!(
                        "`{base}` is used in a condition before the store ({})",
                        site(scan, cond.0)
                    ));
                }
                for arm in std::iter::once(then.as_ref()).chain(els.as_deref()) {
                    if stmt_mentions(tokens, arm, base) && !always_returns(arm) {
                        return Err(format!(
                            "`{base}` is used on a path to the store ({})",
                            site(scan, cond.0)
                        ));
                    }
                }
            }
            Stmt::Return(..) | Stmt::Empty | Stmt::Break | Stmt::Continue => {}
            other => {
                if stmt_mentions(tokens, other, base) {
                    return Err(format!(
                        "`{base}` is used in a nested statement before the store"
                    ));
                }
            }
        }
    }
    Err("the store is not reached on the function's top level".to_owned())
}

/// Every token of the file set that may write a field or a whole object,
/// indexed once: `(definition, token index)` per field name for `B->F`/`B.F`
/// followed by an assignment or `++`/`--` or preceded by `&`, and every
/// `memset`/`memcpy`/`memmove` and `*B =` statement.
#[derive(Default)]
pub(super) struct WriteIndex {
    fields: BTreeMap<String, Vec<(usize, usize)>>,
    whole: Vec<(usize, usize)>,
}

pub(super) fn write_index(scans: &[FileScan], definitions: &[Definition]) -> WriteIndex {
    let mut index = WriteIndex::default();
    for (id, definition) in definitions.iter().enumerate() {
        let tokens = &scans[definition.file].tokens;
        let (open, close) = definition.body;
        for at in open + 1..close {
            let text = tokens[at].text.as_str();
            if at >= 2 && matches!(tokens[at - 1].text.as_str(), "->" | ".") && is_identifier(text)
            {
                let after = tokens.get(at + 1).map_or("", |t| t.text.as_str());
                if ASSIGNMENTS.contains(&after)
                    || matches!(after, "++" | "--")
                    || field_address_taken(tokens, at)
                {
                    index
                        .fields
                        .entry(text.to_owned())
                        .or_default()
                        .push((id, at));
                }
            }
            let whole_call = matches!(text, "memset" | "memcpy" | "memmove")
                && tokens.get(at + 1).is_some_and(|t| t.text == "(");
            let whole_store = text == "*"
                && matches!(tokens[at - 1].text.as_str(), ";" | "{" | "}")
                && tokens.get(at + 2).is_some_and(|t| t.text == "=");
            if whole_call || whole_store {
                index.whole.push((id, at));
            }
        }
    }
    index
}

/// Whether the field at token `at` (`B->F`) has its own address taken:
/// `&B->F` or `&(B->F)` with nothing after `F` that continues the operand
/// (`&B->F->g` takes the address of `g`, not of `B->F`).
fn field_address_taken(tokens: &[Token], at: usize) -> bool {
    let text = |i: usize| tokens.get(i).map_or("", |t| t.text.as_str());
    let after = text(at + 1);
    let base = at.wrapping_sub(2);
    (text(base.wrapping_sub(1)) == "&" && !matches!(after, "->" | "." | "["))
        || (text(base.wrapping_sub(1)) == "(" && text(base.wrapping_sub(2)) == "&" && after == ")")
}

/// Every write of field `F` into an object of type `ty` in the file set.
fn field_writes(
    scans: &[FileScan],
    definitions: &[Definition],
    types: &Types,
    index: &WriteIndex,
    ty: &str,
    field: &str,
    tables: &SyncTables,
) -> FieldWrites {
    let mut stores = Vec::new();
    let mut unresolved = Vec::new();
    for (id, at) in index.fields.get(field).into_iter().flatten() {
        let definition = &definitions[*id];
        let scan = &scans[definition.file];
        let tokens = &scan.tokens;
        let after = tokens.get(at + 1).map_or("", |t| t.text.as_str());
        let base_index = at - 2;
        let base = tokens[base_index].text.as_str();
        let before_base = tokens
            .get(base_index.wrapping_sub(1))
            .map_or("", |t| t.text.as_str());
        let address = field_address_taken(tokens, *at);
        let store = ASSIGNMENTS.contains(&after) || matches!(after, "++" | "--");
        // A designated initialiser (`{ .F = …`, `, .F = …`) starts a new
        // object.
        if tokens[at - 1].text == "." && matches!(base, "{" | ",") {
            continue;
        }
        let simple = is_identifier(base) && !matches!(before_base, "." | "->" | "]" | ")" | "*");
        let here = site(scan, *at);
        if !simple {
            unresolved.push(format!(
                "{here}: a write of `{field}` through a base the reader does not read"
            ));
            continue;
        }
        match declared_type(scan, definition, base, types) {
            Some((found, _)) if found != ty => {}
            Some(_) if address && !store => {
                unresolved.push(format!("{here}: the address of `{base}->{field}` is taken"));
            }
            Some(_) => stores.push((here, pre_publication(scan, definition, base, *at, tables))),
            None => unresolved.push(format!(
                "{here}: a store of `{field}` through `{base}`, whose type is not read"
            )),
        }
    }
    for (id, at) in &index.whole {
        let definition = &definitions[*id];
        let scan = &scans[definition.file];
        let tokens = &scan.tokens;
        let text = tokens[*at].text.as_str();
        let name = if text == "*" {
            tokens.get(at + 1).map(|t| t.text.clone())
        } else {
            matching(tokens, at + 1, "(", ")")
                .and_then(|end| arguments(tokens, at + 1, end).first().copied())
                .and_then(|first| match texts(tokens, first).as_slice() {
                    [name] if is_identifier(name) => Some((*name).to_owned()),
                    _ => None,
                })
        };
        if let Some(name) = name
            && declared_type(scan, definition, &name, types).is_some_and(|(found, _)| found == ty)
        {
            unresolved.push(format!(
                "{}: `{text}` writes a whole `{ty}` through `{name}`",
                site(scan, *at)
            ));
        }
    }
    (stores, unresolved)
}

/// Read every null test of a field in the given (read) function trees.
pub(super) fn null_tests(
    scans: &[FileScan],
    definitions: &[Definition],
    trees: &BTreeMap<usize, Result<Vec<Stmt>, String>>,
    types: &Types,
    tables: &SyncTables,
) -> Vec<NullTestReading> {
    let mut out = Vec::new();
    let mut index: Option<WriteIndex> = None;
    let mut writes_cache: BTreeMap<(String, String), FieldWrites> = BTreeMap::new();
    for (id, tree) in trees {
        let Ok(tree) = tree else { continue };
        let definition = &definitions[*id];
        let scan = &scans[definition.file];
        let tokens = &scan.tokens;
        let mut found = Vec::new();
        walk_null_tests(tree, &mut Vec::new(), &mut found);
        for (cond, preceding) in found {
            let Some((base, field, null_when_true)) = null_test(tokens, cond, tables) else {
                continue;
            };
            let dominating_dereference = preceding
                .iter()
                .find(|range| dereferences(tokens, **range, &base, &field))
                .map(|range| site(scan, range.0));
            let local_defeat = local_defeat(tokens, definition.body, &base, &field);
            let base_type = declared_type(scan, definition, &base, types).map(|(ty, _)| ty);
            // The program-wide store evidence is read only where the local
            // part of the derivation holds: without it the arm stays anyway.
            let (stores, unresolved) = match &base_type {
                Some(ty) if dominating_dereference.is_some() && local_defeat.is_none() => {
                    let index = index.get_or_insert_with(|| write_index(scans, definitions));
                    writes_cache
                        .entry((ty.clone(), field.clone()))
                        .or_insert_with(|| {
                            field_writes(scans, definitions, types, index, ty, &field, tables)
                        })
                        .clone()
                }
                _ => (Vec::new(), Vec::new()),
            };
            out.push(NullTestReading {
                file: scan.path.clone(),
                line: tokens[cond.0].line,
                function: definition.census.clone(),
                base,
                field,
                null_when_true,
                dominating_dereference,
                local_defeat,
                base_type,
                stores,
                unresolved,
                excluded: Err("not decided".to_owned()),
                cond_start: cond.0,
                file_index: definition.file,
            });
        }
    }
    out
}

// -------------------------------------------------- thread identifiers

/// The entries a thread-identifier argument may name, read in the function
/// that passes it (as the path reader reads a join).
fn identity(
    scan: &FileScan,
    definition: &Definition,
    argument: (usize, usize),
    tables: &SyncTables,
    thread_fields: &BTreeMap<String, BTreeSet<String>>,
    types: &BTreeMap<String, String>,
) -> Result<BTreeSet<String>, String> {
    let tokens = &scan.tokens;
    let words = texts(tokens, argument);
    let field_set = |base: Option<&str>, field: &str| -> Result<BTreeSet<String>, String> {
        base.and_then(|b| types.get(b))
            .and_then(|ty| thread_fields.get(&format!("{ty}.{field}")))
            .or_else(|| thread_fields.get(field))
            .cloned()
            .ok_or_else(|| format!("no spawn stores an identifier in field `{field}`"))
    };
    match words.as_slice() {
        [base, "->" | ".", field] => field_set(Some(base), field),
        [.., "->" | ".", field] if !words.contains(&"(") => field_set(None, field),
        ["pthread_self", "(", ")"] => Err("the caller's own identifier".to_owned()),
        [local] if is_identifier(local) => {
            // Every write of the local in the function.
            let (open, close) = definition.body;
            let mut set = BTreeSet::new();
            let mut writes = 0usize;
            for (name_index, call_open, call_close) in calls_in(tokens, open, close) {
                let name = tokens[name_index].text.as_str();
                let args = arguments(tokens, call_open, call_close);
                if let Some((_, entry_arg, Some(out_arg))) =
                    tables.spawns.iter().find(|(n, _, _)| *n == name)
                    && args
                        .get(*out_arg)
                        .is_some_and(|r| texts(tokens, *r) == ["&", *local])
                {
                    writes += 1;
                    match args
                        .get(*entry_arg)
                        .and_then(|r| super::root_name(tokens, *r))
                    {
                        Some(entry) => {
                            set.insert(entry);
                        }
                        None => {
                            return Err(format!(
                                "a spawn at {} names no entry",
                                site(scan, name_index)
                            ));
                        }
                    }
                }
                if name == "memcpy"
                    && let [destination, source, ..] = args.as_slice()
                    && texts(tokens, *destination) == ["&", *local]
                {
                    writes += 1;
                    match texts(tokens, *source).as_slice() {
                        [.., "->" | ".", field] => set.extend(field_set(None, field)?),
                        _ => {
                            return Err(format!(
                                "the copy at {} is not read",
                                site(scan, name_index)
                            ));
                        }
                    }
                }
            }
            for index in open + 1..close {
                if tokens[index].text == *local
                    && tokens.get(index + 1).is_some_and(|t| t.text == "=")
                    && !matches!(tokens[index - 1].text.as_str(), "." | "->")
                {
                    writes += 1;
                    let end = super::expression_end(tokens, index + 2, close);
                    let value = texts(tokens, (index + 2, end));
                    match value.as_slice() {
                        [base, "->" | ".", field] => set.extend(field_set(Some(base), field)?),
                        [.., "->" | ".", field] if !value.contains(&"(") => {
                            set.extend(field_set(None, field)?);
                        }
                        _ => {
                            return Err(format!(
                                "`{local}` is assigned a value the reader does not read ({})",
                                site(scan, index)
                            ));
                        }
                    }
                }
            }
            if writes == 0 {
                return Err(format!(
                    "`{local}` is not written in the function (a parameter or a global)"
                ));
            }
            Ok(set)
        }
        _ => Err("the identifier expression is not read".to_owned()),
    }
}

/// Every join and detach site, every `pthread_self` site and every spawn
/// wrapper's joinability, program-wide.
pub(super) fn thread_sites(
    scans: &[FileScan],
    definitions: &[Definition],
    tables: &SyncTables,
    thread_fields: &BTreeMap<String, BTreeSet<String>>,
) -> (Vec<ThreadIdSite>, Vec<SelfSite>, Vec<WrapperReading>) {
    let mut ids = Vec::new();
    let mut selves = Vec::new();
    for definition in definitions {
        let scan = &scans[definition.file];
        let tokens = &scan.tokens;
        let types = super::struct_types(tokens, definition);
        let (open, close) = definition.body;
        let calls = calls_in(tokens, open, close);
        for (name_index, call_open, call_close) in &calls {
            let name = tokens[*name_index].text.as_str();
            let call = if name == tables.join {
                "join"
            } else if name == "pthread_detach" {
                "detach"
            } else if name == "pthread_self" {
                let published = !calls.iter().any(|(outer, o, c)| {
                    matches!(
                        tokens[*outer].text.as_str(),
                        "pthread_equal" | "pthread_kill"
                    ) && arguments(tokens, *o, *c)
                        .iter()
                        .any(|(s, e)| *s == *name_index && *e == call_close + 1)
                });
                selves.push(SelfSite {
                    file: scan.path.clone(),
                    line: tokens[*name_index].line,
                    function: definition.census.clone(),
                    published,
                });
                continue;
            } else {
                continue;
            };
            let identity = arguments(tokens, *call_open, *call_close)
                .first()
                .map_or_else(
                    || Err("no argument".to_owned()),
                    |argument| identity(scan, definition, *argument, tables, thread_fields, &types),
                );
            ids.push(ThreadIdSite {
                file: scan.path.clone(),
                line: tokens[*name_index].line,
                function: definition.census.clone(),
                call,
                identity,
            });
        }
    }
    let mut wrappers = Vec::new();
    for (wrapper, _, out_arg) in tables.spawns {
        if *wrapper == "pthread_create" {
            wrappers.push(WrapperReading {
                wrapper: (*wrapper).to_owned(),
                joinable: Err("the attribute object is not read".to_owned()),
            });
            continue;
        }
        let bodies: Vec<&Definition> = definitions
            .iter()
            .filter(|d| {
                d.census == *wrapper
                    || d.census
                        .rsplit_once("::")
                        .is_some_and(|(_, n)| n == *wrapper)
            })
            .collect();
        let joinable = match bodies.as_slice() {
            [definition] => wrapper_joinable(scans, definition, *out_arg),
            [] => Err(format!("`{wrapper}` has no read body")),
            _ => Err(format!("`{wrapper}` has more than one body")),
        };
        wrappers.push(WrapperReading {
            wrapper: (*wrapper).to_owned(),
            joinable,
        });
    }
    wrappers.push(WrapperReading {
        wrapper: DEFAULT_ATTRIBUTE_CREATE.to_owned(),
        joinable: Ok("POSIX's default detach state".to_owned()),
    });
    (ids, selves, wrappers)
}

/// A wrapper creates its thread joinable when its body calls
/// `pthread_create` exactly once, writing the identifier through the
/// wrapper's own out-parameter, with a null attribute object or one whose
/// every detach-state setting is `PTHREAD_CREATE_JOINABLE`, and detaches
/// nothing.
fn wrapper_joinable(
    scans: &[FileScan],
    definition: &Definition,
    out_arg: Option<usize>,
) -> Result<String, String> {
    let scan = &scans[definition.file];
    let tokens = &scan.tokens;
    let (open, close) = definition.body;
    let words = texts(tokens, (open, close));
    if words.contains(&"PTHREAD_CREATE_DETACHED") || words.contains(&"pthread_detach") {
        return Err("the wrapper's body names a detached state or `pthread_detach`".to_owned());
    }
    let calls = calls_in(tokens, open, close);
    let creates: Vec<_> = calls
        .iter()
        .filter(|(n, _, _)| tokens[*n].text == "pthread_create")
        .collect();
    let [(create, create_open, create_close)] = creates.as_slice() else {
        return Err("the wrapper does not call `pthread_create` exactly once".to_owned());
    };
    let args = arguments(tokens, *create_open, *create_close);
    let parameter =
        out_arg.and_then(|index| definition.parameters.get(index).map(|(n, _)| n.clone()));
    match (parameter, args.first()) {
        (Some(parameter), Some(first)) if texts(tokens, *first) == [parameter.as_str()] => {}
        _ => {
            return Err(
                "the created identifier is not written through the wrapper's out-parameter"
                    .to_owned(),
            );
        }
    }
    for (name, call_open, call_close) in &calls {
        if tokens[*name].text == "pthread_attr_setdetachstate" {
            let set = arguments(tokens, *call_open, *call_close);
            if set
                .get(1)
                .is_none_or(|r| texts(tokens, *r) != ["PTHREAD_CREATE_JOINABLE"])
            {
                return Err(format!(
                    "a detach state other than `PTHREAD_CREATE_JOINABLE` is set ({})",
                    site(scan, *name)
                ));
            }
        }
    }
    Ok(format!(
        "{} (`pthread_create`, joinable)",
        site(scan, *create)
    ))
}
