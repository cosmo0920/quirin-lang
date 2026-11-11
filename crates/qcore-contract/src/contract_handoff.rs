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

//! Card **SUPPLIED-OWNED-RETURN-CODE-HANDOFF**: a formal the callback takes
//! depending on its return code (contract specification §C04.19.5, grammar
//! G05 and G05d, gateway rules 12m and 13l), read total-or-reject like every
//! other clause.
//!
//! ```text
//! alias #PASS = @RD_KAFKA_OP_RES_PASS;                                  // module level
//! quant live #rko: #owner |-> #callback when #result != @RD_KAFKA_OP_RES_PASS;
//! quant live #rko: #owner |-> #callback when #result != #PASS;          // the same clause
//! quant live #rko: #owner |-> #callback when #result == @RD_KAFKA_OP_RES_KEEP;
//! ```
//!
//! USER DECISIONS 2026-10-15 (recorded verbatim in the card's report): `#`
//! marks binders (`#result`, `#rko`) and value keywords (`#absent`, `#owner`,
//! `#callback`); the clause-kind word after `quant` stays bare; `@NAME` is a
//! C name Clang resolves (an enumerator of the guarded value's own enum
//! type, pinned at both revisions); aliases are module-local, declared once,
//! their right-hand side an `@` name, their name colliding with no reserved
//! keyword, binder or parameter of the module; both spellings load to one
//! canonical [`HandOffGuard`]. This loader reads the text: Clang's
//! resolution is the gateway's (per unit) and `quirin-audit pin`'s (the
//! enumeration's reviewed identity).

use std::collections::BTreeMap;

use qcore_frontend::frontend::SourceProgram;
use qcore_frontend::lexer::{MAPS_TO, Symbol, TokenKind, lex};
use qcore_schemas::contract::{
    ResourceFunctionContract, ResourceParameterMode, ResourceReturnMode,
};
use qcore_schemas::handoff::{GuardOp, HandOff, HandOffGuard, RESERVED_KEYWORDS};

/// The owner keyword, the left of the maps-to.
pub(super) const OWNER: &str = "owner";
/// The callback keyword, the right of the maps-to.
pub(super) const CALLBACK: &str = "callback";
/// The guarded value's binder.
const RESULT: &str = "result";

/// The integer result types a guard may compare (unsigned and enum
/// projections admitted: `rd_kafka_op_res_t` is `u32`).
const INTEGER_ABI_TYPES: [&str; 10] = [
    "i8", "i16", "i32", "i64", "isize", "u8", "u16", "u32", "u64", "usize",
];

/// One hand-off clause as written, its constant already canonical (an alias
/// resolved to its `@` name).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Written {
    /// The binder's formal, without its `#`.
    pub parameter: String,
    pub guard: HandOffGuard,
    /// The clause, for diagnostics.
    pub text: String,
}

fn is_identifier(text: &str) -> bool {
    !text.is_empty()
        && !text.starts_with(|ch: char| ch.is_ascii_digit())
        && text
            .chars()
            .all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

/// The module's aliases, `#NAME` -> the C name of its `@` right-hand side.
/// The parser admits each name once; this rejects a name that is a reserved
/// keyword or a parameter (a binder) of any declaration of the module.
///
/// # Errors
/// The module's rejection, naming the alias.
pub(super) fn module_aliases(parsed: &SourceProgram) -> Result<BTreeMap<String, String>, String> {
    let mut aliases = BTreeMap::new();
    for alias in parsed.aliases() {
        let name = &alias.value.name;
        if RESERVED_KEYWORDS.contains(&name.as_str()) {
            return Err(format!(
                "alias-collision: `#{name}` is a reserved keyword, not an alias"
            ));
        }
        if let Some(declaration) = parsed.declarations.iter().find(|declaration| {
            declaration
                .value
                .parameters
                .iter()
                .any(|parameter| parameter.name.value == *name)
        }) {
            return Err(format!(
                "alias-collision: `#{name}` is also the binder of a parameter of `{}`",
                declaration.value.name
            ));
        }
        aliases.insert(name.clone(), alias.value.target.clone());
    }
    Ok(aliases)
}

/// Whether the text after `quant live` is a hand-off (`#P: …` with `P` not
/// `result`), as opposed to `#result` (a constructor's result state, rule
/// 14e). `quant live #P;` alone stays the result-state reader's rejection (a
/// formal's liveness part belongs to a bounds module).
pub(super) fn binds_a_formal(source: &str) -> bool {
    let Ok(tokens) = lex(source) else {
        return false;
    };
    let kind = |at: usize| tokens.get(at).map(|token| &token.kind);
    matches!(
        (kind(0), kind(1), kind(2)),
        (
            Some(TokenKind::Symbol(Symbol::Hash)),
            Some(TokenKind::Word(word)),
            Some(TokenKind::Symbol(Symbol::Colon))
        ) if word != RESULT
    )
}

/// Read the text after `quant live` of a hand-off:
/// `#P: #owner |-> #callback when #result (== | !=) (@NAME | #ALIAS)`. Every
/// other shape rejects the module.
pub(super) fn quant_live_clause(
    source: &str,
    aliases: &BTreeMap<String, String>,
) -> Result<Written, String> {
    let shape = || {
        format!(
            "quant-live-handoff-shape: a hand-off is `quant live #P: #{OWNER} {MAPS_TO} \
             #{CALLBACK} when #{RESULT} != @NAME;` (or `==`, or a module alias `#NAME`), not \
             `quant live {source};`"
        )
    };
    let tokens = lex(source)
        .map_err(|error| format!("quant-live-handoff-shape: {error}"))?
        .into_iter()
        .map(|token| token.kind)
        .filter(|kind| *kind != TokenKind::Eof)
        .collect::<Vec<_>>();
    let word = |at: usize| match tokens.get(at) {
        Some(TokenKind::Word(word)) => Some(word.as_str()),
        _ => None,
    };
    let symbol = |at: usize, expected: Symbol| tokens.get(at) == Some(&TokenKind::Symbol(expected));
    // `# P :`
    if !symbol(0, Symbol::Hash) || !symbol(2, Symbol::Colon) {
        return Err(shape());
    }
    let parameter = word(1)
        .filter(|name| is_identifier(name))
        .ok_or_else(shape)?;
    if RESERVED_KEYWORDS.contains(&parameter) {
        return Err(format!(
            "quant-live-handoff-binder: `#{parameter}` is a reserved keyword, not a formal's binder"
        ));
    }
    // `# owner |-> # callback`
    if !(symbol(3, Symbol::Hash) && word(4) == Some(OWNER)) {
        return Err(format!(
            "quant-live-handoff-owner: the left of `{MAPS_TO}` is the keyword `#{OWNER}`: {}",
            shape()
        ));
    }
    if symbol(5, Symbol::Arrow) {
        return Err(format!(
            "quant-live-maps-to: `->` is C member access, not maps-to: write `#{OWNER} {MAPS_TO} \
             #{CALLBACK}` (`{MAPS_TO}` is the ASCII `↦`)"
        ));
    }
    if !(symbol(5, Symbol::MapsTo) && symbol(6, Symbol::Hash) && word(7) == Some(CALLBACK)) {
        return Err(format!(
            "quant-live-handoff-owner: the owner maps to the keyword `#{CALLBACK}`: {}",
            shape()
        ));
    }
    // `when # result OP`
    if word(8) != Some("when") || !symbol(9, Symbol::Hash) || word(10) != Some(RESULT) {
        return Err(format!(
            "quant-live-handoff-guard: a hand-off is guarded by `when #{RESULT} OP @NAME`: {}",
            shape()
        ));
    }
    let op = if symbol(11, Symbol::EqualEqual) {
        GuardOp::Eq
    } else if symbol(11, Symbol::NotEqual) {
        GuardOp::Ne
    } else {
        return Err(format!(
            "quant-live-handoff-guard: the guard compares with `==` or `!=` only: {}",
            shape()
        ));
    };
    // `@ NAME` or `# ALIAS`, then the end.
    let constant_shape = || {
        format!(
            "quant-live-handoff-constant: the guard's constant is `@NAME` (a C enumerator) or a \
             module alias `#NAME`, never a number or a bare word: {}",
            shape()
        )
    };
    if !(symbol(12, Symbol::At) || symbol(12, Symbol::Hash)) {
        return Err(constant_shape());
    }
    if tokens.len() != 14 {
        return Err(shape());
    }
    let name = word(13)
        .filter(|name| is_identifier(name))
        .ok_or_else(constant_shape)?;
    let constant = if symbol(12, Symbol::At) {
        name.to_owned()
    } else {
        aliases.get(name).cloned().ok_or_else(|| {
            format!("quant-live-handoff-alias: `#{name}` is not an alias this module declares")
        })?
    };
    Ok(Written {
        parameter: parameter.to_owned(),
        guard: HandOffGuard { op, constant },
        text: format!("quant live {source}"),
    })
}

/// Validate one declaration's hand-off clauses against its loaded custody
/// contract: at most one, on a depth-one pointer formal that the declaration
/// states `resource_owned(P, nonnull)` (the clause's liveness part), the
/// result an integer code (`resource_return_none()`).
///
/// # Errors
/// The module's rejection.
pub(super) fn declaration(
    written: &[Written],
    contract: &ResourceFunctionContract,
    parameter_types: &[String],
    names: &BTreeMap<&str, usize>,
    result_type: &str,
) -> Result<Option<HandOff>, String> {
    let [clause] = written else {
        return if written.is_empty() {
            Ok(None)
        } else {
            Err("a declaration states one hand-off".into())
        };
    };
    let at = names
        .get(clause.parameter.as_str())
        .copied()
        .ok_or_else(|| {
            format!(
                "quant-live-handoff-binder: `{}`: `#{}` names no parameter",
                clause.text, clause.parameter
            )
        })?;
    if !matches!(
        parameter_types[at].as_str(),
        "ptr<mut, u8>" | "ptr<const, u8>"
    ) {
        return Err(format!(
            "quant-live-handoff-binder: `{}`: the handed formal is a depth-one pointer",
            clause.text
        ));
    }
    if contract.parameters[at] != (ResourceParameterMode::Owned { nullable: false }) {
        return Err(format!(
            "quant-live-unbacked: `{}`: the hand-off's liveness part needs \
             `resource_owned({}, nonnull)`: the caller hands a live object",
            clause.text, clause.parameter
        ));
    }
    if !INTEGER_ABI_TYPES.contains(&result_type) || contract.result != ResourceReturnMode::None {
        return Err(format!(
            "quant-live-handoff-result: `{}`: the guarded `#{RESULT}` is an integer or enumeration \
             code with `resource_return_none()`",
            clause.text
        ));
    }
    Ok(Some(HandOff {
        parameter: at,
        guard: clause.guard.clone(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn aliases() -> BTreeMap<String, String> {
        BTreeMap::from([("PASS".to_owned(), "RD_KAFKA_OP_RES_PASS".to_owned())])
    }

    #[test]
    fn both_spellings_read_as_one_guard() {
        let direct = quant_live_clause(
            &format!("#rko: #owner {MAPS_TO} #callback when #result != @RD_KAFKA_OP_RES_PASS"),
            &aliases(),
        )
        .expect("reads");
        let aliased = quant_live_clause(
            &format!("#rko: #owner {MAPS_TO} #callback when #result != #PASS"),
            &aliases(),
        )
        .expect("reads");
        assert_eq!(direct.guard, aliased.guard);
        assert_eq!(direct.parameter, "rko");
        assert_eq!(direct.guard.op, GuardOp::Ne);
        let keep = quant_live_clause(
            &format!("#rko: #owner {MAPS_TO} #callback when #result == @RD_KAFKA_OP_RES_KEEP"),
            &BTreeMap::new(),
        )
        .expect("reads");
        assert_eq!(keep.guard.op, GuardOp::Eq);
        assert_eq!(keep.guard.constant, "RD_KAFKA_OP_RES_KEEP");
    }

    #[test]
    fn every_other_shape_rejects_with_a_named_diagnostic() {
        let cases = [
            (
                "#rko: owner {M} #callback when #result != @K",
                "quant-live-handoff-owner:",
            ),
            (
                "#rko: #owner {M} callback when #result != @K",
                "quant-live-handoff-owner:",
            ),
            (
                "#rko: #owner -> #callback when #result != @K",
                "quant-live-maps-to:",
            ),
            (
                "#rko: #owner {M} #callback when result != @K",
                "quant-live-handoff-guard:",
            ),
            (
                "#rko: #owner {M} #callback when #result < @K",
                "quant-live-handoff-guard:",
            ),
            (
                "#rko: #owner {M} #callback when #result != K",
                "quant-live-handoff-constant:",
            ),
            (
                "#rko: #owner {M} #callback when #result != 0",
                "quant-live-handoff-constant:",
            ),
            (
                "#rko: #owner {M} #callback when #result != #NOPE",
                "quant-live-handoff-alias:",
            ),
            ("#rko: #owner {M} #callback", "quant-live-handoff-guard:"),
            (
                "#owner: #owner {M} #callback when #result != @K",
                "quant-live-handoff-binder:",
            ),
            (
                "#rko: #owner {M} #callback when #result != @K @K",
                "quant-live-handoff-shape:",
            ),
        ];
        for (text, diagnostic) in cases {
            let text = text.replace("{M}", MAPS_TO);
            let error = quant_live_clause(&text, &aliases()).expect_err(&text);
            assert!(error.starts_with(diagnostic), "{text}: {error}");
        }
    }

    #[test]
    fn a_formal_binder_is_told_from_the_result() {
        assert!(binds_a_formal(
            "#rko: #owner |-> #callback when #result != @K"
        ));
        assert!(!binds_a_formal(
            "#result: rd_kafka_op_s.rko_op_cb |-> #absent"
        ));
        assert!(!binds_a_formal("#result"));
        assert!(!binds_a_formal("#rko"));
    }
}
