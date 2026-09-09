//! Declared literal-argument premises (card **HASH-CONTRACTS**).
//!
//! # Why this exists
//!
//! A resource-contract v1 declaration names one custody mode per parameter and
//! says nothing about argument *values* (`docs/QCORE_CONTRACT_SPEC_r0_3.md`
//! §C04.2 has no value-dependent mode; §C04.4 records that success-dependent
//! parameter modes are outside v1). Some real C entry points switch behaviour on
//! a scalar argument. `flb_hash_table_add(ht, key, key_len, val, val_size)` is
//! the measured instance: at `val_size == 0` the table stores the caller's
//! pointer with no copy and never frees it, and at a positive size it allocates
//! its own copy and reads `val_size` bytes through `val`. A declaration read off
//! one arm must not be believed on the other.
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
//! Card **HASH-CLAUSE** promotes the premise to a first-class v1 clause:
//!
//! ```text
//! requires resource_literal_argument(val_size, 0);
//! ```
//!
//! `qcore_contract::contract_loader` parses and validates it total-or-reject —
//! an unknown parameter name, a malformed literal or a second premise on one
//! parameter rejects the **whole module** at load — and
//! [`LiteralPremises::read_clauses`] rederives it from the environment's own
//! retained texts, which is what makes W4 derivation, W4 validation and
//! certificate rederivation reproduce this restriction from the same bytes. A
//! rederivation failure poisons the set rather than emptying it, because an
//! empty set is the fail-open direction.
//!
//! The paragraphs below describe the **retiring** carrier, which this module
//! still reads during the migration.
//!
//! # The retiring comment carrier, and why it was a comment
//!
//! The premise is a line of the contract module's own text:
//!
//! ```text
//! // qcore-premise: literal_argument(flb_hash_table_add, 4, 0)
//! ```
//!
//! It is not a contract clause. The v1 clause grammar is total-or-reject and
//! closed (`crates/qcore-contract/src/contract_loader.rs`): an unknown clause
//! introducer rejects the whole module, and representing a new clause would
//! require a new `qcore_schemas::contract` variant — a published-schema change.
//! The directive instead rides in the module text that
//! `ResourceContracts::source_assumptions` already retains verbatim, so it
//! travels inside every safety certificate, is reloaded by
//! `check_source_assumptions`, and is reproduced identically by W4 summary
//! derivation, by validation, and by certificate rederivation, all of which
//! hold that same text. The deviation is recorded in
//! `docs/reports/remaining-hash-contracts/README.md`; a first-class clause is
//! the named follow-on `HASH-CONTRACTS-CLAUSE`.
//!
//! # Total-or-reject, and fail-closed on malformation
//!
//! A directive must match the grammar exactly, name a function the environment
//! declares, and name a parameter index inside that function's declared
//! parameter list. Anything else **poisons** the whole premise set, and a
//! poisoned set refuses *every* contracted call in the analysis rather than
//! silently applying the contracts a typo left unrestricted.

use std::collections::BTreeMap;

use qcore_schemas::contract::ResourceContracts;

use super::Token;

/// The exact directive introducer. A line is a directive when its trimmed text
/// starts with `//`, and the remainder after the slashes starts with this.
const DIRECTIVE: &str = "qcore-premise:";

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
    /// The first malformed directive, verbatim. `Some` poisons the whole set.
    malformed: Option<String>,
}

/// The clause introducer, scanned for before the sources are re-parsed.
const CLAUSE: &str = "resource_literal_argument";

impl LiteralPremises {
    /// Read every premise the environment's declared module texts carry, from
    /// **both** carriers during the migration: the first-class
    /// `requires resource_literal_argument(p, LITERAL);` clause (card
    /// HASH-CLAUSE) and the retiring comment directive (card HASH-CONTRACTS).
    ///
    /// Merging is sound because either carrier can only *withdraw* call sites:
    /// the union of two premise sets refuses at least what each refuses alone,
    /// and two premises on one parameter poison rather than compete.
    pub(super) fn read(contracts: &ResourceContracts) -> Self {
        let mut premises = Self::default();
        premises.read_clauses(contracts);
        for source in contracts.source_assumptions().values() {
            for line in source.lines() {
                let Some(body) = directive_body(line) else {
                    continue;
                };
                match parse_directive(body, contracts) {
                    Ok((function, premise)) => premises.declare(line, function, premise),
                    Err(()) => premises.poison(line),
                }
            }
        }
        premises
    }

    /// The premises the loader's own clause vocabulary declares.
    ///
    /// A rederivation failure over sources that mention the clause **poisons**
    /// the set rather than yielding an empty one: an empty premise set applies
    /// every declaration everywhere, which is the fail-open direction, so a
    /// text this loader cannot read must refuse every contracted call instead.
    fn read_clauses(&mut self, contracts: &ResourceContracts) {
        if !contracts
            .source_assumptions()
            .values()
            .any(|source| source.contains(CLAUSE))
        {
            return;
        }
        match qcore_contract::contract_loader::declared_literal_argument_premises(contracts) {
            Ok(declared) => {
                for (function, premises) in declared {
                    for premise in premises {
                        self.declare(
                            CLAUSE,
                            function.clone(),
                            LiteralArgument {
                                index: premise.parameter,
                                literal: premise.literal,
                            },
                        );
                    }
                }
            }
            Err(message) => self.poison(&message),
        }
    }

    /// Record one premise, poisoning if the parameter already carries one.
    fn declare(&mut self, origin: &str, function: String, premise: LiteralArgument) {
        let declared = self.required.entry(function).or_default();
        if declared.iter().any(|other| other.index == premise.index) {
            self.poison(origin);
            return;
        }
        declared.push(premise);
    }

    fn poison(&mut self, line: &str) {
        if self.malformed.is_none() {
            self.malformed = Some(line.trim().to_owned());
        }
    }

    /// The first malformed directive, if the set is poisoned.
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

/// The text after `//` when `line` is a premise directive.
fn directive_body(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    let comment = trimmed.strip_prefix("//")?.trim_start();
    Some(comment.strip_prefix(DIRECTIVE)?.trim())
}

/// `literal_argument(NAME, INDEX, LITERAL)`, exactly. Nothing else is admitted,
/// and every field is checked against the environment that declared it.
fn parse_directive(
    body: &str,
    contracts: &ResourceContracts,
) -> Result<(String, LiteralArgument), ()> {
    let inner = body
        .strip_prefix("literal_argument(")
        .and_then(|rest| rest.strip_suffix(')'))
        .ok_or(())?;
    let fields = inner.split(',').map(str::trim).collect::<Vec<_>>();
    let [function, index, literal] = fields.as_slice() else {
        return Err(());
    };
    if !is_c_identifier(function) {
        return Err(());
    }
    let contract = contracts.contract(function).ok_or(())?;
    let index = index.parse::<usize>().map_err(|_| ())?;
    if index >= contract.parameters.len() {
        return Err(());
    }
    Ok((
        (*function).to_owned(),
        LiteralArgument {
            index,
            literal: literal_tokens(literal).ok_or(())?,
        },
    ))
}

/// A decimal integer literal, optionally negated, as the tokens a C lexer
/// produces for it. No suffixes, no other bases, no whitespace: a spelling this
/// table cannot compare exactly is rejected rather than approximated.
fn literal_tokens(text: &str) -> Option<Vec<String>> {
    let (sign, digits) = match text.strip_prefix('-') {
        Some(rest) => (Some("-".to_owned()), rest),
        None => (None, text),
    };
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    Some(sign.into_iter().chain([digits.to_owned()]).collect())
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

    const MODULE: &str = r"module contract;
// qcore-premise: literal_argument(publish, 2, 0)
extern fn publish(table: ptr<mut, u8>, value: ptr<mut, u8>, size: isize) -> i32
contract {
    requires resource_contract_v1;
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

    #[test]
    fn a_premise_naming_an_undeclared_function_poisons_the_set() {
        let text = MODULE.replace("literal_argument(publish,", "literal_argument(absent,");
        let premises = LiteralPremises::read(&environment(&text));
        assert!(premises.malformed().is_some(), "{premises:?}");
    }

    #[test]
    fn a_premise_naming_a_parameter_beyond_the_signature_poisons_the_set() {
        let text = MODULE.replace("publish, 2, 0", "publish, 3, 0");
        let premises = LiteralPremises::read(&environment(&text));
        assert!(premises.malformed().is_some(), "{premises:?}");
    }

    #[test]
    fn a_misspelled_directive_poisons_the_set_rather_than_being_ignored() {
        for spelling in [
            "literal_argument(publish, 2)",
            "literal_argument(publish, 2, 0",
            "literal_argument(publish, two, 0)",
            "literal_argument(publish, 2, 0x0)",
            "literal_argument(publish, 2, 0u)",
            "literal_arguments(publish, 2, 0)",
            "",
        ] {
            let text = MODULE.replace("literal_argument(publish, 2, 0)", spelling);
            let premises = LiteralPremises::read(&environment(&text));
            assert!(premises.malformed().is_some(), "admitted `{spelling}`");
        }
    }

    #[test]
    fn two_premises_on_one_parameter_poison_the_set() {
        let text = MODULE.replace(
            "// qcore-premise: literal_argument(publish, 2, 0)",
            "// qcore-premise: literal_argument(publish, 2, 0)\n// qcore-premise: literal_argument(publish, 2, 1)",
        );
        let premises = LiteralPremises::read(&environment(&text));
        assert!(premises.malformed().is_some(), "{premises:?}");
    }

    #[test]
    fn a_negative_literal_is_two_tokens() {
        let text = MODULE.replace("publish, 2, 0", "publish, 2, -1");
        let premises = LiteralPremises::read(&environment(&text));
        assert_eq!(premises.malformed(), None);
        assert_eq!(
            premises.declared("publish")[0].literal,
            ["-".to_owned(), "1".to_owned()]
        );
    }

    // ----------------------------------------------------------------------
    // Card HASH-CLAUSE: the first-class clause, and its parity with the
    // directive it replaces.
    // ----------------------------------------------------------------------

    /// The same declaration, restricted by the clause instead of the directive.
    const CLAUSE_MODULE: &str = r"module contract;
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
    fn a_declared_clause_is_read_from_the_module_text() {
        let premises = LiteralPremises::read(&environment(CLAUSE_MODULE));
        assert_eq!(premises.malformed(), None);
        assert_eq!(
            premises.declared("publish"),
            [LiteralArgument {
                index: 2,
                literal: vec!["0".to_owned()],
            }]
        );
    }

    /// **The migration parity measurement.** The clause and the directive
    /// produce the same premise set, so `restrict_to_declared_literals` — which
    /// is not touched by this card — refuses exactly the same call sites under
    /// either carrier.
    #[test]
    fn the_clause_and_the_directive_declare_the_same_premise_set() {
        let directive = LiteralPremises::read(&environment(MODULE));
        let clause = LiteralPremises::read(&environment(CLAUSE_MODULE));
        assert_eq!(directive, clause);
    }

    /// Declaring the same restriction twice, once per carrier, is a duplicate
    /// declaration and poisons — it is not silently deduplicated.
    #[test]
    fn declaring_the_premise_in_both_carriers_poisons_the_set() {
        let text = CLAUSE_MODULE.replace(
            "module contract;\n",
            "module contract;\n// qcore-premise: literal_argument(publish, 2, 0)\n",
        );
        let premises = LiteralPremises::read(&environment(&text));
        assert!(premises.malformed().is_some(), "{premises:?}");
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
            let text =
                CLAUSE_MODULE.replace("requires resource_literal_argument(size, 0)", spelling);
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
        let text = CLAUSE_MODULE.replace(
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
        let text = CLAUSE_MODULE.replace("(size, 0)", "(size, -1)");
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
        let mut contracts = environment(CLAUSE_MODULE);
        let mut substituted = BTreeMap::new();
        substituted.insert(
            "premise.qcore".to_owned(),
            CLAUSE_MODULE.replace("(size, 0)", "(absent, 0)"),
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
