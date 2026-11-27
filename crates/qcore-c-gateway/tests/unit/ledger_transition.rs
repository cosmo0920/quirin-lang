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
