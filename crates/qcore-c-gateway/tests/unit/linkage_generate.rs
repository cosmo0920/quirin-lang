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

//! Card W48-QRLIB-LINKAGE: the generator, the stale-manifest refusal and the
//! link plan, over the checked-in reader fixtures.
use std::collections::BTreeMap;

use qcore_core::native_library::{
    ExportBinding, LinkageKind, RuntimePremise, load_native_library_manifest,
};
use qcore_core::scalar_types::{PointerWidth, RecordedTarget};

use super::*;
use crate::linkage::{SymbolRefusal, link};

const ELF_DEMO: &[u8] = include_bytes!("../fixtures/linkage/elf/libqr_demo.a");
const ELF_VERSIONED: &[u8] = include_bytes!("../fixtures/linkage/elf/libqr_versioned.so");
const COFF_IMPORT: &[u8] = include_bytes!("../fixtures/linkage/coff/qr_import.lib");

fn target(triple: &str) -> RecordedTarget {
    RecordedTarget {
        pointer_width: PointerWidth::W64,
        triple: triple.to_owned(),
        compiler: "clang version 21".to_owned(),
    }
}

fn loader(entries: &[(&str, &[u8])]) -> impl Fn(&str) -> Result<Vec<u8>, String> + use<> {
    let map: BTreeMap<String, Vec<u8>> = entries
        .iter()
        .map(|(name, bytes)| ((*name).to_owned(), bytes.to_vec()))
        .collect();
    move |name: &str| {
        map.get(name)
            .cloned()
            .ok_or_else(|| format!("no file `{name}`"))
    }
}

fn scalar_contract(name: &str, parameters: &[(&str, &str)], result: &str) -> String {
    let mut text = format!(
        "extern fn {name}({}) -> {result}\ncontract {{\n    requires resource_contract_v1;\n",
        parameters
            .iter()
            .map(|(p, ty)| format!("{p}: {ty}"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    for (p, _) in parameters {
        text.push_str(&format!("    ensures normal resource_borrowed({p});\n"));
    }
    text.push_str("    ensures normal resource_return_none();\n};\n");
    text
}

fn module(declarations: &[String]) -> String {
    format!("module contract;\n\n{}", declarations.join("\n"))
}

fn inputs<'a>(
    linkage: LinkageKind,
    artifact: &'a str,
    target: &'a RecordedTarget,
    contracts: &'a BTreeMap<String, String>,
) -> GenerationInputs<'a> {
    GenerationInputs {
        library: "qr_demo",
        version: "0.1.0",
        linkage,
        artifact,
        header: None,
        target,
        unwinding: "abort",
        allocator_custody: "closed",
        thread_posture: "single_caller",
        contracts,
        os_build: None,
    }
}

fn demo_contracts() -> BTreeMap<String, String> {
    BTreeMap::from([(
        "demo.qv".to_owned(),
        module(&[
            scalar_contract("qr_add", &[("a", "i32"), ("b", "i32")], "i32"),
            scalar_contract("qr_weak", &[], "i32"),
        ]),
    )])
}

#[test]
fn a_static_manifest_is_generated_from_the_archive_and_reads_back() {
    let target = target("x86_64-pc-linux-gnu");
    let contracts = demo_contracts();
    let files = loader(&[("lib/libqr_demo.a", ELF_DEMO)]);
    let generated = generate(
        &inputs(LinkageKind::Static, "lib/libqr_demo.a", &target, &contracts),
        &files,
        None,
    )
    .unwrap();
    let digest = format!("sha256:{}", qcore_schemas::digest::sha256_hex(ELF_DEMO));
    assert_eq!(
        generated.text,
        format!(
            "native_library manifest v2;\n\
             # Generated on every build from the artifact's bytes and the build's inputs; never edited by hand.\n\
             library qr_demo;\nversion 0.1.0;\nlinkage static;\nformat elf;\ntarget x86_64-pc-linux-gnu;\n\
             artifact lib/libqr_demo.a;\nhash {digest};\ndata_model lp64;\nunwinding abort;\n\
             allocator_custody closed;\nthread_posture single_caller;\n\
             contract fn qr_add(int32_t, int32_t) -> int32_t from demo.qv export qr_add global in lib/libqr_demo.a(qr_ok.o);\n\
             contract fn qr_weak() -> int32_t from demo.qv export qr_weak weak in lib/libqr_demo.a(qr_ok.o);\n"
        )
    );
    assert_eq!(
        load_native_library_manifest(&generated.text).unwrap(),
        generated.manifest
    );
    let weak = &generated.manifest.contracted["qr_weak"];
    assert_eq!(weak.export.as_ref().unwrap().binding, ExportBinding::Weak);
}

#[test]
fn a_stored_manifest_that_differs_from_the_regenerated_one_is_stale() {
    let target = target("x86_64-pc-linux-gnu");
    let contracts = demo_contracts();
    let files = loader(&[("libqr_demo.a", ELF_DEMO)]);
    let inputs = inputs(LinkageKind::Static, "libqr_demo.a", &target, &contracts);
    let fresh = generate(&inputs, &files, None).unwrap();
    assert!(check_stale(&fresh.text, &inputs, &files, None).is_ok());
    // An edited hash, an edited binding, and a stored manifest the artifact has
    // moved under since it was generated.
    let edited = fresh.text.replace("weak in", "global in");
    assert!(matches!(
        check_stale(&edited, &inputs, &files, None),
        Err(GenerationRefusal::Stale { line: 15, .. })
    ));
    let mut rebuilt = ELF_DEMO.to_vec();
    let last = rebuilt.len() - 1;
    rebuilt[last] ^= 0x01;
    let moved = loader(&[("libqr_demo.a", &rebuilt)]);
    let refusal = check_stale(&fresh.text, &inputs, &moved, None).unwrap_err();
    assert!(
        matches!(refusal, GenerationRefusal::Stale { line: 9, .. }),
        "{refusal}"
    );
}

#[test]
fn a_manifest_naming_a_symbol_the_archive_does_not_export_is_not_generated() {
    let target = target("x86_64-pc-linux-gnu");
    let files = loader(&[("libqr_demo.a", ELF_DEMO)]);
    for (symbol, expected) in [
        ("qr_stripped", "absent or stripped"),
        ("qr_localized", "local symbol"),
        ("qr_undefined_elsewhere", "never defined"),
        ("qr_dup", "more than one linkable definition"),
    ] {
        let contracts = BTreeMap::from([(
            "demo.qv".to_owned(),
            module(&[scalar_contract(symbol, &[], "i32")]),
        )]);
        let refusal = generate(
            &inputs(LinkageKind::Static, "libqr_demo.a", &target, &contracts),
            &files,
            None,
        )
        .unwrap_err();
        assert!(
            refusal.to_string().contains(expected),
            "{symbol}: {refusal}"
        );
    }
}

#[test]
fn the_declared_linkage_kind_must_be_the_artifacts() {
    let target = target("x86_64-pc-linux-gnu");
    let contracts = demo_contracts();
    let files = loader(&[("libqr_demo.a", ELF_DEMO)]);
    assert!(matches!(
        generate(
            &inputs(LinkageKind::Dynamic, "libqr_demo.a", &target, &contracts),
            &files,
            None
        ),
        Err(GenerationRefusal::LinkageKind { .. })
    ));
}

#[test]
fn a_dynamic_elf_manifest_records_the_version_weakness_and_soname() {
    let target = target("x86_64-pc-linux-gnu");
    let contracts = BTreeMap::from([(
        "v.qv".to_owned(),
        module(&[
            scalar_contract("qr_old", &[], "i32"),
            scalar_contract("qr_soft", &[], "i32"),
        ]),
    )]);
    let files = loader(&[("libqr_versioned.so", ELF_VERSIONED)]);
    let generated = generate(
        &inputs(
            LinkageKind::Dynamic,
            "libqr_versioned.so",
            &target,
            &contracts,
        ),
        &files,
        None,
    )
    .unwrap();
    assert!(generated.text.contains(
        "contract fn qr_old() -> int32_t from v.qv export qr_old version VERS_2 global in libqr_versioned.so;"
    ));
    assert!(generated.text.contains(
        "contract fn qr_soft() -> int32_t from v.qv export qr_soft version VERS_1 weak in libqr_versioned.so;"
    ));
    assert!(
        generated
            .text
            .contains("runtime soname libqr_versioned.so.1 in libqr_versioned.so;")
    );
    // A symbol with only a hidden version is not generated.
    let hidden = BTreeMap::from([(
        "v.qv".to_owned(),
        module(&[scalar_contract("qr_legacy", &[], "i32")]),
    )]);
    let refusal = generate(
        &inputs(LinkageKind::Dynamic, "libqr_versioned.so", &target, &hidden),
        &files,
        None,
    )
    .unwrap_err();
    assert!(
        matches!(
            refusal,
            GenerationRefusal::Symbol {
                refusal: SymbolRefusal::HiddenVersionOnly { .. },
                ..
            }
        ),
        "{refusal}"
    );
}

#[test]
fn a_windows_import_is_by_name_and_its_runtime_export_is_not_a_forwarder() {
    let target = target("x86_64-pc-windows-msvc");
    let files = loader(&[("qr_import.lib", COFF_IMPORT)]);
    let pe = crate::linkage::tests::pe_bytes();
    let runtime = loader(&[("qr_runtime.dll", &pe)]);
    let generate_for = |symbol: &str, os_build: Option<&str>| {
        let contracts = BTreeMap::from([(
            "rt.qv".to_owned(),
            module(&[scalar_contract(symbol, &[("status", "i32")], "u32")]),
        )]);
        let mut request = inputs(LinkageKind::Dynamic, "qr_import.lib", &target, &contracts);
        request.os_build = os_build;
        generate(&request, &files, Some(&runtime))
    };
    let generated = generate_for("qr_by_name", Some("10.0.20348")).unwrap();
    assert!(generated.text.contains("data_model llp64;"));
    assert!(
        generated.text.contains(
            "export qr_by_name global dll qr_runtime.dll in qr_import.lib(qr_runtime.dll);"
        )
    );
    assert!(
        generated
            .text
            .contains("runtime dll qr_runtime.dll os_build 10.0.20348;")
    );
    assert_eq!(
        generated.manifest.linkage.as_ref().unwrap().runtime,
        vec![RuntimePremise::Dll {
            name: "qr_runtime.dll".into(),
            os_build: "10.0.20348".into()
        }]
    );
    for (symbol, expected) in [
        ("qr_by_ordinal", "by ordinal only"),
        ("qr_forwarded", "forwarder to `other.qr_target`"),
        ("qr_gone", "does not export `qr_gone` by name"),
    ] {
        let refusal = generate_for(symbol, Some("10.0.20348")).unwrap_err();
        assert!(
            refusal.to_string().contains(expected),
            "{symbol}: {refusal}"
        );
    }
    assert!(matches!(
        generate_for("qr_by_name", None),
        Err(GenerationRefusal::Runtime(_))
    ));
}

#[test]
fn a_struct_position_needs_a_header_and_is_spelled_with_its_tag() {
    let target = target("x86_64-pc-linux-gnu");
    let files = loader(&[("libqr_demo.a", ELF_DEMO)]);
    let contracts = BTreeMap::from([(
        "p.qv".to_owned(),
        "module contract;\n\n[| quant #c_ffi |]\nextern fn qr_add(point: ptr<const, c\"struct qr_point\">, dx: i32) -> c\"struct qr_point\"\ncontract {\n    requires resource_contract_v1;\n    ensures normal resource_borrowed(point);\n    ensures normal resource_borrowed(dx);\n    ensures normal resource_return_none();\n};\n".to_owned(),
    )]);
    let request = inputs(LinkageKind::Static, "libqr_demo.a", &target, &contracts);
    assert!(matches!(
        generate(&request, &files, None),
        Err(GenerationRefusal::StructWithoutHeader { .. })
    ));
    let header = b"struct qr_point { int x; };\n";
    let mut with_header = request.clone();
    with_header.header = Some(("qr_point.h", header));
    let generated = generate(&with_header, &files, None).unwrap();
    assert!(generated.text.contains(
        "contract fn qr_add(const struct qr_point *, int32_t) -> struct qr_point from p.qv export qr_add global in libqr_demo.a(qr_ok.o);"
    ));
    assert!(generated.text.contains(&format!(
        "header_hash sha256:{};",
        qcore_schemas::digest::sha256_hex(header)
    )));
}

#[test]
fn a_declared_input_the_manifest_cannot_state_is_refused() {
    let target = target("x86_64-pc-linux-gnu");
    let contracts = demo_contracts();
    let files = loader(&[("libqr_demo.a", ELF_DEMO)]);
    let mut request = inputs(LinkageKind::Static, "libqr_demo.a", &target, &contracts);
    request.library = "qr demo";
    assert!(matches!(
        generate(&request, &files, None),
        Err(GenerationRefusal::Input { .. })
    ));
    let mut request = inputs(LinkageKind::Static, "libqr_demo.a", &target, &contracts);
    request.unwinding = "unwind";
    assert!(generate(&request, &files, None).is_err());
    let unknown = target_for("riscv64-unknown-linux-gnu");
    let request = inputs(LinkageKind::Static, "libqr_demo.a", &unknown, &contracts);
    assert!(matches!(
        generate(&request, &files, None),
        Err(GenerationRefusal::Target { .. })
    ));
}

fn target_for(triple: &str) -> RecordedTarget {
    target(triple)
}

#[test]
fn the_link_plan_hashes_every_pinned_file_and_admits_only_the_artifact() {
    let root = std::env::temp_dir().join(format!("qcore-w48-qrlib-link-{}", std::process::id()));
    std::fs::create_dir_all(root.join("lib")).unwrap();
    let archive = root.join("lib/libqr_demo.a");
    std::fs::write(&archive, ELF_DEMO).unwrap();
    let script = root.join("libqr.a");
    std::fs::write(&script, format!("GROUP ( {} )\n", archive.display())).unwrap();
    let target = target("x86_64-pc-linux-gnu");
    let contracts = demo_contracts();
    let script_ref = script.display().to_string();
    let read = |name: &str| std::fs::read(name).map_err(|e| e.to_string());
    let generated = generate(
        &inputs(LinkageKind::Static, &script_ref, &target, &contracts),
        &read,
        None,
    )
    .unwrap();
    assert!(generated.text.contains(&format!(
        "input {} sha256:{};",
        archive.display(),
        qcore_schemas::digest::sha256_hex(ELF_DEMO)
    )));
    let resolve = |reference: &str| std::path::PathBuf::from(reference);
    let plan = link::plan(&generated.manifest, &resolve).unwrap();
    assert_eq!(plan.input, script);
    assert_eq!(plan.verified.len(), 2);
    assert!(plan.admit(&script).is_ok());
    assert!(matches!(
        plan.admit(&archive),
        Err(link::LinkRefusal::OtherArtifact { .. })
    ));
    // The archive the script names changed after generation: refused.
    let mut rebuilt = ELF_DEMO.to_vec();
    let last = rebuilt.len() - 1;
    rebuilt[last] ^= 0x01;
    std::fs::write(&archive, &rebuilt).unwrap();
    assert!(matches!(
        link::plan(&generated.manifest, &resolve),
        Err(link::LinkRefusal::DigestMismatch { .. })
    ));
    std::fs::write(&archive, ELF_DEMO).unwrap();
    // The script now names another file: refused, even with its own hash
    // restored to the pinned one is impossible, so the digest check fires first.
    std::fs::write(&script, "GROUP ( /elsewhere/libqr_demo.a )\n").unwrap();
    assert!(link::plan(&generated.manifest, &resolve).is_err());
    let _ = std::fs::remove_dir_all(&root);
}
