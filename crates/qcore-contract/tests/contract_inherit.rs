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

//! Card CONTRACT-INHERIT: contract inclusion and inheritance against fixture
//! contract crates. Positive fixtures show an inherited environment loading
//! exactly like its hand-written equivalent, with its provenance; one negative
//! control per refusal code shows each refusal is reachable and fails closed.
//!
//! No real provider crate is read: the fixtures under
//! `tests/fixtures/contract_inherit/` stand in for them.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use qcore_contract::contract_inclusion::{
    ContractLoadPath, PublishedContractCrate, derive_contract_namespace,
};
use qcore_contract::contract_loader::{
    PublishedModuleRef, check_source_assumptions, declared_abi_signatures_of,
    declared_literal_argument_premises, declared_variadic_data_tails, literal_argument_premises,
    load_published_resource_contracts, load_resource_contract_sources, variadic_data_tails,
};
use qcore_schemas::contract::ResourceContracts;
use qcore_schemas::sha256_hex;

const LIBC: &str = "quirin-provider-libc";
const CFL: &str = "quirin-provider-cfl";
const FLB: &str = "quirin-provider-fluent-bit";
const APP: &str = "quirin-provider-app";

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/contract_inherit")
}

fn read_crate_modules(name: &str) -> BTreeMap<String, String> {
    let root = fixtures().join(name).join("contracts");
    let mut modules = BTreeMap::new();
    let mut pending = vec![root.clone()];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
            } else {
                let logical = path
                    .strip_prefix(&root)
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .replace('\\', "/");
                // Normalise a Windows checkout's CRLF so digests are stable.
                let text = std::fs::read_to_string(&path)
                    .unwrap()
                    .replace("\r\n", "\n");
                modules.insert(logical, text);
            }
        }
    }
    modules
}

fn krate(
    name: &str,
    dependencies: &[&str],
    modules: BTreeMap<String, String>,
    published: &[&str],
) -> PublishedContractCrate {
    PublishedContractCrate {
        name: name.to_owned(),
        namespace: derive_contract_namespace(name).expect("fixture crate names derive"),
        version: "0.1.0".to_owned(),
        dependencies: dependencies.iter().map(|d| (*d).to_owned()).collect(),
        modules,
        published: published.iter().map(|m| (*m).to_owned()).collect(),
    }
}

/// The fixture load path: libc and cfl have no dependencies, fluent-bit
/// depends on both, app depends on fluent-bit only.
fn fixture_crates() -> Vec<PublishedContractCrate> {
    vec![
        krate(
            LIBC,
            &[],
            read_crate_modules(LIBC),
            &["libc/string.qv", "libc/heap.qv"],
        ),
        krate(CFL, &[], read_crate_modules(CFL), &["cfl/table.qv"]),
        krate(
            FLB,
            &[LIBC, CFL],
            read_crate_modules(FLB),
            &["fluent_bit/wrappers.qv"],
        ),
        krate(APP, &[FLB], read_crate_modules(APP), &[]),
    ]
}

fn fixture_load_path() -> ContractLoadPath {
    ContractLoadPath::new(fixture_crates()).unwrap()
}

fn load(
    load_path: &ContractLoadPath,
    selection: &[(&str, &str)],
) -> Result<ResourceContracts, String> {
    let selection: Vec<_> = selection
        .iter()
        .map(|(c, m)| PublishedModuleRef::new(*c, *m))
        .collect();
    load_published_resource_contracts(load_path, &selection)
}

fn hand_written() -> ResourceContracts {
    let text = read_crate_modules(FLB)["fluent_bit/hand_written.qv"].clone();
    load_resource_contract_sources(&BTreeMap::from([("hand_written.qcore".to_owned(), text)]))
        .unwrap()
}

// ---------------------------------------------------------------------------
// Positive fixtures
// ---------------------------------------------------------------------------

#[test]
fn inherited_wrappers_load_exactly_like_their_hand_written_equivalent() {
    let inherited = load(&fixture_load_path(), &[(FLB, "fluent_bit/wrappers.qv")]).unwrap();
    let written = hand_written();
    // Custody modes, owner indices and result modes: identical.
    assert_eq!(inherited.functions(), written.functions());
    assert_eq!(inherited.functions().len(), 6);
    // Every rederived fact a consumer reads is identical too.
    assert_eq!(
        declared_abi_signatures_of(&inherited).unwrap(),
        declared_abi_signatures_of(&written).unwrap()
    );
    assert_eq!(
        declared_literal_argument_premises(&inherited).unwrap(),
        declared_literal_argument_premises(&written).unwrap()
    );
    assert_eq!(
        declared_variadic_data_tails(&inherited).unwrap(),
        BTreeSet::from(["flb_printf".to_owned()])
    );
    // Only the selected module's declarations enter the environment: an
    // included module contributes parents, not functions.
    for parent in [
        "memcpy",
        "free",
        "realloc",
        "printf",
        "malloc",
        "cfl_table_put",
    ] {
        assert!(inherited.contract(parent).is_none(), "{parent} leaked");
    }
    check_source_assumptions(&inherited).unwrap();
}

#[test]
fn the_retained_source_is_the_parent_contract_under_the_positional_renaming() {
    let inherited = load(&fixture_load_path(), &[(FLB, "fluent_bit/wrappers.qv")]).unwrap();
    let key = format!("{FLB}:fluent_bit/wrappers.qv");
    let text = &inherited.source_assumptions()[&key];
    assert!(!text.contains("#include"));
    assert!(!text.contains("inherit"));
    // memcpy's `dest, src, n` became flb_memcpy's `to, from, bytes`.
    assert!(text.contains(
        "contract { requires resource_contract_v1; ensures normal resource_borrowed(to); \
         ensures normal resource_borrowed(from); ensures normal resource_borrowed(bytes); \
         ensures normal resource_return_none(); };"
    ));
    // The owner index of `resource_stored` and the premise's parameter follow
    // the position, not the parent's names.
    assert!(text.contains("requires resource_literal_argument(val_size, 0);"));
    assert!(text.contains("resource_stored(val, ht, nonnull)"));
    // The retained text loads on its own, like any hand-written module.
    let alone = load_resource_contract_sources(&BTreeMap::from([(key, text.clone())])).unwrap();
    assert_eq!(alone.functions(), inherited.functions());
    assert_eq!(
        literal_argument_premises(inherited.source_assumptions()).unwrap()["flb_table_put"][0]
            .parameter,
        2
    );
    assert_eq!(
        variadic_data_tails(inherited.source_assumptions()).unwrap(),
        BTreeSet::from(["flb_printf".to_owned()])
    );
}

#[test]
fn evidence_carries_each_chain_with_crate_version_module_and_digest() {
    let inherited = load(&fixture_load_path(), &[(FLB, "fluent_bit/wrappers.qv")]).unwrap();
    let key = format!("{FLB}:fluent_bit/wrappers.qv");
    let evidence = &inherited.inclusion_evidence()[&key];
    let libc = read_crate_modules(LIBC);
    assert_eq!(evidence.module.crate_name, FLB);
    assert_eq!(evidence.module.crate_version, "0.1.0");
    assert_eq!(
        evidence.module.sha256,
        sha256_hex(evidence.source.as_bytes())
    );
    assert_eq!(
        evidence.included.keys().cloned().collect::<Vec<_>>(),
        [
            format!("{CFL}:cfl/table.qv"),
            format!("{LIBC}:libc/heap.qv"),
            format!("{LIBC}:libc/string.qv"),
        ]
    );
    let memcpy = &evidence.inherited["flb_memcpy"];
    assert_eq!(memcpy.len(), 1);
    assert_eq!(memcpy[0].declaration, "memcpy");
    assert_eq!(memcpy[0].module.crate_name, LIBC);
    assert_eq!(memcpy[0].module.module, "libc/string.qv");
    assert_eq!(
        memcpy[0].module.sha256,
        sha256_hex(libc["libc/string.qv"].as_bytes())
    );
    // The own-module chain: flb_sds_destroy -> flb_free -> libc::free.
    let chain: Vec<_> = evidence.inherited["flb_sds_destroy"]
        .iter()
        .map(|link| (link.declaration.as_str(), link.module.crate_name.as_str()))
        .collect();
    assert_eq!(chain, [("flb_free", FLB), ("free", LIBC)]);
    assert_eq!(evidence.inherited.len(), 6);
}

#[test]
fn a_chain_crosses_three_crates_through_direct_dependencies_only() {
    let inherited = load(&fixture_load_path(), &[(APP, "app/chain.qv")]).unwrap();
    let key = format!("{APP}:app/chain.qv");
    let evidence = &inherited.inclusion_evidence()[&key];
    let chain: Vec<_> = evidence.inherited["app_copy"]
        .iter()
        .map(|link| (link.declaration.as_str(), link.module.crate_name.as_str()))
        .collect();
    assert_eq!(chain, [("flb_memcpy", FLB), ("memcpy", LIBC)]);
    let release: Vec<_> = evidence.inherited["app_release"]
        .iter()
        .map(|link| link.declaration.as_str())
        .collect();
    assert_eq!(release, ["flb_sds_destroy", "flb_free", "free"]);
    // The transitive include closure is retained so the replay needs no load
    // path, even though app may not include libc itself.
    assert_eq!(evidence.included.len(), 4);
    assert_eq!(
        inherited.contract("app_copy"),
        hand_written().contract("flb_memcpy")
    );
    check_source_assumptions(&inherited).unwrap();
}

#[test]
fn selecting_the_parent_module_beside_the_child_is_still_one_function_each() {
    let both = load(
        &fixture_load_path(),
        &[(FLB, "fluent_bit/wrappers.qv"), (LIBC, "libc/string.qv")],
    )
    .unwrap();
    assert!(both.contract("memcpy").is_some());
    assert_eq!(both.contract("flb_memcpy"), both.contract("memcpy"));
    // A module without inclusion carries no evidence.
    assert_eq!(both.inclusion_evidence().len(), 1);
    check_source_assumptions(&both).unwrap();
}

// ---------------------------------------------------------------------------
// Provenance, identity and replay
// ---------------------------------------------------------------------------

fn environment_digest(contracts: &ResourceContracts) -> String {
    sha256_hex(&serde_json::to_vec(contracts).unwrap())
}

#[test]
fn a_change_to_the_parent_text_changes_the_childs_contract_identity() {
    let before = load(&fixture_load_path(), &[(FLB, "fluent_bit/wrappers.qv")]).unwrap();
    let mut crates = fixture_crates();
    // A comment-only edit: the elaborated text and the modes do not move, but
    // the parent's identity does, and so does the environment's.
    crates[0]
        .modules
        .get_mut("libc/string.qv")
        .unwrap()
        .push_str("// edited\n");
    let after = load(
        &ContractLoadPath::new(crates).unwrap(),
        &[(FLB, "fluent_bit/wrappers.qv")],
    )
    .unwrap();
    assert_eq!(before.functions(), after.functions());
    assert_eq!(before.source_assumptions(), after.source_assumptions());
    assert_ne!(before.inclusion_evidence(), after.inclusion_evidence());
    assert_ne!(environment_digest(&before), environment_digest(&after));
}

#[test]
fn an_environment_without_inclusion_serializes_exactly_as_before() {
    let written = hand_written();
    let json = serde_json::to_value(&written).unwrap();
    let keys: Vec<_> = json.as_object().unwrap().keys().cloned().collect();
    assert_eq!(keys, ["functions", "source_assumptions"]);
    let inherited = load(&fixture_load_path(), &[(FLB, "fluent_bit/wrappers.qv")]).unwrap();
    let json = serde_json::to_value(&inherited).unwrap();
    assert!(json.get("inclusion_evidence").is_some());
    let round: ResourceContracts = serde_json::from_value(json).unwrap();
    assert_eq!(round, inherited);
    check_source_assumptions(&round).unwrap();
}

fn assert_code(result: Result<impl std::fmt::Debug, String>, code: &str) -> String {
    let error = result.expect_err("expected a refusal");
    assert!(error.starts_with(code), "expected {code}, got: {error}");
    eprintln!("refusal: {error}");
    error
}

#[test]
fn tampered_or_orphaned_evidence_is_refused_on_recheck() {
    let inherited = load(&fixture_load_path(), &[(FLB, "fluent_bit/wrappers.qv")]).unwrap();
    let key = format!("{FLB}:fluent_bit/wrappers.qv");
    let json = serde_json::to_value(&inherited).unwrap();
    let mutate = |edit: &dyn Fn(&mut serde_json::Value)| {
        let mut value = json.clone();
        edit(&mut value);
        let contracts: ResourceContracts = serde_json::from_value(value).unwrap();
        check_source_assumptions(&contracts)
    };
    // A retained parent edited without its digest.
    assert_code(
        mutate(&|v| {
            v["inclusion_evidence"][&key]["included"][format!("{LIBC}:libc/heap.qv")]["source"] =
                "module contract;\n".into();
        }),
        "QI016",
    );
    // A retained parent edited *with* a matching digest: the replay no longer
    // reproduces the retained elaborated text.
    let edited = read_crate_modules(LIBC)["libc/heap.qv"].replace(
        "nullable);\n    ensures normal resource_return_none",
        "nonnull);\n    ensures normal resource_return_none",
    );
    assert_code(
        mutate(&|v| {
            let entry =
                &mut v["inclusion_evidence"][&key]["included"][format!("{LIBC}:libc/heap.qv")];
            entry["source"] = edited.clone().into();
            entry["identity"]["sha256"] = sha256_hex(edited.as_bytes()).into();
        }),
        "QI016",
    );
    // A retained module the replay never reaches.
    assert_code(
        mutate(&|v| {
            let extra =
                v["inclusion_evidence"][&key]["included"][format!("{CFL}:cfl/table.qv")].clone();
            v["inclusion_evidence"][&key]["included"]["quirin-provider-cfl:cfl/extra.qcore"] =
                extra;
        }),
        "QI016",
    );
    // A chain edited in the evidence.
    assert_code(
        mutate(&|v| {
            v["inclusion_evidence"][&key]["inherited"]["flb_free"][0]["declaration"] =
                "malloc".into();
        }),
        "QI016",
    );
    // Evidence for a module the environment does not retain.
    assert_code(
        mutate(&|v| {
            let entry = v["inclusion_evidence"][&key].clone();
            v["inclusion_evidence"]["quirin-provider-app:app/other.qcore"] = entry;
        }),
        "QI016",
    );
}

// ---------------------------------------------------------------------------
// Namespaces
// ---------------------------------------------------------------------------

#[test]
fn recorded_namespaces_equal_the_derived_ones() {
    for (name, namespace) in [
        ("quirin-provider-libc", "libc"),
        ("quirin-provider-cfl", "cfl"),
        ("quirin-provider-mpack", "mpack"),
        ("quirin-provider-msgpack", "msgpack"),
        ("quirin-provider-fluent-bit", "fluent_bit"),
        ("quirin-provider-shared", "shared"),
        // Card CONTRACT-INHERIT-QCORE-PROVIDER-NAMING (the user's option B):
        // the second admitted prefix derives the same way.
        ("qcore-provider-chunkio", "chunkio"),
        ("qcore-provider-cmetrics", "cmetrics"),
        ("qcore-provider-ctraces", "ctraces"),
        ("qcore-provider-libevent", "libevent"),
        ("qcore-provider-librdkafka", "librdkafka"),
    ] {
        assert_eq!(derive_contract_namespace(name).as_deref(), Some(namespace));
    }
    // Two prefixes, one remainder: one namespace (the load path refuses the
    // pair, `qi017_…`).
    assert_eq!(
        derive_contract_namespace("quirin-provider-libc"),
        derive_contract_namespace("qcore-provider-libc")
    );
    for crate_ in fixture_crates() {
        assert_eq!(
            derive_contract_namespace(&crate_.name).as_deref(),
            Some(crate_.namespace.as_str())
        );
    }
    for refused in [
        "other-provider-chunkio",
        "qcore-chunkio",
        "quirin-plugin-chunkio",
        "libc",
        "quirin-provider-",
        "quirin-provider-1x",
        "qcore-provider-",
        "qcore-provider-1x",
        "qcore-provider-Libc",
    ] {
        assert_eq!(derive_contract_namespace(refused), None, "{refused}");
    }
}

// ---------------------------------------------------------------------------
// Negative controls, one per refusal code
// ---------------------------------------------------------------------------

const HEADER: &str = "module contract;\n";

/// A load path where fluent-bit's module is `child` and libc's published
/// modules are the fixture ones plus `extra`.
fn with_child(child: &str, extra_libc: &[(&str, &str)]) -> Result<ResourceContracts, String> {
    let mut crates = fixture_crates();
    for (module, text) in extra_libc {
        crates[0]
            .modules
            .insert((*module).to_owned(), (*text).to_owned());
        crates[0].published.insert((*module).to_owned());
    }
    crates[2]
        .modules
        .insert("fluent_bit/child.qcore".into(), child.to_owned());
    load(
        &ContractLoadPath::new(crates).map_err(|e| e.to_string())?,
        &[(FLB, "fluent_bit/child.qcore")],
    )
}

fn child(includes: &[&str], body: &str) -> String {
    let mut text = HEADER.to_owned();
    for include in includes {
        text.push_str(&format!("#include \"{include}\";\n"));
    }
    text.push_str(body);
    text
}

const STRING: &str = "quirin-provider-libc:libc/string.qv";
const HEAP: &str = "quirin-provider-libc:libc/heap.qv";
const COPY: &str = "extern fn c(a: ptr<mut, u8>, b: ptr<const, u8>, n: usize) -> ptr<mut, u8>";

#[test]
fn qi001_malformed_inclusion_forms_are_refused() {
    // `inherit` on `unsafe extern fn`.
    assert_code(
        with_child(
            &child(
                &[HEAP],
                "unsafe extern fn f(p: ptr<mut, u8>) -> unit contract inherit libc::free;\n",
            ),
            &[],
        ),
        "QI001",
    );
    // A directive after a declaration.
    assert_code(
        with_child(
            &format!(
                "{HEADER}extern fn f(p: ptr<mut, u8>) -> unit contract inherit libc::free;\n#include \"{HEAP}\";\n"
            ),
            &[],
        ),
        "QI001",
    );
    // Two qualifier levels.
    assert_code(
        with_child(
            &child(
                &[HEAP],
                "extern fn f(p: ptr<mut, u8>) -> unit contract inherit a::libc::free;\n",
            ),
            &[],
        ),
        "QI001",
    );
    // Extra signature syntax on the child.
    assert_code(
        with_child(
            &child(
                &[HEAP],
                "extern fn f(p: ptr<mut, u8>) -> unit effects { io } contract inherit libc::free;\n",
            ),
            &[],
        ),
        "QI001",
    );
}

#[test]
fn qi002_extension_clauses_are_refused() {
    assert_code(
        with_child(
            &child(
                &[STRING],
                &format!(
                    "{COPY} contract inherit libc::memcpy {{ requires resource_literal_argument(n, 0); }};\n"
                ),
            ),
            &[],
        ),
        "QI002",
    );
}

#[test]
fn qi003_malformed_targets_are_refused() {
    for target in [
        "libc/string.qv",
        "quirin-provider-libc:../libc/string.qv",
        "quirin-provider-libc:/libc/string.qv",
    ] {
        assert_code(
            with_child(
                &child(
                    &[target],
                    &format!("{COPY} contract inherit libc::memcpy;\n"),
                ),
                &[],
            ),
            "QI003",
        );
    }
}

#[test]
fn qi004_an_undeclared_or_absent_crate_is_refused() {
    // app declares only fluent-bit, so libc is not reachable from it.
    let mut crates = fixture_crates();
    crates[3].modules.insert(
        "app/direct.qcore".into(),
        child(
            &[STRING],
            &format!("{COPY} contract inherit libc::memcpy;\n"),
        ),
    );
    assert_code(
        load(
            &ContractLoadPath::new(crates).unwrap(),
            &[(APP, "app/direct.qcore")],
        ),
        "QI004",
    );
    // A crate that is not on the load path at all.
    assert_code(
        with_child(
            &child(
                &["quirin-provider-mpack:mpack/writer.qv"],
                "extern fn f() -> unit contract inherit mpack::g;\n",
            ),
            &[],
        ),
        "QI004",
    );
    // libc, with no dependencies, can include nothing from another crate.
    let mut crates = fixture_crates();
    crates[0].modules.insert(
        "libc/up.qcore".into(),
        child(&["quirin-provider-cfl:cfl/table.qv"], "extern fn f(p: ptr<mut, u8>, q: ptr<mut, u8>, n: usize) -> i32 contract inherit cfl::cfl_table_put;\n"),
    );
    assert_code(
        load(
            &ContractLoadPath::new(crates).unwrap(),
            &[(LIBC, "libc/up.qcore")],
        ),
        "QI004",
    );
}

#[test]
fn qi005_an_unpublished_or_absent_module_is_refused() {
    assert_code(
        with_child(
            &child(
                &["quirin-provider-libc:libc/internal.qv"],
                "extern fn f(p: ptr<mut, u8>) -> unit contract inherit libc::internal_release;\n",
            ),
            &[],
        ),
        "QI005",
    );
    assert_code(
        with_child(
            &child(
                &["quirin-provider-libc:libc/absent.qcore"],
                "extern fn f(p: ptr<mut, u8>) -> unit contract inherit libc::free;\n",
            ),
            &[],
        ),
        "QI005",
    );
    // Own-crate includes need publication too: hand_written.qcore is not published.
    assert_code(
        with_child(
            &child(
                &["quirin-provider-fluent-bit:fluent_bit/hand_written.qv"],
                "extern fn f(p: ptr<mut, u8>) -> unit contract inherit fluent_bit::flb_free;\n",
            ),
            &[],
        ),
        "QI005",
    );
}

#[test]
fn qi006_include_cycles_are_refused() {
    let mut crates = fixture_crates();
    crates[0].modules.insert(
        "libc/a.qcore".into(),
        child(
            &["quirin-provider-libc:libc/b.qcore"],
            "extern fn a(p: ptr<mut, u8>) -> unit contract inherit libc::b;\n",
        ),
    );
    crates[0].modules.insert(
        "libc/b.qcore".into(),
        child(
            &["quirin-provider-libc:libc/a.qcore"],
            "extern fn b(p: ptr<mut, u8>) -> unit contract inherit libc::a;\n",
        ),
    );
    crates[0]
        .published
        .extend(["libc/a.qcore".to_owned(), "libc/b.qcore".to_owned()]);
    let error = assert_code(
        load(
            &ContractLoadPath::new(crates).unwrap(),
            &[(LIBC, "libc/a.qcore")],
        ),
        "QI006",
    );
    assert!(
        error.contains(
            "libc/a.qcore -> quirin-provider-libc:libc/b.qcore -> quirin-provider-libc:libc/a.qcore"
        ),
        "{error}"
    );
    // Self-inclusion.
    let mut crates = fixture_crates();
    crates[0].modules.insert(
        "libc/self.qcore".into(),
        child(
            &["quirin-provider-libc:libc/self.qcore"],
            "extern fn s(p: ptr<mut, u8>) -> unit contract inherit libc::s;\n",
        ),
    );
    crates[0].published.insert("libc/self.qcore".to_owned());
    assert_code(
        load(
            &ContractLoadPath::new(crates).unwrap(),
            &[(LIBC, "libc/self.qcore")],
        ),
        "QI006",
    );
}

#[test]
fn qi007_a_duplicate_include_is_refused() {
    assert_code(
        with_child(
            &child(
                &[STRING, STRING],
                &format!("{COPY} contract inherit libc::memcpy;\n"),
            ),
            &[],
        ),
        "QI007",
    );
}

#[test]
fn qi008_an_unknown_parent_is_refused() {
    assert_code(
        with_child(
            &child(
                &[STRING],
                &format!("{COPY} contract inherit libc::memmove;\n"),
            ),
            &[],
        ),
        "QI008",
    );
    assert_code(
        with_child(
            &child(&[], &format!("{COPY} contract inherit nowhere;\n")),
            &[],
        ),
        "QI008",
    );
}

#[test]
fn qi009_two_published_modules_of_one_crate_declaring_the_parent_are_ambiguous() {
    let duplicate = read_crate_modules(LIBC)["libc/string.qv"].clone();
    let error = assert_code(
        with_child(
            &child(
                &[STRING, "quirin-provider-libc:libc/string2.qcore"],
                &format!("{COPY} contract inherit libc::memcpy;\n"),
            ),
            &[("libc/string2.qcore", &duplicate)],
        ),
        "QI009",
    );
    assert!(error.contains("2 declarations"), "{error}");
}

#[test]
fn qi010_signature_disagreements_are_refused() {
    for signature in [
        // Arity.
        "extern fn c(a: ptr<mut, u8>, b: ptr<const, u8>) -> ptr<mut, u8>",
        // A parameter type.
        "extern fn c(a: ptr<mut, u8>, b: ptr<mut, u8>, n: usize) -> ptr<mut, u8>",
        // The result type.
        "extern fn c(a: ptr<mut, u8>, b: ptr<const, u8>, n: usize) -> unit",
    ] {
        assert_code(
            with_child(
                &child(
                    &[STRING],
                    &format!("{signature} contract inherit libc::memcpy;\n"),
                ),
                &[],
            ),
            "QI010",
        );
    }
}

#[test]
fn qi011_inheritance_cycles_are_refused() {
    assert_code(
        with_child(
            &child(
                &[],
                "extern fn f(p: ptr<mut, u8>) -> unit contract inherit g;\nextern fn g(p: ptr<mut, u8>) -> unit contract inherit f;\n",
            ),
            &[],
        ),
        "QI011",
    );
    assert_code(
        with_child(
            &child(
                &[],
                "extern fn f(p: ptr<mut, u8>) -> unit contract inherit f;\n",
            ),
            &[],
        ),
        "QI011",
    );
}

#[test]
fn qi012_a_capturing_or_diverging_renaming_is_refused() {
    // Capture: the child's parameter `nullable` would be captured by the
    // parent's keyword `nullable`.
    let error = assert_code(
        with_child(
            &child(
                &[HEAP],
                "extern fn f(nullable: ptr<mut, u8>) -> unit contract inherit libc::free;\n",
            ),
            &[],
        ),
        "QI012",
    );
    assert!(error.contains("captured"), "{error}");
    // Divergence under the v1 reading: a parent parameter *named* `nullable`
    // is a parameter word the renaming replaces, while v1 reads the second
    // argument of `resource_owned` as a keyword. Renamed onto a child parameter
    // named `nonnull`, the text loads with the opposite nullability.
    let parent = "module contract;\n\
        extern fn odd(nullable: ptr<mut, u8>, x: ptr<mut, u8>) -> unit contract {\n\
            requires resource_contract_v1;\n\
            ensures normal resource_borrowed(nullable);\n\
            ensures normal resource_owned(x, nullable);\n\
            ensures normal resource_return_none();\n};\n";
    let error = assert_code(
        with_child(
            &child(
                &["quirin-provider-libc:libc/odd.qcore"],
                "extern fn f(nonnull: ptr<mut, u8>, y: ptr<mut, u8>) -> unit contract inherit libc::odd;\n",
            ),
            &[("libc/odd.qcore", parent)],
        ),
        "QI012",
    );
    assert!(error.contains("loads differently"), "{error}");
    // A parent that is not a resource-v1 declaration cannot be read by this
    // consumer at all.
    let observation = "module contract;\nextern fn obs(x: i32) -> i32 contract { requires x >= 0; ensures normal result == x; };\n";
    assert_code(
        with_child(
            &child(
                &["quirin-provider-libc:libc/obs.qcore"],
                "extern fn f(y: i32) -> i32 contract inherit libc::obs;\n",
            ),
            &[("libc/obs.qcore", observation)],
        ),
        "QI012",
    );
}

#[test]
fn qi013_entry_points_without_a_load_path_refuse_inclusion() {
    let text = read_crate_modules(FLB)["fluent_bit/wrappers.qv"].clone();
    let sources = BTreeMap::from([("wrappers.qcore".to_owned(), text)]);
    assert_code(load_resource_contract_sources(&sources), "QI013");
    assert_code(variadic_data_tails(&sources), "QI013");
    assert_code(literal_argument_premises(&sources), "QI013");
    // An inherit tail alone, without any directive, is refused the same way.
    let inherit_only = BTreeMap::from([(
        "own.qcore".to_owned(),
        "module contract;\nextern fn f(p: ptr<mut, u8>) -> unit contract inherit g;\n".to_owned(),
    )]);
    assert_code(load_resource_contract_sources(&inherit_only), "QI013");
}

#[test]
fn qi014_invalid_load_paths_are_refused() {
    let refuse = |edit: &dyn Fn(&mut Vec<PublishedContractCrate>)| {
        let mut crates = fixture_crates();
        edit(&mut crates);
        ContractLoadPath::new(crates)
            .map(|_| ())
            .map_err(|e| e.to_string())
    };
    // Duplicate crate.
    assert_code(
        refuse(&|c| {
            let copy = c[0].clone();
            c.push(copy);
        }),
        "QI014",
    );
    // A published module the crate does not own.
    assert_code(
        refuse(&|c| {
            c[0].published.insert("libc/absent.qcore".into());
        }),
        "QI014",
    );
    // A dependency absent from the load path, and a self-dependency.
    assert_code(
        refuse(&|c| {
            c[3].dependencies.insert("quirin-provider-mpack".into());
        }),
        "QI014",
    );
    assert_code(
        refuse(&|c| {
            c[0].dependencies.insert(LIBC.into());
        }),
        "QI014",
    );
    // A dependency cycle.
    let error = assert_code(
        refuse(&|c| {
            c[0].dependencies.insert(APP.into());
        }),
        "QI014",
    );
    assert!(error.contains("cycle"), "{error}");
    // A malformed module path or version.
    assert_code(
        refuse(&|c| {
            c[0].modules.insert("../escape.qcore".into(), HEADER.into());
        }),
        "QI014",
    );
    assert_code(
        refuse(&|c| {
            c[0].version = String::new();
        }),
        "QI014",
    );
}

#[test]
fn qi015_an_uninheritable_parent_is_refused() {
    let parents = "module contract;\n\
        unsafe extern fn raw(p: ptr<mut, u8>) -> unit;\n\
        extern fn effected(p: ptr<mut, u8>) -> unit effects { io } contract {\n\
            requires resource_contract_v1;\n\
            ensures normal resource_released(p, nullable);\n\
            ensures normal resource_return_none();\n};\n";
    for parent in ["raw", "effected"] {
        assert_code(
            with_child(
                &child(
                    &["quirin-provider-libc:libc/parents.qcore"],
                    &format!(
                        "extern fn f(p: ptr<mut, u8>) -> unit contract inherit libc::{parent};\n"
                    ),
                ),
                &[("libc/parents.qcore", parents)],
            ),
            "QI015",
        );
    }
    // An included module that is not a contract module.
    assert_code(
        with_child(
            &child(
                &["quirin-provider-libc:libc/lib.qcore"],
                "extern fn f(p: ptr<mut, u8>) -> unit contract inherit libc::g;\n",
            ),
            &[(
                "libc/lib.qcore",
                "module library;\nfn g() -> unit { return; }\n",
            )],
        ),
        "QI015",
    );
}

#[test]
fn qi017_crate_names_and_namespaces_are_checked_when_the_load_path_is_built() {
    let refuse = |edit: &dyn Fn(&mut Vec<PublishedContractCrate>)| {
        let mut crates = fixture_crates();
        edit(&mut crates);
        ContractLoadPath::new(crates)
            .map(|_| ())
            .map_err(|e| e.to_string())
    };
    // A crate with neither admitted prefix. (Before card
    // CONTRACT-INHERIT-QCORE-PROVIDER-NAMING this control used
    // `qcore-provider-chunkio`, which the second prefix now admits.)
    assert_code(
        refuse(&|c| {
            c[3].name = "other-provider-chunkio".into();
            c[3].namespace = "chunkio".into();
        }),
        "QI017",
    );
    // The second prefix is admitted, and its namespace must still be derived.
    let mut crates = fixture_crates();
    crates[3].name = "qcore-provider-chunkio".into();
    crates[3].namespace = "chunkio".into();
    ContractLoadPath::new(crates).expect("a qcore-provider-* crate derives its namespace");
    assert_code(
        refuse(&|c| {
            c[3].name = "qcore-provider-chunkio".into();
            c[3].namespace = "qcore_chunkio".into();
        }),
        "QI017",
    );
    // One remainder under both prefixes: one namespace, two crates.
    let error = assert_code(
        refuse(&|c| {
            let mut twin = c[0].clone();
            twin.name = "qcore-provider-libc".into();
            c.push(twin);
        }),
        "QI017",
    );
    assert!(
        error.contains("namespace `libc` is also the namespace of crate"),
        "{error}"
    );
    // A recorded namespace that is not the derived one.
    assert_code(refuse(&|c| c[0].namespace = "c".into()), "QI017");
    // Two crates that derive the same namespace.
    let error = assert_code(
        refuse(&|c| {
            let mut twin = c[2].clone();
            twin.name = "quirin-provider-fluent_bit".into();
            c.push(twin);
        }),
        "QI017",
    );
    assert!(error.contains("fluent_bit"), "{error}");
}

#[test]
fn qi018_an_unqualified_parent_found_only_in_an_included_module_is_refused() {
    let error = assert_code(
        with_child(
            &child(&[STRING], &format!("{COPY} contract inherit memcpy;\n")),
            &[],
        ),
        "QI018",
    );
    assert!(error.contains("contract inherit libc::memcpy"), "{error}");
}

#[test]
fn qi019_a_namespace_that_names_no_directly_included_crate_is_refused() {
    // Unknown namespace.
    assert_code(
        with_child(
            &child(
                &[STRING],
                &format!("{COPY} contract inherit posix::memcpy;\n"),
            ),
            &[],
        ),
        "QI019",
    );
    // A namespace on the load path whose crate this module does not include.
    assert_code(
        with_child(
            &child(&[HEAP], &format!("{COPY} contract inherit cfl::memcpy;\n")),
            &[],
        ),
        "QI019",
    );
    // Includes are not re-exported: app includes fluent-bit's wrappers, which
    // include libc, but `libc::` is not in app's scope.
    let mut crates = fixture_crates();
    crates[3].modules.insert(
        "app/reexport.qcore".into(),
        child(
            &["quirin-provider-fluent-bit:fluent_bit/wrappers.qv"],
            &format!("{COPY} contract inherit libc::memcpy;\n"),
        ),
    );
    assert_code(
        load(
            &ContractLoadPath::new(crates).unwrap(),
            &[(APP, "app/reexport.qcore")],
        ),
        "QI019",
    );
}

#[test]
fn qi020_a_child_named_like_its_qualified_parent_is_refused() {
    assert_code(
        with_child(
            &child(
                &[STRING],
                "extern fn memcpy(a: ptr<mut, u8>, b: ptr<const, u8>, n: usize) -> ptr<mut, u8> contract inherit libc::memcpy;\n",
            ),
            &[],
        ),
        "QI020",
    );
}

#[test]
fn a_refused_module_leaves_no_partial_environment() {
    // One good and one refused selection: nothing is returned.
    let mut crates = fixture_crates();
    crates[2].modules.insert(
        "fluent_bit/bad.qcore".into(),
        child(
            &[STRING],
            &format!("{COPY} contract inherit libc::memmove;\n"),
        ),
    );
    let result = load(
        &ContractLoadPath::new(crates).unwrap(),
        &[
            (FLB, "fluent_bit/wrappers.qv"),
            (FLB, "fluent_bit/bad.qcore"),
        ],
    );
    assert_code(result, "QI008");
}
