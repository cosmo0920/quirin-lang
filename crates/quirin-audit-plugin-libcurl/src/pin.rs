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

//! The pinned libcurl release, and the fail-closed check of a prepared tree
//! against it.
//!
//! **How the release was determined** (card W48-LIBCURL-CONTRACT-PLUGIN,
//! report `docs/reports/w48-libcurl-contract-plugin/README.md` §1). The
//! measured routes were prepared on Ubuntu 26.04 hosts (`Ubuntu clang
//! 21.1.8`, GNU ld 2.46); W43-LIBC-CONTRACTS records the routes' libcurl
//! declarations at `/usr/include/x86_64-linux-gnu/curl/curl.h`
//! (`libcurl4-openssl-dev`), and FLUENT-BIT-LINK-PREPARE links `fluent-bit`
//! against the distribution's `libcurl.so.4`. Ubuntu 26.04 ships curl
//! 8.18.0 (`8.18.0-1ubuntu2.x`). Ubuntu's `curl_8.18.0.orig.tar.gz` (from
//! the `.dsc` of `8.18.0-1ubuntu2.8`) has exactly the SHA-256 of the
//! upstream release tarball pinned here, and its signature file is byte
//! identical too. The 44 `Wcurl_easy_*_err_*` names of the librdkafka
//! census are exactly the warning stubs 8.18.0's `typecheck-gcc.h` defines
//! (8.17.0 and 8.19.0 define the same set, so the stubs corroborate but do
//! not discriminate the version).
//!
//! What is pinned is the **upstream** release. Ubuntu's build of
//! `8.18.0-1ubuntu2.8` applies its own patches (`debian/patches/series`).
//! Four touch files the checker reads: `include/curl/curl.h` (marks
//! `CURLOPT_STREAM_DEPENDS{,_E}` deprecated), `lib/urlapi.c` (appends
//! `Curl_url_same_origin`), and `lib/getinfo.c` and `lib/setopt.c`, which
//! the checker reads only for the evidence of `Unknown` answers. No patched
//! hunk is inside a function a `Safe` claim rests on; every claim names the
//! upstream release as an assumption.

use std::path::Path;

/// The release.
pub const VERSION: &str = "8.18.0";

/// The release tarball's file name, as `curl.se/download/` publishes it.
pub const TARBALL: &str = "curl-8.18.0.tar.gz";

/// The release tarball's SHA-256 (equal to Ubuntu's
/// `curl_8.18.0.orig.tar.gz`).
pub const TARBALL_SHA256: &str = "e9274a5f8ab5271c0e0e6762d2fce194d5f98acc568e4ce816845b2dcc0cf88f";

/// The release's git tag and the commit it names (`git ls-remote`).
pub const TAG: &str = "curl-8_18_0";
pub const TAG_COMMIT: &str = "2eebc58c4b8d68c98c8344381a9f6df4cca838fd";

/// The top-level directory the tarball unpacks to.
pub const SOURCE_DIRECTORY: &str = "curl-8.18.0";

/// Every file the checker reads, with its SHA-256 in the pinned release.
/// The check reads nothing else, and a prepared tree is accepted only when
/// every one of these files has exactly this digest.
pub const FILES: &[(&str, &str)] = &[
    (
        "include/curl/curl.h",
        "697555e3f5829c8fbea3fc3ef783fb6003219f3263c464b6de4019f3b09ea604",
    ),
    (
        "include/curl/easy.h",
        "4b94482a7b8f7f70928cc5113eb532fc7e7db49cab20f2d63a0318eda59f0d79",
    ),
    (
        "include/curl/urlapi.h",
        "4dda5954c042ff23f7561a6767ee6cf8929ddbbca7c8ae1dae15044d1dcfeafb",
    ),
    (
        "include/curl/typecheck-gcc.h",
        "8cc9c4827abc4ca51a73a4ef236b0caf1a3876edc9da14f4d5671e1a61876d74",
    ),
    (
        "lib/curl_setup.h",
        "84bbb5f4915d53ce8759d0962d49c3eed2ca92e5372721e717763068b8cb3d32",
    ),
    (
        "lib/easy.c",
        "2e52bdc0560e1e864fb820ab3eb73a2dc49e12cbf034d348d8af2f11af1c3661",
    ),
    (
        "lib/slist.c",
        "8bdaa480609ef676ddf6fa5628bf46038150730e057471c870cb068759640f6c",
    ),
    (
        "lib/escape.c",
        "be3d650fe9007000c64741c1c2d2e5fda3f61a9f920cfc6792d48bdc8b348b77",
    ),
    (
        "lib/urlapi.c",
        "da7f3127436028b85a8c6d22c9d827a30dd6e35524a2ec157fd02b5464e30eaf",
    ),
    (
        "lib/setopt.c",
        "82c6526a8156d643b561cb7dddf8afa3768b7874791ecbd161f5711ad621dfa4",
    ),
    (
        "lib/getinfo.c",
        "30eac4f54f562e42f379dcb39a8f511c4bf4f496ddbf8c7c3c0bea793159d76a",
    ),
    (
        "lib/curlx/dynbuf.c",
        "274a3e728d5e277e4e08544ebec00b9093ae6e5bef383b298528320280874941",
    ),
];

/// The pinned digest of `path`, if the checker reads it.
#[must_use]
pub fn pinned_digest(path: &str) -> Option<&'static str> {
    FILES
        .iter()
        .find(|(file, _)| *file == path)
        .map(|(_, digest)| *digest)
}

/// One file of a prepared tree that does not match the pin.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mismatch {
    pub file: String,
    pub expected: String,
    /// The digest found, or `None` when the file is missing or unreadable.
    pub found: Option<String>,
}

/// Check every pinned file under `root` (the unpacked `curl-8.18.0`
/// directory). Returns the mismatches; the tree is the pinned release's,
/// for the files the checker reads, exactly when the list is empty.
#[must_use]
pub fn verify_tree(root: &Path) -> Vec<Mismatch> {
    FILES
        .iter()
        .filter_map(|(file, expected)| {
            let found = std::fs::read(root.join(file))
                .ok()
                .map(|bytes| qcore_schemas::sha256_hex(&bytes));
            (found.as_deref() != Some(*expected)).then(|| Mismatch {
                file: (*file).to_owned(),
                expected: (*expected).to_owned(),
                found,
            })
        })
        .collect()
}

/// The mismatches, as one reason.
#[must_use]
pub fn describe(mismatches: &[Mismatch]) -> String {
    mismatches
        .iter()
        .map(|mismatch| {
            format!(
                "{}: expected {}, found {}",
                mismatch.file,
                mismatch.expected,
                mismatch.found.as_deref().unwrap_or("nothing (missing)")
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}
