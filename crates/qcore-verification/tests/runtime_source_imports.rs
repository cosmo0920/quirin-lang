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

use qcore_frontend::frontend::SourceModuleMode;
use qcore_verification::driver::{
    CompileOptions, SourceFile, compile_files_with_registry, compile_source_bundle_with_registry,
};
use qcore_verification::source_imports::{ImportLimits, SourceLoading, resolve_source_bundle};
use qcore_verifier_host::VerifierRegistry;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

fn file(path: &str, source: &str) -> SourceFile {
    SourceFile::new(path, source)
}
fn compile(
    roots: &[&str],
    files: &[SourceFile],
) -> Result<
    qcore_verification::driver::CompiledModule,
    Vec<qcore_verification::driver::FileDiagnostic>,
> {
    compile_source_bundle_with_registry(
        &roots.iter().map(|s| (*s).into()).collect::<Vec<_>>(),
        files,
        SourceLoading::SnapshotsOnly,
        &ImportLimits::default(),
        &CompileOptions::default(),
        &VerifierRegistry::default(),
    )
}
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "qcore-runtime-import-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    fn path(&self, name: &str) -> String {
        self.0.join(name).to_str().unwrap().into()
    }
    fn write(&self, name: &str, text: &str) {
        std::fs::write(self.0.join(name), text).unwrap();
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn nested_extension_forms_deduplicate_diamonds_and_keep_originals() {
    let files = [
        file(
            "app/main.qr",
            "#import \"../lib/a\"; #import \"../lib/b.qr\"; #import \"../lib/a.qr\"; fn main() -> i32 { return a(); }",
        ),
        file(
            "lib/a.qr",
            "#import \"./shared\"; fn a() -> i32 { return shared(); }",
        ),
        file("lib/b.qr", "#import \"./shared.qr\";"),
        file("lib/shared.qr", "fn shared() -> i32 { return 7; }"),
    ];
    let module = compile(&["app/main.qr"], &files).unwrap();
    let evidence = module.source_bundle.unwrap();
    assert_eq!(
        evidence
            .sources
            .iter()
            .map(|f| f.path.as_str())
            .collect::<Vec<_>>(),
        ["app/main.qr", "lib/a.qr", "lib/shared.qr", "lib/b.qr"]
    );
    assert_eq!(evidence.imports.len(), 5);
    assert_eq!(evidence.sources[0], files[0]);
    assert!(matches!(
        module.reference_observation.unwrap().outcome,
        qcore_core::cfg::ExecutionOutcome::Ok { value: 7, .. }
    ));
}

#[test]
fn imported_and_direct_multifile_programs_emit_identically() {
    let root = "#import \"./helper\";\nfn main() -> i32 { return helper(3); }";
    let helper = "module library; fn helper(x: i32) -> i32 { return x + 1; }";
    let imported = compile(
        &["main.qr"],
        &[file("main.qr", root), file("helper.qr", helper)],
    )
    .unwrap();
    let blank = format!(
        "{}\nfn main() -> i32 {{ return helper(3); }}",
        " ".repeat("#import \"./helper\";".len())
    );
    let direct = compile_files_with_registry(
        &[file("main.qr", &blank), file("helper.qr", helper)],
        &CompileOptions::default(),
        &VerifierRegistry::default(),
    )
    .unwrap();
    assert_eq!(imported.c_source, direct.c_source);
    assert_eq!(imported.reference_observation, direct.reference_observation);
    assert_eq!(imported.execution_profile, direct.execution_profile);
}

#[test]
fn cycles_and_ambiguous_snapshots_refuse() {
    for files in [
        vec![file("a.qr", "#import \"a\";")],
        vec![
            file("a.qr", "#import \"b\";"),
            file("b.qr", "#import \"./a.qr\";"),
        ],
        vec![file("a.qr", ""), file("./a.qr", "")],
        vec![
            file("a.qr", ""),
            file("a.qr", "fn f() -> i32 { return 1; }"),
        ],
    ] {
        let error = compile(&["a.qr"], &files).unwrap_err();
        assert_eq!(error[0].code, "Q014");
        assert!(
            error[0].message.contains("cycle") || error[0].message.contains("ambiguous"),
            "{error:?}"
        );
    }
}

#[test]
fn imported_modes_and_root_modes_are_distinct() {
    for header in ["module contract;", "module executable;"] {
        let error = compile(
            &["main.qr"],
            &[
                file(
                    "main.qr",
                    "#import \"helper\"; fn main() -> i32 { return 0; }",
                ),
                file("helper.qr", header),
            ],
        )
        .unwrap_err();
        assert!(matches!(error[0].code.as_str(), "Q014" | "Q403"));
    }
    let library = compile(
        &["lib.qr"],
        &[
            file("lib.qr", "module library; #import \"helper\";"),
            file("helper.qr", "fn helper() -> i32 { return 2; }"),
        ],
    )
    .unwrap();
    assert_eq!(library.mode, SourceModuleMode::Library);
    assert!(library.entry.is_none());
    assert!(library.c_source.is_none());
    let error = compile(
        &["contract.qv"],
        &[
            file("contract.qv", "module contract; #import \"helper\";"),
            file("helper.qr", ""),
        ],
    )
    .unwrap_err();
    assert_eq!(error[0].code, "Q014");
}

#[test]
fn unused_pool_entries_do_not_become_roots_but_all_selected_definitions_are_checked() {
    let module = compile(
        &["main.qr"],
        &[
            file("main.qr", "fn main() -> i32 { return 0; }"),
            file("unused.qr", "this is not syntax"),
        ],
    )
    .unwrap();
    assert_eq!(module.source_bundle.unwrap().sources.len(), 1);
    let error = compile(
        &["main.qr"],
        &[
            file(
                "main.qr",
                "#import \"unused\"; fn main() -> i32 { return 0; }",
            ),
            file("unused.qr", "fn unused() -> i32 { return missing; }"),
        ],
    )
    .unwrap_err();
    assert_eq!(error[0].location.path, "unused.qr");
}

#[test]
fn capabilities_never_flow_in_either_import_direction() {
    for (needed, body) in [
        ("std.io", "fn bad() -> i32 { print(1); return 0; }"),
        ("std.concurrency", "fn bad() -> unit { thread scope s {} }"),
        ("std.extern.unsafe", "unsafe extern fn c() -> i32;"),
    ] {
        for reverse in [false, true] {
            let (root, helper) = if reverse {
                (
                    format!(
                        "#import \"helper\"; #import {needed}; fn main() -> i32 {{ return 0; }}"
                    ),
                    body.to_owned(),
                )
            } else {
                (
                    format!("#import \"helper\"; {body}"),
                    format!("#import {needed};"),
                )
            };
            let error = compile(
                &["main.qr"],
                &[file("main.qr", &root), file("helper.qr", &helper)],
            )
            .unwrap_err();
            assert!(
                matches!(error[0].code.as_str(), "Q015" | "Q032"),
                "{error:?}"
            );
            assert_eq!(
                error[0].location.path,
                if reverse { "helper.qr" } else { "main.qr" }
            );
        }
    }
}

#[test]
fn extern_calls_require_callers_own_import_even_with_foreign_declarations() {
    for declaration in [
        "extern fn c() -> i32 contract { };",
        "unsafe extern fn c() -> i32;",
    ] {
        for directive in ["", "#import \"helper\";"] {
            let files = [
                file(
                    "main.qr",
                    &format!("{directive} fn main() -> i32 {{ let result = c(); return 0; }}"),
                ),
                file(
                    "helper.qr",
                    &format!("#import std.extern; #import std.extern.unsafe; {declaration}"),
                ),
            ];
            let error = compile_files_with_registry(
                &files,
                &CompileOptions::default(),
                &VerifierRegistry::default(),
            )
            .unwrap_err();
            assert_eq!(error[0].code, "Q022", "{error:?}");
            assert_eq!(error[0].location.path, "main.qr");
        }
    }
}

#[test]
fn dependency_locations_preserve_unicode_crlf_and_original_lines() {
    let root = file(
        "./main.qr",
        "#import \"./helper\";\r\nfn main() -> i32 { return 0; }",
    );
    let helper = file("./helper.qr", "// λ\r\n/*λ*/ #import \"./missing\";\r\n");
    let error = compile(&["./main.qr"], &[root, helper]).unwrap_err();
    assert_eq!(error[0].code, "Q014");
    assert_eq!(error[0].location.path, "./helper.qr");
    assert_eq!(error[0].location.line, 2);
    assert_eq!(error[0].location.column, 7);
    assert_eq!(
        error[0].location.line_source,
        "/*λ*/ #import \"./missing\";"
    );
}

#[test]
fn unsupported_paths_and_missing_dependencies_never_search_alternatives() {
    for target in [
        "absent",
        "helper.qv",
        "helper.qcore",
        "helper.qrlib",
        "helper.txt",
        "/helper.qr",
        "C:/helper.qr",
        "helper/",
    ] {
        let error = compile(
            &["main.qr"],
            &[
                file("main.qr", &format!("#import \"{target}\";")),
                file("helper.qr", ""),
                file("absent.qcore", ""),
            ],
        )
        .unwrap_err();
        assert_eq!(error[0].code, "Q014", "{target}: {error:?}");
    }
}

#[test]
fn finite_limits_refuse_instead_of_truncating_the_graph() {
    let files = [
        file("a.qr", "#import \"b\";"),
        file("b.qr", "#import \"c\";"),
        file("c.qr", ""),
    ];
    for limits in [
        ImportLimits {
            depth: 1,
            ..ImportLimits::default()
        },
        ImportLimits {
            modules: 2,
            ..ImportLimits::default()
        },
        ImportLimits {
            bytes: 1,
            ..ImportLimits::default()
        },
    ] {
        let errors = resolve_source_bundle(
            &["a.qr".into()],
            &files,
            SourceLoading::SnapshotsOnly,
            limits,
        )
        .unwrap_err();
        assert_eq!(errors[0].code, "Q014");
        assert!(errors[0].message.contains("bound"));
    }
}

#[test]
fn filesystem_loading_is_explicit_and_snapshots_override_disk() {
    let temp = Temp::new();
    temp.write(
        "helper.qr",
        "this disk source must never override the supplied snapshot",
    );
    let root = file(
        &temp.path("main.qr"),
        "#import \"helper\"; fn main() -> i32 { return helper(); }",
    );
    let helper = file(&temp.path("helper.qr"), "fn helper() -> i32 { return 9; }");
    let module = compile_source_bundle_with_registry(
        std::slice::from_ref(&root.path),
        &[root.clone(), helper.clone()],
        SourceLoading::FileSystem,
        &ImportLimits::default(),
        &CompileOptions::default(),
        &VerifierRegistry::default(),
    )
    .unwrap();
    assert_eq!(module.source_bundle.unwrap().sources[1], helper);
    assert!(
        compile(&[&root.path], std::slice::from_ref(&root)).unwrap_err()[0]
            .message
            .contains("filesystem loading is disabled")
    );
    temp.write("helper.qr", "fn helper() -> i32 { return 4; }");
    let bundle = resolve_source_bundle(
        std::slice::from_ref(&root.path),
        std::slice::from_ref(&root),
        SourceLoading::FileSystem,
        ImportLimits::default(),
    )
    .unwrap();
    assert!(bundle.evidence().sources[1].source.contains("return 4"));
}

#[test]
fn filesystem_missing_directory_and_invalid_utf8_are_refusals() {
    let temp = Temp::new();
    std::fs::create_dir(temp.0.join("directory.qr")).unwrap();
    std::fs::write(temp.0.join("binary.qr"), [0xff]).unwrap();
    for target in ["absent", "directory", "binary"] {
        let root = file(&temp.path("main.qr"), &format!("#import \"{target}\";"));
        let errors = resolve_source_bundle(
            std::slice::from_ref(&root.path),
            std::slice::from_ref(&root),
            SourceLoading::FileSystem,
            ImportLimits::default(),
        )
        .unwrap_err();
        assert_eq!(errors[0].code, "Q014");
    }
}

#[cfg(unix)]
#[test]
fn disk_parent_steps_follow_symlinks_and_alias_cycles_refuse() {
    use std::os::unix::fs::symlink;
    let temp = Temp::new();
    std::fs::create_dir_all(temp.0.join("real/child")).unwrap();
    symlink(temp.0.join("real/child"), temp.0.join("link")).unwrap();
    temp.write("helper.qr", "this lexical-parent decoy must not be read");
    temp.write("real/helper.qr", "fn helper() -> i32 { return 5; }");
    let root = file(
        &temp.path("main.qr"),
        "#import \"link/../helper\"; fn main() -> i32 { return helper(); }",
    );
    let module = compile_source_bundle_with_registry(
        std::slice::from_ref(&root.path),
        std::slice::from_ref(&root),
        SourceLoading::FileSystem,
        &ImportLimits::default(),
        &CompileOptions::default(),
        &VerifierRegistry::default(),
    );
    let module = module.unwrap();
    assert!(matches!(
        module.reference_observation.unwrap().outcome,
        qcore_core::cfg::ExecutionOutcome::Ok { value: 5, .. }
    ));
    temp.write("cycle.qr", "#import \"alias\";");
    symlink(temp.0.join("cycle.qr"), temp.0.join("alias.qr")).unwrap();
    let cycle = file(&temp.path("cycle.qr"), "#import \"alias\";");
    assert!(
        resolve_source_bundle(
            std::slice::from_ref(&cycle.path),
            std::slice::from_ref(&cycle),
            SourceLoading::FileSystem,
            ImportLimits::default()
        )
        .unwrap_err()[0]
            .message
            .contains("cycle")
    );
}

#[test]
fn imported_task_and_epoch_sources_keep_independent_replay_checks() {
    let ordinary = "#import std.concurrency; concurrency checked; fn worker(v:i32)->own<Buffer> effects {alloc,throws<AllocError>} {let b=qcore_buffer_new(v)?;return move b;} fn main()->i32 effects {throws<AllocError>,throws<SpawnError>,spawn,join,may_block} {let v=42;thread scope s {let h=spawn s worker(copy v)?;let b=join move h?;let value=qcore_buffer_read(b);drop b;return value;}}";
    let epoch = include_str!("../../../compiler/tests/fixtures/epoch_credit/CT25/case.qr");
    for (source, expected) in [
        (
            ordinary,
            qcore_verification::driver::SourceExecutionProfile::ScopedOwnedTasksV1,
        ),
        (
            epoch,
            qcore_verification::driver::SourceExecutionProfile::ComposedEpochTasksV1,
        ),
    ] {
        let start = source.find("fn worker").unwrap();
        let end = source.find("fn main").unwrap();
        let helper = format!("#import std.concurrency;\n{}", &source[start..end]);
        let root = format!(
            "{}#import \"worker\";\n{}",
            &source[..start],
            &source[end..]
        );
        let imported = compile(
            &["main.qr"],
            &[file("main.qr", &root), file("worker.qr", &helper)],
        )
        .unwrap();
        assert_eq!(imported.execution_profile, expected);
        let direct_root = root.replace(
            "#import \"worker\";",
            &" ".repeat("#import \"worker\";".len()),
        );
        let direct = compile_files_with_registry(
            &[file("main.qr", &direct_root), file("worker.qr", &helper)],
            &CompileOptions::default(),
            &VerifierRegistry::default(),
        )
        .unwrap();
        assert_eq!(imported.c_source, direct.c_source);
        assert_eq!(imported.source_map, direct.source_map);
    }
}

#[test]
fn staging_keeps_unicode_directive_byte_offsets_for_semantic_diagnostics() {
    let source = "#import \"./héλ\"; fn main() -> i32 { return missing; }";
    let errors = compile(&["main.qr"], &[file("main.qr", source), file("héλ.qr", "")]).unwrap_err();
    assert_eq!(errors[0].code, "Q013");
    assert_eq!(errors[0].location.path, "main.qr");
    assert_eq!(errors[0].location.line_source, source);
    assert_eq!(
        errors[0].location.column,
        source[..source.find("return").unwrap()].chars().count() + 1
    );
}

#[test]
fn disk_discovery_obeys_module_and_byte_limits() {
    let temp = Temp::new();
    temp.write("helper.qr", "fn helper() -> i32 { return 0; }");
    let root = file(&temp.path("main.qr"), "#import \"helper\";");
    for limits in [
        ImportLimits {
            modules: 1,
            ..ImportLimits::default()
        },
        ImportLimits {
            bytes: root.source.len() + 1,
            ..ImportLimits::default()
        },
    ] {
        let errors = resolve_source_bundle(
            std::slice::from_ref(&root.path),
            std::slice::from_ref(&root),
            SourceLoading::FileSystem,
            limits,
        )
        .unwrap_err();
        assert_eq!(errors[0].code, "Q014");
        assert!(errors[0].message.contains("bound"), "{errors:?}");
    }
}
