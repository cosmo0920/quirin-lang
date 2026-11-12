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

//! Native Windows C execution, shared by the emitter, source-task and CT tests.
use std::fs;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

#[allow(dead_code)]
pub fn run(source: &str, header: Option<&str>, defines: &[String]) -> Output {
    run_with_sources(source, header, defines, &[])
}

/// Compile and run with Microsoft's native `cl.exe`, refusing an MSVC-like
/// substitute such as clang-cl. Cards that publish Visual Studio/MSVC evidence
/// use this entry point so the compiler named by the evidence is executable,
/// not merely inferred from the Rust target.
#[allow(dead_code)]
pub fn run_msvc(source: &str, header: Option<&str>, defines: &[String]) -> Output {
    run_with_sources_impl(source, header, defines, &[], true, &[])
}

#[allow(dead_code)]
pub fn run_with_sources(
    source: &str,
    header: Option<&str>,
    defines: &[String],
    extra_sources: &[(&str, &str)],
) -> Output {
    run_with_sources_impl(source, header, defines, extra_sources, false, &[])
}

/// Card `W48-QRLIB-LINKAGE`: compile with Microsoft's `cl.exe` and link the
/// given libraries (a static `.lib` or an SDK import library) by path — the
/// pinned artifacts a verified link plan names, and nothing else.
#[allow(dead_code)]
pub fn run_msvc_with_libraries(source: &str, libraries: &[&Path]) -> Output {
    run_with_sources_impl(source, None, &[], &[], true, libraries)
}

fn run_with_sources_impl(
    source: &str,
    header: Option<&str>,
    defines: &[String],
    extra_sources: &[(&str, &str)],
    require_cl: bool,
    libraries: &[&Path],
) -> Output {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let directory = std::env::temp_dir().join(format!(
        "qcore-windows-threads-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&directory).unwrap();
    let c = directory.join("module.c");
    let exe = directory.join("module.exe");
    fs::write(&c, source).unwrap();
    let extra_source_paths = extra_sources
        .iter()
        .map(|(name, contents)| {
            let path = directory.join(name);
            fs::write(&path, contents).unwrap();
            path
        })
        .collect::<Vec<_>>();
    let version = Command::new("rustc").arg("-vV").output().unwrap();
    assert!(version.status.success());
    let version = String::from_utf8(version.stdout).unwrap();
    let host = version
        .lines()
        .find_map(|s| s.strip_prefix("host: "))
        .unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .find(|p| {
            p.join("crates/qcore-core/src/c11_threads_compat.h")
                .is_file()
        })
        .unwrap();
    let mut build = cc::Build::new();
    build
        .host(host)
        .target(host)
        .out_dir(&directory)
        .opt_level(1)
        .debug(false)
        .cargo_metadata(false)
        .warnings(true)
        .warnings_into_errors(true)
        .include(root.join("crates/qcore-core/src"))
        .include(root.join("compiler/tests/fixtures/compat-threads"));
    if cfg!(target_env = "msvc") {
        build.flags(["/std:c11", "/utf-8", "/experimental:c11atomics"]);
        // MSVC (ARM64 and, as the windows-2022 x64 runner showed, x64)
        // diagnoses intentionally constant source-level conditions as C4127.
        // Keep /W4 and /WX for every other warning; these conditions are
        // executable fixtures for the checked bitwise and control-flow
        // lowering, not accidental dead code.
        build.flag("/wd4127");
        if header.is_some() {
            // Constant-failure hooks intentionally make success arms unreachable.
            build.flag("/wd4702");
        }
    } else {
        build.flag("-std=c11");
    }
    for define in defines {
        let (name, value) = define
            .strip_prefix("-D")
            .unwrap()
            .split_once('=')
            .map_or((define.strip_prefix("-D").unwrap(), None), |(k, v)| {
                (k, Some(v))
            });
        build.define(name, value);
    }
    let compiler = build.get_compiler();
    if require_cl {
        let compiler_name = compiler
            .path()
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        assert!(
            compiler.is_like_msvc() && compiler_name.eq_ignore_ascii_case("cl"),
            "native MSVC evidence requires Visual Studio cl.exe, got {}",
            compiler.path().display()
        );
    }
    let mut command = compiler.to_command();
    command.current_dir(&directory);
    if let Some(header) = header {
        let path = directory.join("fault.h");
        fs::write(&path, header).unwrap();
        if compiler.is_like_msvc() {
            command.arg(format!("/FI{}", path.display()));
        } else {
            command.arg("-include").arg(path);
        }
    }
    command.arg(&c);
    command.args(&extra_source_paths);
    command.args(libraries);
    if compiler.is_like_msvc() {
        command.arg(format!("/Fe:{}", exe.display()));
    } else {
        command.arg("-o").arg(&exe);
    }
    let compiled = command.output().unwrap();
    assert!(
        compiled.status.success(),
        "{command:?}\n{compiled:?}\nartifact: {}",
        c.display()
    );
    let mut child = Command::new(&exe)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    while child.try_wait().unwrap().is_none() {
        if std::time::Instant::now() >= deadline {
            child.kill().unwrap();
            let output = child.wait_with_output().unwrap();
            panic!(
                "Windows thread fixture timed out: {output:?}; artifact: {}",
                c.display()
            );
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let mut output = child.wait_with_output().unwrap();
    // CRT text streams use CRLF; trace contents are platform-independent.
    output.stdout = String::from_utf8(output.stdout)
        .unwrap()
        .replace("\r\n", "\n")
        .into_bytes();
    output.stderr = String::from_utf8(output.stderr)
        .unwrap()
        .replace("\r\n", "\n")
        .into_bytes();
    fs::remove_dir_all(directory).unwrap();
    output
}
