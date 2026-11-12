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

//! Card **MEASURE-LOOKUP-RECORD-REUSE** (R8 option A): the gateway's
//! per-unit lookup record.
//!
//! A consumer analysing one unit opens a [`LookupSession`] over the
//! environment it would pass the gateway and passes the session's
//! environment instead: a copy with a lookup recorder attached
//! ([`qcore_schemas::contract_lookup_record`]), which answers every read
//! exactly as the original and records every read of itself and of every
//! clone the analysis makes, on any thread. While [`LookupSession::scoped`]
//! runs, every decided callback candidate the analysis relies on
//! (`contract_dependencies::relied`, card HANDOFF-CANDIDATE-OWN-CALLBACKS) is
//! noted too. [`LookupSession::record`] then publishes the
//! [`UnitLookupRecord`]: every name read, found and missed, the relied
//! candidates, and whether any read covered a whole table (with the
//! accessors that did).
//!
//! The record is checked ([`UnitLookupRecord::check`]) before it is used: a
//! name recorded as found must be held by the environment it was recorded
//! under, a relied candidate must have been read by name, and the whole-map
//! flag must agree with the accessor list. A record that fails the check, or
//! that has a whole-map read, cannot key reuse on the names it read
//! ([`restricted_sha256`] answers `None`), so a consumer falls back to the
//! whole environment (fail-closed).
//!
//! The record is evidence only: nothing in the analysis reads it, and a unit
//! analysed without a session analyses exactly as before.

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::sync::{Arc, Mutex, PoisonError};

use qcore_schemas::contract::ResourceContracts;
use qcore_schemas::contract_lookup_record::LookupRecorder;
use serde::{Deserialize, Serialize};

/// What one unit's analysis read of its environment.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnitLookupRecord {
    /// Names whose read found an entry.
    pub found: BTreeSet<String>,
    /// Names whose read found none.
    pub missed: BTreeSet<String>,
    /// Decided callback candidates the analysis relied on.
    pub relied: BTreeSet<String>,
    /// Some read covered more than named entries.
    pub whole_map_read: bool,
    /// The accessors that did.
    pub whole_map_reads: BTreeSet<String>,
}

impl UnitLookupRecord {
    /// Every name the record keys on: found, missed and relied.
    #[must_use]
    pub fn closure(&self) -> BTreeSet<String> {
        self.found
            .iter()
            .chain(&self.missed)
            .chain(&self.relied)
            .cloned()
            .collect()
    }

    /// Check the record against the environment it was recorded under.
    ///
    /// # Errors
    ///
    /// The first inconsistency: a whole-map flag that disagrees with its
    /// accessor list, a found name the environment does not hold, or a relied
    /// candidate that was never read by name.
    pub fn check(&self, environment: &ResourceContracts) -> Result<(), String> {
        if self.whole_map_read == self.whole_map_reads.is_empty() {
            return Err("the whole-map flag disagrees with its accessor list".into());
        }
        if let Some(name) = self
            .found
            .iter()
            .find(|name| !environment.holds_name(name))
        {
            return Err(format!(
                "`{name}` is recorded as found but the environment holds no entry for it"
            ));
        }
        if let Some(name) = self
            .relied
            .iter()
            .find(|name| !self.found.contains(*name) && !self.missed.contains(*name))
        {
            return Err(format!(
                "relied candidate `{name}` was never read by name, so its lookups cannot be enumerated"
            ));
        }
        Ok(())
    }
}

/// SHA-256 of `environment` restricted to the union of `records`' closures,
/// when every record checks against `checked_under` and none read a whole
/// table; `None` otherwise (the consumer then keys on the whole
/// environment).
///
/// # Errors
///
/// An entry that does not serialize.
pub fn restricted_sha256(
    records: &[UnitLookupRecord],
    checked_under: &ResourceContracts,
    environment: &ResourceContracts,
) -> Result<Option<String>, String> {
    if records
        .iter()
        .any(|record| record.whole_map_read || record.check(checked_under).is_err())
    {
        return Ok(None);
    }
    let names = records
        .iter()
        .flat_map(UnitLookupRecord::closure)
        .collect::<BTreeSet<_>>();
    environment.restricted_sha256(&names).map(Some)
}

/// One unit's recording session.
pub struct LookupSession {
    environment: ResourceContracts,
    recorder: Arc<LookupRecorder>,
    relied: Arc<Mutex<BTreeSet<String>>>,
}

thread_local! {
    static RELIED: RefCell<Option<Arc<Mutex<BTreeSet<String>>>>> = const { RefCell::new(None) };
}

/// Note a decided callback candidate the current work relied on, into the
/// session scoped on this thread (if any).
pub(crate) fn relied(name: &str) {
    RELIED.with(|slot| {
        if let Some(relied) = slot.borrow().as_ref() {
            relied
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .insert(name.to_owned());
        }
    });
}

impl LookupSession {
    /// A session over a recording copy of `environment`.
    #[must_use]
    pub fn open(environment: &ResourceContracts) -> Self {
        let (environment, recorder) = environment.with_lookup_recorder();
        Self {
            environment,
            recorder,
            relied: Arc::default(),
        }
    }

    /// The recording environment to analyse under.
    #[must_use]
    pub fn environment(&self) -> &ResourceContracts {
        &self.environment
    }

    /// Run `work` with this session's relied candidates noted on this thread.
    pub fn scoped<T>(&self, work: impl FnOnce() -> T) -> T {
        struct Restore(Option<Arc<Mutex<BTreeSet<String>>>>);
        impl Drop for Restore {
            fn drop(&mut self) {
                let previous = self.0.take();
                RELIED.with(|slot| *slot.borrow_mut() = previous);
            }
        }
        let previous = RELIED.with(|slot| slot.borrow_mut().replace(self.relied.clone()));
        let _restore = Restore(previous);
        work()
    }

    /// The record so far (reads made later, by a clone that outlives the
    /// unit, are recorded too: take the record when every consumer of the
    /// unit's products is done).
    #[must_use]
    pub fn record(&self) -> UnitLookupRecord {
        record_of(&self.recorder, &self.relied)
    }

    /// Drop the session's own copy of the environment and keep what records:
    /// a clone the unit's products still hold keeps recording into it.
    #[must_use]
    pub fn into_handle(self) -> LookupHandle {
        LookupHandle {
            recorder: self.recorder,
            relied: self.relied,
        }
    }
}

/// A finished session's recorder, without the environment copy.
pub struct LookupHandle {
    recorder: Arc<LookupRecorder>,
    relied: Arc<Mutex<BTreeSet<String>>>,
}

impl LookupHandle {
    /// The record so far.
    #[must_use]
    pub fn record(&self) -> UnitLookupRecord {
        record_of(&self.recorder, &self.relied)
    }
}

fn record_of(recorder: &LookupRecorder, relied: &Mutex<BTreeSet<String>>) -> UnitLookupRecord {
    let log = recorder.snapshot();
    UnitLookupRecord {
        found: log.found,
        missed: log.missed,
        relied: relied.lock().unwrap_or_else(PoisonError::into_inner).clone(),
        whole_map_read: !log.whole_map_reads.is_empty(),
        whole_map_reads: log.whole_map_reads,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qcore_schemas::contract::ResourceApiKind;

    fn environment() -> ResourceContracts {
        ResourceContracts::new([
            ("free", ResourceApiKind::Release),
            ("malloc", ResourceApiKind::Acquire),
        ])
    }

    #[test]
    fn a_session_records_named_reads_relied_candidates_and_whole_map_reads() {
        let base = environment();
        let session = LookupSession::open(&base);
        session.scoped(|| {
            assert!(session.environment().contract("free").is_some());
            assert!(session.environment().contract("cb").is_none());
            relied("cb");
        });
        // Outside the scope a relied note goes nowhere.
        relied("elsewhere");
        let record = session.record();
        assert_eq!(record.found, BTreeSet::from(["free".to_owned()]));
        assert!(record.missed.contains("cb"));
        assert_eq!(record.relied, BTreeSet::from(["cb".to_owned()]));
        assert!(!record.whole_map_read);
        record.check(&base).unwrap();
        let digest = restricted_sha256(std::slice::from_ref(&record), &base, &base)
            .unwrap()
            .unwrap();
        // An entry outside the closure does not move it; one inside does.
        let mut outside = environment();
        outside.insert("malloc", ResourceApiKind::Neutral);
        assert_eq!(
            restricted_sha256(std::slice::from_ref(&record), &base, &outside).unwrap(),
            Some(digest.clone())
        );
        let mut inside = environment();
        inside.insert("cb", ResourceApiKind::Release);
        assert_ne!(
            restricted_sha256(std::slice::from_ref(&record), &base, &inside).unwrap(),
            Some(digest)
        );
        // A whole-map read keys on nothing.
        let _ = session.environment().functions();
        let whole = session.record();
        assert!(whole.whole_map_read);
        assert_eq!(
            whole.whole_map_reads,
            BTreeSet::from(["functions".to_owned()])
        );
        assert_eq!(restricted_sha256(&[whole], &base, &base).unwrap(), None);
    }

    #[test]
    fn a_record_that_does_not_check_keys_on_nothing() {
        let base = environment();
        let forged_found = UnitLookupRecord {
            found: BTreeSet::from(["realloc".to_owned()]),
            ..UnitLookupRecord::default()
        };
        assert!(forged_found.check(&base).is_err());
        let unread_relied = UnitLookupRecord {
            relied: BTreeSet::from(["cb".to_owned()]),
            ..UnitLookupRecord::default()
        };
        assert!(unread_relied.check(&base).is_err());
        let flag = UnitLookupRecord {
            whole_map_read: true,
            ..UnitLookupRecord::default()
        };
        assert!(flag.check(&base).is_err());
        for record in [forged_found, unread_relied, flag] {
            assert_eq!(restricted_sha256(&[record], &base, &base).unwrap(), None);
        }
    }
}
