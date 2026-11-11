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

//! Public Clang-to-certificate evidence for nullable removal and custody transfers.
//! External contracts are explicit fixture assumptions, loaded from QCore.
use qcore_compiler::contract_loader::load_resource_contract_sources;
use qcore_compiler::legacy_c::{
    C11TranslationUnit, ContractTrust, LegacySafetyCertificate, ResourceContracts,
    ResourceFunctionContract, ResourceParameterMode as P, ResourceReturnMode as R,
    UnrestrictedLegacyResult, analyze_unrestricted_c11, check_legacy_safety_certificate,
};

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent nullable-removal test: {reason}");
            false
        }
    }
}

const DECLARATIONS: &str = "void *alloc(void); void *maybe_alloc(void); void release(void *); \
    void store(void *, void *); void detach(void *, void *); void strict_detach(void *, void *); \
    void opaque(void *); void transfer(void *); void *resize(void *, __SIZE_TYPE__); void *resize_nonnull(void *, __SIZE_TYPE__); void *resize_nullable_old(void *, __SIZE_TYPE__);";
const API: &str = r#"
module contract;
extern fn alloc() -> ptr<mut, u8> contract {
 requires resource_contract_v1; ensures normal resource_return_owned(nonnull);
};
extern fn maybe_alloc() -> ptr<mut, u8> contract {
 requires resource_contract_v1; ensures normal resource_return_owned(nullable);
};
extern fn release(p: ptr<mut, u8>) -> unit contract {
 requires resource_contract_v1; ensures normal resource_released(p, nullable);
 ensures normal resource_return_none();
};
extern fn store(owner: ptr<mut, u8>, child: ptr<mut, u8>) -> unit contract {
 requires resource_contract_v1; ensures normal resource_owned(owner, nonnull);
 ensures normal resource_stored(child, owner, nullable); ensures normal resource_return_none();
};
extern fn detach(owner: ptr<mut, u8>, child: ptr<mut, u8>) -> unit contract {
 requires resource_contract_v1; ensures normal resource_owned(owner, nonnull);
 ensures normal resource_removed(child, owner, nullable); ensures normal resource_return_none();
};
extern fn transfer(p: ptr<mut, u8>) -> unit contract {
 requires resource_contract_v1; ensures normal resource_consumed(p, nonnull);
 ensures normal resource_return_none();
};
extern fn resize(p: ptr<mut, u8>, size: usize) -> ptr<mut, u8> contract {
 requires resource_contract_v1; ensures normal resource_owned(p, nonnull);
 ensures normal resource_borrowed(size); ensures normal resource_return_reallocated(p, nullable);
};
extern fn resize_nonnull(p: ptr<mut, u8>, size: usize) -> ptr<mut, u8> contract {
 requires resource_contract_v1; ensures normal resource_owned(p, nonnull);
 ensures normal resource_borrowed(size); ensures normal resource_return_reallocated(p, nonnull);
};
extern fn resize_nullable_old(p: ptr<mut, u8>, size: usize) -> ptr<mut, u8> contract {
 requires resource_contract_v1; ensures normal resource_owned(p, nullable);
 ensures normal resource_borrowed(size); ensures normal resource_return_reallocated(p, nullable);
};
extern fn strict_detach(owner: ptr<mut, u8>, child: ptr<mut, u8>) -> unit contract {
 requires resource_contract_v1; ensures normal resource_owned(owner, nonnull);
 ensures normal resource_removed(child, owner, nonnull); ensures normal resource_return_none();
};
"#;

fn contracts(extra: Option<&str>) -> ResourceContracts {
    let mut sources = std::collections::BTreeMap::from([("api.qcore".into(), API.into())]);
    if let Some(extra) = extra {
        sources.insert("declared.qcore".into(), extra.into());
    }
    load_resource_contract_sources(&sources).unwrap()
}

fn analyze(body: &str, extra: Option<&str>) -> UnrestrictedLegacyResult {
    let source = format!("{DECLARATIONS}\n{body}");
    analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        "subject",
        &contracts(extra),
    )
    .unwrap_or_else(|e| panic!("Clang fixture failed: {e:?}\n{source}"))
}

fn certified(body: &str) -> LegacySafetyCertificate {
    let result = analyze(body, None);
    let UnrestrictedLegacyResult::Safe(certificate) = result else {
        panic!("expected checked nullable removal: {result:?}");
    };
    check_legacy_safety_certificate(&certificate).unwrap();
    certificate
}

fn expected(owner: usize) -> ResourceFunctionContract {
    let mut modes = vec![P::Owned { nullable: false }; 2];
    modes[1 - owner] = P::Removed {
        owner_parameter: owner,
        nullable: true,
    };
    ResourceFunctionContract::new(modes, R::None)
}

fn assert_summary(certificate: &LegacySafetyCertificate, function: &str, owner: usize) {
    let record = certificate
        .summaries
        .iter()
        .find(|s| s.function == function)
        .unwrap();
    assert_eq!(record.trust, ContractTrust::InferredValidated);
    assert_eq!(record.contract, expected(owner));
}

#[test]
fn conditional_then_unconditional_detach_never_erases_the_double_removal_path() {
    if !clang_available() {
        return;
    }
    let body = "void take(void *o, void *p, int flag) { if(flag) { strict_detach(o,p); } strict_detach(o,p); } \
        void subject(void) { void *o=alloc(); void *p=alloc(); store(o,p); take(o,p,1); release(p); release(o); }";
    let result = analyze(body, None);
    assert!(
        !matches!(result, UnrestrictedLegacyResult::Safe(_)),
        "double removal became Safe: {result:?}"
    );
}

#[test]
fn nullable_removal_handles_null_maybe_null_and_nonnull_callers() {
    if !clang_available() {
        return;
    }
    let wrapper = "void take(void *o, void *p) { detach(o,p); }";
    for allocation in ["0", "maybe_alloc()", "alloc()"] {
        let certificate = certified(&format!(
            "{wrapper} void subject(void) {{ void *o=alloc(); void *p={allocation}; store(o,p); take(o,p); release(p); release(o); }}"
        ));
        assert_summary(&certificate, "take", 0);
    }
}

#[test]
fn guarded_nonnull_removal_accepts_nullable_entry_and_reordered_forwarding() {
    if !clang_available() {
        return;
    }
    for body in [
        "if(p) { strict_detach(o,p); }",
        "if(!p) { return; } strict_detach(o,p);",
        "if(p == 0) { return; } strict_detach(o,p);",
        "if(p != 0) { strict_detach(o,p); }",
    ] {
        let source = format!(
            "void take(void *o, void *p) {{ {body} }} \
          void reversed(void *p, void *o) {{ take(o,p); }} \
          void subject(void) {{ void *o=alloc(); void *p=maybe_alloc(); store(o,p); reversed(p,o); release(p); release(o); }}"
        );
        let certificate = certified(&source);
        assert_summary(&certificate, "take", 0);
        assert_summary(&certificate, "reversed", 1);
    }
}

#[test]
fn null_child_refinement_preserves_another_contained_child() {
    if !clang_available() {
        return;
    }
    let source = "void take(void *o, void *p, void *q) { \
        if(p) { strict_detach(o,p); } strict_detach(o,q); } \
        void subject(void) { void *o=alloc(); void *p=maybe_alloc(); void *q=alloc(); \
        store(o,p); store(o,q); take(o,p,q); release(p); release(q); release(o); }";
    let certificate = certified(source);
    let record = certificate
        .summaries
        .iter()
        .find(|s| s.function == "take")
        .unwrap();
    assert_eq!(
        record.contract,
        ResourceFunctionContract::new(
            [
                P::Owned { nullable: false },
                P::Removed {
                    owner_parameter: 0,
                    nullable: true
                },
                P::Removed {
                    owner_parameter: 0,
                    nullable: false
                },
            ],
            R::None
        )
    );
    // A distinct sibling still owned by the aggregate is reclaimed with it.
    certified(
        "void take(void *o, void *p) { if(!p) { return; } strict_detach(o,p); } \
        void subject(void) { void *o=alloc(); void *q=alloc(); void *p=maybe_alloc(); \
        store(o,q); store(o,p); take(o,p); release(p); release(o); }",
    );
}

#[test]
fn nullable_summary_cannot_hide_original_child_or_owner_losses() {
    if !clang_available() {
        return;
    }
    for (case, body) in [
        ("overwrite child", "p=0; detach(o,p);"),
        ("overwrite owner", "o=alloc(); detach(o,p);"),
        (
            "null owner release",
            "if(!p) { release(o); return; } strict_detach(o,p);",
        ),
        ("skip real child", "if(p) { return; } detach(o,p);"),
        ("reattach", "detach(o,p); store(o,p);"),
        ("release child", "detach(o,p); release(p);"),
        ("release owner", "detach(o,p); release(o);"),
        ("unknown effect", "detach(o,p); opaque(p);"),
    ] {
        let result = analyze(
            &format!(
                "void take(void *o, void *p) {{ {body} }} \
            void subject(void) {{ void *o=alloc(); void *p=maybe_alloc(); store(o,p); take(o,p); release(p); release(o); }}"
            ),
            None,
        );
        assert!(
            !matches!(result, UnrestrictedLegacyResult::Safe(_)),
            "{case}: {result:?}"
        );
    }
}

#[test]
fn detached_nullable_child_remains_a_caller_obligation() {
    if !clang_available() {
        return;
    }
    for caller in [
        "store(o,p); take(o,p); release(o);",
        "take(o,p); release(p); release(o);",
        "store(o,p); take(o,p); take(o,p); release(p); release(o);",
        "void *wrong=alloc(); store(o,p); take(wrong,p); release(p); release(o); release(wrong);",
    ] {
        let result = analyze(
            &format!(
                "void take(void *o, void *p) {{ detach(o,p); }} \
            void subject(void) {{ void *o=alloc(); void *p=maybe_alloc(); {caller} }}"
            ),
            None,
        );
        assert!(
            !matches!(result, UnrestrictedLegacyResult::Safe(_)),
            "{caller}: {result:?}"
        );
    }
}

#[test]
fn nullable_removal_certificates_rederive_modes_and_source() {
    if !clang_available() {
        return;
    }
    let source = "void take(void *o, void *p) { if(p) { strict_detach(o,p); } } \
        void subject(void) { void *o=alloc(); void *p=maybe_alloc(); store(o,p); take(o,p); release(p); release(o); }";
    let certificate = certified(source);
    let mut mode = certificate.clone();
    mode.summaries
        .iter_mut()
        .find(|s| s.function == "take")
        .unwrap()
        .contract
        .parameters[1] = P::Removed {
        owner_parameter: 0,
        nullable: false,
    };
    assert!(check_legacy_safety_certificate(&mode).is_err());
    let mut changed = certificate;
    changed.preprocessed_source = changed
        .preprocessed_source
        .replace("strict_detach(o,p)", "opaque(p)");
    assert!(check_legacy_safety_certificate(&changed).is_err());
}

#[test]
fn merged_containment_cannot_choose_a_convenient_owner() {
    if !clang_available() {
        return;
    }
    let result = analyze(
        "void subject(int flag) { void *a=alloc(); void *b=alloc(); void *p=alloc(); \
        if(flag) { store(a,p); } else { store(b,p); } strict_detach(a,p); \
        release(p); release(a); release(b); }",
        None,
    );
    assert!(
        !matches!(result, UnrestrictedLegacyResult::Safe(_)),
        "alternative owner was erased: {result:?}"
    );
}

#[test]
fn selected_child_alias_cannot_detach_every_possible_child() {
    if !clang_available() {
        return;
    }
    let result = analyze(
        "void subject(int flag) { void *o=alloc(); void *p=alloc(); void *q=alloc(); \
        store(o,p); store(o,q); void *selected; if(flag) { selected=p; } else { selected=q; } \
        strict_detach(o,selected); release(p); release(q); release(o); }",
        None,
    );
    assert!(
        !matches!(result, UnrestrictedLegacyResult::Safe(_)),
        "ambiguous child detached both possibilities: {result:?}"
    );
}

#[test]
fn releasing_a_selected_alias_does_not_discharge_both_allocations() {
    if !clang_available() {
        return;
    }
    let result = analyze(
        "void subject(int flag) { void *p=alloc(); void *q=alloc(); void *selected; \
        if(flag) { selected=p; } else { selected=q; } release(selected); }",
        None,
    );
    assert!(
        !matches!(result, UnrestrictedLegacyResult::Safe(_)),
        "one release discharged both allocations: {result:?}"
    );
}

#[test]
fn storing_a_selected_alias_does_not_transfer_both_allocations() {
    if !clang_available() {
        return;
    }
    let result = analyze(
        "void subject(int flag) { void *o=alloc(); void *p=alloc(); void *q=alloc(); void *selected; \
        if(flag) { selected=p; } else { selected=q; } store(o,selected); release(o); }",
        None,
    );
    assert!(
        !matches!(result, UnrestrictedLegacyResult::Safe(_)),
        "one store transferred both allocations: {result:?}"
    );
}

#[test]
fn reallocation_does_not_replace_every_possible_old_allocation() {
    if !clang_available() {
        return;
    }
    let result = analyze(
        "void subject(int flag) { void *p=alloc(); void *q=alloc(); void *selected; \
        if(flag) { selected=p; } else { selected=q; } void *r=resize(selected,8); \
        if(r) { release(r); } else { release(p); release(q); } }",
        None,
    );
    assert!(
        !matches!(result, UnrestrictedLegacyResult::Safe(_)),
        "reallocation replaced both old resources: {result:?}"
    );
}

#[test]
fn conditional_containment_cannot_discharge_a_directly_live_alternative() {
    if !clang_available() {
        return;
    }
    let result = analyze(
        "void subject(int flag) { void *o=alloc(); void *p=alloc(); \
        if(flag) { store(o,p); } release(o); }",
        None,
    );
    assert!(
        !matches!(result, UnrestrictedLegacyResult::Safe(_)),
        "cascade erased unstored child: {result:?}"
    );
}

#[test]
fn consumed_custody_cannot_be_reused_after_release_or_prior_escape() {
    if !clang_available() {
        return;
    }
    for first in ["release(p)", "transfer(p)"] {
        let result = analyze(
            &format!("void subject(void) {{ void *p=alloc(); {first}; transfer(p); }}"),
            None,
        );
        assert!(
            !matches!(result, UnrestrictedLegacyResult::Safe(_)),
            "{first}: {result:?}"
        );
    }
}

#[test]
fn uncertain_selected_release_keeps_unrelated_leak_evidence() {
    if !clang_available() {
        return;
    }
    let result = analyze(
        "void subject(int flag) { void *unrelated=alloc(); void *p=alloc(); void *q=alloc(); \
        void *selected; if(flag) { selected=p; } else { selected=q; } release(selected); }",
        None,
    );
    let UnrestrictedLegacyResult::Unsafe(summary) = result else {
        panic!("independent leak disappeared: {result:?}")
    };
    assert!(
        summary.functions["subject"]
            .diagnostics
            .iter()
            .any(|d| matches!(
                d.kind,
                qcore_compiler::legacy_c::LegacyDiagnosticKind::Leak { .. }
            ))
    );
}

#[test]
fn reallocation_cannot_erase_a_previously_released_alternative() {
    if !clang_available() {
        return;
    }
    let result = analyze(
        "void subject(int flag) { void *p=alloc(); if(flag) { release(p); } \
        void *r=resize_nonnull(p,8); release(r); }",
        None,
    );
    assert!(
        !matches!(result, UnrestrictedLegacyResult::Safe(_)),
        "reallocation erased prior release: {result:?}"
    );
}

#[test]
fn singleton_aliases_still_transfer_custody_normally() {
    if !clang_available() {
        return;
    }
    certified(
        "void subject(void) { void *o=alloc(); void *p=alloc(); void *alias=p; \
        store(o,alias); strict_detach(o,p); release(alias); release(o); }",
    );
    certified("void subject(void) { void *p=alloc(); void *alias=p; transfer(alias); }");
}

#[test]
fn declared_removal_precondition_is_never_silently_replaced() {
    if !clang_available() {
        return;
    }
    let declaration = r#"module contract;
        extern fn take(o: ptr<mut, u8>, p: ptr<mut, u8>) -> unit contract {
            requires resource_contract_v1; ensures normal resource_owned(o, nonnull);
            ensures normal resource_removed(p,o,nonnull); ensures normal resource_return_none();
        };"#;
    let result = analyze(
        "void take(void *o, void *p) { detach(o,p); } \
        void subject(void) { void *o=alloc(); void *p=alloc(); store(o,p); take(o,p); release(p); release(o); }",
        Some(declaration),
    );
    let summary = match &result {
        UnrestrictedLegacyResult::Unknown {
            partial: Some(summary),
            ..
        } => summary,
        UnrestrictedLegacyResult::Unsafe(summary) => summary,
        _ => panic!("validated declaration conflict was lost: {result:?}"),
    };
    assert!(
        summary
            .functions
            .values()
            .any(|f| f.diagnostics.iter().any(|d| matches!(
                d.kind,
                qcore_compiler::legacy_c::LegacyDiagnosticKind::SummaryConflict { .. }
            )))
    );
}

#[test]
fn nullable_old_reallocation_respects_its_explicit_contract() {
    if !clang_available() {
        return;
    }
    for old in ["0", "maybe_alloc()", "alloc()"] {
        certified(&format!(
            "void subject(void) {{ void *p={old}; void *q=resize_nullable_old(p,8); \
            if(q) {{ release(q); }} else {{ release(p); }} }}"
        ));
    }
}
