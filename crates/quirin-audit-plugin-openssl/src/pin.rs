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

//! The pinned OpenSSL release and the tree digests that bind a prepared
//! source to it.
//!
//! The release is the one Fluent Bit builds against on the measured routes:
//! Fluent Bit `046accf6` (system route) and `66910c10` (corpus) vendor no
//! OpenSSL; CMake's `find_package(OpenSSL)` finds the distribution's
//! `libssl-dev` 3.5.5 (`/usr/include`, Ubuntu `3.5.5-1ubuntu3.7`), whose
//! upstream is the `openssl-3.5.5` release. The plugin checks that upstream
//! release: the distribution's patches are outside the check and are named as
//! an assumption of every claim (`docs/reports/w48-openssl-contract-plugin/`).
//!
//! Three digests bind a tree, each fail-closed:
//!
//! * the release tarball's SHA-256, the one OpenSSL publishes beside it;
//! * [`Pin::source_tree_sha256`]: the extracted tree, before configuration;
//! * [`Pin::prepared_tree_sha256`]: the tree after the pinned configuration
//!   and header generation, every file but the [`BYPRODUCTS`], which spell
//!   the absolute directory and are read by nothing the check runs.
//!
//! A tree digest is the SHA-256 of the lines `<sha256>  <path>\n`, one per
//! regular file in byte order of the relative path; any other kind of entry
//! (a symbolic link, a device) refuses the tree.

use std::path::Path;

/// One pinned release.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pin {
    /// The release version (`VERSION.dat`'s `MAJOR.MINOR.PATCH`).
    pub version: &'static str,
    /// The release's top directory in the tarball.
    pub directory: &'static str,
    /// The release tag and the commit it names.
    pub tag: &'static str,
    pub tag_commit: &'static str,
    /// Where the tarball is published.
    pub tarball_url: &'static str,
    pub tarball_sha256: &'static str,
    pub source_tree_sha256: &'static str,
    pub prepared_tree_sha256: &'static str,
    /// The configuration the definitions are read under.
    pub configure: &'static [&'static str],
}

/// The pinned OpenSSL release.
pub const OPENSSL: Pin = Pin {
    version: "3.5.5",
    directory: "openssl-3.5.5",
    tag: "openssl-3.5.5",
    tag_commit: "67b5686b4419b4cb8caa502711c41815f5279751",
    tarball_url: "https://github.com/openssl/openssl/releases/download/openssl-3.5.5/openssl-3.5.5.tar.gz",
    tarball_sha256: "b28c91532a8b65a1f983b4c28b7488174e4a01008e29ce8e69bd789f28bc2a89",
    source_tree_sha256: "d62a5bdb90f2eb3cf772c0416bcf25178ea70d4e2379a13c04fed7d429edd106",
    prepared_tree_sha256: "8364e1c0d8e85f8d3ed592e59be9e545c0e4ddde0c08adf8131f376567e0bda4",
    configure: &["linux-x86_64", "no-asm"],
};

/// Files the configuration writes that spell the absolute build directory;
/// the check reads none of them, so the prepared digest leaves them out.
pub const BYPRODUCTS: [&str; 6] = [
    "Makefile",
    "builddata.pm",
    "configdata.pm",
    "libcrypto.pc",
    "libssl.pc",
    "openssl.pc",
];

/// The digest of every regular file under `root` but `exclude`.
///
/// # Errors
///
/// An unreadable entry, a non-UTF-8 name, or an entry that is neither a
/// regular file nor a directory.
pub fn tree_digest(root: &Path, exclude: &[&str]) -> Result<(String, usize), String> {
    let mut files = Vec::new();
    walk(root, root, &mut files)?;
    files.retain(|(relative, _)| !exclude.contains(&relative.as_str()));
    files.sort();
    let mut text = String::new();
    for (relative, digest) in &files {
        text.push_str(digest);
        text.push_str("  ");
        text.push_str(relative);
        text.push('\n');
    }
    Ok((qcore_schemas::sha256_hex(text.as_bytes()), files.len()))
}

fn walk(root: &Path, directory: &Path, files: &mut Vec<(String, String)>) -> Result<(), String> {
    let entries =
        std::fs::read_dir(directory).map_err(|error| format!("{}: {error}", directory.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("{}: {error}", directory.display()))?;
        let path = entry.path();
        let kind = entry
            .file_type()
            .map_err(|error| format!("{}: {error}", path.display()))?;
        if kind.is_dir() {
            walk(root, &path, files)?;
        } else if kind.is_file() {
            let relative = path
                .strip_prefix(root)
                .map_err(|error| error.to_string())?
                .to_str()
                .ok_or_else(|| format!("non-UTF-8 path {}", path.display()))?
                .replace('\\', "/");
            let bytes =
                std::fs::read(&path).map_err(|error| format!("{}: {error}", path.display()))?;
            files.push((relative, qcore_schemas::sha256_hex(&bytes)));
        } else {
            return Err(format!(
                "{} is neither a regular file nor a directory",
                path.display()
            ));
        }
    }
    Ok(())
}

/// `MAJOR.MINOR.PATCH` of a release's `VERSION.dat`, with an empty
/// pre-release tag.
///
/// # Errors
///
/// A missing or malformed file, or a pre-release.
pub fn release_version(tree: &Path) -> Result<String, String> {
    let path = tree.join("VERSION.dat");
    let text =
        std::fs::read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))?;
    let value = |key: &str| {
        text.lines()
            .find_map(|line| line.strip_prefix(key)?.strip_prefix('='))
            .map(|value| value.trim().trim_matches('"').to_owned())
            .ok_or_else(|| format!("{}: no {key}", path.display()))
    };
    let pre = value("PRE_RELEASE_TAG")?;
    if !pre.is_empty() {
        return Err(format!("{}: a pre-release (`{pre}`)", path.display()));
    }
    Ok(format!(
        "{}.{}.{}",
        value("MAJOR")?,
        value("MINOR")?,
        value("PATCH")?
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> std::path::PathBuf {
        let directory = std::env::temp_dir().join(format!(
            "quirin-openssl-pin-{}-{name}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).unwrap();
        directory
    }

    #[test]
    fn a_tree_digest_is_order_free_and_sees_every_byte_and_name() {
        let a = scratch("a");
        std::fs::create_dir_all(a.join("x")).unwrap();
        std::fs::write(a.join("x/one.h"), "1").unwrap();
        std::fs::write(a.join("two.c"), "2").unwrap();
        std::fs::write(a.join("Makefile"), "/abs/path").unwrap();
        let (digest, count) = tree_digest(&a, &BYPRODUCTS).unwrap();
        assert_eq!(count, 2);
        std::fs::write(a.join("Makefile"), "/another/path").unwrap();
        assert_eq!(tree_digest(&a, &BYPRODUCTS).unwrap().0, digest);
        std::fs::write(a.join("two.c"), "3").unwrap();
        assert_ne!(tree_digest(&a, &BYPRODUCTS).unwrap().0, digest);
        std::fs::write(a.join("two.c"), "2").unwrap();
        std::fs::write(a.join("three.h"), "").unwrap();
        assert_ne!(tree_digest(&a, &BYPRODUCTS).unwrap().0, digest);
        std::fs::remove_file(a.join("three.h")).unwrap();
        assert_eq!(tree_digest(&a, &BYPRODUCTS).unwrap().0, digest);
        let _ = std::fs::remove_dir_all(&a);
    }

    #[cfg(unix)]
    #[test]
    fn a_symbolic_link_refuses_the_tree() {
        let a = scratch("link");
        std::fs::write(a.join("one.h"), "1").unwrap();
        std::os::unix::fs::symlink("one.h", a.join("two.h")).unwrap();
        assert!(tree_digest(&a, &[]).is_err());
        let _ = std::fs::remove_dir_all(&a);
    }

    #[test]
    fn the_release_version_is_read_and_a_pre_release_refused() {
        let a = scratch("version");
        std::fs::write(
            a.join("VERSION.dat"),
            "MAJOR=3\nMINOR=5\nPATCH=5\nPRE_RELEASE_TAG=\nBUILD_METADATA=\nRELEASE_DATE=\"27 Jan 2026\"\n",
        )
        .unwrap();
        assert_eq!(release_version(&a).unwrap(), "3.5.5");
        std::fs::write(
            a.join("VERSION.dat"),
            "MAJOR=3\nMINOR=6\nPATCH=0\nPRE_RELEASE_TAG=alpha1\n",
        )
        .unwrap();
        assert!(release_version(&a).is_err());
        let _ = std::fs::remove_dir_all(&a);
    }
}
