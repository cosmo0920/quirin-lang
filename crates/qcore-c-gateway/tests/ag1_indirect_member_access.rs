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

//! Card AG1-INDIRECT-MEMBER-ACCESS (gateway rule 61 (1)): a local aggregate
//! pointer spelled through a struct alias is the struct pointer for the
//! aggregate store's premises (`docs/reports/ag1-indirect-member-access/`).
//!
//! Each admitted fixture is a census shape of the user's 046accf6 `current`
//! run (`rd_kafka_Node_new`, `rd_kafka_topic_partition_new`,
//! `rd_kafka_UserScramCredentialDeletion_new`): a constructor that allocates
//! its record, stores owned results into its members and returns it. The
//! near-misses keep a non-`Safe` answer: a leak through a released owner, an
//! aliased base, a release before the store, an overwrite in a loop, a
//! disagreeing or union alias, a formal base, two declarators, and a store
//! through a second path.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11, check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const GATE: &str = "indirect memory access is outside the resource projection";

fn contracts() -> ResourceContracts {
    let files: BTreeMap<String, String> = [
        (
            "libc/memory.qv".to_owned(),
            include_str!("../../../providers/quirin-provider-libc/contracts/libc/memory.qv")
                .to_owned(),
        ),
        (
            "libc/string.qv".to_owned(),
            include_str!("../../../providers/quirin-provider-libc/contracts/libc/string.qv")
                .to_owned(),
        ),
    ]
    .into_iter()
    .collect();
    load_resource_contract_sources(&files).expect("the libc modules load")
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent AG1-INDIRECT-MEMBER-ACCESS test: {reason}");
            false
        }
    }
}

/// The entry's status and its diagnostics' descriptions.
// Target-portable spellings: plain `long` (and a typedef of it) is unreadable to
// the declared-ABI reader unless the analysing Clang is LP64 (rule 11q), and
// the UCRT's headers are unread on Windows, so these synthetic declarations
// spell sizes `__SIZE_TYPE__`/`__PTRDIFF_TYPE__` and 64-bit `long long`.
fn row(entry: &str, source: &str) -> (AnalysisStatus, Vec<String>) {
    let text = format!(
        "#define NULL ((void *)0)\nvoid *calloc(__SIZE_TYPE__ count, __SIZE_TYPE__ size);\nvoid free(void *ptr);\nextern char *strdup(const char *s);\n{source}\n"
    );
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
        entry,
        &contracts(),
    )
    .unwrap_or_else(|error| panic!("fixture rejected: {error:?}"));
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
    (
        function.status,
        function
            .diagnostics
            .iter()
            .map(|diagnostic| match &diagnostic.kind {
                LegacyDiagnosticKind::Unsupported { description } => description.clone(),
                other => format!("{other:?}"),
            })
            .collect(),
    )
}

fn assert_safe(entry: &str, source: &str) {
    let (status, diagnostics) = row(entry, source);
    assert_eq!(status, AnalysisStatus::Safe, "{entry}: {diagnostics:?}");
    assert!(diagnostics.is_empty(), "{entry}: {diagnostics:?}");
}

fn assert_not_safe(entry: &str, source: &str) -> Vec<String> {
    let (status, diagnostics) = row(entry, source);
    assert_ne!(status, AnalysisStatus::Safe, "{entry}: {diagnostics:?}");
    diagnostics
}

fn assert_gate_refusal(entry: &str, source: &str) {
    let diagnostics = assert_not_safe(entry, source);
    assert!(
        diagnostics.iter().any(|d| d == GATE),
        "{entry}: {diagnostics:?}"
    );
}

const DEFINING: &str = "typedef struct node_s { int id; char *host; char *rack; } node_t;\n";
const FORWARD_AND_DEFINING: &str = "typedef struct node_s node_t;\ntypedef struct node_s { int id; char *host; char *rack; } node_t;\n";

#[test]
fn a_constructor_spelled_through_a_defining_alias_is_safe() {
    if !clang_available() {
        return;
    }
    // `rd_kafka_topic_partition_new`: one defining typedef.
    assert_safe(
        "node_new",
        &format!(
            "{DEFINING}node_t *node_new(int id, const char *host) {{\n  node_t *ret = calloc(1, sizeof(*ret));\n  if (ret == NULL) return NULL;\n  ret->id = id;\n  ret->host = strdup(host);\n  return ret;\n}}"
        ),
    );
}

#[test]
fn a_constructor_spelled_through_a_forward_and_a_defining_alias_is_safe() {
    if !clang_available() {
        return;
    }
    // `rd_kafka_Node_new`: `rdkafka.h` forward-declares the alias and
    // `rdkafka_aux.h` defines it again with the struct body; `sizeof(*ret)`
    // is an unevaluated operand of a struct type.
    assert_safe(
        "node_new",
        &format!(
            "{FORWARD_AND_DEFINING}node_t *node_new(int id, const char *host) {{\n  node_t *ret = calloc(1, sizeof(*ret));\n  if (ret == NULL) return NULL;\n  ret->id = id;\n  ret->host = strdup(host);\n  return ret;\n}}"
        ),
    );
    // `rd_kafka_Node_new` itself also stores `rack` on one arm only: the
    // store is now read, and the join's partial link is rule 11c's refusal
    // at the escape (a re-attribution, never an admission).
    let diagnostics = assert_not_safe(
        "node_new_rack",
        &format!(
            "{FORWARD_AND_DEFINING}node_t *node_new_rack(int id, const char *host, const char *rack) {{\n  node_t *ret = calloc(1, sizeof(*ret));\n  if (ret == NULL) return NULL;\n  ret->id = id;\n  ret->host = strdup(host);\n  if (rack != NULL)\n    ret->rack = strdup(rack);\n  return ret;\n}}"
        ),
    );
    assert!(
        diagnostics.iter().all(|d| d != GATE)
            && diagnostics
                .iter()
                .any(|d| d.starts_with("field-link escape")),
        "{diagnostics:?}"
    );
    // A separate declaration and binding (`rd_kafka_UserScramCredentialDeletion_new`).
    assert_safe(
        "node_new2",
        &format!(
            "{FORWARD_AND_DEFINING}node_t *node_new2(const char *host) {{\n  node_t *ret;\n  ret = calloc(1, sizeof(*ret));\n  if (ret == NULL) return NULL;\n  ret->host = strdup(host);\n  ret->id = 0;\n  return ret;\n}}"
        ),
    );
}

#[test]
fn a_released_owner_still_leaks_its_linked_child() {
    if !clang_available() {
        return;
    }
    // P-B08-SHALLOW: releasing the record does not release the stored child.
    assert_not_safe(
        "node_leak",
        &format!(
            "{DEFINING}int node_leak(const char *host) {{\n  node_t *ret = calloc(1, sizeof(*ret));\n  if (ret == NULL) return -1;\n  ret->host = strdup(host);\n  free(ret);\n  return 0;\n}}"
        ),
    );
    // Both released: safe, as for the keyword spelling.
    assert_safe(
        "node_both",
        &format!(
            "{DEFINING}int node_both(const char *host) {{\n  node_t *ret = calloc(1, sizeof(*ret));\n  if (ret == NULL) return -1;\n  ret->host = strdup(host);\n  free(ret->host);\n  free(ret);\n  return 0;\n}}"
        ),
    );
}

#[test]
fn every_other_store_premise_still_decides() {
    if !clang_available() {
        return;
    }
    // An aliased base (premise 5).
    assert_gate_refusal(
        "node_alias",
        &format!(
            "{DEFINING}node_t *node_alias(const char *host) {{\n  node_t *ret = calloc(1, sizeof(*ret));\n  node_t *other;\n  if (ret == NULL) return NULL;\n  other = ret;\n  ret->host = strdup(host);\n  return other;\n}}"
        ),
    );
    // A release before the store: no live owner.
    assert_not_safe(
        "node_freed",
        &format!(
            "{DEFINING}int node_freed(const char *host) {{\n  node_t *ret = calloc(1, sizeof(*ret));\n  if (ret == NULL) return -1;\n  free(ret);\n  ret->host = strdup(host);\n  return 0;\n}}"
        ),
    );
    // An overwrite in a loop: the earlier child is never released.
    assert_not_safe(
        "node_loop",
        &format!(
            "{DEFINING}node_t *node_loop(const char *host, int n) {{\n  int i;\n  node_t *ret = calloc(1, sizeof(*ret));\n  if (ret == NULL) return NULL;\n  for (i = 0; i < n; i++)\n    ret->host = strdup(host);\n  return ret;\n}}"
        ),
    );
    // A store through a second path to the member.
    assert_not_safe(
        "node_second_path",
        &format!(
            "{DEFINING}node_t *node_second_path(const char *host) {{\n  char **slot;\n  node_t *ret = calloc(1, sizeof(*ret));\n  if (ret == NULL) return NULL;\n  slot = &ret->host;\n  *slot = strdup(host);\n  ret->rack = strdup(host);\n  return ret;\n}}"
        ),
    );
}

#[test]
fn an_alias_that_does_not_name_one_struct_keeps_the_refusal() {
    if !clang_available() {
        return;
    }
    // Two declarations of the alias naming different structs cannot compile;
    // an alias of a union and an alias of a pointer type are the near-misses.
    assert_gate_refusal(
        "union_new",
        "typedef union u_s { int id; char *host; } u_t;\nu_t *union_new(const char *host) {\n  u_t *ret = calloc(1, sizeof(*ret));\n  if (ret == NULL) return NULL;\n  ret->host = strdup(host);\n  return ret;\n}",
    );
    assert_gate_refusal(
        "pointer_alias",
        "struct p_s { int id; char *host; };\ntypedef struct p_s *p_t;\np_t pointer_alias(const char *host) {\n  p_t ret = calloc(1, sizeof(struct p_s));\n  if (ret == NULL) return NULL;\n  ret->host = strdup(host);\n  return ret;\n}",
    );
    // Two declarators in one declaration.
    assert_gate_refusal(
        "two_declarators",
        &format!(
            "{DEFINING}node_t *two_declarators(const char *host) {{\n  node_t *ret = calloc(1, sizeof(*ret)), *spare = NULL;\n  if (ret == NULL) return spare;\n  ret->host = strdup(host);\n  return ret;\n}}"
        ),
    );
}

#[test]
fn a_formal_base_keeps_the_member_store_through_formal_name() {
    if !clang_available() {
        return;
    }
    let diagnostics = assert_not_safe(
        "node_set",
        &format!(
            "{DEFINING}void node_set(node_t *n, const char *host) {{\n  n->host = strdup(host);\n}}"
        ),
    );
    assert!(
        diagnostics.iter().all(|d| d != GATE),
        "the store keeps rule 38i's name, not a new admission: {diagnostics:?}"
    );
}
