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

//! The verifier-host binaries' path to the shared fixture writer.
//!
//! The one implementation lives in the repository's
//! `tests/support/process_fixture.rs` (card
//! `PROCESS-FIXTURES-FS-WRITE-OUTSIDE-HOST-CRATE` moved it there from this
//! file, where card `EXECUTION-QUOTA-LOAD-INDEPENDENCE` wrote it, so every
//! crate that writes an executable fixture uses the same writer). This file
//! keeps the `#[path = "support/process_fixture.rs"]` includes of this
//! directory resolving, and re-exports everything they use.

#[path = "../../../../tests/support/process_fixture.rs"]
mod shared;
#[allow(unused_imports)]
pub use shared::{READ_REQUEST, copy_executable, write_executable, write_executable_mode};
