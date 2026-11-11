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

//! Bounded first-order proposition reader. Relation and function interpretations
//! are explicit inputs, never source facts. Integers are mathematical, not C values.
use crate::lexer::{Symbol as S, TokenKind as T, lex};
use qcore_schemas::quant_proposition::{
    Comparison, Definition, Formula as F, Parameter, Sort, Term,
};
use std::collections::BTreeMap;

const MAX_BYTES: usize = 65536;
const MAX_NODES: usize = 256;
const MAX_DEPTH: usize = 32;
// Backtracking never restores this fuel, so malformed ambiguous prefixes stay bounded.
const MAX_STEPS: usize = 131072;
fn error() -> String {
    "quant-proposition: malformed, unsupported or ill-typed formula".into()
}
fn identifier(s: &str) -> bool {
    s.len() <= 128
        && s.bytes().next().is_some_and(|c| c.is_ascii_lowercase())
        && s.bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_')
}
fn value_sort(s: &str) -> bool {
    identifier(s) && !matches!(s, "predicate" | "function")
}
fn decimal(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes().all(|c| c.is_ascii_digit())
        && (s == "0" || !s.starts_with('0'))
}
struct Reader {
    tokens: Vec<T>,
    at: usize,
    nodes: usize,
    steps: usize,
}
impl Reader {
    fn node(&mut self, depth: usize) -> Result<(), String> {
        self.nodes += 1;
        self.steps += 1;
        if depth > MAX_DEPTH || self.nodes > MAX_NODES || self.steps > MAX_STEPS {
            Err(error())
        } else {
            Ok(())
        }
    }
    fn take(&mut self, token: T) -> bool {
        self.steps += 1;
        if self.steps <= MAX_STEPS && self.tokens.get(self.at) == Some(&token) {
            self.at += 1;
            true
        } else {
            false
        }
    }
    fn need(&mut self, token: T) -> Result<(), String> {
        if self.take(token) {
            Ok(())
        } else {
            Err(error())
        }
    }
    fn word(&mut self) -> Result<String, String> {
        self.steps += 1;
        if self.steps <= MAX_STEPS
            && let Some(T::Word(w)) = self.tokens.get(self.at)
            && identifier(w)
        {
            self.at += 1;
            Ok(w.clone())
        } else {
            Err(error())
        }
    }
    fn variable(&mut self) -> Result<String, String> {
        self.need(T::Symbol(S::Hash))?;
        self.word()
    }
    fn sort(&mut self) -> Result<Sort, String> {
        let name = self.word()?;
        if !matches!(name.as_str(), "predicate" | "function") {
            return Ok(Sort::Value { name });
        }
        self.need(T::Symbol(S::Less))?;
        let mut arguments = Vec::new();
        if !self.take(T::Symbol(S::Greater)) {
            loop {
                arguments.push(self.word()?);
                if arguments.len() > 17 {
                    return Err(error());
                }
                if self.take(T::Symbol(S::Greater)) {
                    break;
                }
                self.need(T::Symbol(S::Comma))?;
            }
        }
        if name == "function" {
            let result = arguments.pop().ok_or_else(error)?;
            Ok(Sort::Function { arguments, result })
        } else if arguments.len() <= 16 {
            Ok(Sort::Predicate { arguments })
        } else {
            Err(error())
        }
    }
    fn arguments(&mut self, depth: usize) -> Result<Vec<Term>, String> {
        let mut arguments = Vec::new();
        if !self.take(T::Symbol(S::RightParen)) {
            loop {
                arguments.push(self.term(depth, 0)?);
                if arguments.len() > 16 {
                    return Err(error());
                }
                if self.take(T::Symbol(S::RightParen)) {
                    break;
                }
                self.need(T::Symbol(S::Comma))?;
            }
        }
        Ok(arguments)
    }
    // Precedence climbing: multiplication binds more tightly than addition/subtraction.
    fn term(&mut self, depth: usize, minimum: u8) -> Result<Term, String> {
        self.node(depth)?;
        let mut left = if self.take(T::Symbol(S::Minus)) {
            Term::Neg {
                term: Box::new(self.term(depth + 1, 3)?),
            }
        } else if self.take(T::Symbol(S::LeftParen)) {
            let term = self.term(depth + 1, 0)?;
            self.need(T::Symbol(S::RightParen))?;
            self.nodes -= 1; // grouping is not an AST node
            term
        } else if let Some(T::Integer(value)) = self.tokens.get(self.at) {
            let value = value.clone();
            if !decimal(&value) {
                return Err(error());
            }
            self.at += 1;
            Term::Integer { decimal: value }
        } else {
            let name = self.variable()?;
            if self.take(T::Symbol(S::LeftParen)) {
                Term::Apply {
                    function: name,
                    arguments: self.arguments(depth + 1)?,
                }
            } else {
                Term::Variable { name }
            }
        };
        loop {
            let op = self.tokens.get(self.at).cloned();
            let precedence = match op {
                Some(T::Symbol(S::Plus | S::Minus)) => 1,
                Some(T::Symbol(S::Star)) => 2,
                _ => break,
            };
            if precedence < minimum {
                break;
            }
            self.at += 1;
            self.node(depth)?;
            let right = Box::new(self.term(depth + 1, precedence + 1)?);
            let boxed = Box::new(left);
            left = match op {
                Some(T::Symbol(S::Plus)) => Term::Add { left: boxed, right },
                Some(T::Symbol(S::Minus)) => Term::Sub { left: boxed, right },
                _ => Term::Mul { left: boxed, right },
            };
        }
        Ok(left)
    }
    fn atom(&mut self, depth: usize) -> Result<F, String> {
        let left = self.term(depth + 1, 0)?;
        let op = self.tokens.get(self.at).cloned();
        if matches!(
            op,
            Some(T::Symbol(
                S::EqualEqual | S::NotEqual | S::Less | S::LessEqual | S::Greater | S::GreaterEqual
            ))
        ) {
            self.at += 1;
            let right = self.term(depth + 1, 0)?;
            return Ok(match op {
                Some(T::Symbol(S::EqualEqual)) => F::Equal { left, right },
                Some(T::Symbol(S::NotEqual)) => {
                    self.node(depth + 1)?;
                    F::Not {
                        body: Box::new(F::Equal { left, right }),
                    }
                }
                _ => F::Compare {
                    operator: match op {
                        Some(T::Symbol(S::Less)) => Comparison::Less,
                        Some(T::Symbol(S::LessEqual)) => Comparison::LessEqual,
                        Some(T::Symbol(S::Greater)) => Comparison::Greater,
                        _ => Comparison::GreaterEqual,
                    },
                    left,
                    right,
                },
            });
        }
        if let Term::Apply {
            function: name,
            arguments,
        } = left
        {
            self.nodes -= 1; // the application is the predicate formula itself
            Ok(F::Predicate { name, arguments })
        } else {
            Err(error())
        }
    }
    fn formula(&mut self, depth: usize) -> Result<F, String> {
        self.node(depth)?;
        if self.take(T::Symbol(S::Bang)) {
            return Ok(F::Not {
                body: Box::new(self.formula(depth + 1)?),
            });
        }
        for universal in [true, false] {
            if self.take(T::Word(if universal { "forall" } else { "exists" }.into())) {
                self.need(T::Symbol(S::LeftParen))?;
                let variable = self.variable()?;
                self.need(T::Symbol(S::Colon))?;
                let sort = self.word()?;
                if !value_sort(&sort) {
                    return Err(error());
                }
                self.need(T::Symbol(S::RightParen))?;
                self.need(T::Symbol(S::LeftBrace))?;
                let body = Box::new(self.formula(depth + 1)?);
                self.need(T::Symbol(S::RightBrace))?;
                return Ok(if universal {
                    F::Forall {
                        variable,
                        sort,
                        body,
                    }
                } else {
                    F::Exists {
                        variable,
                        sort,
                        body,
                    }
                });
            }
        }
        let checkpoint = (self.at, self.nodes);
        if let Ok(atom) = self.atom(depth) {
            return Ok(atom);
        }
        // Only a parenthesized Boolean formula can follow a failed term attempt.
        // Restore AST accounting, but never the work fuel.
        (self.at, self.nodes) = checkpoint;
        self.need(T::Symbol(S::LeftParen))?;
        let left = Box::new(self.formula(depth + 1)?);
        if self.take(T::Symbol(S::RightParen)) {
            // Grouping contributes parser work/depth but no additional AST node.
            self.nodes -= 1;
            return Ok(*left);
        }
        let op = self.tokens.get(self.at).cloned().ok_or_else(error)?;
        if !matches!(op, T::Symbol(S::AndAnd | S::PipePipe | S::FatArrow)) {
            return Err(error());
        }
        self.at += 1;
        let right = Box::new(self.formula(depth + 1)?);
        self.need(T::Symbol(S::RightParen))?;
        Ok(match op {
            T::Symbol(S::AndAnd) => F::And { left, right },
            T::Symbol(S::PipePipe) => F::Or { left, right },
            _ => F::Implies { left, right },
        })
    }
}
/// The expression after `quant proposition`, without its final semicolon.
pub fn parse(source: &str) -> Result<Definition, String> {
    if source.len() > MAX_BYTES {
        return Err(error());
    }
    let tokens = lex(source)
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|t| t.kind)
        .filter(|t| *t != T::Eof)
        .collect();
    let mut r = Reader {
        tokens,
        at: 0,
        nodes: 0,
        steps: 0,
    };
    let id = r.variable()?;
    r.need(T::Symbol(S::LeftParen))?;
    let mut parameters = Vec::new();
    if !r.take(T::Symbol(S::RightParen)) {
        loop {
            let name = r.variable()?;
            r.need(T::Symbol(S::Colon))?;
            parameters.push(Parameter {
                name,
                sort: r.sort()?,
            });
            if parameters.len() > 64 {
                return Err(error());
            }
            if r.take(T::Symbol(S::RightParen)) {
                break;
            }
            r.need(T::Symbol(S::Comma))?;
        }
    }
    r.need(T::Symbol(S::Equal))?;
    let formula = r.formula(0)?;
    if r.at != r.tokens.len() {
        return Err(error());
    }
    let definition = Definition {
        id,
        parameters,
        formula,
    };
    validate(&definition)?;
    Ok(definition)
}
fn node(depth: usize, nodes: &mut usize) -> Result<(), String> {
    *nodes += 1;
    if depth > MAX_DEPTH || *nodes > MAX_NODES {
        Err(error())
    } else {
        Ok(())
    }
}
fn term_sort(
    term: &Term,
    scope: &BTreeMap<String, Sort>,
    depth: usize,
    nodes: &mut usize,
) -> Result<String, String> {
    node(depth, nodes)?;
    match term {
        Term::Variable { name } => match scope.get(name) {
            Some(Sort::Value { name: sort }) => Ok(sort.clone()),
            _ => Err(error()),
        },
        Term::Integer { decimal: value } => {
            if decimal(value) {
                Ok("integer".into())
            } else {
                Err(error())
            }
        }
        Term::Apply {
            function,
            arguments,
        } => {
            let Some(Sort::Function {
                arguments: sorts,
                result,
            }) = scope.get(function)
            else {
                return Err(error());
            };
            check_arguments(arguments, sorts, scope, depth + 1, nodes)?;
            Ok(result.clone())
        }
        Term::Add { left, right } | Term::Sub { left, right } | Term::Mul { left, right } => {
            if term_sort(left, scope, depth + 1, nodes)? != "integer"
                || term_sort(right, scope, depth + 1, nodes)? != "integer"
            {
                return Err(error());
            }
            Ok("integer".into())
        }
        Term::Neg { term } => {
            if term_sort(term, scope, depth + 1, nodes)? == "integer" {
                Ok("integer".into())
            } else {
                Err(error())
            }
        }
    }
}
fn check_arguments(
    arguments: &[Term],
    sorts: &[String],
    scope: &BTreeMap<String, Sort>,
    depth: usize,
    nodes: &mut usize,
) -> Result<(), String> {
    if arguments.len() != sorts.len() || arguments.len() > 16 {
        return Err(error());
    }
    for (argument, sort) in arguments.iter().zip(sorts) {
        if term_sort(argument, scope, depth, nodes)? != *sort {
            return Err(error());
        }
    }
    Ok(())
}
/// Recheck source and wire inputs with the same shared formula/term budget.
pub fn validate(definition: &Definition) -> Result<(), String> {
    let mut scope = BTreeMap::new();
    if !identifier(&definition.id)
        || definition.id.starts_with("legacy_")
        || definition.parameters.len() > 64
    {
        return Err(error());
    }
    for p in &definition.parameters {
        let good = match &p.sort {
            Sort::Value { name } => value_sort(name),
            Sort::Predicate { arguments } => {
                arguments.len() <= 16 && arguments.iter().all(|s| value_sort(s))
            }
            Sort::Function { arguments, result } => {
                arguments.len() <= 16
                    && arguments.iter().all(|s| value_sort(s))
                    && value_sort(result)
            }
        };
        if !identifier(&p.name) || !good || scope.insert(p.name.clone(), p.sort.clone()).is_some() {
            return Err(error());
        }
    }
    fn check(
        f: &F,
        scope: &mut BTreeMap<String, Sort>,
        depth: usize,
        nodes: &mut usize,
    ) -> Result<(), String> {
        node(depth, nodes)?;
        match f {
            F::Predicate { name, arguments } => {
                let Some(Sort::Predicate { arguments: sorts }) = scope.get(name) else {
                    return Err(error());
                };
                check_arguments(arguments, sorts, scope, depth + 1, nodes)?;
            }
            F::Equal { left, right } => {
                if term_sort(left, scope, depth + 1, nodes)?
                    != term_sort(right, scope, depth + 1, nodes)?
                {
                    return Err(error());
                }
            }
            F::Compare { left, right, .. } => {
                if term_sort(left, scope, depth + 1, nodes)? != "integer"
                    || term_sort(right, scope, depth + 1, nodes)? != "integer"
                {
                    return Err(error());
                }
            }
            F::Not { body } => check(body, scope, depth + 1, nodes)?,
            F::And { left, right } | F::Or { left, right } | F::Implies { left, right } => {
                check(left, scope, depth + 1, nodes)?;
                check(right, scope, depth + 1, nodes)?;
            }
            F::Forall {
                variable,
                sort,
                body,
            }
            | F::Exists {
                variable,
                sort,
                body,
            } => {
                if !identifier(variable) || !value_sort(sort) || scope.contains_key(variable) {
                    return Err(error());
                }
                scope.insert(variable.clone(), Sort::Value { name: sort.clone() });
                check(body, scope, depth + 1, nodes)?;
                scope.remove(variable);
            }
        }
        Ok(())
    }
    check(&definition.formula, &mut scope, 0, &mut 0)
}
