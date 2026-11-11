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

//! Card `W48-QRLIB-LINKAGE`: the generated manifest (`native_library manifest
//! v2`) — its linkage section, the per-entry `export` clause, the struct and
//! float spellings, and a named refusal for every grammar violation.
use super::*;

const DIGEST: &str = "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const ARCHIVE_DIGEST: &str =
    "sha256:1111111111111111111111111111111111111111111111111111111111111111";

fn v1_text(body: &str) -> String {
    format!(
        "native_library manifest v1;\nlibrary demo;\nversion 0.1.0;\nartifact libdemo.a;\nhash {DIGEST};\ndata_model lp64;\nunwinding abort;\nallocator_custody closed;\nthread_posture single_caller;\n{body}\n"
    )
}

fn generated_text(linkage: &str, runtime: &str, data_model: &str, entries: &str) -> String {
    format!(
        "native_library manifest v2;\nlibrary demo;\nversion 0.1.0;\nlinkage {linkage};\nformat elf;\ntarget x86_64-pc-linux-gnu;\nartifact libdemo.a;\nhash {DIGEST};\ninput /usr/lib/libdemo-1.a {ARCHIVE_DIGEST};\n{runtime}data_model {data_model};\nunwinding abort;\nallocator_custody closed;\nthread_posture single_caller;\n{entries}\n"
    )
}

const STATIC_ENTRIES: &str = "contract fn qr_add(int32_t, int32_t) -> int32_t from demo.qv export qr_add global in libdemo.a(qr.o);\ncontract fn qr_scale(double, float) -> double from demo.qv export qr_scale weak in libdemo.a(qr.o);\ncontract fn qr_sum(const struct qr_pair *) -> struct qr_pair from demo.qv export qr_sum global in libdemo.a(qr.o);";

const ENTRY: &str =
    "contract fn qr_add(int32_t) -> int32_t from demo.qv export qr_add global in libdemo.a(qr.o);";

#[test]
fn a_generated_manifest_loads_its_linkage_and_renders_back_byte_for_byte() {
    let text = generated_text("static", "", "lp64", STATIC_ENTRIES);
    let manifest = load_native_library_manifest(&text).unwrap();
    let linkage = manifest.linkage.as_ref().unwrap();
    assert_eq!(linkage.kind, LinkageKind::Static);
    assert_eq!(linkage.format, "elf");
    assert_eq!(linkage.target, "x86_64-pc-linux-gnu");
    assert_eq!(
        linkage.inputs,
        vec![LinkedInput {
            reference: "/usr/lib/libdemo-1.a".into(),
            digest: ARCHIVE_DIGEST.into(),
            as_needed: false
        }]
    );
    assert!(linkage.runtime.is_empty());
    let scale = &manifest.contracted["qr_scale"];
    assert_eq!(
        scale.parameters,
        vec![EntryType::Abi(AbiType::F64), EntryType::Abi(AbiType::F32)]
    );
    let export = scale.export.as_ref().unwrap();
    assert_eq!(export.binding, ExportBinding::Weak);
    assert_eq!(export.file, "libdemo.a(qr.o)");
    let sum = &manifest.contracted["qr_sum"];
    assert_eq!(
        sum.parameters,
        vec![EntryType::StructPointer {
            tag: "qr_pair".into(),
            constant: true
        }]
    );
    assert_eq!(sum.result, EntryType::Struct("qr_pair".into()));
    assert_eq!(sum.abi_signature(), None);
    // The canonical rendering reads back to the same manifest and renders to
    // the same text again: the text is a function of the manifest.
    let rendered = render_manifest(&manifest).unwrap();
    let reread = load_native_library_manifest(&rendered).unwrap();
    assert_eq!(reread, manifest);
    assert_eq!(render_manifest(&reread).unwrap(), rendered);
    // A v1 manifest has no canonical rendering: it is hand-written.
    let v1 = load_native_library_manifest(&v1_text("unchecked fn k(int32_t) -> void;")).unwrap();
    assert_eq!(render_manifest(&v1), None);
}

#[test]
fn a_dynamic_manifest_names_its_runtime_and_versions() {
    let text = generated_text(
        "dynamic",
        "runtime soname libdemo.so.1 in /usr/lib/libdemo-1.a;\n",
        "lp64",
        "contract fn qr_v(int32_t) -> int32_t from demo.qv export qr_v version DEMO_2 weak ifunc in libdemo.so.1;",
    );
    let manifest = load_native_library_manifest(&text).unwrap();
    let linkage = manifest.linkage.as_ref().unwrap();
    assert_eq!(
        linkage.runtime,
        vec![RuntimePremise::Soname {
            soname: "libdemo.so.1".into(),
            file: "/usr/lib/libdemo-1.a".into()
        }]
    );
    let export = manifest.contracted["qr_v"].export.as_ref().unwrap();
    assert_eq!(export.version.as_deref(), Some("DEMO_2"));
    assert!(export.ifunc);
    for runtime in [
        "runtime install_name /usr/lib/libSystem.B.dylib current_version 1351;\n",
        "runtime dll ntdll.dll os_build 10.0.20348;\n",
    ] {
        let text = generated_text(
            "dynamic",
            runtime,
            "llp64",
            "contract fn qr_v(int32_t) -> int32_t from demo.qv export qr_v global dll ntdll.dll in ntdll.lib(ntdll.dll);",
        );
        let manifest = load_native_library_manifest(&text).unwrap();
        assert_eq!(manifest.premises.data_model, DataModel::Llp64);
        let rendered = render_manifest(&manifest).unwrap();
        assert_eq!(
            load_native_library_manifest(&rendered).unwrap(),
            manifest,
            "{runtime}"
        );
        assert!(rendered.contains(runtime), "{runtime}");
    }
}

#[test]
fn every_generated_grammar_violation_refuses_by_name() {
    for (text, expected) in [
        (
            generated_text(
                "static",
                "",
                "lp64",
                "contract fn qr_add(int32_t) -> int32_t from demo.qv;",
            ),
            ManifestRefusal::MissingExport {
                symbol: "qr_add".into(),
            },
        ),
        (
            generated_text(
                "static",
                "",
                "lp64",
                "unchecked fn qr_add(int32_t) -> int32_t export qr_add global in x;",
            ),
            ManifestRefusal::UncheckedInGenerated {
                symbol: "qr_add".into(),
            },
        ),
        (
            generated_text("dynamic", "", "lp64", ENTRY),
            ManifestRefusal::MissingRuntime,
        ),
        (
            generated_text(
                "static",
                "runtime soname libdemo.so.1 in x;\n",
                "lp64",
                ENTRY,
            ),
            ManifestRefusal::RuntimeInStaticLinkage,
        ),
        (
            generated_text(
                "static",
                "",
                "lp64",
                "contract fn qr_add(int32_t) -> int32_t from demo.qv export qr_add strong in x;",
            ),
            ManifestRefusal::MalformedExport {
                symbol: "qr_add".into(),
                clause: "qr_add strong in x".into(),
            },
        ),
        (
            generated_text("static", "", "ilp32", ENTRY),
            ManifestRefusal::UnknownValue {
                field: "data_model".into(),
                value: "ilp32".into(),
            },
        ),
        (
            generated_text("static", "runtime soname only;\n", "lp64", ENTRY),
            ManifestRefusal::MalformedRuntime {
                statement: "runtime soname only".into(),
            },
        ),
        (
            generated_text("static", "", "lp64", ENTRY).replace("linkage static;\n", ""),
            ManifestRefusal::MissingField { field: "linkage" },
        ),
        (
            generated_text("static", "", "lp64", ENTRY).replace("linkage static;", "linkage lazy;"),
            ManifestRefusal::UnknownValue {
                field: "linkage".into(),
                value: "lazy".into(),
            },
        ),
    ] {
        assert_eq!(load_native_library_manifest(&text), Err(expected), "{text}");
    }
    let bad_input = generated_text("static", "", "lp64", ENTRY)
        .replace(&format!("{ARCHIVE_DIGEST};"), "md5:00;");
    assert!(matches!(
        load_native_library_manifest(&bad_input),
        Err(ManifestRefusal::MalformedHash { .. })
    ));
}

#[test]
fn the_two_grammars_keep_their_own_statements() {
    assert_eq!(
        load_native_library_manifest(&v1_text(
            "linkage static;\nunchecked fn k(int32_t) -> void;"
        )),
        Err(ManifestRefusal::WrongVersionField {
            field: "linkage".into(),
            version: "v1"
        })
    );
    let with_include = generated_text(
        "static",
        "",
        "lp64",
        "include <x.h> from x.h sha256:0000000000000000000000000000000000000000000000000000000000000000;",
    );
    assert!(matches!(
        load_native_library_manifest(&with_include),
        Err(ManifestRefusal::WrongVersionField { .. })
    ));
    // `llp64` and the struct spellings are a generated manifest's alone.
    assert!(matches!(
        load_native_library_manifest(
            &v1_text("unchecked fn k(int32_t) -> void;").replace("lp64", "llp64")
        ),
        Err(ManifestRefusal::UnknownValue { .. })
    ));
    assert!(matches!(
        load_native_library_manifest(&v1_text(
            "contract fn qr_sum(const struct qr_pair *) -> int32_t from demo.qv;"
        )),
        Err(ManifestRefusal::UnknownParameterSpelling { .. })
    ));
}

#[test]
fn float_and_double_are_spelled_in_both_grammars() {
    let manifest =
        load_native_library_manifest(&v1_text("unchecked fn k_half(float) -> double;")).unwrap();
    assert_eq!(manifest.unchecked["k_half"].parameters, vec![AbiType::F32]);
    assert_eq!(manifest.unchecked["k_half"].result, AbiType::F64);
}

#[test]
fn the_ffi_type_gate_compares_struct_positions_and_the_abi_gate_refuses_them() {
    let manifest =
        load_native_library_manifest(&generated_text("static", "", "lp64", STATIC_ENTRIES))
            .unwrap();
    let sum = &manifest.contracted["qr_sum"];
    let pointer = EntryType::StructPointer {
        tag: "qr_pair".into(),
        constant: true,
    };
    let value = EntryType::Struct("qr_pair".into());
    assert_eq!(
        check_ffi_agreement(sum, std::slice::from_ref(&pointer), &value),
        Ok(())
    );
    let other = EntryType::Struct("qr_other".into());
    assert_eq!(
        check_ffi_agreement(sum, &[pointer], &other)
            .unwrap_err()
            .to_string(),
        "the result is `struct qr_other` in the contract and `struct qr_pair` in the manifest"
    );
    assert_eq!(
        check_contracted_agreement(sum, &[AbiType::PtrConstU8], AbiType::I32),
        Err(ManifestDisagreement::StructEntry {
            symbol: "qr_sum".into()
        })
    );
}
