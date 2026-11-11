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

//! Cycle 17 native threaded backend and explicit platform evidence gate.
//!
//! Worker operations run on native `std::thread` threads.  The scheduler uses
//! channels only to realize a chosen checked schedule; that schedule is not
//! treated as C11 happens-before evidence (the weak-memory checker owns that
//! proof obligation).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;

pub type NativeTaskId = u32;
pub type NativeResourceId = u32;
pub type NativeCellId = u32;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NativeOperation {
    Store {
        cell: NativeCellId,
        value: i64,
    },
    Load {
        cell: NativeCellId,
        observation: u32,
    },
    TransferCustody {
        resource: NativeResourceId,
        to: NativeTaskId,
    },
    Destroy {
        resource: NativeResourceId,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeScheduleStep {
    pub task: NativeTaskId,
    pub operation: NativeOperation,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NativeProgram {
    pub tasks: BTreeSet<NativeTaskId>,
    pub cells: BTreeMap<NativeCellId, i64>,
    pub initial_custody: BTreeMap<NativeResourceId, NativeTaskId>,
    pub schedule: Vec<NativeScheduleStep>,
    pub expected_observations: BTreeMap<u32, i64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeExecutionCertificate {
    pub platform: String,
    pub observations: BTreeMap<u32, i64>,
    pub destroyed_resources: BTreeSet<NativeResourceId>,
    pub joined_tasks: BTreeSet<NativeTaskId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NativeBackendError {
    UnknownTask(NativeTaskId),
    UnknownCell(NativeCellId),
    DuplicateObservation(u32),
    WrongCustodian {
        task: NativeTaskId,
        resource: NativeResourceId,
    },
    UnknownTransferTarget(NativeTaskId),
    UndestroyedResource(NativeResourceId),
    ObservationMismatch,
    WorkerFailed,
}

impl fmt::Display for NativeBackendError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for NativeBackendError {}

enum WorkerCommand {
    Run(
        NativeOperation,
        mpsc::Sender<Result<Option<(u32, i64)>, NativeBackendError>>,
    ),
    Stop,
}

/// Execute a selected schedule on native worker threads and check observation,
/// custody, exactly-once destruction, and join closure.
pub fn run_native_backend(
    program: &NativeProgram,
) -> Result<NativeExecutionCertificate, NativeBackendError> {
    for step in &program.schedule {
        if !program.tasks.contains(&step.task) {
            return Err(NativeBackendError::UnknownTask(step.task));
        }
        match step.operation {
            NativeOperation::Store { cell, .. } | NativeOperation::Load { cell, .. }
                if !program.cells.contains_key(&cell) =>
            {
                return Err(NativeBackendError::UnknownCell(cell));
            }
            NativeOperation::TransferCustody { to, .. } if !program.tasks.contains(&to) => {
                return Err(NativeBackendError::UnknownTransferTarget(to));
            }
            _ => {}
        }
    }

    let cells = Arc::new(
        program
            .cells
            .iter()
            .map(|(&id, &value)| (id, AtomicI64::new(value)))
            .collect::<BTreeMap<_, _>>(),
    );
    let custody = Arc::new(Mutex::new(program.initial_custody.clone()));
    let destroyed = Arc::new(Mutex::new(BTreeSet::new()));
    let mut senders = BTreeMap::new();
    let mut handles = Vec::new();
    for &task in &program.tasks {
        let (sender, receiver) = mpsc::channel();
        senders.insert(task, sender);
        let cells = Arc::clone(&cells);
        let custody = Arc::clone(&custody);
        let destroyed = Arc::clone(&destroyed);
        handles.push((
            task,
            thread::spawn(move || {
                while let Ok(command) = receiver.recv() {
                    match command {
                        WorkerCommand::Run(operation, reply) => {
                            let result =
                                execute_operation(task, &operation, &cells, &custody, &destroyed);
                            let _ = reply.send(result);
                        }
                        WorkerCommand::Stop => break,
                    }
                }
            }),
        ));
    }

    let mut observations = BTreeMap::new();
    let mut failure = None;
    for step in &program.schedule {
        let (reply_sender, reply_receiver) = mpsc::channel();
        if senders[&step.task]
            .send(WorkerCommand::Run(step.operation.clone(), reply_sender))
            .is_err()
        {
            failure = Some(NativeBackendError::WorkerFailed);
            break;
        }
        match reply_receiver.recv() {
            Ok(Ok(Some((label, value)))) => {
                if observations.insert(label, value).is_some() {
                    failure = Some(NativeBackendError::DuplicateObservation(label));
                    break;
                }
            }
            Ok(Ok(None)) => {}
            Ok(Err(error)) => {
                failure = Some(error);
                break;
            }
            Err(_) => {
                failure = Some(NativeBackendError::WorkerFailed);
                break;
            }
        }
    }
    for sender in senders.values() {
        let _ = sender.send(WorkerCommand::Stop);
    }
    let mut joined_tasks = BTreeSet::new();
    for (task, handle) in handles {
        if handle.join().is_err() {
            failure.get_or_insert(NativeBackendError::WorkerFailed);
        } else {
            joined_tasks.insert(task);
        }
    }
    if let Some(error) = failure {
        return Err(error);
    }
    let destroyed_resources = destroyed
        .lock()
        .map_err(|_| NativeBackendError::WorkerFailed)?
        .clone();
    for resource in program.initial_custody.keys() {
        if !destroyed_resources.contains(resource) {
            return Err(NativeBackendError::UndestroyedResource(*resource));
        }
    }
    if observations != program.expected_observations {
        return Err(NativeBackendError::ObservationMismatch);
    }
    Ok(NativeExecutionCertificate {
        platform: std::env::consts::OS.to_owned(),
        observations,
        destroyed_resources,
        joined_tasks,
    })
}

fn execute_operation(
    task: NativeTaskId,
    operation: &NativeOperation,
    cells: &BTreeMap<NativeCellId, AtomicI64>,
    custody: &Mutex<BTreeMap<NativeResourceId, NativeTaskId>>,
    destroyed: &Mutex<BTreeSet<NativeResourceId>>,
) -> Result<Option<(u32, i64)>, NativeBackendError> {
    match *operation {
        NativeOperation::Store { cell, value } => {
            cells[&cell].store(value, Ordering::SeqCst);
            Ok(None)
        }
        NativeOperation::Load { cell, observation } => {
            Ok(Some((observation, cells[&cell].load(Ordering::SeqCst))))
        }
        NativeOperation::TransferCustody { resource, to } => {
            let mut ledger = custody
                .lock()
                .map_err(|_| NativeBackendError::WorkerFailed)?;
            if ledger.get(&resource) != Some(&task) {
                return Err(NativeBackendError::WrongCustodian { task, resource });
            }
            ledger.insert(resource, to);
            Ok(None)
        }
        NativeOperation::Destroy { resource } => {
            let ledger = custody
                .lock()
                .map_err(|_| NativeBackendError::WorkerFailed)?;
            let mut completed = destroyed
                .lock()
                .map_err(|_| NativeBackendError::WorkerFailed)?;
            if ledger.get(&resource) != Some(&task) || !completed.insert(resource) {
                return Err(NativeBackendError::WrongCustodian { task, resource });
            }
            Ok(None)
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RequiredPlatform {
    Linux,
    Windows,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlatformRun {
    pub platform: RequiredPlatform,
    pub passed: bool,
    pub sanitizer: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlatformMatrixCertificate {
    pub linux: PlatformRun,
    pub windows: PlatformRun,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlatformMatrixError {
    Missing(RequiredPlatform),
    Failed(RequiredPlatform),
    SanitizerMissing(RequiredPlatform),
}

pub fn check_platform_matrix(
    runs: &[PlatformRun],
) -> Result<PlatformMatrixCertificate, PlatformMatrixError> {
    fn checked(
        runs: &[PlatformRun],
        platform: RequiredPlatform,
    ) -> Result<PlatformRun, PlatformMatrixError> {
        let run = runs
            .iter()
            .find(|run| run.platform == platform)
            .cloned()
            .ok_or(PlatformMatrixError::Missing(platform))?;
        if !run.passed {
            return Err(PlatformMatrixError::Failed(platform));
        }
        if !run.sanitizer {
            return Err(PlatformMatrixError::SanitizerMissing(platform));
        }
        Ok(run)
    }
    Ok(PlatformMatrixCertificate {
        linux: checked(runs, RequiredPlatform::Linux)?,
        windows: checked(runs, RequiredPlatform::Windows)?,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PromotedConcurrencyFeature {
    DynamicTaskCollections,
    ConditionVariables,
    OwnershipQueues,
    DynamicMutexInvariants,
    AtomicPublication,
    Reclamation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FeatureDisposition {
    Accepted,
    Unknown,
}

pub const PROMOTED_FEATURE_PROFILE: &[(PromotedConcurrencyFeature, FeatureDisposition)] = &[
    (
        PromotedConcurrencyFeature::DynamicTaskCollections,
        FeatureDisposition::Unknown,
    ),
    (
        PromotedConcurrencyFeature::ConditionVariables,
        FeatureDisposition::Unknown,
    ),
    (
        PromotedConcurrencyFeature::OwnershipQueues,
        FeatureDisposition::Unknown,
    ),
    (
        PromotedConcurrencyFeature::DynamicMutexInvariants,
        FeatureDisposition::Unknown,
    ),
    (
        PromotedConcurrencyFeature::AtomicPublication,
        FeatureDisposition::Accepted,
    ),
    (
        PromotedConcurrencyFeature::Reclamation,
        FeatureDisposition::Unknown,
    ),
];

#[cfg(test)]
#[path = "../tests/unit/native_concurrency.rs"]
mod tests;
