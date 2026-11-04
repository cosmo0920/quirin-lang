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
//! tip entries in the bounds locks exactly as in the resource locks. Card
//! **CELL-GUARD-RESULT-RANGE-CHECKED**: a C library result set (`requires
//! bounds_revision(flexible);`, C09.4) is the one flexible bounds declaration
//! and is listed in its lock's `flexible` entries.
//!
//! Card **OUTER-CONTRACT** (rule 31): `--outer` selects the outer contract
//! locks (`outer-contracts.lock`, one per crate with a non-empty outer view,
//! [`crate::outer_selection`]), exactly as `--bounds` selects the bounds
//! locks; each entry pins a loader declaration to the loader's body.
//! `--verify-contracts` verifies all three kinds of lock.
//!
//! Card **CONTRACT-SUPERSESSION** (contract spec §C06.6, user decision 9a):
//! every lock is written in format version 4, and every entry a run adds or
//! changes carries the run's **pin stamp** `pinned_at` (`--pinned-at STAMP`,
//! `YYYY-MM-DDTHH:MM:SSZ`; by default the clock at the run). A re-pin carries
//! a later stamp than the entry it replaces: an explicit stamp that is not
//! later refuses, and a default stamp that is not later (two runs in one
//! second, or a clock behind the lock) is moved to one second after the
//! replaced stamp. An entry a run leaves unchanged keeps its stamp. Card
//! **PIN-REGENERATE-STAMP-MONOTONICITY**: the replaced entry is the
//! function's entry in the lock on disk before the run, of whatever kind (a
//! revision-mode change replaces the entry of the other kind), and under
//! `--regenerate` too, which otherwise starts from no lock. Every lock of a
//! run is computed before any is written, so a refusal writes none.
//! `--stamp-only` (with `--source-root`) is the one-time migration: it stamps
//! every unstamped entry whose contract text is current, changes no identity,
//! and ties each lock to the checkout's `HEAD` (`fluent_bit_revision`).
//! `--verify-contracts` also requires version 4, a valid stamp on every entry
//! and a revision tie, and checks the lock-set window ([`crate::pin_window`])
//! when its manifest exists. `--window-create REV`, `--window-add REV`,
//! `--window-rotate REV`, `--window-retire` and `--window-check` maintain the
//! window of at most three lock sets.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use qcore_contract::load_path_manifest::load_contract_load_path_manifest;
use qcore_schemas::contract::ResourceContracts;
use qcore_schemas::contract_revision::RevisionMode;

use crate::c_identity::{NORMALISATION, SourceTree};
use crate::contract_revision::{
    ContractLock, FlexibleEntry, LOCK_FILE, LOCK_VERSION, PinStanding, PinnedEntry, RestedOn,
    ResultDerivation, ReviewedSource, pin_standing, with_helper_sites,
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
pin --source-root DIR --function NAME... --untrack-tip | pin --source-root DIR --stamp-only \
[--pinned-at STAMP] | pin --window-create REV | --window-add REV | --window-rotate REV | \
--window-retire | --window-check [--load-path FILE] (`--bounds` or `--outer` with a pin, \
--check, --regenerate, --stamp-only or --drop-revision selects the bounds or the outer locks; \
`--pinned-at STAMP` sets the review stamp of a pin)";

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
    /// Card CELL-GUARD-RESULT-RANGE-CHECKED (rule 38e): derive every chosen
    /// pinned `nonnull_result_range` from the configured unit's body chain
    /// (`--compile-database`) and record the derivation.
    derive_results: bool,
    /// The compile database the derivation reads the configured unit from.
    compile_database: Option<PathBuf>,
    /// Card CONTRACT-SUPERSESSION: the review stamp of this run.
    pinned_at: Option<String>,
    /// Card CONTRACT-SUPERSESSION: stamp unstamped entries, nothing else.
    stamp_only: bool,
    /// Card CONTRACT-SUPERSESSION: one window operation.
    window: Option<WindowOperation>,
}

/// Card CONTRACT-SUPERSESSION: a lock-set window operation.
#[derive(Clone, Debug, PartialEq, Eq)]
enum WindowOperation {
    Create(String),
    Add(String),
    Rotate(String),
    Retire,
    Check,
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
            "--derive-results" => parsed.derive_results = true,
            "--compile-database" => {
                parsed.compile_database = Some(PathBuf::from(value("--compile-database")?));
            }
            "--outer" => parsed.outer = true,
            "--pinned-at" => parsed.pinned_at = Some(value("--pinned-at")?),
            "--stamp-only" => parsed.stamp_only = true,
            "--window-create" | "--window-add" | "--window-rotate" | "--window-retire"
            | "--window-check" => {
                if parsed.window.is_some() {
                    return Err("one window operation at a time".into());
                }
                parsed.window = Some(match argument.as_str() {
                    "--window-create" => WindowOperation::Create(value("--window-create")?),
                    "--window-add" => WindowOperation::Add(value("--window-add")?),
                    "--window-rotate" => WindowOperation::Rotate(value("--window-rotate")?),
                    "--window-retire" => WindowOperation::Retire,
                    _ => WindowOperation::Check,
                });
            }
            other => return Err(format!("unknown argument `{other}`")),
        }
    }
    if let Some(stamp) = &parsed.pinned_at {
        qcore_schemas::contract_supersession::validate_stamp(stamp)?;
    }
    if parsed.window.is_some() {
        let only_load_path = Arguments {
            load_path: None,
            window: None,
            ..Arguments::default()
        };
        let rest = Arguments {
            load_path: None,
            window: None,
            ..clone_arguments(&parsed)
        };
        if !arguments_equal(&rest, &only_load_path) {
            return Err("a window operation takes only --load-path".into());
        }
        return Ok(parsed);
    }
    if parsed.stamp_only
        && (parsed.source_root.is_none()
            || parsed.check
            || parsed.regenerate
            || parsed.revision.is_some()
            || parsed.verify_contracts
            || parsed.drop_revision.is_some()
            || parsed.track_tip.is_some()
            || parsed.untrack_tip
            || !parsed.functions.is_empty())
    {
        return Err(
            "--stamp-only takes --source-root (the revision it ties the locks to) and no \
             --check, --regenerate, --revision, --verify-contracts, --drop-revision, \
             --track-tip, --untrack-tip or --function"
                .into(),
        );
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
    // Card CELL-GUARD-RESULT-RANGE-CHECKED: a derivation is a bounds pin of a
    // reviewed checkout, read through its configured compile database.
    if (parsed.derive_results || parsed.compile_database.is_some())
        && (!parsed.bounds
            || !parsed.derive_results
            || parsed.compile_database.is_none()
            || parsed.check
            || parsed.revision.is_some()
            || parsed.stamp_only
            || parsed.track_tip.is_some()
            || parsed.untrack_tip
            || parsed.drop_revision.is_some())
    {
        return Err(
            "--derive-results and --compile-database go together, with --bounds and a pin of              --source-root (no --check, --revision, --stamp-only, --track-tip, --untrack-tip or              --drop-revision)"
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

fn clone_arguments(a: &Arguments) -> Arguments {
    Arguments {
        source_root: a.source_root.clone(),
        load_path: a.load_path.clone(),
        modules: a.modules.clone(),
        crates: a.crates.clone(),
        functions: a.functions.clone(),
        check: a.check,
        verify_contracts: a.verify_contracts,
        regenerate: a.regenerate,
        drop_revision: a.drop_revision.clone(),
        revision: a.revision.clone(),
        track_tip: a.track_tip.clone(),
        untrack_tip: a.untrack_tip,
        repository_name: a.repository_name.clone(),
        repository_url: a.repository_url.clone(),
        release_label: a.release_label.clone(),
        bounds: a.bounds,
        outer: a.outer,
        derive_results: a.derive_results,
        compile_database: a.compile_database.clone(),
        pinned_at: a.pinned_at.clone(),
        stamp_only: a.stamp_only,
        window: a.window.clone(),
    }
}

fn arguments_equal(a: &Arguments, b: &Arguments) -> bool {
    a.source_root == b.source_root
        && a.load_path == b.load_path
        && a.modules == b.modules
        && a.crates == b.crates
        && a.functions == b.functions
        && a.check == b.check
        && a.verify_contracts == b.verify_contracts
        && a.regenerate == b.regenerate
        && a.drop_revision == b.drop_revision
        && a.revision == b.revision
        && a.track_tip == b.track_tip
        && a.untrack_tip == b.untrack_tip
        && a.repository_name == b.repository_name
        && a.repository_url == b.repository_url
        && a.release_label == b.release_label
        && a.bounds == b.bounds
        && a.outer == b.outer
        && a.derive_results == b.derive_results
        && a.compile_database == b.compile_database
        && a.pinned_at == b.pinned_at
        && a.stamp_only == b.stamp_only
        && a.window == b.window
}

/// Card CONTRACT-SUPERSESSION: the stamp a run writes into each entry it adds
/// or changes.
struct Stamper {
    stamp: String,
    explicit: bool,
}

impl Stamper {
    fn new(explicit: Option<&str>) -> Self {
        match explicit {
            Some(stamp) => Self {
                stamp: stamp.to_owned(),
                explicit: true,
            },
            None => Self {
                stamp: epoch_stamp(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map_or(0, |elapsed| i64::try_from(elapsed.as_secs()).unwrap_or(0)),
                ),
                explicit: false,
            },
        }
    }

    /// The stamp of an entry that replaces one stamped `old`: later than it.
    fn replacing(&self, function: &str, old: Option<&str>) -> Result<String, String> {
        let Some(old) =
            old.filter(|old| qcore_schemas::contract_supersession::validate_stamp(old).is_ok())
        else {
            return Ok(self.stamp.clone());
        };
        if self.stamp.as_str() > old {
            return Ok(self.stamp.clone());
        }
        if self.explicit {
            return Err(format!(
                "`{function}`: a re-pin must carry a later stamp than the entry it replaces \
                 ({old}); --pinned-at {} is not later",
                self.stamp
            ));
        }
        Ok(epoch_stamp(stamp_epoch(old) + 1))
    }
}

/// Seconds since the Unix epoch of a valid stamp.
fn stamp_epoch(stamp: &str) -> i64 {
    let field = |range: std::ops::Range<usize>| stamp[range].parse::<i64>().unwrap_or(0);
    let (year, month, day) = (field(0..4), field(5..7), field(8..10));
    // Days from civil (H. Hinnant's algorithm).
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let day_of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    let days = era * 146_097 + day_of_era - 719_468;
    days * 86_400 + field(11..13) * 3_600 + field(14..16) * 60 + field(17..19)
}

/// The stamp of a time in seconds since the Unix epoch.
fn epoch_stamp(seconds: i64) -> String {
    let days = seconds.div_euclid(86_400);
    let rest = seconds.rem_euclid(86_400);
    // Civil from days (H. Hinnant's algorithm).
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rest / 3_600,
        rest % 3_600 / 60,
        rest % 60
    )
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
    /// Card CELL-GUARD-RESULT-RANGE-CHECKED: each pinned bounds declaration
    /// stating a `nonnull_result_range`, with its declared set.
    ranges: BTreeMap<String, qcore_contract::bounds_contract::ResultSet>,
    /// Card CELL-RESULT-SET-WRITTEN-SET-DERIVATION: each pinned bounds
    /// declaration's `written_set(SLOT[K], SET) when #result == C` clauses,
    /// as (slot spelling, parameter position, K, C, declared set).
    written: BTreeMap<String, Vec<DeclaredWritten>>,
    /// Card CELL-GUARD-RESULT-RANGE-CHECKED: the derivations this run
    /// computed (`--derive-results`), by function.
    derivations: BTreeMap<String, ResultDerivation>,
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
            ranges: BTreeMap::new(),
            written: BTreeMap::new(),
            derivations: BTreeMap::new(),
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
                        // Card CELL-GUARD-RESULT-RANGE-CHECKED: a C library
                        // result set is flexible (C09.4), listed so that a
                        // change of mode is a change of the lock.
                        if contract.flexible {
                            RevisionMode::Flexible
                        } else {
                            RevisionMode::Pinned
                        },
                    ),
                )
            })
            .collect();
        let ranges = contracts
            .functions
            .iter()
            .filter(|(_, contract)| !contract.flexible)
            .filter_map(|(function, contract)| {
                contract.result_values().map(|set| (function.clone(), set))
            })
            .collect();
        let written = contracts
            .functions
            .iter()
            .filter(|(_, contract)| !contract.flexible)
            .filter_map(|(function, contract)| {
                let declared = declared_written(contract);
                (!declared.is_empty()).then(|| (function.clone(), declared))
            })
            .collect();
        targets.push(Target {
            label: format!("{name} (bounds)"),
            owner: name.clone(),
            lock: root.join(crate::bounds_selection::BOUNDS_LOCK_FILE),
            declared,
            c_names: BTreeMap::new(),
            ranges,
            written,
            derivations: BTreeMap::new(),
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
            ranges: BTreeMap::new(),
            written: BTreeMap::new(),
            derivations: BTreeMap::new(),
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
        fluent_bit_revision: None,
        pinned: Vec::new(),
        flexible: Vec::new(),
        tip: Vec::new(),
    }
}

/// Card PIN-REGENERATE-STAMP-MONOTONICITY: the stamp of the entry that an
/// entry of `function` written now replaces, read from `replaced`, the lock
/// on disk before the run: the latest valid stamp of any entry of `function`
/// in it, pinned, flexible or tip, since a revision-mode change replaces the
/// entry of the other kind. `None` when the lock holds no stamped entry of
/// it (or there is no readable lock).
fn replaced_stamp<'a>(replaced: Option<&'a ContractLock>, function: &str) -> Option<&'a str> {
    let lock = replaced?;
    lock.pinned
        .iter()
        .filter(|e| e.function == function)
        .map(|e| e.pinned_at.as_deref())
        .chain(
            lock.flexible
                .iter()
                .filter(|e| e.function == function)
                .map(|e| e.pinned_at.as_deref()),
        )
        .chain(
            lock.tip
                .iter()
                .filter(|e| e.function == function)
                .map(|e| e.pinned_at.as_deref()),
        )
        .flatten()
        .filter(|stamp| qcore_schemas::contract_supersession::validate_stamp(stamp).is_ok())
        .max()
}

/// Card CELL-RESULT-SET-WRITTEN-SET-DERIVATION: one declared written set of
/// a pinned bounds declaration.
#[derive(Clone, Debug)]
struct DeclaredWritten {
    /// `PARAMETER[K]`.
    slot: String,
    position: usize,
    index: u32,
    class: i128,
    set: qcore_contract::bounds_contract::ResultSet,
}

/// A declaration's `written_set` clauses.
fn declared_written(
    contract: &qcore_contract::bounds_contract::BoundsFunctionContract,
) -> Vec<DeclaredWritten> {
    contract
        .clauses
        .iter()
        .filter_map(|entry| match &entry.clause {
            qcore_contract::bounds_contract::BoundsClause::WrittenSet {
                parameter,
                index,
                set,
                class,
            } => Some(DeclaredWritten {
                slot: format!("{parameter}[{index}]"),
                position: contract
                    .parameters
                    .iter()
                    .position(|p| &p.name == parameter)?,
                index: *index,
                class: *class,
                set: set.clone(),
            }),
            _ => None,
        })
        .collect()
}

/// Card CELL-GUARD-RESULT-RANGE-CHECKED (contract specification C09.4,
/// gateway rule 38e): derive every chosen pinned `nonnull_result_range` of
/// `target` from the body chain of its one defining unit in the configured
/// compile database, reading only the C library's result sets (the flexible
/// `result_set` declarations of `quirin-provider-libc` on the load path).
/// The derived set must lie inside the declared range; anything else
/// refuses, and nothing is pinned. Card CELL-RESULT-SET-WRITTEN-SET-DERIVATION
/// (rule 38e as amended): every declared `written_set(SLOT[K], SET) when
/// #result == C` of a chosen declaration is derived the same way (forwarding,
/// or a store after the C library call's failure exit with the returned
/// local narrowed; `qcore_c_gateway::null_deref::derive_written_sets_c11`)
/// and must lie inside the declared set; one that derives nothing, or
/// something outside, refuses the whole run.
fn derive_results(
    target: &mut Target,
    tree: &SourceTree,
    root: &Path,
    functions: &BTreeSet<String>,
    database: &Path,
    load_path: &Path,
) -> Result<(), String> {
    let chosen = target
        .ranges
        .keys()
        .chain(target.written.keys())
        .filter(|name| functions.is_empty() || functions.contains(*name))
        .cloned()
        .collect::<BTreeSet<_>>();
    if chosen.is_empty() {
        return Ok(());
    }
    let manifest = load_contract_load_path_manifest(
        &load_path
            .canonicalize()
            .map_err(|error| format!("{}: {error}", load_path.display()))?,
    )?;
    let (mut environment, _) = crate::bounds_selection::load_views(&manifest)?;
    environment.functions.retain(|_, contract| {
        contract.flexible
            && contract
                .module
                .starts_with(&format!("{}:", crate::contract_revision::LIBC_CRATE))
    });
    let text =
        fs::read_to_string(database).map_err(|error| format!("{}: {error}", database.display()))?;
    let entries = crate::compile_db::compile_entries(&text)?;
    let sites = tree.sites(&chosen);
    for name in chosen {
        let definitions = sites
            .get(&name)
            .into_iter()
            .flatten()
            .filter(|site| site.kind == crate::c_identity::SiteKind::Definition)
            .map(|site| site.path.clone())
            .collect::<BTreeSet<_>>();
        let [path] = definitions.iter().collect::<Vec<_>>()[..] else {
            return Err(format!(
                "`{name}`: a result derivation needs exactly one defining file in the source \
                 tree, found {}",
                definitions.len()
            ));
        };
        let file = root.join(path);
        // Card CELL-RESULT-SET-WRITTEN-SET-DERIVATION: every configuration
        // of the defining file decides its own sets (a file built into two
        // libraries is read twice); each must lie inside the declaration, and
        // the record is their union.
        let configurations = crate::compile_db::select_configurations(&entries, &file)
            .map_err(|why| format!("`{name}`: {why}"))?;
        if configurations.is_empty() {
            return Err(format!(
                "`{name}`: compilation database contains no command for {}",
                file.display()
            ));
        }
        let mut helpers = BTreeSet::new();
        let mut rested = BTreeSet::new();
        let mut result_set: Option<qcore_contract::bounds_contract::ResultSet> = None;
        let declared_written = target.written.get(&name).cloned().unwrap_or_default();
        let slots = declared_written
            .iter()
            .map(|declared| (declared.position, declared.index, declared.class))
            .collect::<Vec<_>>();
        let mut written: Vec<Option<qcore_contract::bounds_contract::ResultSet>> =
            vec![None; declared_written.len()];
        let union = |acc: Option<qcore_contract::bounds_contract::ResultSet>,
                     set: &qcore_contract::bounds_contract::ResultSet| {
            Some(acc.map_or_else(|| set.clone(), |acc| acc.union(set)))
        };
        for entry in configurations {
            let source = fs::read_to_string(&entry.file)
                .map_err(|error| format!("{}: {error}", entry.file.display()))?;
            let unit = crate::member_prepass::c_unit(entry, &source)?;
            if let Some(declared) = target.ranges.get(&name) {
                let derived =
                    qcore_c_gateway::null_deref::derive_result_set_c11(&unit, &name, &environment)
                        .map_err(|why| format!("`{name}`: {why}"))?;
                if derived.set.universe() != declared.universe() || !derived.set.is_subset(declared)
                {
                    return Err(format!(
                        "`{name}`: its body chain returns {} (helpers {}; C library sets {}), not \
                         inside the declared {}; nothing is pinned",
                        derived.set.canonical(),
                        derived
                            .helpers
                            .iter()
                            .cloned()
                            .collect::<Vec<_>>()
                            .join(", "),
                        derived
                            .rests_on
                            .iter()
                            .cloned()
                            .collect::<Vec<_>>()
                            .join(", "),
                        declared.canonical()
                    ));
                }
                helpers.extend(derived.helpers);
                rested.extend(derived.rests_on);
                result_set = union(result_set, &derived.set);
            }
            if slots.is_empty() {
                continue;
            }
            let derived = qcore_c_gateway::null_deref::derive_written_sets_c11(
                &unit,
                &name,
                &slots,
                &environment,
            )
            .map_err(|why| format!("`{name}`: {why}"))?;
            for ((declared, derived), acc) in declared_written.iter().zip(derived).zip(&mut written)
            {
                let derived = derived.map_err(|why| {
                    format!(
                        "`{name}`: its written set of `{}` when it returns {} is not derived \
                         ({why}); nothing is pinned",
                        declared.slot, declared.class
                    )
                })?;
                if derived.set.universe() != declared.set.universe()
                    || !derived.set.is_subset(&declared.set)
                {
                    return Err(format!(
                        "`{name}`: its body chain writes {} at `{}` when it returns {} (helpers \
                         {}; C library sets {}), not inside the declared {}; nothing is pinned",
                        derived.set.canonical(),
                        declared.slot,
                        declared.class,
                        derived
                            .helpers
                            .iter()
                            .cloned()
                            .collect::<Vec<_>>()
                            .join(", "),
                        derived
                            .rests_on
                            .iter()
                            .cloned()
                            .collect::<Vec<_>>()
                            .join(", "),
                        declared.set.canonical()
                    ));
                }
                helpers.extend(derived.helpers);
                rested.extend(derived.rests_on);
                *acc = union(acc.take(), &derived.set);
            }
        }
        let result_set = result_set.map(|set| set.canonical());
        let mut written_sets = declared_written
            .iter()
            .zip(written)
            .filter_map(|(declared, set)| {
                let set = set?;
                Some(i64::try_from(declared.class).map(|class| {
                    crate::contract_revision::WrittenDerivation {
                        slot: declared.slot.clone(),
                        class,
                        written_set: set.canonical(),
                    }
                }))
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| format!("`{name}`: a class outside `i64`"))?;
        written_sets.sort();
        let rests_on = rested
            .iter()
            .map(|function| {
                let contract = &environment.functions[function];
                RestedOn {
                    function: function.clone(),
                    module: contract.module.clone(),
                    contract_sha256: contract.text_sha256.clone(),
                }
            })
            .collect();
        target.derivations.insert(
            name,
            ResultDerivation {
                result_set,
                written_sets,
                helpers: helpers.into_iter().collect(),
                rests_on,
            },
        );
    }
    Ok(())
}

/// Add or refresh the identities `tree` gives: every current declaration (or
/// only `functions`) is recorded at the tree's source. Identities reviewed at
/// other sources are kept, unless the contract text changed, which voids
/// them. Entries no longer declared go (only on a full update). Returns the
/// new lock and one line per changed entry. Each entry written anew is
/// stamped later than the entry of its function in `replaced` (the lock on
/// disk before the run: `existing` for an update, the lock a regeneration
/// replaces otherwise).
fn update(
    existing: Option<&ContractLock>,
    replaced: Option<&ContractLock>,
    target: &Target,
    tree: &SourceTree,
    functions: &BTreeSet<String>,
    untrack: bool,
    stamper: &Stamper,
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
    // Card CELL-GUARD-RESULT-RANGE-CHECKED: a derivation's helpers' sites
    // join their function's identity.
    let mut scanned = crate::c_identity::identity_names(&pinned_names, &target.c_names);
    scanned.extend(
        target
            .derivations
            .values()
            .flat_map(|derivation| derivation.helpers.iter().cloned()),
    );
    let sites = tree.sites(&scanned);
    let old = existing.cloned().unwrap_or_else(|| empty_lock(target));
    let mut lock = ContractLock {
        pinned: Vec::new(),
        flexible: Vec::new(),
        fluent_bit_revision: old.fluent_bit_revision.clone(),
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
                // Card CELL-GUARD-RESULT-RANGE-CHECKED (rule 38e): an entry
                // that carries a derivation is re-pinned only with one.
                let derivation = target.derivations.get(name);
                let old_derivation = old_pinned(name).and_then(|e| e.derivation.as_ref());
                if derivation.is_none() && old_derivation.is_some() {
                    return Err(format!(
                        "`{name}` carries a result derivation (rule 38e): pin it again with \
                         --derive-results --compile-database FILE"
                    ));
                }
                let mut entry = match old_pinned(name) {
                    Some(entry)
                        if derivation.is_some()
                            && old_derivation.is_some()
                            && old_derivation != derivation
                            && entry.contract_sha256 == *digest =>
                    {
                        lines.push(format!(
                            "! {name}: result derivation changed; {} earlier identit(ies) voided",
                            entry.identities.len()
                        ));
                        PinnedEntry {
                            identities: Vec::new(),
                            ..entry.clone()
                        }
                    }
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
                            pinned_at: None,
                            identities: Vec::new(),
                            derivation: None,
                        }
                    }
                };
                let unchanged = old_pinned(name).cloned();
                let before = entry.identities.clone();
                entry.module.clone_from(module);
                entry.contract_sha256.clone_from(digest);
                // Card SUPPLIED-OWNED-RETURN-CODE-HANDOFF (rule 13l): the
                // enumerations of its `@` names join the identity; an `@`
                // name the tree does not declare refuses the pin.
                let mut identity = crate::c_identity::identity_sites(&sites, name, &target.c_names)
                    .map_err(|why| format!("`{name}`: {why} at {}", source.label()))?;
                if let Some(derivation) = derivation {
                    identity = with_helper_sites(&sites, identity, &derivation.helpers)
                        .map_err(|why| format!("`{name}`: {why} at {}", source.label()))?;
                    if old_derivation != Some(derivation) {
                        let mut derived = derivation
                            .result_set
                            .iter()
                            .map(|set| format!("result set {set}"))
                            .collect::<Vec<_>>();
                        derived.extend(derivation.written_sets.iter().map(|written| {
                            format!(
                                "written set {} of `{}` when it returns {}",
                                written.written_set, written.slot, written.class
                            )
                        }));
                        lines.push(format!(
                            "~ {name}: {} derived at {} (helpers {}; rests on {})",
                            derived.join(", "),
                            source.label(),
                            derivation.helpers.join(", "),
                            derivation
                                .rests_on
                                .iter()
                                .map(|rested| rested.function.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        ));
                    }
                }
                entry.derivation = derivation.cloned();
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
                // Card CONTRACT-SUPERSESSION: an entry this run adds or
                // changes carries the run's stamp, later than the replaced
                // one; an unchanged entry keeps its own. Card
                // PIN-REGENERATE-STAMP-MONOTONICITY: the replaced one is the
                // lock's entry of the function before the run, whatever its
                // kind, also under a regeneration.
                let same = unchanged.as_ref().is_some_and(|old| {
                    old.module == entry.module
                        && old.contract_sha256 == entry.contract_sha256
                        && old.identities == entry.identities
                        && old.derivation == entry.derivation
                });
                if !same || entry.pinned_at.is_none() {
                    entry.pinned_at =
                        Some(stamper.replacing(name, replaced_stamp(replaced, name))?);
                }
                lock.pinned.push(entry);
            }
            RevisionMode::Flexible => {
                let previous = old_flexible(name);
                let mut entry = FlexibleEntry {
                    function: name.clone(),
                    module: module.clone(),
                    contract_sha256: digest.clone(),
                    pinned_at: previous.and_then(|old| old.pinned_at.clone()),
                };
                if previous != Some(&entry) {
                    lines.push(format!("~ {name}: flexible"));
                }
                // Card CONTRACT-SUPERSESSION: as for a pinned entry.
                let same = previous.is_some_and(|old| {
                    old.module == entry.module && old.contract_sha256 == entry.contract_sha256
                });
                if !same || entry.pinned_at.is_none() {
                    entry.pinned_at =
                        Some(stamper.replacing(name, replaced_stamp(replaced, name))?);
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
    stamper: &Stamper,
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
        // Card CONTRACT-SUPERSESSION: a tip entry records a review; its stamp
        // is later than the entry it replaces.
        let replaced_stamp = lock
            .pinned
            .iter()
            .find(|e| &e.function == function)
            .and_then(|e| e.pinned_at.clone())
            .or_else(|| {
                lock.tip
                    .iter()
                    .find(|e| &e.function == function)
                    .and_then(|e| e.pinned_at.clone())
            });
        let pinned_at = stamper.replacing(function, replaced_stamp.as_deref())?;
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
            pinned_at: Some(pinned_at),
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

/// Card CELL-GUARD-RESULT-RANGE-CHECKED: every derivation of `lock` rests on
/// C library declarations whose text is still the one it read.
fn derivation_problems(lock: &ContractLock, load_path: &Path) -> Result<Vec<String>, String> {
    let manifest = load_contract_load_path_manifest(
        &load_path
            .canonicalize()
            .map_err(|error| format!("{}: {error}", load_path.display()))?,
    )?;
    let (environment, _) = crate::bounds_selection::load_views(&manifest)?;
    let mut problems = Vec::new();
    for entry in &lock.pinned {
        for rested in entry.derivation.iter().flat_map(|d| &d.rests_on) {
            let current = environment
                .functions
                .get(&rested.function)
                .filter(|contract| {
                    contract.flexible
                        && contract.module == rested.module
                        && contract.text_sha256 == rested.contract_sha256
                });
            if current.is_none() {
                problems.push(format!(
                    "`{}`'s result derivation rests on `{}` ({}), whose declaration changed or \
                     went: derive it again",
                    entry.function, rested.function, rested.module
                ));
            }
        }
    }
    Ok(problems)
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
    let helpers_of = |name: &str| {
        lock.and_then(|lock| lock.pinned.iter().find(|e| e.function == name))
            .and_then(|entry| entry.derivation.as_ref())
            .map(|derivation| derivation.helpers.clone())
            .unwrap_or_default()
    };
    let mut scanned = crate::c_identity::identity_names(&pinned_names, &target.c_names);
    for name in &pinned_names {
        scanned.extend(helpers_of(name));
    }
    let sites = tree.sites(&scanned);
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
                // Card CELL-GUARD-RESULT-RANGE-CHECKED: and the derivation's
                // helpers (a missing helper is a mismatch, never in force).
                let helpers = helpers_of(name);
                let found = if helpers.is_empty() {
                    found
                } else {
                    with_helper_sites(&sites, found.clone(), &helpers).unwrap_or(found)
                };
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

/// The provider tree of a run: the directory of its load path.
fn provider_root(arguments: &Arguments) -> PathBuf {
    let load_path = arguments
        .load_path
        .as_deref()
        .unwrap_or_else(|| Path::new(DEFAULT_LOAD_PATH));
    load_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf)
}

/// Card CONTRACT-SUPERSESSION: what `--verify-contracts` requires of a lock
/// beyond its declarations: version 4, a valid stamp on every entry and a
/// revision tie.
fn stamp_problems(lock: &ContractLock) -> Vec<String> {
    use qcore_schemas::contract_supersession::validate_stamp;
    let mut problems = Vec::new();
    if lock.lock_version < LOCK_VERSION {
        problems.push(format!(
            "lock version {} carries no pin stamps (run `pin --stamp-only` after review)",
            lock.lock_version
        ));
        return problems;
    }
    if lock.fluent_bit_revision.is_none() {
        problems.push("the lock is tied to no Fluent Bit revision".into());
    }
    let entries = lock
        .pinned
        .iter()
        .map(|e| (&e.function, &e.pinned_at))
        .chain(lock.flexible.iter().map(|e| (&e.function, &e.pinned_at)))
        .chain(lock.tip.iter().map(|e| (&e.function, &e.pinned_at)));
    for (function, stamp) in entries {
        match stamp {
            None => problems.push(format!("`{function}` carries no pin stamp")),
            Some(stamp) if validate_stamp(stamp).is_err() => {
                problems.push(format!("`{function}`'s pin stamp `{stamp}` is malformed"));
            }
            Some(_) => {}
        }
    }
    problems
}

/// Card CONTRACT-SUPERSESSION: `--window-*`.
fn run_window(arguments: &Arguments, operation: &WindowOperation) -> Result<bool, String> {
    let root = provider_root(arguments);
    match operation {
        WindowOperation::Check => {
            let problems = crate::pin_window::check(&root)?;
            for problem in &problems {
                println!("pin: lock-set window: {problem}");
            }
            if problems.is_empty() {
                let window = crate::pin_window::PinLockWindow::read(
                    &crate::pin_window::manifest_path(&root),
                )?;
                println!(
                    "pin: lock-set window: {} of {} set(s), live set tied to {}",
                    window.sets.len(),
                    window.capacity,
                    window.latest()
                );
            }
            Ok(problems.is_empty())
        }
        WindowOperation::Create(revision) => {
            crate::pin_window::create(&root, revision)?;
            println!("pin: lock-set window created; live set tied to {revision}");
            Ok(true)
        }
        WindowOperation::Add(revision) | WindowOperation::Rotate(revision) => {
            let rotate = matches!(operation, WindowOperation::Rotate(_));
            let (window, retired) = crate::pin_window::add(&root, revision, rotate)?;
            if let Some(set) = retired {
                println!(
                    "pin: lock-set window: retired the set of {}",
                    set.fluent_bit_revision
                );
            }
            println!(
                "pin: lock-set window: {} of {} set(s); live set now tied to {revision} (re-pin \
                 the declarations whose sites changed at it)",
                window.sets.len(),
                window.capacity
            );
            Ok(true)
        }
        WindowOperation::Retire => {
            let set = crate::pin_window::retire(&root)?;
            println!(
                "pin: lock-set window: retired the set of {}",
                set.fluent_bit_revision
            );
            Ok(true)
        }
    }
}

/// Card CONTRACT-SUPERSESSION: `--stamp-only`, the one-time migration. Every
/// unstamped entry whose contract text is current gets the run's stamp, no
/// identity changes, and the lock is tied to the checkout's `HEAD`. A lock
/// whose declarations differ from its entries refuses: a stamp records the
/// review of the current text, and a stale entry was not reviewed.
fn stamp_only(
    target: &Target,
    root: &Path,
    stamper: &Stamper,
) -> Result<(ContractLock, usize), String> {
    let Some(mut lock) = read_existing(target)? else {
        return Err(format!("{}: no lock to stamp", target.label));
    };
    let problems = contracts_agree(&lock, &declarations(target)?);
    if !problems.is_empty() {
        return Err(format!(
            "{}: the lock is stale ({}); pin it after review instead",
            target.label,
            problems.join("; ")
        ));
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
    let revision = match git_output(&root, &["rev-parse", "--verify", "HEAD^{commit}"])? {
        (0, revision) if revision.len() == 40 => revision,
        _ => return Err(format!("{} has no HEAD commit", root.display())),
    };
    match &lock.fluent_bit_revision {
        Some(tied) if *tied != revision => {
            return Err(format!(
                "{}: the lock is tied to {tied}, not {revision}; move the lock set with \
                 `pin --window-add`",
                target.label
            ));
        }
        _ => lock.fluent_bit_revision = Some(revision),
    }
    let mut stamped = 0;
    for stamp in lock
        .pinned
        .iter_mut()
        .map(|e| &mut e.pinned_at)
        .chain(lock.flexible.iter_mut().map(|e| &mut e.pinned_at))
        .chain(lock.tip.iter_mut().map(|e| &mut e.pinned_at))
    {
        if stamp.is_none() {
            *stamp = Some(stamper.stamp.clone());
            stamped += 1;
        }
    }
    Ok((lock, stamped))
}

fn run(arguments: Arguments) -> Result<bool, String> {
    if let Some(operation) = &arguments.window {
        return run_window(&arguments, operation);
    }
    let stamper = Stamper::new(arguments.pinned_at.as_deref());
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
                Some(lock) => {
                    let mut problems = contracts_agree(lock, &declared);
                    problems.extend(stamp_problems(lock));
                    // Card CELL-GUARD-RESULT-RANGE-CHECKED: a derivation
                    // rests on the C library declarations it read.
                    if lock.pinned.iter().any(|entry| entry.derivation.is_some()) {
                        let load_path = arguments
                            .load_path
                            .as_deref()
                            .unwrap_or_else(|| Path::new(DEFAULT_LOAD_PATH));
                        problems.extend(derivation_problems(lock, load_path)?);
                    }
                    problems
                }
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
        // Card CONTRACT-SUPERSESSION: the lock-set window, when it exists.
        if arguments.modules.is_none() {
            let provider_root = provider_root(&arguments);
            if crate::pin_window::manifest_path(&provider_root).is_file() {
                let problems = crate::pin_window::check(&provider_root)?;
                if problems.is_empty() {
                    let window = crate::pin_window::PinLockWindow::read(
                        &crate::pin_window::manifest_path(&provider_root),
                    )?;
                    println!(
                        "pin: lock-set window: {} of {} set(s), live set tied to {}",
                        window.sets.len(),
                        window.capacity,
                        window.latest()
                    );
                } else {
                    clean = false;
                    for problem in problems {
                        println!("pin: lock-set window: {problem}");
                    }
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
    if arguments.stamp_only {
        let mut written = Vec::new();
        for target in &targets {
            written.push((target, stamp_only(target, root, &stamper)?));
        }
        for (target, (lock, stamped)) in written {
            fs::write(&target.lock, lock.to_text()?)
                .map_err(|error| format!("{}: {error}", target.lock.display()))?;
            println!(
                "pin: {}: stamped {stamped} entr(ies) at {}; tied to {}; wrote {}",
                target.label,
                stamper.stamp,
                lock.fluent_bit_revision.as_deref().unwrap_or_default(),
                target.lock.display()
            );
        }
        return Ok(true);
    }
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
            let (lock, lines, recorded) =
                track(existing.as_ref(), target, root, &arguments, &stamper)?;
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
    // Card CELL-GUARD-RESULT-RANGE-CHECKED (rule 38e): the result derivations,
    // computed before any lock is written.
    let mut targets = targets;
    if let Some(database) = arguments
        .compile_database
        .as_deref()
        .filter(|_| arguments.derive_results)
    {
        let load_path = arguments
            .load_path
            .as_deref()
            .unwrap_or_else(|| Path::new(DEFAULT_LOAD_PATH));
        for target in &mut targets {
            derive_results(
                target,
                &tree,
                root,
                &arguments.functions,
                database,
                load_path,
            )?;
        }
    }
    let mut clean = true;
    let mut tips = TipResolver::from_environment()?;
    // Card PIN-REGENERATE-STAMP-MONOTONICITY: every lock is computed before
    // any is written, so a refused entry (a stamp not later than the one it
    // replaces) leaves every lock as it was.
    let mut writes = Vec::new();
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
        // lock, saying so); an update needs a lock it can read. Card
        // PIN-REGENERATE-STAMP-MONOTONICITY: the lock a regeneration
        // replaces still orders its stamps.
        let (existing, previous, replaced) = if arguments.regenerate {
            match read_existing(target) {
                Ok(Some(lock)) => (
                    None,
                    Some(lock),
                    Some("replaces the lock from this source alone".to_owned()),
                ),
                Ok(None) => (None, None, None),
                Err(error) => (
                    None,
                    None,
                    Some(format!("replaces an unreadable lock: {error}")),
                ),
            }
        } else {
            (read_existing(target)?, None, None)
        };
        // Card CONTRACT-PIN-TIP-MODE: a regeneration keeps tip entries as
        // written (they name no revision of this source).
        let kept_tips = previous
            .as_ref()
            .map(|lock| lock.tip.clone())
            .unwrap_or_default();
        let regenerated_from = (!kept_tips.is_empty()).then(|| ContractLock {
            tip: kept_tips,
            ..empty_lock(target)
        });
        let (mut lock, mut lines) = update(
            existing.as_ref().or(regenerated_from.as_ref()),
            existing.as_ref().or(previous.as_ref()),
            target,
            &tree,
            &arguments.functions,
            arguments.untrack_tip,
            &stamper,
        )?;
        // Card CONTRACT-SUPERSESSION: a new lock of the provider tree joins
        // the live lock set, tied to the window's latest revision.
        if lock.fluent_bit_revision.is_none() && arguments.modules.is_none() {
            let manifest = crate::pin_window::manifest_path(&provider_root(&arguments));
            if manifest.is_file() {
                lock.fluent_bit_revision = Some(
                    crate::pin_window::PinLockWindow::read(&manifest)?
                        .latest()
                        .to_owned(),
                );
            }
        }
        if let Some(replaced) = replaced {
            lines.insert(0, format!("! {replaced}"));
        }
        lines.extend(tip_notices(&lock, target, &tree, &mut tips)?);
        let text = lock.to_text()?;
        writes.push((target, lock, text, lines));
    }
    for (target, lock, text, lines) in writes {
        fs::write(&target.lock, text)
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

#[cfg(test)]
mod stamp_tests {
    #[test]
    fn stamps_round_trip_through_epoch_seconds() {
        for stamp in [
            "1970-01-01T00:00:00Z",
            "2000-02-29T12:34:56Z",
            "2026-10-26T23:59:59Z",
            "2100-03-01T00:00:00Z",
        ] {
            assert_eq!(super::epoch_stamp(super::stamp_epoch(stamp)), stamp);
        }
        assert_eq!(super::stamp_epoch("1970-01-02T00:00:01Z"), 86_401);
        assert_eq!(
            super::epoch_stamp(super::stamp_epoch("2026-12-31T23:59:59Z") + 1),
            "2027-01-01T00:00:00Z"
        );
    }

    #[test]
    fn a_re_pin_carries_a_later_stamp() {
        let explicit = super::Stamper::new(Some("2026-10-26T00:00:00Z"));
        assert_eq!(
            explicit
                .replacing("f", Some("2026-10-25T00:00:00Z"))
                .unwrap(),
            "2026-10-26T00:00:00Z"
        );
        assert!(
            explicit
                .replacing("f", Some("2026-10-26T00:00:00Z"))
                .is_err()
        );
        assert!(
            explicit
                .replacing("f", Some("2026-10-27T00:00:00Z"))
                .is_err()
        );
        assert_eq!(
            explicit.replacing("f", None).unwrap(),
            "2026-10-26T00:00:00Z"
        );
        let clock = super::Stamper {
            stamp: "2026-10-26T00:00:00Z".into(),
            explicit: false,
        };
        assert_eq!(
            clock.replacing("f", Some("2026-10-26T00:00:00Z")).unwrap(),
            "2026-10-26T00:00:01Z"
        );
    }
}
