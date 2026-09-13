//! Conditional identity facts for acyclic custody CFGs.
//!
//! `site -> names` means: on every path on which this acquisition exists,
//! each listed name denotes it. Absence is the neutral element at a join;
//! two present predecessors intersect names. No caller resource is seeded.
//! Cyclic CFGs derive no facts. This pre-pass never changes custody scheduling.
use super::*;

type Bindings = BTreeMap<usize, BTreeSet<String>>;

#[derive(Clone, Default)]
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
        return vec![Bindings::new(); cfg.nodes.len()];
    }
    let mut incoming = vec![None::<Facts>; cfg.nodes.len()];
    incoming[cfg.entry] = Some(Facts::default());
    for node in order {
        let Some(mut out) = incoming[node].clone() else {
            continue;
        };
        out.step(&cfg.nodes[node].instruction);
        for names in out.bindings.values_mut() {
            names.retain(|name| cfg.conditional_names.contains(name));
        }
        for (edge, &next) in cfg.nodes[node].successors.iter().enumerate() {
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
