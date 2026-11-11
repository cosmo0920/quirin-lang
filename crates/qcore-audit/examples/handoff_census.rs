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

//! Card SUPPLIED-OWNED-RETURN-CODE-HANDOFF: a census aid (not evidence by
//! itself). Reads the member facts of every compile-database unit whose path
//! contains FILTER, exactly as the pinned route's pre-pass reads a unit
//! (`member_prepass::read_facts`), and runs rule 12m's caller branch check
//! (`handoff_branch::admit`) for each hand-off the card's census predicts,
//! printing the answer and, for each function on the paths, its reading.
//! Card HANDOFF-DIRECT-CALL-READING adds the calling bodies' readings of the
//! four direct calls of `rd_kafka_poll_cb` (`direct_admission`).
//!
//! usage: handoff_census DATABASE FILTER JOBS OUT.json

use std::collections::BTreeMap;
use std::sync::Mutex;

use qcore_c_gateway::callback_member_closure::MemberFacts;
use qcore_c_gateway::handoff_branch::{
    Handed, Invocation, Query, SiteCallee, UnitBodies, admit, direct_admission, resolve,
};
use qcore_schemas::handoff::{GuardOp, HandOffGuard};

fn main() -> Result<(), String> {
    // Card RULE-SWITCH-ENTRY-POINTS: this example records no gateway rule set.
    qcore_c_gateway::gateway_rules::exit_if_unrecorded("example handoff_census");
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    let [database, filter, jobs, out] = arguments.as_slice() else {
        return Err("usage: handoff_census DATABASE FILTER JOBS OUT.json".into());
    };
    let jobs: usize = jobs.parse().map_err(|error| format!("JOBS: {error}"))?;
    let text = std::fs::read_to_string(database).map_err(|error| error.to_string())?;
    let entries = qcore_audit::compile_db::compile_entries(&text)?
        .into_iter()
        .filter(|entry| entry.file.to_string_lossy().contains(filter.as_str()))
        .collect::<Vec<_>>();
    let next = Mutex::new(0usize);
    let read = Mutex::new(BTreeMap::<String, Result<MemberFacts, String>>::new());
    std::thread::scope(|scope| {
        for _ in 0..jobs.max(1) {
            scope.spawn(|| {
                loop {
                    let index = {
                        let mut next = next.lock().expect("lock");
                        let index = *next;
                        *next += 1;
                        index
                    };
                    let Some(entry) = entries.get(index) else {
                        return;
                    };
                    let facts = qcore_audit::member_prepass::read_facts(entry);
                    read.lock()
                        .expect("lock")
                        .insert(entry.file.to_string_lossy().into_owned(), facts);
                }
            });
        }
    });
    let read = read.into_inner().expect("lock");
    let units = read
        .values()
        .filter_map(|facts| facts.as_ref().ok())
        .collect::<Vec<_>>();
    println!("{} unit(s) read of {}", units.len(), entries.len());
    let not_pass = HandOffGuard {
        op: GuardOp::Ne,
        constant: "RD_KAFKA_OP_RES_PASS".into(),
    };
    let keep = HandOffGuard {
        op: GuardOp::Eq,
        constant: "RD_KAFKA_OP_RES_KEEP".into(),
    };
    let member = |object: Option<usize>, path: &str| Invocation::Member {
        object,
        path: path.to_owned(),
    };
    let queries: Vec<(&str, &str, Invocation, Handed, &HandOffGuard)> = vec![
        (
            "rd_kafka_poll_cb",
            "rd_kafka_q_serve0",
            Invocation::Formal(4),
            Handed::Own,
            &not_pass,
        ),
        (
            "rd_kafka_poll_cb",
            "rd_kafka_q_serve",
            Invocation::Formal(4),
            Handed::Own,
            &not_pass,
        ),
        (
            "rd_kafka_poll_cb",
            "rd_kafka_q_serve_maybe_consume",
            Invocation::Formal(4),
            Handed::Own,
            &not_pass,
        ),
        (
            "rd_kafka_poll_cb",
            "rd_kafka_q_pop_serve0",
            Invocation::Formal(4),
            Handed::Own,
            &not_pass,
        ),
        (
            "rd_kafka_poll_cb",
            "rd_kafka_q_pop_serve",
            Invocation::Formal(4),
            Handed::Own,
            &not_pass,
        ),
        (
            "rd_kafka_poll_cb",
            "rd_kafka_q_pop_serve_maybe_consume",
            Invocation::Formal(4),
            Handed::Own,
            &not_pass,
        ),
        (
            "rd_kafka_poll_cb",
            "rd_kafka_q_serve0",
            member(None, "rko_serve"),
            Handed::Own,
            &not_pass,
        ),
        (
            "rd_kafka_txn_op_send_offsets_to_transaction",
            "rd_kafka_q_serve0",
            member(None, "rko_op_cb"),
            Handed::Own,
            &keep,
        ),
        (
            "rd_kafka_txn_op_send_offsets_to_transaction",
            "rd_kafka_q_pop_serve0",
            member(None, "rko_op_cb"),
            Handed::Own,
            &keep,
        ),
        (
            "rd_kafka_txn_op_send_offsets_to_transaction",
            "rd_kafka_op_destroy",
            member(Some(0), "rko_op_cb"),
            Handed::Parameter(0),
            &keep,
        ),
        // Card HANDOFF-CANDIDATE-OWN-CALLBACKS: the `rko_serve` invocation
        // through `rd_kafka_op_handle`'s exact copy, at every declared
        // invoker of the member; the two other `== KEEP` hand-offs at
        // `rd_kafka_op_destroy`; and the three as `rd_kafka_poll_cb`'s own
        // `rko_op_cb` invocation (the composition's inner check).
        (
            "rd_kafka_poll_cb",
            "rd_kafka_q_serve",
            member(None, "rko_serve"),
            Handed::Own,
            &not_pass,
        ),
        (
            "rd_kafka_poll_cb",
            "rd_kafka_q_serve_maybe_consume",
            member(None, "rko_serve"),
            Handed::Own,
            &not_pass,
        ),
        (
            "rd_kafka_poll_cb",
            "rd_kafka_q_pop_serve0",
            member(None, "rko_serve"),
            Handed::Own,
            &not_pass,
        ),
        (
            "rd_kafka_poll_cb",
            "rd_kafka_op_req0",
            member(None, "rko_serve"),
            Handed::Own,
            &not_pass,
        ),
        (
            "rd_kafka_admin_worker",
            "rd_kafka_op_destroy",
            member(Some(0), "rko_op_cb"),
            Handed::Parameter(0),
            &keep,
        ),
        (
            "rd_kafka_topic_partition_list_query_leaders_async_worker_op_cb",
            "rd_kafka_op_destroy",
            member(Some(0), "rko_op_cb"),
            Handed::Parameter(0),
            &keep,
        ),
        (
            "rd_kafka_txn_op_send_offsets_to_transaction",
            "rd_kafka_poll_cb",
            member(Some(2), "rko_op_cb"),
            Handed::Parameter(2),
            &keep,
        ),
        (
            "rd_kafka_admin_worker",
            "rd_kafka_poll_cb",
            member(Some(2), "rko_op_cb"),
            Handed::Parameter(2),
            &keep,
        ),
    ];
    let mut answers = serde_json::Map::new();
    for (candidate, invoker, invocation, handed, guard) in queries {
        let answer = resolve(&units, candidate, &guard.constant).and_then(|value| {
            admit(
                &UnitBodies(units.clone()),
                &Query {
                    invoker,
                    invocation: invocation.clone(),
                    position: 2,
                    handed,
                    guard,
                    value,
                },
            )
            .map(|()| value)
        });
        let line = format!(
            "{candidate} at {invoker} {invocation:?} `{}`: {}",
            guard.spelling(),
            match &answer {
                Ok(value) => format!("admitted (value {value})"),
                Err(why) => format!("refused: {why}"),
            }
        );
        println!("{line}");
        answers.insert(
            format!("{candidate} @ {invoker} {invocation:?}"),
            serde_json::json!({
                "guard": guard.spelling(),
                "admitted": answer.is_ok(),
                "why": answer.err(),
            }),
        );
    }
    // Card HANDOFF-DIRECT-CALL-READING (rule 12m amended): the calling
    // body's reading of each direct call of `rd_kafka_poll_cb`
    // (`handoff_branch::direct_admission`), in the unit that defines it.
    let mut direct = serde_json::Map::new();
    for caller in [
        "rd_kafka_consume0",
        "rd_kafka_consumer_close0",
        "rd_kafka_q_serve_rkmessages",
        "rd_kafka_background_queue_serve",
    ] {
        let found = read
            .iter()
            .filter_map(|(file, facts)| {
                let facts = facts.as_ref().ok()?;
                let handling = facts.handling.get(caller)?;
                Some((file.clone(), facts, handling))
            })
            .collect::<Vec<_>>();
        let answer = match found.as_slice() {
            [(file, facts, handling)] => resolve(&[*facts], "rd_kafka_poll_cb", &not_pass.constant)
                .map_err(|why| format!("not read: {why}"))
                .and_then(|value| {
                    direct_admission(handling, "rd_kafka_poll_cb", 2, &not_pass, value)
                        .map_err(|refusal| format!("{refusal:?}"))
                })
                .map(|()| file.clone()),
            [] => Err("no unit reads the caller".to_owned()),
            _ => Err(format!("{} units read the caller", found.len())),
        };
        let sites = found
            .first()
            .map(|(_, _, handling)| {
                handling
                    .sites
                    .iter()
                    .filter(|site| site.callee == SiteCallee::Direct("rd_kafka_poll_cb".to_owned()))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        println!(
            "direct rd_kafka_poll_cb at {caller}: {}",
            match &answer {
                Ok(file) => format!("admitted ({file})"),
                Err(why) => format!("refused: {why}"),
            }
        );
        direct.insert(
            caller.to_owned(),
            serde_json::json!({
                "admitted": answer.is_ok(),
                "why": answer.err(),
                "sites": serde_json::to_value(&sites).map_err(|error| error.to_string())?,
            }),
        );
    }
    let mut readings = serde_json::Map::new();
    for function in [
        "rd_kafka_q_serve",
        "rd_kafka_q_serve0",
        "rd_kafka_q_pop_serve0",
        "rd_kafka_op_handle",
        "rd_kafka_op_handle_std",
        "rd_kafka_op_call",
        "rd_kafka_op_destroy",
    ] {
        let found = units
            .iter()
            .filter_map(|facts| facts.handling.get(function))
            .collect::<Vec<_>>();
        readings.insert(
            function.to_owned(),
            serde_json::to_value(found.first()).map_err(|error| error.to_string())?,
        );
        println!(
            "{function}: {} reading(s), {} distinct",
            found.len(),
            found
                .iter()
                .map(|reading| serde_json::to_string(reading).unwrap_or_default())
                .collect::<std::collections::BTreeSet<_>>()
                .len()
        );
    }
    std::fs::write(
        out,
        serde_json::to_vec_pretty(&serde_json::json!({
            "answers": answers,
            "direct": direct,
            "readings": readings,
        }))
        .map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}
