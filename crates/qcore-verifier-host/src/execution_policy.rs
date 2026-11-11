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

//! The configured route by which a caller selects a process execution profile
//! (card `P03-F`, closing P03's "default profile wiring" open item).
//!
//! Before this module, [`ProcessExecutionProfile::LinuxRlimitV1`] and
//! [`ProcessExecutionProfile::LinuxConfinedV1`] were reachable only from Rust:
//! a caller had to build a [`ConfinementPolicy`] value and chain
//! `with_execution_profile`. That left every configured route — the CLI's
//! `--config`/`--plugins` selection, and any audit that loads packages from
//! data — permanently on the historical profile, with no way to say otherwise.
//!
//! **The default does not move.** An absent `execution` block is exactly
//! [`ProcessExecutionProfile::UnenforcedLegacy`], byte-for-byte the historical
//! behavior, and this module's `Default` is that profile with no policy. What
//! it adds is the *ability to write down* a stronger selection, and the
//! fail-closed checks that keep a written selection from being weaker than it
//! reads:
//!
//! * a policy under a profile that enforces none is refused here, before a
//!   provider is even built, as well as at [`ProcessProvider::enforced_limits`];
//! * a confining profile with no policy and no named starting point is refused
//!   here for the same reason it is refused there — a policy nobody wrote is a
//!   policy nobody reviewed;
//! * a relative admitted path is refused, because what it would admit depends
//!   on the reader's working directory rather than on the configuration.
//!
//! Nothing here weakens a refusal: every check this module makes is also made
//! by the provider, and this module only makes them *earlier*, where the
//! configuration author can see them.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::confinement::ConfinementPolicy;
use crate::{ProcessExecutionProfile, ProcessProvider};

/// The configuration schema version this module accepts. A future shape change
/// bumps it and rejects the old one rather than guessing.
pub const EXECUTION_POLICY_VERSION: u16 = 1;

/// Which named filesystem starting point a confining selection begins from.
///
/// There is deliberately no "whatever the host needs" answer: the runtime
/// defaults are a *named* policy a configuration opts into, exactly as
/// [`ConfinementPolicy::linux_runtime_defaults`] is a named value a Rust caller
/// opts into.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ConfinementBase {
    /// Admit nothing but the package root and the paths named below.
    #[default]
    None,
    /// Begin from [`ConfinementPolicy::linux_runtime_defaults`] — the
    /// interpreter, dynamic loader and shared libraries a dynamically linked
    /// package needs — and add the paths named below.
    LinuxRuntimeDefaults,
}

/// A written execution-profile selection.
///
/// Serialized shape, with every field optional so an absent block and an empty
/// object both mean the historical default:
///
/// ```json
/// {
///   "version": 1,
///   "profile": "linux-confined-v1",
///   "confinement": {
///     "base": "linux-runtime-defaults",
///     "read_only": ["/etc/qcore/policy.d"],
///     "read_write": ["/var/tmp/qcore-scratch"]
///   }
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionPolicy {
    #[serde(default = "default_version")]
    pub version: u16,
    /// The profile every provider this configuration builds runs under.
    #[serde(default)]
    pub profile: ProcessExecutionProfile,
    /// The filesystem reach a confining profile grants. Required by a
    /// confining profile and refused under every other one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confinement: Option<ConfinementSelection>,
}

const fn default_version() -> u16 {
    EXECUTION_POLICY_VERSION
}

impl Default for ExecutionPolicy {
    /// The recorded default: the historical, explicitly labeled unenforcing
    /// profile with no confinement policy.
    fn default() -> Self {
        Self {
            version: EXECUTION_POLICY_VERSION,
            profile: ProcessExecutionProfile::UnenforcedLegacy,
            confinement: None,
        }
    }
}

/// The written form of a [`ConfinementPolicy`].
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfinementSelection {
    #[serde(default)]
    pub base: ConfinementBase,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub read_execute: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub read_only: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub read_write: Vec<String>,
}

impl ConfinementSelection {
    fn policy(&self) -> ConfinementPolicy {
        let mut policy = match self.base {
            ConfinementBase::None => ConfinementPolicy::default(),
            ConfinementBase::LinuxRuntimeDefaults => ConfinementPolicy::linux_runtime_defaults(),
        };
        policy
            .read_execute
            .extend(self.read_execute.iter().cloned());
        policy.read_only.extend(self.read_only.iter().cloned());
        policy.read_write.extend(self.read_write.iter().cloned());
        policy
    }

    fn paths(&self) -> impl Iterator<Item = &String> {
        self.read_execute
            .iter()
            .chain(&self.read_only)
            .chain(&self.read_write)
    }
}

/// Why a written selection cannot be applied. Every variant is a refusal, and
/// none of them has a weaker fallback: a configuration this module rejects
/// builds no provider at all.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExecutionPolicyError {
    UnsupportedVersion(u16),
    /// A confining profile with neither a named base nor an admitted path.
    NoPolicy(ProcessExecutionProfile),
    /// A policy under a profile that installs no confinement.
    PolicyWithoutConfinement(ProcessExecutionProfile),
    /// An admitted path whose meaning depends on the reader's directory.
    RelativePath(String),
}

impl fmt::Display for ExecutionPolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedVersion(version) => write!(
                formatter,
                "unsupported execution policy version {version}; this host accepts \
                 {EXECUTION_POLICY_VERSION}"
            ),
            Self::NoPolicy(profile) => write!(
                formatter,
                "execution profile `{profile}` requires an explicit confinement policy; give it a \
                 `confinement` block, because it admits no filesystem path beyond the package root \
                 and has no default"
            ),
            Self::PolicyWithoutConfinement(profile) => write!(
                formatter,
                "execution profile `{profile}` enforces no confinement policy, so the configured \
                 `confinement` block would be a declaration nothing installs; select `{}` instead",
                ProcessExecutionProfile::LinuxConfinedV1
            ),
            Self::RelativePath(path) => write!(
                formatter,
                "admitted path `{path}` is relative, so what it admits would depend on the \
                 reader's working directory rather than on this configuration"
            ),
        }
    }
}

impl std::error::Error for ExecutionPolicyError {}

impl ExecutionPolicy {
    /// The historical default, named rather than implied.
    #[must_use]
    pub fn unenforced_legacy() -> Self {
        Self::default()
    }

    /// Checks the written selection without building anything.
    ///
    /// Called by [`Self::apply`] and separately available so a configuration
    /// loader can refuse a bad selection before it opens a package.
    pub fn validate(&self) -> Result<(), ExecutionPolicyError> {
        if self.version != EXECUTION_POLICY_VERSION {
            return Err(ExecutionPolicyError::UnsupportedVersion(self.version));
        }
        match (&self.confinement, self.profile.confines()) {
            (None, true) => return Err(ExecutionPolicyError::NoPolicy(self.profile)),
            (Some(_), false) => {
                return Err(ExecutionPolicyError::PolicyWithoutConfinement(self.profile));
            }
            _ => {}
        }
        if let Some(selection) = &self.confinement {
            if selection.base == ConfinementBase::None && selection.paths().next().is_none() {
                return Err(ExecutionPolicyError::NoPolicy(self.profile));
            }
            for path in selection.paths() {
                if !path.starts_with('/') {
                    return Err(ExecutionPolicyError::RelativePath(path.clone()));
                }
            }
        }
        Ok(())
    }

    /// Applies the written selection to a provider.
    ///
    /// Additive: a default policy returns the provider with the profile it
    /// already had, which is the historical one.
    pub fn apply(
        &self,
        provider: ProcessProvider,
    ) -> Result<ProcessProvider, ExecutionPolicyError> {
        self.validate()?;
        let provider = provider.with_execution_profile(self.profile);
        Ok(match &self.confinement {
            Some(selection) => provider.with_confinement_policy(selection.policy()),
            None => provider,
        })
    }

    /// The same selection over a package that has not been verified.
    ///
    /// Separate on purpose (card `P01-F3`): applying a policy must not be a way
    /// to turn an [`crate::UnverifiedProvider`] into a registrable one, so this
    /// returns the unverified type it was given.
    pub fn apply_unverified(
        &self,
        provider: crate::UnverifiedProvider,
    ) -> Result<crate::UnverifiedProvider, ExecutionPolicyError> {
        self.validate()?;
        let provider = provider.with_execution_profile(self.profile);
        Ok(match &self.confinement {
            Some(selection) => provider.with_confinement_policy(selection.policy()),
            None => provider,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_recorded_default_is_the_historical_unenforcing_profile() {
        let policy = ExecutionPolicy::default();
        assert_eq!(policy.profile, ProcessExecutionProfile::UnenforcedLegacy);
        assert!(policy.confinement.is_none());
        assert!(policy.validate().is_ok());
        // An absent block and an empty object are the same selection, so a
        // configuration that says nothing selects the default explicitly.
        let empty: ExecutionPolicy = serde_json::from_str("{}").unwrap();
        assert_eq!(empty, policy);
    }

    #[test]
    fn a_confining_selection_without_a_policy_is_refused() {
        let policy = ExecutionPolicy {
            profile: ProcessExecutionProfile::LinuxConfinedV1,
            ..ExecutionPolicy::default()
        };
        assert_eq!(
            policy.validate(),
            Err(ExecutionPolicyError::NoPolicy(
                ProcessExecutionProfile::LinuxConfinedV1
            ))
        );
        // An empty block is also no policy: it would admit only the package
        // root, which cannot start a dynamically linked package, and it names
        // nothing a reviewer could have reviewed.
        let empty = ExecutionPolicy {
            profile: ProcessExecutionProfile::LinuxConfinedV1,
            confinement: Some(ConfinementSelection::default()),
            ..ExecutionPolicy::default()
        };
        assert_eq!(
            empty.validate(),
            Err(ExecutionPolicyError::NoPolicy(
                ProcessExecutionProfile::LinuxConfinedV1
            ))
        );
    }

    #[test]
    fn a_policy_under_a_non_confining_profile_is_refused_in_the_other_direction() {
        for profile in [
            ProcessExecutionProfile::UnenforcedLegacy,
            ProcessExecutionProfile::LinuxRlimitV1,
        ] {
            let policy = ExecutionPolicy {
                profile,
                confinement: Some(ConfinementSelection {
                    base: ConfinementBase::LinuxRuntimeDefaults,
                    ..ConfinementSelection::default()
                }),
                ..ExecutionPolicy::default()
            };
            assert_eq!(
                policy.validate(),
                Err(ExecutionPolicyError::PolicyWithoutConfinement(profile))
            );
        }
    }

    #[test]
    fn a_relative_admitted_path_is_refused() {
        let policy = ExecutionPolicy {
            profile: ProcessExecutionProfile::LinuxConfinedV1,
            confinement: Some(ConfinementSelection {
                base: ConfinementBase::LinuxRuntimeDefaults,
                read_write: vec!["scratch".into()],
                ..ConfinementSelection::default()
            }),
            ..ExecutionPolicy::default()
        };
        assert_eq!(
            policy.validate(),
            Err(ExecutionPolicyError::RelativePath("scratch".into()))
        );
    }

    #[test]
    fn an_unsupported_version_is_refused_rather_than_guessed() {
        let policy = ExecutionPolicy {
            version: 2,
            ..ExecutionPolicy::default()
        };
        assert_eq!(
            policy.validate(),
            Err(ExecutionPolicyError::UnsupportedVersion(2))
        );
    }

    #[test]
    fn a_named_base_expands_to_the_same_policy_a_rust_caller_would_build() {
        let selection = ConfinementSelection {
            base: ConfinementBase::LinuxRuntimeDefaults,
            read_write: vec!["/var/tmp/scratch".into()],
            ..ConfinementSelection::default()
        };
        let expected = ConfinementPolicy::linux_runtime_defaults()
            .admitting_read_write("/var/tmp/scratch".to_owned());
        assert_eq!(selection.policy(), expected);
    }

    #[test]
    fn the_written_form_round_trips() {
        let policy = ExecutionPolicy {
            version: 1,
            profile: ProcessExecutionProfile::LinuxConfinedV1,
            confinement: Some(ConfinementSelection {
                base: ConfinementBase::LinuxRuntimeDefaults,
                read_only: vec!["/etc/qcore".into()],
                ..ConfinementSelection::default()
            }),
        };
        let text = serde_json::to_string(&policy).unwrap();
        assert!(text.contains("linux-confined-v1"));
        assert!(text.contains("linux-runtime-defaults"));
        assert_eq!(
            serde_json::from_str::<ExecutionPolicy>(&text).unwrap(),
            policy
        );
        // The default serializes without a confinement key at all, so a
        // configuration that keeps the default keeps its historical bytes.
        let default = serde_json::to_string(&ExecutionPolicy::default()).unwrap();
        assert!(!default.contains("confinement"));
        assert!(default.contains("unenforced-legacy"));
    }
}
