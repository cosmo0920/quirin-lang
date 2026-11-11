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

//! Card CONTRACT-INHERIT-ADOPTION: contract inclusion and inheritance wired
//! into the real provider crates.
//!
//! * The real load path (`providers/contract-load-path.json`, generated from
//!   Cargo metadata) reads and validates, and says what the crates record.
//! * The adopted inheritance is **parity**: `fluent_bit/resource_memory.qv`
//!   elaborates, declaration by declaration and token by token, to the
//!   contracts it carried when they were written by hand
//!   (`fixtures/contract_inherit_adoption/resource_memory_hand_written.qcore`,
//!   the file as it stood at `dfe10c92`), and the loaded records are equal.
//! * The resource manifest's schema 2 selects the adopted module by
//!   `crate:module` reference, publishes the inheritance chains, and loads an
//!   environment whose functions and rederived facts equal the hand-written
//!   selection's; path entries load exactly as in schema 1.
//! * Negative controls: every new refusal is reachable and fails closed.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use qcore_contract::contract_inclusion::elaborate_contract_module;
use qcore_contract::contract_loader::{
    check_source_assumptions, declared_abi_signatures, inclusion_provenance,
    literal_argument_premises, load_resource_contract_manifest, load_resource_contract_sources,
    variadic_data_tails,
};
use qcore_contract::load_path_manifest::load_contract_load_path_manifest;
use qcore_frontend::frontend::parse_source_program;
use qcore_schemas::sha256_hex;

#[path = "support/contract_corpus.rs"]
mod contract_corpus;

const FLB: &str = "quirin-provider-fluent-bit";
const LIBC: &str = "quirin-provider-libc";
const RESOURCE_MEMORY: &str = "fluent_bit/resource_memory.qv";

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_owned()
}

fn hand_written() -> String {
    fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/contract_inherit_adoption/resource_memory_hand_written.qv"),
    )
    .unwrap()
    .replace("\r\n", "\n")
}

fn real_load_path() -> qcore_contract::load_path_manifest::LoadPathManifest {
    load_contract_load_path_manifest(&repository().join("providers/contract-load-path.json"))
        .unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn the_real_load_path_reads_and_records_what_the_crates_declare() {
    let manifest = real_load_path();
    let crates: Vec<_> = manifest.contract_roots.keys().cloned().collect();
    // Card CONTRACT-INHERIT-QCORE-PROVIDER-NAMING (deliberate update): the
    // five executable `qcore-provider-*` crates joined the load path; their
    // records are pinned in `contract_inherit_qcore_provider.rs`.
    assert_eq!(
        crates,
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
    let flb = manifest.load_path.crate_named(FLB).unwrap();
    assert_eq!(flb.namespace, "fluent_bit");
    assert_eq!(
        flb.dependencies.iter().collect::<Vec<_>>(),
        [&LIBC.to_owned()]
    );
    assert!(flb.published.is_empty());
    let libc = manifest.load_path.crate_named(LIBC).unwrap();
    assert!(libc.dependencies.is_empty());
    assert_eq!(
        libc.published.iter().collect::<Vec<_>>(),
        ["libc/memory.qv", "libc/string.qv"]
    );
    // Every root the corpus index names for a load-path crate maps back to it.
    for (name, root) in &manifest.contract_roots {
        assert_eq!(manifest.crate_for_root(root), Some(name.as_str()));
    }
    // The corpus index names this same load path (spelled from `contract/`,
    // so its roots are other spellings of the same directories).
    let corpus = contract_corpus::load_path().unwrap();
    assert_eq!(corpus.load_path, manifest.load_path);
    for (name, root) in &corpus.contract_roots {
        assert_eq!(manifest.crate_for_root(root), Some(name.as_str()));
    }
}

/// The brief's parity requirement: the elaboration of every converted
/// declaration is the hand-written declaration it replaced, token for token
/// (whitespace and comments aside), and the loaded records are equal.
#[test]
fn adopted_inheritance_elaborates_to_the_hand_written_contracts() {
    let manifest = real_load_path();
    let elaborated = elaborate_contract_module(&manifest.load_path, FLB, RESOURCE_MEMORY).unwrap();
    let evidence = elaborated
        .evidence
        .expect("the adopted module uses inclusion");
    let before = hand_written();
    let parsed_after = parse_source_program(&elaborated.text).unwrap();
    let parsed_before = parse_source_program(&before).unwrap();
    let names = |parsed: &qcore_frontend::frontend::SourceProgram| {
        parsed
            .declarations
            .iter()
            .map(|d| d.value.name.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(names(&parsed_after), names(&parsed_before));
    for (after, before) in parsed_after
        .declarations
        .iter()
        .zip(&parsed_before.declarations)
    {
        assert_eq!(
            after.value.tokens, before.value.tokens,
            "{}: the elaboration is not the hand-written declaration",
            after.value.name
        );
    }
    // Loaded records: modes, ABI projections, premises and tails.
    let after_sources = BTreeMap::from([("m".to_owned(), elaborated.text.clone())]);
    let before_sources = BTreeMap::from([("m".to_owned(), before.clone())]);
    assert_eq!(
        load_resource_contract_sources(&after_sources)
            .unwrap()
            .functions(),
        load_resource_contract_sources(&before_sources)
            .unwrap()
            .functions()
    );
    assert_eq!(
        declared_abi_signatures(&after_sources).unwrap(),
        declared_abi_signatures(&before_sources).unwrap()
    );
    assert_eq!(
        literal_argument_premises(&after_sources).unwrap(),
        literal_argument_premises(&before_sources).unwrap()
    );
    assert_eq!(
        variadic_data_tails(&after_sources).unwrap(),
        variadic_data_tails(&before_sources).unwrap()
    );
    // Provenance: three chains of one link each, into the published libc
    // module, whose digest is its pin in the load-path manifest.
    let libc_memory = fs::read_to_string(
        repository().join("providers/quirin-provider-libc/contracts/libc/memory.qv"),
    )
    .unwrap();
    let chains: Vec<_> = evidence
        .inherited
        .iter()
        .map(|(child, chain)| {
            assert_eq!(chain.len(), 1, "{child}");
            assert_eq!(chain[0].module.crate_name, LIBC);
            assert_eq!(chain[0].module.module, "libc/memory.qv");
            assert_eq!(chain[0].module.sha256, sha256_hex(libc_memory.as_bytes()));
            (child.clone(), chain[0].declaration.clone())
        })
        .collect();
    assert_eq!(
        chains,
        [
            ("flb_calloc".to_owned(), "calloc".to_owned()),
            ("flb_free".to_owned(), "free".to_owned()),
            ("flb_malloc".to_owned(), "malloc".to_owned()),
        ]
    );
    assert_eq!(
        evidence.included.keys().collect::<Vec<_>>(),
        ["quirin-provider-libc:libc/memory.qv"]
    );
}

/// `fluent_bit/resource_string.qv` (user-directed): `flb_strdup` and
/// `flb_strndup` elaborate to exactly the written-out `strdup`/`strndup`
/// statements under their own parameter names, with one-link chains into the
/// published `libc/string.qv`.
#[test]
fn string_wrappers_elaborate_to_the_libc_statements() {
    let manifest = real_load_path();
    let elaborated =
        elaborate_contract_module(&manifest.load_path, FLB, "fluent_bit/resource_string.qv")
            .unwrap();
    let written_out = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/contract_inherit_adoption/resource_string_written_out.qv"),
    )
    .unwrap()
    .replace("\r\n", "\n");
    let after = parse_source_program(&elaborated.text).unwrap();
    let before = parse_source_program(&written_out).unwrap();
    assert_eq!(after.declarations.len(), 2);
    for (a, b) in after.declarations.iter().zip(&before.declarations) {
        assert_eq!(a.value.name, b.value.name);
        assert_eq!(a.value.tokens, b.value.tokens, "{}", a.value.name);
    }
    let evidence = elaborated.evidence.unwrap();
    let parents: Vec<_> = evidence
        .inherited
        .iter()
        .map(|(child, chain)| {
            assert_eq!(chain.len(), 1);
            assert_eq!(chain[0].module.module, "libc/string.qv");
            (child.as_str(), chain[0].declaration.as_str())
        })
        .collect();
    assert_eq!(
        parents,
        [("flb_strdup", "strdup"), ("flb_strndup", "strndup")]
    );
}

/// The Cycle 19 manifest selects the adopted module by reference; the
/// environment equals the one the hand-written module gave, in every fact a
/// consumer rederives, and carries the chains a report publishes.
#[test]
fn the_resource_manifest_selects_the_adopted_module_with_equal_meaning() {
    let manifest = repository().join("cycle19-resource-contracts.json");
    let contracts = load_resource_contract_manifest(&manifest).unwrap();
    let key = format!("{FLB}:{RESOURCE_MEMORY}");
    assert!(contracts.source_assumptions().contains_key(&key));
    let strings = format!("{FLB}:fluent_bit/resource_string.qv");
    // Card CONTRACT-INHERIT-QCORE-PROVIDER-NAMING adds the two converted
    // provider modules (`contract_inherit_qcore_provider.rs`).
    assert_eq!(
        contracts.inclusion_evidence().keys().collect::<Vec<_>>(),
        [
            &"qcore-provider-libevent:monkey/memory.qv".to_owned(),
            &"qcore-provider-librdkafka:librdkafka/memory.qv".to_owned(),
            &key,
            &strings
        ]
    );
    check_source_assumptions(&contracts).unwrap();
    let provenance = inclusion_provenance(&contracts).unwrap();
    assert!(provenance["modules"][&key]["inherited"]["flb_malloc"].is_array());

    // The same selection with the hand-written module in the adopted one's
    // place: equal functions and equal rederived facts.
    let mut hand = contracts.source_assumptions().clone();
    hand.remove(&key);
    hand.insert("hand-written".to_owned(), hand_written());
    let hand_loaded = load_resource_contract_sources(&hand).unwrap();
    assert_eq!(hand_loaded.functions(), contracts.functions());
    let adopted = contracts.source_assumptions();
    assert_eq!(
        declared_abi_signatures(&hand).unwrap(),
        declared_abi_signatures(adopted).unwrap()
    );
    assert_eq!(
        literal_argument_premises(&hand).unwrap(),
        literal_argument_premises(adopted).unwrap()
    );
    assert_eq!(
        variadic_data_tails(&hand).unwrap(),
        variadic_data_tails(adopted).unwrap()
    );
}

/// A scratch directory with one load-path crate and manifests beside it.
struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "qcore-inherit-adoption-{tag}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("quirin-provider-demo/contracts/demo")).unwrap();
        fs::create_dir_all(root.join("quirin-provider-app/contracts/app")).unwrap();
        fs::write(
            root.join("quirin-provider-demo/contracts/demo/heap.qcore"),
            HEAP,
        )
        .unwrap();
        fs::write(
            root.join("quirin-provider-app/contracts/app/wrap.qcore"),
            WRAP,
        )
        .unwrap();
        let scratch = Self(root);
        scratch.write_load_path(&load_path_json(&sha256_hex(HEAP.as_bytes())));
        scratch
    }

    fn write_load_path(&self, value: &serde_json::Value) {
        fs::write(
            self.0.join("load-path.json"),
            serde_json::to_vec_pretty(value).unwrap(),
        )
        .unwrap();
    }

    fn manifest(&self, text: &str) -> PathBuf {
        let path = self.0.join("selection.json");
        fs::write(&path, text).unwrap();
        path
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

const HEAP: &str = "module contract;\n\
extern fn free(ptr: ptr<mut, u8>) -> unit\n\
contract {\n\
    requires resource_contract_v1;\n\
    ensures normal resource_released(ptr, nullable);\n\
    ensures normal resource_return_none();\n\
};\n";

const WRAP: &str = "module contract;\n\
#include \"quirin-provider-demo:demo/heap.qcore\";\n\
extern fn app_free(p: ptr<mut, u8>) -> unit contract inherit demo::free;\n";

fn load_path_json(pin: &str) -> serde_json::Value {
    serde_json::json!({
        "schema_version": 1,
        "crates": [
            {
                "name": "quirin-provider-app", "namespace": "app", "version": "0.1.0",
                "contracts": "quirin-provider-app/contracts",
                "dependencies": ["quirin-provider-demo"],
                "modules": ["app/wrap.qcore"], "published": {}
            },
            {
                "name": "quirin-provider-demo", "namespace": "demo", "version": "0.1.0",
                "contracts": "quirin-provider-demo/contracts",
                "dependencies": [],
                "modules": ["demo/heap.qcore"], "published": { "demo/heap.qcore": pin }
            }
        ]
    })
}

#[test]
fn schema_two_path_entries_load_exactly_as_schema_one() {
    let scratch = Scratch::new("paths");
    let one = load_resource_contract_manifest(&scratch.manifest(
        r#"{"schema_version":1,"modules":["quirin-provider-demo/contracts/demo/heap.qcore"]}"#,
    ))
    .unwrap();
    let two = load_resource_contract_manifest(&scratch.manifest(
        r#"{"schema_version":2,"load_path":"load-path.json","modules":["quirin-provider-demo/contracts/demo/heap.qcore"]}"#,
    ))
    .unwrap();
    assert_eq!(one, two);
    assert_eq!(
        serde_json::to_vec(&one).unwrap(),
        serde_json::to_vec(&two).unwrap()
    );
    assert!(two.inclusion_evidence().is_empty());
}

#[test]
fn schema_two_references_elaborate_and_mix_with_paths() {
    let scratch = Scratch::new("mixed");
    let loaded = load_resource_contract_manifest(&scratch.manifest(
        r#"{"schema_version":2,"load_path":"load-path.json","modules":["quirin-provider-demo/contracts/demo/heap.qcore","quirin-provider-app:app/wrap.qcore"]}"#,
    ))
    .unwrap();
    assert_eq!(
        loaded.contract("app_free"),
        loaded.contract("free"),
        "the inherited contract is its parent's"
    );
    assert!(
        loaded
            .inclusion_evidence()
            .contains_key("quirin-provider-app:app/wrap.qcore")
    );
    check_source_assumptions(&loaded).unwrap();
}

#[test]
fn schema_and_selection_refusals_fail_closed() {
    let scratch = Scratch::new("refusals");
    for (text, expected) in [
        // Schema 2 requires a load path; schema 1 admits none.
        (
            r#"{"schema_version":2,"modules":["quirin-provider-app:app/wrap.qcore"]}"#,
            "unsupported version",
        ),
        (
            r#"{"schema_version":1,"load_path":"load-path.json","modules":["quirin-provider-demo/contracts/demo/heap.qcore"]}"#,
            "unsupported version",
        ),
        (
            r#"{"schema_version":3,"load_path":"load-path.json","modules":["quirin-provider-app:app/wrap.qcore"]}"#,
            "unsupported version",
        ),
        // A malformed reference is refused, never read as a path.
        (
            r#"{"schema_version":2,"load_path":"load-path.json","modules":["Quirin-Provider-App:app/wrap.qcore"]}"#,
            "is not a `crate:module` reference",
        ),
        (
            r#"{"schema_version":2,"load_path":"load-path.json","modules":["quirin-provider-app:../wrap.qcore"]}"#,
            "is not a `crate:module` reference",
        ),
        // A crate that is not on the load path, and a module it does not own.
        (
            r#"{"schema_version":2,"load_path":"load-path.json","modules":["quirin-provider-other:app/wrap.qcore"]}"#,
            "QI004",
        ),
        (
            r#"{"schema_version":2,"load_path":"load-path.json","modules":["quirin-provider-app:app/absent.qcore"]}"#,
            "QI005",
        ),
        // A module that uses inclusion selected by path: no load path applies.
        (
            r#"{"schema_version":2,"load_path":"load-path.json","modules":["quirin-provider-app/contracts/app/wrap.qcore"]}"#,
            "QI013",
        ),
        // A load path outside the manifest's directory.
        (
            r#"{"schema_version":2,"load_path":"../load-path.json","modules":["quirin-provider-app:app/wrap.qcore"]}"#,
            "relative without traversal",
        ),
    ] {
        let error = load_resource_contract_manifest(&scratch.manifest(text))
            .expect_err(text)
            .to_string();
        assert!(error.contains(expected), "{text}: {error}");
    }
}

/// Card CONTRACT-SUPERSESSION (contract spec C06.6): one module selected
/// twice, as a path and as a reference, declares every function twice under
/// two keys. With no pin stamps each is withheld as a contract conflict: none
/// is in force, never an optimistic choice.
#[test]
fn a_module_selected_twice_withholds_its_functions() {
    let scratch = Scratch::new("twice");
    let loaded = load_resource_contract_manifest(&scratch.manifest(
        r#"{"schema_version":2,"load_path":"load-path.json","modules":["quirin-provider-demo/contracts/demo/heap.qcore","quirin-provider-demo:demo/heap.qcore"]}"#,
    ))
    .unwrap();
    assert!(!loaded.contract_conflict().is_empty());
    assert!(loaded.functions().is_empty());
    check_source_assumptions(&loaded).unwrap();
}

#[test]
fn a_stale_or_invalid_load_path_is_refused() {
    let scratch = Scratch::new("stale");
    let selection = r#"{"schema_version":2,"load_path":"load-path.json","modules":["quirin-provider-app:app/wrap.qcore"]}"#;
    let valid = load_path_json(&sha256_hex(HEAP.as_bytes()));
    type Mutation = Box<dyn Fn(&mut serde_json::Value)>;
    let mutations: Vec<(&str, Mutation)> = vec![
        (
            "differs from its pinned SHA-256",
            Box::new(|v| {
                v["crates"][1]["published"]["demo/heap.qcore"] = sha256_hex(b"other").into();
            }),
        ),
        (
            "invalid SHA-256 pin",
            Box::new(|v| {
                v["crates"][1]["published"]["demo/heap.qcore"] = "0".into();
            }),
        ),
        (
            "differ from the contract directory",
            Box::new(|v| {
                v["crates"][1]["modules"] =
                    serde_json::json!(["demo/absent.qcore", "demo/heap.qcore"]);
            }),
        ),
        (
            "not sorted and unique",
            Box::new(|v| {
                v["crates"][1]["modules"] =
                    serde_json::json!(["demo/heap.qcore", "demo/heap.qcore"]);
            }),
        ),
        (
            "relative without traversal",
            Box::new(|v| {
                v["crates"][1]["contracts"] = "../quirin-provider-demo/contracts".into();
            }),
        ),
        (
            "QI017",
            Box::new(|v| {
                v["crates"][1]["namespace"] = "other".into();
            }),
        ),
        (
            // Neither admitted prefix. (`qcore-provider-app` is admitted since
            // card CONTRACT-INHERIT-QCORE-PROVIDER-NAMING, and derives `app`.)
            "QI017",
            Box::new(|v| {
                v["crates"][0]["name"] = "other-provider-app".into();
            }),
        ),
        (
            "is not on the load path",
            Box::new(|v| {
                v["crates"][0]["dependencies"] = serde_json::json!(["quirin-provider-absent"]);
            }),
        ),
        (
            "malformed load-path manifest",
            Box::new(|v| {
                v["crates"][0]["extra"] = true.into();
            }),
        ),
        (
            "unsupported load-path manifest schema",
            Box::new(|v| {
                v["schema_version"] = 2.into();
            }),
        ),
    ];
    for (expected, mutate) in mutations {
        let mut value = valid.clone();
        mutate(&mut value);
        scratch.write_load_path(&value);
        let error = load_resource_contract_manifest(&scratch.manifest(selection))
            .expect_err(expected)
            .to_string();
        assert!(error.contains(expected), "{expected}: {error}");
    }
    // An unlisted module on disk is refused too, and the valid path loads.
    scratch.write_load_path(&valid);
    load_resource_contract_manifest(&scratch.manifest(selection)).unwrap();
    fs::write(
        scratch
            .0
            .join("quirin-provider-demo/contracts/demo/stray.qcore"),
        HEAP,
    )
    .unwrap();
    let error = load_resource_contract_manifest(&scratch.manifest(selection))
        .expect_err("stray module")
        .to_string();
    assert!(error.contains("unlisted"), "{error}");
    // A changed parent without a regenerated pin is refused (QI014), not read.
    fs::remove_file(
        scratch
            .0
            .join("quirin-provider-demo/contracts/demo/stray.qcore"),
    )
    .unwrap();
    fs::write(
        scratch
            .0
            .join("quirin-provider-demo/contracts/demo/heap.qcore"),
        HEAP.replace("nullable", "nonnull"),
    )
    .unwrap();
    let error = load_resource_contract_manifest(&scratch.manifest(selection))
        .expect_err("changed parent")
        .to_string();
    assert!(error.starts_with("QI014"), "{error}");
}
