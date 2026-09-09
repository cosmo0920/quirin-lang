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
//! [`borrowed_actual_liveness`] answers [`BorrowLiveness::DefinitelyReleased`]
//! only under a conjunction of *must* facts, all read from the settled state:
//!
//! 1. the name is **bound with a settled denotation** — exactly one target
//!    site, not `unknown`, not `may_be_null` — reusing ALIAS-LOCALS's
//!    [`crate::may_alias::Denotation`] machinery, so the projection has
//!    committed to one identity on every reaching path;
//! 2. that identity carries **no unresolved effect**, so the projection has not
//!    already declined to commit to what happened to it;
//! 3. its joined status is **exactly `RELEASED`** — no `LIVE`, no `ESCAPED`, no
//!    `CONTAINED` bit anywhere. Because the status join is a bitwise union,
//!    the absence of every other bit means no reaching path leaves the identity
//!    in any other state.
//!
//! Every other shape is [`BorrowLiveness::NotDecided`] and keeps the existing
//! answer byte-for-byte: an unbound name (**every scalar `Borrowed` actual is
//! one** — §C04.2 admits scalars in exactly this position and nowhere else), an
//! unknown, ambiguous or possibly-null binding, a live identity, an escaped one
//! (custody moved is not custody destroyed), any mixed status, an identity this
//! function never acquired, and any site under an unresolved effect.
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
//! # What the refusal does not touch
//!
//! Nothing here writes to `AbstractState`, so `C02-P-ALIAS-LEAK-ORDER` is
//! undisturbed by construction: the custody work list, its fixed point, the
//! leak inventory and every per-exit residual are unchanged. The rule runs as a
//! read-only pass over the **settled** incoming states, after the work list is
//! stable, for the same reason ALIAS-LOCALS put its alias facts in a pre-pass —
//! a *must* fact read on an intermediate iterate would depend on scheduling.

use std::collections::{BTreeMap, BTreeSet};

use crate::may_alias::Denotation;

/// What the projection's own bindings say about one `Borrowed` actual.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BorrowLiveness {
    /// The projection committed to exactly one identity for this name, and that
    /// identity is released on every path reaching the call. The precondition
    /// this card implements is violated.
    DefinitelyReleased,
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

/// The status bitmap of an identity that is released on every reaching path.
///
/// Kept as a local constant rather than importing the analyzer's flags, so the
/// decision below is stated over one literal value and the Lean model can pin
/// the same one. `legacy_c::RELEASED` is asserted equal to it by a test there.
pub(crate) const RELEASED_ONLY: u8 = 2;

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
    if site.resources.get(&identity) == Some(&RELEASED_ONLY) {
        BorrowLiveness::DefinitelyReleased
    } else {
        BorrowLiveness::NotDecided
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIVE: u8 = 1;
    const RELEASED: u8 = 2;
    const ESCAPED: u8 = 4;
    const CONTAINED: u8 = 8;

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
    fn a_settled_name_denoting_a_released_identity_is_definitely_released() {
        assert_eq!(
            Fixture::settled(RELEASED).decide(),
            BorrowLiveness::DefinitelyReleased
        );
    }

    #[test]
    fn the_released_only_status_is_the_analyzer_flag() {
        assert_eq!(RELEASED_ONLY, RELEASED);
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
    /// obligation "without claiming that the callee destroys it" (§C04.2).
    #[test]
    fn an_escaped_identity_is_not_decided() {
        assert_eq!(
            Fixture::settled(ESCAPED).decide(),
            BorrowLiveness::NotDecided
        );
        assert_eq!(
            Fixture::settled(ESCAPED | RELEASED).decide(),
            BorrowLiveness::NotDecided
        );
    }

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
        assert_eq!(fixture.decide(), BorrowLiveness::DefinitelyReleased);
        fixture.unresolved.insert(1);
        assert_eq!(fixture.decide(), BorrowLiveness::NotDecided);
    }
}
