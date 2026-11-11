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

//! Card **CG10-INDIRECT-ACCESS-SOLE-RESIDUALS** round 2 (gateway rule 14c):
//! the address of a scalar element handed to a `Borrowed` or declared
//! data-tail actual.
//!
//! Rule 10n(1) reads an address `&PATH` handed to a callee only when the
//! place is exactly a member whose Clang type is not a pointer, an array or a
//! function pointer (O04-B09B10's disjointness premise: a callee writing
//! through the address reaches no member the projection links). The element
//! form `&B->F[I]` (`cmt_atomic_load(&metric->hist_buckets[id])`) was refused
//! although its element type is a scalar: the array the element belongs to
//! holds scalars only, so a write through the address reaches no pointer
//! either. This module admits exactly that element form. Everything else of
//! rule 10n(1) stands: the reader of [`crate::typed_access_path::Reader::path`]
//! carries every per-hop obligation of `PATH` (the dereferenced root and each
//! link, the index read as a scalar), the place itself is not read, and every
//! root must be never bound (`StatementIndex::argument`'s address premise), so
//! the loaded pointer is outside the tracked graph.
//!
//! Premises stated, not proved: faithful Clang extraction of the element's
//! type (trust boundary 28), and, as for every W44 actual, that the callee
//! writes within the array the element belongs to (bounds are outside the
//! property). The Lean model is `QCore.ResidualAccessReadings`.

use serde_json::Value;

use crate::typed_c_ast::kind;

/// The rule's arm switch: registry rule `14c` (card GATEWAY-RULE-API, which
/// replaced the compile-time `ELEMENT_ARM`); off restores rule 10n(1)'s
/// member-only shape.
fn element_arm() -> bool {
    crate::gateway_rules::enabled("14c")
}

/// The decision table the Lean model mirrors (`decision_vectors_hold`): the
/// place's kind, whether its type is a scalar, and whether rule 10n(1) or
/// this rule admits the address at an actual. Rule 10n(1)'s own type test
/// (no pointer, array or function type) and its root premise apply after.
#[cfg(test)]
pub(crate) const DECISION_VECTORS: &[(&str, bool, bool)] = &[
    ("MemberExpr", true, true),
    ("MemberExpr", false, true),
    ("ArraySubscriptExpr", true, true),
    ("ArraySubscriptExpr", false, false),
    ("ParenExpr", true, false),
    ("DeclRefExpr", true, false),
    ("UnaryOperator", true, false),
];

/// The handed-to-a-callee shape decision of rule 10n(1) with this rule:
/// `place` (the operand of the `&`) is a member, or an element whose type
/// the reader proves a scalar.
pub(crate) fn admits(place: &Value, scalar: impl Fn(&Value) -> bool) -> bool {
    shape_admitted(
        kind(place),
        kind(place) == "ArraySubscriptExpr" && scalar(place),
    )
}

/// [`admits`] on the decision table's terms.
pub(crate) fn shape_admitted(place_kind: &str, scalar: bool) -> bool {
    place_kind == "MemberExpr" || (place_kind == "ArraySubscriptExpr" && scalar && element_arm())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn the_decision_vectors_are_the_rust_decision() {
        for (place_kind, scalar, admitted) in DECISION_VECTORS {
            assert_eq!(
                shape_admitted(place_kind, *scalar),
                *admitted,
                "{place_kind} {scalar}"
            );
            let node = json!({ "kind": place_kind });
            assert_eq!(
                admits(&node, |_| *scalar),
                *admitted,
                "{place_kind} {scalar}"
            );
        }
    }
}
