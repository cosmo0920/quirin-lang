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

//! Declared Monkey syntax bounds for the shared typed C path importer.
//! Calls remain opaque; source projection does not establish custody or safety.
use crate::legacy_c::C11TranslationUnit;
use crate::typed_source::{
    ControlAdmission, import_source_functions_from_profile,
    import_source_functions_from_profile_admitting_control,
};
pub use crate::typed_source::{SourceExpression, SourceOperation, SourceRegion, SourceValue};
pub use crate::typed_source::{
    SourcePath as MonkeySourcePath, SourceResult as MonkeySourceResult,
    SourceSummary as MonkeySourceSummary,
};
use qcore_c_gateway::typed_source_nodes::{Classification, NodeAdmission};
use qcore_schemas::profile::ImportRole;

// Q5: source-contract text remains an interpreter input. Its declaration order
// preserves the historical wrapper report ordering; selection comes from data.
const WRAPPER_SOURCE_CONTRACT: &str =
    include_str!("../../../../providers/qcore-provider-libevent/contracts/monkey/wrapper_paths.qv");

/// Import default profile callbacks. Revision is a caller assertion; the pinned
/// external audit separately checks Git/source/build identity. Each successful
/// summary carries the exact declared profile assumptions, never verified facts.
pub fn import_monkey_callbacks(
    unit: &C11TranslationUnit<'_>,
    source_revision: &str,
) -> Result<Vec<MonkeySourceResult>, String> {
    let profile =
        crate::source_profiles::load_profile(crate::source_profiles::FLUENT_BIT_SOURCE_SET)
            .map_err(|error| error.to_string())?;
    import_source_functions_from_profile(
        unit,
        source_revision,
        &profile,
        "monkey",
        ImportRole::Callback,
    )
}

/// The Monkey wrapper projection's classification of route T's CG01 nodes
/// (card CG01-T-OPERATOR-CLOSURE, rule 10r). This projection certifies
/// nothing: `pinned_route` reports every wrapper as opaque calls and custody
/// Unknown (and, with a scan loop, bounds, arithmetic and termination
/// Unknown), and `monkey-source.json` marks it `source_projection_only`. So a
/// node with no fault case and no store is custody-neutral, and every other
/// node is an obligation the projection records and never discharges. The
/// wrapper summaries have no other reader; the Monkey callbacks and the
/// observed-wait wrapper import keep the legacy admission.
pub const MONKEY_WRAPPER_ADMISSION: NodeAdmission = NodeAdmission::new(
    "Monkey wrapper projection",
    &[
        ("binary -", Classification::ObligationBearing),
        ("binary <=", Classification::CustodyNeutral),
        ("binary >=", Classification::CustodyNeutral),
        ("binary ^", Classification::CustodyNeutral),
        ("binary <<", Classification::ObligationBearing),
        ("binary >>", Classification::ObligationBearing),
        ("unary +", Classification::CustodyNeutral),
        ("unary ~", Classification::CustodyNeutral),
        ("compound +=", Classification::ObligationBearing),
        ("compound -=", Classification::ObligationBearing),
        ("compound *=", Classification::ObligationBearing),
        ("compound /=", Classification::ObligationBearing),
        ("compound %=", Classification::ObligationBearing),
        ("compound <<=", Classification::ObligationBearing),
        ("compound >>=", Classification::ObligationBearing),
        ("compound &=", Classification::ObligationBearing),
        ("compound ^=", Classification::ObligationBearing),
        ("compound |=", Classification::ObligationBearing),
        ("literal floating", Classification::CustodyNeutral),
        ("literal character", Classification::CustodyNeutral),
    ],
);

/// The Monkey wrapper projection's control admission (card
/// CG09-TYPED-SOURCE-CONTROL, rule 11l): every acyclic form, lowered by route
/// T into guarded paths of the existing vocabulary. The projection certifies
/// nothing, as for [`MONKEY_WRAPPER_ADMISSION`]: a lowered path is reported
/// as opaque calls and custody Unknown. No admission can list a back-edge
/// form (a general loop or a backward `goto`), so those stay refused.
pub const MONKEY_WRAPPER_CONTROL: ControlAdmission = ControlAdmission::new(
    "Monkey wrapper projection",
    &["switch", "break", "continue", "goto"],
);

/// Preserve the bounded wrapper grammar and opaque-call behavior. No missing
/// profile or malformed selection can fall back to a Rust table.
pub fn import_monkey_wrappers(
    unit: &C11TranslationUnit<'_>,
    source_revision: &str,
) -> Result<Vec<MonkeySourceResult>, String> {
    let profile =
        crate::source_profiles::load_profile(crate::source_profiles::FLUENT_BIT_SOURCE_SET)
            .map_err(|error| error.to_string())?;
    let mut results = import_source_functions_from_profile_admitting_control(
        unit,
        source_revision,
        &profile,
        "monkey",
        ImportRole::Wrapper,
        &MONKEY_WRAPPER_ADMISSION,
        &MONKEY_WRAPPER_CONTROL,
    )?;
    let names = wrapper_source_order()?;
    let result_name = |result: &MonkeySourceResult| match result {
        MonkeySourceResult::Summary(summary) => summary.function().to_owned(),
        MonkeySourceResult::Unknown { function, .. } => function.clone(),
    };
    if results.len() != names.len()
        || results
            .iter()
            .any(|result| !names.contains(&result_name(result)))
    {
        return Err("Monkey wrapper profile does not match source-contract declarations".into());
    }
    results.sort_by_key(|result| names.iter().position(|name| *name == result_name(result)));
    Ok(results)
}

pub(crate) fn wrapper_source_order() -> Result<Vec<String>, String> {
    let parsed = crate::frontend::parse_source_program(WRAPPER_SOURCE_CONTRACT)
        .map_err(|error| format!("invalid wrapper source contract: {error:?}"))?;
    Ok(parsed
        .declarations
        .into_iter()
        .map(|decl| decl.value.name)
        .collect())
}

#[cfg(test)]
#[path = "../../tests/unit/monkey_source.rs"]
mod tests;
