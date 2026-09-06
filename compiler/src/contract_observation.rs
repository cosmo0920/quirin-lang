//! Evaluate native QCore predicates against supplied runtime observations.
//!
//! This checks only the supplied executions. It is deliberately not a static
//! source certificate, and does not authenticate observations or prove caller
//! preconditions. Producers must record their source/build and instrumentation.

use crate::frontend::{
    DeclarationKind, SourceContractClauseKind, SourceContractExitSelector, parse_source_program,
};
use crate::lexer::{Symbol, TokenKind, lex};
use std::collections::BTreeMap;

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
            TokenKind::Symbol(Symbol::Minus) => self
                .expression(6, depth + 1)?
                .checked_neg()
                .ok_or("predicate overflow")?,
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
