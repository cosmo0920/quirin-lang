//! Bounded independent scalar alternatives. Synthetic q_* declarations are
//! trusted test assumptions; every Safe certificate is independently replayed.
use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyInstruction, LegacyProgramSummary, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11, check_legacy_safety_certificate_for,
    legacy_certificate_expected_context,
};
use qcore_contract::contract_loader::load_resource_contract_sources;
use std::collections::BTreeMap;

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([
        (
            "base.qv".into(),
            format!("{}\nextern fn q_observe(p: ptr<mut, u8>) -> unit contract {{ requires resource_contract_v1; ensures normal resource_borrowed(p); ensures normal resource_return_none(); }};", include_str!("fixtures/contracts/q_base.qv")),
        ),
        (
            "untracked.qv".into(),
            include_str!("fixtures/contracts/bound_untracked.qv").into(),
        ),
    ]))
    .expect("fixture declarations load")
}

fn run(body: &str) -> LegacyProgramSummary {
    let contracts = contracts();
    let source = format!(
        "\
struct S {{ int x; struct S *next; }};
void *q_alloc(void); void q_free(void *p); void q_observe(void *p);
void *bu_get(void *p); void unknown(void *p);
{body}"
    );
    let unit = C11TranslationUnit {
        source: &source,
        ..Default::default()
    };
    let expected = legacy_certificate_expected_context(&unit).expect("caller-held source scope");
    let result = analyze_unrestricted_c11(&unit, "subject", &contracts).expect("valid fixture C");
    match result {
        UnrestrictedLegacyResult::Safe(certificate) => {
            check_legacy_safety_certificate_for(&certificate, &expected)
                .expect("independent source-bound replay");
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

#[test]
fn exact_baseline_observation() {
    let summary = run("void subject(struct S *p, struct S *peer) { q_free(peer); p->x = 1; }");
    let row = &summary.functions["subject"];
    println!("{row:#?}");
    assert_eq!(row.status, AnalysisStatus::Safe);
    assert!(row.cfg.nodes.iter().any(|n| matches!(n.instruction, LegacyInstruction::NoOp)));
}
