//! Thread-entry census: derived thread identity for the loop-owner obligation
//! (card **W47-LOOP-OWNER-CONTRACTS**, gateway rule 10p).
//!
//! # What it reads
//!
//! A closed set of C source files — one program, for example the in_tail
//! runtime test binary's library, plugin and test sources — read lexically
//! with the gateway's own tokenizer. Every definition is found at file scope;
//! a `static` one is named `FILE::NAME`, an external one `NAME`.
//! [`census`] builds a [`ThreadProgram`] for
//! [`qcore_core::loop_owner`] and decides the owner obligation at every call
//! to a function whose contract declares one.
//!
//! # The hop kinds it follows (W47-TAIL-PAUSE-BINDINGS §2.1)
//!
//! * **Direct** calls by name.
//! * **A**: each function listed in the test table (`TEST_LIST = { {"n", f},
//!   ... }`) is a root on the test thread `T`.
//! * **S**: a function named at a spawn wrapper's entry argument
//!   ([`HopTables::spawns`]) is a new thread named after it, in the spawner's
//!   mode.
//! * **V / V\***: a function stored in a named field (`.F = f` in an
//!   initializer, `x->F = f` or `x.F = f` at run time, the revised test's
//!   copied vtable included) is dispatched at every `->F (` / `.F (` call, in
//!   the dispatcher's context. Field-keyed: a registrant is attributed every
//!   dispatcher of that field name, whatever object it was stored in.
//! * **R**: a function named at a collector registration's callback argument
//!   ([`HopTables::collector_registrations`]) is registered on
//!   [`HopTables::collector_field`].
//! * **L**: a dispatch of the collector field inside a loop dispatcher
//!   ([`HopTables::loop_dispatchers`]) runs the callback on the dispatcher's
//!   thread in that loop's mode.
//! * **F**: a call inside the then/else block of `if (P(x))` or
//!   `if (!P(x))`, `P` the threaded-flag predicate, requires that mode.
//!
//! # Fail-closed
//!
//! Every other occurrence of a defined function's name — an address passed
//! to anything but a recognised hop, stored in a variable, returned, listed
//! in a positional initializer, or written in a macro replacement list — marks
//! it unresolved, and so is every field whose value is read other than by a
//! call or a test (its registrants may be called from anywhere). A file whose
//! conditional groups this reader cannot structure marks every function it
//! defines unresolved. Unresolved reasons flow to every function reached.
//!
//! Premises stated, not proved (trust boundary 28): the tokenizer's reading
//! of the files; that no code outside the read files calls or takes the
//! address of a read function by name; that a function stored in a field is
//! called only through that field's name; and that along a derived path the
//! instance a collector operation names is the instance whose mode the path
//! fixed (P-INSTANCE: `ctx->ins` is the dispatching instance).

use std::collections::{BTreeMap, BTreeSet};

use qcore_contract::contract_loader::{DeclaredLoopOwner, LoopOwnerOperation};
use qcore_core::loop_owner::{
    Context, Derived, Edge, HopKind, Mode, ModeEffect, OwnerVerdict, Owners, ThreadProgram,
    ThreadRoot, derive, owner_verdict,
};

use crate::legacy_c::{ConditionalDirective, Token, is_identifier, tokenize_with_conditionals};

/// The closed hop vocabulary of one program family.
#[derive(Clone, Copy, Debug)]
pub struct HopTables {
    /// `(wrapper, index of the entry argument)`.
    pub spawns: &'static [(&'static str, usize)],
    /// `(registration, index of the callback argument)`.
    pub collector_registrations: &'static [(&'static str, usize)],
    /// The field a collector registration stores its callback in.
    pub collector_field: &'static str,
    /// `(dispatcher function, mode)`: a dispatch of the collector field inside
    /// that function runs on its thread in that mode.
    pub loop_dispatchers: &'static [(&'static str, Mode)],
    /// The test-table variable (hop A).
    pub test_table: &'static str,
    /// The thread name of a test-table root.
    pub test_thread: &'static str,
    /// The threaded-flag predicate (hop F).
    pub threaded_predicate: &'static str,
}

/// Fluent Bit's hops as W47-TAIL-PAUSE-BINDINGS bound them at `90978ac6`.
pub const FLUENT_BIT_HOPS: HopTables = HopTables {
    spawns: &[
        // lib/monkey/mk_core/mk_utils.c:297 (mk_worker_start.callback).
        ("mk_utils_worker_spawn", 0),
        // src/flb_worker.c:77 (flb_worker.func).
        ("flb_worker_create", 0),
        // src/flb_thread_pool.c:63 (flb_tp_thread.params.func).
        ("flb_tp_thread_create", 1),
        ("pthread_create", 2),
    ],
    collector_registrations: &[
        ("flb_input_set_collector_time", 1),
        ("flb_input_set_collector_event", 1),
        ("flb_input_set_collector_socket", 1),
        ("flb_input_set_collector_server", 1),
    ],
    collector_field: "cb_collect",
    loop_dispatchers: &[
        // src/flb_input.c: the engine loop, config->evl.
        ("flb_input_collector_fd", Mode::NonThreaded),
        // src/flb_input_thread.c: the worker loop, thi->evl.
        ("input_collector_fd", Mode::Threaded),
    ],
    test_table: "TEST_LIST",
    test_thread: "T",
    threaded_predicate: "flb_input_is_threaded",
};

/// One file of the program.
#[derive(Clone, Debug)]
pub struct CensusFile {
    pub path: String,
    pub text: String,
}

/// The obligation's answer at one call site.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnerSite {
    pub file: String,
    pub line: usize,
    /// The calling function's census name.
    pub function: String,
    pub callee: String,
    pub operation: LoopOwnerOperation,
    pub verdict: OwnerVerdict,
}

/// The census of one program.
#[derive(Clone, Debug)]
pub struct Census {
    pub program: ThreadProgram,
    pub derived: Vec<Derived>,
    pub owners: Owners,
    pub sites: Vec<OwnerSite>,
    /// Census names, index-aligned with `program.functions`.
    pub index: BTreeMap<String, usize>,
}

impl Census {
    /// The derivation of a function by census name.
    #[must_use]
    pub fn derived(&self, name: &str) -> Option<&Derived> {
        self.index.get(name).map(|id| &self.derived[*id])
    }

    /// A witness path's names.
    #[must_use]
    pub fn names(&self, path: &[usize]) -> Vec<String> {
        path.iter()
            .map(|id| self.program.functions[*id].clone())
            .collect()
    }
}

struct Definition {
    file: usize,
    name: String,
    census_name: String,
    /// The name token, the body's `{` and its `}`.
    header: usize,
    body: (usize, usize),
    /// Parameter names, which shadow a function of the same name inside the
    /// definition.
    parameters: BTreeSet<String>,
}

struct Scan {
    tokens: Vec<Token>,
    definitions: Vec<usize>,
    /// Token indices of definition names, which are not uses.
    headers: BTreeSet<usize>,
    /// Token indices of file-scope declarations' names.
    declarations: BTreeSet<usize>,
    unstructured: bool,
}

const KEYWORDS: &[&str] = &[
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
    "__builtin_expect",
    "defined",
];

fn is_callable_name(text: &str) -> bool {
    is_identifier(text) && !KEYWORDS.contains(&text)
}

/// Find file-scope definitions and declarations, reading conditional groups
/// as the raw enumeration does: every arm is scanned from the group's entry
/// depth, and a group whose arms end at different depths is unstructured.
fn scan_file(file: usize, path: &str, text: &str, definitions: &mut Vec<Definition>) -> Scan {
    let (tokens, marks) = tokenize_with_conditionals(text);
    let mut scan = Scan {
        tokens: Vec::new(),
        definitions: Vec::new(),
        headers: BTreeSet::new(),
        declarations: BTreeSet::new(),
        unstructured: false,
    };
    let mut depth = 0usize;
    let mut frames: Vec<(usize, Vec<usize>, bool)> = Vec::new();
    let mut next_mark = 0;
    let mut open: Option<(usize, usize)> = None; // (name token, body open)
    let mut linkage: Vec<usize> = Vec::new();
    let mut statement_start = 0usize;
    let mut index = 0;
    while index < tokens.len() {
        while next_mark < marks.len() && marks[next_mark].token_index() <= index {
            match marks[next_mark].directive() {
                ConditionalDirective::Open => frames.push((depth, Vec::new(), false)),
                ConditionalDirective::Alternative | ConditionalDirective::Otherwise => {
                    if let Some(frame) = frames.last_mut() {
                        frame.1.push(depth);
                        frame.2 |= marks[next_mark].directive() == ConditionalDirective::Otherwise;
                        depth = frame.0;
                    } else {
                        scan.unstructured = true;
                    }
                }
                ConditionalDirective::Close => {
                    if let Some((entry, mut arms, saw_else)) = frames.pop() {
                        arms.push(depth);
                        if !saw_else {
                            arms.push(entry);
                        }
                        if arms.iter().any(|arm| *arm != arms[0]) {
                            scan.unstructured = true;
                        }
                        depth = arms.into_iter().max().unwrap_or(entry);
                    } else {
                        scan.unstructured = true;
                    }
                }
            }
            next_mark += 1;
        }
        let text = tokens[index].text.as_str();
        match text {
            "{" if depth == 0
                && index >= 2
                && tokens[index - 2].text == "extern"
                && tokens[index - 1].text.starts_with("__qcore_literal") =>
            {
                // An `extern "C" {` linkage block is not a scope.
                linkage.push(depth);
            }
            "}" if depth == 0 && linkage.last() == Some(&depth) => {
                linkage.pop();
                statement_start = index + 1;
            }
            "{" => {
                if depth == 0
                    && let Some((name, _)) = open.take().filter(|(_, body)| *body == index)
                {
                    open = Some((name, index));
                } else if depth == 0 {
                    open = None;
                }
                depth += 1;
            }
            "}" => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    if let Some((name, body)) = open.take()
                        && tokens[body].text == "{"
                    {
                        let is_static = tokens[statement_start..name]
                            .iter()
                            .any(|token| token.text == "static");
                        let name_text = tokens[name].text.clone();
                        let census_name = if is_static {
                            format!("{path}::{name_text}")
                        } else {
                            name_text.clone()
                        };
                        scan.definitions.push(definitions.len());
                        scan.headers.insert(name);
                        definitions.push(Definition {
                            file,
                            name: name_text,
                            census_name,
                            header: name,
                            body: (body, index),
                            parameters: parameter_names(&tokens, name + 1, body),
                        });
                    }
                    statement_start = index + 1;
                }
            }
            ";" if depth == 0 => {
                open = None;
                statement_start = index + 1;
            }
            _ if depth == 0
                && open.is_none()
                && is_callable_name(text)
                && tokens.get(index + 1).is_some_and(|next| next.text == "(") =>
            {
                if let Some(close) = crate::legacy_c::matching(&tokens, index + 1, "(", ")") {
                    // Skip attribute lists and K&R-free trailers up to `{` or `;`.
                    let after = close + 1;
                    match tokens.get(after).map(|token| token.text.as_str()) {
                        Some("{") => open = Some((index, after)),
                        _ => {
                            scan.declarations.insert(index);
                        }
                    }
                }
            }
            _ => {}
        }
        index += 1;
    }
    if !frames.is_empty() || depth != 0 {
        scan.unstructured = true;
    }
    scan.tokens = tokens;
    scan
}

/// The names a parameter list declares: an identifier directly before `,`,
/// `)` or `[` at the list's own depth, or a function pointer's `(*NAME)`.
fn parameter_names(tokens: &[Token], open: usize, end: usize) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut depth = 0usize;
    for index in open..end {
        match tokens[index].text.as_str() {
            "(" => depth += 1,
            ")" => depth = depth.saturating_sub(1),
            text if is_identifier(text) => {
                let next = tokens.get(index + 1).map_or("", |t| t.text.as_str());
                let previous = tokens[index - 1].text.as_str();
                if (depth == 1 && matches!(next, "," | ")" | "["))
                    || (previous == "*" && next == ")")
                {
                    names.insert(text.to_owned());
                }
            }
            _ => {}
        }
    }
    names
}

/// The names written in `#define` replacement lists (the tokenizer skips
/// directives, so they are read here from the raw text).
fn macro_names(text: &str) -> BTreeMap<String, usize> {
    let mut names = BTreeMap::new();
    let mut logical = String::new();
    let mut start_line = 0;
    for (number, line) in text.lines().enumerate() {
        if logical.is_empty() {
            let trimmed = line.trim_start();
            if !trimmed.starts_with('#') || !trimmed[1..].trim_start().starts_with("define") {
                continue;
            }
            start_line = number + 1;
        }
        if let Some(stripped) = line.strip_suffix('\\') {
            logical.push_str(stripped);
            logical.push(' ');
            continue;
        }
        logical.push_str(line);
        let body = logical.trim_start().trim_start_matches('#').trim_start();
        let body = body.strip_prefix("define").unwrap_or(body);
        let mut word = String::new();
        let mut first = true;
        for character in body.chars().chain(std::iter::once(' ')) {
            if character.is_ascii_alphanumeric() || character == '_' {
                word.push(character);
            } else {
                if !word.is_empty() {
                    // The first word is the macro's own name.
                    if !first {
                        names.entry(word.clone()).or_insert(start_line);
                    }
                    first = false;
                    word.clear();
                }
                if first && character == '(' {
                    first = false;
                }
            }
        }
        logical.clear();
    }
    names
}

fn arguments(tokens: &[Token], open: usize) -> Option<(Vec<(usize, usize)>, usize)> {
    let close = crate::legacy_c::matching(tokens, open, "(", ")")?;
    let mut ranges = Vec::new();
    let mut depth = 0usize;
    let mut start = open + 1;
    for index in open + 1..close {
        match tokens[index].text.as_str() {
            "(" | "[" | "{" => depth += 1,
            ")" | "]" | "}" => depth = depth.saturating_sub(1),
            "," if depth == 0 => {
                ranges.push((start, index));
                start = index + 1;
            }
            _ => {}
        }
    }
    if start < close || !ranges.is_empty() {
        ranges.push((start, close));
    }
    Some((ranges, close))
}

/// The end (inclusive) of the statement or block starting at `start`.
fn statement_end(tokens: &[Token], start: usize, limit: usize) -> usize {
    if tokens.get(start).is_some_and(|token| token.text == "{") {
        return crate::legacy_c::matching(tokens, start, "{", "}").unwrap_or(limit);
    }
    let mut depth = 0usize;
    for index in start..limit {
        match tokens[index].text.as_str() {
            "(" | "[" | "{" => depth += 1,
            ")" | "]" | "}" => depth = depth.saturating_sub(1),
            ";" if depth == 0 => return index,
            _ => {}
        }
    }
    limit
}

/// Token ranges of a body in which the threaded flag is fixed (hop F).
fn mode_ranges(
    tokens: &[Token],
    body: (usize, usize),
    predicate: &str,
) -> Vec<(usize, usize, Mode)> {
    let mut ranges = Vec::new();
    for index in body.0..body.1 {
        if tokens[index].text != "if" || tokens.get(index + 1).is_none_or(|t| t.text != "(") {
            continue;
        }
        let Some(close) = crate::legacy_c::matching(tokens, index + 1, "(", ")") else {
            continue;
        };
        let condition = tokens[index + 2..close]
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        let (negated, rest) = match condition.as_slice() {
            ["!", rest @ ..] => (true, rest),
            rest => (false, rest),
        };
        let fixed = match rest {
            [name, "(", argument, ")"] if *name == predicate && is_identifier(argument) => {
                if negated {
                    Mode::NonThreaded
                } else {
                    Mode::Threaded
                }
            }
            _ => continue,
        };
        let then_end = statement_end(tokens, close + 1, body.1);
        ranges.push((close + 1, then_end, fixed));
        if tokens.get(then_end + 1).is_some_and(|t| t.text == "else") {
            let else_end = statement_end(tokens, then_end + 2, body.1);
            let other = if fixed == Mode::Threaded {
                Mode::NonThreaded
            } else {
                Mode::Threaded
            };
            ranges.push((then_end + 2, else_end, other));
        }
    }
    ranges
}

/// Build the program, derive it and decide every declared obligation.
#[must_use]
pub fn census(
    files: &[CensusFile],
    obligations: &BTreeMap<String, DeclaredLoopOwner>,
    hops: &HopTables,
) -> Census {
    let mut definitions = Vec::new();
    let scans = files
        .iter()
        .enumerate()
        .map(|(file, source)| scan_file(file, &source.path, &source.text, &mut definitions))
        .collect::<Vec<_>>();
    let mut program = ThreadProgram::default();
    let mut index = BTreeMap::new();
    for definition in &definitions {
        if !index.contains_key(&definition.census_name) {
            index.insert(definition.census_name.clone(), program.functions.len());
            program.functions.push(definition.census_name.clone());
        }
    }
    let plain_name = |id: usize| {
        let label = &program.functions[id];
        label
            .rsplit_once("::")
            .map_or(label.as_str(), |(_, name)| name)
            .to_owned()
    };
    // Name resolution: a static of the same file, else every static header
    // definition of the name, else the external definition.
    let mut by_name: BTreeMap<&str, Vec<&Definition>> = BTreeMap::new();
    for definition in &definitions {
        by_name
            .entry(&definition.name)
            .or_default()
            .push(definition);
    }
    let resolve = |file: usize, name: &str| -> Vec<usize> {
        let Some(candidates) = by_name.get(name) else {
            return Vec::new();
        };
        let own = candidates
            .iter()
            .filter(|d| d.file == file && d.census_name.contains("::"))
            .map(|d| index[&d.census_name])
            .collect::<Vec<_>>();
        if !own.is_empty() {
            return own;
        }
        let mut found = candidates
            .iter()
            .filter(|d| !d.census_name.contains("::") || files[d.file].path.ends_with(".h"))
            .map(|d| index[&d.census_name])
            .collect::<Vec<_>>();
        found.sort_unstable();
        found.dedup();
        found
    };
    let mut unresolved: BTreeMap<usize, BTreeSet<String>> = BTreeMap::new();
    let mut escape = |ids: &[usize], reason: String| {
        for id in ids {
            unresolved.entry(*id).or_default().insert(reason.clone());
        }
    };
    let mut registrants: BTreeMap<String, BTreeSet<usize>> = BTreeMap::new();
    let mut dispatches: Vec<(usize, String, ModeEffect)> = Vec::new();
    let mut poisoned_fields: BTreeMap<String, String> = BTreeMap::new();
    let mut edges = BTreeSet::new();
    let mut spawns = BTreeSet::new();
    let mut roots = BTreeSet::new();
    let mut calls: Vec<(usize, usize, String, usize)> = Vec::new(); // (file, caller id, callee name, line)

    for (file, scan) in scans.iter().enumerate() {
        let path = &files[file].path;
        let tokens = &scan.tokens;
        if scan.unstructured {
            for definition in &scan.definitions {
                let id = index[&definitions[*definition].census_name];
                escape(
                    &[id],
                    format!("{path}: a conditional group this reader cannot structure"),
                );
            }
        }
        for (name, line) in macro_names(&files[file].text) {
            let ids = resolve(file, &name);
            if !ids.is_empty() {
                escape(&ids, format!("named in a macro at {path}:{line}"));
            }
        }
        // Which definition each token belongs to.
        let mut owner_of = vec![None; tokens.len()];
        let mut shadow_of: Vec<Option<&BTreeSet<String>>> = vec![None; tokens.len()];
        let mut modes: Vec<(usize, usize, Mode)> = Vec::new();
        for definition in &scan.definitions {
            let definition = &definitions[*definition];
            let id = index[&definition.census_name];
            for slot in &mut owner_of[definition.body.0..=definition.body.1] {
                *slot = Some(id);
            }
            for slot in &mut shadow_of[definition.header + 1..=definition.body.1] {
                *slot = Some(&definition.parameters);
            }
            modes.extend(mode_ranges(
                tokens,
                definition.body,
                hops.threaded_predicate,
            ));
        }
        let mode_at = |position: usize| {
            modes
                .iter()
                .filter(|(start, end, _)| *start <= position && position <= *end)
                .map(|(_, _, mode)| *mode)
                .collect::<BTreeSet<_>>()
        };
        let mut consumed = BTreeSet::new();
        // The test table (hop A).
        for position in 0..tokens.len() {
            if tokens[position].text != hops.test_table
                || tokens.get(position + 1).is_none_or(|t| t.text != "=")
                || tokens.get(position + 2).is_none_or(|t| t.text != "{")
            {
                continue;
            }
            let Some(end) = crate::legacy_c::matching(tokens, position + 2, "{", "}") else {
                continue;
            };
            for entry in position + 3..end {
                // `{ "name", f }`
                if tokens[entry].text == "{"
                    && tokens.get(entry + 2).is_some_and(|t| t.text == ",")
                    && tokens.get(entry + 4).is_some_and(|t| t.text == "}")
                {
                    for id in resolve(file, &tokens[entry + 3].text) {
                        roots.insert((id, hops.test_thread.to_owned()));
                        consumed.insert(entry + 3);
                    }
                }
            }
        }
        // The tokenizer emits every punctuator byte on its own: `->` is `-`
        // `>`, `==` is `=` `=`, `&&` is `&` `&`.
        let at = |position: usize| tokens.get(position).map_or("", |t| t.text.as_str());
        for position in 0..tokens.len() {
            let text = tokens[position].text.as_str();
            let previous = position.checked_sub(1).map_or("", at);
            let before_previous = position.checked_sub(2).map_or("", at);
            let member = previous == "." || (previous == ">" && before_previous == "-");
            // A field read, store or dispatch.
            if member && is_identifier(text) {
                let next = tokens.get(position + 1).map(|t| t.text.as_str());
                let after_next = at(position + 2);
                let next = match (next, after_next) {
                    (Some("=" | "!"), "=") => Some("=="),
                    (Some("&"), "&") | (Some("|"), "|") => Some("&&"),
                    (Some("-"), ">") => Some("->"),
                    (Some("+"), "+") | (Some("-"), "-") => Some("++"),
                    (other, _) => other,
                };
                match next {
                    Some("(") => {
                        if let Some(caller) = owner_of[position] {
                            let caller_name = plain_name(caller);
                            let effect = hops
                                .loop_dispatchers
                                .iter()
                                .find(|(dispatcher, _)| {
                                    *dispatcher == caller_name && text == hops.collector_field
                                })
                                .map_or(ModeEffect::Keep, |(_, mode)| ModeEffect::Dispatch(*mode));
                            dispatches.push((caller, text.to_owned(), effect));
                        }
                    }
                    Some("=") => {
                        let value = tokens.get(position + 2).map(|t| t.text.as_str());
                        let end = tokens.get(position + 3).map(|t| t.text.as_str());
                        if let Some(value) = value
                            && matches!(end, Some(";" | "," | "}"))
                        {
                            let ids = resolve(file, value);
                            if !ids.is_empty() {
                                registrants.entry(text.to_owned()).or_default().extend(ids);
                                consumed.insert(position + 2);
                            }
                        }
                    }
                    Some("==" | "&&" | "?" | "[" | "." | "->" | "++") | None => {}
                    Some(")") if closes_a_test(tokens, position + 1) => {}
                    Some(_) => {
                        // The field's value is read into something other than
                        // a call or a test: its registrants may be called from
                        // anywhere that value reaches.
                        let line = tokens[position].line;
                        poisoned_fields
                            .entry(text.to_owned())
                            .or_insert_with(|| format!("{path}:{line}"));
                    }
                }
                continue;
            }
            if !is_callable_name(text) || consumed.contains(&position) {
                continue;
            }
            // A parameter of the same name shadows the function.
            if shadow_of[position].is_some_and(|names| names.contains(text)) {
                continue;
            }
            if scan.headers.contains(&position) || scan.declarations.contains(&position) {
                continue;
            }
            let next = tokens.get(position + 1).map(|t| t.text.as_str());
            let address_of = previous == "&" && before_previous != "&";
            if next == Some("(") && !address_of {
                let Some(caller) = owner_of[position] else {
                    continue;
                };
                let Some((ranges, _)) = arguments(tokens, position + 1) else {
                    continue;
                };
                let line = tokens[position].line;
                // Spawns (hop S) and collector registrations (hop R).
                let spawn = hops.spawns.iter().find(|(name, _)| *name == text);
                let registration = hops
                    .collector_registrations
                    .iter()
                    .find(|(name, _)| *name == text);
                for (table_hit, is_spawn) in [(spawn, true), (registration, false)] {
                    let Some((_, argument)) = table_hit else {
                        continue;
                    };
                    if let Some((start, end)) = ranges.get(*argument)
                        && end - start == 1
                    {
                        let ids = resolve(file, &tokens[*start].text);
                        if !ids.is_empty() {
                            consumed.insert(*start);
                            for id in ids {
                                if is_spawn {
                                    spawns.insert((caller, id));
                                } else {
                                    registrants
                                        .entry(hops.collector_field.to_owned())
                                        .or_default()
                                        .insert(id);
                                }
                            }
                        }
                    }
                }
                let fixed = mode_at(position);
                let effect = match fixed.len() {
                    0 => ModeEffect::Keep,
                    1 => ModeEffect::Require(*fixed.iter().next().unwrap_or(&Mode::Either)),
                    // Nested contradictory branches: this call is unreachable.
                    _ => continue,
                };
                for callee in resolve(file, text) {
                    edges.insert(Edge {
                        caller,
                        callee,
                        hop: HopKind::Direct,
                        effect,
                    });
                }
                calls.push((file, caller, text.to_owned(), line));
                // Arguments are scanned in their own positions.
                continue;
            }
            // Any other occurrence of a defined function's name escapes.
            let ids = resolve(file, text);
            if !ids.is_empty() {
                let line = tokens[position].line;
                escape(&ids, format!("address of `{text}` taken at {path}:{line}"));
            }
        }
    }
    // Field dispatch edges (hops V, V*, R, L).
    for (caller, field, effect) in &dispatches {
        let hop = if field == hops.collector_field {
            HopKind::CollectorRegistry
        } else {
            HopKind::Vtable
        };
        for callee in registrants.get(field).into_iter().flatten() {
            edges.insert(Edge {
                caller: *caller,
                callee: *callee,
                hop,
                effect: *effect,
            });
        }
    }
    for (field, at) in &poisoned_fields {
        let ids = registrants
            .get(field)
            .into_iter()
            .flatten()
            .copied()
            .collect::<Vec<_>>();
        escape(&ids, format!("field `{field}` read as a value at {at}"));
    }
    program.edges = edges.into_iter().collect();
    program.spawns = spawns.into_iter().collect();
    program.roots = roots
        .into_iter()
        .map(|(function, thread)| ThreadRoot {
            function,
            context: Context {
                thread,
                mode: Mode::Either,
            },
        })
        .collect();
    program.unresolved = unresolved;
    // A spawned thread is named by its entry's census name.
    let derived = derive(&program);
    let dispatcher = |name: &str| {
        let ids = by_name
            .get(name)
            .map(|found| {
                found
                    .iter()
                    .map(|d| index[&d.census_name])
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        (ids.len() == 1).then(|| &derived[ids[0]])
    };
    let engine = hops
        .loop_dispatchers
        .iter()
        .find(|(_, mode)| *mode == Mode::NonThreaded)
        .and_then(|(name, _)| dispatcher(name));
    let worker = hops
        .loop_dispatchers
        .iter()
        .find(|(_, mode)| *mode == Mode::Threaded)
        .and_then(|(name, _)| dispatcher(name));
    let owners = Owners::from_dispatchers(engine, worker);
    let mut sites = Vec::new();
    for (file, caller, callee, line) in calls {
        let Some(obligation) = obligations.get(&callee) else {
            continue;
        };
        sites.push(OwnerSite {
            file: files[file].path.clone(),
            line,
            function: program.functions[caller].clone(),
            callee,
            operation: obligation.operation,
            verdict: owner_verdict(&derived[caller], &owners),
        });
    }
    Census {
        program,
        derived,
        owners,
        sites,
        index,
    }
}

/// Whether the `)` at `close` ends a condition (`if (…)`, `while (…)`, a
/// parenthesised test) rather than a call's argument list.
fn closes_a_test(tokens: &[Token], close: usize) -> bool {
    let mut depth = 0usize;
    for index in (0..=close).rev() {
        match tokens[index].text.as_str() {
            ")" => depth += 1,
            "(" => {
                depth -= 1;
                if depth == 0 {
                    let before = index
                        .checked_sub(1)
                        .map(|p| tokens[p].text.as_str())
                        .unwrap_or("");
                    return matches!(before, "if" | "while" | "(" | "!" | "&&" | "||" | "return")
                        || !is_identifier(before);
                }
            }
            _ => {}
        }
    }
    false
}
