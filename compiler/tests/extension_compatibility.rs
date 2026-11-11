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

use qcore_compiler::{CompileOptions, SourceFile, SourceModuleMode, compile_files};

fn compile(
    path: &str,
    source: &str,
) -> Result<qcore_compiler::CompiledModule, Vec<qcore_compiler::FileDiagnostic>> {
    compile_files(&[SourceFile::new(path, source)], &CompileOptions::default())
}

#[test]
fn extension_guards_preserve_modes_and_do_not_mint_an_entry() {
    for (mode, text, suffix) in [
        (
            SourceModuleMode::Executable,
            "fn main() -> i32 { return 7; }",
            ".qr",
        ),
        (
            SourceModuleMode::Library,
            "module library; fn f() -> i32 { return 7; }",
            ".qr",
        ),
        (
            SourceModuleMode::Contract,
            "module contract; extern fn f() -> unit;",
            ".qv",
        ),
    ] {
        let old = compile("input.qcore", text).unwrap();
        let name = format!("input{suffix}");
        let new = compile(&name, text).unwrap();
        assert_eq!(new.mode, mode);
        assert_eq!(old.mode, new.mode);
        assert_eq!(old.entry, new.entry);
        assert_eq!(old.execution_profile, new.execution_profile);
        assert_eq!(
            old.c_source,
            new.c_source.map(|s| s.replace(&name, "input.qcore"))
        );
        assert_eq!(old.extension_warnings.len(), 1);
        assert_eq!(old.extension_warnings[0].recommended_extension, suffix);
        assert!(new.extension_warnings.is_empty());
        let wrong = if suffix == ".qv" {
            "input.qr"
        } else {
            "input.qv"
        };
        assert_eq!(compile(wrong, text).unwrap_err()[0].code, "Q403");
    }
}

#[test]
fn aliases_are_ambiguous_but_runtime_and_contract_inputs_remain_distinct() {
    let contract = "module contract; extern fn api() -> unit;";
    let runtime = "fn main() -> i32 { return 0; }";
    for paths in [["input.qcore", "input.qr"], ["input.qr", "input.qcore"]] {
        let errors = compile_files(
            &[
                SourceFile::new(paths[0], runtime),
                SourceFile::new(paths[1], runtime),
            ],
            &CompileOptions::default(),
        )
        .unwrap_err();
        assert_eq!(errors[0].code, "Q403");
        assert!(errors[0].message.contains("ambiguous extension aliases"));
    }
    for reverse in [false, true] {
        let mut files = vec![
            SourceFile::new("input.qv", contract),
            SourceFile::new("input.qr", runtime),
        ];
        if reverse {
            files.reverse();
        }
        let compiled = compile_files(&files, &CompileOptions::default()).unwrap();
        assert_eq!(compiled.mode, SourceModuleMode::Executable);
        assert_eq!(compiled.entry.as_deref(), Some("main"));
    }
}

#[test]
fn renamed_native_manifest_preserves_artifact_premises_and_emission() {
    let source = include_str!("fixtures/unsafe_linkage/round_trip.qr");
    let manifest = include_str!("fixtures/native_library/unchecked_demo.qrlib");
    let run = |paths: &[&str]| {
        compile_files(
            &[SourceFile::new("input.qr", source)],
            &CompileOptions {
                native_libraries: Some(
                    paths
                        .iter()
                        .map(|p| (p.to_string(), manifest.to_owned()))
                        .collect(),
                ),
                ..CompileOptions::default()
            },
        )
    };
    let old = run(&["demo.qcorelib"]).unwrap();
    let new = run(&["demo.qrlib"]).unwrap();
    assert_eq!(old.c_source, new.c_source);
    assert_eq!(
        format!("{:?}", old.declared_assumptions()),
        format!("{:?}", new.declared_assumptions()).replace("demo.qrlib", "demo.qcorelib")
    );
    assert_eq!(old.extension_warnings[0].recommended_extension, ".qrlib");
    assert!(new.extension_warnings.is_empty());
    assert_eq!(
        run(&["demo.qcorelib", "demo.qrlib"]).unwrap_err()[0].code,
        "Q039"
    );
    assert_eq!(run(&["demo.qv"]).unwrap_err()[0].code, "Q039");
}

#[test]
fn qrlib_contract_references_require_the_exact_selected_qv_path() {
    let source = include_str!("fixtures/extern_calls/round_trip.qr");
    let text = include_str!("fixtures/native_library/contracts/buffer_lib.qv");
    let manifest = include_str!("fixtures/native_library/buffer_lib.qrlib");
    let old = "fixtures/native_library/contracts/buffer_lib.qcore";
    let new = "fixtures/native_library/contracts/buffer_lib.qv";
    let run = |reference: &str| {
        compile_files(
            &[SourceFile::new("input.qr", source)],
            &CompileOptions {
                extern_contracts: Some([(new.to_owned(), text.to_owned())].into_iter().collect()),
                native_libraries: Some(
                    [("buffer.qrlib".into(), manifest.replace(new, reference))]
                        .into_iter()
                        .collect(),
                ),
                ..CompileOptions::default()
            },
        )
    };
    let current = run(new).unwrap();
    assert!(
        current
            .declared_assumptions()
            .iter()
            .any(|a| format!("{a:?}").contains(new))
    );
    assert!(
        run(old).is_err(),
        "a stale manifest reference must not choose an alternate suffix"
    );
}
