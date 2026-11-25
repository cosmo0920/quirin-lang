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

//! Onigmo regex resource contracts and their checker plugin (card
//! **ONIGMO-PROVIDER-PLUGIN**; user decisions of 2026-11-25: "We must write
//! down quirin-provider-onigmo with contracts of onigmo's regex functions."
//! and "This must be depending on quirin-provider-plugin crate and using its
//! plugin mechanism.").
//!
//! A user crate on the user-contract SDK (`quirin-provider-plugin`), laid out
//! as `quirin-provider-libcurl`: `contracts/onigmo/` (resource-v1
//! declarations, pinned, each stating the Onigmo versions it holds for:
//! `documented "6.1.2"`, the series of Onigmo's API reference), `contracts.lock`
//! (`quirin-audit pin` on the user load path, with the vendored Onigmo
//! version), `library-versions.json` (`onigmo.h`, `ONIGMO_VERSION_MAJOR`,
//! `_MINOR`, `_TEENY`), `quirin-plugin.json` (one checker binding per
//! function) and the checker `quirin-onigmo-check` ([`check`]).
//!
//! Fluent Bit vendors Onigmo 6.2.0 at `lib/onigmo`; every clause is read off
//! the vendored body at Fluent Bit `046accf6` or Onigmo's own `doc/API`, and
//! the lock pins each function's sites there. Like the OpenSSL and libcurl
//! crates, this one is a member of the providers workspace but **not** on
//! `providers/contract-load-path.json` (rule 53): it is loaded only through
//! the generated user load path `providers/user-crates-load-path.json`,
//! behind the plugin firewall, where a checker `Safe` is `Pending`
//! (`plugin-claim-uncertified`) until a trusted certificate checker exists.

pub mod check;

/// The crate's contract-inheritance namespace.
pub const NAMESPACE: &str = "onigmo";

/// The owned modules another contract module may `#include`, sorted. None.
pub const PUBLISHED: &[&str] = &[];

/// Every owned module, logical path to text, sorted: what the checker binds
/// a request's module digest to.
pub const MODULES: &[(&str, &str)] = &[
    (
        "onigmo/init.qv",
        include_str!("../contracts/onigmo/init.qv"),
    ),
    (
        "onigmo/names.qv",
        include_str!("../contracts/onigmo/names.qv"),
    ),
    (
        "onigmo/regex.qv",
        include_str!("../contracts/onigmo/regex.qv"),
    ),
    (
        "onigmo/region.qv",
        include_str!("../contracts/onigmo/region.qv"),
    ),
    (
        "onigmo/search.qv",
        include_str!("../contracts/onigmo/search.qv"),
    ),
];

/// The vendored header's prototypes and typedefs
/// (`scripts/pin_prototypes.py`).
pub const PINNED_PROTOTYPES: &str = include_str!("../pinned/onigmo-6.2.0.json");

/// The sections of Onigmo's API reference each contract cites
/// (`scripts/snapshot_docs.py`).
pub const DOC_SNAPSHOTS: &str = include_str!("../doc-snapshots.json");
