//! Card W45-AUDIT-CHECKPOINTS: record the source tree's `HEAD` commit in the
//! binary as `QCORE_AUDIT_BUILD_COMMIT`, the analyzer build commit a `.qo`
//! checkpoint's identity names beside the binary's own SHA-256.
//!
//! An explicit `QCORE_AUDIT_BUILD_COMMIT` in the build environment wins. Without
//! Git or a repository (a packaged build), nothing is recorded and the identity
//! carries no commit; the binary digest remains the identity either way. The
//! commit is `HEAD`'s, not a claim that the tree was clean: uncommitted changes
//! are reflected by the binary digest, not by this string.

use std::path::{Path, PathBuf};
use std::process::Command;

fn git(directory: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(directory)
        .args(args)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout)
        .ok()
        .map(|text| text.trim().to_owned())
        .filter(|text| !text.is_empty())
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=QCORE_AUDIT_BUILD_COMMIT");
    if std::env::var_os("QCORE_AUDIT_BUILD_COMMIT").is_some() {
        return;
    }
    let Some(manifest) = std::env::var_os("CARGO_MANIFEST_DIR").map(PathBuf::from) else {
        return;
    };
    let Some(commit) = git(&manifest, &["rev-parse", "--verify", "HEAD^{commit}"]) else {
        return;
    };
    println!("cargo:rustc-env=QCORE_AUDIT_BUILD_COMMIT={commit}");
    // Rebuild when HEAD moves: its own file, the branch it names, packed refs.
    let mut watched = vec!["HEAD".to_owned(), "packed-refs".to_owned()];
    if let Some(branch) = git(&manifest, &["symbolic-ref", "-q", "HEAD"]) {
        watched.push(branch);
    }
    for name in watched {
        if let Some(path) = git(&manifest, &["rev-parse", "--git-path", &name]) {
            let path = manifest.join(path);
            if path.is_file() {
                println!("cargo:rerun-if-changed={}", path.display());
            }
        }
    }
}
