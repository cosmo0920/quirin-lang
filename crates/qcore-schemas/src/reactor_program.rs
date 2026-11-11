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

//! The transported projection of one admitted reactor callback family.
//!
//! This is the *request-side* counterpart of [`crate::reactor`]'s proposal
//! records: it carries the reactor program a receiver already holds, in a
//! bounded serializable vocabulary, so a separate process can compute the
//! callback assignment inventory on its own. It carries **no derived result** —
//! no proposal, no certificate, no verdict — exactly as the counter projection
//! carries a path and not a reading.
//!
//! A raw record conveys no model authority. The complete input's digest is
//! pinned into the verification task, and the receiver checks any returned
//! proposal against its own program by re-running its own model checker. The
//! declared build profile travels as opaque key/value metadata: a producer must
//! not interpret it, and a receiver binds it only through the digest, so a
//! different build retires the task rather than reusing its evidence.
//!
//! The vocabulary is deliberately narrower than the reactor model. Bufferevent
//! operations, raw socket-descriptor custody, ordering assumptions and loop
//! control have no record here, so a program that uses them cannot be projected
//! at all and its callbacks stay `Unknown`.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::reactor::{
    ActivationReasonRecord, CallbackSummaryRecord, EventKeyRecord, PayloadAccessKindRecord,
};

/// Required schema version of a reactor program projection.
pub const REACTOR_PROGRAM_INPUT_VERSION: u16 = 1;

/// Complete-input digest key bound into the verification task.
pub const REACTOR_PROGRAM_INPUT_KEY: &str = "qcore:reactor-program-input:v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum DeleteModeRecord {
    Ordinary,
    Blocking,
    Nonblocking,
}

/// The execution context an access is attributed to. `Bufferevent` has no
/// record: bufferevent custody is outside this projection's family.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CallbackContextRecord {
    Event { event: EventKeyRecord },
    Caller { context: u32 },
    Worker { context: u32 },
}

/// The bounded operation vocabulary an admitted callback family may use.
/// There is no catch-all variant and no fallback encoding: a program using
/// anything else cannot be projected.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum ReactorOperationRecord {
    /// The declared build profile is carried once, on the projection itself.
    CreateBase {
        base: u32,
    },
    BeginDispatch {
        base: u32,
        dispatcher: u32,
    },
    EndDispatch {
        base: u32,
    },
    /// Callback registration: the event identity carries its generation, the
    /// descriptor is the event's borrowed descriptor custody, and the summary
    /// is the declared callback authority.
    Assign {
        event: EventKeyRecord,
        base: u32,
        descriptor: Option<i32>,
        flags: u16,
        priority: u8,
        summary: CallbackSummaryRecord,
    },
    Add {
        event: EventKeyRecord,
    },
    Reschedule {
        event: EventKeyRecord,
    },
    Activate {
        event: EventKeyRecord,
        reason: ActivationReasonRecord,
        cross_thread: bool,
    },
    CallbackEnter {
        event: EventKeyRecord,
        reason: ActivationReasonRecord,
    },
    CallbackExit {
        event: EventKeyRecord,
    },
    /// Callback deregistration.
    Delete {
        event: EventKeyRecord,
        mode: DeleteModeRecord,
    },
    Finalize {
        event: EventKeyRecord,
    },
    FreeEvent {
        event: EventKeyRecord,
    },
    Access {
        context: CallbackContextRecord,
        payload: u32,
        kind: PayloadAccessKindRecord,
        synchronized: bool,
    },
    DropArgument {
        argument: u32,
    },
    /// Parent custody: the base outlives every event assigned to it.
    FreeBase {
        base: u32,
    },
}

/// One projected reactor program, without any derived result.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReactorProgramInput {
    pub schema_version: u16,
    pub library: String,
    pub source_revision: String,
    /// Declared build attributes, opaque to a producer and bound by digest.
    pub build_profile: BTreeMap<String, String>,
    pub live_arguments: Vec<u32>,
    pub operations: Vec<ReactorOperationRecord>,
    pub profile_assumptions: BTreeMap<String, String>,
}

impl ReactorProgramInput {
    #[must_use]
    pub fn version_matches(&self) -> bool {
        self.schema_version == REACTOR_PROGRAM_INPUT_VERSION
    }

    #[must_use]
    pub const fn digest_key(&self) -> &'static str {
        REACTOR_PROGRAM_INPUT_KEY
    }

    #[must_use]
    pub fn digest(&self) -> String {
        crate::digest::sha256_hex(
            &serde_json::to_vec(self).expect("serializable reactor program projection"),
        )
    }
}

/// A proposed callback assignment inventory for one projected program.
///
/// Equality with the inventory the receiver exports from its own program binds
/// the proposal to that program; it never discharges the model obligations,
/// which the receiver's own checker decides separately.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReactorCallbackWitness {
    pub task_digest: String,
    pub proposal: crate::reactor::ReactorCallbackProposal,
}
