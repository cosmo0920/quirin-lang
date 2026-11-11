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

//! Read-only executable package identity checks. This is a local snapshot check,
//! not signature verification, an OS sandbox or a semantic certificate.
use crate::embedded_template::verify_embedded_template_binding;
use crate::executable_identity::OpenExecutable;
use crate::{PackageLoadError, PluginEntrypoint, PluginManifest, ProcessProvider};
use std::path::Path;

/// Check manifest syntax, executable containment, its declared SHA-256, and the
/// binding of every other declaration to the template embedded in the
/// executable's bytes.
///
/// Does not execute the package: the embedded template is *read* out of the
/// same bytes the digest is computed over
/// ([`crate::embedded_template`]). Consequently a schema-valid,
/// digest-intact edit of `limits`, `targets`, `capabilities` or any other
/// declared field — including the fields the `linux-rlimit-v1` and
/// `linux-confined-v1` execution profiles install — is refused by name rather
/// than accepted.
///
/// This remains a snapshot identity, not a signature, source authenticity or an
/// immutable-execution guarantee: an actor who can rebuild or forge the
/// executable controls its embedded template too, and the files may change
/// after this check. Callers must protect package files against changes after
/// this check if they require an immutable execution identity.
pub fn verify_executable_package(
    root: impl AsRef<Path>,
) -> Result<ProcessProvider, PackageLoadError> {
    let supplied = root.as_ref();
    let error = |message: String| PackageLoadError {
        path: supplied.display().to_string(),
        message,
    };
    let root = supplied.canonicalize().map_err(|e| error(e.to_string()))?;
    let manifest_path = root.join("verifier-plugin.json");
    let canonical_manifest = manifest_path
        .canonicalize()
        .map_err(|e| error(e.to_string()))?;
    if !canonical_manifest.starts_with(&root) {
        return Err(error("manifest escapes package directory".into()));
    }
    let bytes = std::fs::read(&canonical_manifest).map_err(|e| error(e.to_string()))?;
    let manifest: PluginManifest =
        serde_json::from_slice(&bytes).map_err(|e| error(e.to_string()))?;
    manifest.validate().map_err(|e| error(e.to_string()))?;
    let PluginEntrypoint::Executable { path } = &manifest.entrypoint else {
        return Err(error("expected an executable package".into()));
    };
    let executable = root
        .join(path)
        .canonicalize()
        .map_err(|e| error(e.to_string()))?;
    if !executable.starts_with(&root) || !executable.is_file() {
        return Err(error(
            "executable escapes package directory or is not a file".into(),
        ));
    }
    // Opened once and *kept* open: the same descriptor decides the digest here
    // and is re-read immediately before every spawn, so the time-of-check to
    // time-of-use window is narrowed rather than left open (card `P01-F3`).
    let opened = OpenExecutable::open(&executable).map_err(|e| error(e.to_string()))?;
    let digest = qcore_schemas::sha256_hex(opened.bytes());
    if digest != manifest.package_digest {
        return Err(error("executable digest mismatch".into()));
    }
    // Bind the remaining declarations to the executable's own embedded template,
    // still without executing it. A digest-intact manifest edit fails here.
    verify_embedded_template_binding(&manifest, opened.bytes())
        .map_err(|e| error(e.to_string()))?;
    ProcessProvider::verified(manifest, root, opened.into_verified(digest))
        .map_err(|e| error(e.to_string()))
}
