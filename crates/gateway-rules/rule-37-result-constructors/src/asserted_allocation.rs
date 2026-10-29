//! Arm `asserted-allocation-premise` (classification), the registry's route
//! `asserted_allocation::decide`. The gateway asks it from
//! `legacy_checked_spec::settle` through `result_constructor_rule`.

use qcore_gateway_rule_api::{Answer, SpecificationPremiseArm, SpecificationQuery};

use crate::PACKAGE;

/// The arm's decision: admit a specification naming `asserted_allocation`
/// with that premise as its obligation, else refuse.
#[must_use]
pub fn decide(query: &SpecificationQuery<'_>) -> Answer<()> {
    SpecificationPremiseArm::ask(&PACKAGE, query)
}
