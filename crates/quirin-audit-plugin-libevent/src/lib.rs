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

//! `quirin-audit libevent prepare` (card **W45-AUDIT-SUBCOMMAND-PLUGINS**):
//! the pinned `libevent-core` configured-audit inputs, prepared by the same
//! git and CMake invocations the retired preparation script of card G03-F
//! ran, with the same output files byte for byte
//! (`docs/reports/w45-audit-subcommand-plugins/`).
//!
//! The audited checkout is the repository's own pinned `third_party/libevent`
//! submodule. CMake is configured with Clang so Libevent's generated
//! `event2/event-config.h` and `evconfig-private.h` exist, and
//! `event_core_static` is built natively from the same tree. CMake's
//! unmodified compilation database is the audit input; `audit.json` names the
//! target whose object directory the audit reads. This prepares inputs and
//! records provenance; it certifies no Libevent behaviour and claims no
//! verdict.
//!
//! ```text
//! quirin-audit --plugin-path DIR --output OUT libevent prepare \
//!     [--repository DIR] [--clang PATH] [--clangxx PATH] [--cmake PATH] [--write-profile]
//! ```
//!
//! Two deliberate differences from the script, neither reaching a prepared
//! byte:
//!
//! * The output is the host's declared `--output` (required; the script
//!   defaulted to `build/cycle19`). An output inside the repository's
//!   `build/cycle19` or `third_party` is refused: the build writes nothing
//!   into the submodule or the protected corpus tree.
//! * `--write-profile` writes the regenerated `profile.json` into the build
//!   directory as an artifact, for a person to review and copy to
//!   `contract/profiles/libevent-core/profile.json`, instead of overwriting the
//!   committed profile: a plugin writes only inside its output.
//!
//! The repository is `--repository DIR` (against the invocation directory),
//! else the one whose `providers/contract-load-path.json` is the request's
//! load path, else the invocation directory; it must hold the profile.

use std::path::{Path, PathBuf};

use quirin_audit_plugin::json_text::JsonText;
use quirin_audit_plugin::{Request, Response, Severity};
use serde_json::Value;

pub mod audit;

use audit::{
    capture, cmake_cache, digest, logged, lookup, read_text, resolve, suffix, text, which,
    write_text,
};

/// Usage text.
pub const USAGE: &str = "usage: quirin-audit --output DIR libevent prepare [--repository DIR] \
[--clang PATH] [--clangxx PATH] [--cmake PATH] [--write-profile]";

/// The profile directory, relative to the repository.
pub const PROFILE_DIRECTORY: &str = "contract/profiles/libevent-core";

/// The object directory of the selected target.
pub const OBJECT_PREFIX: &str = "CMakeFiles/event_core_static.dir/";

const SCOPE: &str = "CMake's unmodified compilation database is the audit input. The audit \
reads only the records whose object output lies under the declared target prefix; every \
remaining build option is transported into preprocessing, syntax and the typed AST. Bounded C \
resource projection only: not GCC equivalence, not code generation, not whole-library safety.";

/// The flags of `prepare`.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Options {
    pub repository: Option<String>,
    pub clang: Option<String>,
    pub clangxx: Option<String>,
    pub cmake: Option<String>,
    pub write_profile: bool,
}

/// Parse `prepare`'s flags (after the word `prepare`).
///
/// # Errors
///
/// An unknown, repeated or valueless flag.
pub fn parse(arguments: &[String]) -> Result<Options, String> {
    let mut options = Options::default();
    let mut index = 0;
    while index < arguments.len() {
        let flag = arguments[index].as_str();
        if flag == "--write-profile" {
            if options.write_profile {
                return Err("--write-profile is given twice".into());
            }
            options.write_profile = true;
            index += 1;
            continue;
        }
        let slot = match flag {
            "--repository" => &mut options.repository,
            "--clang" => &mut options.clang,
            "--clangxx" => &mut options.clangxx,
            "--cmake" => &mut options.cmake,
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

/// Answer a request: `prepare` and its flags.
///
/// # Errors
///
/// A refusal or failure, with its reason (the response is then `failed`).
pub fn answer(request: &Request) -> Result<Response, String> {
    match request.arguments.split_first() {
        Some((step, rest)) if step == "prepare" => prepare(request, &parse(rest)?),
        _ => Err(USAGE.into()),
    }
}

/// The repository whose profile and submodule are prepared.
///
/// # Errors
///
/// A directory without the libevent-core profile.
pub fn repository(request: &Request, flag: Option<&str>) -> Result<PathBuf, String> {
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
    if !root.join(PROFILE_DIRECTORY).join("profile.json").is_file() {
        return Err(format!(
            "{} is not the QCore-C repository (no {PROFILE_DIRECTORY}/profile.json); pass \
             --repository DIR",
            root.display()
        ));
    }
    Ok(root)
}

/// The root-level `.c` files the selected target compiles.
///
/// # Errors
///
/// A malformed entry (a missing `file`, a trailing `-o`, a non-string
/// output).
pub fn selected_units(
    entries: &[Value],
    checkout: &Path,
    base: &Path,
) -> Result<Vec<String>, String> {
    let mut units = std::collections::BTreeSet::new();
    for entry in entries {
        let command = match entry.get("command").and_then(Value::as_str) {
            Some(command) if !command.is_empty() => command.to_owned(),
            _ => entry["arguments"]
                .as_array()
                .ok_or("compile database entry without a command or arguments")?
                .iter()
                .map(|word| word.as_str().ok_or("a non-string argument"))
                .collect::<Result<Vec<_>, _>>()?
                .join(" "),
        };
        let output = match entry.get("output") {
            Some(Value::Null) | None => {
                let words: Vec<&str> = command.split_whitespace().collect();
                match words.iter().position(|word| *word == "-o") {
                    Some(index) => words
                        .get(index + 1)
                        .ok_or("a command ends with -o")?
                        .to_string(),
                    None => String::new(),
                }
            }
            Some(Value::String(output)) => output.clone(),
            Some(_) => return Err("compile database entry with a non-string output".into()),
        };
        if !output.starts_with(OBJECT_PREFIX) {
            continue;
        }
        let file = entry["file"]
            .as_str()
            .ok_or("compile database entry without a file")?;
        let source = resolve(Path::new(file), base);
        if source.parent() == Some(checkout) && suffix(&source) == ".c" {
            let name = source
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or("a non-UTF-8 source name")?;
            units.insert(name.to_owned());
        }
    }
    Ok(units.into_iter().collect())
}

/// Refuse an output inside the repository's protected trees.
///
/// # Errors
///
/// An output inside `build/cycle19` or `third_party` of the repository.
pub fn check_output(repository: &Path, output: &Path) -> Result<(), String> {
    for protected in ["build/cycle19", "third_party"] {
        let protected = repository.join(protected);
        if output.starts_with(&protected) {
            return Err(format!(
                "the output {} lies inside the protected {}; choose an output outside it",
                output.display(),
                protected.display()
            ));
        }
    }
    Ok(())
}

/// Prepare the pinned `libevent-core` inputs.
///
/// # Errors
///
/// Every refusal of the preparation, with its message.
pub fn prepare(request: &Request, options: &Options) -> Result<Response, String> {
    let root = repository(request, options.repository.as_deref())?;
    let profile_dir = root.join(PROFILE_DIRECTORY);
    let output = request.output_directory();
    check_output(&root, &output)?;
    let profile_path = profile_dir.join("profile.json");
    let mut profile = JsonText::parse(&read_text(&profile_path)?)
        .map_err(|error| format!("{}: {error}", profile_path.display()))?;
    let pin = profile
        .get("pins")
        .and_then(|pins| pins.get("libevent"))
        .and_then(|pin| pin.get("git_commit"))
        .and_then(JsonText::as_str)
        .ok_or("the profile has no pins.libevent.git_commit")?
        .to_owned();

    let checkout_spelled = root.join("third_party/libevent");
    let checkout = checkout_spelled
        .canonicalize()
        .map_err(|error| format!("{}: {error}", checkout_spelled.display()))?;
    let checkout_text = text(&checkout)?.to_owned();
    if capture("git", &["-C", &checkout_text, "rev-parse", "HEAD"])? != pin {
        return Err("third_party/libevent HEAD does not match the configured source pin".into());
    }
    let tracked_changes = || {
        capture(
            "git",
            &[
                "-C",
                &checkout_text,
                "status",
                "--porcelain",
                "--untracked-files=no",
            ],
        )
    };
    if !tracked_changes()?.is_empty() {
        return Err("third_party/libevent has tracked changes; the pin is not clean".into());
    }

    let (Some(clang), Some(clangxx)) = (
        which(options.clang.as_deref().unwrap_or("clang")),
        which(options.clangxx.as_deref().unwrap_or("clang++")),
    ) else {
        return Err("both Clang C and C++ compilers are required".into());
    };
    let compiler_version = capture(&clang, &["--version"])?;
    if !compiler_version.contains("clang version") {
        return Err("C compiler does not identify itself as Clang".into());
    }
    let cmake = options.cmake.clone().unwrap_or_else(|| "cmake".into());

    let build =
        resolve(&output, Path::new(&request.invocation_directory)).join("libevent-clang-build");
    std::fs::create_dir_all(&build).map_err(|error| format!("{}: {error}", build.display()))?;
    let cache = build.join("CMakeCache.txt");
    if cache.exists() {
        let values = cmake_cache(&read_text(&cache)?);
        if lookup(&values, "CMAKE_HOME_DIRECTORY:INTERNAL") != Some(checkout_text.as_str()) {
            return Err("build cache belongs to another source; choose a fresh output".into());
        }
        let cached = match lookup(&values, "CMAKE_C_COMPILER:FILEPATH") {
            Some(value) => Some(value),
            None => lookup(&values, "CMAKE_C_COMPILER:STRING"),
        };
        let base = Path::new(&request.invocation_directory);
        let same = cached.is_some_and(|cached| {
            !cached.is_empty()
                && resolve(Path::new(cached), base) == resolve(Path::new(&clang), base)
        });
        if !same {
            return Err("build cache uses another compiler; choose a fresh output".into());
        }
    }

    let build_text = text(&build)?.to_owned();
    let configure: Vec<String> = vec![
        cmake.clone(),
        "-S".into(),
        checkout_text.clone(),
        "-B".into(),
        build_text.clone(),
        format!("-DCMAKE_C_COMPILER={clang}"),
        format!("-DCMAKE_CXX_COMPILER={clangxx}"),
        "-DCMAKE_C_STANDARD=17".into(),
        "-DCMAKE_EXPORT_COMPILE_COMMANDS=ON".into(),
        "-DEVENT__LIBRARY_TYPE=STATIC".into(),
        "-DEVENT__DISABLE_OPENSSL=ON".into(),
        "-DEVENT__DISABLE_MBEDTLS=ON".into(),
        "-DEVENT__DISABLE_TESTS=ON".into(),
        "-DEVENT__DISABLE_REGRESS=ON".into(),
        "-DEVENT__DISABLE_SAMPLES=ON".into(),
        "-DEVENT__DISABLE_BENCHMARK=ON".into(),
    ];
    logged(&configure, &build.join("configure.log"))?;
    let native: Vec<String> = vec![
        cmake,
        "--build".into(),
        build_text.clone(),
        "--target".into(),
        "event_core_static".into(),
        "-j2".into(),
    ];
    logged(&native, &build.join("native-build.log"))?;
    if !tracked_changes()?.is_empty() {
        return Err("the native build changed tracked source files".into());
    }

    let database = build.join("compile_commands.json");
    let entries: Value = serde_json::from_str(&read_text(&database)?)
        .map_err(|error| format!("{}: {error}", database.display()))?;
    let entries = entries
        .as_array()
        .ok_or_else(|| format!("{} is not a JSON array", database.display()))?;
    let units = selected_units(entries, &checkout, Path::new(&request.invocation_directory))?;
    if units.is_empty() {
        return Err("the selected target compiles no root-level C source".into());
    }

    let mut regenerated = None;
    if options.write_profile {
        let files = units
            .iter()
            .map(|name| {
                Ok(JsonText::Object(vec![
                    ("path".into(), JsonText::string(name)),
                    (
                        "sha256".into(),
                        JsonText::string(digest(&checkout.join(name))?),
                    ),
                ]))
            })
            .collect::<Result<Vec<_>, String>>()?;
        profile
            .get_mut("libraries")
            .and_then(|libraries| libraries.get_mut("libevent-core"))
            .ok_or("the profile has no libraries.libevent-core")?
            .set("audited_files", JsonText::Array(files))?;
        let path = build.join("profile.json");
        write_text(&path, &profile.to_indented_text()?)?;
        regenerated = Some(path);
    } else {
        let declared = profile
            .get("libraries")
            .and_then(|libraries| libraries.get("libevent-core"))
            .and_then(|library| library.get("audited_files"))
            .ok_or("the profile has no libraries.libevent-core.audited_files")?;
        let JsonText::Array(declared) = declared else {
            return Err("audited_files is not a list".into());
        };
        // Path to digest; the last digest of a repeated path wins.
        let mut table: Vec<(String, String)> = Vec::new();
        for entry in declared {
            let (Some(path), Some(sha256)) = (
                entry.get("path").and_then(JsonText::as_str),
                entry.get("sha256").and_then(JsonText::as_str),
            ) else {
                return Err("an audited file without a path or sha256".into());
            };
            match table.iter_mut().find(|(name, _)| name == path) {
                Some((_, slot)) => sha256.clone_into(slot),
                None => table.push((path.to_owned(), sha256.to_owned())),
            }
        }
        let mut names: Vec<&String> = table.iter().map(|(name, _)| name).collect();
        names.sort();
        if names.into_iter().ne(units.iter()) {
            return Err(
                "the selected target's unit set differs from the declared audited files; rerun \
                 with --write-profile and review the diff"
                    .into(),
            );
        }
        for (name, expected) in &table {
            if &digest(&checkout.join(name))? != expected {
                return Err(format!(
                    "audited file {name} does not match its declared digest"
                ));
            }
        }
    }

    let audit_path = profile_dir.join("audit.json");
    let mut config = JsonText::parse(&read_text(&audit_path)?)
        .map_err(|error| format!("{}: {error}", audit_path.display()))?;
    config.set("profile_directory", JsonText::string(text(&profile_dir)?))?;
    config.set("checkout", JsonText::string(checkout_text.clone()))?;
    config.set("compile_database", JsonText::string(text(&database)?))?;
    config.set(
        "report",
        JsonText::string(text(&build.join("libevent-core-audit.json"))?),
    )?;
    let config_path = build.join("libevent-core-audit.json.config");
    write_text(&config_path, &config.to_indented_text()?)?;

    let source_sha256 = units
        .iter()
        .map(|name| {
            Ok((
                name.clone(),
                JsonText::string(digest(&checkout.join(name))?),
            ))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let provenance = JsonText::Object(vec![
        ("schema_version".into(), JsonText::Integer(1)),
        ("source_set".into(), JsonText::string("libevent-core")),
        ("source_revision".into(), JsonText::string(pin)),
        ("checkout".into(), JsonText::string(checkout_text)),
        ("build".into(), JsonText::string(build_text)),
        (
            "compiler_version".into(),
            JsonText::string(compiler_version),
        ),
        ("configure_command".into(), JsonText::strings(configure)),
        ("native_build_command".into(), JsonText::strings(native)),
        ("object_prefix".into(), JsonText::string(OBJECT_PREFIX)),
        ("selected_units".into(), JsonText::strings(units.clone())),
        (
            "cmake_database_sha256".into(),
            JsonText::string(digest(&database)?),
        ),
        (
            "cmake_database_entries".into(),
            JsonText::Integer(i128::try_from(entries.len()).map_err(|error| error.to_string())?),
        ),
        (
            "static_library_sha256".into(),
            JsonText::string(digest(&build.join("lib/libevent_core.a"))?),
        ),
        ("source_sha256".into(), JsonText::Object(source_sha256)),
        ("scope".into(), JsonText::string(SCOPE)),
    ]);
    let provenance_path = build.join("libevent-build-provenance.json");
    write_text(&provenance_path, &provenance.to_indented_text()?)?;

    let mut response = Response::completed();
    for (path, kind) in [
        (&config_path, "audit-configuration"),
        (&provenance_path, "provenance"),
        (&database, "compile-database"),
        (&build.join("lib/libevent_core.a"), "static-library"),
        (&build.join("configure.log"), "log"),
        (&build.join("native-build.log"), "log"),
    ] {
        response.push_artifact(&output, path, kind)?;
    }
    if let Some(path) = &regenerated {
        response.push_artifact(&output, path, "regenerated-profile")?;
        response.push_diagnostic(
            Severity::Warning,
            "profile-regenerated",
            format!(
                "review {} and copy it to {} (generated digests are committed only alongside \
                 the record that validated them)",
                path.display(),
                profile_path.display()
            ),
        );
    }
    response.push_diagnostic(
        Severity::Info,
        "audit-configuration",
        config_path.display().to_string(),
    );
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn flags_parse_once_each() {
        let options = parse(&[
            "--clang".into(),
            "/c".into(),
            "--write-profile".into(),
            "--repository".into(),
            "r".into(),
        ])
        .unwrap();
        assert_eq!(options.clang.as_deref(), Some("/c"));
        assert!(options.write_profile);
        assert_eq!(options.repository.as_deref(), Some("r"));
        assert!(parse(&["--clang".into()]).is_err());
        assert!(parse(&["--cmake".into(), "a".into(), "--cmake".into(), "b".into()]).is_err());
        assert!(parse(&["--output".into(), "x".into()]).is_err());
    }

    #[test]
    fn the_selection_reads_the_target_prefix_and_root_level_c_files() {
        let checkout = Path::new("/src/libevent");
        let entries = vec![
            json!({"file": "/src/libevent/buffer.c", "command": "cc -o CMakeFiles/event_core_static.dir/buffer.c.o -c buffer.c"}),
            json!({"file": "/src/libevent/event.c", "arguments": ["cc", "-o", "CMakeFiles/event_core_static.dir/event.c.o"]}),
            json!({"file": "/src/libevent/http.c", "command": "cc -o CMakeFiles/event_extra_static.dir/http.c.o"}),
            json!({"file": "/src/libevent/sub/x.c", "output": "CMakeFiles/event_core_static.dir/sub/x.c.o", "command": "cc"}),
            json!({"file": "/src/libevent/buffer.c", "output": "CMakeFiles/event_core_static.dir/buffer.c.o", "command": "cc"}),
            json!({"file": "/src/libevent/.c", "output": "CMakeFiles/event_core_static.dir/y.o", "command": "cc"}),
        ];
        assert_eq!(
            selected_units(&entries, checkout, Path::new("/")).unwrap(),
            vec!["buffer.c".to_owned(), "event.c".to_owned()]
        );
        let trailing = vec![json!({"file": "/src/libevent/a.c", "command": "cc -o"})];
        assert!(selected_units(&trailing, checkout, Path::new("/")).is_err());
    }

    #[test]
    fn a_protected_output_is_refused() {
        let root = Path::new("/repo");
        assert!(check_output(root, Path::new("/repo/build/cycle19/x")).is_err());
        assert!(check_output(root, Path::new("/repo/third_party/libevent/b")).is_err());
        assert!(check_output(root, Path::new("/repo/build/cycle19")).is_err());
        assert!(check_output(root, Path::new("/repo/build/cycle190")).is_ok());
        assert!(check_output(root, Path::new("/elsewhere/out")).is_ok());
    }

    #[test]
    fn only_prepare_is_answered() {
        let output = Path::new("/nonexistent-output");
        let request = Request::new("libevent", vec!["build".into()], output, output);
        assert!(answer(&request).unwrap_err().starts_with("usage:"));
    }
}
