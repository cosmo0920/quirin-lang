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

//! Scoped conditional caller obligations (S3). Unknown never selects the
//! complement. Conditions are keyed by both binder and runtime object root;
//! no fact or premise is global merely because a block is written locally.

/// Ordinary lookup/summary publication cannot erase a guarded interface.
#[must_use]
pub const fn ordinary_publication_allowed(guarded: bool, discharged: bool) -> bool {
    !guarded && discharged
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Truth {
    Yes,
    No,
    Unknown,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Guard {
    pub binder: usize,
    pub root: usize,
    pub negated: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fact {
    pub binder: usize,
    pub root: usize,
    pub value: bool,
}

/// Read only the exact root. Conflicting facts are unresolved, never an
/// inconsistent assumption from which an obligation can be discharged.
#[must_use]
pub fn reading(guard: Guard, facts: &[Fact]) -> Truth {
    let mut value = None;
    for fact in facts
        .iter()
        .filter(|f| f.binder == guard.binder && f.root == guard.root)
    {
        if value.is_some_and(|v| v != fact.value) {
            return Truth::Unknown;
        }
        value = Some(fact.value);
    }
    match value {
        Some(v) if v != guard.negated => Truth::Yes,
        Some(_) => Truth::No,
        None => Truth::Unknown,
    }
}
/// A conditional prerequisite is waived only by a proved inactive branch.
/// Otherwise its caller must discharge it, including when its guard is unknown.
#[must_use]
pub const fn prerequisite(condition: Truth, discharged: bool) -> bool {
    matches!(condition, Truth::No) || discharged
}
/// Facts survive a branch join only where both sides establish the same fact.
#[must_use]
pub fn join(left: &[Fact], right: &[Fact]) -> Vec<Fact> {
    left.iter()
        .filter(|fact| right.contains(fact))
        .copied()
        .collect()
}
/// A local scope consumes no caller facts: its temporary assumption is passed
/// in a fresh value, so it cannot leak into the caller or another instance.
#[must_use]
pub fn scoped(facts: &[Fact], local: Fact) -> Vec<Fact> {
    let mut inner = facts.to_vec();
    inner.push(local);
    inner
}

/// Shared with QCore.LocalCondition, `(truth, discharged, admitted)`.
pub const VECTORS: [(Truth, bool, bool); 6] = [
    (Truth::Yes, false, false),
    (Truth::Yes, true, true),
    (Truth::No, false, true),
    (Truth::No, true, true),
    (Truth::Unknown, false, false),
    (Truth::Unknown, true, true),
];
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn guarded_publication_vectors() {
        for (guarded, discharged, allowed) in [
            (false, false, false),
            (false, true, true),
            (true, false, false),
            (true, true, false),
        ] {
            assert_eq!(ordinary_publication_allowed(guarded, discharged), allowed);
        }
    }
    #[test]
    fn shared_vectors() {
        for (t, d, want) in VECTORS {
            assert_eq!(prerequisite(t, d), want);
        }
    }
    #[test]
    fn another_instance_does_not_supply_a_condition() {
        let g = Guard {
            binder: 0,
            root: 1,
            negated: false,
        };
        assert_eq!(
            reading(
                g,
                &[Fact {
                    binder: 0,
                    root: 2,
                    value: false
                }]
            ),
            Truth::Unknown
        );
        assert!(!prerequisite(reading(g, &[]), false));
    }
    #[test]
    fn conflicting_facts_do_not_prove_either_polarity() {
        let f = [
            Fact {
                binder: 0,
                root: 1,
                value: true,
            },
            Fact {
                binder: 0,
                root: 1,
                value: false,
            },
        ];
        for negated in [false, true] {
            assert_eq!(
                reading(
                    Guard {
                        binder: 0,
                        root: 1,
                        negated
                    },
                    &f
                ),
                Truth::Unknown
            );
        }
    }
    #[test]
    fn join_forgets_one_sided_and_opposite_branch_facts() {
        let yes = Fact {
            binder: 0,
            root: 1,
            value: true,
        };
        let no = Fact {
            value: false,
            ..yes
        };
        assert!(join(&[yes], &[no]).is_empty());
        assert!(join(&[yes], &[]).is_empty());
        let outer = vec![];
        assert_eq!(scoped(&outer, yes), vec![yes]);
        assert!(outer.is_empty());
    }
}
