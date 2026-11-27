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

//! Card FB-BACKLOG-ACCOUNTING, probe: the base behaviour the ledger checker
//! (gateway rule 92) starts from, pinned before any semantic change.
//!
//! * The custody gateway (G) reads an unsigned ledger update through a member
//!   chain as custody-neutral: a bare, wrapping `L -= d` is `Safe` there,
//!   exactly like the saturating Fluent Bit form. Accounting is not part of
//!   the custody property, so no custody verdict can carry it.
//! * The typed-source route (T) refuses both pinned Fluent Bit ledger helpers
//!   under the legacy node admission: no consumer classifies `compound -=`
//!   or `binary -`.
//!
//! The fixtures reproduce the ledger statements of Fluent Bit `046accf6`
//! (`plugins/in_storage_backlog/sb.c:226-238, 277-287`,
//! `src/flb_input_chunk.c:66-92`); the census is
//! `docs/reports/fb-backlog-accounting/census.json`.

#[path = "support/clang_gap.rs"]
mod clang_gap;

use clang_gap::*;

const RECORDS: &str = "\
struct out_ins { __SIZE_TYPE__ fs_backlog_chunks_size; };
struct sb_q { struct out_ins *ins; };
struct sb_ch { __SIZE_TYPE__ size; };
";

#[test]
fn custody_route_reads_every_ledger_update_as_custody_neutral() {
    if !clang_available() {
        return;
    }
    for body in [
        // sb.c:228-238, the saturating discharge.
        "void subject(struct sb_ch *chunk, struct sb_q *backlog) {\n if (chunk->size > backlog->ins->fs_backlog_chunks_size) {\n backlog->ins->fs_backlog_chunks_size = 0;\n }\n else {\n backlog->ins->fs_backlog_chunks_size -= chunk->size;\n }\n}",
        // sb.c:277-287, the saturating charge.
        "void subject(size_t n, struct sb_q *backlog) {\n if (n > (~(size_t)0) - backlog->ins->fs_backlog_chunks_size) {\n backlog->ins->fs_backlog_chunks_size = (~(size_t)0);\n }\n else {\n backlog->ins->fs_backlog_chunks_size += n;\n }\n}",
        // The wrapping mutation: Safe as well, since custody cannot see it.
        "void subject(struct sb_ch *chunk, struct sb_q *backlog) {\n backlog->ins->fs_backlog_chunks_size -= chunk->size;\n}",
        "void subject(struct sb_ch *chunk, struct sb_q *backlog) {\n backlog->ins->fs_backlog_chunks_size -= chunk->size;\n backlog->ins->fs_backlog_chunks_size -= chunk->size;\n}",
    ] {
        let source = format!("{RECORDS}{body}");
        assert!(g(&source).safe(), "{source}");
    }
}

/// `src/flb_input_chunk.c:66-92` at `046accf6`; `flb_warn` is its
/// `flb_log.h:259-261` expansion over declared functions, `SIZE_MAX` its
/// LP64 value.
pub const HELPERS: &str = "\
typedef __SIZE_TYPE__ size_t;
struct flb_output_instance { int id; size_t fs_chunks_size; };
int flb_log_check(int level);
void flb_log_print(int type, const char *file, int line, const char *fmt, ...);
const char *flb_output_name(struct flb_output_instance *in);
void flb_input_chunk_output_size_subtract(struct flb_output_instance *o_ins,
                                          size_t bytes)
{
    if (bytes > o_ins->fs_chunks_size) {
        if (flb_log_check(2)) flb_log_print(2, 0, 0, \"underflow %s %zu %zu\",
                 flb_output_name(o_ins), o_ins->fs_chunks_size, bytes);
        o_ins->fs_chunks_size = 0;
        return;
    }

    o_ins->fs_chunks_size -= bytes;
}

void input_chunk_output_size_add(struct flb_output_instance *o_ins,
                                 size_t bytes)
{
    if (bytes > (18446744073709551615UL) - o_ins->fs_chunks_size) {
        if (flb_log_check(2)) flb_log_print(2, 0, 0, \"overflow %s %zu %zu\",
                 flb_output_name(o_ins), o_ins->fs_chunks_size, bytes);
        o_ins->fs_chunks_size = (18446744073709551615UL);
        return;
    }

    o_ins->fs_chunks_size += bytes;
}
";

#[test]
fn typed_route_refuses_the_ledger_helpers_under_the_legacy_admission() {
    if !clang_available() {
        return;
    }
    let types: &[&str] = &["struct flb_output_instance *", "unsigned long"];
    for (function, node) in [
        ("flb_input_chunk_output_size_subtract", "compound -="),
        ("input_chunk_output_size_add", "binary -"),
    ] {
        let results = qcore_c_gateway::typed_source::import_source_functions(
            &qcore_c_gateway::legacy_c::C11TranslationUnit {
                source: HELPERS,
                ..Default::default()
            },
            "046accf6",
            &[function],
            &qcore_c_gateway::typed_source::SourceImportProfile {
                records: &["flb_output_instance"],
                sizeof_types: T_SIZEOF,
                signature_constraints: &[(function, "void", types)],
                restricted_calls: &["flb_log_check", "flb_log_print", "flb_output_name"],
            },
        )
        .expect("unit admitted");
        let [qcore_c_gateway::typed_source::SourceResult::Unknown { reason, .. }] =
            results.as_slice()
        else {
            panic!("{function}: {results:?}");
        };
        assert_eq!(
            reason,
            &format!("typed-source node `{node}` is not classified by the legacy typed-source consumer"),
            "{function}"
        );
    }
}
