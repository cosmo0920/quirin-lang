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

//! Card **VERDICT-CONTRACT-DEPENDENCIES** (gateway rule 13f): the names a
//! resource environment was asked for while a consumer kept a log.
//!
//! A [`crate::contract::ResourceContracts`] lookup by name
//! ([`crate::contract::ResourceContracts::contract`],
//! [`crate::contract::ResourceContracts::relational_contract`],
//! [`crate::contract::ResourceContracts::retaining_contract`]) notes the name
//! it was asked for, and whether it found a declaration, in the log of the
//! current thread when one is open. Nothing is noted when none is open, which
//! is every caller but the audit's dependency recording. The log is evidence
//! only: no lookup reads it, so it cannot change what a lookup returns, and a
//! consumer that never opens one sees exactly the environment it saw before.
//!
//! Logs nest: [`open`] starts a fresh log and hands back the enclosing one,
//! [`close`] restores it and returns what the inner log noted. A consumer that
//! reads a whole map ([`crate::contract::ResourceContracts::functions`] and the
//! side tables) and then decides about one name records it with [`note`].

use std::cell::RefCell;
use std::collections::BTreeSet;

/// What one log noted: every name looked up, by whether the lookup found a
/// declaration in the environment it was asked.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Consulted {
    /// Names whose lookup found a contract.
    pub found: BTreeSet<String>,
    /// Names whose lookup found none.
    pub missed: BTreeSet<String>,
    /// Card NULL-SAFETY-PROPERTY (gateway rule 14n): the named null premises
    /// (`null-deref:FUNCTION:LINE:VARIABLE`) the logged work relied on.
    pub null_premises: BTreeSet<String>,
    /// Card HANDOFF-CANDIDATE-OWN-CALLBACKS: declarations a decided callback
    /// invocation read (its candidates, and a composed candidate's own inner
    /// candidates), which the body need not spell.
    pub relied: BTreeSet<String>,
}

impl Consulted {
    /// Add every name `other` noted.
    pub fn absorb(&mut self, other: &Self) {
        self.found.extend(other.found.iter().cloned());
        self.missed.extend(other.missed.iter().cloned());
        self.null_premises
            .extend(other.null_premises.iter().cloned());
        self.relied.extend(other.relied.iter().cloned());
    }

    /// Whether the log noted nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.found.is_empty()
            && self.missed.is_empty()
            && self.null_premises.is_empty()
            && self.relied.is_empty()
    }
}

/// Card NULL-SAFETY-PROPERTY (gateway rule 14n): note that the current work
/// relied on the named null premise `name`, when a log is open.
pub fn note_null_premise(name: &str) {
    LOG.with(|log| {
        if let Some(current) = log.borrow_mut().as_mut()
            && !current.null_premises.contains(name)
        {
            current.null_premises.insert(name.to_owned());
        }
    });
}

/// Card HANDOFF-CANDIDATE-OWN-CALLBACKS: note that the current work relied
/// on the declaration `name` through a decided callback invocation (the
/// body need not spell it), when a log is open.
pub fn note_relied(name: &str) {
    LOG.with(|log| {
        if let Some(current) = log.borrow_mut().as_mut()
            && !current.relied.contains(name)
        {
            current.relied.insert(name.to_owned());
        }
    });
}

thread_local! {
    static LOG: RefCell<Option<Consulted>> = const { RefCell::new(None) };
}

/// Start a fresh log on this thread and return the enclosing one (if any),
/// which the matching [`close`] restores.
#[must_use]
pub fn open() -> Option<Consulted> {
    LOG.with(|log| log.borrow_mut().replace(Consulted::default()))
}

/// End the current log, restore `enclosing`, and return what the ended log
/// noted (empty when none was open).
pub fn close(enclosing: Option<Consulted>) -> Consulted {
    LOG.with(|log| std::mem::replace(&mut *log.borrow_mut(), enclosing).unwrap_or_default())
}

/// Add `consulted` to the current log, when one is open (an inner log's
/// names are the enclosing consumer's consultations too).
pub fn absorb(consulted: &Consulted) {
    LOG.with(|log| {
        if let Some(current) = log.borrow_mut().as_mut() {
            current.absorb(consulted);
        }
    });
}

/// Note one lookup of `name` in the current log, when one is open.
pub fn note(name: &str, found: bool) {
    LOG.with(|log| {
        let mut log = log.borrow_mut();
        let Some(current) = log.as_mut() else {
            return;
        };
        let set = if found {
            &mut current.found
        } else {
            &mut current.missed
        };
        if !set.contains(name) {
            set.insert(name.to_owned());
        }
    });
}

/// Whether a log is open on this thread.
#[must_use]
pub fn is_open() -> bool {
    LOG.with(|log| log.borrow().is_some())
}

/// A copy of the current log on this thread (`None` when none is open), for a
/// classification probe to hand back to [`restore`] if it declines.
#[must_use]
pub fn snapshot() -> Option<Consulted> {
    LOG.with(|log| log.borrow().clone())
}

/// Restore the current log on this thread to `saved` (`None` when none was
/// open at [`snapshot`]), dropping every consultation noted since the snapshot.
pub fn restore(saved: &Option<Consulted>) {
    LOG.with(|log| log.borrow_mut().clone_from(saved));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::{ResourceApiKind, ResourceContracts};

    #[test]
    fn a_lookup_is_noted_only_while_a_log_is_open_and_logs_nest() {
        let contracts = ResourceContracts::new([("free", ResourceApiKind::Release)]);
        assert!(contracts.contract("free").is_some());
        assert!(!is_open());
        let outer = open();
        assert!(outer.is_none());
        assert!(contracts.contract("free").is_some());
        let enclosing = open();
        assert!(contracts.contract("absent").is_none());
        let inner = close(enclosing);
        assert_eq!(inner.missed, BTreeSet::from(["absent".to_owned()]));
        assert!(inner.found.is_empty());
        absorb(&inner);
        let all = close(outer);
        assert_eq!(all.found, BTreeSet::from(["free".to_owned()]));
        assert_eq!(all.missed, BTreeSet::from(["absent".to_owned()]));
        assert!(!is_open());
        // Nothing is noted once every log is closed.
        assert!(contracts.contract("free").is_some());
        assert!(close(None).is_empty());
    }

    #[test]
    fn snapshot_and_restore_drop_declined_consultations() {
        let contracts = ResourceContracts::new([
            ("free", ResourceApiKind::Release),
            ("realloc", ResourceApiKind::Neutral),
        ]);
        let outer = open();
        assert!(contracts.contract("free").is_some());
        let saved = snapshot();
        assert!(contracts.contract("realloc").is_some());
        assert!(contracts.contract("missing").is_none());
        note_null_premise("null-deref:f:1:p");
        note_relied("cb");
        restore(&saved);
        let all = close(outer);
        assert_eq!(all.found, BTreeSet::from(["free".to_owned()]));
        assert!(all.missed.is_empty());
        assert!(all.null_premises.is_empty());
        assert!(all.relied.is_empty());
    }
}
