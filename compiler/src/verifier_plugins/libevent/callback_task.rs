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

//! Adapter from the Libevent facade program to the process-portable reactor
//! callback capability.
//!
//! This is transport only. The in-process route
//! ([`super::event_driven::check_event_program`]) is unchanged and retained;
//! this module converts the same facade program and the same loaded
//! accepted-build matrix into the trusted core's sealed import, so both routes
//! decide the identical question with the identical model checker.
//!
//! Nothing here derives a callback summary, a build profile or an accepted
//! build from C source. Those remain the declared model premises the Cycle 18
//! route already carried, and they travel in the task's assumptions.

use std::collections::BTreeMap;

use qcore_schemas::profile::BuildProfileSelector;
use qcore_verification::reactor_source::{
    ReactorCallbackTask, ReactorImportError, ReactorProgramIdentity, ReactorProgramImport,
    import_reactor_program,
};

use super::event_driven::{EventProgram, LibeventBuildProfile};

/// The declared build attributes that identify a Libevent build profile.
/// They are opaque metadata to a provider and are bound only by digest, so a
/// different backend or threading mode retires the recorded task.
#[must_use]
pub fn declared_build_profile(profile: LibeventBuildProfile) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("backend".into(), profile.backend.as_str().to_owned()),
        (
            "threads".into(),
            profile.threads.as_str().unwrap_or("disabled").to_owned(),
        ),
        ("openssl".into(), profile.openssl.to_string()),
        ("debug_mode".into(), profile.debug_mode.to_string()),
    ])
}

/// Seal one facade program for the process-portable route.
///
/// The accepted-build decision is taken exactly as the in-process route takes
/// it: a `CreateBase` profile is accepted only when some selector in the loaded
/// matrix matches. An empty or unmatched matrix yields no accepted profile, so
/// the import refuses and the program's callbacks stay `Unknown`.
pub fn import_libevent_program(
    program: &EventProgram,
    accepted_builds: &[BuildProfileSelector],
    library: &str,
    source_revision: &str,
    profile_assumptions: BTreeMap<String, String>,
) -> Result<ReactorProgramImport<LibeventBuildProfile>, ReactorImportError> {
    let accepted_profiles = program
        .operations
        .iter()
        .filter_map(|operation| match operation {
            super::event_driven::EventOperation::CreateBase { profile, .. }
                if profile.accepted(accepted_builds) =>
            {
                Some(*profile)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    let build_profile = accepted_profiles
        .first()
        .copied()
        .map(declared_build_profile)
        .unwrap_or_default();
    let model: qcore_core::event_reactor::EventProgram<LibeventBuildProfile> =
        program.clone().into();
    import_reactor_program(
        &model,
        &accepted_profiles,
        ReactorProgramIdentity {
            library: library.into(),
            source_revision: source_revision.into(),
            build_profile,
            profile_assumptions,
        },
    )
}

/// Derive the evidence-v2 task for one sealed Libevent program.
pub fn libevent_callback_task(
    import: ReactorProgramImport<LibeventBuildProfile>,
    translation_unit: &str,
    target: qcore_schemas::plugin::TargetProfile,
) -> Result<ReactorCallbackTask<LibeventBuildProfile>, String> {
    ReactorCallbackTask::derive(import, translation_unit, target)
}
