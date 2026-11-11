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

//! Bounded typed `quant effect` model declarations. The exit selector belongs
//! to the source clause. Parsing establishes no execution, atomicity, memory
//! ordering, synchronization, C binding, evidence or callable summary.
use crate::lexer::{Symbol as S, Token, TokenKind as T, lex};
use crate::quant_proposition;
use qcore_schemas::quant_effect::Definition;
use qcore_schemas::quant_proposition::{
    Definition as Proposition, Formula as F, Parameter, Sort, Term,
};

const MAX_BYTES: usize = 65536;
const MAX_NODES: usize = 256;
const MAX_DEPTH: usize = 32;
fn error() -> String {
    "quant-effect: malformed, unsupported or ill-typed relation".into()
}
struct Reader<'a> {
    source: &'a str,
    tokens: Vec<Token>,
    at: usize,
}
impl Reader<'_> {
    fn take(&mut self, kind: T) -> bool {
        if self.tokens.get(self.at).is_some_and(|t| t.kind == kind) {
            self.at += 1;
            true
        } else {
            false
        }
    }
    fn need(&mut self, kind: T) -> Result<(), String> {
        if self.take(kind) {
            Ok(())
        } else {
            Err(error())
        }
    }
    fn name(&mut self) -> Result<String, String> {
        self.need(T::Symbol(S::Hash))?;
        let Some(Token {
            kind: T::Word(name),
            ..
        }) = self.tokens.get(self.at)
        else {
            return Err(error());
        };
        let name = name.clone();
        self.at += 1;
        Ok(name)
    }
    fn text(&self, start: usize, end: usize) -> Result<&str, String> {
        if start >= end {
            return Err(error());
        }
        Ok(&self.source[self.tokens[start].span.start..self.tokens[end - 1].span.end])
    }
    // Contextual field delimiters are recognized only outside parentheses/braces,
    // and never as the name following `#` in a bound mathematical expression.
    fn field(&mut self, delimiter: T) -> Result<&str, String> {
        let start = self.at;
        let mut stack = Vec::new();
        while let Some(t) = self.tokens.get(self.at) {
            if stack.is_empty()
                && t.kind == delimiter
                && (self.at == start || self.tokens[self.at - 1].kind != T::Symbol(S::Hash))
            {
                let end = self.at;
                self.at += 1;
                return self.text(start, end);
            }
            match t.kind {
                T::Symbol(S::LeftParen) => stack.push(S::RightParen),
                T::Symbol(S::LeftBrace) => stack.push(S::RightBrace),
                T::Symbol(S::RightParen | S::RightBrace) => {
                    if stack.pop().is_none_or(|s| t.kind != T::Symbol(s)) {
                        return Err(error());
                    }
                }
                T::Symbol(S::Semicolon) => return Err(error()),
                _ => {}
            }
            if stack.len() > MAX_DEPTH {
                return Err(error());
            }
            self.at += 1;
        }
        Err(error())
    }
    fn footprints(&mut self, header: &str, fresh: &str) -> Result<Vec<Term>, String> {
        self.need(T::Symbol(S::LeftParen))?;
        let mut result = Vec::new();
        if self.take(T::Symbol(S::RightParen)) {
            return Ok(result);
        }
        loop {
            let start = self.at;
            let mut depth = 0usize;
            while let Some(t) = self.tokens.get(self.at) {
                match t.kind {
                    T::Symbol(S::RightParen | S::Comma) if depth == 0 => break,
                    T::Symbol(S::LeftParen) => depth += 1,
                    T::Symbol(S::RightParen) => depth -= 1,
                    T::Symbol(S::Semicolon | S::LeftBrace | S::RightBrace) => return Err(error()),
                    _ => {}
                }
                if depth > MAX_DEPTH {
                    return Err(error());
                }
                self.at += 1;
            }
            let source = self.text(start, self.at)?;
            let wrapped = quant_proposition::parse(&format!(
                "{header} = exists (#{fresh}: object) {{ {source} == #{fresh} }}"
            ))?;
            let F::Exists { body, .. } = wrapped.formula else {
                return Err(error());
            };
            let F::Equal { left, .. } = *body else {
                return Err(error());
            };
            if result.contains(&left) || result.len() >= 64 {
                return Err(error());
            }
            result.push(left);
            if self.take(T::Symbol(S::RightParen)) {
                break;
            }
            self.need(T::Symbol(S::Comma))?;
        }
        Ok(result)
    }
}
fn fresh_name(parameters: &[Parameter]) -> String {
    (0..=64)
        .map(|i| format!("quant_effect_object_{i}"))
        .find(|n| parameters.iter().all(|p| p.name != *n))
        .expect("at most64 parameters")
}
/// Read the expression after `quant effect SELECTOR`, without the semicolon.
pub fn parse(source: &str) -> Result<Definition, String> {
    if source.len() > MAX_BYTES {
        return Err(error());
    }
    let tokens = lex(source)
        .map_err(|_| error())?
        .into_iter()
        .filter(|t| t.kind != T::Eof)
        .collect();
    let mut reader = Reader {
        source,
        tokens,
        at: 0,
    };
    reader.name()?;
    reader.need(T::Symbol(S::LeftParen))?;
    // Parameter syntax cannot contain parentheses; its detailed types are
    // checked with the unchanged proposition header below.
    while reader
        .tokens
        .get(reader.at)
        .is_some_and(|t| t.kind != T::Symbol(S::RightParen))
    {
        reader.at += 1;
    }
    reader.need(T::Symbol(S::RightParen))?;
    let header = reader.text(0, reader.at)?.to_owned();
    reader.need(T::Word("from".into()))?;
    let entry = reader.name()?;
    reader.need(T::Word("to".into()))?;
    let exit = reader.name()?;
    reader.need(T::Word("when".into()))?;
    let guard_source = reader.field(T::Word("reads".into()))?.to_owned();
    let guard = quant_proposition::parse(&format!("{header} = {guard_source}"))?;
    let fresh = fresh_name(&guard.parameters);
    let reads = reader.footprints(&header, &fresh)?;
    reader.need(T::Word("writes".into()))?;
    let writes = reader.footprints(&header, &fresh)?;
    reader.need(T::Word("frame".into()))?;
    let frame_source = reader.field(T::Symbol(S::Equal))?.to_owned();
    let frame = quant_proposition::parse(&format!("{header} = {frame_source}"))?.formula;
    let transition_source = reader.text(reader.at, reader.tokens.len())?;
    let transition = quant_proposition::parse(&format!("{header} = {transition_source}"))?.formula;
    let definition = Definition {
        id: guard.id,
        parameters: guard.parameters,
        entry,
        exit,
        guard: guard.formula,
        reads,
        writes,
        frame,
        transition,
    };
    validate(&definition)?;
    Ok(definition)
}
fn count(depth: usize, nodes: &mut usize) -> Result<(), String> {
    *nodes += 1;
    if depth > MAX_DEPTH || *nodes > MAX_NODES {
        Err(error())
    } else {
        Ok(())
    }
}
fn term_nodes(term: &Term, depth: usize, nodes: &mut usize) -> Result<(), String> {
    count(depth, nodes)?;
    match term {
        Term::Apply { arguments, .. } => {
            for t in arguments {
                term_nodes(t, depth + 1, nodes)?;
            }
        }
        Term::Add { left, right } | Term::Sub { left, right } | Term::Mul { left, right } => {
            term_nodes(left, depth + 1, nodes)?;
            term_nodes(right, depth + 1, nodes)?;
        }
        Term::Neg { term } => term_nodes(term, depth + 1, nodes)?,
        Term::Variable { .. } | Term::Integer { .. } => {}
    }
    Ok(())
}
fn formula_nodes(formula: &F, depth: usize, nodes: &mut usize) -> Result<(), String> {
    count(depth, nodes)?;
    match formula {
        F::Predicate { arguments, .. } => {
            for t in arguments {
                term_nodes(t, depth + 1, nodes)?;
            }
        }
        F::Equal { left, right } | F::Compare { left, right, .. } => {
            term_nodes(left, depth + 1, nodes)?;
            term_nodes(right, depth + 1, nodes)?;
        }
        F::Not { body } | F::Forall { body, .. } | F::Exists { body, .. } => {
            formula_nodes(body, depth + 1, nodes)?
        }
        F::And { left, right } | F::Or { left, right } | F::Implies { left, right } => {
            formula_nodes(left, depth + 1, nodes)?;
            formula_nodes(right, depth + 1, nodes)?;
        }
    }
    Ok(())
}
/// Wire input receives the same type checks, footprint constraints and aggregate
/// budget. The existential wrapper checks `object` without adding a parameter.
pub fn validate(effect: &Definition) -> Result<(), String> {
    for state in [&effect.entry, &effect.exit] {
        if !effect.parameters.iter().any(|p| {
            p.name == *state
                && p.sort
                    == (Sort::Value {
                        name: "state".into(),
                    })
        }) {
            return Err(error());
        }
    }
    if effect.entry == effect.exit || effect.reads.len() > 64 || effect.writes.len() > 64 {
        return Err(error());
    }
    let mut proposition = Proposition {
        id: effect.id.clone(),
        parameters: effect.parameters.clone(),
        formula: effect.guard.clone(),
    };
    let mut nodes = 0;
    for formula in [&effect.guard, &effect.frame, &effect.transition] {
        proposition.formula = formula.clone();
        quant_proposition::validate(&proposition)?;
        formula_nodes(formula, 0, &mut nodes)?;
    }
    let fresh = fresh_name(&effect.parameters);
    for list in [&effect.reads, &effect.writes] {
        for (i, term) in list.iter().enumerate() {
            if list[..i].contains(term) {
                return Err(error());
            }
            term_nodes(term, 2, &mut nodes)?;
            proposition.formula = F::Exists {
                variable: fresh.clone(),
                sort: "object".into(),
                body: Box::new(F::Equal {
                    left: term.clone(),
                    right: Term::Variable {
                        name: fresh.clone(),
                    },
                }),
            };
            quant_proposition::validate(&proposition)?;
        }
    }
    Ok(())
}
