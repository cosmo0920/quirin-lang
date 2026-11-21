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

//! OpenSSL resource contracts and their checker plugin (cards
//! **OPENSSL-PROVIDER-PLUGIN** and **OPENSSL-PROVIDER-REMAINING-CALLEES**; user decisions of 2026-09-28 and 2026-11-16).
//!
//! The first in-repository crate built on the user-contract SDK
//! (`quirin-provider-plugin`), laid out as a user crate: `contracts/openssl/`
//! (flexible resource-v1 declarations, each stating the OpenSSL versions it
//! holds for), `contracts.lock` (`quirin-audit pin` on the user load path,
//! with the pinned OpenSSL version), `library-versions.json` (where the
//! version is read), `quirin-plugin.json` (one checker binding per function)
//! and the checker `quirin-openssl-check` ([`check`]).
//!
//! It is a member of the providers workspace, so the provider gate builds,
//! tests and packages it, but it is **not** on `providers/contract-load-path.json`:
//! the shipped load path carries no checker bindings, and an OpenSSL contract
//! applied without its checker would discharge calls nobody checked. It is
//! loaded only through the generated user load path
//! `providers/user-crates-load-path.json`, behind the plugin firewall, where
//! a checker `Safe` is `Pending` (`plugin-claim-uncertified`) until a trusted
//! certificate checker exists.

pub mod check;
pub mod documented;

/// The crate's contract-inheritance namespace.
pub const NAMESPACE: &str = "openssl";

/// The owned modules another contract module may `#include`, sorted. None.
pub const PUBLISHED: &[&str] = &[];

/// Every owned module, logical path to text, sorted: what the checker binds
/// a request's module digest to.
pub const MODULES: &[(&str, &str)] = &[
    (
        "openssl/bio.qv",
        include_str!("../contracts/openssl/bio.qv"),
    ),
    (
        "openssl/bio_slots.qv",
        include_str!("../contracts/openssl/bio_slots.qv"),
    ),
    (
        "openssl/engine.qv",
        include_str!("../contracts/openssl/engine.qv"),
    ),
    (
        "openssl/err.qv",
        include_str!("../contracts/openssl/err.qv"),
    ),
    (
        "openssl/evp.qv",
        include_str!("../contracts/openssl/evp.qv"),
    ),
    (
        "openssl/evp_digest.qv",
        include_str!("../contracts/openssl/evp_digest.qv"),
    ),
    (
        "openssl/init.qv",
        include_str!("../contracts/openssl/init.qv"),
    ),
    (
        "openssl/lookup.qv",
        include_str!("../contracts/openssl/lookup.qv"),
    ),
    (
        "openssl/pem.qv",
        include_str!("../contracts/openssl/pem.qv"),
    ),
    (
        "openssl/pkcs12.qv",
        include_str!("../contracts/openssl/pkcs12.qv"),
    ),
    (
        "openssl/ssl.qv",
        include_str!("../contracts/openssl/ssl.qv"),
    ),
    (
        "openssl/ssl_ctx.qv",
        include_str!("../contracts/openssl/ssl_ctx.qv"),
    ),
    (
        "openssl/ssl_handshake.qv",
        include_str!("../contracts/openssl/ssl_handshake.qv"),
    ),
    (
        "openssl/ssl_handshake_setters.qv",
        include_str!("../contracts/openssl/ssl_handshake_setters.qv"),
    ),
    (
        "openssl/ssl_io.qv",
        include_str!("../contracts/openssl/ssl_io.qv"),
    ),
    (
        "openssl/stack.qv",
        include_str!("../contracts/openssl/stack.qv"),
    ),
    (
        "openssl/teardown_slots.qv",
        include_str!("../contracts/openssl/teardown_slots.qv"),
    ),
    (
        "openssl/x509.qv",
        include_str!("../contracts/openssl/x509.qv"),
    ),
    (
        "openssl/x509_lookup_slots.qv",
        include_str!("../contracts/openssl/x509_lookup_slots.qv"),
    ),
    (
        "openssl/x509_store.qv",
        include_str!("../contracts/openssl/x509_store.qv"),
    ),
    (
        "openssl/x509_store_slots.qv",
        include_str!("../contracts/openssl/x509_store_slots.qv"),
    ),
];

/// The pinned OpenSSL headers' prototypes (`scripts/pin_prototypes.py`).
pub const PINNED_PROTOTYPES: &str = include_str!("../pinned/openssl-3.5.5.json");

/// The official manual pages each contract cites, as fetched
/// (`scripts/fetch_docs.py`).
pub const DOC_SNAPSHOTS: &str = include_str!("../doc-snapshots.json");
