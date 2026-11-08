//! Card **W44-LEAK-PIN-AUDIT**: the leak-witness census.
//!
//! A **measurement**, not a rule, behind the off-by-default `indirect-census`
//! feature exactly like [`crate::indirect_census`]: with the feature off none
//! of this exists in the compiled crate, and with it on nothing here is read by
//! any analysis. It is called once per analysed CFG, after the custody fixed
//! point has settled and every diagnostic has been recorded, and it writes one
//! JSON line per `Leak` diagnostic that analysis reported.
//!
//! # What a witness is
//!
//! `report_leaks` publishes a site and an exit line, never a path. For each
//! leak this census reconstructs the analyzer's own evidence for it from the
//! **settled** states: it re-runs each reachable node's transfer and edge
//! refinement exactly as the work list does, and searches for a CFG path from
//! the node that produced the site to the reporting exit along which the
//! settled state keeps the site `LIVE` on every edge taken. Because states are
//! joined, such a path is the projection's may-evidence, not a proof that one
//! execution follows it; the card's classification reads each one against the
//! source.
//!
//! Three paths are searched: the shortest one; the shortest **clean** one,
//! which additionally avoids every node that refused its statement while that
//! statement's value side handed on a place bound to the site (the population
//! [`super::refused_operands`] withdraws); and the shortest **strict** one,
//! which also avoids every refused node that passes such a place, or its
//! address, to a call. A leak with no clean path is one every witness of which
//! passes a refused store or return of the leaked identity; a leak with a
//! clean path but no strict one rests on what an unmodelled callee does with
//! it. A fourth, the **handed** path, is the refused hand-on rule's exact
//! population under the union join: producer → a refused node whose hand-on
//! marks the site (the `withdraw` cascade included) → the exit over feasible
//! edges. A leak with a handed path is one the rule withdraws.
//!
//! It also records whether the leak is reproduced at the settled exit at all:
//! `report_leaks` runs on intermediate iterates of the work list
//! (`C02-P-ALIAS-LEAK-ORDER`), so a leaf may be reported on an iterate that the
//! settled state no longer supports.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::sync::Mutex;

use super::{
    AbstractState, EntryCustody, LIVE, LegacyCfg, LegacyDiagnostic, LegacyDiagnosticKind,
    LegacyInstruction, NodeAliasFacts, PointerRefinement, TransferContext,
    instruction_produced_sites, joined_provenance, refine_collection_edge, refine_edge,
    refused_operands, refused_test, transfer,
};

/// The environment variable naming the witness stream. Read only with the
/// `indirect-census` feature on; unset, [`record`] returns at once.
pub const LEAK_WITNESS_PATH_VARIABLE: &str = "QCORE_LEAK_WITNESS";

static WITNESSES: Mutex<Option<Option<BufWriter<File>>>> = Mutex::new(None);
static INVOCATION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// One node of a witness path.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Step {
    pub node: usize,
    pub line: usize,
    pub instruction: String,
    /// The node's own transfer reported `Unsupported`.
    pub refused: bool,
    /// The node refused its statement and the statement's value side handed
    /// on a place bound to the leaked site (the refused-escape rule's
    /// population).
    pub refused_read: bool,
    /// The node refused its statement and the statement passed a place bound
    /// to the leaked site, or its address, to a call.
    pub refused_call: bool,
    /// The edge this path takes out of the node was a pointer refinement the
    /// analyzer could not decide (`UnknownAlias` on the test).
    pub edge_unknown: bool,
    /// Card SUP07-REFUSED-TEST-ACCUSATION: that edge is the null (invalid)
    /// edge of a refused test whose tested binding may denote the site.
    pub edge_refused_null: bool,
    /// ... and joined provenance ties the site to the tested name there.
    pub edge_tied: bool,
    /// Places bound to the site in the node's settled incoming state.
    pub bound: Vec<String>,
}

/// One reported leak and its reconstructed evidence.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Witness {
    pub unit_seq: u64,
    pub invocation: u64,
    pub function: String,
    /// Every leak key this invocation reported, so a reader can match the
    /// invocation whose diagnostics were published.
    pub reported: Vec<(usize, usize)>,
    pub line: usize,
    pub acquisition_line: usize,
    pub exit_node: Option<usize>,
    pub site: Option<usize>,
    /// Card LOOP-REACQUISITION-LEAK-ACCUSATION: which generation of its
    /// acquisition site `site` names (`current`, `previous` or `older`,
    /// `crate::loop_generations::generation`). One witness per reported leak
    /// is the current generation's, as before; the earlier generations'
    /// witnesses are emitted beside it when the settled exit keeps them live
    /// or a path reaches the exit.
    pub generation: &'static str,
    /// The settled analysis refuses this site's re-acquisition
    /// (`loop re-acquisition: an earlier generation that may be live is
    /// still held`).
    pub reacquisition_refused: bool,
    /// How many nodes of `path` after its first acquire the site again: an
    /// earlier generation's witness passes at least one re-acquisition.
    pub path_reacquisitions: Option<usize>,
    /// The settled exit state still reports this leak.
    pub settled: bool,
    /// The site's status bits at the settled exit (`LIVE` 1, `RELEASED` 2,
    /// `ESCAPED` 4, `CONTAINED` 8).
    pub status: u8,
    /// The site carries an unresolved effect at the settled exit.
    pub unresolved: bool,
    /// The site is exempt at the settled exit (a caller-held or out-slot site).
    pub exempt: bool,
    pub exit_unmodeled: bool,
    pub exit_widened: bool,
    pub bound_at_exit: Vec<String>,
    pub path: Option<Vec<Step>>,
    pub clean_path: Option<Vec<Step>>,
    /// A path avoiding every refused node that hands the site on at all, as a
    /// value, an address or a call argument.
    pub strict_path: Option<Vec<Step>>,
    /// The refused hand-on rule's exact population under the union join: a
    /// path from a producer of the site to a refused node whose hand-on marks
    /// the site (directly or through the `withdraw` cascade over `contained`
    /// and `field_links`), and on over feasible edges to the exit. With the
    /// rule on, the unresolved effect this node adds reaches the exit along
    /// that path and the leak is not claimed. `None` means no such node.
    pub handed_path: Option<Vec<Step>>,
    /// Card SUP07-REFUSED-TEST-ACCUSATION: a path avoiding every null
    /// (invalid) edge of a refused test whose tested binding may denote the
    /// site. A leak with a path but no such path is one whose every witness
    /// keeps the site live only because a refused test did not decide it.
    pub untested_path: Option<Vec<Step>>,
    /// A path avoiding only those of the edges above at which joined
    /// provenance ties the site to the tested name.
    pub untied_path: Option<Vec<Step>>,
}

/// Card SUP07-REFUSED-TEST-ACCUSATION: one refused test's null (invalid)
/// edge, recorded for every analysed body (leaks or none), so the population
/// the rule reads is sized independently of the leak leaves.
#[derive(Clone, Debug, serde::Serialize)]
pub struct RefusedTestEdge {
    /// Always `"refused_test_edge"`, telling these lines from witnesses.
    pub record: &'static str,
    pub unit_seq: u64,
    pub function: String,
    pub line: usize,
    pub source: String,
    /// Acquisition lines of the sites the tested binding may denote.
    pub targets: Vec<usize>,
    /// Acquisition lines of those sites joined provenance ties to the name.
    pub tied: Vec<usize>,
    /// Whether any of the tied sites is `LIVE` in the test's state.
    pub tied_live: bool,
}

/// Per edge of a refused test: the tested binding's targets and the tied ones,
/// on the edge that carries no resource at the tested name.
#[derive(Clone, Default)]
struct AbsentEdge {
    targets: BTreeSet<usize>,
    tied: BTreeSet<usize>,
}

struct Settled {
    output: Option<AbstractState>,
    refused: bool,
    read: BTreeSet<usize>,
    called: BTreeSet<usize>,
    /// Every site the rule would mark at this node, cascade included.
    marked: BTreeSet<usize>,
    edges: Vec<Option<AbstractState>>,
    edge_unknown: Vec<bool>,
    /// Card SUP07-REFUSED-TEST-ACCUSATION: per edge, `Some` exactly on a
    /// refused test's null (invalid) edge.
    absent: Vec<Option<AbsentEdge>>,
    instruction: String,
}

fn label(instruction: &LegacyInstruction) -> String {
    match instruction {
        LegacyInstruction::Unknown { description, .. } => format!("Unknown: {description}"),
        LegacyInstruction::ContractCall { function, .. } => format!("ContractCall {function}"),
        LegacyInstruction::Release { source } => format!("Release {source}"),
        LegacyInstruction::Escape { source } => format!("Escape {source}"),
        LegacyInstruction::EscapeIfOwned { source } => format!("EscapeIfOwned {source}"),
        LegacyInstruction::Alias {
            destination,
            source,
        } => format!("Alias {destination} = {source}"),
        LegacyInstruction::Acquire { destination, .. } => format!("Acquire {destination}"),
        LegacyInstruction::BranchOnNull { source, .. } => format!("BranchOnNull {source}"),
        LegacyInstruction::NoOp => "NoOp".to_owned(),
        other => {
            let text = format!("{other:?}");
            text.split([' ', '{', '(']).next().unwrap_or("").to_owned()
        }
    }
}

fn bound(state: &AbstractState, site: usize) -> Vec<String> {
    state
        .pointers
        .iter()
        .filter(|(_, pointer)| pointer.targets.contains(&site))
        .map(|(place, _)| place.clone())
        .collect()
}

fn live(state: &AbstractState, site: usize) -> bool {
    state
        .resources
        .get(&site)
        .is_some_and(|status| status & LIVE != 0)
}

/// Record the witnesses of one settled analysis. A no-op unless
/// [`LEAK_WITNESS_PATH_VARIABLE`] names a path.
#[allow(clippy::too_many_arguments)]
pub(super) fn record(
    cfg: &LegacyCfg,
    incoming: &[Option<AbstractState>],
    acquisition_lines: &BTreeMap<usize, usize>,
    context: &TransferContext<'_>,
    alias: &[NodeAliasFacts],
    joined: &[BTreeMap<usize, BTreeSet<String>>],
    entry: &EntryCustody,
    diagnostics: &BTreeSet<LegacyDiagnostic>,
) {
    if std::env::var_os(LEAK_WITNESS_PATH_VARIABLE).is_none() {
        return;
    }
    // Card SUP07-REFUSED-TEST-ACCUSATION: rule 12e's provenance, as in the
    // work list.
    let tie_owned = refused_test::tie_facts(cfg);
    let tie = tie_owned.as_deref().unwrap_or(joined);
    record_refused_tests(cfg, incoming, acquisition_lines, context, tie);
    let reported = diagnostics
        .iter()
        .filter_map(|diagnostic| match diagnostic.kind {
            LegacyDiagnosticKind::Leak { acquisition_line } => {
                Some((diagnostic.line, acquisition_line))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    if reported.is_empty() {
        return;
    }
    let invocation = INVOCATION.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let settled = settle(
        cfg,
        incoming,
        acquisition_lines,
        context,
        alias,
        joined,
        tie,
    );
    let producers = cfg
        .nodes
        .iter()
        .enumerate()
        .flat_map(|(node, n)| {
            instruction_produced_sites(&n.instruction)
                .into_iter()
                .map(move |site| (site, node))
        })
        .fold(
            BTreeMap::<usize, Vec<usize>>::new(),
            |mut map, (site, node)| {
                map.entry(site).or_default().push(node);
                map
            },
        );
    let refused_reacquisitions =
        super::loop_generations_route::refused_reacquisitions(cfg, incoming);
    for &(line, acquisition_line) in &reported {
        let bases = acquisition_lines
            .iter()
            .filter(|(_, acquired)| **acquired == acquisition_line)
            .map(|(site, _)| *site)
            .collect::<Vec<_>>();
        let exits = cfg
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| node.is_exit && node.line == line)
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        // Card LOOP-REACQUISITION-LEAK-ACCUSATION: every generation of each
        // site, the current one first (the witness this census always wrote).
        for generation in [
            crate::loop_generations::Generation::Current,
            crate::loop_generations::Generation::Previous,
            crate::loop_generations::Generation::Older,
        ] {
            let sites = bases
                .iter()
                .map(|&site| match generation {
                    crate::loop_generations::Generation::Current => site,
                    crate::loop_generations::Generation::Previous => {
                        crate::loop_generations::previous(site)
                    }
                    crate::loop_generations::Generation::Older => {
                        crate::loop_generations::older(site)
                    }
                })
                .collect::<Vec<_>>();
            let best = best_witness(
                cfg,
                incoming,
                entry,
                &settled,
                &producers,
                &refused_reacquisitions,
                invocation,
                &reported,
                (line, acquisition_line),
                &exits,
                &sites,
                generation,
            );
            match (generation, best) {
                (crate::loop_generations::Generation::Current, best) => {
                    emit(&best.unwrap_or_else(|| Witness {
                        unit_seq: crate::indirect_census::unit_seq(),
                        invocation,
                        function: cfg.function.clone(),
                        reported: reported.clone(),
                        line,
                        acquisition_line,
                        exit_node: None,
                        site: None,
                        generation: "current",
                        reacquisition_refused: false,
                        path_reacquisitions: None,
                        settled: false,
                        status: 0,
                        unresolved: false,
                        exempt: false,
                        exit_unmodeled: false,
                        exit_widened: false,
                        bound_at_exit: Vec::new(),
                        path: None,
                        clean_path: None,
                        strict_path: None,
                        handed_path: None,
                        untested_path: None,
                        untied_path: None,
                    }));
                }
                (_, Some(best)) if best.settled || best.path.is_some() => emit(&best),
                _ => {}
            }
        }
    }
}

fn generation_code(generation: crate::loop_generations::Generation) -> &'static str {
    match generation {
        crate::loop_generations::Generation::Current => "current",
        crate::loop_generations::Generation::Previous => "previous",
        crate::loop_generations::Generation::Older => "older",
    }
}

/// The best witness of one reported leak among `sites` (one generation of
/// each acquisition site at the leak's acquisition line) over `exits`.
#[allow(clippy::too_many_arguments)]
fn best_witness(
    cfg: &LegacyCfg,
    incoming: &[Option<AbstractState>],
    entry: &EntryCustody,
    settled: &[Settled],
    producers: &BTreeMap<usize, Vec<usize>>,
    refused_reacquisitions: &BTreeSet<usize>,
    invocation: u64,
    reported: &[(usize, usize)],
    (line, acquisition_line): (usize, usize),
    exits: &[usize],
    sites: &[usize],
    generation: crate::loop_generations::Generation,
) -> Option<Witness> {
    let mut best: Option<Witness> = None;
    {
        for &exit in exits {
            for &site in sites {
                let base = crate::loop_generations::base(site);
                let Some(output) = settled[exit].output.as_ref() else {
                    continue;
                };
                let exempt = entry.exempt_sites(output);
                let status = output.resources.get(&site).copied().unwrap_or(0);
                let reproduced = status & LIVE != 0
                    && !exempt.contains(&site)
                    && !output.unresolved_effect_sites.contains(&site);
                // An earlier generation is created by the shift at its base
                // site's producer, so the search starts there too.
                let starts = producers.get(&base).cloned().unwrap_or_default();
                let find = |avoid, edges| {
                    search(cfg, settled, incoming, &starts, exit, site, avoid, edges)
                };
                let path = find(Avoid::Nothing, EdgeAvoid::Nothing);
                let path_reacquisitions = path.as_ref().map(|path| {
                    path.iter()
                        .skip(1)
                        .filter(|step| starts.contains(&step.node))
                        .count()
                });
                let clean_path = find(Avoid::Escapes, EdgeAvoid::Nothing);
                let strict_path = find(Avoid::HandOns, EdgeAvoid::Nothing);
                let untested_path = find(Avoid::Nothing, EdgeAvoid::RefusedNull);
                let untied_path = find(Avoid::Nothing, EdgeAvoid::Tied);
                let handed_path = handed(cfg, settled, incoming, &starts, exit, site);
                let candidate = Witness {
                    unit_seq: crate::indirect_census::unit_seq(),
                    invocation,
                    function: cfg.function.clone(),
                    reported: reported.to_vec(),
                    line,
                    acquisition_line,
                    exit_node: Some(exit),
                    site: Some(site),
                    generation: generation_code(generation),
                    reacquisition_refused: refused_reacquisitions.contains(&base),
                    path_reacquisitions,
                    settled: reproduced,
                    status,
                    unresolved: output.unresolved_effect_sites.contains(&site),
                    exempt: exempt.contains(&site),
                    exit_unmodeled: output.unmodeled,
                    exit_widened: output.widened,
                    bound_at_exit: bound(output, site),
                    path,
                    clean_path,
                    strict_path,
                    handed_path,
                    untested_path,
                    untied_path,
                };
                let better = match &best {
                    None => true,
                    Some(current) => {
                        (candidate.settled, candidate.path.is_some())
                            > (current.settled, current.path.is_some())
                    }
                };
                if better {
                    best = Some(candidate);
                }
            }
        }
    }
    best
}

/// Re-run every reachable node's transfer and edge refinement from its settled
/// incoming state, as the work list does.
fn settle(
    cfg: &LegacyCfg,
    incoming: &[Option<AbstractState>],
    acquisition_lines: &BTreeMap<usize, usize>,
    context: &TransferContext<'_>,
    alias: &[NodeAliasFacts],
    joined: &[BTreeMap<usize, BTreeSet<String>>],
    tie: &[BTreeMap<usize, BTreeSet<String>>],
) -> Vec<Settled> {
    cfg.nodes
        .iter()
        .enumerate()
        .map(|(node_id, node)| {
            let instruction = label(&node.instruction);
            let Some(state) = incoming[node_id].clone() else {
                return Settled {
                    output: None,
                    refused: false,
                    read: BTreeSet::new(),
                    called: BTreeSet::new(),
                    marked: BTreeSet::new(),
                    edges: Vec::new(),
                    edge_unknown: Vec::new(),
                    absent: Vec::new(),
                    instruction,
                };
            };
            // Card CG04-LOOP-SITE-GENERATIONS: as in the work list.
            let mut local = BTreeSet::new();
            let (state, shifted) =
                super::loop_generations_route::shift(&cfg.function, node, state, &mut local);
            let read = refused_operands::handed_sites(
                &state,
                &cfg.publications.line_reads,
                node,
                refused_operands::HandOn::Escape,
            );
            let called = refused_operands::handed_sites(
                &state,
                &cfg.publications.line_reads,
                node,
                refused_operands::HandOn::Operand,
            );
            let withdrawn =
                refused_operands::read_sites(&state, &cfg.publications.line_reads, node);
            let conditional = joined_provenance::release_site(node, &state, &joined[node_id]);
            let mut output = transfer(
                &cfg.function,
                node,
                state,
                acquisition_lines,
                context,
                &alias[node_id],
                &mut local,
            );
            joined_provenance::finish_release(conditional, &mut output, &local);
            refused_operands::apply(&mut output, &withdrawn, &local);
            super::loop_generations_route::settle_refused(&shifted, &mut output, &local);
            let refused = refused_operands::refused(&local);
            let marked = if refused && !withdrawn.is_empty() {
                let mut probe = output.clone();
                probe.unresolved_effect_sites.clear();
                refused_operands::withdraw(&mut probe, &withdrawn);
                probe.unresolved_effect_sites
            } else {
                BTreeSet::new()
            };
            let mut edge_unknown = Vec::new();
            let mut absent = Vec::new();
            let edges = node
                .successors
                .iter()
                .enumerate()
                .map(|(edge, _)| {
                    let mut edge_output = output.clone();
                    let refinement = refine_edge(
                        &node.instruction,
                        edge,
                        &mut edge_output,
                        &context.site_kinds,
                    )
                    .map(|(_, refinement)| refinement);
                    edge_unknown.push(matches!(refinement, Some(PointerRefinement::Unknown)));
                    // Card SUP07-REFUSED-TEST-ACCUSATION (rule 12e): as in
                    // the work list.
                    if matches!(refinement, Some(PointerRefinement::Unknown)) {
                        refused_test::settle_absent_edge(
                            &node.instruction,
                            edge,
                            &output,
                            &tie[node_id],
                            &mut edge_output,
                        );
                    }
                    absent.push(
                        matches!(refinement, Some(PointerRefinement::Unknown))
                            .then(|| refused_test::absent_edge_source(&node.instruction, edge))
                            .flatten()
                            .map(|source| {
                                let targets = output
                                    .pointers
                                    .get(source)
                                    .map(|pointer| pointer.targets.clone())
                                    .unwrap_or_default();
                                let tied = refused_test::tied_sites(&tie[node_id], source)
                                    .intersection(&targets)
                                    .copied()
                                    .collect();
                                AbsentEdge { targets, tied }
                            }),
                    );
                    if matches!(refinement, Some(PointerRefinement::Infeasible)) {
                        if node.loop_exit_edge == Some(edge) {
                            edge_output = output.clone();
                        } else {
                            return None;
                        }
                    }
                    refine_collection_edge(
                        node,
                        edge,
                        &mut edge_output,
                        &context.collection_owners(),
                    )
                    .then_some(edge_output)
                })
                .collect();
            Settled {
                output: Some(output),
                refused,
                read,
                called,
                marked,
                edges,
                edge_unknown,
                absent,
                instruction,
            }
        })
        .collect()
}

/// Card SUP07-REFUSED-TEST-ACCUSATION: which refused-test edges a witness
/// search may not take.
#[derive(Clone, Copy, PartialEq, Eq)]
enum EdgeAvoid {
    Nothing,
    /// The null (invalid) edge of a refused test whose binding may denote the
    /// site.
    RefusedNull,
    /// Only those at which joined provenance ties the site to the tested name.
    Tied,
}

fn absent_edge(settled: &[Settled], node: usize, edge: usize) -> Option<&AbsentEdge> {
    settled[node].absent.get(edge).and_then(Option::as_ref)
}

/// Which refused nodes a witness search may not pass.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Avoid {
    Nothing,
    /// Refused nodes whose value side hands the site on.
    Escapes,
    /// Refused nodes that hand the site on in any way.
    HandOns,
}

/// Breadth-first search from any producer of `site` to `exit` over edges whose
/// settled output keeps `site` live, avoiding the refused nodes `avoid` names.
#[allow(clippy::too_many_arguments)]
fn search(
    cfg: &LegacyCfg,
    settled: &[Settled],
    incoming: &[Option<AbstractState>],
    starts: &[usize],
    exit: usize,
    site: usize,
    avoid: Avoid,
    edges: EdgeAvoid,
) -> Option<Vec<Step>> {
    let skipped = |node: usize, edge: usize| match (edges, absent_edge(settled, node, edge)) {
        (EdgeAvoid::Nothing, _) | (_, None) => false,
        (EdgeAvoid::RefusedNull, Some(absent)) => absent.targets.contains(&site),
        (EdgeAvoid::Tied, Some(absent)) => absent.tied.contains(&site),
    };
    let blocked = |node: usize| {
        let entry = &settled[node];
        entry.refused
            && match avoid {
                Avoid::Nothing => false,
                Avoid::Escapes => entry.read.contains(&site),
                Avoid::HandOns => entry.read.contains(&site) || entry.called.contains(&site),
            }
    };
    let mut previous = BTreeMap::<usize, Option<usize>>::new();
    let mut queue = VecDeque::new();
    for &start in starts {
        let reaches = settled[start]
            .output
            .as_ref()
            .is_some_and(|output| live(output, site));
        if reaches && !blocked(start) && !previous.contains_key(&start) {
            previous.insert(start, None);
            queue.push_back(start);
        }
    }
    while let Some(node) = queue.pop_front() {
        if node == exit {
            let mut path = Vec::new();
            let mut cursor = Some(node);
            let mut next: Option<usize> = None;
            while let Some(current) = cursor {
                let entry = &settled[current];
                let taken = |test: &dyn Fn(usize) -> bool| {
                    next.is_some_and(|following| {
                        cfg.nodes[current]
                            .successors
                            .iter()
                            .enumerate()
                            .any(|(edge, &successor)| successor == following && test(edge))
                    })
                };
                let edge_unknown =
                    taken(&|edge| entry.edge_unknown.get(edge).copied().unwrap_or(false));
                let edge_refused_null = taken(&|edge| {
                    absent_edge(settled, current, edge)
                        .is_some_and(|absent| absent.targets.contains(&site))
                });
                let edge_tied = taken(&|edge| {
                    absent_edge(settled, current, edge)
                        .is_some_and(|absent| absent.tied.contains(&site))
                });
                path.push(Step {
                    node: current,
                    line: cfg.nodes[current].line,
                    instruction: entry.instruction.clone(),
                    refused: entry.refused,
                    refused_read: entry.refused && entry.read.contains(&site),
                    refused_call: entry.refused && entry.called.contains(&site),
                    edge_unknown,
                    edge_refused_null,
                    edge_tied,
                    bound: incoming[current]
                        .as_ref()
                        .map(|state| bound(state, site))
                        .unwrap_or_default(),
                });
                next = Some(current);
                cursor = previous.get(&current).copied().flatten();
            }
            path.reverse();
            return Some(path);
        }
        for (edge, &successor) in cfg.nodes[node].successors.iter().enumerate() {
            let Some(Some(edge_output)) = settled[node].edges.get(edge) else {
                continue;
            };
            if !live(edge_output, site)
                || previous.contains_key(&successor)
                || blocked(successor)
                || skipped(node, edge)
            {
                continue;
            }
            let arrives = settled[successor]
                .output
                .as_ref()
                .is_some_and(|output| live(output, site));
            if !arrives {
                continue;
            }
            previous.insert(successor, Some(node));
            queue.push_back(successor);
        }
    }
    None
}

/// Feasible-edge reachability from `from` to `to`, ignoring the site's status:
/// the union join carries an unresolved effect along every feasible edge.
fn reach(cfg: &LegacyCfg, settled: &[Settled], from: &[usize], to: usize) -> Option<Vec<usize>> {
    let mut previous = BTreeMap::<usize, Option<usize>>::new();
    let mut queue = VecDeque::new();
    for &start in from {
        if settled[start].output.is_some() && !previous.contains_key(&start) {
            previous.insert(start, None);
            queue.push_back(start);
        }
    }
    while let Some(node) = queue.pop_front() {
        if node == to {
            let mut path = vec![node];
            let mut cursor = previous.get(&node).copied().flatten();
            while let Some(current) = cursor {
                path.push(current);
                cursor = previous.get(&current).copied().flatten();
            }
            path.reverse();
            return Some(path);
        }
        for (edge, &successor) in cfg.nodes[node].successors.iter().enumerate() {
            if matches!(settled[node].edges.get(edge), Some(Some(_)))
                && settled[successor].output.is_some()
                && !previous.contains_key(&successor)
            {
                previous.insert(successor, Some(node));
                queue.push_back(successor);
            }
        }
    }
    None
}

/// The shortest producer → marking node → exit path over feasible edges.
fn handed(
    cfg: &LegacyCfg,
    settled: &[Settled],
    incoming: &[Option<AbstractState>],
    starts: &[usize],
    exit: usize,
    site: usize,
) -> Option<Vec<Step>> {
    let best = (0..cfg.nodes.len())
        .filter(|&node| settled[node].marked.contains(&site))
        .filter_map(|node| {
            let before = reach(cfg, settled, starts, node)?;
            let after = reach(cfg, settled, &[node], exit)?;
            Some(
                before
                    .into_iter()
                    .chain(after.into_iter().skip(1))
                    .collect::<Vec<_>>(),
            )
        })
        .min_by_key(Vec::len)?;
    Some(
        best.iter()
            .map(|&current| {
                let entry = &settled[current];
                Step {
                    node: current,
                    line: cfg.nodes[current].line,
                    instruction: entry.instruction.clone(),
                    refused: entry.refused,
                    refused_read: entry.refused && entry.read.contains(&site),
                    refused_call: entry.marked.contains(&site),
                    edge_unknown: false,
                    edge_refused_null: false,
                    edge_tied: false,
                    bound: incoming[current]
                        .as_ref()
                        .map(|state| bound(state, site))
                        .unwrap_or_default(),
                }
            })
            .collect(),
    )
}

/// Card SUP07-REFUSED-TEST-ACCUSATION: every refused test's null (invalid)
/// edge whose tested binding denotes some site, from the settled incoming
/// states. A test node's transfer is empty, so its settled incoming state is
/// the state its edges are refined from (as `report_retained_loop_exit_edges`
/// reads it).
fn record_refused_tests(
    cfg: &LegacyCfg,
    incoming: &[Option<AbstractState>],
    acquisition_lines: &BTreeMap<usize, usize>,
    context: &TransferContext<'_>,
    joined: &[BTreeMap<usize, BTreeSet<String>>],
) {
    let line_of = |sites: &BTreeSet<usize>| {
        sites
            .iter()
            .map(|site| acquisition_lines.get(site).copied().unwrap_or(0))
            .collect::<Vec<_>>()
    };
    for (node_id, node) in cfg.nodes.iter().enumerate() {
        let Some(state) = incoming[node_id].as_ref() else {
            continue;
        };
        for edge in 0..node.successors.len() {
            let Some(source) = refused_test::absent_edge_source(&node.instruction, edge) else {
                continue;
            };
            let mut probe = state.clone();
            if !matches!(
                refine_edge(&node.instruction, edge, &mut probe, &context.site_kinds),
                Some((_, PointerRefinement::Unknown))
            ) {
                continue;
            }
            let targets = state
                .pointers
                .get(source)
                .map(|pointer| pointer.targets.clone())
                .unwrap_or_default();
            if targets.is_empty() {
                continue;
            }
            let tied = refused_test::tied_sites(&joined[node_id], source)
                .intersection(&targets)
                .copied()
                .collect::<BTreeSet<_>>();
            let record = RefusedTestEdge {
                record: "refused_test_edge",
                unit_seq: crate::indirect_census::unit_seq(),
                function: cfg.function.clone(),
                line: node.line,
                source: source.to_owned(),
                targets: line_of(&targets),
                tied: line_of(&tied),
                tied_live: tied.iter().any(|site| live(state, *site)),
            };
            emit_line(&record);
        }
    }
}

fn emit(witness: &Witness) {
    emit_line(witness);
}

fn emit_line<T: serde::Serialize>(value: &T) {
    let Ok(mut guard) = WITNESSES.lock() else {
        return;
    };
    let opened = guard.get_or_insert_with(|| {
        let path = std::env::var_os(LEAK_WITNESS_PATH_VARIABLE)?;
        File::create(path).ok().map(BufWriter::new)
    });
    let Some(writer) = opened.as_mut() else {
        return;
    };
    if let Ok(line) = serde_json::to_string(value) {
        let _ = writeln!(writer, "{line}");
        let _ = writer.flush();
    }
}
