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

//! What a process fixture must do so that a concurrent workload cannot change
//! its result (cards `EXECUTION-QUOTA-LOAD-INDEPENDENCE` and
//! `PROCESS-FIXTURES-FS-WRITE-OUTSIDE-HOST-CRATE`).
//!
//! Included with `#[path]` by every test or example that writes an executable
//! it, or a process it starts, later executes: the verifier-host binaries
//! (through `crates/qcore-verifier-host/tests/support/process_fixture.rs`),
//! the compiler's plugin tests and follow-up example, the audit driver's CLI
//! tests, the gateway's Clang-wrapper tests and the provider conformance
//! tests. This directory is not a test target. Each item closes one mechanism
//! that was measured; neither is a retry, a sleep or a widened bound. A file
//! that is never executed (a manifest, a response frame, source text) is
//! written with `fs::write` as usual.
#![allow(dead_code)]

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

/// The first command of every fixture script: read the whole request frame to
/// end of file, as a conforming provider reads its request before answering.
///
/// Without it a fixture can exit before the host has written the request; the
/// host then meets a closed pipe and refuses the run with `Io(Broken pipe)`,
/// which is the host's deliberate rule (a successful exit cannot admit a
/// partial request, `VERIFIER-HOST-CLOSED-INPUT-DEADLINE`). Whether that
/// happened depended only on whether the host was descheduled between `spawn`
/// and its first write for longer than the fixture ran. The host closes the
/// pipe once the whole frame is written, so this loop ends only after the
/// request is complete, and the fixture's own exit can no longer race the
/// host's write. `read` is a shell builtin: it starts no process, so no quota
/// or `RLIMIT_NPROC` count a row asserts moves.
pub const READ_REQUEST: &str = "while read -r line; do :; done";

/// Writes `bytes` to `path` as an owner-only (`0o700`) executable without this
/// process ever holding a write descriptor on the file.
///
/// `fs::write` (or `fs::copy`) from a multi-threaded test binary is not
/// enough, even though Rust opens it close-on-exec: while the descriptor is
/// open, another test thread's spawn can `fork`, and the forked child holds a
/// duplicate until its own `exec` — after the confining `pre_exec` steps, or
/// never for the host's namespace probe, which `_exit`s. An `execve` of the
/// file in that window, by this process or by any process it starts, fails
/// with `ETXTBSY` (`Spawn("Text file busy (os error 26)")`). Writing to a
/// temporary file and renaming does not help: the rename keeps the inode that
/// was open for writing. Here the only write descriptor belongs to a
/// single-threaded `cat` that forks nothing and has exited before this
/// returns. An existing file is truncated in place, so its inode is kept.
pub fn write_executable(path: &Path, bytes: &[u8]) {
    write_executable_mode(path, bytes, 0o700);
}

/// [`write_executable`] with an explicit permission `mode` (for example
/// `0o755`, the mode a site installed before it used this writer). The file
/// is created under `umask 077` and then given exactly `mode`.
pub fn write_executable_mode(path: &Path, bytes: &[u8], mode: u32) {
    let mut writer = Command::new("/bin/sh")
        .arg("-c")
        .arg("umask 077 && /bin/cat > \"$1\" && /bin/chmod \"$2\" \"$1\"")
        .arg("sh")
        .arg(path)
        .arg(format!("{mode:o}"))
        .stdin(Stdio::piped())
        .spawn()
        .expect("the fixture writer starts");
    // The input is closed (dropped) before the wait, so `cat` sees its end.
    // A writer that failed to open the file breaks this pipe; its own status
    // is the more useful report, so both are checked after it exits.
    let sent = writer
        .stdin
        .take()
        .expect("the fixture writer's input is piped")
        .write_all(bytes);
    let status = writer.wait().expect("the fixture writer exits");
    assert!(
        status.success() && sent.is_ok(),
        "the fixture writer failed for {}: {status}, input {sent:?}",
        path.display()
    );
}

/// Copies the executable at `from` to `to` with `mode` through
/// [`write_executable_mode`], in place of `fs::copy`, whose destination
/// descriptor is the same hazard. The source is only read.
pub fn copy_executable(from: &Path, to: &Path, mode: u32) {
    let bytes = std::fs::read(from)
        .unwrap_or_else(|error| panic!("the executable {} reads: {error}", from.display()));
    write_executable_mode(to, &bytes, mode);
}
