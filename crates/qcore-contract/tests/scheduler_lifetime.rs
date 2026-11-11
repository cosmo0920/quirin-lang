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

use qcore_contract::scheduler_lifetime::*;
use std::collections::{BTreeMap, BTreeSet};
fn model() -> Model {
    Model {
        worker: 1,
        current_worker: 1,
        current_thread: None,
        tls_params: Some(9),
        selected: vec![2],
        other: vec![],
        threads: BTreeMap::from([(
            2,
            Thread {
                parent: 2,
                coroutine: 3,
                suspended: true,
                borrowed: [4, 5, 6],
                callback: None,
            },
        )]),
        live: BTreeSet::from([2, 3, 4, 5, 6, 9, 10]),
        callback_owned: BTreeSet::from([10]),
        event_connection: BTreeSet::new(),
    }
}
fn trace() -> Vec<Step> {
    vec![
        Step::FreeTls(9),
        Step::ClearTls,
        Step::Unlink(2),
        Step::ResetEvent(6),
        Step::DeleteCoroutine(3),
        Step::FreeParent(2),
    ]
}
#[test]
fn cleanup_retires_only_owned_parent_coroutine_tls_and_preserves_borrows() {
    let checked = check_model(&model(), &trace()).unwrap();
    assert_eq!(checked.count(), 1);
    assert!(checked.after().selected.is_empty());
    assert_eq!(checked.after().live, BTreeSet::from([4, 5, 6, 10]));
    assert!(checked.after().event_connection.contains(&6));
}
#[test]
fn tls_absent_and_empty_list_are_distinct_real_alternatives() {
    let mut m = model();
    m.tls_params = None;
    assert!(check_model(&m, &trace()[2..]).is_ok());
    m.other = std::mem::take(&mut m.selected);
    let checked = check_model(&m, &[]).unwrap();
    assert_eq!(checked.after(), &m);
    m.tls_params = Some(9);
    assert_eq!(check_model(&m, &[]).unwrap().after().tls_params, Some(9));
}
#[test]
fn positive_callback_effects_are_checked_and_not_assumed_neutral() {
    for params in [false, true] {
        for effects in [
            vec![],
            vec![
                CallbackEffect::Read(2),
                CallbackEffect::Write(10),
                CallbackEffect::Release(10),
            ],
        ] {
            let mut m = model();
            if !params {
                m.tls_params = None;
            }
            m.threads.get_mut(&2).unwrap().callback = Some(effects.clone());
            let mut t = if params {
                trace()
            } else {
                trace()[2..].to_vec()
            };
            t.insert(
                if params { 4 } else { 2 },
                Step::Callback(2, effects.clone()),
            );
            let checked = check_model(&m, &t).unwrap();
            assert_eq!(checked.after().live.contains(&10), effects.is_empty());
            t.remove(if params { 4 } else { 2 });
            assert!(check_model(&m, &t).is_err());
        }
    }
}
#[test]
fn callback_cannot_release_parent_callee_borrow_or_saved_successor() {
    for resource in [2, 3, 4, 5, 6, 9] {
        let mut m = model();
        let effects = vec![CallbackEffect::Release(resource)];
        m.threads.get_mut(&2).unwrap().callback = Some(effects.clone());
        let mut t = trace();
        t.insert(4, Step::Callback(2, effects));
        assert!(check_model(&m, &t).is_err());
    }
    let mut m = model();
    m.threads.insert(
        7,
        Thread {
            parent: 7,
            coroutine: 8,
            suspended: true,
            borrowed: [4, 5, 6],
            callback: None,
        },
    );
    m.live.extend([7, 8]);
    m.selected.push(7);
    m.threads.get_mut(&2).unwrap().callback = Some(vec![CallbackEffect::Release(7)]);
    assert!(
        check_model(&m, &trace())
            .unwrap_err()
            .contains("callback invalidates")
    );
}
#[test]
fn invalid_membership_forbids_cleanup_and_refusal_preserves_input() {
    let mut m = model();
    m.selected.push(2);
    let original = m.clone();
    assert!(check_model(&m, &trace()).is_err());
    assert_eq!(m, original);
    m = model();
    m.other.push(2);
    assert!(check_model(&m, &trace()).is_err());
    m = model();
    m.selected.push(99);
    assert!(check_model(&m, &trace()).is_err());
}
#[test]
fn worker_running_coroutine_stale_tls_and_unowned_borrows_refuse() {
    let mut m = model();
    m.current_worker = 88;
    assert!(check_model(&m, &trace()).is_err());
    m = model();
    m.threads.get_mut(&2).unwrap().suspended = false;
    assert!(check_model(&m, &trace()).is_err());
    m = model();
    m.current_thread = Some(2);
    assert!(check_model(&m, &trace()).is_err());
    m = model();
    m.live.remove(&4);
    assert!(check_model(&m, &trace()).is_err());
    m = model();
    m.tls_params = Some(4);
    assert!(check_model(&m, &trace()).is_err());
}
#[test]
fn every_order_branch_and_terminal_effect_is_required() {
    for index in 0..trace().len() {
        let mut t = trace();
        t.remove(index);
        assert!(check_model(&model(), &t).is_err());
    }
    let mut t = trace();
    t.swap(2, 3);
    assert!(check_model(&model(), &t).is_err());
    t = trace();
    t.push(Step::FreeParent(2));
    assert!(check_model(&model(), &t).is_err());
}
#[test]
fn callback_double_release_unowned_write_and_finite_bounds_refuse() {
    for effects in [
        vec![CallbackEffect::Release(10), CallbackEffect::Read(10)],
        vec![CallbackEffect::Write(11)],
    ] {
        let mut m = model();
        m.live.insert(11);
        m.threads.get_mut(&2).unwrap().callback = Some(effects.clone());
        let mut t = trace();
        t.insert(4, Step::Callback(2, effects));
        assert!(check_model(&m, &t).is_err());
    }
    let m = model();
    assert!(check_model(&m, &vec![Step::ClearTls; 2561]).is_err());
    let mut m = model();
    m.threads.get_mut(&2).unwrap().callback = Some(vec![CallbackEffect::Read(4); 65]);
    assert!(check_model(&m, &trace()).is_err());
}
#[test]
fn source_binding_refuses_missing_added_and_modified_inventory() {
    assert!(SourceBinding::verify(&BTreeMap::new()).is_err());
    let mut sources: BTreeMap<_, _> = SOURCE_FILES
        .iter()
        .map(|(p, _)| (p.to_string(), vec![]))
        .collect();
    assert!(SourceBinding::verify(&sources).is_err());
    sources.insert("extra".into(), vec![]);
    assert!(SourceBinding::verify(&sources).is_err());
}

#[test]
fn event_state_and_current_tls_cannot_alias_callback_owned_storage() {
    let mut m = model();
    m.event_connection.insert(2);
    assert!(check_model(&m, &trace()).is_err());
    m = model();
    m.current_thread = Some(10);
    assert!(check_model(&m, &trace()).is_err());
}

#[test]
fn current_tls_callee_and_params_retirement_aliases_refuse_by_role() {
    // Both aliases were previously accepted and then left stale after free.
    for alias in [3, 9] {
        let mut m = model();
        m.current_thread = Some(alias);
        let original = m.clone();
        assert!(check_model(&m, &trace()).unwrap_err().contains("TLS role"));
        assert_eq!(m, original);
    }
}
#[test]
fn unselected_parent_tls_remains_live_after_cleanup() {
    let mut m = model();
    m.threads.insert(
        7,
        Thread {
            parent: 7,
            coroutine: 8,
            suspended: true,
            borrowed: [4, 5, 6],
            callback: None,
        },
    );
    m.live.extend([7, 8]);
    m.other.push(7);
    m.current_thread = Some(7);
    let checked = check_model(&m, &trace()).unwrap();
    assert_eq!(checked.after().current_thread, Some(7));
    assert!(checked.after().live.contains(&7));
    assert!(checked.after().live.contains(&8));
    assert_eq!(checked.after().other, vec![7]);
}
#[test]
fn empty_cleanup_preserves_valid_current_parent_and_params_but_rejects_wrong_roles() {
    let mut m = model();
    m.other = std::mem::take(&mut m.selected);
    m.current_thread = Some(2);
    assert_eq!(check_model(&m, &[]).unwrap().after(), &m);
    assert_eq!(m.tls_params, Some(9));
    assert!(m.live.contains(&9));
    for alias in [3, 9] {
        m.current_thread = Some(alias);
        assert!(check_model(&m, &[]).unwrap_err().contains("TLS role"));
    }
}
