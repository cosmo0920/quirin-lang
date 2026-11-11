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

//! The SDK for **user-defined contracts and checker plugins** (card
//! W45-USER-CONTRACT-PLUGINS; contract specification §C11.1).
//!
//! A user writes `.qv` resource contracts in their own crate, outside the
//! Quirin repository, laid out like the `quirin-provider-*` crates, and may
//! attach an out-of-process **checker plugin** to a module's contracts. This
//! crate is the user's side of that arrangement. It carries **no production
//! contracts** and links no core crate:
//!
//! * [`user_crate`] — the crate layout and its `quirin-plugin.json` (checker
//!   bindings and explicit overrides of shipped contracts), read and
//!   validated total-or-refuse;
//! * [`load_path`] — the user load path: a load-path manifest that extends
//!   the shipped `providers/contract-load-path.json` with the user's crates;
//! * [`template`] — a new crate from the template (`quirin-plugin new`);
//! * [`package`] — a checker package from an executable built with
//!   [`checker`], and its verification (the Rust counterpart, for checker
//!   packages, of `providers/package.py build`/`verify`);
//! * [`checker`] — the runtime a checker binary is written against: the
//!   shared provider transport, answering evidence-v2 requests.
//!
//! What stays in the core: loading, selection, collisions with shipped
//! contracts, locks (`quirin-audit pin`), the resource view
//! (`quirin-audit plugin views --write`), running each checker as a verified
//! process package and the **firewall** that classifies its answer. A
//! plugin's `Safe` counts only with a certificate the audit checks itself,
//! otherwise it is `Pending`; its `Unsafe` counts only with a witness the audit
//! validates, otherwise it is `Pending`; a crash, timeout, quota breach,
//! protocol error or mismatch is `Unknown`. Nothing this crate does can
//! change that: the SDK builds requests' answers, the core decides.

pub mod checker;
pub mod load_path;
pub mod package;
pub mod template;
pub mod user_crate;

/// The package-name prefix every contract crate on a load path carries,
/// user crates included (`qcore_contract::contract_inclusion`).
pub const CONTRACT_CRATE_PREFIX: &str = "quirin-provider-";

/// The per-crate SDK manifest, beside the crate's `Cargo.toml`.
pub const PLUGIN_MANIFEST: &str = "quirin-plugin.json";

/// The only `quirin-plugin.json` schema this SDK reads and writes.
pub const PLUGIN_MANIFEST_SCHEMA: u32 = 1;

/// The load-path manifest schema the core reads (`LOAD_PATH_MANIFEST_SCHEMA`).
pub const LOAD_PATH_SCHEMA: u32 = 1;

/// The namespace a contract crate's package name derives: the name without
/// `quirin-provider-`, `-` read as `_`, identifier-shaped (the core's
/// `derive_contract_namespace`, restricted to the data-crate prefix).
#[must_use]
pub fn derive_namespace(name: &str) -> Option<String> {
    let namespace = name.strip_prefix(CONTRACT_CRATE_PREFIX)?.replace('-', "_");
    let mut chars = namespace.chars();
    let first = chars.next()?;
    ((first.is_ascii_lowercase() || first == '_')
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'))
    .then_some(namespace)
}

/// SHA-256 of `bytes`, lowercase hexadecimal (the schema crate's digest).
#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    qcore_schemas::sha256_hex(bytes)
}

/// Whether `value` is a relative path of normal components only.
#[must_use]
pub fn relative_without_traversal(value: &str) -> bool {
    !value.is_empty()
        && !value.contains('\\')
        && std::path::Path::new(value)
            .components()
            .all(|component| matches!(component, std::path::Component::Normal(_)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn namespaces_derive_like_the_load_path() {
        assert_eq!(
            derive_namespace("quirin-provider-acme").as_deref(),
            Some("acme")
        );
        assert_eq!(
            derive_namespace("quirin-provider-acme-pool").as_deref(),
            Some("acme_pool")
        );
        assert_eq!(derive_namespace("acme"), None);
        assert_eq!(derive_namespace("quirin-provider-"), None);
        assert_eq!(derive_namespace("quirin-provider-9x"), None);
        assert!(relative_without_traversal("checkers/pool"));
        assert!(!relative_without_traversal("../pool"));
        assert!(!relative_without_traversal("/abs"));
    }
}
