use super::*;
use qcore_c_gateway::typed_source_nodes::{NODE_TABLE, facts};

/// CG01 (rule 10r): the wrapper projection classifies every CG01 node, as
/// strongly as each node's facts allow and no more: exactly the nodes with no
/// fault case and no store are custody-neutral; the rest are obligations the
/// projection records and never discharges.
#[test]
fn monkey_wrapper_admission_classifies_every_cg01_node() {
    MONKEY_WRAPPER_ADMISSION.validate().unwrap();
    let rows = MONKEY_WRAPPER_ADMISSION.rows();
    assert_eq!(rows.len(), NODE_TABLE.len());
    for (spelling, class) in rows {
        let facts = facts(spelling).unwrap();
        let neutral = facts.faults.is_empty() && !facts.store;
        assert_eq!(
            *class,
            if neutral {
                Classification::CustodyNeutral
            } else {
                Classification::ObligationBearing
            },
            "{spelling}"
        );
    }
    // The callbacks keep the legacy admission: nothing is admitted.
    assert!(NodeAdmission::LEGACY.admit("binary ^").is_err());
}
