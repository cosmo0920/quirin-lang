//! The registry's routes for rule 33's two arms: `selection_route::expand`
//! (statement rewrite, `removed-selection-expansion`) and
//! `selection_route::row` (classification, `removed-selection-entry`). Each
//! is the arm's decision; the gateway asks them from `legacy_selection`.

use qcore_gateway_rule_api::{Answer, BodyQuery, EntryRow, Expanded, StatementExpansion};

use crate::{Frame, RemovedSelection, ValidationRun};

/// The expansion arm: a checked frame's own body, or its decimal-constant
/// caller statements, become the constituent calls; refuses when nothing
/// changes.
#[must_use]
pub fn expand(query: &BodyQuery<'_, Frame>) -> Answer<Expanded> {
    StatementExpansion::ask(&RemovedSelection, query)
}

/// The entry arm: admits a validated run only.
#[must_use]
pub fn row(run: &ValidationRun) -> Answer<()> {
    EntryRow::ask(&RemovedSelection, run)
}
