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

//! Card W44-QUIRIN-AUDIT-PREPARE: `quirin-audit prepare` end to end on a
//! fixture checkout with stand-in `clang`, `clang++` and `cmake` programs, and
//! its fail-closed arms. Parity with the retired Python scripts on a real
//! Fluent Bit checkout is in `docs/reports/w44-quirin-audit-prepare/`.
#![cfg(unix)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

// The stand-in programs are written by a separate process, so a sibling test's concurrent
// spawn cannot make its `execve` fail with `ETXTBSY`
// (`tests/support/process_fixture.rs`). Data files stay `fs::write`.
#[path = "../../../tests/support/process_fixture.rs"]
mod process_fixture;

static NEXT: AtomicUsize = AtomicUsize::new(0);

const FAKE_CLANG: &str = "#!/bin/sh\necho 'fixture clang version 99.0.0'\n";

const FAKE_NOT_CLANG: &str = "#!/bin/sh\necho 'cc (GCC) 99.0.0'\n";

/// Stands in for CMake's Monkey configuration: the generated header in the
/// source directory, a cache naming both compilers, and a one-entry database.
const FAKE_CMAKE: &str = r#"#!/bin/sh
src=; build=; cc=; cxx=
while [ $# -gt 0 ]; do
  case "$1" in
    -S) src=$2; shift 2 ;;
    -B) build=$2; shift 2 ;;
    -DCMAKE_C_COMPILER=*) cc=${1#-DCMAKE_C_COMPILER=}; shift ;;
    -DCMAKE_CXX_COMPILER=*) cxx=${1#-DCMAKE_CXX_COMPILER=}; shift ;;
    *) shift ;;
  esac
done
mkdir -p "$build" "$src/include/monkey"
echo '/* generated */' > "$src/include/monkey/mk_static_plugins.h"
printf 'CMAKE_C_COMPILER:FILEPATH=%s\nCMAKE_CXX_COMPILER:FILEPATH=%s\n' "$cc" "$cxx" > "$build/CMakeCache.txt"
printf '[\n{\n  "directory": "%s",\n  "command": "%s -DMK_EVENT_LOOP_LIBEVENT -c %s/mk_core/mk_event.c",\n  "file": "%s/mk_core/mk_event.c"\n}\n]\n' "$build" "$cc" "$src" "$src" > "$build/compile_commands.json"
echo '-- Configuring done'
"#;

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn git(directory: &Path, args: &[&str]) {
    let output = Command::new("git")
        .current_dir(directory)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn executable(path: &Path, text: &str) {
    process_fixture::write_executable_mode(path, text.as_bytes(), 0o755);
}

struct Fixture {
    root: PathBuf,
    upstream: PathBuf,
    bin: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "qcore-prepare-cli-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let upstream = root.join("fluent-bit");
        let monkey = upstream.join("lib/monkey");
        fs::create_dir_all(monkey.join("mk_core")).unwrap();
        fs::write(monkey.join("mk_core/mk_event.c"), "int mk_event;\n").unwrap();
        fs::write(monkey.join("CMakeLists.txt"), "project(monkey C)\n").unwrap();
        git(&upstream, &["init", "--quiet"]);
        git(&upstream, &["add", "lib"]);
        git(
            &upstream,
            &[
                "-c",
                "user.name=Prepare Fixture",
                "-c",
                "user.email=prepare@example.invalid",
                "commit",
                "--quiet",
                "-m",
                "Prepare fixture",
            ],
        );
        let bin = root.join("bin");
        fs::create_dir_all(&bin).unwrap();
        executable(&bin.join("clang"), FAKE_CLANG);
        executable(&bin.join("clang++"), FAKE_CLANG);
        executable(&bin.join("not-clang"), FAKE_NOT_CLANG);
        executable(&bin.join("cmake"), FAKE_CMAKE);
        Self {
            root,
            upstream,
            bin,
        }
    }

    fn tool(&self, name: &str) -> String {
        self.bin.join(name).to_str().unwrap().to_owned()
    }

    fn prepare(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_quirin-audit"))
            .current_dir(&self.root)
            .env_remove("QCORE_CLANG")
            .arg("prepare")
            .args(args)
            .output()
            .unwrap()
    }

    fn monkey(&self, output: &Path) -> Output {
        let repository = repository();
        self.prepare(&[
            "monkey",
            "--repository",
            repository.to_str().unwrap(),
            "--fluent-bit",
            self.upstream.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--clang",
            &self.tool("clang"),
            "--clangxx",
            &self.tool("clang++"),
            "--cmake",
            &self.tool("cmake"),
        ])
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn the_monkey_step_prepares_records_and_reuses_only_an_unchanged_copy() {
    let fixture = Fixture::new();
    let out = fixture.root.join("inputs");
    let first = fixture.monkey(&out);
    assert!(first.status.success(), "{}", stderr(&first));
    // Preparation reports the physical output path, resolving temp-directory
    // symlinks such as /var -> /private/var on macOS.
    let database = out
        .join("monkey-build/compile_commands.json")
        .canonicalize()
        .unwrap();
    assert_eq!(
        String::from_utf8_lossy(&first.stdout).trim(),
        database.to_str().unwrap()
    );
    let marker = fs::read_to_string(out.join("qcore-monkey-inputs.json")).unwrap();
    // The script's insertion order and `json.dumps(indent=2)` layout.
    let keys: Vec<&str> = marker
        .lines()
        .filter(|line| line.starts_with("  \"") && !line.starts_with("   "))
        .map(|line| line.trim_start().split('"').nth(1).unwrap())
        .collect();
    assert_eq!(
        keys,
        [
            "schema_version",
            "source_revision",
            "upstream",
            "source",
            "build",
            "source_sha256",
            "status",
            "cmake_command",
            "compile_entries",
            "compiler",
            "compiler_version",
            "generated_source_sha256",
            "compile_database",
            "compile_database_sha256",
        ]
    );
    assert!(marker.starts_with("{\n  \"schema_version\": 1,\n"));
    assert!(marker.contains("\"status\": \"configured\""));
    assert!(marker.contains("\"include/monkey/mk_static_plugins.h\": \""));
    assert!(marker.contains("\"-DMK_EVENT_LOOP_LIBEVENT=ON\""));
    assert!(marker.ends_with("\n}\n"));
    assert_eq!(
        fs::read_to_string(out.join("monkey-source/mk_core/mk_event.c")).unwrap(),
        "int mk_event;\n"
    );
    assert!(
        fs::read_to_string(out.join("configure.log"))
            .unwrap()
            .contains("Configuring done")
    );

    // An unchanged output is reused.
    let again = fixture.monkey(&out);
    assert!(again.status.success(), "{}", stderr(&again));

    // A changed copy refuses.
    fs::write(out.join("monkey-source/mk_core/mk_event.c"), "int x;\n").unwrap();
    let changed = fixture.monkey(&out);
    assert!(!changed.status.success());
    assert!(
        stderr(&changed).contains("persistent source copy changed"),
        "{}",
        stderr(&changed)
    );
}

#[test]
fn the_monkey_step_refuses_local_changes_and_unmanaged_outputs() {
    let fixture = Fixture::new();
    let unmanaged = fixture.root.join("unmanaged");
    fs::create_dir_all(&unmanaged).unwrap();
    fs::write(unmanaged.join("stray"), "").unwrap();
    let refused = fixture.monkey(&unmanaged);
    assert!(!refused.status.success());
    assert!(
        stderr(&refused).contains("is not a managed QCore Monkey input directory"),
        "{}",
        stderr(&refused)
    );

    fs::write(
        fixture.upstream.join("lib/monkey/CMakeLists.txt"),
        "edited\n",
    )
    .unwrap();
    let dirty = fixture.monkey(&fixture.root.join("fresh"));
    assert!(!dirty.status.success());
    assert!(
        stderr(&dirty).contains("lib/monkey has local changes"),
        "{}",
        stderr(&dirty)
    );
}

#[test]
fn the_clang_build_step_refuses_before_cloning() {
    let fixture = Fixture::new();
    let repository = repository();
    let common = |output: &Path, clang: &str| {
        fixture.prepare(&[
            "clang-build",
            "--repository",
            repository.to_str().unwrap(),
            "--fluent-bit",
            fixture.upstream.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--clang",
            clang,
            "--clangxx",
            &fixture.tool("clang++"),
            "--cmake",
            &fixture.tool("cmake"),
        ])
    };
    let inside = common(&fixture.upstream.join("out"), &fixture.tool("clang"));
    assert!(!inside.status.success());
    assert!(
        stderr(&inside).contains("use an output outside the upstream checkout"),
        "{}",
        stderr(&inside)
    );
    let outside = fixture.root.join("out");
    let gcc = common(&outside, &fixture.tool("not-clang"));
    assert!(!gcc.status.success());
    assert!(
        stderr(&gcc).contains("does not identify itself as Clang"),
        "{}",
        stderr(&gcc)
    );
    let missing = common(&outside, "/no-such-qcore-dir/clang");
    assert!(!missing.status.success());
    assert!(
        stderr(&missing).contains("both Clang C and C++ compilers are required"),
        "{}",
        stderr(&missing)
    );
    assert!(!outside.join("fluent-bit-clang-source").exists());
}

#[test]
fn a_chained_run_refuses_another_configured_clang_before_building() {
    let fixture = Fixture::new();
    let repository = repository();
    let output = fixture.root.join("chained");
    let refused = Command::new(env!("CARGO_BIN_EXE_quirin-audit"))
        .current_dir(&fixture.root)
        .env("QCORE_CLANG", fixture.tool("not-clang"))
        .args([
            "prepare",
            "--repository",
            repository.to_str().unwrap(),
            "--fluent-bit",
            fixture.upstream.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--clang",
            &fixture.tool("clang"),
            "--clangxx",
            &fixture.tool("clang++"),
            "--cmake",
            &fixture.tool("cmake"),
            "--run",
            "--jobs",
            "2",
        ])
        .output()
        .unwrap();
    assert!(!refused.status.success());
    assert!(
        stderr(&refused).contains("is not the prepared C compiler"),
        "{}",
        stderr(&refused)
    );
    assert!(!output.exists());
}

#[test]
fn preparation_needs_the_repository() {
    let fixture = Fixture::new();
    let refused = fixture.prepare(&["monkey", "--fluent-bit", "fluent-bit"]);
    assert!(!refused.status.success());
    assert!(
        stderr(&refused).contains("is not the QCore-C repository"),
        "{}",
        stderr(&refused)
    );
}
