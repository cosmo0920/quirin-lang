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

//! Declared literal-argument premises (cards **HASH-CONTRACTS**,
//! **HASH-CLAUSE**, **HASH-REFINE**).
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
//! whose argument at a named parameter is exactly a named integer constant.
//! Everywhere else, the contract is **not applied**: the call is treated exactly
//! as a call to an uncontracted function, which is what this analyzer already
//! does when it has no model. The premise can therefore only ever refuse call
//! sites the declaration would otherwise have covered; it can never admit one.
//! Which call-site *spellings* count as that constant — and which are refused
//! for want of the evaluation this premise deliberately lacks — is decided by
//! [`satisfies`], per §C04.5.1.
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
pub(crate) const UNMET_PREFIX: &str = "qcore_premise_unmet__";

/// The mark that follows [`UNMET_PREFIX`] when the unmet premise is a declared
/// loop-owner obligation (card **W47-LOOP-OWNER-CONTRACTS**, rule 10p). It
/// sits under the same reserved prefix, so every reader that already
/// recognises an unmet premise recognises this one, and the reserved-spelling
/// escape (rule 10i) covers it.
const LOOP_OWNER_MARK: &str = "loop_owner__";

/// The mark that follows [`UNMET_PREFIX`] when the unmet premise is a declared
/// thread-synchronization obligation (card **W47-CONDVAR-JOIN-PUBLICATION**,
/// rule 10q), under the same reserved prefix for the same reasons.
const THREAD_SYNC_MARK: &str = "thread_sync__";

/// The mark that follows [`UNMET_PREFIX`] when the unmet premise is a declared
/// lifecycle obligation (card **W47-FLB-LIB-LIFECYCLE**, rule 10q as amended,
/// contract specification C04.11), under the same reserved prefix.
const LIFECYCLE_MARK: &str = "lifecycle__";

/// The callee a rewritten name stands for, for diagnostics.
pub(super) fn unmet_callee(function: &str) -> Option<&str> {
    let callee = function.strip_prefix(UNMET_PREFIX)?;
    Some(
        callee
            .strip_prefix(LOOP_OWNER_MARK)
            .or_else(|| callee.strip_prefix(THREAD_SYNC_MARK))
            .or_else(|| callee.strip_prefix(LIFECYCLE_MARK))
            .unwrap_or(callee),
    )
}

/// The refusal wording for a rewritten callee: a declared loop-owner
/// obligation is named as such, and every literal-argument refusal keeps its
/// wording byte for byte.
pub(super) fn unmet_description(function: &str) -> Option<String> {
    let rest = function.strip_prefix(UNMET_PREFIX)?;
    if let Some(callee) = rest.strip_prefix(LIFECYCLE_MARK) {
        return Some(format!(
            "call to `{callee}` outside its declared lifecycle obligation (no derived context \
             state on this route)"
        ));
    }
    if let Some(callee) = rest.strip_prefix(THREAD_SYNC_MARK) {
        return Some(format!(
            "call to `{callee}` outside its declared thread-synchronization obligation (no \
             derived thread, predicate loop or join on this route)"
        ));
    }
    Some(match rest.strip_prefix(LOOP_OWNER_MARK) {
        Some(callee) => format!(
            "call to `{callee}` outside its declared loop-owner obligation (no derived thread \
             on this route)"
        ),
        None => format!("call to `{rest}` outside its declared literal-argument premise"),
    })
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
    /// Card W47-LOOP-OWNER-CONTRACTS: every callee with a declared loop-owner
    /// obligation. No per-unit route derives a thread, so each of their call
    /// sites is refused here, whatever its arguments.
    loop_owner: std::collections::BTreeSet<String>,
    /// Card W47-CONDVAR-JOIN-PUBLICATION: every callee with a declared
    /// thread-synchronization obligation, refused at every call for the same
    /// reason (no per-unit route derives a predicate loop or a join).
    thread_sync: std::collections::BTreeSet<String>,
    /// Card W47-FLB-LIB-LIFECYCLE: every callee with a declared lifecycle
    /// obligation, refused at every call (no per-unit route derives a
    /// context's lifecycle state).
    lifecycle: std::collections::BTreeSet<String>,
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
        if contracts
            .source_assumptions()
            .values()
            .any(|source| source.contains(qcore_contract::contract_loader::LOOP_OWNER_CLAUSE))
        {
            match qcore_contract::contract_loader::declared_loop_owner_obligations(contracts) {
                Ok(declared) => premises.loop_owner.extend(declared.into_keys()),
                Err(message) => premises.poison(&message),
            }
        }
        if contracts
            .source_assumptions()
            .values()
            .any(|source| source.contains(qcore_contract::contract_loader::THREAD_SYNC_CLAUSE))
        {
            match qcore_contract::contract_loader::declared_thread_sync_obligations(contracts) {
                Ok(declared) => premises.thread_sync.extend(declared.into_keys()),
                Err(message) => premises.poison(&message),
            }
        }
        if contracts
            .source_assumptions()
            .values()
            .any(|source| source.contains(qcore_contract::contract_loader::LIFECYCLE_CLAUSE))
        {
            match qcore_contract::contract_loader::declared_lifecycle_obligations(contracts) {
                Ok(declared) => premises.lifecycle.extend(declared.into_keys()),
                Err(message) => premises.poison(&message),
            }
        }
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
        self.required.is_empty()
            && self.loop_owner.is_empty()
            && self.thread_sync.is_empty()
            && self.lifecycle.is_empty()
            && self.malformed.is_none()
    }

    /// Whether `function` carries a declared loop-owner obligation.
    pub(super) fn loop_owner(&self, function: &str) -> bool {
        self.loop_owner.contains(function)
    }

    /// Whether `function` carries a declared thread-synchronization obligation.
    pub(super) fn thread_sync(&self, function: &str) -> bool {
        self.thread_sync.contains(function)
    }

    /// Whether `function` carries a declared lifecycle obligation.
    pub(super) fn lifecycle(&self, function: &str) -> bool {
        self.lifecycle.contains(function)
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
                .is_none_or(|actual| !satisfies(actual, &premise.literal))
        })
    }
}

/// The closed table of C integer-constant suffixes (C11 6.4.4.1): an optional
/// unsigned suffix and an optional long/long-long suffix, in either order, with
/// each half's case free but `ll`/`LL` never mixed. The empty suffix is present
/// so that an unsuffixed literal takes the same path.
///
/// A suffix fixes the constant's *type*; it does not change its value, which is
/// what the premise names. Anything outside this table — `0x0`, `0b1`, `1e5`,
/// `0i32` — is refused rather than interpreted.
const INTEGER_SUFFIXES: &[&str] = &[
    "", "u", "U", "l", "L", "ll", "LL", "ul", "uL", "Ul", "UL", "lu", "lU", "Lu", "LU", "ull",
    "uLL", "Ull", "ULL", "llu", "llU", "LLu", "LLU",
];

/// Whether a call site's actual satisfies a declared literal (card
/// **HASH-REFINE**).
///
/// The declared literal is canonical by construction — the loader admits one
/// spelling per value — but a call site may spell the same integer constant
/// several ways. Two spellings are admitted beyond exact token equality,
/// each **decided** from the token stream with no evaluation whatever:
///
/// * **matched enclosing parentheses.** `(0)` and `((0))` are the same primary
///   expression as `0`; parenthesisation is grammar, not arithmetic.
/// * **an integer suffix.** `0L`, `0u`, `0ULL` are the same value as `0` with a
///   different declared *type*; the digit run is untouched.
///
/// Everything else keeps its refusal, because deciding it would need something
/// this premise deliberately does not have:
///
/// * `0 + 0`, `1 - 1`, `-(0)` — **arithmetic**. The premise names a literal, not
///   an expression that evaluates to one.
/// * `-0` against a declared `0` — the same, one operator smaller: equating them
///   is evaluating unary minus.
/// * `0x0`, `0b0`, `00` — **a base**. Reading them needs digit interpretation,
///   and `010` is 8, so stripping a leading zero would be wrong, not merely
///   incomplete.
/// * `(void *) 0`, `(long) 0` — a **cast**, whose operand this relation does not
///   reach into.
/// * an `enum` constant, or any other identifier — **symbol resolution**. Note
///   that an object-like *macro* is not in this list: the analyzer reads
///   preprocessed text, so a macro spelling the literal has already become the
///   literal before this relation sees it.
///
/// The relation is a strict widening of exact equality — every vector that
/// matched before still matches — so it can only ever *apply* a declaration
/// where the declared premise says it should have applied all along. That is a
/// deliberate movement in the applying direction, recorded in
/// `docs/reports/remaining-hash-refine/README.md`.
fn satisfies(actual: &[String], literal: &[String]) -> bool {
    actual == literal || canonical_integer(actual).as_deref() == Some(literal)
}

/// The declared spelling an actual stands for, or `None` if this relation does
/// not decide it. The result is the loader's own canonical form: `["0"]`, or
/// `["-", "1"]`.
fn canonical_integer(actual: &[String]) -> Option<Vec<String>> {
    let unwrapped = strip_matched_parentheses(actual);
    let (sign, rest) = match unwrapped.split_first() {
        Some((first, rest)) if first == "-" => (Some("-".to_owned()), rest),
        _ => (None, unwrapped),
    };
    let [token] = rest else {
        return None;
    };
    let digits = decimal_digits(token)?;
    // `-0` is not a second spelling of `0`: the loader refuses it as a declared
    // literal, and equating it here would be evaluating the sign.
    if sign.is_some() && digits == "0" {
        return None;
    }
    Some(sign.into_iter().chain([digits.to_owned()]).collect())
}

/// Remove enclosing parentheses, and only ones that actually match each other:
/// `(0)` unwraps, `(0) + (0)` does not, and neither does the cast `(long) 0`.
fn strip_matched_parentheses(mut tokens: &[String]) -> &[String] {
    while tokens.len() > 2 && outer_parentheses_match(tokens) {
        tokens = &tokens[1..tokens.len() - 1];
    }
    tokens
}

/// Whether the first token is a `(` whose matching `)` is the last token.
fn outer_parentheses_match(tokens: &[String]) -> bool {
    if tokens.first().map(String::as_str) != Some("(") {
        return false;
    }
    let mut depth = 0usize;
    for (index, token) in tokens.iter().enumerate() {
        match token.as_str() {
            "(" => depth += 1,
            ")" => match depth.checked_sub(1) {
                Some(0) => return index + 1 == tokens.len(),
                Some(remaining) => depth = remaining,
                None => return false,
            },
            _ => {}
        }
    }
    false
}

/// The decimal digit run of one integer-constant token, if the whole token is a
/// decimal constant with an admitted suffix and no leading zero.
///
/// The gateway's C tokenizer takes a digit and every alphanumeric byte after it
/// as one token, so `0L` arrives as the single token `0L` and `0x0` as `0x0`;
/// the split is done here rather than assumed.
fn decimal_digits(token: &str) -> Option<&str> {
    let end = token
        .find(|character: char| !character.is_ascii_digit())
        .unwrap_or(token.len());
    let (digits, suffix) = token.split_at(end);
    // A leading zero is an octal constant, and this relation decides no base.
    if digits.is_empty() || (digits.len() > 1 && digits.starts_with('0')) {
        return None;
    }
    INTEGER_SUFFIXES.contains(&suffix).then_some(digits)
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
        // Card W47-LOOP-OWNER-CONTRACTS: a loop-owner obligation is never
        // discharged on this route, so its callee is refused at every call.
        if !poisoned && premises.loop_owner(callee) {
            let target = rewritten.get_or_insert_with(|| tokens.to_vec());
            target[index].text = format!("{UNMET_PREFIX}{LOOP_OWNER_MARK}{callee}");
            continue;
        }
        // Card W47-CONDVAR-JOIN-PUBLICATION: nor is a thread-synchronization
        // obligation (a predicate loop, a paired mutex, a quiescent phase or
        // a checked join), so its callee is refused at every call too.
        if !poisoned && premises.thread_sync(callee) {
            let target = rewritten.get_or_insert_with(|| tokens.to_vec());
            target[index].text = format!("{UNMET_PREFIX}{THREAD_SYNC_MARK}{callee}");
            continue;
        }
        // Card W47-FLB-LIB-LIFECYCLE: nor is a lifecycle obligation (the
        // context's derived state and the clauses checked against the
        // declared bodies), so its callee is refused at every call too.
        if !poisoned && premises.lifecycle(callee) {
            let target = rewritten.get_or_insert_with(|| tokens.to_vec());
            target[index].text = format!("{UNMET_PREFIX}{LIFECYCLE_MARK}{callee}");
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

    // ----------------------------------------------------------------------
    // Card HASH-REFINE: which call-site spellings the premise decides.
    // ----------------------------------------------------------------------

    /// Tokenize a call-site actual exactly as the gateway's C tokenizer does,
    /// so these rows are over real token vectors rather than hand-built ones.
    fn actual(spelling: &str) -> Vec<String> {
        crate::legacy_c::tokenize(spelling)
            .into_iter()
            .map(|token| token.text)
            .collect()
    }

    /// **The spelling-equivalence table.** Every row is a call-site spelling
    /// against the declared literal `0`, with the ground on which it is decided
    /// or refused. Admission is structural: parentheses and an integer suffix
    /// are erased, and nothing else is.
    #[test]
    fn the_admitted_spellings_of_the_declared_literal() {
        let zero = [String::from("0")];
        for spelling in [
            "0",     // the declared spelling itself
            "(0)",   // matched parentheses
            "((0))", // repeated, still matched
            "( 0 )", // whitespace is not a token
            "0L",    // suffix: long
            "0l", "0U", "0u", "0LL", "0ll", "0UL", "0ul", "0lu", "0LU", "0ULL", "0uLL", "0llu",
            "0LLU", // every admitted suffix spelling
            "(0L)", // both at once
            "((0ULL))",
        ] {
            assert!(
                satisfies(&actual(spelling), &zero),
                "`{spelling}` must satisfy the declared literal `0`"
            );
        }
    }

    /// The refused spellings, each with the capability it would need. None of
    /// these is admitted, and every one keeps the refusal it had before this
    /// card.
    #[test]
    fn the_refused_spellings_and_their_grounds() {
        let zero = [String::from("0")];
        for (spelling, ground) in [
            ("0 + 0", "arithmetic"),
            ("1 - 1", "arithmetic"),
            ("-0", "arithmetic: unary minus"),
            ("-(0)", "arithmetic: unary minus"),
            ("+0", "arithmetic: unary plus"),
            ("0x0", "a base"),
            ("0X0", "a base"),
            ("0b0", "a base"),
            ("00", "a base: a leading zero is octal"),
            ("(void *) 0", "a cast"),
            ("(long) 0", "a cast"),
            ("(0) + (0)", "arithmetic, and the parentheses do not match"),
            ("(0, 0)", "evaluation of a comma expression"),
            ("ZERO_CONSTANT", "symbol resolution"),
            ("'\\0'", "a character constant"),
            ("0.0", "a floating constant"),
            ("0i32", "an unrecognised suffix"),
            ("n", "symbol resolution"),
            ("sizeof(struct p_value)", "evaluation"),
        ] {
            assert!(
                !satisfies(&actual(spelling), &zero),
                "`{spelling}` must stay refused; deciding it would need {ground}"
            );
        }
    }

    /// **The widening is strict.** Anything that satisfied the old exact-token
    /// test still satisfies this one, so no call site the premise used to admit
    /// can have been withdrawn by this card.
    #[test]
    fn exact_token_equality_still_satisfies() {
        for (declared, spelling) in [
            (vec!["0".to_owned()], "0"),
            (vec!["-".to_owned(), "1".to_owned()], "-1"),
            (vec!["8".to_owned()], "8"),
            (vec!["4294967296".to_owned()], "4294967296"),
        ] {
            assert!(satisfies(&actual(spelling), &declared));
        }
    }

    /// A negative declared literal admits the same two erasures, and nothing
    /// crosses between the signs.
    #[test]
    fn a_negative_literal_admits_the_same_spellings() {
        let minus_one = [String::from("-"), String::from("1")];
        for spelling in ["-1", "(-1)", "-1L", "(-1LL)"] {
            assert!(satisfies(&actual(spelling), &minus_one), "{spelling}");
        }
        for spelling in ["1", "(1)", "1L", "- 1 - 0", "-2", "-1.0", "-01"] {
            assert!(
                !satisfies(&actual(spelling), &minus_one),
                "`{spelling}` must not satisfy the declared literal `-1`"
            );
        }
        // Nor does the negative spelling satisfy the positive declaration.
        assert!(!satisfies(&actual("-1"), &[String::from("1")]));
    }

    /// A leading zero is refused in **both** directions rather than normalised:
    /// `010` is 8, so stripping it would be a wrong answer, not a missing one.
    #[test]
    fn an_octal_spelling_is_refused_rather_than_normalised() {
        assert!(!satisfies(&actual("010"), &[String::from("10")]));
        assert!(!satisfies(&actual("010"), &[String::from("8")]));
        assert!(!satisfies(&actual("00"), &[String::from("0")]));
        // And the loader refuses to declare one, so there is no declared side
        // for it to match exactly either.
        for spelling in ["010", "-0", "00"] {
            let text = MODULE.replace("(size, 0)", &format!("(size, {spelling})"));
            let mut sources = BTreeMap::new();
            sources.insert("premise.qcore".to_owned(), text);
            assert!(
                load_resource_contract_sources(&sources).is_err(),
                "the loader admitted the declared literal `{spelling}`"
            );
        }
    }

    /// An object-like macro is **not** a refused spelling: the analyzer reads
    /// preprocessed text, so a macro naming the literal has already become the
    /// literal. This corrects HASH-CLAUSE's recorded boundary, which listed
    /// macro spellings among the refusals.
    #[test]
    fn a_macro_spelling_arrives_already_expanded() {
        // What the preprocessor hands the tokenizer for `#define ZERO 0`.
        assert!(satisfies(&actual("0"), &[String::from("0")]));
        // What it hands over for `#define ZERO (0)`, likewise decided now.
        assert!(satisfies(&actual("(0)"), &[String::from("0")]));
        // `NULL` expands to a cast on this platform, which stays refused.
        assert!(!satisfies(&actual("((void *)0)"), &[String::from("0")]));
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
