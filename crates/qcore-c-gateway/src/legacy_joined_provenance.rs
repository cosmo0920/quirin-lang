//! Conditional identity facts for custody CFGs.
//!
//! `site -> names` means: on every path on which this acquisition exists,
//! each listed name denotes it. Absence is the neutral element at a join;
//! two present predecessors intersect names. No caller resource is seeded.
//! This pre-pass never changes custody scheduling.
//!
//! An **acyclic** CFG is solved in one topological pass, byte for byte as card
//! C-GATEWAY-WAVE (wave 31) wrote it. A **cyclic** CFG is solved by card
//! JOINED-PROVENANCE-CYCLES' greatest fixed point over the same transfer, and
//! derives facts only for sites whose creating node lies on no cycle — a site
//! created inside a loop stands for one abstract resource per *site* rather
//! than per dynamic object, so no per-site identity fact is a statement about
//! one of them. Declining is always available and always safe.
//!
//! A **call** no longer forgets every relation (card
//! JOINED-PROVENANCE-CALL-FRAMES). It forgets the places it writes — its
//! destination and `Out` slots, already removed by `rebound_names`, and a
//! `Reallocated` parameter — and every name outside the closed list of slots
//! no lvalue but the name itself designates; a name on that list is one no
//! callee can reach, so the call cannot change what it denotes. An
//! unrecognised name is always forgotten.
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

    /// One instruction's transfer. `preserved` is the closed list of names
    /// whose *slot* no lvalue but the name itself designates — card
    /// JOINED-PROVENANCE-CALL-FRAMES reads it at a call, and a name outside it
    /// is always forgotten there.
    fn step(&mut self, instruction: &LegacyInstruction, preserved: &BTreeSet<String>) {
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
                arguments,
                produced_sites,
                ..
            } => {
                // Card **JOINED-PROVENANCE-CALL-FRAMES**: a call forgets every
                // relation it could have written, and keeps only those it
                // provably could not.
                //
                // A name in `preserved` is an unshadowed object of *automatic*
                // storage duration this body declared, is not one of its
                // formals, and has no `&` anywhere in the body taking its
                // address. No pointer to that slot therefore exists for a
                // callee to hold, and the object has no linkage, so under the
                // same C object model constraint 10a already rests on — a
                // callee reaches only what its arguments and file-scope
                // objects give it — no call can change the value the name
                // denotes. Passing the name *by value* hands over that value,
                // never the slot.
                //
                // The places this call does write are already gone:
                // `rebound_names` forgot its destination and every `Out` slot
                // before this arm ran. A `Reallocated` result replaces the
                // value at its parameter, so that name is forgotten here too.
                // Everything else — a member, a subscript, a `*out`, a formal,
                // an address-taken local, a name this reader cannot classify —
                // is outside `preserved` and is forgotten exactly as before.
                if let ResourceReturnMode::Reallocated { parameter, .. } = contract.result
                    && let Some(Some(name)) = arguments.get(parameter)
                {
                    self.forget(name);
                }
                for names in self.bindings.values_mut() {
                    names.retain(|name| preserved.contains(name));
                }
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
    out.step(&cfg.nodes[node].instruction, &cfg.conditional_names);
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

    /// `0 -> 1 -> 2 -> {1, 3}`: nodes 1 and 2 lie on a cycle, nodes 0 and 3 do
    /// not, and the whole graph has a back edge so the topological pass
    /// declines it and the fixed point runs.
    fn cyclic_cfg(acquire_at: usize) -> LegacyCfg {
        let plain = |successors: Vec<usize>| LegacyCfgNode {
            instruction: LegacyInstruction::NoOp,
            successors,
            line: 0,
            is_exit: false,
        };
        let mut nodes = vec![
            plain(vec![1]),
            plain(vec![2]),
            plain(vec![1, 3]),
            plain(Vec::new()),
        ];
        nodes[acquire_at].instruction = LegacyInstruction::Acquire {
            destination: "p".to_string(),
            site: 7,
        };
        nodes[3].is_exit = true;
        LegacyCfg {
            function: "subject".to_string(),
            entry: 0,
            nodes,
            publications: PublicationFacts::default(),
            conditional_names: BTreeSet::from(["p".to_string()]),
        }
    }

    #[test]
    fn a_back_edge_no_longer_declines_an_off_cycle_acquisition() {
        // Card JOINED-PROVENANCE-CYCLES. Wave 31 returned empty bindings for
        // every node of this graph purely because it carries a back edge.
        let facts = derive(&cyclic_cfg(0));
        assert_eq!(
            facts[3].get(&7),
            Some(&BTreeSet::from(["p".to_string()])),
            "{facts:?}"
        );
    }

    #[test]
    fn a_site_created_inside_the_cycle_derives_nothing() {
        // The eligibility rule itself, driven where nothing downstream can
        // mask it: node 2 lies on the cycle, so site 7 is not a statement
        // about one object and no node may carry a fact for it.
        let facts = derive(&cyclic_cfg(2));
        assert!(
            facts.iter().all(|bindings| bindings.is_empty()),
            "{facts:?}"
        );
    }

    #[test]
    fn an_eligible_site_does_not_carry_an_ineligible_one_with_it() {
        // Both kinds of site in one graph, so the `sites.is_empty()` early
        // return cannot stand in for the published-bindings filter: node 0's
        // site 7 is eligible and must survive, while node 2's site 8 is
        // created on the cycle and must appear on no node.
        let mut cfg = cyclic_cfg(0);
        cfg.nodes[2].instruction = LegacyInstruction::Acquire {
            destination: "q".to_string(),
            site: 8,
        };
        cfg.conditional_names.insert("q".to_string());
        let facts = derive(&cfg);
        assert_eq!(
            facts[3].get(&7),
            Some(&BTreeSet::from(["p".to_string()])),
            "{facts:?}"
        );
        assert!(
            facts.iter().all(|bindings| !bindings.contains_key(&8)),
            "{facts:?}"
        );
    }

    /// One contracted call node. Every aggregate and nested field is empty,
    /// which is what the overwhelming majority of the pinned corpus's calls
    /// carry; the arms this card adds read none of them.
    fn call(
        destination: Option<&str>,
        arguments: Vec<Option<&str>>,
        contract: ResourceFunctionContract,
    ) -> LegacyInstruction {
        LegacyInstruction::ContractCall {
            function: "callee".to_string(),
            destination: destination.map(str::to_string),
            arguments: arguments
                .into_iter()
                .map(|argument| argument.map(str::to_string))
                .collect(),
            contract,
            produced_sites: Vec::new(),
            aggregate_reads: Vec::new(),
            aggregate_consumed: Vec::new(),
            aggregate_addresses: Vec::new(),
            alias_reads: Vec::new(),
            nested: None,
        }
    }

    /// `site 7 -> {p}`, as an `Acquire` would have left it.
    fn named(name: &str) -> Facts {
        Facts {
            present: BTreeSet::from([7]),
            bindings: BTreeMap::from([(7, BTreeSet::from([name.to_string()]))]),
        }
    }

    fn preserved(names: [&str; 1]) -> BTreeSet<String> {
        names.iter().map(|name| (*name).to_string()).collect()
    }

    /// What site 7 is still claimed to be named by. A site whose entry is gone
    /// and a site whose entry carries no name are the same fact — `release_site`
    /// asks only `facts.get(&site)?.contains(source)` — so the assertions below
    /// read the names rather than the shape of the map.
    fn names_of_site(facts: &Facts) -> BTreeSet<String> {
        facts.bindings.get(&7).cloned().unwrap_or_default()
    }

    #[test]
    fn a_call_no_longer_forgets_a_relation_it_cannot_write() {
        // Card JOINED-PROVENANCE-CALL-FRAMES, the whole point: `callee(q)`
        // writes neither `p`'s slot nor anything that designates it, so the
        // relation stands. Before this card every call cleared it.
        let mut facts = named("p");
        facts.step(
            &call(
                None,
                vec![Some("q")],
                ResourceFunctionContract::new(
                    [ResourceParameterMode::Borrowed],
                    ResourceReturnMode::None,
                ),
            ),
            &preserved(["p"]),
        );
        assert_eq!(
            facts.bindings.get(&7),
            Some(&BTreeSet::from(["p".to_string()]))
        );
    }

    #[test]
    fn a_call_forgets_a_name_the_frame_list_does_not_carry() {
        // The guard is built positively: a name the frame reader did not
        // recognise is forgotten at a call exactly as before. Driven here
        // rather than through `derive`, because `outgoing` applies the same
        // membership test after every node, so no graph can present this
        // `step` with an unrecognised name — see the report's
        // CALL-FRAMES-ALLOWLIST-LOCAL-ONLY.
        let mut facts = named("p");
        facts.step(
            &call(
                None,
                vec![Some("q")],
                ResourceFunctionContract::new(
                    [ResourceParameterMode::Borrowed],
                    ResourceReturnMode::None,
                ),
            ),
            &preserved(["elsewhere"]),
        );
        assert!(names_of_site(&facts).is_empty(), "{:?}", facts.bindings);
    }

    #[test]
    fn a_call_still_forgets_its_own_destination() {
        let mut facts = named("p");
        facts.step(
            &call(
                Some("p"),
                Vec::new(),
                ResourceFunctionContract::new([], ResourceReturnMode::None),
            ),
            &preserved(["p"]),
        );
        assert!(names_of_site(&facts).is_empty(), "{:?}", facts.bindings);
    }

    #[test]
    fn a_call_still_forgets_an_out_slot() {
        let mut facts = named("p");
        facts.step(
            &call(
                None,
                vec![Some("p")],
                ResourceFunctionContract::new(
                    [ResourceParameterMode::Out { nullable: false }],
                    ResourceReturnMode::None,
                ),
            ),
            &preserved(["p"]),
        );
        assert!(names_of_site(&facts).is_empty(), "{:?}", facts.bindings);
    }

    #[test]
    fn a_call_still_forgets_a_reallocated_parameter() {
        // The result replaces the value at parameter 0, so `p` no longer
        // denotes what it denoted, even though no callee wrote `p`'s slot.
        let mut facts = named("p");
        facts.step(
            &call(
                Some("q"),
                vec![Some("p")],
                ResourceFunctionContract::new(
                    [ResourceParameterMode::Owned { nullable: false }],
                    ResourceReturnMode::Reallocated {
                        parameter: 0,
                        nullable: false,
                    },
                ),
            ),
            &preserved(["p"]),
        );
        assert!(names_of_site(&facts).is_empty(), "{:?}", facts.bindings);
    }

    #[test]
    fn an_acquisition_survives_an_intervening_call_end_to_end() {
        // The same rule reached through `derive`, which is the route the
        // analyzer actually takes: acquire, call, exit. Before this card the
        // call cleared the relation and node 2 carried nothing.
        let plain = |instruction, successors: Vec<usize>| LegacyCfgNode {
            instruction,
            successors,
            line: 0,
            is_exit: false,
        };
        let mut nodes = vec![
            plain(
                LegacyInstruction::Acquire {
                    destination: "p".to_string(),
                    site: 7,
                },
                vec![1],
            ),
            plain(
                call(
                    None,
                    vec![Some("q")],
                    ResourceFunctionContract::new(
                        [ResourceParameterMode::Borrowed],
                        ResourceReturnMode::None,
                    ),
                ),
                vec![2],
            ),
            plain(LegacyInstruction::NoOp, Vec::new()),
        ];
        nodes[2].is_exit = true;
        let cfg = LegacyCfg {
            function: "subject".to_string(),
            entry: 0,
            nodes,
            publications: PublicationFacts::default(),
            conditional_names: BTreeSet::from(["p".to_string()]),
        };
        assert_eq!(
            derive(&cfg)[2].get(&7),
            Some(&BTreeSet::from(["p".to_string()]))
        );
    }

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
