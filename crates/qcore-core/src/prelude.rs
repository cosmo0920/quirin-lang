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

//! Explicit imports for reusable QCore semantic features.
//!
//! This is a Rust library prelude for compiler and library implementers. QCore
//! source opts in through `#import std.concurrency;`; no source feature is
//! enabled by default. See [`crate::module_imports`] for bundled-module resolution.

/// Checked concurrency vocabulary, models and backend entry points.
///
/// The implementations belong to this crate; these exports preserve their
/// original type identities, validation rules, diagnostics and assumptions.
/// A context is constructed explicitly as a [`ScopedTaskLedger`]. Importing the
/// prelude creates no threads or authority.
///
/// Model namespaces stay separate: for example, `scoped_tasks::TaskId` and
/// `concurrency::TaskId` describe different models and are not interchangeable.
/// Task/mutex composition still requires its own checked semantics; importing
/// both does not prove that composition. Native execution and C emission retain
/// their existing platform and refinement bounds.
///
/// Lean entry point: `import QCore.ConcurrencyPrelude`. Its exported theorems
/// cover the corresponding abstract models, not unrestricted Rust/C execution.
///
/// ```
/// use qcore_core::prelude::concurrency::{
///     ScopedTaskLedger,
///     scoped_tasks::{ROOT_TASK, ResourceId, TaskLedgerError, TaskOutcome},
/// };
///
/// # fn main() -> Result<(), TaskLedgerError> {
/// let mut context = ScopedTaskLedger::default();
/// let buffer: ResourceId = 7;
/// context.enter_scope(ROOT_TASK, 1)?;
/// context.acquire(ROOT_TASK, buffer)?;
/// context.reserve(ROOT_TASK, 1, 2, &[buffer])?;
/// // A successful external creation commits the reserved custody to the child.
/// // A failed creation would instead call context.rollback(ROOT_TASK, 2).
/// context.commit(ROOT_TASK, 2, 3, 4)?;
/// context.complete(3, TaskOutcome::OwnedBuffer(buffer), &[buffer])?;
/// // Completion retains custody until the creator explicitly joins.
/// assert!(context.use_resource(ROOT_TASK, buffer).is_err());
/// assert_eq!(context.exit_scope(ROOT_TASK, 1), Err(TaskLedgerError::ScopeBusy(1)));
/// let completion = context.join(ROOT_TASK, 4)?;
/// assert_eq!(completion.resources, vec![buffer]);
/// context.drop_resource(ROOT_TASK, buffer)?;
/// context.exit_scope(ROOT_TASK, 1)?;
/// # Ok(())
/// # }
/// ```
pub mod concurrency {
    // Keep each model's identity and certificate vocabulary qualified.
    pub use crate::{
        concurrency, epoch_accounting, epoch_credit_recovery, mutex_c_emit, mutex_invariant,
        native_concurrency, region_task_credit, scoped_tasks, task_c_emit, task_execution,
        task_trace_relation, weak_memory,
    };

    // Common construction/checking operations are available without a glob
    // import of conflicting TaskId, ResourceId, State or Outcome definitions.
    pub use crate::concurrency::{ConcurrentProgram, check_concurrent};
    pub use crate::epoch_accounting::{
        FlatEpochProgram, check_flat_epoch, epoch_residency, epoch_wall_clock_bound,
        recheck_epoch_bound,
    };
    pub use crate::epoch_credit_recovery::{
        EpochCreditRecoveryPlan, check_epoch_credit_recovery, recheck_epoch_credit_recovery,
    };
    pub use crate::mutex_invariant::{
        MutexInvariantProgram, check_mutex_invariant_program, recheck_mutex_invariant_certificate,
    };
    pub use crate::scoped_tasks::ScopedTaskLedger;
    pub use crate::task_execution::{CheckedTaskModule, TaskModule, check_task_module};
}
