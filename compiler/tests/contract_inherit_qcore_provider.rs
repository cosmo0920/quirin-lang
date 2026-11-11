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

//! Card CONTRACT-INHERIT-QCORE-PROVIDER-NAMING: the executable
//! `qcore-provider-*` crates on the contract load path (the user's option B),
//! the six provider-owned wrappers that retain inheritance, two checked result
//! specifications, their
//! real packages' include pins, and inheritance from derived-result parents.
//!
//! * The real load path now carries the five `qcore-provider-*` crates, each
//!   with the namespace derived from its package name (`librdkafka`,
//!   `libevent`, ...), and exactly two new contract-use edges to libc.
//! * **Parity**, per conversion: `librdkafka/memory.qv` and
//!   `monkey/memory.qv` elaborate, declaration by declaration and token by
//!   token after erasing the eight separately validated proposed symbolic
//!   annotations, to the files as they stood at `2d0b77e1`
//!   (`fixtures/contract_inherit_qcore_provider/*_hand_written.qcore`); the
//!   loaded records retain parity except for the exact two rule-37 non-null
//!   results; ABI projections, literal premises and tails remain equal. The
//!   checked markers and named abort premises are asserted separately.
//! * The real package inventories pin their included libc parents, and the
//!   Rust `ContractBundle` accepts the real layout and refuses a changed or a
//!   missing parent.
//! * `CONTRACT-INHERIT-ALIAS-RESULT-PARENTS`: a child inherits a
//!   `resource_return_alias` or `resource_return_interior` parent under a
//!   renamed base parameter and loads to the parent's record, base index
//!   included; a child that reorders the parameters is refused, and a child
//!   parameter spelled like the predicate's second argument is captured and
//!   refused.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use qcore_contract::bundle::ContractBundle;
use qcore_contract::contract_inclusion::{
    ContractLoadPath, PublishedContractCrate, elaborate_contract_module,
};
use qcore_contract::contract_loader::{
    PublishedModuleRef, check_source_assumptions, declared_abi_signatures, inclusion_provenance,
    literal_argument_premises, load_published_resource_contracts, load_resource_contract_manifest,
    load_resource_contract_sources, variadic_data_tails,
};
use qcore_contract::load_path_manifest::load_contract_load_path_manifest;
use qcore_frontend::frontend::parse_source_program;
use qcore_schemas::contract::{DerivedExtent, ResourceReturnMode};
use qcore_schemas::sha256_hex;

#[path = "../../tests/support/tracked_files.rs"]
mod tracked_files;

const LIBC: &str = "quirin-provider-libc";
const LIBRDKAFKA: &str = "qcore-provider-librdkafka";
const LIBEVENT: &str = "qcore-provider-libevent";

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_owned()
}

fn fixture(name: &str) -> String {
    fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/contract_inherit_qcore_provider")
            .join(name),
    )
    .unwrap()
    .replace("\r\n", "\n")
}

fn real_load_path() -> qcore_contract::load_path_manifest::LoadPathManifest {
    load_contract_load_path_manifest(&repository().join("providers/contract-load-path.json"))
        .unwrap_or_else(|e| panic!("{e}"))
}

fn libc_text(module: &str) -> String {
    fs::read_to_string(
        repository()
            .join("providers/quirin-provider-libc/contracts")
            .join(module),
    )
    .unwrap()
}

/// One chain link: (child, parent declaration, parent module).
type Chain = (&'static str, &'static str, &'static str);

/// The six retained conversions: (crate, module, historical fixture, chains).
const CONVERSIONS: [(&str, &str, &str, &[Chain]); 2] = [
    (
        LIBRDKAFKA,
        "librdkafka/memory.qv",
        "librdkafka_memory_hand_written.qv",
        &[
            ("rd_free", "free", "libc/memory.qv"),
            ("rd_malloc", "malloc", "libc/memory.qv"),
            ("rd_strndup", "strndup", "libc/string.qv"),
        ],
    ),
    (
        LIBEVENT,
        "monkey/memory.qv",
        "monkey_memory_hand_written.qv",
        &[
            ("mk_mem_alloc", "malloc", "libc/memory.qv"),
            ("mk_mem_alloc_z", "malloc", "libc/memory.qv"),
            ("mk_mem_free", "free", "libc/memory.qv"),
        ],
    ),
];

#[test]
fn the_executable_crates_are_on_the_load_path_with_derived_namespaces() {
    let manifest = real_load_path();
    let crates: Vec<_> = manifest.contract_roots.keys().cloned().collect();
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
    for (name, namespace) in [
        ("qcore-provider-chunkio", "chunkio"),
        ("qcore-provider-cmetrics", "cmetrics"),
        ("qcore-provider-ctraces", "ctraces"),
        (LIBEVENT, "libevent"),
        (LIBRDKAFKA, "librdkafka"),
    ] {
        let krate = manifest.load_path.crate_named(name).unwrap();
        assert_eq!(krate.namespace, namespace);
        assert!(krate.published.is_empty(), "{name} publishes nothing");
        let expected: BTreeSet<String> = if name == LIBEVENT || name == LIBRDKAFKA {
            [LIBC, "quirin-provider-shared"]
                .into_iter()
                .map(str::to_owned)
                .collect()
        } else {
            ["quirin-provider-shared".to_owned()].into()
        };
        assert_eq!(krate.dependencies, expected, "{name}");
    }
    // The corpus index names the same load path, and every root maps back.
    for (name, root) in &manifest.contract_roots {
        assert_eq!(manifest.crate_for_root(root), Some(name.as_str()));
    }
}

/// The brief's parity standard, per conversion: the elaboration of every
/// converted declaration is the hand-written declaration it replaced, token for
/// token (whitespace and comments aside), except for the two exact checked
/// specifications asserted below. Only their result nullability evolves;
/// each converted declaration has one chain link into the published libc
/// module whose digest is that module's pin.
// Rule 37 deliberately changes exactly these two historical nullable results.
// The frozen fixture stays unchanged; the other six inheritance chains retain
// their complete token and loaded-record parity.
fn constructor_result_evolution(
    mut functions: BTreeMap<String, qcore_schemas::contract::ResourceFunctionContract>,
) -> BTreeMap<String, qcore_schemas::contract::ResourceFunctionContract> {
    for name in ["rd_calloc", "rd_strdup"] {
        if let Some(contract) = functions.get_mut(name) {
            assert_eq!(
                contract.result,
                ResourceReturnMode::Owned { nullable: true }
            );
            contract.result = ResourceReturnMode::Owned { nullable: false };
        }
    }
    functions
}

fn constructor_specification_tokens(name: &str) -> Vec<qcore_frontend::lexer::TokenKind> {
    let (parameters, borrowed) = match name {
        "rd_calloc" => (
            "count: usize, size: usize",
            "ensures normal resource_borrowed(count); ensures normal resource_borrowed(size);",
        ),
        "rd_strdup" => (
            "str: ptr<const, u8>",
            "ensures normal resource_borrowed(str);",
        ),
        _ => panic!("not a rule-37 checked wrapper"),
    };
    let source = format!(
        r#"module contract; extern fn {name}({parameters}) -> ptr<mut, u8> contract {{ requires resource_contract_v1; requires resource_body_checked; requires resource_premise(asserted_allocation, "the deployed build preserves the analyzed assertion and its declared terminating failure call; allocation failure has no normal return"); {borrowed} ensures normal resource_return_owned(nonnull); }};"#
    );
    parse_source_program(&source)
        .unwrap()
        .declarations
        .remove(0)
        .value
        .tokens
}

#[test]
fn converted_wrappers_elaborate_to_the_hand_written_contracts() {
    let manifest = real_load_path();
    let mut converted = 0;
    for (krate, module, hand, expected_chains) in CONVERSIONS {
        let elaborated = elaborate_contract_module(&manifest.load_path, krate, module).unwrap();
        let evidence = elaborated.evidence.expect("the module uses inclusion");
        let before = fixture(hand);
        // Keep the historical token fixture frozen. Only the new, separately
        // validated symbolic annotations are omitted from this legacy token
        // comparison; the full current text is loaded and compared below.
        let mut omitted = 0;
        let legacy_text = elaborated
            .text
            .lines()
            .filter(|line| {
                let line = line.trim();
                let symbolic = line.starts_with("requires resource_lifecycle_v1(introduce_plugin_")
                    || line.starts_with(
                        "requires resource_premise_flow_v1(decompose_rk_plugin_callback_closure,",
                    );
                omitted += usize::from(symbolic);
                !symbolic
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(omitted, if krate == LIBRDKAFKA { 8 } else { 0 }, "{module}");
        let legacy_text = if krate == LIBRDKAFKA {
            // Restore only the seven independently archived opaque atoms for
            // this frozen token comparison. Current typed loading is checked
            // separately below; the checkpoint never supplies live truth.
            let checkpoint = include_str!(
                "../../docs/reports/quant-contract-hypotheses/plugin/before/providers/qcore-provider-librdkafka/contracts/librdkafka/memory.qv"
            );
            let mut restored = 0;
            let text = legacy_text
                .lines()
                .map(|line| {
                    if let Some(formal) = line.trim().strip_prefix("quant proposition #plugin_") {
                        let id = format!("plugin_{}", formal.split('(').next().unwrap());
                        let prefix = format!("requires resource_hypothesis({id},");
                        let old = checkpoint
                            .lines()
                            .find(|l| l.trim().starts_with(&prefix))
                            .expect("typed atom has no independent historical counterpart");
                        restored += 1;
                        old
                    } else {
                        line
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");
            assert_eq!(restored, 7);
            text
        } else {
            legacy_text
        };
        let after = parse_source_program(&legacy_text).unwrap();
        let hand_parsed = parse_source_program(&before).unwrap();
        let names = |parsed: &qcore_frontend::frontend::SourceProgram| {
            parsed
                .declarations
                .iter()
                .map(|d| d.value.name.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(names(&after), names(&hand_parsed), "{module}");
        for (a, b) in after.declarations.iter().zip(&hand_parsed.declarations) {
            if krate == LIBRDKAFKA && matches!(a.value.name.as_str(), "rd_calloc" | "rd_strdup") {
                assert_eq!(
                    a.value.tokens,
                    constructor_specification_tokens(&a.value.name),
                    "{module}#{}",
                    a.value.name
                );
                continue;
            }
            assert_eq!(
                a.value.tokens, b.value.tokens,
                "{module}#{}: the elaboration is not the hand-written declaration",
                a.value.name
            );
        }
        let after_sources = BTreeMap::from([("m".to_owned(), elaborated.text.clone())]);
        assert_eq!(
            qcore_contract::contract_loader::checked_specifications(&after_sources).unwrap(),
            if krate == LIBRDKAFKA {
                BTreeSet::from(["rd_calloc".into(), "rd_strdup".into()])
            } else {
                BTreeSet::new()
            }
        );
        let before_sources = BTreeMap::from([("m".to_owned(), before.clone())]);
        assert_eq!(
            load_resource_contract_sources(&after_sources)
                .unwrap()
                .functions(),
            &constructor_result_evolution(
                load_resource_contract_sources(&before_sources)
                    .unwrap()
                    .functions()
                    .clone()
            ),
            "{module}"
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
        let chains: Vec<_> = evidence
            .inherited
            .iter()
            .map(|(child, chain)| {
                assert_eq!(chain.len(), 1, "{child}");
                let link = &chain[0];
                assert_eq!(link.module.crate_name, LIBC);
                assert_eq!(link.module.crate_version, "0.1.0");
                assert_eq!(
                    link.module.sha256,
                    sha256_hex(libc_text(&link.module.module).as_bytes())
                );
                (
                    child.as_str(),
                    link.declaration.as_str(),
                    link.module.module.as_str(),
                )
            })
            .collect();
        assert_eq!(chains, expected_chains, "{module}");
        converted += chains.len();
        // The module's own identity names the owning crate.
        assert_eq!(evidence.module.crate_name, krate);
    }
    assert_eq!(converted, 6); // Rule 37: two wrappers are checked specifications.
}

/// The Cycle 19 manifest selects both converted modules by reference; with the
/// historical modules in their place, only the two named result modes evolve;
/// ABI, literal and tail facts remain equal and retained chains are published.
#[test]
fn the_resource_manifest_selects_the_converted_modules_with_equal_meaning() {
    let contracts =
        load_resource_contract_manifest(&repository().join("cycle19-resource-contracts.json"))
            .unwrap();
    check_source_assumptions(&contracts).unwrap();
    let keys: Vec<_> = contracts.inclusion_evidence().keys().cloned().collect();
    assert_eq!(
        keys,
        [
            "qcore-provider-libevent:monkey/memory.qv",
            "qcore-provider-librdkafka:librdkafka/memory.qv",
            "quirin-provider-fluent-bit:fluent_bit/resource_memory.qv",
            "quirin-provider-fluent-bit:fluent_bit/resource_string.qv",
        ]
    );
    let provenance = inclusion_provenance(&contracts).unwrap();
    for child in ["rd_malloc", "rd_strndup", "rd_free"] {
        assert!(
            provenance["modules"]["qcore-provider-librdkafka:librdkafka/memory.qv"]["inherited"]
                [child]
                .is_array(),
            "{child}"
        );
    }
    let mut hand = contracts.source_assumptions().clone();
    for (krate, module, fixture_name, _) in CONVERSIONS {
        let key = format!("{krate}:{module}");
        assert!(hand.remove(&key).is_some(), "{key} is selected");
        assert!(
            hand.insert(format!("hand-written:{module}"), fixture(fixture_name))
                .is_none()
        );
    }
    let hand_loaded = load_resource_contract_sources(&hand).unwrap();
    assert_eq!(
        &constructor_result_evolution(hand_loaded.functions().clone()),
        contracts.functions()
    );
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

/// A scratch copy of a real package's contract layout: the crate's
/// `contracts.json` and `contracts/` tree, plus each pinned included parent
/// installed from the load path under `includes/<crate>/<module>`, as
/// `providers/package.py build` installs them.
struct ScratchPackage(PathBuf);

/// Recover the published fixture bytes when checkout newline conversion has
/// changed their spelling. The declared digest still selects the exact bytes:
/// any content change beyond CRLF conversion fails here, never changes a pin.
fn pinned_libc_text(pin: &serde_json::Value) -> String {
    let text = libc_text(pin["module"].as_str().unwrap());
    let expected = pin["sha256"].as_str().unwrap();
    if sha256_hex(text.as_bytes()) == expected {
        return text;
    }
    let published = text.replace("\r\n", "\n");
    assert_eq!(
        sha256_hex(published.as_bytes()),
        expected,
        "the published parent must match its exact inventory pin"
    );
    published
}

impl ScratchPackage {
    fn new(krate: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "qcore-inherit-qcore-provider-{krate}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let source = repository().join("providers").join(krate);
        // Card W45-TEST-HYGIENE: the package's tracked contract tree only.
        for path in tracked_files::tracked_files(&source.join("contracts")) {
            let child = root.join(path.strip_prefix(&source).unwrap());
            fs::create_dir_all(child.parent().unwrap()).unwrap();
            fs::write(child, fs::read(&path).unwrap()).unwrap();
        }
        fs::write(
            root.join("contracts.json"),
            fs::read(source.join("contracts.json")).unwrap(),
        )
        .unwrap();
        let scratch = Self(root);
        for (_, pin) in scratch.pins() {
            let text = pinned_libc_text(&pin);
            let target = scratch.include_path(&pin);
            fs::create_dir_all(target.parent().unwrap()).unwrap();
            fs::write(target, text).unwrap();
        }
        scratch
    }

    fn inventory(&self) -> serde_json::Value {
        serde_json::from_slice(&fs::read(self.0.join("contracts.json")).unwrap()).unwrap()
    }

    fn pins(&self) -> Vec<(String, serde_json::Value)> {
        self.inventory()["includes"]
            .as_object()
            .expect("the inventory pins its included parents")
            .iter()
            .map(|(key, pin)| (key.clone(), pin.clone()))
            .collect()
    }

    fn include_path(&self, pin: &serde_json::Value) -> PathBuf {
        self.0
            .join("includes")
            .join(pin["crate"].as_str().unwrap())
            .join(pin["module"].as_str().unwrap())
    }

    fn refused(&self, owner: &str, what: &str) -> String {
        match self.load(owner) {
            Ok(_) => panic!("{what} must be refused"),
            Err(error) => error,
        }
    }

    fn load(&self, owner: &str) -> Result<ContractBundle, String> {
        let manifest = self.0.join("contracts.json");
        let digest = sha256_hex(&fs::read(&manifest).unwrap());
        ContractBundle::load(&manifest, &digest, owner)
    }
}

impl Drop for ScratchPackage {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// The real inventories of `qcore.librdkafka` and `qcore.libevent` pin every
/// libc module their contracts reach, at the load path's pins; the Rust
/// `ContractBundle` admits the real layout and serves the pinned texts, and
/// refuses a changed parent and a missing one. Each mutation is undone by
/// rewriting the original bytes and the round trip is asserted.
#[test]
fn real_package_inventories_pin_their_included_parents() {
    let manifest = real_load_path();
    let libc = manifest.load_path.crate_named(LIBC).unwrap();
    for (krate, owner, expected) in [
        (
            LIBRDKAFKA,
            "qcore.librdkafka",
            &["libc/memory.qv", "libc/string.qv"][..],
        ),
        (LIBEVENT, "qcore.libevent", &["libc/memory.qv"][..]),
    ] {
        let scratch = ScratchPackage::new(krate);
        let pins = scratch.pins();
        let modules: Vec<_> = pins
            .iter()
            .map(|(_, pin)| pin["module"].as_str().unwrap().to_owned())
            .collect();
        assert_eq!(modules, expected, "{krate}");
        for (key, pin) in &pins {
            assert_eq!(pin["crate"], LIBC);
            assert_eq!(pin["version"], "0.1.0");
            let module = pin["module"].as_str().unwrap();
            assert_eq!(*key, format!("{LIBC}:{module}"));
            // The package pin is the load path's pin: one identity per parent.
            let text = pinned_libc_text(pin);
            assert_eq!(pin["sha256"], sha256_hex(text.as_bytes()));
            assert!(libc.published.contains(module));
        }
        let bundle = scratch
            .load(owner)
            .unwrap_or_else(|e| panic!("{krate}: {e}"));
        for (key, pin) in &pins {
            assert_eq!(bundle.included(key).unwrap(), pinned_libc_text(pin));
        }
        for (key, pin) in &pins {
            let path = scratch.include_path(pin);
            let original = fs::read(&path).unwrap();
            let mut changed = original.clone();
            changed.extend_from_slice(b"// changed\n");
            fs::write(&path, &changed).unwrap();
            let error = scratch.refused(owner, "a changed parent");
            assert!(error.contains("included parent changed"), "{key}: {error}");
            fs::remove_file(&path).unwrap();
            let error = scratch.refused(owner, "a missing parent");
            assert!(
                error.contains("included parent file set differs"),
                "{key}: {error}"
            );
            fs::write(&path, &original).unwrap();
            assert_eq!(fs::read(&path).unwrap(), original, "{key}: round trip");
            scratch.load(owner).unwrap();
        }
    }
}

// ---------------------------------------------------------------------------
// CONTRACT-INHERIT-ALIAS-RESULT-PARENTS
// ---------------------------------------------------------------------------

const APP: &str = "quirin-provider-app";
const ALIAS_MODULE: &str = "app/alias.qcore";

/// The real libc crate (its real `libc/string.qv` and `libc/time.qv`
/// texts, with `libc/time.qv` additionally published **in this test's copy
/// only**, for the index-1 base of `gmtime_r`) and one application crate that
/// depends on it and owns `app/alias.qcore`.
fn alias_load_path(child: &str) -> ContractLoadPath {
    let manifest = real_load_path();
    let mut libc = manifest.load_path.crate_named(LIBC).unwrap().clone();
    libc.published.insert("libc/time.qv".to_owned());
    let app = PublishedContractCrate {
        name: APP.into(),
        namespace: "app".into(),
        version: "0.1.0".into(),
        dependencies: [LIBC.to_owned()].into(),
        modules: [(ALIAS_MODULE.to_owned(), child.to_owned())].into(),
        published: BTreeSet::new(),
    };
    ContractLoadPath::new([libc, app]).unwrap()
}

fn load_alias(child: &str) -> Result<qcore_schemas::contract::ResourceContracts, String> {
    let load_path = alias_load_path(child);
    load_published_resource_contracts(
        &load_path,
        &[
            PublishedModuleRef::new(APP, ALIAS_MODULE),
            PublishedModuleRef::new(LIBC, "libc/string.qv"),
            PublishedModuleRef::new(LIBC, "libc/time.qv"),
        ],
    )
}

const ALIAS_CHILDREN: &str = "module contract;\n\
#include \"quirin-provider-libc:libc/string.qv\";\n\
#include \"quirin-provider-libc:libc/time.qv\";\n\
extern fn app_copy(to: ptr<mut, u8>, from: ptr<const, u8>, count: usize) -> ptr<mut, u8> contract inherit libc::memcpy;\n\
extern fn app_find(text: ptr<const, u8>, wanted: i32) -> ptr<mut, u8> contract inherit libc::strchr;\n\
extern fn app_search(hay: ptr<const, u8>, pin: ptr<const, u8>) -> ptr<mut, u8> contract inherit libc::strstr;\n\
extern fn app_gmtime(clock: ptr<const, u8>, out: ptr<mut, u8>) -> ptr<mut, u8> contract inherit libc::gmtime_r;\n";

/// An exact-alias and an interior-pointer parent, each inherited under renamed
/// parameters: the loaded child record **equals** the parent's, and the
/// derived result keeps the parent's base **index** (0 for `memcpy`,
/// `strchr` and `strstr`; 1 for `gmtime_r`), with its extent, nullability and
/// `const_dropped` flag. The renamed text names the child's own base.
#[test]
fn alias_and_interior_parents_are_inherited_with_their_base_index() {
    let loaded = load_alias(ALIAS_CHILDREN).unwrap_or_else(|e| panic!("{e}"));
    for (child, parent, parameter, extent, nullable, const_dropped) in [
        ("app_copy", "memcpy", 0, DerivedExtent::Exact, false, false),
        ("app_find", "strchr", 0, DerivedExtent::Interior, true, true),
        (
            "app_search",
            "strstr",
            0,
            DerivedExtent::Interior,
            true,
            true,
        ),
        (
            "app_gmtime",
            "gmtime_r",
            1,
            DerivedExtent::Exact,
            true,
            false,
        ),
    ] {
        let child_contract = loaded.contract(child).unwrap();
        assert_eq!(Some(child_contract), loaded.contract(parent), "{child}");
        assert_eq!(
            child_contract.result,
            ResourceReturnMode::Derived {
                parameter,
                extent,
                nullable,
                const_dropped
            },
            "{child}"
        );
    }
    let key = format!("{APP}:{ALIAS_MODULE}");
    let text = &loaded.source_assumptions()[&key];
    assert!(
        text.contains("resource_return_alias(to, nonnull)"),
        "{text}"
    );
    assert!(
        text.contains("resource_return_interior(text, nullable)"),
        "{text}"
    );
    assert!(
        text.contains("resource_return_alias(out, nullable)"),
        "{text}"
    );
    let chains = &loaded.inclusion_evidence()[&key].inherited;
    assert_eq!(chains["app_copy"][0].declaration, "memcpy");
    assert_eq!(chains["app_gmtime"][0].module.module, "libc/time.qv");
    check_source_assumptions(&loaded).unwrap();
}

fn refused(child: &str, code: &str) -> String {
    let error = load_alias(child).expect_err("refused");
    assert!(error.starts_with(code), "{code}: {error}");
    error
}

/// A child that reorders its parameters is refused, never re-bound by name:
/// binding is by position, and a reordered signature disagrees with the
/// parent's at the first moved position (`QI010`). Each derived parent is
/// tried with its base moved.
#[test]
fn a_child_that_reorders_the_parameters_is_refused() {
    for child in [
        "extern fn app_copy(from: ptr<const, u8>, to: ptr<mut, u8>, count: usize) -> ptr<mut, u8> contract inherit libc::memcpy;",
        "extern fn app_copy(count: usize, to: ptr<mut, u8>, from: ptr<const, u8>) -> ptr<mut, u8> contract inherit libc::memcpy;",
        "extern fn app_find(wanted: i32, text: ptr<const, u8>) -> ptr<mut, u8> contract inherit libc::strchr;",
        "extern fn app_gmtime(out: ptr<mut, u8>, clock: ptr<const, u8>) -> ptr<mut, u8> contract inherit libc::gmtime_r;",
    ] {
        let module = format!(
            "module contract;\n#include \"quirin-provider-libc:libc/string.qv\";\n#include \"quirin-provider-libc:libc/time.qv\";\n{child}\n"
        );
        refused(&module, "QI010");
    }
}

/// Binding is by position, so two same-typed parameters whose **names** are
/// swapped relative to the parent's are not a reordering the loader can see:
/// the child's first parameter is the base, whatever it is called. The loaded
/// record keeps base index 0. This is the positional rule C15 states, recorded
/// here so that nobody reads a parameter name as re-binding the base
/// (`CONTRACT-INHERIT-PERMUTED-PARAMETER-NAMES`).
#[test]
fn swapped_parameter_names_do_not_move_the_base() {
    let module = "module contract;\n\
#include \"quirin-provider-libc:libc/string.qv\";\n\
extern fn app_search(needle: ptr<const, u8>, haystack: ptr<const, u8>) -> ptr<mut, u8> contract inherit libc::strstr;\n";
    let loaded = load_alias(module).unwrap();
    let child = loaded.contract("app_search").unwrap();
    assert_eq!(Some(child), loaded.contract("strstr"));
    assert!(matches!(
        child.result,
        ResourceReturnMode::Derived { parameter: 0, .. }
    ));
    let text = &loaded.source_assumptions()[&format!("{APP}:{ALIAS_MODULE}")];
    assert!(text.contains("resource_return_interior(needle, nullable)"));
}

/// The predicates' second argument is a word (`nonnull`, `nullable`) that is
/// not a parameter. A child parameter spelled like it would be captured by
/// the renaming, and is refused (`QI012`) rather than renamed into a
/// different reading.
#[test]
fn a_child_parameter_spelled_like_the_second_argument_is_captured() {
    for (child, word) in [
        (
            "extern fn app_copy(nonnull: ptr<mut, u8>, from: ptr<const, u8>, count: usize) -> ptr<mut, u8> contract inherit libc::memcpy;",
            "nonnull",
        ),
        (
            "extern fn app_find(text: ptr<const, u8>, nullable: i32) -> ptr<mut, u8> contract inherit libc::strchr;",
            "nullable",
        ),
    ] {
        let module = format!(
            "module contract;\n#include \"quirin-provider-libc:libc/string.qv\";\n{child}\n"
        );
        let error = refused(&module, "QI012");
        assert!(error.contains(word), "{error}");
    }
}
