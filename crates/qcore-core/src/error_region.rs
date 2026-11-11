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

//! Executable semantics for Cycle 6 error regions and sealed cleanup packs.
//!
//! The model is deliberately path-sensitive.  Every error edge carries the
//! exact cleanup pack produced by its path, and opening that pack creates a
//! linear capability which must be discharged before the handler can exit.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::{Effect, EffectRow, ErrorDeclarations, ErrorLedger, ErrorName, ResourceId};

pub type RegionName = String;
pub type CleanupBinding = String;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CleanupAction {
    Destroy {
        resource: ResourceId,
        noexcept: bool,
    },
    Rollback {
        name: String,
        noexcept: bool,
    },
}

impl CleanupAction {
    fn is_noexcept(&self) -> bool {
        match self {
            Self::Destroy { noexcept, .. } | Self::Rollback { noexcept, .. } => *noexcept,
        }
    }

    fn resource(&self) -> Option<ResourceId> {
        match self {
            Self::Destroy { resource, .. } => Some(*resource),
            Self::Rollback { .. } => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CleanupEvent {
    Destroy(ResourceId),
    Rollback(String),
    AbandonCredit(u64),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CleanupPack {
    region: RegionName,
    error: ErrorName,
    baseline: ErrorLedger,
    actions: Vec<CleanupAction>,
    credit_remainder: u64,
}

impl CleanupPack {
    pub fn region(&self) -> &str {
        &self.region
    }

    pub fn error(&self) -> &str {
        &self.error
    }

    pub fn baseline(&self) -> &ErrorLedger {
        &self.baseline
    }

    pub fn actions(&self) -> &[CleanupAction] {
        &self.actions
    }

    pub fn credit_remainder(&self) -> u64 {
        self.credit_remainder
    }

    pub fn cleanup_resources(&self) -> impl Iterator<Item = ResourceId> + '_ {
        self.actions.iter().filter_map(CleanupAction::resource)
    }

    fn is_empty(&self) -> bool {
        self.actions.is_empty() && self.credit_remainder == 0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuantOpen {
    pub region: RegionName,
    pub binding: CleanupBinding,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegionCatchClause {
    pub error: ErrorName,
    pub quant_open: Option<QuantOpen>,
    pub handler: Vec<RegionOp>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RegionOp {
    Acquire {
        resource: ResourceId,
        destructor_noexcept: bool,
    },
    Drop(ResourceId),
    MoveToErrorPayload(ResourceId),
    DropPayload(ResourceId),
    AddStrictCredit(u64),
    SpendStrictCredit(u64),
    Defer {
        name: String,
        noexcept: bool,
    },
    Raise(ErrorName),
    Branch {
        then_ops: Vec<RegionOp>,
        else_ops: Vec<RegionOp>,
    },
    ErrorRegion {
        region: RegionName,
        body: Vec<RegionOp>,
        catches: Vec<RegionCatchClause>,
    },
    /// Surface `quant open` belongs on a catch clause.  This IR operation is
    /// retained so an invalid opening outside a catch has an explicit Q302.
    QuantOpen {
        region: RegionName,
        binding: CleanupBinding,
    },
    Discharge(CleanupBinding),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RegionExitTag {
    Normal,
    Error(ErrorName),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RegionFrame {
    name: RegionName,
    baseline: ErrorLedger,
    actions: Vec<CleanupAction>,
    credit_remainder: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct OpenCapability {
    binding: CleanupBinding,
    pack: CleanupPack,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegionExitState {
    ledger: ErrorLedger,
    payload: BTreeSet<ResourceId>,
    sealed_pack: Option<CleanupPack>,
    cleanup_trace: Vec<CleanupEvent>,
    frames: Vec<RegionFrame>,
    open_capability: Option<OpenCapability>,
    consumed_bindings: BTreeSet<CleanupBinding>,
}

impl RegionExitState {
    pub fn ledger(&self) -> &ErrorLedger {
        &self.ledger
    }

    pub fn payload(&self) -> impl Iterator<Item = ResourceId> + '_ {
        self.payload.iter().copied()
    }

    pub fn sealed_pack(&self) -> Option<&CleanupPack> {
        self.sealed_pack.as_ref()
    }

    pub fn cleanup_trace(&self) -> &[CleanupEvent] {
        &self.cleanup_trace
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RegionExitMap(BTreeMap<RegionExitTag, Vec<RegionExitState>>);

impl RegionExitMap {
    pub fn states(&self, tag: &RegionExitTag) -> &[RegionExitState] {
        self.0.get(tag).map(Vec::as_slice).unwrap_or(&[])
    }

    pub fn iter(&self) -> impl Iterator<Item = (&RegionExitTag, &[RegionExitState])> {
        self.0.iter().map(|(tag, states)| (tag, states.as_slice()))
    }

    fn singleton_normal(state: RegionExitState) -> Self {
        let mut exits = Self::default();
        exits.push(RegionExitTag::Normal, state);
        exits
    }

    fn push(&mut self, tag: RegionExitTag, state: RegionExitState) {
        self.0.entry(tag).or_default().push(state);
    }

    fn append(&mut self, other: Self) {
        for (tag, states) in other.0 {
            self.0.entry(tag).or_default().extend(states);
        }
    }
}

struct Analyzer<'a> {
    declarations: &'a ErrorDeclarations,
    allowed: &'a EffectRow,
}

impl Analyzer<'_> {
    fn block(
        &self,
        mut exits: RegionExitMap,
        operations: &[RegionOp],
    ) -> Result<RegionExitMap, ErrorRegionError> {
        for operation in operations {
            let mut next = RegionExitMap::default();
            for (tag, states) in exits.0 {
                if tag == RegionExitTag::Normal {
                    for state in states {
                        next.append(self.operation(state, operation)?);
                    }
                } else {
                    next.0.entry(tag).or_default().extend(states);
                }
            }
            exits = next;
        }
        Ok(exits)
    }

    fn operation(
        &self,
        mut state: RegionExitState,
        operation: &RegionOp,
    ) -> Result<RegionExitMap, ErrorRegionError> {
        match operation {
            RegionOp::Acquire {
                resource,
                destructor_noexcept,
            } => {
                if !destructor_noexcept {
                    return Err(ErrorRegionError::FallibleCleanup);
                }
                if state.payload.contains(resource)
                    || state
                        .open_capability
                        .as_ref()
                        .is_some_and(|cap| cap.pack.cleanup_resources().any(|id| id == *resource))
                {
                    return Err(ErrorRegionError::DuplicateResourceIdentity(*resource));
                }
                state.ledger.acquire_public(*resource)?;
                let frame = state
                    .frames
                    .last_mut()
                    .ok_or(ErrorRegionError::AcquisitionOutsideRegion)?;
                frame.actions.push(CleanupAction::Destroy {
                    resource: *resource,
                    noexcept: true,
                });
                Ok(RegionExitMap::singleton_normal(state))
            }
            RegionOp::Drop(resource) => {
                if Self::resource_is_sealed(&state, *resource) {
                    return Err(ErrorRegionError::ResourceHeldByCleanupPack(*resource));
                }
                state.ledger.release_public(*resource)?;
                Self::remove_resource_action(&mut state.frames, *resource)?;
                Ok(RegionExitMap::singleton_normal(state))
            }
            RegionOp::MoveToErrorPayload(resource) => {
                if Self::resource_is_sealed(&state, *resource) {
                    return Err(ErrorRegionError::ResourceHeldByCleanupPack(*resource));
                }
                state.ledger.release_public(*resource)?;
                Self::remove_resource_action(&mut state.frames, *resource)?;
                if !state.payload.insert(*resource) {
                    return Err(ErrorRegionError::DuplicateResourceIdentity(*resource));
                }
                Ok(RegionExitMap::singleton_normal(state))
            }
            RegionOp::DropPayload(resource) => {
                if !state.payload.remove(resource) {
                    return Err(ErrorRegionError::PayloadResourceNotOwned(*resource));
                }
                state.cleanup_trace.push(CleanupEvent::Destroy(*resource));
                Ok(RegionExitMap::singleton_normal(state))
            }
            RegionOp::AddStrictCredit(amount) => {
                let frame = state
                    .frames
                    .last_mut()
                    .ok_or(ErrorRegionError::CreditOutsideRegion)?;
                frame.credit_remainder = frame
                    .credit_remainder
                    .checked_add(*amount)
                    .ok_or(ErrorRegionError::CreditOverflow)?;
                Ok(RegionExitMap::singleton_normal(state))
            }
            RegionOp::SpendStrictCredit(amount) => {
                let frame = state
                    .frames
                    .last_mut()
                    .ok_or(ErrorRegionError::CreditOutsideRegion)?;
                frame.credit_remainder = frame
                    .credit_remainder
                    .checked_sub(*amount)
                    .ok_or(ErrorRegionError::InsufficientCredit)?;
                Ok(RegionExitMap::singleton_normal(state))
            }
            RegionOp::Defer { name, noexcept } => {
                if !noexcept {
                    return Err(ErrorRegionError::FallibleCleanup);
                }
                let frame = state
                    .frames
                    .last_mut()
                    .ok_or(ErrorRegionError::DeferOutsideRegion)?;
                frame.actions.push(CleanupAction::Rollback {
                    name: name.clone(),
                    noexcept: true,
                });
                Ok(RegionExitMap::singleton_normal(state))
            }
            RegionOp::Raise(error) => {
                self.require_error(error)?;
                if state.open_capability.is_some() {
                    return Err(ErrorRegionError::RethrowWithUndischargedCleanup);
                }
                let mut exits = RegionExitMap::default();
                exits.push(RegionExitTag::Error(error.clone()), state);
                Ok(exits)
            }
            RegionOp::Branch { then_ops, else_ops } => {
                let left = self.block(RegionExitMap::singleton_normal(state.clone()), then_ops)?;
                let right = self.block(RegionExitMap::singleton_normal(state), else_ops)?;
                let mut result = left;
                result.append(right);
                Self::validate_normal_join(&result)?;
                Ok(result)
            }
            RegionOp::ErrorRegion {
                region,
                body,
                catches,
            } => self.error_region(state, region, body, catches),
            RegionOp::QuantOpen { .. } => Err(ErrorRegionError::QuantOpenOutsideCatch),
            RegionOp::Discharge(binding) => self.discharge(state, binding),
        }
    }

    fn error_region(
        &self,
        mut state: RegionExitState,
        region: &str,
        body: &[RegionOp],
        catches: &[RegionCatchClause],
    ) -> Result<RegionExitMap, ErrorRegionError> {
        if state.frames.iter().any(|frame| frame.name == region) {
            return Err(ErrorRegionError::DuplicateActiveRegion(region.into()));
        }
        let mut clauses = BTreeMap::<&str, &RegionCatchClause>::new();
        for clause in catches {
            self.require_error(&clause.error)?;
            if clauses.insert(&clause.error, clause).is_some() {
                return Err(ErrorRegionError::DuplicateCatchClause(clause.error.clone()));
            }
        }

        state.frames.push(RegionFrame {
            name: region.into(),
            baseline: state.ledger.clone(),
            actions: Vec::new(),
            credit_remainder: 0,
        });
        let body_exits = self.block(RegionExitMap::singleton_normal(state), body)?;
        let mut result = RegionExitMap::default();

        for (tag, states) in body_exits.0 {
            for mut exit in states {
                let frame = exit
                    .frames
                    .pop()
                    .ok_or_else(|| ErrorRegionError::InternalMissingFrame(region.into()))?;
                if frame.name != region {
                    return Err(ErrorRegionError::InternalMissingFrame(region.into()));
                }
                match &tag {
                    RegionExitTag::Normal => {
                        if frame.credit_remainder != 0 {
                            return Err(ErrorRegionError::UnusedStrictCredit {
                                region: region.into(),
                                remainder: frame.credit_remainder,
                            });
                        }
                        Self::run_actions(&mut exit, frame.actions.iter().rev())?;
                        result.push(RegionExitTag::Normal, exit);
                    }
                    RegionExitTag::Error(error) => {
                        if exit.sealed_pack.is_some() {
                            return Err(ErrorRegionError::NestedPackNotDischarged);
                        }
                        let pack = CleanupPack {
                            region: region.into(),
                            error: error.clone(),
                            baseline: frame.baseline,
                            actions: frame.actions,
                            credit_remainder: frame.credit_remainder,
                        };
                        if let Some(resource) = pack
                            .cleanup_resources()
                            .find(|resource| exit.payload.contains(resource))
                        {
                            return Err(ErrorRegionError::ResourceInPayloadAndCleanupPack(
                                resource,
                            ));
                        }
                        if let Some(clause) = clauses.get(error.as_str()) {
                            result.append(self.handle_catch(exit, pack, clause)?);
                        } else {
                            exit.sealed_pack = Some(pack);
                            result.push(tag.clone(), exit);
                        }
                    }
                }
            }
        }
        Self::validate_normal_join(&result)?;
        Ok(result)
    }

    fn handle_catch(
        &self,
        mut state: RegionExitState,
        pack: CleanupPack,
        clause: &RegionCatchClause,
    ) -> Result<RegionExitMap, ErrorRegionError> {
        match &clause.quant_open {
            None => {
                if !pack.is_empty() {
                    return Err(ErrorRegionError::CatchWithoutRequiredQuantOpen {
                        region: pack.region,
                        error: pack.error,
                    });
                }
            }
            Some(open) => {
                if open.region != pack.region {
                    return Err(ErrorRegionError::WrongRegionOpened {
                        expected: pack.region,
                        actual: open.region.clone(),
                    });
                }
                if state.consumed_bindings.contains(&open.binding) {
                    return Err(ErrorRegionError::CleanupPackDuplicated(
                        open.binding.clone(),
                    ));
                }
                state.open_capability = Some(OpenCapability {
                    binding: open.binding.clone(),
                    pack,
                });
            }
        }
        state.sealed_pack = None;
        let exits = self.block(RegionExitMap::singleton_normal(state), &clause.handler)?;
        for (tag, states) in &exits.0 {
            for state in states {
                if state.open_capability.is_some() {
                    return Err(match tag {
                        RegionExitTag::Normal => ErrorRegionError::CleanupPackNotDischarged,
                        RegionExitTag::Error(_) => ErrorRegionError::RethrowWithUndischargedCleanup,
                    });
                }
            }
        }
        Ok(exits)
    }

    fn discharge(
        &self,
        mut state: RegionExitState,
        binding: &str,
    ) -> Result<RegionExitMap, ErrorRegionError> {
        let capability = state.open_capability.take().ok_or_else(|| {
            if state.consumed_bindings.contains(binding) {
                ErrorRegionError::CleanupPackDuplicated(binding.into())
            } else {
                ErrorRegionError::UnknownCleanupCapability(binding.into())
            }
        })?;
        if capability.binding != binding {
            state.open_capability = Some(capability);
            return Err(ErrorRegionError::UnknownCleanupCapability(binding.into()));
        }
        if capability
            .pack
            .actions
            .iter()
            .any(|action| !action.is_noexcept())
        {
            return Err(ErrorRegionError::FallibleCleanup);
        }
        Self::run_actions(&mut state, capability.pack.actions.iter().rev())?;
        if capability.pack.credit_remainder != 0 {
            state.cleanup_trace.push(CleanupEvent::AbandonCredit(
                capability.pack.credit_remainder,
            ));
        }
        state.consumed_bindings.insert(binding.into());
        Ok(RegionExitMap::singleton_normal(state))
    }

    fn run_actions<'a>(
        state: &mut RegionExitState,
        actions: impl Iterator<Item = &'a CleanupAction>,
    ) -> Result<(), ErrorRegionError> {
        for action in actions {
            match action {
                CleanupAction::Destroy { resource, .. } => {
                    state.ledger.release_public(*resource)?;
                    state.cleanup_trace.push(CleanupEvent::Destroy(*resource));
                }
                CleanupAction::Rollback { name, .. } => {
                    state
                        .cleanup_trace
                        .push(CleanupEvent::Rollback(name.clone()));
                }
            }
        }
        Ok(())
    }

    fn remove_resource_action(
        frames: &mut [RegionFrame],
        resource: ResourceId,
    ) -> Result<(), ErrorRegionError> {
        for frame in frames.iter_mut().rev() {
            if let Some(index) = frame
                .actions
                .iter()
                .position(|action| action.resource() == Some(resource))
            {
                frame.actions.remove(index);
                return Ok(());
            }
        }
        Err(ErrorRegionError::ResourceHasNoCleanupAction(resource))
    }

    fn resource_is_sealed(state: &RegionExitState, resource: ResourceId) -> bool {
        state
            .open_capability
            .as_ref()
            .is_some_and(|cap| cap.pack.cleanup_resources().any(|id| id == resource))
    }

    fn require_error(&self, error: &str) -> Result<(), ErrorRegionError> {
        if !self.declarations.contains(error) {
            return Err(ErrorRegionError::UnknownErrorType(error.into()));
        }
        if !self.allowed.contains(&Effect::Throws(error.into())) {
            return Err(ErrorRegionError::ErrorNotInEffectRow(error.into()));
        }
        Ok(())
    }

    fn validate_normal_join(exits: &RegionExitMap) -> Result<(), ErrorRegionError> {
        let normal = exits.states(&RegionExitTag::Normal);
        if normal.first().is_some_and(|first| {
            normal[1..].iter().any(|state| {
                state.ledger != first.ledger
                    || state.payload != first.payload
                    || state.frames != first.frames
                    || state.open_capability != first.open_capability
            })
        }) {
            Err(ErrorRegionError::NormalOwnershipJoinMismatch)
        } else {
            Ok(())
        }
    }
}

pub fn analyze_error_regions(
    declarations: &ErrorDeclarations,
    allowed_effects: &EffectRow,
    initial_ledger: ErrorLedger,
    operations: &[RegionOp],
) -> Result<RegionExitMap, ErrorRegionError> {
    let initial = RegionExitState {
        ledger: initial_ledger,
        payload: BTreeSet::new(),
        sealed_pack: None,
        cleanup_trace: Vec::new(),
        frames: Vec::new(),
        open_capability: None,
        consumed_bindings: BTreeSet::new(),
    };
    Analyzer {
        declarations,
        allowed: allowed_effects,
    }
    .block(RegionExitMap::singleton_normal(initial), operations)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ErrorRegionError {
    UnknownErrorType(ErrorName),
    ErrorNotInEffectRow(ErrorName),
    DuplicateCatchClause(ErrorName),
    DuplicateActiveRegion(RegionName),
    ResourceAlreadyLive(ResourceId),
    ResourceNotLive(ResourceId),
    DuplicateResourceIdentity(ResourceId),
    ResourceHasNoCleanupAction(ResourceId),
    ResourceHeldByCleanupPack(ResourceId),
    PayloadResourceNotOwned(ResourceId),
    AcquisitionOutsideRegion,
    CreditOutsideRegion,
    DeferOutsideRegion,
    CreditOverflow,
    InsufficientCredit,
    UnusedStrictCredit {
        region: RegionName,
        remainder: u64,
    },
    QuantOpenOutsideCatch,
    WrongRegionOpened {
        expected: RegionName,
        actual: RegionName,
    },
    CleanupPackDuplicated(CleanupBinding),
    CleanupPackNotDischarged,
    CatchWithoutRequiredQuantOpen {
        region: RegionName,
        error: ErrorName,
    },
    RethrowWithUndischargedCleanup,
    FallibleCleanup,
    ResourceInPayloadAndCleanupPack(ResourceId),
    UnknownCleanupCapability(CleanupBinding),
    NestedPackNotDischarged,
    NormalOwnershipJoinMismatch,
    InternalMissingFrame(RegionName),
}

impl fmt::Display for ErrorRegionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownErrorType(error) => write!(f, "unknown error type `{error}`"),
            Self::ErrorNotInEffectRow(error) => {
                write!(f, "Q300: raised error `{error}` is not in the effect row")
            }
            Self::DuplicateCatchClause(error) => {
                write!(f, "error `{error}` has more than one catch clause")
            }
            Self::DuplicateActiveRegion(region) => write!(f, "region `{region}` is already active"),
            Self::ResourceAlreadyLive(id) => write!(f, "resource `{id}` is already live"),
            Self::ResourceNotLive(id) => write!(f, "resource `{id}` is not live"),
            Self::DuplicateResourceIdentity(id) => {
                write!(f, "resource `{id}` has more than one owner")
            }
            Self::ResourceHasNoCleanupAction(id) => {
                write!(f, "resource `{id}` has no region cleanup action")
            }
            Self::ResourceHeldByCleanupPack(id) => {
                write!(f, "resource `{id}` is sealed in a cleanup pack")
            }
            Self::PayloadResourceNotOwned(id) => {
                write!(f, "payload does not own resource `{id}`")
            }
            Self::AcquisitionOutsideRegion => {
                f.write_str("resource acquisition has no region home")
            }
            Self::CreditOutsideRegion => f.write_str("strict credit has no region home"),
            Self::DeferOutsideRegion => f.write_str("defer has no active error region"),
            Self::CreditOverflow => f.write_str("strict credit arithmetic overflow"),
            Self::InsufficientCredit => f.write_str("insufficient strict credit"),
            Self::UnusedStrictCredit { region, remainder } => write!(
                f,
                "normal exit from region `{region}` leaves strict credit `{remainder}`"
            ),
            Self::QuantOpenOutsideCatch => f.write_str("Q302: quant open outside matching catch"),
            Self::WrongRegionOpened { expected, actual } => write!(
                f,
                "Q303: quant open refers to `{actual}`, expected region `{expected}`"
            ),
            Self::CleanupPackDuplicated(binding) => {
                write!(
                    f,
                    "Q304: cleanup pack `{binding}` is consumed more than once"
                )
            }
            Self::CleanupPackNotDischarged => f.write_str("Q305: cleanup pack is not discharged"),
            Self::CatchWithoutRequiredQuantOpen { region, error } => write!(
                f,
                "Q306: catch for `{error}` requires quant open of region `{region}`"
            ),
            Self::RethrowWithUndischargedCleanup => {
                f.write_str("Q307: error leaves handler with undischarged cleanup")
            }
            Self::FallibleCleanup => {
                f.write_str("Q308: error cleanup action is not total and noexcept")
            }
            Self::ResourceInPayloadAndCleanupPack(id) => write!(
                f,
                "Q309: resource `{id}` occurs in both the payload and cleanup pack"
            ),
            Self::UnknownCleanupCapability(binding) => {
                write!(f, "unknown cleanup capability `{binding}`")
            }
            Self::NestedPackNotDischarged => {
                f.write_str("inner cleanup pack must be discharged before propagation")
            }
            Self::NormalOwnershipJoinMismatch => f.write_str("normal ownership states do not join"),
            Self::InternalMissingFrame(region) => {
                write!(f, "internal error: missing region frame `{region}`")
            }
        }
    }
}

impl std::error::Error for ErrorRegionError {}

trait LedgerAccess {
    fn acquire_public(&mut self, id: ResourceId) -> Result<(), ErrorRegionError>;
    fn release_public(&mut self, id: ResourceId) -> Result<(), ErrorRegionError>;
}

impl LedgerAccess for ErrorLedger {
    fn acquire_public(&mut self, id: ResourceId) -> Result<(), ErrorRegionError> {
        if self.contains(id) {
            return Err(ErrorRegionError::ResourceAlreadyLive(id));
        }
        let ids = self.ids().chain(std::iter::once(id)).collect::<Vec<_>>();
        *self = ErrorLedger::new(ids);
        Ok(())
    }

    fn release_public(&mut self, id: ResourceId) -> Result<(), ErrorRegionError> {
        if !self.contains(id) {
            return Err(ErrorRegionError::ResourceNotLive(id));
        }
        let ids = self
            .ids()
            .filter(|candidate| *candidate != id)
            .collect::<Vec<_>>();
        *self = ErrorLedger::new(ids);
        Ok(())
    }
}

#[cfg(test)]
#[path = "../tests/unit/error_region.rs"]
mod tests;
