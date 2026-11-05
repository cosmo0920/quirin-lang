//! Card **MEMBER-EFFECT-SUMMARY** (gateway rule 38j): what a body does to the
//! members of the objects its formals designate, beside its custody contract.
//!
//! USER DECISION 2026-11-05 (verbatim): "Option 1 (rec.)", a member-effect
//! summary (docs/reports/member-access-projection/README.md section 3).
//!
//! A W4 summary states what a call does to each argument; it says nothing of
//! the objects reachable through one. A body that releases `P->F` and keeps
//! `P` (`cmt_opts_exit`, `metric_release_storage`) therefore has no summary
//! mode that states its effect, and reading its custody contract alone
//! (`resource_borrowed(P)`) would hide the release from the caller. Such a
//! body's validated contract travels **only** with its effect, in a table of
//! its own ([`crate::contract::ResourceContracts::member_effect`]), never in
//! the ordinary function table: every consumer that does not read the effect
//! (cross-unit records, callback candidates, indirect calls) sees an
//! uncontracted function, so it fails closed.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::contract::ResourceFunctionContract;

/// What the call leaves in a released member.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemberFate {
    /// The object is released and the member still designates it.
    Stale,
    /// The object is released and the member then reads a null constant.
    Cleared,
}

/// Per formal position, the member paths (`F` or `F.G…`, below the object the
/// formal designates) whose object the call releases, each with its fate.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemberEffect {
    pub released: BTreeMap<usize, BTreeMap<String, MemberFate>>,
}

impl MemberEffect {
    /// Whether the effect states nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.released.values().all(BTreeMap::is_empty)
    }

    /// The members released through the formal at `index`.
    #[must_use]
    pub fn at(&self, index: usize) -> Option<&BTreeMap<String, MemberFate>> {
        self.released.get(&index).filter(|paths| !paths.is_empty())
    }
}

/// A validated custody contract with the member effect it travels with.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemberEffectSummary {
    pub contract: ResourceFunctionContract,
    pub effect: MemberEffect,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_effect_with_only_empty_positions_is_empty() {
        let mut effect = MemberEffect::default();
        assert!(effect.is_empty());
        effect.released.insert(0, BTreeMap::new());
        assert!(effect.is_empty());
        assert!(effect.at(0).is_none());
        effect
            .released
            .get_mut(&0)
            .unwrap()
            .insert("ns".into(), MemberFate::Stale);
        assert!(!effect.is_empty());
        assert_eq!(effect.at(0).map(BTreeMap::len), Some(1));
    }
}
