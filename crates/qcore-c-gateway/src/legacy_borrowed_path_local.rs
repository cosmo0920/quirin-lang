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

//! Card **K5-LOCAL-ORIGIN-ALIASES** (gateway rule 77): the parser's and the
//! transfer's adapter for [`crate::borrowed_path_local`].
//!
//! The reading is asked once per function, after the CFG is built, and only
//! where a null test or a `return` names a local the declaration reader does
//! not call a scalar, so a function with neither asks Clang for nothing more.
//! The transfer reads it at exactly two places: the edge refinement of a
//! `BranchOnNull` (work list and leak-witness census alike) and the
//! `EscapeIfOwned` of a `return`.

use std::collections::BTreeSet;

use super::{AbstractState, LegacyCfg, LegacyInstruction, Parser, PointerRefinement};
use crate::borrowed_path_local::{
    BorrowedPathLocals, guard_reads_untracked, read, return_reads_untracked, untracked,
};

impl Parser<'_> {
    pub(super) fn borrowed_path_locals(
        &self,
        cfg: &LegacyCfg,
        scalars: &BTreeSet<String>,
    ) -> BorrowedPathLocals {
        let asked = cfg.nodes.iter().any(|node| match &node.instruction {
            LegacyInstruction::BranchOnNull { source, .. }
            | LegacyInstruction::EscapeIfOwned { source } => {
                !scalars.contains(source) && !self.formal_parameters.contains(source)
            }
            _ => false,
        });
        if !asked {
            return BorrowedPathLocals::new();
        }
        let Some(index) = self.typed_positions_index() else {
            return BorrowedPathLocals::new();
        };
        let function = std::sync::Arc::clone(&index.function);
        read(&function, &|name| self.position_pointer_root(name))
    }
}

/// Whether `name`'s binding in `state` is the untracked one rule 77 reads:
/// no target, the identity derivation, an unwidened state. An absent binding
/// is not (the older arms already decide it).
fn untracked_binding(state: &AbstractState, name: &str) -> bool {
    state.pointers.get(name).is_some_and(|pointer| {
        untracked(
            pointer.targets.is_empty(),
            pointer.derivation.is_identity(),
            state.widened,
        )
    })
}

/// The refinement of a null test's edge under rule 77: where the older arm
/// could not decide (rule 10b's identity refusal) a test of an admitted local
/// whose incoming binding is untracked, the narrowing it already applied
/// stands and the test is decided (`Refined`): no identity refusal, no
/// unmodelled edge. Every other refinement is returned unchanged.
pub(super) fn guard_refinement(
    refinement: PointerRefinement,
    instruction: &LegacyInstruction,
    before: &AbstractState,
    admitted: &BorrowedPathLocals,
) -> PointerRefinement {
    match (&refinement, instruction) {
        (PointerRefinement::Unknown, LegacyInstruction::BranchOnNull { source, .. })
            if !before.status_witnesses.contains_key(source)
                && !before.cell_witnesses.contains_key(source)
                && guard_reads_untracked(
                    admitted.contains(source),
                    untracked_binding(before, source),
                ) =>
        {
            PointerRefinement::Refined
        }
        _ => refinement,
    }
}

/// Whether `return source;` escapes nothing under rule 77.
pub(super) fn return_escapes_nothing(
    state: &AbstractState,
    source: &str,
    admitted: &BorrowedPathLocals,
) -> bool {
    return_reads_untracked(admitted.contains(source), untracked_binding(state, source))
}
