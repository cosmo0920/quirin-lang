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

//! Card **LERI-CROSS-UNIT-ELEMENTS-RELEASED**: a released-element summary
//! (gateway rule 43) crosses units through the link-standing route
//! (`quirin-audit current` with a rule 40 link-resolution artifact), end to
//! end. The home unit's `drain` hands every element of its formal's list to
//! an element destroyer and publishes `ElementsReleased` at that formal;
//! the caller unit's `subject` calls it.
//!
//! Without the artifact the call is uncontracted. With it, `subject` is
//! `Safe`, and `published`, which linked an object into the drained list and
//! unlinks it after the call, is refused on that object (the clause made it
//! forget its publication) and accused of nothing. With rule 43 switched off
//! the home publishes no summary, so nothing crosses.
#![cfg(unix)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

use serde_json::{Value, json};

static NEXT: AtomicUsize = AtomicUsize::new(0);

const LIST: &str = "module contract;\n\
extern fn q_alloc() -> ptr<mut, u8>\n\
contract { requires resource_contract_v1; requires resource_revision(flexible); \
ensures normal resource_return_owned(nonnull); };\n\
extern fn q_free(p: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; requires resource_revision(flexible); \
ensures normal resource_released(p, nonnull); ensures normal resource_return_none(); };\n\
extern fn mk_list_add(entry: ptr<mut, u8>, head: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; requires resource_revision(flexible); \
ensures normal resource_borrowed(entry); ensures normal resource_borrowed(head); \
ensures normal resource_published(entry, head); ensures normal resource_return_none(); };\n\
extern fn mk_list_del(entry: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; requires resource_revision(flexible); \
ensures normal resource_borrowed(entry); ensures normal resource_unpublished(entry); \
ensures normal resource_return_none(); };\n\
extern fn item_destroy(e: ptr<mut, u8>) -> i32\n\
contract { requires resource_contract_v1; requires resource_revision(flexible); \
ensures normal resource_unpublished(e, _head); ensures normal resource_released(e, nonnull); \
ensures normal resource_return_none(); };\n";

const PRELUDE: &str = "#include <stddef.h>\n\
#define container_of(ptr, type, member) ({ \\\n\
      const typeof( ((type *)0)->member ) *__mptr = (ptr); \\\n\
      (type *)( (char *)__mptr - offsetof(type,member) );})\n\
struct mk_list { struct mk_list *prev, *next; };\n\
#define mk_list_foreach_safe(curr, n, head) \\\n\
    for (curr = (head)->next, n = curr->next; curr != (head); curr = n, n = curr->next)\n\
#define mk_list_entry( ptr, type, member ) container_of( ptr, type, member )\n\
void mk_list_add(struct mk_list *entry, struct mk_list *head);\n\
void mk_list_del(struct mk_list *entry);\n\
void *q_alloc(void); void q_free(void *);\n\
struct item { int close; struct mk_list _head; };\n\
struct queue { struct mk_list items; };\n\
int item_destroy(struct item *e);\n\
int drain(struct queue *q);\n";

const HOME: &str = "int drain(struct queue *q) { int c = 0; struct mk_list *it, *tmp;\n\
     struct item *e; mk_list_foreach_safe(it, tmp, &q->items) {\n\
     e = mk_list_entry(it, struct item, _head); item_destroy(e); c++; } return c; }\n";

const CALLER: &str = "void subject(struct queue *q) { drain(q); }\n\
void published(struct queue *q) { struct item *x = q_alloc(); if (!x) return;\n\
     mk_list_add(&x->_head, &q->items); drain(q); mk_list_del(&x->_head); q_free(x); }\n";

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
    fn new(driver: &Path, home: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "qcore-link-standing-elements-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let checkout = root.join("checkout");
        fs::create_dir_all(checkout.join("src")).unwrap();
        let units = [("src/home.c", home), ("src/caller.c", CALLER)];
        for (path, body) in units {
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
                "Released-element fixture",
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
        fs::write(root.join("list.qv"), LIST).unwrap();
        fs::write(
            root.join("contracts.json"),
            json!({"schema_version": 1, "modules": ["list.qv"]}).to_string(),
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
                    "description":"released-element fixture"}},
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
        let records = units
            .iter()
            .map(|(path, _)| {
                let file = checkout.join(path);
                json!({
                    "directory": checkout, "file": file,
                    "arguments": [driver, "-std=gnu17", "-c", file, "-o", file.with_extension("o")],
                })
            })
            .collect::<Vec<_>>();
        let database = Value::Array(records).to_string();
        fs::write(root.join("compile_commands.json"), &database).unwrap();
        for name in [
            "fluent-bit",
            "link.txt",
            "fluent-bit.map",
            "clang",
            "ld",
            "home.o",
        ] {
            fs::write(root.join(name), name).unwrap();
        }
        Self { root, database }
    }

    /// The artifact: `drain` is one strong default-visibility definition from
    /// `home.c`'s linked object, and both configurations are linked.
    fn artifact(&self) -> PathBuf {
        let bound = |name: &str| {
            let path = self.root.join(name);
            (path.clone(), sha256(&fs::read(&path).unwrap()))
        };
        let entries = qcore_audit::compile_db::compile_entries(&self.database).unwrap();
        let configurations = entries
            .iter()
            .enumerate()
            .map(|(id, entry)| {
                json!({
                    "id": id, "file": entry.file, "directory": entry.directory,
                    "command_sha256": sha256(entry.command.as_bytes()),
                    "object": Value::Null, "linked": true, "reproduction": "equal",
                    "route_unit": Value::Null,
                })
            })
            .collect::<Vec<_>>();
        let (executable, executable_sha256) = bound("fluent-bit");
        let (link_txt, link_txt_sha256) = bound("link.txt");
        let (map, map_sha256) = bound("fluent-bit.map");
        let (driver, driver_sha256) = bound("clang");
        let (linker, linker_sha256) = bound("ld");
        let (_, object_sha256) = bound("home.o");
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
            "inputs": [{"id": 0, "label": "home.o", "sha256": object_sha256, "configurations": [0]}],
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
                "home_unit": Value::Null, "decision": "derived", "reason": Value::Null,
                "detail": "",
            }],
        });
        let path = self.root.join("link-resolution.json");
        fs::write(&path, artifact.to_string()).unwrap();
        path
    }

    fn run(&self, name: &str, environment: &[(&str, &str)]) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_quirin-audit"));
        command.current_dir(&self.root);
        for (key, value) in environment {
            command.env(key, value);
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
                "--report",
                &format!("{name}/report.json"),
                "--jobs",
                "1",
            ])
            .output()
            .unwrap()
    }

    /// One caller row: its status and diagnostics' texts.
    fn row(&self, name: &str, function: &str) -> (String, String) {
        let report: Value = serde_json::from_str(
            &fs::read_to_string(self.root.join(name).join("report.json")).unwrap(),
        )
        .unwrap();
        let row = report["units"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|unit| unit["functions"].as_array().cloned().unwrap_or_default())
            .find(|row| row["function"] == function)
            .unwrap_or_else(|| panic!("no {function} row: {report}"));
        (
            row["status"].as_str().unwrap().to_owned(),
            row["diagnostics"].to_string(),
        )
    }
}

fn succeeded(output: &Output) {
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn clang() -> Option<PathBuf> {
    qcore_c_gateway::clang_command()
        .map(|command| PathBuf::from(command.get_program()))
        .ok()
}

#[test]
fn a_released_element_summary_crosses_units_on_the_link_standing_route() {
    let Some(clang) = clang() else { return };
    let fixture = Fixture::new(&clang, HOME);
    let artifact = fixture.artifact();
    let artifact = artifact.to_str().unwrap();

    // Without the artifact the cross-unit call is uncontracted.
    succeeded(&fixture.run("plain", &[]));
    let (status, diagnostics) = fixture.row("plain", "subject");
    assert_eq!(status, "Unknown");
    assert!(
        diagnostics.contains("call to uncontracted function `drain`"),
        "{diagnostics}"
    );

    // Rule 40 names the home; rule 4c carries its summary with the clause.
    let output = fixture.run("derived", &[("QCORE_CROSS_UNIT_LINK_RESOLUTION", artifact)]);
    succeeded(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("derived (rule 40)"), "{stdout}");
    assert_eq!(fixture.row("derived", "drain").0, "Safe");
    let (status, diagnostics) = fixture.row("derived", "subject");
    assert_eq!(status, "Safe", "{diagnostics}\n{stdout}");

    // The caller applied the clause: its publication is forgotten, so the
    // unlink after the call refuses, and nothing is accused.
    let (status, diagnostics) = fixture.row("derived", "published");
    assert_eq!(status, "Unknown", "{diagnostics}");
    assert!(
        !diagnostics.contains("call to uncontracted function `drain`"),
        "{diagnostics}"
    );
    assert!(
        !diagnostics.contains("Leak") && !diagnostics.contains("DoubleRelease"),
        "{diagnostics}"
    );

    // With rule 43 off the home publishes no summary: nothing crosses.
    let output = fixture.run(
        "rule-43-off",
        &[
            ("QCORE_CROSS_UNIT_LINK_RESOLUTION", artifact),
            ("QCORE_GATEWAY_RULES_OFF", "43"),
        ],
    );
    succeeded(&output);
    let (status, diagnostics) = fixture.row("rule-43-off", "subject");
    assert_eq!(status, "Unknown");
    assert!(
        diagnostics.contains("call to uncontracted function `drain`"),
        "{diagnostics}"
    );
}

/// The home with a conditional group whose arms leave different file-scope
/// brace depths in the raw scan (the shape of the tail-pause slice's
/// `tail_file.c`): its definition census is incomplete.
const INCOMPLETE_HOME: &str = "int drain(struct queue *q) { int c = 0; struct mk_list *it, *tmp;\n\
     struct item *e; mk_list_foreach_safe(it, tmp, &q->items) {\n\
     e = mk_list_entry(it, struct item, _head); item_destroy(e); c++; } return c; }\n\
#if 0\n\
static void never(void) {\n\
#endif\n";

#[test]
fn an_incomplete_home_census_refuses_the_imports_and_keeps_the_route() {
    let Some(clang) = clang() else { return };
    let fixture = Fixture::new(&clang, INCOMPLETE_HOME);
    let artifact = fixture.artifact();
    let output = fixture.run(
        "incomplete",
        &[(
            "QCORE_CROSS_UNIT_LINK_RESOLUTION",
            artifact.to_str().unwrap(),
        )],
    );
    // Before this card the home's export analysis failed on the census and
    // rejected the whole route; now the census refuses the import phase.
    succeeded(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("cross-unit import phase refused: incomplete definition census"),
        "{stdout}"
    );
    let (status, diagnostics) = fixture.row("incomplete", "subject");
    assert_eq!(status, "Unknown");
    assert!(
        diagnostics.contains("call to uncontracted function `drain`"),
        "{diagnostics}"
    );
}
