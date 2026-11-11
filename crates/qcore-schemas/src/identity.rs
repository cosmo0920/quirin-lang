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

//! Source identity and pinned-revision records.
//!
//! Moved verbatim from `qcore-compiler`'s `external_source` module (W0 of
//! `docs/GENERALIZATION_PLAN.md`). The pinned per-library path tables and
//! the profile derivation stay in the compiler crate pending W2.
//! Unit/build identity for verification requests lives in
//! [`crate::plugin::UnitIdentity`].

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalSourceBundle {
    pub source_revision: String,
    pub files: BTreeMap<String, String>,
}
