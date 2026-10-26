//! Bounded first-order proposition reader. Relation parameters are explicitly
//! typed assumptions; parsing and type checking never assign them truth.
use crate::lexer::{Symbol as S, TokenKind as T, lex};
use qcore_schemas::quant_proposition::{Definition, Formula as F, Parameter, Sort};
use std::collections::BTreeMap;

const MAX_BYTES: usize = 65536;
const MAX_NODES: usize = 256;
const MAX_DEPTH: usize = 32;
fn error() -> String {
    "quant-proposition: malformed, unsupported or ill-typed formula".into()
}
fn identifier(s: &str) -> bool {
    s.len() <= 128
        && s.bytes().next().is_some_and(|c| c.is_ascii_lowercase())
        && s.bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_')
}
struct Reader {
    tokens: Vec<T>,
    at: usize,
    nodes: usize,
}
impl Reader {
    fn take(&mut self, token: T) -> bool {
        if self.tokens.get(self.at) == Some(&token) {
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
        if let Some(T::Word(w)) = self.tokens.get(self.at)
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
        if name != "predicate" {
            return Ok(Sort::Value { name });
        }
        self.need(T::Symbol(S::Less))?;
        let mut arguments = Vec::new();
        if !self.take(T::Symbol(S::Greater)) {
            loop {
                arguments.push(self.word()?);
                if arguments.len() > 16 {
                    return Err(error());
                }
                if self.take(T::Symbol(S::Greater)) {
                    break;
                }
                self.need(T::Symbol(S::Comma))?;
            }
        }
        Ok(Sort::Predicate { arguments })
    }
    fn formula(&mut self, depth: usize) -> Result<F, String> {
        self.nodes += 1;
        if depth > MAX_DEPTH || self.nodes > MAX_NODES {
            return Err(error());
        }
        if self.take(T::Symbol(S::Bang)) {
            return Ok(F::Not {
                body: Box::new(self.formula(depth + 1)?),
            });
        }
        for universal in [true, false] {
            let keyword = if universal { "forall" } else { "exists" };
            if self.take(T::Word(keyword.into())) {
                self.need(T::Symbol(S::LeftParen))?;
                let variable = self.variable()?;
                self.need(T::Symbol(S::Colon))?;
                let sort = self.word()?;
                if sort == "predicate" {
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
        if self.take(T::Symbol(S::LeftParen)) {
            let left = Box::new(self.formula(depth + 1)?);
            let op = self.tokens.get(self.at).cloned().ok_or_else(error)?;
            if !matches!(op, T::Symbol(S::AndAnd | S::PipePipe | S::FatArrow)) {
                return Err(error());
            }
            self.at += 1;
            let right = Box::new(self.formula(depth + 1)?);
            self.need(T::Symbol(S::RightParen))?;
            return Ok(match op {
                T::Symbol(S::AndAnd) => F::And { left, right },
                T::Symbol(S::PipePipe) => F::Or { left, right },
                _ => F::Implies { left, right },
            });
        }
        let name = self.variable()?;
        if self.take(T::Symbol(S::EqualEqual)) {
            return Ok(F::Equal {
                left: name,
                right: self.variable()?,
            });
        }
        self.need(T::Symbol(S::LeftParen))?;
        let mut arguments = Vec::new();
        if !self.take(T::Symbol(S::RightParen)) {
            loop {
                arguments.push(self.variable()?);
                if arguments.len() > 16 {
                    return Err(error());
                }
                if self.take(T::Symbol(S::RightParen)) {
                    break;
                }
                self.need(T::Symbol(S::Comma))?;
            }
        }
        Ok(F::Predicate { name, arguments })
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
/// Recheck the typed schema independently of source parsing (including wire inputs).
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
            Sort::Value { name } => identifier(name) && name != "predicate",
            Sort::Predicate { arguments } => {
                arguments.len() <= 16 && arguments.iter().all(|s| identifier(s) && s != "predicate")
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
        *nodes += 1;
        if depth > MAX_DEPTH || *nodes > MAX_NODES {
            return Err(error());
        }
        match f {
            F::Predicate { name, arguments } => {
                let Some(Sort::Predicate { arguments: sorts }) = scope.get(name) else {
                    return Err(error());
                };
                if arguments.len() != sorts.len()
                    || !arguments.iter().zip(sorts).all(|(arg, sort)| {
                        scope.get(arg) == Some(&Sort::Value { name: sort.clone() })
                    })
                {
                    return Err(error());
                }
            }
            F::Equal { left, right } => {
                if !matches!(scope.get(left), Some(Sort::Value { .. }))
                    || scope.get(left) != scope.get(right)
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
                if !identifier(variable)
                    || !identifier(sort)
                    || sort == "predicate"
                    || scope.contains_key(variable)
                {
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
