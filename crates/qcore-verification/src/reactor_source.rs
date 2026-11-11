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

//! Unit-scoped reactor callback evidence for one admitted callback family.
//!
//! Only [`import_reactor_program`] can construct the replay context, and it
//! refuses any program outside the admitted family. The declared reactor
//! program, its accepted build profiles and the callback authority it names
//! remain *caller-supplied model premises*, exactly as they are for the
//! in-process route: this module transports and replays them, it does not
//! derive them from C source.
//!
//! A provider's proposal is compared with the assignment inventory the core
//! exports from its own program, and acceptance still terminates in the core
//! running [`qcore_core::event_reactor::check_event_program`] itself. Agreement
//! alone never discharges an obligation, and a proposal can neither add an
//! assignment, remove one, retarget a generation nor widen callback authority.

use std::collections::{BTreeMap, BTreeSet};

use qcore_core::event_reactor as model;
use qcore_schemas::obligation::*;
use qcore_schemas::plugin::*;
use qcore_schemas::reactor as proposal_wire;
use qcore_schemas::reactor_program::*;
use qcore_schemas::{SemanticWitness, sha256_hex};
use qcore_verifier_host::WitnessChecker;

use crate::reactor_callback::{
    CallbackProposalError, check_declared_callback_proposal, declared_callback_proposal,
};

pub const REACTOR_SEMANTIC_PROFILE: &str = "qcore.reactor_callback_program.v1";

/// Bounds the projected program before any structural reading.
const MAX_OPERATIONS: usize = 1024;

/// Why a declared program is outside the admitted callback family. Every
/// variant keeps the program's callbacks `Unknown`; there is no fallback
/// encoding and no partial projection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReactorImportError {
    /// The operation has no record in the bounded projection vocabulary:
    /// bufferevent custody, raw socket-descriptor custody, an ordering
    /// assumption or loop control.
    UnsupportedOperation(usize),
    /// An access attributed to a bufferevent context.
    UnsupportedAccessContext(usize),
    /// The family is one base under one declared build profile.
    ProfileNotSingular,
    /// The program declares no accepted build profile, so the backend the
    /// base would run under is not a supported one.
    UnsupportedBackend,
    EmptyProgram,
    ProgramTooLarge(usize),
    /// A declared identity field the task must bind was empty.
    IncompleteIdentity,
}

impl std::fmt::Display for ReactorImportError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

/// The declared, digest-bound identity of one projected reactor program.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReactorProgramIdentity {
    pub library: String,
    pub source_revision: String,
    /// Opaque declared build attributes. They are bound by digest only; the
    /// model's own profile decides acceptance.
    pub build_profile: BTreeMap<String, String>,
    pub profile_assumptions: BTreeMap<String, String>,
}

/// Sealed replay context. A raw wire projection cannot construct one: the
/// fields are private, there is no public constructor, no `Deserialize` and no
/// conversion from [`ReactorProgramInput`].
///
/// ```compile_fail
/// # use qcore_verification::reactor_source::ReactorProgramImport;
/// # fn f(bytes: &[u8]) {
/// let _: ReactorProgramImport<()> = serde_json::from_slice(bytes).unwrap();
/// # }
/// ```
pub struct ReactorProgramImport<P: model::ReactorProfile> {
    program: model::EventProgram<P>,
    accepted_profiles: Vec<P>,
    identity: ReactorProgramIdentity,
    input: ReactorProgramInput,
}

impl<P: model::ReactorProfile> ReactorProgramImport<P> {
    /// The core's own program. Never reconstructed from a proposal.
    pub fn program(&self) -> &model::EventProgram<P> {
        &self.program
    }

    pub fn accepted_profiles(&self) -> &[P] {
        &self.accepted_profiles
    }

    pub fn identity(&self) -> &ReactorProgramIdentity {
        &self.identity
    }

    /// The projection that travels to a provider. It carries the program and
    /// no proposal: no assignment inventory, no certificate, no verdict.
    pub fn source_input(&self) -> ReactorProgramInput {
        self.input.clone()
    }
}

fn event(key: model::EventKey) -> proposal_wire::EventKeyRecord {
    proposal_wire::EventKeyRecord {
        id: key.id,
        generation: key.generation,
    }
}

fn reason(value: model::ActivationReason) -> proposal_wire::ActivationReasonRecord {
    match value {
        model::ActivationReason::Read => proposal_wire::ActivationReasonRecord::Read,
        model::ActivationReason::Write => proposal_wire::ActivationReasonRecord::Write,
        model::ActivationReason::Timeout => proposal_wire::ActivationReasonRecord::Timeout,
        model::ActivationReason::Signal => proposal_wire::ActivationReasonRecord::Signal,
        model::ActivationReason::Explicit => proposal_wire::ActivationReasonRecord::Explicit,
    }
}

fn access_kind(value: model::PayloadAccessKind) -> proposal_wire::PayloadAccessKindRecord {
    match value {
        model::PayloadAccessKind::Read => proposal_wire::PayloadAccessKindRecord::Read,
        model::PayloadAccessKind::Write => proposal_wire::PayloadAccessKindRecord::Write,
        model::PayloadAccessKind::AtomicRead => proposal_wire::PayloadAccessKindRecord::AtomicRead,
        model::PayloadAccessKind::AtomicWrite => {
            proposal_wire::PayloadAccessKindRecord::AtomicWrite
        }
    }
}

fn summary(value: &model::CallbackSummary) -> proposal_wire::CallbackSummaryRecord {
    proposal_wire::CallbackSummaryRecord {
        argument: value.argument,
        argument_mode: match value.argument_mode {
            model::ArgumentMode::Owned => proposal_wire::ArgumentModeRecord::Owned,
            model::ArgumentMode::Borrowed => proposal_wire::ArgumentModeRecord::Borrowed,
            model::ArgumentMode::Shared => proposal_wire::ArgumentModeRecord::Shared,
        },
        trigger_reasons: value.trigger_reasons.iter().copied().map(reason).collect(),
        payload_authority: value
            .payload_authority
            .iter()
            .map(|(&payload, &kind)| proposal_wire::PayloadAuthorityRecord {
                payload,
                kind: access_kind(kind),
            })
            .collect(),
        may_defer: value.may_defer,
        may_spawn_tasks: value.may_spawn_tasks,
    }
}

/// Project one operation, or refuse the whole program. Bufferevent custody,
/// raw socket-descriptor custody, ordering assumptions and loop control have
/// no record: they keep their programs `Unknown` rather than being dropped.
fn operation<P>(
    index: usize,
    value: &model::EventOperation<P>,
) -> Result<ReactorOperationRecord, ReactorImportError> {
    Ok(match value {
        model::EventOperation::CreateBase { base, .. } => {
            ReactorOperationRecord::CreateBase { base: *base }
        }
        model::EventOperation::BeginDispatch { base, dispatcher } => {
            ReactorOperationRecord::BeginDispatch {
                base: *base,
                dispatcher: *dispatcher,
            }
        }
        model::EventOperation::EndDispatch { base } => {
            ReactorOperationRecord::EndDispatch { base: *base }
        }
        model::EventOperation::Assign {
            event: key,
            base,
            descriptor,
            flags,
            priority,
            summary: callback,
        } => ReactorOperationRecord::Assign {
            event: event(*key),
            base: *base,
            descriptor: *descriptor,
            flags: flags.bits(),
            priority: *priority,
            summary: summary(callback),
        },
        model::EventOperation::Add { event: key } => {
            ReactorOperationRecord::Add { event: event(*key) }
        }
        model::EventOperation::Reschedule { event: key } => {
            ReactorOperationRecord::Reschedule { event: event(*key) }
        }
        model::EventOperation::Activate {
            event: key,
            reason: value,
            cross_thread,
        } => ReactorOperationRecord::Activate {
            event: event(*key),
            reason: reason(*value),
            cross_thread: *cross_thread,
        },
        model::EventOperation::CallbackEnter {
            event: key,
            reason: value,
        } => ReactorOperationRecord::CallbackEnter {
            event: event(*key),
            reason: reason(*value),
        },
        model::EventOperation::CallbackExit { event: key } => {
            ReactorOperationRecord::CallbackExit { event: event(*key) }
        }
        model::EventOperation::Delete { event: key, mode } => ReactorOperationRecord::Delete {
            event: event(*key),
            mode: match mode {
                model::DeleteMode::Ordinary => DeleteModeRecord::Ordinary,
                model::DeleteMode::Blocking => DeleteModeRecord::Blocking,
                model::DeleteMode::Nonblocking => DeleteModeRecord::Nonblocking,
            },
        },
        model::EventOperation::Finalize { event: key } => {
            ReactorOperationRecord::Finalize { event: event(*key) }
        }
        model::EventOperation::FreeEvent { event: key } => {
            ReactorOperationRecord::FreeEvent { event: event(*key) }
        }
        model::EventOperation::Access {
            context,
            payload,
            kind,
            synchronized,
        } => ReactorOperationRecord::Access {
            context: match context {
                model::CallbackContext::Event(key) => {
                    CallbackContextRecord::Event { event: event(*key) }
                }
                model::CallbackContext::Caller(id) => {
                    CallbackContextRecord::Caller { context: *id }
                }
                model::CallbackContext::Worker(id) => {
                    CallbackContextRecord::Worker { context: *id }
                }
                model::CallbackContext::Bufferevent(_) => {
                    return Err(ReactorImportError::UnsupportedAccessContext(index));
                }
            },
            payload: *payload,
            kind: access_kind(*kind),
            synchronized: *synchronized,
        },
        model::EventOperation::DropArgument { argument } => ReactorOperationRecord::DropArgument {
            argument: *argument,
        },
        model::EventOperation::FreeBase { base } => {
            ReactorOperationRecord::FreeBase { base: *base }
        }
        _ => return Err(ReactorImportError::UnsupportedOperation(index)),
    })
}

/// Seal one declared reactor program for transport and replay.
///
/// `accepted_profiles` is the caller's own accepted-build decision, unchanged
/// from the in-process route. An empty list means no base may be created, so
/// the backend is unsupported and the program stays `Unknown` — absence is
/// never acceptance.
pub fn import_reactor_program<P: model::ReactorProfile>(
    program: &model::EventProgram<P>,
    accepted_profiles: &[P],
    identity: ReactorProgramIdentity,
) -> Result<ReactorProgramImport<P>, ReactorImportError> {
    if identity.library.is_empty() || identity.source_revision.is_empty() {
        return Err(ReactorImportError::IncompleteIdentity);
    }
    if program.operations.is_empty() {
        return Err(ReactorImportError::EmptyProgram);
    }
    if program.operations.len() > MAX_OPERATIONS {
        return Err(ReactorImportError::ProgramTooLarge(
            program.operations.len(),
        ));
    }
    if accepted_profiles.is_empty() {
        return Err(ReactorImportError::UnsupportedBackend);
    }
    // The family is one base under one declared build profile, so the opaque
    // `build_profile` metadata identifies exactly the profile the model used.
    let declared = program
        .operations
        .iter()
        .filter_map(|operation| match operation {
            model::EventOperation::CreateBase { profile, .. } => Some(*profile),
            _ => None,
        })
        .collect::<Vec<_>>();
    if declared.len() != 1 || !accepted_profiles.contains(&declared[0]) {
        return Err(ReactorImportError::ProfileNotSingular);
    }
    let operations = program
        .operations
        .iter()
        .enumerate()
        .map(|(index, value)| operation(index, value))
        .collect::<Result<Vec<_>, _>>()?;
    let input = ReactorProgramInput {
        schema_version: REACTOR_PROGRAM_INPUT_VERSION,
        library: identity.library.clone(),
        source_revision: identity.source_revision.clone(),
        build_profile: identity.build_profile.clone(),
        live_arguments: program.live_arguments.iter().copied().collect(),
        operations,
        profile_assumptions: identity.profile_assumptions.clone(),
    };
    Ok(ReactorProgramImport {
        program: program.clone(),
        accepted_profiles: accepted_profiles.to_vec(),
        identity,
        input,
    })
}

/// Sealed replay context for one imported reactor program.
pub struct ReactorCallbackTask<P: model::ReactorProfile> {
    import: ReactorProgramImport<P>,
    task: VerificationTask,
}

impl<P: model::ReactorProfile> ReactorCallbackTask<P> {
    pub fn derive(
        import: ReactorProgramImport<P>,
        translation_unit: &str,
        target: TargetProfile,
    ) -> Result<Self, String> {
        let identity = import.identity();
        if translation_unit.is_empty()
            || target.profile != REACTOR_SEMANTIC_PROFILE
            || target.library != identity.library
            || target.revision != identity.source_revision
            || target.dialect.is_empty()
            || target.platform.is_empty()
            || target.backend.is_empty()
            || target.backend == "*"
        {
            return Err("incomplete or incompatible reactor target identity".into());
        }
        let input = import.source_input();
        let serialized_identity = serde_json::to_vec(&(
            &identity.library,
            &identity.source_revision,
            &identity.build_profile,
            &identity.profile_assumptions,
        ))
        .map_err(|e| e.to_string())?;
        let contract_sources = BTreeMap::from([
            (REACTOR_PROGRAM_INPUT_KEY.to_owned(), input.digest()),
            (
                "qcore:reactor-import:v1".into(),
                sha256_hex(&serialized_identity),
            ),
        ]);
        // The declared program, its callback authority and the accepted build
        // decision are premises of the model, not results of this route.
        let assumptions = BTreeMap::from([
            ("reactor_program_is_declared_trusted".into(), true),
            ("callback_authority_is_declared".into(), true),
            ("accepted_build_profile_is_declared".into(), true),
            (
                "declared_source_revision_is_outer_audit_assumption".into(),
                true,
            ),
        ]);
        let bounds = TaskBounds::single_path();
        let mut task = VerificationTask {
            task_version: VERIFICATION_TASK_VERSION,
            unit: UnitIdentity {
                translation_unit: translation_unit.into(),
                source_digest: input.digest(),
                build_digest: sha256_hex(&serialized_identity),
            },
            target,
            semantic_profile: REACTOR_SEMANTIC_PROFILE.into(),
            contract_sources,
            bounds: bounds.clone(),
            assumptions: assumptions.clone(),
            evidence_version: 2,
            obligations: Vec::new(),
            derivation_incomplete: Vec::new(),
        };
        let capability = VerifierCapability::EventTraces;
        // One indivisible model run decides both properties: `check_event_program`
        // rejects a stale generation and an unquiescent teardown in the same
        // pass, so neither can be discharged without the other.
        for property in [
            ObligationProperty::EventGenerationDischarged,
            ObligationProperty::CallbackQuiescentAtTeardown,
        ] {
            let subject = ObligationSubject::Unit;
            task.obligations.push(Obligation {
                obligation_id: task.obligation_id(capability, property, &subject),
                capability,
                property,
                subject,
                scope: ObligationScope::WholeUnit,
                bounds: bounds.clone(),
                assumptions: assumptions.clone(),
                evidence_version: 2,
            });
        }
        task.validate().map_err(|e| e.to_string())?;
        Ok(Self { import, task })
    }

    pub fn task(&self) -> &VerificationTask {
        &self.task
    }

    pub fn import(&self) -> &ReactorProgramImport<P> {
        &self.import
    }

    pub fn request(&self, plugin_id: PluginId) -> VerificationRequestV2 {
        VerificationRequestV2 {
            protocol_version: 1,
            evidence_version: 2,
            request_id: format!("reactor-callback:{}", self.task.task_digest()),
            plugin_id,
            capability: VerifierCapability::EventTraces,
            target: self.task.target.clone(),
            unit: self.task.unit.clone(),
            facts: Vec::new(),
            assumptions: self.task.assumptions.clone(),
            task_digest: self.task.task_digest(),
            obligations: self.task.obligations.clone(),
            source_effect: None,
            counter_effect: None,
            reactor_program: Some(self.import.source_input()),
            teardown_source: None,
            call_use_source: None,
            cleanup_replay_source: None,
            composition_source: None,
        }
    }

    /// Run the core's own reactor model checker over the core's own program.
    /// No provider input reaches it.
    pub fn evaluate(&self) -> Result<model::EventCertificate, Vec<model::EventDiagnostic>> {
        model::check_event_program(self.import.program(), self.import.accepted_profiles())
    }

    /// The assignment inventory the core exports from its own program.
    pub fn declared_proposal(&self) -> proposal_wire::ReactorCallbackProposal {
        declared_callback_proposal(self.import.program())
    }
}

impl<P: model::ReactorProfile> WitnessChecker for ReactorCallbackTask<P> {
    fn capability(&self) -> VerifierCapability {
        VerifierCapability::EventTraces
    }

    fn check(
        &self,
        task: &VerificationTask,
        witness: &SemanticWitness,
    ) -> Result<BTreeSet<String>, String> {
        if task != &self.task {
            return Err("reactor task differs from its checked program context".into());
        }
        let SemanticWitness::ReactorCallback(proposed) = witness else {
            return Err("expected reactor callback witness".into());
        };
        if proposed.task_digest != task.task_digest() {
            return Err("reactor witness is bound to a different task".into());
        }
        // Acceptance terminates in the core's own model run over the core's own
        // program. The proposal is bound to that program's complete assignment
        // inventory first: a missing, added, duplicated, retargeted or widened
        // record is refused before the model is consulted at all.
        check_declared_callback_proposal(
            self.import.program(),
            self.import.accepted_profiles(),
            &proposed.proposal,
        )
        .map_err(|error| match error {
            CallbackProposalError::Model(diagnostics) => format!(
                "the reactor model refused the core's own program: {}",
                diagnostics
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            other => format!("reactor witness differs from the checked assignments: {other:?}"),
        })?;
        // Both obligations come from one indivisible run: the coverage set is
        // the core's own derivation for this capability, never the provider's
        // claim. `task == self.task` above pins that derivation exactly.
        Ok(task.coverage_for(VerifierCapability::EventTraces))
    }
}
