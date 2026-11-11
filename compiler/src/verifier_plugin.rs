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

//! Compatibility facade for the extracted verifier host.
//!
//! Generic registry, transport, firewall and composition live in
//! `qcore-verifier-host`; configured first-party adapters live in `first_party`.

pub use crate::first_party::{
    first_party_manifests, first_party_registry, load_first_party_manifests,
    load_first_party_registry,
};
pub use qcore_verifier_host::*;
