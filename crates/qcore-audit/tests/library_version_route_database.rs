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

//! Card LIBRARY-VERSION-PROBE-ROUTE-DATABASE (rule 53), through the shipped
//! entry points: the library version probe reads the include configurations
//! of the compile database the run analyses, on the current route
//! (`quirin-audit current --compile-database`), the configured route
//! (`quirin-audit CONFIG.json`, its `compile_database`) and `quirin-audit
//! pin --compile-database`. A bundled header set the database names wins
//! over the system one; two configurations disagreeing hold the declaration
//! `Unknown` (ambiguous); a database or configuration that cannot be read
//! holds it `Unknown` (unreadable) and refuses a pin, never falling back to
//! the default search path; `QCORE_LIBRARY_VERSION_COMPILE_DATABASE` is an
//! explicit override, recorded as one.
//!
//! The "system" header set is a directory on `C_INCLUDE_PATH`, which Clang
//! searches after every `-I` directory on every platform, so the fixture
//! needs no host library.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

use qcore_contract::profile_loader::sha256_hex;
use serde_json::{Value, json};

static NEXT: AtomicUsize = AtomicUsize::new(0);

const VARIABLE: &str = "QCORE_LIBRARY_VERSION_COMPILE_DATABASE";

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn clang_available() -> bool {
    if qcore_c_gateway::clang_command().is_ok() {
        return true;
    }
    assert!(
        std::env::var_os("QCORE_CLANG").is_none(),
        "explicit QCORE_CLANG is invalid"
    );
    eprintln!("skipping Clang-dependent library version route test");
    false
}

/// The audited units: the ranged function's caller, and a second unit so a
/// database can configure two units differently (the configured route
/// refuses two commands for one file).
const UNITS: [(&str, &str); 2] = [
    (
        "src/reset_user.c",
        "void fakelib_reset(void);\n\nvoid use_reset(void) {\n    fakelib_reset();\n}\n",
    ),
    ("src/other.c", "int other(int x) {\n    return x + 1;\n}\n"),
];

/// The compiler the databases name: the gateway's Clang, which the current
/// route requires.
fn driver() -> String {
    qcore_c_gateway::clang_command()
        .unwrap()
        .get_program()
        .to_string_lossy()
        .into_owned()
}

fn git(directory: &Path, args: &[&str]) -> String {
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
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "qcore-library-version-route-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        let fixture = Self(root.clone());
        // The audited checkout: one unit calling the ranged function.
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
                "user.name=Library Version Fixture",
                "-c",
                "user.email=library-version@example.invalid",
                "commit",
                "--quiet",
                "-m",
                "Library version route fixture",
            ],
        );
        let commit = git(&checkout, &["rev-parse", "HEAD"]);
        // The system header set (2.3.1), the bundled one (2.5.0), another
        // bundled copy (3.0.0) for a disagreeing configuration.
        fixture.header("system", 2, 3, 1);
        fixture.header("bundled", 2, 5, 0);
        fixture.header("other", 3, 0, 0);
        // The user crate stating the range, on a user load path.
        let krate = root.join("user/quirin-provider-fakelib");
        fs::create_dir_all(krate.join("src")).unwrap();
        fs::create_dir_all(krate.join("contracts/fakelib")).unwrap();
        fs::write(
            krate.join("Cargo.toml"),
            "[package]\nname = \"quirin-provider-fakelib\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        )
        .unwrap();
        fs::write(
            krate.join("src/lib.rs"),
            "pub const NAMESPACE: &str = \"fakelib\";\npub const PUBLISHED: &[&str] = &[];\n",
        )
        .unwrap();
        fs::write(
            krate.join("library-versions.json"),
            serde_json::to_vec_pretty(&json!({
                "schema_version": 1,
                "libraries": {"fakelib": {"header": "fakelib/fakelibv.h",
                    "major": "FAKELIB_VERSION_MAJOR", "minor": "FAKELIB_VERSION_MINOR",
                    "patch": "FAKELIB_VERSION_PATCH"}},
            }))
            .unwrap(),
        )
        .unwrap();
        fs::write(
            krate.join("contracts/fakelib/reset.qv"),
            "module contract;\n\nextern fn fakelib_reset() -> unit\ncontract {\n    \
             requires resource_contract_v1;\n    requires resource_revision(flexible);\n    \
             requires resource_library_version(fakelib, since \"2.0\");\n    \
             ensures normal resource_return_none();\n};\n",
        )
        .unwrap();
        fs::write(
            root.join("user/user-load-path.json"),
            serde_json::to_vec_pretty(&json!({
                "schema_version": 1,
                "extends": repository().join("providers/contract-load-path.json"),
                "crates": [{
                    "name": "quirin-provider-fakelib", "namespace": "fakelib",
                    "version": "0.1.0", "contracts": "quirin-provider-fakelib/contracts",
                    "dependencies": [], "modules": ["fakelib/reset.qv"], "published": {},
                }],
                "overrides": [],
                "checkers": [],
            }))
            .unwrap(),
        )
        .unwrap();
        succeeded(&fixture.quirin(&[
            "plugin",
            "views",
            "--load-path",
            "user/user-load-path.json",
            "--write",
        ]));
        // The source profile of the current and configured routes.
        let profile = root.join("profiles/sample");
        fs::create_dir_all(&profile).unwrap();
        fs::write(
            profile.join("profile.json"),
            json!({
                "schema_version": 1, "source_set": "sample",
                "pins": {"sample": {"git_commit": commit, "clean_paths": [],
                                    "description": "library version route fixture"}},
                "libraries": {"lib": {
                    "pin": "sample", "source_roots": [{"root": "src", "selection": "top_level"}],
                    "audited_files": UNITS.iter().map(|(path, text)| json!({
                        "path": path, "sha256": sha256_hex(text.as_bytes())})).collect::<Vec<_>>(),
                    "source_witnesses": [], "permitted_mechanisms": [], "operation_summaries": {},
                    "api_dispositions": {}, "accepted_builds": [], "required_evidence": [],
                    "typed_import": null, "callback_summaries": {}, "boolean_field_models": [],
                    "provider": null,
                }},
                "resource_contract_manifest": null,
                "provider_manifests": [],
            })
            .to_string(),
        )
        .unwrap();
        fs::write(
            profile.join("audit.json"),
            json!({"schema_version": 1, "libraries": [{"library": "lib", "label": null}]})
                .to_string(),
        )
        .unwrap();
        fixture.database("compile_commands.json", &[&["-Ibundled"]]);
        fixture
    }

    fn header(&self, directory: &str, major: u32, minor: u32, patch: u32) {
        let path = self.0.join(directory).join("fakelib");
        fs::create_dir_all(&path).unwrap();
        fs::write(
            path.join("fakelibv.h"),
            format!(
                "#define FAKELIB_VERSION_MAJOR {major}\n#define FAKELIB_VERSION_MINOR \
                 {minor}\n#define FAKELIB_VERSION_PATCH {patch}\n"
            ),
        )
        .unwrap();
    }

    /// A compile database of both units: unit `i` takes flag list `i`, or the
    /// last one. Include directories are relative to the fixture root, which
    /// is every record's directory.
    fn database(&self, name: &str, configurations: &[&[&str]]) {
        let driver = driver();
        let entries = UNITS
            .iter()
            .enumerate()
            .map(|(index, (path, _))| {
                let flags = configurations[index.min(configurations.len() - 1)];
                let file = self.0.join("checkout").join(path);
                let mut arguments = vec![driver.clone(), "-std=c11".to_owned()];
                arguments.extend(flags.iter().map(|flag| (*flag).to_owned()));
                arguments.extend(["-c".to_owned(), file.display().to_string()]);
                json!({"directory": self.0, "file": file, "arguments": arguments})
            })
            .collect::<Vec<_>>();
        fs::write(self.0.join(name), serde_json::to_vec(&entries).unwrap()).unwrap();
    }

    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_quirin-audit"));
        command
            .current_dir(&self.0)
            .env_remove(VARIABLE)
            .env("C_INCLUDE_PATH", self.0.join("system"));
        command
    }

    fn quirin(&self, args: &[&str]) -> Output {
        self.command().args(args).output().unwrap()
    }

    fn lock(&self) -> Value {
        self.json("user/quirin-provider-fakelib/contracts.lock")
    }

    fn json(&self, relative: &str) -> Value {
        serde_json::from_slice(&fs::read(self.0.join(relative)).unwrap()).unwrap()
    }

    fn pin(&self, extra: &[&str]) -> Output {
        self.quirin(
            &[
                &[
                    "pin",
                    "--load-path",
                    "user/user-load-path.json",
                    "--source-root",
                    "checkout",
                ],
                extra,
            ]
            .concat(),
        )
    }

    /// The current route over `database`; `use_reset`'s status and the
    /// version record.
    fn current(
        &self,
        out: &str,
        database: &str,
        override_database: Option<&str>,
    ) -> (String, Value) {
        let mut command = self.command();
        if let Some(path) = override_database {
            command.env(VARIABLE, self.0.join(path));
        }
        succeeded(
            &command
                .args([
                    "current",
                    "--checkout",
                    "checkout",
                    "--profile-directory",
                    "profiles/sample",
                    "--compile-database",
                    database,
                    "--load-path",
                    "user/user-load-path.json",
                    "--provider",
                    "quirin-provider-fakelib",
                    "--report",
                    &format!("{out}/report.json"),
                    "--monkey-source-report",
                    &format!("{out}/monkey-source.json"),
                    "--jobs",
                    "1",
                ])
                .output()
                .unwrap(),
        );
        (
            status(&self.json(&format!("{out}/report.json"))),
            self.json(&format!("{out}/report.contracts.json"))["contract_revision"]
                ["library_versions"]
                .clone(),
        )
    }

    /// The configured route over `database`; `use_reset`'s status and the
    /// version record.
    fn configured(&self, out: &str, database: &str) -> (String, Value) {
        fs::create_dir_all(self.0.join(out)).unwrap();
        let config = format!("{out}/audit.json");
        fs::write(
            self.0.join(&config),
            json!({
                "schema_version": 1,
                "profile_directory": "../profiles/sample",
                "checkout": "../checkout",
                "compile_database": format!("../{database}"),
                "libraries": ["lib"],
                "report": "report.json",
                "source_revision_policy": "checkout_head",
                "load_path": "../user/user-load-path.json",
                "providers": ["quirin-provider-fakelib"],
                "jobs": 1,
            })
            .to_string(),
        )
        .unwrap();
        succeeded(&self.quirin(&[&config]));
        (
            status(&self.json(&format!("{out}/report.json"))),
            self.json(&format!("{out}/report.assumptions.json"))["contract_revision"]
                ["library_versions"]
                .clone(),
        )
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
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

fn status(report: &Value) -> String {
    report["units"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|unit| unit["functions"].as_array().unwrap())
        .find(|row| row["function"] == "use_reset")
        .unwrap_or_else(|| panic!("no use_reset row: {report}"))["status"]
        .as_str()
        .unwrap()
        .to_owned()
}

fn declaration(record: &Value) -> &Value {
    &record["declarations"][0]
}

/// `pin` records the version of the headers the database names (the
/// bundled set beats the system one on the search path), the default
/// search path without the flag, and refuses a database it cannot read.
#[test]
fn pin_probes_the_database_it_is_given() {
    if !clang_available() {
        return;
    }
    let fixture = Fixture::new();
    succeeded(&fixture.pin(&[]));
    assert_eq!(
        fixture.lock()["library_versions"]["fakelib"]["version"],
        "2.3.1"
    );
    succeeded(&fixture.pin(&["--compile-database", "compile_commands.json"]));
    let lock = fixture.lock();
    assert_eq!(lock["library_versions"]["fakelib"]["version"], "2.5.0");
    let header = lock["library_versions"]["fakelib"]["header"]
        .as_str()
        .unwrap();
    assert!(
        Path::new(header).starts_with(fixture.0.join("bundled")),
        "{header}"
    );
    // An unreadable database refuses the pin and writes nothing.
    let before = fs::read(
        fixture
            .0
            .join("user/quirin-provider-fakelib/contracts.lock"),
    )
    .unwrap();
    let refused = fixture.pin(&["--compile-database", "missing.json"]);
    assert!(!refused.status.success());
    assert!(
        String::from_utf8_lossy(&refused.stderr).contains("library-version-unreadable"),
        "{}",
        String::from_utf8_lossy(&refused.stderr)
    );
    // So does a configuration the probe cannot reproduce.
    fixture.database("sysroot.json", &[&["-Ibundled", "--sysroot=/nowhere"]]);
    assert!(
        !fixture
            .pin(&["--compile-database", "sysroot.json"])
            .status
            .success()
    );
    // Two configurations disagreeing: no version is pinned.
    fixture.database("two.json", &[&["-Ibundled"], &["-Iother"]]);
    assert!(
        !fixture
            .pin(&["--compile-database", "two.json"])
            .status
            .success()
    );
    assert_eq!(
        fs::read(
            fixture
                .0
                .join("user/quirin-provider-fakelib/contracts.lock")
        )
        .unwrap(),
        before
    );
    // The flag is the probe's on a pin of resource locks only.
    for refused in [
        &["--check", "--compile-database", "compile_commands.json"][..],
        &["--outer", "--compile-database", "compile_commands.json"][..],
        &["--bounds", "--compile-database", "compile_commands.json"][..],
    ] {
        assert!(!fixture.pin(refused).status.success(), "{refused:?}");
    }
}

/// The current route probes its own `--compile-database`: in force at the
/// bundled version pinned from that database (the system headers report
/// another, so the default search path would hold it Pending), ambiguous
/// over two disagreeing configurations, unreadable over a configuration it
/// cannot reproduce, and the variable an explicit override.
#[test]
fn the_current_route_probes_its_own_database() {
    if !clang_available() {
        return;
    }
    let fixture = Fixture::new();
    succeeded(&fixture.pin(&["--compile-database", "compile_commands.json"]));
    let (status, record) = fixture.current("current-bundled", "compile_commands.json", None);
    assert_eq!(status, "Safe", "{record}");
    assert_eq!(declaration(&record)["in_force"], true, "{record}");
    assert_eq!(declaration(&record)["audited_version"], "2.5.0");
    let database = fixture.0.join("compile_commands.json");
    assert_eq!(
        record["configurations"],
        format!("compile-database:{}", database.display())
    );
    // Two configurations of the run disagree: ambiguous, Unknown.
    fixture.database("two.json", &[&["-Ibundled"], &["-Iother"]]);
    let (status, record) = fixture.current("current-two", "two.json", None);
    assert_eq!(status, "Unknown", "{record}");
    assert_eq!(declaration(&record)["reason"], "library-version-ambiguous");
    // A configuration the probe cannot reproduce (the other unit's; the
    // caller's is the pinned one): unreadable, Unknown, never the default
    // search path.
    fixture.database(
        "sysroot.json",
        &[&["-Ibundled"], &["-Ibundled", "--sysroot=/nowhere"]],
    );
    let (status, record) = fixture.current("current-sysroot", "sysroot.json", None);
    assert_eq!(status, "Unknown", "{record}");
    assert_eq!(declaration(&record)["reason"], "library-version-unreadable");
    // The variable overrides the route's database and says so: the system
    // headers (a database with no include directory) report 2.3.1, which
    // the lock does not pin.
    fixture.database("plain.json", &[&[]]);
    let (status, record) = fixture.current(
        "current-override",
        "compile_commands.json",
        Some("plain.json"),
    );
    assert_eq!(status, "Pending", "{record}");
    assert_eq!(declaration(&record)["reason"], "library-version-changed");
    assert_eq!(declaration(&record)["audited_version"], "2.3.1");
    assert!(
        record["configurations"]
            .as_str()
            .unwrap()
            .starts_with(&format!("override {VARIABLE}: compile-database:")),
        "{record}"
    );
}

/// The configured route probes its configuration's `compile_database`.
#[test]
fn the_configured_route_probes_its_own_database() {
    if !clang_available() {
        return;
    }
    let fixture = Fixture::new();
    succeeded(&fixture.pin(&["--compile-database", "compile_commands.json"]));
    let (status, record) = fixture.configured("configured-bundled", "compile_commands.json");
    assert_eq!(status, "Safe", "{record}");
    assert_eq!(declaration(&record)["in_force"], true, "{record}");
    assert_eq!(declaration(&record)["audited_version"], "2.5.0");
    fixture.database("two.json", &[&["-Ibundled"], &["-Iother"]]);
    let (status, record) = fixture.configured("configured-two", "two.json");
    assert_eq!(status, "Unknown", "{record}");
    assert_eq!(declaration(&record)["reason"], "library-version-ambiguous");
}
