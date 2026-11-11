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

//! Proposed unsigned unit-counter update records at the provider boundary.
//!
//! These are untrusted serializable records, not source-import certificates.
//! A receiver must establish source/build/identity binding separately, and a
//! successful comparison proves only that a producer recomputed the same
//! structural update from the supplied path. Contract evaluation, caller
//! storage, exclusive access and the unsigned-C/usize ABI binding stay with
//! the trusted core.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Required schema version of a counter source projection.
pub const COUNTER_SOURCE_INPUT_VERSION: u16 = 1;

/// Complete-input digest key bound into the verification task.
pub const COUNTER_SOURCE_INPUT_KEY: &str = "qcore:counter-source-input:v1";

/// The bounded typed expression vocabulary a counter path may use. Anything
/// else keeps its function Unknown at import; there is no fallback encoding.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CounterExpressionRecord {
    Parameter {
        index: u32,
    },
    /// A value produced by an earlier operation of this same path.
    Symbol {
        operation: u32,
    },
    /// Decimal literal text, never a host-evaluated number.
    Integer {
        value: String,
    },
    Field {
        base: Box<CounterValueRecord>,
        name: String,
    },
    Binary {
        operator: String,
        left: Box<CounterValueRecord>,
        right: Box<CounterValueRecord>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CounterValueRecord {
    pub c_type: String,
    pub expression: CounterExpressionRecord,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CounterOperationRecord {
    Read {
        place: CounterValueRecord,
    },
    Write {
        place: CounterValueRecord,
        value: CounterValueRecord,
    },
    ReturnValue {
        value: CounterValueRecord,
    },
}

/// One imported single-path counter function, without any derived result.
/// A raw record conveys no import authority: its complete digest is pinned in
/// the task, and the receiver checks a proposal against its own import.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CounterSourceInput {
    pub schema_version: u16,
    pub function: String,
    pub source_revision: String,
    /// The imported C signature, exactly as the source declared it.
    pub signature: String,
    pub root_c_type: String,
    pub operations: Vec<CounterOperationRecord>,
    pub profile_assumptions: BTreeMap<String, String>,
}

impl CounterSourceInput {
    #[must_use]
    pub fn version_matches(&self) -> bool {
        self.schema_version == COUNTER_SOURCE_INPUT_VERSION
    }

    #[must_use]
    pub const fn digest_key(&self) -> &'static str {
        COUNTER_SOURCE_INPUT_KEY
    }

    #[must_use]
    pub fn digest(&self) -> String {
        crate::digest::sha256_hex(
            &serde_json::to_vec(self).expect("serializable counter source projection"),
        )
    }
}

/// A proposed structural reading of one counter path. Equality with the core's
/// own reading binds the proposal to the import; it never discharges the
/// pinned contract, which the core evaluates separately.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CounterUpdateRecord {
    pub function: String,
    pub source_revision: String,
    pub root_c_type: String,
    pub counter_c_type: String,
    pub field: String,
    /// `1` for an increment, `-1` for a decrement. No other value is derivable.
    pub delta: i8,
    pub result_is_updated_counter: bool,
    pub only_selected_field_written: bool,
    pub profile_assumptions: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CounterContractWitness {
    pub task_digest: String,
    pub summary: CounterUpdateRecord,
}
