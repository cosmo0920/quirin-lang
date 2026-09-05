//! Lowering from a checked linear source path to a cleanup-explicit typed CFG.
//!
//! Every potentially failing operation receives an edge-specific cleanup block.
//! That block contains exactly the resources live before the failing edge, in
//! reverse acquisition order.  No runtime proof objects survive this pass.

use std::collections::BTreeSet;
use std::fmt;

use crate::ResourceId;
use crate::cfg::{
    BasicBlock, CfgError, CfgInstruction, CfgTerminator, ErrorAbi, ExecutionObservation,
    ExecutionOutcome, TraceEvent, TypedCfg,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceOp {
    /// Acquire a fresh runtime resource.  A failed acquisition does not make
    /// the resource live and transfers control to the error cleanup edge.
    Acquire {
        resource: ResourceId,
        succeeds: bool,
        error: String,
    },
    /// A fallible operation that leaves the resource ledger unchanged.
    Check {
        passes: bool,
        error: String,
    },
    Drop(ResourceId),
    TransferToResult(ResourceId),
    Return(i32),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceProgram {
    function: String,
    resources: Vec<ResourceId>,
    error_abi: ErrorAbi,
    operations: Vec<SourceOp>,
}

impl SourceProgram {
    pub fn new(
        function: impl Into<String>,
        resources: Vec<ResourceId>,
        errors: impl IntoIterator<Item = String>,
        operations: Vec<SourceOp>,
    ) -> Result<Self, LoweringError> {
        let error_abi = ErrorAbi::new(errors).map_err(LoweringError::InvalidCfg)?;
        let program = Self {
            function: function.into(),
            resources,
            error_abi,
            operations,
        };
        program.validate()?;
        Ok(program)
    }

    pub fn function(&self) -> &str {
        &self.function
    }

    pub fn resources(&self) -> &[ResourceId] {
        &self.resources
    }

    pub fn error_abi(&self) -> &ErrorAbi {
        &self.error_abi
    }

    pub fn operations(&self) -> &[SourceOp] {
        &self.operations
    }

    fn validate(&self) -> Result<(), LoweringError> {
        if self.operations.is_empty() {
            return Err(LoweringError::MissingReturn);
        }
        let declared = self.resources.iter().copied().collect::<BTreeSet<_>>();
        if declared.len() != self.resources.len() {
            let mut seen = BTreeSet::new();
            let duplicate = self
                .resources
                .iter()
                .find(|resource| !seen.insert(**resource))
                .copied()
                .expect("length mismatch guarantees a duplicate");
            return Err(LoweringError::DuplicateResource(duplicate));
        }

        let mut live = Vec::new();
        let mut acquired = BTreeSet::new();
        let mut transferred = BTreeSet::new();
        let mut returned = false;
        for operation in &self.operations {
            if returned {
                return Err(LoweringError::OperationAfterReturn);
            }
            match operation {
                SourceOp::Acquire {
                    resource, error, ..
                } => {
                    self.require_error(error)?;
                    if !transferred.is_empty() {
                        return Err(LoweringError::FallibleOperationAfterTransfer);
                    }
                    if !declared.contains(resource) {
                        return Err(LoweringError::UnknownResource(*resource));
                    }
                    if !acquired.insert(*resource) {
                        return Err(LoweringError::ResourceAlreadyOwned(*resource));
                    }
                    live.push(*resource);
                }
                SourceOp::Check { error, .. } => {
                    self.require_error(error)?;
                    if !transferred.is_empty() {
                        return Err(LoweringError::FallibleOperationAfterTransfer);
                    }
                }
                SourceOp::Drop(resource) | SourceOp::TransferToResult(resource) => {
                    let Some(position) = live.iter().position(|candidate| candidate == resource)
                    else {
                        return Err(LoweringError::ResourceNotLive(*resource));
                    };
                    live.remove(position);
                    if matches!(operation, SourceOp::TransferToResult(_)) {
                        transferred.insert(*resource);
                    }
                }
                SourceOp::Return(_) => {
                    if !live.is_empty() {
                        return Err(LoweringError::LiveResourcesAtNormalReturn(live));
                    }
                    returned = true;
                }
            }
        }
        if returned {
            Ok(())
        } else {
            Err(LoweringError::MissingReturn)
        }
    }

    fn require_error(&self, error: &str) -> Result<(), LoweringError> {
        if self.error_abi.variant(error).is_some() {
            Ok(())
        } else {
            Err(LoweringError::UnknownError(error.into()))
        }
    }
}

pub fn lower_cleanup_cfg(program: &SourceProgram) -> Result<TypedCfg, LoweringError> {
    program.validate()?;
    let mut live = Vec::<ResourceId>::new();
    let mut blocks = Vec::new();
    let mut next_cleanup = program.operations.len();

    for (index, operation) in program.operations.iter().enumerate() {
        let next = index + 1;
        match operation {
            SourceOp::Acquire {
                resource,
                succeeds,
                error,
            } => {
                let cleanup = next_cleanup;
                next_cleanup += 1;
                blocks.push(BasicBlock::new(
                    index,
                    false,
                    Vec::new(),
                    CfgTerminator::TryAcquire {
                        resource: *resource,
                        succeeds: *succeeds,
                        success: next,
                        failure: cleanup,
                    },
                ));
                blocks.push(cleanup_block(program, cleanup, error, &live)?);
                live.push(*resource);
            }
            SourceOp::Check { passes, error } => {
                let cleanup = next_cleanup;
                next_cleanup += 1;
                blocks.push(BasicBlock::new(
                    index,
                    false,
                    Vec::new(),
                    CfgTerminator::Check {
                        passes: *passes,
                        success: next,
                        failure: cleanup,
                    },
                ));
                blocks.push(cleanup_block(program, cleanup, error, &live)?);
            }
            SourceOp::Drop(resource) => {
                remove_live(&mut live, *resource)?;
                blocks.push(BasicBlock::new(
                    index,
                    false,
                    vec![CfgInstruction::Release(*resource)],
                    CfgTerminator::Goto(next),
                ));
            }
            SourceOp::TransferToResult(resource) => {
                remove_live(&mut live, *resource)?;
                blocks.push(BasicBlock::new(
                    index,
                    false,
                    vec![CfgInstruction::TransferToResult(*resource)],
                    CfgTerminator::Goto(next),
                ));
            }
            SourceOp::Return(value) => blocks.push(BasicBlock::new(
                index,
                false,
                Vec::new(),
                CfgTerminator::ReturnOk(*value),
            )),
        }
    }

    TypedCfg::new(
        program.function.clone(),
        0,
        program.resources.clone(),
        program.error_abi.clone(),
        blocks,
    )
    .map_err(LoweringError::InvalidCfg)
}

fn cleanup_block(
    program: &SourceProgram,
    id: usize,
    error: &str,
    live: &[ResourceId],
) -> Result<BasicBlock, LoweringError> {
    let variant = program
        .error_abi
        .variant(error)
        .ok_or_else(|| LoweringError::UnknownError(error.into()))?;
    let instructions = live
        .iter()
        .rev()
        .copied()
        .map(CfgInstruction::Release)
        .collect();
    Ok(BasicBlock::new(
        id,
        true,
        instructions,
        CfgTerminator::ReturnError(variant.tag()),
    ))
}

fn remove_live(live: &mut Vec<ResourceId>, resource: ResourceId) -> Result<(), LoweringError> {
    let position = live
        .iter()
        .position(|candidate| *candidate == resource)
        .ok_or(LoweringError::ResourceNotLive(resource))?;
    live.remove(position);
    Ok(())
}

/// Independent reference semantics used for translation validation against
/// both the typed CFG interpreter and compiled generated C.
pub fn interpret_source(program: &SourceProgram) -> Result<ExecutionObservation, LoweringError> {
    program.validate()?;
    let mut live = Vec::<ResourceId>::new();
    let mut payload = BTreeSet::new();
    let mut trace = Vec::new();
    for operation in &program.operations {
        match operation {
            SourceOp::Acquire {
                resource,
                succeeds,
                error,
            } => {
                if !succeeds {
                    return error_observation(program, error, &live, trace);
                }
                live.push(*resource);
                trace.push(TraceEvent::Acquire(*resource));
            }
            SourceOp::Check { passes, error } => {
                if !passes {
                    return error_observation(program, error, &live, trace);
                }
            }
            SourceOp::Drop(resource) => {
                remove_live(&mut live, *resource)?;
                trace.push(TraceEvent::Release(*resource));
            }
            SourceOp::TransferToResult(resource) => {
                remove_live(&mut live, *resource)?;
                payload.insert(*resource);
                trace.push(TraceEvent::Transfer(*resource));
            }
            SourceOp::Return(value) => {
                return Ok(ExecutionObservation {
                    outcome: ExecutionOutcome::Ok {
                        value: *value,
                        payload: payload.into_iter().collect(),
                    },
                    trace,
                });
            }
        }
    }
    Err(LoweringError::MissingReturn)
}

fn error_observation(
    program: &SourceProgram,
    error: &str,
    live: &[ResourceId],
    mut trace: Vec<TraceEvent>,
) -> Result<ExecutionObservation, LoweringError> {
    for resource in live.iter().rev() {
        trace.push(TraceEvent::Release(*resource));
    }
    let variant = program
        .error_abi
        .variant(error)
        .ok_or_else(|| LoweringError::UnknownError(error.into()))?;
    Ok(ExecutionObservation {
        outcome: ExecutionOutcome::Error {
            name: error.into(),
            tag: variant.tag(),
        },
        trace,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LoweringError {
    InvalidCfg(CfgError),
    DuplicateResource(ResourceId),
    UnknownResource(ResourceId),
    ResourceAlreadyOwned(ResourceId),
    ResourceNotLive(ResourceId),
    UnknownError(String),
    LiveResourcesAtNormalReturn(Vec<ResourceId>),
    FallibleOperationAfterTransfer,
    OperationAfterReturn,
    MissingReturn,
}

impl fmt::Display for LoweringError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for LoweringError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cfg::interpret_cfg;

    fn program(operations: Vec<SourceOp>) -> SourceProgram {
        SourceProgram::new(
            "load",
            vec![1, 2],
            vec!["IoError".into(), "ParseError".into()],
            operations,
        )
        .unwrap()
    }

    #[test]
    fn partial_initialization_uses_edge_specific_cleanup() {
        let source = program(vec![
            SourceOp::Acquire {
                resource: 1,
                succeeds: true,
                error: "IoError".into(),
            },
            SourceOp::Acquire {
                resource: 2,
                succeeds: false,
                error: "IoError".into(),
            },
            SourceOp::Drop(2),
            SourceOp::Drop(1),
            SourceOp::Return(0),
        ]);
        let cfg = lower_cleanup_cfg(&source).unwrap();
        let expected = interpret_source(&source).unwrap();
        assert_eq!(interpret_cfg(&cfg).unwrap(), expected);
        assert_eq!(
            expected.trace,
            vec![TraceEvent::Acquire(1), TraceEvent::Release(1)]
        );

        let cleanup_blocks = cfg
            .blocks()
            .filter(|block| block.is_cleanup())
            .collect::<Vec<_>>();
        assert_eq!(cleanup_blocks.len(), 2);
        assert!(cleanup_blocks[0].instructions().is_empty());
        assert_eq!(
            cleanup_blocks[1].instructions(),
            &[CfgInstruction::Release(1)]
        );
    }

    #[test]
    fn cleanup_is_lifo_and_releases_each_identity_once() {
        let source = program(vec![
            SourceOp::Acquire {
                resource: 1,
                succeeds: true,
                error: "IoError".into(),
            },
            SourceOp::Acquire {
                resource: 2,
                succeeds: true,
                error: "IoError".into(),
            },
            SourceOp::Check {
                passes: false,
                error: "ParseError".into(),
            },
            SourceOp::Drop(2),
            SourceOp::Drop(1),
            SourceOp::Return(0),
        ]);
        let observation = interpret_cfg(&lower_cleanup_cfg(&source).unwrap()).unwrap();
        assert_eq!(
            observation.trace,
            vec![
                TraceEvent::Acquire(1),
                TraceEvent::Acquire(2),
                TraceEvent::Release(2),
                TraceEvent::Release(1),
            ]
        );
    }

    #[test]
    fn normal_transfer_is_explicit_and_not_cleaned_locally() {
        let source = program(vec![
            SourceOp::Acquire {
                resource: 2,
                succeeds: true,
                error: "IoError".into(),
            },
            SourceOp::TransferToResult(2),
            SourceOp::Return(17),
        ]);
        let expected = interpret_source(&source).unwrap();
        assert_eq!(
            interpret_cfg(&lower_cleanup_cfg(&source).unwrap()).unwrap(),
            expected
        );
        assert_eq!(
            expected.outcome,
            ExecutionOutcome::Ok {
                value: 17,
                payload: vec![2]
            }
        );
        assert_eq!(
            expected.trace,
            vec![TraceEvent::Acquire(2), TraceEvent::Transfer(2)]
        );
    }

    #[test]
    fn invalid_normal_leak_is_rejected() {
        assert!(matches!(
            SourceProgram::new(
                "leak",
                vec![1],
                vec!["IoError".into()],
                vec![
                    SourceOp::Acquire {
                        resource: 1,
                        succeeds: true,
                        error: "IoError".into(),
                    },
                    SourceOp::Return(0),
                ],
            ),
            Err(LoweringError::LiveResourcesAtNormalReturn(_))
        ));
    }

    #[test]
    fn resource_identity_cannot_be_reacquired_after_release() {
        assert_eq!(
            SourceProgram::new(
                "duplicate_identity",
                vec![1],
                vec!["IoError".into()],
                vec![
                    SourceOp::Acquire {
                        resource: 1,
                        succeeds: true,
                        error: "IoError".into(),
                    },
                    SourceOp::Drop(1),
                    SourceOp::Acquire {
                        resource: 1,
                        succeeds: true,
                        error: "IoError".into(),
                    },
                    SourceOp::Drop(1),
                    SourceOp::Return(0),
                ],
            ),
            Err(LoweringError::ResourceAlreadyOwned(1))
        );
    }
}
