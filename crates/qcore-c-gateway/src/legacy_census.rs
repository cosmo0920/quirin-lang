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

//! Read-only census of the W4 recursive routes' reach over a corpus (card
//! **C03-REACH**).
//!
//! # What this module is, and what it is not
//!
//! Card C03-NFR implemented neutral-frame recursion and measured its reach on
//! the pinned Cycle 19 corpus as **zero**: every self-recursive function that
//! reached derivation refused on a blocker other than the recursion. That
//! measurement was an inference from published diagnostics. This module
//! replaces the inference with a direct reading: it re-runs the *production*
//! derivation and reports, per recursive strongly connected component, which
//! route disposed of it and — when the neutral route refused — which of the
//! design's V1–V10 obligations refused first, in that route's own words.
//!
//! Nothing here participates in a verdict. The census is derived from
//! [`crate::legacy_summary::derive_custody_summaries`], which is the same call
//! the analyzer makes; the census values are carried alongside the derivation
//! and read by no analysis, no certificate and no report. A census run cannot
//! promote, demote or otherwise move any result, because it publishes none.
//!
//! # Why the refusal is named at the route rather than reconstructed
//!
//! A function's published diagnostics are computed under the *ordinary*
//! environment, in which its own recursive call is an uncontracted call. The
//! neutral route examines the same body under an environment in which every
//! member of its component carries `N(m)`, so a diagnostic inventory taken from
//! the report is a different measurement from the one the route makes. Reading
//! the refusal off the route removes that gap.
use qcore_schemas::contract::ResourceContracts;

use crate::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyCError, LegacyDiagnostic, LegacyFunctionScope,
    SourceDefinitionInventory, enumerate_source_definitions,
};

/// One recursive strongly connected component of one translation unit's call
/// graph, with the route that disposed of it.
#[derive(Clone, Debug)]
pub struct NeutralComponentCensus {
    /// The component's members, in the derivation's own order.
    pub members: Vec<String>,
    /// Whether the component is a single function calling itself, as opposed to
    /// a mutually recursive cycle of two or more.
    pub self_recursive: bool,
    pub outcome: NeutralComponentOutcome,
}

/// How one recursive component was disposed of.
///
/// The first three variants record that the neutral route was never consulted,
/// which is obligations **V1** (declared precedence) and **V2** (ordinary
/// inference and both ranked recursive routes run first) doing their work — a
/// component another route already published is not a neutral-route refusal
/// and must not be counted as one.
#[derive(Clone, Debug)]
pub enum NeutralComponentOutcome {
    /// **V1.** Some member carries a declared `.qcore` contract, so the
    /// component stays on the declared route.
    DeclaredContract,
    /// **V2.** Ordinary (non-recursive) inference published every member, so
    /// the component never reached any recursive route.
    PublishedByOrdinaryInference,
    /// **V2.** The contextual or unsigned-countdown ranked recursive route
    /// published the component.
    PublishedByRankedRecursion { members: Vec<String> },
    /// **V2.** Ordinary inference published *some* members. The neutral route
    /// is consulted only for a component that published nothing, so it never
    /// runs here and the remainder stays uncontracted.
    PartiallyPublished { published: Vec<String> },
    /// The neutral route published these members.
    Admitted {
        members: Vec<String>,
        /// Members the descent demoted before reaching its fixed point.
        refused: Vec<NeutralMemberRefusal>,
    },
    /// The neutral route published nothing.
    Refused { members: Vec<NeutralMemberRefusal> },
}

/// The first obligation that refused one member, under the all-`Neutral` seed.
#[derive(Clone, Debug)]
pub struct NeutralMemberRefusal {
    pub function: String,
    pub refusal: NeutralRefusal,
}

/// Which of the route's checks refused a member, in the order they are made.
#[derive(Clone, Debug)]
pub enum NeutralRefusal {
    /// **V3.** The member's parameter list could not be read, so no `N(m)`
    /// candidate exists for it.
    ParametersUnreadable,
    /// **V5.** The body's resource CFG could not be built.
    CfgUnavailable(String),
    /// **V5.** The entry state prescribed by `N(m)` could not be constructed.
    EntryUnavailable,
    /// **V5.** The entry state is not the empty one — a parameter site, an out
    /// slot or a nullable result would make `N(m)` assert a precondition.
    NonEmptyEntry,
    /// **V5 (containment).** The CFG retains an `Unknown` instruction, which
    /// the certificate firewall refuses downstream. The payload is every such
    /// instruction's description, in CFG order: this check precedes the
    /// abstract interpretation, so these descriptions — not the published
    /// per-function diagnostics — are what actually blocks the member.
    RetainedUnknownInstruction {
        /// `(preprocessed line, description)` for each retained instruction.
        descriptions: Vec<(usize, String)>,
    },
    /// **V5.** The body did not analyse `Safe` with zero diagnostics. This is
    /// the blocker family the corpus census is about: the diagnostics are the
    /// ones the neutral environment produced, not the ones the report
    /// publishes.
    BodyNotClean {
        status: AnalysisStatus,
        diagnostics: Vec<LegacyDiagnostic>,
    },
    /// **V8.** No exit is reachable in the abstract semantics.
    NoReachableExit,
    /// **V6.** Some reachable exit disagrees with `N(m)`.
    ExitsDisagree,
    /// **V7.** Validation reads a contract other than `N(m)` off the body.
    ContractNotReadBack,
}

impl NeutralRefusal {
    /// A stable key for aggregation, naming the obligation.
    pub fn obligation(&self) -> &'static str {
        match self {
            Self::ParametersUnreadable => "V3-parameters-unreadable",
            Self::CfgUnavailable(_) => "V5-cfg-unavailable",
            Self::EntryUnavailable => "V5-entry-unavailable",
            Self::NonEmptyEntry => "V5-non-empty-entry",
            Self::RetainedUnknownInstruction { .. } => "V5-retained-unknown-instruction",
            Self::BodyNotClean { .. } => "V5-body-not-clean",
            Self::NoReachableExit => "V8-no-reachable-exit",
            Self::ExitsDisagree => "V6-exits-disagree",
            Self::ContractNotReadBack => "V7-contract-not-read-back",
        }
    }
}

/// One translation unit's census, with the preprocessed text the line numbers
/// in it refer to.
pub struct UnitCensus {
    pub preprocessed_source: String,
    pub components: Vec<NeutralComponentCensus>,
}

/// Run the production derivation over one preprocessed translation unit and
/// return its recursive-component census.
///
/// The unit is preprocessed and its analysed scope selected exactly as
/// [`crate::legacy_c::analyze_unrestricted_c11_with_inventory`] does, so the
/// census sees the same token stream, the same function inventory and the same
/// declared contracts the audit sees.
pub fn neutral_reach_census(
    unit: &C11TranslationUnit<'_>,
    contracts: &ResourceContracts,
    inventory: Option<&SourceDefinitionInventory>,
) -> Result<UnitCensus, LegacyCError> {
    let source_functions = match unit.function_scope {
        LegacyFunctionScope::PreprocessedTranslationUnit => None,
        LegacyFunctionScope::MainSourceDefinitions => Some(inventory.map_or_else(
            || enumerate_source_definitions(unit).definitions().to_vec(),
            |inventory| inventory.definitions().to_vec(),
        )),
    };
    let preprocessed_source = crate::legacy_c::validate_unrestricted_c11(unit)?;
    let components = crate::legacy_c::recursive_component_census(
        &preprocessed_source,
        contracts,
        source_functions.as_deref(),
    );
    Ok(UnitCensus {
        preprocessed_source,
        components,
    })
}
