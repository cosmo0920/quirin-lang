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

//! Card **MEASURE-LOOKUP-RECORD-REUSE** (R8 option A): what one consumer
//! read of a resource environment, for reuse keyed on what was read.
//!
//! A [`LookupRecorder`] is attached to one environment instance
//! ([`crate::contract::ResourceContracts::with_lookup_recorder`]). From then
//! on every read of that instance, and of every clone of it (a clone shares
//! the recorder), is recorded, on whatever thread it happens:
//!
//! * a read of one name (`contract`, `relational_contract`,
//!   `retaining_contract`, `lifetime_contract`, `has_guarded_interface`,
//!   `revision_mode`, `lifetime_summary`, `member_effect`,
//!   `result_range_in_force`, and the name a mutation consults) records the
//!   name, as `found` when the read found an entry and `missed` otherwise;
//! * a read that can depend on more than named entries (any accessor that
//!   hands out a whole map or iterates one, serialization, `Debug`, equality,
//!   a copy from another environment) records a **whole-map read**, by the
//!   accessor's name.
//!
//! Unlike the rule 13f consultation log
//! ([`crate::contract_consultation`]), the record is never rolled back (a
//! declined probe still read what it read) and is not tied to a thread, so a
//! read on a helper thread the consumer spawns is recorded too.
//!
//! The record is evidence for reuse only: nothing in a lookup reads it, so an
//! environment with a recorder answers every read exactly as one without, and
//! the recorder is invisible to serialization, `Debug` output and equality.
//!
//! **The reuse premise.** A deterministic consumer that recorded no
//! whole-map read made every decision from the entries of the names it
//! recorded and from the environment's scalars. Another environment that
//! agrees with this one on those entries and scalars
//! ([`crate::contract::ResourceContracts::restricted_sha256`]) therefore
//! leads it through the same reads to the same result
//! (`QCore.LookupRestrictedReuse.run_eq_of_agree`).

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex, PoisonError};

use serde::{Deserialize, Serialize};

/// What a recorder saw.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LookupLog {
    /// Names whose read found an entry.
    pub found: BTreeSet<String>,
    /// Names whose read found none.
    pub missed: BTreeSet<String>,
    /// The accessors that read more than named entries; empty when every
    /// read was of one name.
    pub whole_map_reads: BTreeSet<String>,
}

impl LookupLog {
    /// Every name read, found or missed.
    #[must_use]
    pub fn names(&self) -> BTreeSet<String> {
        self.found.union(&self.missed).cloned().collect()
    }
}

/// The shared log of one environment instance and its clones.
#[derive(Debug, Default)]
pub struct LookupRecorder {
    log: Mutex<LookupLog>,
}

impl LookupRecorder {
    /// A fresh recorder.
    #[must_use]
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    fn with<T>(&self, update: impl FnOnce(&mut LookupLog) -> T) -> T {
        update(&mut self.log.lock().unwrap_or_else(PoisonError::into_inner))
    }

    /// Record a read of `name`.
    pub fn name(&self, name: &str, found: bool) {
        self.with(|log| {
            let set = if found {
                &mut log.found
            } else {
                &mut log.missed
            };
            if !set.contains(name) {
                set.insert(name.to_owned());
            }
        });
    }

    /// Record a read of more than named entries, by `accessor`.
    pub fn whole(&self, accessor: &str) {
        self.with(|log| {
            if !log.whole_map_reads.contains(accessor) {
                log.whole_map_reads.insert(accessor.to_owned());
            }
        });
    }

    /// A copy of what was recorded so far.
    #[must_use]
    pub fn snapshot(&self) -> LookupLog {
        self.with(|log| log.clone())
    }
}

/// The recorder slot of an environment: invisible to serialization (always
/// skipped), equal to every other slot, empty in `Default`. Its clone shares
/// the recorder, so reads of a derived environment are the original's.
#[derive(Clone, Default)]
pub struct RecorderSlot(Option<Arc<LookupRecorder>>);

impl RecorderSlot {
    /// A slot holding `recorder`.
    #[must_use]
    pub fn holding(recorder: Arc<LookupRecorder>) -> Self {
        Self(Some(recorder))
    }

    /// Record a read of `name`, when a recorder is attached.
    #[inline]
    pub fn name(&self, name: &str, found: bool) {
        if let Some(recorder) = &self.0 {
            recorder.name(name, found);
        }
    }

    /// Record a whole-map read by `accessor`, when a recorder is attached.
    #[inline]
    pub fn whole(&self, accessor: &str) {
        if let Some(recorder) = &self.0 {
            recorder.whole(accessor);
        }
    }

    /// Serialization hook (`skip_serializing_if`): serializing the
    /// environment reads all of it. Always skips the slot itself.
    #[must_use]
    pub fn serialized(&self) -> bool {
        self.whole("serialize");
        true
    }
}

/// Two environments compared read both in full; the slot is compared first
/// (the environment's first field), so the read is recorded whatever the
/// comparison's outcome.
impl PartialEq for RecorderSlot {
    fn eq(&self, other: &Self) -> bool {
        self.whole("eq");
        other.whole("eq");
        true
    }
}

impl Eq for RecorderSlot {}

impl std::fmt::Debug for RecorderSlot {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("RecorderSlot")
    }
}

/// Never written (the field is always skipped); required by the derive.
impl Serialize for RecorderSlot {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_unit()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::{ResourceApiKind, ResourceContracts};

    fn environment() -> ResourceContracts {
        ResourceContracts::new([
            ("free", ResourceApiKind::Release),
            ("malloc", ResourceApiKind::Acquire),
        ])
    }

    #[test]
    fn named_reads_are_recorded_found_or_missed_and_clones_share_the_recorder() {
        let plain = environment();
        let (recorded, recorder) = plain.with_lookup_recorder();
        assert!(recorded.contract("free").is_some());
        assert!(recorded.contract("absent").is_none());
        let derived = recorded.clone();
        assert!(derived.retaining_contract("malloc").is_none());
        let helper = std::thread::scope(|scope| {
            scope
                .spawn(|| derived.has_guarded_interface("on_thread"))
                .join()
                .unwrap()
        });
        assert!(!helper);
        let log = recorder.snapshot();
        assert_eq!(log.found, BTreeSet::from(["free".to_owned()]));
        // `contract` first asks the guarded tables, which miss `free`: a name
        // can be found in one table and missed in another.
        assert_eq!(
            log.missed,
            BTreeSet::from([
                "absent".to_owned(),
                "free".to_owned(),
                "malloc".to_owned(),
                "on_thread".to_owned()
            ])
        );
        assert!(log.whole_map_reads.is_empty());
        // An environment without a recorder records nothing anywhere.
        assert!(plain.contract("free").is_some());
        assert_eq!(recorder.snapshot(), log);
    }

    #[test]
    fn whole_map_reads_serialization_debug_and_equality_are_recorded() {
        let (recorded, recorder) = environment().with_lookup_recorder();
        let _ = recorded.functions();
        assert_eq!(
            recorder.snapshot().whole_map_reads,
            BTreeSet::from(["functions".to_owned()])
        );
        let (recorded, recorder) = environment().with_lookup_recorder();
        let bytes = serde_json::to_vec(&recorded).unwrap();
        assert_eq!(bytes, serde_json::to_vec(&environment()).unwrap());
        assert!(recorder.snapshot().whole_map_reads.contains("serialize"));
        let (recorded, recorder) = environment().with_lookup_recorder();
        assert_eq!(format!("{recorded:?}"), format!("{:?}", environment()));
        assert_eq!(format!("{recorded:#?}"), format!("{:#?}", environment()));
        assert!(recorder.snapshot().whole_map_reads.contains("debug"));
        let (recorded, recorder) = environment().with_lookup_recorder();
        let mut other = environment();
        other.insert("realloc", ResourceApiKind::Neutral);
        assert_ne!(recorded, other);
        assert!(recorder.snapshot().whole_map_reads.contains("eq"));
        let (recorded, _) = environment().with_lookup_recorder();
        assert_eq!(recorded, environment());
    }

    #[test]
    fn the_restricted_digest_covers_exactly_the_named_entries_and_the_scalars() {
        let names = BTreeSet::from(["free".to_owned(), "absent".to_owned()]);
        let base = environment();
        let digest = base.restricted_sha256(&names).unwrap();
        // An entry outside the names does not move it.
        let mut other = environment();
        other.insert("realloc", ResourceApiKind::Neutral);
        other.insert("malloc", ResourceApiKind::Neutral);
        assert_eq!(other.restricted_sha256(&names).unwrap(), digest);
        // A changed, added or removed entry of a name moves it.
        let mut changed = environment();
        changed.insert("free", ResourceApiKind::Neutral);
        assert_ne!(changed.restricted_sha256(&names).unwrap(), digest);
        let mut added = environment();
        added.insert("absent", ResourceApiKind::Release);
        assert_ne!(added.restricted_sha256(&names).unwrap(), digest);
        // A scalar moves it whatever the names.
        let mut scalar = environment();
        scalar.set_link_resolution_sha256(Some("00".into()));
        assert_ne!(scalar.restricted_sha256(&names).unwrap(), digest);
        // Computing it is not a read.
        let (recorded, recorder) = environment().with_lookup_recorder();
        assert_eq!(recorded.restricted_sha256(&names).unwrap(), digest);
        assert_eq!(recorder.snapshot(), LookupLog::default());
    }
}
