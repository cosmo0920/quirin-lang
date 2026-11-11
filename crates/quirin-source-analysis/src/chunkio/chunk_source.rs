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

//! Compatibility facade for checked Boolean-field source effects.
//!
//! Typed import lives in the C gateway. Default profile/revision selection and
//! binding a checked effect to the core lifecycle model remain in this adapter.

use super::PINNED_FLUENT_BIT_REVISION;
use crate::chunk_lifecycle::{ChunkId, ChunkOperation, ChunkState};
use crate::legacy_c::C11TranslationUnit;
pub use gateway::{ChunkFrameSummary, ChunkSourceField, ChunkSourcePath};
use qcore_c_gateway::boolean_source as gateway;
use qcore_contract::profile_loader::LoadedProfile;
use qcore_schemas::chunk_summary::ChunkSourceSummaryRecord;
use serde::Serialize;
use std::collections::BTreeMap;

/// Constructed only by the typed source importer. The schema record alone
/// cannot be promoted into this source-checked wrapper.
///
/// ```compile_fail
/// use quirin_source_analysis::chunkio::chunk_source::ChunkSourceSummary;
/// use qcore_schemas::chunk_summary::ChunkSourceSummaryRecord;
/// fn forge(record: ChunkSourceSummaryRecord) -> ChunkSourceSummary { record.into() }
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct ChunkSourceSummary(gateway::ChunkSourceSummary);

impl std::ops::Deref for ChunkSourceSummary {
    type Target = ChunkSourceSummaryRecord;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ChunkSourceSummary {
    /// Borrow the gateway's opaque source-checked result for trusted replay.
    pub fn source_checked(&self) -> &gateway::ChunkSourceSummary {
        &self.0
    }

    pub fn function(&self) -> &str {
        &self.function
    }

    pub fn source_revision(&self) -> &str {
        &self.source_revision
    }

    pub fn field(&self) -> ChunkSourceField {
        self.field
    }

    pub fn paths(&self) -> &[ChunkSourcePath; 2] {
        &self.paths
    }

    pub fn dependencies(&self) -> &BTreeMap<String, String> {
        &self.dependencies
    }

    pub fn profile_assumptions(&self) -> &BTreeMap<String, String> {
        &self.profile_assumptions
    }

    /// Bind the source parameter to one live model identity. The caller must
    /// establish that its C pointer denotes this chunk, has exclusive access,
    /// and that the concrete field equals this model state's Boolean value.
    /// A declared record/field mapping alone proves none of those premises.
    /// This does not infer pointer provenance or synchronization.
    /// Non-lifecycle helper writes remain in paths/frame_writes and the native
    /// contract certificate; ChunkState does not model error/metadata storage.
    pub fn instantiate(
        &self,
        state: &ChunkState,
        chunk: ChunkId,
    ) -> Result<(i32, Vec<ChunkOperation>), String> {
        let record = state
            .chunks
            .get(&chunk)
            .filter(|record| record.live)
            .ok_or_else(|| format!("source call requires live chunk {chunk}"))?;
        let value = match self.field {
            ChunkSourceField::Busy => record.busy,
            ChunkSourceField::Locked => record.locked,
        };
        let path = &self.paths[usize::from(value)];
        let mut previous = value;
        let operations = path
            .writes
            .iter()
            .filter(|value| {
                let changed = previous != **value;
                previous = **value;
                changed
            })
            .map(|value| match self.field {
                ChunkSourceField::Busy => ChunkOperation::SetBusy {
                    chunk,
                    busy: *value,
                },
                ChunkSourceField::Locked if *value => ChunkOperation::Lock { chunk },
                ChunkSourceField::Locked => ChunkOperation::Unlock { chunk },
            })
            .collect();
        Ok((path.return_value, operations))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum ChunkSourceResult {
    /// A complete field-effect summary, not a Safe translation-unit verdict.
    Summary(Box<ChunkSourceSummary>),
    Unknown {
        function: String,
        reason: String,
        /// Mirrors the gateway row's closed §L21.1 category
        /// (`qcore_c_gateway::boolean_source::ChunkSourceResult`, card
        /// REASON-CODES-B2). It is carried across this compatibility boundary
        /// rather than re-derived, so this facade cannot become a second
        /// classification of the same text.
        reason_code: Option<qcore_schemas::reason::ReasonCategory>,
    },
}

/// Report every discovered main-source function, including definitions absent
/// after preprocessing. The revision is a caller assertion; the external audit
/// additionally verifies Git identity and a clean source tree.
///
/// P09-G removed the zero-helper convenience overload `import_chunk_source`;
/// call this with an empty helper slice instead. Parity was measured first and
/// is archived in `docs/reports/remaining-p09-routes/g-route-parity.json`.
pub fn import_chunk_source_with_helpers(
    unit: &C11TranslationUnit<'_>,
    source_revision: &str,
    helpers: &[ChunkFrameSummary],
) -> Result<Vec<ChunkSourceResult>, String> {
    let profile =
        crate::source_profiles::load_profile(crate::source_profiles::FLUENT_BIT_SOURCE_SET)
            .map_err(|error| error.to_string())?;
    let libraries = profile
        .manifest()
        .libraries
        .iter()
        .filter(|(_, library)| !library.boolean_field_models.is_empty())
        .map(|(name, _)| name.as_str())
        .collect::<Vec<_>>();
    import_chunk_source_with_profile(unit, source_revision, &profile, &libraries, helpers)
}

/// Import with explicit profile selection, preserving the compatibility API.
pub fn import_chunk_source_with_profile(
    unit: &C11TranslationUnit<'_>,
    source_revision: &str,
    profile: &LoadedProfile,
    libraries: &[&str],
    helpers: &[ChunkFrameSummary],
) -> Result<Vec<ChunkSourceResult>, String> {
    gateway::import_chunk_source_with_profile(
        unit,
        source_revision,
        profile,
        libraries,
        helpers,
        crate::source_contract::RULES,
    )
    .map(|results| {
        results
            .into_iter()
            .map(|result| match result {
                gateway::ChunkSourceResult::Summary(summary) => {
                    ChunkSourceResult::Summary(Box::new(ChunkSourceSummary(*summary)))
                }
                gateway::ChunkSourceResult::Unknown {
                    function,
                    reason,
                    reason_code,
                } => ChunkSourceResult::Unknown {
                    function,
                    reason,
                    reason_code,
                },
            })
            .collect()
    })
}

pub fn import_chunk_frame_helper(
    unit: &C11TranslationUnit<'_>,
    revision: &str,
    function: &str,
) -> Result<ChunkFrameSummary, String> {
    gateway::import_chunk_frame_helper(
        unit,
        revision,
        PINNED_FLUENT_BIT_REVISION,
        function,
        crate::source_contract::RULES,
    )
}

#[cfg(test)]
#[path = "../../tests/unit/chunk_source.rs"]
mod tests;
