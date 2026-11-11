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

//! Non-owning indexed lookup. Identities come from the supplied coherent
//! child list. This model allocates nothing and never proves coherence or
//! caller membership merely from borrowed-from provenance.

#[must_use]
pub fn lookup(children: &[usize], index: usize) -> Option<usize> {
    children.get(index).copied()
}

/// The source extractor supplies completeness and canonical tag equality.
/// Unknown or mismatched records cannot authorize a physical prefix reading.
#[must_use]
pub const fn record_eligible(known: bool, same: bool) -> bool {
    known && same
}

/// Resolve only an explicitly known physical prefix. `None` is Unknown;
/// `Some(None)` is a checked NULL, never inferred from an absent link.
#[must_use]
pub fn resolve_prefix(
    index: usize,
    head: Option<Option<usize>>,
    next: impl Fn(usize) -> Option<Option<usize>>,
    valid_member: impl Fn(usize) -> bool,
) -> Option<Option<usize>> {
    let mut current = head?;
    let mut remaining = index;
    let mut seen = std::collections::BTreeSet::new();
    loop {
        let Some(item) = current else {
            return Some(None);
        };
        if !valid_member(item) || !seen.insert(item) {
            return None;
        }
        if remaining == 0 {
            return Some(Some(item));
        }
        remaining -= 1;
        current = next(item)?;
    }
}

/// Conditions for a source-guarded inference view. A name-only site may be
/// viewed as untracked; the view never hides a real ownership/status entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReadViewFacts {
    pub virtual_site: bool,
    pub tracked_status: bool,
    pub unresolved: bool,
    pub conflict: bool,
    pub cursor_relation: bool,
}
#[must_use]
pub const fn allow_read_view(f: ReadViewFacts) -> bool {
    f.virtual_site && !f.tracked_status && !f.unresolved && !f.conflict && f.cursor_relation
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn audited_record_eligibility_vectors_match_rust() {
        let lean = include_str!("../../../proof/QCore/ListLookup.lean");
        let table = lean
            .split("def recordVectors : List (Bool × Bool × Bool) := [")
            .nth(1)
            .unwrap()
            .split("\n]")
            .next()
            .unwrap();
        let mut count = 0;
        for line in table
            .lines()
            .map(str::trim)
            .filter(|line| line.starts_with('('))
        {
            let values = line
                .trim_start_matches('(')
                .trim_end_matches(',')
                .trim_end_matches(')')
                .split(',')
                .map(|value| match value.trim() {
                    "true" => true,
                    "false" => false,
                    other => panic!("{other}"),
                })
                .collect::<Vec<_>>();
            assert_eq!(values.len(), 3);
            assert_eq!(record_eligible(values[0], values[1]), values[2]);
            count += 1;
        }
        assert_eq!(count, 4);
    }
    #[test]
    fn lean_borrowed_view_vectors_match_the_rust_decision() {
        let lean = include_str!("../../../proof/QCore/ListLookup.lean");
        let table = lean
            .split("def readViewVectors : List (ReadViewFacts × Bool) := [")
            .nth(1)
            .unwrap()
            .split("\n]")
            .next()
            .unwrap();
        let mut count = 0;
        for line in table
            .lines()
            .map(str::trim)
            .filter(|line| line.starts_with("(⟨"))
        {
            let (facts, result) = line.trim_start_matches("(⟨").split_once("⟩,").unwrap();
            let values = facts
                .split(',')
                .map(|s| match s.trim() {
                    "true" => true,
                    "false" => false,
                    other => panic!("{other}"),
                })
                .collect::<Vec<_>>();
            assert_eq!(values.len(), 5);
            let expected = result.trim().trim_end_matches(',').trim_end_matches(')');
            let f = ReadViewFacts {
                virtual_site: values[0],
                tracked_status: values[1],
                unresolved: values[2],
                conflict: values[3],
                cursor_relation: values[4],
            };
            assert_eq!(allow_read_view(f), expected == "true");
            assert!(matches!(expected, "true" | "false"));
            if f.tracked_status {
                assert!(!allow_read_view(f));
            }
            count += 1;
        }
        assert_eq!(count, 7);
    }
    #[test]
    fn lean_vectors_execute_both_rust_lookup_models() {
        let lean = include_str!("../../../proof/QCore/ListLookup.lean");
        let table = lean
            .split("def vectors : List (List Nat × Nat × Option Nat) := [")
            .nth(1)
            .unwrap()
            .split("\n]")
            .next()
            .unwrap();
        let mut count = 0;
        for line in table
            .lines()
            .map(str::trim)
            .filter(|line| line.starts_with('('))
        {
            let (list, rest) = line.trim_start_matches('(').split_once("],").unwrap();
            let children = list
                .trim_start_matches('[')
                .split(',')
                .map(str::trim)
                .filter(|x| !x.is_empty())
                .map(|x| x.parse::<usize>().unwrap())
                .collect::<Vec<_>>();
            let (index, result) = rest.split_once(',').unwrap();
            let index = index.trim().parse::<usize>().unwrap();
            let result = result
                .trim()
                .trim_end_matches(',')
                .trim_end_matches(')')
                .trim();
            let expected = if result == "none" {
                None
            } else {
                Some(
                    result
                        .strip_prefix("some ")
                        .unwrap()
                        .parse::<usize>()
                        .unwrap(),
                )
            };
            assert_eq!(lookup(&children, index), expected);
            let next = |item| {
                children
                    .iter()
                    .position(|x| *x == item)
                    .map(|position| children.get(position + 1).copied())
            };
            assert_eq!(
                resolve_prefix(index, Some(children.first().copied()), next, |item| {
                    children.contains(&item)
                }),
                Some(expected)
            );
            count += 1;
        }
        assert_eq!(count, 7);
    }
    #[test]
    fn explicit_prefix_membership_null_and_unknown_are_distinct() {
        let links = std::collections::BTreeMap::from([(10, Some(20)), (20, Some(30)), (30, None)]);
        let next = |item| links.get(&item).copied();
        let member = |item| [10, 20, 30].contains(&item);
        for (index, result) in [
            (0, Some(10)),
            (1, Some(20)),
            (2, Some(30)),
            (3, None),
            (usize::MAX, None),
        ] {
            assert_eq!(
                resolve_prefix(index, Some(Some(10)), next, member),
                Some(result)
            );
        }
        assert_eq!(resolve_prefix(0, None, next, member), None);
        assert_eq!(resolve_prefix(1, Some(Some(10)), |_| None, member), None);
        assert_eq!(resolve_prefix(0, Some(Some(10)), next, |_| false), None);
        assert_eq!(
            resolve_prefix(2, Some(Some(10)), |_| Some(Some(10)), member),
            None
        );
    }
    #[test]
    fn every_return_is_an_existing_member_and_missing_is_null() {
        for len in 0..8 {
            let children = (10..10 + len).collect::<Vec<_>>();
            for index in (0..10).chain([usize::MAX]) {
                let before = children.clone();
                let result = lookup(&children, index);
                assert_eq!(result, children.get(index).copied());
                if let Some(item) = result {
                    assert!(children.contains(&item));
                }
                assert_eq!(children, before);
            }
        }
    }
}
