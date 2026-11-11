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

//! Typed symbolic lifecycle syntax. No operation is executed by this module.
use crate::lexer::{Symbol, TokenKind, lex};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    Create,
    Retire,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    Resource(String),
    Hypothesis(String),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub operation: Operation,
    pub target: Target,
}

pub fn is_clause(source: &str) -> bool {
    source
        .trim_start()
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .next()
        .is_some_and(|head| head.starts_with("resource_lifecycle_v"))
}

/// IDs are literal declaration-local metadata, never C places or formals.
pub fn parse_clause(source: &str) -> Result<Option<(String, Entry)>, String> {
    if !is_clause(source) {
        return Ok(None);
    }
    if source.len() > 4096 {
        return Err("lifecycle: clause byte limit".into());
    }
    let tokens = lex(source)
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|t| t.kind)
        .collect::<Vec<_>>();
    let [
        TokenKind::Word(head),
        TokenKind::Symbol(Symbol::LeftParen),
        TokenKind::Word(id),
        TokenKind::Symbol(Symbol::Comma),
        TokenKind::Symbol(op),
        TokenKind::Word(kind),
        TokenKind::Symbol(Symbol::LeftParen),
        TokenKind::Word(target),
        TokenKind::Symbol(Symbol::RightParen),
        TokenKind::Symbol(Symbol::RightParen),
        TokenKind::Eof,
    ] = tokens.as_slice()
    else {
        return Err("lifecycle: expected resource_lifecycle_v1(ID, |^+ or |^- resource(ID) or hypothesis(ID))".into());
    };
    if head != "resource_lifecycle_v1" {
        return Err("lifecycle: unsupported source version".into());
    }
    if [id, target]
        .iter()
        .any(|s| s.len() > 128 || !crate::premise_flow::identifier(s))
    {
        return Err("lifecycle: malformed or oversized ID".into());
    }
    let operation = match op {
        Symbol::LifecycleCreate => Operation::Create,
        Symbol::LifecycleRetire => Operation::Retire,
        _ => return Err("lifecycle: expected |^+ or |^-".into()),
    };
    let target = match kind.as_str() {
        "resource" => Target::Resource(target.clone()),
        "hypothesis" => Target::Hypothesis(target.clone()),
        _ => return Err("lifecycle: expected typed resource or hypothesis target".into()),
    };
    Ok(Some((id.clone(), Entry { operation, target })))
}
