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

use qcore_contract::logger_effects::{Event, Plan, Site, Source, Terminal, check};
use std::collections::BTreeMap;
const REVIEWED: &[u8] = include_bytes!("../../../docs/reports/logger-effect-contracts/model.json");

#[test]
fn reviewed_model_has_an_independently_pinned_digest() {
    assert_eq!(
        qcore_schemas::sha256_hex(REVIEWED),
        "1c1ffedb049c363f1003c3eaa1e3110c08828375ab85e12e647578a3f423ed18"
    );
}

// Synthetic bytes test the checker independently of access to external source
// checkouts. Actual pinned C replay is a separate measured example invocation.
fn fixture() -> (Plan, BTreeMap<String, Vec<u8>>) {
    let mut p: Plan = serde_json::from_slice(REVIEWED).unwrap();
    let bytes = b"first\nsecond\n".to_vec();
    p.sources = BTreeMap::from([(
        "test.c".into(),
        Source {
            sha256: qcore_schemas::sha256_hex(&bytes),
            sites: vec![Site {
                first: 2,
                last: 2,
                sha256: qcore_schemas::sha256_hex(b"second\n"),
            }],
        },
    )]);
    (p, BTreeMap::from([("test.c".into(), bytes)]))
}
fn run(
    p: &Plan,
    sources: &BTreeMap<String, Vec<u8>>,
) -> Result<qcore_contract::logger_effects::CheckedPlan, String> {
    let b = serde_json::to_vec(p).unwrap();
    check(&b, &qcore_schemas::sha256_hex(&b), sources)
}
#[test]
fn cyclic_closure_retains_every_open_obligation_and_all_alternatives() {
    let (p, s) = fixture();
    let c = run(&p, &s).unwrap();
    assert_eq!(c.reachable().len(), 10);
    assert_eq!(c.open_obligations().len(), 15);
    assert_eq!(c.terminals().len(), 10);
    assert_eq!(c.terminals()["forwarded_queue"], Terminal::Queued);
    assert_eq!(
        c.terminals()["forwarded_disabled_no_reply"],
        Terminal::Destroyed
    );
    assert!(c.open_obligations().contains("log_callback_effects"));
    assert!(c.open_obligations().contains("forwarding_termination"));
    assert!(c.open_obligations().contains("source_model_correspondence"));
}
#[test]
fn independent_digest_refuses_deleted_branch_or_changed_proposition() {
    let (mut p, s) = fixture();
    let old = serde_json::to_vec(&p).unwrap();
    let expected = qcore_schemas::sha256_hex(&old);
    p.paths.remove("disabled_reply_queue");
    assert!(check(&serde_json::to_vec(&p).unwrap(), &expected, &s).is_err());
    let (mut p, s) = fixture();
    p.obligations
        .insert("log_callback_effects".into(), "neutral".into());
    assert!(check(&serde_json::to_vec(&p).unwrap(), &expected, &s).is_err());
}
#[test]
fn source_bytes_sites_domain_and_paths_are_exact() {
    let (p, mut s) = fixture();
    s.get_mut("test.c").unwrap()[0] = b'X';
    assert!(run(&p, &s).is_err());
    let (mut p, s) = fixture();
    p.sources.get_mut("test.c").unwrap().sites[0].last = 3;
    assert!(run(&p, &s).is_err());
    let (mut p, s) = fixture();
    p.sources.get_mut("test.c").unwrap().sites[0].sha256 = "0".repeat(64);
    assert!(run(&p, &s).is_err());
    let (p, mut s) = fixture();
    s.insert("extra.c".into(), vec![]);
    assert!(run(&p, &s).is_err());
    let (mut p, mut s) = fixture();
    let pin = p.sources.remove("test.c").unwrap();
    p.sources.insert("../test.c".into(), pin);
    let bytes = s.remove("test.c").unwrap();
    s.insert("../test.c".into(), bytes);
    assert!(run(&p, &s).is_err());
}
#[test]
fn missing_callees_or_leaves_never_become_neutral() {
    let (mut p, s) = fixture();
    p.nodes.remove("rd_kafka_op_reply");
    assert!(run(&p, &s).is_err());
    let (mut p, s) = fixture();
    p.obligations.remove("destroy_callback_effects");
    assert!(run(&p, &s).is_err());
    let (mut p, s) = fixture();
    p.nodes
        .get_mut("rd_kafka_q_enq1")
        .unwrap()
        .calls
        .push("rd_kafka_q_enq1".into());
    assert!(run(&p, &s).is_err());
    let (mut p, s) = fixture();
    p.nodes.get_mut("rd_kafka_log_buf").unwrap().calls.clear();
    assert!(run(&p, &s).is_err());
}
#[test]
fn double_creation_double_free_leak_and_use_after_transfer_refuse() {
    for events in [
        vec![Event::CreateOp, Event::CreateOp],
        vec![Event::CreateOp, Event::CreateText, Event::DestroyOp],
        vec![
            Event::CreateOp,
            Event::CreateText,
            Event::DestroyText,
            Event::DestroyText,
        ],
        vec![Event::CreateOp, Event::CreateText],
        vec![
            Event::CreateOp,
            Event::CreateText,
            Event::Transfer,
            Event::Callback,
        ],
        vec![
            Event::CreateOp,
            Event::CreateText,
            Event::Transfer,
            Event::DestroyText,
        ],
    ] {
        let (mut p, s) = fixture();
        p.paths.get_mut("local_queue").unwrap().events = events;
        assert!(run(&p, &s).is_err());
    }
}
#[test]
fn every_unchosen_branch_and_terminal_is_checked() {
    let (mut p, s) = fixture();
    p.paths.get_mut("disabled_no_reply").unwrap().terminal = Terminal::Queued;
    assert!(run(&p, &s).is_err());
    let (mut p, s) = fixture();
    p.paths.get_mut("immediate_callback").unwrap().events = vec![Event::DestroyOp];
    assert!(run(&p, &s).is_err());
}
#[test]
fn recursion_is_bounded_without_assuming_an_acyclic_effect_graph() {
    let (mut p, s) = fixture();
    let path = p.paths.get_mut("forwarded_queue").unwrap();
    path.events = vec![Event::CreateOp, Event::CreateText];
    path.events.extend(vec![Event::Forward; 16]);
    path.events.push(Event::Transfer);
    assert!(run(&p, &s).is_ok());
    p.paths
        .get_mut("forwarded_queue")
        .unwrap()
        .events
        .insert(2, Event::Reply);
    assert!(run(&p, &s).is_err());
}
#[test]
fn versions_unknown_events_and_hard_bounds_refuse() {
    let (mut p, s) = fixture();
    p.version = "logger-effects-v2".into();
    assert!(run(&p, &s).is_err());
    let (mut p, s) = fixture();
    p.max_nodes = 65;
    assert!(run(&p, &s).is_err());
    let (mut p, s) = fixture();
    p.max_events = 65;
    assert!(run(&p, &s).is_err());
    let (mut p, s) = fixture();
    p.paths.clear();
    assert!(run(&p, &s).is_err());
    let bad = String::from_utf8(REVIEWED.to_vec())
        .unwrap()
        .replace("create-op", "consume-op");
    assert!(serde_json::from_str::<Plan>(&bad).is_err());
}
