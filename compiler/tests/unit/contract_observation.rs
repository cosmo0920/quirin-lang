use super::*;

#[test]
fn native_delete_observations_check_eligibility_arguments_and_frames() {
    let contract =
        include_str!("../../../contract/qcore/chunkio/filesystem_delete_observation.qcore");
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
    let contract = include_str!("../../../contract/qcore/chunkio/filesystem_observation.qcore");
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
