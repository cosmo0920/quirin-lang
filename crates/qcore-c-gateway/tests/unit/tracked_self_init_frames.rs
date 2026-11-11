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

const SOURCE: &str = "struct Ring { struct Ring *next, *prev; }; struct Item { int n; }; struct Owner { struct Ring head; struct Item *payload; }; static void reset(struct Ring *p) { p->next=p; p->prev=p; } struct Owner *make_owner(void); struct Item *make_child(void); void drop_owner(void *p); void drop_child(void *p); void inspect(struct Item *p);";
const START: &str = "struct Owner *p=make_owner(); struct Item *c=make_child(); p->payload=c;";

fn contracts() -> ResourceContracts {
    let mut c = ResourceContracts::default();
    for name in ["make_owner", "make_child"] {
        c.insert_contract(
            name,
            ResourceFunctionContract::new([], ResourceReturnMode::Owned { nullable: false }),
        );
    }
    for name in ["drop_owner", "drop_child"] {
        c.insert_contract(
            name,
            ResourceFunctionContract::new(
                [ResourceParameterMode::Released { nullable: false }],
                ResourceReturnMode::None,
            ),
        );
    }
    c.insert_contract(
        "inspect",
        ResourceFunctionContract::new([ResourceParameterMode::Borrowed], ResourceReturnMode::None),
    );
    c
}
fn analyze(source: &str, c: &ResourceContracts) -> Option<UnrestrictedLegacyResult> {
    if let Err(reason) = crate::clang_command() {
        assert!(std::env::var_os("QCORE_CLANG").is_none(), "{reason}");
        eprintln!("Clang unavailable: {reason}");
        return None;
    }
    Some(
        analyze_unrestricted_c11(
            &C11TranslationUnit {
                source,
                ..Default::default()
            },
            "subject",
            c,
        )
        .expect("C fixture"),
    )
}
fn summary(r: &UnrestrictedLegacyResult) -> &LegacyProgramSummary {
    match r {
        UnrestrictedLegacyResult::Safe(c) => &c.summary,
        UnrestrictedLegacyResult::Unsafe(s)
        | UnrestrictedLegacyResult::Unknown {
            partial: Some(s), ..
        } => s,
        other => panic!("{other:?}"),
    }
}
fn protected(r: &UnrestrictedLegacyResult) -> Vec<&String> {
    summary(r).functions["subject"]
        .cfg
        .nodes
        .iter()
        .filter_map(|n| {
            if let LegacyInstruction::TrackedSelfInit {
                protected_fields, ..
            } = &n.instruction
            {
                Some(protected_fields.iter())
            } else {
                None
            }
        })
        .flatten()
        .collect()
}

#[test]
fn real_embedded_and_root_slots_keep_their_children_and_replay() {
    for source in [
        format!("{SOURCE} void subject(void) {{ {START} reset(&p->head); inspect(p->payload); drop_child(c); drop_owner(p); }}"),
        format!("{} void subject(void) {{ {START} reset(&p->inner.head); inspect(p->payload); drop_child(c); drop_owner(p); }}", SOURCE.replace("struct Owner { struct Ring head;", "struct Inner { struct Ring head; }; struct Owner { struct Inner inner;")),
        "struct Item { int n; }; struct Ring { struct Ring *next,*prev; struct Item *payload; }; static void reset(struct Ring *p) { p->next=p; p->prev=p; } struct Ring *make_owner(void); struct Item *make_child(void); void drop_owner(void*); void drop_child(void*); void inspect(struct Item*); void subject(void) { struct Ring*p=make_owner(); struct Item*c=make_child(); p->payload=c; reset(p); inspect(p->payload); drop_child(c); drop_owner(p); }".to_owned(),
    ] {
        let Some(r) = analyze(&source, &contracts()) else { return };
        let UnrestrictedLegacyResult::Safe(certificate) = &r else { panic!("{r:?}") };
        assert!(protected(&r).iter().any(|f| f.as_str()=="payload"));
        assert!(certificate.summaries.is_empty());
        check_legacy_safety_certificate(certificate).expect("independent source/CFG replay");
        let mut forged = certificate.clone();
        for n in &mut forged.summary.functions.get_mut("subject").unwrap().cfg.nodes {
            if let LegacyInstruction::TrackedSelfInit { protected_fields, .. } = &mut n.instruction { protected_fields.push("head".into()); }
        }
        assert!(check_legacy_safety_certificate(&forged).is_err());
    }
}

#[test]
fn source_retyping_hidden_rebinding_loaded_assignments_and_old_alias_stores_refuse() {
    for addition in [
        "struct Other { struct Item *payload; struct Ring head; }; struct Other *q=(struct Other*)p; q->payload=c; q=0;",
        "struct Owner **slot=&p;",
        "struct Item *d=p->payload;",
        "struct Owner *q=p;",
        "unprovided(p);",
    ] {
        let source = format!(
            "{SOURCE} void unprovided(struct Owner*); void subject(void) {{ {START} {addition} reset(&p->head); inspect(p->payload); drop_child(c); drop_owner(p); }}"
        );
        let Some(r) = analyze(&source, &contracts()) else {
            return;
        };
        if addition == "struct Owner *q=p;" {
            assert!(
                !protected(&r).is_empty(),
                "same-type alias proof does not override the older store gate"
            );
        } else {
            assert!(protected(&r).is_empty(), "{addition}: {r:?}");
        }
        assert_ne!(
            summary(&r).functions["subject"].status,
            AnalysisStatus::Safe,
            "{addition}"
        );
    }
    let mut c = contracts();
    c.insert_contract(
        "change",
        ResourceFunctionContract::new(
            [ResourceParameterMode::Out { nullable: false }],
            ResourceReturnMode::None,
        ),
    );
    let source = format!(
        "{SOURCE} void change(struct Item**); void subject(void) {{ {START} change(&c); reset(&p->head); drop_child(c); drop_owner(p); }}"
    );
    let Some(r) = analyze(&source, &c) else {
        return;
    };
    assert!(protected(&r).is_empty());
    assert_ne!(
        summary(&r).functions["subject"].status,
        AnalysisStatus::Safe
    );
    let mut c = contracts();
    c.insert_contract(
        "look",
        ResourceFunctionContract::new([ResourceParameterMode::Borrowed], ResourceReturnMode::None),
    );
    let source = format!(
        "{SOURCE} void look(struct Owner**); void subject(void) {{ {START} look(&p); reset(&p->head); drop_child(c); drop_owner(p); }}"
    );
    let Some(r) = analyze(&source, &c) else {
        return;
    };
    assert!(
        protected(&r).is_empty(),
        "pointer address guard must decline despite an ordinary Borrowed call"
    );
    assert_ne!(
        summary(&r).functions["subject"].status,
        AnalysisStatus::Safe
    );
}

#[test]
fn shallow_release_partial_escape_and_later_lifetime_violations_are_not_erased() {
    for ending in [
        "drop_owner(p);",
        "drop_child(c); drop_child(c); drop_owner(p);",
        "drop_owner(p); inspect(p->payload); drop_child(c);",
    ] {
        let source = format!("{SOURCE} void subject(void) {{ {START} reset(&p->head); {ending} }}");
        let Some(r) = analyze(&source, &contracts()) else {
            return;
        };
        assert!(!protected(&r).is_empty());
        assert_ne!(
            summary(&r).functions["subject"].status,
            AnalysisStatus::Safe,
            "{ending}"
        );
        if ending == "drop_owner(p);" {
            assert!(
                summary(&r).functions["subject"]
                    .diagnostics
                    .iter()
                    .any(|d| matches!(d.kind, LegacyDiagnosticKind::Leak { .. })),
                "{r:?}"
            );
        }
    }
    for (signature, ending, safe) in [
        (
            "void subject(int n)",
            "drop_child(a); drop_child(b); drop_owner(p);",
            true,
        ),
        ("struct Owner *subject(int n)", "return p;", false),
    ] {
        let source = format!(
            "{SOURCE} {signature} {{ struct Owner*p=make_owner(); struct Item*a=make_child(); struct Item*b=make_child(); if(n) p->payload=a; else p->payload=b; reset(&p->head); {ending} }}"
        );
        let Some(r) = analyze(&source, &contracts()) else {
            return;
        };
        assert!(!protected(&r).is_empty());
        assert_eq!(
            summary(&r).functions["subject"].status == AnalysisStatus::Safe,
            safe,
            "{r:?}"
        );
    }
}

fn initial() -> AbstractState {
    let pointer = |site| AbstractPointer {
        targets: BTreeSet::from([site]),
        ..Default::default()
    };
    AbstractState {
        pointers: BTreeMap::from([("p".into(), pointer(7)), ("p->payload".into(), pointer(8))]),
        resources: BTreeMap::from([(7, LIVE), (8, LIVE)]),
        field_links: BTreeMap::from([(
            7,
            BTreeMap::from([("payload".into(), BTreeSet::from([8]))]),
        )]),
        ..Default::default()
    }
}
fn framed(s: &AbstractState) -> Option<AbstractState> {
    apply_frame(
        s,
        "p",
        &["payload".into()],
        &["p".into(), "q".into()],
        &TransferContext::default(),
    )
}

#[test]
fn the_complete_ledger_partial_links_and_plain_or_derived_aliases_survive() {
    let mut s = initial();
    assert!(
        apply(&s, "p").is_none(),
        "old route remains default-frame only"
    );
    s.partial_links = s.field_links.clone();
    let p = s.pointers["p"].clone();
    s.pointers.insert("q".into(), p.clone());
    s.pointers
        .insert("q->payload".into(), s.pointers["p->payload"].clone());
    let mut derived = p.clone();
    derived.derivation = Derivation::Identity.step(true);
    s.pointers.insert("inner".into(), derived);
    s.pointers.insert("inner->next".into(), p.clone());
    s.pointers.insert("p->head.next".into(), p);
    s.pointers.insert(
        "other->payload".into(),
        AbstractPointer {
            unknown: true,
            ..Default::default()
        },
    );
    let out = framed(&s).expect("source-sealed compatible aliases and partial subset");
    for key in ["p", "q", "inner", "p->payload", "q->payload"] {
        assert_eq!(out.pointers[key], s.pointers[key]);
    }
    for key in ["inner->next", "p->head.next", "other->payload"] {
        assert!(!out.pointers.contains_key(key));
    }
    let mut restored = out;
    restored.pointers = s.pointers.clone();
    assert_eq!(restored, s, "no other state field changes");
    for root_previous in [true, false] {
        let mut s = initial();
        let old = if root_previous { 7 } else { 8 };
        let previous = crate::loop_generations::previous(old);
        s.resources.remove(&old);
        s.resources.insert(previous, LIVE);
        if root_previous {
            s.pointers.get_mut("p").unwrap().targets = BTreeSet::from([previous]);
            let links = s.field_links.remove(&7).unwrap();
            s.field_links.insert(previous, links);
        } else {
            s.pointers.get_mut("p->payload").unwrap().targets = BTreeSet::from([previous]);
            s.field_links
                .get_mut(&7)
                .unwrap()
                .insert("payload".into(), BTreeSet::from([previous]));
        }
        assert!(
            framed(&s).is_some(),
            "a precise previous generation is still one known object"
        );
    }
}

#[test]
fn unsupported_frames_and_malformed_partial_relations_keep_refusing() {
    let base = initial();
    let mut states = vec![];
    for status in [RELEASED, ESCAPED, LIVE | RELEASED] {
        let mut s = base.clone();
        s.resources.insert(8, status);
        states.push(s);
    }
    let mut s = base.clone();
    s.nullable_sites.insert(8);
    states.push(s);
    let mut s = base.clone();
    let older = crate::loop_generations::older(8);
    s.resources.insert(older, LIVE);
    s.pointers.get_mut("p->payload").unwrap().targets = BTreeSet::from([older]);
    s.field_links
        .get_mut(&7)
        .unwrap()
        .insert("payload".into(), BTreeSet::from([older]));
    states.push(s);
    let mut s = base.clone();
    s.partial_links
        .insert(7, BTreeMap::from([("payload".into(), BTreeSet::from([9]))]));
    states.push(s);
    let mut s = base.clone();
    s.field_links
        .insert(9, BTreeMap::from([("payload".into(), BTreeSet::from([8]))]));
    states.push(s);
    let mut s = base.clone();
    s.contained.insert(7, BTreeSet::from([8]));
    states.push(s);
    let mut s = base.clone();
    s.unresolved_effect_sites.insert(8);
    states.push(s);
    let mut s = base.clone();
    s.unmodeled = true;
    states.push(s);
    let mut s = base.clone();
    s.widened = true;
    states.push(s);
    let mut s = base.clone();
    s.pointers.insert("*out".into(), AbstractPointer::default());
    states.push(s);
    let mut s = base.clone();
    s.pointers
        .insert("p->head.next".into(), s.pointers["p->payload"].clone());
    states.push(s);
    let mut s = base.clone();
    s.pointers
        .insert("unsealed_alias".into(), s.pointers["p"].clone());
    states.push(s);
    for s in states {
        assert!(framed(&s).is_none(), "{s:?}");
    }
    for kind in ResourceKind::ALL.into_iter().filter(|k| !k.is_heap()) {
        for site in [7, 8] {
            let mut ctx = TransferContext::default();
            ctx.site_kinds.insert(site, kind);
            assert!(apply_frame(&base, "p", &["payload".into()], &["p".into()], &ctx).is_none());
        }
    }
    let mut ctx = TransferContext::default();
    ctx.caller_sites.insert(7);
    assert!(apply_frame(&base, "p", &["payload".into()], &["p".into()], &ctx).is_none());
    let mut ctx = TransferContext::default();
    ctx.sanctioned.insert("*out");
    assert!(apply_frame(&base, "p", &["payload".into()], &["p".into()], &ctx).is_none());
}

#[test]
fn a_proved_sibling_does_not_authorize_an_overlapping_owning_slot() {
    let source = "struct Ring { struct Ring *next,*prev,*payload; }; void reset(struct Ring*p) { p->next=p; p->prev=p; } struct Ring*make_owner(void); struct Ring*make_child(void); void drop_owner(void*); void drop_child(void*); void subject(void) { struct Ring*p=make_owner(); struct Ring*c=make_child(); p->next=c; p->payload=0; reset(p); drop_child(c); drop_owner(p); }";
    let Some(r) = analyze(source, &contracts()) else {
        return;
    };
    assert!(
        protected(&r).iter().any(|f| f.as_str() == "payload"),
        "must reach source layout admission"
    );
    assert_ne!(
        summary(&r).functions["subject"].status,
        AnalysisStatus::Safe,
        "the actual child link overlaps next"
    );
}

#[test]
fn a_complete_parent_escape_has_no_ordinary_or_cross_unit_summary() {
    let source =
        format!("{SOURCE} struct Owner *subject(void) {{ {START} reset(&p->head); return p; }}");
    let c = contracts();
    let Some(r) = analyze(&source, &c) else {
        return;
    };
    let UnrestrictedLegacyResult::Safe(certificate) = r else {
        panic!("{r:?}")
    };
    assert!(certificate.summaries.is_empty());
    check_legacy_safety_certificate(&certificate).unwrap();
    let unit = prepare_translation_unit(&C11TranslationUnit {
        source: &source,
        function_scope: LegacyFunctionScope::MainSourceDefinitions,
        ..Default::default()
    });
    assert!(
        export_cross_unit_homes(
            &unit,
            &c,
            &BTreeSet::from(["subject".into()]),
            &"a".repeat(64)
        )
        .unwrap()
        .is_empty()
    );
}

#[test]
fn a_guarded_interface_cannot_supply_the_ordinary_call_closure() {
    let text = r#"module contract; extern fn touch(p: ptr<mut, u8>) -> unit contract {
      requires resource_contract_v1;
      requires resource_premise(s3_condition_snapshot_stable, "Every member read by this function's condition bindings preserves its entry value until each guarded use.");
      ensures normal resource_borrowed(p); ensures normal resource_return_none();
      quant #active = (#p.flag != 0);
      quant #active in { requires resource_premise(entry_protocol, "The entry protocol holds"); }: [[local]];
    };"#;
    let mut c = crate::contract_loader::load_resource_contract_sources(&BTreeMap::from([(
        "guarded.qv".into(),
        text.into(),
    )]))
    .unwrap();
    assert!(c.has_guarded_interface("touch"));
    for (name, contract) in contracts().functions() {
        c.insert_contract(name, contract.clone());
    }
    let source = format!(
        "{} void touch(struct Owner*); void subject(void) {{ {START} touch(p); reset(&p->head); drop_child(c); drop_owner(p); }}",
        SOURCE.replace("struct Owner {", "struct Owner { int flag;")
    );
    let Some(r) = analyze(&source, &c) else {
        return;
    };
    assert!(protected(&r).is_empty());
    assert_ne!(
        summary(&r).functions["subject"].status,
        AnalysisStatus::Safe
    );
}

#[test]
fn a_declined_optional_probe_drops_its_unevaluated_call_consultation() {
    let source = format!(
        "{SOURCE} int proof_only(struct Owner*); void subject(void) {{ {START} unsigned long n=sizeof(proof_only(p)); reset(&p->head); drop_child(c); drop_owner(p); }}"
    );
    let c = contracts();
    let Some(r) = analyze(&source, &c) else {
        return;
    };
    assert!(
        protected(&r).is_empty(),
        "optional source closure actually declines"
    );
    let _scope = crate::typed_access_path::enter_source(&source, CCompilerDialect::Gnu17, None);
    let stream = tokenize(&source);
    let index = StatementIndex::read("subject").unwrap();
    let (ty, declaration, writes) = self_link_init::checked_effect(&stream, "reset", &c).unwrap();
    let mut work = vec![index.function.as_ref()];
    let place = loop {
        let node = work.pop().expect("initializer call");
        if kind(node) == "CallExpr" && direct_callee(node).is_some_and(|(name, _)| name == "reset")
        {
            let [_, arg] = children(node) else {
                panic!("one argument")
            };
            break actual(&index, arg, &ty).unwrap();
        }
        work.extend(children(node));
    };
    // Non-vacuity: the raw optional question really makes this new lookup.
    let enclosing = qcore_schemas::contract_consultation::open();
    assert!(source_frame(&index, &place, &declaration, &writes, &c).is_none());
    let raw = qcore_schemas::contract_consultation::close(enclosing);
    assert!(raw.missed.contains("proof_only"), "{raw:?}");
    // Exercise the actual classifier, without unrelated whole-unit prepasses.
    let mut parser = super::super::probe_state::tests::bare(&stream, &c);
    parser.typed_path_index = std::cell::OnceCell::from(Some(index));
    let body = find_function_body(&stream, "subject").unwrap().0;
    let start = stream
        .iter()
        .enumerate()
        .find(|(n, t)| *n > body && t.text == "reset")
        .unwrap()
        .0;
    let end = matching(&stream, start + 1, "(", ")").unwrap();
    parser.cursor = start;
    let enclosing = qcore_schemas::contract_consultation::open();
    let instruction = parser.tracked_self_init_call(
        &stream[start..=end],
        LegacyInstruction::Unknown {
            description: "call to uncontracted function `reset`".into(),
            writes: RefusedWrites::Unbounded,
        },
    );
    let noted = qcore_schemas::contract_consultation::close(enclosing);
    assert!(
        matches!(instruction,LegacyInstruction::TrackedSelfInit {protected_fields,..} if protected_fields.is_empty()),
        "the original tracked route still admits its marker"
    );
    assert!(
        !noted.missed.contains("proof_only"),
        "declined optional lookup leaked: {noted:?}"
    );
}

// The same ordered criteria as TrackedSelfInitDisjointFrame.flowVectors.
#[test]
fn frame_flow_vectors_are_bound_to_the_lean_table() {
    let initial = initial();
    let mut answers = vec![framed(&initial).is_some()];
    let mut s = initial.clone();
    s.resources.insert(8, RELEASED);
    answers.push(framed(&s).is_some());
    let mut s = initial.clone();
    s.nullable_sites.insert(8);
    answers.push(framed(&s).is_some());
    let mut s = initial.clone();
    s.partial_links = s.field_links.clone();
    answers.push(framed(&s).is_some());
    let mut s = initial.clone();
    s.partial_links
        .insert(7, BTreeMap::from([("payload".into(), BTreeSet::from([9]))]));
    answers.push(framed(&s).is_some());
    answers.push(
        apply_frame(
            &initial,
            "p",
            &[],
            &["p".into()],
            &TransferContext::default(),
        )
        .is_some(),
    );
    let mut s = initial.clone();
    s.field_links
        .insert(7, BTreeMap::from([("head".into(), BTreeSet::from([8]))]));
    answers.push(framed(&s).is_some());
    let mut s = initial.clone();
    s.unmodeled = true;
    answers.push(framed(&s).is_some());
    let mut s = initial.clone();
    s.pointers
        .insert("unsealed".into(), s.pointers["p"].clone());
    answers.push(framed(&s).is_some());
    let mut s = initial.clone();
    s.pointers.insert("*out".into(), AbstractPointer::default());
    answers.push(framed(&s).is_some());
    let mut ctx = TransferContext::default();
    ctx.caller_sites.insert(7);
    answers.push(apply_frame(&initial, "p", &["payload".into()], &["p".into()], &ctx).is_some());
    for site in [8, 7] {
        let mut ctx = TransferContext::default();
        ctx.site_kinds.insert(site, ResourceKind::Regex);
        answers
            .push(apply_frame(&initial, "p", &["payload".into()], &["p".into()], &ctx).is_some());
    }
    let mut s = initial.clone();
    s.resources.insert(7, RELEASED);
    answers.push(framed(&s).is_some());
    let mut s = initial.clone();
    s.pointers.get_mut("p").unwrap().may_be_null = true;
    answers.push(framed(&s).is_some());
    let mut s = initial.clone();
    s.pointers.get_mut("p").unwrap().derivation = Derivation::Identity.step(true);
    answers.push(framed(&s).is_some());
    let mut s = initial.clone();
    s.field_links
        .insert(9, BTreeMap::from([("payload".into(), BTreeSet::from([8]))]));
    answers.push(framed(&s).is_some());
    let older = crate::loop_generations::older(7);
    let mut s = initial.clone();
    s.pointers.get_mut("p").unwrap().targets = BTreeSet::from([older]);
    s.resources.insert(older, LIVE);
    let links = s.field_links.remove(&7).unwrap();
    s.field_links.insert(older, links);
    answers.push(framed(&s).is_some());
    let mut s = initial.clone();
    s.pointers
        .insert("p->head.next".into(), s.pointers["p->payload"].clone());
    answers.push(framed(&s).is_some());
    let mut s = initial;
    s.pointers.insert(
        "foreign->next".into(),
        AbstractPointer {
            unknown: true,
            ..Default::default()
        },
    );
    answers.push(framed(&s).is_some());
    assert_eq!(
        answers,
        [
            true, false, false, true, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false, true
        ]
    );
    assert!(include_str!("../../../../proof/QCore/TrackedSelfInitDisjointFrame.lean").contains("[true,false,false,true,false,false,false,false,false,false,false,false,false,false,false,false,false,false,false,true]"));
}

#[test]
fn actual_core_layout_decisions_are_bound_to_the_lean_vectors() {
    use crate::footprint_connection::{MemberLayout, core_members_disjoint};
    let separate = MemberLayout {
        size: 16,
        members: vec![("a".into(), 0, None), ("b".into(), 8, None)],
    };
    let overlap = MemberLayout {
        size: 8,
        members: vec![("a".into(), 0, None), ("b".into(), 0, None)],
    };
    let answers = [
        core_members_disjoint(&separate, "a", "b", false),
        core_members_disjoint(&separate, "b", "a", false),
        core_members_disjoint(&overlap, "a", "b", false),
        core_members_disjoint(&separate, "a", "a", false),
    ];
    assert_eq!(answers, [true, true, false, false]);
    assert!(
        include_str!("../../../../proof/QCore/TrackedSelfInitDisjointFrame.lean")
            .contains("layoutVectors = [true,true,false,false]")
    );
    assert!(crate::footprint_connection::parse_member_layout("", "Owner").is_none());
}
