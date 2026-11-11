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

//! Gateway rule 35 (`docs/agents/rules/35.md`; card W48-BU-SINGLE-LEAF-BODIES,
//! moved into its own rule package by card RULE35-PILOT, the first pilot of
//! GATEWAY-RULE-API-PILOT-MIGRATIONS).
//!
//! Two arms, each registered in `crates/gateway-rules/registry.toml` under
//! rule id `35`, each amending one predicate of the host:
//!
//! * [`literal_read::LiteralRead`], the classification arm `literal-read`:
//!   a `*` applied to a string literal that is only read is not an access to
//!   the lexical access gate.
//! * [`array_member_decay::ArrayMemberDecay`], the typed-reader arm
//!   `array-member-decay`: an array member decayed to a pointer at a call
//!   actual is the address of its initial element, rule 14c's `&PATH[0]`.
//!
//! The package reads the host only through the rule API's capabilities
//! ([`qcore_gateway_rule_api::LexicalHost`],
//! [`qcore_gateway_rule_api::TypedHost`]) and decides nothing else. Its
//! switch is the host's: with `35` in `QCORE_GATEWAY_RULES_OFF` neither arm
//! is asked and each predicate keeps its old refusal. The Lean model is
//! `QCore.LiteralDecayReadings`, whose decision vectors are this crate's
//! `DECISION_VECTORS` tables.

pub mod array_member_decay;
pub mod literal_read;

pub use array_member_decay::ArrayMemberDecay;
pub use literal_read::LiteralRead;

/// The package's registry id.
pub const ID: &str = "35";

#[cfg(test)]
mod toy;
