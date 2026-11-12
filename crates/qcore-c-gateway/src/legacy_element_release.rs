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

//! Card **LIST-ENTRY-RELEASE-IDENTITY** (gateway rule 43): the released list
//! element's identity at an element destroyer, and the released-element
//! summary.
//!
//! Rule 10f(2) gives a body custody of an element of a container rooted at an
//! unrebound formal when it unlinks it (`mk_list_del`), and rule 34 lets an
//! element destroyer (`resource_unpublished(p)` beside `resource_released(p)`)
//! take that custody and end it in one call. Rule 10f(3) then withholds every
//! summary of such a body: a caller would read `Borrowed` for the container's
//! formal and keep restoring its own published objects the callee released.
//! This module adds two things.
//!
//! 1. **A held object at an element destroyer.** Rule 34 admitted only the
//!    entries rule 10d/10f's unlink admits (a published site restored, an
//!    admitted foreign element). An entry that denotes one live, unpublished
//!    identity this path already holds, and that no arriving path unlinked
//!    ([`destroyer_takes_held`]), is admitted too: the unlink takes nothing
//!    (the body holds the object), and the call's own release ends exactly
//!    that custody. A plain unlink of such an object keeps rule 10d's
//!    refusal, and an object any path already unlinked keeps it as well (a
//!    second unlink of one link is not a removal: `mk_list_del` clears the
//!    links, so the destroyer's own unlink would read them cleared).
//! 2. **The released-element summary** (`W45-FOREIGN-UNLINK-SUMMARY`). A body
//!    whose every element custody is taken from containers rooted at its
//!    formals — each foreign unlink's [`super::Foreign::roots`], each applied
//!    summary clause's [`super::Operand::ElementsReleased`] root — publishes
//!    its summary with `ContainerEffect::ElementsReleased` at exactly those
//!    formals ([`required_effects`], [`attach`], [`gate`]). A root that is a
//!    static object, a local, a callee's container or anything else leaves
//!    the summary withheld, as before. At a call the clause is applied after
//!    the call's own reading ([`forget_published`]): every site the caller had
//!    published leaves the published set and is withdrawn (an unresolved
//!    effect), since the call may have released it; a later unlink, release
//!    or use of one refuses. A caller whose actual is rooted at its own formal
//!    carries the clause on in its own summary; any other actual withholds
//!    the caller's summary. Every consumer that reads a call as a borrow
//!    already refuses a contract with a container effect
//!    (`has_container_effect`), so the clause is never read as `Borrowed`.
//!
//! Premises, stated and not proved: P-CRR-MEMBER and P-CRR-LINKED (rule 10f),
//! and P-LERI-CALLER-PUBLISHED — the only objects a caller tracks that a
//! container its actual reaches may hold are those it published there (an
//! object it holds `LIVE` was never linked by a modelled step, and an
//! unmodelled link refuses). The Lean model is
//! `QCore.ElementReleaseSummary`.

use std::collections::BTreeMap;
#[cfg(test)]
use std::collections::BTreeSet;

use qcore_schemas::contract::{ContainerEffect, ResourceFunctionContract};

use super::{AbstractState, Container, LIVE, Operand, mark_unresolved_effect};

/// The rule's id in `crates/gateway-rules/registry.toml`.
pub(crate) const RULE: &str = "43";

/// Whether rule 43 is on (`QCORE_GATEWAY_RULES_OFF`).
fn enabled() -> bool {
    crate::gateway_rules::enabled(RULE)
}

/// Part 1: whether an element destroyer may take `site`, which its entry
/// denotes as one settled identity: live, unpublished, and unlinked on no
/// arriving path.
pub(in crate::legacy_c) fn destroyer_takes_held(state: &AbstractState, site: usize) -> bool {
    enabled()
        && state.resources.get(&site) == Some(&LIVE)
        && !state.published.contains(&site)
        && !state.unlinked.contains(&site)
}

/// Part 2, at a call: the call may have released every object this path
/// published, so none of them is published any longer, and each is withdrawn.
pub(in crate::legacy_c) fn forget_published(state: &mut AbstractState) {
    let published = std::mem::take(&mut state.published);
    if published.is_empty() {
        return;
    }
    let withdrawn = super::super::AbstractPointer {
        targets: published,
        ..Default::default()
    };
    mark_unresolved_effect(state, &withdrawn);
}

/// Whether `cfg` takes element custody: a foreign unlink, or a call whose
/// summary states `ElementsReleased`.
pub(crate) fn has_element_release(cfg: &super::super::LegacyCfg) -> bool {
    super::has_foreign_unlink(cfg)
        || operands(cfg).any(|operand| matches!(operand, Operand::ElementsReleased { .. }))
}

fn operands(cfg: &super::super::LegacyCfg) -> impl Iterator<Item = &Operand> {
    cfg.nodes.iter().flat_map(|node| match &node.instruction {
        super::LegacyInstruction::ContractCall {
            container: Some(operands),
            ..
        } => operands.0.iter(),
        _ => [].iter(),
    })
}

/// The `ElementsReleased` clauses `cfg`'s element custody requires of its
/// summary, by the position of each root formal in `parameters`; `None` when
/// some element custody is not rooted at a formal (the summary is withheld),
/// or when rule 43 is off.
pub(crate) fn required_effects(
    cfg: &super::super::LegacyCfg,
    parameters: &[Option<String>],
) -> Option<BTreeMap<usize, ContainerEffect>> {
    if !enabled() {
        return None;
    }
    let mut roots = Vec::new();
    for operand in operands(cfg) {
        match operand {
            Operand::Unpublished {
                foreign: Some(foreign),
                ..
            } => match &foreign.roots {
                Some(names) => roots.extend(names.iter().cloned().map(Root::Formal)),
                None => roots.push(Root::Unnamed),
            },
            Operand::ElementsReleased {
                container: Container::CallerFormal(_),
                root: Some(root),
            } => roots.push(Root::Formal(root.clone())),
            Operand::ElementsReleased { .. } => roots.push(Root::Unnamed),
            Operand::Published { .. } | Operand::Unpublished { foreign: None, .. } => {}
        }
    }
    required(&roots, parameters)
}

/// Where one element custody is taken from (`QCore.ElementReleaseSummary.Root`):
/// a container rooted at the named formal, or anything a clause cannot name
/// (a static object, a local, a callee's container).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Root {
    Formal(String),
    Unnamed,
}

/// `QCore.ElementReleaseSummary.required`: the positions of the root formals,
/// or `None` when one root is not a formal of `parameters`.
pub(crate) fn required(
    roots: &[Root],
    parameters: &[Option<String>],
) -> Option<BTreeMap<usize, ContainerEffect>> {
    let position = |root: &str| {
        parameters
            .iter()
            .position(|name| name.as_deref() == Some(root))
    };
    roots
        .iter()
        .map(|root| match root {
            Root::Formal(name) => Some((position(name)?, ContainerEffect::ElementsReleased)),
            Root::Unnamed => None,
        })
        .collect()
}

/// The `ElementsReleased` clauses of `contract`.
fn stated(contract: &ResourceFunctionContract) -> BTreeMap<usize, ContainerEffect> {
    contract
        .container_effects
        .iter()
        .filter(|(_, effect)| matches!(effect, ContainerEffect::ElementsReleased))
        .map(|(&index, &effect)| (index, effect))
        .collect()
}

/// The W4 gate's condition for element custody: a body that takes none
/// states no clause; a body that takes some states exactly the clauses
/// [`required_effects`] reads, and is refused when it reads none (rule
/// 10f(3)'s withheld summary).
pub(crate) fn gate(
    contract: &ResourceFunctionContract,
    cfg: &super::super::LegacyCfg,
    parameters: &[Option<String>],
) -> bool {
    if has_element_release(cfg) {
        required_effects(cfg, parameters).is_some_and(|required| required == stated(contract))
    } else {
        stated(contract).is_empty()
    }
}

/// Attach the clauses a candidate summary must state; `None` when the body's
/// element custody is not expressible, so no summary is published.
pub(crate) fn attach(
    candidate: &mut ResourceFunctionContract,
    cfg: &super::super::LegacyCfg,
    parameters: &[Option<String>],
) -> Option<()> {
    if has_element_release(cfg) {
        let required = required_effects(cfg, parameters)?;
        if required
            .keys()
            .any(|index| candidate.container_effects.contains_key(index))
        {
            return None;
        }
        candidate.container_effects.extend(required);
    }
    Some(())
}

/// The vectors `QCore.ElementReleaseSummary.vectors_hold` checks of the Lean
/// `required`: roots by position (`Some(p)` a formal at `p`, `None` a static
/// or other root) and the required positions (`None`: withheld).
#[cfg(test)]
type Vector = (&'static [Option<usize>], Option<&'static [usize]>);

/// The vectors themselves.
#[cfg(test)]
pub(crate) const ELEMENT_RELEASE_VECTORS: &[Vector] = &[
    (&[], Some(&[])),
    (&[Some(0)], Some(&[0])),
    (&[Some(0), Some(0)], Some(&[0, 0])),
    (&[Some(1), Some(0)], Some(&[1, 0])),
    (&[Some(0), None], None),
    (&[None], None),
    (&[None, Some(0)], None),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_lean_vectors_hold() {
        let parameters = vec![Some("p0".to_owned()), Some("p1".to_owned())];
        for (roots, expected) in ELEMENT_RELEASE_VECTORS {
            let roots = roots
                .iter()
                .map(|root| match root {
                    Some(p) => Root::Formal(format!("p{p}")),
                    None => Root::Unnamed,
                })
                .collect::<Vec<_>>();
            let got =
                required(&roots, &parameters).map(|map| map.into_keys().collect::<BTreeSet<_>>());
            let want = expected.map(|positions| positions.iter().copied().collect::<BTreeSet<_>>());
            assert_eq!(got, want, "{roots:?}");
        }
    }

    #[test]
    fn a_root_that_is_no_parameter_withholds_the_summary() {
        let parameters = vec![Some("q".to_owned())];
        assert_eq!(required(&[Root::Formal("other".into())], &parameters), None);
    }
}
