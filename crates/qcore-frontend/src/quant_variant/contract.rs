// SPDX-License-Identifier: Apache-2.0

//! Source declarations for generation-indexed custody results (C04.13.2).
//! The adapter retains the underlying relational contract for checking
//! against the selected extern assumption.

use std::collections::{BTreeMap, BTreeSet};

use crate::lexer::{Symbol as S, Token, TokenKind};
use qcore_schemas::contract::{
    ResourceFunctionContract, ResourceParameterMode, ResourceReturnMode,
};
use qcore_schemas::relational_post::{
    ExitClass, RelationOp, RelationalContract, RelationalOrigin, ReturnRelation,
};

#[derive(Debug, Clone)]
pub(super) struct Arm {
    pub name: String,
    pub custody: bool,
    pub code: bool,
}

#[derive(Debug, Clone)]
pub(super) struct Variant {
    pub name: String,
    pub subject: String,
    pub arms: Vec<Arm>,
}

#[derive(Debug, Clone)]
pub struct Adapter {
    pub name: String,
    pub(super) variant: String,
    pub(super) classes: Vec<(ReturnRelation, usize)>,
    declared: RelationalContract,
}

impl Adapter {
    /// Compare the independently loaded assumption, including every resource
    /// mode and kind. Clause order has no effect on this comparison. The
    /// caller must also compare the selected function's ABI and identity.
    pub fn matches_selected_contract(&self, selected: &RelationalContract) -> bool {
        selected.origin == RelationalOrigin::Declared
            && selected.withdrawn.is_none()
            && selected.classes.len() == self.declared.classes.len()
            && self
                .declared
                .classes
                .iter()
                .all(|class| selected.classes.contains(class))
    }
}

pub(super) struct Reader<'a> {
    pub tokens: &'a [Token],
    pub pos: usize,
}

impl Reader<'_> {
    pub fn peek(&self) -> Option<&TokenKind> {
        self.tokens.get(self.pos).map(|token| &token.kind)
    }

    pub fn eat_symbol(&mut self, symbol: S) -> bool {
        if self.peek() != Some(&TokenKind::Symbol(symbol)) {
            return false;
        }
        self.pos += 1;
        true
    }

    pub fn symbol(&mut self, symbol: S) -> Result<(), String> {
        self.eat_symbol(symbol)
            .then_some(())
            .ok_or_else(|| format!("expected {symbol:?}"))
    }

    pub fn eat_word(&mut self, expected: &str) -> bool {
        if !matches!(self.peek(), Some(TokenKind::Word(word)) if word == expected) {
            return false;
        }
        self.pos += 1;
        true
    }

    pub fn word(&mut self, expected: &str) -> Result<(), String> {
        self.eat_word(expected)
            .then_some(())
            .ok_or_else(|| format!("expected `{expected}`"))
    }

    pub fn ident(&mut self) -> Result<String, String> {
        let Some(TokenKind::Word(word)) = self.peek() else {
            return Err("expected an identifier".into());
        };
        let name = word.clone();
        self.pos += 1;
        Ok(name)
    }

    fn relation(&mut self) -> Result<ReturnRelation, String> {
        self.symbol(S::LeftParen)?;
        let op =
            RelationOp::from_spelling(&self.ident()?).ok_or("expected a return-code relation")?;
        self.symbol(S::Comma)?;
        let negative = self.eat_symbol(S::Minus);
        let Some(TokenKind::Integer(digits)) = self.peek() else {
            return Err("expected a signed 32-bit decimal constant".into());
        };
        if !digits.bytes().all(|b| b.is_ascii_digit())
            || (digits.starts_with('0') && (negative || digits.len() != 1))
        {
            return Err("expected a canonical return-code constant".into());
        }
        let value = digits
            .parse::<i64>()
            .map_err(|_| "return code overflows i32")?;
        let value = if negative { -value } else { value };
        i32::try_from(value).map_err(|_| "return code overflows i32")?;
        self.pos += 1;
        Ok(ReturnRelation::new(op, value))
    }
}

pub(super) fn parse_variant(tokens: &[Token], pos: &mut usize) -> Result<Variant, String> {
    let mut r = Reader { tokens, pos: *pos };
    r.word("quant")?;
    r.word("variant")?;
    let name = r.ident()?;
    r.symbol(S::LeftParen)?;
    let subject = r.ident()?;
    r.symbol(S::RightParen)?;
    r.symbol(S::LeftBrace)?;
    let mut arms = Vec::new();
    let mut names = BTreeSet::new();
    while !r.eat_symbol(S::RightBrace) {
        let tag = r.ident()?;
        if !names.insert(tag.clone()) {
            return Err(format!("duplicate variant arm `{tag}`"));
        }
        let code = r.eat_symbol(S::LeftParen);
        if code {
            r.word("code")?;
            r.symbol(S::Colon)?;
            r.word("i32")?;
            r.symbol(S::RightParen)?;
        }
        r.word("carries")?;
        let custody = if r.eat_word("custody") {
            r.symbol(S::LeftParen)?;
            r.word(&subject)?;
            r.symbol(S::RightParen)?;
            true
        } else {
            if r.eat_word("protocol") {
                return Err("protocol fragments require verified transition evidence".into());
            }
            r.word("none")?;
            false
        };
        r.symbol(S::Semicolon)?;
        arms.push(Arm {
            name: tag,
            custody,
            code,
        });
    }
    if arms.is_empty() {
        return Err("a quant variant requires at least one tag".into());
    }
    *pos = r.pos;
    Ok(Variant {
        name,
        subject,
        arms,
    })
}

fn disposition(r: &mut Reader<'_>, subject: &str) -> Result<ExitClass, String> {
    let relation = r.relation()?;
    r.symbol(S::Comma)?;
    let mode = match r.ident()?.as_str() {
        "resource_consumed" => ResourceParameterMode::Consumed { nullable: false },
        "resource_owned" => ResourceParameterMode::Owned { nullable: false },
        _ => return Err("expected a nonnull consumed or owned disposition".into()),
    };
    r.symbol(S::LeftParen)?;
    r.word(subject)?;
    r.symbol(S::Comma)?;
    r.word("nonnull")?;
    r.symbol(S::RightParen)?;
    r.symbol(S::RightParen)?;
    Ok(ExitClass {
        relation,
        contract: ResourceFunctionContract::new([mode], ResourceReturnMode::None),
    })
}

pub(super) fn parse_adapter(
    tokens: &[Token],
    pos: &mut usize,
    variants: &BTreeMap<String, Variant>,
) -> Result<Adapter, String> {
    let mut r = Reader { tokens, pos: *pos };
    r.word("extern")?;
    r.word("fn")?;
    let name = r.ident()?;
    r.symbol(S::LeftParen)?;
    let subject = r.ident()?;
    r.symbol(S::Colon)?;
    r.word("ptr")?;
    r.symbol(S::Less)?;
    r.word("mut")?;
    r.symbol(S::Comma)?;
    r.word("u8")?;
    r.symbol(S::Greater)?;
    r.symbol(S::RightParen)?;
    r.symbol(S::Arrow)?;
    r.word("i32")?;
    r.word("contract")?;
    r.symbol(S::LeftBrace)?;
    let mut required = false;
    let mut scalar_result = false;
    let mut dispositions = Vec::new();
    let mut mappings = Vec::new();
    while !r.eat_symbol(S::RightBrace) {
        if r.eat_word("requires") {
            if required {
                return Err("duplicate resource contract requirement".into());
            }
            r.word("resource_contract_v1")?;
            required = true;
        } else if r.eat_word("ensures") {
            r.word("normal")?;
            if r.eat_word("resource_return_none") {
                if scalar_result {
                    return Err("duplicate result disposition".into());
                }
                r.symbol(S::LeftParen)?;
                r.symbol(S::RightParen)?;
                scalar_result = true;
            } else {
                r.word("resource_on_return")?;
                dispositions.push(disposition(&mut r, &subject)?);
            }
        } else {
            r.word("quant")?;
            r.word("variant")?;
            r.word("on_return")?;
            let relation = r.relation()?;
            r.symbol(S::RightParen)?;
            let variant = r.ident()?;
            r.symbol(S::ColonColon)?;
            mappings.push((relation, variant, r.ident()?));
        }
        r.symbol(S::Semicolon)?;
    }
    r.symbol(S::Semicolon)?;
    if !required || !scalar_result {
        return Err("mapped extern needs resource_contract_v1 and resource_return_none()".into());
    }
    let declared = RelationalContract {
        origin: RelationalOrigin::Declared,
        classes: dispositions,
        withdrawn: None,
    };
    if !declared.disjoint() || !declared.total() || declared.join().is_some() {
        return Err("return-code dispositions must be disjoint, total and distinct".into());
    }
    let variant = mappings
        .first()
        .and_then(|(_, name, _)| variants.get(name))
        .ok_or("expected a mapping to a declared quant variant")?;
    if variant.subject != subject {
        return Err("variant subject does not name the extern formal".into());
    }
    let mut remaining: BTreeSet<_> = declared.classes.iter().map(|c| c.relation).collect();
    let mut seen_tags = BTreeSet::new();
    let mut classes = Vec::new();
    for (relation, variant_name, tag) in mappings {
        if variant_name != variant.name
            || !remaining.remove(&relation)
            || !seen_tags.insert(tag.clone())
        {
            return Err("return-code mapping repeats or changes a class, variant or tag".into());
        }
        let arm = variant
            .arms
            .iter()
            .position(|arm| arm.name == tag)
            .ok_or_else(|| format!("unknown variant tag `{tag}`"))?;
        let selected = declared
            .select(relation)
            .ok_or("no unique disposition for this class")?;
        let retains = matches!(
            selected.parameters.as_slice(),
            [ResourceParameterMode::Owned { nullable: false }]
        );
        if retains != variant.arms[arm].custody {
            return Err(format!(
                "tag `{tag}` does not carry its class's post-call custody"
            ));
        }
        classes.push((relation, arm));
    }
    if !remaining.is_empty() || seen_tags.len() != variant.arms.len() {
        return Err("every return class and every variant tag must be mapped exactly once".into());
    }
    *pos = r.pos;
    Ok(Adapter {
        name,
        variant: variant.name.clone(),
        classes,
        declared,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex;

    const SOURCE: &str = r#"
        quant variant Receipt(slot) {
            Returned(code: i32) carries custody(slot);
            Taken carries none;
        }
        extern fn accept(slot: ptr<mut, u8>) -> i32 contract {
            requires resource_contract_v1;
            ensures normal resource_on_return(lt, -1, resource_owned(slot, nonnull));
            ensures normal resource_on_return(ge, -1, resource_consumed(slot, nonnull));
            ensures normal resource_return_none();
            quant variant on_return(ge, -1) Receipt::Taken;
            quant variant on_return(lt, -1) Receipt::Returned;
        };
    "#;

    fn parse(source: &str) -> Result<Adapter, String> {
        let tokens = lex(source).map_err(|e| e.to_string())?;
        let mut pos = 0;
        let variant = parse_variant(&tokens, &mut pos)?;
        let variants = BTreeMap::from([(variant.name.clone(), variant)]);
        let adapter = parse_adapter(&tokens, &mut pos, &variants)?;
        assert_eq!(tokens[pos].kind, TokenKind::Eof);
        Ok(adapter)
    }

    #[test]
    fn uses_explicit_classes_independently_of_arm_order() {
        let adapter = parse(SOURCE).unwrap();
        assert_eq!(
            adapter.classes,
            [
                (ReturnRelation::new(RelationOp::Ge, -1), 1),
                (ReturnRelation::new(RelationOp::Lt, -1), 0)
            ]
        );
        let mut selected = adapter.declared.clone();
        selected.classes.reverse();
        assert!(adapter.matches_selected_contract(&selected));
        selected.withdrawn = Some("body contradicts the declaration".into());
        assert!(!adapter.matches_selected_contract(&selected));
        selected.withdrawn = None;
        selected.classes[0].contract.parameters.clear();
        assert!(!adapter.matches_selected_contract(&selected));
    }

    #[test]
    fn incomplete_ambiguous_or_forged_contracts_refuse() {
        for (old, new) in [
            ("lt, -1, resource_owned", "le, -1, resource_owned"),
            ("ge, -1, resource_consumed", "gt, -1, resource_consumed"),
            ("resource_consumed(slot", "resource_owned(slot"),
            ("resource_return_none();", "resource_return_owned(nonnull);"),
            ("requires resource_contract_v1;", ""),
            ("on_return(ge, -1) Receipt", "on_return(gt, -2) Receipt"),
            ("Receipt::Returned;", "Receipt::Taken;"),
            ("Taken carries none", "Taken carries custody(slot)"),
            ("custody(slot);", "custody(alias);"),
            ("custody(slot);", "protocol(slot, Ready);"),
            ("Receipt::Returned;", "Receipt::Returned using #unverified;"),
            ("-1", "-0"),
            ("-1", "2147483648"),
        ] {
            assert!(parse(&SOURCE.replace(old, new)).is_err(), "{new}");
        }
    }
}
