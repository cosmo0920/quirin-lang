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

//! Compatibility surface for the process-transported composition capability.
//!
//! The sealed typed import lives in the C gateway, the relocated composition
//! decision procedure and the trusted task in the verification adapter; this
//! module only re-exports all three under one path. The in-process
//! `verifier_plugins::chunkio::unmap_wrapper` adapter is retained (hard
//! invariant 3) and now delegates to the same relocated procedure, so the two
//! routes cannot drift.
pub use qcore_c_gateway::composition_source::*;
pub use qcore_verification::composition_source::*;
pub use qcore_verification::unmap_composition::{
    UnmapCase, UnmapCertificate, UnmapCompositionContracts, check_unmap_composition,
};
