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
use crate::contract_revision::{ContractLock, PinStanding, ReviewedSource, pin_standing};
use crate::contract_tip::TipResolver;

/// The bounds lock's file name, beside a provider crate's `Cargo.toml`.
pub const BOUNDS_LOCK_FILE: &str = "bounds-contracts.lock";

/// The one selection rule of the audit routes.
pub const SELECTION: &str = "all-bounds-views";

/// What the selection record says it means.
pub const RULE: &str = "with QCORE_BOUNDS_PROPERTY set, the pinned and configured routes load every bounds view on the load path their resource manifest names (each crate's owned modules the bounds contract v1 loader accepts, generated and checked by providers/contracts.py, disjoint from its resource view), compare each declaration with its crate's bounds-contracts.lock over the audited source (C06.2: in force only where the contract text is the pinned one and the audited sites equal a reviewed identity), hold every declaration not in force (contract-pin-mismatch: Pending at most, never Safe, never accusing), and analyse every unit's bounds rows under that environment";

struct Installed {
    contracts: Arc<BoundsContracts>,
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

/// The selection record of the installed environment, if any.
#[must_use]
pub fn installed_record() -> Option<Value> {
    INSTALLED
        .read()
        .ok()
        .and_then(|installed| installed.as_ref().map(|i| i.record.clone()))
}

fn install(contracts: BoundsContracts, record: Value) -> Result<(), String> {
    let mut slot = INSTALLED
        .write()
        .map_err(|_| "the bounds environment lock is poisoned".to_owned())?;
    *slot = Some(Installed {
        contracts: Arc::new(contracts),
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
    if sources.is_empty() {
        return Ok((BoundsContracts::default(), modules));
    }
    let contracts = load_bounds_contract_sources(&sources)
        .map_err(|error| format!("the bounds views do not load together: {error}"))?;
    Ok((contracts, modules))
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
        .keys()
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
    let scan = SourceTree::read_sites(source_root, &wanted)?;
    let mut tips = TipResolver::from_environment()?;
    let mut outcomes = Vec::new();
    let mut holds = Vec::new();
    for (name, contract) in &contracts.functions {
        let lock_file = lock_of[name].as_ref();
        let lock = lock_file.and_then(|path| locks.get(path));
        let sites = scan.sites.get(name).cloned().unwrap_or_default();
        let standing = pin_standing(
            lock,
            name,
            &contract.text_sha256,
            &sites,
            &contract.module,
            &mut tips,
        );
        let (reason, detail, matched): (Option<&str>, String, Vec<ReviewedSource>) = match standing
        {
            PinStanding::InForce { reviewed_at } => (None, String::new(), reviewed_at),
            PinStanding::InForceAtTip { .. } => (None, String::new(), Vec::new()),
            PinStanding::Mismatch { reason, detail, .. } => {
                (Some(reason.as_str()), detail, Vec::new())
            }
        };
        if let Some(reason) = reason {
            holds.push((name.clone(), reason.to_owned(), detail.clone()));
        }
        outcomes.push(json!({
            "function": name,
            "module": contract.module,
            "lock": lock_file.map(|path| path.display().to_string()),
            "contract_sha256": contract.text_sha256,
            "standing": if reason.is_some() { "not-in-force" } else { "in-force" },
            "reason": reason,
            "detail": detail,
            "reviewed_at": matched,
            "sites": sites,
        }));
    }
    let held = holds.len();
    for (name, reason, detail) in holds {
        contracts.hold_for_pin_mismatch(&name, &reason, &detail)?;
    }
    Ok(json!({
        "source": {"kind": scan.kind, "revision": scan.revision},
        "locks": lock_records,
        "in_force": outcomes.len() - held,
        "held": held,
        "outcomes": outcomes,
    }))
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
) -> Result<(), String> {
    if std::env::var_os(crate::projection::BOUNDS_SELECTION).is_none() {
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
    let record = json!({
        "schema_version": 1,
        "property": qcore_c_gateway::index_extent::BOUNDS_PROPERTY,
        "contract_version": qcore_contract::bounds_contract::BOUNDS_CONTRACT_VERSION,
        "route": route,
        "selection": SELECTION,
        "load_path": load_path,
        "load_path_sha256": sha256_hex(&fs::read(&load_path).map_err(|e| e.to_string())?),
        "modules": modules,
        "declarations": contracts.functions.keys().collect::<Vec<_>>(),
        "environment_sha256": contracts.digest(),
        "pins": pins,
        "trust": "declared bounds assumptions (C09.1: a resolved declaration is a Declared premise, not a proof of its C body), in force only where pinned",
        "rule": RULE,
    });
    install(contracts, record)
}
