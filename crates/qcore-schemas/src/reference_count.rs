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

//! Reference counts (card **REFCOUNT-RESOURCE-KIND**; contract spec §C04.17,
//! grammar G05, gateway rules 13b and 13c).
//!
//! USER DECISION 2026-10-05 (`W48-CALLEE-DECISION-TABLE` class 3, reference
//! counts → a new reference-count resource kind): taking a reference ("keep")
//! is one more owned reference; dropping one ("destroy", "drop") is one fewer;
//! each function must balance the references it takes or hand them off
//! (return, store into a tracked owner, a declared transfer), exactly as with
//! other owned resources. A plain counter that is not a reference needs its
//! own declaration and never inherits the reference reading. Treating keep and
//! drop as custody-neutral was **rejected**: a missing drop would read `Safe`.
//!
//! ```text
//! ensures normal resource_reference_kept(p);      // +1 on the object p designates
//! ensures normal resource_reference_dropped(p);   // -1; the last drop may free it
//! ```
//!
//! Both clauses ride on [`crate::retained_by::RetainingContract`]: a
//! declaration carrying either is **withheld** from the function map (the
//! §C06.1 precedent), so a consumer that knows nothing of references sees an
//! uncontracted callee and never the borrowed (neutral) reading of `p`. The
//! obligation a keep creates is of [`crate::contract::ResourceKind::Reference`],
//! which no `resource_kind` clause spells.

use serde::{Deserialize, Serialize};

/// What a reference clause does to the object its parameter designates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceOperation {
    /// `resource_reference_kept(p)`: one more reference, owed by the caller.
    Kept,
    /// `resource_reference_dropped(p)`: one fewer; the last may free the object.
    Dropped,
}

impl ReferenceOperation {
    /// The clause's predicate name.
    #[must_use]
    pub const fn predicate(self) -> &'static str {
        match self {
            Self::Kept => "resource_reference_kept",
            Self::Dropped => "resource_reference_dropped",
        }
    }
}

/// One reference clause: the operation on the parameter at `parameter`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ReferenceClause {
    pub parameter: usize,
    pub operation: ReferenceOperation,
}
