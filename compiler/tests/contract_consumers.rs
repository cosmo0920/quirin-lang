//! R08: syntax acceptance, resource assumptions and observation evaluation are
//! distinct consumers, with no implicit promotion between them.
use std::collections::BTreeMap;

use qcore_compiler::contract_loader::load_resource_contract_sources;
use qcore_compiler::contract_observation::{
    ObservationVerdict, check_contract_facts, check_contract_observation,
};
use qcore_compiler::frontend::parse_source_program;

#[test]
fn structural_acceptance_does_not_bypass_consumer_clause_or_observation_coverage() {
    let contract = "module contract; extern fn observed() -> unit contract { requires size > 0; ensures normal live == 1; };";
    assert!(parse_source_program(contract).is_ok());
    assert!(
        load_resource_contract_sources(&BTreeMap::from([(
            "observation.qc".into(),
            contract.into()
        )]))
        .is_err()
    );
    let values = BTreeMap::from([("size".into(), 1), ("live".into(), 1)]);
    assert_eq!(
        check_contract_observation(contract, "observed", &values),
        Ok(ObservationVerdict::Checked { postconditions: 1 })
    );
    for bad in [
        contract.replace("live == 1", "false"),
        contract.replace("live == 1", "unobserved == 1"),
        contract.replace("ensures normal", "ensures any"),
        contract.replace("ensures normal live == 1;", ""),
    ] {
        assert!(parse_source_program(&bad).is_ok());
        assert!(check_contract_observation(&bad, "observed", &values).is_err());
    }
    let outside = contract.replace("size > 0", "false");
    assert!(matches!(
        check_contract_observation(&outside, "observed", &values),
        Ok(ObservationVerdict::OutsidePreconditions { .. })
    ));
    assert!(check_contract_facts(&outside, "observed", &[("size", 1), ("live", 1)]).is_err());
    // A false precondition cannot conceal an unsupported/missing post-state.
    let unknown = outside.replace("live == 1", "unobserved == 1");
    assert!(check_contract_observation(&unknown, "observed", &values).is_err());
}
