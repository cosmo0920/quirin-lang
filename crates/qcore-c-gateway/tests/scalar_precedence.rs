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

#[test]
fn selected_w42_store_premise_is_rechecked_against_caller_held_context() {
    let source = "struct S { int x; struct S *next; };
void subject(struct S *root) { struct S *p = root->next; p->x = 1; }";
    let unit = C11TranslationUnit {
        source,
        ..Default::default()
    };
    let expected = legacy_certificate_expected_context(&unit).expect("independent input context");
    let result = analyze_unrestricted_c11(&unit, "subject", &contracts()).expect("fixture C");
    let UnrestrictedLegacyResult::Safe(certificate) = result else {
        panic!("{result:?}")
    };
    let row = &certificate.summary.functions["subject"];
    assert_eq!(row.null_premises.len(), 1, "{row:?}");
    assert_eq!(row.null_premises[0].variable, "p");
    assert!(row.cfg.nodes.iter().any(|node| matches!(
        node.instruction,
        LegacyInstruction::ScalarMemberStore {
            typed_alternative: Some(_),
            ..
        }
    )));
    check_legacy_safety_certificate_for(&certificate, &expected)
        .expect("declared premise rechecks");
    let mut stripped = certificate;
    stripped
        .summary
        .functions
        .get_mut("subject")
        .expect("subject")
        .null_premises
        .clear();
    assert!(
        check_legacy_safety_certificate_for(&stripped, &expected).is_err(),
        "a selected scalar store cannot hide its new null assumption"
    );
}
