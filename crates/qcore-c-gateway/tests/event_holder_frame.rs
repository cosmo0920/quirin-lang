//! Source-level holder frame and collector-root separation controls.
use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;
use std::collections::BTreeMap;

fn check(body: &str, helper: &str) -> UnrestrictedLegacyResult {
    let declarations = r#"module contract;
extern fn add(loop: ptr<mut, u8>, event: ptr<mut, u8>) -> unit contract { requires resource_contract_v1; ensures normal resource_borrowed(loop); ensures normal resource_borrowed(event); ensures normal resource_retained_by(event, loop); ensures normal resource_return_none(); };
extern fn del(loop: ptr<mut, u8>, event: ptr<mut, u8>) -> unit contract { requires resource_contract_v1; ensures normal resource_borrowed(loop); ensures normal resource_borrowed(event); quant live #loop: #holds(event) |-> #absent; ensures normal resource_return_none(); };
extern fn opaque(p: ptr<mut, u8>) -> unit contract { requires resource_contract_v1; ensures normal resource_borrowed(p); ensures normal resource_return_none(); };
extern fn protocol(c: ptr<mut, u8>) -> unit contract { requires resource_contract_v1; ensures normal resource_borrowed(c); ensures normal resource_return_none(); quant registered #c: collector.event |-> collector.evl; };
"#;
    let env =
        load_resource_contract_sources(&BTreeMap::from([("s3.qv".into(), declarations.into())]))
            .unwrap();
    let c = format!(
        "struct loop {{ int fd; }}; struct event {{ int state; }}; struct collector {{ struct loop *evl; struct loop *other; struct event event; int id; }}; void add(struct loop *, void *); void del(struct loop *, void *); void opaque(void *); {helper} void caller(struct collector *c, struct collector *other) {{ {body} }}"
    );
    analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &c,
            ..Default::default()
        },
        "caller",
        &env,
    )
    .unwrap()
}
const START: &str = "static void starts(struct collector *c) { add(c->evl, &c->event); }";

#[test]
fn whole_record_call_preserves_only_its_source_checked_holder() {
    let result = check("starts(c); del(c->evl, &c->event);", START);
    let UnrestrictedLegacyResult::Safe(cert) = result else {
        panic!("checked frame refused: {result:?}")
    };
    check_legacy_safety_certificate(&cert).unwrap();
    assert_eq!(cert.summary.functions["caller"].lifetime_effects.len(), 0);
}

#[test]
fn loop_snapshot_identity_is_exact_and_does_not_come_from_collector_type_or_id() {
    let result = check(
        "struct loop *l = c->evl; starts(c); del(l, &c->event);",
        START,
    );
    let UnrestrictedLegacyResult::Safe(cert) = result else {
        panic!("loop snapshot refused: {result:?}")
    };
    check_legacy_safety_certificate(&cert).unwrap();
    assert!(cert.summary.functions["caller"].lifetime_effects.is_empty());
    for body in [
        "struct loop *l = other->evl; starts(c); del(l, &c->event); del(c->evl, &c->event);",
        "struct loop *l = c->evl; starts(c); l = other->evl; del(l, &c->event);",
        "starts(c); del(other->evl, &c->event); del(c->evl, &c->event);",
        "c->id = other->id; starts(c); del(other->evl, &c->event); del(c->evl, &c->event);",
        "starts(c); c->evl = other->evl; del(c->evl, &c->event);",
        "struct loop *l = c->evl; starts(c); opaque(&l); del(l, &c->event);",
        "struct loop *l = c->evl; { struct loop *l = other->evl; del(l, &c->event); } starts(c); del(l, &c->event);",
    ] {
        if let UnrestrictedLegacyResult::Safe(cert) = check(body, START) {
            assert!(
                cert.summary.functions["caller"]
                    .lifetime_effects
                    .iter()
                    .any(|e| e.registration.poisoned),
                "false collector/loop identity: {body}"
            );
            check_legacy_safety_certificate(&cert).unwrap();
        }
    }
}

#[test]
fn ordinary_borrow_and_mutating_or_escaping_helpers_supply_no_holder_frame() {
    for (body, helper) in [
        (
            "add(c->evl, &c->event); opaque(c); del(c->evl, &c->event);",
            "",
        ),
        (
            "starts(c); del(c->evl, &c->event);",
            "static void starts(struct collector *c) { add(c->evl, &c->event); c->evl = c->other; }",
        ),
        (
            "starts(c); del(c->evl, &c->event);",
            "static void starts(struct collector *c) { opaque(c); add(c->evl, &c->event); }",
        ),
        (
            "starts(c); del(c->evl, &c->event);",
            "void starts(struct collector *c) { add(c->evl, &c->event); }",
        ),
    ] {
        assert!(
            !matches!(check(body, helper), UnrestrictedLegacyResult::Safe(_)),
            "unchecked frame admitted: {body} / {helper}"
        );
    }
}

#[test]
fn exact_collector_copies_and_bottom_up_frames_compose_without_a_type_wide_binder() {
    let helper = format!("{START} static void forwards(struct collector *c) {{ starts(c); }}");
    let result = check("forwards(c); del(c->evl, &c->event);", &helper);
    let UnrestrictedLegacyResult::Safe(cert) = result else {
        panic!("bottom-up frame refused: {result:?}")
    };
    assert!(cert.summary.functions["caller"].lifetime_effects.is_empty());
    check_legacy_safety_certificate(&cert).unwrap();
    for body in [
        "struct collector *copy = c; starts(copy); del(c->evl, &copy->event);",
        "struct collector *copy = c; struct loop *l = copy->evl; starts(copy); del(l, &c->event);",
    ] {
        let result = check(body, &helper);
        let UnrestrictedLegacyResult::Unknown {
            partial: Some(summary),
            ..
        } = result
        else {
            panic!("collector alias liveness was silently weakened: {body}: {result:?}")
        };
        assert!(
            summary.functions["caller"].lifetime_effects.is_empty(),
            "identity did not discharge: {body}"
        );
        assert!(
            summary.functions["caller"]
                .diagnostics
                .iter()
                .any(|d| matches!(
                    d.kind,
                    qcore_c_gateway::legacy_c::LegacyDiagnosticKind::UnknownAlias { .. }
                )),
            "the independent alias-liveness refusal disappeared"
        );
    }
    for body in [
        "struct collector *copy = c; starts(copy); copy = other; del(copy->evl, &c->event);",
        "struct collector *copy = c; starts(copy); opaque(&copy); del(c->evl, &c->event);",
        "struct collector *copy = c; { struct collector *copy = other; starts(copy); } del(c->evl, &c->event);",
    ] {
        if let UnrestrictedLegacyResult::Safe(cert) = check(body, &helper) {
            assert!(
                !cert.summary.functions["caller"].lifetime_effects.is_empty(),
                "unknown collector identity was discharged: {body}"
            );
        }
    }
}
