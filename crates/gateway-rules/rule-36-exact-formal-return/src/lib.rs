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

//! Gateway rule 36 (`docs/agents/rules/36.md`): **an exact borrowed-formal
//! return preserves identity and custody.** Card RULE36-PILOT moved its body
//! here from `qcore-c-gateway` (plan `docs/plans/pluggable-cards/README.md`,
//! seam B); the rule is unchanged.
//!
//! The package has four parts, each a reading the host asks only while the
//! rule's switch is on (registry id [`ID`], `QCORE_GATEWAY_RULES_OFF`):
//!
//! * [`source::candidate`], the lexical pre-filter: a body with a bare
//!   `return P;` of a formal the declaration reads as a matching depth-one
//!   object pointer ([`source::pointer_formals`]) is worth a derived-result
//!   CFG, and its neutral `None` summary is worth refining.
//! * [`source::exact_formal_returns`], the source conjunction: the typed
//!   reader confirms the formal's index, declaration identity, original
//!   reference, complete spans and matching lexical/AST occurrences, and that
//!   the body never writes or takes the address of the formal; a missing fact
//!   declines.
//! * [`route::ExactFormalReturnRoute`], the first `ReturnRoute` package
//!   (route map §4 step 2, asked first in `return_statement`): in the
//!   derived-result CFG only, a confirmed `return P;` lowers to the host's
//!   exact-formal instruction.
//! * [`flow::classify`], the exit classifier the host's transfer and exit
//!   observation read: only definitely NULL, or the one status-free original
//!   formal site with the matching index and no unresolved effect, is known;
//!   everything else refuses.
//!
//! Off, the host asks none of them: the candidate is false (no derived CFG is
//! built for this rule and no neutral summary is refined), the route falls
//! through to the existing return lowering, and the classifier refuses. Off
//! only removes admissions.
//!
//! The Lean model is `QCore.ExactFormalReturn` (source conjunction,
//! classifier, transfer, shared flow vectors [`flow::FLOW_VECTORS`]) and
//! `QCore.ExactFormalReturnSwitch` (off never widens). Faithful Clang
//! extraction and the Rust–Lean correspondence remain premise 28.

pub mod flow;
pub mod route;
pub mod source;

pub use flow::{Binding, Flow, classify};
pub use route::{ExactFormalHost, ExactFormalReturnRoute};
pub use source::{FormalReturnHost, FunctionReading, SourceHost};

/// The registry id this package answers under.
pub const ID: &str = "36";
