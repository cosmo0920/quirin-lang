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

//! Instrumented census of the conditional-provenance fixed point's
//! **rounds to convergence** (card **JOINED-PROVENANCE-ROUND-BOUND**).
//!
//! # What this module is, and what it is not
//!
//! It is a **measurement**, not a rule. Nothing here is consulted by any
//! analysis, certificate, summary or report: [`record`] is called after
//! [`super::fixed_point`] has already decided what it
//! returns, it is handed borrowed values it cannot mutate, and its own probe
//! sweep's result is dropped on the floor. It moves no verdict, admits nothing
//! and refuses nothing.
//!
//! The whole module — and its one call site — is behind the off-by-default
//! cargo feature `provenance-rounds-census`, on the
//! `C-GATEWAY-INDIRECT-CENSUS-2` precedent. With the feature off none of this
//! code exists in the compiled crate. The card publishes the complementary
//! half: with the feature **on**, the pinned corpus audit still produces the
//! archived artifacts byte for byte, so the probe perturbs no decision either.
//!
//! # Why a second sweep, and why the same one
//!
//! The question the card must answer is how many rounds a function *needs*,
//! which is not observable from a capped run: a decline reports only that the
//! cap was not enough. The census therefore runs
//! [`iterate`] again under a larger cap — the
//! **same function** the analyzer runs, never a re-implementation that could
//! drift from it — and records where that one settles. The production result is
//! computed first and is untouched by the probe.
//!
//! The probe's cap is read from [`PROBE_CAP_VARIABLE`], defaulting to
//! [`DEFAULT_PROBE_CAP`], so a targeted re-measurement of a handful of
//! functions can push the cap far higher than a whole-corpus run affords.
use std::fs::File;
use std::io::{BufWriter, Write};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use super::{BTreeSet, LegacyCfg, Settled, created_sites, iterate, round_bound};

/// What one sweep did, in the census's own serializable vocabulary.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum Sweep {
    /// Settled after `rounds` sweeps, the last of which changed nothing, and
    /// published a non-empty binding set at `derived_nodes` nodes covering
    /// `derived_sites` distinct sites.
    Converged {
        rounds: usize,
        derived_nodes: usize,
        derived_sites: usize,
    },
    /// Still moving at the cap. The function derives nothing.
    Declined { cap: usize },
}

/// One cyclic function's record.
#[derive(Clone, Debug, serde::Serialize)]
pub struct RoundsLeaf {
    /// Ordinal of this record in the stream, so a re-run can be diffed.
    pub seq: u64,
    /// The analysed function's name, as the CFG carries it.
    pub function: String,
    /// Nodes in the CFG, and the reachable subset the sweep actually walks.
    pub nodes: usize,
    pub reachable: usize,
    /// Edges among the reachable nodes: the sweep's per-round cost axis.
    pub edges: usize,
    /// Sites created anywhere in the reachable subgraph, and the **eligible**
    /// subset — those whose creating node lies on no cycle. A cyclic function
    /// with no eligible site derives nothing whatever the iteration does, and
    /// is recorded with `production: null`.
    pub created_sites: usize,
    pub eligible_sites: usize,
    /// `conditional_names`: the finite name universe one round's intersections
    /// range over, which is the `N` of the card's proved bound.
    pub names: usize,
    /// The bound the card proves for this graph:
    /// `reachable * (1 + eligible_and_created * (1 + names))`, saturating.
    pub proved_bound: u128,
    /// What the production sweep did, at this graph's own [`round_bound`].
    /// `None` when no sweep ran because no site was eligible.
    pub production: Option<Sweep>,
    /// What the same sweep does under the probe cap. `None` for the same
    /// reason.
    pub probe: Option<Sweep>,
    /// The cap the probe used.
    pub probe_cap: usize,
}

static SINK: Mutex<Option<Option<BufWriter<File>>>> = Mutex::new(None);
static SEQ: AtomicU64 = AtomicU64::new(0);

/// The environment variable that turns collection on and names the file the
/// records are streamed to as JSON lines. This, together with the
/// `provenance-rounds-census` cargo feature, is the census's whole entry point.
pub const CENSUS_PATH_VARIABLE: &str = "QCORE_PROVENANCE_ROUNDS";

/// The environment variable that overrides the probe's cap.
pub const PROBE_CAP_VARIABLE: &str = "QCORE_PROVENANCE_ROUNDS_CAP";

/// The probe's cap when [`PROBE_CAP_VARIABLE`] names none.
pub const DEFAULT_PROBE_CAP: usize = 1024;

fn probe_cap() -> usize {
    std::env::var(PROBE_CAP_VARIABLE)
        .ok()
        .and_then(|text| text.trim().parse::<usize>().ok())
        .filter(|cap| *cap > 0)
        .unwrap_or(DEFAULT_PROBE_CAP)
}

fn summarize(settled: &Settled, cap: usize) -> Sweep {
    match settled {
        Settled::Converged { rounds, solution } => Sweep::Converged {
            rounds: *rounds,
            derived_nodes: solution
                .iter()
                .filter(|bindings| !bindings.is_empty())
                .count(),
            derived_sites: solution
                .iter()
                .flat_map(|bindings| bindings.keys().copied())
                .collect::<BTreeSet<_>>()
                .len(),
        },
        Settled::Declined => Sweep::Declined { cap },
    }
}

/// Record one cyclic function's iteration behaviour.
///
/// `settled` is the production sweep's own outcome, already computed;
/// `None` means no sweep ran because the function has no eligible site. A
/// no-op unless [`CENSUS_PATH_VARIABLE`] names a writable path — a census that
/// cannot write is **silent rather than fatal**, because it must not be able to
/// change what an audit run does.
pub(crate) fn record(
    cfg: &LegacyCfg,
    reachable: &BTreeSet<usize>,
    sites: &BTreeSet<usize>,
    settled: Option<&Settled>,
) {
    if std::env::var_os(CENSUS_PATH_VARIABLE).is_none() {
        return;
    }
    let cap = probe_cap();
    let created = created_sites(cfg, reachable);
    let names = cfg.conditional_names.len() as u128;
    let leaf = RoundsLeaf {
        seq: SEQ.fetch_add(1, Ordering::Relaxed),
        function: cfg.function.clone(),
        nodes: cfg.nodes.len(),
        reachable: reachable.len(),
        edges: reachable
            .iter()
            .map(|node| cfg.nodes[*node].successors.len())
            .sum(),
        created_sites: created.len(),
        eligible_sites: sites.len(),
        names: cfg.conditional_names.len(),
        proved_bound: (reachable.len() as u128).saturating_mul(
            1u128.saturating_add((created.len() as u128).saturating_mul(1 + names)),
        ),
        production: settled.map(|settled| summarize(settled, round_bound(cfg, reachable))),
        // The probe re-runs the analyzer's own sweep under a larger cap. Its
        // result is summarized and dropped: nothing downstream sees it.
        probe: settled.map(|_| summarize(&iterate(cfg, reachable, sites, cap), cap)),
        probe_cap: cap,
    };
    let Ok(mut guard) = SINK.lock() else {
        return;
    };
    let opened = guard.get_or_insert_with(|| {
        let path = std::env::var_os(CENSUS_PATH_VARIABLE)?;
        File::create(path).ok().map(BufWriter::new)
    });
    let Some(writer) = opened.as_mut() else {
        return;
    };
    if let Ok(line) = serde_json::to_string(&leaf) {
        let _ = writeln!(writer, "{line}");
        let _ = writer.flush();
    }
}
