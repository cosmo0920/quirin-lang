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

//! `qcore.chunkio`: bounded source-derived chunk field summaries.
//!
//! Lifecycle and contract certificate checking remain in the core during this
//! extraction. Source summaries retain their private construction boundary.

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
pub mod pinned_lifecycle;
pub mod teardown_source;
pub mod traversal;
pub mod unmap_wrapper;
pub mod unsupported_close;
pub use chunk_source::*;

pub use quirin_source_analysis::chunkio::PINNED_FLUENT_BIT_REVISION;

// The per-plugin `manifest()`/`provider()` selection route was removed by P09
// (`docs/reports/remaining-p09/README.md`). Use
// `crate::first_party::{load_first_party_manifests, load_first_party_registry}`,
// which reads the same validated profile packages and honours each manifest's
// declared entrypoint instead of forcing a built-in adapter.
