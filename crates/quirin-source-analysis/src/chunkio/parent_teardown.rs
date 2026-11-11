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

//! Stream/context allocation release after nested child traversal. Source-body
//! equality is conditional on separate child effects and allocation provenance.
use super::PINNED_FLUENT_BIT_REVISION;
use crate::legacy_c::C11TranslationUnit;
use crate::typed_source_match::{SourceFieldProfile, SourceMatch, check_source_match};
use serde::Serialize;

pub const CONTRACT: &str = include_str!(
    "../../../../providers/qcore-provider-chunkio/contracts/chunkio/parent_teardown.qv"
);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum ParentKind {
    Stream,
    Context,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ParentSource {
    kind: ParentKind,
    source: SourceMatch,
    contract_source: String,
}
impl ParentSource {
    pub fn kind(&self) -> ParentKind {
        self.kind
    }
    pub fn source(&self) -> &SourceMatch {
        &self.source
    }
}

pub fn check_parent_source(
    unit: &C11TranslationUnit<'_>,
    revision: &str,
    kind: ParentKind,
) -> Result<ParentSource, String> {
    if revision != PINNED_FLUENT_BIT_REVISION {
        return Err("parent teardown revision mismatch".into());
    }
    let (function, reference) = match kind {
        ParentKind::Stream => ("cio_stream_destroy", STREAM),
        ParentKind::Context => ("cio_destroy", CONTEXT),
    };
    let fields: SourceFieldProfile<'_> = match kind {
        ParentKind::Stream => &[(
            "cio_stream",
            &[("_head", "struct mk_list"), ("name", "char *")],
        )],
        ParentKind::Context => &[
            (
                "cio_ctx",
                &[
                    ("options", "struct cio_options"),
                    ("processed_user", "void *"),
                    ("processed_group", "void *"),
                ],
            ),
            (
                "cio_options",
                &[
                    ("user", "char *"),
                    ("group", "char *"),
                    ("chmod", "char *"),
                    ("root_path", "char *"),
                ],
            ),
        ],
    };
    let source = check_source_match(unit, revision, function, reference, fields)?;
    check_policy(CONTRACT)?;
    Ok(ParentSource {
        kind,
        source,
        contract_source: CONTRACT.into(),
    })
}

fn check_policy(contract: &str) -> Result<(), String> {
    crate::contract_observation::check_contract_facts(
        contract,
        "parent_teardown",
        &[
            (
                "nullable_owned_parent_and_distinct_nullable_owned_fields",
                1,
            ),
            ("exclusive_live_parent_storage_until_final_free", 1),
            (
                "child_traversal_frames_parent_fields_and_unrelated_storage",
                1,
            ),
            ("stream_membership_has_live_reciprocal_neighbors", 1),
            ("exact_child_unlink_source_abi_and_libc_free_bindings", 1),
            ("complete_typed_reference_body", 1),
            ("null_parent_has_no_effect", 1),
            ("child_traversal_precedes_parent_release", 1),
            ("owned_field_releases_precede_parent_release", 1),
            ("nonnull_parent_released_exactly_once", 1),
            ("nested_discharge_established_by_parent_free", 0),
        ],
    )
}

const STREAM: &str = r#"
void qcore_source_reference(struct cio_stream *st) {
    if (!st) { return; }
    cio_chunk_close_stream(st);
    mk_list_del(&st->_head);
    free(st->name);
    free(st);
}
"#;
const CONTEXT: &str = r#"
void qcore_source_reference(struct cio_ctx *ctx) {
    if (!ctx) { return; }
    cio_stream_destroy_all(ctx);
    if (ctx->options.user != ((void *)0)) { free(ctx->options.user); }
    if (ctx->options.group != ((void *)0)) { free(ctx->options.group); }
    if (ctx->options.chmod != ((void *)0)) { free(ctx->options.chmod); }
    if (ctx->processed_user != ((void *)0)) { free(ctx->processed_user); }
    if (ctx->processed_group != ((void *)0)) { free(ctx->processed_group); }
    if (ctx->options.root_path != ((void *)0)) { free(ctx->options.root_path); }
    free(ctx);
}
"#;

#[cfg(test)]
#[path = "../../tests/unit/parent_teardown.rs"]
pub(crate) mod tests;
