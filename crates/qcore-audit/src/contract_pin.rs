//! `quirin-audit pin` (card **W45-CONTRACT-REVISION-MODE**): write or check
//! contract locks after human review.
//!
//! ```text
//! quirin-audit pin --source-root DIR [--load-path FILE] [--crate NAME]... [--function NAME]... [--regenerate]
//! quirin-audit pin --source-root DIR [--revision REV] --check [--crate NAME]... [--function NAME]...
//! quirin-audit pin --drop-revision REV [--load-path FILE] [--crate NAME]... [--function NAME]...
//! quirin-audit pin --verify-contracts [--load-path FILE] [--crate NAME]...
//! (`--modules DIR` in place of `--load-path`/`--crate` pins a module directory)
//! ```
//!
//! A pinned entry holds one or more **reviewed identities**, each the sites
//! of its function at the revisions it was reviewed at. `pin --source-root
//! DIR` **adds or refreshes** the identity of that checkout's `HEAD` for every
//! declaration of every crate with a resource view (so pinning new contracts
//! is one command), and never drops an identity recorded at another
//! revision; a contract whose text changed loses every earlier identity.
//! `--function NAME` restricts it to named entries, `--regenerate` rebuilds
//! each lock from this source alone, and `--drop-revision REV` removes the
//! identities recorded at REV (a full revision, a prefix of at least seven
//! characters, or `directory`). `--check` never writes: per revision it says
//! which entries are recorded there, in force through an identity reviewed
//! at another revision, or out of force, and exits 1 unless every entry is
//! recorded there; `--revision REV` checks REV's blobs instead of `HEAD`'s.
//! `--verify-contracts` needs no source: it checks that each lock lists
//! exactly the current declarations, modes and contract digests, which
//! `providers/contracts.py check` runs. Nothing here runs during an audit.
//!
//! Card **CONTRACT-PIN-TIP-MODE**: `--track-tip REF` (with `--function`)
//! records a **tip entry** instead, reviewed at REF's current commit of the
//! repository at `--source-root` and in force wherever the audited sites
//! equal those at REF's commit when the audit runs
//! ([`crate::contract_tip`]); `--untrack-tip` turns it back into a reviewed
//! identity of `HEAD`. Every other run keeps tip entries as written. Every
//! command that meets a tip entry prints its notice: the contract targets a
//! future, unreleased release (`info` in force, `warning` not).
//!
//! Card **CHUNKIO-META-LEN-BOUNDS-CONTRACTS**: `--bounds` selects the bounds
//! locks instead (`bounds-contracts.lock`, one per crate with a non-empty
//! bounds view, [`crate::bounds_selection`]): `pin --bounds`, `pin --bounds
//! --check`, `--regenerate` and `--drop-revision` act on them exactly as on
//! the resource locks; every bounds declaration is pinned (bounds v1 has no
//! flexible mode). `--verify-contracts` verifies the resource and the bounds
//! locks. Card **CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS** (rule 14i):
//! `--bounds --track-tip REF` and `--bounds --untrack-tip` record and turn back
//! tip entries in the bounds locks exactly as in the resource locks.
//!
//! Card **OUTER-CONTRACT** (rule 31): `--outer` selects the outer contract
//! locks (`outer-contracts.lock`, one per crate with a non-empty outer view,
//! [`crate::outer_selection`]), exactly as `--bounds` selects the bounds
//! locks; each entry pins a loader declaration to the loader's body.
//! `--verify-contracts` verifies all three kinds of lock.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use qcore_contract::load_path_manifest::load_contract_load_path_manifest;
use qcore_schemas::contract::ResourceContracts;
use qcore_schemas::contract_revision::RevisionMode;

use crate::c_identity::{NORMALISATION, SourceTree};
use crate::contract_revision::{
    ContractLock, FlexibleEntry, LOCK_FILE, LOCK_VERSION, PinStanding, PinnedEntry, ReviewedSource,
    pin_standing,
};
use crate::contract_tip::{
    NoticeStanding, TipEntry, TipRepository, TipResolver, TipReview, git_output, is_repository,
    notice, valid_name,
};
use crate::providers::{ContractInput, SelectionSource, resolve_providers};

/// Usage text of `quirin-audit pin`.
pub const USAGE: &str = "usage: quirin-audit pin --source-root DIR [--load-path FILE | --modules \
DIR] [--crate NAME]... [--function NAME]... [--regenerate | --check [--revision REV]] | pin \
--drop-revision REV [--load-path FILE | --modules DIR] [--crate NAME]... [--function NAME]... | \
pin --verify-contracts [--load-path FILE] [--crate NAME]... | pin --source-root DIR --function \
NAME... --track-tip REF [--repository-name NAME] [--repository-url URL] [--release-label TEXT] | \
pin --source-root DIR --function NAME... --untrack-tip (`--bounds` or `--outer` with a pin, \
--check, --regenerate or --drop-revision selects the bounds or the outer locks)";

/// The lock's generated-by line.
pub const GENERATED_BY: &str = "quirin-audit pin";

/// The lock's note.
pub const NOTE: &str = "Generated by `quirin-audit pin` after human review of every pinned contract against each recorded source revision. Do not edit by hand: a hand-edited identity is a pin nobody reviewed. A pinned contract is in force only where its contract text is the pinned one and the audited source matches one of its reviewed identities.";

/// The default load path, relative to the working directory.
pub const DEFAULT_LOAD_PATH: &str = "providers/contract-load-path.json";

#[derive(Default)]
struct Arguments {
    source_root: Option<PathBuf>,
    load_path: Option<PathBuf>,
    modules: Option<PathBuf>,
    crates: Vec<String>,
    functions: BTreeSet<String>,
    check: bool,
    verify_contracts: bool,
    regenerate: bool,
    drop_revision: Option<String>,
    revision: Option<String>,
    /// Card CONTRACT-PIN-TIP-MODE: track the tip of this ref.
    track_tip: Option<String>,
    /// Card CONTRACT-PIN-TIP-MODE: turn tip entries back into reviewed pins.
    untrack_tip: bool,
    repository_name: Option<String>,
    repository_url: Option<String>,
    release_label: Option<String>,
    /// Card CHUNKIO-META-LEN-BOUNDS-CONTRACTS: act on the bounds locks.
    bounds: bool,
    /// Card OUTER-CONTRACT: act on the outer contract locks.
    outer: bool,
}

fn parse(arguments: Vec<String>) -> Result<Arguments, String> {
    let mut parsed = Arguments::default();
    let mut iter = arguments.into_iter();
    while let Some(argument) = iter.next() {
        let mut value = |name: &str| iter.next().ok_or_else(|| format!("{name} needs a value"));
        match argument.as_str() {
            "--source-root" => parsed.source_root = Some(PathBuf::from(value("--source-root")?)),
            "--load-path" => parsed.load_path = Some(PathBuf::from(value("--load-path")?)),
            "--modules" => parsed.modules = Some(PathBuf::from(value("--modules")?)),
            "--crate" => parsed.crates.push(value("--crate")?),
            "--function" => {
                parsed.functions.insert(value("--function")?);
            }
            "--drop-revision" => parsed.drop_revision = Some(value("--drop-revision")?),
            "--revision" => parsed.revision = Some(value("--revision")?),
            "--check" => parsed.check = true,
            "--regenerate" => parsed.regenerate = true,
            "--verify-contracts" => parsed.verify_contracts = true,
            "--track-tip" => parsed.track_tip = Some(value("--track-tip")?),
            "--untrack-tip" => parsed.untrack_tip = true,
            "--repository-name" => parsed.repository_name = Some(value("--repository-name")?),
            "--repository-url" => parsed.repository_url = Some(value("--repository-url")?),
            "--release-label" => parsed.release_label = Some(value("--release-label")?),
            "--bounds" => parsed.bounds = true,
            "--outer" => parsed.outer = true,
            other => return Err(format!("unknown argument `{other}`")),
        }
    }
    let tip_details = parsed.repository_name.is_some()
        || parsed.repository_url.is_some()
        || parsed.release_label.is_some();
    if tip_details && parsed.track_tip.is_none() {
        return Err(
            "--repository-name, --repository-url and --release-label go with --track-tip".into(),
        );
    }
    if parsed.track_tip.is_some() || parsed.untrack_tip {
        if parsed.functions.is_empty() {
            return Err("--track-tip and --untrack-tip need --function".into());
        }
        if parsed.track_tip.is_some() && parsed.untrack_tip {
            return Err("--track-tip and --untrack-tip exclude each other".into());
        }
        if parsed.check
            || parsed.regenerate
            || parsed.revision.is_some()
            || parsed.verify_contracts
            || parsed.drop_revision.is_some()
        {
            return Err(
                "--track-tip and --untrack-tip take no --check, --regenerate, --revision, \
                 --verify-contracts or --drop-revision"
                    .into(),
            );
        }
    }
    let reads_source = parsed.check || parsed.regenerate || parsed.revision.is_some();
    if parsed.verify_contracts {
        if parsed.source_root.is_some()
            || parsed.modules.is_some()
            || reads_source
            || parsed.drop_revision.is_some()
            || !parsed.functions.is_empty()
        {
            return Err("--verify-contracts takes only --load-path and --crate".into());
        }
    } else if parsed.drop_revision.is_some() {
        if parsed.source_root.is_some() || reads_source {
            return Err(
                "--drop-revision reads no source and takes no --source-root, --check, \
                        --regenerate or --revision"
                    .into(),
            );
        }
    } else if parsed.source_root.is_none() {
        return Err("--source-root is required".into());
    }
    if parsed.regenerate && (parsed.check || !parsed.functions.is_empty()) {
        return Err("--regenerate rebuilds whole locks: no --check or --function".into());
    }
    if parsed.revision.is_some() && !parsed.check {
        return Err("--revision is for --check; a pin records the checkout's HEAD".into());
    }
    if parsed.modules.is_some() && (parsed.load_path.is_some() || !parsed.crates.is_empty()) {
        return Err("--modules is not combined with --load-path or --crate".into());
    }
    if parsed.outer && (parsed.bounds || parsed.modules.is_some() || parsed.verify_contracts) {
        return Err(
            "--outer pins the outer views of load-path crates: no --bounds, --modules or \
             --verify-contracts (which verifies every kind of lock)"
                .into(),
        );
    }
    if parsed.bounds && (parsed.modules.is_some() || parsed.verify_contracts) {
        return Err(
            "--bounds pins the bounds views of load-path crates: no --modules or \
             --verify-contracts (which verifies both kinds of lock)"
                .into(),
        );
    }
    Ok(parsed)
}

/// One lock target: the declarations to pin and where its lock lives.
struct Target {
    owner: String,
    /// How the target is named in output: the owner, and `(bounds)` for a
    /// bounds lock (card CHUNKIO-META-LEN-BOUNDS-CONTRACTS).
    label: String,
    lock: PathBuf,
    /// Function to (module name, contract digest, mode).
    declared: BTreeMap<String, (String, String, RevisionMode)>,
    /// Card SUPPLIED-OWNED-RETURN-CODE-HANDOFF (rule 13l): function to the
    /// `@` names its declaration spells, whose enumerations join its
    /// identity.
    c_names: BTreeMap<String, BTreeSet<String>>,
}

impl Target {
    fn resource(
        owner: String,
        lock: PathBuf,
        contracts: &ResourceContracts,
    ) -> Result<Self, String> {
        Ok(Self {
            label: owner.clone(),
            owner,
            lock,
            declared: resource_declarations(contracts)?,
            c_names: qcore_contract::contract_revision::declaration_c_names(
                contracts.source_assumptions(),
            )?,
        })
    }
}

/// Card CHUNKIO-META-LEN-BOUNDS-CONTRACTS: the bounds lock targets, one per
/// crate (every crate with a non-empty bounds view, or the named ones that
/// have one when `all` is false and `crates` names them).
fn bounds_targets(
    load_path: &Path,
    crates: &[String],
    strict: bool,
) -> Result<Vec<Target>, String> {
    let load_path = &load_path
        .canonicalize()
        .map_err(|error| format!("{}: {error}", load_path.display()))?;
    let manifest = load_contract_load_path_manifest(load_path)?;
    let names = if crates.is_empty() {
        manifest
            .bounds_views
            .iter()
            .filter(|(_, view)| !view.is_empty())
            .filter(|(name, _)| {
                manifest
                    .extension
                    .as_ref()
                    .is_none_or(|extension| extension.user_crates.contains(*name))
            })
            .map(|(name, _)| name.clone())
            .collect::<Vec<_>>()
    } else {
        crates.to_vec()
    };
    let mut targets = Vec::new();
    for name in names {
        let root = manifest
            .contract_roots
            .get(&name)
            .and_then(|contracts| contracts.parent())
            .ok_or_else(|| format!("`{name}` is not a crate on {}", load_path.display()))?;
        let Some(view) = manifest
            .bounds_views
            .get(&name)
            .filter(|view| !view.is_empty())
        else {
            if strict {
                return Err(format!("`{name}` declares no bounds view to pin"));
            }
            continue;
        };
        let single = qcore_contract::load_path_manifest::LoadPathManifest {
            bounds_views: BTreeMap::from([(name.clone(), view.clone())]),
            ..manifest.clone()
        };
        let (contracts, _) = crate::bounds_selection::load_views(&single)?;
        let declared = contracts
            .functions
            .iter()
            .map(|(function, contract)| {
                (
                    function.clone(),
                    (
                        module_name(&contract.module),
                        contract.text_sha256.clone(),
                        RevisionMode::Pinned,
                    ),
                )
            })
            .collect();
        targets.push(Target {
            label: format!("{name} (bounds)"),
            owner: name.clone(),
            lock: root.join(crate::bounds_selection::BOUNDS_LOCK_FILE),
            declared,
            c_names: BTreeMap::new(),
        });
    }
    Ok(targets)
}

/// Card OUTER-CONTRACT: the outer contract lock targets, one per crate with a
/// non-empty outer view (or the named ones that have one when `strict`).
fn outer_targets(load_path: &Path, crates: &[String], strict: bool) -> Result<Vec<Target>, String> {
    let load_path = &load_path
        .canonicalize()
        .map_err(|error| format!("{}: {error}", load_path.display()))?;
    let manifest = load_contract_load_path_manifest(load_path)?;
    let names = if crates.is_empty() {
        manifest
            .outer_views
            .iter()
            .filter(|(_, view)| !view.is_empty())
            .filter(|(name, _)| {
                manifest
                    .extension
                    .as_ref()
                    .is_none_or(|extension| extension.user_crates.contains(*name))
            })
            .map(|(name, _)| name.clone())
            .collect::<Vec<_>>()
    } else {
        crates.to_vec()
    };
    let mut targets = Vec::new();
    for name in names {
        let root = manifest
            .contract_roots
            .get(&name)
            .and_then(|contracts| contracts.parent())
            .ok_or_else(|| format!("`{name}` is not a crate on {}", load_path.display()))?;
        let Some(view) = manifest
            .outer_views
            .get(&name)
            .filter(|view| !view.is_empty())
        else {
            if strict {
                return Err(format!("`{name}` declares no outer view to pin"));
            }
            continue;
        };
        let single = qcore_contract::load_path_manifest::LoadPathManifest {
            outer_views: BTreeMap::from([(name.clone(), view.clone())]),
            ..manifest.clone()
        };
        let (contracts, _) = crate::outer_selection::load_views(&single)?;
        let declared = contracts
            .loaders
            .iter()
            .map(|(function, loader)| {
                (
                    function.clone(),
                    (
                        module_name(&loader.module),
                        loader.text_sha256.clone(),
                        RevisionMode::Pinned,
                    ),
                )
            })
            .collect();
        targets.push(Target {
            label: format!("{name} (outer)"),
            owner: name.clone(),
            lock: root.join(crate::outer_selection::OUTER_LOCK_FILE),
            declared,
            c_names: BTreeMap::new(),
        });
    }
    Ok(targets)
}

fn crate_targets(load_path: &Path, crates: &[String]) -> Result<Vec<Target>, String> {
    let load_path = &load_path
        .canonicalize()
        .map_err(|error| format!("{}: {error}", load_path.display()))?;
    let manifest = load_contract_load_path_manifest(load_path)?;
    let names = if crates.is_empty() {
        // Card W45-USER-CONTRACT-PLUGINS: on a user load path only the
        // user's own crates are pinned; the extended load path's crates keep
        // the locks they ship with.
        manifest
            .resource_views
            .iter()
            .filter(|(_, view)| !view.is_empty())
            .filter(|(name, _)| {
                manifest
                    .extension
                    .as_ref()
                    .is_none_or(|extension| extension.user_crates.contains(*name))
            })
            .map(|(name, _)| name.clone())
            .collect::<Vec<_>>()
    } else {
        crates.to_vec()
    };
    let mut targets = Vec::new();
    for name in names {
        let root = manifest
            .contract_roots
            .get(&name)
            .and_then(|contracts| contracts.parent())
            .ok_or_else(|| format!("`{name}` is not a crate on {}", load_path.display()))?;
        if manifest.resource_views.get(&name).is_none_or(Vec::is_empty) {
            return Err(format!("`{name}` declares no resource view to pin"));
        }
        let resolved = resolve_providers(
            load_path,
            std::slice::from_ref(&name),
            false,
            SelectionSource::CommandLine,
        )?;
        let contracts = ContractInput::providers(&resolved, None).load()?;
        targets.push(Target::resource(
            name.clone(),
            root.join(LOCK_FILE),
            &contracts,
        )?);
    }
    Ok(targets)
}

fn module_targets(directory: &Path) -> Result<Vec<Target>, String> {
    let mut paths = Vec::new();
    let mut entries = fs::read_dir(directory)
        .map_err(|error| format!("{}: {error}", directory.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("{}: {error}", directory.display()))?;
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        if path
            .extension()
            .is_some_and(|ext| ext == "qcore" || ext == "qv")
            && path.is_file()
        {
            paths.push(path);
        }
    }
    if paths.is_empty() {
        return Err(format!(
            "{} holds no .qcore or .qv module",
            directory.display()
        ));
    }
    let contracts = qcore_contract::contract_loader::load_resource_contract_files(&paths)?;
    let owner = directory
        .canonicalize()
        .ok()
        .and_then(|path| {
            path.file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| directory.display().to_string());
    Ok(vec![Target::resource(
        owner,
        directory.join(LOCK_FILE),
        &contracts,
    )?])
}

/// The module name a lock records: the crate-relative module path, or the
/// key as retained.
fn module_name(key: &str) -> String {
    if let Some((_, module)) = key.split_once(':').filter(|_| !key.starts_with('/')) {
        return module.to_owned();
    }
    let path = Path::new(key);
    let components = path
        .components()
        .map(|part| part.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    match components.iter().rposition(|part| part == "contracts") {
        Some(position) => components[position + 1..].join("/"),
        None => path.file_name().map_or_else(
            || key.to_owned(),
            |name| name.to_string_lossy().into_owned(),
        ),
    }
}

/// The current declarations of a resource environment: function to (module
/// name, contract digest, mode).
fn resource_declarations(
    contracts: &ResourceContracts,
) -> Result<BTreeMap<String, (String, String, RevisionMode)>, String> {
    Ok(
        qcore_contract::contract_revision::declaration_digests(contracts.source_assumptions())?
            .into_iter()
            .map(|(name, (module, digest))| {
                let mode = contracts.revision_mode(&name);
                (name, (module_name(&module), digest, mode))
            })
            .collect(),
    )
}

/// The current declarations of a target.
#[allow(clippy::unnecessary_wraps)]
fn declarations(
    target: &Target,
) -> Result<BTreeMap<String, (String, String, RevisionMode)>, String> {
    Ok(target.declared.clone())
}

/// An empty lock for a target.
fn empty_lock(target: &Target) -> ContractLock {
    ContractLock {
        lock_version: LOCK_VERSION,
        generated_by: GENERATED_BY.into(),
        note: NOTE.into(),
        owner: target.owner.clone(),
        normalisation: NORMALISATION.into(),
        pinned: Vec::new(),
        flexible: Vec::new(),
        tip: Vec::new(),
    }
}

/// Add or refresh the identities `tree` gives: every current declaration (or
/// only `functions`) is recorded at the tree's source. Identities reviewed at
/// other sources are kept, unless the contract text changed, which voids
/// them. Entries no longer declared go (only on a full update). Returns the
/// new lock and one line per changed entry.
fn update(
    existing: Option<&ContractLock>,
    target: &Target,
    tree: &SourceTree,
    functions: &BTreeSet<String>,
    untrack: bool,
) -> Result<(ContractLock, Vec<String>), String> {
    let declared = declarations(target)?;
    let known = declared
        .keys()
        .chain(existing.iter().flat_map(|lock| {
            lock.pinned
                .iter()
                .map(|e| &e.function)
                .chain(lock.flexible.iter().map(|e| &e.function))
                .chain(lock.tip.iter().map(|e| &e.function))
        }))
        .collect::<BTreeSet<_>>();
    if let Some(unknown) = functions.iter().find(|name| !known.contains(name)) {
        return Err(format!(
            "`{unknown}` is declared by no contract of {}",
            target.label
        ));
    }
    // Card CONTRACT-PIN-TIP-MODE: `--untrack-tip` names tip entries only.
    if untrack
        && let Some(name) = functions.iter().find(|name| {
            !existing.is_some_and(|lock| lock.tip.iter().any(|e| &e.function == *name))
        })
    {
        return Err(format!("`{name}` does not track a tip in {}", target.label));
    }
    let source = ReviewedSource::of(tree);
    let chosen = |name: &str| functions.is_empty() || functions.contains(name);
    let pinned_names = declared
        .iter()
        .filter(|(name, (_, _, mode))| *mode == RevisionMode::Pinned && chosen(name))
        .map(|(name, _)| name.clone())
        .collect::<BTreeSet<_>>();
    let sites = tree.sites(&crate::c_identity::identity_names(
        &pinned_names,
        &target.c_names,
    ));
    let old = existing.cloned().unwrap_or_else(|| empty_lock(target));
    let mut lock = ContractLock {
        pinned: Vec::new(),
        flexible: Vec::new(),
        ..empty_lock(target)
    };
    let mut lines = Vec::new();
    let old_pinned = |name: &str| old.pinned.iter().find(|e| e.function == name);
    let old_flexible = |name: &str| old.flexible.iter().find(|e| e.function == name);
    let old_tip = |name: &str| old.tip.iter().find(|e| e.function == name);
    for (name, (module, digest, mode)) in &declared {
        if !chosen(name) {
            // Untouched: kept exactly as the lock has it.
            if let Some(entry) = old_pinned(name) {
                lock.pinned.push(entry.clone());
            }
            if let Some(entry) = old_flexible(name) {
                lock.flexible.push(entry.clone());
            }
            if let Some(entry) = old_tip(name) {
                lock.tip.push(entry.clone());
            }
            continue;
        }
        // Card CONTRACT-PIN-TIP-MODE: a tip entry is written only by
        // `--track-tip` and left only by `--untrack-tip`; every other run
        // keeps it as written.
        if let (RevisionMode::Pinned, Some(entry)) = (mode, old_tip(name)) {
            if untrack {
                lines.push(format!(
                    "~ {name}: no longer tracks the tip `{}` of `{}`; pinned at {}",
                    entry.reference,
                    entry.repository.name,
                    ReviewedSource::of(tree).label()
                ));
            } else {
                if entry.contract_sha256 != *digest {
                    lines.push(format!(
                        "! {name}: contract text changed; its tip entry stays out of force until \
                         `pin --track-tip` records it again after review"
                    ));
                }
                lock.tip.push(entry.clone());
                continue;
            }
        }
        match mode {
            RevisionMode::Pinned => {
                let mut entry = match old_pinned(name) {
                    Some(entry) if entry.contract_sha256 == *digest => entry.clone(),
                    Some(entry) => {
                        lines.push(format!(
                            "! {name}: contract text changed; {} earlier identit(ies) voided",
                            entry.identities.len()
                        ));
                        PinnedEntry {
                            identities: Vec::new(),
                            ..entry.clone()
                        }
                    }
                    None => {
                        lines.push(format!("+ {name}"));
                        PinnedEntry {
                            function: name.clone(),
                            module: module.clone(),
                            contract_sha256: digest.clone(),
                            identities: Vec::new(),
                        }
                    }
                };
                let before = entry.identities.clone();
                entry.module.clone_from(module);
                entry.contract_sha256.clone_from(digest);
                // Card SUPPLIED-OWNED-RETURN-CODE-HANDOFF (rule 13l): the
                // enumerations of its `@` names join the identity; an `@`
                // name the tree does not declare refuses the pin.
                let identity = crate::c_identity::identity_sites(&sites, name, &target.c_names)
                    .map_err(|why| format!("`{name}`: {why} at {}", source.label()))?;
                entry.record(&source, identity);
                if before != entry.identities && old_pinned(name).is_some() {
                    let joined = entry
                        .identities
                        .iter()
                        .find(|identity| identity.reviewed_at.contains(&source))
                        .map(|identity| {
                            identity
                                .reviewed_at
                                .iter()
                                .filter(|reviewed| **reviewed != source)
                                .map(ReviewedSource::label)
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    lines.push(if joined.is_empty() {
                        format!("~ {name}: new identity at {}", source.label())
                    } else {
                        format!(
                            "~ {name}: {} recorded beside {}",
                            source.label(),
                            joined.join(", ")
                        )
                    });
                }
                lock.pinned.push(entry);
            }
            RevisionMode::Flexible => {
                let entry = FlexibleEntry {
                    function: name.clone(),
                    module: module.clone(),
                    contract_sha256: digest.clone(),
                };
                if old_flexible(name) != Some(&entry) {
                    lines.push(format!("~ {name}: flexible"));
                }
                lock.flexible.push(entry);
            }
        }
    }
    for entry in &old.pinned {
        if !declared.contains_key(&entry.function) {
            if functions.is_empty() {
                lines.push(format!("- {}", entry.function));
            } else {
                lock.pinned.push(entry.clone());
            }
        }
    }
    for entry in &old.flexible {
        if !declared.contains_key(&entry.function) {
            if functions.is_empty() {
                lines.push(format!("- {}", entry.function));
            } else {
                lock.flexible.push(entry.clone());
            }
        }
    }
    for entry in &old.tip {
        if !declared.contains_key(&entry.function) {
            if functions.is_empty() {
                lines.push(format!("- {}", entry.function));
            } else {
                lock.tip.push(entry.clone());
            }
        }
    }
    lock.pinned.sort_by(|a, b| a.function.cmp(&b.function));
    lock.flexible.sort_by(|a, b| a.function.cmp(&b.function));
    lock.tip.sort_by(|a, b| a.function.cmp(&b.function));
    Ok((lock, lines))
}

/// Card CONTRACT-PIN-TIP-MODE: `--track-tip REF`: record a tip entry for
/// each of `functions` (pinned declarations of the target), reviewed at
/// REF's current commit in the repository at `root`, replacing any reviewed
/// entry. Returns the lock, one line per change and the entries recorded.
fn track(
    existing: Option<&ContractLock>,
    target: &Target,
    root: &Path,
    arguments: &Arguments,
) -> Result<(ContractLock, Vec<String>, Vec<TipEntry>), String> {
    let reference = arguments.track_tip.as_deref().expect("parsed");
    let functions = &arguments.functions;
    let declared = declarations(target)?;
    for name in functions {
        match declared.get(name) {
            Some((_, _, RevisionMode::Pinned)) => {}
            Some(_) => {
                return Err(format!(
                    "`{name}` is flexible: only a pinned contract tracks a tip"
                ));
            }
            None => {
                return Err(format!(
                    "`{name}` is declared by no contract of {}",
                    target.label
                ));
            }
        }
    }
    let root = root
        .canonicalize()
        .map_err(|error| format!("{}: {error}", root.display()))?;
    if !is_repository(&root) {
        return Err(format!(
            "{} is not the top of a Git repository",
            root.display()
        ));
    }
    let full = if reference.starts_with("refs/") {
        reference.to_owned()
    } else {
        format!("refs/heads/{reference}")
    };
    let commit = match git_output(
        &root,
        &[
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("{full}^{{commit}}"),
        ],
    )? {
        (0, commit) if !commit.is_empty() => commit,
        _ => return Err(format!("`{full}` names no commit in {}", root.display())),
    };
    let name = match &arguments.repository_name {
        Some(name) => name.clone(),
        None => root
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default(),
    };
    if !valid_name(&name) {
        return Err(format!(
            "repository name `{name}` is not letters, digits, `.`, `_` and `-`; pass \
             --repository-name"
        ));
    }
    let url = match &arguments.repository_url {
        Some(url) => Some(url.clone()),
        None => match git_output(&root, &["remote", "get-url", "origin"])? {
            (0, url) if !url.is_empty() => Some(url),
            _ => None,
        },
    };
    let scan = SourceTree::read_sites_at(&root, &commit, functions)?;
    let mut lock = existing.cloned().unwrap_or_else(|| empty_lock(target));
    let mut lines = Vec::new();
    let mut recorded = Vec::new();
    for function in functions {
        let sites = scan.sites.get(function).cloned().unwrap_or_default();
        if sites.is_empty() {
            return Err(format!(
                "`{full}` at {commit} has no site of `{function}`: nothing to track"
            ));
        }
        let (module, digest, _) = &declared[function];
        if let Some(position) = lock.pinned.iter().position(|e| &e.function == function) {
            let dropped = lock.pinned.remove(position);
            lines.push(format!(
                "~ {function}: now tracks the tip `{full}` of `{name}` ({} reviewed identit(ies) \
                 dropped)",
                dropped.identities.len()
            ));
        } else if lock.tip.iter().any(|e| &e.function == function) {
            lines.push(format!(
                "~ {function}: tip `{full}` of `{name}` recorded again at {commit}"
            ));
        } else {
            lines.push(format!("+ {function}: tracks the tip `{full}` of `{name}`"));
        }
        lock.tip.retain(|e| &e.function != function);
        let entry = TipEntry {
            function: function.clone(),
            module: module.clone(),
            contract_sha256: digest.clone(),
            repository: TipRepository {
                name: name.clone(),
                url: url.clone(),
                path: Some(root.display().to_string()),
            },
            reference: full.clone(),
            release_label: arguments.release_label.clone(),
            reviewed: TipReview {
                commit: commit.clone(),
                sites,
            },
        };
        entry.validate()?;
        lock.tip.push(entry.clone());
        recorded.push(entry);
    }
    lock.tip.sort_by(|a, b| a.function.cmp(&b.function));
    Ok((lock, lines, recorded))
}

/// Card CONTRACT-PIN-TIP-MODE: the notice of every tip entry of `lock` at the
/// tree's source, as an audit of that source would decide it.
fn tip_notices(
    lock: &ContractLock,
    target: &Target,
    tree: &SourceTree,
    tips: &mut TipResolver,
) -> Result<Vec<String>, String> {
    if lock.tip.is_empty() {
        return Ok(Vec::new());
    }
    let declared = declarations(target)?;
    let names = lock
        .tip
        .iter()
        .map(|e| e.function.clone())
        .collect::<BTreeSet<_>>();
    let sites = tree.sites(&crate::c_identity::identity_names(&names, &target.c_names));
    let mut lines = Vec::new();
    for entry in &lock.tip {
        let Some((module, digest, _)) = declared.get(&entry.function) else {
            continue;
        };
        let found = crate::c_identity::identity_sites(&sites, &entry.function, &target.c_names)
            .unwrap_or_else(|_| sites.get(&entry.function).cloned().unwrap_or_default());
        let standing = pin_standing(Some(lock), &entry.function, digest, &found, module, tips);
        if let Some(notice) = standing.notice(&entry.function) {
            lines.push(notice.line());
        }
    }
    Ok(lines)
}

/// Remove the named source from every identity (of `functions` only, when
/// given). Returns the lock, the number of identities touched and the sources
/// the spelling named.
fn drop_revision(
    lock: &ContractLock,
    spelling: &str,
    functions: &BTreeSet<String>,
) -> (ContractLock, usize, BTreeSet<ReviewedSource>) {
    let named = lock
        .reviewed_sources()
        .into_iter()
        .filter(|source| source.is_named_by(spelling))
        .collect::<BTreeSet<_>>();
    let mut lock = lock.clone();
    let mut touched = 0;
    for entry in &mut lock.pinned {
        if !functions.is_empty() && !functions.contains(&entry.function) {
            continue;
        }
        for source in &named {
            if entry.drop_source(source) {
                touched += 1;
            }
        }
    }
    (lock, touched, named)
}

/// Whether a lock lists exactly the current declarations, modes and contract
/// digests (sites are not compared: no source is read).
fn contracts_agree(
    lock: &ContractLock,
    declared: &BTreeMap<String, (String, String, RevisionMode)>,
) -> Vec<String> {
    let mut locked = BTreeMap::new();
    for entry in &lock.pinned {
        locked.insert(
            entry.function.clone(),
            (RevisionMode::Pinned, &entry.contract_sha256),
        );
    }
    for entry in &lock.flexible {
        locked.insert(
            entry.function.clone(),
            (RevisionMode::Flexible, &entry.contract_sha256),
        );
    }
    // Card CONTRACT-PIN-TIP-MODE: a tip entry is a pinned declaration.
    for entry in &lock.tip {
        locked.insert(
            entry.function.clone(),
            (RevisionMode::Pinned, &entry.contract_sha256),
        );
    }
    let mut lines = Vec::new();
    for (name, (_, digest, mode)) in declared {
        match locked.get(name) {
            None => lines.push(format!("`{name}` ({}) is not in the lock", mode.as_str())),
            Some((locked_mode, _)) if locked_mode != mode => lines.push(format!(
                "`{name}` is {} but the lock records it {}",
                mode.as_str(),
                locked_mode.as_str()
            )),
            Some((_, locked_digest)) if *locked_digest != digest => lines.push(format!(
                "`{name}`'s contract text changed since it was pinned"
            )),
            Some(_) => {}
        }
    }
    for name in locked.keys().filter(|name| !declared.contains_key(*name)) {
        lines.push(format!("`{name}` is in the lock but no longer declared"));
    }
    lines
}

/// `--check`: how every current declaration stands at the tree's source.
/// Returns whether every pinned declaration is recorded at that source and
/// every flexible one is listed.
fn check(
    target: &Target,
    lock: Option<&ContractLock>,
    tree: &SourceTree,
    functions: &BTreeSet<String>,
) -> Result<bool, String> {
    let declared = declarations(target)?;
    let source = ReviewedSource::of(tree);
    let chosen = |name: &str| functions.is_empty() || functions.contains(name);
    let pinned_names = declared
        .iter()
        .filter(|(name, (_, _, mode))| *mode == RevisionMode::Pinned && chosen(name))
        .map(|(name, _)| name.clone())
        .collect::<BTreeSet<_>>();
    let sites = tree.sites(&crate::c_identity::identity_names(
        &pinned_names,
        &target.c_names,
    ));
    let mut tips = TipResolver::from_environment()?;
    let (mut recorded, mut via_other, mut via_tip, mut out, mut stale) = (0, 0, 0, 0, 0);
    let mut lines = Vec::new();
    for (name, (module, digest, mode)) in &declared {
        if !chosen(name) {
            continue;
        }
        match mode {
            RevisionMode::Pinned => {
                // An unresolvable `@` name leaves the identity without its
                // enumeration: a mismatch, never in force.
                let found = crate::c_identity::identity_sites(&sites, name, &target.c_names)
                    .unwrap_or_else(|_| sites.get(name).cloned().unwrap_or_default());
                let standing = pin_standing(lock, name, digest, &found, module, &mut tips);
                let notice = standing.notice(name);
                match standing {
                    // Card CONTRACT-PIN-TIP-MODE: in force through the tip.
                    PinStanding::InForceAtTip { tip } => {
                        via_tip += 1;
                        lines.push(format!(
                            "t {name}: in force through the tip `{}` of `{}` at {}",
                            tip.reference,
                            tip.repository.name,
                            tip.resolved_commit.as_deref().unwrap_or_default()
                        ));
                    }
                    PinStanding::InForce { reviewed_at } if reviewed_at.contains(&source) => {
                        recorded += 1;
                    }
                    PinStanding::InForce { reviewed_at } => {
                        via_other += 1;
                        lines.push(format!(
                            "= {name}: in force through the identity reviewed at {}; {} not recorded",
                            reviewed_at
                                .iter()
                                .map(ReviewedSource::label)
                                .collect::<Vec<_>>()
                                .join(", "),
                            source.label()
                        ));
                    }
                    PinStanding::Mismatch { reason, detail, .. } => {
                        out += 1;
                        lines.push(format!(
                            "x {name}: not in force ({}): {detail}",
                            reason.as_str()
                        ));
                    }
                }
                if let Some(notice) = notice {
                    lines.push(notice.line());
                }
            }
            RevisionMode::Flexible => {
                let listed = lock.is_some_and(|lock| {
                    lock.flexible
                        .iter()
                        .any(|e| e.function == *name && e.contract_sha256 == *digest)
                });
                if !listed {
                    stale += 1;
                    lines.push(format!(
                        "~ {name}: flexible, not listed as such in the lock"
                    ));
                }
            }
        }
    }
    if functions.is_empty() {
        for entry in lock.iter().flat_map(|lock| {
            lock.pinned
                .iter()
                .map(|e| &e.function)
                .chain(lock.flexible.iter().map(|e| &e.function))
                .chain(lock.tip.iter().map(|e| &e.function))
        }) {
            if !declared.contains_key(entry) {
                stale += 1;
                lines.push(format!("- {entry}: in the lock but no longer declared"));
            }
        }
    }
    // Card CONTRACT-PIN-TIP-MODE: the tip count only when a tip entry exists.
    let tip_clause = if lock.is_some_and(|lock| !lock.tip.is_empty()) {
        format!("{via_tip} in force through the tip of an unreleased branch, ")
    } else {
        String::new()
    };
    println!(
        "pin: {} at {}: {recorded} recorded, {via_other} in force through another revision, \
         {tip_clause}{out} not in force, {stale} stale ({})",
        target.label,
        source.label(),
        target.lock.display()
    );
    for line in lines {
        println!("  {line}");
    }
    Ok(via_other == 0 && out == 0 && stale == 0)
}

fn read_existing(target: &Target) -> Result<Option<ContractLock>, String> {
    if target.lock.is_file() {
        ContractLock::read(&target.lock).map(Some)
    } else {
        Ok(None)
    }
}

fn run(arguments: Arguments) -> Result<bool, String> {
    let targets = match &arguments.modules {
        Some(directory) => module_targets(directory)?,
        None => {
            let load_path = arguments
                .load_path
                .as_deref()
                .unwrap_or_else(|| Path::new(DEFAULT_LOAD_PATH));
            // Card CHUNKIO-META-LEN-BOUNDS-CONTRACTS: `--verify-contracts`
            // verifies both kinds of lock; `--bounds` selects the bounds locks.
            if arguments.verify_contracts {
                let mut targets = crate_targets(load_path, &arguments.crates)?;
                targets.extend(bounds_targets(load_path, &arguments.crates, false)?);
                // Card OUTER-CONTRACT: and the outer locks.
                targets.extend(outer_targets(load_path, &arguments.crates, false)?);
                targets
            } else if arguments.outer {
                outer_targets(load_path, &arguments.crates, true)?
            } else if arguments.bounds {
                bounds_targets(load_path, &arguments.crates, true)?
            } else {
                crate_targets(load_path, &arguments.crates)?
            }
        }
    };
    if arguments.verify_contracts {
        let mut clean = true;
        let mut tips = TipResolver::from_environment()?;
        for target in &targets {
            let declared = declarations(target)?;
            let existing = read_existing(target)?;
            let problems = match &existing {
                Some(lock) => contracts_agree(lock, &declared),
                None => vec![format!(
                    "no {}",
                    target
                        .lock
                        .file_name()
                        .map_or_else(|| LOCK_FILE.into(), |name| name.to_string_lossy())
                )],
            };
            if problems.is_empty() {
                let pinned = declared
                    .values()
                    .filter(|(_, _, mode)| *mode == RevisionMode::Pinned)
                    .count();
                // Card CONTRACT-PIN-TIP-MODE: tip entries are counted and
                // noticed (no audited source: info when the tip resolves on
                // this host, warning when it does not).
                let tip = existing.as_ref().map_or(&[][..], |lock| &lock.tip[..]);
                let tip_clause = if tip.is_empty() {
                    String::new()
                } else {
                    format!(
                        ", {} of them tracking the tip of an unreleased branch",
                        tip.len()
                    )
                };
                println!(
                    "pin: {}: lock lists every declaration ({} pinned{tip_clause}, {} flexible)",
                    target.label,
                    pinned,
                    declared.len() - pinned
                );
                for entry in tip {
                    let resolution = tips.resolve(entry);
                    println!(
                        "  {}",
                        notice(&entry.function, &resolution, NoticeStanding::NoSource).line()
                    );
                }
            } else {
                clean = false;
                for problem in problems {
                    println!("pin: {}: stale lock: {problem}", target.label);
                }
            }
        }
        if !clean {
            println!("pin: run `quirin-audit pin --source-root <reviewed checkout>` after review");
        }
        return Ok(clean);
    }
    if let Some(spelling) = &arguments.drop_revision {
        let mut named = BTreeSet::new();
        let mut writes = Vec::new();
        for target in &targets {
            let Some(lock) = read_existing(target)? else {
                continue;
            };
            let (dropped, touched, sources) = drop_revision(&lock, spelling, &arguments.functions);
            named.extend(sources);
            writes.push((target, dropped, touched));
        }
        if named.is_empty() {
            return Err(format!(
                "`{spelling}` names no reviewed source of the selected locks"
            ));
        }
        if named.len() > 1 {
            return Err(format!(
                "`{spelling}` names {} reviewed sources; give more of the revision",
                named.len()
            ));
        }
        for (target, lock, touched) in writes {
            if touched == 0 {
                continue;
            }
            fs::write(&target.lock, lock.to_text()?)
                .map_err(|error| format!("{}: {error}", target.lock.display()))?;
            println!(
                "pin: {}: dropped {} from {touched} entr(ies); wrote {}",
                target.label,
                named
                    .iter()
                    .next()
                    .map(ReviewedSource::label)
                    .unwrap_or_default(),
                target.lock.display()
            );
        }
        return Ok(true);
    }
    let root = arguments.source_root.as_deref().expect("parsed");
    if arguments.track_tip.is_some() {
        // Card CONTRACT-PIN-TIP-MODE: record tip entries for the named
        // functions, which some selected crate must declare.
        let mut tips = TipResolver::from_environment()?;
        let mut tracked = BTreeSet::new();
        for target in &targets {
            let declared = declarations(target)?;
            if !arguments.functions.iter().any(|f| declared.contains_key(f)) {
                continue;
            }
            let existing = read_existing(target)?;
            let (lock, lines, recorded) = track(existing.as_ref(), target, root, &arguments)?;
            fs::write(&target.lock, lock.to_text()?)
                .map_err(|error| format!("{}: {error}", target.lock.display()))?;
            println!(
                "pin: {}: {} pinned, {} tracking a tip, {} flexible; wrote {}",
                target.label,
                lock.pinned.len(),
                lock.tip.len(),
                lock.flexible.len(),
                target.lock.display()
            );
            for line in &lines {
                println!("  {line}");
            }
            for entry in &recorded {
                tracked.insert(entry.function.clone());
                let resolution = tips.resolve(entry);
                println!(
                    "  {}",
                    notice(&entry.function, &resolution, NoticeStanding::NoSource).line()
                );
            }
        }
        if let Some(missing) = arguments.functions.iter().find(|f| !tracked.contains(*f)) {
            return Err(format!(
                "`{missing}` is declared by no selected crate; nothing tracks it"
            ));
        }
        return Ok(true);
    }
    let tree = match &arguments.revision {
        Some(revision) => SourceTree::read_revision(root, revision)?,
        None => SourceTree::read(root)?,
    };
    let mut clean = true;
    let mut tips = TipResolver::from_environment()?;
    for target in &targets {
        if arguments.check {
            clean &= check(
                target,
                read_existing(target)?.as_ref(),
                &tree,
                &arguments.functions,
            )?;
            continue;
        }
        // A regeneration starts from nothing (and replaces an unreadable
        // lock, saying so); an update needs a lock it can read.
        let (existing, replaced) = if arguments.regenerate {
            let replaced = match read_existing(target) {
                Ok(Some(_)) => Some("replaces the lock from this source alone".to_owned()),
                Ok(None) => None,
                Err(error) => Some(format!("replaces an unreadable lock: {error}")),
            };
            (None, replaced)
        } else {
            (read_existing(target)?, None)
        };
        // Card CONTRACT-PIN-TIP-MODE: a regeneration keeps tip entries as
        // written (they name no revision of this source).
        let kept_tips = if arguments.regenerate {
            read_existing(target)
                .ok()
                .flatten()
                .map(|lock| lock.tip)
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        let regenerated_from = (!kept_tips.is_empty()).then(|| ContractLock {
            tip: kept_tips,
            ..empty_lock(target)
        });
        let (lock, mut lines) = update(
            existing.as_ref().or(regenerated_from.as_ref()),
            target,
            &tree,
            &arguments.functions,
            arguments.untrack_tip,
        )?;
        if let Some(replaced) = replaced {
            lines.insert(0, format!("! {replaced}"));
        }
        lines.extend(tip_notices(&lock, target, &tree, &mut tips)?);
        fs::write(&target.lock, lock.to_text()?)
            .map_err(|error| format!("{}: {error}", target.lock.display()))?;
        println!(
            "pin: {}: {} pinned, {} flexible; recorded at {}; wrote {}",
            target.label,
            lock.pinned.len(),
            lock.flexible.len(),
            ReviewedSource::of(&tree).label(),
            target.lock.display()
        );
        for line in &lines {
            println!("  {line}");
        }
    }
    Ok(clean)
}

/// Entry point of `quirin-audit pin`.
#[must_use]
pub fn main(arguments: Vec<String>) -> ExitCode {
    let parsed = match parse(arguments) {
        Ok(parsed) => parsed,
        Err(error) => {
            eprintln!("{error}\n{USAGE}");
            return ExitCode::FAILURE;
        }
    };
    match run(parsed) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(1),
        Err(error) => {
            eprintln!("pin refused: {error}");
            ExitCode::from(2)
        }
    }
}
