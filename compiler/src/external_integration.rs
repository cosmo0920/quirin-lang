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

//! Compatibility imports and library-specific Monkey composition.
pub use crate::verifier_plugins::chunkio::PINNED_FLUENT_BIT_REVISION;
use crate::verifier_plugins::libevent::{
    EventBackend, EventCertificate, EventDiagnostic, EventOperation, EventProgram,
    LibeventBuildProfile, ObservedKind, check_event_program, check_observation_refinement,
};
pub use qcore_core::library_custody::*;
use qcore_schemas::profile::BuildProfileSelector;
pub use qcore_verification::library_import::*;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum MonkeyCallback {
    Event,
    Timeout,
    WaitTwoTimeout,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MonkeyCallbackSummary {
    pub argument_borrowed: bool,
    pub writes_fired_array: bool,
    pub releases_event_map: bool,
    pub closes_descriptor: bool,
    pub no_effect: bool,
}

impl MonkeyCallback {
    fn function_name(self) -> &'static str {
        match self {
            Self::Event => "cb_event",
            Self::Timeout => "cb_timeout",
            Self::WaitTwoTimeout => "cb_wait_2_timeout",
        }
    }
}

impl MonkeyCallbackSummary {
    /// Read the default declared callback effect. Unavailable profiles are an
    /// error, never fabricated no-effect summaries or built-in fallback tables.
    pub fn audited(callback: MonkeyCallback) -> Result<Self, String> {
        let profile =
            crate::source_profiles::load_profile(crate::source_profiles::FLUENT_BIT_SOURCE_SET)
                .map_err(|error| error.to_string())?;
        Self::audited_from_profile(&profile, callback)
    }

    pub fn audited_from_profile(
        profile: &qcore_contract::profile_loader::LoadedProfile,
        callback: MonkeyCallback,
    ) -> Result<Self, String> {
        let declared = profile
            .library("monkey")
            .and_then(|library| library.callback_summaries.get(callback.function_name()))
            .ok_or_else(|| {
                format!(
                    "profile lacks callback summary `{}`",
                    callback.function_name()
                )
            })?;
        Ok(Self {
            argument_borrowed: declared.argument_borrowed,
            writes_fired_array: declared.writes_fired_array,
            releases_event_map: declared.releases_event_map,
            closes_descriptor: declared.closes_descriptor,
            no_effect: declared.no_effect,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DispatcherObservation {
    pub dispatcher: ExternalContextId,
    pub backend: EventBackend,
    pub trace: Vec<ObservedKind>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MonkeyCompositionProfile {
    pub source_revision: String,
    pub build: LibeventBuildProfile,
    pub callbacks: BTreeMap<MonkeyCallback, MonkeyCallbackSummary>,
    pub event_map_custody: bool,
    pub nullable_base_creation: bool,
    pub nullable_event_creation: bool,
    pub failed_registration: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MonkeyCompositionProgram {
    pub profile: MonkeyCompositionProfile,
    pub events: EventProgram,
    pub observation: DispatcherObservation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MonkeyCompositionCertificate {
    pub profile_assumptions: BTreeMap<String, String>,
    pub source_revision: String,
    pub event: EventCertificate,
    pub callbacks: BTreeMap<MonkeyCallback, MonkeyCallbackSummary>,
    pub dispatcher: ExternalContextId,
    pub backend: EventBackend,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MonkeyCompositionDiagnostic {
    ProfileUnavailable(String),
    SourceRevisionMismatch,
    UnknownBuildProfile,
    MissingCallback(MonkeyCallback),
    InvalidCallbackSummary(MonkeyCallback),
    MissingEventMapCustody,
    MissingNullableBasePath,
    MissingNullableEventPath,
    MissingRegistrationFailurePath,
    EventBuildMismatch,
    BackendMismatch,
    DispatcherMismatch,
    Event(Vec<EventDiagnostic>),
    Observation(EventDiagnostic),
}

/// Compose under the default declared source profile. Q5 source-contract text
/// remains an interpreter input; callback effect expectations come from data.
pub fn check_monkey_composition(
    program: &MonkeyCompositionProgram,
    accepted_builds: &[BuildProfileSelector],
) -> Result<MonkeyCompositionCertificate, Vec<MonkeyCompositionDiagnostic>> {
    let profile =
        crate::source_profiles::load_profile(crate::source_profiles::FLUENT_BIT_SOURCE_SET)
            .map_err(|error| {
                vec![MonkeyCompositionDiagnostic::ProfileUnavailable(
                    error.to_string(),
                )]
            })?;
    check_monkey_composition_with_profile(program, accepted_builds, &profile)
}

/// The explicit profile is a declared assumption, retained in the certificate.
pub fn check_monkey_composition_with_profile(
    program: &MonkeyCompositionProgram,
    accepted_builds: &[BuildProfileSelector],
    source_profile: &qcore_contract::profile_loader::LoadedProfile,
) -> Result<MonkeyCompositionCertificate, Vec<MonkeyCompositionDiagnostic>> {
    let pin = source_profile.manifest().pin_for("monkey").ok_or_else(|| {
        vec![MonkeyCompositionDiagnostic::ProfileUnavailable(
            "missing Monkey pin".into(),
        )]
    })?;
    let mut diagnostics = Vec::new();
    if program.profile.source_revision != pin.git_commit {
        diagnostics.push(MonkeyCompositionDiagnostic::SourceRevisionMismatch);
    }
    if !program.profile.build.accepted(accepted_builds) {
        diagnostics.push(MonkeyCompositionDiagnostic::UnknownBuildProfile);
    }
    for callback in [
        MonkeyCallback::Event,
        MonkeyCallback::Timeout,
        MonkeyCallback::WaitTwoTimeout,
    ] {
        let expected = MonkeyCallbackSummary::audited_from_profile(source_profile, callback)
            .map_err(|error| vec![MonkeyCompositionDiagnostic::ProfileUnavailable(error)])?;
        match program.profile.callbacks.get(&callback) {
            None => diagnostics.push(MonkeyCompositionDiagnostic::MissingCallback(callback)),
            Some(summary) if *summary != expected => {
                diagnostics.push(MonkeyCompositionDiagnostic::InvalidCallbackSummary(
                    callback,
                ));
            }
            Some(_) => {}
        }
    }
    if !program.profile.event_map_custody {
        diagnostics.push(MonkeyCompositionDiagnostic::MissingEventMapCustody);
    }
    if !program.profile.nullable_base_creation {
        diagnostics.push(MonkeyCompositionDiagnostic::MissingNullableBasePath);
    }
    if !program.profile.nullable_event_creation {
        diagnostics.push(MonkeyCompositionDiagnostic::MissingNullableEventPath);
    }
    if !program.profile.failed_registration {
        diagnostics.push(MonkeyCompositionDiagnostic::MissingRegistrationFailurePath);
    }
    let created_bases = program
        .events
        .operations
        .iter()
        .filter_map(|operation| match operation {
            EventOperation::CreateBase { base, profile } => Some((*base, *profile)),
            _ => None,
        })
        .collect::<BTreeMap<_, _>>();
    let dispatched_bases = program
        .events
        .operations
        .iter()
        .filter_map(|operation| match operation {
            EventOperation::BeginDispatch { base, .. } => Some(*base),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    if dispatched_bases.is_empty()
        || dispatched_bases
            .iter()
            .any(|base| created_bases.get(base) != Some(&program.profile.build))
    {
        diagnostics.push(MonkeyCompositionDiagnostic::EventBuildMismatch);
    }
    if program.profile.build.backend != program.observation.backend {
        diagnostics.push(MonkeyCompositionDiagnostic::BackendMismatch);
    }
    let dispatchers = program
        .events
        .operations
        .iter()
        .filter_map(|operation| match operation {
            EventOperation::BeginDispatch { dispatcher, .. } => Some(*dispatcher),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    if dispatchers != BTreeSet::from([program.observation.dispatcher]) {
        diagnostics.push(MonkeyCompositionDiagnostic::DispatcherMismatch);
    }
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }
    let event = check_event_program(&program.events, accepted_builds)
        .map_err(|errors| vec![MonkeyCompositionDiagnostic::Event(errors)])?;
    check_observation_refinement(&event, &program.observation.trace)
        .map_err(|error| vec![MonkeyCompositionDiagnostic::Observation(error)])?;
    let mut profile_assumptions = source_profile.assumptions();
    profile_assumptions.insert("library".into(), "monkey".into());
    Ok(MonkeyCompositionCertificate {
        profile_assumptions,
        source_revision: program.profile.source_revision.clone(),
        event,
        callbacks: program.profile.callbacks.clone(),
        dispatcher: program.observation.dispatcher,
        backend: program.observation.backend,
    })
}

#[cfg(test)]
#[path = "../tests/unit/external_integration.rs"]
mod tests;
