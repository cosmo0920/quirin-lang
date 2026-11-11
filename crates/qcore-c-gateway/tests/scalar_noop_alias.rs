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
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyInstruction,
    LegacyProgramSummary, ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate_for, legacy_certificate_expected_context,
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
fn released_alias_of_scalar_store_cannot_be_safe() {
    let summary = run("void subject(struct S *p, struct S *peer) { q_free(peer); p->x = 1; }");
    let row = &summary.functions["subject"];
    assert_eq!(row.status, AnalysisStatus::Unknown);
    assert!(row.diagnostics.iter().any(|diagnostic| matches!(
        &diagnostic.kind,
        LegacyDiagnosticKind::UnknownAlias { variable } if variable == "p/peer"
    )));
    assert!(row.cfg.nodes.iter().any(|n| matches!(n.instruction,
        LegacyInstruction::IndirectScalarAccess { ref bases, .. } if bases == &["p".to_owned()])));
}

#[test]
fn all_older_scalar_positions_keep_the_alias_read() {
    for (label, body) in [
        (
            "store",
            "void subject(struct S *p, struct S *peer) { q_free(peer); p->x = 1; }",
        ),
        (
            "assignment",
            "int subject(struct S *p, struct S *peer) { q_free(peer); int x = p->x; return x; }",
        ),
        (
            "read",
            "void subject(struct S *p, struct S *peer) { q_free(peer); p->x; }",
        ),
        (
            "return",
            "int subject(struct S *p, struct S *peer) { q_free(peer); return p->x; }",
        ),
        (
            "if",
            "void subject(struct S *p, struct S *peer) { q_free(peer); if (p->x) {} }",
        ),
        (
            "call_return",
            "int data(void); int subject(struct S *p, struct S *peer) { q_free(peer); return p->x + 1; }",
        ),
    ] {
        let s = run(body);
        let row = &s.functions["subject"];
        assert_eq!(row.status, AnalysisStatus::Unknown, "{label}");
        assert!(
            row.diagnostics.iter().any(|diagnostic| matches!(
                &diagnostic.kind,
            LegacyDiagnosticKind::UnknownAlias { variable } if variable == "p/peer"
            )),
            "{label}"
        );
    }
}

#[test]
fn unbound_scalar_base_keeps_existing_safe_admission() {
    let summary = run("void subject(struct S *p) { p->x = 1; }");
    assert_eq!(summary.functions["subject"].status, AnalysisStatus::Safe);
}
