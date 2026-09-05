//! Cycle 15 advanced-control fragments.
//!
//! Cancellation is observed only at explicit safe points.  Fallible cleanup
//! records every secondary failure while preserving an existing error or
//! cancellation as the primary outcome.  Resumable and multi-shot handlers
//! may not capture owned resources in this restricted model.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub type AdvancedResourceId = u32;
pub type DestructorId = u32;
pub type HandlerId = u32;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DestructorOutcome {
    Ok,
    Error(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DestructorSpec {
    pub id: DestructorId,
    pub outcome: DestructorOutcome,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HandlerKind {
    OneShot,
    Resumable,
    MultiShot,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaptureMode {
    Owned,
    Shared,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Capture {
    pub resource: AdvancedResourceId,
    pub mode: CaptureMode,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HandlerSpec {
    pub id: HandlerId,
    pub kind: HandlerKind,
    pub captures: Vec<Capture>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AdvancedOp {
    Acquire {
        resource: AdvancedResourceId,
        destructor: DestructorId,
    },
    Drop(AdvancedResourceId),
    RequestCancellation,
    SafePoint,
    Invoke(HandlerId),
    Raise(String),
    Return(i32),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AdvancedProgram {
    pub destructors: Vec<DestructorSpec>,
    pub handlers: Vec<HandlerSpec>,
    pub operations: Vec<AdvancedOp>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PrimaryFailure {
    Error(String),
    Cancelled,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AdvancedOutcome {
    Ok(i32),
    Error(String),
    Cancelled,
    CleanupFailure(Vec<String>),
    DoubleError {
        primary: PrimaryFailure,
        cleanup: Vec<String>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AdvancedTraceEvent {
    Acquire(AdvancedResourceId),
    Destroy(AdvancedResourceId),
    DestructorFailed {
        resource: AdvancedResourceId,
        error: String,
    },
    CancellationRequested,
    CancellationDelivered,
    HandlerInvoked(HandlerId),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdvancedExecution {
    pub outcome: AdvancedOutcome,
    pub trace: Vec<AdvancedTraceEvent>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AdvancedControlError {
    DuplicateDestructor(DestructorId),
    DuplicateHandler(HandlerId),
    DuplicateCapture(AdvancedResourceId),
    OwnedCaptureRequiresOneShot(HandlerId),
    UnknownDestructor(DestructorId),
    UnknownHandler(HandlerId),
    DuplicateAcquire(AdvancedResourceId),
    ResourceNotLive(AdvancedResourceId),
    OneShotHandlerInvokedTwice(HandlerId),
    OwnedCaptureNotConsumed(HandlerId),
    OperationAfterExit,
    MissingExit,
}

impl fmt::Display for AdvancedControlError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for AdvancedControlError {}

enum PrimaryExit {
    Normal(i32),
    Error(String),
    Cancelled,
}

pub fn run_advanced(program: &AdvancedProgram) -> Result<AdvancedExecution, AdvancedControlError> {
    let destructors = validate_destructors(&program.destructors)?;
    let handlers = validate_handlers(&program.handlers)?;
    let mut invocations = BTreeMap::<HandlerId, usize>::new();
    let mut acquired = BTreeSet::new();
    let mut live = Vec::<(AdvancedResourceId, DestructorId)>::new();
    let mut cancellation_pending = false;
    let mut trace = Vec::new();

    for operation in &program.operations {
        match operation {
            AdvancedOp::Acquire {
                resource,
                destructor,
            } => {
                if !destructors.contains_key(destructor) {
                    return Err(AdvancedControlError::UnknownDestructor(*destructor));
                }
                if !acquired.insert(*resource) {
                    return Err(AdvancedControlError::DuplicateAcquire(*resource));
                }
                live.push((*resource, *destructor));
                trace.push(AdvancedTraceEvent::Acquire(*resource));
            }
            AdvancedOp::Drop(resource) => {
                let Some(position) = live.iter().position(|entry| entry.0 == *resource) else {
                    return Err(AdvancedControlError::ResourceNotLive(*resource));
                };
                let entry = live.remove(position);
                if let Some(error) = run_destructor(entry, &destructors, &mut trace) {
                    require_owned_captures_consumed(&handlers, &invocations)?;
                    return Ok(unwind(
                        PrimaryExit::Error(error),
                        &mut live,
                        &destructors,
                        trace,
                    ));
                }
            }
            AdvancedOp::RequestCancellation => {
                cancellation_pending = true;
                trace.push(AdvancedTraceEvent::CancellationRequested);
            }
            AdvancedOp::SafePoint => {
                if cancellation_pending {
                    require_owned_captures_consumed(&handlers, &invocations)?;
                    trace.push(AdvancedTraceEvent::CancellationDelivered);
                    return Ok(unwind(
                        PrimaryExit::Cancelled,
                        &mut live,
                        &destructors,
                        trace,
                    ));
                }
            }
            AdvancedOp::Invoke(handler) => {
                let Some(spec) = handlers.get(handler) else {
                    return Err(AdvancedControlError::UnknownHandler(*handler));
                };
                let count = invocations.entry(*handler).or_default();
                if spec.kind == HandlerKind::OneShot && *count != 0 {
                    return Err(AdvancedControlError::OneShotHandlerInvokedTwice(*handler));
                }
                *count += 1;
                trace.push(AdvancedTraceEvent::HandlerInvoked(*handler));
            }
            AdvancedOp::Raise(error) => {
                require_owned_captures_consumed(&handlers, &invocations)?;
                return Ok(unwind(
                    PrimaryExit::Error(error.clone()),
                    &mut live,
                    &destructors,
                    trace,
                ));
            }
            AdvancedOp::Return(value) => {
                let primary = if cancellation_pending {
                    trace.push(AdvancedTraceEvent::CancellationDelivered);
                    PrimaryExit::Cancelled
                } else {
                    PrimaryExit::Normal(*value)
                };
                require_owned_captures_consumed(&handlers, &invocations)?;
                return Ok(unwind(primary, &mut live, &destructors, trace));
            }
        }
    }
    Err(AdvancedControlError::MissingExit)
}

fn validate_destructors(
    specs: &[DestructorSpec],
) -> Result<BTreeMap<DestructorId, &DestructorSpec>, AdvancedControlError> {
    let mut result = BTreeMap::new();
    for spec in specs {
        if result.insert(spec.id, spec).is_some() {
            return Err(AdvancedControlError::DuplicateDestructor(spec.id));
        }
    }
    Ok(result)
}

fn validate_handlers(
    specs: &[HandlerSpec],
) -> Result<BTreeMap<HandlerId, &HandlerSpec>, AdvancedControlError> {
    let mut result = BTreeMap::new();
    let mut owned = BTreeSet::new();
    for spec in specs {
        if result.insert(spec.id, spec).is_some() {
            return Err(AdvancedControlError::DuplicateHandler(spec.id));
        }
        for capture in &spec.captures {
            if capture.mode == CaptureMode::Owned {
                if spec.kind != HandlerKind::OneShot {
                    return Err(AdvancedControlError::OwnedCaptureRequiresOneShot(spec.id));
                }
                if !owned.insert(capture.resource) {
                    return Err(AdvancedControlError::DuplicateCapture(capture.resource));
                }
            }
        }
    }
    Ok(result)
}

fn require_owned_captures_consumed(
    handlers: &BTreeMap<HandlerId, &HandlerSpec>,
    invocations: &BTreeMap<HandlerId, usize>,
) -> Result<(), AdvancedControlError> {
    for (&id, handler) in handlers {
        if handler
            .captures
            .iter()
            .any(|capture| capture.mode == CaptureMode::Owned)
            && invocations.get(&id).copied().unwrap_or(0) != 1
        {
            return Err(AdvancedControlError::OwnedCaptureNotConsumed(id));
        }
    }
    Ok(())
}

fn run_destructor(
    (resource, destructor): (AdvancedResourceId, DestructorId),
    destructors: &BTreeMap<DestructorId, &DestructorSpec>,
    trace: &mut Vec<AdvancedTraceEvent>,
) -> Option<String> {
    trace.push(AdvancedTraceEvent::Destroy(resource));
    match &destructors[&destructor].outcome {
        DestructorOutcome::Ok => None,
        DestructorOutcome::Error(error) => {
            trace.push(AdvancedTraceEvent::DestructorFailed {
                resource,
                error: error.clone(),
            });
            Some(error.clone())
        }
    }
}

fn unwind(
    primary: PrimaryExit,
    live: &mut Vec<(AdvancedResourceId, DestructorId)>,
    destructors: &BTreeMap<DestructorId, &DestructorSpec>,
    mut trace: Vec<AdvancedTraceEvent>,
) -> AdvancedExecution {
    let mut cleanup = Vec::new();
    while let Some(entry) = live.pop() {
        if let Some(error) = run_destructor(entry, destructors, &mut trace) {
            cleanup.push(error);
        }
    }
    let outcome = match (primary, cleanup.is_empty()) {
        (PrimaryExit::Normal(value), true) => AdvancedOutcome::Ok(value),
        (PrimaryExit::Normal(_), false) => AdvancedOutcome::CleanupFailure(cleanup),
        (PrimaryExit::Error(error), true) => AdvancedOutcome::Error(error),
        (PrimaryExit::Error(error), false) => AdvancedOutcome::DoubleError {
            primary: PrimaryFailure::Error(error),
            cleanup,
        },
        (PrimaryExit::Cancelled, true) => AdvancedOutcome::Cancelled,
        (PrimaryExit::Cancelled, false) => AdvancedOutcome::DoubleError {
            primary: PrimaryFailure::Cancelled,
            cleanup,
        },
    };
    AdvancedExecution { outcome, trace }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn destructors() -> Vec<DestructorSpec> {
        vec![
            DestructorSpec {
                id: 1,
                outcome: DestructorOutcome::Ok,
            },
            DestructorSpec {
                id: 2,
                outcome: DestructorOutcome::Error("close failed".into()),
            },
        ]
    }

    #[test]
    fn cancellation_is_deferred_to_a_safe_point_and_cleans_lifo() {
        let program = AdvancedProgram {
            destructors: destructors(),
            operations: vec![
                AdvancedOp::Acquire {
                    resource: 10,
                    destructor: 1,
                },
                AdvancedOp::RequestCancellation,
                AdvancedOp::Acquire {
                    resource: 11,
                    destructor: 1,
                },
                AdvancedOp::SafePoint,
            ],
            ..AdvancedProgram::default()
        };
        let execution = run_advanced(&program).unwrap();
        assert_eq!(execution.outcome, AdvancedOutcome::Cancelled);
        assert_eq!(
            execution.trace,
            vec![
                AdvancedTraceEvent::Acquire(10),
                AdvancedTraceEvent::CancellationRequested,
                AdvancedTraceEvent::Acquire(11),
                AdvancedTraceEvent::CancellationDelivered,
                AdvancedTraceEvent::Destroy(11),
                AdvancedTraceEvent::Destroy(10),
            ]
        );
    }

    #[test]
    fn fallible_cleanup_on_normal_return_has_a_distinct_outcome() {
        let program = AdvancedProgram {
            destructors: destructors(),
            operations: vec![
                AdvancedOp::Acquire {
                    resource: 10,
                    destructor: 2,
                },
                AdvancedOp::Return(7),
            ],
            ..AdvancedProgram::default()
        };
        assert_eq!(
            run_advanced(&program).unwrap().outcome,
            AdvancedOutcome::CleanupFailure(vec!["close failed".into()])
        );
    }

    #[test]
    fn double_error_preserves_the_primary_error() {
        let program = AdvancedProgram {
            destructors: destructors(),
            operations: vec![
                AdvancedOp::Acquire {
                    resource: 10,
                    destructor: 2,
                },
                AdvancedOp::Raise("parse failed".into()),
            ],
            ..AdvancedProgram::default()
        };
        assert_eq!(
            run_advanced(&program).unwrap().outcome,
            AdvancedOutcome::DoubleError {
                primary: PrimaryFailure::Error("parse failed".into()),
                cleanup: vec!["close failed".into()]
            }
        );
    }

    #[test]
    fn owned_capture_is_linear_and_one_shot() {
        let handler = HandlerSpec {
            id: 4,
            kind: HandlerKind::OneShot,
            captures: vec![Capture {
                resource: 10,
                mode: CaptureMode::Owned,
            }],
        };
        let accepted = AdvancedProgram {
            handlers: vec![handler.clone()],
            operations: vec![AdvancedOp::Invoke(4), AdvancedOp::Return(0)],
            ..AdvancedProgram::default()
        };
        assert!(run_advanced(&accepted).is_ok());

        let duplicated = AdvancedProgram {
            handlers: vec![handler],
            operations: vec![
                AdvancedOp::Invoke(4),
                AdvancedOp::Invoke(4),
                AdvancedOp::Return(0),
            ],
            ..AdvancedProgram::default()
        };
        assert_eq!(
            run_advanced(&duplicated),
            Err(AdvancedControlError::OneShotHandlerInvokedTwice(4))
        );
    }

    #[test]
    fn multi_shot_and_resumable_handlers_reject_owned_capture() {
        for kind in [HandlerKind::MultiShot, HandlerKind::Resumable] {
            let program = AdvancedProgram {
                handlers: vec![HandlerSpec {
                    id: 4,
                    kind,
                    captures: vec![Capture {
                        resource: 10,
                        mode: CaptureMode::Owned,
                    }],
                }],
                operations: vec![AdvancedOp::Return(0)],
                ..AdvancedProgram::default()
            };
            assert_eq!(
                run_advanced(&program),
                Err(AdvancedControlError::OwnedCaptureRequiresOneShot(4))
            );
        }
    }

    #[test]
    fn multi_shot_shared_capture_can_be_invoked_repeatedly() {
        let program = AdvancedProgram {
            handlers: vec![HandlerSpec {
                id: 4,
                kind: HandlerKind::MultiShot,
                captures: vec![Capture {
                    resource: 10,
                    mode: CaptureMode::Shared,
                }],
            }],
            operations: vec![
                AdvancedOp::Invoke(4),
                AdvancedOp::Invoke(4),
                AdvancedOp::Return(0),
            ],
            ..AdvancedProgram::default()
        };
        assert!(run_advanced(&program).is_ok());
    }

    #[test]
    fn normal_cleanup_destroys_each_resource_exactly_once() {
        let program = AdvancedProgram {
            destructors: destructors(),
            operations: vec![
                AdvancedOp::Acquire {
                    resource: 10,
                    destructor: 1,
                },
                AdvancedOp::Acquire {
                    resource: 11,
                    destructor: 1,
                },
                AdvancedOp::Drop(10),
                AdvancedOp::Return(0),
            ],
            ..AdvancedProgram::default()
        };
        let execution = run_advanced(&program).unwrap();
        let destroyed = execution
            .trace
            .iter()
            .filter_map(|event| match event {
                AdvancedTraceEvent::Destroy(resource) => Some(*resource),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(destroyed, vec![10, 11]);
    }
}
