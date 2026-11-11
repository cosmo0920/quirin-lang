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

//! `quirin-audit prepare` (card **W44-QUIRIN-AUDIT-PREPARE**): the Fluent Bit
//! audit inputs, prepared by the same git, CMake and build invocations that
//! the two retired Python preparation scripts (the CMetrics Clang build and
//! the Cycle 19 Monkey inputs) ran.
//!
//! Two steps, each a line-by-line port of one script, with the script's flags,
//! defaults, refusals and output files:
//!
//! * `prepare clang-build`: an independent, revision-selected clone of the
//!   Fluent Bit checkout (`git clone --no-hardlinks --no-checkout`, then a
//!   detached checkout), CMake configuration with explicit Clang C and C++
//!   compilers and `-std=gnu17`, and the `cmetrics-static` build. It writes
//!   `cmetrics-audit.json.config` and `build-provenance.json`.
//! * `prepare monkey`: a byte-identified copy of the checkout's tracked
//!   `lib/monkey` files and a forced-Libevent CMake configuration. It writes
//!   the `qcore-monkey-inputs.json` marker and `configure.log`.
//!
//! `prepare` with no step runs both into `build/cycle19/clang/<commit>/`, the
//! layout `quirin-audit current` reads: `fluent-bit-clang-source`,
//! `fluent-bit-clang-production` and `monkey/` (Monkey prepared from the
//! clone). It adds `prepare.json`, which records both compilers (the C++
//! compiler included), CMake, the step records' digests and the argv.
//! `--run` then chains into `quirin-audit current` over those inputs, with
//! `--jobs N` and any arguments after `--` passed through. Card
//! **CURRENT-MONKEY-LINKED-CONFIGURATION**: the chained run passes
//! `--monkey-linked`, so Monkey is analysed as the Fluent Bit compile
//! database configures it (the configuration linked into `fluent-bit`); the
//! prepared Monkey database is passed too and only accounted for.
//!
//! Card **FLUENT-BIT-LINK-PREPARE**: `--link` (with `prepare` or `prepare
//! clang-build`) also builds and links the `fluent-bit` executable in the
//! same build tree and writes the link-resolution artifact and a candidate
//! rule 4c link map under `link-resolution/` ([`crate::link_resolution`]).
//! `--build-jobs N` sets that build's and the object reproduction's
//! parallelism (default 2). Without `--link` nothing is linked and every
//! file is written as before.
//!
//! The step records are written byte for byte as the scripts wrote them
//! (Python `json.dumps(indent=2)`, insertion order, ASCII escapes; see
//! [`crate::prepare_json`]). Nothing here reads the network, edits the
//! upstream checkout or reuses its build: the clone is the checkout's own
//! committed objects, and a reused output is accepted only when its identity
//! checks pass. The prepare step is tooling; it proves no compiler
//! equivalence and changes no verdict.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::Instant;

use qcore_contract::profile_loader::sha256_hex;
use serde_json::{Value, json};

use crate::prepare_json::PyJson;

/// Usage text of `quirin-audit prepare`.
pub const USAGE: &str = "usage: quirin-audit prepare [--fluent-bit DIR] [--revision REV] \
[--output DIR] [--clang PATH] [--clangxx PATH] [--cmake PATH] [--repository DIR] \
[--link [--build-jobs N]] [--run [--jobs N] [-- CURRENT-ARGUMENTS...]]
       quirin-audit prepare clang-build [--fluent-bit DIR] [--revision REV] [--output DIR] \
[--clang PATH] [--clangxx PATH] [--cmake PATH] [--repository DIR] [--link [--build-jobs N]]
       quirin-audit prepare monkey --fluent-bit DIR [--output DIR] [--clang PATH] \
[--clangxx PATH] [--cmake PATH] [--repository DIR]";

/// Files Monkey's CMake configuration writes into its source directory.
const GENERATED_SOURCE_FILES: [&str; 2] = ["include/monkey/mk_static_plugins.h", "monkey.service"];

/// Card **CURRENT-RUN-DEFECTS**: the CMake targets that generate, at build
/// time, sources and headers units of the compile database read. Without
/// them a configured-only build leaves 34 of Fluent Bit 046accf6's 1,419
/// units unpreprocessable: LuaJIT's buildvm outputs (`lj_vm.S`, `lj_bcdef.h`,
/// `lj_ffdef.h`, `lj_libdef.h`, `lj_recdef.h`, `lj_folddef.h`,
/// `buildvm_arch.h`, `luajit.h`), the flex/bison outputs of the record
/// accessor, stream processor and SQL processor grammars, and libbacktrace's
/// installed `backtrace.h`. The member-facts pre-pass (rule 12n) cannot read
/// such a unit, so it opens every invoked member. Only the targets the
/// configuration defines are built (a build without LuaJIT, a parser or
/// libbacktrace has no unit needing them); the step changes no compile
/// command and no tracked source file.
const GENERATED_INPUT_TARGETS: [&str; 8] = [
    "buildvm_arch_h",
    "lj_gen_headers",
    "lj_gen_folddef",
    "libluajit",
    "flb-ra-parser",
    "flb-sp-parser",
    "processor-sql-parser",
    "backtrace",
];

/// The [`GENERATED_INPUT_TARGETS`] a build's `help` target lists, in that
/// order. Reads both generators' listings: the Makefile generator's
/// `... NAME` lines and Ninja's `NAME: phony` lines.
fn generated_input_targets(help: &str) -> Vec<&'static str> {
    let listed = help
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            let name = line.strip_prefix("... ").unwrap_or(line);
            name.split([':', ' '])
                .next()
                .filter(|name| !name.is_empty())
        })
        .collect::<BTreeSet<_>>();
    GENERATED_INPUT_TARGETS
        .into_iter()
        .filter(|target| listed.contains(target))
        .collect()
}

const CLANG_BUILD_SCOPE: &str = "Unmodified Clang GNU17 CMake command; bounded C resource/function audit, not GCC, code generation, executable-stack or whole-library safety equivalence.";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Step {
    All,
    ClangBuild,
    Monkey,
}

#[derive(Debug, Default, PartialEq)]
struct Options {
    fluent_bit: Option<PathBuf>,
    revision: Option<String>,
    output: Option<PathBuf>,
    clang: Option<String>,
    clangxx: Option<String>,
    cmake: Option<String>,
    repository: Option<PathBuf>,
    run: bool,
    jobs: Option<String>,
    forwarded: Vec<String>,
    link: bool,
    build_jobs: Option<usize>,
}

fn parse(arguments: &[String]) -> Result<(Step, Options), String> {
    let (step, rest) = match arguments.first().map(String::as_str) {
        Some("clang-build") => (Step::ClangBuild, &arguments[1..]),
        Some("monkey") => (Step::Monkey, &arguments[1..]),
        _ => (Step::All, arguments),
    };
    let mut options = Options::default();
    let mut index = 0;
    while index < rest.len() {
        let flag = rest[index].as_str();
        if flag == "--" {
            if !options.run {
                return Err(format!("`--` forwards arguments to --run only\n{USAGE}"));
            }
            options.forwarded = rest[index + 1..].to_vec();
            break;
        }
        if flag == "--link" {
            if step == Step::Monkey {
                return Err(format!(
                    "--link links fluent-bit; use `prepare --link` or `prepare clang-build --link`\n{USAGE}"
                ));
            }
            if options.link {
                return Err("--link is given twice".into());
            }
            options.link = true;
            index += 1;
            continue;
        }
        if flag == "--run" {
            if step != Step::All {
                return Err(format!(
                    "--run needs both steps; use `prepare --run`\n{USAGE}"
                ));
            }
            if options.run {
                return Err("--run is given twice".into());
            }
            options.run = true;
            index += 1;
            continue;
        }
        let value = rest
            .get(index + 1)
            .ok_or_else(|| format!("{flag} requires a value\n{USAGE}"))?
            .clone();
        let twice = || format!("{flag} is given twice");
        let set_text = |slot: &mut Option<String>| {
            if slot.replace(value.clone()).is_some() {
                return Err(twice());
            }
            Ok(())
        };
        let set_path = |slot: &mut Option<PathBuf>| {
            if slot.replace(PathBuf::from(&value)).is_some() {
                return Err(twice());
            }
            Ok(())
        };
        match flag {
            "--fluent-bit" => set_path(&mut options.fluent_bit)?,
            "--output" => set_path(&mut options.output)?,
            "--repository" => set_path(&mut options.repository)?,
            "--revision" if step != Step::Monkey => set_text(&mut options.revision)?,
            "--clang" => set_text(&mut options.clang)?,
            "--clangxx" => set_text(&mut options.clangxx)?,
            "--cmake" => set_text(&mut options.cmake)?,
            "--jobs" => {
                crate::parallel::resolve_jobs(Some(&value))?;
                set_text(&mut options.jobs)?;
            }
            "--build-jobs" => {
                let jobs = value
                    .parse::<usize>()
                    .ok()
                    .filter(|jobs| *jobs > 0)
                    .ok_or_else(|| {
                        format!("--build-jobs needs a positive integer, not `{value}`")
                    })?;
                if options.build_jobs.replace(jobs).is_some() {
                    return Err(twice());
                }
            }
            other => return Err(format!("unknown argument `{other}`\n{USAGE}")),
        }
        index += 2;
    }
    if options.build_jobs.is_some() && !options.link {
        return Err("--build-jobs sets the link step's parallelism; it needs --link".into());
    }
    if options.jobs.is_some() && !options.run {
        return Err("--jobs sets the chained audit's job count; it needs --run".into());
    }
    if step == Step::Monkey && options.fluent_bit.is_none() {
        return Err(format!(
            "prepare monkey requires --fluent-bit DIR (the checkout whose lib/monkey is copied)\n{USAGE}"
        ));
    }
    Ok((step, options))
}

/// Parse, prepare and optionally run. Returns the process exit code.
#[must_use]
pub fn main(arguments: Vec<String>) -> ExitCode {
    match run(&arguments) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("audit preparation rejected: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(arguments: &[String]) -> Result<ExitCode, String> {
    let (step, options) = parse(arguments)?;
    eprintln!("prepare: checking checkout and tools");
    let cwd = std::env::current_dir().map_err(|error| error.to_string())?;
    let repository = repository_root(options.repository.as_deref(), &cwd)?;
    let tools = Tools {
        clang: options.clang.clone().unwrap_or_else(|| "clang".into()),
        clangxx: options.clangxx.clone().unwrap_or_else(|| "clang++".into()),
        cmake: options.cmake.clone().unwrap_or_else(|| "cmake".into()),
    };
    let revision = options.revision.clone().unwrap_or_else(|| "HEAD".into());
    let default_upstream = || {
        repository
            .parent()
            .map(|parent| parent.join("fluent-bit"))
            .ok_or_else(|| "the repository has no parent directory".to_owned())
    };
    match step {
        Step::ClangBuild => {
            let upstream = match &options.fluent_bit {
                Some(path) => path.clone(),
                None => default_upstream()?,
            };
            let prepared = prepare_clang_build(&ClangBuildRequest {
                repository: &repository,
                fluent_bit: &upstream,
                revision: &revision,
                output: options.output.as_deref(),
                tools: &tools,
            })?;
            println!("{}", prepared.config_path.display());
            if options.link {
                let linked = link(&prepared, &repository, &tools, &options)?;
                println!("{}", linked.artifact.display());
                println!("{}", linked.candidate.display());
            }
            Ok(ExitCode::SUCCESS)
        }
        Step::Monkey => {
            let upstream = options
                .fluent_bit
                .as_deref()
                .ok_or("prepare monkey requires --fluent-bit DIR")?;
            let prepared = prepare_monkey(&MonkeyRequest {
                repository: &repository,
                fluent_bit: upstream,
                output: options.output.as_deref(),
                tools: &tools,
            })?;
            println!("{}", prepared.database.display());
            Ok(ExitCode::SUCCESS)
        }
        Step::All => {
            let upstream = match &options.fluent_bit {
                Some(path) => path.clone(),
                None => default_upstream()?,
            };
            prepare_all(
                &options,
                arguments,
                &cwd,
                &repository,
                &upstream,
                &revision,
                &tools,
            )
        }
    }
}

/// Card FLUENT-BIT-LINK-PREPARE: link `fluent-bit` in the prepared build
/// tree and read the link (`--link`).
fn link(
    prepared: &ClangBuildPrepared,
    repository: &Path,
    tools: &Tools,
    options: &Options,
) -> Result<crate::link_resolution::LinkPrepared, String> {
    crate::link_resolution::prepare_link(&crate::link_resolution::LinkRequest {
        source: &prepared.source,
        build: &prepared.build,
        database: &prepared.database,
        cmake: &tools.cmake,
        jobs: options.build_jobs.unwrap_or(2),
        output: &prepared.output.join("link-resolution"),
        profile_directory: &repository.join("contract/profiles/fluent-bit"),
    })
}

/// The QCore-C repository whose profiles and providers preparation names.
/// The scripts used their own location; the binary uses `--repository` or the
/// working directory, and refuses a directory without the CMetrics profile.
fn repository_root(flag: Option<&Path>, cwd: &Path) -> Result<PathBuf, String> {
    let candidate = flag.map_or_else(|| cwd.to_owned(), |path| cwd.join(path));
    let root = candidate
        .canonicalize()
        .map_err(|error| format!("{}: {error}", candidate.display()))?;
    if !root
        .join("contract/profiles/cmetrics-counter/profile.json")
        .is_file()
    {
        return Err(format!(
            "{} is not the QCore-C repository (no contract/profiles/cmetrics-counter/profile.json); \
             run from the repository root or pass --repository DIR",
            root.display()
        ));
    }
    Ok(root)
}

struct Tools {
    clang: String,
    clangxx: String,
    cmake: String,
}

struct ClangBuildRequest<'a> {
    repository: &'a Path,
    fluent_bit: &'a Path,
    revision: &'a str,
    output: Option<&'a Path>,
    tools: &'a Tools,
}

struct ClangBuildPrepared {
    pin: String,
    upstream: PathBuf,
    output: PathBuf,
    source: PathBuf,
    build: PathBuf,
    database: PathBuf,
    config_path: PathBuf,
    provenance_path: PathBuf,
    clang: String,
    clangxx: String,
    compiler_version: String,
    configure: Vec<String>,
}

/// Port of the retired CMetrics Clang-build preparation script.
fn prepare_clang_build(request: &ClangBuildRequest) -> Result<ClangBuildPrepared, String> {
    let upstream = strict_resolve(request.fluent_bit)?;
    let profile_dir = request
        .repository
        .join("contract/profiles/cmetrics-counter");
    let profile: Value = serde_json::from_str(&read_text(&profile_dir.join("profile.json"))?)
        .map_err(|error| format!("{}: {error}", profile_dir.join("profile.json").display()))?;
    let pin = capture(
        "git",
        &[
            "-C",
            text(&upstream)?,
            "rev-parse",
            "--verify",
            "--end-of-options",
            &format!("{}^{{commit}}", request.revision),
        ],
    )?;
    let pinned = profile["pins"]["fluent-bit"]["git_commit"]
        .as_str()
        .ok_or("the CMetrics profile has no pins.fluent-bit.git_commit")?;
    let historical = pin == pinned;
    let output = resolve(&request.output.map_or_else(
        || {
            request
                .repository
                .join("build/cycle19/revisions")
                .join(&pin)
        },
        Path::to_owned,
    ))?;
    let source = output.join("fluent-bit-clang-source");
    let build = output.join("fluent-bit-clang-production");
    if output.starts_with(&upstream) || source == upstream {
        return Err("use an output outside the upstream checkout".into());
    }
    let (Some(clang), Some(clangxx)) = (which(&request.tools.clang), which(&request.tools.clangxx))
    else {
        return Err("both Clang C and C++ compilers are required".into());
    };
    let compiler_version = capture(&clang, &["--version"])?;
    if !compiler_version.contains("clang version") {
        return Err("C compiler does not identify itself as Clang".into());
    }
    create_dir_all(&output)?;
    if !source.exists() {
        eprintln!(
            "prepare: cloning {} into {}",
            upstream.display(),
            source.display()
        );
        // No shared object alternates: the persistent checkout is independent.
        capture(
            "git",
            &[
                "clone",
                "--no-hardlinks",
                "--no-checkout",
                text(&upstream)?,
                text(&source)?,
            ],
        )?;
        capture("git", &["-C", text(&source)?, "checkout", "--detach", &pin])?;
    }
    eprintln!("prepare: validating source {}", source.display());
    if !source.join(".git").is_dir() {
        return Err("existing source is not an independent Git checkout".into());
    }
    if capture("git", &["-C", text(&source)?, "rev-parse", "HEAD"])? != pin {
        return Err("existing source has a different revision; choose a fresh output".into());
    }
    let tracked_changes = || {
        capture(
            "git",
            &[
                "-C",
                text(&source)?,
                "status",
                "--porcelain",
                "--untracked-files=no",
            ],
        )
    };
    if !tracked_changes()?.is_empty() {
        return Err("existing source has tracked changes; choose a fresh output".into());
    }
    if source.join(".git/objects/info/alternates").exists() {
        return Err("existing source borrows Git objects; use an independent clone".into());
    }
    create_dir_all(&build)?;
    let cache = build.join("CMakeCache.txt");
    if cache.exists() {
        let values = cmake_cache(&read_text(&cache)?);
        if lookup_exact(&values, "CMAKE_HOME_DIRECTORY:INTERNAL") != Some(text(&source)?) {
            return Err("build cache belongs to another source; choose a fresh output".into());
        }
        let cached = lookup_typed(&values, "CMAKE_C_COMPILER");
        if !cached.is_some_and(|cached| same_file(cached, &clang)) {
            return Err("build cache uses another compiler; choose a fresh output".into());
        }
    }
    let configure: Vec<String> = vec![
        request.tools.cmake.clone(),
        "-S".into(),
        text(&source)?.into(),
        "-B".into(),
        text(&build)?.into(),
        format!("-DCMAKE_C_COMPILER={clang}"),
        format!("-DCMAKE_CXX_COMPILER={clangxx}"),
        "-DCMAKE_C_STANDARD=17".into(),
        "-DCMAKE_C_FLAGS:STRING=-std=gnu17".into(),
        "-DCMAKE_EXPORT_COMPILE_COMMANDS=ON".into(),
        "-DFLB_DEV=ON".into(),
        "-DFLB_EXAMPLES=ON".into(),
    ];
    logged(&configure, &build.join("configure.log"))?;
    let native: Vec<String> = vec![
        request.tools.cmake.clone(),
        "--build".into(),
        text(&build)?.into(),
        "--target".into(),
        "cmetrics-static".into(),
        "-j2".into(),
    ];
    logged(&native, &build.join("native-build.log"))?;
    // Card CURRENT-RUN-DEFECTS: the build-time generated inputs, so every
    // unit of the database preprocesses under its own command.
    let build_text = text(&build)?;
    let help = capture(
        &request.tools.cmake,
        &["--build", build_text, "--target", "help"],
    )?;
    let generated_targets = generated_input_targets(&help);
    let generated = (!generated_targets.is_empty()).then(|| {
        let mut command: Vec<String> = vec![
            request.tools.cmake.clone(),
            "--build".into(),
            build_text.into(),
            "--target".into(),
        ];
        command.extend(generated_targets.iter().map(|target| (*target).to_owned()));
        command.push("-j2".into());
        command
    });
    if let Some(command) = &generated {
        logged(command, &build.join("generated-inputs-build.log"))?;
    }
    let database = build.join("compile_commands.json");
    let entries = PyJson::parse(&read_text(&database)?)
        .map_err(|error| format!("{}: {error}", database.display()))?;
    let PyJson::Array(entries) = entries else {
        return Err(format!("{} is not a JSON array", database.display()));
    };
    let selected_source = source.join("lib/cmetrics/src/cmt_counter.c");
    let mut selected = Vec::new();
    for entry in &entries {
        let file = entry
            .get("file")
            .and_then(PyJson::as_str)
            .ok_or("compile database entry without a file")?;
        if resolve(Path::new(file))? == selected_source {
            selected.push(entry.clone());
        }
    }
    if selected.len() != 1 {
        return Err("expected exactly one compiled cmt_counter.c entry".into());
    }
    if !tracked_changes()?.is_empty() {
        return Err("native build changed tracked source files".into());
    }
    let audit_path = profile_dir.join("audit.json");
    let mut config = PyJson::parse(&read_text(&audit_path)?)
        .map_err(|error| format!("{}: {error}", audit_path.display()))?;
    let config_path = build.join("cmetrics-audit.json.config");
    config.set("profile_directory", PyJson::string(text(&profile_dir)?))?;
    config.set("checkout", PyJson::string(text(&source)?))?;
    config.set("compile_database", PyJson::string(text(&database)?))?;
    config.set(
        "report",
        PyJson::string(text(&build.join("cmetrics-audit.json"))?),
    )?;
    if historical {
        let contracts = config
            .get_mut("source_contracts")
            .ok_or("audit.json has no source_contracts")?;
        contracts.set(
            "configuration",
            PyJson::string(text(&profile_dir.join("source-contracts.json"))?),
        )?;
        contracts.set(
            "source_rules",
            PyJson::string(text(&request.repository.join(
                "providers/quirin-provider-shared/contracts/library/source_projection.qv",
            ))?),
        )?;
    } else {
        config.set("source_revision_policy", PyJson::string("checkout_head"))?;
        config.remove("source_contracts")?;
    }
    write_text(&config_path, &config.dumps_indent2()?)?;
    let provenance = PyJson::Object(vec![
        ("schema_version".into(), PyJson::Integer(1)),
        ("source_revision".into(), PyJson::string(&pin)),
        (
            "requested_revision".into(),
            PyJson::string(request.revision),
        ),
        (
            "source_revision_policy".into(),
            PyJson::string(if historical {
                "profile_pin"
            } else {
                "checkout_head"
            }),
        ),
        ("upstream".into(), PyJson::string(text(&upstream)?)),
        ("source".into(), PyJson::string(text(&source)?)),
        ("build".into(), PyJson::string(text(&build)?)),
        ("compiler_version".into(), PyJson::string(&compiler_version)),
        (
            "configure_command".into(),
            PyJson::strings(configure.clone()),
        ),
        ("native_build_command".into(), PyJson::strings(native)),
        (
            "generated_inputs_build_command".into(),
            generated.map_or(PyJson::Null, PyJson::strings),
        ),
        (
            "compile_database_sha256".into(),
            PyJson::string(digest(&database)?),
        ),
        ("compile_entry".into(), selected.remove(0)),
        (
            "source_sha256".into(),
            PyJson::string(digest(&selected_source)?),
        ),
        ("cmake_cache_sha256".into(), PyJson::string(digest(&cache)?)),
        (
            "static_library_sha256".into(),
            PyJson::string(digest(&build.join("library/libcmetrics.a"))?),
        ),
        ("scope".into(), PyJson::string(CLANG_BUILD_SCOPE)),
    ]);
    let provenance_path = build.join("build-provenance.json");
    write_text(&provenance_path, &provenance.dumps_indent2()?)?;
    Ok(ClangBuildPrepared {
        pin,
        upstream,
        output,
        source,
        build,
        database,
        config_path,
        provenance_path,
        clang,
        clangxx,
        compiler_version,
        configure,
    })
}

struct MonkeyRequest<'a> {
    repository: &'a Path,
    fluent_bit: &'a Path,
    output: Option<&'a Path>,
    tools: &'a Tools,
}

struct MonkeyPrepared {
    marker: PathBuf,
    database: PathBuf,
    command: Vec<String>,
}

/// Port of the retired Cycle 19 Monkey preparation script.
fn prepare_monkey(request: &MonkeyRequest) -> Result<MonkeyPrepared, String> {
    eprintln!("prepare: preparing Monkey inputs");
    let upstream = strict_resolve(request.fluent_bit)?;
    let out = resolve(
        &request
            .output
            .map_or_else(|| request.repository.join("build/cycle19"), Path::to_owned),
    )?;
    let clang = which(&request.tools.clang);
    let clangxx = which(&request.tools.clangxx);
    let (Some(clang), Some(clangxx)) = (clang, clangxx) else {
        return Err("explicit Clang C and C++ compilers are required for audit preparation".into());
    };
    if !capture(&clang, &["--version"])?.contains("clang version") {
        return Err("explicit Clang C and C++ compilers are required for audit preparation".into());
    }
    let cache = out.join("monkey-build/CMakeCache.txt");
    if cache.exists() {
        let values = cmake_cache(&read_text(&cache)?);
        for (key, expected) in [
            ("CMAKE_C_COMPILER", &clang),
            ("CMAKE_CXX_COMPILER", &clangxx),
        ] {
            if !lookup_typed(&values, key).is_some_and(|cached| same_file(cached, expected)) {
                return Err(
                    "existing Monkey build uses another compiler; choose a fresh --output directory"
                        .into(),
                );
            }
        }
    }
    let upstream_text = text(&upstream)?;
    let revision = capture("git", &["-C", upstream_text, "rev-parse", "HEAD"])?;
    if !capture(
        "git",
        &[
            "-C",
            upstream_text,
            "status",
            "--porcelain",
            "--",
            "lib/monkey",
        ],
    )?
    .is_empty()
    {
        return Err(
            "Fluent Bit lib/monkey has local changes; source revision would be ambiguous".into(),
        );
    }
    let marker = out.join("qcore-monkey-inputs.json");
    if out.exists() && !directory_is_empty(&out)? && !marker.exists() {
        return Err(format!(
            "{} is not a managed QCore Monkey input directory; choose a fresh output",
            out.display()
        ));
    }
    create_dir_all(&out)?;
    let source = out.join("monkey-source");
    let build = out.join("monkey-build");
    let tracked = run_raw(
        "git",
        &["-C", upstream_text, "ls-files", "-z", "--", "lib/monkey"],
    )?;
    let names: Vec<String> = tracked
        .split('\0')
        .filter(|name| !name.is_empty())
        .map(|name| {
            Path::new(name)
                .strip_prefix("lib/monkey")
                .map_err(|_| format!("{name} is not under lib/monkey"))
                .and_then(|relative| text(relative).map(str::to_owned))
        })
        .collect::<Result<_, _>>()?;
    if names.is_empty() {
        return Err("checkout has no tracked Monkey source files".into());
    }
    let upstream_monkey = upstream.join("lib/monkey");
    let source_hashes = file_hashes(&upstream_monkey, names.iter().map(String::as_str))?;
    let mut metadata = PyJson::Object(vec![
        ("schema_version".into(), PyJson::Integer(1)),
        ("source_revision".into(), PyJson::string(&revision)),
        ("upstream".into(), PyJson::string(upstream_text)),
        ("source".into(), PyJson::string(text(&source)?)),
        ("build".into(), PyJson::string(text(&build)?)),
        ("source_sha256".into(), hashes_json(&source_hashes)),
        ("status".into(), PyJson::string("configuring")),
    ]);
    if marker.exists() {
        let previous = PyJson::parse(&read_text(&marker)?)
            .map_err(|error| format!("{}: {error}", marker.display()))?;
        if !matches!(previous, PyJson::Object(_)) {
            return Err(format!("{} is not a JSON object", marker.display()));
        }
        let recorded: BTreeMap<String, Option<String>> = match previous.get("source_sha256") {
            None => BTreeMap::new(),
            Some(PyJson::Object(entries)) => entries
                .iter()
                .filter(|(name, _)| !GENERATED_SOURCE_FILES.contains(&name.as_str()))
                .map(|(name, digest)| (name.clone(), digest.as_str().map(str::to_owned)))
                .collect(),
            Some(_) => {
                return Err("managed input identity changed; use a fresh output directory".into());
            }
        };
        let expected: BTreeMap<String, Option<String>> = source_hashes
            .iter()
            .map(|(name, digest)| (name.clone(), Some(digest.clone())))
            .collect();
        if previous.get("source_revision").and_then(PyJson::as_str) != Some(revision.as_str())
            || recorded != expected
        {
            return Err("managed input identity changed; use a fresh output directory".into());
        }
    }
    if source.exists() {
        if file_hashes(&source, names.iter().map(String::as_str))? != source_hashes {
            return Err("persistent source copy changed; use a fresh output directory".into());
        }
    } else {
        write_text(&marker, &metadata.dumps_indent2()?)?;
        for name in &names {
            let destination = source.join(name);
            if let Some(parent) = destination.parent() {
                create_dir_all(parent)?;
            }
            copy_with_times(&upstream_monkey.join(name), &destination)?;
        }
    }
    let known: BTreeSet<&str> = names.iter().map(String::as_str).collect();
    let extras = regular_files(&source)?;
    if extras.iter().any(|name| {
        !known.contains(name.as_str()) && !GENERATED_SOURCE_FILES.contains(&name.as_str())
    }) {
        return Err("persistent source copy has unexpected extra files".into());
    }
    write_text(&marker, &metadata.dumps_indent2()?)?;
    let mut command: Vec<String> = vec![
        request.tools.cmake.clone(),
        "-S".into(),
        text(&source)?.into(),
        "-B".into(),
        text(&build)?.into(),
        format!("-DCMAKE_C_COMPILER={clang}"),
        format!("-DCMAKE_CXX_COMPILER={clangxx}"),
        "-DCMAKE_C_STANDARD=17".into(),
        "-DCMAKE_C_FLAGS:STRING=-std=gnu17".into(),
        "-DCMAKE_BUILD_TYPE=Debug".into(),
        "-DCMAKE_EXPORT_COMPILE_COMMANDS=ON".into(),
        "-DMK_EVENT_LOOP_LIBEVENT=ON".into(),
        "-DMK_EVENT_LOOP_EPOLL=OFF".into(),
        "-DMK_EVENT_LOOP_KQUEUE=OFF".into(),
        "-DMK_EVENT_LOOP_POLL=OFF".into(),
        "-DMK_EVENT_LOOP_SELECT=OFF".into(),
        "-DMK_WITHOUT_BIN=ON".into(),
        "-DMK_WITHOUT_CONF=ON".into(),
        "-DMK_TLS=OFF".into(),
        "-DMK_TESTS=OFF".into(),
    ];
    command.extend(
        [
            "AUTH",
            "CGI",
            "CHEETAH",
            "DIRLISTING",
            "FASTCGI",
            "LOGGER",
            "MANDRIL",
        ]
        .map(|name| format!("-DMK_PLUGIN_{name}=OFF")),
    );
    logged_with_order(&command, &out.join("configure.log"), true)?;
    let database = build.join("compile_commands.json");
    let entries = PyJson::parse(&read_text(&database)?)
        .map_err(|error| format!("{}: {error}", database.display()))?;
    let PyJson::Array(entries) = entries else {
        return Err(format!("{} is not a JSON array", database.display()));
    };
    let mut event = Vec::new();
    for entry in &entries {
        let file = entry
            .get("file")
            .and_then(PyJson::as_str)
            .ok_or("compile database entry without a file")?;
        if pure_posix(file).ends_with("/mk_core/mk_event.c") {
            event.push(entry);
        }
    }
    let libevent = |entry: &PyJson| match entry.get("command") {
        Some(PyJson::String(command)) => command.contains("-DMK_EVENT_LOOP_LIBEVENT"),
        Some(PyJson::Array(words)) => words
            .iter()
            .any(|word| word.as_str() == Some("-DMK_EVENT_LOOP_LIBEVENT")),
        _ => false,
    };
    if event.len() != 1 || !libevent(event[0]) {
        return Err("generated database lacks the unique Libevent mk_event.c entry".into());
    }
    for entry in &entries {
        let file = entry.get("file").and_then(PyJson::as_str).unwrap_or("");
        let directory = entry.get("directory").and_then(PyJson::as_str);
        let directory_ok = directory.is_some_and(|directory| Path::new(directory).is_dir());
        if !Path::new(file).is_file() || !directory_ok {
            return Err(
                "generated compile database has missing source or working directories".into(),
            );
        }
    }
    let generated: Vec<&str> = GENERATED_SOURCE_FILES
        .iter()
        .copied()
        .filter(|name| source.join(name).is_file())
        .collect();
    metadata.set("status", PyJson::string("configured"))?;
    metadata.set("cmake_command", PyJson::strings(command.clone()))?;
    metadata.set(
        "compile_entries",
        PyJson::Integer(i128::try_from(entries.len()).map_err(|error| error.to_string())?),
    )?;
    metadata.set(
        "compiler",
        PyJson::string(text(&resolve(Path::new(&clang))?)?),
    )?;
    metadata.set(
        "compiler_version",
        PyJson::string(capture(&clang, &["--version"])?),
    )?;
    metadata.set(
        "generated_source_sha256",
        hashes_json(&file_hashes(&source, generated)?),
    )?;
    metadata.set("compile_database", PyJson::string(text(&database)?))?;
    metadata.set(
        "compile_database_sha256",
        PyJson::string(digest(&database)?),
    )?;
    write_text(&marker, &metadata.dumps_indent2()?)?;
    Ok(MonkeyPrepared {
        marker,
        database,
        command,
    })
}

/// Both steps into one output, the provenance record, and the optional run.
fn prepare_all(
    options: &Options,
    arguments: &[String],
    cwd: &Path,
    repository: &Path,
    fluent_bit: &Path,
    revision: &str,
    tools: &Tools,
) -> Result<ExitCode, String> {
    let upstream = strict_resolve(fluent_bit)?;
    let pin = capture(
        "git",
        &[
            "-C",
            text(&upstream)?,
            "rev-parse",
            "--verify",
            "--end-of-options",
            &format!("{revision}^{{commit}}"),
        ],
    )?;
    let output = match &options.output {
        Some(path) => path.clone(),
        None => repository.join("build/cycle19/clang").join(&pin),
    };
    // A chained run checks its Clang before the long build, not after it.
    let audit_clang = if options.run {
        Some(chained_clang(tools)?)
    } else {
        None
    };
    let clang_build = prepare_clang_build(&ClangBuildRequest {
        repository,
        fluent_bit: &upstream,
        revision,
        output: Some(&output),
        tools,
    })?;
    if clang_build.pin != pin {
        return Err("the Fluent Bit revision moved during preparation".into());
    }
    println!("{}", clang_build.config_path.display());
    let monkey_output = clang_build.output.join("monkey");
    let monkey = prepare_monkey(&MonkeyRequest {
        repository,
        fluent_bit: &clang_build.source,
        output: Some(&monkey_output),
        tools,
    })?;
    println!("{}", monkey.database.display());
    let linked = if options.link {
        let linked = link(&clang_build, repository, tools, options)?;
        println!("{}", linked.artifact.display());
        println!("{}", linked.candidate.display());
        Some(linked)
    } else {
        None
    };
    let reports = clang_build.output.join("reports");
    let mut current: Vec<String> = vec![
        "current".into(),
        "--checkout".into(),
        text(&clang_build.source)?.into(),
        "--profile-directory".into(),
        text(&repository.join("contract/profiles/fluent-bit"))?.into(),
        "--compile-database".into(),
        text(&clang_build.database)?.into(),
        "--monkey-database".into(),
        text(&monkey.database)?.into(),
        // Card CURRENT-MONKEY-LINKED-CONFIGURATION (user decision
        // 2026-11-08, "Analyse linked config"): Monkey is analysed as the
        // compile database configures it, the configuration linked into
        // fluent-bit; the standalone database above is only accounted for.
        "--monkey-linked".into(),
        "--monkey-source-report".into(),
        text(&reports.join("monkey-source.json"))?.into(),
        "--report".into(),
        text(&reports.join("resource-diagnostics.json"))?.into(),
    ];
    if let Some(jobs) = &options.jobs {
        current.extend(["--jobs".into(), jobs.clone()]);
    }
    current.extend(options.forwarded.iter().cloned());
    let version = |program: &str| capture(program, &["--version"]);
    let sha = |path: &Path| digest(path);
    let record = json!({
        "schema_version": 1,
        "tool": "quirin-audit prepare",
        "tool_version": env!("CARGO_PKG_VERSION"),
        "build_profile": if cfg!(debug_assertions) { "dev" } else { "release" },
        "argv": std::iter::once("quirin-audit".to_owned()).chain(std::iter::once("prepare".to_owned())).chain(arguments.iter().cloned()).collect::<Vec<_>>(),
        "cwd": cwd,
        "repository": repository,
        "upstream": clang_build.upstream,
        "requested_revision": revision,
        "source_revision": pin,
        "output": clang_build.output,
        "compilers": {
            "c": {
                "requested": tools.clang,
                "path": clang_build.clang,
                "resolved": resolve(Path::new(&clang_build.clang))?,
                "version": clang_build.compiler_version,
            },
            "cxx": {
                "requested": tools.clangxx,
                "path": clang_build.clangxx,
                "resolved": resolve(Path::new(&clang_build.clangxx))?,
                "version": version(&clang_build.clangxx)?,
            },
        },
        "cmake": {"program": tools.cmake, "version": version(&tools.cmake)?},
        "clang_build": {
            "source": clang_build.source,
            "build": clang_build.build,
            "configure_command": clang_build.configure,
            "configuration": clang_build.config_path,
            "configuration_sha256": sha(&clang_build.config_path)?,
            "provenance": clang_build.provenance_path,
            "provenance_sha256": sha(&clang_build.provenance_path)?,
            "compile_database": clang_build.database,
            "compile_database_sha256": sha(&clang_build.database)?,
        },
        "monkey": {
            "output": monkey_output,
            "cmake_command": monkey.command,
            "marker": monkey.marker,
            "marker_sha256": sha(&monkey.marker)?,
            "compile_database": monkey.database,
            "compile_database_sha256": sha(&monkey.database)?,
        },
        "run": {
            "requested": options.run,
            "current_argv": options.run.then(|| current.clone()),
            "QCORE_CLANG": audit_clang,
        },
        "scope": "provenance of the preparation only; not an input to any verdict. The Clang CMake build is CMake's unmodified output of an independent clone and proves no GCC, code-generation or whole-library equivalence; the C++ compiler is used by preparation only, never by the audit",
    });
    let mut record = record;
    if let Some(linked) = linked {
        record["link_resolution"] = linked.record;
    }
    let record_path = clang_build.output.join("prepare.json");
    let record_text = serde_json::to_string_pretty(&record).map_err(|error| error.to_string())?;
    write_text(&record_path, &(record_text + "\n"))?;
    println!("{}", record_path.display());
    let Some(audit_clang) = audit_clang else {
        return Ok(ExitCode::SUCCESS);
    };
    create_dir_all(&reports)?;
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    eprintln!(
        "running: QCORE_CLANG={audit_clang} {} {}",
        executable.display(),
        current.join(" ")
    );
    let status = Command::new(&executable)
        .args(&current)
        .env("QCORE_CLANG", &audit_clang)
        .status()
        .map_err(|error| format!("{}: {error}", executable.display()))?;
    Ok(match status.code() {
        Some(0) => ExitCode::SUCCESS,
        Some(code) => ExitCode::from(u8::try_from(code).unwrap_or(1)),
        None => ExitCode::FAILURE,
    })
}

/// The Clang the chained audit runs: the prepared C compiler. An explicit
/// `QCORE_CLANG` naming another executable refuses rather than letting the
/// audit reject every unit after the build.
fn chained_clang(tools: &Tools) -> Result<String, String> {
    let clang = which(&tools.clang).ok_or("both Clang C and C++ compilers are required")?;
    if let Some(configured) = std::env::var_os("QCORE_CLANG") {
        let configured = configured
            .into_string()
            .map_err(|value| format!("non-UTF-8 QCORE_CLANG {value:?}"))?;
        let found = which(&configured)
            .ok_or_else(|| format!("QCORE_CLANG `{configured}` cannot be resolved"))?;
        if !same_file(&found, &clang) {
            return Err(format!(
                "QCORE_CLANG ({configured}) is not the prepared C compiler ({clang}); the audit \
                 requires the compiler its compile database names"
            ));
        }
    }
    Ok(clang)
}

// ---- Python-equivalent helpers -------------------------------------------

fn text(path: &Path) -> Result<&str, String> {
    path.to_str()
        .ok_or_else(|| format!("non-UTF-8 path {}", path.display()))
}

fn read_text(path: &Path) -> Result<String, String> {
    fs::read(path)
        .map_err(|error| format!("{}: {error}", path.display()))
        .and_then(decode)
}

fn write_text(path: &Path, contents: &str) -> Result<(), String> {
    fs::write(path, contents).map_err(|error| format!("{}: {error}", path.display()))
}

fn create_dir_all(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path).map_err(|error| format!("{}: {error}", path.display()))
}

fn digest(path: &Path) -> Result<String, String> {
    fs::read(path)
        .map(|bytes| sha256_hex(&bytes))
        .map_err(|error| format!("{}: {error}", path.display()))
}

/// Python's text mode: strict UTF-8 with universal newlines.
fn decode(bytes: Vec<u8>) -> Result<String, String> {
    let text = String::from_utf8(bytes).map_err(|error| error.to_string())?;
    Ok(if text.contains('\r') {
        text.replace("\r\n", "\n").replace('\r', "\n")
    } else {
        text
    })
}

/// `str.strip()`: Unicode whitespace plus the ASCII separators Python counts.
fn python_strip(text: &str) -> &str {
    text.trim_matches(|character: char| {
        character.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&character)
    })
}

/// `subprocess.run(..., check=True, text=True, capture_output=True).stdout`.
fn run_raw(program: &str, arguments: &[&str]) -> Result<String, String> {
    let output = Command::new(program)
        .args(arguments)
        .output()
        .map_err(|error| format!("{program}: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "command {:?} failed ({}): {}",
            std::iter::once(program)
                .chain(arguments.iter().copied())
                .collect::<Vec<_>>(),
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    decode(output.stdout)
}

/// The scripts' `capture`/`run`: checked, stripped standard output.
fn capture(program: &str, arguments: &[&str]) -> Result<String, String> {
    run_raw(program, arguments).map(|output| python_strip(&output).to_owned())
}

/// Stream combined output to the log while the command runs, retaining the
/// scripts' final UTF-8/universal-newline representation.
fn logged(command: &[String], path: &Path) -> Result<(), String> {
    logged_with_order(command, path, false)
}

/// Monkey's historical log places stdout before stderr. Keep a live stderr
/// sibling during execution, then concatenate the two in that same order.
/// An interrupted command leaves its available output on disk.
fn logged_with_order(command: &[String], path: &Path, separate_stderr: bool) -> Result<(), String> {
    let stdout = fs::File::create(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let stderr_path = path.with_extension("stderr.log");
    let stderr = if separate_stderr {
        fs::File::create(&stderr_path)
            .map_err(|error| format!("{}: {error}", stderr_path.display()))?
    } else {
        stdout.try_clone().map_err(|error| error.to_string())?
    };
    eprintln!(
        "prepare: running {}; live log: {}",
        command[0],
        path.display()
    );
    if separate_stderr {
        eprintln!("prepare: live stderr: {}", stderr_path.display());
    }
    let started = Instant::now();
    let status = Command::new(&command[0])
        .args(&command[1..])
        .stdout(stdout)
        .stderr(stderr)
        .status()
        .map_err(|error| format!("{}: {error}; see {}", command[0], path.display()))?;
    let mut contents = read_text(path)?;
    if separate_stderr {
        contents.push_str(&read_text(&stderr_path)?);
    }
    write_text(path, &contents)?;
    if separate_stderr {
        fs::remove_file(&stderr_path)
            .map_err(|error| format!("{}: {error}", stderr_path.display()))?;
    }
    eprintln!(
        "prepare: {} finished ({status}) in {:.1}s; log: {}",
        command[0],
        started.elapsed().as_secs_f64(),
        path.display()
    );
    if !status.success() {
        let code = status
            .code()
            .map_or_else(|| status.to_string(), |code| code.to_string());
        return Err(format!("command failed ({code}); see {}", path.display()));
    }
    Ok(())
}

/// `Path.resolve(strict=True)`.
fn strict_resolve(path: &Path) -> Result<PathBuf, String> {
    path.canonicalize()
        .map_err(|error| format!("{}: {error}", path.display()))
}

/// `Path.resolve()` (non-strict): symlinks resolved for the components that
/// exist, the rest applied lexically, relative paths against the working
/// directory.
fn resolve(path: &Path) -> Result<PathBuf, String> {
    let absolute = if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir()
            .map_err(|error| error.to_string())?
            .join(path)
    };
    let mut resolved = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::Prefix(_) | Component::RootDir => resolved.push(component),
            Component::CurDir => {}
            Component::ParentDir => {
                resolved.pop();
            }
            Component::Normal(name) => {
                let candidate = resolved.join(name);
                resolved = candidate.canonicalize().unwrap_or(candidate);
            }
        }
    }
    Ok(resolved)
}

fn same_file(left: &str, right: &str) -> bool {
    match (resolve(Path::new(left)), resolve(Path::new(right))) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

/// `PurePath(text).as_posix()` for the suffix test: repeated separators and
/// `.` components collapse.
fn pure_posix(text: &str) -> String {
    let path = Path::new(text);
    let mut out = String::new();
    for component in path.components() {
        match component {
            Component::RootDir => out.push('/'),
            other => {
                if !out.is_empty() && !out.ends_with('/') {
                    out.push('/');
                }
                out.push_str(&other.as_os_str().to_string_lossy());
            }
        }
    }
    out
}

/// `shutil.which(name)`: a name with a separator is checked as given;
/// otherwise the first executable, non-directory `PATH` entry, spelled as
/// `os.path.join(directory, name)`.
fn which(name: &str) -> Option<String> {
    let usable = |path: &Path| path.exists() && executable(path) && !path.is_dir();
    if name.chars().any(std::path::is_separator) {
        return usable(Path::new(name)).then(|| name.to_owned());
    }
    let search: OsString = std::env::var_os("PATH").unwrap_or_else(|| "/bin:/usr/bin".into());
    if search.is_empty() {
        return None;
    }
    let mut seen = BTreeSet::new();
    for directory in std::env::split_paths(&search) {
        if !seen.insert(directory.clone()) {
            continue;
        }
        let candidate = directory.join(name);
        if usable(&candidate) {
            return candidate.to_str().map(str::to_owned);
        }
    }
    None
}

#[cfg(unix)]
fn executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .is_ok_and(|metadata| metadata.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn executable(path: &Path) -> bool {
    path.is_file()
}

/// `dict(line.split('=', 1) for line in text.splitlines() if ...)`: a key
/// keeps its first position and takes its last value.
fn cmake_cache(text: &str) -> Vec<(String, String)> {
    let mut values: Vec<(String, String)> = Vec::new();
    for line in text.lines() {
        if line.starts_with('#') || line.starts_with("//") {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match values.iter_mut().find(|(name, _)| name == key) {
            Some((_, slot)) => value.clone_into(slot),
            None => values.push((key.to_owned(), value.to_owned())),
        }
    }
    values
}

fn lookup_exact<'a>(values: &'a [(String, String)], key: &str) -> Option<&'a str> {
    values
        .iter()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value.as_str())
}

/// The first cache entry whose name before `:` is `key`.
fn lookup_typed<'a>(values: &'a [(String, String)], key: &str) -> Option<&'a str> {
    values
        .iter()
        .find(|(name, _)| name.split(':').next() == Some(key))
        .map(|(_, value)| value.as_str())
}

fn file_hashes<'a>(
    root: &Path,
    names: impl IntoIterator<Item = &'a str>,
) -> Result<BTreeMap<String, String>, String> {
    names
        .into_iter()
        .map(|name| Ok((name.to_owned(), digest(&root.join(name))?)))
        .collect()
}

fn hashes_json(hashes: &BTreeMap<String, String>) -> PyJson {
    PyJson::Object(
        hashes
            .iter()
            .map(|(name, digest)| (name.clone(), PyJson::string(digest)))
            .collect(),
    )
}

fn directory_is_empty(path: &Path) -> Result<bool, String> {
    Ok(fs::read_dir(path)
        .map_err(|error| format!("{}: {error}", path.display()))?
        .next()
        .is_none())
}

/// `shutil.copy2`: contents, permission bits and access/modification times.
fn copy_with_times(from: &Path, to: &Path) -> Result<(), String> {
    fs::copy(from, to)
        .map_err(|error| format!("{} -> {}: {error}", from.display(), to.display()))?;
    let metadata = fs::metadata(from).map_err(|error| format!("{}: {error}", from.display()))?;
    let times = fs::FileTimes::new()
        .set_accessed(metadata.accessed().map_err(|error| error.to_string())?)
        .set_modified(metadata.modified().map_err(|error| error.to_string())?);
    fs::File::options()
        .write(true)
        .open(to)
        .and_then(|file| file.set_times(times))
        .map_err(|error| format!("{}: {error}", to.display()))
}

/// `{p.relative_to(root) for p in root.rglob('*') if p.is_file()}`; directory
/// symlinks are not descended.
fn regular_files(root: &Path) -> Result<BTreeSet<String>, String> {
    let mut found = BTreeSet::new();
    let mut pending = vec![root.to_owned()];
    while let Some(directory) = pending.pop() {
        let entries = fs::read_dir(&directory)
            .map_err(|error| format!("{}: {error}", directory.display()))?;
        for entry in entries {
            let entry = entry.map_err(|error| error.to_string())?;
            let path = entry.path();
            let kind = entry.file_type().map_err(|error| error.to_string())?;
            if kind.is_dir() {
                pending.push(path.clone());
            }
            if path.is_file() {
                let relative = path.strip_prefix(root).map_err(|error| error.to_string())?;
                found.insert(text(relative)?.to_owned());
            }
        }
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    /// Card CURRENT-RUN-DEFECTS: the generated-input targets a build defines,
    /// read from either generator's `help` listing, in the fixed order;
    /// a target the configuration does not define is never asked for.
    #[test]
    fn generated_input_targets_are_the_listed_ones() {
        let make = "The following are some of the valid targets for this Makefile:\n\
                    ... all (the default if no target is provided)\n\
                    ... clean\n\
                    ... backtrace\n\
                    ... flb-ra-parser\n\
                    ... libluajit\n\
                    ... lj_gen_headers\n\
                    ... lj_gen_folddef\n\
                    ... buildvm_arch_h\n\
                    ... lj_vm.o\n";
        assert_eq!(
            generated_input_targets(make),
            [
                "buildvm_arch_h",
                "lj_gen_headers",
                "lj_gen_folddef",
                "libluajit",
                "flb-ra-parser",
                "backtrace"
            ]
        );
        let ninja = "all: phony\nflb-sp-parser: phony\nprocessor-sql-parser: phony\n\
                     libluajit.a: phony\n";
        assert_eq!(
            generated_input_targets(ninja),
            ["flb-sp-parser", "processor-sql-parser"]
        );
        assert!(generated_input_targets("... cmetrics-static\n... libluajit-extra\n").is_empty());
    }

    #[test]
    fn steps_and_flags_parse_and_misuse_refuses() {
        let (step, options) = parse(&args(&[
            "--fluent-bit",
            "../fluent-bit",
            "--clang",
            "/usr/lib/llvm-21/bin/clang",
            "--run",
            "--jobs",
            "4",
            "--",
            "--contracts-only",
        ]))
        .unwrap();
        assert_eq!(step, Step::All);
        assert!(options.run);
        assert_eq!(options.jobs.as_deref(), Some("4"));
        assert_eq!(options.forwarded, args(&["--contracts-only"]));
        assert_eq!(
            parse(&args(&["monkey", "--fluent-bit", "x"])).unwrap().0,
            Step::Monkey
        );
        assert_eq!(parse(&args(&["clang-build"])).unwrap().0, Step::ClangBuild);
        // Card FLUENT-BIT-LINK-PREPARE: `--link` is explicit, off by default.
        assert!(!parse(&args(&[])).unwrap().1.link);
        let (step, options) =
            parse(&args(&["clang-build", "--link", "--build-jobs", "6"])).unwrap();
        assert_eq!(step, Step::ClangBuild);
        assert!(options.link);
        assert_eq!(options.build_jobs, Some(6));
        assert!(parse(&args(&["--link", "--run"])).unwrap().1.link);
        for misuse in [
            &["monkey", "--fluent-bit", "x", "--link"][..],
            &["--build-jobs", "2"],
            &["--link", "--build-jobs", "0"],
            &["--link", "--link"],
            &["--link", "--build-jobs", "2", "--build-jobs", "3"],
            &["--jobs", "2"],
            &["--run", "--jobs", "0"],
            &["--", "--contracts-only"],
            &["clang-build", "--run"],
            &["monkey"],
            &["monkey", "--fluent-bit", "x", "--revision", "HEAD"],
            &["--clang", "a", "--clang", "b"],
            &["--output"],
            &["--frobnicate", "x"],
            &["--run", "--run"],
        ] {
            assert!(parse(&args(misuse)).is_err(), "{misuse:?}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn preparation_logs_are_readable_before_exit_and_keep_final_order_on_failure() {
        use std::time::{Duration, SystemTime, UNIX_EPOCH};
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "qcore-prepare-live-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir(&root).unwrap();
        let path = root.join("configure.log");
        let worker_path = path.clone();
        let script = "printf 'out\\r\\n'; printf 'err\\r\\n' >&2; n=0; while [ ! -e \"$1/release\" ] && [ \"$n\" -lt 1000 ]; do sleep 0.02; n=$((n+1)); done; exit 7";
        let command = vec![
            "/bin/sh".into(),
            "-c".into(),
            script.into(),
            "fixture".into(),
            root.display().to_string(),
        ];
        let worker = std::thread::spawn(move || logged_with_order(&command, &worker_path, true));
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut observed = false;
        while Instant::now() < deadline && !worker.is_finished() {
            if fs::read(&path).ok().as_deref() == Some(b"out\r\n")
                && fs::read(path.with_extension("stderr.log")).ok().as_deref() == Some(b"err\r\n")
            {
                observed = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        // Always release before asserting: a failed observation cannot strand a child.
        fs::write(root.join("release"), b"").unwrap();
        let result = worker.join().unwrap();
        assert!(
            observed,
            "logs were not readable while the command was blocked"
        );
        assert!(result.unwrap_err().contains("command failed (7)"));
        assert_eq!(fs::read_to_string(&path).unwrap(), "out\nerr\n");
        assert!(!path.with_extension("stderr.log").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn cmake_cache_keeps_the_first_position_and_the_last_value() {
        let values = cmake_cache(
            "# comment\n// doc=ignored\nCMAKE_C_COMPILER:FILEPATH=/a\nX:INTERNAL=1\n\
             CMAKE_C_COMPILER:FILEPATH=/b\nCMAKE_C_COMPILER:STRING=/c\nno equals\n",
        );
        assert_eq!(lookup_typed(&values, "CMAKE_C_COMPILER"), Some("/b"));
        assert_eq!(lookup_exact(&values, "X:INTERNAL"), Some("1"));
        assert_eq!(lookup_exact(&values, "doc"), None);
        assert_eq!(values.len(), 3);
    }

    #[test]
    fn helpers_follow_python_semantics() {
        assert_eq!(python_strip("\u{1c} x \n"), "x");
        assert_eq!(decode(b"a\r\nb\rc".to_vec()).unwrap(), "a\nb\nc");
        assert!(decode(vec![0xff]).is_err());
        assert_eq!(
            pure_posix("/a//b/./mk_core/mk_event.c"),
            "/a/b/mk_core/mk_event.c"
        );
        let missing = std::env::temp_dir().join("no-such-qcore-dir");
        assert_eq!(
            resolve(&missing.join("x/../y")).unwrap(),
            resolve(&missing).unwrap().join("y")
        );
        assert_eq!(which("/no-such-qcore-dir/clang"), None);
        assert_eq!(which("no-such-qcore-program-name"), None);
    }
}
