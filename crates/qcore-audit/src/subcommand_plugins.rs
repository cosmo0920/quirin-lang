//! External `quirin-audit` subcommands (card **W45-AUDIT-SUBCOMMAND-PLUGINS**;
//! contract specification §C11.2): cargo-style plugins, run out of process,
//! whose verdict claims enter the result only through the checker-plugin
//! firewall.
//!
//! ```text
//! quirin-audit [--plugin-path DIR]... --output DIR [HOST-OPTIONS] NAME [ARGUMENTS...]
//! quirin-audit [--plugin-path DIR]... --list-plugins
//! ```
//!
//! * **Dispatch.** `NAME` is a subcommand name (`[a-z0-9][a-z0-9-]*`); a
//!   built-in name ([`BUILTINS`]) always wins and an external
//!   `quirin-audit-NAME` of that name is never run. A configuration path, the
//!   other first-argument form, contains a `.` or a `/`.
//! * **Search.** Only the declared path: every `--plugin-path DIR`, in order,
//!   then [`PLUGIN_PATH_VARIABLE`]. `PATH` is never searched. An unknown name
//!   is a usage error that lists the search path.
//! * **Arguments and environment.** The arguments after `NAME` pass verbatim.
//!   The environment is cleared except [`ENVIRONMENT_ALLOW_LIST`] and each
//!   `--plugin-env NAME`; the invocation record lists every name passed.
//! * **Protocol.** One [`Request`] (JSON) on standard input, one `Response`
//!   (JSON) on standard output, both [`PROTOCOL_VERSION`]
//!   (`quirin-audit-plugin`).
//! * **Quotas and confinement.** A wall-time quota and byte quotas on standard
//!   output and on standard error, which is captured to a file; a breach kills
//!   the plugin's process group (its own, created at spawn and captured by
//!   PID). The working directory is the declared output, and every artifact
//!   must resolve inside it with its declared SHA-256. This is containment of
//!   a process group and of the artifacts the host accepts, **not an OS
//!   sandbox** (standing constraint 26): nothing stops the plugin writing
//!   elsewhere, and a descendant that leaves the group escapes the kill.
//!   The wall-time quota counts analysis time: time during which the plugin
//!   keeps the [`SLOT_WAIT_MARKER`] in its output (all its work waits for a
//!   gate slot) is excluded, up to `--slot-wait-limit-ms` (default the quota;
//!   `0` counts every wait), and both are recorded (card
//!   MEASURE-QUOTA-AND-INCREMENTAL, requirement R7).
//! * **Firewall.** Each claim is classified as a checker plugin's answer is
//!   ([`qcore_verifier_host::PluginClaim`]): the command line has no trusted
//!   checker for a subcommand plugin's certificate or witness, so an uncertified
//!   `Safe` or `Unsafe` is `Pending` (`plugin-claim-uncertified`), an `Unknown`
//!   is `Unknown`, and a spawn failure, timeout, quota breach, nonzero exit,
//!   malformed response, protocol-version mismatch, plugin-reported failure or
//!   artifact that does not verify is `Unknown`. A `CoreCertificate` has no
//!   public constructor, so no path here can make a claim `Safe`.
//! * **The join.** Every row of the result is the join (`Safe < Pending <
//!   Unknown < Unsafe`) of the host's verdict for that artifact (from
//!   `--audit-report`, else none), the run's verdict (`Unknown` for a failed
//!   run, else nothing) and the classified claims about it; the result's
//!   verdict is the join of every row and the run. A join never lowers a host
//!   verdict.
//!
//! Everything is recorded in `<report>` and `<report>.invocation.json`: the
//! plugin's path, SHA-256, protocol version and exit, the request, the
//! quotas, the verified artifacts and each claim with the plugin's identity.
//! The route writes no `.qo`; a `--qo` checkpoint is an input, verified whole
//! and passed by content digest.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
#[cfg(unix)]
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
#[cfg(unix)]
use std::time::{Duration, Instant};

use qcore_schemas::reason::ReasonCategory;
use qcore_schemas::sha256_hex;
use qcore_schemas::verdict::{PendingReason, ReasonedVerdict};
use qcore_verifier_host::{PluginClaim, PluginDiagnostic};
use quirin_audit_plugin::{
    AuditIdentity, ENVIRONMENT_ALLOW_LIST, EXECUTABLE_PREFIX, FileIdentity, Outcome,
    PLUGIN_PATH_VARIABLE, PROTOCOL_VERSION, ProposedVerdict, Quotas, Request, Response,
    SLOT_WAIT_MARKER, VerdictClaim, VerifiedArtifact, parse_response, verify_artifacts,
};
use serde_json::{Value, json};

/// Usage text of the external-subcommand route.
pub const USAGE: &str = "usage: quirin-audit [--plugin-path DIR]... --output DIR [--report FILE] \
[--audit-report FILE] [--qo FILE]... [--load-path FILE] [--provider NAME]... [--timeout-ms N] \
[--slot-wait-limit-ms N] [--stdout-limit-bytes N] [--stderr-limit-bytes N] [--plugin-env NAME]... \
NAME [ARGUMENTS...]
       quirin-audit [--plugin-path DIR]... --list-plugins";

/// Every built-in subcommand of `quirin-audit` (`src/main.rs`). An external
/// executable of one of these names is never run.
pub const BUILTINS: [&str; 18] = [
    // Card CG19-ATOMIC-SOURCE-ROUTE (gateway rule 11i).
    "atomic-publication",
    "callback-task-handoff",
    "callback-task-handoff-fixture",
    "checkpoint",
    "compare",
    "contracts",
    "current",
    "cycle18-composition",
    "cycle18-obligations",
    "event-payload",
    "native-platform-matrix",
    "observed-wait",
    "pinned",
    "pin",
    "plugin",
    "prepare",
    "timeout-teardown",
    "wait-failures",
];

/// The host options that take a value, all before `NAME`.
const VALUE_OPTIONS: [&str; 12] = [
    "--plugin-path",
    "--output",
    "--report",
    "--audit-report",
    "--qo",
    "--load-path",
    "--provider",
    "--timeout-ms",
    "--slot-wait-limit-ms",
    "--stdout-limit-bytes",
    "--stderr-limit-bytes",
    "--plugin-env",
];

const LIST: &str = "--list-plugins";

/// What `quirin-audit-NAME` names: lowercase ASCII letters, digits and `-`,
/// not starting with `-`.
#[must_use]
pub fn is_subcommand_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    bytes
        .next()
        .is_some_and(|first| first.is_ascii_lowercase() || first.is_ascii_digit())
        && bytes.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

/// Whether `name` is a built-in subcommand.
#[must_use]
pub fn is_builtin(name: &str) -> bool {
    BUILTINS.contains(&name)
}

/// Whether this route handles a command line whose first argument is
/// `first`: a host option, `--list-plugins`, or a subcommand name that is not
/// built in. `main` consults it after every built-in, before the
/// configuration-path form.
#[must_use]
pub fn claims(first: &str) -> bool {
    first == LIST
        || VALUE_OPTIONS.contains(&first)
        || (is_subcommand_name(first) && !is_builtin(first))
}

/// The parsed host options.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct HostOptions {
    pub plugin_paths: Vec<PathBuf>,
    pub list: bool,
    pub output: Option<PathBuf>,
    pub report: Option<PathBuf>,
    pub audit_report: Option<PathBuf>,
    pub checkpoints: Vec<PathBuf>,
    pub load_path: Option<PathBuf>,
    pub providers: Vec<String>,
    pub timeout_ms: Option<u64>,
    /// How much declared slot-wait time ([`SLOT_WAIT_MARKER`]) the wall-time
    /// quota excludes; `0` counts every wait (default: the quota itself).
    pub slot_wait_limit: Option<u64>,
    pub stdout_limit: Option<u64>,
    pub stderr_limit: Option<u64>,
    pub plugin_env: Vec<String>,
    pub name: Option<String>,
    pub arguments: Vec<String>,
}

/// Parse the host options, the name and the verbatim arguments.
///
/// # Errors
///
/// A usage error.
pub fn parse(arguments: &[String]) -> Result<HostOptions, String> {
    let mut options = HostOptions::default();
    let mut index = 0;
    while index < arguments.len() {
        let flag = arguments[index].as_str();
        if flag == LIST {
            options.list = true;
            index += 1;
            continue;
        }
        if !flag.starts_with("--") {
            options.name = Some(flag.to_owned());
            options.arguments = arguments[index + 1..].to_vec();
            break;
        }
        if !VALUE_OPTIONS.contains(&flag) {
            return Err(format!("unknown host option `{flag}`"));
        }
        let value = arguments
            .get(index + 1)
            .ok_or_else(|| format!("{flag} requires a value"))?
            .clone();
        let once = |slot: &mut Option<PathBuf>| {
            if slot.replace(PathBuf::from(&value)).is_some() {
                return Err(format!("{flag} is given twice"));
            }
            Ok(())
        };
        let number = |slot: &mut Option<u64>| {
            let parsed = value
                .parse::<u64>()
                .ok()
                .filter(|parsed| *parsed > 0)
                .ok_or_else(|| format!("{flag} requires a positive integer"))?;
            if slot.replace(parsed).is_some() {
                return Err(format!("{flag} is given twice"));
            }
            Ok(())
        };
        match flag {
            "--plugin-path" => options.plugin_paths.push(PathBuf::from(&value)),
            "--output" => once(&mut options.output)?,
            "--report" => once(&mut options.report)?,
            "--audit-report" => once(&mut options.audit_report)?,
            "--load-path" => once(&mut options.load_path)?,
            "--qo" => options.checkpoints.push(PathBuf::from(&value)),
            "--provider" => options.providers.push(value.clone()),
            "--timeout-ms" => number(&mut options.timeout_ms)?,
            "--slot-wait-limit-ms" => {
                let parsed = value
                    .parse::<u64>()
                    .map_err(|_| format!("{flag} requires a non-negative integer"))?;
                if options.slot_wait_limit.replace(parsed).is_some() {
                    return Err(format!("{flag} is given twice"));
                }
            }
            "--stdout-limit-bytes" => number(&mut options.stdout_limit)?,
            "--stderr-limit-bytes" => number(&mut options.stderr_limit)?,
            "--plugin-env" => options.plugin_env.push(value.clone()),
            _ => unreachable!("every value option is matched"),
        }
        index += 2;
    }
    if options.list && options.name.is_some() {
        return Err("--list-plugins takes no subcommand".into());
    }
    if !options.list && options.name.is_none() {
        return Err("no subcommand name".into());
    }
    Ok(options)
}

/// The declared search path: every `--plugin-path`, then the variable's
/// entries. Relative directories are taken against `cwd`.
#[must_use]
pub fn search_path(flags: &[PathBuf], variable: Option<OsString>, cwd: &Path) -> Vec<PathBuf> {
    let mut directories: Vec<PathBuf> = flags.iter().map(|path| cwd.join(path)).collect();
    if let Some(variable) = variable {
        directories.extend(
            std::env::split_paths(&variable)
                .filter(|path| !path.as_os_str().is_empty())
                .map(|path| cwd.join(path)),
        );
    }
    directories
}

fn executable_name(name: &str) -> String {
    format!("{EXECUTABLE_PREFIX}{name}{}", std::env::consts::EXE_SUFFIX)
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.is_file()
}

/// The first `quirin-audit-NAME` on the search path, or none. A built-in name
/// is never looked up.
#[must_use]
pub fn find(name: &str, directories: &[PathBuf]) -> Option<PathBuf> {
    if is_builtin(name) || !is_subcommand_name(name) {
        return None;
    }
    directories
        .iter()
        .map(|directory| directory.join(executable_name(name)))
        .find(|candidate| is_executable(candidate))
}

/// Every `quirin-audit-*` executable on the search path, in search order:
/// (name, path, how it is reached).
#[must_use]
pub fn list(directories: &[PathBuf]) -> Vec<(String, PathBuf, &'static str)> {
    let mut found = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for directory in directories {
        let Ok(entries) = fs::read_dir(directory) else {
            continue;
        };
        let mut names: Vec<(String, PathBuf)> = entries
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let file = entry.file_name().into_string().ok()?;
                let stem = file.strip_suffix(std::env::consts::EXE_SUFFIX)?;
                let name = stem.strip_prefix(EXECUTABLE_PREFIX)?;
                (is_subcommand_name(name) && is_executable(&entry.path()))
                    .then(|| (name.to_owned(), entry.path()))
            })
            .collect();
        names.sort();
        for (name, path) in names {
            let how = if is_builtin(&name) {
                "shadowed by a built-in; never run"
            } else if !seen.insert(name.clone()) {
                "shadowed by an earlier directory"
            } else {
                "runs"
            };
            found.push((name, path, how));
        }
    }
    found
}

/// Entry point of the external-subcommand route. Exit 0: the plugin completed
/// and the report is written; 1: the run failed (the report is written, its
/// verdict `Unknown`); 2: a usage error or a refused input, nothing run.
#[must_use]
pub fn main(arguments: Vec<String>) -> ExitCode {
    let usage = |error: String| {
        eprintln!("quirin-audit: {error}\n{USAGE}");
        ExitCode::from(2)
    };
    let options = match parse(&arguments) {
        Ok(options) => options,
        Err(error) => return usage(error),
    };
    let cwd = match std::env::current_dir() {
        Ok(cwd) => cwd,
        Err(error) => return usage(error.to_string()),
    };
    let directories = search_path(
        &options.plugin_paths,
        std::env::var_os(PLUGIN_PATH_VARIABLE),
        &cwd,
    );
    if options.list {
        println!("plugin search path (--plugin-path, then {PLUGIN_PATH_VARIABLE}; never PATH):");
        for directory in &directories {
            println!("  {}", directory.display());
        }
        for (name, path, how) in list(&directories) {
            println!("{name}\t{}\t{how}", path.display());
        }
        return ExitCode::SUCCESS;
    }
    let name = options.name.clone().expect("parse requires a name");
    if is_builtin(&name) {
        return usage(format!(
            "`{name}` is a built-in subcommand: host options precede only external subcommands, \
             and an external `{}` is never run",
            executable_name(&name)
        ));
    }
    if !is_subcommand_name(&name) {
        return usage(format!("`{name}` is not a subcommand name"));
    }
    let Some(executable) = find(&name, &directories) else {
        return usage(format!(
            "no subcommand `{name}`: it is not built in, and no executable `{}` is on the plugin \
             search path [{}] (--plugin-path DIR, then {PLUGIN_PATH_VARIABLE}; PATH is not \
             searched)",
            executable_name(&name),
            directories
                .iter()
                .map(|directory| directory.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    };
    let prepared = match prepare(&options, &name, &executable, &cwd) {
        Ok(prepared) => prepared,
        Err(error) => return usage(error),
    };
    match execute(&prepared, &arguments, &directories) {
        Ok(completed) => {
            if completed {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(error) => {
            eprintln!("quirin-audit: the subcommand record could not be written: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Everything decided before the plugin runs.
#[derive(Debug)]
pub struct Prepared {
    pub name: String,
    pub executable: PathBuf,
    pub executable_sha256: String,
    pub request: Request,
    pub report: PathBuf,
    pub stderr_log: PathBuf,
    pub environment: Vec<(String, OsString)>,
    pub host: BTreeMap<String, ReasonedVerdict>,
    /// The most declared slot-wait time the wall-time quota excludes.
    pub slot_wait_limit_ms: u64,
}

fn read(path: &Path) -> Result<Vec<u8>, String> {
    fs::read(path).map_err(|error| format!("{}: {error}", path.display()))
}

fn identity_of(path: &Path) -> Result<FileIdentity, String> {
    let canonical = path
        .canonicalize()
        .map_err(|error| format!("{}: {error}", path.display()))?;
    Ok(FileIdentity {
        sha256: sha256_hex(&read(&canonical)?),
        path: canonical.display().to_string(),
    })
}

/// A report path's canonical spelling (its parent created and resolved).
fn settle(path: &Path) -> Result<PathBuf, String> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty());
    let name = path
        .file_name()
        .ok_or_else(|| format!("{} names no file", path.display()))?;
    let parent = match parent {
        Some(parent) => {
            fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
            parent
                .canonicalize()
                .map_err(|error| format!("{}: {error}", parent.display()))?
        }
        None => std::env::current_dir().map_err(|error| error.to_string())?,
    };
    Ok(parent.join(name))
}

/// Check every input and build the request. Nothing runs yet.
///
/// # Errors
///
/// A missing `--output`, an unreadable or invalid input, a provider without
/// a load path or not on it, or checkpoints of different runs.
pub fn prepare(
    options: &HostOptions,
    name: &str,
    executable: &Path,
    cwd: &Path,
) -> Result<Prepared, String> {
    let output = options
        .output
        .as_ref()
        .ok_or("an external subcommand requires --output DIR (its working directory and the only place its artifacts may lie)")?;
    let output = cwd.join(output);
    fs::create_dir_all(&output).map_err(|error| format!("{}: {error}", output.display()))?;
    let output = output
        .canonicalize()
        .map_err(|error| format!("{}: {error}", output.display()))?;
    let report = settle(&options.report.as_ref().map_or_else(
        || output.join(format!("{EXECUTABLE_PREFIX}{name}.report.json")),
        |report| cwd.join(report),
    ))?;
    let stderr_log = report.with_extension("stderr.log");

    let load_path = match &options.load_path {
        None => {
            if !options.providers.is_empty() {
                return Err(
                    "--provider selects crates on a load path; pass --load-path FILE".into(),
                );
            }
            None
        }
        Some(path) => {
            let path = cwd.join(path);
            let manifest =
                qcore_contract::load_path_manifest::load_contract_load_path_manifest(&path)
                    .map_err(|error| format!("{}: {error}", path.display()))?;
            for provider in &options.providers {
                if manifest.load_path.crate_named(provider).is_none() {
                    return Err(format!(
                        "provider `{provider}` is not on the load path {}",
                        path.display()
                    ));
                }
            }
            Some(identity_of(&path)?)
        }
    };

    let (host, audit_report) = match &options.audit_report {
        None => (BTreeMap::new(), None),
        Some(path) => {
            let path = cwd.join(path);
            let identity = identity_of(&path)?;
            let document: Value = serde_json::from_slice(&read(&path)?)
                .map_err(|error| format!("{}: {error}", path.display()))?;
            let host =
                host_verdicts(&document).map_err(|error| format!("{}: {error}", path.display()))?;
            (host, Some(identity))
        }
    };

    let mut identity = AuditIdentity::default();
    let mut checkpoints = Vec::new();
    for path in &options.checkpoints {
        let path = cwd.join(path);
        let mut reader = crate::checkpoint::CheckpointReader::open(&path)?;
        reader.verify()?;
        let section: crate::checkpoint::IdentitySection = reader.section("identity")?;
        let contracts = serde_json::to_value(&section.run.contracts)
            .map_err(|error| error.to_string())
            .and_then(|value| crate::checkpoint::json_sha256(&value))?;
        let this = AuditIdentity {
            revision: Some(section.run.source_revision.clone()),
            contract_set_sha256: Some(contracts),
            refuted_table_sha256: Some(section.run.contracts.refuted_table_sha256.clone()),
        };
        if checkpoints.is_empty() {
            identity = this;
        } else if identity != this {
            return Err(format!(
                "{} belongs to another run (revision, contract set or refutation table differs)",
                path.display()
            ));
        }
        checkpoints.push(FileIdentity {
            path: path
                .canonicalize()
                .map_err(|error| error.to_string())?
                .display()
                .to_string(),
            sha256: reader.content_sha256()?,
        });
    }

    let executable = executable
        .canonicalize()
        .map_err(|error| format!("{}: {error}", executable.display()))?;
    let executable_sha256 = sha256_hex(&read(&executable)?);

    let mut environment = Vec::new();
    for variable in ENVIRONMENT_ALLOW_LIST
        .iter()
        .map(|name| (*name).to_owned())
        .chain(options.plugin_env.iter().cloned())
    {
        if environment.iter().any(|(name, _)| *name == variable) {
            continue;
        }
        if let Some(value) = std::env::var_os(&variable) {
            environment.push((variable, value));
        }
    }

    let defaults = Quotas::default();
    let request = Request {
        load_path,
        providers: options.providers.clone(),
        identity,
        audit_report,
        checkpoints,
        quotas: Quotas {
            wall_time_ms: options.timeout_ms.unwrap_or(defaults.wall_time_ms),
            stdout_bytes: options.stdout_limit.unwrap_or(defaults.stdout_bytes),
            stderr_bytes: options.stderr_limit.unwrap_or(defaults.stderr_bytes),
        },
        report: report.display().to_string(),
        ..Request::new(name, options.arguments.clone(), cwd, &output)
    };
    let slot_wait_limit_ms = options
        .slot_wait_limit
        .unwrap_or(request.quotas.wall_time_ms);
    Ok(Prepared {
        name: name.to_owned(),
        executable,
        executable_sha256,
        request,
        report,
        stderr_log,
        environment,
        host,
        slot_wait_limit_ms,
    })
}

/// The host's verdicts from an audit report: each unit row's `status` by its
/// `file` (a `data_only` row carries none; a `gateway_failure` is `Unknown`).
///
/// # Errors
///
/// A report without `units`, a row without `file` or `status`, or a status
/// outside the vocabulary.
pub fn host_verdicts(report: &Value) -> Result<BTreeMap<String, ReasonedVerdict>, String> {
    let units = report["units"]
        .as_array()
        .ok_or("the audit report has no `units` array")?;
    let mut host = BTreeMap::<String, ReasonedVerdict>::new();
    for unit in units {
        let file = unit["file"].as_str().ok_or("a unit row has no `file`")?;
        let status = unit["status"]
            .as_str()
            .ok_or("a unit row has no `status`")?;
        let reason = unit["reason"].as_str().unwrap_or("").to_owned();
        let category = unit["reason_code"]
            .as_str()
            .and_then(|code| code.parse::<ReasonCategory>().ok());
        let detail = |fallback: &str| {
            if reason.is_empty() {
                format!("audit report: {file} is {fallback}")
            } else {
                format!("audit report: {reason}")
            }
        };
        let unknown = |category: ReasonCategory, detail: String| {
            ReasonedVerdict::unknown(category, detail)
                .expect("a category that is not human-resolvable")
        };
        let verdict = match status {
            "Safe" => ReasonedVerdict::safe(),
            "Unsafe" => ReasonedVerdict::violation(
                unit["reason_code"].as_str().unwrap_or("audit-report"),
                detail("provisional Unsafe"),
            ),
            "Pending" => match category.and_then(PendingReason::from_category) {
                Some(pending) => ReasonedVerdict::pending(pending, detail("Pending")),
                None => unknown(
                    ReasonCategory::MissingPremise,
                    format!(
                        "audit report: {file} is Pending without a human-resolvable reason code"
                    ),
                ),
            },
            "Unknown" => unknown(
                category
                    .filter(|category| !category.is_human_resolvable())
                    .unwrap_or(ReasonCategory::IncompleteObligationCoverage),
                detail("Unknown"),
            ),
            "gateway_failure" => unknown(
                ReasonCategory::IncompleteObligationCoverage,
                detail("a gateway failure"),
            ),
            "data_only" => continue,
            other => return Err(format!("unit {file}: status `{other}` is not a verdict")),
        };
        let joined = match host.remove(file) {
            Some(existing) => existing.join(verdict),
            None => verdict,
        };
        host.insert(file.to_owned(), joined);
    }
    Ok(host)
}

/// A claim, classified as the firewall classifies a checker plugin's answer.
/// The command line has no trusted checker for a subcommand plugin's
/// certificate or witness, so no claim is certified or validated here.
#[must_use]
pub fn classify_claim(claim: &VerdictClaim) -> PluginClaim {
    match claim.verdict {
        ProposedVerdict::Safe if claim.certificates.is_empty() => PluginClaim::UncertifiedSafe {
            code: "QP008".into(),
            reason: "the claim offers no certificate".into(),
        },
        ProposedVerdict::Safe => PluginClaim::UncertifiedSafe {
            code: "QP016".into(),
            reason: format!(
                "the claim offers {} certificate reference(s), and no trusted-core checker \
                 accepts a subcommand plugin's certificate",
                claim.certificates.len()
            ),
        },
        ProposedVerdict::Unsafe => PluginClaim::UnvalidatedUnsafe {
            code: "QP010".into(),
            reason: "unverified provider allegation: no trusted-core counterexample checker \
                     validates a subcommand plugin's witness"
                .into(),
        },
        ProposedVerdict::Unknown => PluginClaim::Undecided {
            diagnostics: vec![PluginDiagnostic {
                code: "plugin-unknown".into(),
                message: claim.reason.clone(),
                fact_id: None,
            }],
        },
    }
}

fn claim_kind(claim: &PluginClaim) -> &'static str {
    match claim {
        PluginClaim::CertifiedSafe(_) => "certified-safe",
        PluginClaim::ValidatedUnsafe { .. } => "validated-unsafe",
        PluginClaim::UncertifiedSafe { .. } => "uncertified-safe",
        PluginClaim::UnvalidatedUnsafe { .. } => "unvalidated-unsafe",
        PluginClaim::Undecided { .. } => "undecided",
        PluginClaim::Failure { .. } => "failure",
    }
}

/// The result: each row the join of the host's verdict for its artifact, the
/// run's verdict and the classified claims about it; and the join of every
/// row and the run — or `None` when a completed run has no row at all, so a
/// run that claims nothing publishes no verdict (never a vacuous `Safe`).
#[must_use]
pub fn join(
    host: &BTreeMap<String, ReasonedVerdict>,
    run: &ReasonedVerdict,
    claims: &[(String, ReasonedVerdict)],
) -> (BTreeMap<String, ReasonedVerdict>, Option<ReasonedVerdict>) {
    let mut rows: BTreeMap<String, ReasonedVerdict> = host
        .iter()
        .map(|(artifact, verdict)| (artifact.clone(), verdict.clone().join(run.clone())))
        .collect();
    for (artifact, verdict) in claims {
        let row = rows
            .remove(artifact)
            .unwrap_or_else(|| run.clone())
            .join(verdict.clone());
        rows.insert(artifact.clone(), row);
    }
    if rows.is_empty() && run.verdict().is_safe() {
        return (rows, None);
    }
    let aggregate = ReasonedVerdict::fold(rows.values().cloned()).join(run.clone());
    (rows, Some(aggregate))
}

/// Why a supervised run stopped short.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    not(unix),
    expect(dead_code, reason = "process supervision is Unix-only")
)]
enum Breach {
    Timeout,
    Stdout,
    Stderr,
    Escaped,
}

/// What supervision observed.
#[derive(Debug, Default)]
struct Supervised {
    spawn_error: Option<String>,
    breach: Option<Breach>,
    exit_code: Option<i32>,
    signal: Option<i32>,
    success: bool,
    stdout: Vec<u8>,
    stderr_bytes: u64,
    wall_time_ms: u64,
    /// Wall time the plugin declared as waiting for a gate slot
    /// ([`SLOT_WAIT_MARKER`]) and the quota excluded, at most the limit.
    slot_wait_ms: u64,
}

/// The declared slot-wait time a supervised run accrues: the time between
/// two of the host's polls counts when the marker existed at both, never more
/// than the limit in all. The wall-time deadline is the start plus the quota
/// plus what has accrued.
#[cfg(unix)]
#[derive(Debug)]
struct SlotWaits {
    marker: PathBuf,
    limit: Duration,
    accrued: Duration,
    waiting_since: Option<Instant>,
}

#[cfg(unix)]
impl SlotWaits {
    fn new(output: &Path, limit_ms: u64) -> Self {
        Self {
            marker: output.join(SLOT_WAIT_MARKER),
            limit: Duration::from_millis(limit_ms),
            accrued: Duration::ZERO,
            waiting_since: None,
        }
    }

    /// Observe the marker at `now`.
    fn poll(&mut self, now: Instant) {
        let waiting = self.marker.exists();
        if let Some(since) = self.waiting_since.take() {
            self.accrued = (self.accrued + now.saturating_duration_since(since)).min(self.limit);
        }
        if waiting && self.accrued < self.limit {
            self.waiting_since = Some(now);
        }
    }
}

#[cfg(unix)]
fn kill_group(leader: u32) {
    // The leader is not yet reaped when this runs (see `exited`), so its
    // process-group ID cannot name any other group.
    if let Ok(pgid) = i32::try_from(leader) {
        // SAFETY: `kill` with a negative PID signals that process group only.
        unsafe {
            libc::kill(-pgid, libc::SIGKILL);
        }
    }
}

/// Whether the child exited, without reaping it: a zombie keeps its PID and
/// so its process-group ID reserved while the rest of its group is killed.
#[cfg(target_os = "linux")]
fn exited(child: &mut std::process::Child) -> std::io::Result<bool> {
    let pid = libc::id_t::from(child.id());
    // SAFETY: a zeroed `siginfo_t` is a valid out-parameter for `waitid`.
    let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
    // SAFETY: `pid` is this process's own unreaped child; `WNOWAIT` leaves it
    // unreaped.
    let result = unsafe {
        libc::waitid(
            libc::P_PID,
            pid,
            &raw mut info,
            libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
        )
    };
    if result != 0 {
        return Err(std::io::Error::last_os_error());
    }
    // SAFETY: `waitid` filled `info`; `si_pid` is 0 when nothing has exited.
    Ok(unsafe { info.si_pid() } != 0)
}

#[cfg(all(unix, not(target_os = "linux")))]
fn exited(child: &mut std::process::Child) -> std::io::Result<bool> {
    // Without `waitid(WNOWAIT)` the only probe reaps; the post-exit group
    // kill is then skipped (see `supervise`).
    child.try_wait().map(|status| status.is_some())
}

#[cfg(unix)]
fn supervise(prepared: &Prepared, request: &[u8]) -> Supervised {
    use std::os::unix::process::{CommandExt, ExitStatusExt};
    use std::process::{Command, Stdio};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    let quotas = prepared.request.quotas;
    let mut observed = Supervised::default();
    let stderr_file = match fs::File::create(&prepared.stderr_log) {
        Ok(file) => file,
        Err(error) => {
            observed.spawn_error = Some(format!("{}: {error}", prepared.stderr_log.display()));
            return observed;
        }
    };
    let mut command = Command::new(&prepared.executable);
    command
        .args(&prepared.request.arguments)
        .env_clear()
        .envs(
            prepared
                .environment
                .iter()
                .map(|(name, value)| (name, value)),
        )
        .current_dir(&prepared.request.output)
        .process_group(0)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::from(stderr_file));
    // A marker left in a reused output must not excuse this run's time.
    let mut waits = SlotWaits::new(
        Path::new(&prepared.request.output),
        prepared.slot_wait_limit_ms,
    );
    if let Err(error) = fs::remove_file(&waits.marker)
        && error.kind() != std::io::ErrorKind::NotFound
    {
        observed.spawn_error = Some(format!("{}: {error}", waits.marker.display()));
        return observed;
    }
    let start = Instant::now();
    let quota = Duration::from_millis(quotas.wall_time_ms);
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            observed.spawn_error = Some(error.to_string());
            return observed;
        }
    };
    let leader = child.id();
    if let Some(mut stdin) = child.stdin.take() {
        let request = request.to_vec();
        // A plugin that exits without reading breaks the pipe; its exit
        // status and response then decide.
        std::thread::spawn(move || {
            let _ = stdin.write_all(&request);
        });
    }
    let over = Arc::new(AtomicBool::new(false));
    let (sender, receiver) = std::sync::mpsc::channel();
    if let Some(mut stdout) = child.stdout.take() {
        let over = Arc::clone(&over);
        let limit = quotas.stdout_bytes;
        std::thread::spawn(move || {
            let mut buffer = Vec::new();
            let mut chunk = [0u8; 1 << 16];
            loop {
                match stdout.read(&mut chunk) {
                    Ok(0) => break,
                    Ok(count) => {
                        if buffer.len() as u64 + count as u64 > limit {
                            over.store(true, Ordering::SeqCst);
                            break;
                        }
                        buffer.extend_from_slice(&chunk[..count]);
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
                    Err(_) => break,
                }
            }
            let _ = sender.send(buffer);
        });
    }
    let stderr_size = || fs::metadata(&prepared.stderr_log).map_or(0, |metadata| metadata.len());
    loop {
        match exited(&mut child) {
            Ok(true) => break,
            Ok(false) => {}
            Err(error) => {
                observed.spawn_error = Some(format!("waiting for the plugin: {error}"));
                kill_group(leader);
                break;
            }
        }
        let now = Instant::now();
        waits.poll(now);
        let breach = if over.load(Ordering::SeqCst) {
            Some(Breach::Stdout)
        } else if stderr_size() > quotas.stderr_bytes {
            Some(Breach::Stderr)
        } else if now >= start + quota + waits.accrued {
            Some(Breach::Timeout)
        } else {
            None
        };
        if breach.is_some() {
            observed.breach = breach;
            kill_group(leader);
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    // Whatever the plugin left running in its group goes with it, so nothing
    // it spawned outlives the run and its output pipe closes.
    if cfg!(target_os = "linux") {
        kill_group(leader);
    }
    match child.wait() {
        Ok(status) => {
            observed.exit_code = status.code();
            observed.signal = status.signal();
            observed.success = status.success();
        }
        Err(error) => observed.spawn_error = Some(format!("reaping the plugin: {error}")),
    }
    match receiver.recv_timeout(Duration::from_secs(2)) {
        Ok(stdout) => observed.stdout = stdout,
        Err(_) => {
            observed.breach.get_or_insert(Breach::Escaped);
        }
    }
    if observed.breach.is_none() {
        if over.load(Ordering::SeqCst) {
            observed.breach = Some(Breach::Stdout);
        } else if stderr_size() > quotas.stderr_bytes {
            observed.breach = Some(Breach::Stderr);
        }
    }
    observed.stderr_bytes = stderr_size();
    observed.wall_time_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);
    observed.slot_wait_ms = u64::try_from(waits.accrued.as_millis()).unwrap_or(u64::MAX);
    observed
}

#[cfg(not(unix))]
fn supervise(_prepared: &Prepared, _request: &[u8]) -> Supervised {
    Supervised {
        spawn_error: Some(
            "external subcommands need a Unix host: the quota kill is a process-group kill".into(),
        ),
        ..Supervised::default()
    }
}

/// A run's failure, named.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunFailure {
    pub code: &'static str,
    pub reason: String,
}

/// Classify what supervision observed: the response and its verified
/// artifacts, or the named failure.
fn classify_run(
    observed: &Supervised,
    prepared: &Prepared,
) -> (Option<Response>, Result<Vec<VerifiedArtifact>, RunFailure>) {
    let quotas = prepared.request.quotas;
    let fail = |code, reason: String| Err(RunFailure { code, reason });
    if let Some(error) = &observed.spawn_error {
        return (None, fail("plugin-spawn", error.clone()));
    }
    match observed.breach {
        Some(Breach::Timeout) => {
            return (
                None,
                fail(
                    "plugin-timeout",
                    format!(
                        "exceeded the wall-time quota of {} ms ({} ms of declared slot waits \
                         excluded, limit {} ms)",
                        quotas.wall_time_ms, observed.slot_wait_ms, prepared.slot_wait_limit_ms
                    ),
                ),
            );
        }
        Some(Breach::Stdout) => {
            return (
                None,
                fail(
                    "plugin-output-limit",
                    format!(
                        "wrote more than the {}-byte standard-output quota",
                        quotas.stdout_bytes
                    ),
                ),
            );
        }
        Some(Breach::Stderr) => {
            return (
                None,
                fail(
                    "plugin-stderr-limit",
                    format!(
                        "wrote more than the {}-byte standard-error quota",
                        quotas.stderr_bytes
                    ),
                ),
            );
        }
        Some(Breach::Escaped) => {
            return (
                None,
                fail(
                    "plugin-escaped-descendant",
                    "its standard output stayed open after its process group was killed: a \
                     descendant left the group"
                        .into(),
                ),
            );
        }
        None => {}
    }
    let parsed = parse_response(&observed.stdout);
    if !observed.success {
        let detail = match &parsed {
            Ok(Response {
                outcome: Outcome::Failed(reason),
                ..
            }) => format!(": {reason}"),
            _ => String::new(),
        };
        let status = observed.exit_code.map_or_else(
            || format!("signal {}", observed.signal.unwrap_or_default()),
            |code| format!("exit {code}"),
        );
        return (
            parsed.ok(),
            fail(
                "plugin-exit",
                format!("the plugin ended with {status}{detail}"),
            ),
        );
    }
    let response = match parsed {
        Ok(response) => response,
        Err(error) => return (None, fail(error.code(), error.to_string())),
    };
    if let Outcome::Failed(reason) = &response.outcome {
        let reason = reason.clone();
        return (Some(response), fail("plugin-reported-failure", reason));
    }
    let output = PathBuf::from(&prepared.request.output);
    let reserved = [
        prepared.report.clone(),
        prepared.report.with_extension("invocation.json"),
        prepared.stderr_log.clone(),
    ];
    match verify_artifacts(&response, &output, &reserved) {
        Ok(artifacts) => (Some(response), Ok(artifacts)),
        Err(error) => (Some(response), fail("plugin-artifact", error)),
    }
}

/// The plugin's identity, as every plugin-sourced verdict carries it.
fn plugin_label(prepared: &Prepared) -> String {
    format!(
        "{}@{}",
        executable_name(&prepared.name),
        &prepared.executable_sha256[..16]
    )
}

/// Run the plugin, classify, join, and write the report and the invocation
/// record. Returns whether the run completed.
///
/// # Errors
///
/// A record that could not be written.
pub fn execute(
    prepared: &Prepared,
    argv: &[String],
    directories: &[PathBuf],
) -> Result<bool, String> {
    let request_bytes = serde_json::to_vec(&prepared.request).map_err(|error| error.to_string())?;
    let observed = supervise(prepared, &request_bytes);
    let (response, classified) = classify_run(&observed, prepared);
    let label = plugin_label(prepared);
    let plugin = json!({
        "name": prepared.name,
        "executable": prepared.executable.display().to_string(),
        "sha256": prepared.executable_sha256,
        "label": label,
    });

    let (run, artifacts, failure) = match &classified {
        Ok(artifacts) => (ReasonedVerdict::safe(), artifacts.clone(), None),
        Err(failure) => (
            PluginClaim::Failure {
                code: failure.code.into(),
                reason: failure.reason.clone(),
            }
            .verdict(&label),
            Vec::new(),
            Some(failure.clone()),
        ),
    };
    // Claims count only from a completed run; a failed run's claims are
    // recorded and join nothing (the run is `Unknown`, above every Pending).
    let completed = failure.is_none();
    let mut claim_rows = Vec::new();
    let mut joined_claims = Vec::new();
    for claim in response.iter().flat_map(|response| &response.claims) {
        let classified = classify_claim(claim);
        let verdict = classified.verdict(&label);
        claim_rows.push(json!({
            "claim": claim,
            "plugin": plugin,
            "firewall": {
                "claim": claim_kind(&classified),
                "code": classified.code(),
                "verdict": verdict.verdict(),
                "reasons": verdict.reasons(),
            },
            "joined": completed,
        }));
        if completed {
            joined_claims.push((claim.artifact.clone(), verdict));
        }
    }
    let (rows, aggregate) = join(&prepared.host, &run, &joined_claims);
    let rows = rows
        .iter()
        .map(|(artifact, verdict)| {
            json!({
                "artifact": artifact,
                "host": prepared.host.get(artifact).map(|host| json!({
                    "verdict": host.verdict(), "reasons": host.reasons(),
                })),
                "verdict": verdict.verdict(),
                "reasons": verdict.reasons(),
            })
        })
        .collect::<Vec<_>>();
    let protocol_version = response.as_ref().map(|response| response.protocol_version);
    let run_record = json!({
        "outcome": if completed { "completed" } else { "failed" },
        "code": failure.as_ref().map(|failure| failure.code),
        "reason": failure.as_ref().map(|failure| failure.reason.clone()),
        "verdict": run.verdict(),
        "reasons": run.reasons(),
        "exit_code": observed.exit_code,
        "signal": observed.signal,
        "wall_time_ms": observed.wall_time_ms,
        "slot_wait_ms": observed.slot_wait_ms,
        "analysis_time_ms": observed.wall_time_ms.saturating_sub(observed.slot_wait_ms),
        "stdout_bytes": observed.stdout.len(),
        "stderr_bytes": observed.stderr_bytes,
        "stderr_log": prepared.stderr_log.display().to_string(),
    });
    let report = json!({
        "schema_version": 1,
        "route": "external-subcommand",
        "meaning": MEANING,
        "plugin": plugin,
        "protocol_version": PROTOCOL_VERSION,
        "response_protocol_version": protocol_version,
        "request": prepared.request,
        "run": run_record,
        "artifacts": artifacts,
        "diagnostics": response.as_ref().map(|response| response.diagnostics.clone()),
        "claims": claim_rows,
        "rows": rows,
        "verdict": aggregate.as_ref().map(ReasonedVerdict::verdict),
        "reasons": aggregate.as_ref().map(ReasonedVerdict::reasons),
    });
    let mut bytes = serde_json::to_vec_pretty(&report).map_err(|error| error.to_string())?;
    bytes.push(b'\n');
    fs::write(&prepared.report, bytes)
        .map_err(|error| format!("{}: {error}", prepared.report.display()))?;
    let invocation = json!({
        "schema_version": 1,
        "route": "external-subcommand",
        "argv": argv,
        "cwd": prepared.request.invocation_directory,
        "subcommand": prepared.name,
        "search_path": directories.iter().map(|directory| directory.display().to_string()).collect::<Vec<_>>(),
        "plugin": {
            "name": prepared.name,
            "executable": prepared.executable.display().to_string(),
            "sha256": prepared.executable_sha256,
            "protocol_version": PROTOCOL_VERSION,
            "response_protocol_version": protocol_version,
            "exit_code": observed.exit_code,
            "signal": observed.signal,
        },
        "environment": {
            "cleared": true,
            "passed": prepared.environment.iter().map(|(name, _)| name.clone()).collect::<Vec<_>>(),
        },
        "quotas": prepared.request.quotas,
        "slot_wait_limit_ms": prepared.slot_wait_limit_ms,
        "output": prepared.request.output,
        "stderr_log": prepared.stderr_log.display().to_string(),
        "request_sha256": sha256_hex(&request_bytes),
        "outcome": if completed { "completed" } else { "failed" },
        "verdict": aggregate.as_ref().map(ReasonedVerdict::verdict),
    });
    crate::current_route::write_invocation(&prepared.report, &invocation)?;

    println!(
        "subcommand `{}` ({}): {}; {} artifact(s), {} claim(s); verdict {}",
        prepared.name,
        label,
        failure.as_ref().map_or_else(
            || "completed".to_owned(),
            |failure| format!("failed ({}: {})", failure.code, failure.reason)
        ),
        artifacts.len(),
        claim_rows.len(),
        aggregate.as_ref().map_or_else(
            || "none (no claim, no host row)".to_owned(),
            |aggregate| { aggregate.verdict().to_string() }
        )
    );
    for diagnostic in response.iter().flat_map(|response| &response.diagnostics) {
        println!(
            "  {:?} {}: {}",
            diagnostic.severity, diagnostic.code, diagnostic.message
        );
    }
    println!("report: {}", prepared.report.display());
    Ok(completed)
}

/// What the report means, carried in it.
pub const MEANING: &str = "an external quirin-audit subcommand (card W45-AUDIT-SUBCOMMAND-PLUGINS, contract spec C11.2): a quirin-audit-NAME executable found only on the declared plugin search path, run out of process with a cleared environment, its working directory the declared output, under wall-time and output quotas enforced by a process-group kill; its artifacts are accepted only inside the output with their declared SHA-256; each verdict claim is classified by the checker-plugin firewall (an uncertified Safe or Unsafe is Pending, plugin-claim-uncertified; an Unknown is Unknown) and a spawn failure, timeout, quota breach, nonzero exit, malformed response, protocol-version mismatch, reported failure or unverified artifact makes the run Unknown; each row is the join (Safe < Pending < Unknown < Unsafe) of the host's verdict from --audit-report, the run's and the claims about its artifact, and `verdict` is the join of every row and the run (null when a completed run has no row: no claim publishes no verdict)";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_options_and_builtins_are_told_apart() {
        assert!(claims("libevent"));
        assert!(claims("--plugin-path"));
        assert!(claims("--list-plugins"));
        assert!(!claims("plugin"));
        assert!(!claims("prepare"));
        assert!(!claims("config.json"));
        assert!(!claims("./config"));
        assert!(!claims("--jobs"));
        assert!(!is_subcommand_name("-x"));
        assert!(!is_subcommand_name("Upper"));
        assert!(!is_subcommand_name(""));
    }

    #[test]
    fn every_builtin_of_main_is_listed() {
        let main = include_str!("main.rs");
        let mut found = std::collections::BTreeSet::new();
        for piece in main.split("path == \"").skip(1) {
            if let Some(end) = piece.find('"') {
                found.insert(piece[..end].to_owned());
            }
        }
        let listed: std::collections::BTreeSet<String> =
            BUILTINS.iter().map(|name| (*name).to_owned()).collect();
        assert_eq!(found, listed);
    }

    #[test]
    fn arguments_after_the_name_are_verbatim() {
        let options = parse(&[
            "--plugin-path".into(),
            "a".into(),
            "--output".into(),
            "o".into(),
            "tool".into(),
            "--output".into(),
            "--plugin-path".into(),
        ])
        .unwrap();
        assert_eq!(options.name.as_deref(), Some("tool"));
        assert_eq!(options.arguments, vec!["--output", "--plugin-path"]);
        assert!(parse(&["--output".into(), "o".into()]).is_err());
        assert!(parse(&["--list-plugins".into(), "x".into()]).is_err());
        assert!(parse(&["--timeout-ms".into(), "0".into(), "x".into()]).is_err());
        // A slot-wait limit of 0 is meaningful: every wait counts.
        let counted = parse(&["--slot-wait-limit-ms".into(), "0".into(), "x".into()]).unwrap();
        assert_eq!(counted.slot_wait_limit, Some(0));
        assert!(parse(&["--slot-wait-limit-ms".into(), "-1".into(), "x".into()]).is_err());
        assert!(parse(&["--bogus".into(), "x".into()]).is_err());
    }

    #[test]
    fn path_is_never_searched() {
        let directories = search_path(&[], None, Path::new("/"));
        assert!(directories.is_empty());
        assert_eq!(find("ls", &directories), None);
    }

    fn claim(verdict: ProposedVerdict, certificates: usize) -> VerdictClaim {
        VerdictClaim {
            artifact: "a.c".into(),
            property: "p".into(),
            semantic_profile: "s".into(),
            scope: "x".into(),
            bounds: "b".into(),
            assumptions: BTreeMap::new(),
            evidence_version: 1,
            verdict,
            reason: "r".into(),
            certificates: (0..certificates)
                .map(|index| quirin_audit_plugin::CertificateReference {
                    kind: "k".into(),
                    path: format!("c{index}"),
                    sha256: "0".repeat(64),
                })
                .collect(),
        }
    }

    #[test]
    fn no_claim_is_certified_on_the_command_line() {
        use qcore_schemas::verdict::Verdict;
        for (verdict, expected) in [
            (ProposedVerdict::Safe, Verdict::Pending),
            (ProposedVerdict::Unsafe, Verdict::Pending),
            (ProposedVerdict::Unknown, Verdict::Unknown),
        ] {
            for certificates in [0, 1] {
                let classified = classify_claim(&claim(verdict, certificates));
                assert_eq!(classified.verdict("p").verdict(), expected);
            }
        }
    }

    #[test]
    fn the_join_never_lowers_a_host_verdict() {
        use qcore_schemas::verdict::Verdict;
        let host = BTreeMap::from([
            ("safe.c".to_owned(), ReasonedVerdict::safe()),
            (
                "bad.c".to_owned(),
                ReasonedVerdict::violation("leak", "a leak"),
            ),
        ]);
        let failed = PluginClaim::Failure {
            code: "plugin-timeout".into(),
            reason: "slow".into(),
        }
        .verdict("p");
        let (rows, aggregate) = join(&host, &failed, &[]);
        assert_eq!(rows["safe.c"].verdict(), Verdict::Unknown);
        assert_eq!(rows["bad.c"].verdict(), Verdict::Unsafe);
        assert_eq!(aggregate.unwrap().verdict(), Verdict::Unsafe);
        let pending = classify_claim(&claim(ProposedVerdict::Safe, 0)).verdict("p");
        let claims = vec![
            ("bad.c".to_owned(), pending.clone()),
            ("safe.c".to_owned(), pending),
        ];
        let (rows, _) = join(&host, &ReasonedVerdict::safe(), &claims);
        assert_eq!(rows["bad.c"].verdict(), Verdict::Unsafe);
        assert_eq!(rows["safe.c"].verdict(), Verdict::Pending);
        let (rows, aggregate) = join(&BTreeMap::new(), &ReasonedVerdict::safe(), &[]);
        assert!(rows.is_empty());
        assert!(aggregate.is_none());
        let failed = PluginClaim::Failure {
            code: "plugin-exit".into(),
            reason: "exit 3".into(),
        }
        .verdict("p");
        let (_, aggregate) = join(&BTreeMap::new(), &failed, &[]);
        assert_eq!(aggregate.unwrap().verdict(), Verdict::Unknown);
    }

    #[test]
    fn audit_report_rows_become_host_verdicts() {
        use qcore_schemas::verdict::Verdict;
        let report = json!({"units": [
            {"file": "a.c", "status": "Safe"},
            {"file": "b.c", "status": "Unsafe", "reason": "leak", "reason_code": "residual-custody"},
            {"file": "c.c", "status": "Pending", "reason_code": "awaiting-human-review"},
            {"file": "d.c", "status": "Pending", "reason_code": null},
            {"file": "e.c", "status": "data_only"},
            {"file": "f.c", "status": "gateway_failure"},
            {"file": "a.c", "status": "Unknown", "reason_code": "unsupported-syntax"},
        ]});
        let host = host_verdicts(&report).unwrap();
        assert_eq!(host["a.c"].verdict(), Verdict::Unknown);
        assert_eq!(host["b.c"].verdict(), Verdict::Unsafe);
        assert_eq!(host["c.c"].verdict(), Verdict::Pending);
        assert_eq!(host["d.c"].verdict(), Verdict::Unknown);
        assert!(!host.contains_key("e.c"));
        assert_eq!(host["f.c"].verdict(), Verdict::Unknown);
        assert!(host_verdicts(&json!({"units": [{"file": "x", "status": "Maybe"}]})).is_err());
        assert!(host_verdicts(&json!({})).is_err());
    }
}
