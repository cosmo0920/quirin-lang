//! G03: a second source set is added by configuration only.
//!
//! These are generic regressions for the shape the pinned `chunkio-core`
//! source set uses: a nested `lib/<name>/src` root with `top_level` unit
//! selection, an unmodified build database, and no provider binding. They
//! pin the fail-closed boundaries a new source set depends on. They do not
//! certify any library: `Unknown` stays in the denominator.

use qcore_contract::profile_loader::sha256_hex;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

/// Files chosen to reproduce the three resource-projection outcomes the
/// configured driver can publish for a nested-root source set.
fn sources() -> BTreeMap<&'static str, &'static str> {
    BTreeMap::from([
        (
            "balanced.c",
            "extern void *alloc(void); extern void dispose(void *p);\nvoid balanced(void) { void *p = alloc(); dispose(p); }\n",
        ),
        (
            "leak.c",
            "extern void *alloc(void);\nvoid leak(void) { void *p = alloc(); }\n",
        ),
        (
            "opaque.c",
            "extern void opaque(void);\nvoid unknown_effect(void) { opaque(); }\n",
        ),
    ])
}

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "qcore-second-source-set-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let checkout = root.join("checkout");
        let library_root = "lib/example/src";
        fs::create_dir_all(checkout.join(library_root)).unwrap();
        for (name, source) in sources() {
            fs::write(checkout.join(library_root).join(name), source).unwrap();
        }
        for args in [
            vec!["init", "--quiet"],
            vec!["add", "lib"],
            vec![
                "-c",
                "user.name=Second Source Set",
                "-c",
                "user.email=audit@example.invalid",
                "commit",
                "--quiet",
                "-m",
                "Pin a second source set",
            ],
        ] {
            let output = Command::new("git")
                .current_dir(&checkout)
                .args(args)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        let commit = Command::new("git")
            .current_dir(&checkout)
            .args(["rev-parse", "HEAD"])
            .output()
            .unwrap();
        let commit = String::from_utf8(commit.stdout).unwrap().trim().to_owned();

        fs::write(
            root.join("memory.qcore"),
            include_str!("fixtures/alloc_release.qcore"),
        )
        .unwrap();
        fs::write(
            root.join("contracts.json"),
            json!({"schema_version": 1, "modules": ["memory.qcore"]}).to_string(),
        )
        .unwrap();

        let audited = sources()
            .into_iter()
            .map(|(name, source)| {
                json!({
                    "path": format!("{library_root}/{name}"),
                    "sha256": sha256_hex(source.as_bytes()),
                })
            })
            .collect::<Vec<_>>();
        let library = json!({
            "pin": "example",
            "source_roots": [{"root": library_root, "selection": "top_level"}],
            "audited_files": audited,
            "source_witnesses": [], "permitted_mechanisms": [], "operation_summaries": {},
            "api_dispositions": {}, "accepted_builds": [], "required_evidence": [],
            "typed_import": null, "callback_summaries": {}, "boolean_field_models": [],
            "provider": null,
        });
        fs::create_dir_all(root.join("profiles/example")).unwrap();
        fs::write(
            root.join("profiles/example/profile.json"),
            json!({
                "schema_version": 1, "source_set": "example",
                "pins": {"example": {
                    "git_commit": commit, "clean_paths": [],
                    "description": "second source set fixture",
                }},
                "libraries": {"example": library},
                "resource_contract_manifest": "../../contracts.json",
                "provider_manifests": [],
            })
            .to_string(),
        )
        .unwrap();

        let database = sources()
            .into_keys()
            .map(|name| {
                json!({
                    "directory": checkout,
                    "file": format!("{library_root}/{name}"),
                    "arguments": [
                        "clang", "-std=gnu17", "-Wall", "-g", "-fPIC", "-Wa,--noexecstack",
                        "-c", format!("{library_root}/{name}"),
                        "-o", format!("{name}.o"),
                    ],
                })
            })
            .collect::<Vec<_>>();
        fs::write(
            root.join("compile_commands.json"),
            serde_json::to_string(&database).unwrap(),
        )
        .unwrap();
        fs::write(
            root.join("audit.json"),
            json!({
                "schema_version": 1, "profile_directory": "profiles/example",
                "checkout": "checkout", "compile_database": "compile_commands.json",
                "libraries": ["example"], "report": "output/report.json",
            })
            .to_string(),
        )
        .unwrap();
        Self(root)
    }

    fn run(&self) -> Output {
        Command::new(env!("CARGO_BIN_EXE_qcore-audit"))
            .arg(self.0.join("audit.json"))
            .output()
            .unwrap()
    }

    fn mutate(&self, path: &str, change: impl FnOnce(&mut Value)) {
        let path = self.0.join(path);
        let mut value: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        change(&mut value);
        fs::write(path, value.to_string()).unwrap();
    }

    fn report(&self) -> Value {
        serde_json::from_slice(&fs::read(self.0.join("output/report.json")).unwrap()).unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Every published verdict is keyed by its unit, not counted in aggregate.
#[test]
fn a_nested_root_source_set_publishes_keyed_verdicts_from_configuration_alone() {
    let fixture = Fixture::new();
    let output = fixture.run();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report = fixture.report();
    let keyed = report["units"]
        .as_array()
        .unwrap()
        .iter()
        .map(|unit| {
            (
                unit["file"].as_str().unwrap().to_owned(),
                unit["status"].as_str().unwrap().to_owned(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    assert_eq!(
        keyed,
        BTreeMap::from([
            ("lib/example/src/balanced.c".to_owned(), "Safe".to_owned()),
            ("lib/example/src/leak.c".to_owned(), "Unsafe".to_owned()),
            ("lib/example/src/opaque.c".to_owned(), "Unknown".to_owned()),
        ])
    );
    let assumptions: Value = serde_json::from_slice(
        &fs::read(fixture.0.join("output/report.assumptions.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(assumptions["profile"]["source_set"], "example");
    assert!(
        assumptions["scope"]
            .as_str()
            .unwrap()
            .contains("Unsafe is provisional")
    );
}

/// An unmodeled call is reported as `Unknown` with a stated reason; it is
/// never dropped from the report or silently projected as `Safe`.
#[test]
fn an_unknown_source_form_is_reported_with_a_reason_rather_than_omitted() {
    let fixture = Fixture::new();
    assert!(fixture.run().status.success());
    let report = fixture.report();
    let unknown = report["units"]
        .as_array()
        .unwrap()
        .iter()
        .find(|unit| unit["file"] == "lib/example/src/opaque.c")
        .expect("the unmodeled unit stays in the report");
    assert_eq!(unknown["status"], "Unknown");
    assert!(
        unknown["reason"]
            .as_str()
            .is_some_and(|reason| reason.contains("unresolved")),
        "{unknown}"
    );
    assert_eq!(unknown["source_functions"], json!(["unknown_effect"]));
}

/// A pin that no longer names the audited revision rejects the whole audit.
#[test]
fn a_changed_source_pin_rejects_before_any_unit_is_published() {
    let fixture = Fixture::new();
    fixture.mutate("profiles/example/profile.json", |profile| {
        profile["pins"]["example"]["git_commit"] =
            json!("0123456789abcdef0123456789abcdef01234567");
    });
    let output = fixture.run();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(
        stderr.contains("0123456789abcdef0123456789abcdef01234567"),
        "{stderr}"
    );
    assert!(!fixture.0.join("output/report.json").exists());
}

/// A configured source-contract package that is not installed rejects; it
/// never falls back to an unbacked evaluation.
#[test]
fn an_absent_provider_package_rejects_the_configured_source_contract_route() {
    let fixture = Fixture::new();
    fs::write(
        fixture.0.join("source-contracts.json"),
        json!({
            "schema_version": 1,
            "packages": ["packages/qcore.absent"],
            "bindings": [{
                "library": "example", "provider": "qcore.absent",
                "contract": "memory.qcore",
            }],
        })
        .to_string(),
    )
    .unwrap();
    fixture.mutate("audit.json", |config| {
        config["source_contracts"] = json!({
            "configuration": "source-contracts.json",
            "source_rules": "memory.qcore",
            "libraries": [{"library": "example", "source_root": "lib/example/src"}],
        });
    });
    let output = fixture.run();
    assert!(
        !output.status.success(),
        "absent package unexpectedly accepted"
    );
    assert!(!fixture.0.join("output/report.json").exists());
}

/// The four transported build flags are accepted; a warning option the
/// gateway cannot preserve rejects rather than being silently discarded.
#[test]
fn a_compiler_option_the_gateway_cannot_preserve_rejects_the_database() {
    let fixture = Fixture::new();
    fixture.mutate("compile_commands.json", |database| {
        for entry in database.as_array_mut().unwrap() {
            entry["arguments"]
                .as_array_mut()
                .unwrap()
                .insert(2, json!("-Wextra"));
        }
    });
    let output = fixture.run();
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("-Wextra"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!fixture.0.join("output/report.json").exists());
}
