//! Card **W47-SUMMARY-VALIDATION-PROTECTED-FORMALS** (gateway rule 4b): a W4
//! summary is validated on the typed-root premises its function's reported
//! row reads.
//!
//! Every W4 route builds the CFG it infers on in the environment of the
//! moment, which does not yet hold the candidate: the candidate is what the
//! route is deriving. The reported run ([`crate::legacy_c`]'s
//! `analyze_defined_function`) builds the same body with the candidate in
//! force. The two CFGs differ exactly where the body's construction reads the
//! function's own contract: `protected_resource_formals` protects every formal
//! the contract does not declare `Borrowed` (and the descriptor reader names
//! its integer-kinded ones). W48 bottom-up batch 2 also makes this same
//! non-Borrowed set immobile for formal self-steps. The protected set seeds every typed-root premise
//! — the lexical never-bound closure (rule 10o), `may_bind` and the declared
//! locals (`FunctionTypes`), and the AST closure rule 10l joins — so a copy
//! `const T *a = _a;` of a formal the candidate seeds as an entry resource was
//! a never-bound root in the validation run and a bound one in the reported
//! run. A caller could certify on a summary whose own row refused
//! (`rd_kafka_topic_partition_cmp` / `…_cmp_opaque`).
//!
//! [`validates_under_reported_premises`] closes that gap without weakening
//! either run: a candidate the environment does not already govern is
//! re-checked, by the unchanged [`observations_match_contract`] gate, on the
//! CFG built with the candidate as the function's own contract. Only that
//! reading changes; a call to the function inside its own body still resolves
//! through the environment, so no recursive assumption enters (rule 4). A
//! candidate whose every formal is `Borrowed` changes nothing the CFG reads
//! and is accepted as validated; a function the environment already governs
//! (a declaration) was validated under that declaration's protection, which
//! is what its reported run reads.
//!
//! When the typed-path parity stream is enabled, every decision is recorded
//! (`position: "summary-premises"`), with the unit's call graph once per
//! derivation (`position: "summary-premises-graph"`), so the arbiter can name
//! every summary the premise withdraws and every caller that rested on it.

use std::collections::BTreeMap;

use qcore_schemas::contract::{ResourceContracts, ResourceFunctionContract, ResourceParameterMode};

use super::observations_match_contract;
use crate::legacy_c::recursion::{self, ContextFunction};
use crate::legacy_c::{
    EntryCustody, LegacyCfg, Token, analyze_cfg_with_entry, build_function_cfg_under_own_contract,
};
use crate::typed_access_path::without_parity_rows;

/// Which W4 route proposed the candidate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Route {
    LiveEntry,
    RemovedWrapper,
    DerivedResult,
}

impl Route {
    fn label(self) -> &'static str {
        match self {
            Self::LiveEntry => "live-entry",
            Self::RemovedWrapper => "removed-wrapper",
            Self::DerivedResult => "derived-result",
        }
    }
}

/// The formals `contract` protects when it is the function's own contract in
/// force: every named formal it does not declare `Borrowed`.
pub(super) fn own_protected_formals(
    parameters: &[Option<String>],
    contract: &ResourceFunctionContract,
) -> Vec<String> {
    parameters
        .iter()
        .zip(&contract.parameters)
        .filter(|(_, mode)| !matches!(mode, ResourceParameterMode::Borrowed))
        .filter_map(|(name, _)| name.clone())
        .collect()
}

/// Check a validated candidate once more, on the CFG the reported run builds
/// with the candidate as the function's own contract. `inferred_on` is the
/// CFG the route validated on; the re-check is skipped only when the rebuilt
/// CFG is identical to it, since the analysis is a function of the CFG.
pub(super) fn validates_under_reported_premises(
    tokens: &[Token],
    function: &str,
    environment: &ResourceContracts,
    parameters: &[Option<String>],
    candidate: &ResourceFunctionContract,
    route: Route,
    inferred_on: &LegacyCfg,
) -> bool {
    // A declaration governs the reported run and was already in force here.
    if environment.contract(function).is_some() {
        return true;
    }
    let own = own_protected_formals(parameters, candidate);
    if own.is_empty() {
        return true;
    }
    // The rebuild's readers would repeat the first build's parity rows.
    let (cfg_changed, validates) = without_parity_rows(|| {
        match build_function_cfg_under_own_contract(
            tokens,
            function,
            environment,
            candidate,
            route == Route::DerivedResult,
        ) {
            Err(_) => (true, false),
            Ok(cfg) if &cfg == inferred_on => (false, true),
            Ok(cfg) => (true, validates_on(cfg, parameters, candidate)),
        }
    });
    record(function, route, candidate, &own, cfg_changed, validates);
    validates
}

/// Card OUT-PARAMETER-BODY-SUMMARY (rule 11r(6)): [`validates_under_reported_premises`]
/// for an unwritten class, whose `analysis` contract (every slot of
/// `unwritten` `Out`) is what the run reads, checked by the class gate
/// [`super::observations_match_class`]. Recorded as route `unwritten-class`.
pub(super) fn class_validates_under_reported_premises(
    tokens: &[Token],
    function: &str,
    environment: &ResourceContracts,
    parameters: &[Option<String>],
    analysis: &ResourceFunctionContract,
    unwritten: &std::collections::BTreeSet<usize>,
    inferred_on: &LegacyCfg,
) -> bool {
    if environment.contract(function).is_some() {
        return true;
    }
    let own = own_protected_formals(parameters, analysis);
    if own.is_empty() {
        return true;
    }
    let (cfg_changed, validates) = without_parity_rows(|| {
        match build_function_cfg_under_own_contract(tokens, function, environment, analysis, false)
        {
            Err(_) => (true, false),
            Ok(cfg) if &cfg == inferred_on => (false, true),
            Ok(cfg) => (
                true,
                EntryCustody::build(&cfg, parameters, analysis).is_some_and(|entry| {
                    let (summary, observations) = analyze_cfg_with_entry(cfg, &entry);
                    super::observations_match_class(
                        analysis,
                        unwritten,
                        &entry,
                        &summary,
                        &observations,
                    )
                }),
            ),
        }
    });
    record_with_label(
        function,
        "unwritten-class",
        analysis,
        &own,
        cfg_changed,
        validates,
    );
    validates
}

/// The unchanged W4 gate on one CFG.
pub(super) fn validates_on(
    cfg: LegacyCfg,
    parameters: &[Option<String>],
    candidate: &ResourceFunctionContract,
) -> bool {
    let Some(entry) = EntryCustody::build(&cfg, parameters, candidate) else {
        return false;
    };
    let (summary, observations) = analyze_cfg_with_entry(cfg, &entry);
    observations_match_contract(candidate, &entry, &summary, &observations)
}

/// The contextual (ranked recursive) route: both context CFGs rebuilt with
/// the candidate as the function's own contract, combined, and checked by the
/// same gate. The reported run of a contextual function reads the combined
/// CFG the route stores, so the one returned here is the one it stores: both
/// runs then read the protected premises. `None` refuses the candidate.
///
/// `zero_environment` and `nonzero_environment` are the environments the two
/// contexts were built in (the rank-zero contracts are in force only for the
/// nonzero context), and `combined` the CFG the route validated on.
pub(super) fn contextual_under_reported_premises(
    tokens: &[Token],
    function: &str,
    zero_environment: &ResourceContracts,
    nonzero_environment: &ResourceContracts,
    descriptor: &ContextFunction,
    candidate: &ResourceFunctionContract,
    combined: LegacyCfg,
) -> Option<LegacyCfg> {
    let own = own_protected_formals(&descriptor.parameters, candidate);
    if own.is_empty() || nonzero_environment.contract(function).is_some() {
        return Some(combined);
    }
    let decision = without_parity_rows(|| {
        let rebuilt = recursion::contextual_cfg_under(
            tokens,
            function,
            zero_environment,
            descriptor,
            true,
            Some(candidate),
        )
        .zip(recursion::contextual_cfg_under(
            tokens,
            function,
            nonzero_environment,
            descriptor,
            false,
            Some(candidate),
        ))
        .map(|(zero, nonzero)| recursion::combine(zero, nonzero));
        match rebuilt {
            None => None,
            Some(cfg) if cfg == combined => Some((false, cfg)),
            Some(cfg) => {
                validates_on(cfg.clone(), &descriptor.parameters, candidate).then_some((true, cfg))
            }
        }
    });
    let cfg_changed = decision.as_ref().is_none_or(|(changed, _)| *changed);
    record_with_label(
        function,
        "contextual",
        candidate,
        &own,
        cfg_changed,
        decision.is_some(),
    );
    decision.map(|(_, cfg)| cfg)
}

fn record(
    function: &str,
    route: Route,
    candidate: &ResourceFunctionContract,
    own: &[String],
    cfg_changed: bool,
    validates: bool,
) {
    record_with_label(
        function,
        route.label(),
        candidate,
        own,
        cfg_changed,
        validates,
    );
}

fn record_with_label(
    function: &str,
    route: &str,
    candidate: &ResourceFunctionContract,
    own: &[String],
    cfg_changed: bool,
    validates: bool,
) {
    if !crate::typed_access_path::parity_enabled() {
        return;
    }
    crate::typed_access_path::record_row(serde_json::json!({
        "position": "summary-premises",
        "function": function,
        "route": route,
        "candidate": format!("{candidate:?}"),
        "own_protected": own,
        "cfg_changed": cfg_changed,
        "validates_under_reported_premises": validates,
    }));
}

/// The unit's direct-call graph over the derivation's candidates and every
/// summary the derivation validated (whether a declaration overrode it),
/// recorded once per derivation so a withdrawn summary's callers, and every
/// published summary's own row, can be named.
pub(super) fn record_derivation(
    plan: &crate::audit_ir::AuditPlan,
    inferred: &BTreeMap<String, ResourceFunctionContract>,
    declared: &ResourceContracts,
) {
    if !crate::typed_access_path::parity_enabled() {
        return;
    }
    let summaries = inferred
        .iter()
        .map(|(name, contract)| {
            serde_json::json!({
                "function": name,
                "contract": format!("{contract:?}"),
                "declared": declared.contract(name).is_some(),
                "all_borrowed": contract
                    .parameters
                    .iter()
                    .all(|mode| matches!(mode, ResourceParameterMode::Borrowed)),
            })
        })
        .collect::<Vec<_>>();
    crate::typed_access_path::record_row(serde_json::json!({
        "position": "summary-premises-graph",
        "graph": plan.named_graph(),
        "summaries": summaries,
    }));
}
