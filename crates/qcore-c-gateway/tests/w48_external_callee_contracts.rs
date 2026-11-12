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

//! W48-BU-EXTERNAL-CALLEE-CONTRACTS: production contract data, with the
//! existing custody interpretation. These controls claim no buffer bounds,
//! I/O result, errno value, mutex protocol or thread synchronization.
use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyFunctionScope,
    LegacyFunctionSummary, ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
};
use qcore_contract::contract_loader::load_resource_contract_sources;
use qcore_schemas::contract::{ResourceParameterMode, ResourceReturnMode};
use std::collections::BTreeMap;

fn sources() -> BTreeMap<String, String> {
    BTreeMap::from([
        (
            "libc/unistd.qv".into(),
            include_str!("../../../providers/quirin-provider-libc/contracts/libc/unistd.qv")
                .to_owned(),
        ),
        (
            "libc/threads.qv".into(),
            include_str!("../../../providers/quirin-provider-libc/contracts/libc/threads.qv")
                .to_owned(),
        ),
        (
            "libc/memory.qv".into(),
            include_str!("../../../providers/quirin-provider-libc/contracts/libc/memory.qv")
                .to_owned(),
        ),
    ])
}

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&sources()).expect("production libc modules load")
}

// Used only by the Linux-gated header test below.
#[cfg(all(target_os = "linux", target_env = "gnu", target_pointer_width = "64"))]
fn without_declaration(name: &str) -> ResourceContracts {
    let mut input = sources();
    let module = match name {
        "writev" => "libc/unistd.qv",
        "thrd_current" => "libc/threads.qv",
        _ => panic!("unexpected declaration"),
    };
    let text = input.get_mut(module).expect("production module");
    let boundary = text
        .find("// ## Card W48-BU-EXTERNAL-CALLEE-CONTRACTS:")
        .expect("the card's appended declaration section");
    text.truncate(boundary);
    let loaded =
        load_resource_contract_sources(&input).expect("modules without new declaration load");
    assert!(loaded.contract(name).is_none());
    loaded
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(std::env::var_os("QCORE_CLANG").is_none(), "{reason}");
            eprintln!("skipping Clang-dependent W48 external callee control: {reason}");
            false
        }
    }
}

fn analyze(source: &str, contracts: &ResourceContracts) -> LegacyFunctionSummary {
    let result = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source,
            function_scope: LegacyFunctionScope::MainSourceDefinitions,
            ..Default::default()
        },
        "subject",
        contracts,
    )
    .expect("fixture parses");
    match result {
        UnrestrictedLegacyResult::Safe(certificate) => {
            qcore_c_gateway::legacy_c::check_legacy_safety_certificate(&certificate)
                .expect("Safe certificate independently rechecks");
            certificate.summary.functions["subject"].clone()
        }
        UnrestrictedLegacyResult::Unsafe(summary)
        | UnrestrictedLegacyResult::Unknown {
            partial: Some(summary),
            ..
        } => summary.functions["subject"].clone(),
        other => panic!("missing fixture summary: {other:?}"),
    }
}

#[test]
fn production_contracts_return_no_custody_and_do_not_declare_mutex_destruction() {
    let loaded = contracts();
    for (name, arity) in [("writev", 3), ("thrd_current", 0)] {
        let contract = loaded.contract(name).expect("new declaration");
        assert_eq!(
            contract.parameters,
            vec![ResourceParameterMode::Borrowed; arity]
        );
        assert_eq!(contract.result, ResourceReturnMode::None);
    }
    assert!(loaded.contract("mtx_destroy").is_none());
}

#[cfg(all(target_os = "linux", target_env = "gnu", target_pointer_width = "64"))]
#[test]
fn actual_posix_and_c11_headers_agree_and_contract_removal_refuses() {
    if !clang_available() {
        return;
    }
    for (name, source) in [
        (
            "writev",
            "#include <sys/uio.h>\nssize_t subject(int fd, const struct iovec *v, int n) { return writev(fd,v,n); }",
        ),
        (
            "thrd_current",
            "#include <threads.h>\nunsigned long subject(void) { return (unsigned long)(intptr_t)thrd_current(); }",
        ),
    ] {
        let source = format!("#include <stdint.h>\n{source}\n");
        let loaded = contracts();
        assert_eq!(
            analyze(&source, &loaded).status,
            AnalysisStatus::Safe,
            "{name}"
        );
        assert_eq!(
            analyze(&source, &without_declaration(name)).status,
            AnalysisStatus::Unknown,
            "{name}"
        );
    }
}

#[test]
fn a_borrowed_vector_neither_discharges_a_live_object_nor_revives_a_released_one() {
    if !clang_available() {
        return;
    }
    let declarations = "typedef __SIZE_TYPE__ size_t; struct iovec { void *base; size_t len; }; void *malloc(size_t); void free(void *); typedef __INTPTR_TYPE__ ssize_t; ssize_t writev(int,const struct iovec *,int);";
    let leak = analyze(
        &format!(
            "{declarations} void subject(void) {{ struct iovec *v=malloc(sizeof(*v)); if(!v) return; writev(1,v,0); }}"
        ),
        &contracts(),
    );
    assert_eq!(leak.status, AnalysisStatus::Unsafe);
    assert!(
        leak.diagnostics
            .iter()
            .any(|d| matches!(d.kind, LegacyDiagnosticKind::Leak { .. }))
    );
    let released = analyze(
        &format!(
            "{declarations} void subject(void) {{ struct iovec *v=malloc(sizeof(*v)); if(!v) return; free(v); writev(1,v,0); }}"
        ),
        &contracts(),
    );
    assert_eq!(released.status, AnalysisStatus::Unknown);
}

#[test]
fn a_pointer_thread_identifier_withdraws_the_scalar_contract() {
    if !clang_available() {
        return;
    }
    let summary = analyze(
        "extern void *thrd_current(void); void subject(void) { thrd_current(); }",
        &contracts(),
    );
    assert_eq!(summary.status, AnalysisStatus::Unknown);
    assert!(summary.diagnostics.iter().any(|d| matches!(&d.kind, LegacyDiagnosticKind::Unsupported { description } if description.contains("abi-return-type-disagreement"))));
}

#[test]
fn a_global_mutex_destroy_stays_uncontracted() {
    if !clang_available() {
        return;
    }
    let summary = analyze(
        "typedef struct { int state; } mtx_t; void mtx_destroy(mtx_t *); static mtx_t lock; void subject(void) { mtx_destroy(&lock); }",
        &contracts(),
    );
    assert_eq!(summary.status, AnalysisStatus::Unknown);
}

#[test]
fn writev_borrows_the_descriptor_without_reviving_a_closed_one() {
    if !clang_available() {
        return;
    }
    let summary = analyze(
        "int open(const char *, int, ...); int close(int); struct iovec; typedef __INTPTR_TYPE__ ssize_t; ssize_t writev(int,const struct iovec *,int); void subject(const char *path) { int fd; fd = open(path,0); if (fd == -1) { return; } close(fd); writev(fd,0,0); }",
        &contracts(),
    );
    assert_eq!(summary.status, AnalysisStatus::Unsafe);
    assert!(
        summary
            .diagnostics
            .iter()
            .any(|d| matches!(d.kind, LegacyDiagnosticKind::UseAfterClose { .. }))
    );
}
