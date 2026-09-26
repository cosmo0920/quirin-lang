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

/// Build a package in `output` (created; it must not exist) from
/// `executable`, installed under the file name its template's entrypoint
/// names. Returns the installed manifest.
///
/// # Errors
///
/// An executable without exactly one valid template, a template that is not
/// an executable entrypoint, or an existing output directory.
pub fn build(executable: &Path, output: &Path) -> Result<PluginManifest, String> {
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
    fs::write(&file, &bytes).map_err(|error| format!("{}: {error}", file.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&file, fs::Permissions::from_mode(0o755))
            .map_err(|error| error.to_string())?;
    }
    manifest.package_digest = sha256_hex(&bytes);
    manifest.validate().map_err(|error| error.to_string())?;
    let mut installed = serde_json::to_vec_pretty(&manifest).map_err(|error| error.to_string())?;
    installed.push(b'\n');
    fs::write(output.join(INSTALLED_MANIFEST), installed).map_err(|error| error.to_string())?;
    Ok(manifest)
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
