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

//! The opt-in cross-unit carrier must re-derive both bodies and reject a
//! missing or changed home, even when an imported contract looks valid.

use std::collections::{BTreeMap, BTreeSet};

use qcore_c_gateway::legacy_c::{
    AbiOutcome, C11TranslationUnit, LegacyFunctionScope, ResourceContracts,
    UnrestrictedLegacyResult, analyze_prepared_translation_unit_with_cross_unit,
    check_legacy_safety_certificate, declared_abi_census, export_cross_unit_homes,
    prepare_translation_unit,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

fn clang_available() -> bool {
    qcore_c_gateway::clang_command().is_ok()
}

#[test]
fn unrelated_abi_withdrawal_preserves_replayed_import() {
    if !clang_available() {
        return;
    }
    let home = prepare_translation_unit(&C11TranslationUnit {
        source: "void callee(void) { return; }\n",
        function_scope: LegacyFunctionScope::MainSourceDefinitions,
        ..Default::default()
    });
    let digest = "a".repeat(64);
    let exports = export_cross_unit_homes(
        &home,
        &ResourceContracts::default(),
        &BTreeSet::from(["callee".to_owned()]),
        &digest,
    )
    .expect("home export");
    assert_eq!(exports.len(), 1);
    let mut environment = load_resource_contract_sources(&BTreeMap::from([(
        "abi.qcore".to_owned(),
        "module contract; extern fn k_alloc(size: usize) -> unit contract { requires resource_contract_v1; ensures normal resource_borrowed(size); ensures normal resource_return_none(); };".to_owned(),
    )]))
    .expect("loaded ABI fixture");
    environment.set_inferred_imports(BTreeMap::from([(
        "callee".to_owned(),
        exports[0].contract.clone(),
    )]));
    environment.set_cross_unit_import_identities(BTreeMap::from([(
        "callee".to_owned(),
        exports[0].home.import_identity(),
    )]));
    environment.set_link_resolution_sha256(Some(digest));
    let caller = prepare_translation_unit(&C11TranslationUnit {
        source: "void *k_alloc(unsigned long size); void callee(void); void caller(void) { callee(); return; }\n",
        function_scope: LegacyFunctionScope::MainSourceDefinitions,
        ..Default::default()
    });
    assert!(
        declared_abi_census(
            caller.preprocessed_source().expect("caller source"),
            &environment
        )
        .iter()
        .any(|row| row.function == "k_alloc" && matches!(row.outcome, AbiOutcome::Withdrawn(_)))
    );
    let UnrestrictedLegacyResult::Safe(certificate) =
        analyze_prepared_translation_unit_with_cross_unit(
            &caller,
            "caller",
            &environment,
            &[exports[0].home.clone()],
        )
        .expect("caller analysis")
    else {
        panic!("unrelated ABI withdrawal discarded the import");
    };
    check_legacy_safety_certificate(&certificate).expect("both bodies replay");
}

#[test]
fn home_and_caller_replay_and_missing_evidence_refuses() {
    if !clang_available() {
        return;
    }
    let home = prepare_translation_unit(&C11TranslationUnit {
        source: "void callee(void) { return; }\n",
        function_scope: LegacyFunctionScope::MainSourceDefinitions,
        ..Default::default()
    });
    let caller = prepare_translation_unit(&C11TranslationUnit {
        source: "void callee(void); void caller(void) { callee(); return; }\n",
        function_scope: LegacyFunctionScope::MainSourceDefinitions,
        ..Default::default()
    });
    let base = ResourceContracts::default();
    let digest = "a".repeat(64);
    let exports = export_cross_unit_homes(
        &home,
        &base,
        &BTreeSet::from(["callee".to_owned()]),
        &digest,
    )
    .expect("home export");
    assert_eq!(exports.len(), 1);
    let mut environment = base;
    environment.set_inferred_imports(BTreeMap::from([(
        "callee".to_owned(),
        exports[0].contract.clone(),
    )]));
    environment.set_cross_unit_import_identities(BTreeMap::from([(
        "callee".to_owned(),
        exports[0].home.import_identity(),
    )]));
    environment.set_link_resolution_sha256(Some(digest));
    assert!(
        analyze_prepared_translation_unit_with_cross_unit(&caller, "caller", &environment, &[],)
            .is_err()
    );
    let result = analyze_prepared_translation_unit_with_cross_unit(
        &caller,
        "caller",
        &environment,
        &[exports[0].home.clone()],
    )
    .expect("two-sided analysis");
    let UnrestrictedLegacyResult::Safe(mut certificate) = result else {
        panic!("expected conditional Safe caller: {result:?}");
    };
    check_legacy_safety_certificate(&certificate).expect("home and caller replay");

    certificate.cross_unit_homes.clear();
    assert!(
        check_legacy_safety_certificate(&certificate)
            .unwrap_err()
            .contains("imported contracts differ")
    );
    certificate.cross_unit_homes.push(exports[0].home.clone());
    certificate.cross_unit_homes[0]
        .preprocessed_source
        .push(' ');
    assert!(check_legacy_safety_certificate(&certificate).is_err());
    certificate.cross_unit_homes[0] = exports[0].home.clone();
    certificate.cross_unit_homes[0].link_resolution_sha256 = "b".repeat(64);
    assert!(
        check_legacy_safety_certificate(&certificate)
            .unwrap_err()
            .contains("link premise differs")
    );
    certificate.cross_unit_homes[0] = exports[0].home.clone();
    certificate.cross_unit_homes[0].environment_sha256 = "b".repeat(64);
    assert!(check_legacy_safety_certificate(&certificate).is_err());
    certificate.cross_unit_homes[0] = exports[0].home.clone();
    let mut identities = certificate.contracts.cross_unit_import_identities().clone();
    identities
        .get_mut("callee")
        .expect("identity")
        .raw_source_sha256 = "b".repeat(64);
    certificate
        .contracts
        .set_cross_unit_import_identities(identities);
    assert!(
        check_legacy_safety_certificate(&certificate)
            .unwrap_err()
            .contains("identities differ")
    );
}

#[test]
fn static_and_external_inline_are_not_exported() {
    if !clang_available() {
        return;
    }
    let requested = BTreeSet::from(["callee".to_owned()]);
    for source in [
        "static void callee(void) { return; }\n",
        "inline void callee(void) { return; }\n",
        "static void callee(void); void callee(void) { return; }\n",
        "inline void callee(void); void callee(void) { return; }\n",
    ] {
        let home = prepare_translation_unit(&C11TranslationUnit {
            source,
            function_scope: LegacyFunctionScope::MainSourceDefinitions,
            ..Default::default()
        });
        let exports = export_cross_unit_homes(
            &home,
            &ResourceContracts::default(),
            &requested,
            &"a".repeat(64),
        )
        .expect("home analysis");
        assert!(exports.is_empty(), "exported {source}");
    }
}

#[test]
fn lifetime_import_retains_effects_and_replays_home_before_caller_composition() {
    let declarations = r#"module contract;
extern fn ev_add(loop: ptr<mut, u8>, event: ptr<mut, u8>) -> i32 contract { requires resource_contract_v1; ensures normal resource_borrowed(loop); ensures normal resource_borrowed(event); ensures normal resource_retained_by(event, loop); ensures normal resource_return_none(); };
extern fn ev_del(loop: ptr<mut, u8>, event: ptr<mut, u8>) -> i32 contract { requires resource_contract_v1; ensures normal resource_borrowed(loop); ensures normal resource_borrowed(event); quant live #loop: #holds(event) |-> #absent; ensures normal resource_return_none(); };
extern fn release(p: ptr<mut, u8>) -> unit contract { requires resource_contract_v1; ensures normal resource_released(p, nullable); ensures normal resource_return_none(); };
extern fn protocol_host(c: ptr<mut, u8>) -> unit contract { requires resource_contract_v1; ensures normal resource_borrowed(c); ensures normal resource_return_none(); quant registered #c: collector.event |-> collector.evl; };
"#;
    let base =
        load_resource_contract_sources(&BTreeMap::from([("s3.qv".into(), declarations.into())]))
            .unwrap();
    let prelude = "struct loop { int fd; }; struct event { int state; }; struct collector { struct loop *evl; struct event event; }; int ev_add(struct loop *, void *); int ev_del(struct loop *, void *); void release(void *);";
    let home_text =
        format!("{prelude}\nvoid starts(struct collector *c) {{ ev_add(c->evl, &c->event); }}\n");
    let home = prepare_translation_unit(&C11TranslationUnit {
        source: &home_text,
        function_scope: LegacyFunctionScope::MainSourceDefinitions,
        ..Default::default()
    });
    let digest = "a".repeat(64);
    let exports =
        export_cross_unit_homes(&home, &base, &BTreeSet::from(["starts".into()]), &digest).unwrap();
    assert_eq!(exports.len(), 1);
    let effect = exports[0].lifetime.clone().expect("lifetime carrier");
    assert_eq!(effect.effects.len(), 1);
    let mut env = base;
    env.set_lifetime_imports(BTreeMap::from([("starts".into(), effect.clone())]));
    env.set_cross_unit_import_identities(BTreeMap::from([(
        "starts".into(),
        exports[0].home.import_identity(),
    )]));
    env.set_link_resolution_sha256(Some(digest));
    assert!(
        env.contract("starts").is_none(),
        "ordinary consumers cannot read a borrowed projection"
    );
    {
        let body = "starts(c);";
        let expected_effects = 1;
        let source = format!(
            "{prelude}\nvoid starts(struct collector *); void caller(struct collector *c) {{ {body} }}\n"
        );
        let caller = prepare_translation_unit(&C11TranslationUnit {
            source: &source,
            function_scope: LegacyFunctionScope::MainSourceDefinitions,
            ..Default::default()
        });
        assert!(
            analyze_prepared_translation_unit_with_cross_unit(&caller, "caller", &env, &[])
                .is_err()
        );
        let result = analyze_prepared_translation_unit_with_cross_unit(
            &caller,
            "caller",
            &env,
            &[exports[0].home.clone()],
        )
        .unwrap();
        let UnrestrictedLegacyResult::Safe(certificate) = result else {
            panic!("lifetime caller refused: {result:?}");
        };
        assert_eq!(
            certificate.summary.functions["caller"]
                .lifetime_effects
                .len(),
            expected_effects
        );
        check_legacy_safety_certificate(&certificate).unwrap();
        let mut changed = certificate.clone();
        changed.contracts.set_lifetime_imports(BTreeMap::new());
        assert!(check_legacy_safety_certificate(&changed).is_err());
        let mut changed = certificate.clone();
        let mut bad = effect.clone();
        bad.effects[0].registration.poisoned = true;
        changed
            .contracts
            .set_lifetime_imports(BTreeMap::from([("starts".into(), bad)]));
        assert!(check_legacy_safety_certificate(&changed).is_err());
        let mut changed = certificate.clone();
        changed.cross_unit_homes[0].preprocessed_source.push(' ');
        assert!(check_legacy_safety_certificate(&changed).is_err());
    }
    assert_eq!(effect.holder_frames.len(), 1);
    for body in [
        "starts(c); ev_del(c->evl, &c->event);",
        "struct loop *l = c->evl; starts(c); ev_del(l, &c->event);",
    ] {
        let source = format!(
            "{prelude} void starts(struct collector *); void caller(struct collector *c) {{ {body} }}"
        );
        let caller = prepare_translation_unit(&C11TranslationUnit {
            source: &source,
            function_scope: LegacyFunctionScope::MainSourceDefinitions,
            ..Default::default()
        });
        let result = analyze_prepared_translation_unit_with_cross_unit(
            &caller,
            "caller",
            &env,
            &[exports[0].home.clone()],
        )
        .unwrap();
        let UnrestrictedLegacyResult::Safe(cert) = result else {
            panic!("checked frame refused {body}: {result:?}")
        };
        assert!(
            cert.summary.functions["caller"].lifetime_effects.is_empty(),
            "removal did not discharge the exact holder"
        );
        check_legacy_safety_certificate(&cert).unwrap();
        let mut bad = cert.clone();
        let mut altered = effect.clone();
        altered.holder_frames[0].fields[0] = "event".into();
        bad.contracts
            .set_lifetime_imports(BTreeMap::from([("starts".into(), altered)]));
        assert!(check_legacy_safety_certificate(&bad).is_err());
    }
    // Frames preserve exact holder identity; they do not repair poison or release.
    for body in [
        "starts(c); ev_del(other, &c->event); ev_del(c->evl, &c->event);",
        "starts(c); release(c); ev_del(c->evl, &c->event);",
    ] {
        let source = format!(
            "{prelude}\nvoid starts(struct collector *); void caller(struct collector *c, struct loop *other) {{ {body} }}\n"
        );
        let caller = prepare_translation_unit(&C11TranslationUnit {
            source: &source,
            function_scope: LegacyFunctionScope::MainSourceDefinitions,
            ..Default::default()
        });
        let result = analyze_prepared_translation_unit_with_cross_unit(
            &caller,
            "caller",
            &env,
            &[exports[0].home.clone()],
        )
        .unwrap();
        if let UnrestrictedLegacyResult::Safe(cert) = result {
            assert!(
                cert.summary.functions["caller"]
                    .lifetime_effects
                    .iter()
                    .any(|e| e.registration.poisoned),
                "poison was discharged: {body}"
            );
            check_legacy_safety_certificate(&cert).unwrap();
        }
    }
}

#[test]
fn ordered_removal_imports_replay_source_order_and_cannot_be_erased_or_reversed() {
    let declarations = r#"module contract;
extern fn add(loop: ptr<mut, u8>, event: ptr<mut, u8>) -> unit contract { requires resource_contract_v1; ensures normal resource_borrowed(loop); ensures normal resource_borrowed(event); ensures normal resource_retained_by(event, loop); ensures normal resource_return_none(); };
extern fn del(loop: ptr<mut, u8>, event: ptr<mut, u8>) -> unit contract { requires resource_contract_v1; ensures normal resource_borrowed(loop); ensures normal resource_borrowed(event); quant live #loop: #holds(event) |-> #absent; ensures normal resource_return_none(); };
"#;
    let base = load_resource_contract_sources(&BTreeMap::from([(
        "removals.qv".into(),
        declarations.into(),
    )]))
    .unwrap();
    let prelude = "struct loop { int fd; }; struct event { int state; }; struct collector { struct loop *evl; struct event event; }; void add(struct loop *, void *); void del(struct loop *, void *);";
    let caller_text = format!(
        "{prelude}\nvoid clean(struct loop *, struct loop *, void *); void caller(struct collector *c, struct collector *other) {{ add(c->evl, &c->event); clean(c->evl, other->evl, &c->event); }}\n"
    );
    let caller = prepare_translation_unit(&C11TranslationUnit {
        source: &caller_text,
        function_scope: LegacyFunctionScope::MainSourceDefinitions,
        ..Default::default()
    });
    let digest = "a".repeat(64);
    for (body, safe) in [
        ("del(l, e); del(other, e);", true),
        ("del(other, e); del(l, e);", false),
    ] {
        let home_text = format!(
            "{prelude}\nvoid clean(struct loop *l, struct loop *other, void *e) {{ {body} }}\n"
        );
        let home = prepare_translation_unit(&C11TranslationUnit {
            source: &home_text,
            function_scope: LegacyFunctionScope::MainSourceDefinitions,
            ..Default::default()
        });
        let exports =
            export_cross_unit_homes(&home, &base, &BTreeSet::from(["clean".into()]), &digest)
                .unwrap();
        assert_eq!(exports.len(), 1);
        let effect = exports[0].lifetime.clone().expect("ordered effect carrier");
        assert!(effect.effects.is_empty());
        assert_eq!(effect.removals.len(), 2);
        let mut env = base.clone();
        env.set_lifetime_imports(BTreeMap::from([("clean".into(), effect.clone())]));
        env.set_cross_unit_import_identities(BTreeMap::from([(
            "clean".into(),
            exports[0].home.import_identity(),
        )]));
        env.set_link_resolution_sha256(Some(digest.clone()));
        assert!(env.contract("clean").is_none());
        let result = analyze_prepared_translation_unit_with_cross_unit(
            &caller,
            "caller",
            &env,
            &[exports[0].home.clone()],
        )
        .unwrap();
        if safe {
            let UnrestrictedLegacyResult::Safe(certificate) = result else {
                panic!("ordered removals did not compose: {result:?}");
            };
            check_legacy_safety_certificate(&certificate).unwrap();
            for reverse in [true, false] {
                let mut changed = certificate.clone();
                let mut bad = effect.clone();
                if reverse {
                    bad.removals.reverse();
                } else {
                    bad.removals.clear();
                }
                changed
                    .contracts
                    .set_lifetime_imports(BTreeMap::from([("clean".into(), bad)]));
                assert!(check_legacy_safety_certificate(&changed).is_err());
            }
            let mut erased = certificate.clone();
            erased.contracts.set_lifetime_imports(BTreeMap::new());
            erased
                .contracts
                .set_inferred_imports(BTreeMap::from([("clean".into(), effect.contract.clone())]));
            assert!(check_legacy_safety_certificate(&erased).is_err());
            let mut seeded = certificate.clone();
            seeded.contracts.set_lifetime_imports(BTreeMap::new());
            seeded
                .contracts
                .set_cross_unit_import_identities(BTreeMap::new());
            seeded.cross_unit_homes.clear();
            seeded
                .contracts
                .insert_inferred_lifetime("clean".into(), effect.clone());
            assert!(
                check_legacy_safety_certificate(&seeded)
                    .unwrap_err()
                    .contains("local inferred lifetime")
            );
        } else {
            let UnrestrictedLegacyResult::Unknown {
                partial: Some(summary),
                ..
            } = result
            else {
                panic!("wrong-holder poison did not refuse: {result:?}");
            };
            assert!(
                format!("{:?}", summary.functions["caller"].diagnostics)
                    .contains("retention-removal-poisoned")
            );
        }
    }
}
