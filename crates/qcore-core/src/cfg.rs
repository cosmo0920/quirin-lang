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

//! Typed, cleanup-explicit control-flow graph used by the Cycle 7 C backend.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::ResourceId;
use crate::cleanup_lowering::BranchCondition;

pub type BlockId = usize;
pub type ErrorTag = u32;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ErrorVariant {
    name: String,
    tag: ErrorTag,
}

impl ErrorVariant {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn tag(&self) -> ErrorTag {
        self.tag
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ErrorAbi {
    variants: Vec<ErrorVariant>,
}

impl ErrorAbi {
    pub fn new(names: impl IntoIterator<Item = String>) -> Result<Self, CfgError> {
        let mut seen = BTreeSet::new();
        let mut variants = Vec::new();
        for (index, name) in names.into_iter().enumerate() {
            if !is_c_identifier(&name) {
                return Err(CfgError::InvalidCIdentifier(name));
            }
            if !seen.insert(name.clone()) {
                return Err(CfgError::DuplicateError(name));
            }
            let tag = ErrorTag::try_from(index + 1).map_err(|_| CfgError::TooManyErrors)?;
            variants.push(ErrorVariant { name, tag });
        }
        if variants.is_empty() {
            return Err(CfgError::EmptyErrorAbi);
        }
        Ok(Self { variants })
    }

    pub fn variants(&self) -> &[ErrorVariant] {
        &self.variants
    }

    pub fn variant(&self, name: &str) -> Option<&ErrorVariant> {
        self.variants.iter().find(|variant| variant.name == name)
    }

    pub fn variant_by_tag(&self, tag: ErrorTag) -> Option<&ErrorVariant> {
        self.variants.iter().find(|variant| variant.tag == tag)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CfgInstruction {
    Release(ResourceId),
    TransferToResult(ResourceId),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CfgTerminator {
    Goto(BlockId),
    TryAcquire {
        resource: ResourceId,
        succeeds: bool,
        success: BlockId,
        failure: BlockId,
    },
    Check {
        passes: bool,
        success: BlockId,
        failure: BlockId,
    },
    /// Structured two-way branching (card `IF-STATEMENT`, `source.branch.v1`).
    ///
    /// Deliberately **not** spelled as a [`Self::Check`] with a non-cleanup
    /// failure edge, though the two emit the same C shape: `Check`'s failure
    /// edge is an error cleanup edge, and reusing it would make the typed CFG
    /// describe a fallible operation where the source wrote a branch.
    Branch {
        condition: BranchCondition,
        then_block: BlockId,
        else_block: BlockId,
    },
    ReturnOk(i32),
    ReturnError(ErrorTag),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BasicBlock {
    id: BlockId,
    cleanup: bool,
    instructions: Vec<CfgInstruction>,
    terminator: CfgTerminator,
}

impl BasicBlock {
    pub fn new(
        id: BlockId,
        cleanup: bool,
        instructions: Vec<CfgInstruction>,
        terminator: CfgTerminator,
    ) -> Self {
        Self {
            id,
            cleanup,
            instructions,
            terminator,
        }
    }

    pub const fn id(&self) -> BlockId {
        self.id
    }

    pub const fn is_cleanup(&self) -> bool {
        self.cleanup
    }

    pub fn instructions(&self) -> &[CfgInstruction] {
        &self.instructions
    }

    pub fn terminator(&self) -> &CfgTerminator {
        &self.terminator
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypedCfg {
    function: String,
    entry: BlockId,
    resources: Vec<ResourceId>,
    error_abi: ErrorAbi,
    blocks: BTreeMap<BlockId, BasicBlock>,
}

impl TypedCfg {
    pub fn new(
        function: String,
        entry: BlockId,
        resources: Vec<ResourceId>,
        error_abi: ErrorAbi,
        blocks: impl IntoIterator<Item = BasicBlock>,
    ) -> Result<Self, CfgError> {
        if !is_c_identifier(&function) {
            return Err(CfgError::InvalidCIdentifier(function));
        }
        let mut resource_set = BTreeSet::new();
        for resource in &resources {
            if !resource_set.insert(*resource) {
                return Err(CfgError::DuplicateResource(*resource));
            }
        }
        let mut block_map = BTreeMap::new();
        for block in blocks {
            let id = block.id;
            if block_map.insert(id, block).is_some() {
                return Err(CfgError::DuplicateBlock(id));
            }
        }
        if !block_map.contains_key(&entry) {
            return Err(CfgError::MissingBlock(entry));
        }
        for block in block_map.values() {
            for instruction in &block.instructions {
                let resource = match instruction {
                    CfgInstruction::Release(resource)
                    | CfgInstruction::TransferToResult(resource) => resource,
                };
                if !resource_set.contains(resource) {
                    return Err(CfgError::UnknownResource(*resource));
                }
            }
            match &block.terminator {
                CfgTerminator::Goto(target) => require_block(&block_map, *target)?,
                CfgTerminator::TryAcquire {
                    resource,
                    success,
                    failure,
                    ..
                } => {
                    if !resource_set.contains(resource) {
                        return Err(CfgError::UnknownResource(*resource));
                    }
                    require_block(&block_map, *success)?;
                    require_block(&block_map, *failure)?;
                }
                CfgTerminator::Check {
                    success, failure, ..
                } => {
                    require_block(&block_map, *success)?;
                    require_block(&block_map, *failure)?;
                }
                CfgTerminator::Branch {
                    then_block,
                    else_block,
                    ..
                } => {
                    require_block(&block_map, *then_block)?;
                    require_block(&block_map, *else_block)?;
                }
                CfgTerminator::ReturnOk(_) => {}
                CfgTerminator::ReturnError(tag) => {
                    if error_abi.variant_by_tag(*tag).is_none() {
                        return Err(CfgError::UnknownErrorTag(*tag));
                    }
                }
            }
        }
        Ok(Self {
            function,
            entry,
            resources,
            error_abi,
            blocks: block_map,
        })
    }

    pub fn function(&self) -> &str {
        &self.function
    }

    pub const fn entry(&self) -> BlockId {
        self.entry
    }

    pub fn resources(&self) -> &[ResourceId] {
        &self.resources
    }

    pub fn error_abi(&self) -> &ErrorAbi {
        &self.error_abi
    }

    pub fn blocks(&self) -> impl Iterator<Item = &BasicBlock> {
        self.blocks.values()
    }

    pub fn block(&self, id: BlockId) -> Option<&BasicBlock> {
        self.blocks.get(&id)
    }
}

fn require_block(blocks: &BTreeMap<BlockId, BasicBlock>, id: BlockId) -> Result<(), CfgError> {
    if blocks.contains_key(&id) {
        Ok(())
    } else {
        Err(CfgError::MissingBlock(id))
    }
}

pub(crate) fn is_c_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    matches!(chars.next(), Some(first) if first == '_' || first.is_ascii_alphabetic())
        && chars.all(|character| character == '_' || character.is_ascii_alphanumeric())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TraceEvent {
    Acquire(ResourceId),
    Release(ResourceId),
    Transfer(ResourceId),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExecutionOutcome {
    Ok {
        value: i32,
        payload: Vec<ResourceId>,
    },
    Error {
        name: String,
        tag: ErrorTag,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionObservation {
    pub outcome: ExecutionOutcome,
    pub trace: Vec<TraceEvent>,
}

pub fn interpret_cfg(cfg: &TypedCfg) -> Result<ExecutionObservation, CfgError> {
    let mut block_id = cfg.entry;
    let mut live = BTreeSet::new();
    let mut acquired = BTreeSet::new();
    let mut payload = BTreeSet::new();
    let mut trace = Vec::new();
    let mut steps = 0usize;
    let step_limit = cfg.blocks.len().saturating_mul(8).max(32);

    loop {
        steps = steps.checked_add(1).ok_or(CfgError::StepLimitExceeded)?;
        if steps > step_limit {
            return Err(CfgError::StepLimitExceeded);
        }
        let block = cfg
            .block(block_id)
            .ok_or(CfgError::MissingBlock(block_id))?;
        for instruction in &block.instructions {
            match *instruction {
                CfgInstruction::Release(resource) => {
                    if !live.remove(&resource) {
                        return Err(CfgError::ReleaseOfNonLiveResource(resource));
                    }
                    trace.push(TraceEvent::Release(resource));
                }
                CfgInstruction::TransferToResult(resource) => {
                    if !live.remove(&resource) || !payload.insert(resource) {
                        return Err(CfgError::TransferOfNonLiveResource(resource));
                    }
                    trace.push(TraceEvent::Transfer(resource));
                }
            }
        }
        match block.terminator() {
            CfgTerminator::Goto(target) => block_id = *target,
            CfgTerminator::TryAcquire {
                resource,
                succeeds,
                success,
                failure,
            } => {
                let resource = *resource;
                if *succeeds {
                    if !acquired.insert(resource) {
                        return Err(CfgError::DuplicateAcquisitionIdentity(resource));
                    }
                    live.insert(resource);
                    trace.push(TraceEvent::Acquire(resource));
                    block_id = *success;
                } else {
                    block_id = *failure;
                }
            }
            CfgTerminator::Check {
                passes,
                success,
                failure,
            } => block_id = if *passes { *success } else { *failure },
            CfgTerminator::Branch {
                condition,
                then_block,
                else_block,
            } => {
                block_id = if condition.evaluate() {
                    *then_block
                } else {
                    *else_block
                };
            }
            CfgTerminator::ReturnOk(value) => {
                let value = *value;
                if !live.is_empty() {
                    return Err(CfgError::LiveResourcesAtExit(live.into_iter().collect()));
                }
                return Ok(ExecutionObservation {
                    outcome: ExecutionOutcome::Ok {
                        value,
                        payload: payload.into_iter().collect(),
                    },
                    trace,
                });
            }
            CfgTerminator::ReturnError(tag) => {
                let tag = *tag;
                if !live.is_empty() {
                    return Err(CfgError::LiveResourcesAtExit(live.into_iter().collect()));
                }
                if !payload.is_empty() {
                    return Err(CfgError::PayloadOnErrorExit(payload.into_iter().collect()));
                }
                let variant = cfg
                    .error_abi
                    .variant_by_tag(tag)
                    .ok_or(CfgError::UnknownErrorTag(tag))?;
                return Ok(ExecutionObservation {
                    outcome: ExecutionOutcome::Error {
                        name: variant.name.clone(),
                        tag,
                    },
                    trace,
                });
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CfgError {
    EmptyErrorAbi,
    TooManyErrors,
    InvalidCIdentifier(String),
    DuplicateError(String),
    DuplicateResource(ResourceId),
    DuplicateBlock(BlockId),
    MissingBlock(BlockId),
    UnknownResource(ResourceId),
    UnknownErrorTag(ErrorTag),
    DuplicateAcquisitionIdentity(ResourceId),
    ReleaseOfNonLiveResource(ResourceId),
    TransferOfNonLiveResource(ResourceId),
    LiveResourcesAtExit(Vec<ResourceId>),
    PayloadOnErrorExit(Vec<ResourceId>),
    StepLimitExceeded,
}

impl fmt::Display for CfgError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for CfgError {}
