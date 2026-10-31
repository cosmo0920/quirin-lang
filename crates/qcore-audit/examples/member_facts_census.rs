//! Card RKO-OP-CB-RUN-SET-OPEN-THROUGH-UNION: a census aid (not evidence by
//! itself). Reads the member facts of every compile-database unit whose path
//! contains FILTER, exactly as the pinned route's pre-pass reads a unit
//! (`member_prepass::read_facts`), writes each unit's facts as JSON to OUT, and
//! prints the closure of every MEMBER over the units read.
//!
//! usage: member_facts_census DATABASE FILTER OUT.json JOBS MEMBER...

use std::collections::BTreeMap;
use std::sync::Mutex;

use qcore_c_gateway::callback_member_closure::{MemberFacts, closure};

fn main() -> Result<(), String> {
    // Card RULE-SWITCH-ENTRY-POINTS: this example records no gateway rule set.
    qcore_c_gateway::gateway_rules::exit_if_unrecorded("example member_facts_census");
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    let [database, filter, out, jobs, members @ ..] = arguments.as_slice() else {
        return Err("usage: member_facts_census DATABASE FILTER OUT.json JOBS MEMBER...".into());
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
    let mut dump = serde_json::Map::new();
    let mut units = Vec::new();
    for (file, facts) in &read {
        match facts {
            Ok(facts) => {
                dump.insert(
                    file.clone(),
                    serde_json::to_value(facts).map_err(|error| error.to_string())?,
                );
                units.push(facts);
            }
            Err(why) => {
                println!("unread {file}: {why}");
                dump.insert(file.clone(), serde_json::json!({ "unread": why }));
            }
        }
    }
    std::fs::write(
        out,
        serde_json::to_vec(&dump).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    println!("{} unit(s) read of {}", units.len(), entries.len());
    for member in members {
        match closure(&units, member) {
            Ok(names) => println!("{member}: closed over {}: {names:?}", names.len()),
            Err(open) => println!("{member}: open: {open:?}"),
        }
    }
    Ok(())
}
