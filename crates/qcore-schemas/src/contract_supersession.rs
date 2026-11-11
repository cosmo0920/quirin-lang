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

//! Contract bundles compose by supersession (card **CONTRACT-SUPERSESSION**,
//! contract spec §C06.6; `docs/plans/pluggable-cards/README.md` seam G, user
//! decisions 9 and 9a).
//!
//! A contract bundle is a contribution to the loaded environment. Bundles
//! union by function name. Where two modules declare one function, the
//! declaration whose **pin stamp** (`pinned_at`, written by `quirin-audit pin`
//! into the declaration's lock entry when its current text was reviewed) is
//! greatest is in force, and every other declaration is **retained as
//! superseded**, never deleted. The operation is a maximum over a total order,
//! so it is associative and commutative and its result reads only the *set* of
//! candidates ([`decide`]).
//!
//! Its fail-closed residue is exactly what the order does not decide: equal
//! greatest stamps on different texts, or a candidate with no usable stamp (no
//! lock entry, a lock entry without a stamp, a malformed stamp, or a lock entry
//! taken for another text). Those **withhold the function**: no declaration of
//! it is in force, every candidate's text is recorded, and every call relying
//! on it is held under the reason `contract-conflict`, which is `Unknown`,
//! never `Safe`, `Unsafe` or `Pending` ([`crate::reason::ReasonCategory::ContractConflict`]).
//!
//! These are the data types and the pure decision only. The source-level
//! transform lives in `qcore_contract::contract_supersession`, the stamps come
//! from `qcore_audit::contract_revision`'s locks, and the legacy-C gateway
//! holds the call sites. The Lean model is `QCore.ContractSupersession`.

use serde::{Deserialize, Serialize};

/// Validate a pin stamp: an RFC 3339 UTC timestamp spelled exactly
/// `YYYY-MM-DDTHH:MM:SSZ` with every field in range. Under this one spelling
/// the lexicographic order of two stamps is their chronological order, so the
/// supersession order is a total order on strings.
///
/// # Errors
///
/// A stamp of any other spelling or with a field out of range.
pub fn validate_stamp(stamp: &str) -> Result<(), String> {
    let bytes = stamp.as_bytes();
    let shape = bytes.len() == 20
        && bytes.iter().enumerate().all(|(index, byte)| match index {
            4 | 7 => *byte == b'-',
            10 => *byte == b'T',
            13 | 16 => *byte == b':',
            19 => *byte == b'Z',
            _ => byte.is_ascii_digit(),
        });
    if !shape {
        return Err(format!(
            "pin stamp `{stamp}` is not spelled YYYY-MM-DDTHH:MM:SSZ"
        ));
    }
    let field = |range: std::ops::Range<usize>| -> u32 {
        stamp[range]
            .bytes()
            .fold(0, |value, digit| value * 10 + u32::from(digit - b'0'))
    };
    let (year, month, day) = (field(0..4), field(5..7), field(8..10));
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => 0,
    };
    if year == 0
        || day == 0
        || day > days
        || field(11..13) > 23
        || field(14..16) > 59
        || field(17..19) > 59
    {
        return Err(format!("pin stamp `{stamp}` has a field out of range"));
    }
    Ok(())
}

/// How one declaration's stamp stands, as its lock gives it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "standing", content = "pinned_at")]
pub enum StampStanding {
    /// The lock entry is taken for this declaration's text and carries this
    /// valid stamp.
    Stamped(String),
    /// The lock entry is taken for this text but carries no stamp (a lock
    /// written before lock version 4).
    Missing,
    /// The lock entry's stamp is not a valid stamp.
    Malformed(String),
    /// The lock entry was taken for another contract text: the stamp speaks
    /// for a review of a text that is no longer this one.
    Stale,
    /// No lock covers the declaration, or the lock has no entry for it: an
    /// unpinned declaration is not in force under C06.2 and never supersedes.
    Unpinned,
}

impl StampStanding {
    /// The standing of a stamp read from a lock entry taken for this text.
    #[must_use]
    pub fn of_entry(pinned_at: Option<&str>) -> Self {
        match pinned_at {
            None => Self::Missing,
            Some(stamp) if validate_stamp(stamp).is_ok() => Self::Stamped(stamp.to_owned()),
            Some(stamp) => Self::Malformed(stamp.to_owned()),
        }
    }

    /// The valid stamp, if any.
    #[must_use]
    pub fn stamp(&self) -> Option<&str> {
        match self {
            Self::Stamped(stamp) => Some(stamp),
            _ => None,
        }
    }
}

/// Why a function's declarations were withheld.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ConflictReason {
    /// Some candidate's stamp is malformed.
    MalformedStamp,
    /// Some candidate's lock entry carries no stamp.
    MissingStamp,
    /// Some candidate's lock entry was taken for another text.
    StaleStamp,
    /// Some candidate is not pinned (no lock or no entry).
    Unpinned,
    /// Every candidate is stamped, and two candidates with the greatest stamp
    /// have different texts.
    EqualStamps,
}

impl ConflictReason {
    /// The spelling, identical to the serde representation.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MalformedStamp => "malformed-stamp",
            Self::MissingStamp => "missing-stamp",
            Self::StaleStamp => "stale-stamp",
            Self::Unpinned => "unpinned",
            Self::EqualStamps => "equal-stamps",
        }
    }
}

/// One declaration of a function that more than one module declares.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Candidate {
    /// The module's source-assumption key.
    pub module: String,
    /// The SHA-256 of the declaration's normalised text (C06.2).
    pub contract_sha256: String,
    pub stamp: StampStanding,
}

/// What the stamps decide for one function.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Decision {
    /// This candidate is in force; every other one is superseded by it.
    InForce(Candidate),
    /// The order does not decide: no declaration is in force.
    Conflict(ConflictReason),
}

/// Decide one function's candidates. A function of the **set** of candidates:
/// the order and multiplicity of `candidates` never change the result.
///
/// * Every candidate must carry a valid stamp; otherwise the function is
///   withheld, the reason being the first of malformed, missing, stale and
///   unpinned that some candidate has.
/// * Among the candidates with the greatest stamp, all must have one text;
///   otherwise the function is withheld (`equal-stamps`). Byte-identical
///   duplicates are idempotent: the one in force is the least module key of
///   them, and its text is every one's text.
#[must_use]
pub fn decide(candidates: &[Candidate]) -> Decision {
    let unusable = |stamp: &StampStanding| match stamp {
        StampStanding::Stamped(_) => None,
        StampStanding::Malformed(_) => Some(ConflictReason::MalformedStamp),
        StampStanding::Missing => Some(ConflictReason::MissingStamp),
        StampStanding::Stale => Some(ConflictReason::StaleStamp),
        StampStanding::Unpinned => Some(ConflictReason::Unpinned),
    };
    // The precedence is the declaration order of `ConflictReason`, so the
    // reason is a function of the set of standings.
    if let Some(reason) = candidates
        .iter()
        .filter_map(|candidate| unusable(&candidate.stamp))
        .min()
    {
        return Decision::Conflict(reason);
    }
    let Some(top) = candidates
        .iter()
        .filter_map(|candidate| candidate.stamp.stamp())
        .max()
    else {
        return Decision::Conflict(ConflictReason::Unpinned);
    };
    let mut newest = candidates
        .iter()
        .filter(|candidate| candidate.stamp.stamp() == Some(top))
        .collect::<Vec<_>>();
    newest.sort();
    let first = newest[0];
    if newest
        .iter()
        .any(|candidate| candidate.contract_sha256 != first.contract_sha256)
    {
        return Decision::Conflict(ConflictReason::EqualStamps);
    }
    Decision::InForce(first.clone())
}

/// One declaration retained as superseded.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SupersededDeclaration {
    pub module: String,
    pub contract_sha256: String,
    pub pinned_at: String,
    /// The declaration's text as written in its module.
    pub text: String,
    /// The declaration in force in its place.
    pub by_module: String,
    pub by_contract_sha256: String,
    pub by_pinned_at: String,
}

/// One candidate of a withheld function, with its text.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConflictedDeclaration {
    #[serde(flatten)]
    pub candidate: Candidate,
    pub text: String,
}

/// A function no declaration of which is in force, because the stamps do not
/// decide between them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContractConflict {
    pub reason: ConflictReason,
    /// Every candidate, sorted.
    pub declarations: Vec<ConflictedDeclaration>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(module: &str, digest: &str, stamp: StampStanding) -> Candidate {
        Candidate {
            module: module.into(),
            contract_sha256: digest.into(),
            stamp,
        }
    }

    fn stamped(module: &str, digest: &str, stamp: &str) -> Candidate {
        candidate(module, digest, StampStanding::Stamped(stamp.into()))
    }

    #[test]
    fn stamps_are_validated_strictly() {
        for good in ["2026-10-26T00:00:00Z", "2024-02-29T23:59:59Z"] {
            assert!(validate_stamp(good).is_ok(), "{good}");
        }
        for bad in [
            "2026-10-26",
            "2026-10-26T00:00:00",
            "2026-10-26T00:00:00+00:00",
            "2026-10-26 00:00:00Z",
            "2025-02-29T00:00:00Z",
            "2026-13-01T00:00:00Z",
            "2026-10-26T24:00:00Z",
            "0000-01-01T00:00:00Z",
            "2026-10-26T00:00:00.5Z",
        ] {
            assert!(validate_stamp(bad).is_err(), "{bad}");
        }
        assert_eq!(StampStanding::of_entry(None), StampStanding::Missing);
        assert_eq!(
            StampStanding::of_entry(Some("yesterday")),
            StampStanding::Malformed("yesterday".into())
        );
    }

    #[test]
    fn the_newest_stamp_wins_in_any_order() {
        let old = stamped("a.qv", "1", "2026-01-01T00:00:00Z");
        let new = stamped("b.qv", "2", "2026-02-01T00:00:00Z");
        for order in [
            vec![old.clone(), new.clone()],
            vec![new.clone(), old.clone()],
        ] {
            assert_eq!(decide(&order), Decision::InForce(new.clone()));
        }
    }

    #[test]
    fn equal_stamps_on_different_texts_conflict() {
        let a = stamped("a.qv", "1", "2026-01-01T00:00:00Z");
        let b = stamped("b.qv", "2", "2026-01-01T00:00:00Z");
        assert_eq!(
            decide(&[a, b]),
            Decision::Conflict(ConflictReason::EqualStamps)
        );
    }

    #[test]
    fn identical_texts_are_idempotent() {
        let a = stamped("b.qv", "1", "2026-01-01T00:00:00Z");
        let b = stamped("a.qv", "1", "2026-01-01T00:00:00Z");
        assert_eq!(
            decide(&[a.clone(), b.clone()]),
            Decision::InForce(b.clone())
        );
        assert_eq!(decide(&[b.clone(), a, b.clone()]), Decision::InForce(b));
    }

    #[test]
    fn every_unusable_stamp_withholds() {
        let good = stamped("a.qv", "1", "2026-01-01T00:00:00Z");
        for (standing, reason) in [
            (
                StampStanding::Malformed("x".into()),
                ConflictReason::MalformedStamp,
            ),
            (StampStanding::Missing, ConflictReason::MissingStamp),
            (StampStanding::Stale, ConflictReason::StaleStamp),
            (StampStanding::Unpinned, ConflictReason::Unpinned),
        ] {
            let bad = candidate("b.qv", "2", standing);
            assert_eq!(
                decide(&[good.clone(), bad.clone()]),
                Decision::Conflict(reason)
            );
            assert_eq!(decide(&[bad, good.clone()]), Decision::Conflict(reason));
        }
        // An unpinned declaration never supersedes, even beside an older one.
        let unpinned = candidate("c.qv", "3", StampStanding::Unpinned);
        assert_eq!(
            decide(&[good, unpinned]),
            Decision::Conflict(ConflictReason::Unpinned)
        );
    }
}
