//! Card MEMBER-RELEASE-UNTYPED-BASE
//! (`docs/reports/member-release-untyped-base/`).
//!
//! The base arm of W45-MEMBER-RELEASE-UNTYPED-DECLARATIONS. A release actual
//! `B->F` whose formal `B` is spelled through a typedef (`rd_kafka_toppar_t
//! *rktp`, `typedef struct rd_kafka_toppar_s rd_kafka_toppar_t;`; or `rp_t r`,
//! `typedef struct r *rp_t;`) is refused by the lexical recogniser as an
//! untyped base, so rule 1 never admitted it, whatever rule 5k's or rule 38j's
//! body check said. Rule 38j is amended again: at a nullable release of a
//! place rule 5k or rule 38j admitted, where the lexical refusal is the
//! untyped base, Clang's reading stands in when the actual is the
//! `LValueToRValue` value of an arrow member named `F` of the parameter `B`,
//! an object pointer with no `volatile`/`_Atomic`, and the member Clang
//! resolved is a member of a complete `struct` (never a `union`). Every other
//! premise is unchanged.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11, check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

/// `rd_free` and `mk_mem_free` inherit `libc::free` in their providers
/// (`librdkafka/memory.qv`, `monkey/memory.qv`); this standalone loader does
/// not resolve a provider `#include`, so the elaborated clauses, which are
/// `free`'s clause for clause, are stated here.
const WRAPPERS: &str = "module contract;\n\
extern fn rd_free(ptr: ptr<mut, u8>) -> unit\n\
contract {\n\
    requires resource_contract_v1;\n\
    requires resource_revision(flexible);\n\
    ensures normal resource_released(ptr, nullable);\n\
    ensures normal resource_return_none();\n\
};\n\
extern fn mk_mem_free(ptr: ptr<mut, u8>) -> unit\n\
contract {\n\
    requires resource_contract_v1;\n\
    requires resource_revision(flexible);\n\
    ensures normal resource_released(ptr, nullable);\n\
    ensures normal resource_return_none();\n\
};\n";

fn contracts() -> ResourceContracts {
    let files: BTreeMap<String, String> = [
        (
            "libc/memory.qv".to_owned(),
            include_str!("../../../providers/quirin-provider-libc/contracts/libc/memory.qv")
                .to_owned(),
        ),
        (
            "libc/stdio.qv".to_owned(),
            include_str!("../../../providers/quirin-provider-libc/contracts/libc/stdio.qv")
                .to_owned(),
        ),
        (
            "cfl/sds_lifetime.qv".to_owned(),
            include_str!("../../../providers/quirin-provider-cfl/contracts/cfl/sds_lifetime.qv")
                .to_owned(),
        ),
        ("test/wrappers.qv".to_owned(), WRAPPERS.to_owned()),
    ]
    .into_iter()
    .collect();
    load_resource_contract_sources(&files).expect("the modules load")
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent MEMBER-RELEASE-UNTYPED-BASE test: {reason}");
            false
        }
    }
}

const PRELUDE: &str = "#include <stdio.h>\n#include <stdlib.h>\n#include <string.h>\n\
void rd_free(void *ptr);\nvoid mk_mem_free(void *ptr);\n\
typedef char *sds_t;\nvoid cfl_sds_destroy(sds_t s);\n";

/// The entry's status and its diagnostics' descriptions; `None` when the
/// gateway rejects the unit.
fn analyse(entry: &str, body: &str) -> Option<(AnalysisStatus, Vec<String>)> {
    let text = format!("{PRELUDE}{body}\n");
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
        entry,
        &contracts(),
    )
    .ok()?;
    if let UnrestrictedLegacyResult::Safe(certificate) = &outcome {
        check_legacy_safety_certificate(certificate).expect("a Safe result rechecks");
    }
    let summary = match &outcome {
        UnrestrictedLegacyResult::Safe(certificate) => &certificate.summary,
        UnrestrictedLegacyResult::Unsafe(summary) => summary,
        UnrestrictedLegacyResult::Unknown { partial, .. } => {
            partial.as_ref().expect("a partial summary")
        }
    };
    let function = &summary.functions[entry];
    Some((
        function.status,
        function
            .diagnostics
            .iter()
            .map(|diagnostic| match &diagnostic.kind {
                LegacyDiagnosticKind::Unsupported { description } => description.clone(),
                other => format!("{other:?}"),
            })
            .collect(),
    ))
}

fn leaves(entry: &str, body: &str) -> (AnalysisStatus, Vec<String>) {
    analyse(entry, body).unwrap_or_else(|| panic!("fixture rejected: {entry}"))
}

fn named(base: &str, member: &str) -> String {
    format!(
        "release of `{base}->{member}`, a member of the formal `{base}` \
         (member-release-through-formal)"
    )
}

fn assert_safe(entry: &str, unit: &str) {
    let (status, found) = leaves(entry, unit);
    assert_eq!(status, AnalysisStatus::Safe, "{entry}: {found:?}");
}

fn assert_not_safe(entry: &str, unit: &str) -> Vec<String> {
    let (status, found) = leaves(entry, unit);
    assert_ne!(status, AnalysisStatus::Safe, "{entry}: {found:?}");
    found
}

/// `mk_core/mk_memory.c::mk_ptr_free` (Monkey), its record and body verbatim
/// but for whitespace: a base spelled through a typedef of an untagged
/// `struct`; rule 38j (the base is kept), the member left stale.
const MK_PTR: &str = "typedef struct\n{\n    char *data;\n    unsigned long len;\n} mk_ptr_t;\n";
const MK_PTR_FREE: &str =
    "void mk_ptr_free(mk_ptr_t * p)\n{\n    mk_mem_free(p->data);\n    p->len = 0;\n}\n";

/// `rdkafka_mock_sharegrp.c::rd_kafka_mock_sgrp_record_release`, its body
/// verbatim; the three records shortened to the members it reads, each spelled
/// as librdkafka does (a typedef of a tagged `struct` definition).
const SGRP: &str = "typedef enum { RD_KAFKA_MOCK_SGRP_RECORD_AVAILABLE, RD_KAFKA_MOCK_SGRP_RECORD_ACQUIRED, RD_KAFKA_MOCK_SGRP_RECORD_ARCHIVED } rd_kafka_mock_sgrp_record_state_type_t;\n\
typedef struct rd_kafka_mock_sharegroup_s { int max_delivery_attempts; } rd_kafka_mock_sharegroup_t;\n\
typedef struct rd_kafka_mock_sgrp_partmeta_s { int acquired_cnt; } rd_kafka_mock_sgrp_partmeta_t;\n\
typedef struct rd_kafka_mock_sgrp_record_state_s {\n  rd_kafka_mock_sgrp_record_state_type_t state;\n  int delivery_count;\n  char *owner_member_id;\n  long long lock_expiry_ts;\n} rd_kafka_mock_sgrp_record_state_t;\n\
void rd_kafka_mock_sgrp_record_release(\n    rd_kafka_mock_sharegroup_t *mshgrp,\n    rd_kafka_mock_sgrp_partmeta_t *pmeta,\n    rd_kafka_mock_sgrp_record_state_t *state) {\n        if (state->state == RD_KAFKA_MOCK_SGRP_RECORD_ACQUIRED)\n                pmeta->acquired_cnt--;\n        if (mshgrp->max_delivery_attempts > 0 &&\n            state->delivery_count >= mshgrp->max_delivery_attempts) {\n                state->state = RD_KAFKA_MOCK_SGRP_RECORD_ARCHIVED;\n        } else {\n                state->state = RD_KAFKA_MOCK_SGRP_RECORD_AVAILABLE;\n        }\n        rd_free(state->owner_member_id);\n        state->owner_member_id = NULL;\n        state->lock_expiry_ts  = 0;\n}\n";

/// `rdkafka_offset.c::rd_kafka_offset_file_close`, its body verbatim; the
/// record shortened to the member it reads (`FILE *rktp_offset_fp`).
fn offset_file_close(base: &str) -> String {
    format!(
        "typedef struct rd_kafka_toppar_s rd_kafka_toppar_t;\nstruct rd_kafka_toppar_s {{ int rktp_partition; FILE *rktp_offset_fp; }};\n\
         static void rd_kafka_offset_file_close({base}rktp) {{\n        if (!rktp->rktp_offset_fp)\n                return;\n\n        fclose(rktp->rktp_offset_fp);\n        rktp->rktp_offset_fp = NULL;\n}}\n\
         void offset_file_close_entry(rd_kafka_toppar_t *rktp) {{ rd_kafka_offset_file_close(rktp); }}\n"
    )
}

/// The three corpus bodies whose only leaves are rule-38i releases over a
/// typedef-spelled base. The two whose release is nullable are admitted; the
/// third releases through `fclose`, whose contract releases a **non-null**
/// stream, and rule 38j admits no `nonnull` member release whatever spells
/// the base: it stays refused, with the base spelled `struct T *` too.
#[test]
fn the_corpus_bodies_with_a_typedef_spelled_base() {
    if !clang_available() {
        return;
    }
    assert_safe("mk_ptr_free", &format!("{MK_PTR}{MK_PTR_FREE}"));
    assert_safe("rd_kafka_mock_sgrp_record_release", SGRP);
    for base in ["rd_kafka_toppar_t *", "struct rd_kafka_toppar_s *"] {
        let found = assert_not_safe("rd_kafka_offset_file_close", &offset_file_close(base));
        assert_eq!(found, [named("rktp", "rktp_offset_fp")], "{base}");
    }
}

/// The corpus's rule-5k destructors whose other leaf (`unresolved resource
/// identity` of the base) follows from the refused member release: their
/// bodies verbatim, their records shortened, each typedef spelled as
/// librdkafka spells it. `rd_kafka_Node_t` is declared twice, forward in
/// `rdkafka.h` and defining in `rdkafka_aux.h`, which the lexical reader reads
/// as disagreeing typedefs.
#[test]
fn the_corpus_destructors_that_release_the_base_too() {
    if !clang_available() {
        return;
    }
    let node = "typedef struct rd_kafka_Node_s rd_kafka_Node_t;\n\
         typedef struct rd_kafka_Node_s {\n  int id;\n  char *host;\n  unsigned short port;\n  char *rack;\n} rd_kafka_Node_t;\n\
         void rd_kafka_Node_destroy(rd_kafka_Node_t *node) {\n        rd_free(node->host);\n        if (node->rack)\n                rd_free(node->rack);\n        rd_free(node);\n}\n\
         void rd_kafka_Node_free(void *node) {\n        rd_kafka_Node_destroy((rd_kafka_Node_t *)node);\n}\n";
    assert_safe("rd_kafka_Node_destroy", node);
    // Its one-line wrapper (`rdkafka_aux.c`), in the same unit, reads the
    // destructor's published summary.
    assert_safe("rd_kafka_Node_free", node);
    assert_safe(
        "rd_kafka_mock_error_stack_destroy",
        "typedef struct rd_kafka_mock_error_stack_s {\n  int id;\n  int cnt;\n  int size;\n  int *errs;\n} rd_kafka_mock_error_stack_t;\n\
         static void\nrd_kafka_mock_error_stack_destroy(rd_kafka_mock_error_stack_t *errstack) {\n        if (errstack->errs)\n                rd_free(errstack->errs);\n        rd_free(errstack);\n}\n\
         void error_stack_entry(rd_kafka_mock_error_stack_t *e) { rd_kafka_mock_error_stack_destroy(e); }\n",
    );
    let entry = "typedef struct rd_kafka_share_ack_batch_entry_s {\n  long long start_offset;\n  long long end_offset;\n  int size;\n  int types_cnt;\n  int *types;\n} rd_kafka_share_ack_batch_entry_t;\n\
         void rd_kafka_share_ack_batch_entry_destroy(\n    rd_kafka_share_ack_batch_entry_t *entry) {\n        if (!entry)\n                return;\n        rd_free(entry->types);\n        rd_free(entry);\n}\n\
         static void rd_kafka_share_ack_batch_entry_destroy_free(void *ptr) {\n        rd_kafka_share_ack_batch_entry_destroy(\n            (rd_kafka_share_ack_batch_entry_t *)ptr);\n}\n\
         void (*entry_free_cb)(void *) = rd_kafka_share_ack_batch_entry_destroy_free;\n";
    assert_safe("rd_kafka_share_ack_batch_entry_destroy", entry);
    assert_safe("rd_kafka_share_ack_batch_entry_destroy_free", entry);
}

/// Rule 5k's shape over a typedef-spelled base (the parent released after its
/// members), and the spellings Clang resolves through any typedef chain.
#[test]
fn every_typedef_spelling_of_a_struct_pointer_base_is_admitted() {
    if !clang_available() {
        return;
    }
    let record = "typedef struct r { char *s; sds_t t; int n; } r_t;\n";
    // Rule 5k: `rd_kafka_mock_error_stack_destroy`'s shape.
    assert_safe(
        "error_stack_destroy",
        &format!(
            "{record}void error_stack_destroy(r_t *errstack) {{\n  if (errstack->s)\n    rd_free(errstack->s);\n  rd_free(errstack);\n}}\n"
        ),
    );
    // Rule 5k: `rd_kafka_Node_destroy`'s shape, two members then the parent.
    assert_safe(
        "node_destroy",
        &format!(
            "{record}void node_destroy(r_t *node) {{\n  rd_free(node->s);\n  if (node->t)\n    rd_free(node->t);\n  rd_free(node);\n}}\n"
        ),
    );
    // A typedef of the record pointer itself.
    assert_safe(
        "pointer_typedef",
        &format!(
            "{record}typedef struct r *rp_t;\nvoid pointer_typedef(rp_t r) {{\n  if (r->s) {{\n    free(r->s);\n  }}\n}}\n"
        ),
    );
    // A chain of typedefs, and a base and member both untyped lexically.
    assert_safe(
        "chain",
        &format!(
            "{record}typedef r_t r2_t;\nvoid chain(r2_t *r) {{\n  cfl_sds_destroy(r->t);\n  r->t = NULL;\n}}\n"
        ),
    );
}

/// Clang's reading of a typedef-spelled base stands in only for the value of
/// an object-pointer member of a complete `struct`: a union, an incomplete
/// record, an array or function-pointer member and a qualified pointer keep
/// their refusal.
#[test]
fn the_base_reading_admits_only_a_complete_struct() {
    if !clang_available() {
        return;
    }
    // A typedef of a union pointer, and of a union.
    let found = assert_not_safe(
        "union_pointer",
        "union u { sds_t s; int n; };\ntypedef union u *up_t;\nvoid union_pointer(up_t u) {\n  cfl_sds_destroy(u->s);\n}\n",
    );
    assert_eq!(found, [named("u", "s")]);
    let found = assert_not_safe(
        "union_typedef",
        "typedef union { char *s; int n; } u_t;\nvoid union_typedef(u_t *u) {\n  free(u->s);\n}\n",
    );
    assert_eq!(found, [named("u", "s")]);
    // A member of an anonymous union inside the struct is reached through
    // the union, not the parameter (`metric_view_destroy`'s protobuf-c shape).
    assert_not_safe(
        "anonymous_union",
        "typedef struct { int data_case; union { char *gauge; char *sum; }; } m_t;\nvoid anonymous_union(m_t *metric) {\n  free(metric->gauge);\n  free(metric);\n}\n",
    );
    // An incomplete record: Clang rejects `->` on it, so there is no reading
    // to stand in (the unit is rejected or refused, never `Safe`).
    if let Some((status, found)) = analyse(
        "incomplete",
        "typedef struct hidden hidden_t;\nvoid incomplete(hidden_t *h) {\n  free(h->s);\n}\n",
    ) {
        assert_ne!(status, AnalysisStatus::Safe, "{found:?}");
    }
    // An array-typedef member: Clang reads `r->buf` as its decay, an address.
    let found = assert_not_safe(
        "array_member",
        "typedef char name_t[16];\ntypedef struct r { name_t buf; } r_t;\nvoid array_member(r_t *r) {\n  free(r->buf);\n}\n",
    );
    assert_eq!(found, [named("r", "buf")]);
    // A function-pointer member.
    assert_not_safe(
        "hook_member",
        "typedef void (*hook_t)(void);\ntypedef struct r { hook_t cb; } r_t;\nvoid hook_member(r_t *r) {\n  free(r->cb);\n}\n",
    );
    // A volatile pointee, and a volatile pointer.
    assert_not_safe(
        "volatile_pointee",
        "typedef volatile struct r { char *s; } vr_t;\nvoid volatile_pointee(vr_t *r) {\n  free(r->s);\n}\n",
    );
    assert_not_safe(
        "volatile_pointer",
        "typedef struct r { char *s; } r_t;\nvoid volatile_pointer(r_t *volatile r) {\n  free(r->s);\n}\n",
    );
}

/// Every lexical premise of rules 5k and 38j is unchanged under the new
/// reading: a base that is a local, a released member used again, a second
/// release, a release in a loop, and a caller that uses what the callee
/// released all stay refused.
#[test]
fn the_lexical_premises_still_decide() {
    if !clang_available() {
        return;
    }
    let record = "typedef struct r { char *s; char *t; int n; } r_t;\nr_t *r_make(void);\n";
    // A local base, not the parameter.
    assert_not_safe(
        "local_base",
        &format!("{record}void local_base(r_t *p) {{\n  r_t *r = p;\n  free(r->s);\n}}\n"),
    );
    assert_not_safe(
        "local_made",
        &format!(
            "{record}void local_made(void) {{\n  r_t *r = r_make();\n  if (!r) return;\n  free(r->s);\n  free(r);\n}}\n"
        ),
    );
    // A released member used again, and released twice.
    assert_not_safe(
        "used_again",
        &format!("{record}void used_again(r_t *r) {{\n  free(r->s);\n  r->t = r->s;\n}}\n"),
    );
    let found = assert_not_safe(
        "twice",
        &format!("{record}void twice(r_t *r) {{\n  free(r->s);\n  free(r->s);\n}}\n"),
    );
    assert_eq!(found, [named("r", "s"), named("r", "s")]);
    // Released, then the parent read through after its own release.
    assert_not_safe(
        "parent_then_member",
        &format!("{record}void parent_then_member(r_t *r) {{\n  free(r);\n  free(r->s);\n}}\n"),
    );
    // A release in a loop.
    assert_not_safe(
        "in_loop",
        &format!(
            "{record}void in_loop(r_t *r, int n) {{\n  for (int i = 0; i < n; i++) {{\n    free(r->s);\n  }}\n}}\n"
        ),
    );
    // `mk_ptr_free`'s caller using the released member afterwards.
    let found = assert_not_safe(
        "reuse",
        &format!(
            "{MK_PTR}{MK_PTR_FREE}int reuse(mk_ptr_t *p) {{\n  mk_ptr_free(p);\n  return p->data != 0;\n}}\n"
        ),
    );
    assert!(
        found
            .iter()
            .any(|d| d.contains("member-effect-call-refused")),
        "{found:?}"
    );
}
