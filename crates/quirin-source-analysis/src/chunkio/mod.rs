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

//! Source adapters for the pinned ChunkIO audit.
pub mod chunk_source;
pub mod file_attempts;
pub mod file_close;
pub mod file_delete;
pub mod file_outcomes;
pub mod filesystem_chunk_close;
pub mod lifecycle_coverage;
pub mod memory_close;
pub mod nested_cleanup;
pub mod nested_memory;
pub mod parent_teardown;
pub mod teardown_source;
pub mod traversal;
pub mod unmap_wrapper;
pub mod unsupported_close;
pub use chunk_source::*;
pub const PINNED_FLUENT_BIT_REVISION: &str = "66910c10a4d7eafa810b84229cf2dcf7b0f26f97";
