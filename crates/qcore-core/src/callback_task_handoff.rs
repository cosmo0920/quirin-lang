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

//! Generic callback-to-task handoff and deferred-work lifetime kernel.
//!
//! The kernel does not know a source language, callback API, or native
//! observer.  Those layers must supply a complete operation sequence.  A
//! successful check establishes only the lifecycle facts represented here.

use std::collections::BTreeMap;

pub type CallbackId = u32;
pub type TaskId = u32;
pub type DeferredWorkId = u32;
pub type ArgumentId = u32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArgumentLifetime {
    Owned,
    Shared,
    BorrowedFrame,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskState {
    Spawned,
    Running,
    CancelRequested,
    Completed,
    Cancelled,
    Joined,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeferredState {
    Registered,
    Running,
    Completed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HandoffOperation {
    CallbackEnter {
        callback: CallbackId,
        argument: ArgumentId,
        argument_lifetime: ArgumentLifetime,
    },
    SpawnTask {
        callback: CallbackId,
        task: TaskId,
        retains_argument: bool,
    },
    StartTask {
        task: TaskId,
    },
    CompleteTask {
        task: TaskId,
    },
    RequestCancel {
        callback: CallbackId,
        task: TaskId,
    },
    AcknowledgeCancel {
        task: TaskId,
    },
    JoinTask {
        callback: CallbackId,
        task: TaskId,
    },
    RegisterDeferred {
        callback: CallbackId,
        work: DeferredWorkId,
        retains_argument: bool,
    },
    StartDeferred {
        work: DeferredWorkId,
    },
    CompleteDeferred {
        work: DeferredWorkId,
    },
    CallbackExit {
        callback: CallbackId,
    },
    /// Release custody represented by this handoff. For `Shared` this releases
    /// only this model's share; it does not assert that global storage is freed.
    /// A `BorrowedFrame` is caller-owned and this operation refuses it.
    DropArgument {
        argument: ArgumentId,
    },
    Teardown {
        callback: CallbackId,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HandoffDiagnostic {
    DuplicateCallback(CallbackId),
    UnknownCallback(CallbackId),
    CallbackAlreadyExited(CallbackId),
    CallbackStillRunning(CallbackId),
    DuplicateTask(TaskId),
    UnknownTask(TaskId),
    WrongTaskOwner {
        callback: CallbackId,
        task: TaskId,
    },
    InvalidTaskTransition {
        task: TaskId,
        from: TaskState,
    },
    DuplicateDeferred(DeferredWorkId),
    UnknownDeferred(DeferredWorkId),
    InvalidDeferredTransition {
        work: DeferredWorkId,
        from: DeferredState,
    },
    OutstandingTask {
        callback: CallbackId,
        task: TaskId,
    },
    OutstandingDeferred {
        callback: CallbackId,
        work: DeferredWorkId,
    },
    MissingTeardown(CallbackId),
    DuplicateTeardown(CallbackId),
    ArgumentAlreadyClaimed(ArgumentId),
    ArgumentLifetimeMismatch(ArgumentId),
    UnknownArgument(ArgumentId),
    ArgumentStillRetained(ArgumentId),
    BorrowedArgumentEscapes {
        callback: CallbackId,
        argument: ArgumentId,
    },
    BorrowedArgumentCannotDrop(ArgumentId),
    ArgumentAlreadyDropped(ArgumentId),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CallbackRecord {
    argument: ArgumentId,
    argument_lifetime: ArgumentLifetime,
    running: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ArgumentRecord {
    lifetime: ArgumentLifetime,
    live: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TaskRecord {
    owner: CallbackId,
    retains_argument: bool,
    state: TaskState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct DeferredRecord {
    owner: CallbackId,
    retains_argument: bool,
    state: DeferredState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HandoffCertificate {
    pub callbacks_torn_down: Vec<CallbackId>,
    pub tasks_joined: Vec<TaskId>,
    pub deferred_completed: Vec<DeferredWorkId>,
    pub arguments_dropped: Vec<ArgumentId>,
    /// Owned/shared argument custody that this operation sequence did not
    /// discharge.  Presence is explicit and is not a successful release.
    pub remaining_argument_custody: Vec<ArgumentId>,
}

fn callback_running(
    callbacks: &BTreeMap<CallbackId, CallbackRecord>,
    callback: CallbackId,
) -> Result<(), HandoffDiagnostic> {
    let record = callbacks
        .get(&callback)
        .ok_or(HandoffDiagnostic::UnknownCallback(callback))?;
    if !record.running {
        return Err(HandoffDiagnostic::CallbackAlreadyExited(callback));
    }
    Ok(())
}

/// Check one complete callback/task/deferred-work lifecycle.
///
/// Callback exit deliberately permits handed-off work to remain live.  The
/// callback argument cannot be dropped while such work retains it, and
/// teardown requires every task to be joined and every deferred item to have
/// completed.  This profile permits deferred work to survive the callback
/// frame but provides no cancellation operation for it.
pub fn check_callback_task_handoff(
    operations: &[HandoffOperation],
) -> Result<HandoffCertificate, HandoffDiagnostic> {
    let mut callbacks = BTreeMap::<CallbackId, CallbackRecord>::new();
    let mut arguments = BTreeMap::<ArgumentId, ArgumentRecord>::new();
    let mut tasks = BTreeMap::<TaskId, TaskRecord>::new();
    let mut deferred = BTreeMap::<DeferredWorkId, DeferredRecord>::new();
    let mut torn_down = Vec::new();
    let mut joined = Vec::new();
    let mut deferred_completed = Vec::new();
    let mut dropped = Vec::new();

    for operation in operations {
        match *operation {
            HandoffOperation::CallbackEnter {
                callback,
                argument,
                argument_lifetime,
            } => {
                match arguments.get_mut(&argument) {
                    Some(record) if !record.live => {
                        return Err(HandoffDiagnostic::ArgumentAlreadyDropped(argument));
                    }
                    Some(record) if record.lifetime != argument_lifetime => {
                        return Err(HandoffDiagnostic::ArgumentLifetimeMismatch(argument));
                    }
                    Some(_) if argument_lifetime != ArgumentLifetime::Shared => {
                        return Err(HandoffDiagnostic::ArgumentAlreadyClaimed(argument));
                    }
                    Some(_) => {}
                    None => {
                        arguments.insert(
                            argument,
                            ArgumentRecord {
                                lifetime: argument_lifetime,
                                live: true,
                            },
                        );
                    }
                }
                if callbacks
                    .insert(
                        callback,
                        CallbackRecord {
                            argument,
                            argument_lifetime,
                            running: true,
                        },
                    )
                    .is_some()
                {
                    return Err(HandoffDiagnostic::DuplicateCallback(callback));
                }
            }
            HandoffOperation::SpawnTask {
                callback,
                task,
                retains_argument,
            } => {
                callback_running(&callbacks, callback)?;
                let callback_record = callbacks.get(&callback).expect("checked callback");
                if retains_argument
                    && callback_record.argument_lifetime == ArgumentLifetime::BorrowedFrame
                {
                    return Err(HandoffDiagnostic::BorrowedArgumentEscapes {
                        callback,
                        argument: callback_record.argument,
                    });
                }
                if tasks
                    .insert(
                        task,
                        TaskRecord {
                            owner: callback,
                            retains_argument,
                            state: TaskState::Spawned,
                        },
                    )
                    .is_some()
                {
                    return Err(HandoffDiagnostic::DuplicateTask(task));
                }
            }
            HandoffOperation::StartTask { task } => {
                let record = tasks
                    .get_mut(&task)
                    .ok_or(HandoffDiagnostic::UnknownTask(task))?;
                if record.state != TaskState::Spawned {
                    return Err(HandoffDiagnostic::InvalidTaskTransition {
                        task,
                        from: record.state,
                    });
                }
                record.state = TaskState::Running;
            }
            HandoffOperation::CompleteTask { task } => {
                let record = tasks
                    .get_mut(&task)
                    .ok_or(HandoffDiagnostic::UnknownTask(task))?;
                if record.state != TaskState::Running {
                    return Err(HandoffDiagnostic::InvalidTaskTransition {
                        task,
                        from: record.state,
                    });
                }
                record.state = TaskState::Completed;
            }
            HandoffOperation::RequestCancel { callback, task } => {
                let record = tasks
                    .get_mut(&task)
                    .ok_or(HandoffDiagnostic::UnknownTask(task))?;
                if record.owner != callback {
                    return Err(HandoffDiagnostic::WrongTaskOwner { callback, task });
                }
                if !matches!(record.state, TaskState::Spawned | TaskState::Running) {
                    return Err(HandoffDiagnostic::InvalidTaskTransition {
                        task,
                        from: record.state,
                    });
                }
                record.state = TaskState::CancelRequested;
            }
            HandoffOperation::AcknowledgeCancel { task } => {
                let record = tasks
                    .get_mut(&task)
                    .ok_or(HandoffDiagnostic::UnknownTask(task))?;
                if record.state != TaskState::CancelRequested {
                    return Err(HandoffDiagnostic::InvalidTaskTransition {
                        task,
                        from: record.state,
                    });
                }
                record.state = TaskState::Cancelled;
            }
            HandoffOperation::JoinTask { callback, task } => {
                let record = tasks
                    .get_mut(&task)
                    .ok_or(HandoffDiagnostic::UnknownTask(task))?;
                if record.owner != callback {
                    return Err(HandoffDiagnostic::WrongTaskOwner { callback, task });
                }
                if !matches!(record.state, TaskState::Completed | TaskState::Cancelled) {
                    return Err(HandoffDiagnostic::InvalidTaskTransition {
                        task,
                        from: record.state,
                    });
                }
                record.state = TaskState::Joined;
                joined.push(task);
            }
            HandoffOperation::RegisterDeferred {
                callback,
                work,
                retains_argument,
            } => {
                callback_running(&callbacks, callback)?;
                let callback_record = callbacks.get(&callback).expect("checked callback");
                if retains_argument
                    && callback_record.argument_lifetime == ArgumentLifetime::BorrowedFrame
                {
                    return Err(HandoffDiagnostic::BorrowedArgumentEscapes {
                        callback,
                        argument: callback_record.argument,
                    });
                }
                if deferred
                    .insert(
                        work,
                        DeferredRecord {
                            owner: callback,
                            retains_argument,
                            state: DeferredState::Registered,
                        },
                    )
                    .is_some()
                {
                    return Err(HandoffDiagnostic::DuplicateDeferred(work));
                }
            }
            HandoffOperation::StartDeferred { work } => {
                let record = deferred
                    .get_mut(&work)
                    .ok_or(HandoffDiagnostic::UnknownDeferred(work))?;
                if record.state != DeferredState::Registered {
                    return Err(HandoffDiagnostic::InvalidDeferredTransition {
                        work,
                        from: record.state,
                    });
                }
                record.state = DeferredState::Running;
            }
            HandoffOperation::CompleteDeferred { work } => {
                let record = deferred
                    .get_mut(&work)
                    .ok_or(HandoffDiagnostic::UnknownDeferred(work))?;
                if record.state != DeferredState::Running {
                    return Err(HandoffDiagnostic::InvalidDeferredTransition {
                        work,
                        from: record.state,
                    });
                }
                record.state = DeferredState::Completed;
                deferred_completed.push(work);
            }
            HandoffOperation::CallbackExit { callback } => {
                let record = callbacks
                    .get_mut(&callback)
                    .ok_or(HandoffDiagnostic::UnknownCallback(callback))?;
                if !record.running {
                    return Err(HandoffDiagnostic::CallbackAlreadyExited(callback));
                }
                record.running = false;
            }
            HandoffOperation::DropArgument { argument } => {
                let argument_record = arguments
                    .get(&argument)
                    .ok_or(HandoffDiagnostic::UnknownArgument(argument))?;
                if !argument_record.live {
                    return Err(HandoffDiagnostic::ArgumentAlreadyDropped(argument));
                }
                if argument_record.lifetime == ArgumentLifetime::BorrowedFrame {
                    return Err(HandoffDiagnostic::BorrowedArgumentCannotDrop(argument));
                }
                if callbacks
                    .values()
                    .any(|callback| callback.argument == argument && callback.running)
                {
                    return Err(HandoffDiagnostic::ArgumentStillRetained(argument));
                }
                let task_retains = tasks.values().any(|task| {
                    task.retains_argument
                        && task.state != TaskState::Joined
                        && callbacks
                            .get(&task.owner)
                            .is_some_and(|c| c.argument == argument)
                });
                let deferred_retains = deferred.values().any(|work| {
                    work.retains_argument
                        && work.state != DeferredState::Completed
                        && callbacks
                            .get(&work.owner)
                            .is_some_and(|c| c.argument == argument)
                });
                if task_retains || deferred_retains {
                    return Err(HandoffDiagnostic::ArgumentStillRetained(argument));
                }
                arguments.get_mut(&argument).expect("known argument").live = false;
                dropped.push(argument);
            }
            HandoffOperation::Teardown { callback } => {
                if torn_down.contains(&callback) {
                    return Err(HandoffDiagnostic::DuplicateTeardown(callback));
                }
                let record = callbacks
                    .get(&callback)
                    .ok_or(HandoffDiagnostic::UnknownCallback(callback))?;
                if record.running {
                    return Err(HandoffDiagnostic::CallbackStillRunning(callback));
                }
                if let Some((&task, _)) = tasks
                    .iter()
                    .find(|(_, task)| task.owner == callback && task.state != TaskState::Joined)
                {
                    return Err(HandoffDiagnostic::OutstandingTask { callback, task });
                }
                if let Some((&work, _)) = deferred.iter().find(|(_, work)| {
                    work.owner == callback && work.state != DeferredState::Completed
                }) {
                    return Err(HandoffDiagnostic::OutstandingDeferred { callback, work });
                }
                torn_down.push(callback);
            }
        }
    }

    for (&callback, record) in &callbacks {
        if record.running {
            return Err(HandoffDiagnostic::CallbackStillRunning(callback));
        }
        if !torn_down.contains(&callback) {
            return Err(HandoffDiagnostic::MissingTeardown(callback));
        }
    }

    let remaining_argument_custody = arguments
        .into_iter()
        .filter_map(|(argument, record)| {
            (record.live && record.lifetime != ArgumentLifetime::BorrowedFrame).then_some(argument)
        })
        .collect();
    Ok(HandoffCertificate {
        callbacks_torn_down: torn_down,
        tasks_joined: joined,
        deferred_completed,
        arguments_dropped: dropped,
        remaining_argument_custody,
    })
}
