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

//! Card OTEL-PROTO-PHASE-B1: the shipped `quirin-provider-fluent-otel-proto`
//! declarations, read by the gateway as written. An `__unpack` result is an
//! owned, nullable message: released by `__free_unpacked` it is Safe, never
//! released it is a leak (Unsafe), released twice Unsafe, handed to an
//! undeclared callee Unknown — never Safe without its release. An `__init` of
//! memory the function holds moves no custody.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, LegacyProgramSummary, ResourceContracts, analyze_legacy_c_translation_unit,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const GENERATED: &str = include_str!(
    "../../../providers/quirin-provider-fluent-otel-proto/contracts/fluent_otel_proto/generated.qv"
);
const RUNTIME: &str = include_str!(
    "../../../providers/quirin-provider-fluent-otel-proto/contracts/fluent_otel_proto/runtime.qv"
);
const LIBC_MEMORY: &str =
    include_str!("../../../providers/quirin-provider-libc/contracts/libc/memory.qv");

const UNPACK: &str =
    "opentelemetry__proto__collector__trace__v1__export_trace_service_request__unpack";
const FREE: &str =
    "opentelemetry__proto__collector__trace__v1__export_trace_service_request__free_unpacked";
const INIT: &str = "opentelemetry__proto__trace__v1__span__init";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([
        (
            "fluent_otel_proto/generated.qv".to_owned(),
            GENERATED.to_owned(),
        ),
        (
            "fluent_otel_proto/runtime.qv".to_owned(),
            RUNTIME.to_owned(),
        ),
        ("libc/memory.qv".to_owned(), LIBC_MEMORY.to_owned()),
    ]))
    .expect("the crate's modules load with libc's")
}

fn prelude() -> String {
    format!(
        "typedef unsigned long size_t;\n\
         typedef struct R R;\n\
         typedef struct S S;\n\
         R *{UNPACK}(void *allocator, size_t len, const unsigned char *data);\n\
         void {FREE}(R *message, void *allocator);\n\
         void {INIT}(S *message);\n\
         void *calloc(size_t count, size_t size);\n\
         void mystery(R *message);\n"
    )
}

fn analyse(entry: &str, body: &str) -> LegacyProgramSummary {
    let source = format!(
        "{}{}",
        prelude(),
        body.replace("UNPACK", UNPACK)
            .replace("FREE", FREE)
            .replace("INIT", INIT)
    );
    analyze_legacy_c_translation_unit(&source, entry, &contracts()).expect("the fixture tokenizes")
}

fn status(entry: &str, body: &str) -> AnalysisStatus {
    let summary = analyse(entry, body);
    summary
        .functions
        .get(entry)
        .unwrap_or_else(|| panic!("{entry} analysed: {summary:?}"))
        .status
}

#[test]
fn the_declared_set_loads_and_states_its_shapes() {
    let loaded = contracts();
    for name in [UNPACK, FREE, INIT, "protobuf_c_message_unpack"] {
        assert!(loaded.contract(name).is_some(), "{name} is declared");
    }
    assert_eq!(GENERATED.matches("\nextern fn ").count(), 78);
    assert_eq!(RUNTIME.matches("\nextern fn ").count(), 4);
}

#[test]
fn an_unpack_result_released_by_free_unpacked_is_safe() {
    let body = "void a(const unsigned char *d, size_t n) { R *m; m = UNPACK(0, n, d); \
                if (!m) { return; } FREE(m, 0); }";
    assert_eq!(status("a", body), AnalysisStatus::Safe);
}

#[test]
fn an_unpack_result_never_released_is_never_safe() {
    let leak = "void b(const unsigned char *d, size_t n) { R *m; m = UNPACK(0, n, d); \
                if (!m) { return; } }";
    assert_eq!(status("b", leak), AnalysisStatus::Unsafe);
    let twice = "void c(const unsigned char *d, size_t n) { R *m; m = UNPACK(0, n, d); \
                 if (!m) { return; } FREE(m, 0); FREE(m, 0); }";
    assert_eq!(status("c", twice), AnalysisStatus::Unsafe);
    let handed = "void e(const unsigned char *d, size_t n) { R *m; m = UNPACK(0, n, d); \
                  if (!m) { return; } mystery(m); FREE(m, 0); }";
    assert_ne!(status("e", handed), AnalysisStatus::Safe);
}

#[test]
fn an_init_of_held_memory_moves_no_custody() {
    let body = "S *f(void) { S *s; s = calloc(1, 64); if (!s) { return 0; } INIT(s); return s; }";
    assert_eq!(status("f", body), AnalysisStatus::Safe);
    let lost = "void g(void) { S *s; s = calloc(1, 64); if (!s) { return; } INIT(s); }";
    assert_eq!(status("g", lost), AnalysisStatus::Unsafe);
}
