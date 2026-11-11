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

//! Proposed callback summaries at the reactor adapter boundary.
//!
//! These records are untrusted data, not source-import certificates or admitted
//! protocol witnesses. The consumer must check the version and bind every record
//! to its own assignment inventory and declared callback authority. Lists retain
//! duplicate records, triggers and payload permissions so the adapter can reject
//! ambiguity instead of silently accepting a deduplicated proposal.

use serde::{Deserialize, Serialize};

/// Additive record vocabulary version; plugin witness admission is unchanged.
pub const REACTOR_CALLBACK_PROPOSAL_VERSION: u16 = 1;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReactorCallbackProposal {
    pub schema_version: u16,
    pub callbacks: Vec<ReactorCallbackRecord>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReactorCallbackRecord {
    /// Position in the consumer's complete operation inventory.
    pub operation_index: u64,
    pub event: EventKeyRecord,
    pub summary: CallbackSummaryRecord,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventKeyRecord {
    pub id: u32,
    pub generation: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CallbackSummaryRecord {
    pub argument: u32,
    pub argument_mode: ArgumentModeRecord,
    pub trigger_reasons: Vec<ActivationReasonRecord>,
    pub payload_authority: Vec<PayloadAuthorityRecord>,
    pub may_defer: bool,
    pub may_spawn_tasks: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum ArgumentModeRecord {
    Owned,
    Borrowed,
    Shared,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum ActivationReasonRecord {
    Read,
    Write,
    Timeout,
    Signal,
    Explicit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PayloadAuthorityRecord {
    pub payload: u32,
    pub kind: PayloadAccessKindRecord,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum PayloadAccessKindRecord {
    Read,
    Write,
    AtomicRead,
    AtomicWrite,
}
