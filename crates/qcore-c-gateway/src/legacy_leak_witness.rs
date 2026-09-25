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
//! it.
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
    refused_operands, transfer,
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
}

struct Settled {
    output: Option<AbstractState>,
    refused: bool,
    read: BTreeSet<usize>,
    called: BTreeSet<usize>,
    edges: Vec<Option<AbstractState>>,
    edge_unknown: Vec<bool>,
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
    let settled = settle(cfg, incoming, acquisition_lines, context, alias, joined);
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
    for &(line, acquisition_line) in &reported {
        let sites = acquisition_lines
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
        let mut best: Option<Witness> = None;
        for &exit in &exits {
            for &site in &sites {
                let Some(output) = settled[exit].output.as_ref() else {
                    continue;
                };
                let exempt = entry.exempt_sites(output);
                let status = output.resources.get(&site).copied().unwrap_or(0);
                let reproduced = status & LIVE != 0
                    && !exempt.contains(&site)
                    && !output.unresolved_effect_sites.contains(&site);
                let starts = producers.get(&site).cloned().unwrap_or_default();
                let path = search(cfg, &settled, incoming, &starts, exit, site, Avoid::Nothing);
                let clean_path =
                    search(cfg, &settled, incoming, &starts, exit, site, Avoid::Escapes);
                let strict_path =
                    search(cfg, &settled, incoming, &starts, exit, site, Avoid::HandOns);
                let candidate = Witness {
                    unit_seq: crate::indirect_census::unit_seq(),
                    invocation,
                    function: cfg.function.clone(),
                    reported: reported.clone(),
                    line,
                    acquisition_line,
                    exit_node: Some(exit),
                    site: Some(site),
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
        let witness = best.unwrap_or_else(|| Witness {
            unit_seq: crate::indirect_census::unit_seq(),
            invocation,
            function: cfg.function.clone(),
            reported: reported.clone(),
            line,
            acquisition_line,
            exit_node: None,
            site: None,
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
        });
        emit(&witness);
    }
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
                    edges: Vec::new(),
                    edge_unknown: Vec::new(),
                    instruction,
                };
            };
            let read =
                refused_operands::read_sites(&state, &cfg.publications.line_reads, node.line);
            let called = refused_operands::handed_sites(
                &state,
                &cfg.publications.line_reads,
                node.line,
                refused_operands::HandOn::Call,
            );
            let mut local = BTreeSet::new();
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
            refused_operands::apply(&mut output, &read, &local);
            let refused = refused_operands::refused(&local);
            let mut edge_unknown = Vec::new();
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
                edges,
                edge_unknown,
                instruction,
            }
        })
        .collect()
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
fn search(
    cfg: &LegacyCfg,
    settled: &[Settled],
    incoming: &[Option<AbstractState>],
    starts: &[usize],
    exit: usize,
    site: usize,
    avoid: Avoid,
) -> Option<Vec<Step>> {
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
                let edge_unknown =
                    next.is_some_and(|following| {
                        cfg.nodes[current].successors.iter().enumerate().any(
                            |(edge, &successor)| {
                                successor == following
                                    && entry.edge_unknown.get(edge).copied().unwrap_or(false)
                            },
                        )
                    });
                path.push(Step {
                    node: current,
                    line: cfg.nodes[current].line,
                    instruction: entry.instruction.clone(),
                    refused: entry.refused,
                    refused_read: entry.refused && entry.read.contains(&site),
                    refused_call: entry.refused && entry.called.contains(&site),
                    edge_unknown,
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
            if !live(edge_output, site) || previous.contains_key(&successor) || blocked(successor) {
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

fn emit(witness: &Witness) {
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
    if let Ok(line) = serde_json::to_string(witness) {
        let _ = writeln!(writer, "{line}");
        let _ = writer.flush();
    }
}
