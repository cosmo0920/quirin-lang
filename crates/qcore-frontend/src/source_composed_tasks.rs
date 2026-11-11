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

//! Route selection for `source.std-io-tasks.v1` — the composition of
//! `source.std-io.v1` with the checked task route (card `COMPOSE-CONCURRENCY`).
//!
//! This module states one predicate and nothing else. The lowering is
//! [`crate::source_tasks::lower_source_composed_tasks`], which is the task
//! route's own lowering with its line-producing statements admitted in the
//! parent's body; the join is `qcore_core::composed_tasks`.
//!
//! # The conjunction, and what it deliberately leaves alone
//!
//! The predicate is `has_source_tasks && has_source_std_io && !has_source_extern`,
//! and each conjunct is an existing exact predicate — declaring an extern
//! function is not calling one, importing `std.io` is not printing. Route
//! selection places this composition **after** the composed-epoch, epoch and
//! mutex routes and **before** the task route, so:
//!
//! * a module with an epoch scope or a mutex generation that also prints keeps
//!   that more specific concurrent model's refusal, unchanged — those routes are
//!   selected earlier and never reach this line;
//! * a module that extern-calls **and** spawns keeps the task route's `Q600`,
//!   because the third conjunct is false. That is `STD-IO-COMPOSE`'s recorded
//!   exclusion and this card does not lift it: the extern composition is a
//!   straight-line union whose numbers are declared at the site, and putting a
//!   second checked model beside a branching concurrent one is a third join with
//!   its own argument to make;
//! * a module that prints **and** extern-calls **and** spawns keeps it for the
//!   same reason, and is pinned separately so the two exclusions cannot be
//!   confused for one;
//! * a module using only one of the two composed feature sets fails the
//!   conjunction and falls through to that route **byte for byte** — which is
//!   structural rather than tested, because both halves are one lowering with a
//!   flag whose `false` case *is* the old entry point.

use crate::frontend::SourceProgram;

/// Whether this module is a `source.std-io-tasks.v1` module.
#[must_use]
pub fn has_source_composed_tasks(program: &SourceProgram) -> bool {
    crate::source_tasks::has_source_tasks(program)
        && (crate::source_std_io::has_source_std_io(program)
            || crate::source_std_io::has_source_log_level(program))
        && !crate::source_extern::has_source_extern(program)
}
