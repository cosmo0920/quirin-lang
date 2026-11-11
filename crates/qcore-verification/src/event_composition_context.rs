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

//! Caller-held context binding for observed Cycle 18 composition evidence.
//!
//! This module authenticates equality with a caller's recorded intent. It does
//! not attest a compiler, observer, harness, library, or host; it does not check
//! an event program; and a validated context is never a `Safe` or `Composed`
//! verdict. The later composition boundary must join this token with separately
//! sealed source projection and event-program evidence.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

pub const EVENT_COMPOSITION_CONTEXT_VERSION: u32 = 1;
pub const EVENT_COMPOSITION_EVIDENCE_VERSION: &str = "qcore.cycle18.observed-composition.v1";
pub const EVENT_COMPOSITION_PROPERTY: &str = "cycle18-observed-callback-composition";

/// The observation authority this bounded route permits.
///
/// Equality of this value records the premise; it does not turn producer data
/// into runtime attestation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeclaredObservationTrust {
    ProducerSuppliedRuntimeEvidence,
}

/// Caller inputs other than the two evidence payloads whose digests are
/// derived by [`EventCompositionContext::derive`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventCompositionContextInput {
    pub artifact: String,
    pub semantic_profile: String,
    pub scope: BTreeSet<String>,
    pub bounds: BTreeMap<String, u64>,
    pub assumptions: BTreeMap<String, String>,
    pub source_compile_task_sha256: String,
    pub fluent_bit_revision: String,
    pub monkey_source_sha256: String,
    pub monkey_profile_sha256: String,
    pub monkey_build_sha256: String,
    pub libevent_revision: String,
    pub libevent_source_sha256: String,
    pub libevent_profile_sha256: String,
    pub libevent_build_sha256: String,
    pub contracts_sha256: String,
    pub source_rules_sha256: String,
    pub observation_rules_sha256: String,
    pub observer_binary_sha256: String,
    pub observer_harness_sha256: String,
    pub linked_libraries_sha256: BTreeMap<String, String>,
    pub observation_trust: DeclaredObservationTrust,
}

/// The exact byte payloads represented by an observed-run context.
#[derive(Clone, Copy, Debug)]
pub struct EventCompositionPayload<'a> {
    pub observations: &'a [u8],
    pub bindings: &'a [u8],
}

/// Complete identity of one observed Cycle 18 composition claim.
///
/// Construction is checked and payload hashes are not caller-supplied.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct EventCompositionContext {
    schema_version: u32,
    artifact: String,
    property: String,
    semantic_profile: String,
    scope: BTreeSet<String>,
    bounds: BTreeMap<String, u64>,
    assumptions: BTreeMap<String, String>,
    evidence_version: String,
    source_compile_task_sha256: String,
    fluent_bit_revision: String,
    monkey_source_sha256: String,
    monkey_profile_sha256: String,
    monkey_build_sha256: String,
    libevent_revision: String,
    libevent_source_sha256: String,
    libevent_profile_sha256: String,
    libevent_build_sha256: String,
    contracts_sha256: String,
    source_rules_sha256: String,
    observation_rules_sha256: String,
    observer_binary_sha256: String,
    observer_harness_sha256: String,
    linked_libraries_sha256: BTreeMap<String, String>,
    observations_sha256: String,
    bindings_sha256: String,
    observation_trust: DeclaredObservationTrust,
}

/// A successful equality and payload-rederivation check.
///
/// Private fields prevent callers from manufacturing the token. It deliberately
/// has no serialization implementation and carries no safety verdict.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidatedEventCompositionContext {
    context: EventCompositionContext,
}

impl ValidatedEventCompositionContext {
    #[must_use]
    pub fn context(&self) -> &EventCompositionContext {
        &self.context
    }

    /// Canonical report identity for the complete validated context.
    ///
    /// This digest identifies the recorded context; it is not runtime
    /// attestation and does not include a safety verdict.
    #[must_use]
    pub fn context_sha256(&self) -> String {
        self.context.context_sha256()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContextValidationError {
    InvalidField(&'static str),
    PayloadDigestMismatch(&'static str),
    ContextMismatch(&'static str),
}

impl std::fmt::Display for ContextValidationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ContextValidationError {}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty() && value.trim() == value
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn is_git_revision(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

impl EventCompositionContext {
    /// Derive a complete context from caller inputs and the exact evidence
    /// payloads. Empty evidence and incomplete identities refuse.
    pub fn derive(
        input: EventCompositionContextInput,
        payload: EventCompositionPayload<'_>,
    ) -> Result<Self, ContextValidationError> {
        if payload.observations.is_empty() {
            return Err(ContextValidationError::InvalidField("observations"));
        }
        if payload.bindings.is_empty() {
            return Err(ContextValidationError::InvalidField("bindings"));
        }
        let context = Self {
            schema_version: EVENT_COMPOSITION_CONTEXT_VERSION,
            artifact: input.artifact,
            property: EVENT_COMPOSITION_PROPERTY.to_owned(),
            semantic_profile: input.semantic_profile,
            scope: input.scope,
            bounds: input.bounds,
            assumptions: input.assumptions,
            evidence_version: EVENT_COMPOSITION_EVIDENCE_VERSION.to_owned(),
            source_compile_task_sha256: input.source_compile_task_sha256,
            fluent_bit_revision: input.fluent_bit_revision,
            monkey_source_sha256: input.monkey_source_sha256,
            monkey_profile_sha256: input.monkey_profile_sha256,
            monkey_build_sha256: input.monkey_build_sha256,
            libevent_revision: input.libevent_revision,
            libevent_source_sha256: input.libevent_source_sha256,
            libevent_profile_sha256: input.libevent_profile_sha256,
            libevent_build_sha256: input.libevent_build_sha256,
            contracts_sha256: input.contracts_sha256,
            source_rules_sha256: input.source_rules_sha256,
            observation_rules_sha256: input.observation_rules_sha256,
            observer_binary_sha256: input.observer_binary_sha256,
            observer_harness_sha256: input.observer_harness_sha256,
            linked_libraries_sha256: input.linked_libraries_sha256,
            observations_sha256: qcore_schemas::sha256_hex(payload.observations),
            bindings_sha256: qcore_schemas::sha256_hex(payload.bindings),
            observation_trust: input.observation_trust,
        };
        context.validate()?;
        Ok(context)
    }

    fn validate(&self) -> Result<(), ContextValidationError> {
        if self.schema_version != EVENT_COMPOSITION_CONTEXT_VERSION {
            return Err(ContextValidationError::InvalidField("schema_version"));
        }
        for (name, value) in [
            ("artifact", self.artifact.as_str()),
            ("property", self.property.as_str()),
            ("semantic_profile", self.semantic_profile.as_str()),
            ("evidence_version", self.evidence_version.as_str()),
        ] {
            if !nonempty(value) {
                return Err(ContextValidationError::InvalidField(name));
            }
        }
        if self.property != EVENT_COMPOSITION_PROPERTY {
            return Err(ContextValidationError::InvalidField("property"));
        }
        if self.evidence_version != EVENT_COMPOSITION_EVIDENCE_VERSION {
            return Err(ContextValidationError::InvalidField("evidence_version"));
        }
        if self.scope.is_empty() || self.scope.iter().any(|value| !nonempty(value)) {
            return Err(ContextValidationError::InvalidField("scope"));
        }
        if self.bounds.is_empty()
            || self
                .bounds
                .iter()
                .any(|(name, value)| !nonempty(name) || *value == 0)
        {
            return Err(ContextValidationError::InvalidField("bounds"));
        }
        if self.assumptions.is_empty()
            || self
                .assumptions
                .iter()
                .any(|(name, value)| !nonempty(name) || !nonempty(value))
        {
            return Err(ContextValidationError::InvalidField("assumptions"));
        }
        for (name, value) in [
            (
                "source_compile_task_sha256",
                &self.source_compile_task_sha256,
            ),
            ("monkey_source_sha256", &self.monkey_source_sha256),
            ("monkey_profile_sha256", &self.monkey_profile_sha256),
            ("monkey_build_sha256", &self.monkey_build_sha256),
            ("libevent_source_sha256", &self.libevent_source_sha256),
            ("libevent_profile_sha256", &self.libevent_profile_sha256),
            ("libevent_build_sha256", &self.libevent_build_sha256),
            ("contracts_sha256", &self.contracts_sha256),
            ("source_rules_sha256", &self.source_rules_sha256),
            ("observation_rules_sha256", &self.observation_rules_sha256),
            ("observer_binary_sha256", &self.observer_binary_sha256),
            ("observer_harness_sha256", &self.observer_harness_sha256),
            ("observations_sha256", &self.observations_sha256),
            ("bindings_sha256", &self.bindings_sha256),
        ] {
            if !is_sha256(value) {
                return Err(ContextValidationError::InvalidField(name));
            }
        }
        if !is_git_revision(&self.fluent_bit_revision) {
            return Err(ContextValidationError::InvalidField("fluent_bit_revision"));
        }
        if !is_git_revision(&self.libevent_revision) {
            return Err(ContextValidationError::InvalidField("libevent_revision"));
        }
        if !self.linked_libraries_sha256.contains_key("monkey")
            || !self.linked_libraries_sha256.contains_key("libevent")
            || self
                .linked_libraries_sha256
                .iter()
                .any(|(name, digest)| !nonempty(name) || !is_sha256(digest))
        {
            return Err(ContextValidationError::InvalidField(
                "linked_libraries_sha256",
            ));
        }
        Ok(())
    }

    #[must_use]
    pub fn artifact(&self) -> &str {
        &self.artifact
    }

    #[must_use]
    pub fn property(&self) -> &str {
        &self.property
    }

    #[must_use]
    pub fn semantic_profile(&self) -> &str {
        &self.semantic_profile
    }

    #[must_use]
    pub fn scope(&self) -> &BTreeSet<String> {
        &self.scope
    }

    #[must_use]
    pub fn bounds(&self) -> &BTreeMap<String, u64> {
        &self.bounds
    }

    #[must_use]
    pub fn assumptions(&self) -> &BTreeMap<String, String> {
        &self.assumptions
    }

    #[must_use]
    pub fn source_compile_task_sha256(&self) -> &str {
        &self.source_compile_task_sha256
    }

    #[must_use]
    pub fn fluent_bit_revision(&self) -> &str {
        &self.fluent_bit_revision
    }

    #[must_use]
    pub fn monkey_source_sha256(&self) -> &str {
        &self.monkey_source_sha256
    }

    #[must_use]
    pub fn monkey_profile_sha256(&self) -> &str {
        &self.monkey_profile_sha256
    }

    #[must_use]
    pub fn monkey_build_sha256(&self) -> &str {
        &self.monkey_build_sha256
    }

    #[must_use]
    pub fn libevent_revision(&self) -> &str {
        &self.libevent_revision
    }

    #[must_use]
    pub fn libevent_source_sha256(&self) -> &str {
        &self.libevent_source_sha256
    }

    #[must_use]
    pub fn libevent_profile_sha256(&self) -> &str {
        &self.libevent_profile_sha256
    }

    #[must_use]
    pub fn libevent_build_sha256(&self) -> &str {
        &self.libevent_build_sha256
    }

    #[must_use]
    pub fn observer_binary_sha256(&self) -> &str {
        &self.observer_binary_sha256
    }

    #[must_use]
    pub fn observer_harness_sha256(&self) -> &str {
        &self.observer_harness_sha256
    }

    #[must_use]
    pub fn linked_libraries_sha256(&self) -> &BTreeMap<String, String> {
        &self.linked_libraries_sha256
    }

    #[must_use]
    pub fn contracts_sha256(&self) -> &str {
        &self.contracts_sha256
    }

    #[must_use]
    pub fn source_rules_sha256(&self) -> &str {
        &self.source_rules_sha256
    }

    #[must_use]
    pub fn observation_rules_sha256(&self) -> &str {
        &self.observation_rules_sha256
    }

    #[must_use]
    pub fn observations_sha256(&self) -> &str {
        &self.observations_sha256
    }

    #[must_use]
    pub fn bindings_sha256(&self) -> &str {
        &self.bindings_sha256
    }

    #[must_use]
    pub fn evidence_version(&self) -> &str {
        &self.evidence_version
    }

    #[must_use]
    pub const fn observation_trust(&self) -> DeclaredObservationTrust {
        self.observation_trust
    }

    /// Canonical digest suitable for identifying this complete context in a
    /// report. All maps and sets have deterministic order.
    #[must_use]
    pub fn context_sha256(&self) -> String {
        let encoded =
            serde_json::to_vec(self).expect("EventCompositionContext serialization is infallible");
        qcore_schemas::sha256_hex(&encoded)
    }
}

fn require_equal<T: PartialEq>(
    presented: &T,
    expected: &T,
    field: &'static str,
) -> Result<(), ContextValidationError> {
    if presented == expected {
        Ok(())
    } else {
        Err(ContextValidationError::ContextMismatch(field))
    }
}

/// Recompute presented payload identities, validate both complete contexts,
/// and compare every field with the independently held expected context.
///
/// The returned token establishes only this equality-and-rederivation result.
pub fn check_event_composition_context_for(
    presented: &EventCompositionContext,
    presented_payload: EventCompositionPayload<'_>,
    expected: &EventCompositionContext,
) -> Result<ValidatedEventCompositionContext, ContextValidationError> {
    presented.validate()?;
    expected.validate()?;
    if qcore_schemas::sha256_hex(presented_payload.observations) != presented.observations_sha256 {
        return Err(ContextValidationError::PayloadDigestMismatch(
            "observations",
        ));
    }
    if qcore_schemas::sha256_hex(presented_payload.bindings) != presented.bindings_sha256 {
        return Err(ContextValidationError::PayloadDigestMismatch("bindings"));
    }
    require_equal(
        &presented.schema_version,
        &expected.schema_version,
        "schema_version",
    )?;
    require_equal(&presented.artifact, &expected.artifact, "artifact")?;
    require_equal(&presented.property, &expected.property, "property")?;
    require_equal(
        &presented.semantic_profile,
        &expected.semantic_profile,
        "semantic_profile",
    )?;
    require_equal(&presented.scope, &expected.scope, "scope")?;
    require_equal(&presented.bounds, &expected.bounds, "bounds")?;
    require_equal(&presented.assumptions, &expected.assumptions, "assumptions")?;
    require_equal(
        &presented.evidence_version,
        &expected.evidence_version,
        "evidence_version",
    )?;
    require_equal(
        &presented.source_compile_task_sha256,
        &expected.source_compile_task_sha256,
        "source_compile_task_sha256",
    )?;
    require_equal(
        &presented.fluent_bit_revision,
        &expected.fluent_bit_revision,
        "fluent_bit_revision",
    )?;
    require_equal(
        &presented.monkey_source_sha256,
        &expected.monkey_source_sha256,
        "monkey_source_sha256",
    )?;
    require_equal(
        &presented.monkey_profile_sha256,
        &expected.monkey_profile_sha256,
        "monkey_profile_sha256",
    )?;
    require_equal(
        &presented.monkey_build_sha256,
        &expected.monkey_build_sha256,
        "monkey_build_sha256",
    )?;
    require_equal(
        &presented.libevent_revision,
        &expected.libevent_revision,
        "libevent_revision",
    )?;
    require_equal(
        &presented.libevent_source_sha256,
        &expected.libevent_source_sha256,
        "libevent_source_sha256",
    )?;
    require_equal(
        &presented.libevent_profile_sha256,
        &expected.libevent_profile_sha256,
        "libevent_profile_sha256",
    )?;
    require_equal(
        &presented.libevent_build_sha256,
        &expected.libevent_build_sha256,
        "libevent_build_sha256",
    )?;
    require_equal(
        &presented.contracts_sha256,
        &expected.contracts_sha256,
        "contracts_sha256",
    )?;
    require_equal(
        &presented.source_rules_sha256,
        &expected.source_rules_sha256,
        "source_rules_sha256",
    )?;
    require_equal(
        &presented.observation_rules_sha256,
        &expected.observation_rules_sha256,
        "observation_rules_sha256",
    )?;
    require_equal(
        &presented.observer_binary_sha256,
        &expected.observer_binary_sha256,
        "observer_binary_sha256",
    )?;
    require_equal(
        &presented.observer_harness_sha256,
        &expected.observer_harness_sha256,
        "observer_harness_sha256",
    )?;
    require_equal(
        &presented.linked_libraries_sha256,
        &expected.linked_libraries_sha256,
        "linked_libraries_sha256",
    )?;
    require_equal(
        &presented.observations_sha256,
        &expected.observations_sha256,
        "observations_sha256",
    )?;
    require_equal(
        &presented.bindings_sha256,
        &expected.bindings_sha256,
        "bindings_sha256",
    )?;
    require_equal(
        &presented.observation_trust,
        &expected.observation_trust,
        "observation_trust",
    )?;
    Ok(ValidatedEventCompositionContext {
        context: presented.clone(),
    })
}
