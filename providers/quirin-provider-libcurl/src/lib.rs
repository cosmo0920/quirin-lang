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

//! libcurl resource contracts and their checker plugin (card
//! **LIBCURL-PROVIDER-PLUGIN**; user decision of 2026-11-17, on the template
//! of card OPENSSL-PROVIDER-PLUGIN).
//!
//! A user crate on the user-contract SDK (`quirin-provider-plugin`):
//! `contracts/libcurl/` (flexible resource-v1 declarations, each stating the
//! libcurl versions it holds for, from the official manual pages'
//! AVAILABILITY sections), `contracts.lock` (`quirin-audit pin` on the user
//! load path, with the pinned libcurl version), `library-versions.json`
//! (`curl/curlver.h`), `quirin-plugin.json` (one checker binding per
//! function) and the checker `quirin-libcurl-check` ([`check`]).
//!
//! The contracts are the six card W48-LIBCURL-CONTRACT-PLUGIN established
//! against the pinned libcurl 8.18.0 source. Like the OpenSSL crate, this one
//! is a member of the providers workspace but **not** on
//! `providers/contract-load-path.json` (rule 53): it is loaded only through
//! the generated user load path `providers/user-crates-load-path.json`, behind
//! the plugin firewall, where a checker `Safe` is `Pending`
//! (`plugin-claim-uncertified`) until a trusted certificate checker exists.

pub mod check;

/// The crate's contract-inheritance namespace.
pub const NAMESPACE: &str = "libcurl";

/// The owned modules another contract module may `#include`, sorted. None.
pub const PUBLISHED: &[&str] = &[];

/// Every owned module, logical path to text, sorted: what the checker binds
/// a request's module digest to.
pub const MODULES: &[(&str, &str)] = &[
    (
        "libcurl/escape.qv",
        include_str!("../contracts/libcurl/escape.qv"),
    ),
    (
        "libcurl/global.qv",
        include_str!("../contracts/libcurl/global.qv"),
    ),
    (
        "libcurl/slist.qv",
        include_str!("../contracts/libcurl/slist.qv"),
    ),
    ("libcurl/url.qv", include_str!("../contracts/libcurl/url.qv")),
];

/// The pinned libcurl headers' prototypes (`scripts/pin_prototypes.py`).
pub const PINNED_PROTOTYPES: &str = include_str!("../pinned/libcurl-8.18.0.json");

/// The official manual pages each contract cites, as fetched
/// (`scripts/fetch_docs.py`).
pub const DOC_SNAPSHOTS: &str = include_str!("../doc-snapshots.json");
