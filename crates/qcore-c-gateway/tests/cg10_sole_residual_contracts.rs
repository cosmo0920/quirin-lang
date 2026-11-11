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

//! CG10-INDIRECT-ACCESS-SOLE-RESIDUALS — controls for the nine declarations
//! this card lands (`docs/reports/cg10-indirect-access-sole-residuals/README.md`),
//! and the two it deliberately does not (`rd_atomic32_add`/`_set`, which back
//! librdkafka's reference counts).
//!
//! Each section is taken from its production provider module byte for byte
//! (from the card's section marker to the end of the module; the modules'
//! libc parents are not resolved by this standalone loader) and exercised on
//! small held-out bodies through the public gateway entry point:
//!
//! * **accepting:** a member actual handed to a declared borrowed position is
//!   read by rule W44 and the function is `Safe`;
//! * **withdrawal:** without the card's sections the same statement is the
//!   gate's refusal re-attributed by rule 12k to the uncontracted callee;
//! * **refusing:** a borrowed position takes no custody, so an owned buffer
//!   handed to a declared helper and dropped still leaks;
//! * **the header body:** `rd_avg_add`'s shape (`src/rdavg.h:77-93`) with its
//!   one call, `rd_hdr_histogram_record`, declared: a W4 summary of the body
//!   validates and the caller handing `&b->avg` is `Safe`.
//!
//! The fixtures are not claims about any upstream body.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

#[path = "support/text.rs"]
mod text;
use text::normalize_newlines;

const LIBC_MEMORY: &str =
    include_str!("../../../providers/quirin-provider-libc/contracts/libc/memory.qv");
const RDKAFKA_MEMORY: &str =
    include_str!("../../../providers/qcore-provider-librdkafka/contracts/librdkafka/memory.qv");
const RDKAFKA_SECTION: &str =
    "// ## Card CG10-INDIRECT-ACCESS-SOLE-RESIDUALS: custody-neutral scalar helpers";
const CMETRICS_VALUES: &str =
    include_str!("../../../providers/qcore-provider-cmetrics/contracts/cmetrics/metric_values.qv");
const CMETRICS_SECTION: &str =
    "// Card CG10-INDIRECT-ACCESS-SOLE-RESIDUALS (wave 48, round 18): the three";

const DECLARATIONS: &str = "\
typedef __SIZE_TYPE__ size_t; \
typedef int int32_t; typedef long long int64_t; typedef unsigned long long uint64_t; \
typedef struct { volatile int32_t val; } rd_atomic32_t; \
typedef struct { volatile int64_t val; } rd_atomic64_t; \
typedef struct cnd_s cnd_t; typedef struct mtx_s mtx_t; \
typedef struct rd_hdr_histogram_s rd_hdr_histogram_t; \
void *malloc(size_t size); \
void free(void *ptr); \
int32_t rd_atomic32_add(rd_atomic32_t *ra, int32_t v); \
int32_t rd_atomic32_set(rd_atomic32_t *ra, int32_t v); \
int64_t rd_atomic64_add(rd_atomic64_t *ra, int64_t v); \
int64_t rd_atomic64_get(rd_atomic64_t *ra); \
int cnd_timedwait_ms(cnd_t *cnd, mtx_t *mtx, int timeout_ms); \
int rd_hdr_histogram_record(rd_hdr_histogram_t *hdr, int64_t v); \
uint64_t cmt_atomic_load(uint64_t *storage); \
void cmt_atomic_store(uint64_t *storage, uint64_t new_value); \
int cmt_atomic_compare_exchange(uint64_t *storage, uint64_t old_value, uint64_t new_value); \
struct avg { int64_t sum; int64_t cnt; rd_hdr_histogram_t *hdr; }; \
struct k { rd_atomic32_t n32; rd_atomic64_t n64; cnd_t *cnd; mtx_t *lock; struct avg avg; }; \
struct m { uint64_t val; uint64_t count; uint64_t *buckets; };\n";

const UNCONTRACTED: &str = "call to uncontracted function `";

fn section(text: &str, marker: &str) -> String {
    let text = normalize_newlines(text);
    let start = text
        .find(marker)
        .expect("the card's section is in the module");
    format!("module contract;\n{}", &text[start..])
}

fn production() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([
        ("libc/memory.qv".to_owned(), normalize_newlines(LIBC_MEMORY)),
        (
            "librdkafka/cg10_scalar_helpers.qcore".to_owned(),
            section(RDKAFKA_MEMORY, RDKAFKA_SECTION),
        ),
        (
            "cmetrics/cg10_atomic.qcore".to_owned(),
            section(CMETRICS_VALUES, CMETRICS_SECTION),
        ),
    ]))
    .unwrap_or_else(|error| panic!("production contract text rejected: {error}"))
}

/// Only libc's allocator: the card's declarations are absent.
fn without_card() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "libc/memory.qv".to_owned(),
        normalize_newlines(LIBC_MEMORY),
    )]))
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
            eprintln!("skipping Clang-dependent control: {reason}");
            false
        }
    }
}

#[derive(Debug)]
struct Run {
    status: AnalysisStatus,
    kinds: Vec<LegacyDiagnosticKind>,
}

fn run(source: &str, contracts: &ResourceContracts) -> Run {
    let text = format!("{DECLARATIONS}\n{source}\n");
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
        "subject",
        contracts,
    )
    .unwrap_or_else(|error| panic!("fixture rejected by Clang: {error:?}\n{text}"));
    let summary = match &outcome {
        UnrestrictedLegacyResult::Safe(certificate) => Some(&certificate.summary),
        UnrestrictedLegacyResult::Unsafe(summary) => Some(summary),
        UnrestrictedLegacyResult::Unknown { partial, .. } => partial.as_ref(),
    };
    let result = summary
        .and_then(|summary| summary.functions.get("subject"))
        .unwrap_or_else(|| panic!("`subject` projected: {outcome:?}"));
    Run {
        status: result.status,
        kinds: result
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.kind.clone())
            .collect(),
    }
}

const BODIES: &[&str] = &[
    "long long subject(struct k *k) { rd_atomic64_add(&k->n64, 5); return rd_atomic64_get(&k->n64); }",
    "int subject(struct k *k) { return cnd_timedwait_ms(k->cnd, k->lock, 10); }",
    "int subject(struct k *k) { return rd_hdr_histogram_record(k->avg.hdr, 3); }",
    "unsigned long long subject(struct m *m) { cmt_atomic_store(&m->val, 1); \
     return cmt_atomic_load(&m->count); }",
    "int subject(struct m *m) { return cmt_atomic_compare_exchange(&m->val, 0, 1); }",
];

#[test]
fn each_declared_helper_reads_its_member_actuals() {
    if !clang_available() {
        return;
    }
    for body in BODIES {
        let run = run(body, &production());
        assert_eq!(run.status, AnalysisStatus::Safe, "{body}\n{run:?}");
    }
}

#[test]
fn without_the_card_each_statement_names_its_uncontracted_callee() {
    if !clang_available() {
        return;
    }
    for body in BODIES {
        let run = run(body, &without_card());
        assert_eq!(run.status, AnalysisStatus::Unknown, "{body}\n{run:?}");
        assert!(
            run.kinds.iter().all(|kind| matches!(kind,
                LegacyDiagnosticKind::Unsupported { description }
                    if description.starts_with(UNCONTRACTED))),
            "{body}\n{run:?}"
        );
    }
}

#[test]
fn a_borrowed_position_takes_no_custody() {
    if !clang_available() {
        return;
    }
    // An owned buffer handed to a declared helper and dropped still leaks.
    let run = run(
        "void subject(void) { uint64_t *p = malloc(8); if (!p) return; cmt_atomic_store(p, 1); }",
        &production(),
    );
    assert_eq!(run.status, AnalysisStatus::Unsafe, "{run:?}");
}

#[test]
fn the_reference_count_wrappers_stay_undeclared() {
    if !clang_available() {
        return;
    }
    // `rd_refcnt_add0(R)` is `rd_atomic32_add(R, 1)`: declaring it would read
    // a reference `keep` as custody-neutral (decision class 3).
    let run = run(
        "int subject(struct k *k) { return rd_atomic32_add(&k->n32, 1); }",
        &production(),
    );
    assert_eq!(run.status, AnalysisStatus::Unknown, "{run:?}");
    assert!(
        run.kinds.iter().any(|kind| matches!(kind,
            LegacyDiagnosticKind::Unsupported { description }
                if description == "call to uncontracted function `rd_atomic32_add`")),
        "{run:?}"
    );
}

#[test]
fn the_rd_avg_add_body_validates_with_its_histogram_call_declared() {
    if !clang_available() {
        return;
    }
    let source = "\
static void rd_avg_add(struct avg *ra, int64_t v) { \
  ra->sum += v; ra->cnt++; rd_hdr_histogram_record(ra->hdr, v); } \
void subject(struct k *k) { rd_avg_add(&k->avg, 7); }";
    let run_with = run(source, &production());
    assert_eq!(run_with.status, AnalysisStatus::Safe, "{run_with:?}");
    let run_without = run(source, &without_card());
    assert_eq!(
        run_without.status,
        AnalysisStatus::Unknown,
        "{run_without:?}"
    );
}
