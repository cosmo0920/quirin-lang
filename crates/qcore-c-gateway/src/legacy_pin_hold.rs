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

//! Pinned contracts that are not in force (card
//! **W45-CONTRACT-REVISION-MODE**, contract spec §C06.2).
//!
//! A pinned contract is in force only where the audited source matches the
//! identity its lock records. The audit (`qcore_audit::contract_revision`)
//! compares every pinned declaration with its lock before any unit is
//! analysed and withholds each mismatch: the contract leaves the function map
//! and is recorded in `ResourceContracts::pin_mismatch`. A withheld contract
//! discharges nothing and accuses nothing, so every call site that would have
//! applied it stays `Unknown`, and the refusal names the mismatch
//! (`contract-pin-mismatch`) rather than an absent declaration.
//!
//! The hold is applied exactly as a declaration awaiting human review is held
//! (`legacy_review_hold`, itself the LOADER-ABI technique): every call site of
//! a withheld callee is rewritten to a marked, undeclared name before any
//! reader sees the token stream, and the statement that calls it refuses with
//! [`hold_description`]. The rewrite is decided from the token stream and the
//! environment alone, so W4 derivation, W4 validation and certificate
//! rechecks reproduce it. A function *defined* in the unit under a withheld
//! name is analysed as an undeclared definition, and its callers stay held
//! whatever its body proves: the pin says the reviewed identity is not the
//! audited one, and nothing the analysis derives can restore it.
//!
//! Monotone in the refusing direction, for the reason the review hold is: a
//! held call is an unmodelled call, so no verdict moves towards `Safe` or
//! towards `Unsafe` because a pin does not match.

use qcore_schemas::contract::ResourceContracts;

use super::Token;
use super::declared_abi::is_call_position;

/// The stable reason key of a held call, and the spelling of its closed
/// reason category (`ReasonCategory::ContractPinMismatch`).
pub const CONTRACT_PIN_MISMATCH: &str = "contract-pin-mismatch";

/// The prefix a held callee's call sites are rewritten with, reserved for this
/// rewrite. A source that spells it itself is escaped before this rewrite runs
/// (card W45-SR5-SOUNDNESS, `reserved_spelling`): a held call is `Pending`, so
/// a source-spelled prefix read as one would forge a human-resolvable reason.
pub(crate) const HELD_PREFIX: &str = "qcore_pin_mismatch__";

/// The original callee a rewritten name stands for.
pub(crate) fn held_callee(function: &str) -> Option<&str> {
    function.strip_prefix(HELD_PREFIX)
}

/// The refusal a held call site carries.
#[must_use]
pub fn hold_description(callee: &str) -> String {
    format!(
        "call to `{callee}` whose pinned contract is not in force: the audited source does not \
         match its lock ({CONTRACT_PIN_MISMATCH})"
    )
}

/// The callee a held call site's refusal names, if `description` is one.
#[must_use]
pub fn held_call(description: &str) -> Option<&str> {
    description
        .strip_prefix("call to `")?
        .strip_suffix(&format!(
            "` whose pinned contract is not in force: the audited source does not match its lock \
         ({CONTRACT_PIN_MISMATCH})"
        ))
}

/// Rewrite every call site of a withheld pinned callee so that the rest of the
/// analyzer sees an uncontracted, *named* callee. `None` means the stream is
/// unchanged.
pub(crate) fn hold_call_sites(
    tokens: &[Token],
    contracts: &ResourceContracts,
) -> Option<Vec<Token>> {
    let withheld = contracts.pin_mismatch();
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
