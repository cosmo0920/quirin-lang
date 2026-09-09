//! Declared literal-argument premises (cards **HASH-CONTRACTS**,
//! **HASH-CLAUSE**).
//!
//! # Why this exists
//!
//! A resource-contract v1 declaration names one custody mode per parameter and
//! says nothing about argument *values* (`docs/QCORE_CONTRACT_SPEC_r0_3.md`
//! §C04.2 has no value-dependent mode). Some real C entry points switch
//! behaviour on a scalar argument. `flb_hash_table_add(ht, key, key_len, val,
//! val_size)` is the measured instance: at `val_size == 0` the table stores the
//! caller's pointer with no copy and never frees it, and at a positive size it
//! allocates its own copy and reads `val_size` bytes through `val`. A
//! declaration read off one arm must not be believed on the other.
//!
//! A *declared literal-argument premise* restricts a declaration to call sites
//! whose argument at a named parameter is exactly a named integer literal.
//! Everywhere else, the contract is **not applied**: the call is treated exactly
//! as a call to an uncontracted function, which is what this analyzer already
//! does when it has no model. The premise can therefore only ever refuse call
//! sites the declaration would otherwise have covered; it can never admit one.
//!
//! # Where a premise is declared
//!
//! It is a clause of the declaration itself (§C04.5):
//!
//! ```text
//! requires resource_literal_argument(val_size, 0);
//! ```
//!
//! `qcore_contract::contract_loader` parses and validates it total-or-reject —
//! an unknown parameter name, a malformed literal or a second premise on one
//! parameter rejects the **whole module** at load — and
//! [`LiteralPremises::read`] rederives it from the environment's own retained
//! declared texts, which is what makes W4 summary derivation, W4 validation and
//! certificate rederivation reproduce this restriction from the same bytes.
//!
//! Card HASH-CONTRACTS originally carried the premise as a machine-parsed
//! `// qcore-premise: literal_argument(...)` comment, because the v1 clause
//! grammar was closed. That carrier is **retired**: the loader rejects a module
//! still containing its introducer by name, so a leftover directive fails loudly
//! rather than silently losing its restriction. HASH-CLAUSE measured the two
//! carriers' refusal sets equal before removing this one, and pinned the
//! measured set (`crates/qcore-c-gateway/tests/hash_contracts.rs`).
//!
//! # Fail-closed on anything unreadable
//!
//! A source set the loader cannot re-read **poisons** the whole premise set,
//! and a poisoned set refuses *every* contracted call in the analysis. That is
//! deliberately not the empty set: an empty premise set applies every
//! declaration at every call site, which is the fail-open direction.

use std::collections::BTreeMap;

use qcore_schemas::contract::ResourceContracts;

use super::Token;

/// Prefix stamped onto a callee whose declared premise the call site does not
/// satisfy. It is spelled as an ordinary C identifier so that every downstream
/// reader still sees a *call* — one to a function nothing declares — rather
/// than an unrecognised token it might drop. No corpus symbol begins with it
/// (asserted by the tests), so the rewrite can never collide with a real name.
pub(super) const UNMET_PREFIX: &str = "qcore_premise_unmet__";

/// The callee a rewritten name stands for, for diagnostics.
pub(super) fn unmet_callee(function: &str) -> Option<&str> {
    function.strip_prefix(UNMET_PREFIX)
}

/// One declared premise: parameter `index` of `function` must be exactly the
/// token sequence `literal` at every call site the contract is applied to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct LiteralArgument {
    pub(super) index: usize,
    pub(super) literal: Vec<String>,
}

/// Every literal-argument premise a contract environment declares.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct LiteralPremises {
    required: BTreeMap<String, Vec<LiteralArgument>>,
    /// Why the set is unreadable, if it is. `Some` poisons the whole set.
    malformed: Option<String>,
}

/// The clause introducer, scanned for before the sources are re-parsed.
///
/// A source that does not contain this spelling cannot declare the clause, so
/// the common case costs a scan rather than a parse. The retired
/// `qcore-premise` directive is scanned for too: the loader rejects a module
/// carrying it, but an environment assembled by other means could still hold
/// one, and a directive nothing reads any more must not silently leave a
/// declaration unrestricted here either.
const CLAUSE: &str = "resource_literal_argument";

impl LiteralPremises {
    /// Read every premise the environment's declared module texts carry.
    ///
    /// A rederivation failure over sources that mention either spelling
    /// **poisons** the set rather than yielding an empty one: an empty premise
    /// set applies every declaration at every call site, which is the fail-open
    /// direction, so a text the loader cannot read must refuse every contracted
    /// call instead.
    pub(super) fn read(contracts: &ResourceContracts) -> Self {
        let mut premises = Self::default();
        if !contracts.source_assumptions().values().any(|source| {
            source.contains(CLAUSE)
                || source.contains(qcore_contract::contract_loader::RETIRED_PREMISE_DIRECTIVE)
        }) {
            return premises;
        }
        match qcore_contract::contract_loader::declared_literal_argument_premises(contracts) {
            Ok(declared) => {
                for (function, clauses) in declared {
                    let entry = premises.required.entry(function).or_default();
                    for clause in clauses {
                        entry.push(LiteralArgument {
                            index: clause.parameter,
                            literal: clause.literal,
                        });
                    }
                }
            }
            Err(message) => premises.poison(&message),
        }
        premises
    }

    fn poison(&mut self, reason: &str) {
        if self.malformed.is_none() {
            self.malformed = Some(reason.trim().to_owned());
        }
    }

    /// Why the premise set is unreadable, if the set is poisoned.
    pub(super) fn malformed(&self) -> Option<&str> {
        self.malformed.as_deref()
    }

    #[must_use]
    pub(super) fn is_empty(&self) -> bool {
        self.required.is_empty() && self.malformed.is_none()
    }

    /// The premises declared for `function`, in declaration order.
    pub(super) fn declared(&self, function: &str) -> &[LiteralArgument] {
        self.required
            .get(function)
            .map_or(&[][..], |premises| premises.as_slice())
    }

    /// The first premise this argument vector does not satisfy.
    ///
    /// A missing argument never satisfies a premise: a call that does not even
    /// reach the declared parameter is refused rather than assumed.
    pub(super) fn unmet<'a>(
        &'a self,
        function: &str,
        arguments: Option<&[Vec<String>]>,
    ) -> Option<&'a LiteralArgument> {
        self.declared(function).iter().find(|premise| {
            arguments
                .and_then(|arguments| arguments.get(premise.index))
                .is_none_or(|actual| *actual != premise.literal)
        })
    }
}

fn is_c_identifier(text: &str) -> bool {
    let mut bytes = text.bytes();
    bytes
        .next()
        .is_some_and(|byte| byte == b'_' || byte.is_ascii_alphabetic())
        && bytes.all(|byte| byte == b'_' || byte.is_ascii_alphanumeric())
}

/// Rewrite every call whose declared premise the call site does not satisfy so
/// that the rest of the analyzer sees an uncontracted callee.
///
/// Returning a token projection rather than threading the premise set through
/// every contract lookup is deliberate: the premise then holds uniformly at
/// every consumer of the token stream — the resource CFG, the C02-P untracked
/// base seeding, the aggregate-store bases, the member-address and bound-read
/// admissions, the neutral-position reader and the out-alias normalizer — with
/// no lookup site able to miss it. It is the same technique
/// `super::recursion::contextual_cfg` uses for its checked scalar actuals.
///
/// `None` means the stream is unchanged.
pub(super) fn restrict_to_declared_literals(
    tokens: &[Token],
    contracts: &ResourceContracts,
    premises: &LiteralPremises,
) -> Option<Vec<Token>> {
    if premises.is_empty() {
        return None;
    }
    let poisoned = premises.malformed().is_some();
    let mut rewritten: Option<Vec<Token>> = None;
    for index in 0..tokens.len() {
        if tokens.get(index + 1).is_none_or(|token| token.text != "(") {
            continue;
        }
        let callee = &tokens[index].text;
        if !is_c_identifier(callee) || contracts.contract(callee).is_none() {
            continue;
        }
        if !is_call_position(tokens, index) {
            continue;
        }
        if !poisoned && premises.declared(callee).is_empty() {
            continue;
        }
        if !poisoned {
            let arguments = call_arguments(tokens, index + 1);
            if premises.unmet(callee, arguments.as_deref()).is_none() {
                continue;
            }
        }
        let target = rewritten.get_or_insert_with(|| tokens.to_vec());
        target[index].text = format!("{UNMET_PREFIX}{callee}");
    }
    rewritten
}

/// Whether the name at `index` occurs where a *call* can occur.
///
/// A declaration or definition of the same name is left alone: rewriting it
/// would rename a function this unit defines. The test is deliberately
/// conservative in the refusing direction — every position that is not clearly
/// a declarator is rewritten — because leaving a genuine call unrewritten is
/// the only direction that could apply a declaration outside its premise.
fn is_call_position(tokens: &[Token], index: usize) -> bool {
    match index.checked_sub(1).map(|previous| &tokens[previous].text) {
        // Start of the stream, or a declarator's type/pointer prefix.
        None => false,
        Some(previous) if previous == "*" => false,
        Some(previous) if previous == "return" => true,
        Some(previous) => !is_c_identifier(previous),
    }
}

/// Split a call's actual arguments at the `(` in `open` into token texts.
fn call_arguments(tokens: &[Token], open: usize) -> Option<Vec<Vec<String>>> {
    let mut depth = 0usize;
    let mut arguments = Vec::new();
    let mut current: Vec<String> = Vec::new();
    for token in &tokens[open..] {
        match token.text.as_str() {
            "(" | "[" => {
                depth += 1;
                if depth == 1 {
                    continue;
                }
            }
            ")" | "]" => {
                depth -= 1;
                if depth == 0 {
                    if !current.is_empty() || !arguments.is_empty() {
                        arguments.push(current);
                    }
                    return Some(arguments);
                }
            }
            "," if depth == 1 => {
                arguments.push(std::mem::take(&mut current));
                continue;
            }
            _ => {}
        }
        current.push(token.text.clone());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use qcore_contract::contract_loader::load_resource_contract_sources;

    fn environment(text: &str) -> ResourceContracts {
        let mut sources = BTreeMap::new();
        sources.insert("premise.qcore".to_owned(), text.to_owned());
        load_resource_contract_sources(&sources).expect("contract module loads")
    }

    /// The declaration under test, restricted by its `requires` clause.
    const MODULE: &str = r"module contract;
extern fn publish(table: ptr<mut, u8>, value: ptr<mut, u8>, size: isize) -> i32
contract {
    requires resource_contract_v1;
    requires resource_literal_argument(size, 0);
    ensures normal resource_borrowed(table);
    ensures normal resource_borrowed(value);
    ensures normal resource_borrowed(size);
    ensures normal resource_return_none();
};
";

    #[test]
    fn a_declared_premise_is_read_from_the_module_text() {
        let premises = LiteralPremises::read(&environment(MODULE));
        assert_eq!(premises.malformed(), None);
        assert_eq!(
            premises.declared("publish"),
            [LiteralArgument {
                index: 2,
                literal: vec!["0".to_owned()],
            }]
        );
    }

    /// Every malformed clause spelling rejects the **module**, so the analyzer
    /// never sees a typo that left a declaration unrestricted. This is strictly
    /// louder than the directive's poisoning, which admitted the module first.
    #[test]
    fn a_malformed_clause_rejects_the_whole_module() {
        for spelling in [
            "requires resource_literal_argument(size)",
            "requires resource_literal_argument(size, 0, 0)",
            "requires resource_literal_argument(absent, 0)",
            "requires resource_literal_argument(size, zero)",
            "requires resource_literal_argument(size, 0x0)",
            "requires resource_literal_argument(size, 0u)",
            "requires resource_literal_arguments(size, 0)",
            "requires literal_argument(size, 0)",
            "requires resource_literal_argument(size, 0) and true",
        ] {
            let text = MODULE.replace("requires resource_literal_argument(size, 0)", spelling);
            let mut sources = BTreeMap::new();
            sources.insert("premise.qcore".to_owned(), text);
            assert!(
                load_resource_contract_sources(&sources).is_err(),
                "admitted `{spelling}`"
            );
        }
    }

    /// Two clauses on one parameter reject the module.
    #[test]
    fn two_clauses_on_one_parameter_reject_the_module() {
        let text = MODULE.replace(
            "    requires resource_literal_argument(size, 0);\n",
            "    requires resource_literal_argument(size, 0);\n    requires resource_literal_argument(size, 1);\n",
        );
        let mut sources = BTreeMap::new();
        sources.insert("premise.qcore".to_owned(), text);
        assert!(load_resource_contract_sources(&sources).is_err());
    }

    /// A negative literal is the two tokens a C lexer produces for it.
    #[test]
    fn a_negative_clause_literal_is_two_tokens() {
        let text = MODULE.replace("(size, 0)", "(size, -1)");
        let premises = LiteralPremises::read(&environment(&text));
        assert_eq!(premises.malformed(), None);
        assert_eq!(
            premises.declared("publish")[0].literal,
            ["-".to_owned(), "1".to_owned()]
        );
    }

    /// A source the loader cannot re-read must refuse every contracted call,
    /// never silently declare no premise: an empty set applies the declaration
    /// everywhere, which is the fail-open direction.
    #[test]
    fn a_clause_source_that_no_longer_reloads_poisons_the_set() {
        let mut contracts = environment(MODULE);
        let mut substituted = BTreeMap::new();
        substituted.insert(
            "premise.qcore".to_owned(),
            MODULE.replace("(size, 0)", "(absent, 0)"),
        );
        contracts.set_source_assumptions(substituted);
        let premises = LiteralPremises::read(&contracts);
        assert!(premises.malformed().is_some(), "{premises:?}");
    }

    // ----------------------------------------------------------------------
    // The retired comment carrier fails loudly, in both directions.
    // ----------------------------------------------------------------------

    /// The exact directive card HASH-CONTRACTS carried in the module text.
    const RETIRED: &str = "// qcore-premise: literal_argument(publish, 2, 0)";

    /// A module still carrying the directive is **rejected by name**, so a
    /// leftover cannot quietly leave the declaration unrestricted. The message
    /// names the clause that replaced it.
    #[test]
    fn the_retired_comment_directive_rejects_the_module() {
        let text = MODULE
            .replace("    requires resource_literal_argument(size, 0);\n", "")
            .replace(
                "module contract;\n",
                &format!("module contract;\n{RETIRED}\n"),
            );
        let mut sources = BTreeMap::new();
        sources.insert("premise.qcore".to_owned(), text);
        let error = load_resource_contract_sources(&sources)
            .expect_err("the retired directive must reject the module");
        assert!(error.contains("retired"), "{error}");
        assert!(error.contains("resource_literal_argument"), "{error}");
    }

    /// The directive is refused even beside a live clause, so a module cannot
    /// keep the old carrier "for documentation" and rot.
    #[test]
    fn the_retired_directive_is_refused_beside_a_live_clause() {
        let text = MODULE.replace(
            "module contract;\n",
            &format!("module contract;\n{RETIRED}\n"),
        );
        let mut sources = BTreeMap::new();
        sources.insert("premise.qcore".to_owned(), text);
        assert!(load_resource_contract_sources(&sources).is_err());
    }

    /// An environment assembled elsewhere could still hold the directive in its
    /// retained text. Reading premises from it **poisons**, so a carrier nothing
    /// parses any more can never silently widen a declaration back.
    #[test]
    fn a_retained_text_still_holding_the_directive_poisons_the_set() {
        let mut contracts = environment(MODULE);
        let mut substituted = BTreeMap::new();
        substituted.insert(
            "premise.qcore".to_owned(),
            MODULE
                .replace("    requires resource_literal_argument(size, 0);\n", "")
                .replace(
                    "module contract;\n",
                    &format!("module contract;\n{RETIRED}\n"),
                ),
        );
        contracts.set_source_assumptions(substituted);
        let premises = LiteralPremises::read(&contracts);
        assert!(premises.malformed().is_some(), "{premises:?}");
    }

    /// An environment with no retained sources declares no premise, and says so
    /// without claiming one.
    #[test]
    fn an_environment_without_sources_declares_no_premise() {
        let premises = LiteralPremises::read(&ResourceContracts::default());
        assert!(premises.is_empty());
    }

    #[test]
    fn an_absent_argument_never_satisfies_a_premise() {
        let premises = LiteralPremises::read(&environment(MODULE));
        assert!(premises.unmet("publish", None).is_some());
        assert!(
            premises
                .unmet(
                    "publish",
                    Some(&[vec!["t".to_owned()], vec!["v".to_owned()]])
                )
                .is_some()
        );
    }
}
