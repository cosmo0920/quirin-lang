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

//! Bounded theorem application reader. Theorem names never grant truth.
use crate::lexer::{Symbol as S, TokenKind as T, lex};
use crate::quant_proposition;
use qcore_schemas::quant_proposition::{Definition, Formula, Term};
use qcore_schemas::quant_theorem::Application;
fn error() -> String {
    "quant-theorem: malformed or ill-typed application".into()
}
pub fn valid_reference(s: &str) -> bool {
    s.len() <= 1024
        && s.split('.').all(|p| {
            !p.is_empty()
                && p.len() <= 128
                && p.bytes()
                    .next()
                    .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
                && p.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
        })
}
/// Source after `quant theorem`, excluding the final semicolon.
pub fn parse(source: &str) -> Result<Application, String> {
    if source.len() > 65536 {
        return Err(error());
    }
    let tokens = lex(source).map_err(|_| error())?;
    let mut stack = Vec::new();
    let mut using = None;
    for (i, t) in tokens.iter().enumerate() {
        if using.is_none()
            && stack.is_empty()
            && t.kind == T::Word("using".into())
            && (i == 0 || tokens[i - 1].kind != T::Symbol(S::Hash))
        {
            using = Some(i);
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
        if stack.len() > 32 {
            return Err(error());
        }
    }
    if !stack.is_empty() {
        return Err(error());
    }
    let u = using.ok_or_else(error)?;
    let header = &source[..tokens[u].span.start];
    let definition = quant_proposition::parse(&format!("{header}= 0 == 0"))?;
    let mut i = u + 1;
    let mut reference = String::new();
    loop {
        let Some(T::Word(w)) = tokens.get(i).map(|t| &t.kind) else {
            return Err(error());
        };
        reference.push_str(w);
        i += 1;
        if tokens.get(i).is_some_and(|t| t.kind == T::Symbol(S::Dot)) {
            reference.push('.');
            i += 1;
        } else {
            break;
        }
    }
    if !valid_reference(&reference)
        || tokens
            .get(i)
            .is_none_or(|t| t.kind != T::Symbol(S::LeftParen))
    {
        return Err(error());
    }
    i += 1;
    let mut arguments = Vec::new();
    if tokens
        .get(i)
        .is_some_and(|t| t.kind != T::Symbol(S::RightParen))
    {
        loop {
            if tokens.get(i).is_none_or(|t| t.kind != T::Symbol(S::Hash)) {
                return Err(error());
            }
            let Some(T::Word(w)) = tokens.get(i + 1).map(|t| &t.kind) else {
                return Err(error());
            };
            arguments.push(w.clone());
            i += 2;
            if arguments.len() > 64 {
                return Err(error());
            }
            if tokens
                .get(i)
                .is_some_and(|t| t.kind == T::Symbol(S::RightParen))
            {
                break;
            }
            if tokens.get(i).is_none_or(|t| t.kind != T::Symbol(S::Comma)) {
                return Err(error());
            }
            i += 1;
        }
    }
    if tokens
        .get(i)
        .is_none_or(|t| t.kind != T::Symbol(S::RightParen))
    {
        return Err(error());
    }
    let r = i + 1;
    if tokens
        .get(r)
        .is_none_or(|t| t.kind != T::Word("requires".into()))
        || tokens
            .get(r + 1)
            .is_none_or(|t| t.kind != T::Symbol(S::LeftParen))
    {
        return Err(error());
    }
    let mut nesting = 1usize;
    let mut closing = r + 2;
    while nesting > 0 {
        let t = tokens.get(closing).ok_or_else(error)?;
        match t.kind {
            T::Symbol(S::LeftParen) => nesting += 1,
            T::Symbol(S::RightParen) => nesting -= 1,
            T::Eof => return Err(error()),
            _ => {}
        }
        closing += 1;
    }
    let y = closing;
    if tokens
        .get(y)
        .is_none_or(|t| t.kind != T::Word("yields".into()))
    {
        return Err(error());
    }
    let mut prerequisites = Vec::new();
    let mut start = r + 2;
    let mut depth = 0usize;
    for j in r + 2..y - 1 {
        match tokens[j].kind {
            T::Symbol(S::LeftParen | S::LeftBrace) => depth += 1,
            T::Symbol(S::RightParen | S::RightBrace) => {
                depth = depth.checked_sub(1).ok_or_else(error)?
            }
            T::Symbol(S::Comma) if depth == 0 => {
                if start == j {
                    return Err(error());
                }
                prerequisites.push(
                    quant_proposition::parse(&format!(
                        "{header}= {}",
                        &source[tokens[start].span.start..tokens[j].span.start]
                    ))?
                    .formula,
                );
                start = j + 1;
            }
            _ => {}
        }
    }
    if start < y - 1 {
        prerequisites.push(
            quant_proposition::parse(&format!(
                "{header}= {}",
                &source[tokens[start].span.start..tokens[y - 1].span.start]
            ))?
            .formula,
        );
    } else if start != r + 2 {
        return Err(error());
    }
    let conclusion =
        quant_proposition::parse(&format!("{header}= {}", &source[tokens[y].span.end..]))?.formula;
    let app = Application {
        id: definition.id,
        parameters: definition.parameters,
        reference,
        arguments,
        prerequisites,
        conclusion,
    };
    validate(&app)?;
    Ok(app)
}
fn bounded_nodes(f: &Formula) -> Result<usize, String> {
    enum Node<'a> {
        Formula(&'a Formula),
        Term(&'a Term),
    }
    let mut pending = vec![(Node::Formula(f), 0)];
    let mut nodes = 0;
    while let Some((node, depth)) = pending.pop() {
        nodes += 1;
        if nodes > 256 || depth > 32 {
            return Err(error());
        }
        match node {
            Node::Formula(Formula::Predicate { arguments, .. })
            | Node::Term(Term::Apply { arguments, .. }) => {
                if arguments.len() > 16 {
                    return Err(error());
                }
                for t in arguments {
                    pending.push((Node::Term(t), depth + 1));
                }
            }
            Node::Formula(
                Formula::Equal { left, right } | Formula::Compare { left, right, .. },
            ) => {
                pending.push((Node::Term(left), depth + 1));
                pending.push((Node::Term(right), depth + 1));
            }
            Node::Term(
                Term::Add { left, right } | Term::Sub { left, right } | Term::Mul { left, right },
            ) => {
                pending.push((Node::Term(left), depth + 1));
                pending.push((Node::Term(right), depth + 1));
            }
            Node::Formula(
                Formula::And { left, right }
                | Formula::Or { left, right }
                | Formula::Implies { left, right },
            ) => {
                pending.push((Node::Formula(left), depth + 1));
                pending.push((Node::Formula(right), depth + 1));
            }
            Node::Formula(
                Formula::Not { body } | Formula::Forall { body, .. } | Formula::Exists { body, .. },
            ) => pending.push((Node::Formula(body), depth + 1)),
            Node::Term(Term::Neg { term }) => pending.push((Node::Term(term), depth + 1)),
            Node::Term(Term::Integer { .. } | Term::Variable { .. }) => {}
        }
    }
    Ok(nodes)
}
/// Recheck all formula branches before cloning bounded syntax for proposition typing.
pub fn validate_formulas(
    id: &str,
    parameters: &[qcore_schemas::quant_proposition::Parameter],
    prerequisites: &[Formula],
    conclusion: &Formula,
) -> Result<(), String> {
    if parameters.len() > 64 || prerequisites.len() > 32 {
        return Err(error());
    }
    let mut nodes = 0;
    for formula in prerequisites.iter().chain(std::iter::once(conclusion)) {
        nodes += bounded_nodes(formula)?;
        if nodes > 256 {
            return Err(error());
        }
        quant_proposition::validate(&Definition {
            id: id.into(),
            parameters: parameters.to_vec(),
            formula: formula.clone(),
        })?;
    }
    Ok(())
}
pub fn validate(app: &Application) -> Result<(), String> {
    if !valid_reference(&app.reference) || app.arguments.len() > 64 {
        return Err(error());
    }
    validate_formulas(
        &app.id,
        &app.parameters,
        &app.prerequisites,
        &app.conclusion,
    )?;
    if app
        .arguments
        .iter()
        .any(|a| !app.parameters.iter().any(|p| p.name == *a))
    {
        return Err(error());
    }
    Ok(())
}
