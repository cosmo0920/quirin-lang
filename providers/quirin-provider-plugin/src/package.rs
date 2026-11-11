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

//! Checker packages: build one from an executable written with
//! [`crate::checker`], and verify one.
//!
//! A package is a directory holding the executable and `verifier-plugin.json`,
//! the installed manifest: the template the executable embeds, with
//! `package_digest` set to the SHA-256 of the executable's bytes. This is the
//! shape `providers/package.py` installs for the first-party providers and the
//! host's `verify_executable_package` accepts; the host re-verifies every
//! package it runs, so [`verify`] is a user's early check, never the gate.
//!
//! The envelope markers are assembled at run time from two pieces, so no
//! binary that links this crate — a checker built with the SDK among them —
//! ever carries a second contiguous copy of a marker (the host refuses an
//! executable with two).
//!
//! On Unix, [`build`] never holds a write descriptor on the executable it
//! installs (card PLUGIN-PACKAGE-BUILD-WRITES-EXECUTABLE-IN-PROCESS). While a
//! process holds one, a fork by any of its other threads copies it into the
//! child, which keeps it until its own `exec`; an `execve` of the package in
//! that window fails with `ETXTBSY`. Close-on-exec does not close the window,
//! and neither does a temporary file and `rename` (the inode keeps its writer
//! count), so the bytes go to a separate, single-threaded `tee` process that
//! has exited before `build` continues.

use std::fs;
use std::path::Path;

use qcore_schemas::plugin::{PluginEntrypoint, PluginManifest};

use crate::sha256_hex;

/// The installed manifest's file name.
pub const INSTALLED_MANIFEST: &str = "verifier-plugin.json";

fn marker(edge: &str) -> String {
    format!("<<<qcore.provider-template.{}:{edge}>>>", "v1")
}

fn find_all(haystack: &[u8], needle: &[u8]) -> Vec<usize> {
    haystack
        .windows(needle.len())
        .enumerate()
        .filter(|(_, window)| *window == needle)
        .map(|(index, _)| index)
        .collect()
}

/// The manifest template an executable embeds: exactly one envelope, one
/// JSON object inside it, a valid manifest.
///
/// # Errors
///
/// No envelope, more than one, or a payload that is not a valid manifest.
pub fn embedded_template(bytes: &[u8]) -> Result<PluginManifest, String> {
    let (begin, end) = (marker("begin"), marker("end"));
    let (begins, ends) = (
        find_all(bytes, begin.as_bytes()),
        find_all(bytes, end.as_bytes()),
    );
    let (&[start], &[stop]) = (begins.as_slice(), ends.as_slice()) else {
        return Err(format!(
            "the executable must embed exactly one manifest template envelope ({} begin, {} end \
             marker(s))",
            begins.len(),
            ends.len()
        ));
    };
    let payload = bytes
        .get(start + begin.len()..stop)
        .ok_or("the manifest template envelope is damaged")?;
    let payload = std::str::from_utf8(payload).map_err(|error| error.to_string())?;
    let (open, close) = (
        payload
            .find('{')
            .ok_or("the envelope carries no JSON object")?,
        payload
            .rfind('}')
            .ok_or("the envelope carries no JSON object")?,
    );
    let manifest: PluginManifest =
        serde_json::from_str(&payload[open..=close]).map_err(|error| error.to_string())?;
    manifest.validate().map_err(|error| error.to_string())?;
    Ok(manifest)
}

/// The out-of-process writer [`build`] installs the executable through on
/// Unix: `tee -- FILE`, resolved through `PATH`, fed the executable's bytes
/// on its standard input. `tee` is POSIX (coreutils, BusyBox and the macOS
/// base system ship it), single-threaded, and forks nothing, so its write
/// descriptor is gone once it has exited.
#[cfg(unix)]
const EXECUTABLE_WRITER: &[&str] = &["tee", "--"];

/// Build a package in `output` (created; it must not exist) from
/// `executable`, installed under the file name its template's entrypoint
/// names, with mode `0o755` on Unix. Returns the installed manifest.
///
/// On Unix the executable is written by a separate `tee` process found on
/// `PATH`, never through a descriptor of the calling process, so a caller may
/// build a package on
/// one thread while other threads fork and spawn, and execute it as soon as
/// this returns. The bytes the writer installed are read back and must be
/// exactly the bytes digested.
///
/// # Errors
///
/// An executable without exactly one valid template, a template that is not
/// an executable entrypoint, or an existing output directory; on Unix, a
/// writer that cannot start or fails, or a file that is not a regular file
/// holding exactly the executable's bytes. The installed manifest is written
/// last, so a failed build leaves no package that [`verify`] accepts.
pub fn build(executable: &Path, output: &Path) -> Result<PluginManifest, String> {
    #[cfg(unix)]
    let install =
        |file: &Path, bytes: &[u8]| install_out_of_process(EXECUTABLE_WRITER, file, bytes);
    // `ETXTBSY` is a Unix `execve` rule, and std opens Windows handles
    // non-inheritable, so elsewhere the write is unchanged.
    #[cfg(not(unix))]
    let install = |file: &Path, bytes: &[u8]| {
        fs::write(file, bytes).map_err(|error| format!("{}: {error}", file.display()))
    };
    build_with(executable, output, install)
}

fn build_with(
    executable: &Path,
    output: &Path,
    install: impl FnOnce(&Path, &[u8]) -> Result<(), String>,
) -> Result<PluginManifest, String> {
    let bytes =
        fs::read(executable).map_err(|error| format!("{}: {error}", executable.display()))?;
    let mut manifest = embedded_template(&bytes)?;
    let PluginEntrypoint::Executable { path } = &manifest.entrypoint else {
        return Err("a checker package's template must name an executable entrypoint".into());
    };
    let file = output.join(path);
    fs::create_dir(output).map_err(|error| format!("{}: {error}", output.display()))?;
    if let Some(parent) = file.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    install(&file, &bytes)?;
    manifest.package_digest = sha256_hex(&bytes);
    manifest.validate().map_err(|error| error.to_string())?;
    let mut installed = serde_json::to_vec_pretty(&manifest).map_err(|error| error.to_string())?;
    installed.push(b'\n');
    fs::write(output.join(INSTALLED_MANIFEST), installed).map_err(|error| error.to_string())?;
    Ok(manifest)
}

/// Write `bytes` to `file` (which must not exist) through the process
/// `writer` names, with `file` as its last argument and `bytes` on its
/// standard input; then check that `file` is a regular file holding exactly
/// `bytes`, and set its mode to `0o755` by path. This process holds no write
/// descriptor on `file` at any point.
#[cfg(unix)]
fn install_out_of_process(writer: &[&str], file: &Path, bytes: &[u8]) -> Result<(), String> {
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;
    use std::process::{Command, Stdio};

    let name = writer.join(" ");
    let Some((program, arguments)) = writer.split_first() else {
        return Err("no executable writer is configured".into());
    };
    let mut child = Command::new(program)
        .args(arguments)
        .arg(file)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            format!(
                "{}: the executable writer `{name}` could not start: {error} (on Unix the \
                 package's executable is written by `tee` on PATH, so that this process never \
                 holds a write descriptor on it)",
                file.display()
            )
        })?;
    let mut input = child
        .stdin
        .take()
        .ok_or("the executable writer's input is not piped")?;
    // The bytes are fed on a scoped thread while this one collects the
    // writer's error output and status, so neither pipe can stall the other.
    // Dropping `input` when the feeder ends is the writer's end of file.
    let (delivered, waited) = std::thread::scope(|scope| {
        let feeder = scope.spawn(move || input.write_all(bytes));
        let waited = child.wait_with_output();
        (feeder.join(), waited)
    });
    let waited = waited.map_err(|error| {
        format!(
            "{}: waiting for the executable writer `{name}`: {error}",
            file.display()
        )
    })?;
    if !waited.status.success() {
        return Err(format!(
            "{}: the executable writer `{name}` failed ({}): {}",
            file.display(),
            waited.status,
            String::from_utf8_lossy(&waited.stderr).trim()
        ));
    }
    match delivered {
        Ok(Ok(())) => {}
        Ok(Err(error)) => {
            return Err(format!(
                "{}: the executable writer `{name}` did not read the whole executable: {error}",
                file.display()
            ));
        }
        Err(_) => return Err("the executable writer's input thread panicked".into()),
    }
    let kind = fs::symlink_metadata(file)
        .map_err(|error| {
            format!(
                "{}: the executable writer `{name}` left no file: {error}",
                file.display()
            )
        })?
        .file_type();
    if !kind.is_file() {
        return Err(format!(
            "{}: the executable writer `{name}` left something other than a regular file",
            file.display()
        ));
    }
    let written = fs::read(file).map_err(|error| format!("{}: {error}", file.display()))?;
    if written != bytes {
        return Err(format!(
            "{}: the executable writer `{name}` wrote {} bytes that are not the {} bytes read \
             from the executable",
            file.display(),
            written.len(),
            bytes.len()
        ));
    }
    fs::set_permissions(file, fs::Permissions::from_mode(0o755))
        .map_err(|error| format!("{}: {error}", file.display()))
}

/// Verify a package directory: the installed manifest is valid, its
/// executable exists, its digest is the executable's, and every other field
/// equals the template the executable embeds.
///
/// # Errors
///
/// The first departure found.
pub fn verify(package: &Path) -> Result<PluginManifest, String> {
    let installed: PluginManifest = serde_json::from_slice(
        &fs::read(package.join(INSTALLED_MANIFEST))
            .map_err(|error| format!("{INSTALLED_MANIFEST}: {error}"))?,
    )
    .map_err(|error| format!("{INSTALLED_MANIFEST}: {error}"))?;
    installed.validate().map_err(|error| error.to_string())?;
    let PluginEntrypoint::Executable { path } = &installed.entrypoint else {
        return Err("a checker package names an executable entrypoint".into());
    };
    let bytes = fs::read(package.join(path)).map_err(|error| format!("{path}: {error}"))?;
    if sha256_hex(&bytes) != installed.package_digest {
        return Err("the executable's SHA-256 is not the installed package_digest".into());
    }
    let mut embedded = embedded_template(&bytes)?;
    embedded
        .package_digest
        .clone_from(&installed.package_digest);
    if embedded != installed {
        return Err(
            "the installed manifest differs from the template the executable embeds".into(),
        );
    }
    Ok(installed)
}

#[cfg(all(test, unix))]
mod tests {
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    static NEXT: AtomicUsize = AtomicUsize::new(0);

    fn scratch(tag: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "quirin-plugin-package-{tag}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        root
    }

    /// A stand-in executable: arbitrary bytes around one envelope holding the
    /// example checker's template (markers assembled at run time).
    fn stand_in(root: &Path) -> (PathBuf, Vec<u8>) {
        let mut bytes = b"\x7fELF stand-in\0\x01\x02".to_vec();
        bytes.extend(marker("begin").as_bytes());
        bytes.extend(include_bytes!("bin/example-checker-template.json"));
        bytes.extend(marker("end").as_bytes());
        bytes.extend(b"\0trailer");
        let path = root.join("stand-in");
        fs::write(&path, &bytes).unwrap();
        (path, bytes)
    }

    /// `/bin/sh -c SCRIPT sh`: the file to write arrives as `$1`.
    fn shell(script: &'static str) -> [&'static str; 4] {
        ["/bin/sh", "-c", script, "sh"]
    }

    #[test]
    fn the_writer_installs_exactly_the_bytes_with_mode_0755() {
        let root = scratch("writer");
        let file = root.join("checker");
        let bytes: Vec<u8> = (0..=255u8).cycle().take(300_000).collect();
        install_out_of_process(EXECUTABLE_WRITER, &file, &bytes).unwrap();
        assert_eq!(fs::read(&file).unwrap(), bytes);
        let metadata = fs::symlink_metadata(&file).unwrap();
        assert!(metadata.file_type().is_file());
        assert_eq!(metadata.permissions().mode() & 0o7777, 0o755);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn every_writer_failure_is_an_error_naming_its_cause() {
        let bytes = b"checker bytes".as_slice();
        let cases: [(&[&str], &str); 5] = [
            (&["/nonexistent/quirin-plugin-writer"], "could not start"),
            (
                &shell("cat >/dev/null; echo refused >&2; exit 3"),
                "failed (exit status: 3): refused",
            ),
            (&shell("cat >/dev/null"), "left no file"),
            (
                &shell("cat >/dev/null; ln -s /dev/null \"$1\""),
                "other than a regular file",
            ),
            (
                &shell("cat >/dev/null; printf x > \"$1\""),
                "wrote 1 bytes that are not the 13 bytes",
            ),
        ];
        for (writer, expected) in cases {
            let root = scratch("failure");
            let file = root.join("checker");
            let error = install_out_of_process(writer, &file, bytes).unwrap_err();
            assert!(error.contains(expected), "{writer:?}: {error}");
            assert!(error.contains(&file.display().to_string()), "{error}");
            fs::remove_dir_all(&root).unwrap();
        }
    }

    #[test]
    fn a_failed_writer_leaves_no_package_that_verifies() {
        let root = scratch("build");
        let (executable, bytes) = stand_in(&root);
        // The real writer: today's package, byte for byte.
        let good = root.join("good");
        let installed = build(&executable, &good).unwrap();
        assert_eq!(installed.package_digest, sha256_hex(&bytes));
        assert_eq!(verify(&good).unwrap(), installed);
        let mut expected = embedded_template(&bytes).unwrap();
        expected.package_digest = sha256_hex(&bytes);
        let mut manifest = serde_json::to_vec_pretty(&expected).unwrap();
        manifest.push(b'\n');
        assert_eq!(fs::read(good.join(INSTALLED_MANIFEST)).unwrap(), manifest);
        let file = good.join("quirin-plugin-example-checker");
        assert_eq!(fs::read(&file).unwrap(), bytes);
        assert_eq!(
            fs::metadata(&file).unwrap().permissions().mode() & 0o7777,
            0o755
        );
        // A writer that half-writes and fails: an error, no installed
        // manifest, and the directory does not verify.
        let bad = root.join("bad");
        let error = build_with(&executable, &bad, |file, bytes| {
            install_out_of_process(
                &shell("head -c 7 > \"$1\"; cat >/dev/null; exit 1"),
                file,
                bytes,
            )
        })
        .unwrap_err();
        assert!(error.contains("failed (exit status: 1)"), "{error}");
        assert!(!bad.join(INSTALLED_MANIFEST).exists());
        assert!(verify(&bad).is_err());
        fs::remove_dir_all(&root).unwrap();
    }
}
