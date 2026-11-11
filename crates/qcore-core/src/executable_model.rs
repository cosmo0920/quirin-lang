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

//! Reduced executable model used by the Cycle 8 metatheory.
//!
//! This module deliberately models one straight-line path and one error
//! region.  It is small enough to correspond directly to the Lean small-step
//! machine while still integrating natural indices, usage grades, owned
//! resource identities, typed errors, cleanup packs, and observable releases.

use std::collections::BTreeSet;
use std::fmt;

pub type NaturalIndex = u64;
pub type Variable = u32;
pub type ResourceIdentity = u64;
pub type ErrorIdentity = u32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UsageGrade {
    Erased,
    ExactlyOne,
    Affine,
    Many,
}

impl UsageGrade {
    #[must_use]
    pub const fn accepts(self, demand: usize) -> bool {
        match self {
            Self::Erased => demand == 0,
            Self::ExactlyOne => demand == 1,
            Self::Affine => demand <= 1,
            Self::Many => true,
        }
    }

    const fn upper_exceeded(self, demand: usize) -> bool {
        match self {
            Self::Erased => demand > 0,
            Self::ExactlyOne | Self::Affine => demand > 1,
            Self::Many => false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UsageBinding {
    pub variable: Variable,
    pub grade: UsageGrade,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Operation {
    Use(Variable),
    Acquire(ResourceIdentity),
    Drop(ResourceIdentity),
    Raise(ErrorIdentity),
    Return(NaturalIndex),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelProgram {
    pub usage: Vec<UsageBinding>,
    pub declared_errors: Vec<ErrorIdentity>,
    pub body: Vec<Operation>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelBuildError {
    DuplicateBaseline(ResourceIdentity),
    DuplicateUsageBinding(Variable),
    DuplicateErrorDeclaration(ErrorIdentity),
}

impl fmt::Display for ModelBuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateBaseline(resource) => {
                write!(f, "duplicate baseline resource identity {resource}")
            }
            Self::DuplicateUsageBinding(variable) => {
                write!(f, "duplicate usage binding for variable {variable}")
            }
            Self::DuplicateErrorDeclaration(error) => {
                write!(f, "duplicate error declaration {error}")
            }
        }
    }
}

impl std::error::Error for ModelBuildError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelTypedError {
    Raised(ErrorIdentity),
    UndeclaredError(ErrorIdentity),
    DuplicateAcquire(ResourceIdentity),
    InvalidDrop(ResourceIdentity),
    UnknownUsageVariable(Variable),
    UsageUpperExceeded {
        variable: Variable,
        grade: UsageGrade,
        actual: usize,
    },
    UsageObligationUnsatisfied {
        variable: Variable,
        grade: UsageGrade,
        actual: usize,
    },
    LiveResourcesAtReturn,
    MissingTerminator,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelCleanupPack {
    pub baseline: Vec<ResourceIdentity>,
    /// Live region-local resources in release order (newest first).
    pub actions: Vec<ResourceIdentity>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelOutcome {
    Returned(NaturalIndex),
    Error {
        error: ModelTypedError,
        pack: ModelCleanupPack,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelTraceEvent {
    Use(Variable),
    Acquire(ResourceIdentity),
    Release(ResourceIdentity),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelControl {
    Running {
        remaining: Vec<Operation>,
    },
    Cleaning {
        error: ModelTypedError,
        pack: ModelCleanupPack,
    },
    Done(ModelOutcome),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelState {
    pub control: ModelControl,
    pub baseline: Vec<ResourceIdentity>,
    /// Region-local resource stack, newest first.
    pub locals: Vec<ResourceIdentity>,
    /// Released identities, most recent first.
    pub released: Vec<ResourceIdentity>,
    pub trace: Vec<ModelTraceEvent>,
}

impl ModelState {
    #[must_use]
    pub fn is_final(&self) -> bool {
        matches!(self.control, ModelControl::Done(_))
    }

    /// The Cycle 8 preservation invariant: the live locals, released history,
    /// and untouched baseline form a duplicate-free identity partition.
    #[must_use]
    pub fn is_well_formed(&self) -> bool {
        let mut identities = BTreeSet::new();
        self.locals
            .iter()
            .chain(&self.released)
            .chain(&self.baseline)
            .all(|identity| identities.insert(*identity))
    }

    #[must_use]
    pub fn outcome(&self) -> Option<&ModelOutcome> {
        match &self.control {
            ModelControl::Done(outcome) => Some(outcome),
            ModelControl::Running { .. } | ModelControl::Cleaning { .. } => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ExecutableModel {
    program: ModelProgram,
    state: ModelState,
}

impl ExecutableModel {
    pub fn new(
        program: ModelProgram,
        baseline: Vec<ResourceIdentity>,
    ) -> Result<Self, ModelBuildError> {
        reject_duplicate(baseline.iter().copied(), ModelBuildError::DuplicateBaseline)?;
        reject_duplicate(
            program.usage.iter().map(|binding| binding.variable),
            ModelBuildError::DuplicateUsageBinding,
        )?;
        reject_duplicate(
            program.declared_errors.iter().copied(),
            ModelBuildError::DuplicateErrorDeclaration,
        )?;

        let remaining = program.body.clone();
        Ok(Self {
            program,
            state: ModelState {
                control: ModelControl::Running { remaining },
                baseline,
                locals: Vec::new(),
                released: Vec::new(),
                trace: Vec::new(),
            },
        })
    }

    #[must_use]
    pub const fn state(&self) -> &ModelState {
        &self.state
    }

    /// Performs one small step. Returns `false` exactly for final states.
    pub fn step(&mut self) -> bool {
        let control = self.state.control.clone();
        match control {
            ModelControl::Done(_) => false,
            ModelControl::Cleaning { error, pack } => {
                if let Some(resource) = self.state.locals.first().copied() {
                    self.state.locals.remove(0);
                    self.state.released.insert(0, resource);
                    self.state.trace.push(ModelTraceEvent::Release(resource));
                } else {
                    self.state.control = ModelControl::Done(ModelOutcome::Error { error, pack });
                }
                true
            }
            ModelControl::Running { mut remaining } => {
                if remaining.is_empty() {
                    self.begin_cleanup(ModelTypedError::MissingTerminator);
                    return true;
                }

                let operation = remaining.remove(0);
                self.state.control = ModelControl::Running { remaining };
                match operation {
                    Operation::Use(variable) => self.use_variable(variable),
                    Operation::Acquire(resource) => self.acquire(resource),
                    Operation::Drop(resource) => self.drop_resource(resource),
                    Operation::Raise(error) => {
                        let typed_error = if self.program.declared_errors.contains(&error) {
                            ModelTypedError::Raised(error)
                        } else {
                            ModelTypedError::UndeclaredError(error)
                        };
                        self.begin_cleanup(typed_error);
                    }
                    Operation::Return(value) => self.return_value(value),
                }
                true
            }
        }
    }

    pub fn run(&mut self) -> &ModelOutcome {
        while self.step() {
            debug_assert!(self.state.is_well_formed());
        }
        self.state
            .outcome()
            .expect("a machine that cannot step must be final")
    }

    fn use_variable(&mut self, variable: Variable) {
        let Some(binding) = self
            .program
            .usage
            .iter()
            .find(|binding| binding.variable == variable)
            .copied()
        else {
            self.begin_cleanup(ModelTypedError::UnknownUsageVariable(variable));
            return;
        };

        self.state.trace.push(ModelTraceEvent::Use(variable));
        let actual = self.usage_count(variable);
        if binding.grade.upper_exceeded(actual) {
            self.begin_cleanup(ModelTypedError::UsageUpperExceeded {
                variable,
                grade: binding.grade,
                actual,
            });
        }
    }

    fn acquire(&mut self, resource: ResourceIdentity) {
        let duplicate = self
            .state
            .locals
            .iter()
            .chain(&self.state.released)
            .chain(&self.state.baseline)
            .any(|existing| *existing == resource);
        if duplicate {
            self.begin_cleanup(ModelTypedError::DuplicateAcquire(resource));
            return;
        }
        self.state.locals.insert(0, resource);
        self.state.trace.push(ModelTraceEvent::Acquire(resource));
    }

    fn drop_resource(&mut self, resource: ResourceIdentity) {
        if self.state.locals.first() != Some(&resource) {
            self.begin_cleanup(ModelTypedError::InvalidDrop(resource));
            return;
        }
        self.state.locals.remove(0);
        self.state.released.insert(0, resource);
        self.state.trace.push(ModelTraceEvent::Release(resource));
    }

    fn return_value(&mut self, value: NaturalIndex) {
        if !self.state.locals.is_empty() {
            self.begin_cleanup(ModelTypedError::LiveResourcesAtReturn);
            return;
        }
        for binding in &self.program.usage {
            let actual = self.usage_count(binding.variable);
            if !binding.grade.accepts(actual) {
                self.begin_cleanup(ModelTypedError::UsageObligationUnsatisfied {
                    variable: binding.variable,
                    grade: binding.grade,
                    actual,
                });
                return;
            }
        }
        self.state.control = ModelControl::Done(ModelOutcome::Returned(value));
    }

    fn begin_cleanup(&mut self, error: ModelTypedError) {
        let pack = ModelCleanupPack {
            baseline: self.state.baseline.clone(),
            actions: self.state.locals.clone(),
        };
        self.state.control = ModelControl::Cleaning { error, pack };
    }

    fn usage_count(&self, variable: Variable) -> usize {
        self.state
            .trace
            .iter()
            .filter(|event| **event == ModelTraceEvent::Use(variable))
            .count()
    }
}

fn reject_duplicate<T: Ord + Copy, E>(
    values: impl IntoIterator<Item = T>,
    error: impl Fn(T) -> E,
) -> Result<(), E> {
    let mut seen = BTreeSet::new();
    for value in values {
        if !seen.insert(value) {
            return Err(error(value));
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "../tests/unit/executable_model.rs"]
mod tests;
