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
static int other(struct instance *p, int ignored) { return p->other; }
void getter_mutates_outer(struct instance *p) { if (!p->is_threaded) { if (other(p, (p->is_threaded = 1, 0))) target(p, 0); } }
void plain(struct instance *p) { if (!p->is_threaded) target(p, 0); }
void getter(struct instance *p) { if (!threaded(p)) target(p, 0); }
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
    assert_eq!(
        loop_lifetime_summary::standing(&protocol, &[facts()], &BTreeMap::new()),
        Standing::Validated
    );
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

fn guarded_premise(
    field: &str,
    negated: bool,
    statement: &str,
) -> qcore_schemas::loop_lifetime::LoopLifetimeContract {
    let source = format!(
        "module contract; extern fn f(ins: ptr<mut, u8>) -> i32 contract {{ requires resource_contract_v1; requires resource_premise(s3_condition_snapshot_stable, \"Every member read by this function's condition bindings preserves its entry value until each guarded use.\"); ensures normal resource_borrowed(ins); ensures normal resource_return_none(); quant #condition = (#ins.{field} != 0); quant {}#condition in {{ requires resource_premise(entry_protocol, \"{statement}\"); }}: [[local]]; }};",
        if negated { "not " } else { "" }
    );
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
