//! Card **SUP07-REFUSED-TEST-ACCUSATION**: the null edge of a refused test.
//!
//! # The weakness
//!
//! Rule 10b keeps a null test of an *unknown* binding (one that joins an
//! unresolved value with tracked sites, or denotes several) from deciding
//! anything but the tested binding itself: the test reports `UnknownAlias`,
//! narrows the name, and keeps every site and every other alias on both
//! edges. That is right for the *identity* question the test cannot answer,
//! but it leaves each site the name may denote `LIVE` on the **null** edge, and
//! `report_leaks` then accuses it at every exit that edge reaches. Monkey's
//! `mk_http_error_page` (`mk_server/mk_http.c:392-399`) is the measured case:
//! `temp` joins a refused `mk_ptr_to_buf` statement with a nullable
//! `mk_string_dup` copy, `if (!temp) return -1;` is refused, and the copy is
//! accused at the `return -1` although on that edge it is NULL or was never
//! made. The accusation's decisive fact (the copy is live on that edge) is
//! reachable only through the test the analyzer refused.
//!
//! # What this module reads
//!
//! The premise that settles it is joined provenance's (rule 10b's own
//! conditional provenance, `legacy_joined_provenance`): at the test, a site
//! `S` whose binding set contains the tested name `X` is one that `X`
//! denotes **on every path on which `S` exists**. On the null edge `X` is
//! NULL, so on every such path `S`'s acquisition returned NULL (no
//! obligation) or, for a non-nullable site, the path does not take that edge;
//! on every other path `S` does not exist. Either way `S` carries no
//! obligation on the null edge.
//!
//! [`absent_edge_source`] names the tested name on the edge that carries no
//! resource at it (the null edge of a null test, the invalid edge of a
//! validity test), exactly as `legacy_joined_provenance::outgoing` decides it,
//! and [`tied_sites`] reads the sites provenance ties to that name. Both are
//! pure readings, shared by the rule and by the leak-witness census.
//!
//! # The rule (gateway 12e)
//!
//! [`settle_absent_edge`] runs on a test's edge only when the older
//! refinement returned `PointerRefinement::Unknown`, after its `UnknownAlias`
//! and `unmodeled` are recorded. On the absent edge, each site the tested
//! binding may denote and provenance ties to the tested name is **settled**:
//! removed exactly as a decided null test's null edge removes the site its
//! binding denotes (`remove_null_site`). Nothing else changes: the identity
//! refusal stays, so the function is never `Safe` by this rule; the other
//! edge, an untied site and a decided test are left to the older refinement.
//!
//! It settles rather than marks an unresolved effect, because the marker is
//! joined by union: it would also withdraw a leak established on the test's
//! other edge where the two edges meet. And it needs the tie, because without
//! it the site may be live under no name on the null edge (an earlier
//! overwrite of the tested name), where the accusation is established.
//! `QCore.RefusedTestAccusation` proves both directions.
use super::*;

/// The tested name when `edge` is the edge of `instruction` on which that name
/// carries no resource: the null edge of a null test, or the invalid edge of a
/// validity test. `None` for every other edge and instruction.
///
/// The same decision `legacy_joined_provenance::outgoing` makes when it drops
/// the sites the tested name denotes from that edge's provenance.
pub(crate) fn absent_edge_source(instruction: &LegacyInstruction, edge: usize) -> Option<&str> {
    match instruction {
        LegacyInstruction::BranchOnNull {
            source,
            true_is_null,
        } if (edge == 0) == *true_is_null => Some(source),
        LegacyInstruction::BranchOnValidity {
            source,
            when_true,
            when_false,
        } if descriptor_custody::edge_fact(*when_true, *when_false, edge)
            == ValidityFact::Invalid =>
        {
            Some(source)
        }
        _ => None,
    }
}

/// The sites joined provenance ties to `source` at one node: present on the
/// node's incoming facts, with `source` among the names that denote the site
/// on every path on which it exists.
pub(crate) fn tied_sites(
    joined: &BTreeMap<usize, BTreeSet<String>>,
    source: &str,
) -> BTreeSet<usize> {
    joined
        .iter()
        .filter(|(_, names)| names.contains(source))
        .map(|(site, _)| *site)
        .collect()
}

/// What one edge of a test does to one site (gateway rule 12e). The model is
/// `QCore.RefusedTestAccusation.edgeStep`; [`EDGE_VECTORS`] is its
/// `edgeVectors`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EdgeStep {
    /// The site is as the older refinement left it.
    Keep,
    /// The site does not exist on this edge: removed exactly as a decided null
    /// test's null edge removes the site its binding denotes.
    Settle,
}

impl EdgeStep {
    #[cfg(test)]
    pub(crate) const fn code(self) -> &'static str {
        match self {
            Self::Keep => "keep",
            Self::Settle => "settle",
        }
    }
}

/// Rule 12e's decision for one site on one edge: settle exactly on a
/// **refused** test's **absent** (null or invalid) edge, for a site the tested
/// binding may denote (**targeted**) and joined provenance **ties** to the
/// tested name. Every other combination keeps the site: a decided test refines
/// as it always did, the other edge says nothing about the site's existence,
/// and an untied site may be live on that edge under no name at all (its
/// accusation then rests on established facts and stays).
pub(crate) const fn edge_step(refused: bool, absent: bool, tied: bool, targeted: bool) -> EdgeStep {
    if refused && absent && tied && targeted {
        EdgeStep::Settle
    } else {
        EdgeStep::Keep
    }
}

/// `(refused, absent, tied, targeted)` and the step, shared with
/// `QCore.RefusedTestAccusation.edgeVectors`.
pub(crate) type EdgeVector = ((bool, bool, bool, bool), &'static str);

pub(crate) const EDGE_VECTORS: &[EdgeVector] = &[
    ((false, false, false, false), "keep"),
    ((false, false, false, true), "keep"),
    ((false, false, true, false), "keep"),
    ((false, false, true, true), "keep"),
    ((false, true, false, false), "keep"),
    ((false, true, false, true), "keep"),
    ((false, true, true, false), "keep"),
    ((false, true, true, true), "keep"),
    ((true, false, false, false), "keep"),
    ((true, false, false, true), "keep"),
    ((true, false, true, false), "keep"),
    ((true, false, true, true), "keep"),
    ((true, true, false, false), "keep"),
    ((true, true, false, true), "keep"),
    ((true, true, true, false), "keep"),
    ((true, true, true, true), "settle"),
];

/// The work-list hook (rule 12e), called for an edge whose test the older
/// refinement could not decide (`PointerRefinement::Unknown`: the identity
/// refusal is already reported and the edge already carries `unmodeled`).
///
/// `before` is the node's output before the edge was refined (a test's
/// transfer is empty, so it is the test's own state); `joined` is joined
/// provenance's facts at the node. Each site the tested binding denotes in
/// `before` and provenance ties to the tested name is settled on the absent
/// edge; nothing else changes. Returns the settled sites.
///
/// A status witness's test (rule 11v) is not this rule's: its edges decide
/// another name's storage and are left as they are.
pub(crate) fn settle_absent_edge(
    instruction: &LegacyInstruction,
    edge: usize,
    before: &AbstractState,
    joined: &BTreeMap<usize, BTreeSet<String>>,
    edge_output: &mut AbstractState,
) -> BTreeSet<usize> {
    let Some(source) = absent_edge_source(instruction, edge) else {
        return BTreeSet::new();
    };
    if in_place_custody::witnessed_storage(before, source).is_some() {
        return BTreeSet::new();
    }
    let targets = before
        .pointers
        .get(source)
        .map(|pointer| pointer.targets.clone())
        .unwrap_or_default();
    let tied = tied_sites(joined, source);
    let settled = targets
        .iter()
        .copied()
        .filter(|site| edge_step(true, true, tied.contains(site), true) == EdgeStep::Settle)
        .collect::<BTreeSet<_>>();
    for &site in &settled {
        remove_null_site(edge_output, site);
    }
    settled
}

#[cfg(test)]
#[path = "../tests/unit/legacy_refused_test.rs"]
mod tests;
