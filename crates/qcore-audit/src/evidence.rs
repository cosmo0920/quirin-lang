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

//! The evidence an audit admits (card **W44-QUIRIN-AUDIT-PROVIDERS**).
//!
//! `contracts-only` is the route that audits using only Quirin contracts: the
//! resource projection from `.qcore` modules elaborated from the selected
//! providers (or a manifest), and nothing else — no process packages or
//! source-contract checks, no verifier-host execution, no compiler-crate
//! verifier-plugin evidence, no native or observed evidence and no pinned
//! historical evidence. A configuration that also names any of those refuses
//! rather than being silently narrowed, and the mode, with every contract
//! module a verdict rests on, is written into the report and the sidecar so a
//! contracts-only verdict is distinguishable from a full audit's.
//!
//! Lean model: `proof/QCore/ProviderSelection.lean` (`contractsOnly_*`).

use qcore_schemas::contract::ResourceContracts;
use qcore_schemas::sha256_hex;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// Which evidence a route admits.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum EvidenceMode {
    /// The route's own evidence, as before this card.
    #[default]
    Configured,
    /// Resource projection from contract modules only.
    ContractsOnly,
}

impl EvidenceMode {
    #[must_use]
    pub fn is_contracts_only(self) -> bool {
        self == Self::ContractsOnly
    }
}

/// The evidence kinds a contracts-only run never admits.
pub const EXCLUDED_EVIDENCE: [&str; 5] = [
    "process packages and source-contract checks",
    "verifier-host execution",
    "compiler-crate verifier-plugin evidence",
    "native or observed evidence",
    "pinned historical evidence",
];

/// The record written into the report and the sidecar of a contracts-only run:
/// the mode, what it excludes and every contract module (its retained,
/// elaborated text's SHA-256) that the projection read.
#[must_use]
pub fn contracts_only_record(contracts: &ResourceContracts) -> Value {
    json!({
        "mode": "contracts-only",
        "admitted": "resource projection from the elaborated .qcore contract modules listed here; every other obligation stays Unknown",
        "excluded": EXCLUDED_EVIDENCE,
        "contract_modules": contracts
            .source_assumptions()
            .iter()
            .map(|(module, text)| json!({"module": module, "sha256": sha256_hex(text.as_bytes())}))
            .collect::<Vec<_>>(),
        "included_parents": "modules reached only through `#include` are listed, with their digests, under the sidecar's contract_inclusion",
    })
}
