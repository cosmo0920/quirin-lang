#!/usr/bin/env python3
"""Card JOINED-PROVENANCE-CALL-FRAMES: build the attribution instrumentation.

Reads the pristine `legacy_joined_provenance.rs` and writes an instrumented
copy that records, for every (site, name) pair the conditional-provenance
relation ever held, **which rule dropped the name** — and emits one JSON line
per reached `release_site` premise gate naming the causes.

This is temporary measurement code. It is never shipped: the published
BEFORE/AFTER audit artifacts are produced by binaries built without it.

Usage: python3 instrument.py <pristine.rs> <out.rs>
"""
import pathlib
import sys

TRACE = r'''
// ---------------------------------------------------------------------------
// TEMPORARY measurement instrumentation — card JOINED-PROVENANCE-CALL-FRAMES.
// Records which rule removes a name from a site's binding set, so the 14 sites
// that reach `release_site`'s premise gate can be attributed individually.
// Gated on QCORE_JPCF_TRACE; removed before the card publishes.
// ---------------------------------------------------------------------------
#[derive(Default)]
struct Trace {
    function: String,
    cyclic: bool,
    declined: &'static str,
    ever_bound: BTreeSet<(usize, String)>,
    shapes: BTreeMap<usize, &'static str>,
    drops: BTreeMap<(usize, String), BTreeMap<&'static str, usize>>,
}

thread_local! {
    static TRACE: std::cell::RefCell<Trace> = std::cell::RefCell::new(Trace::default());
}

fn tracing() -> bool {
    std::env::var("QCORE_JPCF_TRACE").is_ok()
}

fn trace_reset(cfg: &LegacyCfg, cyclic: bool) {
    if !tracing() {
        return;
    }
    TRACE.with(|t| {
        let mut t = t.borrow_mut();
        *t = Trace::default();
        t.function = cfg.function.clone();
        t.cyclic = cyclic;
    });
}

fn trace_bound(site: usize, name: &str, shape: &'static str) {
    if !tracing() {
        return;
    }
    TRACE.with(|t| {
        let mut t = t.borrow_mut();
        t.ever_bound.insert((site, name.to_owned()));
        t.shapes.entry(site).or_insert(shape);
    });
}

fn trace_shape(site: usize, shape: &'static str) {
    if !tracing() {
        return;
    }
    TRACE.with(|t| {
        t.borrow_mut().shapes.entry(site).or_insert(shape);
    });
}

fn trace_declined(reason: &'static str) {
    if !tracing() {
        return;
    }
    TRACE.with(|t| t.borrow_mut().declined = reason);
}

/// Every (site, name) pair in `before` that `after` no longer carries, charged
/// to `cause`.
fn trace_drops(before: &Bindings, after: &Bindings, cause: &'static str) {
    if !tracing() {
        return;
    }
    TRACE.with(|t| {
        let mut t = t.borrow_mut();
        for (site, names) in before {
            for name in names {
                let kept = after.get(site).is_some_and(|n| n.contains(name));
                if !kept {
                    *t.drops
                        .entry((*site, name.clone()))
                        .or_default()
                        .entry(cause)
                        .or_insert(0) += 1;
                }
            }
        }
    });
}

fn trace_cause(instruction: &LegacyInstruction) -> &'static str {
    match instruction {
        LegacyInstruction::ContractCall { .. } => "call_forget",
        LegacyInstruction::AggregateStore { .. } => "aggregate_store",
        LegacyInstruction::Unknown { .. } => "refused_write",
        _ => "step_other",
    }
}

/// One JSON line per reached `release_site` premise gate.
fn trace_release(site: usize, source: &str, named: bool, blocking: &[&str]) {
    let Ok(path) = std::env::var("QCORE_JPCF_PREMISE") else {
        return;
    };
    use std::io::Write as _;
    let line = TRACE.with(|t| {
        let t = t.borrow();
        let causes = t
            .drops
            .get(&(site, source.to_owned()))
            .map(|m| {
                m.iter()
                    .map(|(c, n)| format!("{c}:{n}"))
                    .collect::<Vec<_>>()
                    .join("+")
            })
            .unwrap_or_default();
        // Which rules dropped this *site*'s names, whatever the name.
        let any_name_causes = t
            .drops
            .iter()
            .filter(|((s, _), _)| *s == site)
            .flat_map(|(_, m)| m.keys().copied())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>()
            .join("+");
        format!(
            "{{\"function\":{:?},\"site\":{},\"source\":{:?},\"named\":{},\
\"ever_bound\":{},\"shape\":{:?},\"cyclic\":{},\"declined\":{:?},\
\"causes\":{:?},\"site_causes\":{:?},\"blocking\":{:?}}}\n",
            t.function,
            site,
            source,
            named,
            t.ever_bound.contains(&(site, source.to_owned())),
            t.shapes.get(&site).copied().unwrap_or("none"),
            t.cyclic,
            t.declined,
            causes,
            any_name_causes,
            blocking.join("+"),
        )
    });
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = file.write_all(line.as_bytes());
    }
}
'''

EDITS = [
    # 1. Insert the trace machinery after the module imports.
    (
        "use super::*;\n\ntype Bindings = BTreeMap<usize, BTreeSet<String>>;",
        "use super::*;\n\ntype Bindings = BTreeMap<usize, BTreeSet<String>>;\n" + TRACE,
    ),
    # 2. step(): snapshot around the rebound forget and around the transfer.
    (
        """    fn step(&mut self, instruction: &LegacyInstruction) {
        let copied = if let LegacyInstruction::Alias { source, .. } = instruction {""",
        """    fn step(&mut self, instruction: &LegacyInstruction) {
        let snapshot_in = self.bindings.clone();
        let copied = if let LegacyInstruction::Alias { source, .. } = instruction {""",
    ),
    (
        """        for name in rebound_names(instruction) {
            self.forget(name);
        }
        match instruction {""",
        """        for name in rebound_names(instruction) {
            self.forget(name);
        }
        trace_drops(&snapshot_in, &self.bindings, "rebound");
        let snapshot_rebound = self.bindings.clone();
        match instruction {""",
    ),
    # 3. Record how each site came to exist, and whether it got a name.
    (
        """            LegacyInstruction::Acquire { site, destination } => {
                self.present.insert(*site);
                self.bindings
                    .insert(*site, BTreeSet::from([destination.clone()]));
            }""",
        """            LegacyInstruction::Acquire { site, destination } => {
                self.present.insert(*site);
                self.bindings
                    .insert(*site, BTreeSet::from([destination.clone()]));
                trace_bound(*site, destination, "acquire");
            }""",
    ),
    (
        """            LegacyInstruction::Alias { destination, .. } => {
                for site in copied {
                    self.bindings
                        .entry(site)
                        .or_default()
                        .insert(destination.clone());
                }
            }""",
        """            LegacyInstruction::Alias { destination, .. } => {
                for site in copied {
                    self.bindings
                        .entry(site)
                        .or_default()
                        .insert(destination.clone());
                    trace_bound(site, destination, "alias");
                }
            }""",
    ),
    (
        """                self.bindings.clear();
                self.present.extend(produced_sites);
                if contract""",
        """                self.bindings.clear();
                self.present.extend(produced_sites);
                for site in produced_sites {
                    trace_shape(*site, "contract_unnamed");
                }
                if contract""",
    ),
    (
        """                {
                    self.bindings
                        .insert(*site, BTreeSet::from([destination.clone()]));
                }
            }
            LegacyInstruction::Unknown { writes, .. } => {""",
        """                {
                    self.bindings
                        .insert(*site, BTreeSet::from([destination.clone()]));
                    trace_bound(*site, destination, "contract_named");
                }
            }
            LegacyInstruction::Unknown { writes, .. } => {""",
    ),
    # 4. Close step() with the transfer-cause charge.
    (
        """            LegacyInstruction::AggregateStore { .. } => self.bindings.clear(),
            _ => {}
        }
    }
}""",
        """            LegacyInstruction::AggregateStore { .. } => self.bindings.clear(),
            _ => {}
        }
        trace_drops(&snapshot_rebound, &self.bindings, trace_cause(instruction));
    }
}""",
    ),
    # 5. join(): charge names lost to the intersection.
    (
        """    fn join(&mut self, other: &Self) {
        for site in self""",
        """    fn join(&mut self, other: &Self) {
        let snapshot_in = self.bindings.clone();
        for site in self""",
    ),
    (
        """        self.present.extend(&other.present);
    }""",
        """        self.present.extend(&other.present);
        trace_drops(&snapshot_in, &self.bindings, "join_intersect");
    }""",
    ),
    # 6. outgoing(): the conditional-name restriction and the null edge.
    (
        """    let mut out = incoming.clone();
    out.step(&cfg.nodes[node].instruction);
    for names in out.bindings.values_mut() {
        names.retain(|name| cfg.conditional_names.contains(name));
    }""",
        """    let mut out = incoming.clone();
    out.step(&cfg.nodes[node].instruction);
    let snapshot_step = out.bindings.clone();
    for names in out.bindings.values_mut() {
        names.retain(|name| cfg.conditional_names.contains(name));
    }
    trace_drops(&snapshot_step, &out.bindings, "not_conditional_name");""",
    ),
    (
        """                for site in absent {
                    branch.present.remove(&site);
                    branch.bindings.remove(&site);
                }""",
        """                let snapshot_branch = branch.bindings.clone();
                for site in absent {
                    branch.present.remove(&site);
                    branch.bindings.remove(&site);
                }
                trace_drops(&snapshot_branch, &branch.bindings, "null_edge");""",
    ),
    # 7. derive(): reset the trace, recording whether the fixed point runs.
    (
        """    if order.len() != reachable.len() {
        return fixed_point(cfg, &reachable);
    }""",
        """    if order.len() != reachable.len() {
        trace_reset(cfg, true);
        return fixed_point(cfg, &reachable);
    }
    trace_reset(cfg, false);""",
    ),
    # 8. fixed_point(): the off-cycle site filter and the declined return.
    (
        """    let sites = acyclic_sites(cfg, reachable);
    if sites.is_empty() {
        return declined;
    }""",
        """    let sites = acyclic_sites(cfg, reachable);
    if sites.is_empty() {
        trace_declined("no_off_cycle_site");
        return declined;
    }""",
    ),
    (
        """                .map(|facts| {
                    let mut bindings = facts.unwrap_or_default().bindings;
                    bindings.retain(|site, names| sites.contains(site) && !names.is_empty());
                    bindings
                })""",
        """                .map(|facts| {
                    let mut bindings = facts.unwrap_or_default().bindings;
                    let snapshot_settled = bindings.clone();
                    bindings.retain(|site, names| sites.contains(site) && !names.is_empty());
                    trace_drops(&snapshot_settled, &bindings, "off_cycle_site");
                    bindings
                })""",
    ),
    (
        """        incoming = next_round;
    }
    declined
}""",
        """        incoming = next_round;
    }
    trace_declined("round_bound");
    declined
}""",
    ),
    # 9. release_site(): emit the premise line with its attribution.
    (
        """    if !facts.get(&site)?.contains(source)
        || state.resources.get(&site) != Some(&LIVE)""",
        """    {
        let named = facts.get(&site).is_some_and(|names| names.contains(source));
        let blocking = [
            ("not_named_by_facts", !named),
            ("not_live", state.resources.get(&site) != Some(&LIVE)),
            ("nullable_site", state.nullable_sites.contains(&site)),
            (
                "unresolved_effect",
                state.unresolved_effect_sites.contains(&site),
            ),
            ("contained", state.contained.contains_key(&site)),
            ("field_linked", state.field_links.contains_key(&site)),
            (
                "conditional_replacement",
                state.conditional_replacements.contains_key(&site),
            ),
        ]
        .iter()
        .filter(|(_, hit)| *hit)
        .map(|(n, _)| *n)
        .collect::<Vec<_>>();
        trace_release(site, source, named, &blocking);
    }
    if !facts.get(&site)?.contains(source)
        || state.resources.get(&site) != Some(&LIVE)""",
    ),
]


# The same instrumentation against the module *after* this card's change: three
# anchors moved (the `step` signature, the call arm, the `outgoing` call site).
AFTER_EDITS = [
    (
        """    fn step(&mut self, instruction: &LegacyInstruction, preserved: &BTreeSet<String>) {
        let copied = if let LegacyInstruction::Alias { source, .. } = instruction {""",
        """    fn step(&mut self, instruction: &LegacyInstruction, preserved: &BTreeSet<String>) {
        let snapshot_in = self.bindings.clone();
        let copied = if let LegacyInstruction::Alias { source, .. } = instruction {""",
    ),
    (
        """                for names in self.bindings.values_mut() {
                    names.retain(|name| preserved.contains(name));
                }
                self.present.extend(produced_sites);
                if contract""",
        """                for names in self.bindings.values_mut() {
                    names.retain(|name| preserved.contains(name));
                }
                self.present.extend(produced_sites);
                for site in produced_sites {
                    trace_shape(*site, "contract_unnamed");
                }
                if contract""",
    ),
    (
        """    let mut out = incoming.clone();
    out.step(&cfg.nodes[node].instruction, &cfg.conditional_names);""",
        """    let mut out = incoming.clone();
    out.step(&cfg.nodes[node].instruction, &cfg.conditional_names);
    let snapshot_step = out.bindings.clone();""",
    ),
]

# Anchors of EDITS that AFTER_EDITS replaces, by their index in EDITS.
SUPERSEDED = {1, 5, 10}


def main():
    src = pathlib.Path(sys.argv[1]).read_text()
    after = "preserved: &BTreeSet<String>" in src
    edits = list(EDITS)
    if after:
        edits = [e for i, e in enumerate(edits) if i not in SUPERSEDED]
        # `outgoing`'s conditional-name retain instrumentation is split across
        # the superseded anchor; re-add its tail here.
        edits.append(
            (
                """    for names in out.bindings.values_mut() {
        names.retain(|name| cfg.conditional_names.contains(name));
    }""",
                """    for names in out.bindings.values_mut() {
        names.retain(|name| cfg.conditional_names.contains(name));
    }
    trace_drops(&snapshot_step, &out.bindings, "not_conditional_name");""",
            )
        )
        edits = AFTER_EDITS + edits
    for old, new in edits:
        if src.count(old) != 1:
            print(f"ANCHOR NOT UNIQUE ({src.count(old)}): {old[:70]!r}", file=sys.stderr)
            return 1
        src = src.replace(old, new)
    pathlib.Path(sys.argv[2]).write_text(src)
    print(f"instrumented -> {sys.argv[2]} ({len(EDITS)} edits)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
