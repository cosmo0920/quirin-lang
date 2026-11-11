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

use super::*;

const LEAF: &str = "struct Ring { struct Ring *next, *prev; };\nstatic void reset(struct Ring *p) { if (p == 0) return; p->next = p; p->prev = p; }\n";
fn contracts() -> ResourceContracts {
    let mut c = ResourceContracts::default();
    c.insert_contract(
        "make",
        ResourceFunctionContract::new(vec![], ResourceReturnMode::Owned { nullable: false }),
    );
    c.insert_contract(
        "drop",
        ResourceFunctionContract::new(
            vec![ResourceParameterMode::Released { nullable: false }],
            ResourceReturnMode::None,
        ),
    );
    c
}
fn run(source: &str) -> Option<UnrestrictedLegacyResult> {
    match crate::clang_command() {
        Ok(_) => {}
        Err(reason) => {
            assert!(std::env::var_os("QCORE_CLANG").is_none(), "{reason}");
            eprintln!("Clang unavailable: {reason}");
            return None;
        }
    }
    Some(
        analyze_unrestricted_c11(
            &C11TranslationUnit {
                source,
                ..Default::default()
            },
            "subject",
            &contracts(),
        )
        .expect("fixture"),
    )
}
fn summary(r: &UnrestrictedLegacyResult) -> &LegacyProgramSummary {
    match r {
        UnrestrictedLegacyResult::Safe(c) => &c.summary,
        UnrestrictedLegacyResult::Unsafe(s) => s,
        UnrestrictedLegacyResult::Unknown {
            partial: Some(s), ..
        } => s,
        other => panic!("{other:?}"),
    }
}
fn owned() -> AbstractState {
    AbstractState {
        pointers: BTreeMap::from([(
            "p".into(),
            AbstractPointer {
                targets: BTreeSet::from([7]),
                ..Default::default()
            },
        )]),
        resources: BTreeMap::from([(7, LIVE)]),
        ..Default::default()
    }
}

#[test]
fn owned_root_and_embedded_targets_replay_their_actual_write_sets() {
    for (records, ty, actual, path) in [
        ("", "Ring", "p", vec![]),
        (
            "struct Owner { struct Ring ring; };",
            "Owner",
            "&p->ring",
            vec!["ring"],
        ),
        (
            "struct Inner { struct Ring ring; }; struct Owner { struct Inner inner; };",
            "Owner",
            "&p->inner.ring",
            vec!["inner", "ring"],
        ),
    ] {
        let source = format!(
            "{LEAF}{records}\nstruct {ty} *make(void); void drop(struct {ty} *p);\nvoid subject(void) {{ struct {ty} *p = make(); reset({actual}); drop(p); }}"
        );
        let Some(result) = run(&source) else { return };
        let UnrestrictedLegacyResult::Safe(certificate) = result else {
            panic!("{actual}: {result:?}")
        };
        assert!(
            certificate.summaries.is_empty(),
            "effect must not escape as a neutral summary"
        );
        let cfg = &certificate.summary.functions["subject"].cfg;
        let effect = cfg
            .nodes
            .iter()
            .find_map(|n| match &n.instruction {
                LegacyInstruction::TrackedSelfInit {
                    base, path, writes, ..
                } => Some((base, path, writes)),
                _ => None,
            })
            .expect("retained write set");
        assert_eq!(effect.0, "p");
        assert_eq!(effect.1, &path);
        assert_eq!(effect.2, &["next", "prev"]);
        check_legacy_safety_certificate(&certificate).expect("independent source replay");
        let mut forged = certificate.clone();
        let node = forged
            .summary
            .functions
            .get_mut("subject")
            .unwrap()
            .cfg
            .nodes
            .iter_mut()
            .find(|n| matches!(n.instruction, LegacyInstruction::TrackedSelfInit { .. }))
            .unwrap();
        if let LegacyInstruction::TrackedSelfInit { writes, .. } = &mut node.instruction {
            writes.clear();
        }
        assert!(
            check_legacy_safety_certificate(&forged).is_err(),
            "a forged empty write set cannot self-authorize"
        );
    }
}

#[test]
fn source_uncertainty_and_escaping_effects_keep_the_old_call() {
    for (records, actual) in [
        (
            "struct Other { struct Other *next, *prev; }; struct Owner { struct Other ring; };",
            "&p->ring",
        ),
        ("struct Owner { struct Ring *ring; };", "p->ring"),
        ("struct Owner { struct Ring ring[2]; };", "&p->ring[0]"),
        (
            "struct Owner { struct Ring ring; };",
            "(struct Ring *)&p->ring",
        ),
        ("struct Owner { volatile struct Ring ring; };", "&p->ring"),
        (
            "union Inner { struct Ring ring; int n; }; struct Owner { union Inner in; };",
            "&p->in.ring",
        ),
    ] {
        let source = format!(
            "{LEAF}{records}\nstruct Owner *make(void); void drop(struct Owner *p); void subject(void) {{ struct Owner *p = make(); reset({actual}); drop(p); }}"
        );
        let Some(r) = run(&source) else { return };
        let row = &summary(&r).functions["subject"];
        assert_ne!(row.status, AnalysisStatus::Safe, "{actual}");
        assert!(!has_call(&row.cfg), "{actual}");
    }
    for leaf in [
        "p->next = p; reset(p);",
        "p->next = p; p->prev = 0;",
        "struct Ring *q = p; p->next = q;",
        "p->next = (struct Ring *)p;",
    ] {
        let source = format!(
            "struct Ring {{ struct Ring *next, *prev; }}; void reset(struct Ring *p) {{ {leaf} }} struct Ring *make(void); void drop(struct Ring *p); void subject(void) {{ struct Ring *p = make(); reset(p); drop(p); }}"
        );
        let Some(r) = run(&source) else { return };
        assert!(!has_call(&summary(&r).functions["subject"].cfg), "{leaf}");
    }
    let source = format!(
        "{LEAF} void (*callback)(struct Ring *) = reset; struct Ring *make(void); void drop(struct Ring *p); void subject(void) {{ struct Ring *p = make(); reset(p); drop(p); }}"
    );
    let Some(r) = run(&source) else { return };
    assert!(!has_call(&summary(&r).functions["subject"].cfg));
}

#[test]
fn borrowed_entries_and_callers_of_effectful_rows_gain_no_summary() {
    let source = format!("{LEAF}void subject(struct Ring *p) {{ struct Ring *q = p; reset(q); }}");
    let Some(r) = run(&source) else { return };
    assert_ne!(
        summary(&r).functions["subject"].status,
        AnalysisStatus::Safe
    );
    let source = format!(
        "{LEAF}struct Ring *make(void); void drop(struct Ring *p); void build(void) {{ struct Ring *p = make(); reset(p); drop(p); }} void subject(void) {{ build(); }}"
    );
    let Some(r) = run(&source) else { return };
    assert_eq!(summary(&r).functions["build"].status, AnalysisStatus::Safe);
    assert_ne!(
        summary(&r).functions["subject"].status,
        AnalysisStatus::Safe
    );
}

#[test]
fn initialization_does_not_discharge_or_revive_the_parent() {
    for body in [
        "reset(p);",
        "drop(p); reset(p);",
        "reset(p); drop(p); drop(p);",
    ] {
        let source = format!(
            "{LEAF}struct Ring *make(void); void drop(struct Ring *p); void subject(void) {{ struct Ring *p = make(); {body} }}"
        );
        let Some(r) = run(&source) else { return };
        assert_ne!(
            summary(&r).functions["subject"].status,
            AnalysisStatus::Safe,
            "{body}"
        );
    }
}

#[test]
fn alias_member_facts_are_forgotten_without_losing_any_custody_or_plain_alias() {
    let mut s = owned();
    let plain = s.pointers["p"].clone();
    let mut interior = plain.clone();
    interior.derivation = Derivation::Identity.step(true);
    s.pointers.insert("q".into(), plain.clone());
    s.pointers.insert("inner".into(), interior.clone());
    for key in ["p->next", "q->ring.next", "inner->next", "(*q).prev"] {
        s.pointers.insert(key.into(), plain.clone());
    }
    s.pointers
        .insert("other->next".into(), AbstractPointer::default());
    s.resources.insert(9, LIVE);
    let out = apply(&s, "p").expect("known self facts carry no different child");
    assert_eq!(out.resources, s.resources);
    assert_eq!(out.nullable_sites, s.nullable_sites);
    for key in ["p", "q", "inner", "other->next"] {
        assert_eq!(out.pointers[key], s.pointers[key]);
    }
    for key in ["p->next", "q->ring.next", "inner->next", "(*q).prev"] {
        assert!(!out.pointers.contains_key(key), "{key}");
    }
    let mut remainder = out.clone();
    remainder.pointers = s.pointers.clone();
    assert_eq!(remainder, s);
}

#[test]
fn old_children_unknown_member_facts_and_every_nonempty_custody_frame_refuse() {
    let mut s = owned();
    s.pointers.insert(
        "p->next".into(),
        AbstractPointer {
            targets: BTreeSet::from([8]),
            ..Default::default()
        },
    );
    assert!(apply(&s, "p").is_none());
    s.pointers.get_mut("p->next").unwrap().targets.clear();
    s.pointers.get_mut("p->next").unwrap().unknown = true;
    assert!(apply(&s, "p").is_none());
    let mut states = vec![];
    let mut s = owned();
    s.field_links
        .insert(7, BTreeMap::from([("next".into(), BTreeSet::from([8]))]));
    states.push(s);
    let mut s = owned();
    s.contained.insert(7, BTreeSet::from([8]));
    states.push(s);
    let mut s = owned();
    s.published.insert(7);
    states.push(s);
    let mut s = owned();
    s.collections
        .insert(7, crate::indexed_collection::Collection::Unknown);
    states.push(s);
    let mut s = owned();
    s.unresolved_effect_sites.insert(7);
    states.push(s);
    let mut s = owned();
    s.unmodeled = true;
    states.push(s);
    let mut s = owned();
    s.widened = true;
    states.push(s);
    for state in states {
        assert!(apply(&state, "p").is_none(), "{state:?}");
    }
}

// Same ordered vectors as TrackedSelfInit.flowVectors in Lean.
#[test]
fn shared_flow_vectors() {
    let initial = owned();
    let mut states = vec![initial.clone()];
    for status in [None, Some(RELEASED), Some(ESCAPED), Some(LIVE | RELEASED)] {
        let mut s = initial.clone();
        if let Some(status) = status {
            s.resources.insert(7, status);
        } else {
            s.resources.clear();
        }
        states.push(s);
    }
    let mut s = initial.clone();
    s.pointers.get_mut("p").unwrap().unknown = true;
    states.push(s);
    let mut s = initial.clone();
    s.pointers.get_mut("p").unwrap().may_be_null = true;
    states.push(s);
    let mut s = initial.clone();
    s.nullable_sites.insert(7);
    states.push(s);
    let mut s = initial.clone();
    s.pointers.get_mut("p").unwrap().derivation = Derivation::Identity.step(true);
    states.push(s);
    let mut s = initial.clone();
    s.pointers.get_mut("p").unwrap().targets.insert(8);
    states.push(s);
    let mut s = initial.clone();
    s.pointers.get_mut("p").unwrap().targets.clear();
    states.push(s);
    let mut s = initial.clone();
    s.published.insert(7);
    states.push(s);
    let mut s = initial;
    s.pointers
        .insert("p->next".into(), AbstractPointer::default());
    states.push(s);
    let actual = states
        .iter()
        .map(|s| apply(s, "p").is_some())
        .collect::<Vec<_>>();
    let expected = [
        true, false, false, false, false, false, false, false, false, false, false, false, true,
    ];
    assert_eq!(actual, expected);
    let lean = include_str!("../../../../proof/QCore/TrackedSelfInit.lean");
    assert!(
        lean.contains(
            "[true,false,false,false,false,false,false,false,false,false,false,false,true]"
        )
    );
}

#[test]
fn unreachable_effects_keep_old_inference_and_declared_calls_keep_precedence() {
    let source = format!(
        "{LEAF}struct Ring *make(void); void drop(struct Ring *p); void subject(void) {{ return; struct Ring *p = make(); reset(p); drop(p); }}"
    );
    if crate::clang_command().is_err() {
        assert!(
            std::env::var_os("QCORE_CLANG").is_none(),
            "invalid explicit Clang"
        );
        return;
    }
    {
        let _scope = crate::typed_access_path::enter_source(&source, CCompilerDialect::Gnu17, None);
        let tokens = tokenize(&source);
        let cfg = build_function_cfg(&tokens, "subject", &contracts()).unwrap();
        assert!(!has_call(&cfg));
        assert!(cfg.nodes.iter().any(|n| matches!(&n.instruction,
            LegacyInstruction::Unknown { description, .. } if description.contains("reset"))));
        let derivation = crate::legacy_summary::derive_custody_summaries(
            &tokens,
            &["reset".into(), "subject".into()],
            &contracts(),
        );
        assert!(derivation.records().iter().any(|r| r.function == "subject"
            && r.contract == ResourceFunctionContract::new(vec![], ResourceReturnMode::None)));
    }
    let source = format!(
        "{LEAF}struct Ring *make(void); void drop(struct Ring *p); void subject(void) {{ struct Ring *p = make(); reset(p); drop(p); }}"
    );
    let mut c = contracts();
    c.insert_contract(
        "reset",
        ResourceFunctionContract::new(
            vec![ResourceParameterMode::Borrowed],
            ResourceReturnMode::None,
        ),
    );
    let r = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        "subject",
        &c,
    )
    .unwrap();
    assert!(!has_call(&summary(&r).functions["subject"].cfg));
    assert!(summary(&r).functions["subject"].cfg.nodes.iter().any(|n| matches!(&n.instruction, LegacyInstruction::ContractCall { function, .. } if function == "reset")));
}

#[test]
fn source_target_vectors_are_bound_to_the_lean_table() {
    let mut answers = vec![];
    for (records, actual) in [
        ("struct Owner { struct Ring ring; };", "&p->ring"),
        ("struct Owner { struct Ring *ring; };", "p->ring"),
        ("struct Owner { struct Ring ring[2]; };", "&p->ring[0]"),
        (
            "struct Owner { struct Ring ring; };",
            "(struct Ring *)&p->ring",
        ),
        ("struct Owner { volatile struct Ring ring; };", "&p->ring"),
        (
            "union Inner { struct Ring ring; int n; }; struct Owner { union Inner in; };",
            "&p->in.ring",
        ),
    ] {
        let source = format!(
            "{LEAF}{records}struct Owner *make(void); void drop(struct Owner *p); void subject(void) {{ struct Owner *p = make(); reset({actual}); drop(p); }}"
        );
        let Some(r) = run(&source) else { return };
        answers.push(has_call(&summary(&r).functions["subject"].cfg));
    }
    assert_eq!(answers, [true, false, false, false, false, false]);
    assert!(
        include_str!("../../../../proof/QCore/TrackedSelfInit.lean")
            .contains("sourceTargetVectors = [true,false,false,false,false,false]")
    );
}

#[test]
fn tracked_rows_have_no_export_and_a_forged_neutral_import_fails_replay() {
    let source = format!(
        "{LEAF}struct Ring *make(void); void drop(struct Ring *p); void subject(void) {{ struct Ring *p = make(); reset(p); drop(p); }}"
    );
    let Some(checked) = run(&source) else { return };
    assert_eq!(
        summary(&checked).functions["subject"].status,
        AnalysisStatus::Safe
    );
    let unit = prepare_translation_unit(&C11TranslationUnit {
        source: &source,
        function_scope: LegacyFunctionScope::MainSourceDefinitions,
        ..Default::default()
    });
    let wanted = BTreeSet::from(["subject".to_owned()]);
    let digest = "a".repeat(64);
    assert!(
        export_cross_unit_homes(&unit, &contracts(), &wanted, &digest)
            .unwrap()
            .is_empty()
    );
    let neutral_source = "struct Ring { struct Ring *next; }; void subject(void) { return; }";
    let neutral = prepare_translation_unit(&C11TranslationUnit {
        source: neutral_source,
        function_scope: LegacyFunctionScope::MainSourceDefinitions,
        ..Default::default()
    });
    let mut exported = export_cross_unit_homes(&neutral, &contracts(), &wanted, &digest)
        .unwrap()
        .remove(0);
    let request = LegacyCertificateRequest {
        include_paths: unit.include_paths().to_vec(),
        definitions: unit.definitions().to_vec(),
        preprocessor_options: unit.preprocessor_options().to_vec(),
        input: unit.input().clone(),
    };
    let scope = build_certificate_scope_from_evidence(
        unit.dialect(),
        LegacyFunctionScope::MainSourceDefinitions,
        unit.preprocessed_source().unwrap(),
        Some(unit.raw_source()),
        unit.line_marked_source(),
        request.clone(),
    )
    .unwrap();
    exported.home.preprocessed_source = unit.preprocessed_source().unwrap().to_owned();
    exported.home.raw_source = unit.raw_source().to_owned();
    exported.home.line_marked_source = unit.line_marked_source().unwrap().to_owned();
    exported.home.request = request;
    exported.home.input_identity = scope.input_identity;
    let mut contracts = contracts();
    contracts.set_inferred_imports(BTreeMap::from([("subject".to_owned(), exported.contract)]));
    contracts.set_cross_unit_import_identities(BTreeMap::from([(
        "subject".to_owned(),
        exported.home.import_identity(),
    )]));
    contracts.set_link_resolution_sha256(Some(digest));
    let error = cross_unit::check_imports(
        "void subject(void); void caller(void) {}",
        &contracts,
        &[exported.home],
    )
    .unwrap_err();
    assert!(error.contains("no validated W4 summary"), "{error}");
}

#[test]
fn initialized_owned_result_can_escape_and_plain_aliases_still_share_parent_release() {
    let source = format!(
        "{LEAF}struct Ring *make(void); struct Ring *subject(void) {{ struct Ring *p = make(); reset(p); reset(p); return p; }}"
    );
    let Some(r) = run(&source) else { return };
    assert_eq!(
        summary(&r).functions["subject"].status,
        AnalysisStatus::Safe
    );
    let source = format!(
        "{LEAF}struct Ring *make(void); void drop(struct Ring *p); void subject(void) {{ struct Ring *p = make(); struct Ring *q = p; reset(q); drop(p); drop(q); }}"
    );
    let Some(r) = run(&source) else { return };
    assert_ne!(
        summary(&r).functions["subject"].status,
        AnalysisStatus::Safe
    );
    assert!(
        summary(&r).functions["subject"]
            .diagnostics
            .iter()
            .any(|d| matches!(d.kind, LegacyDiagnosticKind::DoubleRelease { .. }))
    );
}
