// SPDX-License-Identifier: Apache-2.0
//! A bounded, generation-indexed custody result passed between source functions.
//! Imported declarations remain assumptions to be matched against selected contracts
//! by the compilation driver. Only owned-argument functions have a public C ABI.

mod body;
mod contract;
pub use contract::Adapter;

use crate::lexer::{Span, Symbol as S, Token, TokenKind as T};
use contract::{Reader, Variant};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

#[derive(Clone, Debug)]
pub struct CheckedVariantProgram {
    c: String,
    adapters: Vec<Adapter>,
    functions: Vec<String>,
}

impl CheckedVariantProgram {
    /// Conditional C lowering. The caller must first bind every adapter to a
    /// matching selected extern ABI and relational contract (C04.13.2).
    pub fn emit_c(&self) -> &str {
        &self.c
    }
    pub fn adapters(&self) -> &[Adapter] {
        &self.adapters
    }
    pub fn function_names(&self) -> &[String] {
        &self.functions
    }
}

#[derive(Clone, Debug)]
pub struct VariantDiagnostic {
    pub span: Span,
    pub message: String,
}

impl std::fmt::Display for VariantDiagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

#[derive(Debug)]
enum Parameter {
    Owner,
    Subject,
    Package { variant: String, subject: String },
}

struct Function {
    span: Span,
    name: String,
    parameters: Vec<(String, Parameter)>,
    body: std::ops::Range<usize>,
}

impl Function {
    fn entry(&self) -> bool {
        self.parameters
            .iter()
            .all(|(_, ty)| matches!(ty, Parameter::Owner))
    }

    fn c_name(&self) -> String {
        format!(
            "qv_{}_{}",
            if self.entry() { "entry" } else { "helper" },
            self.name
        )
    }

    fn prototype(&self) -> String {
        let mut params = Vec::new();
        for (idx, (_, ty)) in self.parameters.iter().enumerate() {
            params.push(match ty {
                Parameter::Owner => format!("unsigned char *qv_in_{idx}"),
                Parameter::Subject => format!("unsigned char **qv_slot_{idx}"),
                Parameter::Package { variant, .. } => {
                    format!("struct qv_type_{variant} qv_arg_{idx}")
                }
            });
        }
        let storage = if self.entry() { "" } else { "static " };
        format!("{storage}int {}({})", self.c_name(), params.join(", "))
    }
}

fn function(r: &mut Reader<'_>) -> Result<Function, String> {
    let span = r.tokens[r.pos].span;
    r.word("fn")?;
    let name = r.ident()?;
    r.symbol(S::LeftParen)?;
    let mut parameters = Vec::new();
    let mut names = BTreeSet::new();
    loop {
        let param = r.ident()?;
        if !names.insert(param.clone()) {
            return Err(format!("duplicate parameter {param}"));
        }
        r.symbol(S::Colon)?;
        let ty = if r.eat_word("quant") {
            r.word("variant")?;
            let variant = r.ident()?;
            r.symbol(S::LeftParen)?;
            let subject = r.ident()?;
            r.symbol(S::RightParen)?;
            Parameter::Package { variant, subject }
        } else if r.eat_word("own") {
            r.symbol(S::Less)?;
            r.word("Buffer")?;
            r.symbol(S::Greater)?;
            Parameter::Owner
        } else {
            r.word("ptr")?;
            r.symbol(S::Less)?;
            r.word("mut")?;
            r.symbol(S::Comma)?;
            r.word("u8")?;
            r.symbol(S::Greater)?;
            Parameter::Subject
        };
        parameters.push((param, ty));
        if r.eat_symbol(S::RightParen) {
            break;
        }
        r.symbol(S::Comma)?;
    }
    r.symbol(S::Arrow)?;
    r.word("i32")?;
    r.symbol(S::LeftBrace)?;
    let start = r.pos;
    let mut depth = 1;
    while depth != 0 {
        match r.peek() {
            Some(T::Symbol(S::LeftBrace)) => depth += 1,
            Some(T::Symbol(S::RightBrace)) => depth -= 1,
            None | Some(T::Eof) => return Err(format!("unterminated body of {name}")),
            _ => {}
        }
        r.pos += 1;
    }
    Ok(Function {
        span,
        name,
        parameters,
        body: start..r.pos,
    })
}

struct Module {
    variants: BTreeMap<String, Variant>,
    adapters: BTreeMap<String, Adapter>,
    functions: BTreeMap<String, Function>,
}

/// Checks the source frontend's retained token stream without re-lexing it.
pub fn check_tokens(tokens: &[Token]) -> Result<CheckedVariantProgram, VariantDiagnostic> {
    let tokens = match tokens.split_last() {
        Some((Token { kind: T::Eof, .. }, rest)) => rest,
        _ => tokens,
    };
    let locate = |error: VariantDiagnostic| {
        if error.span.start < error.span.end {
            return error;
        }
        VariantDiagnostic {
            span: tokens
                .iter()
                .filter(|t| t.span.start < t.span.end)
                .min_by_key(|t| t.span.start.abs_diff(error.span.start))
                .map_or(error.span, |t| t.span),
            ..error
        }
    };
    for token in tokens {
        let invalid = match &token.kind {
            T::Word(text) => {
                !text
                    .as_bytes()
                    .first()
                    .is_some_and(|b| b.is_ascii_alphabetic() || *b == b'_')
                    || !text.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
            }
            T::Integer(text) => text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()),
            T::Eof => true,
            _ => false,
        };
        if invalid {
            return Err(locate(VariantDiagnostic {
                span: token.span,
                message: "invalid lexical token".into(),
            }));
        }
    }
    let mut r = Reader { tokens, pos: 0 };
    let module = parse_module(&mut r)
        .map_err(|message| VariantDiagnostic {
            span: tokens
                .get(r.pos)
                .or_else(|| tokens.last())
                .map_or(Span { start: 0, end: 0 }, |t| t.span),
            message,
        })
        .map_err(locate)?;
    module.lower(tokens).map_err(locate)
}

fn parse_module(r: &mut Reader) -> Result<Module, String> {
    if r.eat_word("module") {
        r.word("library")?;
        r.symbol(S::Semicolon)?;
    }
    r.symbol(S::Hash)?;
    r.word("import")?;
    r.word("std")?;
    r.symbol(S::Dot)?;
    r.word("extern")?;
    r.symbol(S::Semicolon)?;
    let mut module = Module {
        variants: BTreeMap::new(),
        adapters: BTreeMap::new(),
        functions: BTreeMap::new(),
    };
    let mut names = BTreeSet::new();
    while !matches!(r.peek(), Some(T::Eof) | None) {
        let name = match r.peek() {
            Some(T::Word(w)) if w == "quant" => {
                let v = contract::parse_variant(r.tokens, &mut r.pos)?;
                let name = v.name.clone();
                module.variants.insert(name.clone(), v);
                name
            }
            Some(T::Word(w)) if w == "extern" => {
                let a = contract::parse_adapter(r.tokens, &mut r.pos, &module.variants)?;
                let name = a.name.clone();
                if !valid_external_name(&name) {
                    return Err(format!("unsupported external C identifier {name}"));
                }
                module.adapters.insert(name.clone(), a);
                name
            }
            Some(T::Word(w)) if w == "fn" => {
                let f = function(r)?;
                let name = f.name.clone();
                module.functions.insert(name.clone(), f);
                name
            }
            _ => return Err("unsupported declaration in quant variant library".into()),
        };
        if !names.insert(name.clone()) {
            return Err(format!("duplicate declaration {name}"));
        }
    }
    if module.adapters.is_empty() || !module.functions.values().any(Function::entry) {
        return Err("expected a mapped extern and an owned library entry".into());
    }
    Ok(module)
}

fn valid_external_name(name: &str) -> bool {
    // These names appear verbatim in the host ABI. All generated symbols use
    // qv_ and every local/source name is mangled. Reserve the C11 keywords,
    // main and the standard library identifiers introduced by the C prelude.
    name.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
        && !name.starts_with("qv_")
        && !matches!(
            name,
            "main"
                | "free"
                | "abort"
                | "aligned_alloc"
                | "at_quick_exit"
                | "atof"
                | "atoi"
                | "atol"
                | "atoll"
                | "strtod"
                | "strtof"
                | "strtold"
                | "strtol"
                | "strtoll"
                | "strtoul"
                | "strtoull"
                | "rand"
                | "srand"
                | "calloc"
                | "malloc"
                | "realloc"
                | "atexit"
                | "exit"
                | "getenv"
                | "quick_exit"
                | "system"
                | "bsearch"
                | "qsort"
                | "abs"
                | "labs"
                | "llabs"
                | "div"
                | "ldiv"
                | "lldiv"
                | "mblen"
                | "mbtowc"
                | "wctomb"
                | "mbstowcs"
                | "wcstombs"
                | "div_t"
                | "ldiv_t"
                | "lldiv_t"
                | "size_t"
                | "ptrdiff_t"
                | "max_align_t"
                | "wchar_t"
                | "auto"
                | "break"
                | "case"
                | "char"
                | "const"
                | "continue"
                | "default"
                | "do"
                | "double"
                | "else"
                | "enum"
                | "extern"
                | "float"
                | "for"
                | "goto"
                | "if"
                | "inline"
                | "int"
                | "long"
                | "register"
                | "restrict"
                | "return"
                | "short"
                | "signed"
                | "sizeof"
                | "static"
                | "struct"
                | "switch"
                | "typedef"
                | "union"
                | "unsigned"
                | "void"
                | "volatile"
                | "while"
        )
}

impl Module {
    fn lower(self, tokens: &[Token]) -> Result<CheckedVariantProgram, VariantDiagnostic> {
        let mut c = String::from(
            "#include <limits.h>\n#include <stdlib.h>\n_Static_assert(CHAR_BIT == 8 && sizeof(int) == 4 && INT_MAX == 2147483647 && INT_MIN == (-2147483647 - 1), \"unsupported quant variant ABI\");\n",
        );
        for v in self.variants.values() {
            writeln!(
                c,
                "struct qv_type_{} {{ unsigned tag; int code; }};",
                v.name
            )
            .unwrap();
        }
        for a in self.adapters.values() {
            writeln!(c, "extern int {}(unsigned char *);", a.name).unwrap();
        }
        let mut definitions = BTreeMap::new();
        for f in self.functions.values() {
            definitions.insert(f.name.clone(), body::lower_body(&self, f, tokens)?);
        }
        self.check_acyclic(&definitions)?;
        // Uncalled helpers are checked above but have no public C interface.
        let mut reachable = BTreeSet::new();
        let mut pending: Vec<_> = self
            .functions
            .values()
            .filter(|f| f.entry())
            .map(|f| f.name.clone())
            .collect();
        while let Some(name) = pending.pop() {
            if reachable.insert(name.clone()) {
                pending.extend(definitions[&name].1.iter().cloned());
            }
        }
        for name in &reachable {
            writeln!(c, "{};", self.functions[name].prototype()).unwrap();
        }
        for name in reachable {
            writeln!(
                c,
                "{} {{\n{}}}",
                self.functions[&name].prototype(),
                definitions[&name].0
            )
            .unwrap();
        }
        Ok(CheckedVariantProgram {
            c,
            adapters: self.adapters.into_values().collect(),
            functions: self.functions.into_keys().collect(),
        })
    }

    fn check_acyclic(
        &self,
        definitions: &BTreeMap<String, (String, Vec<String>)>,
    ) -> Result<(), VariantDiagnostic> {
        let mut done = BTreeSet::new();
        let mut active = BTreeSet::new();
        for root in self.functions.keys() {
            let mut stack = vec![(root, false)];
            while let Some((name, leaving)) = stack.pop() {
                if leaving {
                    active.remove(name);
                    done.insert(name);
                } else if !done.contains(name) {
                    if !active.insert(name) {
                        return Err(VariantDiagnostic {
                            span: self.functions[name].span,
                            message: format!("recursive quant variant call graph at {name}"),
                        });
                    }
                    stack.push((name, true));
                    stack.extend(definitions[name].1.iter().map(|target| (target, false)));
                }
            }
        }
        Ok(())
    }
}
