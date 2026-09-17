//! Conditional identity facts for acyclic custody CFGs.
//!
//! `site -> names` means: on every path on which this acquisition exists,
//! each listed name denotes it. Absence is the neutral element at a join;
//! two present predecessors intersect names. No caller resource is seeded.
//! Cyclic CFGs derive no facts. This pre-pass never changes custody scheduling.
use super::*;

type Bindings = BTreeMap<usize, BTreeSet<String>>;

#[derive(Clone, Default, PartialEq, Eq)]
struct Facts {
    present: BTreeSet<usize>,
    bindings: Bindings,
}

impl Facts {
    fn forget(&mut self, name: &str) {
        for names in self.bindings.values_mut() {
            names.remove(name);
        }
    }

    fn join(&mut self, other: &Self) {
        for site in self
            .present
            .union(&other.present)
            .copied()
            .collect::<Vec<_>>()
        {
            let names = match (self.present.contains(&site), other.present.contains(&site)) {
                (true, true) => self
                    .bindings
                    .get(&site)
                    .into_iter()
                    .flatten()
                    .filter(|name| {
                        other
                            .bindings
                            .get(&site)
                            .is_some_and(|names| names.contains(*name))
                    })
                    .cloned()
                    .collect(),
                (true, false) => self.bindings.get(&site).cloned().unwrap_or_default(),
                (false, true) => other.bindings.get(&site).cloned().unwrap_or_default(),
                (false, false) => unreachable!(),
            };
            self.bindings.insert(site, names);
        }
        self.present.extend(&other.present);
    }

    fn step(&mut self, instruction: &LegacyInstruction) {
        let copied = if let LegacyInstruction::Alias { source, .. } = instruction {
            self.bindings
                .iter()
                .filter_map(|(site, names)| names.contains(source).then_some(*site))
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        for name in rebound_names(instruction) {
            self.forget(name);
        }
        match instruction {
            LegacyInstruction::Acquire { site, destination } => {
                self.present.insert(*site);
                self.bindings
                    .insert(*site, BTreeSet::from([destination.clone()]));
            }
            LegacyInstruction::Alias { destination, .. } => {
                for site in copied {
                    self.bindings
                        .entry(site)
                        .or_default()
                        .insert(destination.clone());
                }
            }
            LegacyInstruction::ContractCall {
                destination,
                contract,
                produced_sites,
                ..
            } => {
                // The contract vocabulary does not establish preservation of
                // arbitrary local slots. Forget prior relations across calls;
                // only a freshly returned local identity is established below.
                self.bindings.clear();
                self.present.extend(produced_sites);
                if contract
                    .parameters
                    .iter()
                    .all(|mode| matches!(mode, ResourceParameterMode::Borrowed))
                    && matches!(contract.result, ResourceReturnMode::Owned { .. })
                    && let (Some(destination), [site]) = (destination, produced_sites.as_slice())
                {
                    self.bindings
                        .insert(*site, BTreeSet::from([destination.clone()]));
                }
            }
            LegacyInstruction::Unknown { writes, .. } => {
                for names in self.bindings.values_mut() {
                    names.retain(|name| !writes.poisons(name));
                }
            }
            LegacyInstruction::AggregateStore { .. } => self.bindings.clear(),
            _ => {}
        }
    }
}

/// TEMPORARY measurement instrumentation for card JOINED-PROVENANCE-CYCLES.
/// Writes one JSON line per analysed function when `QCORE_JPC_MEASURE` names a
/// file. Reports ground truth from the analyzer's own CFG, never a source-text
/// classifier. Removed before the card publishes.
fn measure(cfg: &LegacyCfg, reachable: &BTreeSet<usize>, cyclic: bool) {
    let Ok(path) = std::env::var("QCORE_JPC_MEASURE") else {
        return;
    };
    use std::io::Write as _;
    // `node` lies on a cycle iff it is reachable from one of its own successors.
    let on_cycle = |node: usize| {
        let mut seen = BTreeSet::new();
        let mut pending: Vec<usize> = cfg.nodes[node].successors.clone();
        while let Some(next) = pending.pop() {
            if next == node {
                return true;
            }
            if seen.insert(next) {
                pending.extend(&cfg.nodes[next].successors);
            }
        }
        false
    };
    // Every way a site is created, not just a bare `Acquire`: in the pinned
    // corpus a contracted allocator arrives as a `ContractCall` carrying
    // `produced_sites`, and counting only `Acquire` reports zero everywhere.
    let mut sites = 0usize;
    let mut sites_off_cycle = 0usize;
    let mut bare_acquires = 0usize;
    for &node in reachable {
        let created = match &cfg.nodes[node].instruction {
            LegacyInstruction::Acquire { site, .. } => {
                bare_acquires += 1;
                vec![*site]
            }
            LegacyInstruction::ContractCall { produced_sites, .. } => produced_sites.clone(),
            _ => Vec::new(),
        };
        if created.is_empty() {
            continue;
        }
        sites += created.len();
        if !on_cycle(node) {
            sites_off_cycle += created.len();
        }
    }
    let line = format!(
        "{{\"function\":{:?},\"nodes\":{},\"reachable\":{},\"cyclic\":{},\"bare_acquires\":{},\"sites\":{},\"sites_off_cycle\":{},\"conditional_names\":{}}}\n",
        cfg.function,
        cfg.nodes.len(),
        reachable.len(),
        cyclic,
        bare_acquires,
        sites,
        sites_off_cycle,
        cfg.conditional_names.len(),
    );
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = file.write_all(line.as_bytes());
    }
}

pub(super) fn derive(cfg: &LegacyCfg) -> Vec<Bindings> {
    let mut reachable = BTreeSet::new();
    let mut pending = vec![cfg.entry];
    while let Some(node) = pending.pop() {
        if reachable.insert(node) {
            pending.extend(&cfg.nodes[node].successors);
        }
    }
    let mut degrees = vec![0; cfg.nodes.len()];
    for &node in &reachable {
        for &next in &cfg.nodes[node].successors {
            degrees[next] += 1;
        }
    }
    let mut ready = reachable
        .iter()
        .copied()
        .filter(|node| degrees[*node] == 0)
        .collect::<VecDeque<_>>();
    let mut order = Vec::new();
    while let Some(node) = ready.pop_front() {
        order.push(node);
        for &next in &cfg.nodes[node].successors {
            degrees[next] -= 1;
            if degrees[next] == 0 {
                ready.push_back(next);
            }
        }
    }
    measure(cfg, &reachable, order.len() != reachable.len());
    if order.len() != reachable.len() {
        return fixed_point(cfg, &reachable);
    }
    let mut incoming = vec![None::<Facts>; cfg.nodes.len()];
    incoming[cfg.entry] = Some(Facts::default());
    for node in order {
        let Some(facts) = incoming[node].clone() else {
            continue;
        };
        let branches = outgoing(cfg, node, &facts);
        for (edge, &next) in cfg.nodes[node].successors.iter().enumerate() {
            let branch = branches[edge].clone();
            match &mut incoming[next] {
                Some(current) => current.join(&branch),
                slot => *slot = Some(branch),
            }
        }
    }
    incoming
        .into_iter()
        .map(|facts| facts.unwrap_or_default().bindings)
        .collect()
}

/// One node's outgoing facts, one per successor edge: the instruction's
/// transfer, the conditional-name restriction, then the null edge's exclusion
/// of every site the tested name denotes. Shared by the acyclic pass and the
/// fixed point so the two routes cannot drift apart.
fn outgoing(cfg: &LegacyCfg, node: usize, incoming: &Facts) -> Vec<Facts> {
    let mut out = incoming.clone();
    out.step(&cfg.nodes[node].instruction);
    for names in out.bindings.values_mut() {
        names.retain(|name| cfg.conditional_names.contains(name));
    }
    (0..cfg.nodes[node].successors.len())
        .map(|edge| {
            let mut branch = out.clone();
            if let LegacyInstruction::BranchOnNull {
                source,
                true_is_null,
            } = &cfg.nodes[node].instruction
                && ((edge == 0) == *true_is_null)
            {
                let absent = branch
                    .bindings
                    .iter()
                    .filter_map(|(site, names)| names.contains(source).then_some(*site))
                    .collect::<Vec<_>>();
                for site in absent {
                    branch.present.remove(&site);
                    branch.bindings.remove(&site);
                }
            }
            branch
        })
        .collect()
}

/// The iteration bound. Exceeding it declines the whole function, exactly as
/// this module did for every cyclic CFG before card JOINED-PROVENANCE-CYCLES.
const FIXED_POINT_ROUNDS: usize = 64;

/// Sites whose creating node lies on **no** cycle, and so runs at most once per
/// call. A site created inside a loop stands for one abstract resource per
/// *site*, not per dynamic object, so `site -> names` would not be a statement
/// about one of them; such a site derives no fact here. This is what makes the
/// relation's reading — "on every path on which this acquisition exists, each
/// listed name denotes it" — carry over to a cyclic graph unchanged.
fn acyclic_sites(cfg: &LegacyCfg, reachable: &BTreeSet<usize>) -> BTreeSet<usize> {
    let on_cycle = |node: usize| {
        let mut seen = BTreeSet::new();
        let mut pending: Vec<usize> = cfg.nodes[node].successors.clone();
        while let Some(next) = pending.pop() {
            if next == node {
                return true;
            }
            if seen.insert(next) {
                pending.extend(&cfg.nodes[next].successors);
            }
        }
        false
    };
    let mut sites = BTreeSet::new();
    for &node in reachable {
        let created: Vec<usize> = match &cfg.nodes[node].instruction {
            LegacyInstruction::Acquire { site, .. } => vec![*site],
            LegacyInstruction::ContractCall { produced_sites, .. } => produced_sites.clone(),
            _ => Vec::new(),
        };
        if created.is_empty() || on_cycle(node) {
            continue;
        }
        sites.extend(created);
    }
    sites
}

/// The same relation as the acyclic pre-pass, computed as the **greatest fixed
/// point** over a cyclic graph: every node but the entry starts at the top
/// element — no site present, so every guarantee is vacuous — and predecessors'
/// outgoing facts are met until nothing moves. Absence stays neutral and two
/// present predecessors still intersect, so a claim can never be justified by
/// the cycle alone: the entry's own concrete facts are one of the meet's
/// operands on every path into a loop.
///
/// Iteration only ever weakens a claim, so declining early is always safe: a
/// graph that has not settled within [`FIXED_POINT_ROUNDS`] derives nothing.
fn fixed_point(cfg: &LegacyCfg, reachable: &BTreeSet<usize>) -> Vec<Bindings> {
    let result = fixed_point_inner(cfg, reachable);
    // TEMPORARY: card JOINED-PROVENANCE-CYCLES reachability measurement.
    if let Ok(path) = std::env::var("QCORE_JPC_FIXPOINT") {
        use std::io::Write as _;
        let nodes_with_facts = result.iter().filter(|b| !b.is_empty()).count();
        let sites: BTreeSet<usize> = result.iter().flat_map(|b| b.keys().copied()).collect();
        let line = format!(
            "{{\"function\":{:?},\"nodes_with_facts\":{},\"sites_with_facts\":{}}}\n",
            cfg.function,
            nodes_with_facts,
            sites.len(),
        );
        if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
            let _ = file.write_all(line.as_bytes());
        }
    }
    result
}

fn fixed_point_inner(cfg: &LegacyCfg, reachable: &BTreeSet<usize>) -> Vec<Bindings> {
    let declined = vec![Bindings::new(); cfg.nodes.len()];
    let sites = acyclic_sites(cfg, reachable);
    if sites.is_empty() {
        return declined;
    }
    let mut predecessors = vec![Vec::new(); cfg.nodes.len()];
    for &node in reachable {
        for &next in &cfg.nodes[node].successors {
            predecessors[next].push(node);
        }
    }
    let mut incoming = vec![None::<Facts>; cfg.nodes.len()];
    incoming[cfg.entry] = Some(Facts::default());
    for _ in 0..FIXED_POINT_ROUNDS {
        let mut next_round = incoming.clone();
        for &node in reachable {
            if node == cfg.entry {
                continue;
            }
            let mut arrived = None::<Facts>;
            for &previous in &predecessors[node] {
                // A predecessor still at the top element is neutral: meeting
                // with it changes nothing.
                let Some(facts) = incoming[previous].as_ref() else {
                    continue;
                };
                let branches = outgoing(cfg, previous, facts);
                for (edge, &successor) in cfg.nodes[previous].successors.iter().enumerate() {
                    if successor != node {
                        continue;
                    }
                    match &mut arrived {
                        Some(current) => current.join(&branches[edge]),
                        slot => *slot = Some(branches[edge].clone()),
                    }
                }
            }
            next_round[node] = arrived;
        }
        if next_round == incoming {
            return incoming
                .into_iter()
                .map(|facts| {
                    let mut bindings = facts.unwrap_or_default().bindings;
                    bindings.retain(|site, names| sites.contains(site) && !names.is_empty());
                    bindings
                })
                .collect();
        }
        incoming = next_round;
    }
    declined
}

/// Recognise one plain, shallow release. The identity refusal remains; this
/// only establishes the disposition of the conditional local acquisition.
pub(super) fn release_site(
    node: &LegacyCfgNode,
    state: &AbstractState,
    facts: &Bindings,
) -> Option<(usize, String)> {
    let LegacyInstruction::ContractCall {
        contract,
        arguments,
        destination: None,
        nested: None,
        aggregate_reads,
        aggregate_consumed,
        aggregate_addresses,
        ..
    } = &node.instruction
    else {
        return None;
    };
    if !matches!(contract.result, ResourceReturnMode::None)
        || !aggregate_reads.is_empty()
        || !aggregate_consumed.is_empty()
        || !aggregate_addresses.is_empty()
    {
        return None;
    }
    let mut released = None;
    for (mode, argument) in contract.parameters.iter().zip(arguments) {
        match mode {
            ResourceParameterMode::Borrowed => {}
            ResourceParameterMode::Released { .. } if released.is_none() => {
                released = argument.as_deref()
            }
            _ => return None,
        }
    }
    let source = released?;
    if !is_plain_name_place(source) || arguments.len() != contract.parameters.len() {
        return None;
    }
    let pointer = state.pointers.get(source)?;
    if !pointer.unknown || pointer.may_be_null || pointer.targets.len() != 1 {
        return None;
    }
    let site = *pointer.targets.first()?;
    // TEMPORARY: card JOINED-PROVENANCE-CYCLES premise census.
    if let Ok(path) = std::env::var("QCORE_JPC_PREMISE") {
        use std::io::Write as _;
        let named = facts.get(&site).is_some_and(|names| names.contains(source));
        let blocking = [
            ("not_named_by_facts", !named),
            ("not_live", state.resources.get(&site) != Some(&LIVE)),
            ("nullable_site", state.nullable_sites.contains(&site)),
            ("unresolved_effect", state.unresolved_effect_sites.contains(&site)),
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
        let line = format!(
            "{{\"site\":{},\"blocking\":{:?}}}\n",
            site,
            blocking.join("+")
        );
        if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
            let _ = file.write_all(line.as_bytes());
        }
    }
    if !facts.get(&site)?.contains(source)
        || state.resources.get(&site) != Some(&LIVE)
        || state.nullable_sites.contains(&site)
        || state.unresolved_effect_sites.contains(&site)
        || state.contained.contains_key(&site)
        || state.field_links.contains_key(&site)
        || state.conditional_replacements.contains_key(&site)
    {
        return None;
    }
    Some((site, source.to_owned()))
}

pub(super) fn finish_release(
    candidate: Option<(usize, String)>,
    state: &mut AbstractState,
    diagnostics: &BTreeSet<LegacyDiagnostic>,
) {
    let Some((site, source)) = candidate else {
        return;
    };
    // Any additional failed premise preserves the ordinary refusing result.
    if diagnostics.len() != 1
        || !diagnostics.iter().all(|d| {
            matches!(
        &d.kind, LegacyDiagnosticKind::UnknownAlias { variable } if variable == &source)
        })
    {
        return;
    }
    state.resources.insert(site, RELEASED);
    state.unresolved_effect_sites.remove(&site);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presence_controls_the_join_not_the_pointer_target_union() {
        for left_present in [false, true] {
            for right_present in [false, true] {
                for left_alias in [false, true] {
                    for right_alias in [false, true] {
                        let make = |present, alias| Facts {
                            present: if present {
                                BTreeSet::from([0])
                            } else {
                                BTreeSet::new()
                            },
                            bindings: if present && alias {
                                BTreeMap::from([(0, BTreeSet::from(["p".into()]))])
                            } else {
                                BTreeMap::new()
                            },
                        };
                        let mut result = make(left_present, left_alias);
                        result.join(&make(right_present, right_alias));
                        let guaranteed = result
                            .bindings
                            .get(&0)
                            .is_some_and(|names| names.contains("p"));
                        assert_eq!(
                            guaranteed,
                            (left_present || right_present)
                                && (!left_present || left_alias)
                                && (!right_present || right_alias)
                        );
                    }
                }
            }
        }
    }
}
