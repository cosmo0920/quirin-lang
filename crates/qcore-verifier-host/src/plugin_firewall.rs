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

//! The checker-plugin firewall (card **W45-USER-CONTRACT-PLUGINS**; contract
//! specification §C11.1).
//!
//! A user may attach an out-of-process checker to a declared contract. It is
//! an ordinary process provider: the same package verification, protocol-v1
//! framing, evidence-v2 envelope, deadline, output bound, execution profile
//! and quotas as every other provider, dispatched through
//! [`VerifierRegistry`]. It is never run in-process. What this module adds is
//! the classification of its answer into the four-value verdict:
//!
//! | The plugin | Its verdict |
//! |---|---|
//! | proposed `Safe`, and the registry issued a [`CoreCertificate`] (a trusted-core [`WitnessChecker`] replayed the witness against the core's own obligation inventory) | `Safe` |
//! | proposed `Safe` with no evidence (`QP008`), or with evidence for which no core checker exists (`QP016`) | `Pending` (`plugin-claim-uncertified`) |
//! | proposed `Unsafe`, and a trusted-core [`CounterexampleChecker`] validated its witness for a non-empty set of the core's obligations | `Unsafe` (provisional) |
//! | proposed `Unsafe` with no witness, or with no counterexample checker for the capability (`QP010`) | `Pending` (`plugin-claim-uncertified`) |
//! | proposed `Unknown` | `Unknown` |
//! | a request or task gate (`QP001`–`QP005`), a spawn failure, crash, timeout, quota or output-limit breach or protocol error (`QP006`), an invalid envelope (`QP007`), or evidence that does not bind or that the checker refutes (`QP009`) | `Unknown` |
//!
//! `Pending` is the answer only where the claim was never *checked* — there
//! was nothing checkable, or nothing to check it with — so a person can
//! resolve it by supplying a certificate or witness the audit checks. A claim
//! that was checked and failed, or that does not bind to the request, is not
//! resolved by a person's act and stays `Unknown`. Neither is ever `Safe`.
//!
//! The certificate is unforgeable by construction: [`CoreCertificate`] has no
//! public constructor, and the registry issues one only from the witness
//! replay. No production counterexample checker exists, so no plugin `Unsafe`
//! is validated today; every one is `Pending`.

use std::collections::BTreeSet;

use qcore_schemas::reason::ReasonCategory;
use qcore_schemas::verdict::{PendingReason, ReasonedVerdict};

use super::{
    CoreCertificate, EvidenceRejection, PLUGIN_EVIDENCE_VERSION_V2, PluginDiagnostic,
    PluginManifest, ProposedEvidenceV2, ProposedVerdict, SemanticWitness, VerificationRequestV2,
    VerificationTask, VerifierCapability, VerifierRegistry, WitnessChecker,
};

/// The trusted-core counterpart of [`WitnessChecker`] for a violation claim.
///
/// Like every witness checker, implementations belong to trusted-core crates;
/// no provider-reachable code influences one. It replays a counterexample
/// witness against the core's own task and returns the obligations the
/// witness shows violated. An empty set, or an obligation outside the core's
/// inventory, is a refusal.
pub trait CounterexampleChecker {
    /// The capability this checker validates counterexamples for.
    fn capability(&self) -> VerifierCapability;

    /// Replays `witness` and returns the violated obligation identifiers.
    ///
    /// # Errors
    ///
    /// A witness that does not show a violation is refused with a reason.
    fn check(
        &self,
        task: &VerificationTask,
        witness: &SemanticWitness,
    ) -> Result<BTreeSet<String>, String>;
}

/// A checker plugin's answer, classified by the firewall.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PluginClaim {
    /// `Safe`, with the certificate the registry issued.
    CertifiedSafe(Box<CoreCertificate>),
    /// `Unsafe`, with the obligations a trusted counterexample checker showed
    /// violated.
    ValidatedUnsafe { violated: BTreeSet<String> },
    /// `Safe` proposed with nothing the audit could check.
    UncertifiedSafe { code: String, reason: String },
    /// `Unsafe` proposed with nothing the audit could validate.
    UnvalidatedUnsafe { code: String, reason: String },
    /// The plugin answered `Unknown`.
    Undecided { diagnostics: Vec<PluginDiagnostic> },
    /// A gate, transport, envelope or evidence failure.
    Failure { code: String, reason: String },
}

impl PluginClaim {
    /// The claim's contribution to a chain, with its reason.
    #[must_use]
    pub fn verdict(&self, plugin: &str) -> ReasonedVerdict {
        match self {
            Self::CertifiedSafe(_) => ReasonedVerdict::safe(),
            Self::ValidatedUnsafe { violated } => ReasonedVerdict::violation(
                "plugin-counterexample",
                format!(
                    "`{plugin}` supplied a counterexample the audit validated for {} obligation(s)",
                    violated.len()
                ),
            ),
            Self::UncertifiedSafe { code, reason } => ReasonedVerdict::pending(
                PendingReason::PluginClaimUncertified,
                format!(
                    "plugin claim uncertified: `{plugin}` proposed Safe without a certificate \
                     the audit checks independently ({code}: {reason})"
                ),
            ),
            Self::UnvalidatedUnsafe { code, reason } => ReasonedVerdict::pending(
                PendingReason::PluginClaimUncertified,
                format!(
                    "plugin claim uncertified: `{plugin}` proposed Unsafe without a witness \
                     the audit validates ({code}: {reason})"
                ),
            ),
            Self::Undecided { diagnostics } => ReasonedVerdict::unknown(
                ReasonCategory::IncompleteObligationCoverage,
                format!(
                    "`{plugin}` answered Unknown{}",
                    diagnostics
                        .iter()
                        .map(|diagnostic| format!("; {}: {}", diagnostic.code, diagnostic.message))
                        .collect::<String>()
                ),
            )
            .expect("an analyser category that is not human-resolvable"),
            Self::Failure { code, reason } => ReasonedVerdict::unknown(
                ReasonCategory::ProviderFailure,
                format!("selected provider `{plugin}` failed ({code}: {reason})"),
            )
            .expect("an analyser category that is not human-resolvable"),
        }
    }

    /// The diagnostic code of a refusal, if the claim is one.
    #[must_use]
    pub fn code(&self) -> Option<&str> {
        match self {
            Self::UncertifiedSafe { code, .. }
            | Self::UnvalidatedUnsafe { code, .. }
            | Self::Failure { code, .. } => Some(code),
            _ => None,
        }
    }
}

/// The first diagnostic of a host refusal, as a failure.
fn refusal(outcome: &super::VerificationOutcome) -> PluginClaim {
    let (code, reason) = outcome
        .diagnostics
        .first()
        .map(|diagnostic| (diagnostic.code.clone(), diagnostic.message.clone()))
        .unwrap_or_else(|| ("QP006".into(), "the host refused the request".into()));
    PluginClaim::Failure { code, reason }
}

impl VerifierRegistry {
    /// Dispatch a checker plugin's evidence-v2 request and classify its
    /// answer (see the module table). `checker` is the trusted-core witness
    /// checker for `request.capability`, `counterexample` its violation
    /// counterpart; `None` means the audit has none, which leaves a `Safe` or
    /// an `Unsafe` proposal uncertified.
    #[must_use]
    pub fn verify_plugin_claim(
        &self,
        request: &VerificationRequestV2,
        task: &VerificationTask,
        checker: Option<&dyn WitnessChecker>,
        counterexample: Option<&dyn CounterexampleChecker>,
    ) -> PluginClaim {
        let (manifest, response) = match self.dispatch_v2(request, task) {
            Ok(dispatched) => dispatched,
            Err(outcome) => return refusal(&outcome),
        };
        let proposed = response.verdict;
        let evidence = response.evidence.clone();
        let outcome = self.check_response_v2(manifest, request, task, checker, response);
        match (proposed, outcome.verdict) {
            (ProposedVerdict::Safe, ProposedVerdict::Safe) => match outcome.certificate {
                Some(certificate) => PluginClaim::CertifiedSafe(Box::new(certificate)),
                None => refusal(&outcome),
            },
            (ProposedVerdict::Safe, _) => match outcome.diagnostics.first() {
                Some(first) if matches!(first.code.as_str(), "QP008" | "QP016") => {
                    PluginClaim::UncertifiedSafe {
                        code: first.code.clone(),
                        reason: first.message.clone(),
                    }
                }
                _ => refusal(&outcome),
            },
            (ProposedVerdict::Unsafe, _)
                if outcome
                    .diagnostics
                    .first()
                    .is_some_and(|first| first.code == "QP010") =>
            {
                self.classify_unsafe(manifest, request, task, evidence.as_ref(), counterexample)
            }
            (ProposedVerdict::Unknown, ProposedVerdict::Unknown)
                if outcome
                    .diagnostics
                    .first()
                    .is_none_or(|first| !first.code.starts_with("QP")) =>
            {
                PluginClaim::Undecided {
                    diagnostics: outcome.diagnostics,
                }
            }
            _ => refusal(&outcome),
        }
    }

    /// An `Unsafe` proposal that passed the response envelope: validated by a
    /// trusted counterexample checker, uncertified without a witness or a
    /// checker, and a failure when the witness does not bind or is refuted.
    fn classify_unsafe(
        &self,
        manifest: &PluginManifest,
        request: &VerificationRequestV2,
        task: &VerificationTask,
        evidence: Option<&ProposedEvidenceV2>,
        counterexample: Option<&dyn CounterexampleChecker>,
    ) -> PluginClaim {
        let Some(evidence) = evidence else {
            return PluginClaim::UnvalidatedUnsafe {
                code: "QP010".into(),
                reason: "unverified provider allegation: the Unsafe proposal carries no witness"
                    .into(),
            };
        };
        let Some(checker) =
            counterexample.filter(|checker| checker.capability() == request.capability)
        else {
            return PluginClaim::UnvalidatedUnsafe {
                code: "QP010".into(),
                reason: "unverified provider allegation: evidence v2 has no counterexample \
                         checker"
                    .into(),
            };
        };
        if let Err(EvidenceRejection { code, reason }) = check_envelope(manifest, request, evidence)
        {
            return PluginClaim::Failure {
                code: code.into(),
                reason,
            };
        }
        let required = task.coverage_for(request.capability);
        match checker.check(task, &evidence.witness) {
            Ok(violated) if !violated.is_empty() && violated.is_subset(&required) => {
                PluginClaim::ValidatedUnsafe { violated }
            }
            Ok(_) => PluginClaim::Failure {
                code: "QP009".into(),
                reason: "the counterexample names no obligation of the core's inventory".into(),
            },
            Err(reason) => PluginClaim::Failure {
                code: "QP009".into(),
                reason,
            },
        }
    }
}

/// The evidence envelope every v2 proposal must bind to its request: the same
/// identity checks the certificate path applies before any replay.
fn check_envelope(
    manifest: &PluginManifest,
    request: &VerificationRequestV2,
    evidence: &ProposedEvidenceV2,
) -> Result<(), EvidenceRejection> {
    let reject = |reason: &str| EvidenceRejection {
        code: "QP009",
        reason: reason.to_owned(),
    };
    if evidence.evidence_version != request.evidence_version
        || evidence.evidence_version < PLUGIN_EVIDENCE_VERSION_V2
    {
        return Err(reject(
            "the evidence version does not match the negotiated request",
        ));
    }
    if evidence.unit != request.unit
        || evidence.capability != request.capability
        || evidence.assumptions_used != request.assumptions
        || evidence.reproducibility.provider_version != manifest.package_version
        || evidence.reproducibility.package_digest != manifest.package_digest
        || !evidence.reproducibility.deterministic
    {
        return Err(reject(
            "the core counterexample checker rejected the evidence envelope",
        ));
    }
    if evidence.witness.task_digest() != request.task_digest {
        return Err(reject(
            "the witness is bound to a different verification task",
        ));
    }
    Ok(())
}
