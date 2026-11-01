//! Card **OUTER-CONTRACT** (gateway rule 31, contract specification C04.20):
//! the runtime projection of an audit route.
//!
//! USER DECISIONS 2026-10-17, recorded verbatim in the card's report: D1 "B:
//! loader envelope"; D2 "Yes, as proposed"; D3 "Both from one run": publish
//! static and runtime verdicts side by side from one dependency record, rule
//! 13h amended for this reader; D4 "Later" (the library-mode host).
//!
//! # Selection
//!
//! `QCORE_OUTER_PROJECTION=PATH` selects the projection. The pinned,
//! configured and no-build routes then load every outer view on the load path
//! their resource manifest names ([`select_for_route`]), compare each loader
//! declaration with its crate's [`OUTER_LOCK_FILE`] over the audited source
//! (C06.2: in force only where the text is the pinned one and the loader's
//! sites equal a reviewed identity), read what each `#loaded` binder reaches
//! from Clang's AST of the loader's own unit ([`compute_reach`]), and, after
//! the static report and the null rows are written, write the projection to
//! `PATH` ([`project`]). Unset, nothing is read, installed or written, and
//! every artifact is byte-identical to a run before this card.
//!
//! # The projection
//!
//! The static verdicts are the report's, unchanged. A row's **closed-world
//! facts** are read from its dependency record (rule 13f): the stable and
//! declared non-null members of a null row (`stable:TAG.FIELD`,
//! `member:TAG.FIELD`, rules 14x and 14t), the run-set and stated members of a
//! custody row (`run_set_members`, `stated_members`, rules 12n and 14e), and
//! every closed-world premise either names ([`NOT_CLOSED_WORLD`] lists the
//! premises that are not). Under a scenario (a set of loaded outer
//! contracts), a fact reachable from a contract's `#loaded` binders
//! contributes `Pending` (`outer-contract`, naming `NAME@DIGEST`) when the
//! contract is in force and an envelope line covers it, and `Unknown` (named
//! `outer-uncovered`) otherwise; a premise is reachable from every contract.
//! The runtime verdict is the join (`Safe < Pending < Unknown < Unsafe`) of
//! the static verdict and every contribution: nothing is promoted, and a row
//! with no closed-world fact is unchanged. Lean: `QCore.OuterContract`.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use qcore_c_gateway::outer_record_graph::{Reach, RecordGraph};
use qcore_contract::load_path_manifest::{LoadPathManifest, load_contract_load_path_manifest};
use qcore_contract::outer_contract::{
    EnvelopeLine, MemberEnvelope, OuterContracts, load_outer_contract_sources,
};
use qcore_schemas::sha256_hex;
use qcore_schemas::verdict::{PendingReason, Verdict};
use serde_json::{Value, json};

use crate::c_identity::{SiteKind, SourceSite, SourceTree};
use crate::compile_db::CompileEntry;
use crate::contract_revision::{ContractLock, PinStanding, pin_standing};
use crate::contract_tip::TipResolver;

/// The variable selecting the projection, naming the file it is written to.
pub const PROJECTION: &str = "QCORE_OUTER_PROJECTION";

/// The outer lock's file name, beside a provider crate's `Cargo.toml`.
pub const OUTER_LOCK_FILE: &str = "outer-contracts.lock";

/// The one selection rule.
pub const SELECTION: &str = "all-outer-views";

/// The scenario every loaded contract forms together.
pub const ALL_SCENARIO: &str = "all";

/// Premises that state a build fact, a reading rule or a tracking limit, not
/// the absence of other code; every other named premise is a closed-world
/// fact (fail-closed: a new premise is one until it is listed here).
pub const NOT_CLOSED_WORLD: [&str; 9] = [
    "asserted_allocation",
    "nested_call_admission",
    "literal_argument",
    "P-CELL-STATE-NOT-FORWARDED",
    "untracked_handback",
    "ingestion_active",
    "synchronous_output_reachability",
    "P-RESULT-HOLDER-LAST-DROP",
    "P-RETAINED-PREDEFINED",
];

/// What the projection means, carried in it.
pub const MEANING: &str = "the runtime projection (gateway rule 31, contract specification C04.20; user decisions 2026-10-17 D1 \"B: loader envelope\", D3 \"Both from one run\"): the static verdicts are the report's, unchanged; a row's closed-world facts are read from its dependency record (null rows: stable:TAG.FIELD and member:TAG.FIELD; custody rows: run_set_members and stated_members; both: every named premise not listed in not_closed_world); under a scenario (a set of loaded outer contracts) a fact reachable from a contract's #loaded binders (Clang's record graph of the loader's unit: typed pointers, embedded records, container_of, a void * reaching every record; a premise is reachable from every contract) contributes Pending (outer-contract, NAME@DIGEST) where the contract is in force and an envelope line covers it (|-> #kept: a stable, declared or stated member or a run set; |-> #registered: a run set; #premise NAME: the premise), and Unknown (outer-uncovered) otherwise; the runtime verdict is the join Safe < Pending < Unknown < Unsafe of the static verdict and every contribution, so nothing is promoted and a row with no closed-world fact is unchanged";

/// One loader's evidence for its reach.
#[derive(Clone, Debug)]
struct LoaderReach {
    reach: Reach,
    evidence: Vec<Value>,
}

struct Installed {
    contracts: OuterContracts,
    record: Value,
    /// Per loader, its definition sites in the audited source.
    sites: BTreeMap<String, Vec<SourceSite>>,
    source_root: PathBuf,
    /// Per loader, what its binders reach (absent until computed).
    reach: BTreeMap<String, LoaderReach>,
}

static INSTALLED: RwLock<Option<Installed>> = RwLock::new(None);

/// `true` when the run selected the projection.
#[must_use]
pub fn selected() -> bool {
    std::env::var_os(PROJECTION).is_some()
}

/// Every outer view on a load path, loaded together, each module keyed
/// `crate:module`; and one record per module (crate, module, SHA-256).
///
/// # Errors
///
/// A view the outer loader refuses refuses the whole selection.
pub fn load_views(manifest: &LoadPathManifest) -> Result<(OuterContracts, Vec<Value>), String> {
    let mut sources = BTreeMap::new();
    let mut modules = Vec::new();
    for (crate_name, view) in &manifest.outer_views {
        let krate = manifest.load_path.crate_named(crate_name).ok_or_else(|| {
            format!("outer view of `{crate_name}`, which is not on the load path")
        })?;
        for module in view {
            let text = krate.modules.get(module).ok_or_else(|| {
                format!("outer view of `{crate_name}` names `{module}`, which it does not own")
            })?;
            modules.push(json!({
                "crate": crate_name,
                "module": module,
                "sha256": sha256_hex(text.as_bytes()),
            }));
            sources.insert(format!("{crate_name}:{module}"), text.clone());
        }
    }
    if sources.is_empty() {
        return Ok((OuterContracts::default(), modules));
    }
    let contracts = load_outer_contract_sources(&sources)
        .map_err(|error| format!("the outer views do not load together: {error}"))?;
    Ok((contracts, modules))
}

fn lock_path(manifest: &LoadPathManifest, module: &str) -> Option<PathBuf> {
    let (crate_name, _) = module.split_once(':')?;
    let root = manifest.contract_roots.get(crate_name)?.parent()?;
    Some(root.join(OUTER_LOCK_FILE)).filter(|lock| lock.is_file())
}

/// Compare every loader declaration with its crate's outer lock over the
/// source at `source_root`, and hold every one not in force. Returns the pin
/// section and each loader's sites.
///
/// # Errors
///
/// A malformed lock, an unreadable source tree or a malformed tip variable.
pub fn apply_outer_pins(
    contracts: &mut OuterContracts,
    manifest: &LoadPathManifest,
    source_root: &Path,
) -> Result<(Value, BTreeMap<String, Vec<SourceSite>>), String> {
    let mut locks: BTreeMap<PathBuf, ContractLock> = BTreeMap::new();
    let mut lock_of = BTreeMap::new();
    for (name, loader) in &contracts.loaders {
        let lock = lock_path(manifest, &loader.module);
        if let Some(path) = lock.as_ref().filter(|path| !locks.contains_key(*path)) {
            locks.insert(path.clone(), ContractLock::read(path)?);
        }
        lock_of.insert(name.clone(), lock);
    }
    if contracts.loaders.is_empty() {
        return Ok((
            json!({"source": Value::Null, "in_force": 0, "held": 0, "outcomes": []}),
            BTreeMap::new(),
        ));
    }
    let wanted = contracts.loaders.keys().cloned().collect::<BTreeSet<_>>();
    let scan = SourceTree::read_sites(source_root, &wanted)?;
    let mut tips = TipResolver::from_environment()?;
    let mut outcomes = Vec::new();
    let mut holds = Vec::new();
    for (name, loader) in &contracts.loaders {
        let lock_file = lock_of[name].as_ref();
        let lock = lock_file.and_then(|path| locks.get(path));
        let sites = scan.sites.get(name).cloned().unwrap_or_default();
        let standing = pin_standing(
            lock,
            name,
            &loader.text_sha256,
            &sites,
            &loader.module,
            &mut tips,
        );
        let (reason, detail) = match &standing {
            PinStanding::InForce { .. } | PinStanding::InForceAtTip { .. } => (None, String::new()),
            PinStanding::Mismatch { reason, detail, .. } => {
                (Some(reason.as_str().to_owned()), detail.clone())
            }
        };
        if let Some(reason) = &reason {
            holds.push((name.clone(), reason.clone(), detail.clone()));
        }
        outcomes.push(json!({
            "loader": name,
            "module": loader.module,
            "lock": lock_file.map(|path| path.display().to_string()),
            "contract_sha256": loader.text_sha256,
            "standing": if reason.is_some() { "not-in-force" } else { "in-force" },
            "reason": reason,
            "detail": detail,
            "sites": sites,
        }));
    }
    let held = holds.len();
    for (name, reason, detail) in holds {
        contracts.hold_for_pin_mismatch(&name, &reason, &detail)?;
    }
    Ok((
        json!({
            "source": {"kind": scan.kind, "revision": scan.revision},
            "in_force": outcomes.len() - held,
            "held": held,
            "outcomes": outcomes,
        }),
        scan.sites,
    ))
}

fn install(installed: Installed) -> Result<(), String> {
    let mut slot = INSTALLED
        .write()
        .map_err(|_| "the outer contract environment lock is poisoned".to_owned())?;
    *slot = Some(installed);
    Ok(())
}

/// Select the outer environment of an audit route: nothing unless
/// [`PROJECTION`] is set. `manifest_path` and `manifest_text` are the route's
/// resource contract manifest, whose `load_path` names the providers;
/// `source_root` is the audited checkout.
///
/// # Errors
///
/// An unreadable load path, an outer view that does not load, a malformed
/// lock or an unreadable source refuses the run.
pub fn select_for_route(
    route: &str,
    manifest_path: &Path,
    manifest_text: &str,
    source_root: &Path,
) -> Result<(), String> {
    if !selected() {
        return Ok(());
    }
    let load_path = crate::bounds_selection::manifest_load_path(manifest_path, manifest_text)?;
    let (contracts, modules, pins, sites, load_path_record) = match load_path {
        None => (
            OuterContracts::default(),
            Vec::new(),
            json!({"in_force": 0, "held": 0, "outcomes": []}),
            BTreeMap::new(),
            Value::Null,
        ),
        Some(load_path) => {
            let manifest =
                load_contract_load_path_manifest(&load_path).map_err(|error| error.to_string())?;
            let (mut contracts, modules) = load_views(&manifest)?;
            let (mut pins, sites) = apply_outer_pins(&mut contracts, &manifest, source_root)?;
            pins["list_candidates"] =
                crate::outer_candidates::apply(&mut contracts, &manifest, source_root)?;
            if let Some(outcomes) = pins["outcomes"].as_array_mut() {
                for outcome in outcomes {
                    if let Some(hold) = outcome["loader"]
                        .as_str()
                        .and_then(|name| contracts.loaders.get(name))
                        .and_then(|loader| loader.pin_hold.as_ref())
                    {
                        outcome["standing"] = json!("not-in-force");
                        outcome["reason"] = json!(hold.reason);
                        outcome["detail"] = json!(hold.detail);
                    }
                }
            }
            let held = contracts
                .loaders
                .values()
                .filter(|loader| loader.pin_hold.is_some())
                .count();
            pins["held"] = json!(held);
            pins["in_force"] = json!(contracts.loaders.len() - held);
            let record = json!({
                "path": load_path,
                "sha256": sha256_hex(&fs::read(&load_path).map_err(|e| e.to_string())?),
            });
            (contracts, modules, pins, sites, record)
        }
    };
    println!(
        "outer contracts ({route}): {} contract(s) on {} loader(s) from {} module(s); {} pinned in force, {} held (contract-pin-mismatch)",
        contracts.contracts.len(),
        contracts.loaders.len(),
        modules.len(),
        pins["in_force"],
        pins["held"]
    );
    let record = json!({
        "schema_version": 1,
        "contract_version": qcore_contract::outer_contract::OUTER_CONTRACT_VERSION,
        "route": route,
        "selection": SELECTION,
        "load_path": load_path_record,
        "modules": modules,
        "environment_sha256": contracts.environment_digest(),
        "pins": pins,
        "trust": "declared assumptions about code loaded at runtime (C04.20: an outer contract vouches for what the audit cannot read), in force only where pinned to the loader's body",
    });
    install(Installed {
        contracts,
        record,
        sites,
        source_root: source_root.to_path_buf(),
        reach: BTreeMap::new(),
    })
}

/// Read one unit's record graph, as the member pre-pass reads a unit.
fn read_graph(entry: &CompileEntry) -> Result<RecordGraph, String> {
    let source = fs::read_to_string(&entry.file).map_err(|error| error.to_string())?;
    if let Ok(unit) = crate::member_prepass::c_unit(entry, &source)
        && let Ok(preprocessed) = qcore_c_gateway::legacy_c::preprocess_unrestricted_c11(&unit)
    {
        return qcore_c_gateway::outer_record_graph::read_unit(&preprocessed, &unit);
    }
    let preprocessed = crate::member_prepass::preprocess_own(entry, &["-E", "-P"])?;
    qcore_c_gateway::outer_record_graph::read_unit(
        &preprocessed,
        &qcore_c_gateway::legacy_c::C11TranslationUnit {
            source: &preprocessed,
            dialect: crate::member_prepass::dialect_of(entry),
            ..Default::default()
        },
    )
}

fn reach_value(reach: &Reach) -> Value {
    match reach {
        Reach::All { witness } => json!({"kind": "all-records", "witness": witness}),
        Reach::Records(tags) => json!({"kind": "records", "records": tags}),
    }
}

fn join_reach(left: Reach, right: Reach) -> Reach {
    match (left, right) {
        (all @ Reach::All { .. }, _) | (_, all @ Reach::All { .. }) => all,
        (Reach::Records(mut a), Reach::Records(b)) => {
            a.extend(b);
            Reach::Records(a)
        }
    }
}

/// Read what each loader's `#loaded` binders reach, from Clang's AST of every
/// configuration of the loader's defining unit in `entries`. A held loader,
/// a loader with no definition site, no compile entry or an unreadable unit
/// reaches every record. Nothing unless the projection is selected.
///
/// # Errors
///
/// A poisoned environment lock.
pub fn compute_reach(entries: &[CompileEntry]) -> Result<(), String> {
    if !selected() {
        return Ok(());
    }
    let mut slot = INSTALLED
        .write()
        .map_err(|_| "the outer contract environment lock is poisoned".to_owned())?;
    let Some(installed) = slot.as_mut() else {
        return Ok(());
    };
    let root = installed
        .source_root
        .canonicalize()
        .unwrap_or_else(|_| installed.source_root.clone());
    let mut computed = BTreeMap::new();
    for (name, loader) in &installed.contracts.loaders {
        let all = |witness: String| LoaderReach {
            reach: Reach::All {
                witness: witness.clone(),
            },
            evidence: vec![json!({"witness": witness})],
        };
        if loader.pin_hold.is_some() {
            computed.insert(name.clone(), all("the loader's pin is not in force".into()));
            continue;
        }
        let paths = installed
            .sites
            .get(name)
            .into_iter()
            .flatten()
            .filter(|site| site.kind == SiteKind::Definition)
            .map(|site| site.path.clone())
            .collect::<BTreeSet<_>>();
        if paths.is_empty() {
            computed.insert(
                name.clone(),
                all("no definition site in the audited source".into()),
            );
            continue;
        }
        // Card AUDIT-CANONICAL-PATHS: the loader's unit is found by located
        // path, and two spellings of it are refused (`input-path-alias`).
        let targets = paths
            .iter()
            .map(|path| crate::input_paths::locate(&root.join(path)))
            .collect::<Vec<_>>();
        let matching = entries
            .iter()
            .filter(|entry| targets.contains(&crate::input_paths::locate(&entry.file)))
            .collect::<Vec<_>>();
        crate::input_paths::refuse_aliases(matching.iter().copied())?;
        if matching.is_empty() {
            computed.insert(
                name.clone(),
                all(format!(
                    "no compile entry for {}",
                    paths.iter().cloned().collect::<Vec<_>>().join(", ")
                )),
            );
            continue;
        }
        let mut reach = Reach::Records(BTreeSet::new());
        let mut evidence = Vec::new();
        for entry in matching {
            match read_graph(entry) {
                Ok(graph) => {
                    for condition in &loader.runtime {
                        let one = graph.reach_of_parameter(name, condition.index);
                        evidence.push(json!({
                            "unit": entry.file,
                            "binder": condition.binder,
                            "parameter_type": graph
                                .functions
                                .get(name)
                                .and_then(|params| params.get(condition.index)),
                            "records_read": graph.records.len(),
                            "reach": reach_value(&one),
                        }));
                        reach = join_reach(reach, one);
                    }
                }
                Err(error) => {
                    let witness = format!("{} could not be read: {error}", entry.file.display());
                    evidence.push(json!({"unit": entry.file, "witness": witness}));
                    reach = join_reach(reach, Reach::All { witness });
                }
            }
        }
        computed.insert(name.clone(), LoaderReach { reach, evidence });
    }
    for (name, reach) in &computed {
        println!(
            "outer contracts: `{name}` reaches {}",
            match &reach.reach {
                Reach::All { witness } => format!("every record ({witness})"),
                Reach::Records(tags) => format!("{} record(s)", tags.len()),
            }
        );
    }
    installed.reach = computed;
    Ok(())
}

/// One closed-world fact a row rests on.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Fact {
    /// A stable (`stable:`) or declared (`member:`) non-null member, or a
    /// stated result-state member: covered by `|-> #kept`.
    Member { kind: &'static str, key: String },
    /// A run-set member: covered by `|-> #kept` or `|-> #registered`.
    RunSet { key: String },
    /// A closed-world premise: covered by `#premise NAME`.
    Premise { name: String },
}

impl Fact {
    fn spelling(&self) -> String {
        match self {
            Self::Member { kind, key } => format!("{kind}:{key}"),
            Self::RunSet { key } => format!("run-set:{key}"),
            Self::Premise { name } => format!("premise:{name}"),
        }
    }

    fn tag(&self) -> Option<&str> {
        match self {
            Self::Member { key, .. } | Self::RunSet { key } => {
                Some(key.split_once('.').map_or(key.as_str(), |(tag, _)| tag))
            }
            Self::Premise { .. } => None,
        }
    }

    fn covered_by(&self, envelope: &[EnvelopeLine]) -> bool {
        envelope.iter().any(|line| match (self, line) {
            (
                Self::Member { key, .. },
                EnvelopeLine::Member {
                    tag,
                    path,
                    envelope: MemberEnvelope::Kept,
                },
            ) => *key == format!("{tag}.{path}"),
            (Self::RunSet { key }, EnvelopeLine::Member { tag, path, .. }) => {
                *key == format!("{tag}.{path}")
            }
            (Self::Premise { name }, EnvelopeLine::Premise { name: line }) => name == line,
            _ => false,
        })
    }
}

fn closed_world_premise(name: &str) -> bool {
    !NOT_CLOSED_WORLD.contains(&name)
}

fn strings(value: &Value) -> impl Iterator<Item = &str> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
}

/// The closed-world facts of a null row (its `null` object).
#[must_use]
pub fn null_facts(null: &Value) -> BTreeSet<Fact> {
    let mut facts = BTreeSet::new();
    for dependency in strings(&null["contract_dependencies"]) {
        if let Some(key) = dependency.strip_prefix("stable:") {
            facts.insert(Fact::Member {
                kind: "stable",
                key: key.to_owned(),
            });
        } else if let Some(key) = dependency.strip_prefix("member:") {
            facts.insert(Fact::Member {
                kind: "member",
                key: key.to_owned(),
            });
        }
    }
    for premise in null["premises"].as_array().into_iter().flatten() {
        if let Some(name) = premise
            .get("name")
            .and_then(Value::as_str)
            .or(premise.as_str())
            && closed_world_premise(name)
        {
            facts.insert(Fact::Premise {
                name: name.to_owned(),
            });
        }
    }
    facts
}

/// The closed-world facts of a custody row's dependency field, with the
/// premises each declaration carries (`premises`: declaration to records).
#[must_use]
pub fn custody_facts(dependencies: &Value, premises: &Value) -> BTreeSet<Fact> {
    let mut facts = BTreeSet::new();
    for key in strings(&dependencies[crate::contract_dependencies::RUN_SET_MEMBERS_KEY]) {
        facts.insert(Fact::RunSet {
            key: key.to_owned(),
        });
    }
    for key in strings(&dependencies[crate::contract_dependencies::STATED_MEMBERS_KEY]) {
        facts.insert(Fact::Member {
            kind: "stated",
            key: key.to_owned(),
        });
    }
    for key in strings(&dependencies[crate::contract_dependencies::LIFETIME_PREREQUISITES_KEY]) {
        facts.insert(Fact::Premise {
            name: key.to_owned(),
        });
    }
    for key in strings(&dependencies[crate::contract_dependencies::LIFETIME_MEMBERS_KEY]) {
        facts.insert(Fact::Member {
            kind: "lifetime",
            key: key.to_owned(),
        });
    }
    for list in ["declarations", "held", "withdrawn"] {
        for declaration in strings(&dependencies[list]) {
            for premise in premises[declaration].as_array().into_iter().flatten() {
                if let Some(name) = premise["name"].as_str()
                    && closed_world_premise(name)
                {
                    facts.insert(Fact::Premise {
                        name: name.to_owned(),
                    });
                }
            }
        }
    }
    facts
}

/// One contract as a scenario reads it.
#[derive(Clone, Debug)]
pub struct ScenarioContract {
    pub name: String,
    pub identity: String,
    pub in_force: bool,
    pub envelope: Vec<EnvelopeLine>,
    pub reach: Reach,
}

/// One contribution of a fact under a contract.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Contribution {
    pub verdict: Verdict,
    pub reason: String,
}

/// What a fact contributes under one contract: nothing when the contract
/// cannot reach it, `Pending` when the contract is in force and covers it,
/// `Unknown` otherwise.
#[must_use]
pub fn contribution(fact: &Fact, contract: &ScenarioContract) -> Option<Contribution> {
    let reachable = fact.tag().is_none_or(|tag| contract.reach.contains(tag));
    if !reachable {
        return None;
    }
    let spelling = fact.spelling();
    Some(
        if contract.in_force && fact.covered_by(&contract.envelope) {
            Contribution {
                verdict: Verdict::Pending,
                reason: format!(
                    "rests on outer contract `{}` through {spelling} (runtime projection, rule 31; {})",
                    contract.identity,
                    PendingReason::OuterContract.category().as_str()
                ),
            }
        } else {
            Contribution {
                verdict: Verdict::Unknown,
                reason: format!(
                    "outer-uncovered: {spelling} is reachable from code `{}` loads and {} (runtime projection, rule 31)",
                    contract.name,
                    if contract.in_force {
                        "no envelope line covers it"
                    } else {
                        "the contract is not in force"
                    }
                ),
            }
        },
    )
}

/// The runtime verdict of a row under a scenario, with its reasons: the
/// join of the static verdict and every contribution.
#[must_use]
pub fn runtime_verdict(
    static_verdict: Verdict,
    facts: &BTreeSet<Fact>,
    scenario: &[ScenarioContract],
) -> (Verdict, Vec<String>) {
    let mut verdict = static_verdict;
    let mut reasons = Vec::new();
    for fact in facts {
        for contract in scenario {
            if let Some(contribution) = contribution(fact, contract) {
                verdict = verdict.join(contribution.verdict);
                reasons.push(contribution.reason);
            }
        }
    }
    (verdict, reasons)
}

fn parse_verdict(value: &Value) -> Option<Verdict> {
    match value.as_str()? {
        "Safe" => Some(Verdict::Safe),
        "Pending" => Some(Verdict::Pending),
        "Unknown" => Some(Verdict::Unknown),
        "Unsafe" => Some(Verdict::Unsafe),
        _ => None,
    }
}

fn tally<'a>(verdicts: impl Iterator<Item = &'a str>) -> Value {
    let mut counts = BTreeMap::from([("Safe", 0), ("Pending", 0), ("Unknown", 0), ("Unsafe", 0)]);
    let mut other = 0usize;
    for verdict in verdicts {
        match counts.get_mut(verdict) {
            Some(count) => *count += 1,
            None => other += 1,
        }
    }
    let mut value = json!(counts);
    if other > 0 {
        value["other"] = json!(other);
    }
    value
}

type RowKey = (String, String, String, usize);

/// A null row as the projection reads it: its key, null verdict, facts, and
/// its custody and combined verdicts.
type NullRow = (
    RowKey,
    Verdict,
    BTreeSet<Fact>,
    Option<Verdict>,
    Option<Verdict>,
);

/// Project the static rows: `units` the report's unit rows, `null_rows` the
/// folded null rows when the null property was selected, `premises` the
/// declarations' premises ([`crate::contract_dependencies::premises`]).
/// `None` when the projection is not selected.
#[must_use]
pub fn project(units: &[Value], null_rows: Option<&[Value]>, premises: &Value) -> Option<Value> {
    if !selected() {
        return None;
    }
    let slot = INSTALLED.read().ok()?;
    let installed = slot.as_ref()?;
    let contracts = &installed.contracts;
    let scenario_contract = |name: &str| -> ScenarioContract {
        let contract = &contracts.contracts[name];
        let reach = contract
            .loaders
            .iter()
            .map(|loader| {
                installed.reach.get(loader).map_or_else(
                    || Reach::All {
                        witness: format!("`{loader}`'s reach was not computed on this route"),
                    },
                    |reach| reach.reach.clone(),
                )
            })
            .fold(Reach::Records(BTreeSet::new()), join_reach);
        ScenarioContract {
            name: name.to_owned(),
            identity: contracts.identity(name).unwrap_or_default(),
            in_force: contracts.in_force(name),
            envelope: contract.envelope.clone(),
            reach,
        }
    };
    let mut scenarios: Vec<(String, Vec<ScenarioContract>)> = contracts
        .contracts
        .keys()
        .map(|name| (name.clone(), vec![scenario_contract(name)]))
        .collect();
    scenarios.push((
        ALL_SCENARIO.to_owned(),
        contracts
            .contracts
            .keys()
            .map(|name| scenario_contract(name))
            .collect(),
    ));
    // The custody rows, keyed (library, file, function, occurrence).
    let mut custody: Vec<(RowKey, Verdict, BTreeSet<Fact>)> = Vec::new();
    let mut seen: BTreeMap<(String, String, String), usize> = BTreeMap::new();
    for unit in units {
        let library = unit["library"].as_str().unwrap_or_default().to_owned();
        let file = unit["file"].as_str().unwrap_or_default().to_owned();
        for function in unit["functions"].as_array().into_iter().flatten() {
            let name = function["function"].as_str().unwrap_or_default().to_owned();
            let Some(verdict) = parse_verdict(&function["status"]) else {
                continue;
            };
            let occurrence = seen
                .entry((library.clone(), file.clone(), name.clone()))
                .or_default();
            let key = (library.clone(), file.clone(), name.clone(), *occurrence);
            *occurrence += 1;
            let facts = custody_facts(&function["contract_dependencies"], premises);
            custody.push((key, verdict, facts));
        }
    }
    let custody_facts_of: BTreeMap<RowKey, BTreeSet<Fact>> = custody
        .iter()
        .map(|(key, _, facts)| (key.clone(), facts.clone()))
        .collect();
    let mut null: Vec<NullRow> = Vec::new();
    let mut seen: BTreeMap<(String, String, String), usize> = BTreeMap::new();
    for row in null_rows.into_iter().flatten() {
        let library = row["library"].as_str().unwrap_or_default().to_owned();
        let file = row["file"].as_str().unwrap_or_default().to_owned();
        let name = row["function"].as_str().unwrap_or_default().to_owned();
        let Some(verdict) = parse_verdict(&row["null"]["verdict"]) else {
            continue;
        };
        let occurrence = seen
            .entry((library.clone(), file.clone(), name.clone()))
            .or_default();
        let key = (library, file, name, *occurrence);
        *occurrence += 1;
        null.push((
            key,
            verdict,
            null_facts(&row["null"]),
            parse_verdict(&row["custody"]),
            parse_verdict(&row["combined"]),
        ));
    }
    let static_section = json!({
        "custody": tally(custody.iter().map(|(_, v, _)| v.as_str())),
        "null": tally(null.iter().map(|(_, v, ..)| v.as_str())),
        "combined": tally(null.iter().filter_map(|(.., c)| c.map(Verdict::as_str))),
    });
    let mut scenario_sections = serde_json::Map::new();
    let row_value = |key: &RowKey| json!({"library": key.0, "file": key.1, "function": key.2, "occurrence": key.3});
    for (name, scenario) in &scenarios {
        let mut custody_runtime = Vec::new();
        let mut moved = Vec::new();
        for (key, verdict, facts) in &custody {
            let (runtime, reasons) = runtime_verdict(*verdict, facts, scenario);
            custody_runtime.push(runtime);
            if runtime != *verdict {
                let mut row = row_value(key);
                row["part"] = json!("custody");
                row["static"] = json!(verdict.as_str());
                row["runtime"] = json!(runtime.as_str());
                row["reasons"] = json!(reasons);
                moved.push(row);
            }
        }
        let mut null_runtime = Vec::new();
        let mut combined_runtime = Vec::new();
        for (key, verdict, facts, custody_static, combined_static) in &null {
            let (runtime, reasons) = runtime_verdict(*verdict, facts, scenario);
            null_runtime.push(runtime);
            if runtime != *verdict {
                let mut row = row_value(key);
                row["part"] = json!("null");
                row["static"] = json!(verdict.as_str());
                row["runtime"] = json!(runtime.as_str());
                row["reasons"] = json!(reasons);
                moved.push(row);
            }
            if let Some(combined) = combined_static {
                let custody_facts = custody_facts_of.get(key).cloned().unwrap_or_default();
                let custody_runtime = custody_static
                    .map(|custody| runtime_verdict(custody, &custody_facts, scenario).0);
                let runtime_combined = custody_runtime
                    .map_or(*combined, |c| combined.join(c))
                    .join(runtime);
                combined_runtime.push(runtime_combined);
                if runtime_combined != *combined {
                    let mut row = row_value(key);
                    row["part"] = json!("combined");
                    row["static"] = json!(combined.as_str());
                    row["runtime"] = json!(runtime_combined.as_str());
                    moved.push(row);
                }
            }
        }
        let moves = |part: &str| {
            let mut counts = BTreeMap::<String, usize>::new();
            for row in moved.iter().filter(|row| row["part"] == part) {
                *counts
                    .entry(format!(
                        "{} -> {}",
                        row["static"].as_str().unwrap_or_default(),
                        row["runtime"].as_str().unwrap_or_default()
                    ))
                    .or_default() += 1;
            }
            json!(counts)
        };
        scenario_sections.insert(
            name.clone(),
            json!({
                "contracts": scenario.iter().map(|c| json!({
                    "name": c.name,
                    "identity": c.identity,
                    "in_force": c.in_force,
                    "reach": reach_value(&c.reach),
                })).collect::<Vec<_>>(),
                "custody": tally(custody_runtime.iter().map(|v| v.as_str())),
                "null": tally(null_runtime.iter().map(|v| v.as_str())),
                "combined": tally(combined_runtime.iter().map(|v| v.as_str())),
                "moves": {"custody": moves("custody"), "null": moves("null"), "combined": moves("combined")},
                "moved": moved,
            }),
        );
    }
    let contract_records = contracts
        .contracts
        .values()
        .map(|contract| {
            json!({
                "name": contract.name,
                "identity": contracts.identity(&contract.name),
                "statement": contract.statement,
                "defined_by": contract.defined_by,
                "loaders": contract.loaders,
                "in_force": contracts.in_force(&contract.name),
                "envelope": contract.envelope.iter().map(|line| match line {
                    EnvelopeLine::Member { tag, path, envelope } => {
                        json!({"member": format!("{tag}.{path}"), "envelope": format!("#{}", envelope.word())})
                    }
                    EnvelopeLine::Premise { name } => json!({"premise": name}),
                }).collect::<Vec<_>>(),
            })
        })
        .collect::<Vec<_>>();
    let loaders = installed
        .contracts
        .loaders
        .values()
        .map(|loader| {
            json!({
                "loader": loader.name,
                "contract": loader.contract,
                "runtime": loader.runtime.iter().map(|c| json!({
                    "binder": c.binder,
                    "parameter": c.index,
                    "guard": c.guard.as_ref().map(|g| format!("#result {} {}", match g.op {
                        qcore_contract::outer_contract::GuardOp::Equal => "==",
                        qcore_contract::outer_contract::GuardOp::NotEqual => "!=",
                    }, g.value)),
                })).collect::<Vec<_>>(),
                "pin_hold": loader.pin_hold.as_ref().map(|h| json!({"reason": h.reason, "detail": h.detail})),
                "reach": installed.reach.get(&loader.name).map(|r| json!({
                    "reach": reach_value(&r.reach),
                    "evidence": r.evidence,
                })),
            })
        })
        .collect::<Vec<_>>();
    Some(json!({
        "schema_version": 1,
        "meaning": MEANING,
        "rule": "31",
        "selection": installed.record,
        "not_closed_world": NOT_CLOSED_WORLD,
        "contracts": contract_records,
        "loaders": loaders,
        "static": static_section,
        "scenarios": scenario_sections,
    }))
}

/// Write the projection to [`PROJECTION`]'s path and print its tally line.
///
/// # Errors
///
/// An unwritable path.
pub fn write(projection: &Value) -> Result<(), String> {
    let Some(path) = std::env::var_os(PROJECTION) else {
        return Ok(());
    };
    let path = PathBuf::from(path);
    let mut bytes = serde_json::to_vec_pretty(projection).map_err(|e| e.to_string())?;
    bytes.push(b'\n');
    fs::write(&path, bytes).map_err(|e| format!("could not write {}: {e}", path.display()))?;
    let all = &projection["scenarios"][ALL_SCENARIO];
    println!(
        "outer projection (all loaded): custody {}, null {}, combined {}; static custody {}",
        all["custody"], all["null"], all["combined"], projection["static"]["custody"]
    );
    Ok(())
}
