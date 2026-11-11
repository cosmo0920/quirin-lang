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

//! File-local import resolution, before name/type checking or source lowering.
use crate::frontend::{
    DeclarationKind, SemanticPass, SourceBlock, SourceDiagnostic, SourceProgram, StatementKind,
};
use crate::lexer::{Span, Symbol, TokenKind};
use crate::source_std_io::StdIoUse;
use qcore_core::module_imports::{
    BUFFER_MODULE, CONCURRENCY_MODULE, EVENT_MODULE, EXTERN_MODULE, FFI_MODULE, FMT_MODULE,
    HTTP_MODULE, IO_MODULE, LOGGER_MODULE, ModuleImports, SOCKET_MODULE, STRING_MODULE,
    UNCHECKED_EXTERN_MODULE,
};

/// Importing another compilation input never grants this file a capability.
/// Call this on each input before combining declarations across files.
pub fn check_source_imports(
    program: &SourceProgram,
) -> Result<ModuleImports, Vec<SourceDiagnostic>> {
    let externs = program
        .declarations
        .iter()
        .filter(|d| d.value.kind == DeclarationKind::ExternFunction)
        .map(|d| d.value.name.clone())
        .collect();
    check_source_imports_in_scope(program, &externs)
}

/// As above, with the linked extern-name inventory. Only this file's tokens,
/// declarations and import choices contribute capability uses/permissions.
pub fn check_source_imports_in_scope(
    program: &SourceProgram,
    externs: &std::collections::BTreeSet<String>,
) -> Result<ModuleImports, Vec<SourceDiagnostic>> {
    // Card `PIPE-OPERATOR`, first: a malformed pipeline and a claimed-but-
    // unlowered logical operator are syntactic facts about this file that do
    // not depend on what it imported. Running the operator gate before the
    // module gates means `x |> flush;` reports the pipe refusal rather than a
    // missing-import diagnostic for a capability the statement never reached.
    crate::pipe::check_source_operators(program)?;
    for import in program.imports() {
        ModuleImports::resolve([import.value.as_str()]).map_err(|error| {
            vec![SourceDiagnostic {
                code: "Q014",
                pass: SemanticPass::NameResolution,
                span: import.span,
                message: error.to_string(),
            }]
        })?;
    }
    let imports =
        ModuleImports::resolve(program.imports().iter().map(|import| import.value.as_str()))
            .expect("every module name was resolved above");
    if !imports.concurrency()
        && let Some(span) = concurrency_span(program)
    {
        return Err(vec![SourceDiagnostic {
            code: "Q015",
            pass: SemanticPass::NameResolution,
            span,
            message: format!(
                "concurrency requires an explicit `#import {CONCURRENCY_MODULE};` in this source file"
            ),
        }]);
    }
    if !imports.extern_calls()
        && let Some(span) = crate::source_extern::called_extern_names_in_scope(program, externs)
            .into_values()
            .next()
    {
        return Err(vec![SourceDiagnostic {
            code: "Q022",
            pass: SemanticPass::ExternCall,
            span,
            message: format!(
                "calling a declared extern function requires an explicit `#import {EXTERN_MODULE};` in this source file"
            ),
        }]);
    }
    // Card `UNSAFE-LINKAGE`. Declaring an extern function with no contract is a
    // second, independent opt-in on top of `std.extern`: the call vocabulary
    // never grants the waiver, and the waiver never grants the call. The gate
    // is on the *declaration*, not on a call to it, because declaring an
    // uncontracted symbol is itself the trust act — a file that declares one
    // and never calls it has still linked it.
    if !imports.unchecked_extern_calls()
        && let Some(span) = crate::source_extern::unchecked_extern_declaration_span(program)
    {
        return Err(vec![SourceDiagnostic {
            code: "Q032",
            pass: SemanticPass::UncheckedLinkage,
            span,
            message: format!(
                "declaring an extern function with no contract requires an explicit `#import {UNCHECKED_EXTERN_MODULE};` in this source file"
            ),
        }]);
    }
    // Card `W48-QR-C-STRUCT-IMPORT` (user decision 1): an imported C struct is
    // the FFI trust act, gated by `std.ffi.unsafe` and by no other module —
    // the `UNSAFE-LINKAGE` precedent exactly, so the refusal is that card's
    // `Q032`, widened from "an `unsafe extern fn` declaration" to "an FFI
    // construct" (§L21). A call through a `#c_ffi` contract from a file
    // without the import never reaches a marked contract at all: the loader
    // withholds marked declarations from every route but `source.ffi.v1`.
    if !imports.ffi()
        && let Some(span) = crate::source_ffi::ffi_declaration_span(program)
    {
        return Err(vec![SourceDiagnostic {
            code: "Q032",
            pass: SemanticPass::UncheckedLinkage,
            span,
            message: format!(
                "importing a C struct with `struct! {{ … }}` requires an explicit `#import {FFI_MODULE};` in this source file; the import marks the whole file unsafe"
            ),
        }]);
    }
    // Card `STD-MODULES`. Each capability is gated by its own module, and by no
    // other: `std.io` never grants the logger, `std.logger` never grants
    // standard output, and `std.fmt` alone opens no stream at all. `flush`
    // names no rendering, so a logger-only file that never renders needs no
    // `std.fmt` — measured, not assumed.
    let uses = crate::source_std_io::std_io_uses(program);
    if !imports.io()
        && let Some(span) = uses.get(&StdIoUse::Print)
    {
        return Err(vec![missing_import(*span, IO_MODULE, "`print`")]);
    }
    if !imports.logger()
        && let Some(span) = uses
            .get(&StdIoUse::Log)
            .or_else(|| uses.get(&StdIoUse::LogLevel))
            .or_else(|| uses.get(&StdIoUse::Flush))
    {
        return Err(vec![missing_import(
            *span,
            LOGGER_MODULE,
            "`log`, `log_level`, and `flush`",
        )]);
    }
    if !imports.logger()
        && let Some(span) = crate::source_event::event_log_level_span(program)
    {
        return Err(vec![missing_import(span, LOGGER_MODULE, "`log_level`")]);
    }
    if !imports.fmt()
        && let Some(span) = crate::source_std_io::rendering_span(program)
    {
        return Err(vec![missing_import(span, FMT_MODULE, "naming a rendering")]);
    }
    // Card `STD-BUFFER`, on exactly the same terms as the three above: acquiring
    // an owned byte buffer, indexing into one, reading its extent or releasing
    // one is gated by `std.buffer` and by no other module. `std.io` never grants
    // it and it never grants `std.io`, so a module that both buffers and prints
    // names both — measured, not assumed.
    //
    // `drop NAME;` is deliberately **not** a gated use. `drop` is an established
    // statement of the language with an established meaning, and gating it here
    // would make every existing sequential module that drops a resource depend
    // on a module it never needed.
    if !imports.buffer()
        && let Some(span) = crate::source_std_buffer::buffer_span(program)
    {
        return Err(vec![missing_import(
            span,
            BUFFER_MODULE,
            "an owned byte buffer",
        )]);
    }
    // Card `STRING-SLICE`, on exactly the same terms as the buffer gate above:
    // constructing a string view, reading its byte length or slicing it is gated
    // by `std.string` and by no other module. `std.buffer` never grants it and
    // it never grants `std.buffer` — a buffer is owned and mutable and a string
    // is an immutable view of storage it does not own — and neither output
    // module grants it, because this amendment does not make `std.io` or
    // `std.logger` accept text (`STRING-SLICE-IO`, amendment §7). A module that
    // slices and prints a length therefore names both, measured and not assumed.
    //
    // The detector reads **function-body statements only**, which is what keeps
    // amendment §2's quoted-metadata requirement true by construction: a
    // `verify scope { plugin … version "0.1" profile "…" }` declaration's quoted
    // tokens are not statements and cannot reach it, so recognizing them
    // acquires no import requirement.
    if !imports.string()
        && let Some(span) = crate::source_std_string::string_span(program)
    {
        return Err(vec![missing_import(
            span,
            STRING_MODULE,
            "an immutable UTF-8 string view",
        )]);
    }
    // Card `STD-EVENT`. The same file-local opt-in discipline as concurrency:
    // the event context is unavailable by default and importing another
    // compilation input never grants this file the vocabulary. One detector
    // serves this gate and route selection, so a file can never be routed to
    // `source.event.v1` without having needed the import. `log_level` selects
    // this route but also passes the independent logger gate above.
    if !imports.event()
        && let Some(span) = crate::source_event::source_event_span(program)
    {
        return Err(vec![missing_import(
            span,
            EVENT_MODULE,
            "the checked event vocabulary",
        )]);
    }
    // Card `QUIRIN-SOCKET-V1`. The same file-local opt-in discipline as
    // concurrency and the event vocabulary: a socket is unavailable by default
    // and importing another compilation input never grants this file the
    // capability. One detector serves this gate and route selection, so a file
    // can never be routed to `source.socket.v1` without having needed the
    // import, or need the import without being routed there.
    //
    // `close NAME;` is deliberately **not** a gated use: `close` is an
    // established statement head of two other profiles, and gating it here would
    // make an existing module that closes a mutex guard depend on a module it
    // never needed. That is the exclusion `STD-BUFFER` made for `drop`.
    if !imports.socket()
        && let Some(span) = crate::source_socket::socket_span(program)
    {
        return Err(vec![missing_import(
            span,
            SOCKET_MODULE,
            "the checked socket vocabulary",
        )]);
    }
    // Card `HTTP-QUIRIN-V1`, landing `SOCKET-STRING-PAYLOAD`. A transfer whose
    // payload is a quoted literal carries a **string view's own bytes**, which
    // is `std.string`'s value and not `std.socket`'s, so it needs both modules —
    // exactly as a rendered text line needs both `std.string` and an output
    // module (`STRING-SLICE-IO`, amendment §7). The socket module alone still
    // carries a bracketed byte list and nothing else; the string module alone
    // still opens no socket.
    if !imports.string()
        && let Some(span) = crate::source_socket::text_payload_span(program)
    {
        return Err(vec![missing_import(
            span,
            STRING_MODULE,
            "a string view as a transfer payload",
        )]);
    }
    // Card `HTTP-QUIRIN-V1`. The same file-local opt-in discipline again:
    // declaring an HTTP message is unavailable by default and importing another
    // compilation input never grants this file the vocabulary. One detector
    // serves this gate and route selection.
    //
    // The composition rule is the argued part, and it is two separate
    // statements:
    //
    // * `std.http` grants **no transport**. A message is a value; a program that
    //   actually speaks HTTP names `std.socket` too, and the socket gate above
    //   is what says so. Neither module grants the other.
    // * `std.http` grants **no text**. A quoted token at a method, a target, a
    //   field name, a field value or a content position is *this module's own
    //   syntax* and constructs no `string` value: nothing can bind it, slice it,
    //   measure it or render it. `STRING_SLICE_V1.md` §2 excludes exactly that
    //   case in as many words, so a pure HTTP program needs no `std.string`. A
    //   program that **binds** a payload with `let s = "…";` does construct one,
    //   and the gate below asks for the module then.
    if !imports.http()
        && let Some(span) = crate::source_http::http_span(program)
    {
        return Err(vec![missing_import(
            span,
            HTTP_MODULE,
            "the checked HTTP/1.1 message vocabulary",
        )]);
    }
    if !imports.string()
        && let Some(span) = crate::source_http::text_binding_span(program)
    {
        return Err(vec![missing_import(
            span,
            STRING_MODULE,
            "a bound string view",
        )]);
    }
    Ok(imports)
}

/// `Q015` is the existing "this file uses a capability it did not import"
/// diagnostic; this card reuses it rather than minting a code that would mean
/// the same thing. The message names the module, so the three capabilities are
/// distinguishable without three codes.
fn missing_import(span: Span, module: &str, what: &str) -> SourceDiagnostic {
    SourceDiagnostic {
        code: "Q015",
        pass: SemanticPass::NameResolution,
        span,
        message: format!("{what} requires an explicit `#import {module};` in this source file"),
    }
}

// Inspect structural concurrency forms rather than treating a ghost field named
// `thread` or `lock` in a resource contract as an executable operation.
fn concurrency_span(program: &SourceProgram) -> Option<Span> {
    fn block_span(block: &SourceBlock) -> Option<Span> {
        for statement in &block.statements {
            if matches!(
                statement.value,
                StatementKind::ThreadScope { .. }
                    | StatementKind::WithLock { .. }
                    | StatementKind::Footprint { .. }
            ) {
                return Some(statement.span);
            }
            for block in statement.value.nested_blocks() {
                if let Some(span) = block_span(block) {
                    return Some(span);
                }
            }
        }
        None
    }
    for declaration in &program.declarations {
        if declaration.value.kind == DeclarationKind::ConcurrencyProfile {
            return Some(declaration.span);
        }
        if let Some(span) = declaration.value.body.as_ref().and_then(block_span) {
            return Some(span);
        }
    }
    if let Some(task) = program.task_expressions().first() {
        return Some(task.span);
    }
    let tokens = program.lexical_tokens();
    // Card `QR-LEXICAL-SELECTORS-DECLARATION-NAMES`: the token loop skips a
    // declaration's own name and a parameter's name (an
    // `extern fn qcore_buffer_new(…)` in a contract module was refused here at
    // its name). A record's name and fields stay read (constraint 14's
    // `AGGREGATE-SELECTOR-WORDS`), and so do types and effect rows. The two
    // exclusions below are kept as they were; the scope subsumes them.
    let scope = crate::route_selection::SelectorScope::of(program);
    for (index, token) in tokens.iter().enumerate() {
        if !scope.reads(program, token.span) {
            continue;
        }
        // Card `NATIVE-LIBRARY-SOURCE-BINDING`, constraint 14: an
        // `extern library NAME;` declaration's tokens are a *binding*, never a
        // use, so a library whose name happens to be `spawn` or
        // `qcore_buffer_new` must not select the concurrency route from its own
        // spelling. Same exclusion, same reason as the contract-span skip below.
        if program.in_library_binding(token.span) {
            continue;
        }
        if program.declarations.iter().any(|declaration| {
            declaration
                .value
                .function_contract
                .as_ref()
                .is_some_and(|contract| {
                    contract.span.start <= token.span.start && token.span.end <= contract.span.end
                })
        }) {
            continue;
        }
        let TokenKind::Word(word) = &token.kind else {
            continue;
        };
        let next = tokens.get(index + 1).map(|token| &token.kind);
        let requires = match word.as_str() {
            "qcore_buffer_new"
            | "qcore_buffer_read"
            | "qcore_buffer_write"
            | "qcore_error_payload"
            | "spawn"
            | "join" => true,
            "task" | "mutex" => {
                next == Some(&TokenKind::Symbol(Symbol::Less))
                    // `mutex rank` is card CT33-RANKS's ranked declaration and
                    // needs the concurrency import exactly as `mutex guards`
                    // does; without this arm the ranked spelling would be the
                    // one mutex form usable with no import.
                    || word == "mutex"
                        && matches!(
                            next,
                            Some(TokenKind::Word(word)) if word == "guards" || word == "rank"
                        )
            }
            _ => false,
        };
        if requires {
            return Some(token.span);
        }
    }
    None
}
