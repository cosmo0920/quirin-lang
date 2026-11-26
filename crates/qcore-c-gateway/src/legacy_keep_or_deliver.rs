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

//! Card **OUT-SLOT-CONDITIONAL-DELIVERY** (gateway rule 85): the route-G side
//! of the keep-or-deliver summary of an in-out slot
//! ([`qcore_schemas::keep_or_deliver`]).
//!
//! # The two entry seeds
//!
//! The callee's validation (`legacy_summary`'s `keep_or_deliver` child) runs
//! its body twice on the CFG built with the delivered contract as its own,
//! from two entries that differ only in what the slot `*out` holds:
//! [`seed_null`] (the null constant) and [`seed_owned`] (a fresh live,
//! non-null, caller-held site, which is then exempt from the body's leak
//! reporting exactly as a bound formal is). A body that keeps the slot when
//! it is non-null and stores only when it is null passes both: the first run
//! is W4's `Out` delivery, the second leaves the caller's site in the slot,
//! live, at every exit.
//!
//! # The caller's reading
//!
//! A whole-statement call `F(…, &NAME, …);` (or `D = F(…);`) to a function
//! whose keep-or-deliver summary is in force, refused by the classifier as an
//! uncontracted call, is lowered to the branch the callee itself takes:
//!
//! ```text
//! if (NAME == NULL) F(…, &NAME, …);   // the delivered contract: Out
//! else              F(…, NAME, …);    // the kept contract: Owned, non-null
//! ```
//!
//! each arm classified under a minted name ([`NULL_PREFIX`], [`KEPT_PREFIX`])
//! that no source spelling reaches, then renamed to `F`. The branch is a
//! `BranchOnNull` on `NAME`, the value the callee's slot holds at entry, so
//! the flow decides it: a caller that knows `NAME` null takes custody of a
//! fresh object; one that holds a live object of its own keeps it, the call
//! requiring it live and non-null; anything else (an untracked value, a
//! borrowed member, an unbound name) reaches the kept arm's `Owned` position
//! with no live identity and refuses there, so the row stays `Unknown`.
//! Every other call to such a function (an actual other than `&NAME`, a call
//! inside an expression) is refused by name ([`call_refused`]).
//!
//! The Lean model is `QCore.OutSlotKeepOrDeliver`.

use qcore_schemas::contract::ResourceContracts;
use qcore_schemas::keep_or_deliver::KeepOrDeliverSummary;

use super::LIVE;
use super::transfer::{AbstractPointer, Derivation};
use super::{
    EntryCustody, LegacyCfg, LegacyInstruction, Parser, PointerNullCondition, RefusedWrites,
    Statement, Token, call_argument_spans, is_null_constant, strip_parentheses,
};

/// The prefix the delivered contract is installed under (the null arm).
pub(crate) const NULL_PREFIX: &str = "qcore_keep_or_deliver_null__";
/// The prefix the kept contract is installed under (the non-null arm).
pub(crate) const KEPT_PREFIX: &str = "qcore_keep_or_deliver_kept__";

/// The refusal of a call to a keep-or-deliver function outside the reading.
pub(crate) fn call_refused(callee: &str) -> String {
    format!(
        "call to `{callee}`, whose summary keeps or delivers its in-out slot, outside the \
         direct-call reading (an actual other than `&NAME`)"
    )
}

/// Seed the slot of the `Out` formal at `slot` with the null constant.
/// `None` when the entry names no slot there.
pub(crate) fn seed_null(entry: &mut EntryCustody, slot: usize) -> Option<()> {
    let place = entry.out_slots.get(slot)?.clone()?;
    entry.state.pointers.insert(
        place,
        AbstractPointer {
            may_be_null: true,
            ..AbstractPointer::default()
        },
    );
    Some(())
}

/// Seed the slot of the `Out` formal at `slot` with a fresh site above every
/// site `cfg` and the entry use: a live, non-null, caller-held heap object,
/// observed at every exit as that formal's parameter site. `None` when the
/// entry names no slot there or the formal already carries a site.
pub(crate) fn seed_owned(entry: &mut EntryCustody, cfg: &LegacyCfg, slot: usize) -> Option<usize> {
    let place = entry.out_slots.get(slot)?.clone()?;
    if entry.parameter_sites.get(slot)?.is_some() {
        return None;
    }
    let used = entry
        .state
        .resources
        .keys()
        .chain(entry.parameter_sites.iter().flatten())
        .chain(entry.derived_bases.keys())
        .copied()
        .max();
    let site = super::transfer::first_free_site(cfg)
        .checked_add(entry.parameter_sites.len())?
        .max(used.map_or(Some(0), |used| used.checked_add(1))?);
    entry.state.resources.insert(site, LIVE);
    entry.state.pointers.insert(
        place,
        AbstractPointer {
            targets: std::collections::BTreeSet::from([site]),
            unknown: false,
            may_be_null: false,
            derivation: Derivation::Identity,
        },
    );
    entry.parameter_sites[slot] = Some(site);
    Some(site)
}

thread_local! {
    /// The `(function, *NAME)` slot whose body the validation is building,
    /// while it builds it ([`with_slot`]); `None` everywhere else.
    static VALIDATING: std::cell::RefCell<Option<(String, String)>> =
        const { std::cell::RefCell::new(None) };
}

/// Run `build` with `function`'s slot `place` (`*NAME`) under validation:
/// a null test of exactly that slot in exactly that body is then read as a
/// branch on the slot's binding ([`Parser::slot_null_condition`]). Every
/// other build, the ordinary W4 routes and the reported run included, reads
/// conditions exactly as before.
pub(crate) fn with_slot<T>(function: &str, place: &str, build: impl FnOnce() -> T) -> T {
    /// Restores the enclosing value, on unwinding too.
    struct Restore(Option<(String, String)>);
    impl Drop for Restore {
        fn drop(&mut self) {
            let previous = self.0.take();
            VALIDATING.with(|slot| *slot.borrow_mut() = previous);
        }
    }
    let _restore = Restore(
        VALIDATING.with(|slot| slot.replace(Some((function.to_owned(), place.to_owned())))),
    );
    build()
}

/// `(NAME, true_is_null)` for a null test of `* NAME`: `* NAME`, `! COND`,
/// `COND == NULL`, `COND != NULL`, `NULL == COND`, `NULL != COND`, each
/// possibly parenthesised.
fn slot_test(texts: &[&str]) -> Option<(String, bool)> {
    let texts = strip_parentheses(texts);
    if let Some(rest) = texts.strip_prefix(&["!"]) {
        let (name, true_is_null) = slot_test(rest)?;
        return Some((name, !true_is_null));
    }
    if let ["*", name] = texts
        && crate::typed_field_access::is_name(name)
    {
        return Some(((*name).to_owned(), false));
    }
    for at in 0..texts.len() {
        // The stream spells `==` and `!=` as one token or as two.
        let (equal, width) = match (texts[at], texts.get(at + 1).copied()) {
            ("==", _) => (true, 1),
            ("!=", _) => (false, 1),
            ("=", Some("=")) => (true, 2),
            ("!", Some("=")) if at > 0 => (false, 2),
            _ => continue,
        };
        let (left, right) = (&texts[..at], &texts[at + width..]);
        let tested = if is_null_constant(right) {
            left
        } else if is_null_constant(left) {
            right
        } else {
            return None;
        };
        let (name, true_is_null) = slot_test(tested)?;
        // `*NAME == NULL` is true when null; `!=` when not.
        return Some((name, if equal { !true_is_null } else { true_is_null }));
    }
    None
}

impl Parser<'_> {
    /// The null test of the slot under validation, as a branch on the slot's
    /// binding; `None` for every other condition and every other build.
    pub(super) fn slot_null_condition(&self, condition: &[Token]) -> Option<PointerNullCondition> {
        let (function, place) = VALIDATING.with(|slot| slot.borrow().clone())?;
        if function != self.typed_path_function {
            return None;
        }
        let texts = condition
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        let (name, true_is_null) = slot_test(&texts)?;
        let source = format!("*{name}");
        (source == place).then_some(PointerNullCondition {
            source,
            true_is_null,
            validity: None,
            result_test: None,
        })
    }
}

/// The parser's environment with both readings of every keep-or-deliver
/// summary the stream names installed under their minted names, or `None`
/// when there is none.
pub(super) fn minted_environment(
    tokens: &[Token],
    contracts: &ResourceContracts,
) -> Option<ResourceContracts> {
    let summaries = contracts.keep_or_deliver_summaries();
    if summaries.is_empty()
        || !tokens
            .iter()
            .any(|token| summaries.contains_key(&token.text))
    {
        return None;
    }
    let mut environment = contracts.clone();
    for (name, summary) in summaries {
        let Some(kept) = summary.kept() else {
            continue;
        };
        environment.insert_contract(format!("{NULL_PREFIX}{name}"), summary.delivered.clone());
        environment.insert_contract(format!("{KEPT_PREFIX}{name}"), kept);
    }
    Some(environment)
}

/// The callee's position in `F ( … )` or `D = F ( … )` with a plain `D`,
/// when the actual at the summary's slot is exactly `& NAME`.
fn call_shape(texts: &[&str], summary: &KeepOrDeliverSummary) -> Option<usize> {
    let callee = match texts {
        [_, "(", .., ")"] => 0,
        [destination, "=", _, "(", .., ")"] if crate::typed_field_access::is_name(destination) => 2,
        _ => return None,
    };
    let spans = call_argument_spans(texts, callee)?;
    // The call's closing parenthesis ends the statement.
    if spans.last().map(|(_, end)| *end) != Some(texts.len() - 1)
        || spans.len() != summary.delivered.parameters.len()
    {
        return None;
    }
    let (start, end) = *spans.get(summary.slot)?;
    match &texts[start..end] {
        ["&", name] if crate::typed_field_access::is_name(name) => Some(callee),
        _ => None,
    }
}

impl Parser<'_> {
    /// Rule 85's direct-call route: `instruction`, the lowering of a
    /// whole-statement call to a function whose keep-or-deliver summary is in
    /// force, refused as an uncontracted call. `Some` is the branch on the
    /// slot actual with each arm classified under its reading, or the named
    /// refusal [`call_refused`].
    pub(super) fn keep_or_deliver_call(
        &mut self,
        tokens: &[Token],
        line: usize,
        instruction: &LegacyInstruction,
    ) -> Option<Statement> {
        let LegacyInstruction::Unknown {
            description,
            writes,
        } = instruction
        else {
            return None;
        };
        let texts = tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        let callee_at = match texts.as_slice() {
            [_, "(", .., ")"] => 0,
            [_, "=", _, "(", .., ")"] => 2,
            _ => return None,
        };
        let callee = texts[callee_at].to_owned();
        if *description != super::uncontracted_access::uncontracted_refusal(&callee) {
            return None;
        }
        let summary = self.contracts.keep_or_deliver(&callee)?.clone();
        let refusal = || {
            Statement::Instruction(
                LegacyInstruction::Unknown {
                    description: call_refused(&callee),
                    writes: writes.clone(),
                },
                line,
            )
        };
        let Some(at) = call_shape(&texts, &summary) else {
            return Some(refusal());
        };
        let spans = call_argument_spans(&texts, at).expect("read by call_shape");
        let (slot_start, _) = spans[summary.slot];
        // The null arm: the call as written, under the delivered contract.
        let mut delivered = tokens.to_vec();
        delivered[at].text = format!("{NULL_PREFIX}{callee}");
        // The kept arm: the slot actual read as the object itself.
        let mut kept = tokens.to_vec();
        kept[at].text = format!("{KEPT_PREFIX}{callee}");
        kept.remove(slot_start);
        let saved = self.probe_state();
        let condition = [tokens[slot_start + 1].clone()];
        let arms = (|| {
            let null_condition = self.null_condition(&condition)?;
            if null_condition.validity.is_some() || null_condition.result_test.is_some() {
                return None;
            }
            let mut arm = |synthetic: &[Token], minted: String| {
                let mut classified = self.classify_simple(synthetic);
                match &mut classified {
                    LegacyInstruction::ContractCall { function, .. } if *function == minted => {
                        function.clone_from(&callee);
                        Some(classified)
                    }
                    _ => None,
                }
            };
            let delivered = arm(&delivered, format!("{NULL_PREFIX}{callee}"))?;
            let kept = arm(&kept, format!("{KEPT_PREFIX}{callee}"))?;
            Some((null_condition, delivered, kept))
        })();
        let Some((null_condition, delivered, kept)) = arms else {
            self.restore_probe_state(&saved);
            return Some(refusal());
        };
        let (null_body, kept_body) = (
            vec![Statement::Instruction(delivered, line)],
            vec![Statement::Instruction(kept, line)],
        );
        // `NAME` reads true when it is not null.
        let (then_body, else_body) = if null_condition.true_is_null {
            (null_body, kept_body)
        } else {
            (kept_body, null_body)
        };
        Some(Statement::If {
            line,
            condition_supported: true,
            condition_call: None,
            condition_access: None,
            writes: RefusedWrites::Unbounded,
            null_condition: Some(null_condition),
            distinct_condition: None,
            element_condition: None,
            condition_plan: None,
            cell_test: None,
            then_body,
            else_body,
        })
    }
}
