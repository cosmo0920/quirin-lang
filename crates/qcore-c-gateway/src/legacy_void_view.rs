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

//! Card **CG12-VOID-VIEW-READBACK** (gateway rule 11j): the route-G side of
//! the readback of a `void *` member. The decisions are
//! [`crate::void_view::decide_readback`] and [`crate::void_view::flow_binds`];
//! this adapter reads their facts off the parser (static premises 1–4) and off
//! one [`super::AbstractState`] (premise 5, `bound_read_resolves`).
//!
//! Parity only, every decision is recorded (`position: "void-view-readback"`
//! for the static decision, `"void-view-readback-flow"` for the flow's).

use serde_json::json;

use crate::typed_access_path::{parity_enabled, record_row};
use crate::void_view::{Readback, ReadbackDecision, body_facts, decide_readback, flow_binds};

/// The refusal of a readback whose view differs from a known stored type or
/// from another view of the member (type punning, M22).
pub(super) const PUN_REFUSAL: &str = "void * member read back as a type other than the one stored (type punning) is outside \
     the resource projection";

/// The static readback decision for `candidate` at the statement `tokens`.
pub(super) fn decide(
    parser: &super::Parser<'_>,
    tokens: &[super::Token],
    candidate: &Readback,
) -> ReadbackDecision {
    // 1. The owner is an admitted store base and the member its `void *`.
    let store_base = crate::aggregate_store::admitted_field(
        &candidate.root,
        &candidate.member,
        &parser.store_bases,
        &parser.typed_model,
        &parser.typed_types,
    )
    .is_some_and(|store| store.opaque_pointer);
    // 2–4 over the function's AST (read whenever the typed plan was).
    let function = parser
        .typed_path_index
        .get()
        .and_then(Option::as_ref)
        .map(|index| std::sync::Arc::clone(&index.function));
    let decision = match function {
        None => ReadbackDecision::NoStoreBase,
        Some(function) if store_base => {
            let facts = body_facts(
                &function,
                &candidate.root_decl,
                &candidate.field,
                parser.contracts,
            );
            decide_readback(
                true,
                facts.types_agree(),
                facts.owner_escapes,
                facts.member_escapes,
            )
        }
        Some(_) => decide_readback(false, true, false, false),
    };
    if parity_enabled() {
        let texts = tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        record_row(json!({
            "function": parser.typed_path_function,
            "line": tokens.first().map_or(0, |token| token.line),
            "texts": texts,
            "position": "void-view-readback",
            "place": candidate.place(),
            "decision": decision.code(),
            "old_route": "old_refusal",
            "old_admitted": false,
            "new_admitted": decision == ReadbackDecision::Identity,
            "disagreement": decision != ReadbackDecision::NoStoreBase,
        }));
    }
    decision
}

/// Premise 5 at the node: the binding the destination takes, the member's
/// own, when `bound_read_resolves` holds on `state`; `None` otherwise.
pub(super) fn readback_binding(
    function: &str,
    line: usize,
    state: &super::AbstractState,
    place: &str,
    context: &super::TransferContext<'_>,
) -> Option<super::AbstractPointer> {
    let resolves = super::bound_read_resolves(state, place, &context.caller_sites);
    let binds = flow_binds(ReadbackDecision::Identity, resolves);
    if parity_enabled() {
        record_row(json!({
            "function": function,
            "line": line,
            "position": "void-view-readback-flow",
            "place": place,
            "resolves": resolves,
            "old_route": "old_refusal",
            "old_admitted": false,
            "new_admitted": binds,
            "disagreement": binds,
        }));
    }
    binds.then(|| state.pointers.get(place).cloned()).flatten()
}
