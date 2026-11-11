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

//! Source-bound controls for scalar field reads retaining no opaque identity.
use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, ResourceContracts, UnrestrictedLegacyResult,
    analyze_unrestricted_c11, check_legacy_safety_certificate_for,
    legacy_certificate_expected_context,
};

fn run_with(
    body: &str,
    contracts: &ResourceContracts,
) -> qcore_c_gateway::legacy_c::LegacyProgramSummary {
    let source = format!(
        "struct Stream {{ int type; void *pointer; }}; struct Chunk {{ struct Stream *st; }}; void *q_alloc(void); void q_free(void *p); void unknown(void *p); {body}"
    );
    let unit = C11TranslationUnit {
        source: &source,
        ..Default::default()
    };
    let expected = legacy_certificate_expected_context(&unit).expect("held input");
    let result = analyze_unrestricted_c11(&unit, "subject", contracts).expect("valid C");
    match result {
        UnrestrictedLegacyResult::Safe(certificate) => {
            check_legacy_safety_certificate_for(&certificate, &expected).expect("source replay");
            certificate.summary
        }
        UnrestrictedLegacyResult::Unknown {
            partial: Some(summary),
            ..
        } => summary,
        UnrestrictedLegacyResult::Unsafe(summary) => summary,
        other => panic!("{other:?}"),
    }
}

fn run(body: &str) -> qcore_c_gateway::legacy_c::LegacyProgramSummary {
    run_with(body, &ResourceContracts::default())
}

fn assumed_contracts() -> ResourceContracts {
    qcore_contract::contract_loader::load_resource_contract_sources(
        &std::collections::BTreeMap::from([(
            "base.qv".into(),
            include_str!("fixtures/contracts/q_base.qv").into(),
        )]),
    )
    .expect("synthetic trusted contracts")
}

#[test]
fn source_shaped_scalar_chain_assignment() {
    let summary = run(
        "int subject(struct Chunk *ch) { int type; type = ch->st->type; if (type == 1) return 1; else if (type == 0) return 2; return 0; }",
    );
    assert_eq!(
        summary.functions["subject"].status,
        AnalysisStatus::Safe,
        "{summary:#?}"
    );
    assert!(
        summary.functions["subject"]
            .cfg
            .nodes
            .iter()
            .any(|node| matches!(
        &node.instruction,
        qcore_c_gateway::legacy_c::LegacyInstruction::IndirectScalarAccess {
            destination: Some(name), borrowed_destination: false, ..
        } if name == "type"))
    );
}

#[test]
fn pointer_destination_retains_opaque_identity_refusal() {
    let summary = run(
        "int subject(struct Chunk *ch) { void *p = ch->st->pointer; if (p == 0) return 1; return 0; }",
    );
    assert_ne!(
        summary.functions["subject"].status,
        AnalysisStatus::Safe,
        "{summary:#?}"
    );
    assert!(summary.functions["subject"].cfg.nodes.iter().any(|node| matches!(
        &node.instruction, qcore_c_gateway::legacy_c::LegacyInstruction::OpaqueValue { destination, .. } if destination == "p")));
}

#[test]
fn checked_reads_preserve_released_alias_and_unknown_effect_refusals() {
    for body in [
        "int subject(struct Chunk *ch) { q_free(ch); int type = ch->st->type; if (type == 0) return 1; return 0; }",
        "int subject(struct Chunk *ch, struct Chunk *peer) { q_free(peer); int type = ch->st->type; if (type == 0) return 1; return 0; }",
        "int subject(struct Chunk *ch) { unknown(ch); int type = ch->st->type; if (type == 0) return 1; return 0; }",
    ] {
        let summary = run_with(body, &assumed_contracts());
        assert_ne!(
            summary.functions["subject"].status,
            AnalysisStatus::Safe,
            "{summary:#?}"
        );
    }
}

#[test]
fn scalar_read_does_not_retain_an_old_zero_or_prune_the_other_branch() {
    let summary = run_with(
        "int subject(struct Chunk *ch) { int type = 0; type = ch->st->type; if (type) { void *p = q_alloc(); if (!p) return 0; } return 1; }",
        &assumed_contracts(),
    );
    assert_ne!(
        summary.functions["subject"].status,
        AnalysisStatus::Safe,
        "{summary:#?}"
    );
}

#[test]
fn unsupported_volatile_atomic_and_expression_effects_stay_refused() {
    for body in [
        "struct Hot { volatile int value; }; int subject(struct Hot *p) { int value = p->value; if (value == 0) return 1; return 0; }",
        "struct Atomic { _Atomic int value; }; int subject(struct Atomic *p) { int value = p->value; if (value == 0) return 1; return 0; }",
        "int subject(struct Chunk *ch) { int type; type = (unknown(ch), ch->st->type); if (type == 0) return 1; return 0; }",
        "int subject(struct Chunk *ch) { long type = (long) ch->st->pointer; if (type == 0) return 1; return 0; }",
    ] {
        let summary = run(body);
        assert_ne!(
            summary.functions["subject"].status,
            AnalysisStatus::Safe,
            "source: {body}\n{summary:#?}"
        );
    }
}

#[test]
fn inherited_volatile_destinations_keep_the_old_opaque_route() {
    for body in [
        "int subject(struct Chunk *ch) { volatile int type = ch->st->type; if (type == 0) return 1; return 0; }",
        "int subject(struct Chunk *ch) { _Atomic int type = ch->st->type; if (type == 0) return 1; return 0; }",
    ] {
        let summary = run(body);
        assert!(
            summary.functions["subject"]
                .cfg
                .nodes
                .iter()
                .any(|node| matches!(
            &node.instruction,
            qcore_c_gateway::legacy_c::LegacyInstruction::OpaqueValue { destination, .. }
            if destination == "type"))
        );
        assert!(
            !summary.functions["subject"]
                .cfg
                .nodes
                .iter()
                .any(|node| matches!(
            &node.instruction,
            qcore_c_gateway::legacy_c::LegacyInstruction::IndirectScalarAccess {
                destination: Some(destination), ..
            } if destination == "type"))
        );
    }
}
