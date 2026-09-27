use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyInstruction, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11, check_legacy_safety_certificate,
};

fn run(source: &str) -> UnrestrictedLegacyResult {
    analyze_unrestricted_c11(
        &C11TranslationUnit {
            source,
            ..Default::default()
        },
        "subject",
        &ResourceContracts::default(),
    )
    .expect("valid C")
}

#[test]
fn multiplication_is_not_a_pointer_access() {
    let source = "int subject(void) { int x = 2; x = 3 * 4; return 0; }";
    let result = run(source);
    let UnrestrictedLegacyResult::Safe(certificate) = result else {
        panic!("typed non-access should decide: {result:?}");
    };
    assert_eq!(certificate.summary.status, AnalysisStatus::Safe);
    check_legacy_safety_certificate(&certificate).expect("independent rederivation");
}

#[test]
fn untracked_multi_pointer_declaration_is_not_an_access() {
    let source = "struct S { int x; }; int subject(void *a, void *b) { const struct S *x = a, *y = b; return 0; }";
    let result = run(source);
    let UnrestrictedLegacyResult::Safe(certificate) = result else {
        panic!("untracked pointer aliases should decide: {result:?}");
    };
    check_legacy_safety_certificate(&certificate).expect("independent rederivation");
}

#[test]
fn pointer_store_and_union_still_refuse() {
    for source in [
        "struct S { int *p; }; int subject(struct S *s, int *q) { s->p = q; return 0; }",
        "union U { int x; }; int subject(union U *u) { int x = u->x; return 0; }",
    ] {
        let result = run(source);
        let UnrestrictedLegacyResult::Unknown {
            partial: Some(summary),
            ..
        } = result
        else {
            panic!("unsupported path must stay Unknown: {result:?}");
        };
        assert!(
            summary.functions["subject"]
                .diagnostics
                .iter()
                .any(|diagnostic| {
                    matches!(&diagnostic.kind, LegacyDiagnosticKind::Unsupported { description }
                if description == "indirect memory access is outside the resource projection")
                })
        );
    }
}

#[test]
fn extended_typed_member_chain_uses_untracked_scope() {
    let source = "struct S { struct S *next; int value; }; int subject(struct S *p) { int x = p->next->next->next->next->value; return 0; }";
    let result = run(source);
    let UnrestrictedLegacyResult::Safe(certificate) = result else {
        panic!("typed path should decide the chain: {result:?}");
    };
    check_legacy_safety_certificate(&certificate).expect("independent rederivation");
}

#[test]
fn pointer_value_read_binds_only_a_borrowed_alias() {
    let source = "struct S { struct S *next; int *child; }; int subject(struct S *s) { int *q = s->next->next->next->next->child; return 0; }";
    let result = run(source);
    let UnrestrictedLegacyResult::Safe(certificate) = result else {
        panic!("pointer read should bind a borrowed alias: {result:?}");
    };
    assert!(
        certificate.summary.functions["subject"]
            .cfg
            .nodes
            .iter()
            .any(|node| {
                matches!(&node.instruction, LegacyInstruction::IndirectScalarAccess {
            borrowed_destination: true, destination: Some(name), ..
        } if name == "q")
            })
    );
    check_legacy_safety_certificate(&certificate).expect("independent rederivation");
}
