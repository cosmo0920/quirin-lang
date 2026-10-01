//! Focused custody controls for W46-LIBC-EVENT-CONTRACT-GAPS.
use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyFunctionSummary,
    ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
};
use qcore_contract::contract_loader::load_resource_contract_sources;
use qcore_schemas::contract::{ResourceParameterMode, ResourceReturnMode};

const OBSERVATION: &str = include_str!(
    "../../../providers/qcore-provider-libevent/contracts/monkey/scalar_observation.qv"
);
const MEMORY: &str =
    include_str!("../../../providers/quirin-provider-libc/contracts/libc/memory.qv");
const STRING: &str =
    include_str!("../../../providers/quirin-provider-libc/contracts/libc/string.qv");

#[test]
fn reusable_libc_catalog_declares_only_neutral_queries() {
    let loaded = load_resource_contract_sources(&BTreeMap::from([(
        "libc/string.qv".to_owned(),
        STRING.to_owned(),
    )]))
    .expect("production libc string module loads");
    for (name, arity) in [
        ("memcmp", 3),
        ("strncmp", 3),
        ("strnlen", 2),
        ("strcasecmp", 2),
    ] {
        let contract = loaded
            .contract(name)
            .unwrap_or_else(|| panic!("missing {name}"));
        assert_eq!(
            contract.parameters,
            vec![ResourceParameterMode::Borrowed; arity],
            "{name}"
        );
        assert_eq!(contract.result, ResourceReturnMode::None, "{name}");
    }
}

#[test]
fn libc_scalar_contracts_match_the_host_declared_abi() {
    if !clang_available() {
        return;
    }
    let contracts = load_resource_contract_sources(&BTreeMap::from([(
        "libc/string.qv".to_owned(),
        STRING.to_owned(),
    )]))
    .expect("production libc string module loads");
    for body in [
        "int test(const void *a, const void *b, size_t n) { return memcmp(a,b,n); }",
        "int test(const char *a, const char *b, size_t n) { return strncmp(a,b,n); }",
        "size_t test(const char *a, size_t n) { return strnlen(a,n); }",
        "int test(const char *a, const char *b) { return strcasecmp(a,b); }",
    ] {
        let source =
            format!("#define _GNU_SOURCE 1\n#include <string.h>\n#include <strings.h>\n{body}\n");
        let result = analyze_unrestricted_c11(
            &C11TranslationUnit {
                source: &source,
                ..Default::default()
            },
            "test",
            &contracts,
        )
        .expect("host declaration parses");
        let status = match result {
            UnrestrictedLegacyResult::Safe(_) => AnalysisStatus::Safe,
            UnrestrictedLegacyResult::Unsafe(s) => s.functions["test"].status,
            UnrestrictedLegacyResult::Unknown {
                partial: Some(s), ..
            } => s.functions["test"].status,
            other => panic!("no analyzable summary: {other:?}"),
        };
        assert_eq!(status, AnalysisStatus::Safe, "{body}");
    }
}

fn contracts(with_observation: bool) -> ResourceContracts {
    let mut modules = BTreeMap::from([("libc/memory.qv".to_owned(), MEMORY.to_owned())]);
    if with_observation {
        modules.insert(
            "monkey/scalar_observation.qv".to_owned(),
            OBSERVATION.to_owned(),
        );
    }
    load_resource_contract_sources(&modules).expect("production contract modules load")
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

fn summary(body: &str, with_observation: bool) -> LegacyFunctionSummary {
    let source = format!(
        "typedef __SIZE_TYPE__ size_t; typedef unsigned int uint32_t; \
         struct mk_event {{ int mask; int status; }}; struct event_base; \
         void *malloc(size_t); void free(void *); \
         int _mk_string_search(const char *, const char *, int, int); \
         int mk_string_char_search(const char *, int, int); \
         int mk_string_char_search_r(const char *, int, int); \
         void MK_EVENT_NEW(struct mk_event *); \
         int event_base_get_features(const struct event_base *); \
         size_t event_get_struct_event_size(void); \
         uint32_t event_get_version_number(void); {body}"
    );
    let result = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        "test",
        &contracts(with_observation),
    )
    .expect("fixture compiles");
    let program = match result {
        UnrestrictedLegacyResult::Safe(c) => c.summary,
        UnrestrictedLegacyResult::Unsafe(s)
        | UnrestrictedLegacyResult::Unknown {
            partial: Some(s), ..
        } => s,
        other => panic!("no summary: {other:?}"),
    };
    program.functions.get("test").expect("test summary").clone()
}

#[test]
fn scalar_observations_accept_and_withdraw() {
    if !clang_available() {
        return;
    }
    let bodies = [
        "int test(const char *a, const char *b) { return _mk_string_search(a,b,1,4); }",
        "int test(const char *a) { return mk_string_char_search(a,65,4); }",
        "int test(const char *a) { return mk_string_char_search_r(a,65,4); }",
        "void test(struct mk_event *e) { MK_EVENT_NEW(e); }",
        "int test(const struct event_base *b) { return event_base_get_features(b); }",
        "size_t test(void) { return event_get_struct_event_size(); }",
        "uint32_t test(void) { return event_get_version_number(); }",
    ];
    for body in bodies {
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
            LegacyDiagnosticKind::Unsupported { description } if description.starts_with("call to uncontracted function"))),
            "{body}: {:?}", withdrawn.diagnostics);
    }
}

#[test]
fn borrowed_observation_preserves_leak_and_liveness_refusal() {
    if !clang_available() {
        return;
    }
    let leak = summary(
        "void test(void) { char *s = malloc(16); if (!s) return; _mk_string_search(s, \"x\", 1, 4); }",
        true,
    );
    assert_eq!(
        leak.status,
        AnalysisStatus::Unsafe,
        "{:?}",
        leak.diagnostics
    );
    assert!(
        leak.diagnostics
            .iter()
            .any(|d| matches!(d.kind, LegacyDiagnosticKind::Leak { .. }))
    );
    let stale = summary(
        "void test(void) { char *s = malloc(16); if (!s) return; free(s); _mk_string_search(s, \"x\", 1, 4); }",
        true,
    );
    assert_ne!(
        stale.status,
        AnalysisStatus::Safe,
        "{:?}",
        stale.diagnostics
    );
}
