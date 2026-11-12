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

//! CFL-MPACK-EXTERNAL-DECLARATIONS — controls for the production declarations
//! this card appends to `cfl/sds_lifetime.qv` (quirin-provider-cfl) and
//! `mpack/writer.qv` (quirin-provider-mpack); report
//! `docs/reports/cfl-mpack-external-declarations/README.md`.
//!
//! Every declaration is loaded from its production module, byte for byte, and
//! exercised through the public gateway entry point on small held-out bodies
//! written the way the corpus calls each callee:
//!
//! * **accepting:** correct use is `Safe`;
//! * **owned results:** a constructor's result that no path releases is a
//!   leak candidate, and its NULL arm owes nothing;
//! * **refusing:** a misuse is a leak or violation candidate or a refusal,
//!   never `Safe` (an untested disposition, a borrowed result handed to a
//!   release or used after its parent's release, a bytes insert or append that
//!   may keep the caller's pointer);
//! * **withdrawal:** without the card's section every accepting body is
//!   refused at an uncontracted call again.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AbiOutcome, AbiRefusal, AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind,
    LegacyProgramSummary, ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate, declared_abi_census,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

#[path = "support/text.rs"]
mod text;
use text::normalize_newlines;

const LIBC_MEMORY: &str =
    include_str!("../../../providers/quirin-provider-libc/contracts/libc/memory.qv");
const MPACK_WRITER: &str =
    include_str!("../../../providers/quirin-provider-mpack/contracts/mpack/writer.qv");
const CFL_SDS: &str =
    include_str!("../../../providers/quirin-provider-cfl/contracts/cfl/sds_lifetime.qv");
const SECTION: &str = "// # Card CFL-MPACK-EXTERNAL-DECLARATIONS";

/// The upstream prototypes, as `cfl_*.h` and `mpack.h` spell them at the
/// audited revisions (the gateway compares each declaration against them).
const PRELUDE: &str = "\
typedef __SIZE_TYPE__ size_t;
typedef unsigned int uint32_t;
typedef int int32_t;
typedef __INT64_TYPE__ int64_t;
typedef __UINT64_TYPE__ uint64_t;
typedef char *cfl_sds_t;
struct cfl_kvlist; struct cfl_array; struct cfl_variant; struct cfl_list;
typedef struct mpack_writer_t mpack_writer_t;
typedef struct mpack_reader_t mpack_reader_t;
typedef enum mpack_type_t { mpack_type_missing = 0, mpack_type_nil, mpack_type_uint } mpack_type_t;
typedef enum mpack_error_t { mpack_ok = 0, mpack_error_io = 2, mpack_error_invalid } mpack_error_t;
typedef struct mpack_tag_t { mpack_type_t type; union { uint64_t u; int64_t i; double d; uint32_t n; uint32_t l; } v; } mpack_tag_t;
void *malloc(size_t size);
void free(void *ptr);
void cfl_sds_destroy(cfl_sds_t s);
size_t cfl_sds_len(cfl_sds_t s);
cfl_sds_t cfl_sds_create_len(const char *str, int len);
cfl_sds_t cfl_sds_create(const char *str);
cfl_sds_t cfl_sds_cat(cfl_sds_t s, const char *str, int len);
cfl_sds_t cfl_sds_create_size(size_t size);
void cfl_sds_set_len(cfl_sds_t s, size_t len);
void cfl_sds_len_set(cfl_sds_t s, size_t len);
cfl_sds_t cfl_sds_printf(cfl_sds_t *sds, const char *fmt, ...);
struct cfl_kvlist *cfl_kvlist_create();
void cfl_kvlist_destroy(struct cfl_kvlist *list);
int cfl_kvlist_insert_bytes(struct cfl_kvlist *list, char *key, char *value, size_t value_length, int referenced);
int cfl_kvlist_insert_uint64(struct cfl_kvlist *list, char *key, uint64_t value);
int cfl_kvlist_insert(struct cfl_kvlist *list, char *key, struct cfl_variant *value);
struct cfl_variant *cfl_kvlist_fetch(struct cfl_kvlist *list, char *key);
struct cfl_array *cfl_array_create(size_t slot_count);
void cfl_array_destroy(struct cfl_array *array);
int cfl_array_append(struct cfl_array *array, struct cfl_variant *value);
int cfl_array_append_string(struct cfl_array *array, char *value);
int cfl_array_append_bytes(struct cfl_array *array, char *value, size_t length, int referenced);
int cfl_array_append_bool(struct cfl_array *array, int value);
int cfl_array_append_int64(struct cfl_array *array, int64_t value);
int cfl_array_append_double(struct cfl_array *array, double value);
int cfl_array_append_array(struct cfl_array *array, struct cfl_array *value);
int cfl_array_append_kvlist(struct cfl_array *array, struct cfl_kvlist *value);
struct cfl_variant *cfl_variant_create_from_string(char *value);
struct cfl_variant *cfl_variant_create_from_bool(int value);
struct cfl_variant *cfl_variant_create_from_int64(int64_t value);
struct cfl_variant *cfl_variant_create_from_double(double value);
void cfl_variant_destroy(struct cfl_variant *instance);
struct cfl_list *cfl_utils_split(const char *line, int separator, int max_split);
void cfl_utils_split_free(struct cfl_list *list);
void mpack_writer_flag_error(mpack_writer_t* writer, mpack_error_t error);
void mpack_write_i32(mpack_writer_t* writer, int32_t value);
void mpack_write_u32(mpack_writer_t* writer, uint32_t value);
void mpack_write_u64(mpack_writer_t* writer, uint64_t value);
void mpack_write_nil(mpack_writer_t* writer);
void mpack_start_array(mpack_writer_t* writer, uint32_t count);
void mpack_start_map(mpack_writer_t* writer, uint32_t count);
void mpack_write_int(mpack_writer_t* writer, int64_t value);
void mpack_write_uint(mpack_writer_t* writer, uint64_t value);
void mpack_finish_map(mpack_writer_t* writer);
mpack_type_t mpack_tag_type(mpack_tag_t* tag);
int64_t mpack_tag_int_value(mpack_tag_t* tag);
uint64_t mpack_tag_uint_value(mpack_tag_t* tag);
double mpack_tag_double_value(mpack_tag_t* tag);
uint32_t mpack_tag_array_count(mpack_tag_t* tag);
uint32_t mpack_tag_map_count(mpack_tag_t* tag);
uint32_t mpack_tag_str_length(mpack_tag_t* tag);
uint32_t mpack_tag_bin_length(mpack_tag_t* tag);
void mpack_done_array(mpack_reader_t* reader);
void mpack_done_map(mpack_reader_t* reader);
void mpack_done_str(mpack_reader_t* reader);
void mpack_done_bin(mpack_reader_t* reader);
mpack_error_t mpack_reader_destroy(mpack_reader_t* reader);
void mpack_expect_nil(mpack_reader_t* reader);
void mpack_read_bytes(mpack_reader_t* reader, char* p, size_t count);
void mpack_read_cstr(mpack_reader_t* reader, char* buf, size_t buffer_size, size_t byte_count);
";

fn production() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([
        ("libc/memory.qv".to_owned(), normalize_newlines(LIBC_MEMORY)),
        (
            "mpack/writer.qv".to_owned(),
            normalize_newlines(MPACK_WRITER),
        ),
        (
            "cfl/sds_lifetime.qv".to_owned(),
            normalize_newlines(CFL_SDS),
        ),
    ]))
    .unwrap_or_else(|error| panic!("production contract text rejected: {error}"))
}

/// The same modules up to this card's section.
fn without_card() -> ResourceContracts {
    let cut = |text: &str| -> String {
        let text = normalize_newlines(text);
        let start = text.find(SECTION).expect("card section present");
        text[..start].to_owned()
    };
    load_resource_contract_sources(&BTreeMap::from([
        ("libc/memory.qv".to_owned(), normalize_newlines(LIBC_MEMORY)),
        ("mpack/writer.qv".to_owned(), cut(MPACK_WRITER)),
        ("cfl/sds_lifetime.qv".to_owned(), cut(CFL_SDS)),
    ]))
    .unwrap_or_else(|error| panic!("contract text rejected: {error}"))
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent CFL-MPACK-EXTERNAL-DECLARATIONS test: {reason}");
            false
        }
    }
}

#[derive(Debug)]
struct Row {
    status: AnalysisStatus,
    kinds: Vec<String>,
}

impl Row {
    fn safe(&self) -> bool {
        self.status == AnalysisStatus::Safe
    }
    fn mentions(&self, key: &str) -> bool {
        self.kinds.iter().any(|kind| kind.contains(key))
    }
    fn leaks(&self) -> usize {
        self.kinds
            .iter()
            .filter(|kind| kind.starts_with("Leak"))
            .count()
    }
}

fn summary_of(outcome: &UnrestrictedLegacyResult) -> Option<&LegacyProgramSummary> {
    match outcome {
        UnrestrictedLegacyResult::Safe(certificate) => Some(&certificate.summary),
        UnrestrictedLegacyResult::Unsafe(summary) => Some(summary),
        UnrestrictedLegacyResult::Unknown { partial, .. } => partial.as_ref(),
    }
}

/// The row of `entry` in the unit `PRELUDE + body`.
fn row_with(contracts: &ResourceContracts, body: &str, entry: &str) -> Row {
    let source = format!("{PRELUDE}\n{body}\n");
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        entry,
        contracts,
    )
    .unwrap_or_else(|error| panic!("fixture rejected by Clang: {error:?}\n{source}"));
    if let UnrestrictedLegacyResult::Safe(certificate) = &outcome {
        check_legacy_safety_certificate(certificate).expect("a Safe result rechecks");
    }
    let summary = summary_of(&outcome).unwrap_or_else(|| panic!("no summary: {outcome:?}"));
    let function = summary
        .functions
        .get(entry)
        .unwrap_or_else(|| panic!("missing function `{entry}`"));
    Row {
        status: function.status,
        kinds: function
            .diagnostics
            .iter()
            .map(|d| match &d.kind {
                LegacyDiagnosticKind::Unsupported { description } => description.clone(),
                other => format!("{other:?}"),
            })
            .collect(),
    }
}

fn row(body: &str, entry: &str) -> Row {
    row_with(&production(), body, entry)
}

/// One accepting body per group of declarations, each the way the corpus
/// calls the callee (the entry is the function named `a`).
const ACCEPTING: [(&str, &str); 16] = [
    (
        "sds constructors and length setters",
        "void a(const char *t) { cfl_sds_t s; cfl_sds_t u; cfl_sds_t v; \
         s = cfl_sds_create(t); if (!s) { return; } \
         u = cfl_sds_create_len(t, 3); if (!u) { cfl_sds_destroy(s); return; } \
         v = cfl_sds_create_size(64); if (!v) { cfl_sds_destroy(u); cfl_sds_destroy(s); return; } \
         cfl_sds_set_len(s, 0); cfl_sds_len_set(u, 1); \
         cfl_sds_destroy(v); cfl_sds_destroy(u); cfl_sds_destroy(s); }",
    ),
    (
        "cfl_sds_cat reallocates",
        "void a(void) { cfl_sds_t s; cfl_sds_t t; s = cfl_sds_create_size(4); if (!s) { return; } \
         t = cfl_sds_cat(s, \"abc\", 3); if (!t) { cfl_sds_destroy(s); return; } cfl_sds_destroy(t); }",
    ),
    (
        "cfl_sds_printf moves a local's slot",
        "void a(int n) { cfl_sds_t s; s = cfl_sds_create_size(8); if (!s) { return; } \
         cfl_sds_printf(&s, \"items\\n\"); cfl_sds_destroy(s); }",
    ),
    (
        "cfl_sds_printf passes a caller's slot through",
        "void a(cfl_sds_t *buf) { cfl_sds_printf(buf, \"  [spans]\\n\"); cfl_sds_printf(buf, \"done\\n\"); }",
    ),
    (
        "kvlist create, copying inserts and destroy",
        "int a(char *key, char *data, size_t len) { struct cfl_kvlist *l; l = cfl_kvlist_create(); if (!l) { return -1; } \
         cfl_kvlist_insert_uint64(l, key, 7); cfl_kvlist_insert_bytes(l, key, data, len, 0); \
         cfl_kvlist_destroy(l); return 0; }",
    ),
    (
        "kvlist fetch borrows from the list",
        "int a(struct cfl_kvlist *l) { struct cfl_variant *v; v = cfl_kvlist_fetch(l, \"key\"); if (v == 0) { return -1; } return 0; }",
    ),
    (
        "kvlist insert takes the variant on 0",
        "int a(struct cfl_kvlist *l, char *key) { struct cfl_variant *v; v = cfl_variant_create_from_int64(42); if (!v) { return -1; } \
         if (cfl_kvlist_insert(l, key, v) != 0) { cfl_variant_destroy(v); return -1; } return 0; }",
    ),
    (
        "array create, copying appends and destroy",
        "int a(char *text, char *data, size_t len) { struct cfl_array *arr; arr = cfl_array_create(8); if (!arr) { return -1; } \
         cfl_array_append_string(arr, text); cfl_array_append_bytes(arr, data, len, 0); cfl_array_append_bool(arr, 1); \
         cfl_array_append_int64(arr, -3); cfl_array_append_double(arr, 0.5); cfl_array_destroy(arr); return 0; }",
    ),
    (
        "array append takes the variant on 0",
        "int a(struct cfl_array *arr) { struct cfl_variant *v; v = cfl_variant_create_from_bool(1); if (!v) { return -1; } \
         if (cfl_array_append(arr, v) != 0) { cfl_variant_destroy(v); return -1; } return 0; }",
    ),
    (
        "array append_array and append_kvlist take the container on 0",
        "int a(struct cfl_array *arr) { struct cfl_array *in; struct cfl_kvlist *kv; \
         in = cfl_array_create(2); if (!in) { return -1; } \
         if (cfl_array_append_array(arr, in) != 0) { cfl_array_destroy(in); return -1; } \
         kv = cfl_kvlist_create(); if (!kv) { return -1; } \
         if (cfl_array_append_kvlist(arr, kv) != 0) { cfl_kvlist_destroy(kv); return -1; } return 0; }",
    ),
    (
        "variant constructors and destroy",
        "void a(char *text) { struct cfl_variant *s; struct cfl_variant *d; s = cfl_variant_create_from_string(text); if (!s) { return; } \
         d = cfl_variant_create_from_double(1.5); if (!d) { cfl_variant_destroy(s); return; } \
         cfl_variant_destroy(d); cfl_variant_destroy(s); }",
    ),
    (
        "split and its free",
        "int a(const char *line) { struct cfl_list *parts; parts = cfl_utils_split(line, ',', -1); if (!parts) { return -1; } \
         cfl_utils_split_free(parts); return 0; }",
    ),
    (
        "mpack writer primitives",
        "void a(mpack_writer_t *w, uint64_t u) { mpack_start_map(w, 2); mpack_write_nil(w); mpack_write_u64(w, u); \
         mpack_start_array(w, 3); mpack_write_u32(w, 1); mpack_write_i32(w, -1); mpack_write_int(w, -7); mpack_write_uint(w, u); \
         mpack_finish_map(w); }",
    ),
    (
        "mpack writer error flag",
        "void a(mpack_writer_t *w, int bad) { if (bad) { mpack_writer_flag_error(w, mpack_error_invalid); } }",
    ),
    (
        "mpack tag accessors",
        "uint64_t a(mpack_tag_t *t) { uint64_t n; if (mpack_tag_type(t) != mpack_type_uint) { return 0; } \
         n = mpack_tag_uint_value(t) + (uint64_t) mpack_tag_int_value(t) + mpack_tag_array_count(t) + mpack_tag_map_count(t) \
         + mpack_tag_str_length(t) + mpack_tag_bin_length(t); if (mpack_tag_double_value(t) > 0.0) { n++; } return n; }",
    ),
    (
        "mpack reader primitives",
        "int a(mpack_reader_t *r, char *buf, size_t n) { mpack_expect_nil(r); mpack_read_bytes(r, buf, n); \
         mpack_read_cstr(r, buf, n, n - 1); mpack_done_str(r); mpack_done_bin(r); mpack_done_array(r); mpack_done_map(r); \
         return mpack_reader_destroy(r) == mpack_ok ? 0 : -1; }",
    ),
];

/// The accepting rows whose callee's contract states an mpack enumeration
/// (`mpack_error_t`, `mpack_type_t`) as `u32`, each with that callee and the
/// refusal its declaration gets where the enumeration is `int` (Windows).
const ENUM_CONTRACTED: [(&str, &str, AbiRefusal); 3] = [
    (
        "mpack writer error flag",
        "mpack_writer_flag_error",
        AbiRefusal::ParameterTypeDisagreement,
    ),
    (
        "mpack tag accessors",
        "mpack_tag_type",
        AbiRefusal::ReturnTypeDisagreement,
    ),
    (
        "mpack reader primitives",
        "mpack_reader_destroy",
        AbiRefusal::ReturnTypeDisagreement,
    ),
];

/// The enumeration-typed contracts follow the target's enumeration ABI
/// (`abi_enum_const_pp` pins the probe's per-target answer): agreed where an
/// enumeration without a negative value is `unsigned int` (System V), and
/// withdrawn with the named disagreement where it is `int` (Windows).
#[test]
fn the_enumeration_contracts_follow_the_target_abi() {
    if !clang_available() {
        return;
    }
    let rows = declared_abi_census(PRELUDE, &production());
    for (name, function, refusal) in ENUM_CONTRACTED {
        let row = rows
            .iter()
            .find(|row| row.function == function)
            .unwrap_or_else(|| panic!("{name}: no census row for `{function}` in {rows:?}"));
        let expected = if cfg!(windows) {
            AbiOutcome::Withdrawn(refusal)
        } else {
            AbiOutcome::Agreed
        };
        assert_eq!(row.outcome, expected, "{name}: `{function}`");
    }
}

#[test]
fn every_declared_group_accepts_its_correct_use() {
    if !clang_available() {
        return;
    }
    for (name, body) in ACCEPTING {
        let row = row(body, "a");
        // Withdrawn on Windows (see the enumeration test above): the row
        // fails closed instead of reaching Safe.
        if cfg!(windows)
            && ENUM_CONTRACTED
                .iter()
                .any(|(row_name, ..)| *row_name == name)
        {
            assert_eq!(
                row.status,
                AnalysisStatus::Unknown,
                "{name}: expected Unknown on Windows, got {row:?}"
            );
            continue;
        }
        assert!(row.safe(), "{name}: expected Safe, got {row:?}");
    }
}

#[test]
fn without_the_card_every_accepting_body_is_refused_at_an_uncontracted_call() {
    if !clang_available() {
        return;
    }
    let bare = without_card();
    for (name, body) in ACCEPTING {
        let row = row_with(&bare, body, "a");
        assert!(!row.safe(), "{name}: must not be Safe without the card");
        assert!(
            row.mentions("uncontracted"),
            "{name}: expected an uncontracted call, got {row:?}"
        );
    }
}

#[test]
fn an_unreleased_owned_result_is_a_leak_and_its_null_arm_owes_nothing() {
    if !clang_available() {
        return;
    }
    let leaking = [
        "void l(const char *t) { cfl_sds_t s; s = cfl_sds_create(t); if (!s) { return; } cfl_sds_set_len(s, 0); }",
        "void l(void) { cfl_sds_t s; s = cfl_sds_create_size(16); if (!s) { return; } }",
        "void l(void) { struct cfl_kvlist *k; k = cfl_kvlist_create(); if (!k) { return; } cfl_kvlist_insert_uint64(k, \"n\", 1); }",
        "void l(void) { struct cfl_array *x; x = cfl_array_create(1); if (!x) { return; } cfl_array_append_bool(x, 0); }",
        "void l(void) { struct cfl_variant *v; v = cfl_variant_create_from_int64(1); if (!v) { return; } }",
        "void l(const char *t) { struct cfl_list *p; p = cfl_utils_split(t, ' ', 2); if (!p) { return; } }",
    ];
    for body in leaking {
        let row = row(body, "l");
        assert_eq!(row.status, AnalysisStatus::Unsafe, "{body}: {row:?}");
        assert_eq!(row.leaks(), 1, "{body}: {row:?}");
    }
}

#[test]
fn a_borrowed_position_of_these_callees_releases_nothing() {
    if !clang_available() {
        return;
    }
    // An owned string handed only to a borrowing callee still leaks.
    let bodies = [
        "void l(mpack_writer_t *w) { cfl_sds_t s; s = cfl_sds_create(\"x\"); if (!s) { return; } mpack_write_u64(w, cfl_sds_len(s)); }",
        "void l(struct cfl_array *arr) { cfl_sds_t s; s = cfl_sds_create(\"x\"); if (!s) { return; } cfl_array_append_string(arr, s); }",
    ];
    for body in bodies {
        let row = row(body, "l");
        assert_eq!(row.status, AnalysisStatus::Unsafe, "{body}: {row:?}");
        assert_eq!(row.leaks(), 1, "{body}: {row:?}");
    }
}

#[test]
fn an_untested_disposition_is_refused_never_safe() {
    if !clang_available() {
        return;
    }
    for callee in ["cfl_kvlist_insert(l, \"k\", v)", "cfl_array_append(x, v)"] {
        let body = format!(
            "void u(struct cfl_kvlist *l, struct cfl_array *x) {{ struct cfl_variant *v; v = cfl_variant_create_from_bool(0); if (!v) {{ return; }} {callee}; }}"
        );
        let row = row(&body, "u");
        assert!(!row.safe(), "{callee}: {row:?}");
    }
}

#[test]
fn a_fetched_variant_is_never_handed_to_a_custody_position() {
    if !clang_available() {
        return;
    }
    // Releasing a borrowed result, or inserting it into another list, is
    // refused, never accused: the list keeps it.
    for use_of in [
        "cfl_variant_destroy(v);",
        "if (cfl_kvlist_insert(o, \"k\", v) != 0) { return; }",
    ] {
        let body = format!(
            "void f(struct cfl_kvlist *l, struct cfl_kvlist *o) {{ struct cfl_variant *v; v = cfl_kvlist_fetch(l, \"k\"); if (!v) {{ return; }} {use_of} }}"
        );
        let row = row(&body, "f");
        assert_eq!(row.status, AnalysisStatus::Unknown, "{use_of}: {row:?}");
    }
}

#[test]
fn a_bytes_insert_or_append_that_may_keep_the_pointer_is_undeclared() {
    if !clang_available() {
        return;
    }
    // `referenced` other than the literal 0 is outside the premise: the
    // referenced arm keeps the caller's buffer, which no declaration states.
    for (call, callee) in [
        (
            "cfl_kvlist_insert_bytes(l, \"k\", d, 4, 1)",
            "cfl_kvlist_insert_bytes",
        ),
        (
            "cfl_kvlist_insert_bytes(l, \"k\", d, 4, r)",
            "cfl_kvlist_insert_bytes",
        ),
        (
            "cfl_array_append_bytes(x, d, 4, 1)",
            "cfl_array_append_bytes",
        ),
    ] {
        let body = format!(
            "int p(struct cfl_kvlist *l, struct cfl_array *x, char *d, int r) {{ return {call}; }}"
        );
        let row = row(&body, "p");
        assert_eq!(row.status, AnalysisStatus::Unknown, "{call}: {row:?}");
        assert!(
            row.mentions(&format!(
                "call to `{callee}` outside its declared literal-argument premise"
            )),
            "{call}: {row:?}"
        );
    }
}

#[test]
fn a_printf_with_trailing_arguments_is_outside_the_declaration() {
    if !clang_available() {
        return;
    }
    // The declaration names no tail (the loader refuses a tail beside a slot
    // move), so the arity gate refuses a call that passes one.
    let row = row(
        "void p(cfl_sds_t *buf, int n) { cfl_sds_printf(buf, \"%d\\n\", n); }",
        "p",
    );
    assert!(!row.safe(), "{row:?}");
}

#[test]
fn a_released_string_is_not_reallocated_or_printed_into() {
    if !clang_available() {
        return;
    }
    let bodies = [
        "void u(void) { cfl_sds_t s; cfl_sds_t t; s = cfl_sds_create(\"a\"); if (!s) { return; } cfl_sds_destroy(s); \
         t = cfl_sds_cat(s, \"b\", 1); if (t) { cfl_sds_destroy(t); } }",
        "void u(void) { cfl_sds_t s; s = cfl_sds_create(\"a\"); if (!s) { return; } cfl_sds_destroy(s); cfl_sds_set_len(s, 0); }",
    ];
    for body in bodies {
        let row = row(body, "u");
        assert!(!row.safe(), "{body}: {row:?}");
    }
}
