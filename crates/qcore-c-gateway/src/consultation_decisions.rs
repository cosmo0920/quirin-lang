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

//! Card **GAPO-PARITY-DECISIONS-NONDETERMINISTIC** (rule 12q): the parity
//! decisions of one typed access-path consultation, owned by the consumer
//! that asked it.
//!
//! A typed access-path reader decides places (rule 10t's tracked-root place,
//! rule 12j's record-object place) and classifies pointer-valued stores
//! (rule 10v). Parity runs (`QCORE_TYPED_PATH_PARITY`) record those
//! decisions beside the consultation's answer. They used to go into two
//! thread-local lists that only the recording consumers took, and only after
//! their own node matched: a reader built by a consumer that records nothing
//! left its decisions on the worker thread, and the next recording
//! consultation whose node did not match wrote them as its own row, so the
//! row depended on which units the scheduler gave the thread.
//!
//! Here the decisions live in a [`Decisions`] value on the consumer's stack.
//! A reader writes into one only when its consumer handed it one
//! ([`crate::typed_access_path::Reader::recording`]); every other reader
//! keeps nothing. A consultation therefore starts with an empty list (the
//! value is fresh) and ends with it dropped or recorded on every exit path,
//! early returns and declined probes included, by construction: no state
//! outlives the consumer's frame. Nothing here is read by an analysis
//! decision; the rows are evidence only.

use std::cell::RefCell;

use serde_json::Value;

/// The place decisions and pointer-store classifications one consultation
/// made, in the order its reader made them.
#[derive(Debug)]
pub(crate) struct Decisions {
    /// Whether decisions are kept: on parity runs only.
    kept: bool,
    places: RefCell<Vec<Value>>,
    classifications: RefCell<Vec<Value>>,
}

impl Decisions {
    /// An empty record: one per consultation. It keeps decisions only on
    /// parity runs.
    pub(crate) fn new() -> Self {
        Self {
            kept: crate::typed_access_path::parity_enabled(),
            places: RefCell::default(),
            classifications: RefCell::default(),
        }
    }

    /// Test builds: an empty record that keeps decisions without a parity
    /// stream, so a test can read what a consultation recorded.
    #[cfg(test)]
    pub(crate) fn kept() -> Self {
        Self {
            kept: true,
            ..Self::new()
        }
    }

    /// One place decision (rules 10t and 12j), kept only on parity runs.
    pub(crate) fn place(&self, row: impl FnOnce() -> Value) {
        if self.kept {
            self.places.borrow_mut().push(row());
        }
    }

    /// One pointer-store classification (rule 10v), kept only on parity runs.
    pub(crate) fn classification(&self, row: impl FnOnce() -> Value) {
        if self.kept {
            self.classifications.borrow_mut().push(row());
        }
    }

    /// One `typed-place` row for every place decision this consultation made
    /// at `position`, with its answer; nothing when it made none.
    pub(crate) fn record_places(
        &self,
        function: &str,
        line: usize,
        texts: &[&str],
        position: &str,
        admitted: bool,
    ) {
        let places = std::mem::take(&mut *self.places.borrow_mut());
        if places.is_empty() || !crate::typed_access_path::parity_enabled() {
            return;
        }
        crate::typed_access_path::record_row(serde_json::json!({
            "function": function,
            "line": line,
            "texts": texts,
            "position": "typed-place",
            "of_position": position,
            "old_route": "old_refusal",
            "old_admitted": false,
            "new_admitted": admitted,
            "places": places,
            "disagreement": admitted,
        }));
    }

    /// One `pointer-store` row for the classifications this consultation
    /// made; nothing when it made none.
    pub(crate) fn record_classifications(&self, function: &str, line: usize, texts: &[&str]) {
        let classifications = std::mem::take(&mut *self.classifications.borrow_mut());
        if classifications.is_empty() || !crate::typed_access_path::parity_enabled() {
            return;
        }
        crate::typed_access_path::record_row(serde_json::json!({
            "function": function,
            "line": line,
            "texts": texts,
            "position": "pointer-store",
            "classifications": classifications,
        }));
    }

    /// Test builds: the place decisions held so far.
    #[cfg(test)]
    pub(crate) fn places(&self) -> Vec<Value> {
        self.places.borrow().clone()
    }

    /// Test builds: the classifications held so far.
    #[cfg(test)]
    pub(crate) fn classifications(&self) -> Vec<Value> {
        self.classifications.borrow().clone()
    }
}

#[cfg(test)]
#[path = "../tests/unit/consultation_decisions.rs"]
mod tests;
