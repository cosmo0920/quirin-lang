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

//! Evaluate native QCore predicates against supplied runtime observations.
//!
//! This checks only the supplied executions. It is deliberately not a static
//! source certificate, and does not authenticate observations or prove caller
//! preconditions. Producers must record their source/build and instrumentation.

use qcore_frontend::frontend::{
    DeclarationKind, SourceContractClauseKind, SourceContractExitSelector, parse_source_program,
};
use qcore_frontend::lexer::{Symbol, TokenKind, lex};
use std::collections::BTreeMap;

const MAX_POLICY_SOURCE_BYTES: usize = 64 * 1024;
const MAX_POLICY_FACTS: usize = 256;
const MAX_POLICY_NAME_BYTES: usize = 256;
const MAX_POLICY_TEXT_BYTES: usize = 4096;

/// One typed fact supplied to a bounded native policy contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContractFact {
    Integer(i128),
    Text(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObservationVerdict {
    Checked { postconditions: usize },
    OutsidePreconditions { predicate: String },
}

/// Require acceptance under a native QCore rule using facts derived by an
/// interpreter. A false precondition also rejects the attempted transition.
pub fn check_contract_facts(
    source: &str,
    function: &str,
    facts: &[(&str, i128)],
) -> Result<(), String> {
    let values = facts
        .iter()
        .map(|(name, value)| ((*name).into(), *value))
        .collect();
    match check_contract_observation(source, function, &values)? {
        ObservationVerdict::Checked { .. } => Ok(()),
        ObservationVerdict::OutsidePreconditions { predicate } => {
            Err(format!("{function}: outside precondition: {predicate}"))
        }
    }
}

/// Check a strict, argument-free native policy declaration against typed facts.
///
/// The selected declaration must be a safe `extern fn NAME() -> unit` with no
/// effects and a nonempty set of `ensures normal` clauses. Text values use the
/// frontend's double-quoted string-literal tokens, decoded with
/// `source.string.v1` escapes, and support same-type `==` and `!=` only; all
/// arithmetic and ordering remain integer-only.
pub fn check_typed_contract_facts(
    source: &str,
    function: &str,
    facts: &[(&str, ContractFact)],
) -> Result<(), String> {
    if source.len() > MAX_POLICY_SOURCE_BYTES {
        return Err("contract policy source too large".into());
    }
    if function.is_empty() || function.len() > MAX_POLICY_NAME_BYTES {
        return Err("contract policy function name is out of bounds".into());
    }
    if facts.len() > MAX_POLICY_FACTS {
        return Err("too many contract policy facts".into());
    }
    let mut values = BTreeMap::new();
    for (name, value) in facts {
        if name.is_empty() || name.len() > MAX_POLICY_NAME_BYTES {
            return Err("contract policy fact name is out of bounds".into());
        }
        if matches!(value, ContractFact::Text(text) if text.len() > MAX_POLICY_TEXT_BYTES) {
            return Err(format!("contract policy text fact `{name}` is too large"));
        }
        if values.insert((*name).to_owned(), value.clone()).is_some() {
            return Err(format!("duplicate contract policy fact: {name}"));
        }
    }

    let parsed = parse_source_program(source).map_err(|e| e.to_string())?;
    let declarations: Vec<_> = parsed
        .declarations
        .iter()
        .filter(|declaration| declaration.value.name == function)
        .collect();
    let [declaration] = declarations.as_slice() else {
        return Err(format!("expected one declaration of {function}"));
    };
    validate_typed_policy_declaration(declaration)?;
    let contract = declaration
        .value
        .function_contract
        .as_ref()
        .ok_or("missing contract")?;

    // Evaluate every clause before interpreting any requirement. Unsupported
    // syntax, type errors and missing facts cannot hide in a false Boolean arm
    // or behind a false precondition.
    let mut requirements = Vec::new();
    let mut ensures = Vec::new();
    for clause in &contract.clauses {
        let expression = &clause.value.expression_source;
        let result = evaluate_typed(expression, &values)?;
        match clause.value.kind {
            SourceContractClauseKind::Requires => requirements.push((expression, result)),
            SourceContractClauseKind::Ensures(SourceContractExitSelector::Normal) => {
                ensures.push((expression, result))
            }
            _ => return Err("contract policy supports requires and ensures normal only".into()),
        }
    }
    if ensures.is_empty() {
        return Err("missing postconditions".into());
    }
    for (predicate, value) in requirements {
        if integer_truth(&value)? == 0 {
            return Err(format!("{function}: outside precondition: {predicate}"));
        }
    }
    for (predicate, value) in ensures {
        if integer_truth(&value)? == 0 {
            return Err(format!(
                "{function}: observed contract violation: {predicate}"
            ));
        }
    }
    Ok(())
}

/// Read one uniquely declared literal policy fact.
///
/// The fact must appear as the complete expression of exactly one
/// `ensures normal FACT == LITERAL;` clause in a strict typed policy
/// declaration. This accessor evaluates no caller-supplied state and refuses
/// missing or ambiguous declarations.
pub fn contract_fact_literal(
    source: &str,
    function: &str,
    fact: &str,
) -> Result<ContractFact, String> {
    if source.len() > MAX_POLICY_SOURCE_BYTES {
        return Err("contract policy source too large".into());
    }
    if function.is_empty()
        || function.len() > MAX_POLICY_NAME_BYTES
        || fact.is_empty()
        || fact.len() > MAX_POLICY_NAME_BYTES
    {
        return Err("contract policy name is out of bounds".into());
    }
    let parsed = parse_source_program(source).map_err(|e| e.to_string())?;
    let declarations: Vec<_> = parsed
        .declarations
        .iter()
        .filter(|declaration| declaration.value.name == function)
        .collect();
    let [declaration] = declarations.as_slice() else {
        return Err(format!("expected one declaration of {function}"));
    };
    validate_typed_policy_declaration(declaration)?;
    let contract = declaration
        .value
        .function_contract
        .as_ref()
        .ok_or("missing contract")?;
    let mut found = None;
    for clause in &contract.clauses {
        match clause.value.kind {
            SourceContractClauseKind::Requires
            | SourceContractClauseKind::Ensures(SourceContractExitSelector::Normal) => {}
            _ => return Err("contract policy supports requires and ensures normal only".into()),
        }
        if clause.value.kind
            != SourceContractClauseKind::Ensures(SourceContractExitSelector::Normal)
        {
            continue;
        }
        let tokens: Vec<_> = lex(&clause.value.expression_source)
            .map_err(|error| error.to_string())?
            .into_iter()
            .map(|token| token.kind)
            .filter(|token| *token != TokenKind::Eof)
            .collect();
        let literal = match tokens.as_slice() {
            [
                TokenKind::Word(name),
                TokenKind::Symbol(Symbol::EqualEqual),
                TokenKind::StringLiteral(value),
            ] if name == fact => Some(ContractFact::Text(decode_text_literal(value)?)),
            [
                TokenKind::Word(name),
                TokenKind::Symbol(Symbol::EqualEqual),
                TokenKind::Integer(value),
            ] if name == fact => Some(ContractFact::Integer(
                value.parse().map_err(|_| "invalid predicate integer")?,
            )),
            [
                TokenKind::Word(name),
                TokenKind::Symbol(Symbol::EqualEqual),
                TokenKind::Symbol(Symbol::Minus),
                TokenKind::Integer(value),
            ] if name == fact => Some(ContractFact::Integer(
                format!("-{value}")
                    .parse()
                    .map_err(|_| "invalid predicate integer")?,
            )),
            _ => None,
        };
        if let Some(literal) = literal
            && found.replace(literal).is_some()
        {
            return Err(format!("ambiguous contract policy literal: {fact}"));
        }
    }
    found.ok_or_else(|| format!("missing contract policy literal: {fact}"))
}

fn validate_typed_policy_declaration(
    declaration: &qcore_frontend::frontend::Located<qcore_frontend::frontend::SourceDeclaration>,
) -> Result<(), String> {
    if declaration.value.kind != DeclarationKind::ExternFunction {
        return Err("contract policy requires an extern function".into());
    }
    if declaration.value.body.is_some() {
        return Err("contract policy function must not have a body".into());
    }
    if !declaration.value.parameters.is_empty() {
        return Err("contract policy function must not have parameters".into());
    }
    let result = declaration
        .value
        .result_type
        .as_ref()
        .ok_or("contract policy function requires a unit result")?;
    if result.tokens.as_slice() != [TokenKind::Word("unit".into())] {
        return Err("contract policy function requires a unit result".into());
    }
    let expected_header = [
        TokenKind::Word("extern".into()),
        TokenKind::Word("fn".into()),
        TokenKind::Word(declaration.value.name.clone()),
        TokenKind::Symbol(Symbol::LeftParen),
        TokenKind::Symbol(Symbol::RightParen),
        TokenKind::Symbol(Symbol::Arrow),
        TokenKind::Word("unit".into()),
        TokenKind::Word("contract".into()),
        TokenKind::Symbol(Symbol::LeftBrace),
    ];
    if !declaration.value.tokens.starts_with(&expected_header) {
        return Err(
            "contract policy requires the exact `extern fn NAME() -> unit contract` header".into(),
        );
    }
    Ok(())
}

pub fn check_contract_observation(
    source: &str,
    function: &str,
    values: &BTreeMap<String, i128>,
) -> Result<ObservationVerdict, String> {
    let parsed = parse_source_program(source).map_err(|e| e.to_string())?;
    let declarations: Vec<_> = parsed
        .declarations
        .iter()
        .filter(|d| d.value.name == function)
        .collect();
    let [declaration] = declarations.as_slice() else {
        return Err(format!("expected one declaration of {function}"));
    };
    if declaration.value.kind != DeclarationKind::ExternFunction {
        return Err("observation requires an extern contract".into());
    }
    let contract = declaration
        .value
        .function_contract
        .as_ref()
        .ok_or("missing contract")?;
    let mut requirements = Vec::new();
    let mut ensures = Vec::new();
    // Parse and evaluate all clauses before testing requirements, so unsupported
    // predicates and missing observations cannot hide behind a false requires.
    for clause in &contract.clauses {
        let expression = &clause.value.expression_source;
        let result = evaluate(expression, values)?;
        match clause.value.kind {
            SourceContractClauseKind::Requires => requirements.push((expression, result)),
            SourceContractClauseKind::Ensures(SourceContractExitSelector::Normal) => {
                ensures.push((expression, result))
            }
            _ => return Err("observation supports requires and ensures normal only".into()),
        }
    }
    if ensures.is_empty() {
        return Err("missing postconditions".into());
    }
    for (predicate, value) in requirements {
        if value == 0 {
            return Ok(ObservationVerdict::OutsidePreconditions {
                predicate: predicate.clone(),
            });
        }
    }
    for (predicate, value) in &ensures {
        if *value == 0 {
            return Err(format!(
                "{function}: observed contract violation: {predicate}"
            ));
        }
    }
    Ok(ObservationVerdict::Checked {
        postconditions: ensures.len(),
    })
}

/// What a replayed observation witness established, as recomputed by the core.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObservationReplay {
    /// Hooks with at least one observation that met its preconditions and
    /// satisfied every `ensures normal` clause.
    pub checked_hooks: std::collections::BTreeSet<String>,
    /// Run identities that contributed at least one counted observation.
    pub run_ids: std::collections::BTreeSet<String>,
    /// Observations excluded from coverage because a `requires` clause was
    /// false. Counted separately: preconditions-dodging never earns coverage.
    pub outside_preconditions: usize,
}

/// Replays an evidence-v2 `ContractObservationWitness` against trusted
/// contract text (`docs/design/W3_OBLIGATIONS_EVIDENCE_V2.md` §2.4).
///
/// The predicates come from `contract_sources`, which the caller pinned by
/// digest on the verification task — never from the witness. The provider
/// supplies only recorded `values` and run identities, and a fabricated
/// value cannot weaken a predicate, skip a required hook, or make evaluation
/// trivially succeed: a missing observation variable is an error, not a skip.
/// What remains trust rather than proof — that the recorded values are the
/// ones the run produced — is named as an assumption on the task, not
/// laundered into `Safe`.
pub fn replay_observation_witness(
    contract_sources: &BTreeMap<String, String>,
    required_hooks: &BTreeMap<String, String>,
    witness: &qcore_schemas::witness::ContractObservationWitness,
) -> Result<ObservationReplay, String> {
    let mut replay = ObservationReplay::default();
    let mut seen_runs = std::collections::BTreeSet::new();
    for run in &witness.runs {
        if run.run_id.is_empty() || !seen_runs.insert(&run.run_id) {
            return Err("an observed run has no run identity".into());
        }
        let sha256 =
            |value: &str| value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit());
        if !sha256(&run.binding.emitted_source_digest)
            || !sha256(&run.binding.executable_digest)
            || run.binding.compiler_invocation.is_empty()
            || run.binding.profile.is_empty()
            || run.binding.instrumentation_version.is_empty()
        {
            return Err(format!(
                "observed run `{}` has an incomplete native run binding",
                run.run_id
            ));
        }
        for observation in &run.observations {
            let Some(module) = required_hooks.get(&observation.hook) else {
                return Err(format!(
                    "observation names hook `{}`, which the core did not require",
                    observation.hook
                ));
            };
            // The hook's identity is the contract function the core pinned; a
            // provider cannot point a required hook at a weaker contract.
            if observation.function != observation.hook {
                return Err(format!(
                    "observation for hook `{}` reports function `{}`",
                    observation.hook, observation.function
                ));
            }
            let source = contract_sources.get(module).ok_or_else(|| {
                format!("contract module `{module}` is not among the trusted sources")
            })?;
            match check_contract_observation(source, &observation.function, &observation.values)? {
                ObservationVerdict::Checked { .. } => {
                    replay.checked_hooks.insert(observation.hook.clone());
                    replay.run_ids.insert(run.run_id.clone());
                }
                ObservationVerdict::OutsidePreconditions { .. } => {
                    replay.outside_preconditions += 1;
                }
            }
        }
    }
    Ok(replay)
}

/// The shared interpreter's evaluation of one predicate, for the parity test
/// of `contract_views`' fragment evaluator.
#[cfg(test)]
pub(crate) fn evaluate_predicate(
    source: &str,
    values: &BTreeMap<String, i128>,
) -> Result<i128, String> {
    evaluate(source, values)
}

fn evaluate(source: &str, values: &BTreeMap<String, i128>) -> Result<i128, String> {
    let tokens: Vec<_> = lex(source)
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|t| t.kind)
        .filter(|t| *t != TokenKind::Eof)
        .collect();
    if tokens.len() > 512 {
        return Err("observation predicate too large".into());
    }
    let mut parser = Predicate {
        tokens: &tokens,
        cursor: 0,
        values,
    };
    let value = parser.expression(0, 0)?;
    if parser.cursor != tokens.len() {
        return Err(format!("unsupported predicate syntax: {source}"));
    }
    Ok(value)
}

fn evaluate_typed(
    source: &str,
    values: &BTreeMap<String, ContractFact>,
) -> Result<ContractFact, String> {
    let tokens: Vec<_> = lex(source)
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|token| token.kind)
        .filter(|token| *token != TokenKind::Eof)
        .collect();
    if tokens.len() > 512 {
        return Err("observation predicate too large".into());
    }
    let mut parser = TypedPredicate {
        tokens: &tokens,
        cursor: 0,
        values,
    };
    let value = parser.expression(0, 0)?;
    if parser.cursor != tokens.len() {
        return Err(format!("unsupported predicate syntax: {source}"));
    }
    Ok(value)
}

fn integer(value: ContractFact, context: &str) -> Result<i128, String> {
    match value {
        ContractFact::Integer(value) => Ok(value),
        ContractFact::Text(_) => Err(format!("{context} requires integer operands")),
    }
}

fn integer_truth(value: &ContractFact) -> Result<i128, String> {
    match value {
        ContractFact::Integer(value) => Ok(*value),
        ContractFact::Text(_) => Err("contract policy predicate must produce an integer".into()),
    }
}

pub(crate) fn decode_text_literal(raw: &str) -> Result<String, String> {
    let bytes = qcore_core::std_string::decode_string_literal(raw)
        .map_err(|error| format!("invalid contract policy text literal: {error}"))?;
    if bytes.len() > MAX_POLICY_TEXT_BYTES {
        return Err("contract policy text literal is too large".into());
    }
    String::from_utf8(bytes).map_err(|_| "invalid contract policy text literal".into())
}

struct TypedPredicate<'a> {
    tokens: &'a [TokenKind],
    cursor: usize,
    values: &'a BTreeMap<String, ContractFact>,
}

impl TypedPredicate<'_> {
    fn expression(&mut self, minimum: u8, depth: usize) -> Result<ContractFact, String> {
        if depth > 64 {
            return Err("observation predicate nesting limit".into());
        }
        let token = self
            .tokens
            .get(self.cursor)
            .ok_or("expected predicate term")?
            .clone();
        self.cursor += 1;
        let mut left = match token {
            TokenKind::Integer(number) => {
                ContractFact::Integer(number.parse().map_err(|_| "invalid predicate integer")?)
            }
            TokenKind::StringLiteral(text) => ContractFact::Text(decode_text_literal(&text)?),
            TokenKind::Word(name) if name == "true" => ContractFact::Integer(1),
            TokenKind::Word(name) if name == "false" => ContractFact::Integer(0),
            TokenKind::Word(name) if name == "not" => ContractFact::Integer(i128::from(
                integer(self.expression(6, depth + 1)?, "predicate not")? == 0,
            )),
            TokenKind::Symbol(Symbol::Bang) => ContractFact::Integer(i128::from(
                integer(self.expression(6, depth + 1)?, "predicate not")? == 0,
            )),
            TokenKind::Word(name) => self
                .values
                .get(&name)
                .cloned()
                .ok_or_else(|| format!("missing observation: {name}"))?,
            TokenKind::Symbol(Symbol::Minus) => {
                if let Some(TokenKind::Integer(number)) = self.tokens.get(self.cursor) {
                    self.cursor += 1;
                    ContractFact::Integer(
                        format!("-{number}")
                            .parse()
                            .map_err(|_| "invalid predicate integer")?,
                    )
                } else {
                    ContractFact::Integer(
                        integer(self.expression(6, depth + 1)?, "predicate negation")?
                            .checked_neg()
                            .ok_or("predicate overflow")?,
                    )
                }
            }
            TokenKind::Symbol(Symbol::LeftParen) => {
                let value = self.expression(0, depth + 1)?;
                if self.tokens.get(self.cursor) != Some(&TokenKind::Symbol(Symbol::RightParen)) {
                    return Err("missing predicate closing parenthesis".into());
                }
                self.cursor += 1;
                value
            }
            _ => return Err("unsupported predicate term".into()),
        };
        while let Some(operator) = self.tokens.get(self.cursor).cloned() {
            let priority = match &operator {
                TokenKind::Word(word) if word == "or" => 1,
                TokenKind::Word(word) if word == "and" => 2,
                TokenKind::Symbol(
                    Symbol::EqualEqual
                    | Symbol::NotEqual
                    | Symbol::Less
                    | Symbol::LessEqual
                    | Symbol::Greater
                    | Symbol::GreaterEqual,
                ) => 3,
                TokenKind::Symbol(Symbol::Plus | Symbol::Minus) => 4,
                TokenKind::Symbol(Symbol::Star) => 5,
                _ => break,
            };
            if priority < minimum {
                break;
            }
            self.cursor += 1;
            let right = self.expression(priority + 1, depth + 1)?;
            left = match operator {
                TokenKind::Word(word) if word == "or" => {
                    let lhs = integer(left, "predicate or")?;
                    let rhs = integer(right, "predicate or")?;
                    ContractFact::Integer(i128::from(lhs != 0 || rhs != 0))
                }
                TokenKind::Word(word) if word == "and" => {
                    let lhs = integer(left, "predicate and")?;
                    let rhs = integer(right, "predicate and")?;
                    ContractFact::Integer(i128::from(lhs != 0 && rhs != 0))
                }
                TokenKind::Symbol(Symbol::EqualEqual | Symbol::NotEqual) => {
                    let equal = match (&left, &right) {
                        (ContractFact::Integer(lhs), ContractFact::Integer(rhs)) => lhs == rhs,
                        (ContractFact::Text(lhs), ContractFact::Text(rhs)) => lhs == rhs,
                        _ => {
                            return Err(
                                "predicate equality requires operands of the same type".into()
                            );
                        }
                    };
                    ContractFact::Integer(i128::from(
                        if operator == TokenKind::Symbol(Symbol::EqualEqual) {
                            equal
                        } else {
                            !equal
                        },
                    ))
                }
                TokenKind::Symbol(
                    Symbol::Less | Symbol::LessEqual | Symbol::Greater | Symbol::GreaterEqual,
                ) => {
                    let lhs = integer(left, "predicate ordering")?;
                    let rhs = integer(right, "predicate ordering")?;
                    let result = match operator {
                        TokenKind::Symbol(Symbol::Less) => lhs < rhs,
                        TokenKind::Symbol(Symbol::LessEqual) => lhs <= rhs,
                        TokenKind::Symbol(Symbol::Greater) => lhs > rhs,
                        TokenKind::Symbol(Symbol::GreaterEqual) => lhs >= rhs,
                        _ => unreachable!(),
                    };
                    ContractFact::Integer(i128::from(result))
                }
                TokenKind::Symbol(Symbol::Plus | Symbol::Minus | Symbol::Star) => {
                    let lhs = integer(left, "predicate arithmetic")?;
                    let rhs = integer(right, "predicate arithmetic")?;
                    let result = match operator {
                        TokenKind::Symbol(Symbol::Plus) => lhs.checked_add(rhs),
                        TokenKind::Symbol(Symbol::Minus) => lhs.checked_sub(rhs),
                        TokenKind::Symbol(Symbol::Star) => lhs.checked_mul(rhs),
                        _ => unreachable!(),
                    }
                    .ok_or("predicate overflow")?;
                    ContractFact::Integer(result)
                }
                _ => unreachable!(),
            };
        }
        Ok(left)
    }
}

struct Predicate<'a> {
    tokens: &'a [TokenKind],
    cursor: usize,
    values: &'a BTreeMap<String, i128>,
}
impl Predicate<'_> {
    fn expression(&mut self, minimum: u8, depth: usize) -> Result<i128, String> {
        if depth > 64 {
            return Err("observation predicate nesting limit".into());
        }
        let token = self
            .tokens
            .get(self.cursor)
            .ok_or("expected predicate term")?
            .clone();
        self.cursor += 1;
        let mut left = match token {
            TokenKind::Integer(number) => {
                number.parse().map_err(|_| "invalid predicate integer")?
            }
            TokenKind::Word(name) if name == "true" => 1,
            TokenKind::Word(name) if name == "false" => 0,
            TokenKind::Word(name) if name == "not" => {
                i128::from(self.expression(6, depth + 1)? == 0)
            }
            TokenKind::Symbol(Symbol::Bang) => i128::from(self.expression(6, depth + 1)? == 0),
            TokenKind::Word(name) => *self
                .values
                .get(&name)
                .ok_or_else(|| format!("missing observation: {name}"))?,
            TokenKind::Symbol(Symbol::Minus) => {
                if let Some(TokenKind::Integer(number)) = self.tokens.get(self.cursor) {
                    // Range-check the signed spelling, including i128::MIN;
                    // arithmetic negation of nonliteral terms remains checked.
                    self.cursor += 1;
                    format!("-{number}")
                        .parse()
                        .map_err(|_| "invalid predicate integer")?
                } else {
                    self.expression(6, depth + 1)?
                        .checked_neg()
                        .ok_or("predicate overflow")?
                }
            }
            TokenKind::Symbol(Symbol::LeftParen) => {
                let value = self.expression(0, depth + 1)?;
                if self.tokens.get(self.cursor) != Some(&TokenKind::Symbol(Symbol::RightParen)) {
                    return Err("missing predicate closing parenthesis".into());
                }
                self.cursor += 1;
                value
            }
            _ => return Err("unsupported predicate term".into()),
        };
        while let Some(operator) = self.tokens.get(self.cursor).cloned() {
            let priority = match &operator {
                TokenKind::Word(word) if word == "or" => 1,
                TokenKind::Word(word) if word == "and" => 2,
                TokenKind::Symbol(
                    Symbol::EqualEqual
                    | Symbol::NotEqual
                    | Symbol::Less
                    | Symbol::LessEqual
                    | Symbol::Greater
                    | Symbol::GreaterEqual,
                ) => 3,
                TokenKind::Symbol(Symbol::Plus | Symbol::Minus) => 4,
                TokenKind::Symbol(Symbol::Star) => 5,
                _ => break,
            };
            if priority < minimum {
                break;
            }
            self.cursor += 1;
            let right = self.expression(priority + 1, depth + 1)?;
            left = match operator {
                TokenKind::Word(word) if word == "or" => i128::from(left != 0 || right != 0),
                TokenKind::Word(word) if word == "and" => i128::from(left != 0 && right != 0),
                TokenKind::Symbol(Symbol::EqualEqual) => i128::from(left == right),
                TokenKind::Symbol(Symbol::NotEqual) => i128::from(left != right),
                TokenKind::Symbol(Symbol::Less) => i128::from(left < right),
                TokenKind::Symbol(Symbol::LessEqual) => i128::from(left <= right),
                TokenKind::Symbol(Symbol::Greater) => i128::from(left > right),
                TokenKind::Symbol(Symbol::GreaterEqual) => i128::from(left >= right),
                TokenKind::Symbol(Symbol::Plus) => {
                    left.checked_add(right).ok_or("predicate overflow")?
                }
                TokenKind::Symbol(Symbol::Minus) => {
                    left.checked_sub(right).ok_or("predicate overflow")?
                }
                TokenKind::Symbol(Symbol::Star) => {
                    left.checked_mul(right).ok_or("predicate overflow")?
                }
                _ => unreachable!(),
            };
        }
        Ok(left)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const SOURCE: &str = "extern fn resize() -> unit contract { requires size > 0; ensures normal result_null == 0 or old_live == 1; };";
    #[test]
    fn native_predicates_control_the_verdict() {
        let mut values = BTreeMap::from([
            ("size".into(), 5),
            ("result_null".into(), 1),
            ("old_live".into(), 1),
        ]);
        assert_eq!(
            check_contract_observation(SOURCE, "resize", &values),
            Ok(ObservationVerdict::Checked { postconditions: 1 })
        );
        assert!(
            check_contract_observation(
                &SOURCE.replace("old_live == 1", "old_live == 0"),
                "resize",
                &values
            )
            .is_err()
        );
        values.insert("old_live".into(), 0);
        assert!(check_contract_observation(SOURCE, "resize", &values).is_err());
        values.insert("size".into(), 0);
        assert!(matches!(
            check_contract_observation(SOURCE, "resize", &values),
            Ok(ObservationVerdict::OutsidePreconditions { .. })
        ));
        values.remove("old_live");
        assert!(check_contract_observation(SOURCE, "resize", &values).is_err());
    }
    #[test]
    fn unsupported_and_overflowing_predicates_fail_closed() {
        for expression in [
            "missing",
            "f()",
            "true or missing",
            "170141183460469231731687303715884105727 + 1",
            "1 == 1 trailing",
        ] {
            assert!(
                evaluate(expression, &BTreeMap::new()).is_err(),
                "{expression}"
            );
        }
        assert_eq!(
            evaluate("not (2 * 3 + 1 != 7) and 5 > 4", &BTreeMap::new()),
            Ok(1)
        );
    }
}

#[cfg(test)]
#[path = "../tests/unit/contract_observation.rs"]
mod cleanup_observation_tests;
