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

//! Card **NULLABLE-LAZY-CONSTRUCTOR-IN-LOOP** (gateway rule 74d): a resolved
//! resource that may be null, returned under a result mode that does not
//! permit null, is refused as exactly that.
//!
//! The lazy-initialisation guard `if (!p) p = ctor(…);` was already decided by
//! rules 14l and 10b, in a loop and outside one: the guard's non-null edge
//! keeps the earlier site, its null edge holds no site, the constructing arm
//! binds the fresh non-null site, and a loop head keeps the non-null fact only
//! when every arriving edge has it (the loop entry carries the NULL
//! initialiser, so after the loop the name is null on the zero-iteration path,
//! which is C). The ledger's lazy-loop row
//! (`rd_kafka_cgrp_owned_but_not_exist_partitions`) and its sibling
//! `rd_kafka_cgrp_calculate_subscribe_revoking_partitions` carried their
//! nullable leaf on `return result;`, not on a use inside the loop: each
//! returns NULL on a real path (no partition found; an empty `revoking` list
//! destroyed and set to NULL). A body may return a possibly-null resource only
//! under a result mode that says the caller must null-check it
//! (`EntryCustody::result_nullable`), which the validated in-unit summary
//! supplies (`Owned { nullable: true }`); both bodies carry other refusals, so
//! no summary validates, the row is decided by the contract-free run, and that
//! run's result mode is non-null.
//!
//! The refusal stays. What changes is its name: the identity of the returned
//! binding is resolved (one site, no unknown alternative, an identity
//! derivation), so the `unresolved resource identity` leaf it also carried was
//! not the obstacle, and `used by a non-null contract` named a contract no
//! call carries. [`refuse`] reports the one leaf that is, and marks the site
//! unresolved exactly as `resource_action`'s refusing branch does, so the
//! abstract state, the schedule and every leak claim are unchanged.
//!
//! Two bindings keep the old pair. A formal the entry's contract names (the
//! W4 summary search binds a formal it sees reported as an unresolved
//! identity, so renaming that leaf could change which summary is found), and
//! a binding with no target (a NULL-only binding; a return of it is refused as
//! before). The second is why a lazy loop's return can still carry
//! `unresolved resource identity`: on an iterate before the back edge joined,
//! only the NULL entry edge had reached the return, and rule 6's per-iterate
//! reporting keeps that leaf.

use super::{AbstractState, LegacyDiagnostic, LegacyDiagnosticKind, mark_unresolved_effect};
use std::collections::BTreeSet;

/// The leaf's text.
pub(crate) fn description(source: &str) -> String {
    format!("possibly-null resource `{source}` returned under a non-null result mode")
}

/// Refuse the return of `source` when its only obstacle under a non-null
/// result mode is that it may be null: an identity binding with exactly one
/// known target and no unknown alternative that is null on some path, or
/// whose site is nullable. Answers `false`, touching nothing, for every other
/// binding, which keeps `resource_action`'s refusal or effect.
pub(super) fn refuse(
    function: &str,
    line: usize,
    source: &str,
    state: &mut AbstractState,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) -> bool {
    let Some(pointer) = state.pointers.get(source).cloned() else {
        return false;
    };
    let resolved =
        !pointer.unknown && pointer.targets.len() == 1 && pointer.derivation.is_identity();
    let nullable = pointer.may_be_null
        || pointer
            .targets
            .iter()
            .any(|site| state.nullable_sites.contains(site));
    if !(resolved && nullable) {
        return false;
    }
    diagnostics.insert(LegacyDiagnostic {
        function: function.to_owned(),
        line,
        kind: LegacyDiagnosticKind::Unsupported {
            description: description(source),
        },
    });
    // Exactly the marking `resource_action`'s refusing branch applies.
    mark_unresolved_effect(state, &pointer);
    true
}
