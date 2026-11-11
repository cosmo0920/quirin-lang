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

//! Scoped member invariants of the null property (card
//! **NULL-MEMBER-INVARIANT-SCOPE**, contract specification C09.3, grammar
//! G05f, gateway rules 14x–14z).
//!
//! A global member declaration (rule 14t) is withdrawn by one store of NULL
//! anywhere. Two scoped readings are checkable instead:
//!
//! * **A stable member** (`nonnull_member(TAG, FIELD) where quant #scope [[stable]]`, an object-state
//!   scope): every store the analysed units make to `FIELD` of an *existing*
//!   object is proved non-null; an initializer list writes a fresh object
//!   and may leave it NULL. The state "set" (non-null) is entered at a
//!   non-null store or observation and never left, so a non-null fact about
//!   one object survives every call and every store, on any thread, until
//!   the pointer naming the object is reassigned ([`persist`]). The standing
//!   ([`stable_standing`]) is `withdrawn` when one examined store is not
//!   proved and never `validated`: a whole-object write the walk cannot
//!   attribute (a copy or fill through an untyped pointer) is a premise.
//! * **A function scope** (`nonnull_member(TAG, FIELD, P) where quant #scope [[scope]]`): on entry to
//!   the declared function, `P->FIELD` is non-null; each caller's actual is a
//!   checked obligation. With the member stable the entry fact lasts the
//!   whole body.
//!
//! A member set back to NULL on an existing object (an exit store: a reset,
//! an unlink) has no reading here: whether the exit reaches an object a use
//! is reading is a question of object identity, which these checks do not
//! decide (`QCore.NullMemberScope.an_exit_store_breaks_persistence`).
//!
//! The Lean model is `QCore.NullMemberScope`, bound by shared vectors
//! (`crates/qcore-core/tests/null_member_scope.rs`).

use crate::null_deref::{Nullness, Proof};

/// The dependency and proof name of a stable member `TAG.FIELD`.
#[must_use]
pub fn stable_name(member: &str) -> String {
    format!("stable:{member}")
}

/// The standing of one stable declaration over a run, from the stores the
/// analysed units made to existing objects (`examined`, of which `failed`
/// were not proved non-null). Initializer lists (fresh objects) are counted
/// apart and never examined. `withdrawn` when one failed; `declared-assumption`
/// when none was examined; otherwise `partly-validated` (the `stores` clause
/// covered, the `whole-object` premise never).
#[must_use]
pub fn stable_standing(examined: usize, failed: usize) -> &'static str {
    if failed > 0 {
        "withdrawn"
    } else if examined == 0 {
        "declared-assumption"
    } else {
        "partly-validated"
    }
}

/// What a fact on `V->FIELD` becomes across a call or a store to memory:
/// kept, with the stable member's proof added, only when the member is
/// stable (in force) and the fact is non-null; otherwise forgotten. A null
/// or possibly-null fact is never kept (a call may set the member).
#[must_use]
pub fn persist(fact: &Nullness, member: &str, stable_in_force: bool) -> Option<Nullness> {
    match fact {
        Nullness::NonNull(proofs) if stable_in_force => {
            let mut proofs = proofs.clone();
            proofs.insert(Proof::NonNullResult(stable_name(member)));
            Some(Nullness::NonNull(proofs))
        }
        _ => None,
    }
}
