//! Interpret the bounded lifecycle predicate vocabulary in native QCore
//! contract files and check it against independently imported C field paths.
//! Library function names and postconditions belong in contract/qcore, not here.

use std::collections::{BTreeMap, BTreeSet};

use crate::frontend::{
    DeclarationKind, SourceContractClauseKind, SourceContractExitSelector, parse_source_program,
};
use crate::lexer::{Symbol, TokenKind, lex};
use crate::verifier_plugins::chunkio::{ChunkSourceField, ChunkSourceSummary};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChunkContractCertificate {
    function: String,
    contract_source: String,
    checked_states: usize,
}

impl ChunkContractCertificate {
    pub fn function(&self) -> &str {
        &self.function
    }

    pub fn contract_source(&self) -> &str {
        &self.contract_source
    }

    pub fn checked_states(&self) -> usize {
        self.checked_states
    }
}

/// Discharge all postconditions over both values of each Boolean field. The
/// four preconditions are explicit assumptions, not established facts about
/// arbitrary callers. Unsupported vocabulary or weakened/missing domain
/// requirements fails closed. This is a function-contract result, not a
/// whole-program lifecycle or memory-safety certificate.
pub fn check_chunk_source_contract(
    summary: &ChunkSourceSummary,
    contract_source: &str,
) -> Result<ChunkContractCertificate, String> {
    let parsed = parse_source_program(contract_source).map_err(|error| error.to_string())?;
    let declarations = parsed
        .declarations
        .iter()
        .filter(|declaration| declaration.value.name == summary.function())
        .collect::<Vec<_>>();
    let [declaration] = declarations.as_slice() else {
        return Err(format!(
            "expected exactly one QCore declaration for {}",
            summary.function()
        ));
    };
    let declaration = &declaration.value;
    if declaration.kind != DeclarationKind::ExternFunction || declaration.parameters.len() != 1 {
        return Err("bounded contract requires one borrowed external parameter".into());
    }
    let parameter = &declaration.parameters[0];
    let parameter_tokens = tokens(&parameter.type_source)?;
    let [
        TokenKind::Word(borrow),
        TokenKind::Symbol(Symbol::Less),
        TokenKind::Symbol(Symbol::Apostrophe),
        TokenKind::Word(_lifetime),
        TokenKind::Symbol(Symbol::Comma),
        TokenKind::Word(mode),
        TokenKind::Symbol(Symbol::Comma),
        TokenKind::Word(resource),
        TokenKind::Symbol(Symbol::Greater),
    ] = parameter_tokens.as_slice()
    else {
        return Err("unsupported borrowed chunk parameter type".into());
    };
    let expected_resource = match summary.field() {
        ChunkSourceField::Busy => "InputChunk",
        ChunkSourceField::Locked => "Chunk",
    };
    if borrow != "borrow"
        || resource != expected_resource
        || !matches!(mode.as_str(), "mut" | "const")
    {
        return Err("QCore parameter does not match the C chunk field profile".into());
    }
    if mode == "const" && summary.paths().iter().any(|path| !path.writes.is_empty()) {
        return Err("C source writes through a const QCore borrow".into());
    }
    let arrow = declaration
        .tokens
        .iter()
        .position(|token| *token == TokenKind::Symbol(Symbol::Arrow))
        .ok_or("missing QCore result type")?;
    if declaration.tokens.get(arrow + 1) != Some(&TokenKind::Word("i32".into()))
        || declaration.tokens.get(arrow + 2) != Some(&TokenKind::Word("contract".into()))
    {
        return Err(
            "bounded contract requires an ordinary i32 result without extra effects".into(),
        );
    }
    let contract = declaration
        .function_contract
        .as_ref()
        .ok_or("missing QCore contract")?;
    let mut requirements = BTreeSet::new();
    let mut postconditions = Vec::new();
    let mut variables = BTreeSet::new();
    for clause in &contract.clauses {
        match &clause.value.kind {
            SourceContractClauseKind::Requires => {
                let clause_tokens = tokens(&clause.value.expression_source)?;
                let [TokenKind::Word(name)] = clause_tokens.as_slice() else {
                    return Err("unsupported bounded source precondition".into());
                };
                if !requirements.insert(name.clone()) {
                    return Err(format!("duplicate precondition {name}"));
                }
            }
            SourceContractClauseKind::Ensures(SourceContractExitSelector::Normal) => {
                let clause_tokens = tokens(&clause.value.expression_source)?;
                let mut parser = PredicateParser {
                    tokens: &clause_tokens,
                    cursor: 0,
                    variables: &mut variables,
                };
                let predicate = parser.predicate(0)?;
                if parser.cursor != clause_tokens.len() {
                    return Err(format!(
                        "unsupported trailing predicate syntax: {}",
                        clause.value.expression_source
                    ));
                }
                postconditions.push((clause.value.expression_source.clone(), predicate));
            }
            _ => {
                return Err(
                    "bounded source profile supports requires and ensures normal only".into(),
                );
            }
        }
    }
    let required = [
        "chunk_is_live",
        "exclusive_chunk_access",
        "busy_is_boolean",
        "lock_is_boolean",
    ]
    .map(str::to_owned)
    .into_iter()
    .collect();
    if requirements != required {
        return Err(
            "contract must state live chunk, exclusive access, and both Boolean field domains"
                .into(),
        );
    }
    if !["result", "busy", "lock"]
        .iter()
        .all(|name| variables.contains(*name))
    {
        return Err("contract must constrain result and both exit fields".into());
    }
    for path in summary.paths() {
        for other in [false, true] {
            let (old_busy, old_lock, busy, lock) = match summary.field() {
                ChunkSourceField::Busy => (path.initial_value, other, path.final_value, other),
                ChunkSourceField::Locked => (other, path.initial_value, other, path.final_value),
            };
            let values = BTreeMap::from([
                ("old_busy", i64::from(old_busy)),
                ("old_lock", i64::from(old_lock)),
                ("busy", i64::from(busy)),
                ("lock", i64::from(lock)),
                ("result", i64::from(path.return_value)),
            ]);
            for (source, predicate) in &postconditions {
                if predicate.evaluate(&values)? == 0 {
                    return Err(format!(
                        "contract mismatch in {}: `{source}`; old_busy={}, old_lock={}, busy={}, lock={}, result={}",
                        summary.function(),
                        i32::from(old_busy),
                        i32::from(old_lock),
                        i32::from(busy),
                        i32::from(lock),
                        path.return_value
                    ));
                }
            }
        }
    }
    Ok(ChunkContractCertificate {
        function: summary.function().to_owned(),
        contract_source: contract_source.to_owned(),
        checked_states: 4,
    })
}

fn tokens(source: &str) -> Result<Vec<TokenKind>, String> {
    Ok(lex(source)
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|token| token.kind)
        .filter(|token| *token != TokenKind::Eof)
        .collect())
}

enum Predicate {
    Integer(i64),
    Variable(String),
    Add(Box<Self>, Box<Self>),
    Subtract(Box<Self>, Box<Self>),
    Equal(Box<Self>, Box<Self>),
    NotEqual(Box<Self>, Box<Self>),
}

impl Predicate {
    fn evaluate(&self, values: &BTreeMap<&str, i64>) -> Result<i64, String> {
        Ok(match self {
            Self::Integer(value) => *value,
            Self::Variable(name) => *values.get(name.as_str()).ok_or("unknown ghost variable")?,
            Self::Add(left, right) => left
                .evaluate(values)?
                .checked_add(right.evaluate(values)?)
                .ok_or("contract integer overflow")?,
            Self::Subtract(left, right) => left
                .evaluate(values)?
                .checked_sub(right.evaluate(values)?)
                .ok_or("contract integer overflow")?,
            Self::Equal(left, right) => {
                i64::from(left.evaluate(values)? == right.evaluate(values)?)
            }
            Self::NotEqual(left, right) => {
                i64::from(left.evaluate(values)? != right.evaluate(values)?)
            }
        })
    }
}

struct PredicateParser<'a> {
    tokens: &'a [TokenKind],
    cursor: usize,
    variables: &'a mut BTreeSet<String>,
}

impl PredicateParser<'_> {
    fn take(&mut self, symbol: Symbol) -> bool {
        if self.tokens.get(self.cursor) == Some(&TokenKind::Symbol(symbol)) {
            self.cursor += 1;
            true
        } else {
            false
        }
    }

    fn predicate(&mut self, depth: usize) -> Result<Predicate, String> {
        let left = self.sum(depth)?;
        if self.take(Symbol::EqualEqual) {
            Ok(Predicate::Equal(Box::new(left), Box::new(self.sum(depth)?)))
        } else if self.take(Symbol::NotEqual) {
            Ok(Predicate::NotEqual(
                Box::new(left),
                Box::new(self.sum(depth)?),
            ))
        } else {
            Err("postcondition requires an equality or inequality".into())
        }
    }

    fn sum(&mut self, depth: usize) -> Result<Predicate, String> {
        let mut value = self.atom(depth)?;
        let mut terms = 0;
        loop {
            if self.take(Symbol::Plus) {
                value = Predicate::Add(Box::new(value), Box::new(self.atom(depth)?));
            } else if self.take(Symbol::Minus) {
                value = Predicate::Subtract(Box::new(value), Box::new(self.atom(depth)?));
            } else {
                return Ok(value);
            }
            terms += 1;
            if terms > 64 {
                return Err("contract expression exceeds term limit".into());
            }
        }
    }

    fn atom(&mut self, depth: usize) -> Result<Predicate, String> {
        if depth > 32 {
            return Err("contract expression exceeds depth limit".into());
        }
        if self.take(Symbol::Minus) {
            return Ok(Predicate::Subtract(
                Box::new(Predicate::Integer(0)),
                Box::new(self.atom(depth + 1)?),
            ));
        }
        if self.take(Symbol::LeftParen) {
            let value = self.sum(depth + 1)?;
            if !self.take(Symbol::RightParen) {
                return Err("unclosed contract expression".into());
            }
            return Ok(value);
        }
        let token = self
            .tokens
            .get(self.cursor)
            .ok_or("missing contract operand")?;
        self.cursor += 1;
        match token {
            TokenKind::Integer(value) => value
                .parse()
                .map(Predicate::Integer)
                .map_err(|_| "unsupported contract integer".into()),
            TokenKind::Word(name)
                if matches!(
                    name.as_str(),
                    "old_busy" | "old_lock" | "busy" | "lock" | "result"
                ) =>
            {
                self.variables.insert(name.clone());
                Ok(Predicate::Variable(name.clone()))
            }
            _ => Err(format!("unsupported contract operand {token:?}")),
        }
    }
}

#[cfg(test)]
#[path = "../tests/unit/chunk_contract.rs"]
mod tests;
