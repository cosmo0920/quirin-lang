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

//! Slot moves and derived out slots (card **SLOT-MOVE-AND-INTERIOR-BORROW**;
//! contract spec §C04.15 and §C04.16, grammar G05, gateway rules 12x and 12z).
//!
//! USER DECISION 2026-10-05 (`W48-CALLEE-DECISION-TABLE` classes 2 and 8, and
//! `INTERIOR-OUT-SLOT-MODE`):
//!
//! ```text
//! ensures normal resource_slot_moved(s, n);       // class 2: the slot `T **s`
//! ensures normal resource_out_derived(s, p, n);   // a pointer derived from p through s
//! ```
//!
//! **Slot move.** After the call the slot `*s` still owns an object, possibly
//! a new one; every alias of the old pointer is invalid (a use is refused,
//! never `Safe`); on failure the original stays owned by the slot. `n` is the
//! nullability of `*s` at entry: under `nullable` a NULL `*s` makes the call
//! move nothing. It is the custody counterpart of CG24's `bounds_frame_moves`.
//!
//! **Derived out slot.** The call may write through the slot `s` a pointer
//! derived from parameter `p` at the interior extent (§C04.3.1's rules), or
//! leave the slot as it was; the written pointer owns nothing. `n` is the
//! nullability of `s` itself (`strtol`'s `endptr`).
//!
//! Both clauses ride on [`crate::retained_by::RetainingContract`]: a
//! declaration carrying either is **withheld** from the function map (the
//! §C06.1 precedent), so a consumer that knows nothing of them sees an
//! uncontracted callee, and every slot parameter they name is
//! `resource_borrowed` in the declaration's custody contract.

use serde::{Deserialize, Serialize};

/// `resource_slot_moved(s, n)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SlotMove {
    /// The slot parameter, a `ptr<mut, ptr<mut, u8>>`.
    pub slot: usize,
    /// Whether `*s` may be NULL at entry (the call then moves nothing).
    pub nullable: bool,
}

/// `resource_out_derived(s, p, n)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct DerivedOut {
    /// The slot parameter, `ptr<mut, ptr<mut, u8>>` or `ptr<mut, ptr<const, u8>>`.
    pub slot: usize,
    /// The base parameter the written pointer is derived from.
    pub base: usize,
    /// Whether the slot itself may be NULL (nothing is written then).
    pub nullable: bool,
    /// Whether the base is `ptr<const, u8>` and the slot's pointee pointer is
    /// mutable (`strtol`: `const char *` in, `char **` out), as §C04.3.1's
    /// `const_dropped`.
    pub const_dropped: bool,
}
