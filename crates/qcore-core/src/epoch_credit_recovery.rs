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

//! `EpochCreditRecoveryV1` — the error path over a flat epoch's quantitative
//! credits (card `CT23-CREDIT-ERROR-PATH`).
//!
//! [`crate::epoch_accounting`] checks one flat epoch and grants every task a
//! `credit` out of the epoch's `allocation`. It models exactly one way out of
//! the epoch: the creator creates every task, joins every task, and reaches the
//! epoch exit. It says nothing about the creator's **own error edges** — the
//! creation that failed, and the join that observed a raised child — and so a
//! plan cannot say what becomes of a grant on them. That is the whole of CT23's
//! recorded gap: *"credits yes, an error path over them no."*
//!
//! This module is a **sibling** of [`crate::epoch_accounting`] and a **client**
//! of it, in the same relation [`crate::region_task_credit`] holds to
//! [`crate::error_region`]: the flat-epoch checker is not edited, its
//! certificate is not changed, and every program admitted before this module
//! existed is admitted by it with byte-identical figures. What is added is one
//! further checked statement, published beside the bound rather than inside it.
//!
//! # The accounting law
//!
//! A grant is made to one creation out of the epoch's allocation. On the
//! creator's error edge over that creation the grant has exactly two fates, and
//! it must have one of them:
//!
//! * **restored** — the creation did not consume it (the child was never made,
//!   or it raised and gave it back), so it returns to the epoch; or
//! * **spent** — the creator's own failure handling consumed it.
//!
//! > **Conservation.** For every task `t` of an admitted plan,
//! > `restored(t) + spent(t) = credit(t)` and `restored(t) ≤ credit(t)`
//! > componentwise; and over the whole epoch
//! > `restored_credits + spent_credits = granted_credits ≤ allocation`.
//!
//! Both directions are refusals, not conventions:
//!
//! * **No leak.** [`ErrorArmCredit::AbandonedAtErrorArm`] is the representable
//!   claim that a grant is neither restored nor spent — the creator walked its
//!   error edge and the credit simply stopped being accounted for. It is always
//!   refused ([`EpochCreditDiagnostic::CreditAbandonedAtErrorArm`], `Q635`),
//!   for the same reason [`crate::epoch_accounting::ResultDisposal::DroppedAtCompletion`]
//!   exists and is always refused: the case must be *spellable* so that the core
//!   is the thing that refuses it, rather than being a case nobody checks.
//! * **No mint.** A restoration larger than the grant it comes out of would put
//!   credit into the epoch that the epoch never granted, and the allocation
//!   bound would stop bounding anything. It is refused per task and per axis
//!   ([`EpochCreditDiagnostic::RestoredCreditExceedsGrant`], `Q636`).
//!
//! The corollary is what makes the error edge safe without any new bound:
//! because `spent_credits ≤ granted_credits`, the credit committed on **every**
//! error edge is bounded by the credit committed on the success exit, which
//! `check_flat_epoch` already bounds by the allocation (`Q637`). The error path
//! therefore needs no second allocation check, and this module deliberately does
//! not invent one — see [`EpochCreditRecoveryCertificate::error_arm_committed`].
//!
//! # What this module does not do
//!
//! * **It relaxes nothing.** Restoration does not reduce `granted_credits`, so
//!   no epoch that `check_flat_epoch` refuses becomes admitted, and an epoch
//!   whose every arm is elided produces exactly the accounting that existed
//!   before this module. The whole of its effect is two new refusals and one
//!   published split.
//! * **It derives no figure.** `restored` is a declared quantity, exactly as
//!   `credit`, `peak`, `retained` and the overhead are
//!   ([`crate::epoch_accounting::EpochAssumption::DeclaredTaskFootprints`]).
//!   What is *derived* is the **shape**: whether the creator has an error edge
//!   over a creation at all is read out of the checked task IR by the composed
//!   route, never out of the plan.
//! * **It is not a schedule, a retry model or a cancellation model.** Restored
//!   credit is not re-granted to a replacement creation here: the epoch is flat
//!   and its grants are a single static division of the allocation. A plan that
//!   wants to reuse restored credit is `CT23-RESTORE-REUSE`.
//!
//! ```
//! use qcore_core::epoch_credit_recovery::*;
//! use qcore_core::epoch_accounting::*;
//! use qcore_core::resource_flow::Quantity;
//! # fn quantity(bytes: u64) -> Quantity { Quantity::single("memory", bytes) }
//! let program = FlatEpochProgram {
//!     declared_task_count: 1,
//!     shared_storage: Vec::new(),
//!     overhead: RuntimeOverhead {
//!         per_task: Quantity::zero(),
//!         per_epoch: Quantity::zero(),
//!         basis: OverheadBasis::Declared,
//!     },
//!     allocation: quantity(4096),
//!     tasks: vec![EpochTask {
//!         task: 1,
//!         shared_entry: Vec::new(),
//!         private_peak: quantity(1024),
//!         work: Quantity::zero(),
//!         result: TaskResult::ReleasedInWorker,
//!         credit: quantity(4096),
//!         joined: true,
//!     }],
//!     unsupported: Vec::new(),
//! };
//! let plan = EpochCreditRecoveryPlan {
//!     arms: vec![CreationErrorArm {
//!         task: 1,
//!         edges: CreationErrorEdges { creation_failure: true, join_failure: true },
//!         declared: Some(ErrorArmCredit::RestoredToEpoch { restored: quantity(4096) }),
//!     }],
//! };
//! let certificate = check_epoch_credit_recovery(&program, &plan).unwrap();
//! assert_eq!(certificate.restored_credits, quantity(4096));
//! assert_eq!(certificate.spent_credits, Quantity::zero());
//! assert!(recheck_epoch_credit_recovery(&program, &plan, &certificate).is_ok());
//! ```
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::epoch_accounting::{EpochTaskId, FlatEpochProgram};
use crate::resource_flow::{FlowError, Quantity};

/// The profile label recorded on every certificate this module publishes. It is
/// deliberately *not* a [`crate::concurrency::ConcurrencyProfile`] variant: this
/// is a statement published **beside** `FlatEpochBoundV1`'s certificate, not a
/// second reading of it, and the profile enum's existing exhaustive matches say
/// what a flat epoch is.
pub const PROFILE: &str = "EpochCreditRecoveryV1";

/// Which of the creator's own error edges exist over one creation. **Derived**,
/// never declared: the composed route reads both out of the checked task IR, and
/// the pure declarative route has neither because it has no program.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct CreationErrorEdges {
    /// The creation itself can fail, so the child never exists and never spends
    /// anything of the grant.
    pub creation_failure: bool,
    /// The creator's join can observe a child that raised, and propagate. Here
    /// the child *did* exist and *did* hold the grant, which is the edge CT23's
    /// scenario names.
    pub join_failure: bool,
}

impl CreationErrorEdges {
    /// Whether the creator has any error edge over this creation at all.
    pub const fn any(self) -> bool {
        self.creation_failure || self.join_failure
    }
}

/// What the creator's error edge does with the credit granted to one creation.
///
/// There is no fourth case on purpose: a grant either comes back or is consumed,
/// and the claim that it does neither is [`Self::AbandonedAtErrorArm`], which is
/// always refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ErrorArmCredit {
    /// The creator's failure handling spends the whole grant. This is the
    /// conservative reading and the one an elided arm gets, because it commits
    /// exactly the credit `check_flat_epoch` already committed.
    SpentByCreator,
    /// `restored` of the grant returns to the epoch; the remainder is spent.
    /// `restored` equal to the grant is a full rollback; `restored` zero is
    /// [`Self::SpentByCreator`], and the two are required to agree.
    RestoredToEpoch { restored: Quantity },
    /// The grant is neither restored nor spent: the creator walked its error
    /// edge and the credit stopped being accounted for. **Always refused.**
    AbandonedAtErrorArm,
}

/// One creation's error arm: which edges the creator has over it, and what the
/// plan declares happens to its grant there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreationErrorArm {
    pub task: EpochTaskId,
    pub edges: CreationErrorEdges,
    /// `None` is an **elided** arm, which means [`ErrorArmCredit::SpentByCreator`].
    /// The distinction is kept because a *declared* arm over a creation the
    /// creator cannot fail on is a refusal, and an elided one is not.
    pub declared: Option<ErrorArmCredit>,
}

impl CreationErrorArm {
    /// The disposition this arm carries, with the elision resolved.
    pub fn disposition(&self) -> ErrorArmCredit {
        self.declared
            .clone()
            .unwrap_or(ErrorArmCredit::SpentByCreator)
    }
}

/// One error-path plan: exactly one arm per task of the epoch, in task order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EpochCreditRecoveryPlan {
    pub arms: Vec<CreationErrorArm>,
}

/// Refusals of this module. Every one of them surfaces under a code the flat
/// epoch's own band `Q630`-`Q641` already owns, because they are refusals of the
/// same semantic pass about the same quantities; **no code is added, moved or
/// reused by a second pass**. The per-condition argument is on
/// [`Self::code`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EpochCreditDiagnostic {
    // ---- structural (Q630) ----
    /// The plan does not carry exactly one arm per task.
    ArmCountMismatch {
        declared: usize,
        tasks: usize,
    },
    /// An arm names a task this epoch does not have.
    UnknownTask {
        task: EpochTaskId,
    },
    /// Two arms for one task. Refused rather than resolved: two dispositions of
    /// one grant is two answers to one question.
    DuplicateArm {
        task: EpochTaskId,
    },
    /// An arm is *declared* for a creation the creator has no error edge over,
    /// so it disposes of a grant on a path the program does not have. This is
    /// what the purely declarative `source.epoch.bound.v1` route receives: its
    /// workers are declarations, and a declaration has no creation to fail.
    ErrorArmWithoutErrorEdge {
        task: EpochTaskId,
    },

    // ---- the leak direction (Q635) ----
    /// The grant is neither restored to the epoch nor spent by the creator.
    CreditAbandonedAtErrorArm {
        task: EpochTaskId,
        axis: String,
        abandoned: u64,
    },

    // ---- the mint direction (Q636) ----
    /// More is restored on the error edge than was ever granted to this task.
    RestoredCreditExceedsGrant {
        task: EpochTaskId,
        axis: String,
        restored: u64,
        credit: u64,
    },

    // ---- arithmetic (Q641) ----
    Arithmetic {
        detail: FlowError,
    },
}

impl EpochCreditDiagnostic {
    /// The code this refusal surfaces as.
    ///
    /// Every one is an existing `Q630`-`Q641` condition extended, never a new
    /// code, and the choice of code is argued per band rather than assigned:
    ///
    /// * **`Q630`** is the flat epoch's *structural* band — `EmptyEpoch`,
    ///   `DuplicateTask`, `TaskCountMismatch`, `DuplicateSharedCell`. All four
    ///   structural refusals here are the same kind of statement about the plan
    ///   disagreeing with the epoch it is a plan for.
    /// * **`Q635`** is the flat epoch's *outstanding-at-the-epoch-exit* band, and
    ///   it already carries **two** conditions for that reason
    ///   (`UntransferredResultAtEpochExit`, `UnjoinedTask`). Abandoned credit is
    ///   the third: a quantity the epoch granted and no exit accounts for.
    /// * **`Q636`** is the flat epoch's *per-task credit* band
    ///   (`TaskExceedsItsCredit`: a demand above the grant). A restoration above
    ///   the grant is the same per-task relation read in the other direction.
    /// * **`Q641`** is the arithmetic band, and the error path overflows and
    ///   underflows into it exactly as the success path does.
    ///
    /// `Q637` (`CreditsExceedAllocation`) is deliberately **not** extended: the
    /// conservation law makes the error path's committed credit no larger than
    /// the success path's, which that code already bounds, so a second check
    /// there would be an unreachable condition rather than a gate.
    pub const fn code(&self) -> &'static str {
        match self {
            Self::ArmCountMismatch { .. }
            | Self::UnknownTask { .. }
            | Self::DuplicateArm { .. }
            | Self::ErrorArmWithoutErrorEdge { .. } => "Q630",
            Self::CreditAbandonedAtErrorArm { .. } => "Q635",
            Self::RestoredCreditExceedsGrant { .. } => "Q636",
            Self::Arithmetic { .. } => "Q641",
        }
    }
}

impl fmt::Display for EpochCreditDiagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {self:?}", self.code())
    }
}

/// The checked error-path statement for one admitted epoch. Published beside
/// [`crate::epoch_accounting::EpochBoundCertificate`], never inside it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EpochCreditRecoveryCertificate {
    /// Always [`PROFILE`].
    pub profile: &'static str,
    /// The epoch's granted total, read back out of the program. Equal to the
    /// bound certificate's `granted_credits` by construction, and republished
    /// here so the split below can be read without the other certificate.
    pub granted_credits: Quantity,
    /// Σ restored. The credit the creator's error edges return to the epoch.
    pub restored_credits: Quantity,
    /// Σ (credit − restored). The credit the creator's failure handling spends.
    pub spent_credits: Quantity,
    /// The credit committed on the all-error path, which is exactly
    /// [`Self::spent_credits`]. Published under its own name because it is the
    /// quantity the corollary is about: it is `≤ granted_credits`, which
    /// `check_flat_epoch` has already bounded by the allocation, so no admitted
    /// error edge can exceed the epoch's allocation.
    pub error_arm_committed: Quantity,
    /// Per task, in task order: the grant, what is restored of it, and what is
    /// spent of it. Every row satisfies `restored + spent = credit`.
    pub arms: Vec<CreditArmFigures>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreditArmFigures {
    pub task: EpochTaskId,
    pub credit: Quantity,
    pub restored: Quantity,
    pub spent: Quantity,
}

/// Why an independent recheck refused a supplied error-path certificate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EpochCreditRecheckError {
    /// Re-running the checker on the same program and plan produced refusals.
    PlanRejected(Vec<EpochCreditDiagnostic>),
    FieldMismatch {
        field: &'static str,
    },
    /// The published split does not conserve the grant: the certificate claims a
    /// `restored` and a `spent` whose sum is not what the epoch granted. This is
    /// the certificate-level form of the leak refusal, named rather than folded
    /// into a field mismatch.
    CreditLedgerUnbalanced {
        granted: Quantity,
        restored: Quantity,
        spent: Quantity,
    },
    /// The certificate claims more restored credit than the epoch ever granted.
    /// The certificate-level form of the mint refusal.
    RestoredCreditMinted {
        restored: Quantity,
        granted: Quantity,
    },
    /// A per-task row does not conserve its own grant.
    ArmUnbalanced {
        task: EpochTaskId,
    },
}

// ---------------------------------------------------------------------------
// Quantity arithmetic over the public `resource_flow` surface.
// ---------------------------------------------------------------------------

fn add(left: &Quantity, right: &Quantity) -> Result<Quantity, FlowError> {
    Quantity::from_axes(
        left.iter()
            .chain(right.iter())
            .map(|(axis, amount)| (axis.to_owned(), amount)),
    )
}

fn add_all<'a>(parts: impl IntoIterator<Item = &'a Quantity>) -> Result<Quantity, FlowError> {
    let mut total = Quantity::zero();
    for part in parts {
        total = add(&total, part)?;
    }
    Ok(total)
}

/// `left − right`, refusing an underflow rather than saturating. A saturating
/// subtraction here would silently manufacture the conservation identity this
/// module exists to check.
fn subtract(left: &Quantity, right: &Quantity) -> Result<Quantity, FlowError> {
    let mut axes = BTreeMap::new();
    for (axis, amount) in left.iter() {
        axes.insert(axis.to_owned(), amount);
    }
    for (axis, amount) in right.iter() {
        let slot = axes.entry(axis.to_owned()).or_insert(0);
        *slot = slot
            .checked_sub(amount)
            .ok_or_else(|| FlowError::RuntimeUnderflow(axis.to_owned()))?;
    }
    Quantity::from_axes(axes)
}

/// The first axis, in axis order, on which `required` exceeds `available`.
fn first_excess(required: &Quantity, available: &Quantity) -> Option<(String, u64, u64)> {
    required.iter().find_map(|(axis, amount)| {
        let capacity = available.get(axis);
        (amount > capacity).then(|| (axis.to_owned(), amount, capacity))
    })
}

/// The first axis, in axis order, carrying a nonzero amount.
fn first_nonzero(quantity: &Quantity) -> Option<(String, u64)> {
    quantity
        .iter()
        .find_map(|(axis, amount)| (amount > 0).then(|| (axis.to_owned(), amount)))
}

// ---------------------------------------------------------------------------
// Checking
// ---------------------------------------------------------------------------

/// Structural refusals precede accounting, exactly as in
/// [`crate::epoch_accounting::check_flat_epoch`]: a plan that does not describe
/// this epoch is never given a credit split, not even a conservative one.
fn check_shape(
    program: &FlatEpochProgram,
    plan: &EpochCreditRecoveryPlan,
) -> Vec<EpochCreditDiagnostic> {
    let mut diagnostics = Vec::new();
    if plan.arms.len() != program.tasks.len() {
        diagnostics.push(EpochCreditDiagnostic::ArmCountMismatch {
            declared: plan.arms.len(),
            tasks: program.tasks.len(),
        });
    }
    let tasks = program
        .tasks
        .iter()
        .map(|task| task.task)
        .collect::<BTreeSet<_>>();
    let mut seen = BTreeSet::new();
    for arm in &plan.arms {
        if !tasks.contains(&arm.task) {
            diagnostics.push(EpochCreditDiagnostic::UnknownTask { task: arm.task });
        }
        if !seen.insert(arm.task) {
            diagnostics.push(EpochCreditDiagnostic::DuplicateArm { task: arm.task });
        }
        if arm.declared.is_some() && !arm.edges.any() {
            diagnostics.push(EpochCreditDiagnostic::ErrorArmWithoutErrorEdge { task: arm.task });
        }
    }
    diagnostics
}

/// Check one epoch's error path and derive its credit split.
///
/// `program` must be the same [`FlatEpochProgram`] that
/// [`crate::epoch_accounting::check_flat_epoch`] is given: the grants are read
/// out of it and never out of the plan, so a plan cannot restore credit from a
/// task the epoch does not grant to.
pub fn check_epoch_credit_recovery(
    program: &FlatEpochProgram,
    plan: &EpochCreditRecoveryPlan,
) -> Result<EpochCreditRecoveryCertificate, Vec<EpochCreditDiagnostic>> {
    let shape = check_shape(program, plan);
    if !shape.is_empty() {
        return Err(shape);
    }

    let grants = program
        .tasks
        .iter()
        .map(|task| (task.task, task.credit.clone()))
        .collect::<BTreeMap<_, _>>();

    let mut diagnostics = Vec::new();
    let mut arms = Vec::new();
    for arm in &plan.arms {
        let credit = grants
            .get(&arm.task)
            .cloned()
            .expect("a shape-checked arm names a task of this epoch");
        let restored = match arm.disposition() {
            ErrorArmCredit::SpentByCreator => Quantity::zero(),
            ErrorArmCredit::RestoredToEpoch { restored } => restored,
            ErrorArmCredit::AbandonedAtErrorArm => {
                // The leak direction. The whole grant is what is abandoned, so
                // the refusal names the first axis the epoch actually granted
                // on; a grant of nothing is a `credit 0 effort 0;` row, and
                // abandoning nothing is still a claim about a path, so the
                // refusal is issued either way.
                let (axis, abandoned) =
                    first_nonzero(&credit).unwrap_or_else(|| ("memory".to_owned(), 0));
                diagnostics.push(EpochCreditDiagnostic::CreditAbandonedAtErrorArm {
                    task: arm.task,
                    axis,
                    abandoned,
                });
                continue;
            }
        };
        // The mint direction, per task and per axis.
        if let Some((axis, restored_amount, credit_amount)) = first_excess(&restored, &credit) {
            diagnostics.push(EpochCreditDiagnostic::RestoredCreditExceedsGrant {
                task: arm.task,
                axis,
                restored: restored_amount,
                credit: credit_amount,
            });
            continue;
        }
        let spent = match subtract(&credit, &restored) {
            Ok(spent) => spent,
            Err(detail) => {
                diagnostics.push(EpochCreditDiagnostic::Arithmetic { detail });
                continue;
            }
        };
        arms.push(CreditArmFigures {
            task: arm.task,
            credit,
            restored,
            spent,
        });
    }

    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }

    let totals = (|| -> Result<_, FlowError> {
        let granted = add_all(arms.iter().map(|arm| &arm.credit))?;
        let restored = add_all(arms.iter().map(|arm| &arm.restored))?;
        let spent = add_all(arms.iter().map(|arm| &arm.spent))?;
        Ok((granted, restored, spent))
    })();
    let (granted_credits, restored_credits, spent_credits) = match totals {
        Ok(totals) => totals,
        Err(detail) => return Err(vec![EpochCreditDiagnostic::Arithmetic { detail }]),
    };

    Ok(EpochCreditRecoveryCertificate {
        profile: PROFILE,
        granted_credits,
        restored_credits,
        spent_credits: spent_credits.clone(),
        error_arm_committed: spent_credits,
        arms,
    })
}

/// Rederive every field of a supplied error-path certificate from the program
/// and the plan alone. The two mutations this module exists to refuse — a split
/// that does not conserve the grant, and a restored total larger than the grant
/// — are named, not folded into a generic mismatch.
pub fn recheck_epoch_credit_recovery(
    program: &FlatEpochProgram,
    plan: &EpochCreditRecoveryPlan,
    certificate: &EpochCreditRecoveryCertificate,
) -> Result<(), Vec<EpochCreditRecheckError>> {
    let derived = match check_epoch_credit_recovery(program, plan) {
        Err(diagnostics) => {
            return Err(vec![EpochCreditRecheckError::PlanRejected(diagnostics)]);
        }
        Ok(derived) => derived,
    };
    let mut errors = Vec::new();

    if certificate.profile != PROFILE {
        errors.push(EpochCreditRecheckError::FieldMismatch { field: "profile" });
    }

    // The conservation identity is checked on the *claimed* figures before any
    // field comparison, so a certificate that balances against the wrong epoch
    // is still named for what it is.
    match add(&certificate.restored_credits, &certificate.spent_credits) {
        Ok(sum) if sum == certificate.granted_credits => {}
        _ => errors.push(EpochCreditRecheckError::CreditLedgerUnbalanced {
            granted: certificate.granted_credits.clone(),
            restored: certificate.restored_credits.clone(),
            spent: certificate.spent_credits.clone(),
        }),
    }
    if !certificate
        .restored_credits
        .componentwise_le(&certificate.granted_credits)
    {
        errors.push(EpochCreditRecheckError::RestoredCreditMinted {
            restored: certificate.restored_credits.clone(),
            granted: certificate.granted_credits.clone(),
        });
    }
    for arm in &certificate.arms {
        match add(&arm.restored, &arm.spent) {
            Ok(sum) if sum == arm.credit => {}
            _ => errors.push(EpochCreditRecheckError::ArmUnbalanced { task: arm.task }),
        }
    }

    for (field, claimed, expected) in [
        (
            "granted_credits",
            &certificate.granted_credits,
            &derived.granted_credits,
        ),
        (
            "restored_credits",
            &certificate.restored_credits,
            &derived.restored_credits,
        ),
        (
            "spent_credits",
            &certificate.spent_credits,
            &derived.spent_credits,
        ),
        (
            "error_arm_committed",
            &certificate.error_arm_committed,
            &derived.error_arm_committed,
        ),
    ] {
        if claimed != expected {
            errors.push(EpochCreditRecheckError::FieldMismatch { field });
        }
    }
    if certificate.arms != derived.arms {
        errors.push(EpochCreditRecheckError::FieldMismatch { field: "arms" });
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
#[path = "../tests/unit/epoch_credit_recovery.rs"]
mod tests;
