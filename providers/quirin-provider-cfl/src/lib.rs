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

//! Shared Quirin contract modules for the CFL support library (arena and SDS lifetimes, atomics, hash state, intrusive lists, time).
//!
//! The `.qcore` sources under this crate's `contracts/` are canonical: the
//! corpus index `contract/corpus.json` names that directory as one of its
//! roots, and every consumer selects the modules it needs explicitly. This
//! crate is data only; it links no provider runtime and installs nothing.

/// The crate's contract-inheritance namespace (card CONTRACT-INHERIT-ADOPTION):
/// the package name without `quirin-provider-`, `-` read as `_`. A parent
/// this crate owns is written `cfl::NAME`. Recorded here, checked
/// against the derivation rule by this crate's tests, and carried into
/// `providers/contract-load-path.json`, where the load path checks it again.
pub const NAMESPACE: &str = "cfl";

/// The owned modules another contract module may `#include`, sorted: a
/// subset of [`CONTRACTS`]' logical paths. None yet; publishing a module is
/// a deliberate, reviewed act (card CONTRACT-INHERIT-ADOPTION).
pub const PUBLISHED: &[&str] = &[];

/// Every contract module this crate owns, as `(logical path, source text)`,
/// sorted by logical path. The logical path is relative to `contracts/`, which
/// is the name `contract/corpus.json` resolves (for example `cfl/arena_lifetime.qv`).
pub const CONTRACTS: &[(&str, &str)] = &[
    (
        "cfl/arena_lifetime.qv",
        include_str!("../contracts/cfl/arena_lifetime.qv"),
    ),
    // Card LANDING-REPAIR: quant-contract-hypotheses (6ecb5498d) added this
    // conditional own-event model to the tree and to the load path but not
    // to this index.
    ("cfl/atomic.qv", include_str!("../contracts/cfl/atomic.qv")),
    (
        "cfl/hash_state.qv",
        include_str!("../contracts/cfl/hash_state.qv"),
    ),
    (
        "cfl/intrusive_list.qv",
        include_str!("../contracts/cfl/intrusive_list.qv"),
    ),
    (
        "cfl/null_deref.qv",
        include_str!("../contracts/cfl/null_deref.qv"),
    ),
    (
        "cfl/sds_lifetime.qv",
        include_str!("../contracts/cfl/sds_lifetime.qv"),
    ),
    ("cfl/time.qv", include_str!("../contracts/cfl/time.qv")),
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
