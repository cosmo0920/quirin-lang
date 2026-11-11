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

use qcore_contract::{bounds_contract, bounds_hypotheses, contract_loader};
use qcore_frontend::frontend::parse_source_program;
use std::collections::BTreeMap;

fn sources() -> BTreeMap<String, String> {
    BTreeMap::from([
        (
            "cmetrics/map_lifetime.qv".into(),
            include_str!(
                "../../../providers/qcore-provider-cmetrics/contracts/cmetrics/map_lifetime.qv"
            )
            .into(),
        ),
        (
            "cmetrics/opts_lifetime.qv".into(),
            include_str!(
                "../../../providers/qcore-provider-cmetrics/contracts/cmetrics/opts_lifetime.qv"
            )
            .into(),
        ),
        (
            "cfl/hash_state.qv".into(),
            include_str!("../../../providers/quirin-provider-cfl/contracts/cfl/hash_state.qv")
                .into(),
        ),
    ])
}

#[test]
fn source_audited_api_models_are_retained_without_interpreted_permissions() {
    let s = sources();
    let graphs = bounds_hypotheses::replay_model(&s).unwrap();
    assert_eq!(
        graphs.keys().map(String::as_str).collect::<Vec<_>>(),
        [
            "cfl_hash_64bits_digest",
            "cfl_hash_64bits_reset",
            "cfl_hash_64bits_update",
            "cmt_map_create",
            "cmt_map_destroy",
            "cmt_map_metric_destroy",
            "cmt_map_metric_get",
            "cmt_map_metric_get_val",
            "cmt_map_metrics_expire",
            "cmt_opts_exit",
            "cmt_opts_init",
        ]
    );
    for (name, text) in &s {
        let one = BTreeMap::from([(name.clone(), text.clone())]);
        assert!(contract_loader::load_resource_contract_sources(&one).is_err());
        assert!(bounds_contract::load_bounds_contract_sources(&one).is_err());
        assert!(!text.contains("requires resource_contract_v1;"));
    }
    let clauses = |function: &str| {
        s.values()
            .flat_map(|text| parse_source_program(text).unwrap().declarations)
            .find(|d| d.value.name == function)
            .unwrap()
            .value
            .function_contract
            .unwrap()
            .clauses
            .into_iter()
            .map(|c| c.value.expression_source)
            .collect::<Vec<_>>()
    };
    let get = clauses("cmt_map_metric_get");
    assert!(
        get.iter()
            .any(|c| c == "result_is_null_or_borrowed_metric_from_map")
    );
    assert!(
        get.iter()
            .any(|c| c == "null_result_may_follow_committed_metric_or_storage_changes")
    );
    assert!(
        get.iter()
            .any(|c| c == "read_lookup_may_allocate_or_mutate_secondary_index")
    );
    assert!(get.iter().any(|c| c.starts_with("#hash_nonzero(")));
    let graph = &graphs["cmt_map_metric_get"];
    assert_eq!(graph.version, 4);
    assert!(graph.requirements.iter().any(|id| id == "hash_nonzero"));
    let digest = clauses("cfl_hash_64bits_digest");
    assert!(digest.iter().any(|c| c.starts_with("#digest_value(")));
    assert!(s["cfl/hash_state.qv"].contains("42 d5 68 e1 38 d7 27 ff"));
    let init = clauses("cmt_opts_init");
    assert!(init.iter().any(|c| c.starts_with("#stored_contents(")));
    assert!(
        s["cmetrics/opts_lifetime.qv"]
            .contains("A failed initializer may retain zero or more committed contents")
    );
    let exit = clauses("cmt_opts_exit");
    assert!(exit.iter().any(|c| c.starts_with("#field_bytes(")));
    assert!(exit.iter().any(|c| c.starts_with("#release_once(")));
    for function in ["cmt_opts_init", "cmt_opts_exit"] {
        let graph = &graphs[function];
        assert_eq!(graph.version, 4);
        assert!(graph.requirements.iter().any(|id| id == "call_exclusion"));
        assert!(graph.requirements.iter().any(|id| id == "helper_frame"));
    }
    assert!(
        graphs["cmt_opts_exit"]
            .requirements
            .iter()
            .any(|id| id == "sds_physical_frame")
    );
    let parsed = parse_source_program(&s["cmetrics/opts_lifetime.qv"]).unwrap();
    for declaration in parsed.declarations {
        if let Some(contract) = declaration.value.function_contract {
            assert!(contract.clauses.iter().all(|clause| matches!(
                clause.value.kind,
                qcore_frontend::frontend::SourceContractClauseKind::QuantCondition(_)
            )));
        }
    }
}

#[test]
fn lifecycle_witnesses_match_the_audited_lean_model() {
    let committed = [1_u32, 2];
    let can_exit = |stored: &[u32], owned: &[u32]| {
        stored
            .iter()
            .enumerate()
            .all(|(i, id)| !stored[..i].contains(id) && owned.contains(id))
    };
    let valid = |live: bool, epoch: u32, issued: u32, primary: &[u32], child: Option<u32>| {
        live && epoch == issued && child.is_none_or(|id| primary.contains(&id))
    };
    let attached = [8_u32];
    let trusted = contract_loader::load_resource_contract_sources(&sources()).is_ok();
    let actual = [
        can_exit(&committed, &committed),
        can_exit(&committed, &[]),
        can_exit(&[], &[]),
        valid(true, 0, 0, &[], None),
        valid(true, 0, 0, &[8], Some(8)),
        valid(true, 0, 0, &[8], Some(9)),
        valid(false, 1, 0, &[], Some(8)),
        valid(true, 1, 0, &[], Some(8)),
        attached.contains(&8),
        trusted,
        can_exit(&[1, 1], &[1]),
    ];
    assert_eq!(
        actual,
        [
            true, false, true, true, true, false, false, false, true, false, false
        ]
    );
    let lean = include_str!("../../../proof/QCore/CmetricsApiContracts.lean");
    assert!(lean.contains("[true,false,true,true,true,false,false,false,true,false,false]"));
    let getter_lean = include_str!("../../../proof/QCore/CMetricsGetterOwnership.lean");
    assert!(getter_lean.contains("theorem exact_short_hash_is_zero"));
    assert!(getter_lean.contains("theorem labeled_zero_path_reaches_destruction_leak"));
    // A NULL result after attachment is compatible with a live parent-owned
    // child; a failed initialization also retains its committed cleanup set.
    let returned: Option<u32> = None;
    assert!(returned.is_none() && attached.contains(&8));
    assert!(!can_exit(&committed, &[]));
}
