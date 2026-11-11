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

//! Declared contracts awaiting human review (card
//! **W45-CONTRACT-DEFERRED-REVIEW**, contract spec §C06).
//!
//! # What a review marker means here
//!
//! A contract module may mark a declaration, or one of its clauses, as
//! awaiting human review:
//!
//! ```text
//! requires resource_review_pending("the size argument is unverified");
//! ensures normal resource_review_pending(resource_released(p, nonnull), "reason");
//! ```
//!
//! `qcore_contract::contract_loader` validates the declaration in full and then
//! **withholds** it: the contract is recorded in
//! `ResourceContracts::pending_review` and is never an entry of the function
//! map. A pending contract discharges nothing and accuses nothing, so every
//! call site that would have applied it stays `Unknown`, and the refusal is
//! attributed to the review rather than to an absent declaration.
//!
//! # Why the whole contract, even for one clause
//!
//! A resource-v1 contract is applied at a call site as **one conjunction**:
//! every parameter mode and the result mode together decide the call's effect,
//! and no consumer applies part of a contract. Dropping only the deferred
//! clause would let the remaining, possibly weaker, clauses decide the call —
//! a deferred `resource_released(p)` dropped from a contract whose other clauses
//! still apply would read as a call that takes no custody of `p`. So a
//! clause-level marker holds exactly the call sites a contract-level marker
//! holds; the clause is what the report names, not a narrower refusal.
//!
//! # How the hold is applied
//!
//! Exactly as a withdrawn declaration is (card LOADER-ABI): every call site of
//! a pending callee is rewritten to a marked, undeclared name, so the hold is
//! seen uniformly by every reader of the token stream, and the statement that
//! calls it refuses with [`hold_description`]. The rewrite is decided from the
//! token stream and the environment alone, so W4 derivation, W4 validation and
//! `check_legacy_safety_certificate` reproduce it. A body *defined* in the unit
//! under a pending name is analysed as an undeclared definition — the pending
//! contract is neither its interface nor its accuser — and its callers are held
//! whatever that body's analysis or a W4 summary of it would say.
//!
//! Monotone in the refusing direction: a held call is an unmodelled call, which
//! is `Unknown` and poisons its function's state, and a leak alleged through
//! the refused hand-on of a held release is withdrawn by the refused-hand-on
//! rule (card W44-LEAK-PIN-AUDIT), so no verdict moves towards `Safe` or
//! towards `Unsafe` because of a pending contract.

use qcore_schemas::contract::ResourceContracts;

use super::Token;
use super::declared_abi::is_call_position;

/// The stable reason key of a held call, and the spelling of its closed
/// reason category (`ReasonCategory::AwaitingHumanReview`).
pub const AWAITING_HUMAN_REVIEW: &str = "awaiting-human-review";

/// The prefix a held callee's call sites are rewritten with. No corpus symbol
/// begins with it (asserted by the tests), and a source that spells it is
/// escaped before this rewrite runs (card W45-SR5-SOUNDNESS,
/// `reserved_spelling`), so every name carrying it was minted here.
pub(crate) const HELD_PREFIX: &str = "qcore_review_pending__";

/// The original callee a rewritten name stands for.
pub(crate) fn held_callee(function: &str) -> Option<&str> {
    function.strip_prefix(HELD_PREFIX)
}

/// The refusal a held call site carries.
#[must_use]
pub fn hold_description(callee: &str) -> String {
    format!(
        "call to `{callee}` whose declared contract awaits human review ({AWAITING_HUMAN_REVIEW})"
    )
}

/// The callee a held call site's refusal names, if `description` is one.
#[must_use]
pub fn held_call(description: &str) -> Option<&str> {
    description
        .strip_prefix("call to `")?
        .strip_suffix(&format!(
            "` whose declared contract awaits human review ({AWAITING_HUMAN_REVIEW})"
        ))
}

/// Rewrite every call site of a pending callee so that the rest of the analyzer
/// sees an uncontracted, *named* callee. `None` means the stream is unchanged.
pub(crate) fn hold_call_sites(
    tokens: &[Token],
    contracts: &ResourceContracts,
) -> Option<Vec<Token>> {
    let pending = contracts.pending_review();
    if pending.is_empty() {
        return None;
    }
    let mut rewritten: Option<Vec<Token>> = None;
    for index in 0..tokens.len() {
        if tokens.get(index + 1).is_none_or(|token| token.text != "(") {
            continue;
        }
        if !pending.contains_key(&tokens[index].text) || !is_call_position(tokens, index) {
            continue;
        }
        let target = rewritten.get_or_insert_with(|| tokens.to_vec());
        target[index].text = format!("{HELD_PREFIX}{}", tokens[index].text);
    }
    rewritten
}
