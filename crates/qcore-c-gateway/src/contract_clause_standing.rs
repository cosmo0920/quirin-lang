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

//! Card **VERDICT-BODY-STANDING-CLAUSE-SCOPE** (gateway rules 13i–13k): the
//! clauses of a declaration, and which of them each piece of body evidence
//! covers.
//!
//! # Clauses (rule 13i)
//!
//! A declaration is read as the list of its clauses ([`declaration_clauses`]):
//! one per parameter mode (with its kind), the result mode (with its kind),
//! each container and counted-user clause, each retention, the guard of a
//! guarded retention, each holder reset, slot move, derived out slot and
//! reference clause, each exit class of a return-code disposition, and the
//! clauses that live only in the declaration's retained text
//! ([`text_clauses`]): a named premise (C04.18), a variadic tail and the
//! no-return clause. A literal-argument premise (C04.5) and the loop-owner,
//! thread-synchronization and lifecycle obligations bound where a declaration
//! is believed (its callers' obligations); they state nothing about the body,
//! and are not clauses a body must establish.
//!
//! # What each piece of evidence covers (rule 13i)
//!
//! [`covers`] is the whole table:
//!
//! * `W4` (the final derivation's validated summary of the body equals the
//!   declaration's custody projection) and `entry` (the body under the
//!   declaration's entry custody matches every exit) compare the custody
//!   projection: every parameter mode and kind (and the arity) and the result
//!   mode and kind. Nothing else: not a container or counted-user clause
//!   (`ResourceFunctionContract::custody_projection` clears them). Card
//!   DECLARED-ASSUMPTION-BODY-CHECKS: `13m` is the `entry` check on a body
//!   whose W4 summary only refines its declaration
//!   (`legacy_c::declared_entry_check`), and covers the same.
//! * `11r` compares each exit class's custody projection with the body's
//!   summary: every exit class.
//! * `12w` reads the guards of guarded retentions, not the retentions.
//! * `13a` reads slot moves and derived out slots of a retaining declaration
//!   and the borrowed result of a declaration in the function map (its hook
//!   reads no other table, so a borrowed result of a retaining declaration is
//!   an ordinary result clause, [`ClauseKind::Result`]).
//! * `13c` reads reference keeps and drops.
//! * `13d` reads a returned reference.
//!
//! A structural check (11r, 12w, 13a, 13c, 13d) never covers a clause of
//! another kind: in particular never a `resource_borrowed` formal beside the
//! clause it reads. No evidence covers a container or counted-user clause, a
//! retention, a holder reset or a text clause.
//!
//! # Per unit (rule 13j)
//!
//! [`unit_coverage`] is, for every declaration a unit validated, each clause
//! with the evidence that covered it there (empty when none did). The audit
//! folds it into the run's per-clause standing. Evidence only: nothing in the
//! analysis reads it.

use std::collections::{BTreeMap, BTreeSet};

use qcore_schemas::contract::{
    ContainerEffect, DerivedExtent, ResourceContracts, ResourceFunctionContract, ResourceKind,
    ResourceParameterMode, ResourceReturnMode, UserEffect,
};
use qcore_schemas::retained_by::RetentionHolder;

use crate::contract_dependencies::{BodyOutcome, UnitRecord};

/// What reads a clause.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ClauseKind {
    /// A parameter's custody mode and kind.
    Parameter,
    /// The result's mode and kind (not one of the two below).
    Result,
    /// `resource_return_borrowed_from` of a declaration in the function map.
    BorrowedResult,
    /// `resource_return_owned` of kind `reference`.
    ReferenceResult,
    /// `resource_published` / `resource_unpublished`.
    Container,
    /// A counted-user clause.
    CountedUser,
    /// `resource_retained_by`.
    Retention,
    /// The `when` guard of a retention.
    Guard,
    /// `resource_holder_reset`.
    HolderReset,
    /// `resource_slot_moved`.
    SlotMove,
    /// `resource_out_derived`.
    OutDerived,
    /// `resource_reference_kept` / `resource_reference_dropped`.
    Reference,
    /// One exit class of `resource_on_return`.
    ExitClass,
    /// `resource_premise`.
    Premise,
    /// `resource_variadic_data_tail` / `resource_variadic_borrowed_tail`.
    VariadicTail,
    /// `resource_no_return`.
    NoReturn,
    /// Card CG22-CALLBACK-REGISTRY-CONTRACT: a callback clause no body check
    /// reads (a table dispatch, a registry use, an element clause, a
    /// constructor's registration).
    Callback,
    /// Card CG22-CALLBACK-REGISTRY-CONTRACT (rule 13y): a synchronous
    /// invocation, or a registration in a parameter holder.
    CallbackChecked,
    /// Card CG22-OP-CALLBACKS-AND-SERVED-QUEUES (rule 12o): the discriminant
    /// and the arms of a type-switched release.
    Arm,
    /// Card OP-CONSTRUCTOR-DECLARATIONS (rule 14e): a constructor's stated
    /// result state (`quant live #result`, `resource_discriminant_set`, a
    /// member's registration or `absent`).
    MemberState,
    /// The retained text could not be reread: a clause nothing covers.
    Unreadable,
    /// S3 condition bindings, scoped prerequisites and registered footprints.
    LifetimeProtocol,
}

impl ClauseKind {
    /// Every kind, in declaration order.
    pub const ALL: [Self; 22] = [
        Self::Parameter,
        Self::Result,
        Self::BorrowedResult,
        Self::ReferenceResult,
        Self::Container,
        Self::CountedUser,
        Self::Retention,
        Self::Guard,
        Self::HolderReset,
        Self::SlotMove,
        Self::OutDerived,
        Self::Reference,
        Self::ExitClass,
        Self::Premise,
        Self::VariadicTail,
        Self::NoReturn,
        Self::Callback,
        Self::CallbackChecked,
        Self::Arm,
        Self::MemberState,
        Self::Unreadable,
        Self::LifetimeProtocol,
    ];

    /// The spelling the shared vectors use (`QCore.BodyClauseStanding`).
    #[must_use]
    pub const fn spelling(self) -> &'static str {
        match self {
            Self::Parameter => "parameter",
            Self::Result => "result",
            Self::BorrowedResult => "borrowedResult",
            Self::ReferenceResult => "referenceResult",
            Self::Container => "container",
            Self::CountedUser => "countedUser",
            Self::Retention => "retention",
            Self::Guard => "guard",
            Self::HolderReset => "holderReset",
            Self::SlotMove => "slotMove",
            Self::OutDerived => "outDerived",
            Self::Reference => "reference",
            Self::ExitClass => "exitClass",
            Self::Premise => "premise",
            Self::VariadicTail => "variadicTail",
            Self::NoReturn => "noReturn",
            Self::Callback => "callback",
            Self::CallbackChecked => "callbackChecked",
            Self::Arm => "arm",
            Self::MemberState => "memberState",
            Self::Unreadable => "unreadable",
            Self::LifetimeProtocol => "lifetimeProtocol",
        }
    }

    /// Whether the clause is part of the custody projection W4 and `entry`
    /// compare.
    #[must_use]
    pub const fn is_custody(self) -> bool {
        matches!(
            self,
            Self::Parameter | Self::Result | Self::BorrowedResult | Self::ReferenceResult
        )
    }
}

/// The evidence spellings of rule 13g, in the order the report names them;
/// card DECLARED-ASSUMPTION-BODY-CHECKS adds `13m` (the declaration's own
/// entry check on a body whose W4 summary only refines it); card
/// CG22-CALLBACK-REGISTRY-CONTRACT adds `13y` (the callback body check);
/// card CG22-OP-CALLBACKS-AND-SERVED-QUEUES adds `12o` (the arm check);
/// card OP-CONSTRUCTOR-DECLARATIONS adds `14e` (the result-state check);
/// card OP-CALLBACK-MEMBER-PATH-RELEASE adds `30` (the member-path check).
pub const EVIDENCE: [&str; 12] = [
    "W4", "entry", "11r", "12w", "13a", "13c", "13d", "13m", "13y", "12o", "14e", "30",
];

/// Whether the evidence spelled `evidence` covers a clause of `kind` (rule
/// 13i). Any other spelling covers nothing.
#[must_use]
pub fn covers(evidence: &str, kind: ClauseKind) -> bool {
    match evidence {
        // Card DECLARED-ASSUMPTION-BODY-CHECKS (rule 13m): the `entry` check,
        // on a body whose W4 summary only refines its declaration.
        "W4" | "entry" | "13m" => kind.is_custody(),
        "11r" => kind == ClauseKind::ExitClass,
        "12w" => kind == ClauseKind::Guard,
        "13a" => matches!(
            kind,
            ClauseKind::SlotMove | ClauseKind::OutDerived | ClauseKind::BorrowedResult
        ),
        "13c" => kind == ClauseKind::Reference,
        "13d" => kind == ClauseKind::ReferenceResult,
        // Card CG22-CALLBACK-REGISTRY-CONTRACT (rule 13y).
        "13y" => kind == ClauseKind::CallbackChecked,
        // Card CG22-OP-CALLBACKS-AND-SERVED-QUEUES (rule 12o).
        "12o" => kind == ClauseKind::Arm,
        // Card OP-CONSTRUCTOR-DECLARATIONS (rule 14e).
        "14e" => kind == ClauseKind::MemberState,
        // Card OP-CALLBACK-MEMBER-PATH-RELEASE (rule 30).
        "30" => kind == ClauseKind::MemberState,
        _ => false,
    }
}

/// One clause of a declaration: what reads it and how the report names it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Clause {
    pub kind: ClauseKind,
    pub label: String,
}

impl Clause {
    fn new(kind: ClauseKind, label: String) -> Self {
        Self { kind, label }
    }
}

fn nullability(nullable: bool, kind: ResourceKind) -> &'static str {
    match (kind.is_integer(), nullable) {
        (false, true) => "nullable",
        (false, false) => "nonnull",
        (true, true) => "sentinel",
        (true, false) => "valid",
    }
}

fn kinded(label: String, kind: ResourceKind) -> String {
    if kind.is_heap() {
        label
    } else {
        format!("{label} kind {}", kind.spelling())
    }
}

fn parameter_clause(index: usize, mode: ResourceParameterMode, kind: ResourceKind) -> Clause {
    let label = match mode {
        ResourceParameterMode::Borrowed => format!("resource_borrowed(#{index})"),
        ResourceParameterMode::Owned { nullable } => {
            format!("resource_owned(#{index}, {})", nullability(nullable, kind))
        }
        ResourceParameterMode::Consumed { nullable } => {
            format!(
                "resource_consumed(#{index}, {})",
                nullability(nullable, kind)
            )
        }
        ResourceParameterMode::Released { nullable } => {
            format!(
                "resource_released(#{index}, {})",
                nullability(nullable, kind)
            )
        }
        ResourceParameterMode::Out { nullable } => {
            format!("resource_out(#{index}, {})", nullability(nullable, kind))
        }
        ResourceParameterMode::Stored {
            owner_parameter,
            nullable,
        } => format!(
            "resource_stored(#{index}, #{owner_parameter}, {})",
            nullability(nullable, kind)
        ),
        ResourceParameterMode::Removed {
            owner_parameter,
            nullable,
        } => format!(
            "resource_removed(#{index}, #{owner_parameter}, {})",
            nullability(nullable, kind)
        ),
    };
    Clause::new(ClauseKind::Parameter, kinded(label, kind))
}

/// The result clause; `function_map` says whether the declaration is an entry
/// of the function map (where rule 13a reads a borrowed result).
fn result_clause(contract: &ResourceFunctionContract, function_map: bool) -> Clause {
    let kind = contract.result_kind;
    let (clause_kind, label) = match contract.result {
        ResourceReturnMode::None => (ClauseKind::Result, "resource_return_none()".to_owned()),
        ResourceReturnMode::Owned { nullable } => (
            if kind == ResourceKind::Reference {
                ClauseKind::ReferenceResult
            } else {
                ClauseKind::Result
            },
            format!("resource_return_owned({})", nullability(nullable, kind)),
        ),
        ResourceReturnMode::Removed {
            parameter,
            nullable,
        } => (
            ClauseKind::Result,
            format!(
                "resource_return_removed(#{parameter}, {})",
                nullability(nullable, kind)
            ),
        ),
        ResourceReturnMode::Reallocated {
            parameter,
            nullable,
        } => (
            ClauseKind::Result,
            format!(
                "resource_return_reallocated(#{parameter}, {})",
                nullability(nullable, kind)
            ),
        ),
        ResourceReturnMode::Derived {
            parameter,
            extent,
            nullable,
            ..
        } => {
            let n = nullability(nullable, ResourceKind::Heap);
            match extent {
                DerivedExtent::Exact => (
                    ClauseKind::Result,
                    format!("resource_return_alias(#{parameter}, {n})"),
                ),
                DerivedExtent::Interior => (
                    ClauseKind::Result,
                    format!("resource_return_interior(#{parameter}, {n})"),
                ),
                DerivedExtent::Borrowed => (
                    if function_map {
                        ClauseKind::BorrowedResult
                    } else {
                        ClauseKind::Result
                    },
                    format!("resource_return_borrowed_from(#{parameter}, {n})"),
                ),
            }
        }
    };
    Clause::new(clause_kind, kinded(label, kind))
}

fn user_effect(effect: UserEffect) -> &'static str {
    match effect {
        UserEffect::Added => "resource_user_added",
        UserEffect::Dropped => "resource_user_dropped",
        UserEffect::DroppedChecked => "resource_user_dropped_checked",
        UserEffect::ReleaseCheck => "resource_user_release_check",
    }
}

/// The clauses of one custody contract; `function_map` as [`result_clause`].
fn contract_clauses(contract: &ResourceFunctionContract, function_map: bool) -> Vec<Clause> {
    let mut clauses = contract
        .parameters
        .iter()
        .copied()
        .enumerate()
        .map(|(index, mode)| parameter_clause(index, mode, contract.parameter_kind(index)))
        .collect::<Vec<_>>();
    clauses.push(result_clause(contract, function_map));
    for (index, effect) in &contract.container_effects {
        let label = match effect {
            ContainerEffect::Published {
                container_parameter,
            } => format!("resource_published(#{index}, #{container_parameter})"),
            ContainerEffect::Unpublished => format!("resource_unpublished(#{index})"),
            ContainerEffect::ElementsReleased => {
                format!("resource_elements_released(#{index})")
            }
        };
        clauses.push(Clause::new(ClauseKind::Container, label));
    }
    let users = &contract.counted_users;
    for (index, operation) in &users.effects {
        clauses.push(Clause::new(
            ClauseKind::CountedUser,
            format!(
                "{}(#{index}, {})",
                user_effect(operation.effect),
                operation.counter
            ),
        ));
    }
    if users.neutral {
        clauses.push(Clause::new(
            ClauseKind::CountedUser,
            "resource_user_neutral()".to_owned(),
        ));
    }
    for exit in &users.exits {
        clauses.push(Clause::new(
            ClauseKind::CountedUser,
            format!(
                "resource_user_exit(#{}, {}, {})",
                exit.parameter, exit.returned, exit.delta
            ),
        ));
    }
    clauses
}

/// Every clause the declaration of `name` in `environment` states in its
/// tables (rule 13i), in table order; empty when no table declares it. The
/// text clauses are [`text_clauses`].
#[must_use]
pub fn declaration_clauses(environment: &ResourceContracts, name: &str) -> Vec<Clause> {
    if let Some(contract) = environment.functions().get(name) {
        return contract_clauses(contract, true);
    }
    if let Some(declared) = environment.relational().get(name) {
        return declared
            .classes
            .iter()
            .map(|class| {
                Clause::new(
                    ClauseKind::ExitClass,
                    format!(
                        "resource_on_return({}, {})",
                        class.relation.op.spelling(),
                        class.relation.constant
                    ),
                )
            })
            .collect();
    }
    let Some(declared) = environment
        .retaining()
        .get(name)
        .or_else(|| environment.guarded_retaining().get(name))
    else {
        return Vec::new();
    };
    let mut clauses = contract_clauses(&declared.contract, false);
    if let Some(lifetime) = &declared.lifetime {
        for binding in &lifetime.bindings {
            clauses.push(Clause::new(
                ClauseKind::LifetimeProtocol,
                format!(
                    "quant #{} = (#{}.{} != 0)",
                    binding.name,
                    binding.parameter,
                    binding.members.join(".")
                ),
            ));
        }
        for protocol in &lifetime.registered {
            clauses.push(Clause::new(
                ClauseKind::LifetimeProtocol,
                format!(
                    "quant registered #{}: {}.{} |-> {}.{}",
                    protocol.binder,
                    protocol.record,
                    protocol.interior.join("."),
                    protocol.record,
                    protocol.holder.join(".")
                ),
            ));
        }
        for block in &lifetime.blocks {
            for clause in &block.clauses {
                clauses.push(Clause::new(
                    ClauseKind::LifetimeProtocol,
                    format!(
                        "quant {}#{} in {{ {:?} }}: [[local]]",
                        if block.guard.negated { "not " } else { "" },
                        block.guard.condition,
                        clause
                    ),
                ));
            }
        }
    }

    for retention in &declared.retentions {
        let holder = match retention.holder {
            RetentionHolder::Parameter { index } => format!("#{index}"),
            RetentionHolder::Global => "global".to_owned(),
            RetentionHolder::Result => "result".to_owned(),
        };
        let label = format!("resource_retained_by(#{}, {holder})", retention.retained);
        if let Some(guard) = retention.guard {
            clauses.push(Clause::new(
                ClauseKind::Guard,
                format!(
                    "{label} when #{} {} {}",
                    guard.parameter,
                    guard.relation.spelling(),
                    guard.constant
                ),
            ));
        }
        clauses.push(Clause::new(ClauseKind::Retention, label));
    }
    for holder in &declared.resets {
        clauses.push(Clause::new(
            ClauseKind::HolderReset,
            format!("resource_holder_reset(#{holder})"),
        ));
    }
    for moved in &declared.slot_moves {
        clauses.push(Clause::new(
            ClauseKind::SlotMove,
            format!(
                "resource_slot_moved(#{}, {})",
                moved.slot,
                nullability(moved.nullable, ResourceKind::Heap)
            ),
        ));
    }
    for out in &declared.derived_outs {
        clauses.push(Clause::new(
            ClauseKind::OutDerived,
            format!(
                "resource_out_derived(#{}, #{}, {})",
                out.slot,
                out.base,
                nullability(out.nullable, ResourceKind::Heap)
            ),
        ));
    }
    for reference in &declared.references {
        clauses.push(Clause::new(
            ClauseKind::Reference,
            format!(
                "{}(#{})",
                reference.operation.predicate(),
                reference.parameter
            ),
        ));
    }
    // Card CG22-CALLBACK-REGISTRY-CONTRACT (rules 13u-13y).
    for clause in &declared.callbacks {
        clauses.push(callback_clause(clause));
    }
    // Card CG22-OP-CALLBACKS-AND-SERVED-QUEUES (rule 12o).
    if let Some(discriminant) = &declared.discriminant {
        clauses.push(Clause::new(
            ClauseKind::Arm,
            format!(
                "resource_discriminant(#{}, {}, {})",
                discriminant.parameter, discriminant.field, discriminant.mask
            ),
        ));
        for arm in &declared.arms {
            use qcore_schemas::arm_release::{ArmEffect, ArmGuard};
            let effect = match &arm.effect {
                ArmEffect::Released { path } => {
                    format!("resource_arm_released(#{}, {path})", discriminant.parameter)
                }
                ArmEffect::Empty => format!("resource_arm_empty(#{})", discriminant.parameter),
            };
            let guard = match arm.guard {
                ArmGuard::Equals { value } => format!("when {} == {value}", discriminant.field),
                ArmGuard::Otherwise => "otherwise".to_owned(),
            };
            clauses.push(Clause::new(ClauseKind::Arm, format!("{effect} {guard}")));
        }
    }
    // Card OP-CONSTRUCTOR-DECLARATIONS (rule 14e): the result state, by its
    // canonical parts (both levels of the shorthand read alike).
    let state = &declared.constructed;
    if state.live {
        clauses.push(Clause::new(
            ClauseKind::MemberState,
            "quant live #result".to_owned(),
        ));
    }
    if let Some(set) = &state.discriminant {
        clauses.push(Clause::new(
            ClauseKind::MemberState,
            format!(
                "resource_discriminant_set(result, {}, #{}, {})",
                set.member.spelling(),
                set.parameter,
                set.flags
            ),
        ));
    }
    for member in &state.members {
        use qcore_schemas::constructed_state::{ABSENT, MemberValue};
        let value = match &member.value {
            MemberValue::Callback { callback } => format!("#{callback}"),
            MemberValue::Absent => ABSENT.to_owned(),
            // Card OP-APP-RK-CALLBACKS-VIA-POLL-CB (D1): a stated default.
            MemberValue::Function { function } => function.clone(),
        };
        clauses.push(Clause::new(
            ClauseKind::MemberState,
            format!(
                "resource_callback_member_registered(result, {}, {value})",
                member.member.spelling()
            ),
        ));
    }
    // Card OP-CALLBACK-MEMBER-PATH-RELEASE (rule 30): each member-path state.
    for state in &declared.member_paths {
        let guard = state
            .guard
            .as_ref()
            .map(|guard| format!(" when {}", guard.spelling()))
            .unwrap_or_default();
        clauses.push(Clause::new(
            ClauseKind::MemberState,
            format!(
                "quant live #{}: {} |-> {}{guard}",
                state.parameter,
                state.member.spelling(),
                state.state.spelling()
            ),
        ));
    }
    clauses
}

/// One callback clause, labelled by its positions.
fn callback_clause(clause: &qcore_schemas::callback::CallbackClause) -> Clause {
    use qcore_schemas::callback::{CallbackActual, CallbackClause, CallbackHolder};
    let actuals = |actuals: &[CallbackActual]| {
        actuals
            .iter()
            .map(|actual| match actual {
                CallbackActual::Parameter { index } => format!(", #{index}"),
                CallbackActual::Supplied => ", _".to_owned(),
                CallbackActual::Element => ", element".to_owned(),
                CallbackActual::SuppliedOwned => ", supplied_owned".to_owned(),
            })
            .collect::<String>()
    };
    let keyword = clause.keyword();
    let (kind, arguments) = match clause {
        CallbackClause::Invoked {
            callback,
            actuals: a,
        } => (
            ClauseKind::CallbackChecked,
            format!("#{callback}{}", actuals(a)),
        ),
        CallbackClause::Registered {
            callback,
            holder: CallbackHolder::Parameter { index },
        } => (
            ClauseKind::CallbackChecked,
            format!("#{callback}, #{index}"),
        ),
        CallbackClause::Registered {
            callback,
            holder: CallbackHolder::Result,
        } => (ClauseKind::Callback, format!("#{callback}, result")),
        CallbackClause::Table { table, actuals: a } => {
            (ClauseKind::Callback, format!("#{table}{}", actuals(a)))
        }
        CallbackClause::RegistryInvoked { holder, actuals: a } => {
            (ClauseKind::Callback, format!("#{holder}{}", actuals(a)))
        }
        CallbackClause::ElementAdded { element, holder } => {
            (ClauseKind::Callback, format!("#{element}, #{holder}"))
        }
        CallbackClause::Unregistered { holder }
        | CallbackClause::ElementsDropped { holder }
        | CallbackClause::ReleasedIfConstructed { holder } => {
            (ClauseKind::Callback, format!("#{holder}"))
        }
        // Card CG22-OP-CALLBACKS-AND-SERVED-QUEUES (rule 12n): a declared
        // assumption (no body check reads it).
        CallbackClause::MemberInvoked {
            object,
            member,
            actuals: a,
        } => {
            let object = match object {
                qcore_schemas::callback::MemberObject::Parameter { index } => format!("#{index}"),
                qcore_schemas::callback::MemberObject::SuppliedOwned => "supplied_owned".to_owned(),
            };
            (
                ClauseKind::Callback,
                format!("{object}, {}{}", member.spelling(), actuals(a)),
            )
        }
        // Card OP-APP-RK-CALLBACKS-VIA-POLL-CB (rule 12n, Q2): a setter's
        // registration, a declared assumption.
        CallbackClause::MemberRegistered {
            object,
            member,
            callback,
            by_id: None,
        } => (
            ClauseKind::Callback,
            format!("#{object}, {}, #{callback}", member.spelling()),
        ),
        // Card FB-EVENT-LOOP-RETENTION-AND-REGISTRATION (rule 12n): a
        // registration forwarded through the returned id.
        CallbackClause::MemberRegistered {
            object: _,
            member,
            callback,
            by_id: Some(link),
        } => (
            ClauseKind::Callback,
            format!("#{}, {}, #{callback}", link.binder, member.spelling()),
        ),
    };
    // Card FB-EVENT-LOOP-RETENTION-AND-REGISTRATION: the id's binding.
    let binding = match clause {
        CallbackClause::MemberRegistered {
            object,
            by_id: Some(link),
            ..
        } => format!(
            " where #{} = #result in #{object}.{}",
            link.binder,
            link.list.join(".")
        ),
        _ => String::new(),
    };
    Clause::new(kind, format!("{keyword}({arguments}){binding}"))
}

/// The clauses of every declaration of `environment` that live only in its
/// retained text (rule 13i): named premises, variadic tails and the no-return
/// clause, rederived from the retained sources. A declaration with none is
/// absent; an environment with no retained sources has none.
///
/// # Errors
///
/// The loader's refusal of the retained texts. A caller MUST then treat every
/// declaration it reads as carrying an uncovered [`ClauseKind::Unreadable`]
/// clause, never as carrying none.
pub fn text_clauses(
    environment: &ResourceContracts,
) -> Result<BTreeMap<String, Vec<Clause>>, String> {
    use qcore_contract::contract_loader::{
        PremiseOrigin, declared_no_return_callees, declared_premises,
        declared_variadic_borrowed_tails, declared_variadic_data_tails,
    };
    let mut clauses = BTreeMap::<String, Vec<Clause>>::new();
    for (name, premises) in declared_premises(environment)? {
        for premise in premises {
            if premise.origin == PremiseOrigin::Declared {
                clauses.entry(name.clone()).or_default().push(Clause::new(
                    ClauseKind::Premise,
                    format!("resource_premise({})", premise.name),
                ));
            }
        }
    }
    let borrowed = declared_variadic_borrowed_tails(environment)?;
    for name in &borrowed {
        clauses.entry(name.clone()).or_default().push(Clause::new(
            ClauseKind::VariadicTail,
            "resource_variadic_borrowed_tail()".to_owned(),
        ));
    }
    for name in declared_variadic_data_tails(environment)? {
        if !borrowed.contains(&name) {
            clauses.entry(name).or_default().push(Clause::new(
                ClauseKind::VariadicTail,
                "resource_variadic_data_tail()".to_owned(),
            ));
        }
    }
    for name in declared_no_return_callees(environment)? {
        clauses.entry(name).or_default().push(Clause::new(
            ClauseKind::NoReturn,
            "resource_no_return()".to_owned(),
        ));
    }
    Ok(clauses)
}

/// One validated declaration's coverage in one unit: each clause label with
/// the evidence that covered it there (empty when none did).
pub type Coverage = BTreeMap<String, BTreeSet<String>>;

/// The coverage of `clauses` by the evidence spellings `evidence`.
#[must_use]
pub fn coverage_of<'a>(
    clauses: impl IntoIterator<Item = &'a Clause>,
    evidence: &BTreeSet<String>,
) -> Coverage {
    let mut coverage = Coverage::new();
    for clause in clauses {
        let by = evidence
            .iter()
            .filter(|by| covers(by, clause.kind))
            .cloned()
            .collect::<BTreeSet<_>>();
        coverage.entry(clause.label.clone()).or_default().extend(by);
    }
    coverage
}

/// For every declaration of `environment` this unit validated (rule 13g's
/// [`BodyOutcome::Validated`]), each of its clauses with the evidence that
/// covered it here (rule 13j). Text clauses come from [`text_clauses`], read
/// only when the unit validated a declaration; when the texts cannot be
/// reread, every declaration carries an uncovered `Unreadable` clause.
#[must_use]
pub fn unit_coverage(
    record: &UnitRecord,
    environment: &ResourceContracts,
) -> BTreeMap<String, Coverage> {
    let validated = record
        .body_outcomes()
        .into_iter()
        .filter(|(name, outcome)| {
            *outcome == BodyOutcome::Validated
                && crate::contract_dependencies::declared(environment, name)
        })
        .map(|(name, _)| name)
        .collect::<Vec<_>>();
    if validated.is_empty() {
        return BTreeMap::new();
    }
    let text = text_clauses(environment);
    validated
        .into_iter()
        .map(|name| {
            let mut clauses = declaration_clauses(environment, &name);
            match &text {
                Ok(text) => clauses.extend(text.get(&name).into_iter().flatten().cloned()),
                Err(_) => clauses.push(Clause::new(
                    ClauseKind::Unreadable,
                    "retained text unreadable".to_owned(),
                )),
            }
            let evidence = record.validated.get(&name).cloned().unwrap_or_default();
            let coverage = coverage_of(&clauses, &evidence);
            (name, coverage)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use qcore_schemas::retained_by::{RetainingContract, Retention};

    #[test]
    fn a_structural_check_covers_only_the_kind_it_reads() {
        for kind in ClauseKind::ALL {
            assert_eq!(covers("W4", kind), kind.is_custody());
            assert_eq!(covers("entry", kind), kind.is_custody());
            assert_eq!(covers("13m", kind), kind.is_custody());
            assert!(!covers("13n", kind));
            assert!(!covers("13c", kind) || kind == ClauseKind::Reference);
            assert!(!covers("12w", kind) || kind == ClauseKind::Guard);
            assert!(!covers("unknown", kind));
        }
        for evidence in ["11r", "12w", "13a", "13c", "13d"] {
            assert!(!covers(evidence, ClauseKind::Parameter), "{evidence}");
        }
    }

    #[test]
    fn a_reference_keep_lists_its_formals_reference_and_result() {
        let mut environment = ResourceContracts::default();
        environment.insert_retaining(
            "keep",
            RetainingContract {
                contract: ResourceFunctionContract::new(
                    [ResourceParameterMode::Borrowed],
                    ResourceReturnMode::Derived {
                        parameter: 0,
                        extent: DerivedExtent::Exact,
                        nullable: false,
                        const_dropped: false,
                    },
                ),
                retentions: vec![Retention {
                    retained: 0,
                    holder: RetentionHolder::Global,
                    guard: None,
                }],
                resets: Vec::new(),
                slot_moves: Vec::new(),
                derived_outs: Vec::new(),
                references: vec![qcore_schemas::reference_count::ReferenceClause {
                    parameter: 0,
                    operation: qcore_schemas::reference_count::ReferenceOperation::Kept,
                }],
                references_withdrawn: false,
                callbacks: Vec::new(),
                callbacks_withdrawn: false,
                discriminant: None,
                arms: Vec::new(),
                arms_withdrawn: false,
                constructed: Default::default(),
                constructed_withdrawn: false,
                handoff: None,
                member_paths: Vec::new(),
                member_paths_withdrawn: false,
                path_retentions: Vec::new(),
                removals: Vec::new(),
                descriptor_cells: Default::default(),
                lifetime: None,
            },
        );
        let clauses = declaration_clauses(&environment, "keep");
        let labels = clauses
            .iter()
            .map(|clause| clause.label.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            labels,
            [
                "resource_borrowed(#0)",
                "resource_return_alias(#0, nonnull)",
                "resource_retained_by(#0, global)",
                "resource_reference_kept(#0)",
            ]
        );
        let coverage = coverage_of(&clauses, &BTreeSet::from(["13c".to_owned()]));
        assert_eq!(
            coverage
                .iter()
                .filter(|(_, by)| !by.is_empty())
                .map(|(label, _)| label.as_str())
                .collect::<Vec<_>>(),
            ["resource_reference_kept(#0)"]
        );
    }

    #[test]
    fn a_borrowed_result_outside_the_function_map_is_an_ordinary_result() {
        let contract = ResourceFunctionContract::new(
            [ResourceParameterMode::Borrowed],
            ResourceReturnMode::Derived {
                parameter: 0,
                extent: DerivedExtent::Borrowed,
                nullable: true,
                const_dropped: false,
            },
        );
        assert_eq!(
            result_clause(&contract, true).kind,
            ClauseKind::BorrowedResult
        );
        assert_eq!(result_clause(&contract, false).kind, ClauseKind::Result);
    }
}
