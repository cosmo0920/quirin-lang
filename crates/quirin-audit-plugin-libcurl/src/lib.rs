// SPDX-License-Identifier: Apache-2.0
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! `quirin-audit libcurl` (card **W48-LIBCURL-CONTRACT-PLUGIN**, user
//! decision 2026-09-28): libcurl's contracts, checked by the plugin's own
//! checker against a pinned libcurl release, entering the audit only through
//! the plugin firewall.
//!
//! ```text
//! quirin-audit --plugin-path DIR --output OUT libcurl prepare --tarball FILE
//! quirin-audit --plugin-path DIR --output OUT libcurl check [--prepared DIR]
//! quirin-audit --plugin-path DIR --output OUT libcurl install [--prepared DIR]
//!     [--repository DIR] [--checker FILE]
//! ```
//!
//! * `prepare` accepts the release tarball only with its pinned SHA-256
//!   ([`pin`]), unpacks it under the output (`libcurl-source/`), checks every
//!   file the checker reads against its pinned digest and writes
//!   `libcurl-preparation.json`. It reads no network: the tarball is given.
//! * `check` runs the checker ([`checker`]) over a prepared tree and answers
//!   one verdict claim per binding (artifact `libcurl:FUNCTION`), with its
//!   reasons and the record `libcurl-check.json` as the claim's certificate
//!   reference. The host classifies each claim (contract specification
//!   C11.2): an uncertified `Safe` is `Pending`, an `Unknown` is `Unknown`.
//! * `install` writes the user contract crate `quirin-provider-libcurl`
//!   (the module `libcurl/libcurl.qv`), the C11.1 checker package bound to
//!   each contracted function, and a user load path extending the
//!   repository's shipped one, so `quirin-audit contracts|current
//!   --load-path user/user-load-path.json --provider quirin-provider-libcurl`
//!   applies the contracts and joins each binding's classified verdict into
//!   every function that applies it.
//!
//! Fail-closed throughout: a tarball or a tree that does not match the pin,
//! or anything the plugin cannot do, fails the run, which the host
//! publishes `Unknown` with no claim joined; the checker package answers
//! `Unknown` for every function when its installed files do not match the
//! pin.

use std::path::{Path, PathBuf};

use quirin_audit_plugin::{
    CertificateReference, ProposedVerdict, Request, Response, Severity, VerdictClaim,
};
use serde_json::{Value, json};

pub mod checker;
pub mod pin;
pub mod provider;
pub mod source;

use checker::Answer;

/// Usage text.
pub const USAGE: &str = "usage: quirin-audit --output DIR libcurl prepare --tarball FILE\n       \
quirin-audit --output DIR libcurl check [--prepared DIR]\n       \
quirin-audit --output DIR libcurl install [--prepared DIR] [--repository DIR] [--checker FILE]";

/// Where `prepare` unpacks the release, under the output.
pub const SOURCE_ROOT: &str = "libcurl-source";

/// The preparation record.
pub const PREPARATION_RECORD: &str = "libcurl-preparation.json";

/// The check record.
pub const CHECK_RECORD: &str = "libcurl-check.json";

/// The contract module the plugin carries.
pub const MODULE: &str = "libcurl/libcurl.qv";
pub const MODULE_TEXT: &str = include_str!("../contracts/libcurl/libcurl.qv");

/// The user contract crate `install` writes.
pub const USER_CRATE: &str = "quirin-provider-libcurl";
pub const USER_NAMESPACE: &str = "libcurl";
pub const USER_VERSION: &str = "0.1.0";

/// The checker package's directory, inside the user crate.
pub const PACKAGE: &str = "checkers/libcurl-check";

/// The checker executable, built beside this plugin.
pub const CHECKER_EXECUTABLE: &str = "quirin-libcurl-checker";

/// The semantic profile of a claim.
pub const SEMANTIC_PROFILE: &str = "quirin.libcurl-contract-check.v1";

/// The property a claim is about.
pub const PROPERTY: &str = "libcurl-contract-conformance";

/// The flags of a step.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Options {
    pub tarball: Option<String>,
    pub prepared: Option<String>,
    pub repository: Option<String>,
    pub checker: Option<String>,
}

/// Parse a step's flags against the ones it admits.
///
/// # Errors
///
/// An unknown, repeated or valueless flag.
pub fn parse(arguments: &[String], admitted: &[&str]) -> Result<Options, String> {
    let mut options = Options::default();
    let mut index = 0;
    while index < arguments.len() {
        let flag = arguments[index].as_str();
        if !admitted.contains(&flag) {
            return Err(format!("unknown argument `{flag}`\n{USAGE}"));
        }
        let slot = match flag {
            "--tarball" => &mut options.tarball,
            "--prepared" => &mut options.prepared,
            "--repository" => &mut options.repository,
            "--checker" => &mut options.checker,
            other => return Err(format!("unknown argument `{other}`\n{USAGE}")),
        };
        let value = arguments
            .get(index + 1)
            .ok_or_else(|| format!("{flag} requires a value\n{USAGE}"))?;
        if slot.replace(value.clone()).is_some() {
            return Err(format!("{flag} is given twice"));
        }
        index += 2;
    }
    Ok(options)
}

/// Answer a request.
///
/// # Errors
///
/// A refusal or failure, with its reason (the response is then `failed`).
pub fn answer(request: &Request) -> Result<Response, String> {
    match request.arguments.split_first() {
        Some((step, rest)) if step == "prepare" => prepare(request, &parse(rest, &["--tarball"])?),
        Some((step, rest)) if step == "check" => check(request, &parse(rest, &["--prepared"])?),
        Some((step, rest)) if step == "install" => install(
            request,
            &parse(rest, &["--prepared", "--repository", "--checker"])?,
        ),
        _ => Err(USAGE.into()),
    }
}

fn write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("{}: {error}", parent.display()))?;
    }
    std::fs::write(path, bytes).map_err(|error| format!("{}: {error}", path.display()))
}

fn json_bytes(value: &Value) -> Vec<u8> {
    let mut text = serde_json::to_string_pretty(value).expect("a JSON value serializes");
    text.push('\n');
    text.into_bytes()
}

/// The prepared tree: `--prepared DIR` (against the invocation directory),
/// else `<output>/libcurl-source/curl-8.18.0`.
fn prepared(request: &Request, options: &Options) -> PathBuf {
    options.prepared.as_deref().map_or_else(
        || {
            request
                .output_directory()
                .join(SOURCE_ROOT)
                .join(pin::SOURCE_DIRECTORY)
        },
        |path| request.resolve_argument(path),
    )
}

fn pinned_files() -> Value {
    json!(
        pin::FILES
            .iter()
            .map(|(file, digest)| json!({"path": file, "sha256": digest}))
            .collect::<Vec<_>>()
    )
}

fn pin_record() -> Value {
    json!({
        "version": pin::VERSION,
        "tarball": pin::TARBALL,
        "tarball_sha256": pin::TARBALL_SHA256,
        "tag": pin::TAG,
        "tag_commit": pin::TAG_COMMIT,
        "source_directory": pin::SOURCE_DIRECTORY,
    })
}

/// `prepare --tarball FILE`.
///
/// # Errors
///
/// A missing flag, a tarball whose digest is not the pin, a failed unpack,
/// an unpacked tree that does not match the pin, or an existing output that
/// does not verify.
pub fn prepare(request: &Request, options: &Options) -> Result<Response, String> {
    let tarball = request.resolve_argument(
        options
            .tarball
            .as_deref()
            .ok_or_else(|| format!("prepare requires --tarball FILE\n{USAGE}"))?,
    );
    let bytes =
        std::fs::read(&tarball).map_err(|error| format!("{}: {error}", tarball.display()))?;
    let digest = qcore_schemas::sha256_hex(&bytes);
    if digest != pin::TARBALL_SHA256 {
        return Err(format!(
            "{} has SHA-256 {digest}, not the pinned libcurl {} release tarball {} ({})",
            tarball.display(),
            pin::VERSION,
            pin::TARBALL,
            pin::TARBALL_SHA256
        ));
    }
    let output = request.output_directory();
    let root = output.join(SOURCE_ROOT);
    let tree = root.join(pin::SOURCE_DIRECTORY);
    if root.exists() {
        let mismatches = pin::verify_tree(&tree);
        if !mismatches.is_empty() {
            return Err(format!(
                "{} exists and is not the pinned release ({}); remove it or choose another \
                 output",
                root.display(),
                pin::describe(&mismatches)
            ));
        }
        eprintln!(
            "libcurl: reusing {} (every pinned file verifies)",
            tree.display()
        );
    } else {
        std::fs::create_dir_all(&root).map_err(|error| format!("{}: {error}", root.display()))?;
        let status = std::process::Command::new("tar")
            .arg("-xzf")
            .arg(&tarball)
            .arg("-C")
            .arg(&root)
            .status()
            .map_err(|error| format!("tar: {error}"))?;
        if !status.success() {
            return Err(format!("tar -xzf {} exited {status}", tarball.display()));
        }
        let mismatches = pin::verify_tree(&tree);
        if !mismatches.is_empty() {
            return Err(format!(
                "the unpacked tree is not the pinned release: {}",
                pin::describe(&mismatches)
            ));
        }
    }
    let record = json!({
        "schema": "quirin-libcurl-preparation/1",
        "pin": pin_record(),
        "tarball": {"path": tarball.display().to_string(), "sha256": digest},
        "tree": format!("{SOURCE_ROOT}/{}", pin::SOURCE_DIRECTORY),
        "files": pinned_files(),
        "meaning": "the release tarball had its pinned SHA-256 and every file the checker reads \
                    has its pinned digest; this records provenance and claims no verdict",
    });
    write(&output.join(PREPARATION_RECORD), &json_bytes(&record))?;
    let mut response = Response::completed();
    response.push_artifact(&output, Path::new(PREPARATION_RECORD), "preparation-record")?;
    response.push_diagnostic(
        Severity::Info,
        "libcurl-prepared",
        format!("libcurl {} prepared at {}", pin::VERSION, tree.display()),
    );
    Ok(response)
}

fn assumptions() -> std::collections::BTreeMap<String, String> {
    checker::ASSUMPTIONS
        .iter()
        .map(|(name, text)| ((*name).to_owned(), (*text).to_owned()))
        .collect()
}

/// The check record: every binding's answer and evidence.
fn check_record(outcomes: &[checker::Outcome], tree: &checker::Tree) -> Value {
    json!({
        "schema": "quirin-libcurl-check/1",
        "card": "W48-LIBCURL-CONTRACT-PLUGIN",
        "pin": pin_record(),
        "files": pinned_files(),
        "module": {"path": MODULE, "sha256": qcore_schemas::sha256_hex(MODULE_TEXT.as_bytes())},
        "assumptions": assumptions(),
        "bindings": outcomes.iter().map(checker::outcome_json).collect::<Vec<_>>(),
        "typecheck_stubs": checker::stub_evidence(tree),
        "meaning": "the plugin's own checker: a binding is Safe exactly when the tree is the \
                    pinned release and every lexical obligation of its contract holds, otherwise \
                    Unknown with the failed obligation or the reason no contract is written; the \
                    host's firewall publishes an uncertified Safe as Pending",
    })
}

/// `check [--prepared DIR]`.
///
/// # Errors
///
/// A prepared tree that does not match the pin.
pub fn check(request: &Request, options: &Options) -> Result<Response, String> {
    let root = prepared(request, options);
    let tree = checker::Tree::open(&root)?;
    let outcomes = checker::answer_all(&tree);
    let output = request.output_directory();
    let record = json_bytes(&check_record(&outcomes, &tree));
    write(&output.join(CHECK_RECORD), &record)?;
    let record_sha256 = qcore_schemas::sha256_hex(&record);
    let mut response = Response::completed();
    response.push_artifact(&output, Path::new(CHECK_RECORD), "check-record")?;
    for outcome in &outcomes {
        response.push_claim(claim(outcome, &record_sha256));
    }
    let safe = outcomes
        .iter()
        .filter(|outcome| outcome.answer == Answer::Safe)
        .count();
    response.push_diagnostic(
        Severity::Info,
        "libcurl-checked",
        format!(
            "{} binding(s): {safe} established (published Pending: uncertified), {} Unknown",
            outcomes.len(),
            outcomes.len() - safe
        ),
    );
    Ok(response)
}

/// The claim for one binding.
#[must_use]
pub fn claim(outcome: &checker::Outcome, record_sha256: &str) -> VerdictClaim {
    VerdictClaim {
        artifact: format!("libcurl:{}", outcome.function),
        property: PROPERTY.into(),
        semantic_profile: SEMANTIC_PROFILE.into(),
        scope: format!(
            "the definition of `{}` in libcurl {} ({}), read lexically against its resource-v1 \
             contract in {MODULE}{}",
            outcome.function,
            pin::VERSION,
            pin::TAG,
            if outcome.contracted {
                ""
            } else {
                " (none is written: see the reason)"
            }
        ),
        bounds: "lexical obligations over the pinned files' tokens; the named helpers only; no \
                 path enumeration, no preprocessing"
            .into(),
        assumptions: assumptions(),
        evidence_version: 1,
        verdict: match outcome.answer {
            Answer::Safe => ProposedVerdict::Safe,
            Answer::Unknown => ProposedVerdict::Unknown,
        },
        reason: outcome.reasons.join("; "),
        certificates: vec![CertificateReference {
            kind: "libcurl-check-record".into(),
            path: CHECK_RECORD.into(),
            sha256: record_sha256.into(),
        }],
    }
}

/// The repository whose shipped load path the user load path extends:
/// `--repository DIR`, else the request's load path's repository, else the
/// invocation directory.
fn repository(request: &Request, flag: Option<&str>) -> Result<PathBuf, String> {
    let from_load_path = || {
        let load_path = Path::new(&request.load_path.as_ref()?.path);
        let providers = load_path.parent()?;
        (load_path.file_name()? == "contract-load-path.json"
            && providers.file_name()? == "providers")
            .then(|| providers.parent().map(Path::to_owned))
            .flatten()
    };
    let candidate = match flag {
        Some(flag) => request.resolve_argument(flag),
        None => from_load_path().unwrap_or_else(|| PathBuf::from(&request.invocation_directory)),
    };
    let root = candidate
        .canonicalize()
        .map_err(|error| format!("{}: {error}", candidate.display()))?;
    if !root.join("providers/contract-load-path.json").is_file() {
        return Err(format!(
            "{} has no providers/contract-load-path.json; pass --repository DIR",
            root.display()
        ));
    }
    Ok(root)
}

/// The contracted functions, sorted: one checker binding each.
#[must_use]
pub fn contracted_functions() -> Vec<&'static str> {
    let mut functions: Vec<&'static str> = checker::BINDINGS
        .iter()
        .filter(|binding| matches!(binding.kind, checker::Kind::Contracted { .. }))
        .map(|binding| binding.function)
        .collect();
    functions.sort_unstable();
    functions
}

/// The user load path `install` writes, extending `shipped`.
#[must_use]
pub fn user_load_path(shipped: &Path) -> Value {
    json!({
        "schema_version": 1,
        "extends": shipped.display().to_string(),
        "crates": [{
            "name": USER_CRATE,
            "namespace": USER_NAMESPACE,
            "version": USER_VERSION,
            "contracts": format!("{USER_CRATE}/contracts"),
            "dependencies": [],
            "modules": [MODULE],
            "published": {},
            "resource_view": [MODULE],
        }],
        "overrides": [],
        "checkers": contracted_functions().iter().map(|function| json!({
            "crate": USER_CRATE,
            "module": MODULE,
            "functions": [function],
            "package": format!("{USER_CRATE}/{PACKAGE}"),
        })).collect::<Vec<_>>(),
    })
}

/// Write the user contract crate, its checker package (the executable, its
/// manifest and `files` under `source/`) and the user load path extending
/// `shipped` into `user`. Returns the load path. [`install`] calls it only
/// with a tree verified against the pin; the checker package re-verifies
/// the files it is given on every request, so a package written with other
/// files answers `Unknown` for every function.
///
/// # Errors
///
/// An existing crate directory, or a write failure.
pub fn write_user_crate(
    user: &Path,
    executable: &[u8],
    files: &[(&str, Vec<u8>)],
    shipped: &Path,
) -> Result<PathBuf, String> {
    let crate_root = user.join(USER_CRATE);
    if crate_root.exists() {
        return Err(format!(
            "{} exists; install writes a fresh crate",
            crate_root.display()
        ));
    }
    write(
        &crate_root.join("Cargo.toml"),
        format!(
            "[package]\nname = \"{USER_CRATE}\"\nversion = \"{USER_VERSION}\"\nedition = \"2024\"\n\
             publish = false\ndescription = \"libcurl {} contracts, installed by `quirin-audit \
             libcurl install` (card W48-LIBCURL-CONTRACT-PLUGIN)\"\n\n[lib]\npath = \
             \"src/lib.rs\"\n",
            pin::VERSION
        )
        .as_bytes(),
    )?;
    write(
        &crate_root.join("src/lib.rs"),
        format!(
            "//! libcurl {} contracts (`quirin-audit libcurl install`).\npub const NAMESPACE: \
             &str = \"{USER_NAMESPACE}\";\npub const PUBLISHED: &[&str] = &[];\n",
            pin::VERSION
        )
        .as_bytes(),
    )?;
    write(
        &crate_root.join("contracts").join(MODULE),
        MODULE_TEXT.as_bytes(),
    )?;
    write(
        &crate_root.join("quirin-plugin.json"),
        &json_bytes(&json!({
            "schema_version": 1,
            "checkers": contracted_functions().iter().map(|function| json!({
                "module": MODULE, "functions": [function], "package": PACKAGE,
            })).collect::<Vec<_>>(),
            "overrides": [],
        })),
    )?;
    // The checker package: the executable, its manifest (the template with
    // the executable's digest) and the files it reads.
    let package = crate_root.join(PACKAGE);
    write(&package.join(CHECKER_EXECUTABLE), executable)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(
            package.join(CHECKER_EXECUTABLE),
            std::fs::Permissions::from_mode(0o755),
        )
        .map_err(|error| error.to_string())?;
    }
    let mut manifest: Value =
        serde_json::from_str(provider::TEMPLATE).map_err(|error| error.to_string())?;
    manifest["package_digest"] = json!(qcore_schemas::sha256_hex(executable));
    write(
        &package.join("verifier-plugin.json"),
        serde_json::to_string(&manifest)
            .map_err(|error| error.to_string())?
            .as_bytes(),
    )?;
    for (file, bytes) in files {
        write(&package.join(provider::SOURCE_INPUT).join(file), bytes)?;
    }
    let load_path = user.join("user-load-path.json");
    write(&load_path, &json_bytes(&user_load_path(shipped)))?;
    Ok(load_path)
}

/// `install [--prepared DIR] [--repository DIR] [--checker FILE]`.
///
/// # Errors
///
/// A prepared tree that does not match the pin, a repository without a
/// shipped load path, a missing checker executable, or a write failure.
pub fn install(request: &Request, options: &Options) -> Result<Response, String> {
    let root = prepared(request, options);
    let tree = checker::Tree::open(&root)?;
    let repository = repository(request, options.repository.as_deref())?;
    let checker_executable = match options.checker.as_deref() {
        Some(path) => request.resolve_argument(path),
        None => std::env::current_exe()
            .map_err(|error| error.to_string())?
            .parent()
            .ok_or("the plugin executable has no directory")?
            .join(CHECKER_EXECUTABLE),
    };
    let executable = std::fs::read(&checker_executable).map_err(|error| {
        format!(
            "{}: {error} (build it with `cargo build -p quirin-audit-plugin-libcurl`, or pass \
             --checker FILE)",
            checker_executable.display()
        )
    })?;
    let output = request.output_directory();
    let mut files = Vec::new();
    for (file, _) in pin::FILES {
        let bytes = std::fs::read(root.join(file)).map_err(|error| format!("{file}: {error}"))?;
        files.push((*file, bytes));
    }
    drop(tree);
    let user = output.join("user");
    let load_path = write_user_crate(
        &user,
        &executable,
        &files,
        &repository.join("providers/contract-load-path.json"),
    )?;
    let functions = contracted_functions();
    let mut response = Response::completed();
    response.push_artifact(
        &output,
        Path::new("user/user-load-path.json"),
        "user-load-path",
    )?;
    response.push_artifact(
        &output,
        &Path::new("user")
            .join(USER_CRATE)
            .join("contracts")
            .join(MODULE),
        "contract-module",
    )?;
    response.push_artifact(
        &output,
        &Path::new("user")
            .join(USER_CRATE)
            .join(PACKAGE)
            .join("verifier-plugin.json"),
        "checker-manifest",
    )?;
    response.push_artifact(
        &output,
        &Path::new("user")
            .join(USER_CRATE)
            .join(PACKAGE)
            .join(CHECKER_EXECUTABLE),
        "checker-executable",
    )?;
    response.push_diagnostic(
        Severity::Info,
        "libcurl-installed",
        format!(
            "user crate {USER_CRATE} with {} bound contract(s); run `quirin-audit contracts \
             --load-path {} --provider {USER_CRATE} ...`",
            functions.len(),
            load_path.display()
        ),
    );
    Ok(response)
}
