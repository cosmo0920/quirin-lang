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

//! Ordered summaries must replay removal against the incoming poison state.
use qcore_c_gateway::loop_lifetime_summary::{self as lifetime, Event, Path, Registration, Root};
fn p(index: usize) -> Path {
    Path {
        root: Root::Formal {
            scope: "f".into(),
            index,
        },
        fields: vec![],
    }
}
fn registration(poisoned: bool) -> Event {
    Event::Register(Registration {
        object: p(0),
        holder: p(1),
        poisoned,
    })
}
#[test]
fn ordered_calls_remove_only_exact_unpoisoned_records_before_release() {
    let removes = lifetime::path(&[Event::Remove(Some(p(0)), Some(p(1)))]);
    let safe = lifetime::path(&[
        registration(false),
        Event::Call(removes.clone()),
        Event::Release(p(0)),
    ]);
    assert!(safe.may_register.is_empty());
    assert!(safe.requirements.is_empty());
    for prefix in [
        vec![registration(true)],
        vec![registration(false), Event::Remove(Some(p(0)), Some(p(2)))],
    ] {
        let mut events = prefix;
        events.extend([Event::Call(removes.clone()), Event::Release(p(0))]);
        let refused = lifetime::path(&events);
        assert!(!refused.may_register.is_empty());
        assert!(!refused.requirements.is_empty());
    }
    let late = lifetime::path(&[
        registration(false),
        Event::Release(p(0)),
        Event::Call(removes),
    ]);
    assert!(!late.requirements.is_empty());
}
#[test]
fn substitution_join_and_bounds_withhold_unproved_order() {
    let removes = lifetime::path(&[Event::Remove(Some(p(0)), Some(p(1)))]);
    let substituted = lifetime::substitute(&removes, "f", &[Some(p(2)), Some(p(3))], None, None);
    assert!(substituted.ordered.is_some());
    let unresolved = lifetime::substitute(&removes, "f", &[None, Some(p(3))], None, Some("type"));
    assert!(unresolved.ordered.is_none());
    let joined = lifetime::join(&[removes.clone(), lifetime::path(&[])]);
    assert!(joined.ordered.is_none());
    let refused = lifetime::path(&[
        registration(false),
        Event::Call(joined),
        Event::Release(p(0)),
    ]);
    assert!(!refused.requirements.is_empty());
    assert!(
        lifetime::join(&[removes.clone(), removes])
            .ordered
            .is_some()
    );
    let large = lifetime::path(&vec![Event::Remove(Some(p(0)), Some(p(1))); 4097]);
    assert!(large.ordered.is_none());
    assert!(!large.open.is_empty());
}

#[test]
fn source_ordered_helper_composition_keeps_poison_and_conditional_refusals() {
    use qcore_c_gateway::{legacy_c::C11TranslationUnit, loop_lifetime_source};
    use qcore_contract::contract_loader::load_resource_contract_sources;
    use qcore_schemas::loop_lifetime::RegisteredProtocol;
    use std::collections::{BTreeMap, BTreeSet};
    let texts = BTreeMap::from([("ordered.qv".into(), r#"module contract;
extern fn add(loop: ptr<mut, u8>, event: ptr<mut, u8>) -> unit contract { requires resource_contract_v1; ensures normal resource_borrowed(loop); ensures normal resource_borrowed(event); ensures normal resource_retained_by(event, loop); ensures normal resource_return_none(); };
extern fn del(loop: ptr<mut, u8>, event: ptr<mut, u8>) -> unit contract { requires resource_contract_v1; ensures normal resource_borrowed(loop); ensures normal resource_borrowed(event); quant live #loop: #holds(event) |-> #absent; ensures normal resource_return_none(); };
"#.into())]);
    let env = load_resource_contract_sources(&texts).unwrap();
    let c = "struct loop { int fd; }; struct event { int state; }; struct collector { struct loop *evl; struct loop *other; struct event event; }; void add(struct loop *, void *); void del(struct loop *, void *); static void stop(struct collector *c) { del(c->evl, &c->event); } void sequence(struct collector *c) { add(c->evl, &c->event); stop(c); } void poisoned(struct collector *c) { add(c->evl, &c->event); del(c->other, &c->event); stop(c); } void conditional(struct collector *c, int b) { add(c->evl, &c->event); if (b) stop(c); }";
    let unit = C11TranslationUnit {
        source: c,
        ..Default::default()
    };
    let facts = loop_lifetime_source::read_unit(c, &unit, &env).unwrap();
    let protocol = RegisteredProtocol {
        binder: "c".into(),
        record: "collector".into(),
        interior: vec!["event".into()],
        holder: vec!["evl".into()],
    };
    let summaries = lifetime::derive_source(&facts, &[protocol], &BTreeSet::new(), &env);
    assert!(summaries["sequence"].ordered.is_some());
    assert!(summaries["sequence"].may_register.is_empty());
    assert!(
        summaries["poisoned"]
            .may_register
            .iter()
            .any(|r| r.poisoned)
    );
    assert!(!summaries["conditional"].may_register.is_empty());
    assert!(summaries["conditional"].ordered.is_none());
    let external = c.replace("static void stop", "void stop");
    let unit = C11TranslationUnit {
        source: &external,
        ..Default::default()
    };
    let facts = loop_lifetime_source::read_unit(&external, &unit, &env).unwrap();
    let summaries = lifetime::derive_source(
        &facts,
        &[RegisteredProtocol {
            binder: "c".into(),
            record: "collector".into(),
            interior: vec!["event".into()],
            holder: vec!["evl".into()],
        }],
        &BTreeSet::new(),
        &env,
    );
    assert!(summaries["sequence"].ordered.is_none());
    assert!(!summaries["sequence"].may_register.is_empty());
}
