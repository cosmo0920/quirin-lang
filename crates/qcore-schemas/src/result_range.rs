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

//! Card **CELL-GUARD-RESULT-RANGE-CHECKED** (contract specification C09.4,
//! gateway rule 38e): a pin-derived integer result set as the resource
//! environment carries it to rule 38's result-guard reading.

use serde::{Deserialize, Serialize};

/// How a derived result set stands on the route that installed it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ResultRangeStanding {
    /// Pinned in force, its derivation current: the only standing read.
    InForce,
    /// Held: a review marker, a pin not in force, or a derivation whose C
    /// library sets changed.
    Held,
    /// Declared and pinned but never derived (no derivation in its lock): an
    /// unchecked range, never read by rule 38.
    Unchecked,
}

/// One function's derived result set.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResultRangeFact {
    /// Sorted, disjoint, non-adjacent closed intervals.
    pub intervals: Vec<(i64, i64)>,
    /// The universe of the declared result type.
    pub universe: (i64, i64),
    pub standing: ResultRangeStanding,
    /// Why it is not in force, or the derivation it rests on.
    pub detail: String,
}

/// Card CELL-RESULT-SET-WRITTEN-SET-DERIVATION (rule 38e as amended): the
/// route table's key of a pinned declaration's derived written set,
/// `FUNCTION#P[K]==C` (`P` the parameter's position, `K` the element, `C` the
/// class of `when #result == C`). Rule 38f's creation check reads a slot
/// cell's `#fresh` state under `when #result == C` through it.
#[must_use]
pub fn written_set_key(function: &str, parameter: usize, index: u32, class: i64) -> String {
    format!("{function}#{parameter}[{index}]=={class}")
}
