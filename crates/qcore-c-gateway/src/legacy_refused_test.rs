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
pub(crate) fn tied_sites(joined: &BTreeMap<usize, BTreeSet<String>>, source: &str) -> BTreeSet<usize> {
    joined
        .iter()
        .filter(|(_, names)| names.contains(source))
        .map(|(site, _)| *site)
        .collect()
}
