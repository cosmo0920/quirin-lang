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

//! W46 routing query premises: acceptance, withdrawal, and retained custody.
use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyFunctionSummary,
    ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
};
use qcore_contract::contract_loader::load_resource_contract_sources;
use qcore_schemas::contract::{ResourceParameterMode, ResourceReturnMode};

const CHUNK: &str = include_str!(
    "../../../providers/quirin-provider-fluent-bit/contracts/fluent_bit/input_chunk_is_up.qv"
);
const MASK: &str = include_str!(
    "../../../providers/quirin-provider-fluent-bit/contracts/fluent_bit/routes_mask.qv"
);
const MEMORY: &str =
    include_str!("../../../providers/quirin-provider-libc/contracts/libc/memory.qv");
const CHUNK_MARKER: &str = "// W46-FLUENT-ROUTING-QUERY-CONTRACTS.";
const MASK_MARKER: &str = "// W46-FLUENT-ROUTING-QUERY-CONTRACTS.";

fn contracts(with_card: bool) -> ResourceContracts {
    let select = |text: &str, marker: &str| {
        if with_card {
            text
        } else {
            text.split_once(marker).unwrap().0
        }
        .to_owned()
    };
    load_resource_contract_sources(&BTreeMap::from([
        (
            "fluent_bit/input_chunk_is_up.qv".to_owned(),
            select(CHUNK, CHUNK_MARKER),
        ),
        (
            "fluent_bit/routes_mask.qv".to_owned(),
            select(MASK, MASK_MARKER),
        ),
        ("libc/memory.qv".to_owned(), MEMORY.to_owned()),
    ]))
    .expect("production modules load")
}

fn summary(body: &str, with_card: bool) -> LegacyFunctionSummary {
    let source = format!(
        "typedef __SIZE_TYPE__ size_t; struct flb_input_instance; struct flb_router; \
         int input_chunk_has_magic_bytes(char *, int); \
         int flb_input_chunk_is_mem_overlimit(struct flb_input_instance *); \
         int flb_input_chunk_is_storage_overlimit(struct flb_input_instance *); \
         size_t flb_routes_mask_get_slots(struct flb_router *); \
         int flb_routes_mask_is_empty(unsigned long *, struct flb_router *); \
         void *malloc(size_t); void free(void *); {body}"
    );
    let result = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        "test",
        &contracts(with_card),
    )
    .expect("fixture compiles");
    let program = match &result {
        UnrestrictedLegacyResult::Safe(cert) => &cert.summary,
        UnrestrictedLegacyResult::Unsafe(program)
        | UnrestrictedLegacyResult::Unknown {
            partial: Some(program),
            ..
        } => program,
        other => panic!("no program summary: {other:?}"),
    };
    program.functions["test"].clone()
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(std::env::var_os("QCORE_CLANG").is_none(), "{reason}");
            false
        }
    }
}

#[test]
fn declarations_are_borrowed_scalar_queries() {
    let loaded = contracts(true);
    for (name, arity) in [
        ("input_chunk_has_magic_bytes", 2),
        ("flb_input_chunk_is_mem_overlimit", 1),
        ("flb_input_chunk_is_storage_overlimit", 1),
        ("flb_routes_mask_get_slots", 1),
        ("flb_routes_mask_is_empty", 2),
    ] {
        let contract = loaded
            .contract(name)
            .unwrap_or_else(|| panic!("missing {name}"));
        assert_eq!(
            contract.parameters,
            vec![ResourceParameterMode::Borrowed; arity]
        );
        assert_eq!(contract.result, ResourceReturnMode::None);
        assert!(contract.counted_users.neutral, "{name}");
        assert!(contract.counted_users.effects.is_empty(), "{name}");
    }
    assert!(
        contracts(false)
            .contract("flb_routes_mask_get_size")
            .is_some()
    );
}

#[test]
fn each_query_accepts_and_withdraws() {
    if !clang_available() {
        return;
    }
    for body in [
        "int test(char *p) { return input_chunk_has_magic_bytes(p, 2); }",
        "int test(struct flb_input_instance *p) { return flb_input_chunk_is_mem_overlimit(p); }",
        "int test(struct flb_input_instance *p) { return flb_input_chunk_is_storage_overlimit(p); }",
        "size_t test(struct flb_router *p) { return flb_routes_mask_get_slots(p); }",
        "int test(unsigned long *m, struct flb_router *p) { return flb_routes_mask_is_empty(m, p); }",
    ] {
        let accepted = summary(body, true);
        assert_eq!(
            accepted.status,
            AnalysisStatus::Safe,
            "{body}: {:?}",
            accepted.diagnostics
        );
        let withdrawn = summary(body, false);
        assert_ne!(withdrawn.status, AnalysisStatus::Safe, "{body}");
        assert!(withdrawn.diagnostics.iter().any(|d| matches!(&d.kind,
            LegacyDiagnosticKind::Unsupported { description } if description.contains("uncontracted function"))),
            "{body}: {:?}", withdrawn.diagnostics);
    }
}

#[test]
fn a_borrowed_mask_cannot_release_its_owner() {
    if !clang_available() {
        return;
    }
    let kept = summary(
        "int test(struct flb_router *r) { unsigned long *m = malloc(8); \
        int empty = flb_routes_mask_is_empty(m, r); free(m); return empty; }",
        true,
    );
    assert_eq!(kept.status, AnalysisStatus::Safe, "{:?}", kept.diagnostics);
    let leaked = summary(
        "int test(struct flb_router *r) { unsigned long *m = malloc(8); \
        return flb_routes_mask_is_empty(m, r); }",
        true,
    );
    assert_ne!(leaked.status, AnalysisStatus::Safe);
    assert!(
        leaked
            .diagnostics
            .iter()
            .any(|d| matches!(d.kind, LegacyDiagnosticKind::Leak { .. }))
    );
}
