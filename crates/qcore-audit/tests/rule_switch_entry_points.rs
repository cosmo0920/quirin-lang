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

//! Card RULE-SWITCH-ENTRY-POINTS (discharges
//! `GATEWAY-RULE-API-OTHER-ENTRY-POINTS`): every entry point that links the
//! C gateway, and so may consult its route tables or rule registry, either
//! records `QCORE_GATEWAY_RULES_OFF` in what it publishes or refuses a
//! non-empty list before it starts.
//!
//! The census reads both workspaces' manifests (tracked files only, rule 29):
//! an entry point is a binary or an example of a workspace member, and it
//! links the gateway when `qcore-c-gateway` is in its dependency closure
//! (normal dependencies for a binary; an example also gets its own package's
//! dev-dependencies). Each such `main` must admit the switch:
//! `gateway_rules::exit_if_unrecorded(...)` or `refuse_unrecorded(...)` as its
//! first statement, or `rule_switch_entry::admit(...)` before it dispatches
//! (`quirin-audit`). A new entry point without one fails
//! [`every_gateway_entry_point_admits_the_switch`]; the checker itself is
//! held to mutant mains first.
//!
//! The process tests run the two binaries of this package: every built-in
//! that records nothing refuses a non-empty list, a plugin subcommand (whose
//! environment the host clears) refuses it, and a refused list is refused
//! everywhere. The recording routes' tests are beside their fixtures
//! (`current_cli.rs`, `no_build_cli.rs`, `configured_cli.rs`).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

#[path = "../../../tests/support/tracked_files.rs"]
mod tracked_files;

const GATEWAY: &str = "qcore-c-gateway";

/// The admission calls a `main` may make.
const FIRST_STATEMENT_HOOKS: [&str; 2] = [
    "gateway_rules::exit_if_unrecorded(",
    "gateway_rules::refuse_unrecorded(",
];
const DISPATCH_HOOK: &str = "rule_switch_entry::admit(";

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the repository root")
}

// ----- source reading -------------------------------------------------------

/// `text` with comments removed and every string and character literal
/// emptied, so neither a comment nor a literal can carry an admission and
/// braces in literals do not count.
fn code_only(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut index = 0;
    while index < chars.len() {
        let c = chars[index];
        let next = chars.get(index + 1).copied();
        if c == '/' && next == Some('/') {
            while index < chars.len() && chars[index] != '\n' {
                index += 1;
            }
        } else if c == '/' && next == Some('*') {
            let mut depth = 0;
            while index < chars.len() {
                if chars[index] == '/' && chars.get(index + 1) == Some(&'*') {
                    depth += 1;
                    index += 2;
                } else if chars[index] == '*' && chars.get(index + 1) == Some(&'/') {
                    depth -= 1;
                    index += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    index += 1;
                }
            }
            out.push(' ');
        } else if c == 'r'
            && (next == Some('"') || next == Some('#'))
            && !chars
                .get(index.wrapping_sub(1))
                .is_some_and(|before| before.is_alphanumeric() || *before == '_')
        {
            // A raw string: r"..." or r#"..."#.
            let mut hashes = 0;
            let mut cursor = index + 1;
            while chars.get(cursor) == Some(&'#') {
                hashes += 1;
                cursor += 1;
            }
            if chars.get(cursor) != Some(&'"') {
                out.push(c);
                index += 1;
                continue;
            }
            cursor += 1;
            loop {
                if cursor >= chars.len() {
                    break;
                }
                if chars[cursor] == '"'
                    && (0..hashes).all(|offset| chars.get(cursor + 1 + offset) == Some(&'#'))
                {
                    cursor += 1 + hashes;
                    break;
                }
                cursor += 1;
            }
            out.push_str("\"\"");
            index = cursor;
        } else if c == '"' {
            index += 1;
            while index < chars.len() && chars[index] != '"' {
                if chars[index] == '\\' {
                    index += 1;
                }
                index += 1;
            }
            index += 1;
            out.push_str("\"\"");
        } else if c == '\'' {
            // A character literal ('x', '\n', '\''), or a lifetime, kept.
            if next == Some('\\') {
                let mut cursor = index + 2;
                while cursor < chars.len() && chars[cursor] != '\'' {
                    cursor += 1;
                }
                index = cursor + 1;
                out.push_str("' '");
            } else if chars.get(index + 2) == Some(&'\'') {
                index += 3;
                out.push_str("' '");
            } else {
                out.push(c);
                index += 1;
            }
        } else {
            out.push(c);
            index += 1;
        }
    }
    out
}

/// The body of `fn main` in `code` (already [`code_only`]), without its
/// braces.
fn main_body(code: &str) -> Option<&str> {
    let start = code
        .match_indices("fn main(")
        .find(|(at, _)| {
            code[..*at]
                .chars()
                .next_back()
                .is_none_or(|before| !before.is_alphanumeric() && before != '_')
        })?
        .0;
    let open = start + code[start..].find('{')?;
    let mut depth = 0usize;
    for (offset, c) in code[open..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&code[open + 1..open + offset]);
                }
            }
            _ => {}
        }
    }
    None
}

/// Whether the entry point's source admits the switch, or why not.
fn admits_the_switch(source: &str) -> Result<(), String> {
    let code = code_only(source);
    let body = main_body(&code).ok_or("no `fn main` body")?;
    let first = body.trim_start();
    if FIRST_STATEMENT_HOOKS.iter().any(|hook| {
        first
            .split(';')
            .next()
            .is_some_and(|statement| statement.contains(hook))
    }) {
        return Ok(());
    }
    if let Some(at) = body.find(DISPATCH_HOOK) {
        let dispatch = body.find("return qcore_audit::").unwrap_or(usize::MAX);
        return if at < dispatch {
            Ok(())
        } else {
            Err("`rule_switch_entry::admit` comes after a dispatch".into())
        };
    }
    Err(format!(
        "`main` neither starts with one of {FIRST_STATEMENT_HOOKS:?} nor calls `{DISPATCH_HOOK}` \
         before dispatching"
    ))
}

#[test]
fn the_checker_catches_each_missing_or_misplaced_admission() {
    let hook = "qcore_c_gateway::gateway_rules::exit_if_unrecorded(\"x\");";
    let admitted = [
        format!("fn main() -> ExitCode {{\n    {hook}\n    run()\n}}\n"),
        "fn main() { if qcore_c_gateway::gateway_rules::refuse_unrecorded(\"x\").is_err() { return; } run(); }".to_owned(),
        "fn main() { let p = 1; if let Err(e) = qcore_audit::rule_switch_entry::admit(&p, &r) { return e; }\n\
         if p == \"x\" { return qcore_audit::pinned_cli::main(a); } }"
            .to_owned(),
        format!("/* fn main() {{}} */\nfn main() {{ {hook} let s = \"}}\"; let c = '{{'; }}"),
    ];
    for source in &admitted {
        assert_eq!(admits_the_switch(source), Ok(()), "{source}");
    }
    let refused = [
        "fn main() { run(); }".to_owned(),
        format!("fn main() {{\n    // {hook}\n    run();\n}}"),
        format!("fn main() {{\n    /* {hook} */\n    run();\n}}"),
        format!("fn main() {{ run(); {hook} }}"),
        format!("fn helper() {{ {hook} }}\nfn main() {{ helper(); }}"),
        format!("fn main() {{ let text = r#\"{hook}\"#; run(); }}"),
        format!(
            "fn main() {{ let text = \"{}\"; run(); }}",
            hook.replace('"', "\\\"")
        ),
        "fn main() { if p == \"x\" { return qcore_audit::pinned_cli::main(a); }\n\
         let _ = qcore_audit::rule_switch_entry::admit(&p, &r); }"
            .to_owned(),
        format!("fn not_main() {{ {hook} }}"),
    ];
    for source in &refused {
        assert!(admits_the_switch(source).is_err(), "{source}");
    }
}

// ----- the census -------------------------------------------------------------

struct Package {
    directory: PathBuf,
    manifest: toml::Table,
    normal: BTreeSet<String>,
    dev: BTreeSet<String>,
}

fn dependency_names(table: Option<&toml::Value>) -> BTreeSet<String> {
    let Some(table) = table.and_then(toml::Value::as_table) else {
        return BTreeSet::new();
    };
    table
        .iter()
        .map(|(key, value)| {
            value
                .get("package")
                .and_then(toml::Value::as_str)
                .unwrap_or(key)
                .to_owned()
        })
        .collect()
}

fn read_toml(path: &Path) -> toml::Table {
    let text =
        std::fs::read_to_string(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    text.parse::<toml::Table>()
        .unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// Every member of the root and provider workspaces, by package name.
fn packages(repository: &Path) -> BTreeMap<String, Package> {
    let tracked: BTreeSet<PathBuf> = tracked_files::tracked_files(repository)
        .into_iter()
        .collect();
    let mut packages = BTreeMap::new();
    for workspace in [repository.to_path_buf(), repository.join("providers")] {
        let root = read_toml(&workspace.join("Cargo.toml"));
        let members = root["workspace"]["members"]
            .as_array()
            .expect("workspace members");
        for member in members {
            let directory = workspace.join(member.as_str().expect("a member path"));
            let manifest_path = directory.join("Cargo.toml");
            assert!(
                tracked.contains(&manifest_path),
                "{} is not tracked",
                manifest_path.display()
            );
            let manifest = read_toml(&manifest_path);
            let name = manifest["package"]["name"]
                .as_str()
                .expect("a package name")
                .to_owned();
            let mut normal = dependency_names(manifest.get("dependencies"));
            let mut dev = dependency_names(manifest.get("dev-dependencies"));
            if let Some(targets) = manifest.get("target").and_then(toml::Value::as_table) {
                for target in targets.values() {
                    normal.extend(dependency_names(target.get("dependencies")));
                    dev.extend(dependency_names(target.get("dev-dependencies")));
                }
            }
            packages.insert(
                name,
                Package {
                    directory,
                    manifest,
                    normal,
                    dev,
                },
            );
        }
    }
    packages
}

/// The packages whose normal dependency closure contains the gateway.
fn linking(packages: &BTreeMap<String, Package>) -> BTreeSet<String> {
    let mut linking = BTreeSet::from([GATEWAY.to_owned()]);
    loop {
        let before = linking.len();
        for (name, package) in packages {
            if package
                .normal
                .iter()
                .any(|dependency| linking.contains(dependency))
            {
                linking.insert(name.clone());
            }
        }
        if linking.len() == before {
            return linking;
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct EntryPoint {
    package: String,
    kind: &'static str,
    source: PathBuf,
}

/// Every binary and example of `package`: declared targets, and Cargo's
/// automatic ones unless the manifest turns them off.
fn entry_points(name: &str, package: &Package, tracked: &BTreeSet<PathBuf>) -> Vec<EntryPoint> {
    let mut found = BTreeSet::new();
    for (kind, key, auto, directory) in [
        ("bin", "bin", "autobins", "src/bin"),
        ("example", "example", "autoexamples", "examples"),
    ] {
        if let Some(targets) = package.manifest.get(key).and_then(toml::Value::as_array) {
            for target in targets {
                let path = match target.get("path").and_then(toml::Value::as_str) {
                    Some(path) => package.directory.join(path),
                    None if kind == "bin"
                        && target.get("name").and_then(toml::Value::as_str) == Some(name) =>
                    {
                        package.directory.join("src/main.rs")
                    }
                    None => package.directory.join(directory).join(format!(
                        "{}.rs",
                        target["name"].as_str().expect("a target name")
                    )),
                };
                found.insert((kind, path));
            }
        }
        let automatic = package
            .manifest
            .get("package")
            .and_then(|section| section.get(auto))
            .and_then(toml::Value::as_bool)
            .unwrap_or(true);
        if automatic {
            let base = package.directory.join(directory);
            for path in tracked {
                let Ok(relative) = path.strip_prefix(&base) else {
                    continue;
                };
                let parts: Vec<_> = relative.components().collect();
                let is_target = match parts.as_slice() {
                    [file] => Path::new(file).extension().is_some_and(|ext| ext == "rs"),
                    [_, file] => Path::new(file) == Path::new("main.rs"),
                    _ => false,
                };
                if is_target {
                    found.insert((kind, path.clone()));
                }
            }
            if kind == "bin" {
                let main = package.directory.join("src/main.rs");
                if tracked.contains(&main) {
                    found.insert((kind, main));
                }
            }
        }
    }
    found
        .into_iter()
        .map(|(kind, source)| EntryPoint {
            package: name.to_owned(),
            kind,
            source,
        })
        .collect()
}

/// The census: every entry point of both workspaces, and whether it links
/// the gateway.
fn census(repository: &Path) -> Vec<(EntryPoint, bool)> {
    let packages = packages(repository);
    let linking = linking(&packages);
    let tracked: BTreeSet<PathBuf> = tracked_files::tracked_files(repository)
        .into_iter()
        .collect();
    let mut census = Vec::new();
    for (name, package) in &packages {
        for entry in entry_points(name, package, &tracked) {
            let links = linking.contains(name)
                || (entry.kind == "example"
                    && package
                        .dev
                        .iter()
                        .any(|dependency| linking.contains(dependency)));
            census.push((entry, links));
        }
    }
    census.sort();
    census
}

#[test]
fn every_gateway_entry_point_admits_the_switch() {
    let repository = repository();
    let census = census(&repository);
    // Spelled with `/` on every host, as the expected paths below are.
    let relative = |entry: &EntryPoint| {
        entry
            .source
            .strip_prefix(&repository)
            .unwrap_or(&entry.source)
            .components()
            .map(|component| component.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/")
    };
    let linked: BTreeSet<String> = census
        .iter()
        .filter(|(_, links)| *links)
        .map(|(entry, _)| relative(entry))
        .collect();
    let unlinked: BTreeSet<String> = census
        .iter()
        .filter(|(_, links)| !*links)
        .map(|(entry, _)| relative(entry))
        .collect();
    // The census sees what the card enumerated (a floor, so it cannot pass by
    // finding nothing): the three gateway binaries, examples of every
    // linking crate, and the binaries that do not link the gateway.
    for expected in [
        "crates/qcore-audit/src/main.rs",
        "crates/qcore-audit/src/bin/quirin-dump-checkpoint.rs",
        "crates/qcorec/src/main.rs",
        "crates/qcore-audit/examples/component_check.rs",
        "crates/qcore-c-gateway/examples/member_path_probe.rs",
        "crates/quirin-source-analysis/examples/loop_owner_census.rs",
        "compiler/examples/reevaluation_probe.rs",
    ] {
        assert!(linked.contains(expected), "{expected} not in {linked:#?}");
    }
    for expected in [
        "crates/quirin-audit-plugin-measure/src/main.rs",
        "crates/quirin-audit-plugin-libevent/src/main.rs",
        "crates/quirin-audit-plugin/src/bin/example.rs",
        "crates/qcore-contract/examples/contract_inventory.rs",
        "providers/qcore-provider-libevent/src/main.rs",
        // Its package dev-depends on the gateway; a binary does not get that.
        "providers/conformance/src/bin/sample_boolean_provider.rs",
    ] {
        assert!(
            unlinked.contains(expected),
            "{expected} not in {unlinked:#?}"
        );
    }
    assert!(linked.len() >= 40, "{linked:#?}");
    let missing: Vec<String> = census
        .iter()
        .filter(|(_, links)| *links)
        .filter_map(|(entry, _)| {
            let source = std::fs::read_to_string(&entry.source)
                .unwrap_or_else(|error| panic!("{}: {error}", entry.source.display()));
            admits_the_switch(&source).err().map(|reason| {
                format!(
                    "{} ({} of {}): {reason}",
                    relative(entry),
                    entry.kind,
                    entry.package
                )
            })
        })
        .collect();
    assert!(
        missing.is_empty(),
        "entry points that link the gateway must record QCORE_GATEWAY_RULES_OFF or refuse \
         it (card RULE-SWITCH-ENTRY-POINTS):\n{}",
        missing.join("\n")
    );
}

// ----- the binaries -------------------------------------------------------------

fn run(binary: &str, arguments: &[&str], list: Option<&str>) -> Output {
    let mut command = Command::new(binary);
    command.current_dir(std::env::temp_dir()).args(arguments);
    match list {
        Some(list) => command.env("QCORE_GATEWAY_RULES_OFF", list),
        None => command.env_remove("QCORE_GATEWAY_RULES_OFF"),
    };
    command.output().expect("the binary runs")
}

fn refused_with(output: &Output, expected: &str) {
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(expected),
        "expected `{expected}` in:\n{stderr}"
    );
}

#[test]
fn every_unrecorded_builtin_refuses_a_switch_list_and_a_recording_one_starts() {
    let host = env!("CARGO_BIN_EXE_quirin-audit");
    // Every run names an option no built-in takes, so a command line admitted
    // with nothing off stops at its own argument check and does no work.
    let bogus = "--no-such-option-rule-switch-entry-points";
    for name in qcore_audit::rule_switch_entry::UNRECORDED {
        let output = run(host, &[name, bogus], Some("35"));
        refused_with(
            &output,
            &format!(
                "quirin-audit {name} does not record the gateway rule set it runs under, so it \
                 refuses QCORE_GATEWAY_RULES_OFF=35"
            ),
        );
        // A refused list is refused here as on every command line.
        refused_with(
            &run(host, &[name, bogus], Some("35,no-such-rule")),
            "no-such-rule",
        );
        // With nothing off the command line runs as before: whatever it
        // answers, it is not the switch refusal.
        for list in [None, Some(""), Some(" ")] {
            let output = run(host, &[name, bogus], list);
            assert!(
                !String::from_utf8_lossy(&output.stderr).contains("does not record"),
                "{name} with {list:?}: {output:?}"
            );
        }
    }
    refused_with(
        &run(host, &["prepare", "--output", "/nonexistent"], Some("35")),
        "quirin-audit prepare without --run does not record",
    );
    for plugin in [
        vec!["measure"],
        vec!["--list-plugins"],
        vec!["--plugin-path", "/nonexistent", "libevent"],
    ] {
        refused_with(
            &run(host, &plugin, Some("35")),
            "a quirin-audit subcommand plugin does not record",
        );
    }
    // A recording built-in is admitted and fails on its own arguments.
    for name in ["current", "contracts", "pinned"] {
        let output = run(host, &[name, bogus], Some("35"));
        assert!(!output.status.success(), "{name}: {output:?}");
        assert!(
            !String::from_utf8_lossy(&output.stderr).contains("does not record"),
            "{name}: {output:?}"
        );
    }
    // The pinned replay without a diagnostic report records nothing.
    refused_with(
        &run(host, &["pinned", "/nonexistent"], Some("35")),
        "quirin-audit pinned without a diagnostic report does not record",
    );
}

#[test]
fn the_checkpoint_dump_refuses_a_switch_list() {
    let dump = env!("CARGO_BIN_EXE_quirin-dump-checkpoint");
    refused_with(
        &run(dump, &["--help"], Some("35")),
        "quirin-dump-checkpoint does not record the gateway rule set",
    );
    refused_with(
        &run(dump, &["--help"], Some("nope!")),
        "QCORE_GATEWAY_RULES_OFF",
    );
    for list in [None, Some("")] {
        let output = run(dump, &["--help"], list);
        assert!(output.status.success(), "{list:?}: {output:?}");
    }
}
