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

//! Generic library-mechanism vocabulary.
//!
//! Moved verbatim from `qcore-compiler`'s `external_integration` module (W0
//! of `docs/GENERALIZATION_PLAN.md`). The per-library `permits` tables stay
//! in the compiler crate pending their W2 replacement by data profiles.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LibraryMechanismSummary {
    ReferenceCount,
    IntrusiveList,
    TypedQueue,
    Arena,
    CascadingDestructor,
    AtomicPublication,
    ThreadHandoff,
    Reclamation,
}
