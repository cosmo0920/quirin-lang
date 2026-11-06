//! Card W44-QUIRIN-AUDIT-PROVIDERS: contract selection by provider crate.
//!
//! Fixture load paths exercise every refusal and the dependency closure; the
//! repository's own load path shows that the selection equal to
//! `cycle19-resource-contracts.json` lowers to that manifest byte for byte and
//! elaborates exactly its environment, module for module.
//!
//! Card W44-PROVIDER-RESOURCE-VIEW: a whole-crate entry selects the crate's
//! declared resource view. Fixture views exercise the view's refusals; the
//! repository's views are the loader's own (the probe), every one of the
//! eleven crates selects whole, and all eleven together — which is also the
//! contracts-only all-loaded default — elaborate exactly the Cycle 19
//! manifest's 54 modules (50 before the combined W46 contract-gap cards).

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use qcore_audit::providers::{
    ContractInput, SelectionKind, SelectionSource, agree, probe_resource_views, resolve_all_loaded,
    resolve_providers,
};
use qcore_contract::contract_loader::{inclusion_provenance, load_resource_contract_manifest};
use qcore_schemas::sha256_hex;
use serde_json::{Value, json};

#[path = "../../../tests/support/contract_locks.rs"]
mod contract_locks;

static NEXT: AtomicUsize = AtomicUsize::new(0);

const HEAP: &str = "module contract;\n\
extern fn alloc() -> ptr<mut, u8>\n\
contract {\n\
    requires resource_contract_v1;\n\
    ensures normal resource_return_owned(nonnull);\n\
};\n\
extern fn free(ptr: ptr<mut, u8>) -> unit\n\
contract {\n\
    requires resource_contract_v1;\n\
    ensures normal resource_released(ptr, nullable);\n\
    ensures normal resource_return_none();\n\
};\n";

const EXTRA: &str = "module contract;\n\
extern fn dispose(p: ptr<mut, u8>) -> unit\n\
contract {\n\
    requires resource_contract_v1;\n\
    ensures normal resource_released(p, nonnull);\n\
    ensures normal resource_return_none();\n\
};\n";

const WRAP: &str = "module contract;\n\
#include \"quirin-provider-alpha:alpha/heap.qcore\";\n\
extern fn beta_free(p: ptr<mut, u8>) -> unit contract inherit alpha::free;\n";

/// A module written for another checker: the resource loader refuses it.
const POLICY: &str = "module contract;\n\
extern fn policy_step(p: ptr<mut, u8>) -> unit\n\
contract {\n\
    requires lifecycle_policy_fact;\n\
};\n";

const OTHER: &str = "module contract;\n\
extern fn gamma_close(p: ptr<mut, u8>) -> unit\n\
contract {\n\
    requires resource_contract_v1;\n\
    ensures normal resource_released(p, nonnull);\n\
    ensures normal resource_return_none();\n\
};\n";

/// One fixture crate: name, dependencies, (module, text) pairs, published.
struct Crate {
    name: &'static str,
    dependencies: &'static [&'static str],
    modules: Vec<(&'static str, &'static str)>,
    published: &'static [&'static str],
}

fn namespace(name: &str) -> String {
    name.strip_prefix("quirin-provider-")
        .or_else(|| name.strip_prefix("qcore-provider-"))
        .unwrap_or(name)
        .replace('-', "_")
}

struct Fixture(PathBuf);

impl Fixture {
    fn new(crates: &[Crate]) -> Self {
        let root = std::env::temp_dir().join(format!(
            "qcore-provider-selection-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let providers = root.join("providers");
        let mut records = Vec::new();
        for krate in crates {
            let contracts = format!("{}/contracts", krate.name);
            let mut modules = Vec::new();
            let mut published = serde_json::Map::new();
            for (module, text) in &krate.modules {
                let path = providers.join(&contracts).join(module);
                fs::create_dir_all(path.parent().unwrap()).unwrap();
                fs::write(&path, text).unwrap();
                modules.push((*module).to_owned());
                if krate.published.contains(module) {
                    published.insert((*module).to_owned(), json!(sha256_hex(text.as_bytes())));
                }
            }
            if krate.modules.is_empty() {
                fs::create_dir_all(providers.join(&contracts)).unwrap();
            }
            modules.sort();
            // Every fixture module but `POLICY` is a resource module, so the
            // declared view is what `contracts.py sync` would write.
            let view: Vec<&str> = krate
                .modules
                .iter()
                .filter(|(_, text)| *text != POLICY)
                .map(|(module, _)| *module)
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect();
            let mut dependencies: Vec<&str> = krate.dependencies.to_vec();
            dependencies.sort_unstable();
            records.push(json!({
                "name": krate.name, "namespace": namespace(krate.name), "version": "0.3.0",
                "contracts": contracts, "dependencies": dependencies, "modules": modules,
                "published": published, "resource_view": view,
            }));
        }
        fs::create_dir_all(&providers).unwrap();
        fs::write(
            providers.join("contract-load-path.json"),
            serde_json::to_string_pretty(&json!({"schema_version": 1, "crates": records})).unwrap(),
        )
        .unwrap();
        Self(root)
    }

    fn standard() -> Self {
        Self::new(&[
            Crate {
                name: "quirin-provider-alpha",
                dependencies: &[],
                modules: vec![("alpha/heap.qcore", HEAP), ("alpha/extra.qcore", EXTRA)],
                published: &["alpha/heap.qcore"],
            },
            Crate {
                name: "qcore-provider-beta",
                dependencies: &["quirin-provider-alpha"],
                modules: vec![("beta/wrap.qcore", WRAP)],
                published: &[],
            },
            Crate {
                name: "quirin-provider-gamma",
                dependencies: &[],
                modules: vec![("gamma/other.qcore", OTHER)],
                published: &[],
            },
            Crate {
                name: "quirin-provider-delta",
                dependencies: &["qcore-provider-beta"],
                modules: vec![("delta/other.qcore", OTHER.replace("gamma", "delta").leak())],
                published: &[],
            },
        ])
    }

    fn load_path(&self) -> PathBuf {
        self.0.join("providers/contract-load-path.json")
    }

    /// Rewrite one crate's record in the load path.
    fn edit_record(&self, name: &str, edit: impl FnOnce(&mut Value)) {
        let mut manifest: Value =
            serde_json::from_slice(&fs::read(self.load_path()).unwrap()).unwrap();
        let record = manifest["crates"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|record| record["name"] == name)
            .unwrap();
        edit(record);
        fs::write(
            self.load_path(),
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();
    }

    fn resolve(
        &self,
        entries: &[&str],
    ) -> Result<qcore_audit::providers::ResolvedProviders, String> {
        resolve_providers(
            &self.load_path(),
            &entries.iter().map(|e| (*e).to_owned()).collect::<Vec<_>>(),
            false,
            SelectionSource::CommandLine,
        )
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn a_whole_crate_selects_its_modules_in_load_path_order_and_lowers_to_a_manifest() {
    let fixture = Fixture::standard();
    let resolved = fixture
        .resolve(&["quirin-provider-alpha", "qcore-provider-beta"])
        .unwrap();
    let manifest: Value = serde_json::from_str(resolved.manifest_text()).unwrap();
    assert_eq!(
        manifest,
        json!({
            "schema_version": 2,
            "load_path": "providers/contract-load-path.json",
            "modules": [
                "providers/quirin-provider-alpha/contracts/alpha/extra.qcore",
                "providers/quirin-provider-alpha/contracts/alpha/heap.qcore",
                "qcore-provider-beta:beta/wrap.qcore",
            ],
        })
    );
    assert!(resolved.manifest_text().ends_with("}\n"));
    assert_eq!(
        resolved.manifest_path(),
        fixture.0.join("provider-selection.json")
    );
    // The lowered text loads through the unchanged manifest loader, with the
    // inherited contract elaborated from its dependency's published parent.
    let contracts = ContractInput::providers(&resolved, None).load().unwrap();
    for name in ["alloc", "free", "dispose", "beta_free"] {
        assert!(contracts.contract(name).is_some(), "{name}");
    }
    assert!(inclusion_provenance(&contracts).is_some());
}

#[test]
fn the_dependency_closure_is_resolved_and_recorded_but_not_selected() {
    let fixture = Fixture::standard();
    let resolved = fixture.resolve(&["quirin-provider-delta"]).unwrap();
    assert_eq!(
        resolved.closure_names().collect::<Vec<_>>(),
        [
            "qcore-provider-beta",
            "quirin-provider-alpha",
            "quirin-provider-delta"
        ]
    );
    // Only delta's own module is selected; beta and alpha are resolution scope.
    assert_eq!(resolved.modules().len(), 1);
    let record = resolved.record(Some(Path::new("/profile/contracts.json")));
    let crates = record["crates"].as_array().unwrap();
    let role = |name: &str| {
        crates
            .iter()
            .find(|krate| krate["name"] == name)
            .map(|krate| krate["role"].clone())
            .unwrap()
    };
    assert_eq!(role("quirin-provider-delta"), "selected");
    assert_eq!(role("qcore-provider-beta"), "dependency");
    assert_eq!(role("quirin-provider-alpha"), "dependency");
    let alpha = crates
        .iter()
        .find(|krate| krate["name"] == "quirin-provider-alpha")
        .unwrap();
    assert_eq!(alpha["version"], "0.3.0");
    assert_eq!(alpha["namespace"], "alpha");
    assert_eq!(
        alpha["published"]["alpha/heap.qcore"],
        sha256_hex(HEAP.as_bytes())
    );
    assert_eq!(alpha["selected_modules"], json!({}));
    assert_eq!(
        record["profile_contract_manifest"]["path"],
        "/profile/contracts.json"
    );
    assert_eq!(
        record["load_path_sha256"],
        sha256_hex(&fs::read(fixture.load_path()).unwrap())
    );
}

#[test]
fn unknown_providers_missing_modules_duplicates_and_empty_selections_refuse() {
    let fixture = Fixture::standard();
    let refused = |entries: &[&str], expected: &str| {
        let error = fixture.resolve(entries).unwrap_err();
        assert!(error.contains(expected), "{entries:?}: {error}");
    };
    refused(
        &["quirin-provider-zeta"],
        "unknown provider `quirin-provider-zeta`",
    );
    refused(&["serde"], "is not a provider");
    refused(
        &["quirin-provider-alpha:alpha/absent.qcore"],
        "owns no contract module `alpha/absent.qcore`",
    );
    refused(
        &[
            "quirin-provider-alpha",
            "quirin-provider-alpha:alpha/heap.qcore",
        ],
        "selected twice",
    );
    refused(&[], "provider selection is empty");
}

fn with_policy() -> Fixture {
    Fixture::new(&[
        Crate {
            name: "quirin-provider-alpha",
            dependencies: &[],
            modules: vec![("alpha/heap.qcore", HEAP), ("alpha/policy.qcore", POLICY)],
            published: &[],
        },
        Crate {
            name: "quirin-provider-policy",
            dependencies: &[],
            modules: vec![("policy/only.qcore", POLICY)],
            published: &[],
        },
    ])
}

#[test]
fn a_whole_crate_selects_its_resource_view_not_every_owned_module() {
    let fixture = with_policy();
    let resolved = fixture.resolve(&["quirin-provider-alpha"]).unwrap();
    assert_eq!(
        resolved
            .modules()
            .iter()
            .map(|module| module.module.as_str())
            .collect::<Vec<_>>(),
        ["alpha/heap.qcore"]
    );
    ContractInput::providers(&resolved, None).load().unwrap();
    let record = resolved.record(None);
    assert_eq!(record["selection"], "explicit");
    assert_eq!(
        record["crates"][0]["resource_view"],
        json!(["alpha/heap.qcore"])
    );
    // NAME:MODULE still names any owned module; the loader then refuses one
    // outside the view.
    let explicit = fixture
        .resolve(&["quirin-provider-alpha:alpha/policy.qcore"])
        .unwrap();
    assert!(ContractInput::providers(&explicit, None).load().is_err());
    // The probe is the loader's view: it is what the fixture declares.
    let probe = probe_resource_views(&fixture.load_path()).unwrap();
    assert_eq!(
        probe["crates"]["quirin-provider-alpha"]["resource_view"],
        json!(["alpha/heap.qcore"])
    );
    assert!(
        probe["crates"]["quirin-provider-alpha"]["refused"]
            .get("alpha/policy.qcore")
            .is_some()
    );
    assert_eq!(
        probe["crates"]["quirin-provider-policy"]["resource_view"],
        json!([])
    );
}

#[test]
fn an_empty_view_contributes_nothing_and_a_selection_of_nothing_refuses() {
    let fixture = with_policy();
    let error = fixture.resolve(&["quirin-provider-policy"]).unwrap_err();
    assert!(
        error.contains("resolves to no resource contract module"),
        "{error}"
    );
    let both = fixture
        .resolve(&["quirin-provider-alpha", "quirin-provider-policy"])
        .unwrap();
    assert_eq!(both.modules().len(), 1);
    let record = both.record(None);
    let policy = record["crates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|krate| krate["name"] == "quirin-provider-policy")
        .unwrap();
    assert_eq!(policy["role"], "selected");
    assert_eq!(policy["selected_modules"], json!({}));
}

#[test]
fn a_crate_without_a_declared_view_cannot_be_selected_whole() {
    let fixture = Fixture::standard();
    fixture.edit_record("quirin-provider-alpha", |record| {
        record.as_object_mut().unwrap().remove("resource_view");
    });
    let error = fixture.resolve(&["quirin-provider-alpha"]).unwrap_err();
    assert!(error.contains("declares no resource view"), "{error}");
    // Its modules can still be named one by one.
    fixture
        .resolve(&["quirin-provider-alpha:alpha/heap.qcore"])
        .unwrap();
}

#[test]
fn a_view_that_is_unsorted_or_names_an_unowned_module_refuses_the_load_path() {
    for view in [
        json!(["alpha/heap.qcore", "alpha/extra.qcore"]),
        json!(["alpha/heap.qcore", "alpha/heap.qcore"]),
        json!(["alpha/absent.qcore"]),
    ] {
        let fixture = Fixture::standard();
        fixture.edit_record("quirin-provider-alpha", |record| {
            record["resource_view"] = view.clone();
        });
        let error = fixture.resolve(&["quirin-provider-gamma"]).unwrap_err();
        assert!(
            error.contains("QI014") && error.contains("resource view"),
            "{view}: {error}"
        );
    }
}

#[test]
fn all_loaded_selects_every_crate_whole_in_load_path_order() {
    let fixture = Fixture::standard();
    let all = resolve_all_loaded(&fixture.load_path(), false).unwrap();
    assert_eq!(all.kind(), SelectionKind::AllLoaded);
    let named = fixture
        .resolve(&[
            "qcore-provider-beta",
            "quirin-provider-alpha",
            "quirin-provider-delta",
            "quirin-provider-gamma",
        ])
        .unwrap();
    assert_eq!(all.manifest_text(), named.manifest_text());
    let record = all.record(None);
    assert_eq!(record["selection"], "all-loaded");
    assert_eq!(record["sources"], json!(["default"]));
    // The guard refuses rather than narrows it: beta is a qcore-provider crate.
    let error = resolve_all_loaded(&fixture.load_path(), true).unwrap_err();
    assert!(error.contains("qcore-provider-beta"), "{error}");
}

#[test]
fn a_namespace_conflict_refuses_the_whole_load_path() {
    let fixture = Fixture::new(&[
        Crate {
            name: "quirin-provider-alpha",
            dependencies: &[],
            modules: vec![("alpha/heap.qcore", HEAP)],
            published: &[],
        },
        Crate {
            name: "qcore-provider-alpha",
            dependencies: &[],
            modules: vec![("alpha/extra.qcore", EXTRA)],
            published: &[],
        },
    ]);
    let error = fixture.resolve(&["quirin-provider-alpha"]).unwrap_err();
    assert!(
        error.contains("QI017") && error.contains("namespace"),
        "{error}"
    );
}

#[test]
fn a_dependency_cycle_refuses_the_whole_load_path() {
    let fixture = Fixture::new(&[
        Crate {
            name: "quirin-provider-alpha",
            dependencies: &["quirin-provider-gamma"],
            modules: vec![("alpha/heap.qcore", HEAP)],
            published: &[],
        },
        Crate {
            name: "quirin-provider-gamma",
            dependencies: &["quirin-provider-alpha"],
            modules: vec![("gamma/other.qcore", OTHER)],
            published: &[],
        },
    ]);
    let error = fixture.resolve(&["quirin-provider-gamma"]).unwrap_err();
    assert!(error.contains("dependency cycle"), "{error}");
}

#[test]
fn a_listed_module_missing_on_disk_refuses_the_whole_load_path() {
    let fixture = Fixture::standard();
    fs::remove_file(
        fixture
            .0
            .join("providers/quirin-provider-gamma/contracts/gamma/other.qcore"),
    )
    .unwrap();
    let error = fixture.resolve(&["quirin-provider-alpha"]).unwrap_err();
    assert!(error.contains("absent"), "{error}");
}

#[test]
fn the_quirin_only_guard_refuses_any_qcore_provider_in_the_closure() {
    let fixture = Fixture::standard();
    let guarded = |entries: &[&str]| {
        resolve_providers(
            &fixture.load_path(),
            &entries.iter().map(|e| (*e).to_owned()).collect::<Vec<_>>(),
            true,
            SelectionSource::Configuration,
        )
    };
    assert!(
        guarded(&["qcore-provider-beta"])
            .unwrap_err()
            .contains("qcore-provider-beta")
    );
    // delta is a quirin-provider crate, but it depends on qcore-provider-beta.
    assert!(
        guarded(&["quirin-provider-delta"])
            .unwrap_err()
            .contains("qcore-provider-beta")
    );
    let admitted = guarded(&["quirin-provider-alpha", "quirin-provider-gamma"]).unwrap();
    assert_eq!(admitted.record(None)["quirin_providers_only"], true);
}

#[test]
fn both_forms_agree_only_on_the_same_selection() {
    let fixture = Fixture::standard();
    let entries = |values: &[&str]| values.iter().map(|e| (*e).to_owned()).collect::<Vec<_>>();
    let resolve = |values: &[&str], source| {
        resolve_providers(&fixture.load_path(), &entries(values), false, source).unwrap()
    };
    // Different spellings of one module list are one run.
    let merged = agree(
        resolve(&["quirin-provider-alpha"], SelectionSource::CommandLine),
        resolve(
            &[
                "quirin-provider-alpha:alpha/extra.qcore",
                "quirin-provider-alpha:alpha/heap.qcore",
            ],
            SelectionSource::Configuration,
        ),
    )
    .unwrap();
    assert_eq!(
        merged.record(None)["sources"],
        json!(["command_line", "configuration"])
    );
    // A different set, or the same set in another order, conflicts.
    for other in [
        &["quirin-provider-gamma"][..],
        &[
            "quirin-provider-alpha:alpha/heap.qcore",
            "quirin-provider-alpha:alpha/extra.qcore",
        ][..],
    ] {
        let error = agree(
            resolve(&["quirin-provider-alpha"], SelectionSource::CommandLine),
            resolve(other, SelectionSource::Configuration),
        )
        .unwrap_err();
        assert!(error.contains("conflict"), "{error}");
    }
}

#[test]
fn a_selection_loads_exactly_what_the_equal_hand_written_manifest_loads() {
    let fixture = Fixture::standard();
    let resolved = fixture
        .resolve(&["qcore-provider-beta", "quirin-provider-gamma"])
        .unwrap();
    let written = fixture.0.join("hand-written.json");
    fs::write(&written, resolved.manifest_text()).unwrap();
    let from_file = load_resource_contract_manifest(&written).unwrap();
    let from_selection = ContractInput::providers(&resolved, None).load().unwrap();
    assert_eq!(
        from_selection.source_assumptions(),
        from_file.source_assumptions()
    );
    assert_eq!(
        from_selection.inclusion_evidence(),
        from_file.inclusion_evidence()
    );
    assert_eq!(from_selection.functions(), from_file.functions());
}

/// The repository's Cycle 19 manifest as a provider selection: one
/// `crate:module` entry per manifest entry, in manifest order.
fn cycle19_selection(repository: &Path) -> Vec<String> {
    let manifest: Value = serde_json::from_slice(
        &fs::read(repository.join("cycle19-resource-contracts.json")).unwrap(),
    )
    .unwrap();
    let load_path: Value = serde_json::from_slice(
        &fs::read(repository.join("providers/contract-load-path.json")).unwrap(),
    )
    .unwrap();
    manifest["modules"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| {
            let entry = entry.as_str().unwrap();
            if entry.contains(':') {
                return entry.to_owned();
            }
            let owners = load_path["crates"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(|krate| {
                    let root = format!("providers/{}/", krate["contracts"].as_str().unwrap());
                    entry
                        .strip_prefix(&root)
                        .map(|module| format!("{}:{module}", krate["name"].as_str().unwrap()))
                })
                .collect::<Vec<_>>();
            assert_eq!(owners.len(), 1, "{entry}");
            owners[0].clone()
        })
        .collect()
}

#[test]
fn the_cycle19_provider_selection_is_the_cycle19_manifest_module_for_module() {
    let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let entries = cycle19_selection(&repository);
    // Card CONTRACT-COUNT-PINS-FROM-LOCKS: the selected module count is the
    // providers' resource views as their inventory locks state them
    // (`tests/support/contract_locks.rs`); the former literal and its
    // per-card history are in this file at base 25508ccaf.
    assert_eq!(entries.len(), locked_resource_view_modules(&repository));
    let resolved = resolve_providers(
        &repository.join("providers/contract-load-path.json"),
        &entries,
        false,
        SelectionSource::Configuration,
    )
    .unwrap();
    let manifest_path = repository.join("cycle19-resource-contracts.json");
    // The lowered manifest is the committed manifest, byte for byte, sitting at
    // the same directory, so every digest derived from it is unchanged.
    assert_eq!(
        resolved.manifest_text().replace("\r\n", "\n"),
        fs::read_to_string(&manifest_path)
            .unwrap()
            .replace("\r\n", "\n")
    );
    assert_eq!(resolved.manifest_path().parent(), manifest_path.parent());
    let from_manifest = load_resource_contract_manifest(&manifest_path).unwrap();
    let from_selection = ContractInput::providers(&resolved, None).load().unwrap();
    // Card CONTRACT-COUNT-PINS-FROM-LOCKS: the selected module count is the
    // providers' resource views as their inventory locks state them
    // (`tests/support/contract_locks.rs`); the former literal and its
    // per-card history are in this file at base 25508ccaf.
    assert_eq!(
        from_selection.source_assumptions().len(),
        locked_resource_view_modules(&repository)
    );
    for (module, text) in from_manifest.source_assumptions() {
        assert_eq!(
            from_selection.source_assumptions().get(module),
            Some(text),
            "{module}"
        );
    }
    assert_eq!(
        from_selection.source_assumptions(),
        from_manifest.source_assumptions()
    );
    assert_eq!(
        from_selection.inclusion_evidence(),
        from_manifest.inclusion_evidence()
    );
    assert_eq!(from_selection.functions(), from_manifest.functions());
    // Its closure: the ten selected crates and the shared bundle.
    assert_eq!(
        resolved.closure_names().collect::<Vec<_>>(),
        [
            "qcore-provider-chunkio",
            "qcore-provider-cmetrics",
            "qcore-provider-ctraces",
            "qcore-provider-libevent",
            "qcore-provider-librdkafka",
            "quirin-provider-cfl",
            "quirin-provider-fluent-bit",
            "quirin-provider-libc",
            "quirin-provider-mpack",
            "quirin-provider-msgpack",
            "quirin-provider-shared",
        ]
    );
    // And the guard refuses it: five of its crates are qcore-provider crates.
    assert!(
        resolve_providers(
            &repository.join("providers/contract-load-path.json"),
            &entries,
            true,
            SelectionSource::Configuration,
        )
        .is_err()
    );
}

/// Σ of the providers' resource views, as their inventory locks state them.
fn locked_resource_view_modules(repository: &Path) -> usize {
    contract_locks::lock_totals(repository).resource_view_modules
}

fn repository() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The eleven crates on the repository's load path, in load-path order.
fn real_crates(repository: &Path) -> Vec<String> {
    let load_path: Value = serde_json::from_slice(
        &fs::read(repository.join("providers/contract-load-path.json")).unwrap(),
    )
    .unwrap();
    load_path["crates"]
        .as_array()
        .unwrap()
        .iter()
        .map(|krate| krate["name"].as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn every_real_crate_selected_whole_is_its_loader_view() {
    let repository = repository();
    let load_path = repository.join("providers/contract-load-path.json");
    let crates = real_crates(&repository);
    assert_eq!(crates.len(), 11);
    // The declared views are the loader's own.
    let probe = probe_resource_views(&load_path).unwrap();
    let declared: Value = serde_json::from_slice(&fs::read(&load_path).unwrap()).unwrap();
    let mut total = 0;
    for (name, record) in crates.iter().zip(declared["crates"].as_array().unwrap()) {
        assert_eq!(
            record["resource_view"], probe["crates"][name]["resource_view"],
            "{name}"
        );
        let view: Vec<&str> = record["resource_view"]
            .as_array()
            .unwrap()
            .iter()
            .map(|module| module.as_str().unwrap())
            .collect();
        total += view.len();
        let resolved = resolve_providers(
            &load_path,
            std::slice::from_ref(name),
            false,
            SelectionSource::CommandLine,
        );
        if view.is_empty() {
            // quirin-provider-shared owns only views for other checkers.
            assert_eq!(name, "quirin-provider-shared");
            let error = resolved.unwrap_err();
            assert!(
                error.contains("resolves to no resource contract module"),
                "{error}"
            );
            continue;
        }
        let resolved = resolved.unwrap();
        assert_eq!(
            resolved
                .modules()
                .iter()
                .map(|module| module.module.as_str())
                .collect::<Vec<_>>(),
            view,
            "{name}"
        );
        let contracts = ContractInput::providers(&resolved, None).load().unwrap();
        assert_eq!(contracts.source_assumptions().len(), view.len(), "{name}");
    }
    // Card CONTRACT-COUNT-PINS-FROM-LOCKS: the selected module count is the
    // providers' resource views as their inventory locks state them
    // (`tests/support/contract_locks.rs`); the former literal and its
    // per-card history are in this file at base 25508ccaf.
    assert_eq!(total, locked_resource_view_modules(&repository));
}

#[test]
fn all_eleven_crates_whole_and_all_loaded_are_the_cycle19_manifest_environment() {
    let repository = repository();
    let load_path = repository.join("providers/contract-load-path.json");
    let crates = real_crates(&repository);
    let whole =
        resolve_providers(&load_path, &crates, false, SelectionSource::CommandLine).unwrap();
    let all = resolve_all_loaded(&load_path, false).unwrap();
    assert_eq!(whole.manifest_text(), all.manifest_text());
    assert_eq!(all.record(None)["selection"], "all-loaded");
    let manifest_path = repository.join("cycle19-resource-contracts.json");
    // The same 54 entries, spelled exactly as the manifest spells them; only
    // their order differs (load-path crate order, then each sorted view).
    let entries = |text: &str| {
        let value: Value = serde_json::from_str(text).unwrap();
        let mut entries: Vec<String> = value["modules"]
            .as_array()
            .unwrap()
            .iter()
            .map(|entry| entry.as_str().unwrap().to_owned())
            .collect();
        let load_path = value["load_path"].clone();
        entries.sort();
        (load_path, entries)
    };
    let manifest_text = fs::read_to_string(&manifest_path).unwrap();
    assert_eq!(entries(all.manifest_text()), entries(&manifest_text));
    // Card CONTRACT-COUNT-PINS-FROM-LOCKS: the selected module count is the
    // providers' resource views as their inventory locks state them
    // (`tests/support/contract_locks.rs`); the former literal and its
    // per-card history are in this file at base 25508ccaf.
    assert_eq!(
        entries(&manifest_text).1.len(),
        locked_resource_view_modules(&repository)
    );
    assert_eq!(all.manifest_path().parent(), manifest_path.parent());
    // Elaborated byte for byte as the manifest: every module key and text,
    // the inclusion evidence and the loaded function table.
    let from_manifest = load_resource_contract_manifest(&manifest_path).unwrap();
    for resolved in [&whole, &all] {
        let from_selection = ContractInput::providers(resolved, None).load().unwrap();
        assert_eq!(
            from_selection.source_assumptions(),
            from_manifest.source_assumptions()
        );
        assert_eq!(
            from_selection.inclusion_evidence(),
            from_manifest.inclusion_evidence()
        );
        assert_eq!(from_selection.functions(), from_manifest.functions());
    }
    // Five of the eleven are qcore-provider crates: the guard refuses the
    // all-loaded set rather than narrowing it.
    assert!(resolve_all_loaded(&load_path, true).is_err());
}
