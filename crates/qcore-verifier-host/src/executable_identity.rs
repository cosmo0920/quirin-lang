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

//! Binding a *verified* package executable to the file that is actually run.
//!
//! [`crate::package::verify_executable_package`] decides a package's identity by
//! reading bytes: the declared SHA-256 and the manifest template embedded in the
//! executable (card `P01-F2`). Between that decision and the `execve` that
//! dispatches a request, the package files are ordinary files on an ordinary
//! filesystem, so a writer can replace or rewrite them — the time-of-check to
//! time-of-use window recorded open by P01-F2 §9.
//!
//! This module narrows that window. At verification the executable is opened
//! once and the descriptor is *kept*, together with the verified digest and the
//! file's `(device, inode)` identity. Immediately before every spawn,
//! [`VerifiedExecutable::revalidate`] re-reads the whole file **through that
//! held descriptor** and re-compares the digest, then re-resolves the path and
//! requires it to still name the same file. Together those two checks decide:
//!
//! * the verified file description still carries the verified bytes — an
//!   in-place rewrite of the same inode is refused ([`IdentityRefusal::ContentChanged`]);
//! * the path the spawn will hand to `execve` still names that same file — an
//!   unlink-and-recreate, a rename over it, or a removal is refused
//!   ([`IdentityRefusal::PathReplaced`]).
//!
//! **What this is not.** The window is narrowed, not closed. After the last
//! check returns, and before the kernel reads the image, a writer with access to
//! the package directory can still swap the file; the residual is measured and
//! recorded in `docs/reports/remaining-p01-f3/README.md` rather than claimed
//! away. Executing the held descriptor itself (`fexecve`, or `/proc/self/fd/N`)
//! would close that last gap, and is deliberately not done here: it is measured
//! not to work for an interpreted (`#!`) package, and it would require the
//! confining profile's Landlock policy to admit `/proc`, which is a wider
//! filesystem reach than the package root it admits today. That remains open as
//! `P01-F3-FDEXEC`.
//!
//! Nothing here attests *who built* the executable. An actor who can produce the
//! binary controls its embedded template and its digest alike; only signatures
//! would change that, and there are none.

use std::fmt;
use std::fs::File;
use std::path::{Path, PathBuf};

use crate::PluginManifest;
use crate::embedded_template::{TemplateBindingError, verify_embedded_template_binding};

/// Why a verified executable could not be confirmed unchanged at spawn time.
///
/// Every variant is a refusal. None of them downgrades to "run it anyway", and
/// none of them is reported as a program `Unsafe`: the caller turns them into a
/// scoped `Unknown` exactly as it does a profile refusal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IdentityRefusal {
    /// The held descriptor could not be re-read.
    Unreadable(String),
    /// The verified file description no longer carries the verified bytes: the
    /// same inode was rewritten in place after verification.
    ContentChanged {
        verified: String,
        observed: String,
        bytes: usize,
    },
    /// The executable path no longer names the verified file: it was removed,
    /// replaced by a fresh file, renamed over, or is no longer a regular file.
    PathReplaced { path: String, detail: String },
    /// The re-read bytes no longer bind the installed manifest to an embedded
    /// template.
    ///
    /// Retained as defense in depth. It is unreachable while the digest
    /// comparison above runs first — reaching it needs bytes whose SHA-256
    /// equals the verified digest but whose embedded template differs — and the
    /// report records that measurement rather than pretending otherwise.
    TemplateUnbound(TemplateBindingError),
}

impl fmt::Display for IdentityRefusal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unreadable(detail) => write!(
                formatter,
                "verified executable could not be re-read before spawning: {detail}"
            ),
            Self::ContentChanged {
                verified,
                observed,
                bytes,
            } => write!(
                formatter,
                "verified executable changed since verification: the same file now hashes to \
                 {observed} over {bytes} bytes, not the verified {verified}; refusing to spawn it"
            ),
            Self::PathReplaced { path, detail } => write!(
                formatter,
                "verified executable `{path}` was replaced since verification: {detail}; \
                 refusing to spawn it"
            ),
            Self::TemplateUnbound(error) => write!(
                formatter,
                "verified executable no longer binds its installed manifest before spawning: \
                 {error}"
            ),
        }
    }
}

impl std::error::Error for IdentityRefusal {}

/// The file identity a spawn re-checks. On Unix this is the `(device, inode)`
/// pair, which distinguishes a rewritten file from a replaced one; elsewhere
/// only the re-read content decides, and the path check is limited to "still a
/// regular file of the same length".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FileIdentity {
    #[cfg(unix)]
    device: u64,
    #[cfg(unix)]
    inode: u64,
    length: u64,
}

impl FileIdentity {
    fn of(metadata: &std::fs::Metadata) -> Self {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            Self {
                device: metadata.dev(),
                inode: metadata.ino(),
                length: metadata.len(),
            }
        }
        #[cfg(not(unix))]
        {
            Self {
                length: metadata.len(),
            }
        }
    }

    fn describe(&self) -> String {
        #[cfg(unix)]
        {
            format!(
                "device {}, inode {}, {} bytes",
                self.device, self.inode, self.length
            )
        }
        #[cfg(not(unix))]
        {
            format!("{} bytes", self.length)
        }
    }
}

/// An executable opened for verification, before its digest has been decided.
///
/// Constructing this reads the file exactly once; the same bytes are used for
/// the digest comparison and the template binding, so those two checks cannot
/// disagree about which content they inspected.
pub(crate) struct OpenExecutable {
    path: PathBuf,
    file: File,
    bytes: Vec<u8>,
    identity: FileIdentity,
}

impl OpenExecutable {
    pub(crate) fn open(path: &Path) -> std::io::Result<Self> {
        let file = File::open(path)?;
        let metadata = file.metadata()?;
        if !metadata.is_file() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "package entrypoint is not a regular file",
            ));
        }
        let identity = FileIdentity::of(&metadata);
        let bytes = read_all(&file)?;
        Ok(Self {
            path: path.to_path_buf(),
            file,
            bytes,
            identity,
        })
    }

    pub(crate) fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Records the digest the caller has just accepted, and keeps the descriptor
    /// open for the spawn-time re-check.
    pub(crate) fn into_verified(self, digest: String) -> VerifiedExecutable {
        VerifiedExecutable {
            path: self.path,
            file: self.file,
            digest,
            identity: self.identity,
        }
    }
}

/// A package executable whose bytes were verified, held open so that the same
/// file can be re-checked immediately before each spawn.
#[derive(Debug)]
pub struct VerifiedExecutable {
    path: PathBuf,
    file: File,
    digest: String,
    identity: FileIdentity,
}

impl VerifiedExecutable {
    /// The canonical path verification resolved.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The SHA-256 the package was verified against.
    pub fn digest(&self) -> &str {
        &self.digest
    }

    /// Re-check, immediately before a spawn, that the file this provider will
    /// execute is still the file that was verified.
    ///
    /// Order is deliberate: the content comparison runs first, so a rewritten
    /// file is named as changed content rather than as a manifest divergence,
    /// and the path identity check runs last, as close to the spawn as this
    /// code structure allows.
    pub fn revalidate(&self, manifest: &PluginManifest) -> Result<(), IdentityRefusal> {
        let bytes = read_all(&self.file).map_err(|error| {
            IdentityRefusal::Unreadable(format!("{} : {error}", self.path.display()))
        })?;
        let observed = qcore_schemas::sha256_hex(&bytes);
        if observed != self.digest {
            return Err(IdentityRefusal::ContentChanged {
                verified: self.digest.clone(),
                observed,
                bytes: bytes.len(),
            });
        }
        verify_embedded_template_binding(manifest, &bytes)
            .map_err(IdentityRefusal::TemplateUnbound)?;
        let metadata =
            std::fs::metadata(&self.path).map_err(|error| IdentityRefusal::PathReplaced {
                path: self.path.display().to_string(),
                detail: error.to_string(),
            })?;
        if !metadata.is_file() {
            return Err(IdentityRefusal::PathReplaced {
                path: self.path.display().to_string(),
                detail: "the path is no longer a regular file".into(),
            });
        }
        let identity = FileIdentity::of(&metadata);
        if identity != self.identity {
            return Err(IdentityRefusal::PathReplaced {
                path: self.path.display().to_string(),
                detail: format!(
                    "the path now names {}, not the verified {}",
                    identity.describe(),
                    self.identity.describe()
                ),
            });
        }
        Ok(())
    }
}

/// Reads a whole file through an already-open descriptor, without consuming or
/// disturbing its cursor.
///
/// Reading through the held descriptor is the point: a path-based read would
/// follow whatever the path names *now*, which is exactly the substitution the
/// check exists to detect.
#[cfg(unix)]
fn read_all(file: &File) -> std::io::Result<Vec<u8>> {
    use std::os::unix::fs::FileExt;
    let length = file.metadata()?.len();
    let mut bytes = Vec::with_capacity(usize::try_from(length).unwrap_or(0));
    let mut offset = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file.read_at(&mut buffer, offset)?;
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&buffer[..read]);
        offset += read as u64;
    }
    Ok(bytes)
}

#[cfg(not(unix))]
fn read_all(file: &File) -> std::io::Result<Vec<u8>> {
    use std::io::{Read, Seek, SeekFrom};
    let mut handle = file.try_clone()?;
    handle.seek(SeekFrom::Start(0))?;
    let mut bytes = Vec::new();
    handle.read_to_end(&mut bytes)?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn scratch(name: &str) -> PathBuf {
        let directory = std::env::temp_dir().join(format!(
            "qcore-p01f3-{}-{}-{name}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        directory
    }

    fn manifest(digest: &str) -> PluginManifest {
        use std::collections::{BTreeMap, BTreeSet};
        PluginManifest {
            plugin_id: qcore_schemas::plugin::PluginId::new("example.identity").unwrap(),
            package_version: "1.0.0".into(),
            protocol_versions: BTreeSet::from([1]),
            evidence_versions: BTreeSet::from([1]),
            entrypoint: crate::PluginEntrypoint::Executable {
                path: "provider".into(),
            },
            package_digest: digest.to_owned(),
            targets: vec![crate::SupportedTarget {
                library: "example".into(),
                revisions: BTreeSet::from(["*".into()]),
                profiles: BTreeSet::from(["*".into()]),
                dialects: BTreeSet::from(["c11".into()]),
                platforms: BTreeSet::from(["*".into()]),
                backends: BTreeSet::from(["*".into()]),
            }],
            capabilities: BTreeSet::from([
                qcore_schemas::plugin::VerifierCapability::ResourceContracts,
            ]),
            required_foreign_summaries: BTreeSet::new(),
            required_observation_hooks: BTreeSet::new(),
            filesystem_inputs: BTreeSet::new(),
            network_access: false,
            limits: crate::PluginLimits {
                timeout_ms: 5_000,
                memory_bytes: 268_435_456,
                output_bytes: 4_194_304,
                additional: BTreeMap::new(),
            },
            trust_class: qcore_schemas::plugin::PluginTrustClass::FirstParty,
            certificate_checker: "qcore.example.v1".into(),
            lean_theorem_family: "QCore.Example".into(),
        }
    }

    fn write_package(path: &Path, manifest: &PluginManifest) -> String {
        let template = serde_json::to_string(manifest).unwrap();
        let body = format!(
            "#!/bin/sh\nexit 0\n{}\n",
            crate::embedded_template::envelope(&template)
        );
        let mut file = std::fs::File::create(path).unwrap();
        file.write_all(body.as_bytes()).unwrap();
        drop(file);
        qcore_schemas::sha256_hex(&std::fs::read(path).unwrap())
    }

    /// Builds a package executable whose embedded template matches `installed`
    /// except for the digest slot, and returns the verified handle.
    fn verified(directory: &Path) -> (VerifiedExecutable, PluginManifest) {
        let path = directory.join("provider");
        let template = manifest(&"0".repeat(64));
        let digest = write_package(&path, &template);
        let mut installed = template.clone();
        installed.package_digest = digest.clone();
        let opened = OpenExecutable::open(&path).unwrap();
        assert_eq!(qcore_schemas::sha256_hex(opened.bytes()), digest);
        verify_embedded_template_binding(&installed, opened.bytes()).unwrap();
        (opened.into_verified(digest), installed)
    }

    #[test]
    fn an_unchanged_executable_revalidates() {
        let directory = scratch("unchanged");
        let (executable, installed) = verified(&directory);
        executable.revalidate(&installed).unwrap();
        // Repeatable: the check is not a one-shot consumption of the descriptor.
        executable.revalidate(&installed).unwrap();
        std::fs::remove_dir_all(&directory).ok();
    }

    #[test]
    fn an_in_place_rewrite_is_refused_as_changed_content() {
        let directory = scratch("rewrite");
        let (executable, installed) = verified(&directory);
        let path = directory.join("provider");
        // `write` truncates in place, so the inode is unchanged: only the
        // content comparison can catch this.
        let mut bytes = std::fs::read(&path).unwrap();
        bytes.push(b'\n');
        std::fs::write(&path, &bytes).unwrap();
        let error = executable.revalidate(&installed).unwrap_err();
        let IdentityRefusal::ContentChanged { verified, .. } = &error else {
            panic!("expected changed content, got {error}");
        };
        assert_eq!(verified, executable.digest());
        assert!(error.to_string().contains("changed since verification"));
        std::fs::remove_dir_all(&directory).ok();
    }

    #[test]
    fn a_stripped_envelope_is_refused_before_the_template_check() {
        let directory = scratch("stripped");
        let (executable, installed) = verified(&directory);
        let path = directory.join("provider");
        std::fs::write(&path, b"#!/bin/sh\nexit 0\n").unwrap();
        // Changing the bytes moves the digest, so the content comparison names
        // it first. The template arm stays a defense-in-depth check.
        assert!(matches!(
            executable.revalidate(&installed).unwrap_err(),
            IdentityRefusal::ContentChanged { .. }
        ));
        std::fs::remove_dir_all(&directory).ok();
    }

    #[test]
    #[cfg(unix)]
    fn an_unlink_and_recreate_is_refused_as_a_replaced_path() {
        let directory = scratch("replace");
        let (executable, installed) = verified(&directory);
        let path = directory.join("provider");
        let bytes = std::fs::read(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        // Byte-identical content at a *different* inode: only the path identity
        // check can catch this, and it must, because the fresh file is what an
        // `execve` on the path would load.
        std::fs::write(&path, &bytes).unwrap();
        let error = executable.revalidate(&installed).unwrap_err();
        assert!(
            matches!(error, IdentityRefusal::PathReplaced { .. }),
            "expected a replaced path, got {error}"
        );
        std::fs::remove_dir_all(&directory).ok();
    }

    // FileIdentity documents that non-Unix hosts compare regular-file status
    // and length, not device/inode identity. Pin that limitation explicitly
    // instead of claiming that a byte-identical replacement is detected there.
    #[test]
    #[cfg(not(unix))]
    fn byte_identical_replacement_remains_a_non_unix_identity_limitation() {
        let directory = scratch("replace");
        let (executable, installed) = verified(&directory);
        let path = directory.join("provider");
        let bytes = std::fs::read(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        std::fs::write(&path, &bytes).unwrap();
        executable.revalidate(&installed).unwrap();
        std::fs::remove_dir_all(&directory).ok();
    }

    #[test]
    fn a_removed_executable_is_refused() {
        let directory = scratch("removed");
        let (executable, installed) = verified(&directory);
        std::fs::remove_file(directory.join("provider")).unwrap();
        assert!(matches!(
            executable.revalidate(&installed).unwrap_err(),
            IdentityRefusal::PathReplaced { .. }
        ));
        std::fs::remove_dir_all(&directory).ok();
    }

    #[test]
    fn a_directory_in_place_of_the_executable_is_refused() {
        let directory = scratch("directory");
        let (executable, installed) = verified(&directory);
        let path = directory.join("provider");
        std::fs::remove_file(&path).unwrap();
        std::fs::create_dir(&path).unwrap();
        assert!(matches!(
            executable.revalidate(&installed).unwrap_err(),
            IdentityRefusal::PathReplaced { .. }
        ));
        std::fs::remove_dir_all(&directory).ok();
    }

    #[test]
    fn opening_a_directory_as_an_executable_fails() {
        let directory = scratch("open-directory");
        assert!(OpenExecutable::open(&directory).is_err());
        std::fs::remove_dir_all(&directory).ok();
    }
}
