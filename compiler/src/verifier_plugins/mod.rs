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

//! First-party in-process verifier implementations.
//!
//! Each plugin owns its library semantics and manifest. Typed analysis APIs
//! remain available during extraction; their results are not protocol-v1 core
//! certificates. Registry adapters retain the fail-closed v1 behavior until
//! semantic inputs and independently checked witnesses are supported.

pub mod chunkio;
pub mod libevent;
