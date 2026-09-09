//! File-local import resolution, before name/type checking or source lowering.
use crate::frontend::{
    DeclarationKind, SemanticPass, SourceBlock, SourceDiagnostic, SourceProgram, StatementKind,
};
use crate::lexer::{Span, Symbol, TokenKind};
use crate::source_std_io::StdIoUse;
use qcore_core::module_imports::{
    CONCURRENCY_MODULE, EXTERN_MODULE, FMT_MODULE, IO_MODULE, LOGGER_MODULE, ModuleImports,
};

/// Importing another compilation input never grants this file a capability.
/// Call this on each input before combining declarations across files.
pub fn check_source_imports(
    program: &SourceProgram,
) -> Result<ModuleImports, Vec<SourceDiagnostic>> {
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
                "concurrency requires an explicit `import {CONCURRENCY_MODULE};` in this source file"
            ),
        }]);
    }
    if !imports.extern_calls()
        && let Some(span) = extern_call_span(program)
    {
        return Err(vec![SourceDiagnostic {
            code: "Q022",
            pass: SemanticPass::ExternCall,
            span,
            message: format!(
                "calling a declared extern function requires an explicit `import {EXTERN_MODULE};` in this source file"
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
            .or_else(|| uses.get(&StdIoUse::Flush))
    {
        return Err(vec![missing_import(
            *span,
            LOGGER_MODULE,
            "`log` and `flush`",
        )]);
    }
    if !imports.fmt()
        && let Some(span) = crate::source_std_io::rendering_span(program)
    {
        return Err(vec![missing_import(span, FMT_MODULE, "naming a rendering")]);
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
        message: format!("{what} requires an explicit `import {module};` in this source file"),
    }
}

/// The first call in this file whose callee is one of this file's own `extern
/// fn` declarations, in a statement form `source.extern-call.v1` can express.
/// Declaring an extern function is not calling one, so a contract module and
/// every existing declaration-only file are untouched, and resolution stays
/// file-local: importing another compilation input never grants this file the
/// vocabulary. One detector serves both this gate and route selection, so a
/// file can never be routed to the extern profile without having needed the
/// import, or need the import without being routed there.
fn extern_call_span(program: &SourceProgram) -> Option<Span> {
    crate::source_extern::called_extern_names(program)
        .into_values()
        .next()
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
    for (index, token) in tokens.iter().enumerate() {
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
