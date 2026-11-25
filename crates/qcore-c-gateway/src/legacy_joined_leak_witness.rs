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

//! Card **SETTLED-LEAK-JOINED-GENUINE-WITHDRAWN** (gateway rule 84): a leak
//! rule 74c (A) withdraws is published again when a path avoiding every mark
//! of its identity still carries it.
//!
//! Rule 74c (A) withdraws a held leak report when the settled exit marks the
//! identity unresolved, a refused release is among the marks, and the
//! function carries an `Unknown` leaf. The union join cannot tell a path
//! through the refused release from another path to the same exit, so a
//! genuine leak on the other path was withdrawn with it
//! (`int fd = k_open(path); if (flag && k_close(fd) == 0) {}`: the
//! `flag`-false path leaves a valid descriptor open).
//!
//! Whether an iterate claimed that leak at all depended on the work-list
//! schedule (rule 6): `if (flag) { k_close(fd); }` was never claimed, because
//! the marked arm reached the exit before it was first processed. [`settle`]
//! therefore takes as candidates every report (A) withdraws, held or not: at
//! each exit, the current-generation identities its settled incoming state
//! marks with a refused release among the marks, beside an `Unknown` leaf.
//!
//! [`Replay::witness`] keeps the marks path-separated. It runs the custody
//! work list again for one identity, with the same transfers, edge
//! refinements, joins and schedule as `analyze_cfg_with_entry_collected`,
//! except that an edge whose state marks the identity unresolved, or carries
//! any refusal (`unmodeled`), is not propagated. The restricted states are
//! then joins over exactly the CFG paths on which nothing marked the identity
//! and nothing was refused. Every refusing branch of a release marks its
//! targets (`mark_unresolved_effect`), so none of those paths passes a refused
//! release of it. None passes any other marking refusal of it (a refused
//! test's null edge, a refused hand-on, a refused re-acquisition, a refused
//! escape), any refused statement, an undecided test's edge or a retained
//! infeasible loop exit either: each of those sets `unmodeled`. Without the
//! refusal cut, a refused guard's null edge (rule 10b) carried a site live
//! that the test would have found NULL, the shape rule 74c (A) withdrew in
//! `rd_kafka_mock_handle_Metadata` and `ampersand_run_parity` pins. The
//! report is published only when both hold:
//!
//! * the restricted settled exit claims it by `report_leaks`' own conditions
//!   (`LIVE`, not exempt, not marked, no reference family) and is not
//!   `unmodeled`, read on the exit node's output as the work list reads it;
//! * a path from a producer of the identity to that exit keeps it `LIVE`,
//!   unmarked and unrefused on every restricted edge and node output it takes
//!   (the leak-witness census's search, over the restricted states).
//!
//! Otherwise the report stays withdrawn. The premises are the ones every
//! published leak already rests on (the analyzer's path-feasibility reading:
//! every CFG edge an edge refinement does not prove infeasible is taken, and a
//! `LIVE` bit at the exit is the may-evidence `report_leaks` publishes), plus
//! that the replay is the work list's (checked under `debug_assertions`: the
//! uncut replay must reproduce the settled states exactly). Only
//! `Unknown -> Unsafe` moves follow (or an extra `Leak` leaf in a row already
//! `Unsafe`). Nothing becomes `Safe`.
//!
//! The Lean model is `QCore.SettledLeakJoined`
//! (`proof/QCore/SettledLeakJoined.lean`); [`PUBLISH_VECTORS`] is shared with
//! it.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use qcore_schemas::verdict::Verdict;

use super::{
    AbstractState, EntryCustody, LIVE, LegacyCfg, LegacyDiagnostic, LegacyDiagnosticKind,
    NodeAliasFacts, PointerRefinement, TransferContext, field_link_join_route,
    instruction_produced_sites, is_path_poisoning, joined_provenance, loop_generations_route,
    refine_collection_edge, refine_edge, refused_operands, refused_test, transfer,
};

/// The decision (`QCore.SettledLeakJoined.publishes`): a held report is
/// published unless rule 74c (A) withdraws it and no avoiding path witnesses
/// it.
pub(super) const fn publishes(withdrawn: bool, witnessed: bool) -> bool {
    !withdrawn || witnessed
}

/// `(withdrawn by rule 74c (A), witnessed, published)`, shared with
/// `QCore.SettledLeakJoined.publishVectors` row by row.
#[cfg(test)]
pub(super) const PUBLISH_VECTORS: [(bool, bool, bool); 4] = [
    (false, false, true),
    (false, true, true),
    (true, false, false),
    (true, true, true),
];

/// Everything the custody work list read, so it can be replayed.
pub(super) struct Replay<'a, 'c> {
    pub(super) cfg: &'a LegacyCfg,
    pub(super) context: &'a TransferContext<'c>,
    pub(super) alias: &'a [NodeAliasFacts],
    pub(super) joined: &'a [BTreeMap<usize, BTreeSet<String>>],
    pub(super) tie: Option<&'a [BTreeMap<usize, BTreeSet<String>>]>,
    pub(super) acquisition_lines: &'a BTreeMap<usize, usize>,
    pub(super) entry: &'a EntryCustody,
}

/// One node's output and per-edge outputs, as the work list computes them.
struct Step {
    output: AbstractState,
    edges: Vec<Option<AbstractState>>,
}

fn live(state: &AbstractState, site: usize) -> bool {
    state
        .resources
        .get(&site)
        .is_some_and(|status| status & LIVE != 0)
}

/// No mark of `site` and no refusal at all lies on the paths `state` joins:
/// the identity is not marked unresolved, and the state is not `unmodeled`
/// (no instruction on the path was refused or reported an unresolved effect,
/// no edge taken was an undecided test or a retained infeasible loop exit).
fn clean(state: &AbstractState, site: usize) -> bool {
    !state.unmodeled && !state.unresolved_effect_sites.contains(&site)
}

impl Replay<'_, '_> {
    /// The work list's transfer of `node_id` from `state` and its edge
    /// outputs, exactly as `analyze_cfg_with_entry_collected` computes them
    /// (diagnostics discarded). An edge the refinement removes is `None`.
    fn step(&self, node_id: usize, state: AbstractState) -> Step {
        let cfg = self.cfg;
        let context = self.context;
        let node = &cfg.nodes[node_id];
        let mut local = BTreeSet::new();
        let (state, shifted) =
            loop_generations_route::shift(&cfg.function, node, state, &mut local);
        let conditional_release =
            joined_provenance::release_site(node, &state, &self.joined[node_id]);
        let handed = refused_operands::read_sites(&state, &cfg.publications.line_reads, node);
        let mut output = transfer(
            &cfg.function,
            node,
            state,
            self.acquisition_lines,
            context,
            &self.alias[node_id],
            &mut local,
        );
        joined_provenance::finish_release(conditional_release, &mut output, &local);
        refused_operands::apply(&mut output, &handed, &local);
        loop_generations_route::settle_refused(&shifted, &mut output, &local);
        field_link_join_route::prune(&mut output);
        output.unmodeled |= local.iter().any(is_path_poisoning);
        let edges = (0..node.successors.len())
            .map(|edge| {
                let mut edge_output = output.clone();
                if let Some(step) = cfg.cell_tests.get(&node_id) {
                    super::descriptor_cells_route::refine_protocol(step, edge, &mut edge_output);
                }
                if let Some((_, refinement)) = refine_edge(
                    &node.instruction,
                    edge,
                    &mut edge_output,
                    &context.site_kinds,
                ) {
                    let refinement = super::borrowed_path_local_route::guard_refinement(
                        refinement,
                        &node.instruction,
                        &output,
                        &context.traversal.borrowed_path,
                    );
                    match refinement {
                        PointerRefinement::Infeasible if node.loop_exit_edge == Some(edge) => {
                            edge_output = output.clone();
                            edge_output.unmodeled = true;
                        }
                        PointerRefinement::Infeasible => return None,
                        PointerRefinement::Unknown => {
                            edge_output.unmodeled = true;
                            refused_test::settle_absent_edge(
                                &node.instruction,
                                edge,
                                &output,
                                self.tie.map_or(&self.joined[node_id], |tie| &tie[node_id]),
                                &mut edge_output,
                            );
                        }
                        PointerRefinement::Refined => {}
                    }
                }
                refine_collection_edge(node, edge, &mut edge_output, &context.collection_owners())
                    .then_some(edge_output)
            })
            .collect();
        Step { output, edges }
    }

    /// The work list's fixed point, except that an edge whose state marks
    /// `cut` unresolved is not propagated. With `cut` `None` it is the work
    /// list's own fixed point, schedule included.
    fn fixed_point(&self, cut: Option<usize>) -> Vec<Option<AbstractState>> {
        let cfg = self.cfg;
        let mut incoming = vec![None::<AbstractState>; cfg.nodes.len()];
        incoming[cfg.entry] = Some(self.entry.state.clone());
        let mut work = VecDeque::from([cfg.entry]);
        while let Some(node_id) = work.pop_front() {
            let Some(state) = incoming[node_id].clone() else {
                continue;
            };
            let step = self.step(node_id, state);
            for (edge, edge_output) in step.edges.into_iter().enumerate() {
                let Some(edge_output) = edge_output else {
                    continue;
                };
                if cut.is_some_and(|site| !clean(&edge_output, site)) {
                    continue;
                }
                let successor = cfg.nodes[node_id].successors[edge];
                let changed = match &mut incoming[successor] {
                    Some(current) => current.join(&edge_output, &cfg.traversal.scalars),
                    slot @ None => {
                        *slot = Some(edge_output);
                        true
                    }
                };
                if changed {
                    work.push_back(successor);
                }
            }
        }
        incoming
    }

    /// Whether the restricted exit claims `site` by `report_leaks`' own
    /// conditions.
    fn claims(&self, output: &AbstractState, site: usize) -> bool {
        live(output, site)
            && clean(output, site)
            && !self.entry.exempt_sites(output).contains(&site)
            && !output.unresolved_effect_sites.contains(&site)
            && !output.references.contains_key(&site)
    }

    /// A path from a producer of `site` to `exit` that no mark of `site`
    /// lies on and that keeps it live, or `None`. `settled` is the work
    /// list's own fixed point, read only by the `debug_assertions` check that
    /// the replay is the work list's.
    pub(super) fn witness(
        &self,
        site: usize,
        exit: usize,
        settled: &[Option<AbstractState>],
    ) -> Option<Vec<usize>> {
        if exit >= self.cfg.nodes.len() || self.cfg.entry >= self.cfg.nodes.len() {
            return None;
        }
        debug_assert!(
            self.fixed_point(None) == settled,
            "the replay must reproduce the custody work list"
        );
        let incoming = self.fixed_point(Some(site));
        let steps = incoming
            .iter()
            .enumerate()
            .map(|(node, state)| state.clone().map(|state| self.step(node, state)))
            .collect::<Vec<_>>();
        let arrives = |node: usize| {
            steps[node].as_ref().is_some_and(|step| {
                if node == exit {
                    self.claims(&step.output, site)
                } else {
                    live(&step.output, site) && clean(&step.output, site)
                }
            })
        };
        let base = crate::loop_generations::base(site);
        let mut previous = BTreeMap::<usize, Option<usize>>::new();
        let mut queue = VecDeque::new();
        for (node, n) in self.cfg.nodes.iter().enumerate() {
            if instruction_produced_sites(&n.instruction).contains(&base)
                && arrives(node)
                && !previous.contains_key(&node)
            {
                previous.insert(node, None);
                queue.push_back(node);
            }
        }
        while let Some(node) = queue.pop_front() {
            if node == exit {
                let mut path = vec![node];
                let mut cursor = previous.get(&node).copied().flatten();
                while let Some(current) = cursor {
                    path.push(current);
                    cursor = previous.get(&current).copied().flatten();
                }
                path.reverse();
                return Some(path);
            }
            let Some(step) = &steps[node] else {
                continue;
            };
            for (edge, edge_output) in step.edges.iter().enumerate() {
                let successor = self.cfg.nodes[node].successors[edge];
                let carries = edge_output
                    .as_ref()
                    .is_some_and(|state| live(state, site) && clean(state, site));
                if carries && !previous.contains_key(&successor) && arrives(successor) {
                    previous.insert(successor, Some(node));
                    queue.push_back(successor);
                }
            }
        }
        None
    }
}

/// Publish every leak key rule 74c (A) withdraws, or would withdraw, at a
/// settled exit that a path avoiding every mark of its identity carries.
///
/// The candidates are the current-generation identities an exit's settled
/// incoming state marks unresolved with a refused release among the marks,
/// beside an `Unknown` leaf: exactly the reports rule 74c (A) withdraws,
/// whether or not an iterate claimed them. Whether the work list claimed one
/// depends on its schedule (rule 6: a joined path that arrives before the exit
/// is first processed hides the claim), so each candidate is decided by
/// [`Replay::witness`] alone and the decision reads settled facts only.
pub(super) fn settle(
    replay: &Replay<'_, '_>,
    settled: &[Option<AbstractState>],
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    let unknown_leaf = diagnostics
        .iter()
        .any(|diagnostic| super::verdict::diagnostic_verdict(&diagnostic.kind) == Verdict::Unknown);
    if !unknown_leaf {
        return;
    }
    let mut reports = BTreeSet::new();
    for (exit, node) in replay.cfg.nodes.iter().enumerate() {
        let Some(state) = settled.get(exit).and_then(Option::as_ref) else {
            continue;
        };
        if !node.is_exit {
            continue;
        }
        for &site in state
            .refused_release_sites
            .intersection(&state.unresolved_effect_sites)
        {
            if crate::loop_generations::generation(site)
                != crate::loop_generations::Generation::Current
            {
                continue;
            }
            let key = (
                node.line,
                super::acquisition_line(replay.acquisition_lines, site),
            );
            // A candidate is one rule 74c (A) withdraws.
            if !reports.contains(&key)
                && publishes(true, replay.witness(site, exit, settled).is_some())
            {
                reports.insert(key);
            }
        }
    }
    for (line, acquisition_line) in reports {
        diagnostics.insert(LegacyDiagnostic {
            function: replay.cfg.function.clone(),
            line,
            kind: LegacyDiagnosticKind::Leak { acquisition_line },
        });
    }
}

#[cfg(test)]
#[path = "../tests/unit/legacy_joined_leak_witness.rs"]
mod tests;
