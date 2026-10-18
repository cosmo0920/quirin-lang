//! Bounded independent scalar alternatives. Synthetic q_* declarations are
//! trusted test assumptions; every Safe certificate is independently replayed.
use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyInstruction, LegacyProgramSummary,
    UnrestrictedLegacyResult, analyze_unrestricted_c11, check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;
use std::collections::BTreeMap;

fn run(body: &str) -> LegacyProgramSummary {
    let contracts = load_resource_contract_sources(&BTreeMap::from([
        (
            "base.qv".into(),
            format!("{}\nextern fn q_observe(p: ptr<mut, u8>) -> unit contract {{ requires resource_contract_v1; ensures normal resource_borrowed(p); ensures normal resource_return_none(); }};", include_str!("fixtures/contracts/q_base.qv")),
        ),
        (
            "untracked.qv".into(),
            include_str!("fixtures/contracts/bound_untracked.qv").into(),
        ),
    ]))
    .expect("fixture declarations load");
    let source = format!(
        "\
struct S {{ int x; struct S *next; }};
void *q_alloc(void); void q_free(void *p); void q_observe(void *p);
void *bu_get(void *p); void unknown(void *p);
{body}"
    );
    let result = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        "subject",
        &contracts,
    )
    .expect("valid fixture C");
    match result {
        UnrestrictedLegacyResult::Safe(certificate) => {
            check_legacy_safety_certificate(&certificate).expect("independent replay");
            certificate.summary
        }
        UnrestrictedLegacyResult::Unsafe(summary) => summary,
        UnrestrictedLegacyResult::Unknown {
            partial: Some(summary),
            ..
        } => summary,
        other => panic!("missing summary: {other:?}"),
    }
}

fn is_safe(body: &str) {
    let summary = run(body);
    assert_eq!(
        summary.functions["subject"].status,
        AnalysisStatus::Safe,
        "{summary:?}"
    );
}

fn refused(body: &str) {
    let summary = run(body);
    assert_ne!(
        summary.functions["subject"].status,
        AnalysisStatus::Safe,
        "{summary:?}"
    );
}

#[test]
fn handed_formal_scalar_member_store_uses_independent_flow_plan() {
    is_safe("void subject(struct S *p) { q_observe(p); p->x = 1; }");
}

#[test]
fn scalar_statement_over_a_borrowed_lookup_has_a_named_null_premise() {
    let summary =
        run("void subject(struct S *root) { struct S *p = root->next; p->x = p->x + 1; }");
    assert_eq!(
        summary.functions["subject"].status,
        AnalysisStatus::Safe,
        "{summary:?}"
    );
    assert!(!summary.functions["subject"].null_premises.is_empty());
    assert!(
        summary.functions["subject"]
            .cfg
            .nodes
            .iter()
            .any(|node| matches!(
                &node.instruction,
                LegacyInstruction::IndirectScalarAccess {
                    typed_alternative: Some(_),
                    ..
                }
            ))
    );
}

#[test]
fn condition_chain_uses_its_own_per_hop_obligations() {
    is_safe("void subject(struct S *p) { q_observe(p); if (p->next->x) { q_observe(p); } }");
}

#[test]
fn released_and_literal_null_and_unresolved_effect_bases_stay_refused() {
    for body in [
        "void subject(void) { struct S *p = q_alloc(); if (!p) return; q_free(p); p->x = 1; }",
        "void subject(void) { struct S *p = 0; p->x = 1; }",
        "void subject(struct S *p) { unknown(p); p->x = 1; }",
        "void subject(struct S *p) { unknown(p); if (p->next->x) q_observe(p); }",
    ] {
        refused(body);
    }
}

#[test]
fn same_type_alias_and_custody_and_pointer_stores_are_not_erased() {
    for body in [
        "void subject(struct S *p, struct S *peer) { q_observe(p); q_free(peer); p->x = 1; }",
        "void subject(void) { struct S *p = q_alloc(); if (!p) return; p->x = 1; }",
        "void subject(struct S *p) { struct S *q = q_alloc(); if (!q) return; p->next = q; }",
    ] {
        refused(body);
    }
}

#[test]
fn old_live_identity_admissions_and_scalar_forgetting_stand() {
    is_safe("void subject(void) { struct S *p = q_alloc(); if (!p) return; p->x = 1; q_free(p); }");
    refused(
        "void subject(struct S *p) { int flag = 0; flag = p->x; if (flag) { void *q = q_alloc(); } }",
    );
}

// Keep the public scalar instruction vocabulary explicit in this fixture.
const _: fn(&LegacyInstruction) -> bool = |instruction| {
    matches!(
        instruction,
        LegacyInstruction::ScalarMemberStore { .. }
            | LegacyInstruction::IndirectScalarAccess { .. }
    )
};

#[test]
fn a_declined_typed_probe_keeps_no_speculative_null_premise() {
    let summary =
        run("void subject(struct S *root) { struct S *p = root->next; unknown(root); p->x = 1; }");
    let row = &summary.functions["subject"];
    assert_ne!(row.status, AnalysisStatus::Safe);
    assert!(row.null_premises.is_empty(), "{row:?}");
    assert!(
        row.cfg.nodes.iter().any(|node| matches!(
            node.instruction,
            LegacyInstruction::ScalarMemberStore {
                typed_alternative: Some(_),
                ..
            }
        )),
        "{row:?}"
    );
}
