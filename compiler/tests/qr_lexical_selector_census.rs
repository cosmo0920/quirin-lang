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

//! Card `QR-LEXICAL-SELECTORS-DECLARATION-NAMES`: the declaration-name census
//! of every route selector, extending `qr_print_route_census.rs` (card
//! `QR-PRINT-ROUTE-MATCHES-DECLARATION`) from the one print-call selector to
//! all of them.
//!
//! **What is censused.** Every word a route selector or a capability gate
//! reads (the list below is read off the selectors' code, family by family),
//! in every module mode (`module contract;` in a `.qv`, `module library;` and
//! an executable in a `.qr`), on two bases (no source call at all, and a
//! source call `has_scalar_calls` selects), in every declaration position a
//! word can take: an `extern fn` name with and without an inline contract, a
//! source function name, a record name, an error name, a parameter name that
//! a contract clause names, and a type constructor in a parameter type; plus
//! the one declaration row the scalar route's `throws<ArithmeticError>` test
//! can read. Each synthetic module has a **control**: the same text with the
//! word replaced by `flb_noop`, a name no selector reads.
//!
//! **What is recorded.** For each module: whether it parses, the value of
//! every selector the driver consults in its route order and of the import
//! gate (`check_source_imports`), and the compile outcome (mode, profile and
//! checked variant, or every formatted diagnostic). A row whose selection
//! differs from its control's is a declaration that **moves a selection**.
//!
//! `declaration_name_census` writes the rows only when
//! `QR_SELECTOR_CENSUS_OUT` names a file, so it is `#[ignore]`d and run on
//! purpose (`docs/reports/qr-lexical-selectors-declaration-names/scripts/selector-census.sh`):
//!
//! ```sh
//! QR_SELECTOR_CENSUS_OUT=private/qrls/selector-census-NAME.jsonl \
//!   cargo test -p qcore-compiler --test qr_lexical_selector_census -- --ignored
//! ```
use qcore_compiler::{
    CheckedSourceModule, CompileOptions, CompiledModule, FileDiagnostic, SourceFile, compile_files,
    format_diagnostic, parse_source_program,
};
use qcore_frontend::frontend::SourceProgram;
use std::io::Write;

/// The control name: read by no selector, reserved by nothing.
pub const CONTROL: &str = "flb_noop";

/// Every word a route selector or a capability gate reads, with the selector
/// that reads it. Read off the code at the card's base (`4b5c77041`):
/// `module_imports::concurrency_span`, `source_tasks::has_source_tasks`,
/// `source_epoch::has_source_epoch`, `source_mutex::has_source_mutex`,
/// `source_scalar_calls::{has_scalar_calls, print_call_selection}` and
/// `source_bitfield_concurrency::has_source_bitfield_tasks` read the token
/// stream (or a declaration's tokens); the others read function-body
/// statements and are censused to show that they do.
pub const WORDS: &[(&str, &str)] = &[
    // module_imports::concurrency_span (the Q015 gate) and
    // source_tasks::has_source_tasks: any token.
    ("qcore_buffer_new", "concurrency-gate+tasks"),
    ("qcore_buffer_read", "concurrency-gate+tasks"),
    ("qcore_buffer_write", "concurrency-gate+tasks"),
    // concurrency_span only: any token.
    ("qcore_error_payload", "concurrency-gate"),
    ("spawn", "concurrency-gate"),
    ("join", "concurrency-gate"),
    // concurrency_span: `task <`, `mutex <`, `mutex guards`, `mutex rank`;
    // source_mutex::has_source_mutex: `mutex guards`, `mutex rank`.
    ("task", "concurrency-gate-pair"),
    ("mutex", "concurrency-gate-pair+mutex"),
    ("guards", "concurrency-gate-pair+mutex"),
    ("rank", "concurrency-gate-pair+mutex"),
    // source_epoch::has_source_epoch: `epoch {`.
    ("epoch", "epoch-pair"),
    // has_scalar_calls: any `qcore_acquire`/`qcore_check` deselects; any
    // `throws < ArithmeticError` selects.
    ("qcore_acquire", "scalar-calls-deselect"),
    ("qcore_check", "scalar-calls-deselect"),
    ("throws", "scalar-calls-throws-triple"),
    ("ArithmeticError", "scalar-calls-throws-triple"),
    // print_call_selection: a `print (` pair (fixed by the print card).
    ("print", "print-call+std-io"),
    // has_source_bitfield_tasks: `bits <` / `field <` in a record's tokens.
    ("bits", "bitfield-record-pair"),
    ("field", "bitfield-record-pair"),
    // Function-body statement selectors.
    ("log", "std-io"),
    ("log_level", "std-io"),
    ("flush", "std-io"),
    ("dec", "std-io-rendering"),
    ("hex", "std-io-rendering"),
    ("trace", "std-io-level"),
    ("info", "std-io-level"),
    ("error", "std-io-level"),
    ("bytes", "buffer"),
    ("extent", "buffer"),
    ("lend", "buffer"),
    ("drop", "buffer"),
    ("len", "string"),
    ("string", "string"),
    ("concat", "string"),
    ("borrow_utf8", "string-bridge"),
    ("tcp", "socket"),
    ("udp", "socket"),
    ("bind", "socket"),
    ("listen", "socket"),
    ("connect", "socket"),
    ("accept", "socket"),
    ("send", "socket"),
    ("recv", "socket"),
    ("sendto", "socket"),
    ("recvfrom", "socket"),
    ("close", "socket"),
    ("disjoint", "socket"),
    ("event", "event"),
    ("endpoints", "event"),
    ("watch", "event"),
    ("unwatch", "event"),
    ("wake", "event"),
    ("dispatch", "event"),
    ("request", "http"),
    ("response", "http"),
    ("header", "http"),
    ("content", "http"),
    ("thread", "tasks-statement"),
    ("scope", "tasks-statement"),
    ("footprint", "tasks-statement"),
    ("lock", "tasks-statement"),
    ("struct", "ffi-declaration"),
    ("c_ffi", "ffi-declaration"),
];

pub const MODES: &[&str] = &["contract", "library", "executable"];
pub const BASES: &[&str] = &["plain", "scalar"];
pub const SHAPES: &[&str] = &[
    "extern",
    "extern-contract",
    "function",
    "record",
    "error",
    "clause-parameter",
    "type-argument",
];

/// The one synthetic module of a (mode, base, shape) for `word`.
pub fn module_text(mode: &str, base: &str, shape: &str, word: &str) -> String {
    let mut text = String::new();
    match mode {
        "contract" => text.push_str("module contract;\n\n"),
        "library" => text.push_str("module library;\n\n"),
        _ => {}
    }
    let declaration = match shape {
        "extern" => format!("extern fn {word}(x: i32) -> i32;\n"),
        "extern-contract" => format!(
            "extern fn {word}(size: usize) -> ptr<mut, u8>\ncontract {{\n    requires resource_contract_v1;\n    ensures normal resource_borrowed(size);\n    ensures normal resource_return_owned(nullable);\n}};\n"
        ),
        "function" => format!("fn {word}() -> i32 {{\n    return 0;\n}}\n"),
        "record" => format!("type {word} {{ lo: i32, hi: i32 }}\n"),
        "error" => format!("error {word} {{ Failed; }}\n"),
        "clause-parameter" => format!(
            "extern fn {CONTROL}({word}: usize) -> ptr<mut, u8>\ncontract {{\n    requires resource_contract_v1;\n    ensures normal resource_borrowed({word});\n    ensures normal resource_return_owned(nullable);\n}};\n"
        ),
        "type-argument" => format!("extern fn {CONTROL}(x: {word}<i32>) -> i32;\n"),
        // The scalar route's `throws < ArithmeticError` test, written in an
        // extern declaration's row; its control drops the row.
        "throws-row" => {
            if word == CONTROL {
                format!("extern fn {CONTROL}(x: i32) -> i32;\n")
            } else {
                format!("extern fn {CONTROL}(x: i32) -> i32 throws<ArithmeticError>;\n")
            }
        }
        other => panic!("unknown shape {other}"),
    };
    text.push_str(&declaration);
    text.push('\n');
    match base {
        "plain" => text.push_str("fn main() -> i32 {\n    return 0;\n}\n"),
        _ => text.push_str(
            "fn one(x: i32) -> i32 {\n    return x + 1;\n}\n\nfn main() -> i32 {\n    return one(1);\n}\n",
        ),
    }
    text
}

fn path_for(mode: &str) -> &'static str {
    // A `module contract;` source is a `.qv` file (`Q403` otherwise).
    if mode == "contract" {
        "census.qv"
    } else {
        "census.qr"
    }
}

fn variant(checked: &CheckedSourceModule) -> &'static str {
    match checked {
        CheckedSourceModule::ScalarCalls(_) => "ScalarCalls",
        CheckedSourceModule::Sequential(_) => "Sequential",
        CheckedSourceModule::ScopedTasks(_) => "ScopedTasks",
        CheckedSourceModule::BitfieldTasks(_) => "BitfieldTasks",
        CheckedSourceModule::CheckedMutexGuards(_) => "CheckedMutexGuards",
        CheckedSourceModule::CheckedEpochBound(_) => "CheckedEpochBound",
        CheckedSourceModule::ComposedEpochTasks(_) => "ComposedEpochTasks",
        CheckedSourceModule::ExternCalls(_) => "ExternCalls",
        CheckedSourceModule::StandardIo(_) => "StandardIo",
        CheckedSourceModule::Composed(_) => "Composed",
        CheckedSourceModule::ComposedTasks(_) => "ComposedTasks",
        CheckedSourceModule::StandardBuffer(_) => "StandardBuffer",
        CheckedSourceModule::Event(_) => "Event",
        CheckedSourceModule::StandardString(_) => "StandardString",
        CheckedSourceModule::StandardSocket(_) => "StandardSocket",
        CheckedSourceModule::StandardHttp(_) => "StandardHttp",
        CheckedSourceModule::Ffi(_) => "Ffi",
    }
}

/// Every selector the driver's route selection consults, in its order, and
/// the import gate. Only public functions, so the census runs unchanged on
/// the base and on the fix.
pub fn selection(parsed: &SourceProgram) -> serde_json::Value {
    use qcore_frontend::*;
    let gate = match module_imports::check_source_imports(parsed) {
        Ok(_) => "ok".to_owned(),
        Err(diagnostics) => diagnostics
            .iter()
            .map(|d| format!("{}: {}", d.code, d.message))
            .collect::<Vec<_>>()
            .join(" | "),
    };
    let print = match source_scalar_calls::print_call_selection(parsed) {
        source_scalar_calls::PrintCallSelection::Absent => "absent".to_owned(),
        source_scalar_calls::PrintCallSelection::Selected => "selected".to_owned(),
        source_scalar_calls::PrintCallSelection::Ambiguous(d) => {
            format!("ambiguous: {}", d.message)
        }
    };
    serde_json::json!({
        "imports": gate,
        "closures": !parsed.closures().is_empty(),
        "ffi": source_ffi::has_source_ffi(parsed),
        "bitfield_tasks": source_bitfield_concurrency::has_source_bitfield_tasks(parsed),
        "composed_epoch": source_epoch_tasks::has_composed_epoch(parsed),
        "epoch": source_epoch::has_source_epoch(parsed),
        "mutex": source_mutex::has_source_mutex(parsed),
        "composed_tasks": source_composed_tasks::has_source_composed_tasks(parsed),
        "tasks": source_tasks::has_source_tasks(parsed),
        "composed": source_composed::has_source_composed(parsed),
        "extern": source_extern::has_source_extern(parsed),
        "event": source_event::has_source_event(parsed),
        "socket": source_socket::has_source_socket(parsed),
        "string_bridge": source_std_string::has_source_string_bridge(parsed),
        "resource_calls": source_resource_calls::has_resource_calls(parsed),
        "record": source_record::has_source_record(parsed),
        "buffer": source_std_buffer::has_source_buffer(parsed),
        "http": source_http::has_source_http(parsed),
        "string": source_std_string::has_source_string(parsed),
        "print_call": print,
        "std_io": source_std_io::has_source_std_io(parsed),
        "log_level": source_std_io::has_source_log_level(parsed),
        "scalar_calls": source_scalar_calls::has_scalar_calls(parsed),
    })
}

pub fn outcome(result: Result<CompiledModule, Vec<FileDiagnostic>>) -> serde_json::Value {
    match result {
        Ok(module) => serde_json::json!({
            "outcome": "admitted",
            "mode": format!("{:?}", module.mode),
            "profile": format!("{:?}", module.execution_profile),
            "checked": variant(&module.checked),
            "functions": module.checked.function_count(),
            "declarations": module.declaration_count,
            "entry": module.entry,
        }),
        Err(diagnostics) => serde_json::json!({
            "outcome": "refused",
            "codes": diagnostics.iter().map(|d| d.code.clone()).collect::<Vec<_>>(),
            "diagnostics": diagnostics.iter().map(format_diagnostic).collect::<Vec<_>>(),
        }),
    }
}

/// One census row: the module, its selection and its compile outcome.
pub fn row(mode: &str, base: &str, shape: &str, word: &str) -> serde_json::Value {
    let text = module_text(mode, base, shape, word);
    let selection = match parse_source_program(&text) {
        Ok(parsed) => serde_json::json!({ "parsed": true, "selectors": selection(&parsed) }),
        Err(error) => serde_json::json!({ "parsed": false, "error": format!("{error:?}") }),
    };
    let file = SourceFile::new(path_for(mode), text.clone());
    let compiled = std::panic::catch_unwind(|| {
        compile_files(std::slice::from_ref(&file), &CompileOptions::default())
    });
    let outcome = match compiled {
        Ok(result) => outcome(result),
        Err(_) => serde_json::json!({ "outcome": "panicked" }),
    };
    serde_json::json!({
        "mode": mode,
        "base": base,
        "shape": shape,
        "word": word,
        "selection": selection,
        "compile": outcome,
    })
}

/// Every (mode, base, shape, word) of the census, the controls first.
pub fn census_keys() -> Vec<(&'static str, &'static str, &'static str, &'static str)> {
    let mut keys = Vec::new();
    for mode in MODES {
        for base in BASES {
            for shape in SHAPES {
                keys.push((*mode, *base, *shape, CONTROL));
                for (word, _) in WORDS {
                    keys.push((*mode, *base, *shape, *word));
                }
            }
            keys.push((*mode, *base, "throws-row", CONTROL));
            keys.push((*mode, *base, "throws-row", "throws<ArithmeticError>"));
        }
    }
    keys
}

/// The selector family a census word belongs to.
pub fn family(shape: &str, word: &str) -> &'static str {
    if word == CONTROL {
        return "control";
    }
    if shape == "throws-row" {
        return "scalar-calls-throws-row";
    }
    WORDS
        .iter()
        .find(|(candidate, _)| *candidate == word)
        .map_or("unknown", |(_, family)| family)
}

fn outcome_class(compile: &serde_json::Value) -> serde_json::Value {
    let mut class = compile.clone();
    if let Some(object) = class.as_object_mut() {
        object.remove("diagnostics");
    }
    class
}

/// The selectors a census row is **expected** to move against its control.
/// The name shapes (an `extern fn`'s, a source function's, an error's, a
/// parameter's named in a contract clause) move none. Three shapes are not a
/// declaration's name and keep the base's reading: a **record**'s name
/// (constraint 14's `AGGREGATE-SELECTOR-WORDS`: a record spelling a selector
/// word stays refused at its declaration), a type constructor in a parameter
/// type and an extern `throws` row (type positions are uses). For those the
/// expected moves are exactly the base's (`artifacts/selector-census-before.json`).
pub fn expected_moves(base: &str, shape: &str, word: &str) -> Vec<&'static str> {
    let read = match shape {
        "record" | "type-argument" => true,
        "throws-row" => {
            return if base == "plain" {
                vec!["scalar_calls"]
            } else {
                vec![]
            };
        }
        _ => false,
    };
    if !read {
        return vec![];
    }
    match word {
        "qcore_buffer_new" | "qcore_buffer_read" | "qcore_buffer_write" => vec!["imports", "tasks"],
        "qcore_error_payload" | "spawn" | "join" => vec!["imports"],
        "task" | "mutex" if shape == "type-argument" => vec!["imports"],
        "epoch" if shape == "record" => vec!["epoch"],
        "qcore_acquire" | "qcore_check" if base == "scalar" => vec!["scalar_calls"],
        _ => vec![],
    }
}

/// **No declaration name moves a selection** (always run). Every census row
/// reads as its control — the same value for every selector and for the
/// import gate, and a compile outcome of the same class (route, or refusal
/// codes) — except the moves [`expected_moves`] names, which are exactly the
/// base's for the shapes that are not a declaration's name.
#[test]
fn no_declaration_name_moves_a_selection() {
    let mut controls = std::collections::BTreeMap::new();
    let mut wrong = Vec::new();
    for (mode, base, shape, word) in census_keys() {
        let line = row(mode, base, shape, word);
        if word == CONTROL {
            assert_eq!(line["selection"]["parsed"], true, "{line}");
            controls.insert((mode, base, shape), line);
            continue;
        }
        let control = &controls[&(mode, base, shape)];
        let mine = line["selection"]["selectors"]
            .as_object()
            .expect("the row parses");
        let theirs = control["selection"]["selectors"]
            .as_object()
            .expect("the control parses");
        let moved = mine
            .iter()
            .filter(|(key, value)| theirs.get(*key) != Some(value))
            .map(|(key, _)| key.as_str())
            .collect::<Vec<_>>();
        let expected = expected_moves(base, shape, word);
        if moved != expected {
            wrong.push(format!(
                "{mode}/{base}/{shape}/{word}: moved {moved:?}, expected {expected:?}"
            ));
            continue;
        }
        if expected.is_empty()
            && outcome_class(&line["compile"]) != outcome_class(&control["compile"])
        {
            wrong.push(format!("{mode}/{base}/{shape}/{word}: outcome"));
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}

#[test]
#[ignore = "writes the selector census only when QR_SELECTOR_CENSUS_OUT is set; run on purpose"]
fn declaration_name_census() {
    let Some(out) = std::env::var_os("QR_SELECTOR_CENSUS_OUT") else {
        return;
    };
    let mut file = std::fs::File::create(&out).expect("the census output is writable");
    for (mode, base, shape, word) in census_keys() {
        let mut line = row(mode, base, shape, word);
        line["family"] = family(shape, word).into();
        writeln!(
            file,
            "{}",
            serde_json::to_string(&line).expect("a census row serializes")
        )
        .expect("the census row is written");
    }
}

#[path = "../../tests/support/tracked_files.rs"]
mod tracked_files;

fn within_span(outer: qcore_compiler::lexer::Span, inner: qcore_compiler::lexer::Span) -> bool {
    outer.start <= inner.start && inner.end <= outer.end
}

/// Where one selector-word occurrence lies in its module.
pub fn location(parsed: &SourceProgram, span: qcore_compiler::lexer::Span) -> String {
    use qcore_frontend::frontend::DeclarationKind;
    if parsed.in_library_binding(span) {
        return "library-binding".into();
    }
    let within =
        |outer: qcore_compiler::lexer::Span| outer.start <= span.start && span.end <= outer.end;
    for declaration in &parsed.declarations {
        if !within(declaration.span) {
            continue;
        }
        if declaration
            .value
            .function_contract
            .as_ref()
            .is_some_and(|contract| within(contract.span))
        {
            return "contract-clause".into();
        }
        if declaration.value.kind == DeclarationKind::Function {
            if declaration
                .value
                .body
                .as_ref()
                .is_some_and(|body| within(body.span))
            {
                return "function-body".into();
            }
            if declaration
                .value
                .parameters
                .iter()
                .any(|parameter| within(parameter.name.span))
            {
                return "function-parameter-name".into();
            }
            // `fn NAME`: the name is the declaration's second token.
            let name = parsed
                .lexical_tokens()
                .iter()
                .filter(|token| within_span(declaration.span, token.span))
                .nth(1)
                .map(|token| token.span);
            if name == Some(span) {
                return "function-name".into();
            }
            return "function-header".into();
        }
        return format!("declaration:{:?}", declaration.value.kind);
    }
    "outside-declarations".into()
}

/// Every token-stream selector occurrence: (selector shape, token index).
pub fn selector_occurrences(parsed: &SourceProgram) -> Vec<(&'static str, usize)> {
    use qcore_compiler::lexer::{Symbol, TokenKind};
    let tokens = parsed.lexical_tokens();
    let word = |at: usize| match tokens.get(at).map(|t| &t.kind) {
        Some(TokenKind::Word(w)) => Some(w.as_str()),
        _ => None,
    };
    let symbol =
        |at: usize, s: Symbol| tokens.get(at).map(|t| &t.kind) == Some(&TokenKind::Symbol(s));
    let mut found = Vec::new();
    for at in 0..tokens.len() {
        let shape = match word(at) {
            Some(
                w @ ("qcore_buffer_new"
                | "qcore_buffer_read"
                | "qcore_buffer_write"
                | "qcore_error_payload"
                | "spawn"
                | "join"
                | "qcore_acquire"
                | "qcore_check"),
            ) => WORDS
                .iter()
                .find(|(candidate, _)| *candidate == w)
                .map(|(candidate, _)| *candidate)
                .expect("every single-word selector is in WORDS"),
            Some("task") if symbol(at + 1, Symbol::Less) => "task <",
            Some("mutex") if symbol(at + 1, Symbol::Less) => "mutex <",
            Some("mutex") if word(at + 1) == Some("guards") => "mutex guards",
            Some("mutex") if word(at + 1) == Some("rank") => "mutex rank",
            Some("epoch") if symbol(at + 1, Symbol::LeftBrace) => "epoch {",
            Some("throws")
                if symbol(at + 1, Symbol::Less) && word(at + 2) == Some("ArithmeticError") =>
            {
                "throws < ArithmeticError"
            }
            Some("print") if symbol(at + 1, Symbol::LeftParen) => "print (",
            _ => continue,
        };
        found.push((shape, at));
    }
    found
}

fn tracked_sources(root: &std::path::Path) -> Vec<std::path::PathBuf> {
    tracked_files::tracked_files(root)
        .into_iter()
        .filter(|path| {
            path.extension().is_some_and(|extension| {
                ["qr", "qv", "qcore", "qcorelib"]
                    .iter()
                    .any(|wanted| extension == *wanted)
            })
        })
        .collect()
}

/// Every tracked source's selector occurrences, by shape and location.
pub fn tracked_occurrences() -> std::collections::BTreeMap<(String, String), Vec<String>> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the compiler crate sits directly under the repository root")
        .to_path_buf();
    let mut table: std::collections::BTreeMap<(String, String), Vec<String>> = Default::default();
    for path in tracked_sources(&root) {
        let source = std::fs::read_to_string(&path).expect("a tracked source is readable");
        let Ok(parsed) = parse_source_program(&source) else {
            continue;
        };
        let relative = path
            .strip_prefix(&root)
            .expect("a tracked path lies under the root")
            .display()
            .to_string();
        for (shape, at) in selector_occurrences(&parsed) {
            let span = parsed.lexical_tokens()[at].span;
            let entry = table
                .entry((shape.to_owned(), location(&parsed, span)))
                .or_default();
            if entry.last() != Some(&relative) {
                entry.push(relative.clone());
            }
        }
    }
    table
}

/// Where every token-stream selector word stands in the tracked sources
/// (always run). Inside a source function (its body, or its signature's
/// result type and effect row) is where a selector reads; every other
/// position is pinned by name, so a new declaration spelling a selector word
/// must be added here deliberately. No tracked source names a source
/// function or a parameter after a selector word.
#[test]
fn every_tracked_selector_word_stands_where_pinned() {
    let table = tracked_occurrences();
    let mut outside = Vec::new();
    for ((shape, location), files) in &table {
        if location == "function-body" || location == "function-header" {
            continue;
        }
        for file in files {
            outside.push(format!("{shape} @ {location}: {file}"));
        }
    }
    assert_eq!(
        outside,
        [
            "join @ contract-clause: providers/quirin-provider-libc/contracts/libc/pthread_sync.qv",
            "print ( @ declaration:ExternFunction: docs/reports/premise-decomposition-cycle1/proposed-cjson.qv",
            "print ( @ declaration:ExternFunction: providers/qcore-provider-librdkafka/contracts/librdkafka/cjson.qv",
            "spawn @ contract-clause: providers/qcore-provider-libevent/contracts/monkey/callback_task_handoff.qv",
        ]
    );
    let header = table
        .iter()
        .filter(|((_, location), _)| location == "function-header")
        .map(|((shape, _), files)| (shape.as_str(), files.len()))
        .collect::<Vec<_>>();
    // Effect rows (`effects { spawn, join }`), two archived probes' result
    // types (`-> task<i32>`, `-> mutex<i32>`) and the scalar route's
    // `throws<ArithmeticError>` rows.
    assert_eq!(
        header,
        [
            ("join", 154),
            ("mutex <", 1),
            ("spawn", 154),
            ("task <", 1),
            ("throws < ArithmeticError", 4)
        ]
    );
}
