//! Arm `sizeof-operand-exemption` (typed reader), the registry's route
//! `sizeof_operand::decide`. The gateway asks it from
//! `aggregate_store::store_bases` through `result_constructor_rule`.

use qcore_gateway_rule_api::{Answer, CheckedOperandArm, CheckedOperandQuery};

use crate::PACKAGE;

/// The arm's decision: the checked operand spans when they are well formed,
/// else a refusal of the whole answer.
#[must_use]
pub fn decide(query: &CheckedOperandQuery<'_>) -> Answer<Vec<(usize, usize)>> {
    CheckedOperandArm::ask(&PACKAGE, query)
}
