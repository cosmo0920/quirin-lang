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

//! Gateway rule 32a (`docs/agents/rules/32a.md`; card
//! W48-RK-LOG-CALLER-FRAME, moved into its own rule package by card
//! RULE32A-PILOT, the last pilot of GATEWAY-RULE-API-PILOT-MIGRATIONS).
//!
//! One placement, registered in `crates/gateway-rules/registry.toml` under
//! rule id `32a`: [`hold::CallerFrameHold`], the token hold
//! `caller-frame-hold` (route map §1). A caller-frame declaration (rule 32)
//! serves only callers outside its frame: inside the body of a member of a
//! registered caller frame, a call to a member of the same frame (the
//! member's own recursive call included) whose declaration is in force is
//! held, and the host refuses the held call `Unknown` under the reason key
//! this package reserves, [`hold::SELF_RELIANCE`]
//! (`caller-frame-self-reliance`, category `incomplete-obligation-coverage`).
//!
//! The package decides only which calls are held, from what its host
//! ([`hold::FrameHost`]) reads: the registered frames (the reviewed
//! projections' declarations) and whether a callee's declaration is in force.
//! Which tokens are calls, which function's body holds one, the held spelling
//! and the refusal text stay the host's (`legacy_caller_frame_hold`).
//!
//! **Not switchable.** The hold is a restriction: its admission is the
//! refusal of a reading a declaration would otherwise give. A switch that
//! removed it would let a member's row read its own frame's declaration,
//! which is the halted regression rule 32a exists to refuse, so the registry
//! row is `off = "not-switchable"`, `QCORE_GATEWAY_RULES_OFF` naming `32a` is
//! refused, and the host asks the package under every switch list (the
//! decision is recorded in the pluggable-cards plan, §6 item 10). Lean:
//! `QCore.RkLogCallerFrame` (the hold's reading) and
//! `QCore.CallerFrameHoldSwitch` (this package's decision table, which is
//! [`hold::DECISION_VECTORS`], and the fixed switch).

pub mod hold;

pub use hold::{CallerFrameHold, FrameHost, SELF_RELIANCE};

/// The package's registry id.
pub const ID: &str = "32a";
