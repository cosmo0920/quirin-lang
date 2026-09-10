//! Lowering from a checked linear source path to a cleanup-explicit typed CFG.
//!
//! Every potentially failing operation receives an edge-specific cleanup block.
//! That block contains exactly the resources live before the failing edge, in
//! reverse acquisition order.  No runtime proof objects survive this pass.

use std::collections::BTreeSet;
use std::fmt;

use crate::ResourceId;
use crate::cfg::{
    BasicBlock, BlockId, CfgError, CfgInstruction, CfgTerminator, ErrorAbi, ExecutionObservation,
    ExecutionOutcome, TraceEvent, TypedCfg,
};

/// The comparison relations of `source.branch.v1`'s decidable condition tier
/// (card `IF-STATEMENT`).  Deliberately closed and deliberately small: this is
/// the minimal tier a branch needs, not a boolean expression language.  The
/// connectives `||` and `&&` are **not** here, by construction — supplying them
/// is the successor card's whole content, and they lower onto the branch this
/// module defines rather than extending this enumeration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompareOp {
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
}

impl CompareOp {
    /// The relation itself.  Total and decidable on `i32`.
    pub const fn apply(self, left: i32, right: i32) -> bool {
        match self {
            Self::Equal => left == right,
            Self::NotEqual => left != right,
            Self::Less => left < right,
            Self::LessEqual => left <= right,
            Self::Greater => left > right,
            Self::GreaterEqual => left >= right,
        }
    }

    /// The C11 spelling.  The emitted program computes the comparison itself
    /// rather than receiving the decided literal, so the reference evaluator
    /// and a C compiler are cross-checked by the translation-validation
    /// triangle and by the sanitized native run.
    pub const fn c_operator(self) -> &'static str {
        match self {
            Self::Equal => "==",
            Self::NotEqual => "!=",
            Self::Less => "<",
            Self::LessEqual => "<=",
            Self::Greater => ">",
            Self::GreaterEqual => ">=",
        }
    }
}

/// A `source.branch.v1` condition.  Decidable at check time, exactly as the
/// `succeeds` and `passes` literal outcome controls beside it are: the
/// translation certificate compares one reference observation, one CFG
/// observation and one observation parsed from the compiled program's own
/// output, and a run-time-varying condition would leave "the" observation
/// ill-defined.  Lifting that is named follow-on `IF-RUNTIME-CONDITION`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BranchCondition {
    Literal(bool),
    Compare {
        left: i32,
        operator: CompareOp,
        right: i32,
    },
}

impl BranchCondition {
    /// Total: every condition in the tier has a value.
    pub const fn evaluate(self) -> bool {
        match self {
            Self::Literal(value) => value,
            Self::Compare {
                left,
                operator,
                right,
            } => operator.apply(left, right),
        }
    }
}

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
    /// Structured two-way branching (card `IF-STATEMENT`, `source.branch.v1`).
    ///
    /// Both arms are validated from the same incoming custody state and must
    /// reach the join agreeing on the ordered live list and on the transferred
    /// set; a disagreement is [`LoweringError::BranchArmsDisagree`], never a
    /// silent join.  Neither arm may return: `Return` is the last operation of
    /// a program, and an early exit from an arm has no position in this linear
    /// representation (follow-on `IF-ARM-RETURN`).
    Branch {
        condition: BranchCondition,
        then_arm: Vec<SourceOp>,
        else_arm: Vec<SourceOp>,
    },
}

/// The custody state a statement sequence carries.  `live` is ordered because
/// a cleanup block releases live resources in **reverse acquisition order**, so
/// the order is part of the state rather than a presentation detail; comparing
/// sequences rather than sets is the fail-closed choice at a branch join.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct CustodyState {
    live: Vec<ResourceId>,
    transferred: BTreeSet<ResourceId>,
    returned: bool,
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

        // `acquired` is threaded across the whole program, both arms of every
        // branch included: an acquisition identity names one `void *` slot in
        // the emitted C, so acquiring the same identity in two arms is refused
        // rather than joined.
        let mut acquired = BTreeSet::new();
        let mut custody = CustodyState::default();
        self.validate_sequence(&self.operations, &declared, &mut acquired, &mut custody)?;
        if custody.returned {
            Ok(())
        } else {
            Err(LoweringError::MissingReturn)
        }
    }

    fn validate_sequence(
        &self,
        operations: &[SourceOp],
        declared: &BTreeSet<ResourceId>,
        acquired: &mut BTreeSet<ResourceId>,
        custody: &mut CustodyState,
    ) -> Result<(), LoweringError> {
        for operation in operations {
            if custody.returned {
                return Err(LoweringError::OperationAfterReturn);
            }
            match operation {
                SourceOp::Acquire {
                    resource, error, ..
                } => {
                    self.require_error(error)?;
                    if !custody.transferred.is_empty() {
                        return Err(LoweringError::FallibleOperationAfterTransfer);
                    }
                    if !declared.contains(resource) {
                        return Err(LoweringError::UnknownResource(*resource));
                    }
                    if !acquired.insert(*resource) {
                        return Err(LoweringError::ResourceAlreadyOwned(*resource));
                    }
                    custody.live.push(*resource);
                }
                SourceOp::Check { error, .. } => {
                    self.require_error(error)?;
                    if !custody.transferred.is_empty() {
                        return Err(LoweringError::FallibleOperationAfterTransfer);
                    }
                }
                SourceOp::Drop(resource) | SourceOp::TransferToResult(resource) => {
                    let Some(position) = custody
                        .live
                        .iter()
                        .position(|candidate| candidate == resource)
                    else {
                        return Err(LoweringError::ResourceNotLive(*resource));
                    };
                    custody.live.remove(position);
                    if matches!(operation, SourceOp::TransferToResult(_)) {
                        custody.transferred.insert(*resource);
                    }
                }
                SourceOp::Return(_) => {
                    if !custody.live.is_empty() {
                        return Err(LoweringError::LiveResourcesAtNormalReturn(
                            custody.live.clone(),
                        ));
                    }
                    custody.returned = true;
                }
                SourceOp::Branch {
                    then_arm, else_arm, ..
                } => {
                    // Both arms start from the same state. Each is validated in
                    // full — a leak, a double release or a use of a non-live
                    // identity refuses in the arm the decided condition does not
                    // take exactly as it does in the arm it takes.
                    let mut then_state = custody.clone();
                    self.validate_sequence(then_arm, declared, acquired, &mut then_state)?;
                    let mut else_state = custody.clone();
                    self.validate_sequence(else_arm, declared, acquired, &mut else_state)?;
                    if then_state.returned || else_state.returned {
                        return Err(LoweringError::ReturnInBranchArm);
                    }
                    if then_state != else_state {
                        return Err(LoweringError::BranchArmsDisagree {
                            then_live: then_state.live,
                            else_live: else_state.live,
                            then_transferred: then_state.transferred.into_iter().collect(),
                            else_transferred: else_state.transferred.into_iter().collect(),
                        });
                    }
                    *custody = then_state;
                }
            }
        }
        Ok(())
    }

    fn require_error(&self, error: &str) -> Result<(), LoweringError> {
        if self.error_abi.variant(error).is_some() {
            Ok(())
        } else {
            Err(LoweringError::UnknownError(error.into()))
        }
    }
}

/// The block ids allocated to one operation and, for a branch, to its arms.
///
/// Allocation is a pre-order walk: an operation takes the next id, then its
/// `then` arm, then its `else` arm.  On a **branch-free** program that is
/// exactly the pre-`IF-STATEMENT` numbering — operation `i` takes block `i`,
/// and cleanup blocks start at the operation count — so no emitted C byte moves
/// for a program that does not use the construct.
struct OperationBlocks {
    block: BlockId,
    arms: Option<(Vec<OperationBlocks>, Vec<OperationBlocks>)>,
}

fn allocate_blocks(operations: &[SourceOp], next: &mut BlockId) -> Vec<OperationBlocks> {
    operations
        .iter()
        .map(|operation| {
            let block = *next;
            *next += 1;
            let arms = match operation {
                SourceOp::Branch {
                    then_arm, else_arm, ..
                } => Some((
                    allocate_blocks(then_arm, next),
                    allocate_blocks(else_arm, next),
                )),
                _ => None,
            };
            OperationBlocks { block, arms }
        })
        .collect()
}

pub fn lower_cleanup_cfg(program: &SourceProgram) -> Result<TypedCfg, LoweringError> {
    program.validate()?;
    let mut next_block = 0;
    let allocation = allocate_blocks(&program.operations, &mut next_block);
    let mut lowerer = Lowerer {
        program,
        blocks: Vec::new(),
        next_cleanup: next_block,
    };
    let mut live = Vec::<ResourceId>::new();
    // A validated program's last top-level operation is `Return`, so the
    // top-level continuation is unreachable. It is deliberately an id no block
    // carries: were it ever taken, `TypedCfg::new` refuses with `MissingBlock`
    // rather than silently wiring an edge somewhere plausible.
    lowerer.lower_sequence(&program.operations, &allocation, &mut live, BlockId::MAX)?;

    TypedCfg::new(
        program.function.clone(),
        0,
        program.resources.clone(),
        program.error_abi.clone(),
        lowerer.blocks,
    )
    .map_err(LoweringError::InvalidCfg)
}

struct Lowerer<'a> {
    program: &'a SourceProgram,
    blocks: Vec<BasicBlock>,
    next_cleanup: BlockId,
}

impl Lowerer<'_> {
    fn lower_sequence(
        &mut self,
        operations: &[SourceOp],
        allocation: &[OperationBlocks],
        live: &mut Vec<ResourceId>,
        continuation: BlockId,
    ) -> Result<(), LoweringError> {
        for (position, operation) in operations.iter().enumerate() {
            let index = allocation[position].block;
            let next = allocation
                .get(position + 1)
                .map_or(continuation, |entry| entry.block);
            match operation {
                SourceOp::Acquire {
                    resource,
                    succeeds,
                    error,
                } => {
                    let cleanup = self.next_cleanup;
                    self.next_cleanup += 1;
                    self.blocks.push(BasicBlock::new(
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
                    let block = cleanup_block(self.program, cleanup, error, live)?;
                    self.blocks.push(block);
                    live.push(*resource);
                }
                SourceOp::Check { passes, error } => {
                    let cleanup = self.next_cleanup;
                    self.next_cleanup += 1;
                    self.blocks.push(BasicBlock::new(
                        index,
                        false,
                        Vec::new(),
                        CfgTerminator::Check {
                            passes: *passes,
                            success: next,
                            failure: cleanup,
                        },
                    ));
                    let block = cleanup_block(self.program, cleanup, error, live)?;
                    self.blocks.push(block);
                }
                SourceOp::Drop(resource) => {
                    remove_live(live, *resource)?;
                    self.blocks.push(BasicBlock::new(
                        index,
                        false,
                        vec![CfgInstruction::Release(*resource)],
                        CfgTerminator::Goto(next),
                    ));
                }
                SourceOp::TransferToResult(resource) => {
                    remove_live(live, *resource)?;
                    self.blocks.push(BasicBlock::new(
                        index,
                        false,
                        vec![CfgInstruction::TransferToResult(*resource)],
                        CfgTerminator::Goto(next),
                    ));
                }
                SourceOp::Return(value) => self.blocks.push(BasicBlock::new(
                    index,
                    false,
                    Vec::new(),
                    CfgTerminator::ReturnOk(*value),
                )),
                SourceOp::Branch {
                    condition,
                    then_arm,
                    else_arm,
                } => {
                    let (then_allocation, else_allocation) = allocation[position]
                        .arms
                        .as_ref()
                        .expect("a branch allocates arm blocks");
                    // An empty arm has no first block, so its edge is the join
                    // directly. The join is this branch's own continuation.
                    let then_block = then_allocation.first().map_or(next, |entry| entry.block);
                    let else_block = else_allocation.first().map_or(next, |entry| entry.block);
                    self.blocks.push(BasicBlock::new(
                        index,
                        false,
                        Vec::new(),
                        CfgTerminator::Branch {
                            condition: *condition,
                            then_block,
                            else_block,
                        },
                    ));
                    let mut then_live = live.clone();
                    self.lower_sequence(then_arm, then_allocation, &mut then_live, next)?;
                    let mut else_live = live.clone();
                    self.lower_sequence(else_arm, else_allocation, &mut else_live, next)?;
                    // `validate` proved the two agree before any block was
                    // built; taking `then_live` is a choice between equals.
                    debug_assert_eq!(then_live, else_live);
                    *live = then_live;
                }
            }
        }
        Ok(())
    }
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
    match interpret_sequence(
        program,
        &program.operations,
        &mut live,
        &mut payload,
        &mut trace,
    )? {
        Some(observation) => Ok(observation),
        None => Err(LoweringError::MissingReturn),
    }
}

/// Interpret one statement sequence.  `Some(observation)` means the sequence
/// terminated the function — a return or an error exit — and `None` means it
/// fell through to its continuation, which is what a branch arm does.
fn interpret_sequence(
    program: &SourceProgram,
    operations: &[SourceOp],
    live: &mut Vec<ResourceId>,
    payload: &mut BTreeSet<ResourceId>,
    trace: &mut Vec<TraceEvent>,
) -> Result<Option<ExecutionObservation>, LoweringError> {
    for operation in operations {
        match operation {
            SourceOp::Acquire {
                resource,
                succeeds,
                error,
            } => {
                if !succeeds {
                    return error_observation(program, error, live, std::mem::take(trace))
                        .map(Some);
                }
                live.push(*resource);
                trace.push(TraceEvent::Acquire(*resource));
            }
            SourceOp::Check { passes, error } => {
                if !passes {
                    return error_observation(program, error, live, std::mem::take(trace))
                        .map(Some);
                }
            }
            SourceOp::Drop(resource) => {
                remove_live(live, *resource)?;
                trace.push(TraceEvent::Release(*resource));
            }
            SourceOp::TransferToResult(resource) => {
                remove_live(live, *resource)?;
                payload.insert(*resource);
                trace.push(TraceEvent::Transfer(*resource));
            }
            SourceOp::Return(value) => {
                return Ok(Some(ExecutionObservation {
                    outcome: ExecutionOutcome::Ok {
                        value: *value,
                        payload: payload.iter().copied().collect(),
                    },
                    trace: std::mem::take(trace),
                }));
            }
            SourceOp::Branch {
                condition,
                then_arm,
                else_arm,
            } => {
                // The condition is decidable, so exactly one arm is observed.
                // The arm not observed is nonetheless validated in full and
                // emitted in full; only the *observation* is single-valued.
                let arm = if condition.evaluate() {
                    then_arm
                } else {
                    else_arm
                };
                if let Some(observation) = interpret_sequence(program, arm, live, payload, trace)? {
                    return Ok(Some(observation));
                }
            }
        }
    }
    Ok(None)
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
    /// Card `IF-STATEMENT`. The arms of a branch reached the join in
    /// disagreeing custody states — the conditional leak / conditional
    /// double-release shape. Fail-closed by construction: there is no join
    /// rule that widens or narrows one arm's state to the other's.
    BranchArmsDisagree {
        then_live: Vec<ResourceId>,
        else_live: Vec<ResourceId>,
        then_transferred: Vec<ResourceId>,
        else_transferred: Vec<ResourceId>,
    },
    /// Card `IF-STATEMENT`. `return` inside a branch arm; follow-on
    /// `IF-ARM-RETURN`.
    ReturnInBranchArm,
}

impl fmt::Display for LoweringError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for LoweringError {}

#[cfg(test)]
#[path = "../tests/unit/cleanup_lowering.rs"]
mod tests;
