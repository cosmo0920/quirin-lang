use super::*;

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
