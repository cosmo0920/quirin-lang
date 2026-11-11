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

//! Explicit external-input replay, never repository input discovery.
use qcore_contract::scheduler_lifetime::*;
use qcore_schemas::hypothesis::Context;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(std::env::args().nth(1).ok_or("source root required")?);
    let sources: BTreeMap<_, _> = SOURCE_FILES
        .iter()
        .map(|(p, _)| Ok((p.to_string(), std::fs::read(root.join(p))?)))
        .collect::<Result<_, std::io::Error>>()?;
    let source = SourceBinding::verify(&sources)?;
    let context = Context {
        artifact: "scheduler-model-control".into(),
        property: "finite-lifetime-model".into(),
        semantic_profile: "monkey-scheduler-finite-v1".into(),
        scope: "one suspended coroutine".into(),
        bounds: "32 threads;512 live;64 callback effects;2560 trace steps".into(),
        assumptions: "declared finite model; no C truth or callback closure".into(),
        source_identity: source.digest().into(),
        build_identity: "source-byte-replay-only".into(),
        evidence_version: env!("CARGO_PKG_VERSION").into(),
    };
    let model = Model {
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
        live: BTreeSet::from([2, 3, 4, 5, 6, 9]),
        callback_owned: BTreeSet::new(),
        event_connection: BTreeSet::new(),
    };
    let trace = [
        Step::FreeTls(9),
        Step::ClearTls,
        Step::Unlink(2),
        Step::ResetEvent(6),
        Step::DeleteCoroutine(3),
        Step::FreeParent(2),
    ];
    assert_eq!(
        check_source_bound_model(&source, &context, &context, &model, &trace)?.count(),
        1
    );
    let value = serde_json::to_value(&context)?;
    let keys: Vec<_> = value.as_object().unwrap().keys().cloned().collect();
    for key in &keys {
        let mut changed = value.clone();
        changed[key] = "wrong-context".into();
        let changed: Context = serde_json::from_value(changed)?;
        assert!(check_source_bound_model(&source, &context, &changed, &model, &trace).is_err());
    }
    for (path, _) in SOURCE_FILES {
        let mut changed = sources.clone();
        changed.get_mut(*path).unwrap().push(b' ');
        assert!(SourceBinding::verify(&changed).is_err());
        changed = sources.clone();
        changed.remove(*path);
        assert!(SourceBinding::verify(&changed).is_err());
    }
    let mut changed = sources.clone();
    changed.insert("extra".into(), vec![]);
    assert!(SourceBinding::verify(&changed).is_err());
    let qgd = include_str!("../../../docs/reports/scheduler-lifetime-contracts/scheduler.qgd");
    let clauses = qcore_frontend::guarded_decomposition::parse(qgd)?;
    println!(
        "{}",
        serde_json::json!({"positive_model_controls":1,"context_refusals":keys.len(),
        "source_refusals":SOURCE_FILES.len()*2+1,"parsed_clauses":clauses.len(),"source_digest":source.digest(),
        "scope":"exact pinned source binding plus finite model, no C execution/truth/standing"})
    );
    Ok(())
}
