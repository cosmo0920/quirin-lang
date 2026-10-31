//! Card **GATEWAY-RULE-API** (plan `docs/plans/pluggable-cards/README.md`,
//! seam B, with the rule switches of user decision 3): the interface a C
//! gateway rule package implements, and the composition the gateway applies to
//! packages.
//!
//! * [`Position`]: one per position of the frozen route map
//!   (`docs/reports/gateway-analysis-path-optimization/ROUTE-MAP.md`), and one
//!   trait per position ([`traits`]) whose query a package reads through the
//!   host's own vocabulary ([`Host`]), so a package never depends on
//!   `qcore-c-gateway`.
//! * [`Answer`]: the only answers are *admit with obligations* or *refuse*.
//! * [`consult`]: the route semigroup of plan seam G, unchanged from the
//!   gateway's tables. Within a position the first answer wins; an absent,
//!   disabled or refusing package falls through; the end of the table is the
//!   host's existing refusal. A package cannot edit another's answer.
//! * [`RuleEntry`] and [`check_registry`]: the model of the registry
//!   `crates/gateway-rules/registry.toml`, from which the gateway's tables are
//!   generated (`scripts/gen_gateway_registry.py`).
//! * [`ReasonKey`] and [`check_reservations`]: a package reserves the reason
//!   keys it emits, and no two packages share one.
//! * [`RulesOff`]: the run-time switch `QCORE_GATEWAY_RULES_OFF`, a comma list
//!   of registry ids. A listed package refuses without being asked, so off is
//!   fail-closed by construction; an id the registry does not hold, or one
//!   whose refusal would widen (a hold), is refused, never ignored.
//!
//! * [`transfer`] (card RULE34-PILOT): the transfer position, the order in
//!   which one admitted contract call's declared container effects apply.
//! * [`traits::TokenHold`] (card RULE32A-PILOT, first used by rule 32a's
//!   caller-frame hold): a hold of the token stream. A hold is registered
//!   `not-switchable` and its host asks it under every switch list.
//!
//! No analysis logic lives here and nothing here decides a verdict.

mod answer;
pub mod checked_readings;
mod position;
mod reason;
mod registry;
pub mod statement_expansion;
mod switches;
pub mod traits;
pub mod transfer;

pub use answer::{Admission, Answer, Consulted, Obligation, consult};
pub use checked_readings::{
    CheckedOperandArm, CheckedOperandQuery, SpecificationPremiseArm, SpecificationQuery,
};
pub use position::{Position, UnknownPosition};
/// Card RULE32A-PILOT: a package that reserves a reason key names its closed
/// §L21.1 category, so the category is part of the interface.
pub use qcore_schemas::reason::ReasonCategory;
pub use reason::{ReasonKey, check_reservations};
pub use registry::{Off, Placement, RegistryError, RuleEntry, check_registry, ordered_table};
pub use statement_expansion::{BodyQuery, EntryRow, Expanded, StatementExpansion};
pub use switches::{RULES_OFF_VARIABLE, RulesOff, SwitchError};
pub use traits::{Host, LexicalHost, TypedHost, TypedReading, Witness};
pub use transfer::{DeclaredEffect, Phase, TransferArm, TransferQuery};

#[cfg(test)]
mod tests {
    /// Plan seam B: the API crate depends on `qcore-schemas` and
    /// `qcore-contract` only, so a rule package that depends on it reaches no
    /// analysis crate through it.
    #[test]
    fn the_crate_depends_on_the_schema_surface_only() {
        let manifest = include_str!("../Cargo.toml");
        let mut section = "";
        let mut dependencies = Vec::new();
        for line in manifest.lines() {
            let line = line.trim();
            if line.starts_with('[') {
                section = line;
                continue;
            }
            if section.contains("dependencies") && !line.is_empty() && !line.starts_with('#') {
                dependencies.push(line.split('=').next().unwrap_or("").trim().to_owned());
            }
        }
        assert!(!dependencies.is_empty());
        for dependency in &dependencies {
            assert!(
                ["qcore-schemas", "qcore-contract"].contains(&dependency.as_str()),
                "qcore-gateway-rule-api may depend on qcore-schemas and qcore-contract only, \
                 not {dependency}"
            );
        }
    }
}
