//! What a process-provider fixture must do so that a concurrent workload
//! cannot change its result (card `EXECUTION-QUOTA-LOAD-INDEPENDENCE`).
//!
//! Included with `#[path]` by the verifier-host test binaries that spawn shell
//! fixtures. Each item closes one mechanism the card measured; neither is a
//! retry, a sleep or a widened bound.

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

/// Writes `bytes` to `path` as an owner-only executable without this process
/// ever holding a write descriptor on the file.
///
/// `fs::write` from a multi-threaded test binary is not enough, even though
/// Rust opens it close-on-exec: while the descriptor is open, another test
/// thread's spawn can `fork`, and the forked child holds a duplicate until its
/// own `exec` — after the confining `pre_exec` steps, or never for the host's
/// namespace probe, which `_exit`s. An `execve` of the file in that window
/// fails with `ETXTBSY` (`Spawn("Text file busy (os error 26)")`). Writing to a
/// temporary file and renaming does not help: the rename keeps the inode that
/// was open for writing. Here the only write descriptor belongs to a
/// single-threaded `cat` that forks nothing and has exited before this
/// returns. An existing file is truncated in place, so its inode is kept.
pub fn write_executable(path: &Path, bytes: &[u8]) {
    let mut writer = Command::new("/bin/sh")
        .arg("-c")
        .arg("umask 077 && /bin/cat > \"$1\" && /bin/chmod 700 \"$1\"")
        .arg("sh")
        .arg(path)
        .stdin(Stdio::piped())
        .spawn()
        .expect("the fixture writer starts");
    writer
        .stdin
        .take()
        .expect("the fixture writer's input is piped")
        .write_all(bytes)
        .expect("the fixture writer reads its input");
    let status = writer.wait().expect("the fixture writer exits");
    assert!(
        status.success(),
        "the fixture writer failed for {}: {status}",
        path.display()
    );
}
