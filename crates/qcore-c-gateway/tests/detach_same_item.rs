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

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyFunctionScope, LegacyProgramSummary,
    ResourceContracts, UnrestrictedLegacyResult, analyze_legacy_c, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;
use std::collections::BTreeMap;

const REPRESENTATION: &str = "For this removed-result declaration, the live parent represents exactly its owned children by a finite acyclic child and next chain; child is NULL precisely when the chain is empty, each nonfirst child's prev is its predecessor, the first child's prev is the last child, chain child identities are pairwise distinct, every non-NULL next or prev target is a live child of that same parent, the last child's next is NULL, the parent is not a child, and no concurrent writer mutates these links during the call";
fn environment() -> ResourceContracts {
    let text = format!(
        r#"module contract;
extern fn dt_alloc() -> ptr<mut, u8> contract {{
 requires resource_contract_v1; ensures normal resource_return_owned(nonnull);
}};
extern fn dt_free(p: ptr<mut, u8>) -> unit contract {{
 requires resource_contract_v1; ensures normal resource_released(p, nullable); ensures normal resource_return_none();
}};
extern fn dt_store(parent: ptr<mut, u8>, item: ptr<mut, u8>) -> unit contract {{
 requires resource_contract_v1; ensures normal resource_owned(parent, nonnull);
 ensures normal resource_stored(item, parent, nonnull); ensures normal resource_return_none();
}};
extern fn dt_realloc(parent: ptr<mut, u8>) -> ptr<mut, u8> contract {{
 requires resource_contract_v1; ensures normal resource_owned(parent, nonnull);
 ensures normal resource_return_reallocated(parent, nullable);
}};
extern fn dt_child(parent: ptr<mut, u8>) -> ptr<mut, u8> contract {{
 requires resource_contract_v1; ensures normal resource_borrowed(parent);
 ensures normal resource_return_borrowed_from(parent, nullable);
}};
extern fn dt_detach(parent: ptr<mut, u8>, item: ptr<mut, u8>) -> ptr<mut, u8> contract {{
 requires resource_contract_v1;
 requires resource_premise(detach_list_representation, "{REPRESENTATION}");
 ensures normal resource_owned(parent, nonnull); ensures normal resource_removed(item, parent, nullable);
 ensures normal resource_return_removed(item, nullable);
}};
"#
    );
    load_resource_contract_sources(&BTreeMap::from([("detach.qv".to_owned(), text)])).unwrap()
}
fn raw(body: &str) -> qcore_c_gateway::legacy_c::LegacyFunctionSummary {
    analyze_legacy_c(
        &format!("void subject(void) {{ {body} }}"),
        "subject",
        &environment(),
    )
    .unwrap()
}
#[test]
fn exact_membership_parent_lifetime_and_same_item_alias_are_preserved() {
    for body in [
        "void *p=dt_alloc(); void *i=dt_alloc(); dt_store(p,i); void *r=dt_detach(p,i); dt_free(p); dt_free(r);",
        "void *p=dt_alloc(); void *i=dt_alloc(); void *alias=i; dt_store(p,i); void *r=dt_detach(p,i); dt_free(p); dt_free(alias);",
        "void *p=dt_alloc(); void *i=dt_alloc(); void *j=dt_alloc(); dt_store(p,i); dt_store(p,j); void *r=dt_detach(p,i); dt_free(r); dt_free(p);",
        "void *p=dt_alloc(); void *i=0; void *r=dt_detach(p,i); dt_free(r); dt_free(p);",
        "void *p=dt_alloc(); void *r=dt_detach(p,0); dt_free(r); dt_free(p);",
        "void *p=dt_alloc(); void *i=dt_alloc(); dt_store(p,i); i=dt_detach(p,i); dt_free(p); dt_free(i);",
        "void *p=dt_alloc(); void *q=p; void *i=dt_alloc(); dt_store(p,i); void *r=dt_detach(q,i); dt_free(p); dt_free(r);",
    ] {
        let summary = raw(body);
        assert_eq!(summary.status, AnalysisStatus::Safe, "{body}\n{summary:?}");
    }
}
#[test]
fn missing_membership_wrong_parent_derived_item_and_dead_parent_refuse() {
    for body in [
        "void *p=dt_alloc(); void *i=dt_alloc(); void *r=dt_detach(p,i); dt_free(r); dt_free(p);",
        "void *p=dt_alloc(); void *q=dt_alloc(); void *i=dt_alloc(); dt_store(p,i); void *r=dt_detach(q,i); dt_free(r); dt_free(p); dt_free(q);",
        "void *p=dt_alloc(); void *i=dt_child(p); void *r=dt_detach(p,i); dt_free(r); dt_free(p);",
        "void *p=dt_alloc(); void *i=dt_alloc(); dt_store(p,i); dt_free(p); void *r=dt_detach(p,i); dt_free(r);",
        "void *p=0; void *i=0; void *r=dt_detach(p,i); dt_free(r);",
        "void *p=dt_alloc(); void *i=dt_alloc(); dt_store(p,i); void *replacement=dt_realloc(p); void *r=dt_detach(p,i); dt_free(r); dt_free(replacement);",
    ] {
        let summary = raw(body);
        assert_eq!(
            summary.status,
            AnalysisStatus::Unknown,
            "{body}\n{summary:?}"
        );
        assert!(
            summary
                .diagnostics
                .iter()
                .any(|d| format!("{:?}", d.kind).contains("same-item detach")),
            "{summary:?}"
        );
    }
}
#[test]
fn releasing_both_result_and_old_alias_does_not_create_two_obligations() {
    let summary = raw(
        "void *p=dt_alloc(); void *i=dt_alloc(); dt_store(p,i); void *r=dt_detach(p,i); dt_free(r); dt_free(i); dt_free(p);",
    );
    assert_eq!(summary.status, AnalysisStatus::Unsafe, "{summary:?}");
}
fn partial(outcome: &UnrestrictedLegacyResult) -> &LegacyProgramSummary {
    match outcome {
        UnrestrictedLegacyResult::Safe(c) => &c.summary,
        UnrestrictedLegacyResult::Unsafe(s) => s,
        UnrestrictedLegacyResult::Unknown {
            partial: Some(s), ..
        } => s,
        _ => panic!("no summary: {outcome:?}"),
    }
}
#[test]
fn checked_header_body_allows_a_caller_certificate_and_replays_independently() {
    if let Err(reason) = qcore_c_gateway::clang_command() {
        assert!(
            std::env::var_os("QCORE_CLANG").is_none(),
            "invalid explicit QCORE_CLANG: {reason}"
        );
        return;
    }
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/detach-same-item");
    let source = "#include \"detach_impl.h\"\nstruct DetachNode *dt_alloc(void); void dt_free(struct DetachNode*); void dt_store(struct DetachNode*,struct DetachNode*);\nvoid subject(void) { struct DetachNode *p=dt_alloc(); struct DetachNode *i=dt_alloc(); dt_store(p,i); struct DetachNode *r=dt_detach(p,i); dt_free(p); dt_free(r); }";
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source,
            include_paths: vec![root.to_string_lossy().into_owned()],
            function_scope: LegacyFunctionScope::MainSourceDefinitions,
            ..Default::default()
        },
        "subject",
        &environment(),
    )
    .unwrap();
    let summary = partial(&outcome);
    assert_eq!(
        summary.functions.get("subject").unwrap().status,
        AnalysisStatus::Safe,
        "{summary:?}"
    );
    let UnrestrictedLegacyResult::Safe(certificate) = outcome else {
        panic!("{outcome:?}");
    };
    assert!(
        certificate
            .contracts
            .source_assumptions()
            .values()
            .any(|source| source.contains(REPRESENTATION))
    );
    check_legacy_safety_certificate(&certificate)
        .expect("same-source replay must retain membership, body template and premise");
}

#[test]
fn native_unlink_frame_covers_all_positions_singleton_and_null_paths() {
    let mut clang = match qcore_c_gateway::clang_command() {
        Ok(command) => command,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "invalid explicit QCORE_CLANG: {reason}"
            );
            return;
        }
    };
    let fixtures = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/detach-same-item");
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory = std::env::temp_dir().join(format!(
        "qcore-detach-native-{}-{nonce}",
        std::process::id()
    ));
    std::fs::create_dir(&directory).unwrap();
    let executable = directory.join(if cfg!(windows) {
        "detach.exe"
    } else {
        "detach"
    });
    let built = clang
        .args(["-std=c11", "-Wall", "-Wextra", "-Werror"])
        .arg(fixtures.join("native_case.c"))
        .arg("-o")
        .arg(&executable)
        .output()
        .unwrap();
    assert!(
        built.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&built.stdout),
        String::from_utf8_lossy(&built.stderr)
    );
    let ran = std::process::Command::new(&executable).output().unwrap();
    assert!(
        ran.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&ran.stdout),
        String::from_utf8_lossy(&ran.stderr)
    );
    std::fs::remove_file(&executable).unwrap();
    std::fs::remove_dir(&directory).unwrap();
}

#[test]
fn overwriting_the_only_parent_name_does_not_discharge_its_obligation() {
    for body in [
        "void *p=dt_alloc(); void *i=dt_alloc(); dt_store(p,i); p=dt_detach(p,i); dt_free(p);",
        "void *p=dt_alloc(); p=dt_detach(p,0); dt_free(p);",
    ] {
        let summary = raw(body);
        assert_eq!(
            summary.status,
            AnalysisStatus::Unsafe,
            "{body}\n{summary:?}"
        );
    }
}

#[test]
fn an_unproved_forwarding_wrapper_does_not_become_borrowed_only() {
    let source = "void *forward(void *parent, void *item) { return dt_detach(parent,item); }\nvoid subject(void) { void *p=dt_alloc(); void *i=dt_alloc(); dt_store(p,i); void *r=forward(p,i); dt_free(p); dt_free(r); }";
    let summary = qcore_c_gateway::legacy_c::analyze_legacy_c_translation_unit(
        source,
        "subject",
        &environment(),
    )
    .unwrap();
    assert_eq!(
        summary.functions["forward"].status,
        AnalysisStatus::Unknown,
        "{summary:?}"
    );
    assert_ne!(
        summary.functions["subject"].status,
        AnalysisStatus::Safe,
        "{summary:?}"
    );
}
