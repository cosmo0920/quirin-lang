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

//! A package's answer and the first-answer-wins consultation of a position.

/// A condition an admission rests on, which the host carries into the row.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Obligation {
    /// A named premise: a declared assumption the row must carry in its
    /// assumptions (§L21.2), never a discharged fact.
    Premise(String),
    /// An obligation the admission leaves open, named by a reason key the
    /// package reserved; the host keeps it in the row's denominator.
    Open(String),
}

/// An admission: what the position's decision becomes, and every obligation
/// it rests on beyond those its lowering carries itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Admission<L> {
    pub lowering: L,
    pub obligations: Vec<Obligation>,
}

/// The only two answers a package gives.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Answer<L> {
    /// The package decides the position: the lowering, with its obligations.
    Admit(Admission<L>),
    /// The package does not decide it; the next package is asked.
    Refuse,
}

impl<L> Answer<L> {
    /// An admission whose obligations are all carried by its lowering (the
    /// gateway's in-crate routes: their instructions name their own sites,
    /// premises and leaves).
    #[must_use]
    pub fn admit(lowering: L) -> Self {
        Self::Admit(Admission {
            lowering,
            obligations: Vec::new(),
        })
    }

    /// `Some` admits, `None` refuses.
    #[must_use]
    pub fn from_option(lowering: Option<L>) -> Self {
        lowering.map_or(Self::Refuse, Self::admit)
    }

    /// The admission, or `None` for a refusal.
    #[must_use]
    pub fn into_admission(self) -> Option<Admission<L>> {
        match self {
            Self::Admit(admission) => Some(admission),
            Self::Refuse => None,
        }
    }
}

/// The answer that decided a position, and the index of the entry that gave it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Consulted<L> {
    pub index: usize,
    pub admission: Admission<L>,
}

/// Ask `entries` in order; the first admission wins.
///
/// An entry `enabled` rejects (a package switched off) is not asked and counts
/// as a refusal, so switching a package off can only remove its answers,
/// never add one. `None` is the end of the table: the host's existing
/// refusal (or its existing decision) stands unchanged.
pub fn consult<E, L>(
    entries: &[E],
    enabled: impl Fn(&E) -> bool,
    mut ask: impl FnMut(usize, &E) -> Answer<L>,
) -> Option<Consulted<L>> {
    for (index, entry) in entries.iter().enumerate() {
        if !enabled(entry) {
            continue;
        }
        if let Answer::Admit(admission) = ask(index, entry) {
            return Some(Consulted { index, admission });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{Answer, Obligation, consult};

    #[test]
    fn the_first_admission_wins_and_later_entries_are_not_asked() {
        let entries = ["a", "b", "c"];
        let mut asked = Vec::new();
        let decided = consult(
            &entries,
            |_| true,
            |_, entry| {
                asked.push(*entry);
                if *entry == "a" {
                    Answer::Refuse
                } else {
                    Answer::admit(entry.to_uppercase())
                }
            },
        )
        .expect("b admits");
        assert_eq!(decided.index, 1);
        assert_eq!(decided.admission.lowering, "B");
        assert_eq!(asked, ["a", "b"]);
    }

    #[test]
    fn a_disabled_entry_is_not_asked_and_falls_through() {
        let entries = ["a", "b"];
        let mut asked = Vec::new();
        let decided = consult(
            &entries,
            |entry| *entry != "a",
            |_, entry| {
                asked.push(*entry);
                Answer::admit(*entry)
            },
        )
        .expect("b admits");
        assert_eq!((decided.index, decided.admission.lowering), (1, "b"));
        assert_eq!(asked, ["b"]);
    }

    #[test]
    fn the_end_of_the_table_is_the_existing_refusal() {
        let entries = ["a", "b"];
        assert!(consult(&entries, |_| true, |_, _| Answer::<()>::Refuse).is_none());
        assert!(consult(&entries, |_| false, |_, entry| Answer::admit(*entry)).is_none());
        assert!(consult::<&str, ()>(&[], |_| true, |_, _| Answer::admit(())).is_none());
    }

    #[test]
    fn an_admission_keeps_its_obligations() {
        let answer = Answer::Admit(super::Admission {
            lowering: 1,
            obligations: vec![Obligation::Premise("asserted_allocation".into())],
        });
        let admission = answer.into_admission().expect("admitted");
        assert_eq!(
            admission.obligations,
            [Obligation::Premise("asserted_allocation".into())]
        );
        assert_eq!(Answer::from_option(None::<u8>), Answer::Refuse);
        assert_eq!(Answer::from_option(Some(2)), Answer::admit(2));
    }
}
