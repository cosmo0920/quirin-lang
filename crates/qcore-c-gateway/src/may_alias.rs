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

//! C02-P-ALIAS — the bounded may-alias refinement over declared names.
//!
//! ## The boundary this closes
//!
//! Card C02-P recorded, and deliberately did not repair, a property of the
//! whole resource projection rather than of its own admission
//! (`docs/reports/remaining-c02-p/README.md` §6): **two distinct declared names
//! may denote the same object, and a name-based projection relates them not at
//! all.** The measured shape was
//!
//! ```c
//! void w(struct b_handle *a, struct b_handle *b) { b_release(a); b_probe(b); }
//! ```
//!
//! which answered `Safe`. Nothing in the body establishes `a != b`, so the
//! borrow of `b` may read an object `b_release` has already destroyed. The
//! projection said nothing about `b` because it had bound nothing to `b`, and
//! *silence about a name is what made the answer `Safe`*.
//!
//! ## The rule, v1
//!
//! A **use** of a name `Y` at a position the projection models as reading its
//! actual is *unresolved* — never `Safe` — when this body released some other
//! name `X` on the same path and every one of the following holds:
//!
//! 1. `X` and `Y` are **distinct declared names** of the same function;
//! 2. both are declared **exactly `struct TAG *`** for the same `TAG`, read
//!    from the same preprocessed token stream every other typed rule reads
//!    (`FunctionTypes::record_pointer_names`);
//! 3. the release of `X` **took effect on this path** and `X` has not been
//!    rebound since;
//! 4. neither **discharge** applies (below).
//!
//! Card ALIAS-LOCALS lifted v1's third premise, "at least one of the two is a
//! formal parameter". Two same-tagged **locals** are now related exactly like
//! any other pair. That bound was a blast-radius choice, never a soundness
//! argument, and it was launderable: `one = p; two = q; b_release(one);
//! b_probe(two);` is the card's own measured shape written through locals, and
//! v1 answered `Safe` for it.
//!
//! The rule only ever *adds* an `UnknownAlias` diagnostic. It never removes
//! one, never suppresses a leak and never converts an `Unsafe` into anything
//! else, so the movement it can cause is exactly `Safe` → `Unknown`.
//!
//! ## The three relation kinds
//!
//! **Peer** (above) is a *may*-alias relation read from declared types alone.
//! Two further kinds are decided against the projection's own bindings at the
//! use, in [`crate::legacy_c`]:
//!
//! * **provenance-distinct** — `Y` is bound to exactly one live identity *this
//!   body acquired*, so no peer's release can have reached it. A discharge;
//! * **definite alias** — [`definitely_aliased`]: the projection has bound `X`
//!   and `Y` to the *same* single non-null identity, so they are not merely
//!   *able* to denote one object, they *do*. Not a discharge: a strengthening.
//!
//! ## The two discharges
//!
//! * **Provenance.** `Y` is bound, at the use, to exactly one live identity
//!   *this body acquired* — not a caller-held entry site. A body-made
//!   acquisition returns an object distinct from every object live when it ran,
//!   and the projection's own identity for `Y` is then what answers the
//!   question. Decided in [`crate::legacy_c`] against the abstract state, not
//!   here. This is also the discharge that covers ALIAS-LOCALS' first
//!   local-specific case, a local bound from a body acquisition.
//! * **Pointer inequality.** A branch on `X != Y` (or `X == Y`) reaching the
//!   use on the distinct edge. Decided flow-sensitively in [`crate::legacy_c`]
//!   through `LegacyInstruction::BranchOnDistinct`; a rebinding of either name
//!   drops the fact. **A definite alias defeats it**: a source-level branch may
//!   not overrule the projection's own identity model, and refusing there is
//!   strictly the more refusing answer.
//!
//! Which releases have run and which guards hold are computed by
//! `legacy_c::alias_facts`, a **separate** forward fixed point over the CFG that
//! never touches the custody state. That separation is measured rather than
//! stylistic — see `C02-P-ALIAS-LEAK-ORDER` in the card report.
//!
//! ## The verdict for a definite alias, decided rather than assumed
//!
//! When `X` and `Y` are a definite alias and `X`'s release reached the identity
//! `Y` denotes, the read is not "may read a destroyed object" but "does read
//! one". The published claim is nevertheless the same existing `UnknownAlias`,
//! and that is a deliberate decision recorded in
//! `docs/reports/remaining-alias-locals/README.md` §3: every stronger kind this
//! checker owns (`DoubleRelease`, `ReleaseAfterEscape`, `CustodyConflict`,
//! `Leak`) names a different event *and* belongs to the class
//! `classify_diagnostics` publishes as `Unsafe`, i.e. as a validated reachable
//! violation. This rule establishes neither that the callee dereferences its
//! `Borrowed` actual nor that the path is feasible, so naming it a violation
//! would manufacture an upstream defect allegation. Giving `Borrowed` positions
//! an implemented liveness precondition is a separate, larger change and is
//! recorded as the named follow-on `ALIAS-LOCALS-BORROW-LIVENESS`.
//!
//! ## What is deliberately outside this rule
//!
//! * a function whose declarations the reader **refused**: no type sharing is
//!   established there, so the precondition of the rule never holds and the
//!   pre-existing answer stands (`C02-P-ALIAS-UNTYPED`);
//! * names whose declared type is not exactly `struct TAG *` — `void *`,
//!   multi-star, an unresolved typedef (`C02-P-ALIAS-UNTYPED`). A definite
//!   alias between two such names is *not* related either: the definite-alias
//!   kind refines the peer relation, it never widens it;
//! * **consuming** an argument, as opposed to releasing it
//!   (`C02-P-ALIAS-CONSUME`);
//! * uses that are not modeled reads of the pointer — a scalar-field no-op, a
//!   branch condition, an `Out` slot address (`C02-P-ALIAS-USES`).
//!
//! Each is a *retained* pre-existing answer, never a new admission.

use std::collections::{BTreeMap, BTreeSet};

use crate::typed_field_access::FunctionTypes;

/// The declared record-pointer classes of one function body.
#[derive(Clone, Debug, Default)]
pub(crate) struct MayAliasClasses {
    /// Name -> declared record tag, for names declared exactly `struct TAG *`.
    tags: BTreeMap<String, String>,
}

impl MayAliasClasses {
    /// Read the classes from the same declaration facts every other typed rule
    /// reads. A function whose declarations were refused yields no class at
    /// all, so it relates no pair of names.
    ///
    /// Whether a name is a formal parameter is deliberately **not** read here.
    /// v1 required at least one formal in every related pair; card ALIAS-LOCALS
    /// lifted that bound, and the caller-held half of the question is answered
    /// where it belongs — by the provenance discharge, against the entry sites
    /// the projection itself records.
    pub(crate) fn read(types: &FunctionTypes) -> Self {
        let tags = types
            .record_pointer_names()
            .into_iter()
            .map(|(name, tag)| (name.to_owned(), tag.to_owned()))
            .collect::<BTreeMap<_, _>>();
        Self { tags }
    }

    /// The names whose release leaves a use of `used` unresolved, in a stable
    /// order. Empty whenever the rule's precondition does not hold, which is
    /// the pre-existing answer.
    pub(crate) fn peers(&self, used: &str) -> Vec<String> {
        let Some(tag) = self.tags.get(used) else {
            return Vec::new();
        };
        self.tags
            .iter()
            .filter(|(name, other)| name.as_str() != used && *other == tag)
            .map(|(name, _)| name.clone())
            .collect()
    }
}

/// What the resource projection has bound to one name at one program point.
///
/// Exactly the three fields of `legacy_c::AbstractPointer`, borrowed so that
/// the decision below costs nothing per read. Kept here, away from the custody
/// state, so that the *decision* stays testable in isolation and so that
/// nothing in this module can write to a projection state.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Denotation<'a> {
    pub(crate) targets: &'a BTreeSet<usize>,
    pub(crate) unknown: bool,
    pub(crate) may_be_null: bool,
}

impl Denotation<'_> {
    /// The single non-null identity this name denotes, when the projection
    /// committed to exactly one. `None` for an unknown, ambiguous, unbound or
    /// possibly-null binding — every case in which two such names might still
    /// denote different objects.
    ///
    /// Card `ALIAS-LOCALS-BORROW-LIVENESS` reads the same commitment for a
    /// different question — whether *one* name's identity is definitely dead —
    /// in [`crate::borrow_liveness`]. Sharing this accessor is deliberate: the
    /// two rules must agree on what "the projection committed" means, and
    /// neither may relax it.
    pub(crate) fn settled(&self) -> Option<usize> {
        (!self.unknown && !self.may_be_null && self.targets.len() == 1)
            .then(|| self.targets.iter().copied().next())
            .flatten()
    }
}

/// ALIAS-LOCALS: whether the projection's own bindings show that two names
/// denote **one** object here — the third relation kind.
///
/// This is not a discharge and it never relates a pair the declared types do
/// not already relate. Its one effect is that a pointer-inequality guard cannot
/// discharge such a pair: the branch asserts the two names are distinct while
/// the projection's identity model says they are the same object, and refusing
/// on that contradiction is strictly the more refusing answer.
pub(crate) fn definitely_aliased(one: Denotation<'_>, other: Denotation<'_>) -> bool {
    match (one.settled(), other.settled()) {
        (Some(left), Some(right)) => left == right,
        _ => false,
    }
}

/// The root name a canonical place dereferences: the identifier before the
/// first `->`, `.` or `[`.
///
/// `"storage->cio"` yields `"storage"`, `"b"` yields `"b"`. An indirect place
/// (`"*out"`) yields `None`: it names a slot, not a pointer this rule relates.
pub(crate) fn place_root(place: &str) -> Option<&str> {
    if place.starts_with('*') {
        return None;
    }
    let end = place
        .find(['.', '['])
        .into_iter()
        .chain(place.find("->"))
        .min()
        .unwrap_or(place.len());
    let root = &place[..end];
    (!root.is_empty()).then_some(root)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::typed_field_access::TypedAccessModel;

    fn texts(source: &str) -> Vec<&str> {
        let bytes = source.as_bytes();
        let mut tokens = Vec::new();
        let mut cursor = 0;
        while cursor < bytes.len() {
            let byte = bytes[cursor];
            if byte.is_ascii_whitespace() {
                cursor += 1;
            } else if byte.is_ascii_alphanumeric() || byte == b'_' {
                let start = cursor;
                while cursor < bytes.len()
                    && (bytes[cursor].is_ascii_alphanumeric() || bytes[cursor] == b'_')
                {
                    cursor += 1;
                }
                tokens.push(&source[start..cursor]);
            } else {
                tokens.push(&source[cursor..cursor + 1]);
                cursor += 1;
            }
        }
        tokens
    }

    const DECLARATIONS: &str = "struct rec { int state ; } ; struct other { int state ; } ;";

    /// Build the classes over one parameter list and one body.
    fn classes(parameters: &str, body: &str) -> MayAliasClasses {
        let unit: &'static str =
            Box::leak(format!("{DECLARATIONS} {parameters} {body}").into_boxed_str());
        let model = TypedAccessModel::read(&texts(unit));
        let parameter_texts: &'static [&'static str] =
            Box::leak(texts(Box::leak(parameters.to_owned().into_boxed_str())).into_boxed_slice());
        let body_texts: &'static [&'static str] =
            Box::leak(texts(Box::leak(body.to_owned().into_boxed_str())).into_boxed_slice());
        let protected = BTreeSet::new();
        let types = FunctionTypes::read(&model, parameter_texts, body_texts, &protected);
        MayAliasClasses::read(&types)
    }

    /// A binding of exactly one known, non-null identity.
    fn settled(site: usize) -> (BTreeSet<usize>, bool, bool) {
        (BTreeSet::from([site]), false, false)
    }

    fn denote(binding: &(BTreeSet<usize>, bool, bool)) -> Denotation<'_> {
        Denotation {
            targets: &binding.0,
            unknown: binding.1,
            may_be_null: binding.2,
        }
    }

    #[test]
    fn two_formals_of_the_same_record_pointer_type_are_peers() {
        let classes = classes("struct rec * a , struct rec * b", "");
        assert_eq!(classes.peers("a"), vec!["b".to_owned()]);
        assert_eq!(classes.peers("b"), vec!["a".to_owned()]);
    }

    #[test]
    fn different_record_tags_are_never_peers() {
        let classes = classes("struct rec * a , struct other * b", "");
        assert!(classes.peers("a").is_empty());
        assert!(classes.peers("b").is_empty());
    }

    #[test]
    fn a_formal_and_a_local_of_the_same_type_are_peers_in_both_directions() {
        let classes = classes("struct rec * a", "struct rec * held ;");
        assert_eq!(classes.peers("a"), vec!["held".to_owned()]);
        assert_eq!(classes.peers("held"), vec!["a".to_owned()]);
    }

    /// ALIAS-LOCALS moves v1's blast-radius bound: two locals of one declared
    /// class are peers, in both directions, with no formal in the function at
    /// all. This assertion is the exact inverse of v1's
    /// `two_locals_are_not_peers_in_v1`, which is why the pin moves rather than
    /// being deleted.
    #[test]
    fn two_locals_of_one_class_are_peers() {
        let classes = classes("int n", "struct rec * one ; struct rec * two ;");
        assert_eq!(classes.peers("one"), vec!["two".to_owned()]);
        assert_eq!(classes.peers("two"), vec!["one".to_owned()]);
    }

    /// The load-bearing negative half survives the lift unchanged: two locals
    /// of *different* declared classes stay unrelated, so the relation is still
    /// the declared type and not "any two pointers".
    #[test]
    fn two_locals_of_different_classes_are_still_unrelated() {
        let classes = classes("int n", "struct rec * one ; struct other * two ;");
        assert!(classes.peers("one").is_empty());
        assert!(classes.peers("two").is_empty());
    }

    /// A three-name class relates every ordered pair, formal or not.
    #[test]
    fn a_three_name_class_relates_every_pair() {
        let classes = classes("struct rec * a , struct rec * b", "struct rec * held ;");
        assert_eq!(classes.peers("a"), vec!["b".to_owned(), "held".to_owned()]);
        assert_eq!(classes.peers("held"), vec!["a".to_owned(), "b".to_owned()]);
    }

    /// A name is never its own peer, so a release through it is judged by the
    /// existing double-release model rather than by this rule.
    #[test]
    fn a_name_is_never_its_own_peer() {
        let classes = classes("struct rec * a , struct rec * b", "");
        assert!(!classes.peers("a").contains(&"a".to_owned()));
    }

    /// A refused declaration establishes no sharing, so the pre-existing answer
    /// stands (`C02-P-ALIAS-UNTYPED`).
    #[test]
    fn an_unreadable_function_relates_nothing() {
        let classes = classes(
            "struct rec * a",
            "void ( * hook ) ( int ) ; struct rec * held ;",
        );
        assert!(classes.peers("a").is_empty());
        assert!(classes.peers("held").is_empty());
    }

    /// `void *`, a multi-star and an unresolved typedef are not `struct TAG *`.
    #[test]
    fn a_type_the_reader_will_not_commit_to_relates_nothing() {
        let classes = classes("struct rec * a , struct rec * * b", "");
        assert!(classes.peers("b").is_empty());
        assert!(classes.peers("a").is_empty());
    }

    #[test]
    fn a_place_root_is_the_identifier_before_the_first_selection() {
        assert_eq!(place_root("storage->cio"), Some("storage"));
        assert_eq!(place_root("value.one"), Some("value"));
        assert_eq!(place_root("table[0]"), Some("table"));
        assert_eq!(place_root("b"), Some("b"));
        assert_eq!(place_root("*out"), None);
    }

    #[test]
    fn a_root_stops_at_whichever_selector_comes_first() {
        assert_eq!(place_root("a.b->c"), Some("a"));
        assert_eq!(place_root("a->b.c"), Some("a"));
    }

    // ----------------------------------------------------------------------
    // ALIAS-LOCALS: the definite-alias relation kind
    // ----------------------------------------------------------------------

    /// The positive direction: one identity, committed to on both sides.
    #[test]
    fn two_names_bound_to_one_identity_are_a_definite_alias() {
        let (one, two) = (settled(7), settled(7));
        assert!(definitely_aliased(denote(&one), denote(&two)));
    }

    /// Distinct identities are not, which is what keeps the kind from being
    /// vacuously true of every pair.
    #[test]
    fn two_names_bound_to_different_identities_are_not() {
        let (one, two) = (settled(7), settled(8));
        assert!(!definitely_aliased(denote(&one), denote(&two)));
    }

    /// The relation is symmetric and reflexive on a settled binding, matching
    /// the guard question it answers.
    #[test]
    fn the_definite_relation_is_symmetric_and_holds_of_a_name_and_itself() {
        let (one, two) = (settled(3), settled(3));
        assert_eq!(
            definitely_aliased(denote(&one), denote(&two)),
            definitely_aliased(denote(&two), denote(&one))
        );
        assert!(definitely_aliased(denote(&one), denote(&one)));
    }

    /// Every way the projection can decline to commit fails closed: an unknown
    /// pointer, an ambiguous target set, an unbound name and a possibly-null
    /// binding are each *not* a definite alias, so the guard keeps discharging
    /// exactly the pairs it discharged before.
    #[test]
    fn an_uncommitted_binding_is_never_a_definite_alias() {
        let settled_site = settled(7);
        let unknown = (BTreeSet::from([7]), true, false);
        let ambiguous = (BTreeSet::from([7, 8]), false, false);
        let unbound = (BTreeSet::new(), false, false);
        let nullable = (BTreeSet::from([7]), false, true);
        for other in [&unknown, &ambiguous, &unbound, &nullable] {
            assert!(
                !definitely_aliased(denote(&settled_site), denote(other)),
                "an uncommitted binding must never be definite: {other:?}"
            );
            assert!(!definitely_aliased(denote(other), denote(&settled_site)));
        }
    }
}
