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

//! Compatibility surface for the process-transported teardown capabilities.
//!
//! The sealed typed imports live in the C gateway and the trusted replays in
//! the verification adapter; this module only re-exports both under one path.
//! The in-process `verifier_plugins::chunkio::teardown_source` adapter is
//! retained and unedited — this is an added route, not a replacement.
pub use qcore_c_gateway::teardown_source::*;
pub use qcore_verification::teardown_source::*;
