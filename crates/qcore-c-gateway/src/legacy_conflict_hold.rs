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

//! Functions whose contract bundles conflict (card **CONTRACT-SUPERSESSION**,
//! contract spec §C06.6).
//!
//! When more than one module declares a function, the loader keeps the
//! declaration with the greatest pin stamp in force and retains the others as
//! superseded. Where the stamps do not decide (equal stamps on different
//! texts, or a missing, malformed or stale stamp, or an unpinned declaration),
//! no declaration is in force: the function is recorded under
//! `ResourceContracts::contract_conflict` with every candidate's text and is
//! in no table. A withheld function discharges nothing and accuses nothing, so
//! every call site that would have applied it stays `Unknown`, and the
//! refusal names the conflict (`contract-conflict`) rather than an absent
//! declaration.
//!
//! The hold is applied as a pin mismatch's is (`legacy_pin_hold`, the
//! LOADER-ABI technique): every call site of a withheld callee is rewritten to
//! a marked, undeclared name before any reader sees the token stream, and the
//! statement that calls it refuses with [`hold_description`]. Unlike a pin
//! mismatch it is never `Pending`: which bundle should be in force is not one
//! named human act on the row, so the held call is not a held call of
//! `legacy_verdict::is_held_call` and its row stays `Unknown`. A source that
//! spells the reserved prefix itself is escaped first (`reserved_spelling`).
//!
//! Monotone in the refusing direction: a held call is an unmodelled call, so
//! no verdict moves towards `Safe` or towards `Unsafe` because bundles
//! conflict. With no conflict the stream is unchanged.

use qcore_schemas::contract::ResourceContracts;

use super::Token;
use super::declared_abi::is_call_position;

/// The stable reason key of a held call, and the spelling of its closed
/// reason category (`ReasonCategory::ContractConflict`).
pub const CONTRACT_CONFLICT: &str = "contract-conflict";

/// The prefix a held callee's call sites are rewritten with, reserved for this
/// rewrite.
pub(crate) const HELD_PREFIX: &str = "qcore_contract_conflict__";

/// The original callee a rewritten name stands for.
pub(crate) fn held_callee(function: &str) -> Option<&str> {
    function.strip_prefix(HELD_PREFIX)
}

/// The refusal a held call site carries.
#[must_use]
pub fn hold_description(callee: &str) -> String {
    format!(
        "call to `{callee}` whose contract bundles conflict: their pin stamps do not decide \
         which declaration is in force ({CONTRACT_CONFLICT})"
    )
}

/// The callee a held call site's refusal names, if `description` is one.
#[must_use]
pub fn held_call(description: &str) -> Option<&str> {
    description
        .strip_prefix("call to `")?
        .strip_suffix(&format!(
            "` whose contract bundles conflict: their pin stamps do not decide which \
             declaration is in force ({CONTRACT_CONFLICT})"
        ))
}

/// Rewrite every call site of a function withheld as a contract conflict so
/// that the rest of the analyzer sees an uncontracted, *named* callee. `None`
/// means the stream is unchanged.
pub(crate) fn hold_call_sites(
    tokens: &[Token],
    contracts: &ResourceContracts,
) -> Option<Vec<Token>> {
    let withheld = contracts.contract_conflict();
    if withheld.is_empty() {
        return None;
    }
    let mut rewritten: Option<Vec<Token>> = None;
    for index in 0..tokens.len() {
        if tokens.get(index + 1).is_none_or(|token| token.text != "(") {
            continue;
        }
        if !withheld.contains_key(&tokens[index].text) || !is_call_position(tokens, index) {
            continue;
        }
        let target = rewritten.get_or_insert_with(|| tokens.to_vec());
        target[index].text = format!("{HELD_PREFIX}{}", tokens[index].text);
    }
    rewritten
}
