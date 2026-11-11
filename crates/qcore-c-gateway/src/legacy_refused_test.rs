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
//! The provenance it reads is joined provenance over the **tie names**
//! ([`tie_names`], `LegacyCfg::tie_names`): the conditional names, plus each
//! unique local whose only `&` uses are member addresses through it
//! (`&conn->event`), which C reads as the address of a member of `*conn`,
//! never of `conn` (Monkey's `mk_sched_add_connection`). Only this rule reads
//! them ([`tie_facts`]); rule 10b's conditional release and every frame keep
//! the conditional names.
//!
//! It replaces MONKEY-VHOST-PROPERTY-CUSTODY's interim withhold rule (an
//! unresolved effect on every nullable, possibly-live site the binding may
//! denote, in `refine_pointer_null`), which needed no tie and so also
//! withdrew accusations established on other paths.
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
#[cfg(test)]
pub(crate) type EdgeVector = ((bool, bool, bool, bool), &'static str);

#[cfg(test)]
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
pub(super) fn settle_absent_edge(
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

/// The token that stands in for a member address's `&` when the tie names
/// are read: anything `frame_protected_locals` does not key on.
const MEMBER_ADDRESS: &str = "+";

/// `body` with the `&` of every member address through a name — `&NAME->F`,
/// `&(NAME)->F`, `&(NAME->F)` — replaced by [`MEMBER_ADDRESS`], and whether
/// any was. C reads the operand of that `&` as the postfix expression
/// `NAME->F` (C11 6.5.2.3, 6.5.3.2: `->` binds tighter than unary `&`), the
/// member of the object `NAME` points to, so no pointer to `NAME`'s own slot
/// is formed. Every other `&` (`&NAME`, `&NAME.F`, `&NAME[I]`, `&*NAME`) is
/// left for `frame_protected_locals` to read as it always did.
pub(crate) fn member_addresses_read<'a>(body: &[&'a str]) -> (Vec<&'a str>, bool) {
    let mut read = body.to_vec();
    let mut any = false;
    for index in 0..body.len() {
        if body[index] != "&" {
            continue;
        }
        let mut cursor = index + 1;
        let mut opened = 0usize;
        while body.get(cursor) == Some(&"(") {
            opened += 1;
            cursor += 1;
        }
        if !body.get(cursor).is_some_and(|name| is_identifier(name)) {
            continue;
        }
        cursor += 1;
        let mut closed = 0usize;
        while closed < opened && body.get(cursor) == Some(&")") {
            closed += 1;
            cursor += 1;
        }
        // The tokenizer spells `->` as `-` `>`; accept either spelling.
        let arrow = body.get(cursor) == Some(&"->")
            || (body.get(cursor) == Some(&"-") && body.get(cursor + 1) == Some(&">"));
        if arrow {
            read[index] = MEMBER_ADDRESS;
            any = true;
        }
    }
    (read, any)
}

/// The tie names (`LegacyCfg::tie_names`): the conditional names, plus every
/// unique local the body takes no address of once a member address through
/// it is read as C reads it. Exactly the conditional names when the body
/// spells no member address through a name.
pub(super) fn tie_names(
    tokens: &[Token],
    body: &[&str],
    formals: &BTreeSet<String>,
    unique: &BTreeSet<String>,
    conditional: &BTreeSet<String>,
) -> BTreeSet<String> {
    let (read, any) = member_addresses_read(body);
    if !any {
        return conditional.clone();
    }
    let unit = tokens
        .iter()
        .map(|token| token.text.as_str())
        .collect::<Vec<_>>();
    let protected = frame_protected_locals(&unit, &read, formals);
    unique
        .intersection(&protected)
        .filter(|name| !formals.contains(*name))
        .cloned()
        .chain(conditional.iter().cloned())
        .collect()
}

/// Joined provenance over the tie names, for rule 12e only. `None` when they
/// are the conditional names, whose provenance the work list already holds.
///
/// A tie name outside the conditional names is an unshadowed automatic local
/// whose own slot no `&` reaches, so the premise the call frame and the
/// indirect-store frame rest on (no other lvalue designates the slot) holds of
/// it exactly as of a conditional name; a refused statement still forgets it,
/// because its sparing frame is `frame_protected_locals`' as before.
pub(super) fn tie_facts(cfg: &LegacyCfg) -> Option<Vec<BTreeMap<usize, BTreeSet<String>>>> {
    if cfg.tie_names == cfg.conditional_names {
        return None;
    }
    let mut tie = cfg.clone();
    tie.conditional_names = cfg.tie_names.clone();
    Some(joined_provenance::derive(&tie))
}

#[cfg(test)]
#[path = "../tests/unit/legacy_refused_test.rs"]
mod tests;
