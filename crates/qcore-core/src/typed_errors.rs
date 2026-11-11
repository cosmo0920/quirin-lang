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

//! Path-sensitive typed-error analysis for the cycle-5 core IR.
//!
//! This module deliberately models error edges separately from cleanup packs.
//! A caught edge is therefore legal only when it has exactly the region-entry
//! ledger; cycle 6 will add the linear capability that can discharge a delta.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::ResourceId;

pub type ErrorName = String;

/// Qualitative effects. Quantitative resource bounds remain in `resource_flow`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Effect {
    Io,
    Alloc,
    Mut,
    Unsafe,
    Throws(ErrorName),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EffectRow(BTreeSet<Effect>);

impl EffectRow {
    pub fn new(effects: impl IntoIterator<Item = Effect>) -> Self {
        Self(effects.into_iter().collect())
    }

    pub fn contains(&self, effect: &Effect) -> bool {
        self.0.contains(effect)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Effect> {
        self.0.iter()
    }

    /// True when every effect in this row is admitted by `other`.
    pub fn is_subset_of(&self, other: &Self) -> bool {
        self.0.is_subset(&other.0)
    }

    fn insert(&mut self, effect: Effect) {
        self.0.insert(effect);
    }

    fn remove(&mut self, effect: &Effect) {
        self.0.remove(effect);
    }

    fn extend(&mut self, other: &Self) {
        self.0.extend(other.0.iter().cloned());
    }
}

/// The identities live at one control-flow exit.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct ErrorLedger(BTreeSet<ResourceId>);

impl ErrorLedger {
    pub fn new(ids: impl IntoIterator<Item = ResourceId>) -> Self {
        Self(ids.into_iter().collect())
    }

    pub fn contains(&self, id: ResourceId) -> bool {
        self.0.contains(&id)
    }

    pub fn ids(&self) -> impl Iterator<Item = ResourceId> + '_ {
        self.0.iter().copied()
    }

    fn acquire(&mut self, id: ResourceId) -> Result<(), TypedErrorError> {
        if !self.0.insert(id) {
            return Err(TypedErrorError::ResourceAlreadyLive(id));
        }
        Ok(())
    }

    fn release(&mut self, id: ResourceId) -> Result<(), TypedErrorError> {
        if !self.0.remove(&id) {
            return Err(TypedErrorError::ResourceNotLive(id));
        }
        Ok(())
    }
}

/// Exact ownership change declared by one call exit.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LedgerTransition {
    released: BTreeSet<ResourceId>,
    acquired: BTreeSet<ResourceId>,
}

impl LedgerTransition {
    pub fn identity() -> Self {
        Self::default()
    }

    pub fn new(
        released: impl IntoIterator<Item = ResourceId>,
        acquired: impl IntoIterator<Item = ResourceId>,
    ) -> Result<Self, TypedErrorError> {
        let released = released.into_iter().collect::<BTreeSet<_>>();
        let acquired = acquired.into_iter().collect::<BTreeSet<_>>();
        if let Some(id) = released.intersection(&acquired).next() {
            return Err(TypedErrorError::InvalidTransition(*id));
        }
        Ok(Self { released, acquired })
    }

    fn apply(&self, ledger: &ErrorLedger) -> Result<ErrorLedger, TypedErrorError> {
        let mut result = ledger.clone();
        for id in &self.released {
            result.release(*id)?;
        }
        for id in &self.acquired {
            result.acquire(*id)?;
        }
        Ok(result)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CallExit {
    transition: LedgerTransition,
    effects: EffectRow,
}

impl CallExit {
    pub fn new(transition: LedgerTransition, effects: EffectRow) -> Self {
        Self {
            transition,
            effects,
        }
    }
}

/// Path-sensitive summary consumed by the `?` operation.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CallSummary {
    normal: Vec<CallExit>,
    errors: BTreeMap<ErrorName, Vec<CallExit>>,
}

impl CallSummary {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_normal(mut self, exit: CallExit) -> Self {
        self.normal.push(exit);
        self
    }

    pub fn add_error(mut self, error: impl Into<ErrorName>, exit: CallExit) -> Self {
        self.errors.entry(error.into()).or_default().push(exit);
        self
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ErrorDeclarations(BTreeSet<ErrorName>);

impl ErrorDeclarations {
    pub fn new(errors: impl IntoIterator<Item = ErrorName>) -> Self {
        Self(errors.into_iter().collect())
    }

    pub fn declare(&mut self, error: impl Into<ErrorName>) -> Result<(), TypedErrorError> {
        let error = error.into();
        if !self.0.insert(error.clone()) {
            return Err(TypedErrorError::DuplicateErrorDeclaration(error));
        }
        Ok(())
    }

    pub fn contains(&self, error: &str) -> bool {
        self.0.contains(error)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExitTag {
    Normal,
    Error(ErrorName),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExitState {
    ledger: ErrorLedger,
    effects: EffectRow,
}

impl ExitState {
    pub fn ledger(&self) -> &ErrorLedger {
        &self.ledger
    }

    pub fn effects(&self) -> &EffectRow {
        &self.effects
    }
}

/// Multiple states under one tag are intentionally retained: they represent
/// distinct path conditions even when this reduced IR does not store formulas.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ExitMap(BTreeMap<ExitTag, Vec<ExitState>>);

impl ExitMap {
    pub fn states(&self, tag: &ExitTag) -> &[ExitState] {
        self.0.get(tag).map(Vec::as_slice).unwrap_or(&[])
    }

    pub fn iter(&self) -> impl Iterator<Item = (&ExitTag, &[ExitState])> {
        self.0.iter().map(|(tag, states)| (tag, states.as_slice()))
    }

    pub fn inferred_effects(&self) -> EffectRow {
        let mut result = EffectRow::default();
        for (tag, states) in &self.0 {
            for state in states {
                result.extend(&state.effects);
            }
            if let ExitTag::Error(error) = tag {
                result.insert(Effect::Throws(error.clone()));
            }
        }
        result
    }

    fn singleton_normal(state: ExitState) -> Self {
        let mut exits = Self::default();
        exits.push(ExitTag::Normal, state);
        exits
    }

    fn push(&mut self, tag: ExitTag, state: ExitState) {
        self.0.entry(tag).or_default().push(state);
    }

    fn append(&mut self, other: Self) {
        for (tag, states) in other.0 {
            self.0.entry(tag).or_default().extend(states);
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatchClause {
    pub error: ErrorName,
    pub handler: Vec<ErrorOp>,
}

/// Reduced, cleanup-free error IR for cycle 5.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ErrorOp {
    Acquire(ResourceId),
    Drop(ResourceId),
    Perform(Effect),
    Raise(ErrorName),
    Try(CallSummary),
    Branch {
        then_ops: Vec<ErrorOp>,
        else_ops: Vec<ErrorOp>,
    },
    Catch {
        body: Vec<ErrorOp>,
        clauses: Vec<CatchClause>,
    },
}

struct Analyzer<'a> {
    declarations: &'a ErrorDeclarations,
    allowed: &'a EffectRow,
}

impl Analyzer<'_> {
    fn block(
        &self,
        mut exits: ExitMap,
        operations: &[ErrorOp],
    ) -> Result<ExitMap, TypedErrorError> {
        for operation in operations {
            let mut next = ExitMap::default();
            for (tag, states) in exits.0 {
                if tag == ExitTag::Normal {
                    for state in states {
                        next.append(self.operation(state, operation)?);
                    }
                } else {
                    next.0.entry(tag.clone()).or_default().extend(states);
                }
            }
            exits = next;
        }
        Ok(exits)
    }

    fn operation(
        &self,
        mut state: ExitState,
        operation: &ErrorOp,
    ) -> Result<ExitMap, TypedErrorError> {
        match operation {
            ErrorOp::Acquire(id) => {
                state.ledger.acquire(*id)?;
                state.effects.insert(Effect::Alloc);
                Ok(ExitMap::singleton_normal(state))
            }
            ErrorOp::Drop(id) => {
                state.ledger.release(*id)?;
                Ok(ExitMap::singleton_normal(state))
            }
            ErrorOp::Perform(effect) => {
                if matches!(effect, Effect::Throws(_)) {
                    return Err(TypedErrorError::ThrowsMustUseRaiseOrTry);
                }
                self.require_allowed_effect(effect)?;
                state.effects.insert(effect.clone());
                Ok(ExitMap::singleton_normal(state))
            }
            ErrorOp::Raise(error) => self.raise(state, error),
            ErrorOp::Try(summary) => self.propagate(state, summary),
            ErrorOp::Branch { then_ops, else_ops } => {
                let left = self.block(ExitMap::singleton_normal(state.clone()), then_ops)?;
                let right = self.block(ExitMap::singleton_normal(state), else_ops)?;
                let mut joined = left;
                joined.append(right);
                Self::validate_normal_join(&joined, TypedErrorError::BranchNormalLedgerMismatch)?;
                Ok(joined)
            }
            ErrorOp::Catch { body, clauses } => self.catch(state, body, clauses),
        }
    }

    fn raise(&self, mut state: ExitState, error: &str) -> Result<ExitMap, TypedErrorError> {
        self.require_declared(error)?;
        let effect = Effect::Throws(error.into());
        if !self.allowed.contains(&effect) {
            return Err(TypedErrorError::RaisedErrorNotInEffectRow(error.into()));
        }
        state.effects.insert(effect);
        let mut exits = ExitMap::default();
        exits.push(ExitTag::Error(error.into()), state);
        Ok(exits)
    }

    fn propagate(
        &self,
        state: ExitState,
        summary: &CallSummary,
    ) -> Result<ExitMap, TypedErrorError> {
        let mut exits = ExitMap::default();
        for normal in &summary.normal {
            self.require_allowed_effects(&normal.effects)?;
            let mut next = state.clone();
            next.ledger = normal.transition.apply(&next.ledger)?;
            next.effects.extend(&normal.effects);
            exits.push(ExitTag::Normal, next);
        }
        for (error, error_exits) in &summary.errors {
            self.require_declared(error)?;
            let throws = Effect::Throws(error.clone());
            if !self.allowed.contains(&throws) {
                return Err(TypedErrorError::UnhandledPropagatedError(error.clone()));
            }
            for error_exit in error_exits {
                self.require_allowed_effects(&error_exit.effects)?;
                let mut next = state.clone();
                next.ledger = error_exit.transition.apply(&next.ledger)?;
                next.effects.extend(&error_exit.effects);
                next.effects.insert(throws.clone());
                exits.push(ExitTag::Error(error.clone()), next);
            }
        }
        Ok(exits)
    }

    fn catch(
        &self,
        state: ExitState,
        body: &[ErrorOp],
        clauses: &[CatchClause],
    ) -> Result<ExitMap, TypedErrorError> {
        let baseline = state.ledger.clone();
        let mut by_error = BTreeMap::<&str, &CatchClause>::new();
        for clause in clauses {
            self.require_declared(&clause.error)?;
            if by_error.insert(&clause.error, clause).is_some() {
                return Err(TypedErrorError::DuplicateCatchClause(clause.error.clone()));
            }
        }

        let body_exits = self.block(ExitMap::singleton_normal(state), body)?;
        let mut result = ExitMap::default();
        for (tag, states) in body_exits.0 {
            match tag {
                ExitTag::Error(error) if by_error.contains_key(error.as_str()) => {
                    let clause = by_error[error.as_str()];
                    for mut caught in states {
                        if caught.ledger != baseline {
                            return Err(TypedErrorError::CatchHasLocalObligations {
                                error,
                                baseline,
                                actual: caught.ledger,
                            });
                        }
                        caught.effects.remove(&Effect::Throws(clause.error.clone()));
                        result.append(
                            self.block(ExitMap::singleton_normal(caught), &clause.handler)?,
                        );
                    }
                }
                other => result.0.entry(other).or_default().extend(states),
            }
        }
        Self::validate_normal_join(&result, TypedErrorError::CatchNormalLedgerMismatch)?;
        Ok(result)
    }

    fn require_declared(&self, error: &str) -> Result<(), TypedErrorError> {
        if self.declarations.contains(error) {
            Ok(())
        } else {
            Err(TypedErrorError::UnknownErrorType(error.into()))
        }
    }

    fn require_allowed_effect(&self, effect: &Effect) -> Result<(), TypedErrorError> {
        if self.allowed.contains(effect) {
            Ok(())
        } else {
            Err(TypedErrorError::EffectNotInRow(effect.clone()))
        }
    }

    fn require_allowed_effects(&self, effects: &EffectRow) -> Result<(), TypedErrorError> {
        for effect in effects.iter() {
            self.require_allowed_effect(effect)?;
        }
        Ok(())
    }

    fn validate_normal_join(
        exits: &ExitMap,
        mismatch: TypedErrorError,
    ) -> Result<(), TypedErrorError> {
        let normal = exits.states(&ExitTag::Normal);
        if normal
            .first()
            .is_some_and(|first| normal[1..].iter().any(|state| state.ledger != first.ledger))
        {
            Err(mismatch)
        } else {
            Ok(())
        }
    }
}

/// Builds all normal and typed-error exits. Remaining error exits are the
/// function's declared `throws<...>` result and are not considered leaks.
pub fn analyze_typed_errors(
    declarations: &ErrorDeclarations,
    allowed_effects: &EffectRow,
    initial_ledger: ErrorLedger,
    operations: &[ErrorOp],
) -> Result<ExitMap, TypedErrorError> {
    let initial = ExitState {
        ledger: initial_ledger,
        effects: EffectRow::default(),
    };
    Analyzer {
        declarations,
        allowed: allowed_effects,
    }
    .block(ExitMap::singleton_normal(initial), operations)
}

/// Checks a boundary that promises not to return a recoverable error.
pub fn check_no_unhandled_errors(
    declarations: &ErrorDeclarations,
    allowed_effects: &EffectRow,
    initial_ledger: ErrorLedger,
    operations: &[ErrorOp],
) -> Result<ExitMap, TypedErrorError> {
    let exits = analyze_typed_errors(declarations, allowed_effects, initial_ledger, operations)?;
    if let Some(error) = exits.0.keys().find_map(|tag| match tag {
        ExitTag::Error(error) => Some(error.clone()),
        ExitTag::Normal => None,
    }) {
        Err(TypedErrorError::UnhandledError(error))
    } else {
        Ok(exits)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypedErrorError {
    DuplicateErrorDeclaration(ErrorName),
    UnknownErrorType(ErrorName),
    RaisedErrorNotInEffectRow(ErrorName),
    UnhandledPropagatedError(ErrorName),
    UnhandledError(ErrorName),
    DuplicateCatchClause(ErrorName),
    ResourceAlreadyLive(ResourceId),
    ResourceNotLive(ResourceId),
    InvalidTransition(ResourceId),
    ThrowsMustUseRaiseOrTry,
    EffectNotInRow(Effect),
    BranchNormalLedgerMismatch,
    CatchNormalLedgerMismatch,
    CatchHasLocalObligations {
        error: ErrorName,
        baseline: ErrorLedger,
        actual: ErrorLedger,
    },
}

impl fmt::Display for TypedErrorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateErrorDeclaration(error) => {
                write!(f, "error type `{error}` is declared more than once")
            }
            Self::UnknownErrorType(error) => write!(f, "unknown error type `{error}`"),
            Self::RaisedErrorNotInEffectRow(error) => {
                write!(f, "Q300: raised error `{error}` is not in the effect row")
            }
            Self::UnhandledPropagatedError(error) | Self::UnhandledError(error) => {
                write!(f, "Q301: unhandled typed error `{error}`")
            }
            Self::DuplicateCatchClause(error) => {
                write!(f, "error `{error}` has more than one catch clause")
            }
            Self::ResourceAlreadyLive(id) => write!(f, "resource `{id}` is already live"),
            Self::ResourceNotLive(id) => write!(f, "resource `{id}` is not live"),
            Self::InvalidTransition(id) => {
                write!(
                    f,
                    "call transition both releases and acquires resource `{id}`"
                )
            }
            Self::ThrowsMustUseRaiseOrTry => {
                f.write_str("throws effects must be introduced by raise or ?")
            }
            Self::EffectNotInRow(effect) => {
                write!(f, "effect `{effect:?}` is not in the declared effect row")
            }
            Self::BranchNormalLedgerMismatch => {
                f.write_str("branch normal ownership states do not join")
            }
            Self::CatchNormalLedgerMismatch => {
                f.write_str("catch and body normal ownership states do not join")
            }
            Self::CatchHasLocalObligations { error, .. } => write!(
                f,
                "Q306: catch for `{error}` requires zero region-local obligations"
            ),
        }
    }
}

impl std::error::Error for TypedErrorError {}

#[cfg(test)]
#[path = "../tests/unit/typed_errors.rs"]
mod tests;
