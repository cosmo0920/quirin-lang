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

//! Shared Quirin contract modules for the C standard library and POSIX: `errno`, heap
//! memory, `pthread_mutex`, (card W43-DESCRIPTOR-RESOURCE-TRACKING) `<dirent.h>`, and
//! (card W43-LIBC-CONTRACTS) `<dlfcn.h>`, `<math.h>`,
//! process/regex/rlimit helpers, sockets, `<stdio.h>`, `<stdlib.h>`, `<string.h>`,
//! `<sys/stat.h>`, threads and signals, `<time.h>` and `<unistd.h>`, and (card
//! W43-LIBC-RESTRICT-PROTOTYPES) `<regex.h>`, and (card
//! CELL-GUARD-RESULT-RANGE-CHECKED) the documented result sets of `pipe`,
//! `epoll_ctl` and `open` (`libc/result_sets.qv`, bounds
//! contract v1 value sets, flexible, no custody).
//!
//! The `.qcore` sources under this crate's `contracts/` are canonical: the
//! corpus index `contract/corpus.json` names that directory as one of its
//! roots, and every consumer selects the modules it needs explicitly. This
//! crate is data only; it links no provider runtime and installs nothing.
//!
//! This is the only home for libc contracts: a new libc module goes under
//! `contracts/libc/` here, with its entry in [`CONTRACTS`], and never under
//! `fluent_bit/` or another corpus module. The crate has no dependency and is
//! usable on its own, without the `quirin-provider-shared` bundle.

/// The crate's contract-inheritance namespace (card CONTRACT-INHERIT-ADOPTION):
/// the package name without `quirin-provider-`, `-` read as `_`. A parent
/// this crate owns is written `libc::NAME`. Recorded here, checked
/// against the derivation rule by this crate's tests, and carried into
/// `providers/contract-load-path.json`, where the load path checks it again.
pub const NAMESPACE: &str = "libc";

/// The owned modules another contract module may `#include`, sorted: a
/// subset of [`CONTRACTS`]' logical paths. `libc/memory.qv` (`malloc`,
/// `calloc`, `free`) and `libc/string.qv` (`strdup`, `strndup`) are
/// published: `quirin-provider-fluent-bit` inherits its allocator and
/// string-duplication wrappers from them. Publication makes a module's text
/// part of every inheriting contract's identity, so a published module's
/// SHA-256 is pinned in `providers/contract-load-path.json`.
pub const PUBLISHED: &[&str] = &["libc/memory.qv", "libc/string.qv"];

/// Every contract module this crate owns, as `(logical path, source text)`,
/// sorted by logical path. The logical path is relative to `contracts/`, which
/// is the name `contract/corpus.json` resolves (for example `libc/errno.qv`).
pub const CONTRACTS: &[(&str, &str)] = &[
    (
        "libc/dirent.qv",
        include_str!("../contracts/libc/dirent.qv"),
    ),
    ("libc/dlfcn.qv", include_str!("../contracts/libc/dlfcn.qv")),
    ("libc/errno.qv", include_str!("../contracts/libc/errno.qv")),
    ("libc/math.qv", include_str!("../contracts/libc/math.qv")),
    (
        "libc/memory.qv",
        include_str!("../contracts/libc/memory.qv"),
    ),
    (
        "libc/process.qv",
        include_str!("../contracts/libc/process.qv"),
    ),
    (
        "libc/pthread_mutex.qv",
        include_str!("../contracts/libc/pthread_mutex.qv"),
    ),
    (
        "libc/pthread_sync.qv",
        include_str!("../contracts/libc/pthread_sync.qv"),
    ),
    ("libc/regex.qv", include_str!("../contracts/libc/regex.qv")),
    (
        "libc/result_sets.qv",
        include_str!("../contracts/libc/result_sets.qv"),
    ),
    (
        "libc/socket.qv",
        include_str!("../contracts/libc/socket.qv"),
    ),
    ("libc/stdio.qv", include_str!("../contracts/libc/stdio.qv")),
    (
        "libc/stdlib.qv",
        include_str!("../contracts/libc/stdlib.qv"),
    ),
    (
        "libc/string.qv",
        include_str!("../contracts/libc/string.qv"),
    ),
    (
        "libc/sys_stat.qv",
        include_str!("../contracts/libc/sys_stat.qv"),
    ),
    (
        "libc/threads.qv",
        include_str!("../contracts/libc/threads.qv"),
    ),
    ("libc/time.qv", include_str!("../contracts/libc/time.qv")),
    (
        "libc/unistd.qv",
        include_str!("../contracts/libc/unistd.qv"),
    ),
];

/// The source text of one owned module, by logical path.
pub fn contract(path: &str) -> Option<&'static str> {
    CONTRACTS
        .iter()
        .find(|(name, _)| *name == path)
        .map(|(_, text)| *text)
}

#[cfg(test)]
#[path = "../../../tests/support/tracked_files.rs"]
mod tracked_files;

#[cfg(test)]
mod tests {
    use super::*;

    /// Card CONTRACT-INHERIT-ADOPTION: the recorded namespace is the one the
    /// load path derives from the package name, and every published module
    /// is an owned one, listed once, in order.
    #[test]
    fn namespace_is_derived_and_published_modules_are_owned() {
        let derived = env!("CARGO_PKG_NAME")
            .strip_prefix("quirin-provider-")
            .expect("a contract crate is named quirin-provider-NAME")
            .replace('-', "_");
        assert_eq!(NAMESPACE, derived);
        assert!(PUBLISHED.windows(2).all(|pair| pair[0] < pair[1]));
        for module in PUBLISHED {
            assert!(CONTRACTS.iter().any(|(name, _)| name == module), "{module}");
        }
    }

    /// The index is exactly the `.qcore` tree on disk, byte for byte: a module
    /// added under `contracts/` without an index entry, or an entry without its
    /// file, fails here rather than being silently omitted.
    #[test]
    fn index_is_exactly_the_contract_tree() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("contracts");
        let mut on_disk = Vec::new();
        // Card W45-TEST-HYGIENE: the tracked contract tree only; an
        // untracked or ignored file under `contracts/` is not a module.
        for path in super::tracked_files::tracked_files(&root) {
            assert!(!path.is_symlink(), "symbolic link: {}", path.display());
            let name = path.strip_prefix(&root).unwrap().to_str().unwrap();
            on_disk.push((
                name.replace('\\', "/"),
                std::fs::read_to_string(&path).unwrap(),
            ));
        }
        on_disk.sort();
        let indexed: Vec<_> = CONTRACTS
            .iter()
            .map(|(name, text)| (name.to_string(), text.to_string()))
            .collect();
        assert_eq!(indexed, on_disk);
        assert_eq!(contract(CONTRACTS[0].0), Some(CONTRACTS[0].1));
        assert_eq!(contract("absent.qcore"), None);
    }
}
