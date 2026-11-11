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

//! `ALIAS-LOCALS-BORROW-LIVENESS` — the liveness precondition of a `Borrowed`
//! position, as a decision over the projection's own bindings.
//!
//! # What card ALIAS-LOCALS left open
//!
//! `docs/reports/remaining-alias-locals/README.md` §3.4 and §7 record that a
//! `Borrowed` position has **no implemented precondition at all**: the transfer
//! function's parameter loop matches
//! `ResourceParameterMode::Borrowed | ResourceParameterMode::Out { .. } => {}`
//! and falls through. That is the root of the false `Safe` that card repaired
//! only along its alias relation — a body may copy a handle, release the
//! original, and hand the copy to a `Borrowed` position while the projection
//! itself records the identity `RELEASED`, and nothing asks a question.
//!
//! This module implements the general precondition and states it as one
//! decision so that it is testable in isolation, modelled in Lean
//! (`QCore.BorrowLiveness`), and unable to write to any projection state.
//!
//! # The decision, and what it deliberately does not decide
//!
//! [`borrowed_actual_liveness`] answers [`BorrowLiveness::DefinitelyNotLive`]
//! only under a conjunction of facts read from the settled state:
//!
//! 1. the name is **bound with a settled denotation** — exactly one target
//!    site, not `unknown`, not `may_be_null` — reusing ALIAS-LOCALS's
//!    [`crate::may_alias::Denotation`] machinery, so the projection has
//!    committed to one identity on every reaching path;
//! 2. that identity carries **no unresolved effect**, so the projection has not
//!    already declined to commit to what happened to it;
//! 3. its joined status satisfies [`custody_definitely_ended`]: **no** reaching
//!    path leaves the body a pointer the projection knows to be alive (no
//!    `LIVE`, no `CONTAINED`) and **some** reaching path destroyed the identity
//!    (`RELEASED`). Because the status join is a bitwise union, an absent bit is
//!    a must-fact and a present one a may-fact, so that reads as "custody ended
//!    on every path, and one of them was a destruction".
//!
//! Card BORROW-LIVENESS required the status to be *exactly* `RELEASED`; card
//! HYGIENE-14 derived the condition above from the join semantics and admitted
//! the one further union it covers, `RELEASED | ESCAPED` — a release on one
//! branch joined with a consumption on the other. The unions that stay admitted,
//! and why, are tabulated on [`custody_definitely_ended`].
//!
//! Every other shape is [`BorrowLiveness::NotDecided`] and keeps the existing
//! answer byte-for-byte: an unbound name (**every scalar `Borrowed` actual is
//! one** — §C04.2 admits scalars in exactly this position and nowhere else), an
//! unknown, ambiguous or possibly-null binding, a live identity, a contained
//! one, an escaped one on its own (custody moved is not custody destroyed and
//! nothing is known to have gone wrong), an identity this function never
//! acquired, and any site under an unresolved effect.
//!
//! This is deliberately **stricter in the state dimension than the existing
//! `Unsafe` kinds**: `DoubleRelease` fires on `status & RELEASED != 0`, a *may*
//! fact over reaching paths, whereas this rule requires the *must* fact. It is
//! also deliberately **not** the may-alias relation: ALIAS-LOCALS's rule relates
//! two *names*, is a may fact, and keeps its own obligations unchanged. The two
//! never blur — this module never reads a peer, a declared type or an alias
//! fact.
//!
//! # The published verdict class — decided, not defaulted
//!
//! A refusal is the existing `UnknownAlias`, i.e. the `Unknown` class, and the
//! grounds are recorded in `docs/reports/remaining-borrow-liveness/README.md`
//! §3. In short: every kind `classify_diagnostics` publishes as `Unsafe` names a
//! contradiction against a *declared callee obligation*, and
//! `docs/QCORE_CONTRACT_SPEC_r0_3.md` §C04.2 says `resource_borrowed(p)`
//! imposes **no custody precondition** — there is no declared obligation to
//! contradict. The closest implemented analogue settles it: `Owned`, the one
//! mode whose declaration *does* demand live custody, is checked by
//! `require_live_resource`, and that helper reports `UnknownAlias` for exactly
//! this state. Matching it is consistency, not compromise.
//!
//! # The second decision: destroyed by the call that reads it
//!
//! Card BORROW-RESIDUALS takes the `BORROW-LIVENESS-PRESTATE` residual. The
//! rule above reads the state *entering* the call, which is the standard
//! reading of a precondition and stays that way. What it therefore never asked
//! is the question that lives entirely **inside one call**: this call's own
//! declared effects destroy the identity one of its own `Borrowed` positions
//! reads.
//!
//! ```c
//! y_retire(one, one);   /* declared: resource_released(dead), resource_borrowed(seen) */
//! ```
//!
//! measured `Safe` with no diagnostics at this card's base, in both parameter
//! orders, through two distinct names the projection settled to one identity,
//! and through an aggregate member read.
//!
//! [`borrow_destroyed_by_this_call`] refuses it. The fact is a **must** fact of
//! exactly the kind above — the projection settled *both* actuals to the same
//! identity — and the reason it is refused rather than admitted is that the
//! contract vocabulary carries **no ordering** between a callee's own declared
//! effects. `resource_released(dead)` says the callee destroys that object;
//! `resource_borrowed(seen)` says it reads this one; nothing anywhere says
//! which happens first. An unproven call effect is `Unknown` (AGENTS.md hard
//! invariant 1), so the projection declines to certify.
//!
//! Two bounds carried over deliberately, each mirroring the path rule:
//!
//! * **only `Released` destroys.** `Consumed`, `Stored` and `Removed` move or
//!   detach custody without destroying — §C04.2 says `resource_consumed` "is
//!   not a proof of destruction" — so a consumed-and-borrowed pair keeps its
//!   existing answer, exactly as `ESCAPED` alone does on the path rule
//!   ([`custody_definitely_ended`]). `Out` writes a slot and destroys nothing.
//! * **settled denotations only.** Both sides must settle, so every scalar
//!   actual is untouched. That matters concretely: the one declaration in the
//!   pinned corpus carrying a released and a borrowed parameter at once is
//!   `cio_chunk_close(chunk, delete)`, whose borrowed parameter is a scalar
//!   flag — §C04.2 admits scalars in exactly that position and this rule must
//!   never fire on it.
//!
//! Unlike the path rule this one is **not** restricted to plain names: a
//! composite place is admitted on either side whenever the projection settled
//! it. That is not a widening of `BORROW-LIVENESS-PLACES` — the path rule keeps
//! its plain-name bound, because O04-READ-BOUND and O04-B09B10 already impose a
//! strictly stronger `LIVE` requirement on those places *in the pre-state*.
//! What they do not do, and what no other channel does, is ask this call's own
//! effects about them.
//!
//! # What the refusal does not touch
//!
//! Nothing here writes to `AbstractState`, so `C02-P-ALIAS-LEAK-ORDER` is
//! undisturbed by construction: the custody work list, its fixed point, the
//! leak inventory and every per-exit residual are unchanged. The rule runs as a
//! read-only pass over the **settled** incoming states, after the work list is
//! stable, for the same reason ALIAS-LOCALS put its alias facts in a pre-pass —
//! a *must* fact read on an intermediate iterate would depend on scheduling.

use std::collections::{BTreeMap, BTreeSet};

use crate::legacy_c::{self, RELEASED};
use crate::may_alias::Denotation;

/// What the projection's own bindings say about one `Borrowed` actual.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BorrowLiveness {
    /// The projection committed to exactly one identity for this name; **no**
    /// path reaching the call leaves that identity in a state where this body
    /// holds a pointer the projection knows to be alive, and **some** path
    /// destroyed it. The precondition is violated.
    ///
    /// Card HYGIENE-14 widened this from "released on every path" to the exact
    /// status-set condition [`custody_definitely_ended`] states; the original
    /// case (`RELEASED` on every path) is one of its two members.
    DefinitelyNotLive,
    /// Anything else. No claim is made in either direction and the existing
    /// answer is preserved exactly.
    NotDecided,
}

/// The projection facts one `Borrowed` actual is decided against.
///
/// Passed as data rather than as a borrow of the analyzer's state so that the
/// decision stays testable in isolation and so that nothing in this module can
/// reach a mutable projection.
#[derive(Clone, Copy, Debug)]
pub(crate) struct BorrowSite<'a> {
    /// What the name denotes here, or `None` when the projection binds it at
    /// all — a scalar actual, or any identifier this body never tracked.
    pub(crate) denotation: Option<Denotation<'a>>,
    /// The joined per-site status bitmap.
    pub(crate) resources: &'a BTreeMap<usize, u8>,
    /// Sites whose disposition the projection already declined to decide.
    pub(crate) unresolved: &'a BTreeSet<usize>,
}

/// The status bits whose reaching path leaves this body a pointer the
/// projection knows to be **alive**, so a read of it is entitled there.
///
/// * `LIVE` — the body acquired the identity and still holds custody.
/// * `CONTAINED` — the identity sits inside a container this body still tracks.
///   Containment is not destruction: the object is alive on that path, and the
///   analyzer treats a contained child as an outstanding obligation rather than
///   a finished one (`cascade_resource_action` moves it to a terminal state only
///   when the *owner* is released or escapes).
const ENTITLES_A_READ: u8 = legacy_c::LIVE | legacy_c::CONTAINED;

/// The four bits the analyzer's status bitmap can carry. A status outside this
/// mask is a state this rule was not written for, and is not decided.
const MODELLED: u8 = legacy_c::LIVE | legacy_c::RELEASED | legacy_c::ESCAPED | legacy_c::CONTAINED;

/// The exact status-set condition under which a settled identity is
/// **definitely not live** for this body (card HYGIENE-14,
/// `BORROW-LIVENESS-MIXED`).
///
/// `AbstractState::join` is a bitwise union over reaching paths, so a *present*
/// bit is a may-fact and an *absent* bit is a must-fact. The condition is the
/// conjunction of one must-fact and one may-fact:
///
/// 1. `status & ENTITLES_A_READ == 0` — **no** reaching path leaves the body a
///    pointer this projection knows to be alive;
/// 2. `status & RELEASED != 0` — **some** reaching path destroyed the identity.
///
/// Over the modelled bits that set is exactly `{RELEASED}` and
/// `{RELEASED, ESCAPED}`. The mixed join is the addition: a path that released
/// and a path that consumed both ended this body's custody, and the released one
/// destroyed the value, so there is no path on which the projection can say the
/// borrow reads something it knows to be there.
///
/// The unions this deliberately leaves admitted, each for its own reason —
/// all pinned in the tests below and in `QCore.BorrowLiveness`:
///
/// | Status | Why it is not refused |
/// |---|---|
/// | anything with `LIVE` (`LIVE`, `LIVE\|RELEASED`, …) | a reaching path leaves the body holding live custody; the borrow is entitled there. This is the must-half card BORROW-LIVENESS proved in both directions. |
/// | anything with `CONTAINED` (`CONTAINED`, `CONTAINED\|RELEASED`, …) | containment is not destruction; the object is alive inside its container on that path. |
/// | `ESCAPED` alone | **no** path invalidated it. `resource_consumed` "is not a proof of destruction" (§C04.2), and it is not a proof of survival either — but nothing is known to have gone wrong, so the rule declines. `C02-P-ALIAS-CONSUME` is preserved exactly. |
/// | absent, or `0` | the projection tracks no identity here. |
/// | any bit outside `MODELLED` | a state this rule was not written for. |
///
/// The published class stays `Unknown` (`UnknownAlias`): the refusal says the
/// projection will not certify this read, **not** that the read is a defect.
/// That is why the mixed join is admissible here and would not be under an
/// `Unsafe` claim — on the escaped path the pointee may well still be alive.
pub(crate) fn custody_definitely_ended(status: u8) -> bool {
    status & ENTITLES_A_READ == 0 && status & RELEASED != 0 && status & !MODELLED == 0
}

/// Decide the `Borrowed` liveness precondition for one actual.
///
/// Fails closed to [`BorrowLiveness::NotDecided`] on every shape the
/// projection has not fully committed to; see the module header for the
/// complete conjunction and for what each refusal preserves.
pub(crate) fn borrowed_actual_liveness(site: BorrowSite<'_>) -> BorrowLiveness {
    let Some(denotation) = site.denotation else {
        return BorrowLiveness::NotDecided;
    };
    let Some(identity) = denotation.settled() else {
        return BorrowLiveness::NotDecided;
    };
    if site.unresolved.contains(&identity) {
        return BorrowLiveness::NotDecided;
    }
    match site.resources.get(&identity) {
        Some(status) if custody_definitely_ended(*status) => BorrowLiveness::DefinitelyNotLive,
        _ => BorrowLiveness::NotDecided,
    }
}

/// What **one call's own declared effects** say about one of its `Borrowed`
/// positions (card BORROW-RESIDUALS, `BORROW-LIVENESS-PRESTATE`).
///
/// Deliberately a separate type from [`BorrowLiveness`]. That decision is a
/// fact about the *path* reaching the call; this one is a fact about the
/// *call*, and the two are never conflated — this module's standing rule that
/// separate relations stay separate code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SameCallBorrow {
    /// The projection settled this `Borrowed` actual and one of the same
    /// call's `Released` actuals to **one** identity. The call destroys what it
    /// reads, and no declaration orders the two.
    DestroyedByThisCall,
    /// Anything else. No claim is made in either direction and the existing
    /// answer is preserved exactly.
    NotDecided,
}

/// The facts of one call that one of its `Borrowed` actuals is decided against.
#[derive(Clone, Copy, Debug)]
pub(crate) struct SameCallSite<'a> {
    /// What the `Borrowed` actual under decision denotes here, or `None` when
    /// the projection binds it at all — every scalar actual is one.
    pub(crate) borrowed: Option<Denotation<'a>>,
    /// What the same call's **destroying** positions denote, in argument order.
    /// A position whose declared mode does not destroy contributes `None`, and
    /// so does one whose actual this projection does not bind.
    pub(crate) destroying: &'a [Option<Denotation<'a>>],
}

/// Decide whether one call destroys the identity one of its own `Borrowed`
/// positions reads.
///
/// Fails closed to [`SameCallBorrow::NotDecided`] on every shape either side
/// has not settled; see the module header for the complete argument and for
/// what each refusal preserves.
pub(crate) fn borrow_destroyed_by_this_call(site: SameCallSite<'_>) -> SameCallBorrow {
    let Some(identity) = site.borrowed.and_then(|borrowed| borrowed.settled()) else {
        return SameCallBorrow::NotDecided;
    };
    if site
        .destroying
        .iter()
        .flatten()
        .any(|destroying| destroying.settled() == Some(identity))
    {
        SameCallBorrow::DestroyedByThisCall
    } else {
        SameCallBorrow::NotDecided
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The analyzer's own flags, not copies of their numbers (card HYGIENE-14).
    use crate::legacy_c::{CONTAINED, ESCAPED, LIVE};

    struct Fixture {
        targets: BTreeSet<usize>,
        resources: BTreeMap<usize, u8>,
        unresolved: BTreeSet<usize>,
        unknown: bool,
        may_be_null: bool,
        bound: bool,
    }

    impl Fixture {
        /// One settled name denoting site 1 with the given joined status.
        fn settled(status: u8) -> Self {
            Self {
                targets: BTreeSet::from([1]),
                resources: BTreeMap::from([(1, status)]),
                unresolved: BTreeSet::new(),
                unknown: false,
                may_be_null: false,
                bound: true,
            }
        }

        fn decide(&self) -> BorrowLiveness {
            let denotation = self.bound.then_some(Denotation {
                targets: &self.targets,
                unknown: self.unknown,
                may_be_null: self.may_be_null,
            });
            borrowed_actual_liveness(BorrowSite {
                denotation,
                resources: &self.resources,
                unresolved: &self.unresolved,
            })
        }
    }

    #[test]
    fn a_settled_name_denoting_a_released_identity_is_definitely_not_live() {
        assert_eq!(
            Fixture::settled(RELEASED).decide(),
            BorrowLiveness::DefinitelyNotLive
        );
    }

    /// Card HYGIENE-14, `BORROW-LIVENESS-MIXED`: released on one branch and
    /// consumed on the other. Both arms ended this body's custody and one of
    /// them destroyed the identity, so no reaching path leaves a pointer the
    /// projection knows to be alive.
    #[test]
    fn a_release_joined_with_a_consumption_is_definitely_not_live() {
        assert_eq!(
            Fixture::settled(RELEASED | ESCAPED).decide(),
            BorrowLiveness::DefinitelyNotLive
        );
    }

    /// The whole classification, stated as a table over every subset of the
    /// four modelled bits, so a change to the condition has to move a row here.
    #[test]
    fn the_refusable_status_sets_are_exactly_released_and_released_with_escaped() {
        let refusable: Vec<u8> = (0..=15u8)
            .filter(|status| custody_definitely_ended(*status))
            .collect();
        assert_eq!(refusable, vec![RELEASED, RELEASED | ESCAPED]);
    }

    /// A bit outside the analyzer's four is a state this rule was not written
    /// for; it must not be swept into the refusal by the mask.
    #[test]
    fn an_unmodelled_status_bit_is_not_decided() {
        assert!(!custody_definitely_ended(RELEASED | 16));
        assert_eq!(
            Fixture::settled(RELEASED | 16).decide(),
            BorrowLiveness::NotDecided
        );
    }

    /// Card BORROW-LIVENESS kept a local copy of the released bit and asserted
    /// it equalled the analyzer's; card HYGIENE-14 removed the copy — the module
    /// classifies `legacy_c`'s own flags — so what is worth pinning now is that
    /// the four are the distinct single bits the join unions.
    #[test]
    fn the_classified_flags_are_the_analyzer_s_own_distinct_bits() {
        let bits = [LIVE, RELEASED, ESCAPED, CONTAINED];
        for bit in bits {
            assert_eq!(bit.count_ones(), 1, "each status is one bit");
            assert_eq!(bit & MODELLED, bit);
        }
        assert_eq!(bits.iter().fold(0u8, |all, bit| all | bit), MODELLED);
        assert_eq!(ENTITLES_A_READ, LIVE | CONTAINED);
    }

    #[test]
    fn a_live_identity_is_not_decided() {
        assert_eq!(Fixture::settled(LIVE).decide(), BorrowLiveness::NotDecided);
    }

    /// The heart of the *must* half: released on one path and live on another
    /// joins to `LIVE | RELEASED`, and that is not a definite release.
    #[test]
    fn a_release_on_one_branch_only_is_not_decided() {
        assert_eq!(
            Fixture::settled(LIVE | RELEASED).decide(),
            BorrowLiveness::NotDecided
        );
    }

    /// Custody moved is not custody destroyed: `resource_consumed` transfers the
    /// obligation "without claiming that the callee destroys it" (§C04.2). On
    /// its own it is also not a *destruction*, so nothing is known to have gone
    /// wrong and the rule declines — `C02-P-ALIAS-CONSUME`, preserved by card
    /// HYGIENE-14 while the `RELEASED | ESCAPED` join became refusable.
    #[test]
    fn an_escaped_identity_on_its_own_is_not_decided() {
        assert_eq!(
            Fixture::settled(ESCAPED).decide(),
            BorrowLiveness::NotDecided
        );
    }

    /// Containment is not destruction — the object is alive inside its
    /// container on that path — so a contained bit entitles the read exactly
    /// as `LIVE` does, joined or alone.
    #[test]
    fn a_contained_identity_is_not_decided() {
        assert_eq!(
            Fixture::settled(CONTAINED).decide(),
            BorrowLiveness::NotDecided
        );
        assert_eq!(
            Fixture::settled(CONTAINED | RELEASED).decide(),
            BorrowLiveness::NotDecided
        );
        assert_eq!(
            Fixture::settled(CONTAINED | RELEASED | ESCAPED).decide(),
            BorrowLiveness::NotDecided
        );
    }

    /// The must-half survives the widening: adding *any* entitling path to a
    /// refusable status admits it again.
    #[test]
    fn an_entitling_path_added_to_a_refusable_status_admits_it() {
        for refusable in [RELEASED, RELEASED | ESCAPED] {
            assert!(custody_definitely_ended(refusable));
            assert!(!custody_definitely_ended(refusable | LIVE));
            assert!(!custody_definitely_ended(refusable | CONTAINED));
        }
    }

    /// Every scalar `Borrowed` actual takes this path, and §C04.2 admits
    /// scalars in exactly this position. A rule that fired here would refuse
    /// most of the corpus for no reason at all.
    #[test]
    fn an_unbound_name_is_not_decided() {
        let mut fixture = Fixture::settled(RELEASED);
        fixture.bound = false;
        assert_eq!(fixture.decide(), BorrowLiveness::NotDecided);
    }

    #[test]
    fn an_unknown_binding_is_not_decided() {
        let mut fixture = Fixture::settled(RELEASED);
        fixture.unknown = true;
        assert_eq!(fixture.decide(), BorrowLiveness::NotDecided);
    }

    #[test]
    fn a_possibly_null_binding_is_not_decided() {
        let mut fixture = Fixture::settled(RELEASED);
        fixture.may_be_null = true;
        assert_eq!(fixture.decide(), BorrowLiveness::NotDecided);
    }

    #[test]
    fn an_ambiguous_binding_is_not_decided() {
        let mut fixture = Fixture::settled(RELEASED);
        fixture.targets.insert(2);
        fixture.resources.insert(2, RELEASED);
        assert_eq!(fixture.decide(), BorrowLiveness::NotDecided);
    }

    #[test]
    fn an_empty_binding_is_not_decided() {
        let mut fixture = Fixture::settled(RELEASED);
        fixture.targets.clear();
        assert_eq!(fixture.decide(), BorrowLiveness::NotDecided);
    }

    /// A site the projection declined to decide stays undecided here too: an
    /// unresolved effect is precisely the projection saying it does not know
    /// what happened to this identity.
    #[test]
    fn an_unresolved_effect_site_is_not_decided() {
        let mut fixture = Fixture::settled(RELEASED);
        fixture.unresolved.insert(1);
        assert_eq!(fixture.decide(), BorrowLiveness::NotDecided);
    }

    #[test]
    fn an_identity_absent_from_the_state_is_not_decided() {
        let mut fixture = Fixture::settled(RELEASED);
        fixture.resources.clear();
        assert_eq!(fixture.decide(), BorrowLiveness::NotDecided);
    }

    /// Non-vacuity: the `unresolved` guard is load-bearing rather than
    /// decorative — the same fixture without it does fire.
    #[test]
    fn the_unresolved_guard_is_load_bearing() {
        let mut fixture = Fixture::settled(RELEASED);
        assert_eq!(fixture.decide(), BorrowLiveness::DefinitelyNotLive);
        fixture.unresolved.insert(1);
        assert_eq!(fixture.decide(), BorrowLiveness::NotDecided);
    }

    // ----------------------------------------------------------------------
    // Card BORROW-RESIDUALS, `BORROW-LIVENESS-PRESTATE`: destroyed by the call
    // that reads it.
    // ----------------------------------------------------------------------

    /// One binding, as [`Denotation`] reads it.
    struct Binding {
        targets: BTreeSet<usize>,
        unknown: bool,
        may_be_null: bool,
    }

    impl Binding {
        fn settled_at(site: usize) -> Self {
            Self {
                targets: BTreeSet::from([site]),
                unknown: false,
                may_be_null: false,
            }
        }

        fn denote(&self) -> Denotation<'_> {
            Denotation {
                targets: &self.targets,
                unknown: self.unknown,
                may_be_null: self.may_be_null,
            }
        }
    }

    /// Decide one `Borrowed` actual against one call's destroying positions.
    fn same_call(borrowed: Option<&Binding>, destroying: &[Option<&Binding>]) -> SameCallBorrow {
        let destroying = destroying
            .iter()
            .map(|binding| binding.map(Binding::denote))
            .collect::<Vec<_>>();
        borrow_destroyed_by_this_call(SameCallSite {
            borrowed: borrowed.map(Binding::denote),
            destroying: &destroying,
        })
    }

    /// The headline of this residual: one call that releases and borrows the
    /// same settled identity. Measured `Safe` with no diagnostics at the base.
    #[test]
    fn a_call_that_destroys_the_identity_it_reads_is_refused() {
        let one = Binding::settled_at(1);
        assert_eq!(
            same_call(Some(&one), &[Some(&one)]),
            SameCallBorrow::DestroyedByThisCall
        );
    }

    /// Two *distinct* names the projection settled to one identity — the
    /// definite-alias shape — answer the same way, because the decision is over
    /// identities and not over names.
    #[test]
    fn two_names_settled_to_one_identity_are_refused() {
        let (one, two) = (Binding::settled_at(1), Binding::settled_at(1));
        assert_eq!(
            same_call(Some(&one), &[Some(&two)]),
            SameCallBorrow::DestroyedByThisCall
        );
    }

    /// Position order is not consulted: a released position after the borrowed
    /// one refuses exactly as one before it does, so the answer cannot be an
    /// artefact of the parameter loop's direction.
    #[test]
    fn the_decision_does_not_depend_on_position_order() {
        let (one, other) = (Binding::settled_at(1), Binding::settled_at(2));
        assert_eq!(
            same_call(Some(&one), &[Some(&other), Some(&one)]),
            SameCallBorrow::DestroyedByThisCall
        );
        assert_eq!(
            same_call(Some(&one), &[Some(&one), Some(&other)]),
            SameCallBorrow::DestroyedByThisCall
        );
    }

    /// A different identity at the destroying position is exactly the ordinary
    /// well-formed call, and it must keep its existing answer.
    #[test]
    fn a_distinct_identity_is_not_decided() {
        let (one, other) = (Binding::settled_at(1), Binding::settled_at(2));
        assert_eq!(
            same_call(Some(&one), &[Some(&other)]),
            SameCallBorrow::NotDecided
        );
    }

    /// A call with no destroying position at all — the overwhelming majority —
    /// is untouched.
    #[test]
    fn a_call_that_destroys_nothing_is_not_decided() {
        let one = Binding::settled_at(1);
        assert_eq!(
            same_call(Some(&one), &[None, None]),
            SameCallBorrow::NotDecided
        );
        assert_eq!(same_call(Some(&one), &[]), SameCallBorrow::NotDecided);
    }

    /// **The production shape.** `cio_chunk_close(chunk, delete)` is the one
    /// declaration in the pinned corpus carrying a released and a borrowed
    /// parameter at once, and its borrowed parameter is a scalar flag. §C04.2
    /// admits scalars in exactly that position, and an unbound actual is how
    /// every scalar reaches this decision.
    #[test]
    fn a_scalar_borrowed_actual_is_not_decided() {
        let one = Binding::settled_at(1);
        assert_eq!(same_call(None, &[Some(&one)]), SameCallBorrow::NotDecided);
    }

    /// Every way either side fails to settle fails closed, whatever the other
    /// side is.
    #[test]
    fn an_unsettled_binding_on_either_side_is_not_decided() {
        let settled = Binding::settled_at(1);
        let unsettled = [
            Binding {
                targets: BTreeSet::from([1]),
                unknown: true,
                may_be_null: false,
            },
            Binding {
                targets: BTreeSet::from([1]),
                unknown: false,
                may_be_null: true,
            },
            Binding {
                targets: BTreeSet::from([1, 2]),
                unknown: false,
                may_be_null: false,
            },
            Binding {
                targets: BTreeSet::new(),
                unknown: false,
                may_be_null: false,
            },
        ];
        for binding in &unsettled {
            assert_eq!(
                same_call(Some(binding), &[Some(&settled)]),
                SameCallBorrow::NotDecided,
                "an unsettled borrowed actual must never be decided"
            );
            assert_eq!(
                same_call(Some(&settled), &[Some(binding)]),
                SameCallBorrow::NotDecided,
                "an unsettled destroying actual must never decide anything"
            );
        }
    }

    /// The two decisions are independent: neither reads the other's inputs.
    /// The same-call rule consults no status bitmap at all, so an identity the
    /// path rule admits is still refused when this call destroys it.
    #[test]
    fn the_same_call_rule_reads_no_status() {
        let one = Binding::settled_at(1);
        assert_eq!(
            Fixture::settled(LIVE).decide(),
            BorrowLiveness::NotDecided,
            "the path rule admits a live identity"
        );
        assert_eq!(
            same_call(Some(&one), &[Some(&one)]),
            SameCallBorrow::DestroyedByThisCall,
            "…and the same-call rule still refuses it"
        );
    }
}
