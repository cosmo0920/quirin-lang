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

//! `qcore.libevent`: bounded event semantics, profiles, and observation refinement.
//!
//! # Diagnostic namespace (W3/W7 renumbering)
//!
//! Spec §54 previously listed these Libevent conditions as core `Q700`–`Q712`,
//! colliding with Cycle 20's `Q700`/`Q701` verify-scope codes. Provider
//! manifests own library-specific diagnostic namespaces, so the table moved
//! here as `qcore.libevent.L00`–`L12` with its order and one-line meanings
//! preserved. `Q700`–`Q719` are now reserved for core `verify scope`
//! resolution and dispatch. Archived Cycle 18 reports keep the old
//! identifiers; this table supersedes them.
//!
//! | Old | New | Meaning |
//! |---|---|---|
//! | `Q700` | `qcore.libevent.L00` | event/base lifecycle transition is invalid or stale |
//! | `Q701` | `qcore.libevent.L01` | event, base, bufferevent, or callback argument used after free |
//! | `Q702` | `qcore.libevent.L02` | callback argument outlives its ownership or borrow contract |
//! | `Q703` | `qcore.libevent.L03` | file-descriptor or close-on-free custody is duplicated |
//! | `Q704` | `qcore.libevent.L04` | cross-thread Libevent operation lacks supported thread initialization |
//! | `Q705` | `qcore.libevent.L05` | callback access conflicts across bases, callers, or worker tasks |
//! | `Q706` | `qcore.libevent.L06` | deletion, finalization, or teardown lacks callback quiescence |
//! | `Q707` | `qcore.libevent.L07` | persistent event or deferred callback leaves an owned obligation |
//! | `Q708` | `qcore.libevent.L08` | priority, readiness, or timing is used as synchronization evidence |
//! | `Q709` | `qcore.libevent.L09` | callback target, Libevent API, backend, or build profile is unknown |
//! | `Q710` | `qcore.libevent.L10` | bufferevent callback mode or lock contract is violated |
//! | `Q711` | `qcore.libevent.L11` | loop exit leaves unaccounted event or callback custody |
//! | `Q712` | `qcore.libevent.L12` | observed Libevent trace does not refine the checked event model |

pub mod callback_task;
pub mod event_driven;
pub mod monkey_binding;
pub mod monkey_observation;
pub mod monkey_source;
pub use event_driven::*;

// The per-plugin `manifest()`/`provider()` selection route was removed by P09
// (`docs/reports/remaining-p09/README.md`). Use
// `crate::first_party::{load_first_party_manifests, load_first_party_registry}`,
// which reads the same validated profile packages and honours each manifest's
// declared entrypoint instead of forcing a built-in adapter.
