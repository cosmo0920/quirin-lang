// SPDX-License-Identifier: Apache-2.0

use super::contract::{Adapter, Reader};
use super::{Function, Module, Parameter, VariantDiagnostic};
use crate::lexer::{Span, Symbol as S, Token, TokenKind as T};
use qcore_schemas::relational_post::RelationOp;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

// Each pointer parameter introduces a distinct symbolic lifetime, instantiated
// once per call. Pointer aliases contain only that parameter's index. No
// allocation or address-to-generation inference is present in this profile.
type Generation = usize;

#[derive(Clone, PartialEq, Eq)]
enum Binding {
    Pointer(Generation),
    Package {
        variant: String,
        generation: Generation,
        local: String,
    },
    Code(String),
    Moved,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Custody {
    Held,
    Sealed,
    Disposed,
}

#[derive(Clone, Default, PartialEq, Eq)]
struct Frame {
    names: BTreeMap<String, Binding>,
    custody: BTreeMap<Generation, Custody>,
}

impl Frame {
    fn bind(&mut self, name: String, value: Binding) -> Result<(), String> {
        if self.names.contains_key(&name) {
            return Err(format!("duplicate binding {name}"));
        }
        self.names.insert(name, value);
        Ok(())
    }

    fn lookup(&self, name: &str) -> Result<Binding, String> {
        match self.names.get(name) {
            Some(Binding::Moved) => Err(format!("use after move or opening of {name}")),
            Some(v) => Ok(v.clone()),
            None => Err(format!("unknown binding {name}")),
        }
    }

    fn pointer(&self, name: &str) -> Result<Generation, String> {
        match self.lookup(name)? {
            Binding::Pointer(generation) => Ok(generation),
            _ => Err(format!("expected a resource pointer at {name}")),
        }
    }

    fn move_package(&mut self, name: &str) -> Result<Binding, String> {
        let binding = self.lookup(name)?;
        if !matches!(binding, Binding::Package { .. }) {
            return Err(format!("expected a sealed result at {name}"));
        }
        self.names.insert(name.into(), Binding::Moved);
        Ok(binding)
    }

    fn needs_custody(&self, generation: Generation) -> Result<(), String> {
        match self.custody.get(&generation) {
            Some(Custody::Held) => Ok(()),
            Some(Custody::Sealed) => Err("resource custody is sealed in a variant".into()),
            _ => Err("resource custody has been consumed or released".into()),
        }
    }

    fn exit(&self) -> Result<(), String> {
        if self.custody.values().any(|v| *v != Custody::Disposed) {
            Err("function exit loses live custody or an unopened result".into())
        } else {
            Ok(())
        }
    }
}

pub(super) fn lower_body(
    module: &Module,
    function: &Function,
    tokens: &[Token],
) -> Result<(String, Vec<String>), VariantDiagnostic> {
    let mut body = Body {
        module,
        r: Reader {
            tokens: &tokens[function.body.clone()],
            pos: 0,
        },
        c: String::new(),
        calls: Vec::new(),
        span: function.span,
    };
    body.check_function(function)
        .map_err(|message| VariantDiagnostic {
            span: body.span,
            message,
        })?;
    Ok((body.c, body.calls))
}

struct Body<'a> {
    module: &'a Module,
    r: Reader<'a>,
    c: String,
    calls: Vec<String>,
    span: Span,
}

impl Body<'_> {
    fn check_function(&mut self, function: &Function) -> Result<(), String> {
        let mut frame = Frame::default();
        if function.entry() {
            // Check the observable portion of the public ownership precondition
            // before anything can consume a pointer. Allocation provenance and
            // absence of outside owners remain obligations of the typed caller.
            let inputs: Vec<_> = (0..function.parameters.len())
                .map(|index| format!("qv_in_{index}"))
                .collect();
            writeln!(
                self.c,
                "  unsigned char *qv_roots[] = {{ {} }};",
                inputs.join(", ")
            )
            .unwrap();
            writeln!(self.c, "  for (size_t i = 0; i < {}; ++i) {{", inputs.len()).unwrap();
            self.c.push_str("    if (!qv_roots[i]) abort();\n    for (size_t j = 0; j < i; ++j) if (qv_roots[i] == qv_roots[j]) abort();\n  }\n");
        }
        for (idx, (name, ty)) in function.parameters.iter().enumerate() {
            match ty {
                Parameter::Owner => {
                    writeln!(self.c, "  unsigned char **qv_slot_{idx} = &qv_in_{idx};").unwrap();
                    frame.custody.insert(idx, Custody::Held);
                }
                Parameter::Subject => {}
                Parameter::Package { .. } => continue,
            }
            frame.bind(name.clone(), Binding::Pointer(idx))?;
        }
        for (idx, (name, ty)) in function.parameters.iter().enumerate() {
            if let Parameter::Package { variant, subject } = ty {
                if !self.module.variants.contains_key(variant) {
                    return Err(format!("unknown quant variant {variant}"));
                }
                let generation = frame.pointer(subject)?;
                if frame.custody.insert(generation, Custody::Sealed).is_some() {
                    return Err(format!("duplicate authority in parameters for {subject}"));
                }
                frame.bind(
                    name.clone(),
                    Binding::Package {
                        variant: variant.clone(),
                        generation,
                        local: format!("qv_arg_{idx}"),
                    },
                )?;
            }
        }
        if function
            .parameters
            .iter()
            .enumerate()
            .any(|(i, (_, ty))| matches!(ty, Parameter::Subject) && !frame.custody.contains_key(&i))
        {
            return Err("a raw subject parameter needs its matching sealed result".into());
        }
        if self.block(&mut frame, 0)? {
            return Err(format!("function {} may fall through", function.name));
        }
        Ok(())
    }

    // Returns whether control continues after the closing brace.
    fn block(&mut self, frame: &mut Frame, depth: usize) -> Result<bool, String> {
        if depth > 32 {
            return Err("quant variant match nesting exceeds 32".into());
        }
        let mut live = true;
        while !self.r.eat_symbol(S::RightBrace) {
            if let Some(token) = self.r.tokens.get(self.r.pos) {
                self.span = token.span;
            }
            if !live {
                return Err("unreachable statement after function exit".into());
            }
            if self.r.eat_word("let") {
                self.binding(frame)?;
            } else if self.r.eat_word("match") {
                live = self.open(frame, depth)?;
            } else if self.r.eat_word("drop") {
                let name = self.r.ident()?;
                let generation = frame.pointer(&name)?;
                frame.needs_custody(generation)?;
                self.r.symbol(S::Semicolon)?;
                frame.custody.insert(generation, Custody::Disposed);
                writeln!(self.c, "  free(*qv_slot_{generation});").unwrap();
            } else if self.r.eat_word("return") {
                let value = self.scalar(frame)?;
                self.r.symbol(S::Semicolon)?;
                frame.exit()?;
                writeln!(self.c, "  return {value};").unwrap();
                live = false;
            } else {
                return Err(format!(
                    "unsupported quant variant statement at {:?}",
                    self.r.peek()
                ));
            }
        }
        Ok(live)
    }

    fn binding(&mut self, frame: &mut Frame) -> Result<(), String> {
        let name = self.r.ident()?;
        self.r.symbol(S::Equal)?;
        let value = if self.r.eat_word("move") {
            let src = self.r.ident()?;
            frame.move_package(&src)?
        } else if let Some(T::Word(func)) = self.r.peek()
            && self.module.adapters.contains_key(func)
            && self
                .r
                .tokens
                .get(self.r.pos + 1)
                .is_some_and(|t| t.kind == T::Symbol(S::LeftParen))
        {
            let func = self.r.ident()?;
            self.pack(frame, &self.module.adapters[&func])?
        } else if matches!(self.r.peek(), Some(T::Word(_)))
            && !self
                .r
                .tokens
                .get(self.r.pos + 1)
                .is_some_and(|t| t.kind == T::Symbol(S::LeftParen))
        {
            let src = self.r.ident()?;
            match frame.lookup(&src)? {
                Binding::Package { .. } => {
                    return Err(format!("copy of sealed result {src} requires move"));
                }
                value => value,
            }
        } else {
            let local = format!("qv_value_{}", self.r.pos);
            let expr = self.scalar(frame)?;
            writeln!(self.c, "  int {local} = {expr}; (void){local};").unwrap();
            Binding::Code(local)
        };
        self.r.symbol(S::Semicolon)?;
        frame.bind(name, value)
    }

    fn pack(&mut self, frame: &mut Frame, adapter: &Adapter) -> Result<Binding, String> {
        let site = self.r.pos;
        self.r.symbol(S::LeftParen)?;
        let subject = self.r.ident()?;
        self.r.symbol(S::RightParen)?;
        let generation = frame.pointer(&subject)?;
        frame.needs_custody(generation)?;
        frame.custody.insert(generation, Custody::Sealed);
        let local = format!("qv_package_{site}");
        let code = format!("qv_code_{site}");
        writeln!(
            self.c,
            "  int {code} = {}(*qv_slot_{generation});",
            adapter.name
        )
        .unwrap();
        writeln!(self.c, "  struct qv_type_{} {local};", adapter.variant).unwrap();
        let variant = &self.module.variants[&adapter.variant];
        for (i, (class, tag)) in adapter.classes.iter().enumerate() {
            let op = match class.op {
                RelationOp::Eq => "==",
                RelationOp::Ne => "!=",
                RelationOp::Gt => ">",
                RelationOp::Lt => "<",
                RelationOp::Ge => ">=",
                RelationOp::Le => "<=",
            };
            let otherwise = if i == 0 { "" } else { "else " };
            let data = if variant.arms[*tag].code {
                code.as_str()
            } else {
                "0"
            };
            writeln!(self.c, "  {otherwise}if ({code} {op} {}LL) {{ {local}.tag = {tag}u; {local}.code = {data}; }}", class.constant).unwrap();
        }
        self.c.push_str("  else { abort(); }\n");
        Ok(Binding::Package {
            variant: adapter.variant.clone(),
            generation,
            local,
        })
    }

    fn scalar(&mut self, frame: &mut Frame) -> Result<String, String> {
        let negative = self.r.eat_symbol(S::Minus);
        if let Some(T::Integer(text)) = self.r.peek() {
            let value = text.parse::<i64>().map_err(|_| "expected an i32 literal")?;
            let value = i32::try_from(if negative { -value } else { value })
                .map_err(|_| "literal exceeds i32")?;
            self.r.pos += 1;
            return Ok(value.to_string());
        }
        if negative {
            return Err("expected decimal digits after minus".into());
        }
        let name = self.r.ident()?;
        if self.r.eat_symbol(S::LeftParen) {
            let callee = self
                .module
                .functions
                .get(&name)
                .ok_or_else(|| format!("unknown scalar function {name}"))?;
            let mut inputs = BTreeMap::new();
            let mut indices = BTreeSet::new();
            let mut packages = Vec::new();
            let mut args = Vec::new();
            for (i, (formal, ty)) in callee.parameters.iter().enumerate() {
                if i > 0 {
                    self.r.symbol(S::Comma)?;
                }
                match ty {
                    Parameter::Owner | Parameter::Subject => {
                        if matches!(ty, Parameter::Owner) {
                            self.r.word("move")?;
                        }
                        let arg = self.r.ident()?;
                        let id = frame.pointer(&arg)?;
                        if !indices.insert(id) {
                            return Err("distinct resource parameters alias".into());
                        }
                        let actual = if matches!(ty, Parameter::Owner) {
                            frame.needs_custody(id)?;
                            format!("*qv_slot_{id}")
                        } else {
                            format!("qv_slot_{id}")
                        };
                        inputs.insert(formal.clone(), id);
                        args.push(actual);
                    }
                    Parameter::Package { variant, subject } => {
                        self.r.word("move")?;
                        let arg = self.r.ident()?;
                        let Binding::Package {
                            variant: actual,
                            generation,
                            local,
                        } = frame.move_package(&arg)?
                        else {
                            unreachable!()
                        };
                        if &actual != variant {
                            return Err(format!("wrong variant type for {formal}"));
                        }
                        packages.push((subject, generation));
                        args.push(local);
                    }
                }
            }
            self.r.symbol(S::RightParen)?;
            for (subject, generation) in packages {
                if inputs.get(subject) != Some(&generation) {
                    return Err("sealed result belongs to a different generation".into());
                }
            }
            for id in indices {
                frame.custody.insert(id, Custody::Disposed);
            }
            self.calls.push(name);
            Ok(format!("{}({})", callee.c_name(), args.join(", ")))
        } else if let Binding::Code(local) = frame.lookup(&name)? {
            Ok(local)
        } else {
            Err(format!("expected an i32 value at {name}"))
        }
    }

    fn open(&mut self, frame: &mut Frame, depth: usize) -> Result<bool, String> {
        self.r.eat_word("move");
        let name = self.r.ident()?;
        let Binding::Package {
            variant,
            generation,
            local,
        } = frame.move_package(&name)?
        else {
            unreachable!()
        };
        let declaration = &self.module.variants[&variant];
        self.r.symbol(S::LeftBrace)?;
        writeln!(self.c, "  switch ({local}.tag) {{").unwrap();
        let mut seen = BTreeSet::new();
        let mut join = None;
        while !self.r.eat_symbol(S::RightBrace) {
            self.r.word(&variant)?;
            self.r.symbol(S::ColonColon)?;
            let tag_name = self.r.ident()?;
            let tag = declaration
                .arms
                .iter()
                .position(|a| a.name == tag_name)
                .ok_or_else(|| format!("unknown tag {tag_name}"))?;
            if !seen.insert(tag) {
                return Err(format!("duplicate match arm {tag_name}"));
            }
            let arm = &declaration.arms[tag];
            let mut edge = frame.clone();
            edge.custody.insert(
                generation,
                if arm.custody {
                    Custody::Held
                } else {
                    Custody::Disposed
                },
            );
            if arm.code {
                self.r.symbol(S::LeftBrace)?;
                self.r.word("code")?;
                self.r.symbol(S::Colon)?;
                let binder = self.r.ident()?;
                self.r.symbol(S::RightBrace)?;
                edge.bind(binder, Binding::Code(format!("{local}.code")))?;
            }
            self.r.symbol(S::FatArrow)?;
            self.r.symbol(S::LeftBrace)?;
            writeln!(self.c, "    case {tag}u: {{").unwrap();
            if self.block(&mut edge, depth + 1)? {
                // A local alias or scalar goes out of scope; a sealed result
                // still demands a consumer, including on this branch edge.
                for (local_name, val) in &edge.names {
                    if !frame.names.contains_key(local_name)
                        && matches!(val, Binding::Package { .. })
                    {
                        return Err(format!("match arm loses sealed result {local_name}"));
                    }
                }
                edge.names.retain(|key, _| frame.names.contains_key(key));
                if join.as_ref().is_some_and(|other| other != &edge) {
                    return Err("incompatible custody states after match".into());
                }
                join = Some(edge);
                self.c.push_str("      break;\n");
            }
            self.c.push_str("    }\n");
        }
        if seen.len() != declaration.arms.len() {
            return Err("non-exhaustive quant variant match".into());
        }
        self.c.push_str("    default: abort();\n  }\n");
        if let Some(join) = join {
            *frame = join;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}
