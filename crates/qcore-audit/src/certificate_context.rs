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

//! Audit-owned binding between a gateway certificate and caller-held context.
//!
//! The gateway owns structural certificate replay. This layer owns the facts
//! that exist only in configured audit orchestration: the complete compile-task
//! identity and the declared contract/profile trust environment.

use crate::compile_db::CompileTaskIdentity;
use qcore_c_gateway::legacy_c::{
    LegacyCertificateExpectedContext, LegacySafetyCertificate, PreparedTranslationUnit,
    check_legacy_safety_certificate_for, legacy_certificate_expected_context_from_prepared,
};
use qcore_schemas::contract::ResourceContracts;
use serde::Serialize;

pub const AUDIT_TRUST_CONTEXT_IDENTITY_VERSION: u32 = 1;

/// The trust rule the configured resource projection actually uses.
///
/// Contract modules are declared trusted assumptions. They are reloaded and
/// structurally checked, but this audit does not verify their claims against
/// library bodies and admits no observation/provider trust at this boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditContractTrustPolicy {
    DeclaredSourcesAreTrustedAssumptions,
}

/// Identity of the contract/profile/trust environment intended by the caller.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AuditTrustContextIdentity {
    schema_version: u32,
    resource_contracts_sha256: String,
    contract_sources_sha256: String,
    source_profile_sha256: String,
    contract_trust_policy: AuditContractTrustPolicy,
}

impl AuditTrustContextIdentity {
    /// Derive identity from independently loaded audit inputs.
    pub fn derive(
        contracts: &ResourceContracts,
        source_profile_sha256: &str,
        contract_trust_policy: AuditContractTrustPolicy,
    ) -> Result<Self, String> {
        if source_profile_sha256.len() != 64
            || !source_profile_sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err("source-profile identity is missing or not a SHA-256".into());
        }
        if contracts.source_assumptions().is_empty() {
            return Err(
                "declared-contract trust requires retained contract source assumptions".into(),
            );
        }
        qcore_contract::contract_loader::check_source_assumptions(contracts)?;
        let resource_contracts_sha256 = qcore_schemas::sha256_hex(
            &serde_json::to_vec(contracts).map_err(|error| error.to_string())?,
        );
        let contract_sources_sha256 = qcore_schemas::sha256_hex(
            &serde_json::to_vec(contracts.source_assumptions())
                .map_err(|error| error.to_string())?,
        );
        Ok(Self {
            schema_version: AUDIT_TRUST_CONTEXT_IDENTITY_VERSION,
            resource_contracts_sha256,
            contract_sources_sha256,
            source_profile_sha256: source_profile_sha256.to_owned(),
            contract_trust_policy,
        })
    }

    fn validate_compile_task(&self, task: &CompileTaskIdentity) -> Result<(), String> {
        if task.dependency("source_profile") != Some(self.source_profile_sha256.as_str()) {
            return Err(
                "contract/profile/trust identity disagrees with compile-task source-profile provenance"
                    .into(),
            );
        }
        Ok(())
    }
}

/// A gateway certificate with the audit-owned context presented by its issuer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditBoundLegacySafetyCertificate {
    pub certificate: LegacySafetyCertificate,
    pub compile_task_identity: CompileTaskIdentity,
    pub trust_context: AuditTrustContextIdentity,
}

/// Context independently held by the caller accepting a bound certificate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExpectedAuditCertificateContext {
    pub target: LegacyCertificateExpectedContext,
    pub compile_task_identity: CompileTaskIdentity,
    pub trust_context: AuditTrustContextIdentity,
}

/// Bind an issued certificate to the actual compile task and profile used by
/// configured audit orchestration. Contract identity is derived from the
/// certificate's analyzed contract environment, never supplied as a string.
pub fn bind_audit_legacy_safety_certificate(
    certificate: &LegacySafetyCertificate,
    compile_task_identity: CompileTaskIdentity,
    source_profile_sha256: &str,
    contract_trust_policy: AuditContractTrustPolicy,
) -> Result<AuditBoundLegacySafetyCertificate, String> {
    let trust_context = binding_trust_context(
        certificate,
        &compile_task_identity,
        source_profile_sha256,
        contract_trust_policy,
    )?;
    Ok(AuditBoundLegacySafetyCertificate {
        certificate: certificate.clone(),
        compile_task_identity,
        trust_context,
    })
}

fn binding_trust_context(
    certificate: &LegacySafetyCertificate,
    compile_task_identity: &CompileTaskIdentity,
    source_profile_sha256: &str,
    contract_trust_policy: AuditContractTrustPolicy,
) -> Result<AuditTrustContextIdentity, String> {
    compile_task_identity.validate_certificate_binding(certificate)?;
    let trust_context = AuditTrustContextIdentity::derive(
        &certificate.contracts,
        source_profile_sha256,
        contract_trust_policy,
    )?;
    trust_context.validate_compile_task(compile_task_identity)?;
    Ok(trust_context)
}

/// Admit an already owned certificate without cloning its CFGs, contracts or
/// source artifacts into a temporary wrapper. These are actual issuer inputs;
/// `expected` remains independently held by the accepting caller. This runs
/// exactly the same binding checks and fresh-source replay as the owned route.
pub fn check_audit_legacy_safety_certificate_in_context(
    certificate: &LegacySafetyCertificate,
    compile_task_identity: &CompileTaskIdentity,
    source_profile_sha256: &str,
    contract_trust_policy: AuditContractTrustPolicy,
    expected: &ExpectedAuditCertificateContext,
) -> Result<(), String> {
    let trust_context = binding_trust_context(
        certificate,
        compile_task_identity,
        source_profile_sha256,
        contract_trust_policy,
    )?;
    check_certificate_parts(certificate, compile_task_identity, &trust_context, expected)
}

/// Derive expected values from the caller's prepared request, compile task,
/// independently loaded contracts and source profile.
pub fn expected_audit_certificate_context(
    prepared: &PreparedTranslationUnit,
    compile_task_identity: CompileTaskIdentity,
    contracts: &ResourceContracts,
    source_profile_sha256: &str,
    contract_trust_policy: AuditContractTrustPolicy,
) -> Result<ExpectedAuditCertificateContext, String> {
    compile_task_identity.validate_prepared_binding(prepared)?;
    let target = legacy_certificate_expected_context_from_prepared(prepared)
        .map_err(|error| error.to_string())?;
    let trust_context =
        AuditTrustContextIdentity::derive(contracts, source_profile_sha256, contract_trust_policy)?;
    trust_context.validate_compile_task(&compile_task_identity)?;
    Ok(ExpectedAuditCertificateContext {
        target,
        compile_task_identity,
        trust_context,
    })
}

/// Structural replay plus independently held target, compilation and trust
/// context. Every absent or unequal component refuses.
pub fn check_audit_legacy_safety_certificate_for(
    bound: &AuditBoundLegacySafetyCertificate,
    expected: &ExpectedAuditCertificateContext,
) -> Result<(), String> {
    check_certificate_parts(
        &bound.certificate,
        &bound.compile_task_identity,
        &bound.trust_context,
        expected,
    )
}

fn check_certificate_parts(
    certificate: &LegacySafetyCertificate,
    compile_task_identity: &CompileTaskIdentity,
    trust_context: &AuditTrustContextIdentity,
    expected: &ExpectedAuditCertificateContext,
) -> Result<(), String> {
    compile_task_identity.validate_certificate_binding(certificate)?;
    expected.compile_task_identity.validate_for_certificate()?;
    trust_context.validate_compile_task(compile_task_identity)?;
    expected
        .trust_context
        .validate_compile_task(&expected.compile_task_identity)?;
    let presented_trust = AuditTrustContextIdentity::derive(
        &certificate.contracts,
        &trust_context.source_profile_sha256,
        trust_context.contract_trust_policy,
    )?;
    if presented_trust != *trust_context {
        return Err(
            "certificate contracts differ from the bound contract/profile/trust identity".into(),
        );
    }
    check_legacy_safety_certificate_for(certificate, &expected.target)?;
    if compile_task_identity != &expected.compile_task_identity {
        return Err(
            "certificate compile-task provenance differs from the expected audit task".into(),
        );
    }
    if trust_context != &expected.trust_context {
        return Err(
            "certificate contract/profile/trust identity differs from the expected audit context"
                .into(),
        );
    }
    Ok(())
}
