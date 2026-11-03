//! Bounds contract transport on the audit routes (card
//! **CHUNKIO-META-LEN-BOUNDS-CONTRACTS**, gateway rule 12r, contract
//! specification C09.1 and C06.2).
//!
//! # Placement
//!
//! A bounds contract v1 module lives in its library's provider crate, like a
//! resource module, and is listed on the published load path
//! (`providers/contract-load-path.json`) in the crate's generated
//! **bounds view**: the owned modules the bounds loader accepts alone
//! ([`crate::providers::probe_resource_views`] computes both views;
//! `providers/contracts.py` writes and checks them). A module is in at most
//! one of a crate's resource and bounds views, so no resource selection ever
//! loads a bounds module, and no bounds selection a resource one.
//!
//! # Selection
//!
//! The bounds property is selected by `QCORE_BOUNDS_PROPERTY`
//! ([`crate::projection::BOUNDS_SELECTION`]). When it is set, the pinned route
//! (the corpus arbiter) and the configured route (`current`) call
//! [`select_for_route`] before any unit is analysed: every bounds view on the
//! load path their resource manifest names is loaded (selection
//! `all-bounds-views`, in crate order), its pins are evaluated over the
//! audited source, and the environment is installed for the run; every
//! unit's bounds rows are then analysed under it
//! ([`crate::projection`]'s `record_bounds` reads [`environment`]). Unset,
//! nothing is read or installed, and every artifact is byte-identical to a run
//! before this card. A route that installs nothing (the others) runs with the
//! empty environment, as before.
//!
//! # Pins
//!
//! A bounds declaration is pinned (C06.2's default; bounds v1 has no
//! flexible mode). Its reviewed identities live in the crate's
//! [`BOUNDS_LOCK_FILE`] beside its `Cargo.toml`, in the resource lock's format
//! ([`crate::contract_revision::ContractLock`]), with the declaration's
//! normalised text digest (`text_sha256`, which covers every clause, the quant
//! check's predicate included). `quirin-audit pin --bounds` writes and checks
//! it; nothing here writes a lock. A declaration whose pin is not in force
//! (`no-lock`, `not-in-lock`, `contract-changed`, `source-changed`) is held
//! ([`qcore_contract::bounds_contract::BoundsContracts::hold_for_pin_mismatch`]),
//! exactly as a review marker holds it: it discharges nothing and accuses
//! nothing, so a verdict resting on it is `Pending` at most, never `Safe`.
//!
//! The installed environment is process state, as the selecting variable is.
//! Lean model: `proof/QCore/BoundsPinHold.lean`.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use qcore_contract::bounds_contract::{BoundsContracts, load_bounds_contract_sources};
use qcore_contract::load_path_manifest::{LoadPathManifest, load_contract_load_path_manifest};
use qcore_schemas::sha256_hex;
use serde_json::{Value, json};

use crate::c_identity::SourceTree;
use crate::contract_revision::{
    ContractLock, LIBC_CRATE, PinStanding, ReviewedSource, pin_standing,
};
use crate::contract_tip::TipResolver;

/// The bounds lock's file name, beside a provider crate's `Cargo.toml`.
pub const BOUNDS_LOCK_FILE: &str = "bounds-contracts.lock";

/// The one selection rule of the audit routes.
pub const SELECTION: &str = "all-bounds-views";

/// What the selection record says it means.
pub const RULE: &str = "with QCORE_BOUNDS_PROPERTY set, the pinned and configured routes load every bounds view on the load path their resource manifest names (each crate's owned modules the bounds contract v1 loader accepts, generated and checked by providers/contracts.py, disjoint from its resource view), compare each declaration with its crate's bounds-contracts.lock over the audited source (C06.2: in force only where the contract text is the pinned one and the audited sites equal a reviewed identity), hold every declaration not in force (contract-pin-mismatch: Pending at most, never Safe, never accusing), and analyse every unit's bounds rows under that environment";

struct Installed {
    /// Card CELL-GUARD-RESULT-RANGE-CHECKED: the derived result sets rule 38
    /// reads, with their standing.
    result_ranges: BTreeMap<String, qcore_schemas::result_range::ResultRangeFact>,
    /// The bounds consumer's view (null-deref clauses removed).
    contracts: Arc<BoundsContracts>,
    /// Card NULL-SAFETY-PROPERTY: the environment as loaded and pinned, the
    /// null-dereference property's source of `quant null-deref` clauses.
    full: Arc<BoundsContracts>,
    // Exact selected texts for property-specific evidence replay only.
    hypothesis_sources: BTreeMap<String, String>,
    record: Value,
}

static INSTALLED: RwLock<Option<Installed>> = RwLock::new(None);

/// The bounds environment installed for this run, or the empty one.
#[must_use]
pub fn environment() -> Arc<BoundsContracts> {
    INSTALLED
        .read()
        .ok()
        .and_then(|installed| installed.as_ref().map(|i| Arc::clone(&i.contracts)))
        .unwrap_or_default()
}

/// Card NULL-SAFETY-PROPERTY (rule 14o): the installed environment with its
/// `quant null-deref` clauses (pinned and held exactly as every bounds
/// declaration), or the empty one.
#[must_use]
pub fn null_environment() -> Arc<BoundsContracts> {
    INSTALLED
        .read()
        .ok()
        .and_then(|installed| installed.as_ref().map(|i| Arc::clone(&i.full)))
        .unwrap_or_default()
}

/// Exact already-selected bounds texts; no filesystem lookup or resource-view merge.
pub(crate) fn null_hypothesis_sources() -> Result<BTreeMap<String, String>, String> {
    let slot = INSTALLED
        .read()
        .map_err(|_| "bounds hypothesis: environment lock poisoned")?;
    Ok(slot
        .as_ref()
        .map(|i| i.hypothesis_sources.clone())
        .unwrap_or_default())
}

/// Card CELL-GUARD-RESULT-RANGE-CHECKED (contract specification C09.4,
/// gateway rule 38e): the pinned result sets of the installed environment,
/// for the resource environment rule 38 reads (empty when nothing is
/// installed, so the resource environment is unchanged).
#[must_use]
pub fn result_range_table() -> BTreeMap<String, qcore_schemas::result_range::ResultRangeFact> {
    INSTALLED
        .read()
        .ok()
        .and_then(|installed| installed.as_ref().map(|i| i.result_ranges.clone()))
        .unwrap_or_default()
}

/// Every pinned declaration stating a result set: in force only when it is
/// not held and its lock carries a current derivation (rule 38e); held when
/// held; unchecked when never derived.
fn result_ranges_of(
    contracts: &BoundsContracts,
    record: &Value,
) -> BTreeMap<String, qcore_schemas::result_range::ResultRangeFact> {
    use qcore_schemas::result_range::{ResultRangeFact, ResultRangeStanding};
    let derivations = record["pins"]["outcomes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|outcome| {
            Some((
                outcome["function"].as_str()?.to_owned(),
                outcome.get("derivation")?.clone(),
            ))
        })
        .collect::<BTreeMap<_, _>>();
    let mut out = BTreeMap::new();
    for (name, contract) in &contracts.functions {
        if contract.flexible {
            continue;
        }
        let Some(set) = contract.result_values() else {
            continue;
        };
        let fits = |value: i128| i64::try_from(value).ok();
        let (Some(lo), Some(hi)) = (fits(set.universe().0), fits(set.universe().1)) else {
            continue;
        };
        let Some(intervals) = set
            .intervals()
            .iter()
            .map(|(a, b)| Some((fits(*a)?, fits(*b)?)))
            .collect::<Option<Vec<_>>>()
        else {
            continue;
        };
        let (standing, detail) = if contract.held() {
            (ResultRangeStanding::Held, contract.holds().join("; "))
        } else if let Some(derivation) = derivations.get(name) {
            (ResultRangeStanding::InForce, derivation.to_string())
        } else {
            (
                ResultRangeStanding::Unchecked,
                "the lock records no result derivation (rule 38e)".to_owned(),
            )
        };
        out.insert(
            name.clone(),
            ResultRangeFact {
                intervals,
                universe: (lo, hi),
                standing,
                detail,
            },
        );
    }
    out
}

/// The selection record of the installed environment, if any.
#[must_use]
pub fn installed_record() -> Option<Value> {
    INSTALLED
        .read()
        .ok()
        .and_then(|installed| installed.as_ref().map(|i| i.record.clone()))
}

/// Card CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS (rule 14h): record
/// the bounds refutation table (the declarations some unit's body refutes)
/// in the installed environment, so a route's second phase withdraws each
/// from every unit's callers (C06.4's two phases, `BoundsContracts::refuted`),
/// and name it in the selection record. An empty table changes nothing; with
/// no installed environment there is nothing to withdraw.
pub fn apply_refutation_table(table: &BTreeSet<String>) {
    if table.is_empty() {
        return;
    }
    let mut slot = INSTALLED
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(installed) = slot.as_mut() {
        let mut contracts = BoundsContracts::clone(&installed.contracts);
        contracts.refuted.clone_from(table);
        installed.record["refutation_table"] = json!({
            "meaning": REFUTATION_MEANING,
            "refuted": table,
            "environment_sha256": contracts.digest(),
        });
        installed.contracts = Arc::new(contracts);
    }
}

/// What the record's refutation table means.
pub const REFUTATION_MEANING: &str = "bounds declarations whose own body refuted them in some unit's first phase (an access outside a quant check, a write beyond a declared frame, a return contradicting a declared extent or relation): the route's second phase analysed every unit's bounds rows with each withdrawn from its callers, who then see an uncontracted callee (C06.4's two phases, gateway rule 14h); the defining body keeps its own declaration";

fn install(
    contracts: BoundsContracts,
    record: Value,
    sources: BTreeMap<String, String>,
) -> Result<(), String> {
    let actual = sources
        .iter()
        .map(|(key, text)| (key.clone(), sha256_hex(text.as_bytes())))
        .collect::<BTreeMap<_, _>>();
    if actual != contracts.sources {
        return Err(
            "bounds hypothesis: installed source bytes do not match selected environment".into(),
        );
    }
    let mut slot = INSTALLED
        .write()
        .map_err(|_| "the bounds environment lock is poisoned".to_owned())?;
    let result_ranges = result_ranges_of(&contracts, &record);
    *slot = Some(Installed {
        result_ranges,
        contracts: Arc::new(contracts.for_bounds()),
        full: Arc::new(contracts),
        hypothesis_sources: sources,
        record,
    });
    Ok(())
}

/// The load path a schema-2 contract manifest names, relative to the
/// manifest's directory; `None` for a manifest that names none.
///
/// # Errors
///
/// A manifest that is not JSON refuses.
pub fn manifest_load_path(
    manifest_path: &Path,
    manifest_text: &str,
) -> Result<Option<PathBuf>, String> {
    let manifest: Value = serde_json::from_str(manifest_text)
        .map_err(|error| format!("{}: {error}", manifest_path.display()))?;
    Ok(manifest["load_path"].as_str().map(|load_path| {
        manifest_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(load_path)
    }))
}

/// Every bounds view on a load path, loaded together by the bounds loader,
/// each module keyed `crate:module`; and one record per module (crate,
/// module, SHA-256). An empty set of views is the empty environment.
///
/// # Errors
///
/// A view the bounds loader refuses refuses the whole selection.
pub fn load_views(manifest: &LoadPathManifest) -> Result<(BoundsContracts, Vec<Value>), String> {
    let (sources, modules) = selected_sources(manifest)?;
    if sources.is_empty() {
        return Ok((BoundsContracts::default(), modules));
    }
    let contracts = load_bounds_contract_sources(&sources)
        .map_err(|error| format!("the bounds views do not load together: {error}"))?;
    Ok((contracts, modules))
}

fn selected_sources(
    manifest: &LoadPathManifest,
) -> Result<(BTreeMap<String, String>, Vec<Value>), String> {
    let mut sources = BTreeMap::new();
    let mut modules = Vec::new();
    for (crate_name, view) in &manifest.bounds_views {
        let krate = manifest.load_path.crate_named(crate_name).ok_or_else(|| {
            format!("bounds view of `{crate_name}`, which is not on the load path")
        })?;
        for module in view {
            let text = krate.modules.get(module).ok_or_else(|| {
                format!("bounds view of `{crate_name}` names `{module}`, which it does not own")
            })?;
            modules.push(json!({
                "crate": crate_name,
                "module": module,
                "sha256": sha256_hex(text.as_bytes()),
            }));
            sources.insert(format!("{crate_name}:{module}"), text.clone());
        }
    }
    Ok((sources, modules))
}

/// The bounds lock of the crate owning a `crate:module` key, when the file
/// exists.
#[must_use]
pub fn lock_path(manifest: &LoadPathManifest, module: &str) -> Option<PathBuf> {
    let (crate_name, _) = module.split_once(':')?;
    let root = manifest.contract_roots.get(crate_name)?.parent()?;
    Some(root.join(BOUNDS_LOCK_FILE)).filter(|lock| lock.is_file())
}

/// Compare every bounds declaration with its crate's bounds lock over the
/// source at `source_root`, and hold every declaration not in force. Returns
/// the pin section of the selection record.
///
/// # Errors
///
/// A malformed lock, an unreadable source tree or a malformed tip variable
/// refuses; a missing lock or entry is a mismatch, not an error.
pub fn apply_bounds_pins(
    contracts: &mut BoundsContracts,
    manifest: &LoadPathManifest,
    source_root: &Path,
) -> Result<Value, String> {
    let mut locks: BTreeMap<PathBuf, ContractLock> = BTreeMap::new();
    let mut lock_records = serde_json::Map::new();
    let mut lock_of = BTreeMap::new();
    for (name, contract) in &contracts.functions {
        let lock = lock_path(manifest, &contract.module);
        if let Some(path) = lock.as_ref().filter(|path| !locks.contains_key(*path)) {
            let read = ContractLock::read(path)?;
            lock_records.insert(
                path.display().to_string(),
                json!({
                    "owner": read.owner,
                    "sha256": sha256_hex(&fs::read(path).map_err(|e| e.to_string())?),
                    "reviewed_at": read.reviewed_sources(),
                }),
            );
            locks.insert(path.clone(), read);
        }
        lock_of.insert(name.clone(), lock);
    }
    if contracts.functions.is_empty() {
        return Ok(json!({
            "source": Value::Null,
            "locks": lock_records,
            "in_force": 0,
            "held": 0,
            "outcomes": [],
        }));
    }
    let wanted = contracts
        .functions
        .iter()
        .filter(|(_, contract)| !contract.flexible)
        .map(|(name, _)| name)
        .filter(|name| {
            lock_of[*name]
                .as_ref()
                .and_then(|path| locks.get(path))
                .is_some_and(|lock| {
                    lock.pinned.iter().any(|entry| &entry.function == *name)
                        || lock.tip.iter().any(|entry| &entry.function == *name)
                })
        })
        .cloned()
        .collect::<BTreeSet<_>>();
    // Card CELL-GUARD-RESULT-RANGE-CHECKED (rule 38e): a derived range's
    // helpers' sites join its identity.
    let helpers_of = |name: &str| -> Vec<String> {
        lock_of
            .get(name)
            .and_then(Option::as_ref)
            .and_then(|path| locks.get(path))
            .and_then(|lock| lock.pinned.iter().find(|entry| entry.function == name))
            .and_then(|entry| entry.derivation.as_ref())
            .map(|derivation| derivation.helpers.clone())
            .unwrap_or_default()
    };
    let mut scanned = wanted.clone();
    for name in &wanted {
        scanned.extend(helpers_of(name));
    }
    let scan = SourceTree::read_sites(source_root, &scanned)?;
    let mut tips = TipResolver::from_environment()?;
    let mut outcomes = Vec::new();
    let mut holds = Vec::new();
    // The C library result sets a derivation may rest on: the libc crate's
    // flexible declarations listed in its lock (a held one rests nothing).
    let rest_view = contracts
        .functions
        .iter()
        .filter(|(name, contract)| {
            contract.flexible
                && !contract.held()
                && contract.module.starts_with(&format!("{LIBC_CRATE}:"))
                && lock_of[*name]
                    .as_ref()
                    .and_then(|path| locks.get(path))
                    .is_some_and(|lock| {
                        lock.flexible.iter().any(|entry| {
                            &entry.function == *name
                                && entry.contract_sha256 == contract.text_sha256
                        })
                    })
        })
        .map(|(name, contract)| {
            (
                name.clone(),
                (contract.module.clone(), contract.text_sha256.clone()),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut tip_records = Vec::new();
    for (name, contract) in &contracts.functions {
        let lock_file = lock_of[name].as_ref();
        let lock = lock_file.and_then(|path| locks.get(path));
        // Card CELL-GUARD-RESULT-RANGE-CHECKED (C09.4): a C library result
        // set is flexible: in force by name and declared signature under the
        // standing libc assumption, never pinned; admitted only from the libc
        // provider, held anywhere else.
        if contract.flexible {
            let libc = contract.module.starts_with(&format!("{LIBC_CRATE}:"));
            let listed = lock.is_some_and(|lock| {
                lock.flexible.iter().any(|entry| {
                    &entry.function == name && entry.contract_sha256 == contract.text_sha256
                })
            });
            let (reason, detail) = if !libc {
                (
                    Some("not-in-lock"),
                    format!(
                        "a flexible bounds declaration is admitted only in {LIBC_CRATE} (C09.4)"
                    ),
                )
            } else if !listed {
                (
                    Some("not-in-lock"),
                    "the libc bounds lock lists no flexible entry with this contract text"
                        .to_owned(),
                )
            } else {
                (None, String::new())
            };
            if let Some(reason) = reason {
                holds.push((name.clone(), reason.to_owned(), detail.clone()));
            }
            outcomes.push(json!({
                "function": name,
                "module": contract.module,
                "lock": lock_file.map(|path| path.display().to_string()),
                "contract_sha256": contract.text_sha256,
                "standing": if reason.is_some() { "not-in-force" } else { "flexible" },
                "reason": reason,
                "detail": detail,
                "assumption": crate::contract_revision::LIBC_ASSUMPTION,
            }));
            continue;
        }
        let sites = scan.sites.get(name).cloned().unwrap_or_default();
        let helpers = helpers_of(name);
        let sites = if helpers.is_empty() {
            sites
        } else {
            crate::contract_revision::with_helper_sites(&scan.sites, sites.clone(), &helpers)
                .unwrap_or(sites)
        };
        let standing = pin_standing(
            lock,
            name,
            &contract.text_sha256,
            &sites,
            &contract.module,
            &mut tips,
        );
        // Card CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS (rule 14i): a tip
        // entry is reported as C06.2 reports a resource one.
        let notice = standing.notice(name);
        let tip = standing.tip().cloned();
        let at_tip = matches!(standing, PinStanding::InForceAtTip { .. });
        let (reason, detail, matched): (Option<&str>, String, Vec<ReviewedSource>) = match standing
        {
            PinStanding::InForce { reviewed_at } => (None, String::new(), reviewed_at),
            PinStanding::InForceAtTip { .. } => (None, String::new(), Vec::new()),
            PinStanding::Mismatch { reason, detail, .. } => {
                (Some(reason.as_str()), detail, Vec::new())
            }
        };
        // Card CELL-GUARD-RESULT-RANGE-CHECKED (rule 38e): a derived range
        // rests on the C library sets it read; a changed, missing or held one
        // holds it (`contract-changed`).
        let derivation = lock
            .and_then(|lock| lock.pinned.iter().find(|entry| &entry.function == name))
            .and_then(|entry| entry.derivation.clone());
        let (reason, detail) = match (&reason, &derivation) {
            (None, Some(derivation)) => match stale_rest(derivation, &rest_view) {
                Some(why) => (Some("contract-changed"), why),
                None => (reason, detail),
            },
            _ => (reason, detail),
        };
        if let Some(reason) = reason {
            holds.push((name.clone(), reason.to_owned(), detail.clone()));
        }
        let mut outcome = json!({
            "function": name,
            "module": contract.module,
            "lock": lock_file.map(|path| path.display().to_string()),
            "contract_sha256": contract.text_sha256,
            "standing": if reason.is_some() {
                "not-in-force"
            } else if at_tip {
                "tip-in-force"
            } else {
                "in-force"
            },
            "reason": reason,
            "detail": detail,
            "reviewed_at": matched,
            "sites": sites,
        });
        // Card CELL-GUARD-RESULT-RANGE-CHECKED: the derivation an in-force
        // range rests on.
        if let Some(derivation) = &derivation {
            outcome["derivation"] = serde_json::to_value(derivation).map_err(|e| e.to_string())?;
        }
        if let (Some(tip), Some(notice)) = (tip, notice) {
            outcome["tip"] = serde_json::to_value(&tip).map_err(|e| e.to_string())?;
            outcome["notice"] = serde_json::to_value(&notice).map_err(|e| e.to_string())?;
            tip_records.push((name.clone(), at_tip, tip, notice));
        }
        outcomes.push(outcome);
    }
    let held = holds.len();
    for (name, reason, detail) in holds {
        contracts.hold_for_pin_mismatch(&name, &reason, &detail)?;
    }
    let mut section = json!({
        "source": {"kind": scan.kind, "revision": scan.revision},
        "locks": lock_records,
        "in_force": outcomes.len() - held,
        "held": held,
        "outcomes": outcomes,
    });
    add_tip_tracking(&mut section, &tip_records)?;
    Ok(section)
}

/// Card CELL-GUARD-RESULT-RANGE-CHECKED: why a derivation no longer rests on
/// what it read, if it does not.
fn stale_rest(
    derivation: &crate::contract_revision::ResultDerivation,
    view: &BTreeMap<String, (String, String)>,
) -> Option<String> {
    derivation.rests_on.iter().find_map(|rested| {
        match view.get(&rested.function) {
            Some((module, sha)) if *module == rested.module && *sha == rested.contract_sha256 => {
                None
            }
            Some(_) => Some(format!(
                "the C library result set of `{}` the derivation rested on changed since it was derived",
                rested.function
            )),
            None => Some(format!(
                "the C library result set of `{}` the derivation rested on is not in force",
                rested.function
            )),
        }
    })
}

/// Card CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS (rule 14i): C06.2's tip
/// counts, notices and named assumptions for the bounds declarations that
/// track a tip, added only when one does (a run without one keeps its bytes).
fn add_tip_tracking(
    section: &mut Value,
    tips: &[(
        String,
        bool,
        crate::contract_tip::TipResolution,
        crate::contract_tip::Notice,
    )],
) -> Result<(), String> {
    if tips.is_empty() {
        return Ok(());
    }
    let in_force = tips.iter().filter(|(_, at_tip, ..)| *at_tip).count();
    section["tip_meaning"] = json!(crate::contract_revision::TIP_MEANING);
    section["tip_counts"] = json!({
        "tip_tracked": tips.len(),
        "tip_in_force": in_force,
        "tip_not_in_force": tips.len() - in_force,
    });
    let mut notices = Vec::new();
    let mut assumptions = Vec::new();
    for (function, at_tip, tip, notice) in tips {
        let mut record = serde_json::to_value(notice).map_err(|e| e.to_string())?;
        record["function"] = json!(function);
        notices.push(record);
        if *at_tip {
            assumptions.push(json!({
                "name": format!("{}{function}", crate::contract_tip::TIP_ASSUMPTION_PREFIX),
                "function": function,
                "statement": crate::contract_tip::TIP_STATEMENT,
                "repository": tip.repository,
                "ref": tip.reference,
                "release_label": tip.release_label,
                "resolved_commit": tip.resolved_commit,
                "reviewed_commit": tip.reviewed_commit,
                "moved_since_review": tip.moved_since_review,
                "sites_changed_since_review": tip.sites_changed_since_review,
            }));
        }
    }
    section["tip_notices"] = json!(notices);
    section["assumptions"] = json!({ "tip_tracked": assumptions });
    Ok(())
}

/// Select the bounds environment of an audit route (see the module docs):
/// nothing unless `QCORE_BOUNDS_PROPERTY` is set. `manifest_path` and
/// `manifest_text` are the route's resource contract manifest, whose
/// `load_path` names the providers; `source_root` is the audited checkout.
///
/// # Errors
///
/// An unreadable load path, a bounds view that does not load, a malformed
/// lock or an unreadable source refuses the run.
pub fn select_for_route(
    route: &str,
    manifest_path: &Path,
    manifest_text: &str,
    source_root: &Path,
    resource: &qcore_schemas::contract::ResourceContracts,
) -> Result<(), String> {
    // Card NULL-SAFETY-PROPERTY (rule 14o): the null property reads its
    // clauses from the same views, pinned the same way.
    if std::env::var_os(crate::projection::BOUNDS_SELECTION).is_none()
        && std::env::var_os(crate::projection::NULL_SELECTION).is_none()
    {
        return Ok(());
    }
    let Some(load_path) = manifest_load_path(manifest_path, manifest_text)? else {
        println!(
            "bounds contracts ({route}): the resource manifest names no load path; empty bounds environment"
        );
        return install(
            BoundsContracts::default(),
            json!({
                "schema_version": 1,
                "property": qcore_c_gateway::index_extent::BOUNDS_PROPERTY,
                "route": route,
                "selection": "none",
                "reason": "the route's resource contract manifest names no load path",
                "environment_sha256": BoundsContracts::default().digest(),
                "rule": RULE,
            }),
            BTreeMap::new(),
        );
    };
    let manifest =
        load_contract_load_path_manifest(&load_path).map_err(|error| error.to_string())?;
    let (mut contracts, modules) = load_views(&manifest)?;
    let pins = apply_bounds_pins(&mut contracts, &manifest, source_root)?;
    println!(
        "bounds contracts ({route}): {} declaration(s) from {} module(s); {} pinned in force, {} held (contract-pin-mismatch)",
        contracts.functions.len(),
        modules.len(),
        pins["in_force"],
        pins["held"]
    );
    // Card CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS (rule 14i): one line
    // per tip notice, only when a declaration tracks a tip.
    for notice in pins["tip_notices"].as_array().into_iter().flatten() {
        println!(
            "bounds contract notice [{}]: {}",
            notice["severity"].as_str().unwrap_or_default(),
            notice["message"].as_str().unwrap_or_default()
        );
    }
    // Card NULL-DEREF-QTT-GROUNDING: a liveness part is admitted only where
    // the function's own resource v1 declaration states it; otherwise the
    // route is refused (`quant-live-unbacked`, `quant-live-contradicts-custody`).
    let live_backing = qcore_contract::deref_parts::check_live_backing(&contracts, resource)
        .map_err(|error| format!("bounds contracts ({route}): {error}"))?;
    let mut record = json!({
        "schema_version": 1,
        "property": qcore_c_gateway::index_extent::BOUNDS_PROPERTY,
        "contract_version": qcore_contract::bounds_contract::BOUNDS_CONTRACT_VERSION,
        "route": route,
        "selection": SELECTION,
        "load_path": load_path,
        "load_path_sha256": sha256_hex(&fs::read(&load_path).map_err(|e| e.to_string())?),
        "modules": modules,
        "declarations": contracts.functions.keys().collect::<Vec<_>>(),
        "environment_sha256": contracts.for_bounds().digest(),
        "pins": pins,
        "trust": "declared bounds assumptions (C09.1: a resolved declaration is a Declared premise, not a proof of its C body), in force only where pinned",
        "rule": RULE,
    });
    // Card NULL-DEREF-QTT-GROUNDING: with any liveness part (a shorthand
    // always gives one), every declaration's canonical digest and every
    // backed liveness part; absent otherwise, so an environment without the
    // new forms writes the same record as before.
    if qcore_contract::deref_parts::states_live(&contracts) {
        record["canonical_sha256"] =
            json!(qcore_contract::deref_parts::canonical_digests(&contracts));
        record["live_backing"] = json!(live_backing);
    }
    let (sources, _) = selected_sources(&manifest)?;
    install(contracts, record, sources)
}
