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

//! Complete typed-source matches for the two safe intrusive-list traversals.
//! Finite reciprocal rings, live sentinel/next nodes, container provenance and
//! exact child destructor frames remain premises; calls alone discharge nothing.
use super::PINNED_FLUENT_BIT_REVISION;
use crate::legacy_c::C11TranslationUnit;
use crate::typed_source_match::{SourceFieldProfile, SourceMatch, check_source_match};
use serde::Serialize;

pub const CONTRACT: &str = include_str!(
    "../../../../providers/qcore-provider-chunkio/contracts/chunkio/teardown_traversal.qv"
);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum TraversalKind {
    Chunks,
    Streams,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TraversalSource {
    kind: TraversalKind,
    source: SourceMatch,
    contract_source: String,
}
impl TraversalSource {
    pub fn kind(&self) -> TraversalKind {
        self.kind
    }
    pub fn source(&self) -> &SourceMatch {
        &self.source
    }
}

pub fn check_traversal_source(
    unit: &C11TranslationUnit<'_>,
    revision: &str,
    kind: TraversalKind,
) -> Result<TraversalSource, String> {
    if revision != PINNED_FLUENT_BIT_REVISION {
        return Err("traversal revision mismatch".into());
    }
    let (function, reference) = match kind {
        TraversalKind::Chunks => ("cio_chunk_close_stream", CHUNKS),
        TraversalKind::Streams => ("cio_stream_destroy_all", STREAMS),
    };
    let owner_fields: SourceFieldProfile<'_> = match kind {
        TraversalKind::Chunks => &[
            (
                "mk_list",
                &[("prev", "struct mk_list *"), ("next", "struct mk_list *")],
            ),
            ("cio_stream", &[("chunks", "struct mk_list")]),
            ("cio_chunk", &[("_head", "struct mk_list")]),
        ],
        TraversalKind::Streams => &[
            (
                "mk_list",
                &[("prev", "struct mk_list *"), ("next", "struct mk_list *")],
            ),
            ("cio_ctx", &[("streams", "struct mk_list")]),
            ("cio_stream", &[("_head", "struct mk_list")]),
        ],
    };
    let source = check_source_match(unit, revision, function, reference, owner_fields)?;
    check_policy(CONTRACT)?;
    Ok(TraversalSource {
        kind,
        source,
        contract_source: CONTRACT.into(),
    })
}
fn check_policy(contract: &str) -> Result<(), String> {
    crate::contract_observation::check_contract_facts(
        contract,
        "teardown_traversal",
        &[
            ("finite_exclusive_reciprocal_ring_with_live_sentinel", 1),
            (
                "distinct_live_container_identities_bound_to_primary_links",
                1,
            ),
            (
                "child_destructor_unlinks_only_current_and_preserves_next_and_parent",
                1,
            ),
            ("exact_cross_unit_source_abi_and_call_bindings", 1),
            ("complete_typed_reference_body", 1),
            ("next_saved_before_child_call", 1),
            ("advance_uses_saved_next", 1),
            ("only_current_child_passed", 1),
            ("sentinel_excluded_from_calls", 1),
            ("chunk_delete_argument_zero", 1),
            ("child_custody_discharged_by_source_match", 0),
        ],
    )
}

// Fully expanded references: input macros cannot weaken the specification.
const CHUNKS: &str = r#"
void qcore_source_reference(struct cio_stream *st) {
    struct mk_list *tmp;
    struct mk_list *head;
    struct cio_chunk *ch;
    for (head = (&st->chunks)->next, tmp = head->next; head != (&st->chunks); head = tmp, tmp = head->next) {
        ch = ({ const typeof(((struct cio_chunk *)0)->_head) *__mptr = (head);
                (struct cio_chunk *)((char *)__mptr - __builtin_offsetof(struct cio_chunk, _head)); });
        cio_chunk_close(ch, 0);
    }
}
"#;
const STREAMS: &str = r#"
void qcore_source_reference(struct cio_ctx *ctx) {
    struct mk_list *tmp;
    struct mk_list *head;
    struct cio_stream *st;
    if (!ctx) { return; }
    for (head = (&ctx->streams)->next, tmp = head->next; head != (&ctx->streams); head = tmp, tmp = head->next) {
        st = ({ const typeof(((struct cio_stream *)0)->_head) *__mptr = (head);
                (struct cio_stream *)((char *)__mptr - __builtin_offsetof(struct cio_stream, _head)); });
        cio_stream_destroy(st);
    }
}
"#;

#[cfg(test)]
#[path = "../../tests/unit/teardown_traversal.rs"]
pub(crate) mod tests;
