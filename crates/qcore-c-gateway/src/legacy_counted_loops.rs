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

//! Card **CG27-LOOPS-RECURSION-BUDGETS** (gateway rule 11x): the counted-user
//! pass's loop summary.
//!
//! Before this card a `while`, a `for` or a `do` whose condition is not the
//! literal zero left every clause of its function undecided (gateway
//! constraint 10e). A loop has unboundedly many paths, so the pass cannot
//! enumerate it; it can only carry a *summary* whose invariant it checks.
//!
//! **The invariant.** Every tracked count, every pending release check and
//! every possibly-destroyed mark equals its value at the loop's head. It holds
//! on entry trivially. It is preserved by one iteration when no token of the
//! loop (condition, step and body, nested statements included) performs a
//! counted-user operation:
//!
//! * no direct write to a declared counter member;
//! * no call whose contract in force declares a counted-user operation or exit
//!   clause, wherever it stands;
//! * every other named call is *neutral* as [`Unit::call_effect`] decides it
//!   for a callee anywhere else (a declared neutrality its body verifies, a body
//!   here that is recursively neutral, or a declaration without counted-user
//!   clauses, premise UC-FRAME);
//! * no call through an expression, and no `return` or `goto`, which would
//!   leave the loop on a path this summary does not describe.
//!
//! The check is flow-insensitive, so it holds for every interleaving of body
//! paths and every number of iterations, zero included: the loop's summary is
//! the identity on the counts. Its only other effect is on the scalar values
//! the pass reads to prune branches: every name the loop spells is forgotten,
//! since the loop may assign it. `break` and `continue` stay inside the loop
//! and are admitted.
//!
//! **Refusal.** A loop whose tokens break the invariant is refused exactly as
//! before (every clause of the function undecided, with the pre-existing
//! wording), never assumed. A loop whose extent cannot be read is refused the
//! same way. `QCore.LoopInvariants.CountedLoop` proves the summary
//! (`an_admitted_loop_preserves_delta`, `an_admitted_loop_preserves_pending`)
//! and the refusals.

use std::collections::BTreeSet;

use super::{
    CallEffect, KEYWORDS, Unit, close_of, counter_write, is_identifier, opaque_callee,
    statement_end,
};

/// The end (exclusive) of the statement starting at `start`, read through
/// nested control statements; `None` when it cannot be read.
pub(super) fn statement_extent(texts: &[&str], start: usize, end: usize) -> Option<usize> {
    if start >= end {
        return None;
    }
    let parenthesised = |at: usize| -> Option<usize> {
        (texts.get(at) == Some(&"("))
            .then(|| close_of(texts, at))
            .flatten()
            .filter(|close| *close < end)
    };
    match texts[start] {
        "{" => close_of(texts, start)
            .filter(|close| *close < end)
            .map(|close| close + 1),
        "if" => {
            let close = parenthesised(start + 1)?;
            let after = statement_extent(texts, close + 1, end)?;
            if texts.get(after) == Some(&"else") {
                statement_extent(texts, after + 1, end)
            } else {
                Some(after)
            }
        }
        "while" | "for" | "switch" => {
            let close = parenthesised(start + 1)?;
            statement_extent(texts, close + 1, end)
        }
        "do" => {
            let body = statement_extent(texts, start + 1, end)?;
            if texts.get(body) != Some(&"while") {
                return None;
            }
            let close = parenthesised(body + 1)?;
            (texts.get(close + 1) == Some(&";")).then_some(close + 2)
        }
        _ => {
            let stop = statement_end(texts, start, end);
            (stop > start && texts.get(stop - 1) == Some(&";")).then_some(stop)
        }
    }
}

/// Check the loop occupying `texts[range.0..range.1]` against the invariant.
/// On success, the names the loop spells, whose values it may change.
pub(super) fn neutral_loop(
    unit: &Unit<'_>,
    texts: &[&str],
    (start, end): (usize, usize),
) -> Result<BTreeSet<String>, String> {
    let mut names = BTreeSet::new();
    for index in start..end {
        let text = texts[index];
        if let Some(write) = counter_write(texts, index, &unit.counters) {
            return Err(format!("writes the counter (`{}`)", write.spelling));
        }
        if let Some(reason) = opaque_callee(texts, index) {
            return Err(reason);
        }
        match text {
            "return" => return Err("returns from inside the loop".into()),
            "goto" => return Err("jumps out of the loop".into()),
            _ => {}
        }
        if !is_identifier(text) {
            continue;
        }
        if texts.get(index + 1) == Some(&"(") && !KEYWORDS.contains(&text) {
            if let Some(contract) = unit.contracts.contract(text)
                && (!contract.counted_users.effects.is_empty()
                    || !contract.counted_users.exits.is_empty())
            {
                return Err(format!(
                    "performs a counted-user operation through `{text}`"
                ));
            }
            if let CallEffect::Opaque(reason) = unit.call_effect(text, &mut BTreeSet::new()) {
                return Err(format!("calls `{text}`: {reason}"));
            }
            continue;
        }
        names.insert(text.to_owned());
    }
    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(source: &str) -> Vec<&str> {
        source.split_whitespace().collect()
    }

    #[test]
    fn loop_extents_read_through_nested_statements() {
        let texts = words(
            "while ( i < n ) { if ( a ) f ( ) ; else { g ( ) ; } } x = 1 ; \
             for ( i = 0 ; i < n ; i ++ ) h ( i ) ; \
             do { i ++ ; } while ( i < n ) ; \
             while ( p ) while ( q ) r ( ) ;",
        );
        let first = statement_extent(&texts, 0, texts.len()).unwrap();
        assert_eq!(texts[first..first + 4], ["x", "=", "1", ";"]);
        let second = statement_extent(&texts, first + 4, texts.len()).unwrap();
        assert_eq!(texts[second], "do");
        let third = statement_extent(&texts, second, texts.len()).unwrap();
        assert_eq!(texts[third], "while");
        assert_eq!(
            statement_extent(&texts, third, texts.len()),
            Some(texts.len())
        );
    }

    #[test]
    fn unreadable_extents_are_refused() {
        for source in [
            "while ( i < n )",
            "while i < n ) ;",
            "do { i ++ ; } while ( i ) x",
            "do { i ++ ; } until ( i ) ;",
            "for ( ; ; ) { f ( ) ;",
        ] {
            let texts = words(source);
            assert_eq!(statement_extent(&texts, 0, texts.len()), None, "{source}");
        }
    }
}
