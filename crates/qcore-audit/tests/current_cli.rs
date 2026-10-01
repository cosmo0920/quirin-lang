//! Card W44-QUIRIN-AUDIT-PROVIDERS: `quirin-audit current` end to end, with
//! contracts selected by provider crate on the command line, in a JSON
//! configuration, or both; the contracts-only mode; and the matching
//! `providers` / `mode` fields of the plain `quirin-audit CONFIG.json` route.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};

use qcore_contract::contract_loader::load_resource_contract_manifest_text;
use qcore_contract::profile_loader::sha256_hex;
use serde_json::{Value, json};

static NEXT: AtomicUsize = AtomicUsize::new(0);
static CLANG: OnceLock<Result<PathBuf, String>> = OnceLock::new();

/// The configured Clang, or `None` (and a skip) when none is available.
fn clang() -> Option<PathBuf> {
    match CLANG.get_or_init(|| {
        qcore_c_gateway::clang_command().map(|command| PathBuf::from(command.get_program()))
    }) {
        Ok(path) => Some(path.clone()),
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent current-audit test: {reason}");
            None
        }
    }
}

const HEAP: &str = "module contract;\n\
extern fn alloc() -> ptr<mut, u8>\n\
contract {\n\
    requires resource_contract_v1;\n\
    ensures normal resource_return_owned(nonnull);\n\
};\n\
extern fn dispose(p: ptr<mut, u8>) -> unit\n\
contract {\n\
    requires resource_contract_v1;\n\
    ensures normal resource_released(p, nonnull);\n\
    ensures normal resource_return_none();\n\
};\n";

const OTHER: &str = "module contract;\n\
extern fn close_handle(p: ptr<mut, u8>) -> unit\n\
contract {\n\
    requires resource_contract_v1;\n\
    ensures normal resource_released(p, nonnull);\n\
    ensures normal resource_return_none();\n\
};\n";

const SOURCE: &str = "unsigned char *alloc(void);\n\
void dispose(unsigned char *p);\n\
void subject(void) { unsigned char *p = alloc(); dispose(p); }\n";

/// The fixture's translation units: `unit.c` and five more, some of which
/// release on one path only, so a parallel fold has distinct rows to order.
fn units() -> Vec<(String, String)> {
    let mut units = vec![("src/unit.c".to_owned(), SOURCE.to_owned())];
    for index in 0..5 {
        units.push((
            format!("src/extra_{index}.c"),
            format!(
                "unsigned char *alloc(void);\nvoid dispose(unsigned char *p);\n\
                 void extra_{index}(int c) {{ unsigned char *p = alloc(); if (c > {index}) dispose(p); }}\n"
            ),
        ));
    }
    units
}

struct Fixture(PathBuf);

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

impl Fixture {
    /// A committed checkout, a source profile (whose declared manifest is
    /// `profile_manifest`), a two-crate load path and a Clang database whose
    /// driver is `driver`.
    fn new(driver: &Path, profile_manifest: bool) -> Self {
        let root = std::env::temp_dir().join(format!(
            "qcore-current-cli-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let checkout = root.join("checkout");
        fs::create_dir_all(checkout.join("src")).unwrap();
        for (path, text) in units() {
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
                "Current fixture",
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
        for (krate, module, text) in [
            ("quirin-provider-alpha", "alpha/heap.qcore", HEAP),
            ("qcore-provider-beta", "beta/other.qcore", OTHER),
        ] {
            let path = root
                .join("providers")
                .join(krate)
                .join("contracts")
                .join(module);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, text).unwrap();
        }
        fs::write(
            root.join("providers/contract-load-path.json"),
            serde_json::to_string_pretty(&json!({"schema_version": 1, "crates": [
                {"name": "qcore-provider-beta", "namespace": "beta", "version": "0.2.0",
                 "contracts": "qcore-provider-beta/contracts",
                 "dependencies": ["quirin-provider-alpha"], "modules": ["beta/other.qcore"],
                 "published": {}, "resource_view": ["beta/other.qcore"]},
                {"name": "quirin-provider-alpha", "namespace": "alpha", "version": "0.1.0",
                 "contracts": "quirin-provider-alpha/contracts", "dependencies": [],
                 "modules": ["alpha/heap.qcore"], "published": {},
                 "resource_view": ["alpha/heap.qcore"]},
            ]}))
            .unwrap(),
        )
        .unwrap();
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
            "audited_files": units().iter().map(|(path, text)| json!({"path": path, "sha256": sha256_hex(text.as_bytes())})).collect::<Vec<_>>(),
            "source_witnesses":[], "permitted_mechanisms":[], "operation_summaries":{},
            "api_dispositions":{}, "accepted_builds":[], "required_evidence":[], "typed_import":null,
            "callback_summaries":{}, "boolean_field_models":[], "provider":null,
        });
        fs::write(profile.join("profile.json"), json!({
            "schema_version":1, "source_set":"sample",
            "pins":{"sample":{"git_commit":commit,"clean_paths":[],"description":"current fixture"}},
            "libraries":{"sample":library},
            "resource_contract_manifest": if profile_manifest { json!("../../contracts.json") } else { Value::Null },
            "provider_manifests":[],
        }).to_string()).unwrap();
        fs::write(
            profile.join("audit.json"),
            json!({"schema_version":1,"libraries":[{"library":"sample","label":null}]}).to_string(),
        )
        .unwrap();
        fs::write(
            root.join("compile_commands.json"),
            json!(
                units()
                    .iter()
                    .map(|(path, _)| json!({
                        "directory": checkout, "file": path,
                        "arguments": [driver, "-std=c11", "-c", path, "-o", path.replace(".c", ".o")],
                    }))
                    .collect::<Vec<_>>()
            )
            .to_string(),
        )
        .unwrap();
        Self(root)
    }

    /// The fixed arguments of every run, writing into `out/`.
    fn base_args(&self, out: &str) -> Vec<String> {
        [
            "--checkout",
            "checkout",
            "--profile-directory",
            "profiles/sample",
            "--compile-database",
            "compile_commands.json",
            "--report",
            &format!("{out}/report.json"),
            "--jobs",
            "1",
        ]
        .iter()
        .map(|value| (*value).to_owned())
        .collect()
    }

    fn config(&self, name: &str, value: &Value) -> String {
        fs::write(
            self.0.join(name),
            serde_json::to_string_pretty(value).unwrap(),
        )
        .unwrap();
        name.to_owned()
    }

    fn run(&self, args: &[String]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_quirin-audit"))
            .current_dir(&self.0)
            .arg("current")
            .args(args)
            .output()
            .unwrap()
    }

    fn read(&self, path: &str) -> Value {
        serde_json::from_slice(&fs::read(self.0.join(path)).unwrap()).unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

fn succeeded(output: &Output) {
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn refused(output: &Output, expected: &str) {
    assert!(
        !output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(expected),
        "expected `{expected}` in:\n{stderr}"
    );
}

#[test]
fn command_line_configuration_and_both_forms_produce_the_same_run() {
    let Some(clang) = clang() else { return };
    let fixture = Fixture::new(&clang, true);
    let providers = strings(&[
        "--load-path",
        "providers/contract-load-path.json",
        "--provider",
        "quirin-provider-alpha",
    ]);
    // Command line only.
    let mut args = fixture.base_args("cli");
    args.extend(providers.clone());
    succeeded(&fixture.run(&args));
    // Configuration only: the same settings, relative to the configuration.
    let config = fixture.config(
        "current.json",
        &json!({
            "schema_version": 1, "checkout": "checkout", "profile_directory": "profiles/sample",
            "compile_database": "compile_commands.json", "report": "config/report.json",
            "load_path": "providers/contract-load-path.json",
            "providers": ["quirin-provider-alpha"], "jobs": 1,
        }),
    );
    succeeded(&fixture.run(&strings(&["--config", &config])));
    // Both, declaring the same selection in two spellings.
    let both = fixture.config(
        "both.json",
        &json!({
            "schema_version": 1, "load_path": "providers/contract-load-path.json",
            "providers": ["quirin-provider-alpha:alpha/heap.qcore"],
        }),
    );
    let mut args = fixture.base_args("both");
    args.extend(providers);
    args.extend(strings(&["--config", &both]));
    succeeded(&fixture.run(&args));

    let report = fs::read(fixture.0.join("cli/report.json")).unwrap();
    assert_eq!(
        fs::read(fixture.0.join("config/report.json")).unwrap(),
        report
    );
    assert_eq!(
        fs::read(fixture.0.join("both/report.json")).unwrap(),
        report
    );
    let mut sidecars = Vec::new();
    for (out, sources) in [
        ("cli", json!(["command_line"])),
        ("config", json!(["configuration"])),
        ("both", json!(["command_line", "configuration"])),
    ] {
        let mut sidecar = fixture.read(&format!("{out}/report.contracts.json"));
        assert_eq!(sidecar["provider_selection"]["sources"], sources, "{out}");
        sidecar["provider_selection"]["sources"] = Value::Null;
        sidecars.push(sidecar);
    }
    assert_eq!(sidecars[0], sidecars[1]);
    assert_eq!(sidecars[0], sidecars[2]);
    let units = fixture.read("cli/report.json")["units"].clone();
    assert_eq!(units.as_array().unwrap().len(), 6);
}

#[test]
fn conflicting_forms_refuse_instead_of_merging() {
    let Some(clang) = clang() else { return };
    let fixture = Fixture::new(&clang, true);
    let other = fixture.config(
        "other.json",
        &json!({
            "schema_version": 1, "load_path": "providers/contract-load-path.json",
            "providers": ["qcore-provider-beta"],
        }),
    );
    let mut args = fixture.base_args("out");
    args.extend(strings(&[
        "--load-path",
        "providers/contract-load-path.json",
        "--provider",
        "quirin-provider-alpha",
        "--config",
        &other,
    ]));
    refused(&fixture.run(&args), "provider selection conflict");
    assert!(!fixture.0.join("out/report.json").exists());
    // Any other setting given twice with different values refuses too.
    let checkout = fixture.config(
        "checkout.json",
        &json!({"schema_version": 1, "checkout": "elsewhere"}),
    );
    let mut args = fixture.base_args("out");
    args.extend(strings(&["--config", &checkout]));
    refused(&fixture.run(&args), "`checkout` conflicts");
}

#[test]
fn an_unknown_provider_refuses_before_any_report() {
    let Some(clang) = clang() else { return };
    let fixture = Fixture::new(&clang, true);
    let mut args = fixture.base_args("out");
    args.extend(strings(&[
        "--load-path",
        "providers/contract-load-path.json",
        "--provider",
        "quirin-provider-omega",
    ]));
    refused(
        &fixture.run(&args),
        "unknown provider `quirin-provider-omega`",
    );
    assert!(!fixture.0.join("out").exists());
}

#[test]
fn contract_sources_are_one_declaration_and_there_is_no_default_provider_set() {
    let Some(clang) = clang() else { return };
    // No selection, no manifest, and a profile that declares none.
    let bare = Fixture::new(&clang, false);
    refused(
        &bare.run(&bare.base_args("out")),
        "no resource contracts selected",
    );
    // A selection and an explicit manifest are two declarations of one input.
    let fixture = Fixture::new(&clang, true);
    let mut args = fixture.base_args("out");
    args.extend(strings(&[
        "--load-path",
        "providers/contract-load-path.json",
        "--provider",
        "quirin-provider-alpha",
        "--contract-manifest",
        "contracts.json",
    ]));
    refused(&fixture.run(&args), "two declarations of one input");
    // A load path or a guard without a selection has nothing to act on.
    let mut args = fixture.base_args("out");
    args.extend(strings(&[
        "--load-path",
        "providers/contract-load-path.json",
    ]));
    refused(&fixture.run(&args), "no provider is selected");
    // An explicit manifest alone, and the profile's default alone, both run.
    let mut args = fixture.base_args("manifest");
    args.extend(strings(&["--contract-manifest", "contracts.json"]));
    succeeded(&fixture.run(&args));
    succeeded(&fixture.run(&fixture.base_args("default")));
    // One manifest file reached by two spellings: the same verdicts; the
    // file-path contract keys (and so the digests) name the spelling used.
    assert_eq!(
        fixture.read("manifest/report.json")["units"][0]["status"],
        fixture.read("default/report.json")["units"][0]["status"]
    );
    assert_eq!(
        fixture.read("default/report.contracts.json")["contract_manifest"],
        fs::read_to_string(fixture.0.join("contracts.json")).unwrap()
    );
    assert!(fixture.read("default/report.contracts.json")["provider_selection"].is_null());
}

#[test]
fn the_sidecar_reproduces_the_provider_selection() {
    let Some(clang) = clang() else { return };
    let fixture = Fixture::new(&clang, true);
    let mut args = fixture.base_args("out");
    args.extend(strings(&[
        "--load-path",
        "providers/contract-load-path.json",
        "--provider",
        "qcore-provider-beta",
        "--provider",
        "quirin-provider-alpha:alpha/heap.qcore",
    ]));
    succeeded(&fixture.run(&args));
    let sidecar = fixture.read("out/report.contracts.json");
    let selection = &sidecar["provider_selection"];
    assert_eq!(
        selection["entries"],
        json!([
            "qcore-provider-beta",
            "quirin-provider-alpha:alpha/heap.qcore"
        ])
    );
    // The CLI resolves relative arguments against current_dir(), which resolves
    // temp-directory symlinks (e.g. /var -> /private/var on macOS). Preserve the
    // declared manifest suffix: only the working-directory root is canonical.
    let root = fixture.0.canonicalize().unwrap();
    // GetCurrentDirectoryW returns a regular drive path, while canonicalize()
    // returns a verbatim path whose join() collapses the declared `../..` suffix.
    #[cfg(windows)]
    let root = PathBuf::from(
        root.to_str()
            .unwrap()
            .strip_prefix(r"\\?\")
            .unwrap_or(root.to_str().unwrap()),
    );
    assert_eq!(
        selection["profile_contract_manifest"]["path"],
        json!(root.join("profiles/sample").join("../../contracts.json"))
    );
    // Names, versions, roles and the closure.
    let crates = selection["crates"].as_array().unwrap();
    assert_eq!(crates.len(), 2);
    assert_eq!(crates[0]["name"], "qcore-provider-beta");
    assert_eq!(crates[0]["version"], "0.2.0");
    assert_eq!(crates[0]["dependencies"], json!(["quirin-provider-alpha"]));
    assert_eq!(crates[1]["name"], "quirin-provider-alpha");
    assert_eq!(crates[1]["role"], "selected");
    // Module digests are the files' digests.
    for module in selection["modules"].as_array().unwrap() {
        let path = fixture
            .0
            .join("providers")
            .join(module["crate"].as_str().unwrap())
            .join("contracts")
            .join(module["module"].as_str().unwrap());
        assert_eq!(module["sha256"], sha256_hex(&fs::read(path).unwrap()));
    }
    assert_eq!(
        selection["load_path_sha256"],
        sha256_hex(&fs::read(fixture.0.join("providers/contract-load-path.json")).unwrap())
    );
    // The recorded manifest, placed where the selection places it, reloads
    // exactly the recorded contract texts.
    let text = sidecar["contract_manifest"].as_str().unwrap();
    assert_eq!(
        selection["lowered_manifest_sha256"],
        sha256_hex(text.as_bytes())
    );
    let load_path = PathBuf::from(selection["load_path"].as_str().unwrap());
    let reloaded = load_resource_contract_manifest_text(
        &load_path
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("provider-selection.json"),
        text,
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(reloaded.source_assumptions()).unwrap(),
        sidecar["contracts"]
    );
    // And the invocation record carries what the retired preparation wrapper
    // recorded: command, databases, jobs, profile, compiler and environment.
    let invocation = fixture.read("out/report.invocation.json");
    assert_eq!(invocation["tool"], "quirin-audit current");
    assert_eq!(invocation["argv"][1], "current");
    assert_eq!(
        invocation["compile_database_sha256"],
        sha256_hex(&fs::read(fixture.0.join("compile_commands.json")).unwrap())
    );
    assert_eq!(invocation["contracts"]["source"], "providers");
    assert_eq!(invocation["jobs"]["requested"], 1);
    assert!(invocation["clang"]["program"].is_string());
    assert!(invocation["source_revision"].as_str().unwrap().len() == 40);
}

#[test]
fn contracts_only_records_its_mode_and_changes_nothing_else() {
    let Some(clang) = clang() else { return };
    let fixture = Fixture::new(&clang, true);
    let selection = strings(&[
        "--load-path",
        "providers/contract-load-path.json",
        "--provider",
        "quirin-provider-alpha",
    ]);
    let mut args = fixture.base_args("configured");
    args.extend(selection.clone());
    succeeded(&fixture.run(&args));
    let only = fixture.config(
        "only.json",
        &json!({"schema_version": 1, "mode": "contracts-only"}),
    );
    let mut args = fixture.base_args("only");
    args.extend(selection);
    args.extend(strings(&["--config", &only]));
    let output = fixture.run(&args);
    succeeded(&output);
    assert!(String::from_utf8_lossy(&output.stdout).contains("evidence: contracts-only"));
    let mut report = fixture.read("only/report.json");
    let evidence = report["evidence"].clone();
    assert_eq!(evidence["mode"], "contracts-only");
    let modules = evidence["contract_modules"].as_array().unwrap();
    assert_eq!(modules.len(), 1);
    assert_eq!(modules[0]["sha256"], sha256_hex(HEAP.as_bytes()));
    assert_eq!(evidence["excluded"].as_array().unwrap().len(), 5);
    report.as_object_mut().unwrap().remove("evidence");
    assert_eq!(report, fixture.read("configured/report.json"));
    let mut sidecar = fixture.read("only/report.contracts.json");
    assert_eq!(sidecar["evidence"], evidence);
    sidecar.as_object_mut().unwrap().remove("evidence");
    assert_eq!(sidecar, fixture.read("configured/report.contracts.json"));
    // The flag and the configuration field are one setting.
    let mut args = fixture.base_args("flag");
    args.extend(strings(&[
        "--load-path",
        "providers/contract-load-path.json",
        "--provider",
        "quirin-provider-alpha",
        "--contracts-only",
    ]));
    succeeded(&fixture.run(&args));
    assert_eq!(
        fs::read(fixture.0.join("flag/report.json")).unwrap(),
        fs::read(fixture.0.join("only/report.json")).unwrap()
    );
}

/// Card W44-PROVIDER-RESOURCE-VIEW, the user's definition: contracts only
/// loads all contracts. With no selection and no explicit manifest a
/// contracts-only run selects every load-path crate whole (its resource
/// view), recorded as `"selection": "all-loaded"`; an explicit selection or
/// manifest narrows it; the full mode is unchanged.
#[test]
fn contracts_only_without_a_selection_loads_all_contracts() {
    let Some(clang) = clang() else { return };
    let fixture = Fixture::new(&clang, true);
    let load_path = strings(&["--load-path", "providers/contract-load-path.json"]);
    // The command line.
    let mut args = fixture.base_args("all");
    args.extend(load_path.clone());
    args.push("--contracts-only".into());
    succeeded(&fixture.run(&args));
    let sidecar = fixture.read("all/report.contracts.json");
    let selection = &sidecar["provider_selection"];
    assert_eq!(selection["selection"], "all-loaded");
    assert_eq!(selection["sources"], json!(["default"]));
    assert!(
        selection["profile_contract_manifest"]["role"]
            .as_str()
            .unwrap()
            .contains("loads all contracts")
    );
    assert_eq!(
        selection["entries"],
        json!(["qcore-provider-beta", "quirin-provider-alpha"])
    );
    assert_eq!(
        selection["modules"]
            .as_array()
            .unwrap()
            .iter()
            .map(|module| module["module"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["beta/other.qcore", "alpha/heap.qcore"]
    );
    let invocation = fixture.read("all/report.invocation.json");
    assert_eq!(invocation["contracts"]["selection"], "all-loaded");
    assert_eq!(invocation["contracts"]["source"], "providers");
    assert_eq!(invocation["mode"], "contracts-only");
    // The same crates named explicitly: the same report, byte for byte.
    let mut args = fixture.base_args("named");
    args.extend(load_path.clone());
    args.extend(strings(&[
        "--provider",
        "qcore-provider-beta",
        "--provider",
        "quirin-provider-alpha",
        "--contracts-only",
    ]));
    succeeded(&fixture.run(&args));
    assert_eq!(
        fs::read(fixture.0.join("all/report.json")).unwrap(),
        fs::read(fixture.0.join("named/report.json")).unwrap()
    );
    let named = fixture.read("named/report.contracts.json");
    assert_eq!(named["provider_selection"]["selection"], "explicit");
    assert_eq!(
        fixture.read("named/report.invocation.json")["contracts"]["selection"],
        "explicit"
    );
    assert_eq!(named["contract_manifest"], sidecar["contract_manifest"]);
    assert_eq!(named["contracts"], sidecar["contracts"]);
    // The configuration form is the same run.
    let config = fixture.config(
        "all.json",
        &json!({"schema_version": 1, "mode": "contracts-only",
                "load_path": "providers/contract-load-path.json"}),
    );
    let mut args = fixture.base_args("json");
    args.extend(strings(&["--config", &config]));
    succeeded(&fixture.run(&args));
    assert_eq!(
        fs::read(fixture.0.join("json/report.json")).unwrap(),
        fs::read(fixture.0.join("all/report.json")).unwrap()
    );
    assert_eq!(
        fixture.read("json/report.contracts.json")["provider_selection"]["selection"],
        "all-loaded"
    );
    // An explicit selection narrows it.
    let mut args = fixture.base_args("narrow");
    args.extend(load_path.clone());
    args.extend(strings(&[
        "--provider",
        "quirin-provider-alpha",
        "--contracts-only",
    ]));
    succeeded(&fixture.run(&args));
    let narrow = fixture.read("narrow/report.contracts.json");
    assert_eq!(narrow["provider_selection"]["selection"], "explicit");
    assert_eq!(narrow["contracts"].as_object().unwrap().len(), 1);
    // So does an explicit manifest.
    let mut args = fixture.base_args("manifest");
    args.extend(strings(&[
        "--contract-manifest",
        "contracts.json",
        "--contracts-only",
    ]));
    succeeded(&fixture.run(&args));
    assert!(fixture.read("manifest/report.contracts.json")["provider_selection"].is_null());
    assert!(fixture.read("manifest/report.invocation.json")["contracts"]["selection"].is_null());
    // Without the load path there is nothing to load all of; the profile's
    // default manifest is not a silent substitute.
    let mut args = fixture.base_args("bare");
    args.push("--contracts-only".into());
    refused(
        &fixture.run(&args),
        "loads every crate's resource view on the contract load path",
    );
    // The guard refuses the all-loaded set rather than narrowing it.
    let mut args = fixture.base_args("guarded");
    args.extend(load_path.clone());
    args.extend(strings(&["--contracts-only", "--quirin-providers-only"]));
    refused(
        &fixture.run(&args),
        "--quirin-providers-only refuses `qcore-provider-beta`",
    );
    // The full mode is unchanged: a load path without a selection refuses,
    // and with none the profile's manifest is read, with no selection.
    let mut args = fixture.base_args("full");
    args.extend(load_path);
    refused(&fixture.run(&args), "no provider is selected");
    succeeded(&fixture.run(&fixture.base_args("default")));
    assert!(fixture.read("default/report.contracts.json")["provider_selection"].is_null());
    assert!(fixture.read("default/report.invocation.json")["contracts"]["selection"].is_null());
    // The plain route: the same default, the same refusal without a load path.
    let plain = |name: &str, load_path: bool| {
        let mut config = json!({
            "schema_version": 1, "profile_directory": "profiles/sample", "checkout": "checkout",
            "compile_database": "compile_commands.json", "libraries": ["sample"],
            "report": format!("{name}/report.json"), "source_revision_policy": "checkout_head",
            "mode": "contracts-only",
        });
        if load_path {
            config["load_path"] = json!("providers/contract-load-path.json");
        }
        let config = fixture.config(&format!("{name}.json"), &config);
        Command::new(env!("CARGO_BIN_EXE_quirin-audit"))
            .arg(fixture.0.join(config))
            .output()
            .unwrap()
    };
    succeeded(&plain("plain", true));
    assert_eq!(
        fixture.read("plain/report.assumptions.json")["provider_selection"]["selection"],
        "all-loaded"
    );
    assert_eq!(
        fixture.read("plain/report.invocation.json")["contracts"]["selection"],
        "all-loaded"
    );
    refused(
        &plain("plain-bare", false),
        "loads every crate's resource view on the contract load path",
    );
}

#[test]
fn the_quirin_providers_only_guard_refuses_a_qcore_provider_closure() {
    let Some(clang) = clang() else { return };
    let fixture = Fixture::new(&clang, true);
    let mut args = fixture.base_args("guarded");
    args.extend(strings(&[
        "--load-path",
        "providers/contract-load-path.json",
        "--provider",
        "qcore-provider-beta",
        "--quirin-providers-only",
    ]));
    refused(
        &fixture.run(&args),
        "--quirin-providers-only refuses `qcore-provider-beta`",
    );
    let mut args = fixture.base_args("quirin");
    args.extend(strings(&[
        "--load-path",
        "providers/contract-load-path.json",
        "--provider",
        "quirin-provider-alpha",
        "--quirin-providers-only",
    ]));
    succeeded(&fixture.run(&args));
    assert_eq!(
        fixture.read("quirin/report.contracts.json")["provider_selection"]["quirin_providers_only"],
        true
    );
    // Without a selection the guard has nothing to guard.
    let mut args = fixture.base_args("none");
    args.push("--quirin-providers-only".into());
    refused(&fixture.run(&args), "guards a provider selection");
}

#[test]
fn a_substituted_compiler_refuses_with_the_replacement_instructions() {
    let Some(_) = clang() else { return };
    // An existing non-Clang executable reaches driver validation on every OS.
    let other = std::env::current_exe().unwrap();
    let fixture = Fixture::new(&other, true);
    let output = fixture.run(&fixture.base_args("out"));
    refused(&output, "audit requires the configured Clang executable");
    refused(&output, "quirin-audit current");
    assert!(!fixture.0.join("out/report.json").exists());
}

#[test]
fn a_current_configuration_naming_process_evidence_refuses() {
    let fixture = Fixture(std::env::temp_dir().join(format!(
        "qcore-current-cli-mixed-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )));
    fs::create_dir_all(&fixture.0).unwrap();
    for (name, key) in [
        ("packages.json", "packages"),
        ("source.json", "source_contracts"),
    ] {
        let config = fixture.config(
            name,
            &json!({"schema_version": 1, "mode": "contracts-only", key: ["x"]}),
        );
        refused(
            &fixture.run(&strings(&["--config", &config])),
            "admits no process packages or source-contract checks",
        );
    }
}

#[test]
fn the_plain_route_takes_providers_and_refuses_contracts_only_source_contracts() {
    // The refusal needs no Clang: it precedes every input.
    let fixture = Fixture(std::env::temp_dir().join(format!(
        "qcore-current-cli-plain-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )));
    fs::create_dir_all(&fixture.0).unwrap();
    let config = fixture.config(
        "audit.json",
        &json!({
            "schema_version": 1, "profile_directory": "profiles/sample", "checkout": "checkout",
            "compile_database": "compile_commands.json", "libraries": ["sample"],
            "report": "out/report.json", "mode": "contracts-only",
            "source_contracts": {"configuration": "sc.json", "source_rules": "rules.qcore",
                                 "libraries": [{"library": "sample", "source_root": "src"}]},
        }),
    );
    let output = Command::new(env!("CARGO_BIN_EXE_quirin-audit"))
        .arg(fixture.0.join(config))
        .output()
        .unwrap();
    refused(
        &output,
        "a contracts-only audit admits no source-contract checks",
    );

    let Some(clang) = clang() else { return };
    let fixture = Fixture::new(&clang, true);
    let config = fixture.config(
        "audit.json",
        &json!({
            "schema_version": 1, "profile_directory": "profiles/sample", "checkout": "checkout",
            "compile_database": "compile_commands.json", "libraries": ["sample"],
            "report": "out/report.json", "source_revision_policy": "checkout_head",
            "load_path": "providers/contract-load-path.json",
            "providers": ["quirin-provider-alpha"], "mode": "contracts-only",
        }),
    );
    let output = Command::new(env!("CARGO_BIN_EXE_quirin-audit"))
        .arg(fixture.0.join(config))
        .output()
        .unwrap();
    succeeded(&output);
    let report = fixture.read("out/report.json");
    assert_eq!(report["evidence"]["mode"], "contracts-only");
    let sidecar = fixture.read("out/report.assumptions.json");
    assert_eq!(
        sidecar["provider_selection"]["sources"],
        json!(["configuration"])
    );
    assert!(
        sidecar["provider_selection"]["profile_contract_manifest"]["role"]
            .as_str()
            .unwrap()
            .contains("supersedes")
    );
    assert_eq!(sidecar["evidence"], report["evidence"]);
}

/// The job count: `--jobs` over the configuration's `jobs` over
/// QCORE_AUDIT_JOBS; one job is the sequential loop, and every output but the
/// invocation record is byte-identical at any count, on both routes.
#[test]
fn jobs_resolve_by_precedence_and_never_change_an_output() {
    let Some(clang) = clang() else { return };
    let fixture = Fixture::new(&clang, true);
    let selection = strings(&[
        "--load-path",
        "providers/contract-load-path.json",
        "--provider",
        "quirin-provider-alpha",
    ]);
    let config = fixture.config(
        "jobs.json",
        &json!({"schema_version": 1, "jobs": 3, "mode": "contracts-only"}),
    );
    let run = |out: &str, flag: Option<&str>, environment: Option<&str>| {
        let mut args: Vec<String> = fixture
            .base_args(out)
            .into_iter()
            .filter(|argument| argument != "--jobs" && argument != "1")
            .collect();
        args.extend(selection.clone());
        args.extend(strings(&["--config", &config]));
        if let Some(flag) = flag {
            args.extend(strings(&["--jobs", flag]));
        }
        let mut command = Command::new(env!("CARGO_BIN_EXE_quirin-audit"));
        command.current_dir(&fixture.0).arg("current").args(&args);
        command.env_remove("QCORE_AUDIT_JOBS");
        if let Some(value) = environment {
            command.env("QCORE_AUDIT_JOBS", value);
        }
        succeeded(&command.output().unwrap());
        fixture.read(&format!("{out}/report.invocation.json"))["jobs"].clone()
    };
    assert_eq!(
        run("flag", Some("1"), Some("5")),
        json!({"requested": 1, "effective": 1, "source": "command_line"})
    );
    assert_eq!(
        run("config", None, Some("5")),
        json!({"requested": 3, "effective": 3, "source": "configuration"})
    );
    assert_eq!(
        run("wide", Some("6"), None),
        json!({"requested": 6, "effective": 6, "source": "command_line"})
    );
    for name in ["report.json", "report.contracts.json"] {
        let sequential = fs::read(fixture.0.join("flag").join(name)).unwrap();
        assert_eq!(
            fs::read(fixture.0.join("config").join(name)).unwrap(),
            sequential,
            "{name}"
        );
        assert_eq!(
            fs::read(fixture.0.join("wide").join(name)).unwrap(),
            sequential,
            "{name}"
        );
    }
    assert_eq!(
        fixture.read("wide/report.json")["units"]
            .as_array()
            .unwrap()
            .len(),
        6
    );
    // The environment applies when neither the flag nor the configuration does.
    let bare = fixture.config("bare.json", &json!({"schema_version": 1}));
    let mut args: Vec<String> = fixture
        .base_args("environment")
        .into_iter()
        .filter(|argument| argument != "--jobs" && argument != "1")
        .collect();
    args.extend(selection.clone());
    args.extend(strings(&["--config", &bare]));
    let output = Command::new(env!("CARGO_BIN_EXE_quirin-audit"))
        .current_dir(&fixture.0)
        .arg("current")
        .args(&args)
        .env("QCORE_AUDIT_JOBS", "2")
        .output()
        .unwrap();
    succeeded(&output);
    assert_eq!(
        fixture.read("environment/report.invocation.json")["jobs"],
        json!({"requested": 2, "effective": 2, "source": "environment"})
    );
    // A zero in the configuration is an error, not a fallback.
    let zero = fixture.config("zero.json", &json!({"schema_version": 1, "jobs": 0}));
    let mut args = fixture.base_args("zero");
    args.retain(|argument| argument != "--jobs" && argument != "1");
    args.extend(selection.clone());
    args.extend(strings(&["--config", &zero]));
    refused(&fixture.run(&args), "positive integer");

    // The plain route: the same fold, the same precedence.
    let plain = |out: &str, jobs_field: Option<usize>, flag: Option<&str>| {
        let mut value = json!({
            "schema_version": 1, "profile_directory": "profiles/sample", "checkout": "checkout",
            "compile_database": "compile_commands.json", "libraries": ["sample"],
            "report": format!("{out}/report.json"), "source_revision_policy": "checkout_head",
            "load_path": "providers/contract-load-path.json",
            "providers": ["quirin-provider-alpha"],
        });
        if let Some(jobs) = jobs_field {
            value["jobs"] = json!(jobs);
        }
        let config = fixture.config(&format!("{out}.json"), &value);
        let mut command = Command::new(env!("CARGO_BIN_EXE_quirin-audit"));
        command
            .current_dir(&fixture.0)
            .env_remove("QCORE_AUDIT_JOBS");
        if let Some(flag) = flag {
            command.args(["--jobs", flag]);
        }
        succeeded(&command.arg(fixture.0.join(config)).output().unwrap());
        fixture.read(&format!("{out}/report.invocation.json"))["jobs"].clone()
    };
    assert_eq!(
        plain("plain1", Some(4), Some("1")),
        json!({"requested": 1, "effective": 1, "source": "command_line"})
    );
    assert_eq!(
        plain("plain4", Some(4), None),
        json!({"requested": 4, "effective": 4, "source": "configuration"})
    );
    for name in ["report.json", "report.assumptions.json"] {
        assert_eq!(
            fs::read(fixture.0.join("plain1").join(name)).unwrap(),
            fs::read(fixture.0.join("plain4").join(name)).unwrap(),
            "{name}"
        );
    }
    assert_eq!(
        fixture.read("plain4/report.json")["units"]
            .as_array()
            .unwrap()
            .len(),
        6
    );
}

/// Card W45-FB-SYSTEM-SELECTION: `--selection FILE` replaces the profile's
/// `audit.json` groups with exact files, in the selection's order; a selected
/// file the database does not compile, or the checkout does not hold, stays in
/// the denominator as one environment `Unknown` unit; both sidecars name the
/// selection; and a unit keeps its configuration identity under either route.
#[test]
fn an_explicit_selection_keeps_every_selected_file_in_the_denominator() {
    let Some(clang) = clang() else { return };
    let fixture = Fixture::new(&clang, true);
    // A committed source the database never compiles.
    let checkout = fixture.0.join("checkout");
    fs::create_dir_all(checkout.join("plugins/p")).unwrap();
    fs::write(
        checkout.join("plugins/p/unbuilt.c"),
        "int unbuilt_one(void) { return 1; }\nint unbuilt_two(void) { return 2; }\n",
    )
    .unwrap();
    git(&checkout, &["add", "plugins"]);
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
            "Unbuilt plugin",
        ],
    );
    let selection = fixture.config(
        "selection.json",
        &json!({"schema_version": 1, "name": "slice", "groups": [{
            "library": "sample", "label": "sample-slice",
            "files": ["src/extra_3.c", "plugins/p/unbuilt.c", "src/unit.c", "src/missing.c"],
        }]}),
    );
    let mut args = fixture.base_args("selected");
    args.extend(strings(&["--selection", &selection]));
    let output = fixture.run(&args);
    succeeded(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.starts_with("slice "), "{stdout}");
    assert!(
        stdout.contains("3 source file(s), 4 configuration(s)"),
        "{stdout}"
    );
    let report = fixture.read("selected/report.json");
    let units = report["units"].as_array().unwrap();
    let files: Vec<&str> = units
        .iter()
        .map(|unit| unit["file"].as_str().unwrap())
        .collect();
    assert_eq!(
        files,
        [
            "src/extra_3.c",
            "plugins/p/unbuilt.c",
            "src/unit.c",
            "src/missing.c"
        ]
    );
    assert!(units.iter().all(|unit| unit["library"] == "sample-slice"));
    let unbuilt = &units[1];
    assert_eq!(unbuilt["status"], "Unknown");
    assert_eq!(unbuilt["reason_code"], "environment-unavailable");
    assert!(
        unbuilt["reason"]
            .as_str()
            .unwrap()
            .contains("no compilation database record")
    );
    assert_eq!(
        unbuilt["not_projected"],
        json!(["unbuilt_one", "unbuilt_two"])
    );
    assert_eq!(
        unbuilt["compile_configuration"]["compile_record"],
        Value::Null
    );
    let missing = &units[3];
    assert_eq!(missing["status"], "Unknown");
    assert_eq!(missing["reason_code"], "environment-unavailable");
    assert!(
        missing["reason"]
            .as_str()
            .unwrap()
            .contains("absent from the checkout")
    );
    assert_eq!(missing["source_functions"], json!([]));
    // Both sidecars name the selection by path, digest and contents.
    let digest = sha256_hex(&fs::read(fixture.0.join(&selection)).unwrap());
    for sidecar in [
        fixture.read("selected/report.invocation.json")["selection"].clone(),
        fixture.read("selected/report.contracts.json")["unit_selection"].clone(),
    ] {
        assert_eq!(sidecar["sha256"], json!(digest));
        assert_eq!(sidecar["name"], "slice");
        assert_eq!(sidecar["groups"][0]["files"][1], "plugins/p/unbuilt.c");
    }
    // The profile's own groups: no selection record, and the same unit
    // analysed with the same configuration identity under either route.
    succeeded(&fixture.run(&fixture.base_args("profile")));
    assert!(
        fixture.read("profile/report.invocation.json")["selection"].is_null()
            && fixture.read("profile/report.contracts.json")["unit_selection"].is_null()
    );
    let profile = fixture.read("profile/report.json");
    let profile_unit = profile["units"]
        .as_array()
        .unwrap()
        .iter()
        .find(|unit| unit["file"] == "src/unit.c")
        .unwrap()
        .clone();
    let mut selected_unit = units[2].clone();
    assert_eq!(profile_unit["library"], "sample");
    selected_unit["library"] = json!("sample");
    assert_eq!(selected_unit, profile_unit);
}

/// The selection is one setting like any other: the command line and the
/// configuration must agree, and a malformed selection or one naming a library
/// the profile does not declare refuses before any report.
#[test]
fn a_selection_is_declared_once_and_refuses_when_malformed() {
    let Some(clang) = clang() else { return };
    let fixture = Fixture::new(&clang, true);
    let one = fixture.config(
        "one.json",
        &json!({"schema_version": 1, "name": "one", "groups": [
            {"library": "sample", "files": ["src/unit.c"]}]}),
    );
    let two = fixture.config(
        "two.json",
        &json!({"schema_version": 1, "name": "two", "groups": [
            {"library": "sample", "files": ["src/extra_0.c"]}]}),
    );
    // Configuration only, relative to the configuration file.
    let config = fixture.config(
        "current.json",
        &json!({"schema_version": 1, "selection": "one.json"}),
    );
    let mut args = fixture.base_args("config");
    args.extend(strings(&["--config", &config]));
    succeeded(&fixture.run(&args));
    assert_eq!(
        fixture.read("config/report.json")["units"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    // Both forms, the same file: accepted; different files: refused.
    let mut args = fixture.base_args("both");
    args.extend(strings(&["--selection", &one, "--config", &config]));
    succeeded(&fixture.run(&args));
    let mut args = fixture.base_args("conflict");
    args.extend(strings(&["--selection", &two, "--config", &config]));
    refused(&fixture.run(&args), "`selection` conflicts");
    // A library the profile does not declare.
    let foreign = fixture.config(
        "foreign.json",
        &json!({"schema_version": 1, "name": "foreign", "groups": [
            {"library": "elsewhere", "files": ["src/unit.c"]}]}),
    );
    let mut args = fixture.base_args("foreign");
    args.extend(strings(&["--selection", &foreign]));
    refused(&fixture.run(&args), "does not declare");
    assert!(!fixture.0.join("foreign").exists());
    // A path outside the checkout.
    let escape = fixture.config(
        "escape.json",
        &json!({"schema_version": 1, "name": "escape", "groups": [
            {"library": "sample", "files": ["../outside.c"]}]}),
    );
    let mut args = fixture.base_args("escape");
    args.extend(strings(&["--selection", &escape]));
    refused(&fixture.run(&args), "plain relative path");
    assert!(!fixture.0.join("escape").exists());
}
