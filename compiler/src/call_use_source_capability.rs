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

//! Compatibility surface for the process-transported call-result use capability.
//!
//! The sealed typed import lives in the C gateway and the trusted replay in the
//! verification adapter; this module only re-exports both under one path,
//! together with the shared decision procedure. The in-process
//! `verifier_plugins::chunkio::file_attempts` adapter is retained — it now
//! delegates to the same decision procedure, so the two routes cannot drift.
pub use qcore_c_gateway::call_use_source::*;
pub use qcore_verification::call_use_contract::*;
pub use qcore_verification::call_use_source::*;
