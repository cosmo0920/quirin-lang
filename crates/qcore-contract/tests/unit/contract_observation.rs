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

use super::*;

#[test]
fn native_delete_observations_check_eligibility_arguments_and_frames() {
    let contract = include_str!(
        "../../../../providers/qcore-provider-chunkio/contracts/chunkio/filesystem_delete_observation.qv"
    );
    let observations = include_str!("../fixtures/chunkio/native_delete.jsonl");
    let mut scenarios = std::collections::BTreeSet::new();
    let mut modes = BTreeMap::new();
    for line in observations.lines() {
        let row: serde_json::Value = serde_json::from_str(line).unwrap();
        let values: BTreeMap<String, i128> = serde_json::from_value(row["values"].clone()).unwrap();
        let function = row["function"].as_str().unwrap();
        assert_eq!(function, "native_delete_observation");
        assert_eq!(row["helper"], "cio_file_native_delete");
        assert!(scenarios.insert((
            row["fd"].as_i64().unwrap(),
            values["mapping_active"],
            values["injected_result"],
        )));
        *modes.entry(row["mode"].as_i64().unwrap()).or_insert(0) += 1;
        assert!(matches!(
            check_contract_observation(contract, function, &values).unwrap(),
            ObservationVerdict::Checked { .. }
        ));
        for clause in ["requires ", "ensures normal "] {
            for (i, _) in contract.match_indices(clause) {
                let j = i + contract[i..].find(';').unwrap();
                for predicate in ["false", "unobserved_field"] {
                    let changed =
                        format!("{}{clause}{predicate}{}", &contract[..i], &contract[j..]);
                    let verdict = check_contract_observation(&changed, function, &values);
                    if clause == "requires " && predicate == "false" {
                        assert!(matches!(
                            verdict,
                            Ok(ObservationVerdict::OutsidePreconditions { .. })
                        ));
                    } else {
                        assert!(verdict.is_err());
                    }
                }
            }
        }
        for field in [
            "eligible",
            "unlink_calls",
            "diagnostic_calls",
            "arguments_match",
            "record_preserved",
            "path_preserved",
        ] {
            let mut changed = values.clone();
            changed.insert(field.into(), 1 - values[field]);
            assert!(check_contract_observation(contract, function, &changed).is_err());
        }
        let mut changed = values.clone();
        changed.insert("returned".into(), -1 - values["returned"]);
        assert!(check_contract_observation(contract, function, &changed).is_err());
    }
    let expected = [-1, 0, 7, i32::MAX as i64]
        .into_iter()
        .flat_map(|fd| (0..=1).flat_map(move |mapped| [-1, 0].map(|result| (fd, mapped, result))))
        .collect();
    assert_eq!(scenarios, expected);
    assert_eq!(observations.lines().count(), 16);
    assert_eq!(
        modes,
        BTreeMap::from([(0, 6), (1, 6), (2, 2), (3, 1), (4, 1)])
    );
}

#[test]
fn native_cleanup_observations_execute_every_policy_predicate() {
    let contract = include_str!(
        "../../../../providers/qcore-provider-chunkio/contracts/chunkio/filesystem_observation.qv"
    );
    let observations = include_str!("../fixtures/chunkio/native_cleanup.jsonl");
    let mut outcomes = std::collections::BTreeSet::new();
    for line in observations.lines() {
        let row: serde_json::Value = serde_json::from_str(line).unwrap();
        let values: BTreeMap<String, i128> = serde_json::from_value(row["values"].clone()).unwrap();
        let function = row["function"].as_str().unwrap();
        outcomes.insert((
            row["helper"].as_str().unwrap().to_owned(),
            row["mode"].as_i64().unwrap(),
        ));
        assert!(matches!(
            check_contract_observation(contract, function, &values).unwrap(),
            ObservationVerdict::Checked { .. }
        ));
        for (i, _) in contract.match_indices("ensures normal ") {
            let j = i + contract[i..].find(';').unwrap();
            for predicate in ["false", "unobserved_field"] {
                let changed = format!(
                    "{}ensures normal {predicate}{}",
                    &contract[..i],
                    &contract[j..]
                );
                assert!(check_contract_observation(&changed, function, &values).is_err());
            }
        }
        for field in ["arguments_match", "other_fields_preserved"] {
            let mut changed = values.clone();
            changed.insert(field.into(), 0);
            assert!(check_contract_observation(contract, function, &changed).is_err());
        }
        if values["injected_failure"] == 1 {
            let mut changed = values.clone();
            changed.insert("fields_preserved".into(), 0);
            assert!(check_contract_observation(contract, function, &changed).is_err());
        }
    }
    assert_eq!(outcomes.len(), 8);
    assert_eq!(observations.lines().count(), 18);
}

#[test]
fn review_r07_observation_signed_literal_boundaries() {
    let values = BTreeMap::from([("minimum".into(), i128::MIN)]);
    for text in [
        "0".to_owned(),
        "-0".to_owned(),
        i128::MAX.to_string(),
        i128::MIN.to_string(),
    ] {
        assert_eq!(
            evaluate(&text, &values).unwrap(),
            text.parse::<i128>().unwrap()
        );
    }
    for text in [
        "170141183460469231731687303715884105728",
        "-170141183460469231731687303715884105729",
        "-minimum",
        "minimum - 1",
        "minimum / -1",
        "minimum % -1",
        "--170141183460469231731687303715884105728",
    ] {
        assert!(evaluate(text, &values).is_err(), "accepted {text}");
    }
    assert_eq!(evaluate("minimum + 1", &values).unwrap(), i128::MIN + 1);
    assert_eq!(evaluate("minimum - minimum", &values).unwrap(), 0);
    assert_eq!(
        evaluate(
            "minimum == -170141183460469231731687303715884105728",
            &values
        )
        .unwrap(),
        1
    );
    assert_eq!(evaluate("-(minimum + 1)", &values).unwrap(), i128::MAX);
}

#[test]
fn review_r07_public_observer_checks_minimum_facts() {
    let source = format!(
        "extern fn observed() -> unit contract {{ ensures normal minimum == {}; ensures normal minimum + 1 == {}; }};",
        i128::MIN,
        i128::MIN + 1
    );
    let values = BTreeMap::from([("minimum".into(), i128::MIN)]);
    assert_eq!(
        check_contract_observation(&source, "observed", &values),
        Ok(ObservationVerdict::Checked { postconditions: 2 })
    );
    assert!(
        check_contract_observation(
            &source,
            "observed",
            &BTreeMap::from([("minimum".into(), i128::MIN + 1)])
        )
        .is_err()
    );
    assert!(check_contract_observation(&source, "observed", &BTreeMap::new()).is_err());
}
