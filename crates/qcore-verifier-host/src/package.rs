//! Read-only executable package identity checks. This is a local snapshot check,
//! not signature verification, an OS sandbox or a semantic certificate.
use crate::{PackageLoadError, PluginEntrypoint, PluginManifest, ProcessProvider};
use std::path::Path;

/// Check manifest syntax, executable containment and its declared SHA-256.
/// Does not execute the package. Callers must protect package files against
/// changes after this check if they require an immutable execution identity.
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
    let bytes = std::fs::read(&executable).map_err(|e| error(e.to_string()))?;
    if qcore_schemas::sha256_hex(&bytes) != manifest.package_digest {
        return Err(error("executable digest mismatch".into()));
    }
    ProcessProvider::new(manifest, root).map_err(|e| error(e.to_string()))
}
