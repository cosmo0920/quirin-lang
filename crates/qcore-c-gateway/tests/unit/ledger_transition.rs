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

//! Card FB-BACKLOG-ACCOUNTING (gateway rule 92). The first two tests are the
//! probe: the base behaviour the ledger checker starts from, pinned before
//! any semantic change.
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

use super::*;
use crate::legacy_c::{
    AnalysisStatus, C11TranslationUnit, UnrestrictedLegacyResult, analyze_unrestricted_c11,
};
use crate::test_support::clang_available;
use crate::typed_source::{SourceImportProfile, SourceResult, SourceSummary};

/// The scalar `sizeof` operand types route T is given here.
const T_SIZEOF: &[&str] = &["int", "unsigned long"];

/// The custody gateway's status for function `subject`, with no contracts.
fn custody_status(source: &str) -> AnalysisStatus {
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source,
            ..Default::default()
        },
        "subject",
        &Default::default(),
    )
    .unwrap_or_else(|error| panic!("fixture rejected by Clang: {error:?}\n{source}"));
    match outcome {
        UnrestrictedLegacyResult::Safe(_) => AnalysisStatus::Safe,
        UnrestrictedLegacyResult::Unsafe(_) => AnalysisStatus::Unsafe,
        UnrestrictedLegacyResult::Unknown { .. } => AnalysisStatus::Unknown,
    }
}

/// Clang's spelling of `__SIZE_TYPE__` on the test's target (LLP64 on
/// Windows), so the ledger type and the signature constraints read the same
/// fixture on both; `18446744073709551615UL` has this type on both.
const SIZE_T: &str = if cfg!(windows) {
    "unsigned long long"
} else {
    "unsigned long"
};

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
        let source = format!("typedef __SIZE_TYPE__ size_t;\n{RECORDS}{body}");
        assert_eq!(custody_status(&source), AnalysisStatus::Safe, "{source}");
    }
}

/// `src/flb_input_chunk.c:66-92` at `046accf6`; `flb_warn` is its
/// `flb_log.h:259-261` expansion over declared functions, `SIZE_MAX` its
/// LP64 value.
const HELPERS: &str = "\
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
    let types: &[&str] = &["struct flb_output_instance *", SIZE_T];
    for (function, node) in [
        ("flb_input_chunk_output_size_subtract", "compound -="),
        ("input_chunk_output_size_add", "binary -"),
    ] {
        let results = crate::typed_source::import_source_functions(
            &C11TranslationUnit {
                source: HELPERS,
                ..Default::default()
            },
            "046accf6",
            &[function],
            &SourceImportProfile {
                records: &["flb_output_instance"],
                sizeof_types: T_SIZEOF,
                signature_constraints: &[(function, "void", types)],
                restricted_calls: &["flb_log_check", "flb_log_print", "flb_output_name"],
            },
        )
        .expect("unit admitted");
        let [SourceResult::Unknown { reason, .. }] = results.as_slice() else {
            panic!("{function}: {results:?}");
        };
        assert_eq!(
            reason,
            &format!(
                "typed-source node `{node}` is not classified by the legacy typed-source consumer"
            ),
            "{function}"
        );
    }
}

// ---------------------------------------------------------------------------
// Rule 92: the ledger checker over the same helpers, the storage backlog's
// ledger statements, and the mutation controls.
// ---------------------------------------------------------------------------

use crate::typed_source::import_source_functions_admitting;
use LedgerTransition::*;

const SIZE_MAX: &str = "18446744073709551615";

fn spec(field: &'static str, pairing: Option<LedgerPairing<'static>>) -> LedgerSpec<'static> {
    LedgerSpec {
        field,
        ledger_type: SIZE_T,
        maximum: SIZE_MAX,
        pairing,
    }
}

const LIST: Option<LedgerPairing<'static>> = Some(LedgerPairing {
    link: "mk_list_add",
    unlink: "mk_list_del",
});

/// Import `function` from `source` under the ledger admission.
fn import(
    source: &str,
    function: &str,
    result: &str,
    types: &[&str],
    records: &[&str],
) -> Result<SourceSummary, String> {
    let calls: &[&str] = &[
        "flb_log_check",
        "flb_log_print",
        "flb_output_name",
        "flb_errno_print",
        "sb_allocate_chunk",
        "sb_destroy_chunk",
        "mk_list_add",
        "mk_list_del",
        "hook_fails",
    ];
    let results = import_source_functions_admitting(
        &C11TranslationUnit {
            source,
            ..Default::default()
        },
        "046accf6",
        &[function],
        &SourceImportProfile {
            records,
            sizeof_types: T_SIZEOF,
            signature_constraints: &[(function, result, types)],
            restricted_calls: calls,
        },
        &LEDGER_ADMISSION,
    )
    .expect("unit admitted");
    match results.into_iter().next() {
        Some(SourceResult::Summary(summary)) => Ok(summary),
        Some(SourceResult::Unknown { reason, .. }) => Err(reason),
        None => panic!("no result"),
    }
}

fn transitions(
    summary: &SourceSummary,
    spec: &LedgerSpec<'_>,
) -> Result<Vec<LedgerTransition>, String> {
    check_ledger_transitions(summary, spec).map(|reading| {
        reading
            .sites
            .into_iter()
            .map(|site| site.transition)
            .collect()
    })
}

/// The helpers with `flb_output_name(o_ins)` in the log's argument list
/// replaced by a literal: T refuses the pinned argument list (an opaque call
/// beside reads, unsequenced in C) before rule 92 reads anything; see
/// [`the_pinned_log_argument_list_is_refused_before_the_ledger`].
fn logged(source: &str) -> String {
    source
        .replace("flb_output_name(o_ins)", "\"out\"")
        .replace("flb_output_name(backlog->ins)", "\"out\"")
}

fn helper(function: &str) -> SourceSummary {
    import(
        &logged(HELPERS),
        function,
        "void",
        &["struct flb_output_instance *", SIZE_T],
        &["flb_output_instance"],
    )
    .unwrap_or_else(|reason| panic!("{function}: {reason}"))
}

#[test]
fn the_pinned_fs_chunks_size_helpers_are_saturating_transitions() {
    if !clang_available() {
        return;
    }
    let fs = spec("fs_chunks_size", None);
    assert_eq!(
        transitions(&helper("flb_input_chunk_output_size_subtract"), &fs),
        // The clamp arm forks again on `flb_log_check`.
        Ok(vec![UnderflowClamp, UnderflowClamp, Discharge])
    );
    assert_eq!(
        transitions(&helper("input_chunk_output_size_add"), &fs),
        Ok(vec![OverflowClamp, OverflowClamp, Charge])
    );
    // Another field is not this ledger: nothing to certify, nothing refused.
    assert_eq!(
        transitions(
            &helper("input_chunk_output_size_add"),
            &spec("fs_backlog_chunks_size", None)
        ),
        Ok(vec![])
    );
}

/// The storage backlog's records and declarations at `046accf6`
/// (`sb.c:37-60`, `flb_output.h:517-533`), cut to the fields the ledger
/// statements touch; `mk_list_add`/`mk_list_del` are declared functions here
/// (static inline in `mk_list.h`).
const SB: &str = "\
typedef __SIZE_TYPE__ size_t;
struct mk_list { struct mk_list *prev; struct mk_list *next; };
struct cio_chunk;
struct cio_stream;
struct flb_output_instance { int id; size_t fs_backlog_chunks_size; };
struct sb_out_queue { struct flb_output_instance *ins; struct mk_list chunks; struct mk_list _head; };
struct sb_out_chunk { struct mk_list _head; struct cio_chunk *chunk; struct cio_stream *stream; size_t size; };
int flb_log_check(int level);
void flb_log_print(int type, const char *file, int line, const char *fmt, ...);
const char *flb_output_name(struct flb_output_instance *in);
void flb_errno_print(int errnum, const char *file, int line);
struct sb_out_chunk *sb_allocate_chunk(struct cio_chunk *chunk, struct cio_stream *stream, size_t size);
void sb_destroy_chunk(struct sb_out_chunk *chunk);
void mk_list_add(struct mk_list *_new, struct mk_list *head);
void mk_list_del(struct mk_list *entry);
int hook_fails(void);
";

const SB_RECORDS: &[&str] = &[
    "flb_output_instance",
    "sb_out_queue",
    "sb_out_chunk",
    "mk_list",
];
const APPEND_TYPES: &[&str] = &[
    "struct cio_chunk *",
    "struct cio_stream *",
    SIZE_T,
    "struct sb_out_queue *",
];
const REMOVE_TYPES: &[&str] = &[
    "struct cio_chunk *",
    "struct sb_out_queue *",
    "struct sb_out_chunk *",
    "int",
];

/// `sb_append_chunk_to_segregated_backlog`, `sb.c:262-290`, with
/// `flb_errno()` and `flb_warn` expanded. `CHARGE` is the charge statement
/// (`sb.c:277-287`), `EXTRA` a statement after it.
fn append(before_alloc: &str, charge: &str, extra: &str) -> String {
    format!(
        "{SB}int sb_append(struct cio_chunk *target_chunk, struct cio_stream *stream,
              size_t target_chunk_size, struct sb_out_queue *backlog)
{{
    struct sb_out_chunk *chunk;
    {before_alloc}
    chunk = sb_allocate_chunk(target_chunk, stream, target_chunk_size);
    if (chunk == ((void *)0)) {{
        flb_errno_print(0, \"sb.c\", 271);
        return -1;
    }}

    mk_list_add(&chunk->_head, &backlog->chunks);

    {charge}
    {extra}
    return 0;
}}
"
    )
}

const SB_CHARGE: &str = "if (target_chunk_size > (18446744073709551615UL) - backlog->ins->fs_backlog_chunks_size) {
        if (flb_log_check(2)) flb_log_print(2, 0, 0, \"overflow %s %zu %zu\",
                 flb_output_name(backlog->ins), backlog->ins->fs_backlog_chunks_size, target_chunk_size);
        backlog->ins->fs_backlog_chunks_size = (18446744073709551615UL);
    }
    else {
        backlog->ins->fs_backlog_chunks_size += target_chunk_size;
    }";

/// The matching iteration of `sb_remove_chunk_from_segregated_backlog`
/// (`sb.c:224-244`) as a loop-free function over the iterated element
/// `chunk`; `DISCHARGE` is the statement at `sb.c:228-238`.
fn remove(discharge: &str) -> String {
    format!(
        "{SB}void sb_remove_match(struct cio_chunk *target_chunk, struct sb_out_queue *backlog,
                     struct sb_out_chunk *chunk, int destroy)
{{
    if (chunk->chunk == target_chunk) {{
        mk_list_del(&chunk->_head);

        {discharge}

        if (destroy) {{
            sb_destroy_chunk(chunk);
        }}
    }}
}}
"
    )
}

const SB_DISCHARGE: &str = "if (chunk->size > backlog->ins->fs_backlog_chunks_size) {
            if (flb_log_check(2)) flb_log_print(2, 0, 0, \"underflow %s %zu %zu\",
                     flb_output_name(backlog->ins), backlog->ins->fs_backlog_chunks_size, chunk->size);
            backlog->ins->fs_backlog_chunks_size = 0;
        }
        else {
            backlog->ins->fs_backlog_chunks_size -= chunk->size;
        }";

fn backlog_spec() -> LedgerSpec<'static> {
    spec("fs_backlog_chunks_size", LIST)
}

fn check_append(source: &str) -> Result<Vec<LedgerTransition>, String> {
    let summary = import(
        &logged(source),
        "sb_append",
        "int",
        APPEND_TYPES,
        SB_RECORDS,
    )?;
    transitions(&summary, &backlog_spec())
}

fn check_remove(source: &str) -> Result<Vec<LedgerTransition>, String> {
    let summary = import(
        &logged(source),
        "sb_remove_match",
        "void",
        REMOVE_TYPES,
        SB_RECORDS,
    )?;
    transitions(&summary, &backlog_spec())
}

const NOT_ADMITTED: &str =
    "rule 92: ledger `fs_backlog_chunks_size` write is not an admitted saturating transition";
const NOT_PAIRED: &str = "rule 92: ledger `fs_backlog_chunks_size` transitions are not paired one-to-one with the list operations on a path";

#[test]
fn the_storage_backlog_charge_and_discharge_are_paired_saturating_transitions() {
    if !clang_available() {
        return;
    }
    // Allocation failure: no link, no charge. Success: one link, one charge
    // on each arm of the saturation test.
    assert_eq!(
        check_append(&append("", SB_CHARGE, "")),
        Ok(vec![OverflowClamp, OverflowClamp, Charge])
    );
    // Match: one unlink, one discharge on each arm; no match: neither.
    assert_eq!(
        check_remove(&remove(SB_DISCHARGE)),
        // (clamp with log x2 | discharge) x destroy taken or not.
        Ok(vec![
            UnderflowClamp,
            UnderflowClamp,
            UnderflowClamp,
            UnderflowClamp,
            Discharge,
            Discharge
        ])
    );
}

#[test]
fn a_wrapping_update_refuses() {
    if !clang_available() {
        return;
    }
    // Mutation: the bare decrement and the bare increment (wrap reachable).
    assert_eq!(
        check_remove(&remove(
            "backlog->ins->fs_backlog_chunks_size -= chunk->size;"
        )),
        Err(NOT_ADMITTED.into())
    );
    assert_eq!(
        check_append(&append(
            "",
            "backlog->ins->fs_backlog_chunks_size += target_chunk_size;",
            ""
        )),
        Err(NOT_ADMITTED.into())
    );
    let fs = spec("fs_chunks_size", None);
    let bare = logged(HELPERS).replace(
        "    if (bytes > o_ins->fs_chunks_size) {",
        "    if (bytes > o_ins->id) {",
    );
    let summary = import(
        &bare,
        "flb_input_chunk_output_size_subtract",
        "void",
        &["struct flb_output_instance *", SIZE_T],
        &["flb_output_instance"],
    );
    // A guard that does not read the ledger leaves the update unguarded.
    match summary {
        Ok(summary) => assert_eq!(
            transitions(&summary, &fs),
            Err(
                "rule 92: ledger `fs_chunks_size` write is not an admitted saturating transition"
                    .into()
            )
        ),
        Err(reason) => panic!("{reason}"),
    }
}

#[test]
fn a_misshapen_guard_or_clamp_refuses() {
    if !clang_available() {
        return;
    }
    for discharge in [
        // The guard compares another operand.
        "if (chunk->size + 1 > backlog->ins->fs_backlog_chunks_size) {
            backlog->ins->fs_backlog_chunks_size = 0;
        }
        else {
            backlog->ins->fs_backlog_chunks_size -= chunk->size;
        }",
        // The arms are swapped.
        "if (chunk->size > backlog->ins->fs_backlog_chunks_size) {
            backlog->ins->fs_backlog_chunks_size -= chunk->size;
        }
        else {
            backlog->ins->fs_backlog_chunks_size = 0;
        }",
        // The clamp writes another value.
        "if (chunk->size > backlog->ins->fs_backlog_chunks_size) {
            backlog->ins->fs_backlog_chunks_size = 1;
        }
        else {
            backlog->ins->fs_backlog_chunks_size -= chunk->size;
        }",
        // A store between the guard and the update.
        "if (chunk->size > backlog->ins->fs_backlog_chunks_size) {
            backlog->ins->fs_backlog_chunks_size = 0;
        }
        else {
            chunk->size = chunk->size;
            backlog->ins->fs_backlog_chunks_size -= chunk->size;
        }",
        // A call between the guard and the update.
        "if (chunk->size > backlog->ins->fs_backlog_chunks_size) {
            backlog->ins->fs_backlog_chunks_size = 0;
        }
        else {
            hook_fails();
            backlog->ins->fs_backlog_chunks_size -= chunk->size;
        }",
    ] {
        assert_eq!(
            check_remove(&remove(discharge)),
            Err(NOT_ADMITTED.into()),
            "{discharge}"
        );
    }
    // `>=` would be sound, but the consumer does not admit the node at all.
    assert_eq!(
        check_remove(&remove(
            "if (chunk->size >= backlog->ins->fs_backlog_chunks_size) {
            backlog->ins->fs_backlog_chunks_size = 0;
        }
        else {
            backlog->ins->fs_backlog_chunks_size -= chunk->size;
        }"
        )),
        Err(
            "typed-source node `binary >=` is not classified by the rule 92 ledger consumer".into()
        )
    );
}

#[test]
fn the_pinned_log_argument_list_is_refused_before_the_ledger() {
    if !clang_available() {
        return;
    }
    // The pinned `flb_warn` arguments pass `flb_output_name(...)` beside the
    // ledger reads; T has no ordering model for an opaque call among
    // unsequenced operands, so the faithful bodies stay Unknown for the
    // ledger property (follow-up FB-LEDGER-LOG-ARGUMENT-ORDER).
    const UNSEQUENCED: &str = "unsequenced call/read operands require an explicit ordering model";
    assert_eq!(
        import(
            HELPERS,
            "flb_input_chunk_output_size_subtract",
            "void",
            &["struct flb_output_instance *", SIZE_T],
            &["flb_output_instance"],
        ),
        Err(UNSEQUENCED.into())
    );
    assert_eq!(
        import(
            &append("", SB_CHARGE, ""),
            "sb_append",
            "int",
            APPEND_TYPES,
            SB_RECORDS
        ),
        Err(UNSEQUENCED.into())
    );
}

#[test]
fn a_missing_or_doubled_update_refuses() {
    if !clang_available() {
        return;
    }
    // Double decrement for one removal.
    let doubled = format!("{SB_DISCHARGE}\n        {SB_DISCHARGE}");
    assert_eq!(check_remove(&remove(&doubled)), Err(NOT_PAIRED.into()));
    // Removal that forgets the decrement.
    assert_eq!(check_remove(&remove("")), Err(NOT_PAIRED.into()));
    // Partial rollback after the charge: the unlink without its discharge
    // refuses; with its saturating discharge it is admitted.
    let rollback_without = "if (hook_fails()) {
        mk_list_del(&chunk->_head);
        return -1;
    }";
    assert_eq!(
        check_append(&append("", SB_CHARGE, rollback_without)),
        Err(NOT_PAIRED.into())
    );
    let rollback_with = "if (hook_fails()) {
        mk_list_del(&chunk->_head);
        if (target_chunk_size > backlog->ins->fs_backlog_chunks_size) {
            backlog->ins->fs_backlog_chunks_size = 0;
        }
        else {
            backlog->ins->fs_backlog_chunks_size -= target_chunk_size;
        }
        return -1;
    }";
    assert_eq!(
        check_append(&append("", SB_CHARGE, rollback_with)),
        // Per path, in path order: (clamp | log fork | charge) x (rollback
        // underflow, rollback discharge, no rollback).
        Ok(vec![
            OverflowClamp,
            UnderflowClamp,
            OverflowClamp,
            Discharge,
            OverflowClamp,
            OverflowClamp,
            UnderflowClamp,
            OverflowClamp,
            Discharge,
            OverflowClamp,
            Charge,
            UnderflowClamp,
            Charge,
            Discharge,
            Charge,
        ])
    );
    // The charge moved before the allocation test: the failure path is
    // charged without a link.
    assert_eq!(
        check_append(&append(SB_CHARGE, "", "")),
        Err(NOT_PAIRED.into())
    );
    // Without pairing, a helper may change its ledger once per path.
    let twice = logged(HELPERS).replace(
        "    o_ins->fs_chunks_size -= bytes;\n}",
        "    o_ins->fs_chunks_size -= bytes;\n    o_ins->fs_chunks_size -= bytes;\n}",
    );
    let summary = import(
        &twice,
        "flb_input_chunk_output_size_subtract",
        "void",
        &["struct flb_output_instance *", SIZE_T],
        &["flb_output_instance"],
    )
    .unwrap();
    assert_eq!(
        transitions(&summary, &spec("fs_chunks_size", None)),
        Err(
            "rule 92: ledger `fs_chunks_size` write is not an admitted saturating transition"
                .into()
        )
    );
}

#[test]
fn the_admission_is_valid() {
    LEDGER_ADMISSION.validate().unwrap();
}

/// The decision vectors `QCore.BacklogLedger.decide_vectors` restates.
#[test]
fn decision_vectors() {
    use LedgerTransition::*;
    use WriteForm::*;
    let forms = [Subtract, Add, Zero, Maximum, Other];
    let guards = [
        None,
        Some((false, false)),
        Some((false, true)),
        Some((true, false)),
        Some((true, true)),
    ];
    let mut admitted = Vec::new();
    for form in forms {
        for guard in guards {
            for clean in [false, true] {
                if let Some(t) = decide(form, guard, clean) {
                    admitted.push((form, guard, clean, t));
                }
            }
        }
    }
    assert_eq!(
        admitted,
        vec![
            (Subtract, Some((false, false)), true, Discharge),
            (Add, Some((true, false)), true, Charge),
            (Zero, Some((false, true)), false, UnderflowClamp),
            (Zero, Some((false, true)), true, UnderflowClamp),
            (Maximum, Some((true, true)), false, OverflowClamp),
            (Maximum, Some((true, true)), true, OverflowClamp),
        ]
    );
}

// ---------------------------------------------------------------------------
// FB-LEDGER-MEM-BUF-LIMIT-BOUND: the pause/resume shapes of `mem_buf_limit`.
// ---------------------------------------------------------------------------

/// Card FB-LEDGER-MEM-BUF-LIMIT-BOUND: route T's paths through the three
/// `src/flb_input_chunk.c` functions that write to a memory instance or move
/// its pause state, read row by row against `QCore.MemBufLimitBound`. The
/// census is `docs/reports/fb-ledger-mem-buf-limit-bound/CENSUS.md`.
mod mem_buf_limit {
    use super::*;
    use crate::typed_source::SourcePath;

    /// The fixture: `flb_input_chunk_set_limits` (`:3004-3077`),
    /// `flb_input_chunk_protect` (`:3081-3132`) and `input_chunk_append_raw`
    /// (`:3251-3600`) at `046accf6`, over records cut to the fields they
    /// read. `FLB_HAVE_METRICS` is on, as in the pinned production build.
    /// Stated deviations, none on a statement that changes chunk content or
    /// `mem_buf_status`:
    ///
    /// * the static inline helpers `flb_input_paused`,
    ///   `flb_input_chunk_is_mem_overlimit` and
    ///   `flb_input_chunk_is_storage_overlimit` are declared functions (their
    ///   bodies are census rows of their own);
    /// * every `flb_warn`/`flb_info`/`flb_debug`/`flb_error`/`flb_trace` call
    ///   is dropped: route T refuses their unsequenced `flb_input_name(...)`
    ///   argument lists (FB-LEDGER-LOG-ARGUMENT-ORDER);
    /// * route T reads at most 64 paths per function, so `set_limits` keeps
    ///   only its memory half (the storage half `:3036-3042` and the
    ///   collector resume `:3049-3074` write neither the counter nor
    ///   `mem_buf_status`), and the append is read as three consecutive
    ///   segments, `append_entry` (`:3280-3325`), `append_write`
    ///   (`:3359-3488`) and `append_finish` (`:3490-3600`), each entered only
    ///   where the one before falls through; `ic` and `set_down` are the
    ///   later segments' parameters (`input_chunk_get`'s result and
    ///   out-parameter, `:3358`);
    /// * the filter's out-parameters are read through two declared functions
    ///   (`flb_filter_do_size`, `flb_filter_do_buffer`) because route T does
    ///   not read a local after its address escapes;
    /// * dropped from the append: the memrb drop metrics (`:3293-3304`), the
    ///   tag clean-up (`:3326-3353`), the real-size bookkeeping, record
    ///   metrics and tracing (`:3363-3367`, `:3380-3435`), the filtered buffer's free
    ///   (`:3464-3467`), the size-0 diff reset and the chunk lock
    ///   (`:3500-3507`), and the filesystem-only down branch (`:3558-3576`,
    ///   `cio_chunk_down` is a no-op on a memory chunk, `cio_chunk.c:556-571`);
    ///   the output size update (`:3578-3591`) stays as one unconditional
    ///   call, and `struct flb_sp *` is `void *` with `ic` for the content.
    ///
    /// `size_t` is `__SIZE_TYPE__` so the signatures read the same on LLP64.
    pub(super) const MBL: &str = "\
typedef __SIZE_TYPE__ size_t;
struct cio_chunk;
struct flb_config { int is_running; int is_ingestion_active; int is_shutting_down; void *stream_processor_ctx; };
struct flb_storage_input { int type; };
struct flb_input_instance {
    int storage_type; int flags; int routable;
    int mem_buf_status; int storage_buf_status; int rate_gate_status;
    size_t mem_chunks_size; size_t mem_buf_limit;
    struct flb_storage_input *storage; struct flb_config *config;
};
struct flb_input_chunk { struct cio_chunk *chunk; int event_type; };
int flb_input_paused(struct flb_input_instance *i);
int flb_input_chunk_is_mem_overlimit(struct flb_input_instance *i);
int flb_input_chunk_is_storage_overlimit(struct flb_input_instance *i);
size_t flb_input_chunk_total_size(struct flb_input_instance *in);
int flb_input_rate_gate_protect(struct flb_input_instance *ins);
void flb_input_rate_gate_maybe_resume(struct flb_input_instance *ins);
int flb_input_pause(struct flb_input_instance *ins);
int flb_input_resume(struct flb_input_instance *ins);
int memrb_input_chunk_release_space(struct flb_input_instance *ins, size_t required_space,
                                    size_t *dropped_chunks, size_t *dropped_bytes);
struct flb_input_chunk *input_chunk_get(struct flb_input_instance *in, int event_type,
                                        const char *tag, int tag_len, size_t chunk_size,
                                        int *set_down);
int flb_input_chunk_is_up(struct flb_input_chunk *ic);
int cio_chunk_up_force(struct cio_chunk *ch);
int cio_chunk_down(struct cio_chunk *ch);
int cio_chunk_tx_rollback(struct cio_chunk *ch);
size_t cio_chunk_get_content_size(struct cio_chunk *ch);
size_t flb_filter_do_size(struct flb_input_chunk *ic, const void *data, size_t bytes,
                          const char *tag, int tag_len, struct flb_config *config);
void *flb_filter_do_buffer(struct flb_input_chunk *ic);
int flb_input_chunk_write(void *data, const char *buf, size_t len);
int flb_input_chunk_destroy(struct flb_input_chunk *ic, int del);
int flb_sp_do(void *sp, struct flb_input_instance *in, const char *tag, int tag_len,
              struct flb_input_chunk *ic);
void flb_input_chunk_update_output_instances(struct flb_input_chunk *ic, size_t chunk_size);
size_t flb_input_chunk_set_limits(struct flb_input_instance *in);
int flb_input_chunk_protect(struct flb_input_instance *i, size_t just_written_size);

size_t flb_input_chunk_set_limits(struct flb_input_instance *in)
{
    int mem_limit_cleared;
    int rate_gate_was_paused;
    size_t total;

    total = flb_input_chunk_total_size(in);

    in->mem_chunks_size = total;

    mem_limit_cleared = 0;
    rate_gate_was_paused = (in->rate_gate_status == 0);

    if (flb_input_chunk_is_mem_overlimit(in) == 0 &&
        in->config->is_running == 1 &&
        in->config->is_ingestion_active == 1 &&
        in->mem_buf_status == 0) {
        in->mem_buf_status = 1;
        mem_limit_cleared = 1;
    }
    flb_input_rate_gate_maybe_resume(in);

    return total;
}

int flb_input_chunk_protect(struct flb_input_instance *i, size_t just_written_size)
{
    struct flb_storage_input *storage = i->storage;

    if (flb_input_chunk_is_storage_overlimit(i) == 1) {
        flb_input_pause(i);
        i->storage_buf_status = 0;
        return 1;
    }

    if (flb_input_rate_gate_protect(i) == 1) {
        return 1;
    }

    if (storage->type == 0) {
        return 0;
    }

    if (flb_input_chunk_is_mem_overlimit(i) == 1) {
        if (i->storage_type == 10) {
            return 0;
        }

        flb_input_pause(i);
        i->mem_buf_status = 0;
        return 1;
    }

    return 0;
}

int append_entry(struct flb_input_instance *in, size_t buf_size)
{
    int ret;
    size_t dropped_chunks;
    size_t dropped_bytes;

    if (in->storage_type == 10) {
        ret = flb_input_chunk_is_mem_overlimit(in);
        if (ret) {
            dropped_chunks = 0;
            dropped_bytes = 0;

            ret = memrb_input_chunk_release_space(in, buf_size,
                                                  &dropped_chunks, &dropped_bytes);

            if (ret != 0) {
                return -1;
            }
        }
    }

    if (flb_input_paused(in) == 1 &&
        (in->config->is_shutting_down == 0 ||
         (in->flags & 8192) == 0)) {
        return -1;
    }

    if (buf_size == 0) {
        return -1;
    }

    return 0;
}

int append_write(struct flb_input_instance *in, struct flb_input_chunk *ic, int set_down,
                 int event_type, const char *tag, int tag_len,
                 const void *buf, size_t buf_size)
{
    int ret;
    void *final_data_buffer;
    size_t final_data_size;
    size_t filtered_data_size;

    if (!ic) {
        return -1;
    }

    ret = flb_input_chunk_is_up(ic);
    if (ret == 0) {
        ret = cio_chunk_up_force(ic->chunk);
        if (ret == -1) {
            return -1;
        }
        set_down = 1;
    }

    final_data_buffer = (void *) buf;
    final_data_size = buf_size;

    if (event_type == 0) {
        filtered_data_size = flb_filter_do_size(ic,
                                                buf, buf_size,
                                                tag, tag_len,
                                                in->config);

        final_data_buffer = flb_filter_do_buffer(ic);
        final_data_size = filtered_data_size;
    }

    if (final_data_size > 0){
        ret = flb_input_chunk_write(ic,
                                    final_data_buffer,
                                    final_data_size);
    }
    else {
        ret = 0;
    }

    if (ret == -1) {
        cio_chunk_tx_rollback(ic->chunk);
        if (set_down == 1) {
            cio_chunk_down(ic->chunk);
        }

        return -1;
    }

    return 0;
}

int append_finish(struct flb_input_instance *in, struct flb_input_chunk *ic, int set_down,
                  const char *tag, int tag_len, size_t final_data_size)
{
    size_t content_size;

    content_size = cio_chunk_get_content_size(ic->chunk);

    if (content_size == 0) {
        flb_input_chunk_destroy(ic, 1);
        flb_input_chunk_set_limits(in);
        return 0;
    }
    else if (in->config->stream_processor_ctx &&
             ic->event_type == 0) {
        flb_sp_do(in->config->stream_processor_ctx,
                  in,
                  tag, tag_len,
                  ic);
    }

    if (set_down == 1) {
        cio_chunk_down(ic->chunk);
    }

    if (in->routable == 0) {
        flb_input_chunk_destroy(ic, 1);
        return 0;
    }

    flb_input_chunk_set_limits(in);

    flb_input_chunk_update_output_instances(ic, content_size);

    flb_input_chunk_protect(in, final_data_size);
    return 0;
}
";

    const CALLS: &[&str] = &[
        "flb_input_paused",
        "flb_input_chunk_is_mem_overlimit",
        "flb_input_chunk_is_storage_overlimit",
        "flb_input_chunk_total_size",
        "flb_input_rate_gate_protect",
        "flb_input_rate_gate_maybe_resume",
        "flb_input_pause",
        "flb_input_resume",
        "memrb_input_chunk_release_space",
        "input_chunk_get",
        "flb_input_chunk_is_up",
        "cio_chunk_up_force",
        "cio_chunk_down",
        "cio_chunk_tx_rollback",
        "cio_chunk_get_content_size",
        "flb_filter_do_size",
        "flb_filter_do_buffer",
        "flb_input_chunk_write",
        "flb_input_chunk_destroy",
        "flb_sp_do",
        "flb_input_chunk_update_output_instances",
        "flb_input_chunk_set_limits",
        "flb_input_chunk_protect",
    ];

    fn import_mbl(source: &str, function: &str) -> Result<SourceSummary, String> {
        let (result, types): (&str, &[&str]) = match function {
            "flb_input_chunk_protect" => ("int", &["struct flb_input_instance *", SIZE_T]),
            "flb_input_chunk_set_limits" => (SIZE_T, &["struct flb_input_instance *"]),
            "append_entry" => ("int", &["struct flb_input_instance *", SIZE_T]),
            "append_write" => (
                "int",
                &[
                    "struct flb_input_instance *",
                    "struct flb_input_chunk *",
                    "int",
                    "int",
                    "const char *",
                    "int",
                    "const void *",
                    SIZE_T,
                ],
            ),
            _ => (
                "int",
                &[
                    "struct flb_input_instance *",
                    "struct flb_input_chunk *",
                    "int",
                    "const char *",
                    "int",
                    SIZE_T,
                ],
            ),
        };
        let results = import_source_functions_admitting(
            &C11TranslationUnit {
                source,
                ..Default::default()
            },
            "046accf6",
            &[function],
            &SourceImportProfile {
                records: &[
                    "flb_input_instance",
                    "flb_storage_input",
                    "flb_config",
                    "flb_input_chunk",
                ],
                sizeof_types: T_SIZEOF,
                signature_constraints: &[(function, result, types)],
                restricted_calls: CALLS,
            },
            &LEDGER_ADMISSION,
        )
        .expect("unit admitted");
        match results.into_iter().next() {
            Some(SourceResult::Summary(summary)) => Ok(summary),
            Some(SourceResult::Unknown { reason, .. }) => Err(reason),
            None => panic!("no result"),
        }
    }

    /// One path as the reading sees it, in order.
    #[derive(Clone, Debug, PartialEq, Eq)]
    enum Event {
        /// The callee or field a guard compares, the literal it compares
        /// with (empty for a truth test), and whether it was taken.
        Guard(String, String, bool),
        /// A field written and what was written (a literal or a callee).
        Write(String, String),
        /// A call and what each argument resolves to.
        Call(String, Vec<String>),
        Return(String),
    }

    /// What a value names: the callee whose result it is, the field it reads,
    /// the parameter, or the literal.
    fn resolve(value: &SourceValue, operations: &[Op]) -> String {
        match &value.expression {
            Expr::Symbol { operation } => match operations.get(*operation) {
                Some(Op::Call { function, .. }) => function.clone(),
                Some(Op::Read { place }) => resolve(place, operations),
                _ => format!("op{operation}"),
            },
            Expr::Field { name, .. } => name.clone(),
            Expr::Parameter { index } => format!("param{index}"),
            Expr::Integer { value } => value.clone(),
            Expr::Unary { operator, value } if operator == "-" => {
                format!("-{}", resolve(value, operations))
            }
            Expr::Cast { value, .. } | Expr::Unary { value, .. } => resolve(value, operations),
            Expr::Binary { left, .. } => resolve(left, operations),
            Expr::LocalStorage { local } => format!("local{local}"),
            other => format!("{other:?}"),
        }
    }

    fn events(path: &SourcePath) -> Vec<Event> {
        let operations = path.operations.as_slice();
        operations
            .iter()
            .filter_map(|operation| match operation {
                Op::Guard { condition, taken } => {
                    let literal = match &condition.expression {
                        Expr::Binary { right, .. } => match &right.expression {
                            Expr::Integer { .. } | Expr::Unary { .. } | Expr::Cast { .. } => {
                                resolve(right, operations)
                            }
                            _ => String::new(),
                        },
                        _ => String::new(),
                    };
                    Some(Event::Guard(
                        resolve(condition, operations),
                        literal,
                        *taken,
                    ))
                }
                Op::Write { place, value } => Some(Event::Write(
                    resolve(place, operations),
                    resolve(value, operations),
                )),
                Op::Call {
                    function,
                    arguments,
                    ..
                } => Some(Event::Call(
                    function.clone(),
                    arguments
                        .iter()
                        .map(|argument| resolve(argument, operations))
                        .collect(),
                )),
                Op::ReturnValue { value } => Some(Event::Return(resolve(value, operations))),
                Op::Return => Some(Event::Return(String::new())),
                _ => None,
            })
            .collect()
    }

    fn guard(events: &[Event], subject: &str, literal: &str) -> Option<bool> {
        events.iter().find_map(|event| match event {
            Event::Guard(s, l, taken) if s == subject && l == literal => Some(*taken),
            _ => None,
        })
    }

    fn writes(events: &[Event], field: &str, value: &str) -> bool {
        events
            .iter()
            .any(|event| matches!(event, Event::Write(f, v) if f == field && v == value))
    }

    fn calls(events: &[Event], function: &str) -> Vec<usize> {
        events
            .iter()
            .enumerate()
            .filter_map(|(at, event)| match event {
                Event::Call(name, _) if name == function => Some(at),
                _ => None,
            })
            .collect()
    }

    fn summary(source: &str, function: &str) -> SourceSummary {
        import_mbl(source, function).unwrap_or_else(|reason| panic!("{function}: {reason}"))
    }

    fn paths(summary: &SourceSummary) -> Vec<Vec<Event>> {
        summary.paths().iter().map(events).collect()
    }

    /// `in->storage_type` as `QCore.MemBufLimitBound.Store` names it.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
    enum Store {
        Mem,
        Memrb,
        Fs,
    }

    /// `QCore.MemBufLimitBound.protectPausesMem`.
    fn protect_pauses_mem(storage_over: bool, rate_gate: bool, store: Store, over: bool) -> bool {
        if storage_over || rate_gate || store == Store::Fs {
            false
        } else {
            over && store != Store::Memrb
        }
    }

    type ProtectRow = (bool, bool, Store, bool, bool);

    /// One row per path of `flb_input_chunk_protect`: (storage overlimit,
    /// rate gate fired, store, memory overlimit, pauses). A test the path
    /// does not reach reads as the value `protect_vectors` fills in.
    fn protect_rows(summary: &SourceSummary) -> Vec<ProtectRow> {
        paths(summary)
            .iter()
            .map(|events| {
                let store = if guard(events, "type", "0") == Some(true) {
                    Store::Fs
                } else if guard(events, "storage_type", "10") == Some(true) {
                    Store::Memrb
                } else {
                    Store::Mem
                };
                (
                    guard(events, "flb_input_chunk_is_storage_overlimit", "1") == Some(true),
                    guard(events, "flb_input_rate_gate_protect", "1") == Some(true),
                    store,
                    guard(events, "flb_input_chunk_is_mem_overlimit", "1") != Some(false),
                    writes(events, "mem_buf_status", "0"),
                )
            })
            .collect()
    }

    /// `QCore.MemBufLimitBound.protect_vectors`, row for row.
    const PROTECT_VECTORS: [ProtectRow; 6] = [
        (true, false, Store::Mem, true, false),
        (false, true, Store::Mem, true, false),
        (false, false, Store::Fs, true, false),
        (false, false, Store::Memrb, true, false),
        (false, false, Store::Mem, true, true),
        (false, false, Store::Mem, false, false),
    ];

    #[test]
    fn protect_pauses_only_past_the_limit_and_only_when_no_earlier_return_fires() {
        if !clang_available() {
            return;
        }
        let protect = summary(MBL, "flb_input_chunk_protect");
        let rows = protect_rows(&protect);
        assert_eq!(rows, PROTECT_VECTORS);
        for (storage, rate, store, over, pauses) in rows {
            assert_eq!(pauses, protect_pauses_mem(storage, rate, store, over));
        }
        // The pause write follows `flb_input_pause` on its one path.
        for events in paths(&protect) {
            if writes(&events, "mem_buf_status", "0") {
                let write = events
                    .iter()
                    .position(|e| matches!(e, Event::Write(f, _) if f == "mem_buf_status"))
                    .unwrap();
                assert!(calls(&events[..write], "flb_input_pause").len() == 1);
            }
        }
        // The rate gate's row is the counterexample to the bound without
        // P-MBL-RATE-GATE-QUIET (`rate_gate_breaks_bound`): a memory instance
        // past its limit returns before the memory test.
        assert!(PROTECT_VECTORS.contains(&(false, true, Store::Mem, true, false)));
    }

    #[test]
    fn a_protect_that_pauses_before_the_memory_test_reads_other_rows() {
        if !clang_available() {
            return;
        }
        // Mutation control: the pause moved above the overlimit test.
        let mutated = MBL.replace(
            "    if (flb_input_chunk_is_mem_overlimit(i) == 1) {\n        if (i->storage_type == 10) {",
            "    i->mem_buf_status = 0;\n    if (flb_input_chunk_is_mem_overlimit(i) == 1) {\n        if (i->storage_type == 10) {",
        );
        assert_ne!(mutated, MBL);
        assert_ne!(
            protect_rows(&summary(&mutated, "flb_input_chunk_protect")),
            PROTECT_VECTORS
        );
    }

    /// `QCore.MemBufLimitBound.resumes`.
    fn resumes(over: bool, running: bool, active: bool, paused: bool) -> bool {
        !over && running && active && paused
    }

    type ResumeRow = (bool, bool, bool, bool, bool);

    /// `QCore.MemBufLimitBound.resume_vectors`.
    const RESUME_VECTORS: [ResumeRow; 5] = [
        (false, true, true, true, true),
        (true, true, true, true, false),
        (false, false, true, true, false),
        (false, true, false, true, false),
        (false, true, true, false, false),
    ];

    /// Rows `(memory overlimit, is_running, is_ingestion_active, paused,
    /// resumes)` of `flb_input_chunk_set_limits`; an unreached test reads as
    /// `true`, as in `resume_vectors`.
    fn resume_rows(summary: &SourceSummary) -> Result<Vec<ResumeRow>, String> {
        let mut rows = Vec::new();
        for events in paths(summary) {
            // The snapshot is assigned only from the stream's total, first.
            let snapshot: Vec<_> = events
                .iter()
                .filter_map(|e| match e {
                    Event::Write(f, v) if f == "mem_chunks_size" => Some(v.as_str()),
                    _ => None,
                })
                .collect();
            if snapshot != ["flb_input_chunk_total_size"] {
                return Err(format!("mem_chunks_size written as {snapshot:?}"));
            }
            let row = (
                guard(&events, "flb_input_chunk_is_mem_overlimit", "0") == Some(false),
                guard(&events, "is_running", "1") != Some(false),
                guard(&events, "is_ingestion_active", "1") != Some(false),
                guard(&events, "mem_buf_status", "0") != Some(false),
                writes(&events, "mem_buf_status", "1"),
            );
            if !rows.contains(&row) {
                rows.push(row);
            }
        }
        rows.sort();
        Ok(rows)
    }

    #[test]
    fn set_limits_snapshots_the_total_and_resumes_only_below_the_limit() {
        if !clang_available() {
            return;
        }
        let rows = resume_rows(&summary(MBL, "flb_input_chunk_set_limits")).unwrap();
        let mut expected = RESUME_VECTORS.to_vec();
        expected.sort();
        assert_eq!(rows, expected);
        for (over, running, active, paused, resumed) in rows {
            assert_eq!(resumed, resumes(over, running, active, paused));
        }
        // Mutation controls: a resume without the limit test, and a snapshot
        // written from another value.
        let unguarded = MBL.replace(
            "    if (flb_input_chunk_is_mem_overlimit(in) == 0 &&\n        in->config->is_running == 1 &&",
            "    if (in->config->is_running == 1 &&",
        );
        assert_ne!(unguarded, MBL);
        // Without the test, an overlimit instance is never seen refusing to
        // resume: the `(over, .., no resume)` row is gone.
        let rows = resume_rows(&summary(&unguarded, "flb_input_chunk_set_limits")).unwrap();
        assert!(!rows.contains(&(true, true, true, true, false)));
        assert_ne!(rows, expected);
        let stale = MBL.replace(
            "    in->mem_chunks_size = total;\n\n    mem_limit_cleared",
            "    in->mem_chunks_size = 0;\n\n    mem_limit_cleared",
        );
        assert_ne!(stale, MBL);
        assert!(resume_rows(&summary(&stale, "flb_input_chunk_set_limits")).is_err());
    }

    /// How the gate let a path through: `(paused, is_shutting_down,
    /// SHUTDOWN_FLUSH)` as far as the path tested them.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
    enum Gate {
        /// `flb_input_paused(in)` false.
        Running,
        /// Paused, but shutting down with `FLB_INPUT_SHUTDOWN_FLUSH`: the
        /// counterexample to the bound without P-MBL-NO-SHUTDOWN-FLUSH.
        ShutdownFlush,
    }

    /// The `memrb` checker's arm on a path.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
    enum Memrb {
        NotMemrb,
        BelowLimit,
        Released,
    }

    /// `QCore.MemBufLimitBound.gateC`.
    fn gate_c(paused: bool, shutting_down: bool, flush: bool) -> bool {
        !(paused && (!shutting_down || !flush))
    }

    /// The paths of `append_entry` that fall through to the write, each with
    /// the gate arm and the `memrb` arm it took; every refusal returns -1.
    fn entry_rows(summary: &SourceSummary) -> Result<Vec<(Memrb, Gate)>, String> {
        let mut rows = Vec::new();
        for events in paths(summary) {
            let returned = events.iter().rev().find_map(|e| match e {
                Event::Return(v) => Some(v.clone()),
                _ => None,
            });
            let released = !calls(&events, "memrb_input_chunk_release_space").is_empty();
            if released
                && (guard(&events, "storage_type", "10") != Some(true)
                    || guard(&events, "flb_input_chunk_is_mem_overlimit", "") != Some(true))
            {
                return Err("memrb release outside its arm".into());
            }
            let paused = guard(&events, "flb_input_paused", "1");
            let not_shutting = guard(&events, "is_shutting_down", "0");
            let no_flush = guard(&events, "flags", "0");
            if returned.as_deref() != Some("0") {
                continue;
            }
            let gate = match (paused, not_shutting, no_flush) {
                (Some(false), None, None) => Gate::Running,
                (Some(true), Some(false), Some(false)) => Gate::ShutdownFlush,
                other => return Err(format!("falls through with gate {other:?}")),
            };
            // The admitted rows are exactly `gate_vectors`' admitted rows.
            let row = match gate {
                Gate::Running => (false, false, false),
                Gate::ShutdownFlush => (true, true, true),
            };
            assert!(gate_c(row.0, row.1, row.2));
            let memrb = match (guard(&events, "storage_type", "10"), released) {
                (Some(false), false) => Memrb::NotMemrb,
                (Some(true), false) => Memrb::BelowLimit,
                (Some(true), true) => Memrb::Released,
                other => return Err(format!("memrb arm {other:?}")),
            };
            rows.push((memrb, gate));
        }
        rows.sort();
        Ok(rows)
    }

    #[test]
    fn every_path_to_the_write_passes_the_paused_gate() {
        if !clang_available() {
            return;
        }
        let entry = summary(MBL, "append_entry");
        let mut expected = Vec::new();
        for memrb in [Memrb::NotMemrb, Memrb::BelowLimit, Memrb::Released] {
            for gate in [Gate::Running, Gate::ShutdownFlush] {
                expected.push((memrb, gate));
            }
        }
        assert_eq!(entry_rows(&entry), Ok(expected));
        // The refused rows of `gate_vectors`: paused and not shutting down, or
        // shutting down without the flag.
        assert!(
            !gate_c(true, false, false) && !gate_c(true, false, true) && !gate_c(true, true, false)
        );
        // A failed memrb release returns before the gate.
        assert!(paths(&entry).iter().any(|events| {
            !calls(events, "memrb_input_chunk_release_space").is_empty()
                && guard(events, "flb_input_paused", "1").is_none()
                && events.last() == Some(&Event::Return("-1".into()))
        }));
        // Mutation control: without the gate, a paused instance falls through.
        let ungated = MBL.replace(
            "    if (flb_input_paused(in) == 1 &&\n        (in->config->is_shutting_down == 0 ||\n         (in->flags & 8192) == 0)) {\n        return -1;\n    }\n",
            "",
        );
        assert_ne!(ungated, MBL);
        assert!(entry_rows(&summary(&ungated, "append_entry")).is_err());
    }

    /// The exits of `append_write`.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
    enum WriteExit {
        /// No write on the path: no chunk, `cio_chunk_up_force` failed, or
        /// `final_data_size == 0` (`QCore.MemBufLimitBound.Exit.refused`;
        /// route T keeps the `ret == -1` arm after `ret = 0` unfolded).
        Unwritten,
        /// The write failed and was rolled back; `cio_memfs_write` changes
        /// nothing on failure (`Exit.refused`).
        WriteFailed,
        /// One write of `final_data_size` bytes: `buf_size`, or the filter's
        /// output for a log chunk (the `W` of P-MBL-WRITE-BOUND).
        Written(&'static str),
    }

    fn write_rows(summary: &SourceSummary) -> Result<Vec<WriteExit>, String> {
        let mut rows = Vec::new();
        for events in paths(summary) {
            let written = calls(&events, "flb_input_chunk_write");
            let failed = guard(&events, "flb_input_chunk_write", "-1");
            let row = match written.as_slice() {
                [] => WriteExit::Unwritten,
                [at] => {
                    let Event::Call(_, arguments) = &events[*at] else {
                        unreachable!()
                    };
                    match failed {
                        Some(true) => {
                            if calls(&events[*at..], "cio_chunk_tx_rollback").len() != 1 {
                                return Err("failed write without rollback".into());
                            }
                            WriteExit::WriteFailed
                        }
                        Some(false) => match arguments[2].as_str() {
                            "param7" => WriteExit::Written("buf_size"),
                            "flb_filter_do_size" => WriteExit::Written("filtered"),
                            other => return Err(format!("write of {other}")),
                        },
                        None => return Err("write result untested".into()),
                    }
                }
                _ => return Err("more than one write on a path".into()),
            };
            rows.push(row);
        }
        rows.sort();
        Ok(rows)
    }

    #[test]
    fn one_call_writes_once_and_its_size_is_final_data_size() {
        if !clang_available() {
            return;
        }
        let rows = write_rows(&summary(MBL, "append_write")).unwrap();
        let count = |row| rows.iter().filter(|&&r| r == row).count();
        // Two arms of `cio_chunk_is_up` reach the write, each with or without
        // the filter: four written paths, each a single write.
        assert_eq!(count(WriteExit::Written("buf_size")), 2);
        assert_eq!(count(WriteExit::Written("filtered")), 2);
        assert_eq!(count(WriteExit::WriteFailed), 8);
        assert_eq!(count(WriteExit::Unwritten), 14);
        assert_eq!(rows.len(), 26);
        // Mutation control: a second write after the first.
        let twice = MBL.replace(
            "    if (ret == -1) {\n        cio_chunk_tx_rollback(ic->chunk);",
            "    ret = flb_input_chunk_write(ic, final_data_buffer, final_data_size);\n    if (ret == -1) {\n        cio_chunk_tx_rollback(ic->chunk);",
        );
        assert_ne!(twice, MBL);
        assert!(write_rows(&summary(&twice, "append_write")).is_err());
    }

    /// The exits of `append_finish`, as `QCore.MemBufLimitBound.Exit` names
    /// them.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
    enum Finish {
        Emptied,
        Unroutable,
        Stored,
    }

    /// Calls a finish path may make besides its exit's own: the stream
    /// processor (P-MBL-NO-REENTRY), `cio_chunk_down` (a no-op on a memory
    /// chunk) and the output size update (it releases, never writes).
    const PREMISE_CALLS: &[&str] = &[
        "cio_chunk_get_content_size",
        "flb_sp_do",
        "cio_chunk_down",
        "flb_input_chunk_update_output_instances",
    ];

    fn finish_rows(summary: &SourceSummary) -> Result<Vec<Finish>, String> {
        let mut rows = Vec::new();
        for events in paths(summary) {
            let names: Vec<&str> = events
                .iter()
                .filter_map(|e| match e {
                    Event::Call(name, _) => Some(name.as_str()),
                    _ => None,
                })
                .filter(|name| !PREMISE_CALLS.contains(name))
                .collect();
            let row = match names.as_slice() {
                ["flb_input_chunk_destroy", "flb_input_chunk_set_limits"] => Finish::Emptied,
                ["flb_input_chunk_destroy"] => Finish::Unroutable,
                ["flb_input_chunk_set_limits", "flb_input_chunk_protect"] => {
                    if !matches!(events.iter().rev().find(|e| matches!(e, Event::Call(..))), Some(Event::Call(name, _)) if name == "flb_input_chunk_protect")
                    {
                        return Err("a call after protect".into());
                    }
                    Finish::Stored
                }
                other => return Err(format!("unclassified finish {other:?}")),
            };
            rows.push(row);
        }
        rows.sort();
        Ok(rows)
    }

    #[test]
    fn every_stored_write_reaches_set_limits_then_protect() {
        if !clang_available() {
            return;
        }
        let rows = finish_rows(&summary(MBL, "append_finish")).unwrap();
        let count = |row| rows.iter().filter(|&&r| r == row).count();
        assert_eq!(count(Finish::Emptied), 1);
        assert_eq!(count(Finish::Unroutable), 6);
        assert_eq!(count(Finish::Stored), 6);
        // Mutation controls: protect before the snapshot is refreshed, and a
        // stored path that skips protect.
        let early = MBL.replace(
            "    flb_input_chunk_set_limits(in);\n\n    flb_input_chunk_update_output_instances(ic, content_size);\n\n    flb_input_chunk_protect(in, final_data_size);",
            "    flb_input_chunk_protect(in, final_data_size);\n\n    flb_input_chunk_set_limits(in);\n\n    flb_input_chunk_update_output_instances(ic, content_size);",
        );
        assert_ne!(early, MBL);
        assert!(finish_rows(&summary(&early, "append_finish")).is_err());
        let skipped = MBL.replace(
            "\n    flb_input_chunk_protect(in, final_data_size);\n    return 0;",
            "\n    return 0;",
        );
        assert_ne!(skipped, MBL);
        assert!(finish_rows(&summary(&skipped, "append_finish")).is_err());
    }
}
