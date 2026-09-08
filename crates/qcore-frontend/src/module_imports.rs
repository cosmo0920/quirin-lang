//! File-local import resolution, before name/type checking or source lowering.
use crate::frontend::{
    DeclarationKind, SemanticPass, SourceBlock, SourceDiagnostic, SourceProgram, StatementKind,
};
use crate::lexer::{Span, Symbol, TokenKind};
use qcore_core::module_imports::{CONCURRENCY_MODULE, ModuleImports};

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
    Ok(imports)
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
                    || word == "mutex"
                        && matches!(next, Some(TokenKind::Word(word)) if word == "guards")
            }
            _ => false,
        };
        if requires {
            return Some(token.span);
        }
    }
    None
}
