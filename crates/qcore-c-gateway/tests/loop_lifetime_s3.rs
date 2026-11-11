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

//! S3 source-boundary controls. These are synthetic declared contracts, not
//! evidence about a deployed Fluent Bit/plugin configuration.
use qcore_c_gateway::legacy_c::{C11TranslationUnit, ResourceContracts};
use qcore_c_gateway::{
    loop_lifetime_source::{self, SourceFacts},
    loop_lifetime_summary::{self, Standing},
};
use qcore_contract::contract_loader::load_resource_contract_sources;
use qcore_core::local_condition::Truth;
use qcore_schemas::loop_lifetime::ConditionBinding;
use std::{collections::BTreeMap, sync::OnceLock};
const C: &str = r#"
struct loop { int fd; }; struct event { int state; };
struct instance { int is_threaded; int other; };
struct collector { struct loop *evl; struct instance *instance; struct event event; };
int ev_add(struct loop *, void *); int ev_del(struct loop *, void *);
void ev_destroy(struct loop *); void release(void *);
void target(struct instance *, int); void opaque(struct instance *);
static int threaded(struct instance *p) { return p->is_threaded; }
static unsigned char narrowed(struct instance *p) { return p->is_threaded; }
void narrowing_getter(struct instance *p) { if (!narrowed(p)) target(p, 0); }
void void_bridge(struct instance *p) { void *v = p; if (!p->is_threaded) target(v, 0); }
static int other(struct instance *p, int ignored) { return p->other; }
void getter_mutates_outer(struct instance *p) { if (!p->is_threaded) { if (other(p, (p->is_threaded = 1, 0))) target(p, 0); } }
void plain(struct instance *p) { if (!p->is_threaded) target(p, 0); }
void getter(struct instance *p) { if (!threaded(p)) target(p, 0); }
void address_truth(struct instance *p) { if (&p->is_threaded) target(p, 0); }
void volatile_pointer(struct instance *volatile p) { if (!p->is_threaded) target(p, 0); }
int interposable_getter(struct instance *p) { return p->is_threaded; }
void external_getter(struct instance *p) { if (!interposable_getter(p)) target(p, 0); }
void later_local_assignment(struct instance *p) { struct instance *q; if (!p->is_threaded) target(q, 0); q = p; }
void mutate_alias(struct instance **);
void escaped_local_alias(struct instance *p) { struct instance *q = p; mutate_alias(&q); if (!p->is_threaded) target(q, 0); }
void initialized_local_alias(struct instance *p) { struct instance *q = p; if (!p->is_threaded) target(q, 0); }
void mutated_argument(struct instance *p) { if (!p->is_threaded) target(p, (p->is_threaded = 1, 0)); }
void incremented_argument(struct instance *p) { if (!p->is_threaded) target(p, p->is_threaded++); }
void intervening_call(struct instance *p) { if (!p->is_threaded) { opaque(p); target(p, 0); } }
void bypass(struct instance *p) { goto L; if (!p->is_threaded) { L: target(p, 0); } }
void case_bypass(struct instance *p, int n) { switch (n) { if (!p->is_threaded) { case 1: target(p, 0); } } }
void different_field(struct instance *p) { if (!p->other) target(p, 0); }
void different_object(struct instance *p, struct instance *q) { if (!p->is_threaded) target(q, 0); }
void later_formal_assignment(struct instance *p, struct instance *q) { if (!p->is_threaded) target(q, 0); q = p; }
void starts(struct collector *c) { ev_add(c->evl, &c->event); }
void stops(struct collector *c) { ev_del(c->evl, &c->event); release(c); }
void ends_loop(struct collector *c) { ev_destroy(c->evl); release(c); }
"#;
fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([("s3.qv".into(),r#"module contract;
extern fn ev_add(loop: ptr<mut, u8>, event: ptr<mut, u8>) -> i32 contract { requires resource_contract_v1; ensures normal resource_borrowed(loop); ensures normal resource_borrowed(event); ensures normal resource_retained_by(event, loop); ensures normal resource_return_none(); };
extern fn ev_del(loop: ptr<mut, u8>, event: ptr<mut, u8>) -> i32 contract { requires resource_contract_v1; ensures normal resource_borrowed(loop); ensures normal resource_borrowed(event); quant live #loop: #holds(event) |-> #absent; ensures normal resource_return_none(); };
extern fn ev_destroy(loop: ptr<mut, u8>) -> unit contract { requires resource_contract_v1; ensures normal resource_released(loop, nonnull); ensures normal resource_return_none(); };
extern fn release(p: ptr<mut, u8>) -> unit contract { requires resource_contract_v1; ensures normal resource_released(p, nullable); ensures normal resource_return_none(); };
"#.into())])).unwrap()
}
fn facts() -> &'static SourceFacts {
    static FACTS: OnceLock<SourceFacts> = OnceLock::new();
    FACTS.get_or_init(|| {
        loop_lifetime_source::read_unit(
            C,
            &C11TranslationUnit {
                source: C,
                ..Default::default()
            },
            &contracts(),
        )
        .unwrap()
    })
}
fn binding(field: &str) -> ConditionBinding {
    ConditionBinding {
        name: "threaded".into(),
        parameter: 0,
        members: vec![field.into()],
    }
}
fn call(function: &str) -> &qcore_c_gateway::loop_lifetime_source::CallSite {
    facts().functions[function]
        .calls
        .iter()
        .find(|c| c.callee == "target")
        .unwrap()
}
#[test]
fn exact_field_and_checked_pure_getter_ground_the_local_condition() {
    for f in ["plain", "getter"] {
        assert_eq!(
            loop_lifetime_source::condition_at(&binding("is_threaded"), call(f), &[facts()]),
            Truth::No,
            "{f}"
        );
    }
}
#[test]
fn argument_side_effects_and_intervening_calls_invalidate_the_branch_fact() {
    for f in [
        "mutated_argument",
        "incremented_argument",
        "intervening_call",
        "getter_mutates_outer",
    ] {
        assert!(call(f).conditions.is_empty(), "{f}");
        assert_eq!(
            loop_lifetime_source::condition_at(&binding("is_threaded"), call(f), &[facts()]),
            Truth::Unknown,
            "{f}"
        );
    }
}
#[test]
fn goto_and_case_labels_cannot_borrow_structural_branch_ancestry() {
    for f in ["bypass", "case_bypass"] {
        assert!(call(f).conditions.is_empty(), "{f}");
        assert_eq!(
            loop_lifetime_source::condition_at(&binding("is_threaded"), call(f), &[facts()]),
            Truth::Unknown,
            "{f}"
        );
    }
}
#[test]
fn same_root_different_field_and_same_field_different_root_do_not_match() {
    assert_eq!(
        loop_lifetime_source::condition_at(
            &binding("is_threaded"),
            call("different_field"),
            &[facts()]
        ),
        Truth::Unknown
    );
    assert_eq!(
        loop_lifetime_source::condition_at(&binding("other"), call("different_field"), &[facts()]),
        Truth::No
    );
    assert_eq!(
        loop_lifetime_source::condition_at(
            &binding("is_threaded"),
            call("different_object"),
            &[facts()]
        ),
        Truth::Unknown
    );
}
#[test]
fn a_later_formal_assignment_does_not_rewrite_an_earlier_actual() {
    assert_eq!(
        loop_lifetime_source::condition_at(
            &binding("is_threaded"),
            call("later_formal_assignment"),
            &[facts()]
        ),
        Truth::Unknown
    );
}
#[test]
fn all_registration_and_release_sites_are_retained_and_source_roots_are_stable() {
    let second = loop_lifetime_source::read_unit(
        C,
        &C11TranslationUnit {
            source: C,
            ..Default::default()
        },
        &contracts(),
    )
    .unwrap();
    assert_eq!(facts(), &second);
    let protocol =
        qcore_contract::local_condition::registered("#c: collector.event |-> collector.evl")
            .unwrap();
    assert!(matches!(
        loop_lifetime_summary::standing(&protocol, &[facts()], &BTreeMap::new()),
        Standing::DeclaredAssumption { .. }
    ));
    assert!(!facts().functions["stops"].operations.is_empty());
    assert!(!facts().functions["ends_loop"].operations.is_empty());
    let summaries = loop_lifetime_summary::derive_source(
        facts(),
        &[protocol],
        &std::collections::BTreeSet::new(),
        &contracts(),
    );
    assert_eq!(summaries["starts"].may_register.len(), 1);
    assert!(!summaries["stops"].requirements.is_empty());
}

fn guarded_premise_source(field: &str, negated: bool, statement: &str) -> String {
    let source = format!(
        "module contract; extern fn f(ins: ptr<mut, u8>) -> i32 contract {{ requires resource_contract_v1; requires resource_premise(s3_condition_snapshot_stable, \"Every member read by this function's condition bindings preserves its entry value until each guarded use.\"); ensures normal resource_borrowed(ins); ensures normal resource_return_none(); quant #condition = (#ins.{field} != 0); quant {}#condition in {{ requires resource_premise(entry_protocol, \"{statement}\"); }}: [[local]]; }};",
        if negated { "not " } else { "" }
    );
    source
}
fn guarded_premise(
    field: &str,
    negated: bool,
    statement: &str,
) -> qcore_schemas::loop_lifetime::LoopLifetimeContract {
    let source = guarded_premise_source(field, negated, statement);
    let env =
        load_resource_contract_sources(&BTreeMap::from([("guard.qv".into(), source)])).unwrap();
    env.lifetime_contract("f")
        .unwrap()
        .lifetime
        .clone()
        .unwrap()
}
#[test]
fn an_active_guarded_requirement_uses_only_a_matching_stable_entry_precondition() {
    use loop_lifetime_source::{EntryContext, call_ready};
    let callee = guarded_premise("is_threaded", true, "the declared entry protocol holds");
    let own = callee.clone();
    let mut unstable = own.clone();
    unstable.entry_snapshot_stable = false;
    let run = qcore_c_gateway::member_run_facts::RunMemberFacts::default();
    let entry = EntryContext {
        lifetime: &own,
        body: &facts().functions["plain"],
    };
    assert!(call_ready(&callee, call("plain"), &[facts()], &run, None).is_err());
    let unstable_entry = EntryContext {
        lifetime: &unstable,
        body: &facts().functions["plain"],
    };
    assert!(
        call_ready(
            &callee,
            call("plain"),
            &[facts()],
            &run,
            Some(&unstable_entry)
        )
        .is_err()
    );
    assert!(call_ready(&callee, call("plain"), &[facts()], &run, Some(&entry)).is_ok());
    for (field, negated, statement) in [
        ("other", true, "the declared entry protocol holds"),
        ("is_threaded", false, "the declared entry protocol holds"),
        ("is_threaded", true, "another protocol holds"),
    ] {
        let wrong = guarded_premise(field, negated, statement);
        let entry = EntryContext {
            lifetime: &wrong,
            body: &facts().functions["plain"],
        };
        assert!(call_ready(&callee, call("plain"), &[facts()], &run, Some(&entry)).is_err());
    }
    for f in [
        "different_object",
        "mutated_argument",
        "incremented_argument",
        "intervening_call",
        "later_formal_assignment",
    ] {
        let entry = EntryContext {
            lifetime: &own,
            body: &facts().functions[f],
        };
        assert!(
            call_ready(&callee, call(f), &[facts()], &run, Some(&entry)).is_err(),
            "{f}"
        );
    }
}

#[test]
fn local_guarded_body_use_and_certificate_replay_need_no_ambient_run_facts() {
    use qcore_c_gateway::legacy_c::{
        UnrestrictedLegacyResult, analyze_unrestricted_c11, check_legacy_safety_certificate,
    };
    let source = "struct instance { int is_threaded; }; void target(struct instance *); void caller(struct instance *ins) { if (!ins->is_threaded) target(ins); }";
    let declaration = |name: &str, premise: bool| {
        format!(
            "module contract; extern fn {name}(ins: ptr<mut, u8>) -> unit contract {{ requires resource_contract_v1; {} ensures normal resource_borrowed(ins); ensures normal resource_return_none(); quant #threaded = (#ins.is_threaded != 0); quant not #threaded in {{ requires resource_premise(entry_protocol, \"The entry protocol holds\"); }}: [[local]]; }};",
            if premise {
                format!(
                    "requires resource_premise({}, \"{}\");",
                    qcore_contract::local_condition::SNAPSHOT_PREMISE_NAME,
                    qcore_contract::local_condition::SNAPSHOT_PREMISE_STATEMENT
                )
            } else {
                String::new()
            }
        )
    };
    let env = load_resource_contract_sources(&BTreeMap::from([
        ("target.qv".into(), declaration("target", false)),
        ("caller.qv".into(), declaration("caller", true)),
    ]))
    .unwrap();
    assert!(qcore_c_gateway::member_run_facts::installed().is_none());
    let result = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source,
            ..Default::default()
        },
        "caller",
        &env,
    )
    .unwrap();
    let UnrestrictedLegacyResult::Safe(certificate) = result else {
        panic!("guarded entry use did not discharge: {result:?}")
    };
    check_legacy_safety_certificate(&certificate).unwrap();
}

#[test]
fn an_exact_caller_record_forwards_under_a_named_protocol_and_replays_its_effect() {
    use qcore_c_gateway::legacy_c::{
        UnrestrictedLegacyResult, analyze_unrestricted_c11, check_legacy_safety_certificate,
    };
    let source = "struct loop { int fd; }; struct event { int state; }; struct collector { struct loop *evl; struct event event; struct event other_event; }; int ev_add(struct loop *, void *); void starts(struct collector *c) { ev_add(c->evl, &c->event); }";
    let mut texts = contracts().source_assumptions().clone();
    texts.insert("protocol.qv".into(), "module contract; extern fn protocol_host(c: ptr<mut, u8>) -> unit contract { requires resource_contract_v1; ensures normal resource_borrowed(c); ensures normal resource_return_none(); quant registered #c: collector.event |-> collector.evl; };".into());
    let env = load_resource_contract_sources(&texts).unwrap();
    let result = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source,
            ..Default::default()
        },
        "starts",
        &env,
    )
    .unwrap();
    let UnrestrictedLegacyResult::Safe(certificate) = result else {
        panic!("exact protocol did not forward: {result:?}")
    };
    let effects = &certificate.summary.functions["starts"].lifetime_effects;
    assert_eq!(effects.len(), 1);
    assert!(
        effects
            .first()
            .unwrap()
            .premise
            .starts_with("loop-protocol-declared:protocol_host:")
    );
    assert_eq!(
        effects.first().unwrap().registration.object.fields,
        ["event"]
    );
    check_legacy_safety_certificate(&certificate).unwrap();
    for bad in [
        source.replace("&c->event);", "&c->other_event);"),
        source.replace("c->evl,", "other, ").replace(
            "struct collector *c) {",
            "struct collector *c, struct loop *other) {",
        ),
    ] {
        let result = analyze_unrestricted_c11(
            &C11TranslationUnit {
                source: &bad,
                ..Default::default()
            },
            "starts",
            &env,
        )
        .unwrap();
        assert!(
            !matches!(result, UnrestrictedLegacyResult::Safe(_)),
            "unmatched record/holder must refuse"
        );
    }
}

#[test]
fn a_narrowed_getter_or_void_retyped_actual_cannot_supply_a_typed_guard() {
    // In particular, an int field of256 narrows to0 in the uchar getter;
    // !getter is therefore no proof of a zero int field.
    for f in ["narrowing_getter", "void_bridge"] {
        assert_eq!(
            loop_lifetime_source::condition_at(&binding("is_threaded"), call(f), &[facts()]),
            Truth::Unknown,
            "{f}"
        );
    }
}

#[test]
fn a_same_unit_caller_consumes_and_forwards_the_exact_exported_registration() {
    use qcore_c_gateway::legacy_c::{
        UnrestrictedLegacyResult, analyze_unrestricted_c11, check_legacy_safety_certificate,
    };
    let source = "struct loop { int fd; }; struct event { int state; }; struct collector { struct loop *evl; struct event event; }; int ev_add(struct loop *, void *);\nstatic void starts(struct collector *c) { ev_add(c->evl, &c->event); }\nvoid caller(struct collector *c) { starts(c); }\n";
    let mut texts = contracts().source_assumptions().clone();
    texts.insert("protocol.qv".into(), "module contract; extern fn protocol_host(c: ptr<mut, u8>) -> unit contract { requires resource_contract_v1; ensures normal resource_borrowed(c); ensures normal resource_return_none(); quant registered #c: collector.event |-> collector.evl; };".into());
    let env = load_resource_contract_sources(&texts).unwrap();
    let result = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source,
            ..Default::default()
        },
        "caller",
        &env,
    )
    .unwrap();
    let UnrestrictedLegacyResult::Safe(certificate) = result else {
        panic!("same-unit effect was not consumed: {result:?}")
    };
    assert_eq!(
        certificate.summary.functions["caller"]
            .lifetime_effects
            .len(),
        1
    );
    assert!(
        certificate
            .summaries
            .iter()
            .any(|s| s.function == "starts" && !s.lifetime_effects.is_empty())
    );
    check_legacy_safety_certificate(&certificate).unwrap();

    // An ordinary declaration or same-unit body cannot prove which externally
    // linked definition a call reaches. Transport needs an internal helper or
    // independently replayed external home.
    let external = source.replace("static void starts", "void starts");
    let unreplayed = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &external,
            ..Default::default()
        },
        "caller",
        &env,
    )
    .unwrap();
    assert!(matches!(
        unreplayed,
        UnrestrictedLegacyResult::Unknown { .. }
    ));

    let mut missing_effect = certificate.clone();
    missing_effect
        .summaries
        .iter_mut()
        .find(|s| s.function == "starts")
        .unwrap()
        .lifetime_effects
        .clear();
    assert!(check_legacy_safety_certificate(&missing_effect).is_err());
    let mut missing_body_effect = certificate.clone();
    missing_body_effect
        .summary
        .functions
        .get_mut("caller")
        .unwrap()
        .lifetime_effects
        .clear();
    assert!(check_legacy_safety_certificate(&missing_body_effect).is_err());
}

#[test]
fn unused_protocol_does_not_add_a_premise_but_forwarded_effects_do() {
    use qcore_c_gateway::contract_dependencies::Recording;
    use qcore_c_gateway::legacy_c::{UnrestrictedLegacyResult, analyze_unrestricted_c11};
    let mut texts = contracts().source_assumptions().clone();
    texts.insert("protocol.qv".into(), "module contract; extern fn protocol_host(c: ptr<mut, u8>) -> unit contract { requires resource_contract_v1; ensures normal resource_borrowed(c); ensures normal resource_return_none(); quant registered #c: collector.event |-> collector.evl; };".into());
    let env = load_resource_contract_sources(&texts).unwrap();
    let source = "struct loop { int fd; }; struct event { int state; }; struct collector { struct loop *evl; struct event event; }; void protocol_host(struct collector *); void ordinary(struct collector *c) { protocol_host(c); } int ev_add(struct loop *, void *); void starts(struct collector *c) { ev_add(c->evl, &c->event); }";
    for (function, forwarded) in [("ordinary", false), ("starts", true)] {
        let recording = Recording::start();
        let result = analyze_unrestricted_c11(
            &C11TranslationUnit {
                source,
                ..Default::default()
            },
            function,
            &env,
        )
        .unwrap();
        let record = recording.finish();
        let UnrestrictedLegacyResult::Safe(certificate) = result else {
            panic!("{function}: {result:?}")
        };
        let effects = &certificate.summary.functions[function].lifetime_effects;
        assert_eq!(!effects.is_empty(), forwarded);
        let premises = record
            .lifetime_prerequisites
            .get(function)
            .cloned()
            .unwrap_or_default();
        assert_eq!(!premises.is_empty(), forwarded, "{function}: {premises:?}");
        for effect in effects {
            assert!(premises.contains(&effect.premise));
        }
    }
}

#[test]
fn an_ordinary_custody_declaration_cannot_erase_its_bodys_registration() {
    use qcore_c_gateway::legacy_c::{
        UnrestrictedLegacyResult, analyze_unrestricted_c11, check_legacy_safety_certificate,
    };
    let source = "struct loop { int fd; }; struct event { int state; }; struct collector { struct loop *evl; struct event event; }; int ev_add(struct loop *, void *);\nstatic void starts(struct collector *c) { ev_add(c->evl, &c->event); }\nvoid caller(struct collector *c) { starts(c); }\n";
    let mut texts = contracts().source_assumptions().clone();
    texts.insert("protocol.qv".into(), "module contract; extern fn protocol_host(c: ptr<mut, u8>) -> unit contract { requires resource_contract_v1; ensures normal resource_borrowed(c); ensures normal resource_return_none(); quant registered #c: collector.event |-> collector.evl; }; extern fn starts(c: ptr<mut, u8>) -> unit contract { requires resource_contract_v1; ensures normal resource_borrowed(c); ensures normal resource_return_none(); };".into());
    let env = load_resource_contract_sources(&texts).unwrap();
    let result = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source,
            ..Default::default()
        },
        "caller",
        &env,
    )
    .unwrap();
    let UnrestrictedLegacyResult::Safe(certificate) = result else {
        panic!("declared custody must retain richer body effects: {result:?}")
    };
    assert_eq!(
        certificate.summary.functions["caller"]
            .lifetime_effects
            .len(),
        1
    );
    assert!(
        certificate
            .summaries
            .iter()
            .any(|s| s.function == "starts" && !s.lifetime_effects.is_empty())
    );
    check_legacy_safety_certificate(&certificate).unwrap();

    // An ordinary declaration or same-unit body cannot prove which externally
    // linked definition a call reaches. Transport needs an internal helper or
    // independently replayed external home.
    let external = source.replace("static void starts", "void starts");
    let unreplayed = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &external,
            ..Default::default()
        },
        "caller",
        &env,
    )
    .unwrap();
    assert!(matches!(
        unreplayed,
        UnrestrictedLegacyResult::Unknown { .. }
    ));
}

#[test]
fn scoped_same_spelled_record_declarations_cannot_supply_a_global_field_identity() {
    let source = "struct instance { int is_threaded; }; void target(struct instance *); void f(struct instance *p) { struct instance { char padding; int is_threaded; }; if (!p->is_threaded) target(p); }";
    let facts = loop_lifetime_source::read_unit(
        source,
        &C11TranslationUnit {
            source,
            ..Default::default()
        },
        &contracts(),
    )
    .unwrap();
    assert!(
        facts
            .open
            .iter()
            .any(|r| r.contains("ambiguous scoped record declaration instance"))
    );
    let call = facts.functions["f"]
        .calls
        .iter()
        .find(|c| c.callee == "target")
        .unwrap();
    assert_eq!(
        loop_lifetime_source::condition_at(&binding("is_threaded"), call, &[&facts]),
        Truth::Unknown
    );
}

#[test]
fn retyped_field_aliases_keep_may_effects_but_never_exact_registration_or_endings() {
    use qcore_c_gateway::loop_lifetime_source::OperationKind;
    let source = "struct loop { int fd; }; struct event { int state; }; struct collector { struct loop *evl; struct event event; }; struct different { int padding; struct loop *evl; struct event event; }; int ev_add(struct loop *, void *); int ev_del(struct loop *, void *); void ev_destroy(struct loop *); void bad_add(struct collector *c) { void *v = c; struct different *d = v; ev_add(d->evl, &d->event); } void bad_del(struct collector *c) { void *v = c; struct different *d = v; ev_del(d->evl, &d->event); } void bad_destroy(struct collector *c) { void *v = c; struct different *d = v; ev_destroy(d->evl); } void good_add(struct collector *c) { ev_add(c->evl, &c->event); }";
    let facts = loop_lifetime_source::read_unit(
        source,
        &C11TranslationUnit {
            source,
            ..Default::default()
        },
        &contracts(),
    )
    .unwrap();
    let bad = &facts.functions["bad_add"].calls[0];
    assert!(bad.actuals.iter().all(Option::is_some));
    assert!(bad.typed_effect_actuals.iter().all(Option::is_none));
    let removed = &facts.functions["bad_del"].operations[0];
    assert_eq!(removed.kind, OperationKind::Remove);
    assert!(removed.object.is_none() && removed.holder.is_none());
    assert!(
        facts.functions["bad_destroy"]
            .operations
            .iter()
            .all(|op| op.kind != OperationKind::Destroy)
    );
    let good = &facts.functions["good_add"].calls[0];
    assert!(good.typed_effect_actuals.iter().all(Option::is_some));
}

#[test]
fn source_must_endings_require_stable_holder_values_and_a_checked_frame() {
    let source = "struct loop { int fd; }; struct event { int state; }; struct collector { struct loop *evl; struct event event; }; int ev_add(struct loop *, void *); void ev_destroy(struct loop *); void opaque(struct collector *); void direct(struct collector *c) { ev_add(c->evl, &c->event); ev_destroy(c->evl); } void changed(struct collector *c, struct loop *other) { ev_add(c->evl, &c->event); c->evl = other; ev_destroy(c->evl); } void unknown_frame(struct collector *c) { ev_add(c->evl, &c->event); opaque(c); ev_destroy(c->evl); }";
    let mut env = contracts();
    env.insert_contract(
        "opaque",
        qcore_schemas::contract::ResourceFunctionContract::new(
            [qcore_schemas::contract::ResourceParameterMode::Borrowed],
            qcore_schemas::contract::ResourceReturnMode::None,
        ),
    );
    let facts = loop_lifetime_source::read_unit(
        source,
        &C11TranslationUnit {
            source,
            ..Default::default()
        },
        &env,
    )
    .unwrap();
    let protocol =
        qcore_contract::local_condition::registered("#c: collector.event |-> collector.evl")
            .unwrap();
    let summaries =
        loop_lifetime_summary::derive_source(&facts, &[protocol], &Default::default(), &env);
    assert!(!summaries["direct"].must_end.is_empty());
    assert!(summaries["direct"].open.is_empty());
    for name in ["changed", "unknown_frame"] {
        assert!(summaries[name].must_end.is_empty(), "{name}");
        assert!(!summaries[name].open.is_empty(), "{name}");
        assert!(
            summaries[name]
                .may_release
                .iter()
                .all(|e| e.destroyed_before.is_empty())
        );
    }
}

#[test]
fn pointer_truth_volatile_interposition_and_unproved_local_aliases_do_not_ground_integer_guards() {
    for name in [
        "address_truth",
        "volatile_pointer",
        "external_getter",
        "later_local_assignment",
        "escaped_local_alias",
    ] {
        assert_eq!(
            loop_lifetime_source::condition_at(&binding("is_threaded"), call(name), &[facts()]),
            Truth::Unknown,
            "{name}"
        );
    }
    assert_eq!(
        loop_lifetime_source::condition_at(
            &binding("is_threaded"),
            call("initialized_local_alias"),
            &[facts()]
        ),
        Truth::No
    );
}

#[test]
fn guarded_callback_and_reference_effects_cannot_bypass_through_raw_prepasses() {
    use qcore_c_gateway::legacy_c::{UnrestrictedLegacyResult, analyze_unrestricted_c11};
    for callback in [false, true] {
        let (source, signature, modes, effect) = if callback {
            (
                "struct instance { int is_threaded; }; void gate(struct instance *,void (*)(void)); void cb(void) {} void caller(struct instance *p) { if (!p->is_threaded) gate(p,cb); }",
                "ins: ptr<mut, u8>, cb: fnptr",
                "ensures normal resource_borrowed(ins); ensures normal resource_borrowed(cb);",
                "ensures normal resource_callback_invoked(cb);",
            )
        } else {
            (
                "struct instance { int is_threaded; }; void gate(struct instance *); void caller(struct instance *p) { if (!p->is_threaded) gate(p); }",
                "ins: ptr<mut, u8>",
                "ensures normal resource_borrowed(ins);",
                "ensures normal resource_reference_kept(ins);",
            )
        };
        let text = format!(
            "module contract; extern fn gate({signature}) -> unit contract {{ requires resource_contract_v1; {modes} {effect} ensures normal resource_return_none(); quant #threaded = (#ins.is_threaded != 0); quant #threaded in {{ requires resource_premise(worker, \"The worker protocol holds\"); }}: [[local]]; }};"
        );
        let env = load_resource_contract_sources(&[("guarded.qv".into(), text)].into()).unwrap();
        assert!(env.retaining().is_empty());
        assert!(env.guarded_retaining().contains_key("gate"));
        let result = analyze_unrestricted_c11(
            &C11TranslationUnit {
                source,
                ..Default::default()
            },
            "caller",
            &env,
        )
        .unwrap();
        assert!(
            !matches!(result, UnrestrictedLegacyResult::Safe(_)),
            "{callback}: {result:?}"
        );
        assert!(
            format!("{result:?}").contains("conditional structured effects"),
            "the explicit gate must refuse: {result:?}"
        );
    }
}

#[test]
fn ready_guards_do_not_bypass_declared_c_abi_validation() {
    use qcore_c_gateway::legacy_c::{UnrestrictedLegacyResult, analyze_unrestricted_c11};
    for (source, signature, modes) in [
        (
            "struct instance { int is_threaded; }; void gate(struct instance *,int); void caller(struct instance *p) { if (!p->is_threaded) gate(p,0); }",
            "ins: ptr<mut, u8>, released: ptr<mut, u8>",
            "ensures normal resource_borrowed(ins); ensures normal resource_released(released, nonnull);",
        ),
        (
            "struct instance { int is_threaded; }; int *gate(struct instance *); void caller(struct instance *p) { if (!p->is_threaded) gate(p); }",
            "ins: ptr<mut, u8>",
            "ensures normal resource_borrowed(ins);",
        ),
    ] {
        let text = format!(
            "module contract; extern fn gate({signature}) -> unit contract {{ requires resource_contract_v1; {modes} ensures normal resource_return_none(); quant #threaded = (#ins.is_threaded != 0); quant #threaded in {{ requires resource_premise(worker, \"The worker protocol holds\"); }}: [[local]]; }};"
        );
        let env = load_resource_contract_sources(&[("guarded.qv".into(), text)].into()).unwrap();
        let result = analyze_unrestricted_c11(
            &C11TranslationUnit {
                source,
                ..Default::default()
            },
            "caller",
            &env,
        )
        .unwrap();
        assert!(
            !matches!(result, UnrestrictedLegacyResult::Safe(_)),
            "an inactive prerequisite does not repair an ABI mismatch: {result:?}"
        );
    }
}

#[test]
fn three_function_calls_preserve_conditional_entry_obligations_instead_of_inferencing_them_away() {
    use qcore_c_gateway::legacy_c::{
        UnrestrictedLegacyResult, analyze_unrestricted_c11, check_legacy_safety_certificate,
    };
    let source = "struct instance { int is_threaded; }; void B(struct instance *); void A(struct instance *p) { B(p); } void C_bad(struct instance *p) { A(p); } void C_good(struct instance *p) { A(p); }";
    let texts = ["A", "B", "C_good"]
        .into_iter()
        .map(|name| {
            (
                format!("{name}.qv"),
                guarded_premise_source("is_threaded", true, "the declared entry protocol holds")
                    .replace("extern fn f(", &format!("extern fn {name}("))
                    .replace("-> i32", "-> unit"),
            )
        })
        .collect();
    let env = load_resource_contract_sources(&texts).unwrap();
    let unit = C11TranslationUnit {
        source,
        ..Default::default()
    };
    let bad = analyze_unrestricted_c11(&unit, "C_bad", &env).unwrap();
    assert!(
        !matches!(bad, UnrestrictedLegacyResult::Safe(_)),
        "A's own requires cannot become a caller fact: {bad:?}"
    );
    // The analyzer checks every defined body. Keep the deliberately bad body
    // out of the positive fixture rather than expecting its refusal to vanish.
    let good_source = source.replace("void C_bad(struct instance *p) { A(p); }", "");
    let good_unit = C11TranslationUnit {
        source: &good_source,
        ..Default::default()
    };
    let good = analyze_unrestricted_c11(&good_unit, "C_good", &env).unwrap();
    let UnrestrictedLegacyResult::Safe(certificate) = good else {
        panic!("matching guarded caller should retain its entry obligation: {good:?}")
    };
    assert!(
        !certificate
            .summaries
            .iter()
            .any(|s| ["A", "B", "C_good"].contains(&s.function.as_str()))
    );
    check_legacy_safety_certificate(&certificate).unwrap();
}

#[test]
fn a_pin_held_guarded_neutral_body_cannot_publish_a_bare_summary() {
    use qcore_c_gateway::legacy_c::{UnrestrictedLegacyResult, analyze_unrestricted_c11};
    use qcore_schemas::contract_revision::PinMismatchReason;
    let source = "struct instance { int is_threaded; }; void A(struct instance *p) {} void caller(struct instance *p) { A(p); }";
    let text = guarded_premise_source("is_threaded", true, "the declared entry protocol holds")
        .replace("extern fn f(", "extern fn A(")
        .replace("-> i32", "-> unit");
    let mut env = load_resource_contract_sources(&[("guarded.qv".into(), text)].into()).unwrap();
    env.withhold_for_pin_mismatch(
        "A",
        "guarded.qv".into(),
        PinMismatchReason::NoLock,
        None,
        "unreviewed fixture".into(),
    )
    .unwrap();
    let result = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source,
            ..Default::default()
        },
        "caller",
        &env,
    )
    .unwrap();
    assert!(
        !matches!(result, UnrestrictedLegacyResult::Safe(_)),
        "a held guard remains an interface boundary: {result:?}"
    );
}
