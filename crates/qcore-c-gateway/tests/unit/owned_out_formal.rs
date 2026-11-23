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

//! Unit pins for card OWNED-CONSTRUCTOR-OUT-PARAMETER-DELIVERY (rule 74l):
//! the out-mode vectors against `legacy_summary::out_mode`, the summary each
//! census shape derives, and the refusal's text in the reduced importer.

use std::collections::{BTreeMap, BTreeSet};

use qcore_schemas::contract::{ResourceContracts, ResourceKind, ResourceParameterMode};

use super::*;
use crate::legacy_c::{
    AnalysisStatus, ExitObservation, LegacyDiagnosticKind, LegacyProgramSummary, OutSlotValue,
    ReturnedValue, SiteDisposition, analyze_legacy_c_translation_unit, legacy_c_source_functions,
    tokenize,
};

const Q_BASE: &str = include_str!("../fixtures/contracts/q_base.qv");
const Q_NULLABLE: &str = include_str!("../fixtures/contracts/q_nullable.qv");

fn contracts() -> ResourceContracts {
    crate::contract_loader::load_resource_contract_sources(&BTreeMap::from([
        ("q_base.qcore".to_owned(), Q_BASE.to_owned()),
        ("q_nullable.qcore".to_owned(), Q_NULLABLE.to_owned()),
    ]))
    .unwrap()
}

fn slot(spelling: &str) -> OutSlotValue {
    let site = |disposition, nullable| OutSlotValue::Site {
        site: 7,
        nullable,
        disposition,
        kind: ResourceKind::Heap,
    };
    match spelling {
        "unwritten" => OutSlotValue::Unwritten,
        "null" => OutSlotValue::Null,
        "ambiguous" => OutSlotValue::Ambiguous,
        "site-live" => site(SiteDisposition::Live, false),
        "site-live-nullable" => site(SiteDisposition::Live, true),
        "site-released" => site(SiteDisposition::Released, false),
        "site-escaped" => site(SiteDisposition::Escaped, false),
        "site-contained" => site(SiteDisposition::Contained, false),
        other => panic!("unknown exit spelling {other}"),
    }
}

fn exit(value: OutSlotValue) -> ExitObservation {
    ExitObservation {
        parameters: vec![None],
        returned: ReturnedValue::None,
        out_slots: vec![Some(value)],
        caller_containment: BTreeMap::new(),
        pending_replacements: BTreeMap::new(),
        derived_base_touched: false,
        selected: None,
    }
}

#[test]
fn out_mode_vectors_hold() {
    for (exits, expected) in OUT_MODE_VECTORS {
        let observations = exits
            .iter()
            .map(|spelling| exit(slot(spelling)))
            .collect::<Vec<_>>();
        let read = crate::legacy_summary::out_mode(0, &observations).map(|mode| match mode {
            ResourceParameterMode::Out { nullable: false } => "out",
            ResourceParameterMode::Out { nullable: true } => "out-nullable",
            other => panic!("out_mode read {other:?}"),
        });
        assert_eq!(read, *expected, "{exits:?}");
    }
}

/// A non-heap delivery is never inferred (W43-DESCRIPTOR-RESOURCE-TRACKING);
/// the Lean vectors are heap-kinded, as the probe's slot is.
#[test]
fn a_non_heap_delivery_reads_no_mode() {
    let observations = [exit(OutSlotValue::Site {
        site: 7,
        nullable: false,
        disposition: SiteDisposition::Live,
        kind: ResourceKind::Descriptor,
    })];
    assert_eq!(crate::legacy_summary::out_mode(0, &observations), None);
}

fn summaries(source: &str) -> BTreeMap<String, ResourceParameterMode> {
    let tokens = tokenize(source);
    let names = legacy_c_source_functions(source);
    crate::legacy_summary::derive_custody_summaries(&tokens, &names, &contracts())
        .records()
        .into_iter()
        .filter_map(|record| Some((record.function, *record.contract.parameters.first()?)))
        .collect()
}

const DECLARATIONS: &str = "void *q_alloc(void); void q_free(void *p); void *q_maybe_alloc(void);\n";

/// The sound shape publishes `Out`; every refused shape publishes nothing, so
/// a caller keeps `call to uncontracted function`.
#[test]
fn only_an_every_exit_delivery_publishes_an_out_summary() {
    let delivered = summaries(&format!(
        "{DECLARATIONS}void fill(void **out) {{ *out = q_alloc(); }}"
    ));
    assert_eq!(
        delivered.get("fill"),
        Some(&ResourceParameterMode::Out { nullable: false })
    );
    for body in [
        // Partial path.
        "int fill(void **out, int c) { if (c) *out = q_alloc(); return c; }",
        // In-out slot (rd_kafka_handle_OffsetFetch).
        "void fill(void **out) { if (!*out) *out = q_alloc(); }",
        // Store then release.
        "void fill(void **out) { void *p = q_alloc(); *out = p; q_free(p); }",
        // Second store over the first.
        "void fill(void **out) { *out = q_alloc(); *out = q_alloc(); }",
        // Possibly-null result.
        "void fill(void **out) { *out = q_maybe_alloc(); }",
    ] {
        let derived = summaries(&format!("{DECLARATIONS}{body}"));
        assert_eq!(derived.get("fill"), None, "{body}");
    }
}

fn unsupported(summary: &LegacyProgramSummary, function: &str) -> BTreeSet<String> {
    summary.functions[function]
        .diagnostics
        .iter()
        .filter_map(|diagnostic| match &diagnostic.kind {
            LegacyDiagnosticKind::Unsupported { description } => Some(description.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn an_unsanctioned_slot_store_is_refused_by_name_not_as_a_discard() {
    let source = format!(
        "{DECLARATIONS}void subject(void **out, int c) {{ if (c) *out = q_alloc(); }}"
    );
    let summary = analyze_legacy_c_translation_unit(&source, "subject", &contracts()).unwrap();
    assert_eq!(summary.functions["subject"].status, AnalysisStatus::Unknown);
    let refused = unsupported(&summary, "subject");
    assert!(
        refused.contains(
            "owned result of resource API `q_alloc` stored through `*out` without an out-parameter contract"
        ),
        "{refused:?}"
    );
    assert!(
        refused.iter().all(|text| !text.starts_with("discarded owned result")),
        "{refused:?}"
    );
}

#[test]
fn a_real_discard_keeps_its_text() {
    let source = format!("{DECLARATIONS}void subject(void) {{ q_alloc(); }}");
    let summary = analyze_legacy_c_translation_unit(&source, "subject", &contracts()).unwrap();
    assert_eq!(summary.functions["subject"].status, AnalysisStatus::Unknown);
    assert!(unsupported(&summary, "subject")
        .contains("discarded owned result of resource API `q_alloc`"));
}

#[test]
fn the_refusal_text_names_the_callee_and_the_slot() {
    assert_eq!(
        unsanctioned_owned_store("rd_kafka_topic_partition_list_new", "*partitions"),
        "owned result of resource API `rd_kafka_topic_partition_list_new` stored through \
         `*partitions` without an out-parameter contract"
    );
}
