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

//! Card W48-QRLIB-LINKAGE: every platform's symbol reader, from bytes.
//!
//! The ELF, Mach-O and COFF archives and the ELF shared object are the
//! checked-in fixtures `docs/reports/w48-qrlib-linkage/make_fixtures.sh`
//! builds; a PE DLL and a Mach-O dylib are laid out here byte by byte, and the
//! text formats (`ld` scripts, `.tbd`) are written inline.
use std::collections::BTreeMap;

use super::artifact::{ArtifactRefusal, FileKind, Format, TargetFacts, read_artifact};
use super::*;

const ELF_DEMO: &[u8] = include_bytes!("../fixtures/linkage/elf/libqr_demo.a");
const ELF_NOINDEX: &[u8] = include_bytes!("../fixtures/linkage/elf/libqr_noindex.a");
const ELF_VERSIONED: &[u8] = include_bytes!("../fixtures/linkage/elf/libqr_versioned.so");
const MACHO_DEMO: &[u8] = include_bytes!("../fixtures/linkage/macho/libqr_demo.a");
const COFF_DEMO: &[u8] = include_bytes!("../fixtures/linkage/coff/qr_demo.lib");
const COFF_IMPORT: &[u8] = include_bytes!("../fixtures/linkage/coff/qr_import.lib");

fn elf() -> TargetFacts {
    TargetFacts::from_triple("x86_64-pc-linux-gnu").unwrap()
}

fn mac() -> TargetFacts {
    TargetFacts::from_triple("x86_64-apple-macos11").unwrap()
}

fn win() -> TargetFacts {
    TargetFacts::from_triple("x86_64-pc-windows-msvc").unwrap()
}

/// A loader over an in-memory file set.
pub(super) fn files(entries: &[(&str, &[u8])]) -> impl Fn(&str) -> Result<Vec<u8>, String> + use<> {
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

fn table(reference: &str, bytes: &[u8], target: &TargetFacts) -> SymbolTable {
    read_artifact(reference, &files(&[(reference, bytes)]), target)
        .unwrap()
        .table
}

// ---------------------------------------------------------------------------
// ELF
// ---------------------------------------------------------------------------

#[test]
fn an_elf_archive_admits_a_global_and_a_weak_definition_from_its_index() {
    let table = table("libqr_demo.a", ELF_DEMO, &elf());
    let add = resolve(&table, "qr_add").unwrap();
    assert_eq!(add.binding, Binding::Global);
    assert_eq!(add.origin, "libqr_demo.a(qr_ok.o)");
    assert_eq!(add.version, None);
    let weak = resolve(&table, "qr_weak").unwrap();
    assert_eq!(
        weak.binding,
        Binding::Weak,
        "a defined weak symbol is admitted, recorded weak"
    );
}

#[test]
fn an_elf_archive_refuses_stripped_local_undefined_and_duplicate_symbols() {
    let table = table("libqr_demo.a", ELF_DEMO, &elf());
    assert_eq!(
        resolve(&table, "qr_stripped"),
        Err(SymbolRefusal::Missing {
            symbol: "qr_stripped".into()
        })
    );
    assert_eq!(
        resolve(&table, "qr_localized"),
        Err(SymbolRefusal::LocalOnly {
            symbol: "qr_localized".into(),
            origins: vec!["libqr_demo.a(qr_ok.o)".into()]
        })
    );
    assert_eq!(
        resolve(&table, "qr_undefined_elsewhere"),
        Err(SymbolRefusal::Undefined {
            symbol: "qr_undefined_elsewhere".into(),
            origins: vec!["libqr_demo.a(qr_ok.o)".into()]
        })
    );
    assert_eq!(
        resolve(&table, "qr_dup"),
        Err(SymbolRefusal::Ambiguous {
            symbol: "qr_dup".into(),
            origins: vec![
                "libqr_demo.a(qr_dup_a.o)".into(),
                "libqr_demo.a(qr_dup_b.o)".into()
            ]
        })
    );
}

#[test]
fn an_archive_without_an_index_links_nothing() {
    let table = table("libqr_noindex.a", ELF_NOINDEX, &elf());
    assert_eq!(
        resolve(&table, "qr_add"),
        Err(SymbolRefusal::NotIndexed {
            symbol: "qr_add".into(),
            origin: "libqr_noindex.a(qr_ok.o)".into()
        })
    );
}

#[test]
fn the_gnu_index_is_read_and_points_at_members() {
    let archive = archive::read(ELF_DEMO).unwrap();
    assert_eq!(archive.index_kinds, vec![archive::IndexKind::Gnu32]);
    let names: Vec<&str> = archive.members.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(names, ["qr_ok.o", "qr_dup_a.o", "qr_dup_b.o"]);
    assert_eq!(archive.index["qr_dup"].len(), 2);
    assert!(!archive.index.contains_key("qr_localized"));
}

#[test]
fn an_elf_shared_object_records_versions_weakness_and_its_soname() {
    let shared = elf::shared_object(ELF_VERSIONED, "libqr_versioned.so").unwrap();
    assert_eq!(shared.soname.as_deref(), Some("libqr_versioned.so.1"));
    let v2 = resolve(&shared.table, "qr_v2").unwrap();
    assert_eq!(v2.version.as_deref(), Some("VERS_2"));
    assert_eq!(v2.binding, Binding::Global);
    // Two definitions of one name, `qr_old@VERS_1` (hidden) and
    // `qr_old@@VERS_2`: a link by name binds only the default.
    let old = resolve(&shared.table, "qr_old").unwrap();
    assert_eq!(old.version.as_deref(), Some("VERS_2"));
    let soft = resolve(&shared.table, "qr_soft").unwrap();
    assert_eq!(soft.binding, Binding::Weak);
    assert_eq!(soft.version.as_deref(), Some("VERS_1"));
    assert_eq!(
        resolve(&shared.table, "qr_legacy"),
        Err(SymbolRefusal::HiddenVersionOnly {
            symbol: "qr_legacy".into(),
            versions: vec!["VERS_1".into()]
        })
    );
    assert!(matches!(
        resolve(&shared.table, "qr_hidden"),
        Err(SymbolRefusal::Missing { .. })
    ));
}

#[test]
fn an_elf_file_for_another_machine_or_format_is_refused() {
    let refusal = read_artifact(
        "libqr_demo.a",
        &files(&[("libqr_demo.a", ELF_DEMO)]),
        &mac(),
    )
    .unwrap_err();
    assert!(
        matches!(refusal, ArtifactRefusal::WrongTarget { .. }),
        "{refusal}"
    );
    let arm = TargetFacts::from_triple("aarch64-unknown-linux-gnu").unwrap();
    let refusal =
        read_artifact("libqr_demo.a", &files(&[("libqr_demo.a", ELF_DEMO)]), &arm).unwrap_err();
    assert!(
        matches!(refusal, ArtifactRefusal::WrongTarget { .. }),
        "{refusal}"
    );
}

#[test]
fn a_truncated_artifact_is_a_named_read_error_never_a_panic() {
    for length in [0, 7, 8, 30, 68, 200, ELF_VERSIONED.len() / 2] {
        let bytes = &ELF_VERSIONED[..length.min(ELF_VERSIONED.len())];
        let _ = elf::shared_object(bytes, "cut.so");
        let bytes = &ELF_DEMO[..length.min(ELF_DEMO.len())];
        let _ = archive::read(bytes);
    }
    assert!(elf::shared_object(&ELF_VERSIONED[..100], "cut.so").is_err());
}

// ---------------------------------------------------------------------------
// GNU ld scripts
// ---------------------------------------------------------------------------

const LIBM_A: &str = "/* GNU ld script\n*/\nOUTPUT_FORMAT(elf64-x86-64)\nGROUP ( /usr/lib/x86_64-linux-gnu/libm-2.43.a /usr/lib/x86_64-linux-gnu/libmvec.a )\n";
const LIBM_SO: &str = "/* GNU ld script\n*/\nOUTPUT_FORMAT(elf64-x86-64)\nGROUP ( /usr/lib/x86_64-linux-gnu/libm.so.6  AS_NEEDED ( /usr/lib/x86_64-linux-gnu/libmvec.so.1 ) )\n";

#[test]
fn a_library_script_names_every_file_in_order() {
    assert!(ld_script::is_ld_script(LIBM_A.as_bytes()));
    let inputs = ld_script::read(LIBM_A.as_bytes()).unwrap();
    assert_eq!(
        inputs,
        vec![
            ld_script::ScriptInput {
                path: "/usr/lib/x86_64-linux-gnu/libm-2.43.a".into(),
                as_needed: false
            },
            ld_script::ScriptInput {
                path: "/usr/lib/x86_64-linux-gnu/libmvec.a".into(),
                as_needed: false
            },
        ]
    );
    let shared = ld_script::read(LIBM_SO.as_bytes()).unwrap();
    assert!(!shared[0].as_needed);
    assert!(shared[1].as_needed);
}

#[test]
fn a_script_naming_a_search_a_relative_path_or_another_command_is_refused() {
    for script in [
        "GROUP ( -lm )",
        "GROUP ( libm.a )",
        "SEARCH_DIR(/usr/lib)\nGROUP ( /x.a )",
        "INCLUDE other.ld",
        "GROUP ( /x.a ",
        "/* open",
        "OUTPUT_FORMAT(elf64-x86-64)",
    ] {
        assert!(ld_script::read(script.as_bytes()).is_err(), "{script}");
    }
}

#[test]
fn a_script_is_followed_and_every_file_it_names_is_pinned() {
    let script = "GROUP ( /lib/libqr_demo.a /lib/libqr_noindex.a )";
    let loader = files(&[
        ("libqr.a", script.as_bytes()),
        ("/lib/libqr_demo.a", ELF_DEMO),
        ("/lib/libqr_noindex.a", ELF_NOINDEX),
    ]);
    let read = read_artifact("libqr.a", &loader, &elf()).unwrap();
    assert!(!read.dynamic);
    let kinds: Vec<FileKind> = read.files.iter().map(|file| file.kind).collect();
    assert_eq!(
        kinds,
        [
            FileKind::LdScript,
            FileKind::StaticArchive,
            FileKind::StaticArchive
        ]
    );
    assert_eq!(
        read.files[1].digest,
        format!("sha256:{}", qcore_schemas::digest::sha256_hex(ELF_DEMO))
    );
    // The union: `qr_add` is now defined in both archives, one indexed and
    // one not — two definitions refuse, whichever a linker would pull.
    assert!(matches!(
        resolve(&read.table, "qr_add"),
        Err(SymbolRefusal::Ambiguous { .. })
    ));
    // So does a weak one: two definitions are two, whatever their binding.
    assert!(matches!(
        resolve(&read.table, "qr_weak"),
        Err(SymbolRefusal::Ambiguous { .. })
    ));
}

#[test]
fn a_script_mixing_static_and_shared_files_is_refused() {
    let script = "GROUP ( /lib/libqr_demo.a /lib/libqr_versioned.so )";
    let loader = files(&[
        ("libqr.a", script.as_bytes()),
        ("/lib/libqr_demo.a", ELF_DEMO),
        ("/lib/libqr_versioned.so", ELF_VERSIONED),
    ]);
    assert!(matches!(
        read_artifact("libqr.a", &loader, &elf()),
        Err(ArtifactRefusal::MixedScript { .. })
    ));
}

// ---------------------------------------------------------------------------
// Mach-O
// ---------------------------------------------------------------------------

#[test]
fn a_mach_o_archive_is_read_through_its_symdef_index() {
    let archive = archive::read(MACHO_DEMO).unwrap();
    assert!(
        archive
            .index_kinds
            .iter()
            .all(|kind| matches!(kind, archive::IndexKind::Bsd32 | archive::IndexKind::Bsd64))
    );
    assert!(!archive.index_kinds.is_empty());
    let table = table("libqr_demo.a", MACHO_DEMO, &mac());
    let add = resolve(&table, "_qr_add").unwrap();
    assert_eq!(add.binding, Binding::Global);
    assert_eq!(add.origin, "libqr_demo.a(qr_mac.o)");
    assert_eq!(resolve(&table, "_qr_weak").unwrap().binding, Binding::Weak);
    assert!(matches!(
        resolve(&table, "_qr_private"),
        Err(SymbolRefusal::LocalOnly { .. })
    ));
    assert!(matches!(
        resolve(&table, "_qr_undefined_elsewhere"),
        Err(SymbolRefusal::Undefined { .. })
    ));
    assert_eq!(mac().object_name("qr_add"), "_qr_add");
}

const TBD_V4: &str = "--- !tapi-tbd
tbd-version:     4
targets:         [ x86_64-macos, arm64-macos ]
install-name:    '/usr/lib/libSystem.B.dylib'
current-version: 1351
reexported-libraries:
  - targets:         [ x86_64-macos, arm64-macos ]
    libraries:       [ '/usr/lib/system/libsystem_m.dylib' ]
exports:
  - targets:         [ x86_64-macos, arm64-macos ]
    symbols:         [ _system_only ]
--- !tapi-tbd
tbd-version:     4
targets:         [ x86_64-macos, arm64-macos ]
install-name:    '/usr/lib/system/libsystem_m.dylib'
current-version: 3291
exports:
  - targets:         [ x86_64-macos, arm64-macos ]
    symbols:         [ _fabs, _ilogb,
                       _sqrt ]
    weak-symbols:    [ _weak_m ]
  - targets:         [ arm64-macos ]
    symbols:         [ _arm_only ]
--- !tapi-tbd
tbd-version:     4
targets:         [ x86_64-macos, arm64-macos ]
install-name:    '/usr/lib/system/libunrelated.dylib'
exports:
  - targets:         [ x86_64-macos, arm64-macos ]
    symbols:         [ _not_reexported ]
...
";

#[test]
fn a_tbd_stub_exports_what_its_first_document_and_its_reexports_list() {
    assert_eq!(
        tbd::tbd_target("x86_64-apple-darwin23.1.0").as_deref(),
        Some("x86_64-macos")
    );
    assert_eq!(
        tbd::tbd_target("arm64-apple-macosx14.0.0").as_deref(),
        Some("arm64-macos")
    );
    assert_eq!(tbd::tbd_target("x86_64-pc-linux-gnu"), None);
    let stub = tbd::read(TBD_V4.as_bytes(), "x86_64-macos").unwrap();
    assert_eq!(stub.install_name, "/usr/lib/libSystem.B.dylib");
    assert_eq!(stub.current_version.as_deref(), Some("1351"));
    let sqrt = resolve(&stub.table, "_sqrt").unwrap();
    assert_eq!(sqrt.origin, "/usr/lib/system/libsystem_m.dylib");
    assert_eq!(
        resolve(&stub.table, "_weak_m").unwrap().binding,
        Binding::Weak
    );
    assert!(resolve(&stub.table, "_system_only").is_ok());
    // Listed for another target only, or in a document nothing re-exports.
    assert!(resolve(&stub.table, "_arm_only").is_err());
    assert!(resolve(&stub.table, "_not_reexported").is_err());
    let arm = tbd::read(TBD_V4.as_bytes(), "arm64-macos").unwrap();
    assert!(resolve(&arm.table, "_arm_only").is_ok());
    assert!(tbd::read(TBD_V4.as_bytes(), "x86_64-maccatalyst").is_err());
}

/// A one-document stub listing `targets` with `_qr_probe` exported for
/// `exported`.
fn probe_stub(targets: &str, exported: &str) -> String {
    format!(
        "--- !tapi-tbd\ntbd-version:     4\ntargets:         [ {targets} ]\ninstall-name:    '/usr/lib/libSystem.B.dylib'\nexports:\n  - targets:         [ {exported} ]\n    symbols:         [ _qr_probe ]\n...\n"
    )
}

#[test]
fn an_arm64_link_reads_an_arm64e_slice_only_when_no_arm64_slice_is_listed() {
    // As Apple `ld` does (card W48-QRLIB-MACOS's probe): arm64e stands in for
    // an unlisted arm64, an exact arm64 slice hides the arm64e entries, and no
    // other architecture stands in.
    let only_e = probe_stub("arm64e-macos", "arm64e-macos");
    let stub = tbd::read(only_e.as_bytes(), "arm64-macos").unwrap();
    assert_eq!(stub.slice, "arm64e-macos");
    assert!(resolve(&stub.table, "_qr_probe").is_ok());
    let both = probe_stub("arm64-macos, arm64e-macos", "arm64e-macos");
    let stub = tbd::read(both.as_bytes(), "arm64-macos").unwrap();
    assert_eq!(stub.slice, "arm64-macos");
    assert!(resolve(&stub.table, "_qr_probe").is_err());
    let intel = probe_stub("x86_64-macos", "x86_64-macos");
    assert!(tbd::read(intel.as_bytes(), "arm64-macos").is_err());
    assert!(tbd::read(only_e.as_bytes(), "x86_64-macos").is_err());
    let exact = tbd::read(TBD_V4.as_bytes(), "arm64-macos").unwrap();
    assert_eq!(exact.slice, "arm64-macos");
    let v5 = r#"{"tapi_tbd_version": 5, "main_library": {"target_info": [{"target": "arm64e-macos"}], "install_names": [{"name": "/usr/lib/libSystem.B.dylib"}], "exported_symbols": [{"text": {"global": ["_qr_probe"]}}]}}"#;
    let stub = tbd::read(v5.as_bytes(), "arm64-macos").unwrap();
    assert_eq!(stub.slice, "arm64e-macos");
    assert!(resolve(&stub.table, "_qr_probe").is_ok());
}

/// The SDK's own `libm.tbd` (a link to `libSystem.tbd`) read for this host.
#[cfg(target_os = "macos")]
#[test]
fn the_sdk_libm_stub_exports_the_math_functions_through_libsystem_m() {
    let sdk = std::process::Command::new("xcrun")
        .arg("--show-sdk-path")
        .output()
        .unwrap();
    assert!(sdk.status.success(), "xcrun --show-sdk-path failed");
    let path = std::path::PathBuf::from(String::from_utf8(sdk.stdout).unwrap().trim())
        .join("usr/lib/libm.tbd");
    let data = std::fs::read(&path).unwrap();
    let triple = if cfg!(target_arch = "aarch64") {
        "arm64-apple-darwin"
    } else {
        "x86_64-apple-darwin"
    };
    let stub = tbd::read(&data, &tbd::tbd_target(triple).unwrap()).unwrap();
    assert_eq!(stub.install_name, "/usr/lib/libSystem.B.dylib");
    for name in ["_ilogb", "_sqrt", "_fabs", "_fmax", "_nextafter", "_floor"] {
        let symbol = resolve(&stub.table, name).unwrap_or_else(|refusal| {
            panic!("{name} in {} ({}): {refusal:?}", path.display(), stub.slice)
        });
        assert_eq!(symbol.origin, "/usr/lib/system/libsystem_m.dylib", "{name}");
    }
}

#[test]
fn a_tbd_stub_is_read_as_an_artifact_for_a_mac_target_only() {
    let loader = files(&[("libm.tbd", TBD_V4.as_bytes())]);
    let read = read_artifact("libm.tbd", &loader, &mac()).unwrap();
    assert!(read.dynamic);
    assert_eq!(read.files[0].kind, FileKind::TextStub);
    assert_eq!(
        read.install_name,
        Some(("/usr/lib/libSystem.B.dylib".into(), Some("1351".into())))
    );
    assert!(matches!(
        read_artifact("libm.tbd", &loader, &elf()),
        Err(ArtifactRefusal::WrongTarget { .. })
    ));
}

#[test]
fn a_v3_and_a_v5_stub_are_read_and_an_unknown_version_refused() {
    let v3 = "--- !tapi-tbd-v3\narchs:           [ x86_64 ]\nplatform:        macosx\ninstall-name:    /usr/lib/libz.1.dylib\ncurrent-version: 1.2.12\nexports:\n  - archs:           [ x86_64 ]\n    symbols:         [ _deflate ]\n    weak-def-symbols: [ _zweak ]\n...\n";
    let stub = tbd::read(v3.as_bytes(), "x86_64-macos").unwrap();
    assert_eq!(stub.install_name, "/usr/lib/libz.1.dylib");
    assert!(resolve(&stub.table, "_deflate").is_ok());
    assert_eq!(
        resolve(&stub.table, "_zweak").unwrap().binding,
        Binding::Weak
    );
    let v5 = r#"{"tapi_tbd_version": 5, "main_library": {"target_info": [{"target": "x86_64-macos"}], "install_names": [{"name": "/usr/lib/libSystem.B.dylib"}], "current_versions": [{"version": "1351"}], "reexported_libraries": [{"names": ["/usr/lib/system/libsystem_m.dylib"]}], "exported_symbols": [{"text": {"global": ["_sys"]}}]}, "libraries": [{"target_info": [{"target": "x86_64-macos"}], "install_names": [{"name": "/usr/lib/system/libsystem_m.dylib"}], "exported_symbols": [{"text": {"global": ["_sqrt"], "weak": ["_w"]}}]}]}"#;
    let stub = tbd::read(v5.as_bytes(), "x86_64-macos").unwrap();
    assert_eq!(
        resolve(&stub.table, "_sqrt").unwrap().origin,
        "/usr/lib/system/libsystem_m.dylib"
    );
    assert_eq!(resolve(&stub.table, "_w").unwrap().binding, Binding::Weak);
    assert!(
        tbd::read(
            b"--- !tapi-tbd\ntbd-version: 9\ninstall-name: /x\n",
            "x86_64-macos"
        )
        .is_err()
    );
}

/// A minimal thin x86_64 dylib: `LC_ID_DYLIB` and an export trie holding
/// `_qr_add` (regular), `_qr_weak` (weak) and `_qr_fwd` (re-exported).
fn dylib_bytes() -> Vec<u8> {
    // Trie nodes, each at a fixed offset (every ULEB here is one byte).
    // 0: root, no terminal, one edge "_qr_" -> 8
    // 8: no terminal, three edges: "add" -> 30, "weak" -> 34, "fwd" -> 38
    let mut trie = vec![0u8; 44];
    let root = [0u8, 1, b'_', b'q', b'r', b'_', 0, 8];
    trie[..root.len()].copy_from_slice(&root);
    let mut node = vec![0u8, 3];
    node.extend_from_slice(b"add\0");
    node.push(30);
    node.extend_from_slice(b"weak\0");
    node.push(34);
    node.extend_from_slice(b"fwd\0");
    node.push(38);
    trie[8..8 + node.len()].copy_from_slice(&node);
    trie[30..34].copy_from_slice(&[2, 0x00, 0x10, 0]);
    trie[34..38].copy_from_slice(&[2, 0x04, 0x20, 0]);
    trie[38..43].copy_from_slice(&[3, 0x08, 1, 0, 0]);
    let name = b"/usr/lib/libqr.dylib\0\0\0\0";
    let id_size = 24 + name.len() as u32;
    let trie_cmd_size = 16u32;
    let header_size = 32u32;
    let trie_offset = header_size + id_size + trie_cmd_size;
    let mut out = Vec::new();
    for word in [
        0xfeed_facfu32,
        0x0100_0007,
        3,
        6,
        2,
        id_size + trie_cmd_size,
        0,
        0,
    ] {
        out.extend_from_slice(&word.to_le_bytes());
    }
    for word in [0xdu32, id_size, 24, 0, 0x0001_0000, 0x0001_0000] {
        out.extend_from_slice(&word.to_le_bytes());
    }
    out.extend_from_slice(name);
    for word in [
        0x8000_0033u32,
        trie_cmd_size,
        trie_offset,
        trie.len() as u32,
    ] {
        out.extend_from_slice(&word.to_le_bytes());
    }
    out.extend_from_slice(&trie);
    out
}

#[test]
fn a_dylib_export_trie_is_walked() {
    let dylib = macho::dylib(&dylib_bytes(), "libqr.dylib").unwrap();
    assert_eq!(dylib.install_name.as_deref(), Some("/usr/lib/libqr.dylib"));
    assert_eq!(
        resolve(&dylib.table, "_qr_add").unwrap().binding,
        Binding::Global
    );
    assert_eq!(
        resolve(&dylib.table, "_qr_weak").unwrap().binding,
        Binding::Weak
    );
    assert!(
        resolve(&dylib.table, "_qr_fwd").is_err(),
        "a re-export is not this image's code"
    );
    let loader = files(&[("libqr.dylib", &dylib_bytes())]);
    let read = read_artifact("libqr.dylib", &loader, &mac()).unwrap();
    assert_eq!(read.files[0].kind, FileKind::Dylib);
    assert!(read.dynamic);
}

// ---------------------------------------------------------------------------
// COFF / PE
// ---------------------------------------------------------------------------

#[test]
fn a_coff_static_library_is_read_through_its_linker_members() {
    let archive = archive::read(COFF_DEMO).unwrap();
    assert!(archive.index_kinds.contains(&archive::IndexKind::Gnu32));
    let table = table("qr_demo.lib", COFF_DEMO, &win());
    let add = resolve(&table, "qr_add").unwrap();
    assert_eq!(add.binding, Binding::Global);
    assert_eq!(add.origin, "qr_demo.lib(qr_win.obj)");
    assert!(add.import.is_none());
    assert!(matches!(
        resolve(&table, "qr_undefined_elsewhere"),
        Err(SymbolRefusal::Undefined { .. })
    ));
    assert!(matches!(
        read_artifact("qr_demo.lib", &files(&[("qr_demo.lib", COFF_DEMO)]), &elf()),
        Err(ArtifactRefusal::WrongTarget { .. })
    ));
}

#[test]
fn an_import_library_is_by_name_or_refused_as_ordinal_only() {
    let read = read_artifact(
        "qr_import.lib",
        &files(&[("qr_import.lib", COFF_IMPORT)]),
        &win(),
    )
    .unwrap();
    assert!(read.dynamic);
    assert_eq!(read.files[0].kind, FileKind::ImportLibrary);
    let by_name = resolve(&read.table, "qr_by_name").unwrap();
    let import = by_name.import.unwrap();
    assert_eq!(import.dll, "qr_runtime.dll");
    assert!(import.by_name);
    assert_eq!(import.export_name.as_deref(), Some("qr_by_name"));
    assert_eq!(
        resolve(&read.table, "qr_by_ordinal"),
        Err(SymbolRefusal::OrdinalOnly {
            symbol: "qr_by_ordinal".into(),
            dll: "qr_runtime.dll".into()
        })
    );
    // A data import defines only `__imp_NAME`.
    assert!(resolve(&read.table, "qr_data").is_err());
    assert!(resolve(&read.table, "__imp_qr_data").is_ok());
}

/// A minimal PE32+ DLL `qr_runtime.dll` exporting `qr_by_name` (code) and
/// `qr_forwarded` (a forwarder to `other.qr_target`).
pub(super) fn pe_bytes() -> Vec<u8> {
    let mut out = vec![0u8; 0x200];
    out[0..2].copy_from_slice(b"MZ");
    out[0x3c..0x40].copy_from_slice(&0x40u32.to_le_bytes());
    out[0x40..0x44].copy_from_slice(b"PE\0\0");
    let coff = 0x44;
    out[coff..coff + 2].copy_from_slice(&0x8664u16.to_le_bytes());
    out[coff + 2..coff + 4].copy_from_slice(&1u16.to_le_bytes());
    out[coff + 16..coff + 18].copy_from_slice(&240u16.to_le_bytes());
    let optional = coff + 20;
    out[optional..optional + 2].copy_from_slice(&0x20bu16.to_le_bytes());
    out[optional + 108..optional + 112].copy_from_slice(&16u32.to_le_bytes());
    // The export section: RVA 0x1000, file offset 0x200.
    let mut section = vec![0u8; 0x100];
    let put32 = |buffer: &mut Vec<u8>, at: usize, value: u32| {
        buffer[at..at + 4].copy_from_slice(&value.to_le_bytes());
    };
    let rva = |offset: usize| 0x1000 + offset as u32;
    // Directory at 0; functions at 40; names at 48; ordinals at 56; strings from 64.
    let strings = b"qr_runtime.dll\0qr_by_name\0qr_forwarded\0other.qr_target\0";
    section[64..64 + strings.len()].copy_from_slice(strings);
    let dll_name = 64;
    let by_name = 64 + 15;
    let forwarded = by_name + 11;
    let target = forwarded + 13;
    put32(&mut section, 12, rva(dll_name));
    put32(&mut section, 16, 1);
    put32(&mut section, 20, 2);
    put32(&mut section, 24, 2);
    put32(&mut section, 28, rva(40));
    put32(&mut section, 32, rva(48));
    put32(&mut section, 36, rva(56));
    put32(&mut section, 40, 0x2000);
    put32(&mut section, 44, rva(target));
    put32(&mut section, 48, rva(by_name));
    put32(&mut section, 52, rva(forwarded));
    section[56..58].copy_from_slice(&0u16.to_le_bytes());
    section[58..60].copy_from_slice(&1u16.to_le_bytes());
    let directories = optional + 112;
    out[directories..directories + 4].copy_from_slice(&0x1000u32.to_le_bytes());
    out[directories + 4..directories + 8].copy_from_slice(&0x100u32.to_le_bytes());
    let table = optional + 240;
    out[table..table + 6].copy_from_slice(b".edata");
    for (at, value) in [(8, 0x100u32), (12, 0x1000), (16, 0x100), (20, 0x200)] {
        out[table + at..table + at + 4].copy_from_slice(&value.to_le_bytes());
    }
    out.extend_from_slice(&section);
    out
}

#[test]
fn the_pe_export_directory_names_exports_and_their_forwarders() {
    let exports = coff::pe_exports(&pe_bytes()).unwrap();
    assert_eq!(exports.dll, "qr_runtime.dll");
    let by_name = exports.find("qr_by_name").unwrap();
    assert_eq!(by_name.forwarder, None);
    assert_eq!(by_name.ordinal, 1);
    let forwarded = exports.find("qr_forwarded").unwrap();
    assert_eq!(forwarded.forwarder.as_deref(), Some("other.qr_target"));
    assert!(exports.find("qr_absent").is_none());
    assert!(coff::pe_exports(&pe_bytes()[..0x100]).is_err());
}

#[test]
fn the_target_facts_derive_format_and_decoration() {
    assert_eq!(elf().format, Format::Elf);
    assert_eq!(mac().format, Format::MachO);
    assert_eq!(win().format, Format::Coff);
    assert_eq!(
        win().object_name("RtlNtStatusToDosError"),
        "RtlNtStatusToDosError"
    );
    assert!(TargetFacts::from_triple("riscv64-unknown-linux-gnu").is_none());
    assert!(TargetFacts::from_triple("x86_64-unknown-none").is_none());
}
