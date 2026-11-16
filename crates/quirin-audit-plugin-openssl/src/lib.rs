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

//! `quirin-audit openssl` (card **W48-OPENSSL-CONTRACT-PLUGIN**; standing
//! constraint 52): OpenSSL's contracts, checked by this plugin's own checker
//! against the pinned release Fluent Bit builds against, answered per binding
//! through the host's plugin firewall (rule 10h, contract specification
//! §C11.2).
//!
//! ```text
//! quirin-audit --plugin-path DIR --output OUT openssl prepare --tarball FILE \
//!     [--clang PATH] [--perl PATH] [--make PATH] [--jobs N]
//! quirin-audit --plugin-path DIR --output OUT openssl check \
//!     [--prepared DIR] [--clang PATH] [--binding NAME]...
//! ```
//!
//! **prepare** verifies the release tarball against the pin
//! ([`pin::OPENSSL`]), extracts it into the output, checks the extracted tree's
//! digest, runs the pinned configuration (`perl Configure linux-x86_64
//! no-asm`, `CC` the given Clang) and `make build_generated` (OpenSSL
//! generates its public headers), checks that the build compiles every
//! defining file with the contract set's flags (`make -n OBJECT`), and checks
//! the prepared tree's digest. Any mismatch fails the run (`Unknown`). It
//! claims nothing.
//!
//! **check** answers one claim per binding of the carried contract set
//! ([`contracts::OPENSSL_3_5_5`]), about the artifact
//! `openssl@3.5.5::FUNCTION`. A binding's obligations are the release pin
//! (the prepared tree's digest and version), the public declaration (the
//! header probe's prototype equals the contract's), the definition (exactly
//! one external definition in the contract's file, with that prototype), and
//! the class rule ([`checker`]). The plugin proposes `Safe` only when every
//! obligation is discharged, and `Unknown`, with the first undischarged
//! obligation as its reason, otherwise; it never proposes `Unsafe` (it
//! validates no witness). A binding of a class the checker has no rule for is
//! `Unknown`. A pin, version or contract-set mismatch, an absent tree or a
//! Clang that does not run makes every binding `Unknown`.
//!
//! The host classifies each claim: no trusted-core checker accepts a
//! subcommand plugin's certificate (W45-SUBCOMMAND-CERTIFICATE-ROUTE), so a
//! `Safe` proposal is published `Pending` (`plugin-claim-uncertified`), never
//! `Safe`; a failed run (spawn failure, timeout, quota breach, a reported
//! failure) is `Unknown` and its claims join nothing. The record
//! `openssl-check.json` keeps every obligation, its detail and the evidence
//! (file, line, digests) per binding.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use quirin_audit_plugin::{
    CertificateReference, ProposedVerdict, Request, Response, Severity, VerdictClaim,
};
use serde_json::{Value, json};

pub mod ast;
pub mod checker;
pub mod contracts;
pub mod pin;
pub mod shell;

use contracts::{Binding, ContractSet};
use pin::Pin;

/// Usage text.
pub const USAGE: &str = "usage: quirin-audit --output DIR openssl prepare --tarball FILE \
[--clang PATH] [--perl PATH] [--make PATH] [--jobs N]
       quirin-audit --output DIR openssl check [--prepared DIR] [--clang PATH] \
[--binding NAME]...";

/// The semantic profile of every claim.
pub const SEMANTIC_PROFILE: &str = "quirin.openssl-contract-check.v1";

/// The evidence version of every claim and record.
pub const EVIDENCE_VERSION: u32 = 1;

/// The check record's file name in the output.
pub const CHECK_RECORD: &str = "openssl-check.json";

/// The preparation record's file name in the output.
pub const PREPARATION_RECORD: &str = "openssl-preparation.json";

/// Answer a request.
///
/// # Errors
///
/// A usage error, a refused input or a failed preparation (the response is
/// then `failed`, and the run `Unknown`).
pub fn answer(request: &Request) -> Result<Response, String> {
    let set = ContractSet::parse(contracts::OPENSSL_3_5_5, pin::OPENSSL.version)?;
    match request.arguments.split_first() {
        Some((step, rest)) if step == "prepare" => {
            prepare(request, &pin::OPENSSL, &set, &PrepareOptions::parse(rest)?)
        }
        Some((step, rest)) if step == "check" => {
            check(request, &pin::OPENSSL, &set, &CheckOptions::parse(rest)?)
        }
        _ => Err(USAGE.into()),
    }
}

fn take_value<'a>(arguments: &'a [String], index: usize, flag: &str) -> Result<&'a str, String> {
    arguments
        .get(index + 1)
        .map(String::as_str)
        .ok_or_else(|| format!("{flag} requires a value\n{USAGE}"))
}

fn set_once(slot: &mut Option<String>, flag: &str, value: &str) -> Result<(), String> {
    if slot.replace(value.to_owned()).is_some() {
        return Err(format!("{flag} is given twice"));
    }
    Ok(())
}

/// The flags of `prepare`.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct PrepareOptions {
    pub tarball: Option<String>,
    pub clang: Option<String>,
    pub perl: Option<String>,
    pub make: Option<String>,
    pub jobs: Option<String>,
}

impl PrepareOptions {
    /// Parse `prepare`'s flags.
    ///
    /// # Errors
    ///
    /// An unknown, repeated or valueless flag, or no `--tarball`.
    pub fn parse(arguments: &[String]) -> Result<Self, String> {
        let mut options = Self::default();
        let mut index = 0;
        while index < arguments.len() {
            let flag = arguments[index].as_str();
            let value = take_value(arguments, index, flag);
            let slot = match flag {
                "--tarball" => &mut options.tarball,
                "--clang" => &mut options.clang,
                "--perl" => &mut options.perl,
                "--make" => &mut options.make,
                "--jobs" => &mut options.jobs,
                other => return Err(format!("unknown argument `{other}`\n{USAGE}")),
            };
            set_once(slot, flag, value?)?;
            index += 2;
        }
        if options.tarball.is_none() {
            return Err(format!(
                "prepare requires --tarball FILE (the release tarball, {})\n{USAGE}",
                pin::OPENSSL.tarball_url
            ));
        }
        if let Some(jobs) = &options.jobs
            && !jobs.parse::<u32>().is_ok_and(|jobs| jobs > 0)
        {
            return Err(format!("--jobs `{jobs}` is not a positive integer"));
        }
        Ok(options)
    }
}

/// The flags of `check`.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct CheckOptions {
    pub prepared: Option<String>,
    pub clang: Option<String>,
    pub bindings: Vec<String>,
}

impl CheckOptions {
    /// Parse `check`'s flags.
    ///
    /// # Errors
    ///
    /// An unknown, repeated or valueless flag.
    pub fn parse(arguments: &[String]) -> Result<Self, String> {
        let mut options = Self::default();
        let mut index = 0;
        while index < arguments.len() {
            let flag = arguments[index].as_str();
            let value = take_value(arguments, index, flag)?;
            match flag {
                "--prepared" => set_once(&mut options.prepared, flag, value)?,
                "--clang" => set_once(&mut options.clang, flag, value)?,
                "--binding" => {
                    if options.bindings.iter().any(|binding| binding == value) {
                        return Err(format!("--binding {value} is given twice"));
                    }
                    options.bindings.push(value.to_owned());
                }
                other => return Err(format!("unknown argument `{other}`\n{USAGE}")),
            }
            index += 2;
        }
        Ok(options)
    }
}

fn file_digest(path: &Path) -> Result<String, String> {
    std::fs::read(path)
        .map(|bytes| qcore_schemas::sha256_hex(&bytes))
        .map_err(|error| format!("{}: {error}", path.display()))
}

fn write_json(path: &Path, value: &Value) -> Result<(), String> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    bytes.push(b'\n');
    std::fs::write(path, bytes).map_err(|error| format!("{}: {error}", path.display()))
}

/// Run `program` in `directory`, its output to `log`.
fn logged(
    program: &str,
    arguments: &[&str],
    directory: &Path,
    environment: &[(&str, &str)],
    log: &Path,
) -> Result<(), String> {
    let file = std::fs::File::create(log).map_err(|error| format!("{}: {error}", log.display()))?;
    let error = file
        .try_clone()
        .map_err(|error| format!("{}: {error}", log.display()))?;
    let mut command = Command::new(program);
    command
        .args(arguments)
        .current_dir(directory)
        .stdin(Stdio::null())
        .stdout(file)
        .stderr(error);
    for (name, value) in environment {
        command.env(name, value);
    }
    let status = command
        .status()
        .map_err(|error| format!("{program}: {error}"))?;
    if !status.success() {
        return Err(format!(
            "{program} {} failed ({status}); see {}",
            arguments.join(" "),
            log.display()
        ));
    }
    Ok(())
}

fn captured(program: &str, arguments: &[&str], directory: &Path) -> Result<String, String> {
    let output = Command::new(program)
        .args(arguments)
        .current_dir(directory)
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("{program}: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "{program} {} failed ({}): {}",
            arguments.join(" "),
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn first_line(program: &str, argument: &str) -> String {
    captured(program, &[argument], Path::new("."))
        .map(|text| text.lines().next().unwrap_or("").trim().to_owned())
        .unwrap_or_else(|error| format!("unavailable: {error}"))
}

/// `prepare`: the pinned release, configured and its headers generated.
///
/// # Errors
///
/// A refused input or any pin, digest, flag or build failure.
pub fn prepare(
    request: &Request,
    pin: &Pin,
    set: &ContractSet,
    options: &PrepareOptions,
) -> Result<Response, String> {
    let output = request.output_directory();
    let tarball = request.resolve_argument(options.tarball.as_deref().unwrap_or_default());
    let clang = options.clang.as_deref().unwrap_or("clang");
    let perl = options.perl.as_deref().unwrap_or("perl");
    let make = options.make.as_deref().unwrap_or("make");
    let jobs = options.jobs.as_deref().unwrap_or("4");

    let tarball_sha256 = file_digest(&tarball)?;
    if tarball_sha256 != pin.tarball_sha256 {
        return Err(format!(
            "release pin mismatch: {} has SHA-256 {tarball_sha256}, not the pinned {} of {}",
            tarball.display(),
            pin.tarball_sha256,
            pin.tarball_url
        ));
    }
    let tree = output.join(pin.directory);
    if tree.exists() {
        return Err(format!(
            "{} exists: prepare into a fresh output",
            tree.display()
        ));
    }
    let tarball_text = tarball
        .to_str()
        .ok_or_else(|| format!("non-UTF-8 path {}", tarball.display()))?;
    let output_text = output
        .to_str()
        .ok_or_else(|| format!("non-UTF-8 path {}", output.display()))?;
    captured("tar", &["-xzf", tarball_text, "-C", output_text], &output)?;
    let version = pin::release_version(&tree)?;
    if version != pin.version {
        return Err(format!(
            "release pin mismatch: the tree is {version}, not {}",
            pin.version
        ));
    }
    let (source_tree_sha256, source_files) = pin::tree_digest(&tree, &[])?;
    if source_tree_sha256 != pin.source_tree_sha256 {
        return Err(format!(
            "release pin mismatch: the extracted tree's digest is {source_tree_sha256} \
             ({source_files} files), not the pinned {}",
            pin.source_tree_sha256
        ));
    }

    let clang_path = which(clang).unwrap_or_else(|| clang.to_owned());
    let mut configure = vec!["Configure"];
    configure.extend(pin.configure);
    logged(
        perl,
        &configure,
        &tree,
        &[("CC", &clang_path)],
        &output.join("openssl-configure.log"),
    )?;
    let parallel = format!("-j{jobs}");
    logged(
        make,
        &[&parallel, "build_generated"],
        &tree,
        &[],
        &output.join("openssl-generate.log"),
    )?;

    let mut compiled = BTreeMap::new();
    for binding in &set.bindings {
        let Some(definition) = &binding.definition else {
            continue;
        };
        if compiled.contains_key(&definition.object) {
            continue;
        }
        let printed = captured(make, &["-n", &definition.object], &tree)?;
        let flags = shell::compile_flags(&printed, &definition.object, &definition.file)?;
        if flags != set.flag_sets[&definition.flags] {
            return Err(format!(
                "the pinned build compiles {} with {flags:?}, not the contract set's `{}` flags",
                definition.file, definition.flags
            ));
        }
        compiled.insert(
            definition.object.clone(),
            json!({"source": definition.file, "flags": definition.flags}),
        );
    }

    let (prepared_tree_sha256, prepared_files) = pin::tree_digest(&tree, &pin::BYPRODUCTS)?;
    if prepared_tree_sha256 != pin.prepared_tree_sha256 {
        return Err(format!(
            "release pin mismatch: the prepared tree's digest is {prepared_tree_sha256} \
             ({prepared_files} files), not the pinned {}",
            pin.prepared_tree_sha256
        ));
    }

    let record = json!({
        "schema": "quirin-openssl-preparation/1",
        "plugin": {"name": env!("CARGO_PKG_NAME"), "version": env!("CARGO_PKG_VERSION")},
        "release": {
            "version": pin.version,
            "tag": pin.tag,
            "tag_commit": pin.tag_commit,
            "tarball_url": pin.tarball_url,
        },
        "tarball": {"path": tarball.display().to_string(), "sha256": tarball_sha256},
        "tree": pin.directory,
        "source_tree": {"sha256": source_tree_sha256, "files": source_files},
        "prepared_tree": {
            "sha256": prepared_tree_sha256,
            "files": prepared_files,
            "excluded_byproducts": pin::BYPRODUCTS,
        },
        "configure": {"command": configure, "CC": clang_path},
        "generate": ["make", parallel, "build_generated"],
        "compiled_objects": compiled,
        "tools": {
            "clang": first_line(&clang_path, "--version"),
            "perl": captured(perl, &["-e", "print $^V"], &output)
                .unwrap_or_else(|error| format!("unavailable: {error}")),
            "make": first_line(make, "--version"),
        },
    });
    write_json(&output.join(PREPARATION_RECORD), &record)?;
    let mut response = Response::completed();
    for (path, kind) in [
        (PREPARATION_RECORD, "openssl-preparation"),
        ("openssl-configure.log", "log"),
        ("openssl-generate.log", "log"),
    ] {
        response.push_artifact(&output, Path::new(path), kind)?;
    }
    response.push_diagnostic(
        Severity::Info,
        "openssl-prepared",
        format!(
            "OpenSSL {} prepared at {} (prepared tree {prepared_tree_sha256})",
            pin.version,
            tree.display()
        ),
    );
    Ok(response)
}

/// `name` resolved on `PATH`, when it has no directory part.
fn which(name: &str) -> Option<String> {
    if name.contains('/') {
        return Some(name.to_owned());
    }
    std::env::split_paths(&std::env::var_os("PATH")?)
        .map(|directory| directory.join(name))
        .find(|candidate| candidate.is_file())
        .map(|path| path.display().to_string())
}

/// One obligation of a binding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Obligation {
    pub name: &'static str,
    pub discharged: bool,
    pub detail: String,
}

impl Obligation {
    fn from(name: &'static str, result: Result<String, String>) -> Self {
        match result {
            Ok(detail) => Self {
                name,
                discharged: true,
                detail,
            },
            Err(detail) => Self {
                name,
                discharged: false,
                detail,
            },
        }
    }
}

/// The checker's answer for one binding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BindingCheck {
    pub function: String,
    pub obligations: Vec<Obligation>,
    pub definition: Option<Value>,
}

impl BindingCheck {
    /// `Safe` exactly when every obligation is discharged; never `Unsafe`.
    #[must_use]
    pub fn proposed(&self) -> ProposedVerdict {
        if !self.obligations.is_empty() && self.obligations.iter().all(|o| o.discharged) {
            ProposedVerdict::Safe
        } else {
            ProposedVerdict::Unknown
        }
    }

    /// The reason: every obligation for a `Safe` proposal, the first
    /// undischarged one otherwise.
    #[must_use]
    pub fn reason(&self) -> String {
        match self.obligations.iter().find(|o| !o.discharged) {
            Some(failed) => format!(
                "obligation `{}` is not discharged: {}",
                failed.name, failed.detail
            ),
            None => format!(
                "every obligation discharged ({})",
                self.obligations
                    .iter()
                    .map(|o| o.name)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }
}

/// What the whole run established before any binding.
#[derive(Clone, Debug)]
pub struct Environment {
    /// The release pin: `Err` names the mismatch.
    pub pin: Result<String, String>,
    /// The header probe's prototypes by name, or why there are none.
    pub declarations: Result<BTreeMap<String, Vec<String>>, String>,
    pub clang: String,
    pub clang_version: Result<String, String>,
    pub tree: PathBuf,
}

/// Establish the run's environment: the pin over `tree`, Clang, and the
/// header probe.
#[must_use]
pub fn environment(pin: &Pin, set: &ContractSet, tree: &Path, clang: &str) -> Environment {
    let pin_result = (|| {
        if !tree.is_dir() {
            return Err(format!("no prepared tree at {}", tree.display()));
        }
        let version = pin::release_version(tree)?;
        if version != pin.version {
            return Err(format!("the tree is {version}, not {}", pin.version));
        }
        let (digest, files) = pin::tree_digest(tree, &pin::BYPRODUCTS)?;
        if digest != pin.prepared_tree_sha256 {
            return Err(format!(
                "the prepared tree's digest is {digest} ({files} files), not the pinned {}",
                pin.prepared_tree_sha256
            ));
        }
        Ok(format!(
            "OpenSSL {} (tag {} at {}), prepared tree {digest} ({files} files)",
            pin.version, pin.tag, pin.tag_commit
        ))
    })();
    let clang_version = ast::clang_version(clang);
    let declarations = match (&pin_result, &clang_version) {
        (Err(_), _) => Err("not read: the release pin failed".to_owned()),
        (_, Err(error)) => Err(format!("not read: {error}")),
        (Ok(_), Ok(_)) => ast::header_declarations(clang, tree, &set.headers()).map(|nodes| {
            let mut found: BTreeMap<String, Vec<String>> = BTreeMap::new();
            for node in &nodes {
                if let Some(name) = node["name"].as_str() {
                    found
                        .entry(name.to_owned())
                        .or_default()
                        .push(ast::qual_type(node).to_owned());
                }
            }
            found
        }),
    };
    Environment {
        pin: pin_result,
        declarations,
        clang: clang.to_owned(),
        clang_version,
        tree: tree.to_owned(),
    }
}

fn declared(environment: &Environment, binding: &Binding) -> Result<String, String> {
    let declarations = environment.declarations.as_ref().map_err(Clone::clone)?;
    let spelled = declarations
        .get(&binding.function)
        .ok_or_else(|| format!("{} is not declared by the public headers", binding.function))?;
    if spelled.iter().any(|spelled| spelled != &binding.prototype) {
        return Err(format!(
            "the public declaration(s) {spelled:?} are not the contract's `{}`",
            binding.prototype
        ));
    }
    Ok(format!(
        "declared `{}` by the public headers ({} declaration(s))",
        binding.prototype,
        spelled.len()
    ))
}

/// Check one binding.
#[must_use]
pub fn check_binding(
    environment: &Environment,
    set: &ContractSet,
    binding: &Binding,
) -> BindingCheck {
    let mut check = BindingCheck {
        function: binding.function.clone(),
        obligations: vec![Obligation::from("release-pin", environment.pin.clone())],
        definition: None,
    };
    if environment.pin.is_err() {
        check.obligations.push(Obligation::from(
            "declared",
            Err("not evaluated: the release pin failed".into()),
        ));
        return check;
    }
    check
        .obligations
        .push(Obligation::from("declared", declared(environment, binding)));
    let class = &set.classes[&binding.class];
    let Some(definition) = binding.definition.as_ref().filter(|_| class.checked) else {
        check.obligations.push(Obligation::from(
            "class-rule",
            Err(format!(
                "checker v1 has no rule for contract class `{}` ({})",
                binding.class, class.meaning
            )),
        ));
        return check;
    };
    let flags = &set.flag_sets[&definition.flags];
    let dumped = ast::named_declarations(
        &environment.clang,
        &environment.tree,
        flags,
        &definition.file,
        &binding.function,
    );
    let function = dumped.and_then(|nodes| {
        let functions: Vec<&Value> = nodes
            .iter()
            .filter(|node| ast::kind(node) == "FunctionDecl")
            .collect();
        if let Some(other) = functions
            .iter()
            .find(|node| ast::qual_type(node) != binding.prototype)
        {
            return Err(format!(
                "a declaration in {} has the type `{}`, not the contract's `{}`",
                definition.file,
                ast::qual_type(other),
                binding.prototype
            ));
        }
        let definitions: Vec<&Value> = functions
            .into_iter()
            .filter(|node| ast::body(node).is_some())
            .collect();
        let [function] = definitions.as_slice() else {
            return Err(format!(
                "{} has {} definitions of {}, not one",
                definition.file,
                definitions.len(),
                binding.function
            ));
        };
        if ast::declaration_file(function) != Some(definition.file.as_str()) {
            return Err(format!(
                "the definition is in {:?}, not {}",
                ast::declaration_file(function),
                definition.file
            ));
        }
        if !function["storageClass"].is_null() {
            return Err(format!(
                "the definition has storage class `{}`: not the external definition",
                function["storageClass"]
            ));
        }
        Ok((*function).clone())
    });
    let function = match function {
        Ok(function) => function,
        Err(error) => {
            check
                .obligations
                .push(Obligation::from("defined", Err(error)));
            return check;
        }
    };
    let file_sha256 = file_digest(&environment.tree.join(&definition.file))
        .unwrap_or_else(|error| format!("unavailable: {error}"));
    let line = ast::declaration_line(&function);
    check.obligations.push(Obligation::from(
        "defined",
        Ok(format!(
            "one external definition at {}:{} with the contract's type",
            definition.file,
            line.map_or_else(|| "?".to_owned(), |line| line.to_string())
        )),
    ));
    check.definition = Some(json!({
        "file": definition.file,
        "line": line,
        "file_sha256": file_sha256,
        "object": definition.object,
        "flags": definition.flags,
    }));
    let rule = match binding.class.as_str() {
        "no-pointer-interface" => checker::no_pointer_interface(&function),
        "static-object-return" => {
            let returns = binding.returns.as_deref().unwrap_or_default();
            ast::named_declarations(
                &environment.clang,
                &environment.tree,
                flags,
                &definition.file,
                returns,
            )
            .and_then(|objects| {
                checker::static_object_return(&function, returns, &objects, &definition.file)
            })
        }
        "pure-reader" => checker::pure_reader(&function),
        "member-store" => checker::member_store(
            &function,
            binding.stores.as_deref().unwrap_or_default(),
            binding.escapes.as_deref().unwrap_or_default(),
        ),
        other => Err(format!("checker v1 has no rule for class `{other}`")),
    };
    check.obligations.push(Obligation::from("class-rule", rule));
    check
}

/// The assumptions every claim of `binding` rests on.
#[must_use]
pub fn assumptions(pin: &Pin, binding: &Binding, clang_version: &str) -> BTreeMap<String, String> {
    let mut assumptions = BTreeMap::new();
    assumptions.insert(
        "release".to_owned(),
        format!(
            "the upstream {} release (tag {} at {}, tarball SHA-256 {}); Fluent Bit links the \
             distribution's {} build, whose patches are outside this check",
            pin.directory, pin.tag, pin.tag_commit, pin.tarball_sha256, pin.version
        ),
    );
    assumptions.insert(
        "configuration".to_owned(),
        format!(
            "Configure {}: the definition as that configuration preprocesses it; another \
             configuration may select other code",
            pin.configure.join(" ")
        ),
    );
    assumptions.insert(
        "front-end".to_owned(),
        format!("{clang_version}: its JSON AST is taken as the definition's C semantics"),
    );
    assumptions.insert(
        "link".to_owned(),
        "the caller's call binds this external definition (no interposition or symbol \
         versioning substitutes another)"
            .to_owned(),
    );
    match binding.class.as_str() {
        "no-pointer-interface" => {
            assumptions.insert(
                "library-state".to_owned(),
                "state reachable only through OpenSSL's global or thread-local storage holds \
                 no caller-owned resource"
                    .to_owned(),
            );
        }
        "pure-reader" | "member-store" => {
            assumptions.insert(
                "argument-validity".to_owned(),
                "the caller passes live objects of the declared types for every pointer the \
                 body dereferences (the caller's obligation, not discharged here)"
                    .to_owned(),
            );
        }
        _ => {}
    }
    assumptions
}

/// The claim about one binding.
#[must_use]
pub fn claim(
    pin: &Pin,
    binding: &Binding,
    check: &BindingCheck,
    clang_version: &str,
    record: Option<&CertificateReference>,
) -> VerdictClaim {
    let verdict = check.proposed();
    VerdictClaim {
        artifact: format!("openssl@{}::{}", pin.version, binding.function),
        property: format!("openssl-contract/{}", binding.class),
        semantic_profile: SEMANTIC_PROFILE.to_owned(),
        scope: format!(
            "the external definition of {} in the pinned OpenSSL {} release{}, against its \
             contract: {}",
            binding.function,
            pin.version,
            binding
                .definition
                .as_ref()
                .map(|definition| format!(" ({})", definition.file))
                .unwrap_or_default(),
            binding.contract
        ),
        bounds: "the definition's own body under the class rule's whitelist; no \
                 inter-procedural reasoning"
            .to_owned(),
        assumptions: assumptions(pin, binding, clang_version),
        evidence_version: EVIDENCE_VERSION,
        verdict,
        reason: check.reason(),
        certificates: match (verdict, record) {
            (ProposedVerdict::Safe, Some(record)) => vec![record.clone()],
            _ => Vec::new(),
        },
    }
}

/// The record of one binding's check.
#[must_use]
pub fn binding_record(binding: &Binding, check: &BindingCheck, checked: bool) -> Value {
    json!({
        "function": binding.function,
        "header": binding.header,
        "prototype": binding.prototype,
        "class": binding.class,
        "class_checked": checked,
        "contract": binding.contract,
        "census_pairs": binding.census_pairs,
        "obligations": check.obligations.iter().map(|o| json!({
            "obligation": o.name,
            "discharged": o.discharged,
            "detail": o.detail,
        })).collect::<Vec<_>>(),
        "definition": check.definition,
        "proposed": check.proposed(),
        "reason": check.reason(),
    })
}

/// `check`: one claim per binding.
///
/// # Errors
///
/// A usage error (an unknown `--binding`), or a record that could not be
/// written.
pub fn check(
    request: &Request,
    pin: &Pin,
    set: &ContractSet,
    options: &CheckOptions,
) -> Result<Response, String> {
    for name in &options.bindings {
        if !set.bindings.iter().any(|binding| &binding.function == name) {
            return Err(format!(
                "--binding {name}: the contract set binds no such function"
            ));
        }
    }
    let output = request.output_directory();
    let prepared = options
        .prepared
        .as_deref()
        .map_or_else(|| output.clone(), |path| request.resolve_argument(path));
    let tree = prepared.join(pin.directory);
    let clang = options.clang.as_deref().unwrap_or("clang");
    let environment = environment(pin, set, &tree, clang);
    let clang_version = environment
        .clang_version
        .clone()
        .unwrap_or_else(|error| format!("Clang unavailable ({error})"));
    let selected: Vec<&Binding> = set
        .bindings
        .iter()
        .filter(|binding| {
            options.bindings.is_empty() || options.bindings.contains(&binding.function)
        })
        .collect();
    let mut checks = Vec::new();
    for binding in &selected {
        let mut check = check_binding(&environment, set, binding);
        if let Err(error) = &environment.clang_version {
            check.obligations.push(Obligation::from(
                "front-end",
                Err(format!("Clang does not run: {error}")),
            ));
        }
        checks.push(check);
    }

    let contracts_path = output.join("openssl-contracts.json");
    std::fs::write(&contracts_path, contracts::OPENSSL_3_5_5)
        .map_err(|error| format!("{}: {error}", contracts_path.display()))?;
    let record = json!({
        "schema": "quirin-openssl-check/1",
        "plugin": {"name": env!("CARGO_PKG_NAME"), "version": env!("CARGO_PKG_VERSION")},
        "semantic_profile": SEMANTIC_PROFILE,
        "evidence_version": EVIDENCE_VERSION,
        "release": {
            "version": pin.version,
            "tag": pin.tag,
            "tag_commit": pin.tag_commit,
            "tarball_sha256": pin.tarball_sha256,
            "prepared_tree_sha256": pin.prepared_tree_sha256,
        },
        "tree": tree.display().to_string(),
        "pin": match &environment.pin {
            Ok(detail) => json!({"held": true, "detail": detail}),
            Err(detail) => json!({"held": false, "detail": detail}),
        },
        "clang": {"path": clang, "version": clang_version},
        "contract_set": {
            "schema": set.schema,
            "version": set.contract_set_version,
            "sha256": qcore_schemas::sha256_hex(contracts::OPENSSL_3_5_5.as_bytes()),
        },
        "bindings": selected.iter().zip(&checks).map(|(binding, check)| {
            binding_record(binding, check, set.checked(binding))
        }).collect::<Vec<_>>(),
        "proposed": {
            "safe": checks.iter().filter(|c| c.proposed() == ProposedVerdict::Safe).count(),
            "unknown": checks.iter().filter(|c| c.proposed() == ProposedVerdict::Unknown).count(),
        },
    });
    let record_path = output.join(CHECK_RECORD);
    write_json(&record_path, &record)?;
    let reference = CertificateReference {
        kind: "openssl-check-record".to_owned(),
        path: CHECK_RECORD.to_owned(),
        sha256: file_digest(&record_path)?,
    };
    let mut response = Response::completed();
    response.push_artifact(&output, Path::new(CHECK_RECORD), "openssl-check")?;
    response.push_artifact(
        &output,
        Path::new("openssl-contracts.json"),
        "openssl-contracts",
    )?;
    if let Err(error) = &environment.pin {
        response.push_diagnostic(
            Severity::Error,
            "openssl-pin-mismatch",
            format!("every binding is Unknown: {error}"),
        );
    }
    for (binding, check) in selected.iter().zip(&checks) {
        response.push_claim(claim(pin, binding, check, &clang_version, Some(&reference)));
    }
    Ok(response)
}
