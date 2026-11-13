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

//! Card **LINK-STANDING-SELECTION-TWO-CONFIGURATION-KEYS** (rule 40,
//! condition 13), end to end: `quirin-audit current` with a selection that
//! keeps every configuration of `src/home.c`, as the Fluent Bit system and
//! tail-pause selections do. The database compiles `home.c` twice, static
//! (`home.o`, linked) and position-independent (`home.pic.o`), so both
//! configurations share the route key `sample/src/home.c`.
//!
//! When the link-resolution artifact records the second configuration and no
//! link input is attributed to it, the link used nothing it compiles: the key
//! is left to the linked configuration, the census leaves the other one out,
//! and the caller holds `drain`'s summary. When a link input is attributed to
//! it (the link may have used it), or the artifact does not record it, the
//! home stays refused and the call uncontracted.
#![cfg(unix)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

use serde_json::{Value, json};

static NEXT: AtomicUsize = AtomicUsize::new(0);

const CONTRACTS: &str = "module contract;\n\
extern fn q_alloc() -> ptr<mut, u8>\n\
contract { requires resource_contract_v1; requires resource_revision(flexible); \
ensures normal resource_return_owned(nonnull); };\n\
extern fn q_free(p: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; requires resource_revision(flexible); \
ensures normal resource_released(p, nonnull); ensures normal resource_return_none(); };\n";

const PRELUDE: &str = "void *q_alloc(void); void q_free(void *);\n\
struct queue { void *slot; };\n\
int drain(struct queue *q);\n";

const HOME: &str = "int drain(struct queue *q) { return q->slot != 0; }\n";

const CALLER: &str = "int subject(struct queue *q) { return drain(q); }\n";

const UNCONTRACTED: &str = "call to uncontracted function `drain`";

const AMBIGUOUS: &str =
    "home unit key names another route configuration not shown outside the link";

fn sha256(bytes: &[u8]) -> String {
    qcore_schemas::sha256_hex(bytes)
}

fn git(directory: &Path, args: &[&str]) {
    let output = Command::new("git")
        .current_dir(directory)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// How the artifact records `home.c`'s position-independent configuration.
#[derive(Clone, Copy)]
enum Second {
    /// Recorded, `linked: false`, no link input attributed to it.
    Outside,
    /// Recorded, `linked: false`, but a link input's bytes are its object:
    /// the link may have used it.
    Attributed,
    /// Not recorded by the artifact at all.
    Unrecorded,
}

struct Fixture {
    root: PathBuf,
    database: String,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

impl Fixture {
    fn new(driver: &Path) -> Self {
        let root = std::env::temp_dir().join(format!(
            "qcore-link-standing-two-configurations-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let checkout = root.join("checkout");
        fs::create_dir_all(checkout.join("src")).unwrap();
        for (path, body) in [("src/home.c", HOME), ("src/caller.c", CALLER)] {
            fs::write(checkout.join(path), format!("{PRELUDE}{body}")).unwrap();
        }
        git(&checkout, &["init", "--quiet"]);
        git(&checkout, &["add", "src"]);
        git(
            &checkout,
            &[
                "-c",
                "user.name=Audit Fixture",
                "-c",
                "user.email=audit@example.invalid",
                "commit",
                "--quiet",
                "-m",
                "Two-configuration fixture",
            ],
        );
        let commit = String::from_utf8(
            Command::new("git")
                .current_dir(&checkout)
                .args(["rev-parse", "HEAD"])
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap()
        .trim()
        .to_owned();
        fs::write(root.join("contracts.qv"), CONTRACTS).unwrap();
        fs::write(
            root.join("contracts.json"),
            json!({"schema_version": 1, "modules": ["contracts.qv"]}).to_string(),
        )
        .unwrap();
        let profile = root.join("profiles/sample");
        fs::create_dir_all(&profile).unwrap();
        let library = json!({
            "pin":"sample", "source_roots":[{"root":"src","selection":"top_level"}],
            "audited_files": [], "source_witnesses":[], "permitted_mechanisms":[],
            "operation_summaries":{}, "api_dispositions":{}, "accepted_builds":[],
            "required_evidence":[], "typed_import":null, "callback_summaries":{},
            "boolean_field_models":[], "provider":null,
        });
        fs::write(
            profile.join("profile.json"),
            json!({
                "schema_version":1, "source_set":"sample",
                "pins":{"sample":{"git_commit":commit,"clean_paths":[],
                    "description":"two-configuration fixture"}},
                "libraries":{"sample":library},
                "resource_contract_manifest": "../../contracts.json",
                "provider_manifests":[],
            })
            .to_string(),
        )
        .unwrap();
        fs::write(
            profile.join("audit.json"),
            json!({"schema_version":1,"libraries":[{"library":"sample","label":null}]}).to_string(),
        )
        .unwrap();
        fs::write(
            root.join("selection.json"),
            json!({
                "schema_version": 1, "name": "two-configurations",
                "groups": [{"library": "sample", "files": ["src/home.c", "src/caller.c"]}],
            })
            .to_string(),
        )
        .unwrap();
        let home = checkout.join("src/home.c");
        let caller = checkout.join("src/caller.c");
        let record = |file: &Path, extra: &[&str], object: &str| {
            let mut arguments = vec![json!(driver), json!("-std=gnu17")];
            arguments.extend(extra.iter().map(|flag| json!(flag)));
            arguments.extend([json!("-c"), json!(file), json!("-o"), json!(object)]);
            json!({"directory": checkout, "file": file, "arguments": arguments})
        };
        let database = Value::Array(vec![
            record(&home, &[], "home.o"),
            record(&home, &["-fPIC", "-DQ_SHARED"], "home.pic.o"),
            record(&caller, &[], "caller.o"),
        ])
        .to_string();
        fs::write(root.join("compile_commands.json"), &database).unwrap();
        for name in [
            "fluent-bit",
            "link.txt",
            "fluent-bit.map",
            "clang",
            "ld",
            "home.o",
            "home.pic.o",
            "caller.o",
        ] {
            fs::write(root.join(name), name).unwrap();
        }
        Self { root, database }
    }

    /// The artifact: `drain` is one strong default-visibility definition from
    /// the static `home.c` configuration's linked object.
    fn artifact(&self, second: Second) -> PathBuf {
        let bound = |name: &str| {
            let path = self.root.join(name);
            (path.clone(), sha256(&fs::read(&path).unwrap()))
        };
        let entries = qcore_audit::compile_db::compile_entries(&self.database).unwrap();
        assert_eq!(entries.len(), 3);
        // Database order: 0 home.c static, 1 home.c -fPIC, 2 caller.c.
        let recorded: Vec<usize> = match second {
            Second::Unrecorded => vec![0, 2],
            Second::Outside | Second::Attributed => vec![0, 1, 2],
        };
        let configurations = recorded
            .iter()
            .enumerate()
            .map(|(id, &entry)| {
                let entry = &entries[entry];
                let linked = !entry.command.contains("-fPIC");
                json!({
                    "id": id, "file": entry.file, "directory": entry.directory,
                    "command_sha256": sha256(entry.command.as_bytes()),
                    "object": Value::Null, "linked": linked,
                    "reproduction": if linked { json!("equal") } else { Value::Null },
                    "route_unit": Value::Null,
                })
            })
            .collect::<Vec<_>>();
        let caller_id = recorded.len() - 1;
        let mut inputs = vec![
            json!({"id": 0, "label": "home.o", "sha256": bound("home.o").1, "configurations": [0]}),
            json!({"id": 1, "label": "caller.o", "sha256": bound("caller.o").1,
                "configurations": [caller_id]}),
        ];
        if let Second::Attributed = second {
            inputs.push(
                json!({"id": 2, "label": "home.pic.o", "sha256": bound("home.pic.o").1,
                "configurations": [1]}),
            );
        }
        let (executable, executable_sha256) = bound("fluent-bit");
        let (link_txt, link_txt_sha256) = bound("link.txt");
        let (map, map_sha256) = bound("fluent-bit.map");
        let (driver, driver_sha256) = bound("clang");
        let (linker, linker_sha256) = bound("ld");
        let artifact = json!({
            "schema": "quirin-link-resolution/1",
            "executable": {"path": executable, "sha256": executable_sha256,
                "image": {"is_executable": true}},
            "link": {"link_txt": link_txt, "link_txt_sha256": link_txt_sha256,
                "map": map, "map_sha256": map_sha256, "directory": self.root},
            "toolchain": {
                "driver": {"path": driver, "resolved": driver, "sha256": driver_sha256},
                "linker": {"path": linker, "resolved": linker, "sha256": linker_sha256},
            },
            "compile_database": {"path": "compile_commands.json",
                "sha256": sha256(self.database.as_bytes())},
            "inputs": inputs,
            "configurations": configurations,
            "symbols": [{
                "name": "drain", "kind": "function",
                "definition": {"file": "home.o", "in": "executable", "input": 0,
                    "configuration": 0, "binding": "global", "visibility": "default",
                    "type": "function"},
                "definitions": [{"input": 0, "binding": "global"}],
                "referenced_by": [1],
                "executable": {"entries": [], "in_definer_text": true, "exported": false,
                    "dynamic_relocations": 0},
                "home_unit": Value::Null, "decision": "refused",
                "reason": "home unit key names more than one route configuration",
                "detail": "",
            }],
        });
        let path = self.root.join(format!(
            "link-resolution-{}.json",
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::write(&path, artifact.to_string()).unwrap();
        path
    }

    fn run(&self, name: &str, artifact: Option<&Path>) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_quirin-audit"));
        command.current_dir(&self.root);
        if let Some(artifact) = artifact {
            command.env("QCORE_CROSS_UNIT_LINK_RESOLUTION", artifact);
        }
        command
            .args([
                "current",
                "--checkout",
                "checkout",
                "--profile-directory",
                "profiles/sample",
                "--compile-database",
                "compile_commands.json",
                "--selection",
                "selection.json",
                "--report",
                &format!("{name}/report.json"),
                "--jobs",
                "1",
            ])
            .output()
            .unwrap()
    }

    fn report(&self, name: &str) -> Value {
        serde_json::from_str(&fs::read_to_string(self.root.join(name).join("report.json")).unwrap())
            .unwrap()
    }

    /// Every row of `function`: its status and diagnostics' texts.
    fn rows(&self, name: &str, function: &str) -> Vec<(String, String)> {
        let report = self.report(name);
        let rows = report["units"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|unit| unit["functions"].as_array().cloned().unwrap_or_default())
            .filter(|row| row["function"] == function)
            .map(|row| {
                (
                    row["status"].as_str().unwrap().to_owned(),
                    row["diagnostics"].to_string(),
                )
            })
            .collect::<Vec<_>>();
        assert!(!rows.is_empty(), "no {function} row: {report}");
        rows
    }
}

fn succeeded(output: &Output) -> String {
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn clang() -> Option<PathBuf> {
    qcore_c_gateway::clang_command()
        .map(|command| PathBuf::from(command.get_program()))
        .ok()
}

#[test]
fn an_unlinked_second_configuration_leaves_the_home_to_the_linked_one() {
    let Some(clang) = clang() else { return };
    let fixture = Fixture::new(&clang);

    // Without an artifact the call is uncontracted; both home rows exist.
    succeeded(&fixture.run("plain", None));
    assert_eq!(fixture.rows("plain", "drain").len(), 2);
    let subject = fixture.rows("plain", "subject");
    assert_eq!(subject.len(), 1);
    assert_eq!(subject[0].0, "Unknown");
    assert!(subject[0].1.contains(UNCONTRACTED), "{}", subject[0].1);

    // The position-independent configuration is shown outside the link:
    // the key is the static one's, the census leaves the other out, and the
    // caller holds the summary.
    let artifact = fixture.artifact(Second::Outside);
    let output = fixture.run("outside", Some(&artifact));
    let stdout = succeeded(&output);
    assert!(
        stdout.contains("derived (rule 40)") && stdout.contains(": 1 home(s); refused {}"),
        "{stdout}"
    );
    // One export: the static configuration's; the other exports nothing.
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(": 1 home export(s)"), "{stderr}");
    let subject = fixture.rows("outside", "subject");
    assert_eq!(subject[0].0, "Safe", "{}\n{stdout}", subject[0].1);
    // The contract sidecar's assumption names the configuration left out.
    let sidecar: Value = serde_json::from_str(
        &fs::read_to_string(fixture.root.join("outside/report.contracts.json")).unwrap(),
    )
    .unwrap();
    let section = sidecar.to_string();
    let entries = qcore_audit::compile_db::compile_entries(&fixture.database).unwrap();
    assert!(
        section.contains("\"outside_the_link\":[{")
            && section.contains(&sha256(entries[1].command.as_bytes()))
            && !section.contains(&sha256(entries[0].command.as_bytes())),
        "{section}"
    );
}

#[test]
fn a_second_configuration_the_link_may_have_used_keeps_the_refusal() {
    let Some(clang) = clang() else { return };
    let fixture = Fixture::new(&clang);
    for (name, second) in [
        ("attributed", Second::Attributed),
        ("unrecorded", Second::Unrecorded),
    ] {
        let artifact = fixture.artifact(second);
        let stdout = succeeded(&fixture.run(name, Some(&artifact)));
        assert!(
            stdout.contains(": 0 home(s)") && stdout.contains(AMBIGUOUS),
            "{name}: {stdout}"
        );
        let subject = fixture.rows(name, "subject");
        assert_eq!(subject[0].0, "Unknown", "{name}");
        assert!(
            subject[0].1.contains(UNCONTRACTED),
            "{name}: {}",
            subject[0].1
        );
    }
}
