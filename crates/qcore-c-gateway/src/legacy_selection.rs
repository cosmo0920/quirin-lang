//! Card **DETACH-RESULT-WRAPPER-COMPOSITION** (gateway rule 33): the checked
//! removed-selection composition, connected to the custody gateway. Card
//! **RULE33-PILOT**: the rule's body is the package
//! `crates/gateway-rules/rule-33-removed-selection` (the frame bodies' word
//! shapes, the caller expansion, the selection decisions); this file is the
//! host side: it reads the frame against the contract environment and the
//! type model, builds the selection entry, analyses the row, and asks the
//! package under `QCORE_GATEWAY_RULES_OFF` (`33` off: nothing is expanded and
//! no frame is installed, which is the gateway before rule 33).
//!
//! # The two frames
//!
//! A *selection detach* `F` is a function whose complete body is exactly
//! (after renaming and the existing NULL-spelling normalization)
//!
//! ```c
//! R *F(R *P, int W) { if (W < 0) { return NULL; } return D(P, L(P, (size_t)W)); }
//! ```
//!
//! where `L` passes the existing complete indexed lookup frame
//! ([`super::lookup_route::read`]) with its borrowed-from summary in force,
//! `D` is in force with the exact same-item `Removed` shape and the exact
//! named `detach_list_representation` premise, `P` is declared a pointer to
//! the lookup's canonical record and `size_t` is an unsigned integer type.
//! A *selection release* `G` is exactly `void G(R *P, int W) { X(F(P, W)); }`
//! for such an `F` and an `X` in force with exactly one nullable heap
//! `Released` formal and no result. Anything else is no frame.
//!
//! # What the frame states
//!
//! The selected child is `sel(P, W) = (W < 0) ? NULL : L(P, (size_t)W)`, the
//! value the frame computes. Its membership is a **prerequisite** the caller
//! transports: it is never read off borrowed-from provenance or off the
//! representation premise, which stays a `DeclaredAssumption` of `D`.
//!
//! # Expansion
//!
//! [`expand`] rewrites only the analysed function's body, as the other
//! build-stream rewrites do: a frame's own body becomes its constituent calls
//! (the reserved places are C11 7.1.3 names, required unspelled), and a call
//! `d = F(q, K);`, `R *d = F(q, K);`, `F(q, K);` or `G(q, K);` with a plain
//! parent name and a decimal constant `K` in 0–255 becomes the same constituent
//! calls with the guard decided (`K >= 0`) and `(size_t)K == K`. Every existing
//! lookup-identity, physical-prefix, membership, NULL, alias and same-item
//! check then applies unchanged at the caller, and `D`'s declaration and
//! premise are consulted there. Every other call of `F` or `G` keeps the
//! uncontracted-call refusal.
//!
//! # Validation
//!
//! A frame's own row is analysed by the ordinary analyzer under a selection
//! entry ([`row`]): `P` is one live, settled, non-NULL owned site, `W` is
//! borrowed. At the frame's one lookup call (reached with no earlier call or
//! write) the selected child appears as one nullable child uniquely contained
//! by `P` ([`materialize`]); the existing adapter then decides the detach. The
//! run must be diagnostic-free, select on some path and accept every exit
//! (`rule_33_removed_selection::accept_exit`). Nothing is published as a
//! `ResourceFunctionContract`, so no other route, unit or ordinary W4/fresh
//! result reading sees the selected child as fresh custody. `QCore.RemovedSelection`.

use std::collections::BTreeSet;

use qcore_gateway_rule_api::traits::RulePackage;
use qcore_gateway_rule_api::{BodyQuery, EntryRow, StatementExpansion};
use rule_33_removed_selection::{
    Disposition, ExitFacts, FRAME_SPAN, ITEM_PLACE, MaterializeFacts, RESERVED, RemovedSelection,
    Returned, Selected, ValidationRun, materialize as decide,
};

use super::{
    AbstractPointer, AbstractState, CONTAINED, Derivation, DerivedExtent, EntryCustody, LIVE,
    LegacyCfg, LegacyCfgNode, LegacyFunctionSummary, LegacyInstruction, ResourceContracts,
    ResourceKind, ResourceParameterMode, ResourceReturnMode, ReturnedValue, SiteDisposition, Token,
    find_function_body, function_parameter_range, function_parameters, matching,
};

/// A checked selection frame (the package's own type; the host builds it
/// after its checks).
pub(crate) use rule_33_removed_selection::Frame;

/// Rule 33's package, asked at its two registered arms.
const PACKAGE: [RemovedSelection; 1] = [RemovedSelection];

/// Whether rule 33 answers in this run (`QCORE_GATEWAY_RULES_OFF`).
pub(crate) fn enabled() -> bool {
    crate::gateway_rules::enabled(rule_33_removed_selection::ID)
}

/// The selection the entry hands the analyzer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SelectionEntry {
    pub(crate) owner: String,
    pub(crate) owner_site: usize,
    pub(crate) lookup: String,
    pub(crate) phantom: usize,
    pub(crate) pristine: bool,
}

fn words(tokens: &[Token]) -> Vec<&str> {
    tokens.iter().map(|token| token.text.as_str()).collect()
}

fn identifier(text: &str) -> bool {
    super::is_identifier(text)
}

/// `R * P , int W`: the owner's pointee spelling, the two names.
fn signature(tokens: &[Token], function: &str) -> Option<(Vec<String>, String, String)> {
    let parameters = function_parameters(tokens, function)?;
    let [Some(owner), Some(index)] = parameters.as_slice() else {
        return None;
    };
    let (start, _) = find_function_body(tokens, function)?;
    let (open, close) = function_parameter_range(tokens, function, start)?;
    let list = words(&tokens[open + 1..close]);
    let groups = list.split(|word| *word == ",").collect::<Vec<_>>();
    let [first, second] = groups.as_slice() else {
        return None;
    };
    let pointee = match *first {
        [alias, "*", name] if name == owner && identifier(alias) => vec![(*alias).to_owned()],
        ["struct", tag, "*", name] if name == owner && identifier(tag) => {
            vec!["struct".to_owned(), (*tag).to_owned()]
        }
        _ => return None,
    };
    if *second != ["int", index.as_str()]
        || owner == index
        || [owner.as_str(), index.as_str()]
            .iter()
            .any(|name| name.starts_with(RESERVED))
    {
        return None;
    }
    Some((pointee, owner.clone(), index.clone()))
}

fn small_body(tokens: &[Token], function: &str) -> Option<(usize, usize)> {
    let (start, _) = find_function_body(tokens, function)?;
    let end = matching(tokens, start, "{", "}")?;
    (end - start < FRAME_SPAN).then_some((start, end))
}

/// The cheap lexical shape of `F`'s body, before any type model is read (the
/// package's word shape over the formals the host read).
fn detach_shape(tokens: &[Token], function: &str) -> Option<(String, String, String, String)> {
    let (start, end) = small_body(tokens, function)?;
    let (_, owner, index) = signature(tokens, function)?;
    let (detach, lookup) = rule_33_removed_selection::detach_body(
        &words(&tokens[start..=end]),
        &owner,
        &index,
        function,
    )?;
    Some((owner, index, detach, lookup))
}

/// The cheap lexical shape of `G`'s body.
fn release_shape(tokens: &[Token], function: &str) -> Option<(String, String, String, String)> {
    let (start, end) = small_body(tokens, function)?;
    let (_, owner, index) = signature(tokens, function)?;
    let (release, inner) = rule_33_removed_selection::release_body(
        &words(&tokens[start..=end]),
        &owner,
        &index,
        function,
    )?;
    Some((owner, index, release, inner))
}

fn same_item_detach(contracts: &ResourceContracts, detach: &str) -> bool {
    use qcore_contract::contract_loader::{PremiseOrigin, declared_premises};
    let Some(contract) = contracts.contract(detach) else {
        return false;
    };
    let shape = contract.parameters
        == [
            ResourceParameterMode::Owned { nullable: false },
            ResourceParameterMode::Removed {
                owner_parameter: 0,
                nullable: true,
            },
        ]
        && contract.result
            == ResourceReturnMode::Removed {
                parameter: 1,
                nullable: true,
            }
        && contract.result_kind == ResourceKind::Heap
        && contract.parameter_kind(0) == ResourceKind::Heap
        && contract.parameter_kind(1) == ResourceKind::Heap
        && !contract.has_container_effect()
        && contracts.lifetime_contract(detach).is_none()
        && !contracts.lifetime_summary(detach).is_some()
        && !contracts.retaining().contains_key(detach)
        && !contracts.has_guarded_interface(detach);
    shape
        && declared_premises(contracts)
            .ok()
            .and_then(|premises| premises.get(detach).cloned())
            .is_some_and(|premises| {
                premises.iter().any(|premise| {
                    premise.origin == PremiseOrigin::Declared
                        && premise.name == super::detach_route::REPRESENTATION_NAME
                        && premise.statement == super::detach_route::REPRESENTATION_STATEMENT
                })
            })
}

fn borrowed_lookup(contracts: &ResourceContracts, lookup: &str) -> bool {
    contracts.contract(lookup).is_some_and(|contract| {
        contract.parameters
            == [
                ResourceParameterMode::Borrowed,
                ResourceParameterMode::Borrowed,
            ]
            && contract.result
                == ResourceReturnMode::Derived {
                    parameter: 0,
                    extent: DerivedExtent::Borrowed,
                    nullable: true,
                    const_dropped: true,
                }
            && !contract.has_container_effect()
    }) && contracts.lifetime_contract(lookup).is_none()
        && !contracts.lifetime_summary(lookup).is_some()
}

fn nullable_release(contracts: &ResourceContracts, release: &str) -> bool {
    contracts.contract(release).is_some_and(|contract| {
        contract.parameters == [ResourceParameterMode::Released { nullable: true }]
            && contract.result == ResourceReturnMode::None
            && contract.parameter_kind(0) == ResourceKind::Heap
            && !contract.has_container_effect()
    }) && contracts.lifetime_contract(release).is_none()
        && !contracts.lifetime_summary(release).is_some()
        && !contracts.retaining().contains_key(release)
        && !contracts.has_guarded_interface(release)
        && contracts.relational_contract(release).is_none()
}

/// The owner's record, resolved by the unit's type model, must be the
/// lookup's canonical record, and `size_t` an unsigned integer type.
fn typed(tokens: &[Token], pointee: &[String], record: &str) -> bool {
    let all = words(tokens);
    let model = crate::typed_field_access::TypedAccessModel::read(&all);
    let resolved = match pointee {
        [alias] => model.defining_struct_alias(alias),
        [keyword, tag] if keyword == "struct" && model.record_definition_is_complete(tag) => {
            Some(tag.as_str())
        }
        _ => None,
    };
    resolved == Some(record)
        && model
            .integer_class(&["size_t"])
            .is_some_and(|class| !class.signed)
}

/// An external-linkage `inline` definition may be bypassed for another
/// definition at any call (C11 6.7.4p7), so it is no frame. Every other
/// definition is the body this unit's calls run, as for a selected summary.
fn runs_this_body(tokens: &[Token], function: &str) -> bool {
    let inline = ["inline", "__inline", "__inline__"]
        .iter()
        .any(|specifier| {
            super::callee_scope::definition_has_specifier(tokens, function, specifier)
        });
    !inline || super::callee_scope::has_internal_linkage(tokens, function)
}

fn detach_frame(tokens: &[Token], function: &str, contracts: &ResourceContracts) -> Option<Frame> {
    let (owner, index, detach, lookup) = detach_shape(tokens, function)?;
    if !runs_this_body(tokens, function) {
        return None;
    }
    let (pointee, ..) = signature(tokens, function)?;
    if !same_item_detach(contracts, &detach) || !borrowed_lookup(contracts, &lookup) {
        return None;
    }
    let read = super::lookup_route::read(tokens, &lookup)?;
    typed(tokens, &pointee, &read.record).then_some(Frame {
        disposition: Disposition::Returned,
        owner,
        index,
        pointee,
        record: read.record,
        lookup,
        detach,
        release: None,
        inner: None,
    })
}

/// The checked frame `function`'s body is in this unit under `contracts`.
/// The reserved places must be unspelled in the stream (C11 7.1.3).
pub(crate) fn reserved_unspelled(tokens: &[Token]) -> bool {
    !tokens.iter().any(|token| token.text.starts_with(RESERVED))
}

/// The caller must also check [`reserved_unspelled`].
pub(crate) fn frame(
    tokens: &[Token],
    function: &str,
    contracts: &ResourceContracts,
) -> Option<Frame> {
    if detach_shape(tokens, function).is_some() {
        return detach_frame(tokens, function, contracts);
    }
    let (owner, index, release, inner) = release_shape(tokens, function)?;
    if !runs_this_body(tokens, function) {
        return None;
    }
    let (pointee, ..) = signature(tokens, function)?;
    if !nullable_release(contracts, &release) {
        return None;
    }
    let composed = detach_frame(tokens, &inner, contracts)?;
    let read = super::lookup_route::read(tokens, &composed.lookup)?;
    typed(tokens, &pointee, &read.record).then_some(Frame {
        disposition: Disposition::Released,
        owner,
        index,
        pointee,
        record: read.record,
        lookup: composed.lookup,
        detach: composed.detach,
        release: Some(release),
        inner: Some(inner),
    })
}

/// One pass over the stream: the named functions with a frame-sized body.
fn small_definitions(tokens: &[Token], names: &BTreeSet<&str>) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    for (index, token) in tokens.iter().enumerate() {
        if !names.contains(token.text.as_str())
            || tokens.get(index + 1).map(|t| t.text.as_str()) != Some("(")
        {
            continue;
        }
        let Some(close) = matching(tokens, index + 1, "(", ")") else {
            continue;
        };
        if tokens.get(close + 1).map(|t| t.text.as_str()) != Some("{") {
            continue;
        }
        if matching(tokens, close + 1, "{", "}").is_some_and(|end| end - (close + 1) < FRAME_SPAN) {
            found.insert(token.text.clone());
        }
    }
    found
}

/// Card DETACH-RESULT-WRAPPER-COMPOSITION (rule 33): the build stream with
/// only `function`'s body rewritten, or `None` when nothing applies. Card
/// RULE33-PILOT: the rewrite is the package's [`StatementExpansion`] answer;
/// with rule 33 off nothing is read or rewritten.
pub(crate) fn expand(
    tokens: &[Token],
    function: &str,
    contracts: &ResourceContracts,
) -> Option<Vec<Token>> {
    if !enabled() {
        return None;
    }
    let package = &PACKAGE[0];
    let (start, _) = find_function_body(tokens, function)?;
    let end = matching(tokens, start, "{", "}")?;
    let body = words(&tokens[start..=end]);
    // Cheap prefilters first: a frame-sized body, or a caller statement.
    let own = end - start < FRAME_SPAN
        && (detach_shape(tokens, function).is_some() || release_shape(tokens, function).is_some());
    let callees = package.candidates(function, &body);
    if !own && callees.is_empty() {
        return None;
    }
    if tokens
        .iter()
        .any(|token| token.text.starts_with(package.reserved_prefix()))
    {
        return None;
    }
    let own = if own {
        Some(frame(tokens, function, contracts)?)
    } else {
        None
    };
    let frames = if own.is_some() {
        std::collections::BTreeMap::new()
    } else {
        small_definitions(tokens, &callees)
            .into_iter()
            .filter_map(|callee| frame(tokens, &callee, contracts).map(|frame| (callee, frame)))
            .collect()
    };
    if own.is_none() && frames.is_empty() {
        return None;
    }
    let query = BodyQuery {
        function,
        body: &body,
        own: own.as_ref(),
        callees: &frames,
    };
    let expanded = crate::arm_obligations::consult(
        &PACKAGE,
        |package| package.id(),
        |_| enabled(),
        |_, package| StatementExpansion::ask(package, &query),
    )?
    .lowering;
    // An anchor outside the body is no answer: the body stays verbatim.
    if !expanded.anchored_within(body.len()) {
        return None;
    }
    let mut out = tokens[..start].to_vec();
    out.extend(expanded.words.into_iter().map(|(text, anchor)| Token {
        text,
        line: tokens[start + anchor].line,
    }));
    out.extend_from_slice(&tokens[end + 1..]);
    Some(out)
}

/// The frame's lookup is reached from entry through branches and reserved
/// initializations only: no call or write precedes the selection.
fn pristine(cfg: &LegacyCfg, selection: usize) -> bool {
    let mut forward = BTreeSet::from([cfg.entry]);
    let mut work = vec![cfg.entry];
    while let Some(node) = work.pop() {
        if node == selection {
            continue;
        }
        for &next in &cfg.nodes[node].successors {
            if forward.insert(next) {
                work.push(next);
            }
        }
    }
    let mut backward = BTreeSet::from([selection]);
    let mut changed = true;
    while changed {
        changed = false;
        for (id, node) in cfg.nodes.iter().enumerate() {
            if !backward.contains(&id) && node.successors.iter().any(|s| backward.contains(s)) {
                backward.insert(id);
                changed = true;
            }
        }
    }
    forward
        .intersection(&backward)
        .filter(|id| **id != selection)
        .all(|id| match &cfg.nodes[*id].instruction {
            LegacyInstruction::NoOp => true,
            LegacyInstruction::Clear { destination } => destination.starts_with(RESERVED),
            _ => false,
        })
}

fn selection_node(cfg: &LegacyCfg, frame: &Frame) -> Option<usize> {
    let nodes = cfg
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| {
            matches!(&node.instruction, LegacyInstruction::ContractCall {
                function, destination: Some(destination), arguments, ..
            } if *function == frame.lookup
                && destination == ITEM_PLACE
                && arguments.first() == Some(&Some(frame.owner.clone())))
        })
        .map(|(id, _)| id)
        .collect::<Vec<_>>();
    match nodes.as_slice() {
        [one] => Some(*one),
        _ => None,
    }
}

/// The entry custody a frame's own row is analysed under.
fn entry(cfg: &LegacyCfg, parameters: &[Option<String>], frame: &Frame) -> Option<EntryCustody> {
    if parameters != [Some(frame.owner.clone()), Some(frame.index.clone())] {
        return None;
    }
    let selection = selection_node(cfg, frame)?;
    let base = super::first_free_site(cfg);
    let owner_site = base;
    let phantom = base.checked_add(parameters.len())?;
    let mut custody = EntryCustody {
        parameter_sites: vec![Some(owner_site), None],
        out_slots: vec![None, None],
        result_nullable: frame.disposition == Disposition::Returned,
        ..EntryCustody::default()
    };
    custody.state.resources.insert(owner_site, LIVE);
    custody.state.pointers.insert(
        frame.owner.clone(),
        AbstractPointer {
            targets: BTreeSet::from([owner_site]),
            ..AbstractPointer::default()
        },
    );
    custody.selection = Some(SelectionEntry {
        owner: frame.owner.clone(),
        owner_site,
        lookup: frame.lookup.clone(),
        phantom,
        pristine: pristine(cfg, selection),
    });
    Some(custody)
}

fn settled_live(state: &AbstractState, site: usize) -> bool {
    state.resources.get(&site) == Some(&LIVE)
        && !state.nullable_sites.contains(&site)
        && !state.unresolved_effect_sites.contains(&site)
        && !state.conditional_replacements.contains_key(&site)
        && !state
            .conditional_replacements
            .values()
            .any(|old| old.contains(&site))
}

/// Rule 33: at the frame's one lookup point, the selected child appears as
/// one nullable child uniquely contained by the owner, when every fact holds.
pub(super) fn materialize(node: &LegacyCfgNode, state: &mut AbstractState, entry: &SelectionEntry) {
    let LegacyInstruction::ContractCall {
        function,
        destination: Some(destination),
        arguments,
        ..
    } = &node.instruction
    else {
        return;
    };
    let facts = MaterializeFacts {
        selection_node: *function == entry.lookup
            && destination == ITEM_PLACE
            && arguments.first() == Some(&Some(entry.owner.clone())),
        parent_live_identity: state.pointers.get(&entry.owner).is_some_and(|pointer| {
            !pointer.unknown
                && !pointer.may_be_null
                && pointer.derivation.is_identity()
                && pointer.targets == BTreeSet::from([entry.owner_site])
        }) && settled_live(state, entry.owner_site),
        pristine: entry.pristine,
        fresh: !state.resources.contains_key(&entry.phantom)
            && !state.unresolved_effect_sites.contains(&entry.phantom)
            && !state.nullable_sites.contains(&entry.phantom)
            && !state
                .contained
                .values()
                .any(|children| children.contains(&entry.phantom)),
        borrowed_result: !state.unmodeled
            && !state.widened
            && state.pointers.get(destination).is_some_and(|pointer| {
                !pointer.unknown
                    && pointer.targets == BTreeSet::from([entry.owner_site])
                    && matches!(
                        pointer.derivation,
                        Derivation::Derived {
                            extent: DerivedExtent::Borrowed,
                            ..
                        }
                    )
            }),
    };
    if !decide(facts) {
        return;
    }
    state.resources.insert(entry.phantom, CONTAINED);
    state
        .contained
        .entry(entry.owner_site)
        .or_default()
        .insert(entry.phantom);
    state.nullable_sites.insert(entry.phantom);
    state.pointers.insert(
        destination.clone(),
        AbstractPointer {
            targets: BTreeSet::from([entry.phantom]),
            may_be_null: true,
            ..AbstractPointer::default()
        },
    );
    // Like a `Removed` formal's entry child, the selected child is a contained
    // identity, not a borrowed view: no borrow or lookup alias is recorded. The
    // exact frame reaches the detach directly from this node.
    state.borrows.remove(destination);
    state.lookup_aliases.remove(destination);
}

fn selected(disposition: Option<SiteDisposition>) -> Selected {
    match disposition {
        None | Some(SiteDisposition::Absent) => Selected::Absent,
        Some(SiteDisposition::Contained) => Selected::Contained,
        Some(SiteDisposition::Live) => Selected::Live,
        Some(SiteDisposition::Escaped) => Selected::Escaped,
        Some(SiteDisposition::Released) => Selected::Released,
        Some(SiteDisposition::Mixed) => Selected::Mixed,
    }
}

/// Analyse a frame's own row under its selection entry; `Some` only when the
/// run validates the frame's summary (rule 33).
pub(crate) fn row(
    cfg: LegacyCfg,
    parameters: &[Option<String>],
    frame: &Frame,
) -> Option<LegacyFunctionSummary> {
    let entry = entry(&cfg, parameters, frame)?;
    let selection = entry.selection.clone()?;
    if !selection.pristine {
        return None;
    }
    let (summary, observations) = super::analyze_cfg_with_entry(cfg, &entry);
    let clean = summary.status == super::AnalysisStatus::Safe && summary.diagnostics.is_empty();
    let exits = observations
        .iter()
        .map(|exit| ExitFacts {
            parent_live: exit.parameters.first() == Some(&Some(SiteDisposition::Live))
                && exit.parameters.iter().skip(1).all(Option::is_none),
            frame_clear: exit.caller_containment.is_empty()
                && exit.pending_replacements.is_empty()
                && exit.out_slots.iter().all(Option::is_none)
                && !exit.derived_base_touched,
            selected: selected(exit.selected),
            returned: match exit.returned {
                ReturnedValue::None | ReturnedValue::Null => Returned::Nothing,
                ReturnedValue::Site {
                    site,
                    disposition: SiteDisposition::Escaped,
                    kind: ResourceKind::Heap,
                    ..
                } if site == selection.phantom => Returned::Selected,
                _ => Returned::Other,
            },
        })
        .collect::<Vec<_>>();
    let run = ValidationRun {
        disposition: frame.disposition,
        clean,
        exits,
    };
    crate::arm_obligations::consult(
        &PACKAGE,
        |package| package.id(),
        |_| enabled(),
        |_, package| EntryRow::ask(package, &run),
    )
    .map(|_| summary)
}

/// Whether `function`'s frame validates in this unit (its summary exists).
pub(crate) fn validates_frame(
    tokens: &[Token],
    function: &str,
    contracts: &ResourceContracts,
    frame: &Frame,
) -> bool {
    let Some(parameters) = function_parameters(tokens, function) else {
        return false;
    };
    let Ok(cfg) = super::build_function_cfg(tokens, function, contracts) else {
        return false;
    };
    row(cfg, &parameters, frame).is_some()
}
