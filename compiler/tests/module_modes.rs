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

use qcore_compiler::{
    CompileOptions, SourceFile, SourceModuleMode, compile_files, elaborate_source,
    parse_source_program,
};

#[test]
fn declaration_only_modules_have_no_synthetic_entry_or_execution() {
    for (name, mode) in [
        ("contract", SourceModuleMode::Contract),
        ("library", SourceModuleMode::Library),
    ] {
        let source = format!(
            "// mode declaration\nmodule {name};\nextern fn allocate(n: usize) -> ptr<mut, u8> contract {{ ensures normal caller_obligation; }};"
        );
        let parsed = parse_source_program(&source).unwrap();
        assert_eq!(parsed.mode, mode);
        assert!(parsed.syntax_equivalent(&parse_source_program(&parsed.to_string()).unwrap()));
        assert_eq!(elaborate_source(&parsed).unwrap().mode, mode);
        let module = compile_files(
            &[SourceFile::new("api.qcore", source)],
            &CompileOptions::default(),
        )
        .unwrap();
        assert_eq!(module.mode, mode);
        assert!(module.entry.is_none());
        assert!(module.c_source.is_none());
        assert!(module.reference_observation.is_none());
        assert_eq!(
            module
                .checked
                .as_sequential()
                .unwrap()
                .source
                .declarations
                .len(),
            1
        );
        assert_eq!(
            module.checked.as_sequential().unwrap().source.declarations[0]
                .function_contract
                .as_ref()
                .unwrap()
                .clauses
                .len(),
            1
        );
    }
}

#[test]
fn mode_is_explicit_and_entry_cannot_silently_override_it() {
    let source = "extern fn f() -> unit;";
    assert!(
        compile_files(
            &[SourceFile::new("api.qcore", source)],
            &CompileOptions::default()
        )
        .is_err()
    );
    for mode in ["contract", "library"] {
        let source = format!("module {mode}; fn main() -> i32 {{ return 1; }}");
        let file = SourceFile::new("lib.qcore", source);
        let module =
            compile_files(std::slice::from_ref(&file), &CompileOptions::default()).unwrap();
        assert_eq!(module.checked.function_count(), 1);
        assert!(module.entry.is_none());
        let errors = compile_files(
            &[file],
            &CompileOptions {
                entry: Some("main".into()),
                ..CompileOptions::default()
            },
        )
        .unwrap_err();
        assert_eq!(errors[0].code, "Q403");
    }
    for source in [
        "module;",
        "module contract",
        "module unknown;",
        "module contract; module library;",
        "extern fn f() -> unit; module contract;",
    ] {
        assert!(parse_source_program(source).is_err(), "{source}");
    }
}

#[test]
fn non_executable_mode_still_checks_bodies_and_preserves_diagnostic_locations() {
    let files = [
        SourceFile::new(
            "contracts.qcore",
            "module contract;\nextern fn f() -> unit;",
        ),
        SourceFile::new(
            "library.qcore",
            "// Unicode π\nmodule /* mode π */\nlibrary;\nfn bad() -> i32 { return missing; }",
        ),
    ];
    let errors = compile_files(&files, &CompileOptions::default()).unwrap_err();
    let error = errors
        .iter()
        .find(|e| e.message.contains("missing"))
        .unwrap();
    assert_eq!(error.location.path, "library.qcore");
    assert_eq!(error.location.line, 4);
    assert_eq!(
        error.location.line_source,
        "fn bad() -> i32 { return missing; }"
    );
    let leak = "module library; error Failure { Failed; } fn leak() -> i32 { let r = qcore_acquire(1, true, Failure); return 0; }";
    assert!(
        compile_files(
            &[SourceFile::new("leak.qcore", leak)],
            &CompileOptions::default()
        )
        .is_err()
    );
}

#[test]
fn contract_files_compose_with_libraries_and_explicit_executables() {
    let mut files = vec![
        SourceFile::new(
            "api.qcore",
            "module contract; extern fn external() -> unit;",
        ),
        SourceFile::new(
            "lib.qcore",
            "module library; fn leaf() -> i32 { return 1; }",
        ),
    ];
    let library = compile_files(&files, &CompileOptions::default()).unwrap();
    assert_eq!(library.mode, SourceModuleMode::Library);
    assert!(library.entry.is_none());
    files.push(SourceFile::new(
        "app.qcore",
        "module executable; fn main() -> i32 { leaf(); return 2; }",
    ));
    let app = compile_files(&files, &CompileOptions::default()).unwrap();
    assert_eq!(app.mode, SourceModuleMode::Executable);
    assert_eq!(app.entry.as_deref(), Some("main"));
    assert!(app.c_source.as_ref().unwrap().contains("int main(void)"));
    assert_eq!(app.source_map[0].source.path, "app.qcore");
    assert_eq!(app.source_map[0].source.line, 1);
}
