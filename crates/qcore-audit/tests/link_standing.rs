//! Card **LINK-STANDING** (rule 40): `quirin-audit current` with a
//! link-resolution artifact (`QCORE_CROSS_UNIT_LINK_RESOLUTION`) adopts the
//! artifact's derived home as rule 4c's resolution, end to end through the
//! route: the caller of a Safe home in another unit becomes `Safe` with the
//! artifact named as derived evidence. An artifact for another database, or
//! one that does not link the caller's configuration, leaves the call
//! uncontracted, and a stated map with an artifact is refused.
#![cfg(unix)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

use serde_json::{Value, json};

static NEXT: AtomicUsize = AtomicUsize::new(0);

const HEAP: &str = "module contract;\n\
extern fn alloc() -> ptr<mut, u8>\n\
contract {\n\
    requires resource_contract_v1;\n\
    requires resource_revision(flexible);\n\
    ensures normal resource_return_owned(nonnull);\n\
};\n\
extern fn dispose(p: ptr<mut, u8>) -> unit\n\
contract {\n\
    requires resource_contract_v1;\n\
    requires resource_revision(flexible);\n\
    ensures normal resource_released(p, nonnull);\n\
    ensures normal resource_return_none();\n\
};\n";

const UNITS: [(&str, &str); 2] = [
    (
        "src/home.c",
        "unsigned char *alloc(void);\nvoid dispose(unsigned char *p);\n\
         void release(unsigned char *p) { dispose(p); }\n",
    ),
    (
        "src/caller.c",
        "typedef unsigned char byte;\nbyte *alloc(void);\nvoid release(byte *p);\n\
         void subject(void) { byte *p = alloc(); release(p); }\n",
    ),
];

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

/// A committed checkout, a top-level source profile, a contract module, the
/// compile database and the files a link-resolution artifact binds.
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
            "qcore-link-standing-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let checkout = root.join("checkout");
        fs::create_dir_all(checkout.join("src")).unwrap();
        for (path, text) in UNITS {
            fs::write(checkout.join(path), text).unwrap();
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
                "Link-standing fixture",
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
        fs::write(root.join("heap.qcore"), HEAP).unwrap();
        fs::write(
            root.join("contracts.json"),
            json!({"schema_version": 1, "modules": ["heap.qcore"]}).to_string(),
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
                "pins":{"sample":{"git_commit":commit,"clean_paths":[],"description":"link-standing fixture"}},
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
        let records = UNITS
            .iter()
            .map(|(path, _)| {
                let file = checkout.join(path);
                json!({
                    "directory": checkout, "file": file,
                    "arguments": [driver, "-std=c11", "-c", file, "-o", file.with_extension("o")],
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

    /// The artifact: `release` is one strong definition from `home.c`'s
    /// object; `caller_linked` says whether `caller.c`'s configuration is
    /// linked; `database` is the digest it records.
    fn artifact(&self, caller_linked: bool, database: &str) -> PathBuf {
        let bound = |name: &str| {
            let path = self.root.join(name);
            (path.clone(), sha256(&fs::read(&path).unwrap()))
        };
        let entries = qcore_audit::compile_db::compile_entries(&self.database).unwrap();
        let configurations = entries
            .iter()
            .enumerate()
            .map(|(id, entry)| {
                let linked = id == 0 || caller_linked;
                json!({
                    "id": id, "file": entry.file, "directory": entry.directory,
                    "command_sha256": sha256(entry.command.as_bytes()),
                    "object": Value::Null, "linked": linked,
                    "reproduction": if linked { json!("equal") } else { Value::Null },
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
            "compile_database": {"path": "compile_commands.json", "sha256": database},
            "inputs": [{"id": 0, "label": "home.o", "sha256": object_sha256, "configurations": [0]}],
            "configurations": configurations,
            "symbols": [{
                "name": "release", "kind": "function",
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
        let path = self.root.join(format!(
            "link-resolution-{}.json",
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::write(&path, artifact.to_string()).unwrap();
        path
    }

    fn run(&self, name: &str, environment: &[(&str, &Path)]) -> Output {
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

    /// The `subject` row's status and the run's report texts.
    fn subject(&self, name: &str) -> (String, Value, String) {
        let directory = self.root.join(name);
        let report: Value =
            serde_json::from_str(&fs::read_to_string(directory.join("report.json")).unwrap())
                .unwrap();
        // The report and its contract sidecar, which carries the run's
        // assumptions.
        let texts = ["report.json", "report.contracts.json"]
            .iter()
            .map(|file| fs::read_to_string(directory.join(file)).unwrap_or_default())
            .collect::<String>();
        let caller = report["units"]
            .as_array()
            .unwrap()
            .iter()
            .find(|unit| {
                unit["functions"]
                    .as_array()
                    .is_some_and(|rows| rows.iter().any(|row| row["function"] == "subject"))
            })
            .unwrap_or_else(|| panic!("no subject row: {report}"))
            .clone();
        let status = caller["functions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["function"] == "subject")
            .unwrap()["status"]
            .as_str()
            .unwrap()
            .to_owned();
        (status, caller, texts)
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
fn the_current_route_adopts_a_derived_home_only_under_its_conditions() {
    let Some(clang) = clang() else { return };
    let fixture = Fixture::new(&clang);
    let database = sha256(fixture.database.as_bytes());

    // Without an artifact the cross-unit call is uncontracted.
    succeeded(&fixture.run("plain", &[]));
    let (plain, _, _) = fixture.subject("plain");
    assert_eq!(plain, "Unknown");

    // The derived home is adopted: `subject` is Safe, and the run names the
    // artifact as derived evidence, with no stated premise.
    let adopted = fixture.artifact(true, &database);
    let output = fixture.run(
        "derived",
        &[("QCORE_CROSS_UNIT_LINK_RESOLUTION", adopted.as_path())],
    );
    succeeded(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("derived (rule 40)"), "{stdout}");
    let (status, _, texts) = fixture.subject("derived");
    assert_eq!(status, "Safe", "{stdout}");
    assert!(
        texts.contains("\"evidence\":\"derived\"") || texts.contains("\"evidence\": \"derived\"")
    );

    // An artifact for another database refuses as a whole.
    let other = fixture.artifact(true, &"0".repeat(64));
    let output = fixture.run(
        "other-database",
        &[("QCORE_CROSS_UNIT_LINK_RESOLUTION", other.as_path())],
    );
    succeeded(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("compile database differs from the route's"),
        "{stdout}"
    );
    assert_eq!(fixture.subject("other-database").0, "Unknown");

    // A caller whose configuration is not linked holds no import; its row
    // names the refusal.
    let unlinked = fixture.artifact(false, &database);
    let output = fixture.run(
        "unlinked",
        &[("QCORE_CROSS_UNIT_LINK_RESOLUTION", unlinked.as_path())],
    );
    succeeded(&output);
    let (status, caller, _) = fixture.subject("unlinked");
    assert_eq!(status, "Unknown");
    assert!(
        caller["cross_unit_link_refusals"][0]["reason"]
            .as_str()
            .is_some_and(|reason| reason.starts_with("caller configuration is not a linked")),
        "{caller}"
    );

    // A stated map and an artifact together are refused.
    fs::write(
        fixture.root.join("link-map.json"),
        json!({"schema_version": 1, "premise": "stated",
               "symbols": [{"function": "release", "home_unit": "sample/src/home.c"}]})
        .to_string(),
    )
    .unwrap();
    let map = fixture.root.join("link-map.json");
    let output = fixture.run(
        "both",
        &[
            ("QCORE_CROSS_UNIT_LINK_RESOLUTION", adopted.as_path()),
            ("QCORE_CROSS_UNIT_LINK_MAP", map.as_path()),
        ],
    );
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("exclusive"));
}
