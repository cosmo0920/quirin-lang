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

//! W3 core obligation derivation (trusted core).
//!
//! Implements `docs/design/W3_OBLIGATIONS_EVIDENCE_V2.md` §1.4. The core — not
//! a provider — derives the obligation inventory from the normalized unit, so
//! that a provider proposes witnesses and never "the list of obligations whose
//! absence would make its own proof succeed" (`docs/REEVALUATION_SPEC.md` §6).
//!
//! Derivation is itself fail-closed: any construct the deriver cannot classify
//! records a [`DerivationIncomplete`] marker on the task instead of being
//! silently omitted, and a capability carrying such a marker can never reach
//! `Safe`.
//!
//! Alongside the wire-visible inventory, derivation produces
//! [`CoreReplayInputs`] — the core-side data a witness is replayed against
//! (the lifecycle call inventory, the trusted contract sources). Those inputs
//! never travel to a provider, which is what makes the replay a recomputation
//! rather than an equality check against provider-supplied data.

use std::collections::{BTreeMap, BTreeSet};

use qcore_frontend::frontend::{DeclarationKind, SourceBlock, SourceProgram, StatementKind};
use qcore_frontend::lexer::{Span, Symbol, TokenKind, lex};
use qcore_schemas::digest::sha256_hex;
use qcore_schemas::obligation::{
    ChunkEntityRef, DerivationIncomplete, Obligation, ObligationProperty, ObligationScope,
    ObligationSubject, TaskBounds, VERIFICATION_TASK_VERSION, VerificationTask,
};
use qcore_schemas::plugin::{TargetProfile, UnitIdentity, VerifierCapability};
use qcore_schemas::reason::ReasonCategory;
use qcore_schemas::witness::{ChunkOperationRecord, SourceSite};

/// The chunk-lifecycle operation the core's API table pins for a call site.
///
/// Parameters the core cannot determine from the call itself (a chunk's
/// backend and residency at open, whether a close requests backing deletion)
/// are deliberately left to the provider: that is the bounded freedom the
/// certificate's claim is scoped to. Everything else — the operation kind and
/// every entity identity — is core-assigned and checked exactly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperationTemplate {
    CreateContext {
        context: u32,
    },
    CreateStream {
        context: u32,
        stream: u32,
    },
    /// Backend and residency are provider-chosen.
    OpenChunk {
        context: u32,
        stream: u32,
        chunk: u32,
    },
    SetResidencyUp {
        chunk: u32,
    },
    SetResidencyDown {
        chunk: u32,
    },
    Lock {
        chunk: u32,
    },
    Unlock {
        chunk: u32,
    },
    BeginTransaction {
        chunk: u32,
    },
    CommitTransaction {
        chunk: u32,
    },
    RollbackTransaction {
        chunk: u32,
    },
    /// The `delete` request flag is provider-chosen.
    CloseChunk {
        chunk: u32,
    },
    DestroyStream {
        stream: u32,
    },
    DestroyContext {
        context: u32,
    },
}

impl OperationTemplate {
    /// Accepts a proposed operation record only when its kind and every entity
    /// identity match what the core assigned to this site.
    #[must_use]
    pub fn accepts(self, proposed: ChunkOperationRecord) -> bool {
        let (kind, entities): (&str, Vec<(&str, u32)>) = match self {
            Self::CreateContext { context } => ("create_context", vec![("context", context)]),
            Self::CreateStream { context, stream } => (
                "create_stream",
                vec![("context", context), ("stream", stream)],
            ),
            Self::OpenChunk {
                context,
                stream,
                chunk,
            } => (
                "open_chunk",
                vec![("context", context), ("stream", stream), ("chunk", chunk)],
            ),
            Self::SetResidencyUp { chunk } => ("set_residency", vec![("chunk", chunk)]),
            Self::SetResidencyDown { chunk } => ("set_residency", vec![("chunk", chunk)]),
            Self::Lock { chunk } => ("lock", vec![("chunk", chunk)]),
            Self::Unlock { chunk } => ("unlock", vec![("chunk", chunk)]),
            Self::BeginTransaction { chunk } => ("begin_transaction", vec![("chunk", chunk)]),
            Self::CommitTransaction { chunk } => ("commit_transaction", vec![("chunk", chunk)]),
            Self::RollbackTransaction { chunk } => ("rollback_transaction", vec![("chunk", chunk)]),
            Self::CloseChunk { chunk } => ("close_chunk", vec![("chunk", chunk)]),
            Self::DestroyStream { stream } => ("destroy_stream", vec![("stream", stream)]),
            Self::DestroyContext { context } => ("destroy_context", vec![("context", context)]),
        };
        if proposed.kind() != kind || proposed.entities() != entities {
            return false;
        }
        // Residency direction is pinned by the callee, not by the provider.
        match (self, proposed) {
            (Self::SetResidencyUp { .. }, ChunkOperationRecord::SetResidency { residency, .. }) => {
                residency == qcore_schemas::witness::ChunkResidencyRecord::Up
            }
            (
                Self::SetResidencyDown { .. },
                ChunkOperationRecord::SetResidency { residency, .. },
            ) => residency == qcore_schemas::witness::ChunkResidencyRecord::Down,
            _ => true,
        }
    }
}

/// One core-discovered lifecycle call site with the operation the core's API
/// table maps it to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LifecycleSite {
    pub site: SourceSite,
    pub function: String,
    pub template: OperationTemplate,
}

/// The core's lifecycle call inventory for a unit: the denominator against
/// which a witness schedule's completeness is decided.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LifecycleInventory {
    /// Source order within the single analyzed function (the `SinglePath`
    /// bound). A witness schedule must present exactly this order.
    pub sites: Vec<LifecycleSite>,
    pub contexts: BTreeSet<u32>,
    pub streams: BTreeSet<u32>,
    pub chunks: BTreeSet<u32>,
    /// Chunks with at least one `BeginTransaction` site.
    pub transacted_chunks: BTreeSet<u32>,
}

/// The core's observation inventory: the trusted contract text a witness is
/// evaluated against, and the required-hook denominator for coverage.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ObservationInventory {
    /// Contract module path -> trusted source text (digest-pinned on the task).
    pub contract_sources: BTreeMap<String, String>,
    /// Required hook name -> (contract module, extern function).
    pub required_hooks: BTreeMap<String, (String, String)>,
}

/// Core-side replay inputs. These never travel to a provider.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CoreReplayInputs {
    pub lifecycle: Option<LifecycleInventory>,
    pub observation: Option<ObservationInventory>,
}

/// The trusted inputs a derivation runs over.
#[derive(Clone, Debug)]
pub struct DerivationInputs<'a> {
    pub translation_unit: &'a str,
    pub program: &'a SourceProgram,
    pub target: TargetProfile,
    /// Contract module path -> trusted source text.
    pub contract_sources: BTreeMap<String, String>,
    /// Hooks the provider's manifest declares it requires observations for.
    pub required_observation_hooks: BTreeSet<String>,
    /// The provider this inventory is being derived for, when there is one.
    ///
    /// E03-B3. Some obligations are about a *provider's* denominator rather
    /// than about a named program entity — `ObservationCoverageComplete` says
    /// "every hook this provider requires was checked" — and their subject
    /// (`Unit`) does not distinguish them. This field, together with the
    /// resolved hook denominator, is what makes their identifiers distinct, so
    /// one provider's certificate can never discharge another's. `None` means
    /// the derivation is not provider-scoped; the denominator alone then keys
    /// such obligations.
    pub verifier: Option<String>,
    /// Caller conditions are preserved. Observation derivation adds its required
    /// conditional trust flag only when absent; explicit false fails closed.
    pub assumptions: BTreeMap<String, bool>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DerivedTask {
    pub task: VerificationTask,
    pub replay: CoreReplayInputs,
}

/// The named checker profile for a capability. Part of the task identity, so
/// changing a derivation's semantics must change this string.
#[must_use]
pub const fn semantic_profile(capability: VerifierCapability) -> &'static str {
    match capability {
        VerifierCapability::ResourceContracts => "qcore.resource_contracts.v1",
        VerifierCapability::EventTraces => "qcore.event_traces.v1",
        VerifierCapability::ChunkLifecycle => "qcore.chunk_lifecycle.v1",
        VerifierCapability::Concurrency => "qcore.concurrency.v1",
        VerifierCapability::TranslationUnitProjection => "qcore.tu_projection.v1",
        VerifierCapability::ObservationHooks => "qcore.observation_hooks.v1",
    }
}

/// Derives the complete verification task for one `(unit, capability)` pair.
#[must_use]
pub fn derive_task(inputs: &DerivationInputs<'_>, capability: VerifierCapability) -> DerivedTask {
    let source = inputs.program.source();
    let mut task = VerificationTask {
        task_version: VERIFICATION_TASK_VERSION,
        unit: UnitIdentity {
            source_digest: sha256_hex(source.as_bytes()),
            // The QCore dispatch path compiles from source with no separate
            // build artifact; the build identity is the derivation identity.
            //
            // E03-B2. This digest is deliberately *not* keyed by the
            // capability. A unit identity names the artifact, and one source
            // program is one artifact however many capabilities are asked
            // about it; keying it by the semantic profile made two
            // capabilities of one program certify different units, so
            // `compose_verification` refused every multi-capability scope with
            // `QP015` and a legitimate composition was not representable.
            //
            // Nothing is lost by dropping it. The capability separation lives
            // where it is checked: `semantic_profile` and `capability` are
            // both hashed into every `obligation_id`, the whole task
            // (obligations included) is hashed into `task_digest`, and every
            // witness is bound to that digest by the firewall. A witness
            // minted for one capability's task therefore still cannot be
            // replayed against another's, and a real artifact difference —
            // different source bytes or a different translation unit — still
            // changes this digest and still refuses with `QP015`.
            build_digest: sha256_hex(
                format!("qcore-derivation/v1\u{1f}{}", inputs.translation_unit).as_bytes(),
            ),
            translation_unit: inputs.translation_unit.to_owned(),
        },
        target: inputs.target.clone(),
        semantic_profile: semantic_profile(capability).to_owned(),
        contract_sources: BTreeMap::new(),
        bounds: TaskBounds::single_path(),
        assumptions: inputs.assumptions.clone(),
        evidence_version: qcore_schemas::witness::PLUGIN_EVIDENCE_VERSION_V2,
        obligations: Vec::new(),
        derivation_incomplete: Vec::new(),
    };
    let mut replay = CoreReplayInputs::default();
    match capability {
        VerifierCapability::ChunkLifecycle => {
            derive_chunk_lifecycle(inputs, &mut task, &mut replay);
        }
        VerifierCapability::ObservationHooks => {
            derive_observation_hooks(inputs, &mut task, &mut replay);
        }
        VerifierCapability::ResourceContracts => derive_resource_contracts(inputs, &mut task),
        VerifierCapability::TranslationUnitProjection => derive_projection(inputs, &mut task),
        VerifierCapability::EventTraces | VerifierCapability::Concurrency => {
            // Obligation shapes are reserved in the schema, but no derivation
            // exists in this increment: fail closed rather than report an
            // honestly empty inventory (design §1.4.4).
            task.derivation_incomplete.push(DerivationIncomplete {
                capability,
                // The analyser has no derivation here; this says nothing about
                // the analysed source (§L21.1).
                category: ReasonCategory::IncompleteObligationCoverage,
                reason: "no core derivation for this capability in evidence v2.0 \
                         (event/concurrency model extraction is W5)"
                    .into(),
            });
        }
    }
    task.obligations.sort();
    task.derivation_incomplete.sort();
    DerivedTask { task, replay }
}

/// The identifier of an obligation whose subject does not name what it is
/// about (E03-B3).
///
/// `ObservationCoverageComplete` has subject `Unit`, so two providers over one
/// unit derived one identifier for two different claims: "every hook *I*
/// require was checked". It was sound only because `compose_verification`
/// requires every component to be `Safe`, so no provider's obligation was ever
/// covered by another's certificate alone — a coincidence of the merge rule,
/// not a property of the obligation.
///
/// The identifier is therefore keyed additionally by the core-computed
/// denominator the replay actually decides the obligation against: the
/// provider it was derived for and the resolved required-hook set. The base
/// identity is `VerificationTask::obligation_id`'s, so an unkeyed obligation's
/// bytes are unchanged; the separator makes the keyed pre-image unambiguous.
/// Nothing here is provider-supplied.
fn scoped_obligation_id(
    task: &VerificationTask,
    capability: VerifierCapability,
    property: ObligationProperty,
    subject: &ObligationSubject,
    denominator: &str,
) -> String {
    let base = task.obligation_id(capability, property, subject);
    format!(
        "{}:{}",
        capability.as_str(),
        &sha256_hex(format!("{base}\u{1f}{denominator}").as_bytes())[..24]
    )
}

fn push_obligation(
    task: &mut VerificationTask,
    capability: VerifierCapability,
    property: ObligationProperty,
    subject: ObligationSubject,
    scope: ObligationScope,
    bounds: TaskBounds,
) {
    let obligation_id = task.obligation_id(capability, property, &subject);
    push_identified_obligation(
        task,
        obligation_id,
        capability,
        property,
        subject,
        scope,
        bounds,
    );
}

fn push_identified_obligation(
    task: &mut VerificationTask,
    obligation_id: String,
    capability: VerifierCapability,
    property: ObligationProperty,
    subject: ObligationSubject,
    scope: ObligationScope,
    bounds: TaskBounds,
) {
    if task
        .obligations
        .iter()
        .any(|existing| existing.obligation_id == obligation_id)
    {
        return;
    }
    let evidence_version = task.evidence_version;
    let assumptions = task.assumptions.clone();
    task.obligations.push(Obligation {
        obligation_id,
        capability,
        property,
        subject,
        scope,
        bounds,
        assumptions,
        evidence_version,
    });
}

// ---------------------------------------------------------------------------
// Chunk lifecycle (design §1.4.2)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EntityKind {
    Context,
    Stream,
    Chunk,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ApiShape {
    CreateContext,
    CreateStream,
    OpenChunk,
    ResidencyUp,
    ResidencyDown,
    Lock,
    Unlock,
    BeginTransaction,
    CommitTransaction,
    RollbackTransaction,
    CloseChunk,
    DestroyStream,
    DestroyContext,
}

/// The core's lifecycle-relevant API table. A provider never extends it: a
/// callee outside this table is not a lifecycle site, and a lifecycle site
/// whose shape the core cannot resolve makes the derivation incomplete.
const LIFECYCLE_API: &[(&str, ApiShape)] = &[
    ("cio_ctx_create", ApiShape::CreateContext),
    ("cio_stream_create", ApiShape::CreateStream),
    ("cio_chunk_open", ApiShape::OpenChunk),
    ("cio_chunk_up", ApiShape::ResidencyUp),
    ("cio_chunk_down", ApiShape::ResidencyDown),
    ("cio_chunk_lock", ApiShape::Lock),
    ("cio_chunk_unlock", ApiShape::Unlock),
    ("cio_chunk_tx_begin", ApiShape::BeginTransaction),
    ("cio_chunk_tx_commit", ApiShape::CommitTransaction),
    ("cio_chunk_tx_rollback", ApiShape::RollbackTransaction),
    ("cio_chunk_close", ApiShape::CloseChunk),
    ("cio_stream_destroy", ApiShape::DestroyStream),
    ("cio_ctx_destroy", ApiShape::DestroyContext),
];

fn lifecycle_shape(callee: &str) -> Option<ApiShape> {
    LIFECYCLE_API
        .iter()
        .find(|(name, _)| *name == callee)
        .map(|(_, shape)| *shape)
}

/// One statement's call, as recovered from the retained token stream.
struct StatementCall {
    callee: String,
    arguments: Vec<String>,
    span: Span,
}

fn statement_calls(source: &str, span: Span) -> Option<Vec<StatementCall>> {
    // Recognize a whole statement, never fish a familiar call out of an
    // arbitrary expression (nested calls, short-circuiting and aliases all
    // need an effect importer before they can contribute model operations).
    let text = source.get(span.start..span.end)?;
    let tokens = lex(text).ok()?;
    let tokens = tokens
        .iter()
        .filter(|t| t.kind != TokenKind::Eof)
        .collect::<Vec<_>>();
    if tokens.len() < 4 {
        return None;
    }
    let mut index = 0;
    if matches!(&tokens[0].kind, TokenKind::Word(word) if word == "let") {
        if !matches!(&tokens.get(1)?.kind, TokenKind::Word(_))
            || tokens.get(2)?.kind != TokenKind::Symbol(Symbol::Equal)
        {
            return None;
        }
        index = 3;
    }
    let TokenKind::Word(callee) = &tokens.get(index)?.kind else {
        return None;
    };
    if tokens.get(index + 1)?.kind != TokenKind::Symbol(Symbol::LeftParen) {
        return None;
    }
    let mut arguments = Vec::new();
    let mut cursor = index + 2;
    if tokens.get(cursor)?.kind != TokenKind::Symbol(Symbol::RightParen) {
        loop {
            let TokenKind::Word(argument) = &tokens.get(cursor)?.kind else {
                return None;
            };
            arguments.push(argument.clone());
            cursor += 1;
            if tokens.get(cursor)?.kind == TokenKind::Symbol(Symbol::RightParen) {
                break;
            }
            if tokens.get(cursor)?.kind != TokenKind::Symbol(Symbol::Comma) {
                return None;
            }
            cursor += 1;
        }
    }
    if tokens.get(cursor + 1)?.kind != TokenKind::Symbol(Symbol::Semicolon)
        || cursor + 2 != tokens.len()
    {
        return None;
    }
    Some(vec![StatementCall {
        callee: callee.clone(),
        arguments,
        span: Span {
            start: span.start + tokens[index].span.start,
            end: span.start + tokens[cursor].span.end,
        },
    }])
}

struct LifecycleScan {
    inventory: LifecycleInventory,
    /// Each gap carries its closed L21.1 category at the point it is recorded.
    /// The category is never recovered from the prose afterwards.
    incomplete: Vec<(ReasonCategory, String)>,
    bindings: BTreeMap<String, (EntityKind, u32)>,
    parents: BTreeMap<u32, (u32, u32)>,
}

impl LifecycleScan {
    /// Record a gap whose subject is a construct in the analysed source
    /// (§L21.1 `unsupported-syntax`).
    fn unsupported_syntax(&mut self, reason: String) {
        self.incomplete
            .push((ReasonCategory::UnsupportedSyntax, reason));
    }

    /// Record a gap whose subject is this increment's own coverage bound, not
    /// anything in the analysed source (§L21.1 `incomplete-obligation-coverage`).
    fn incomplete_coverage(&mut self, reason: String) {
        self.incomplete
            .push((ReasonCategory::IncompleteObligationCoverage, reason));
    }

    fn resolve(&mut self, name: Option<&String>, expected: EntityKind) -> Option<u32> {
        match name.and_then(|name| self.bindings.get(name)) {
            Some(&(kind, id)) if kind == expected => Some(id),
            _ => None,
        }
    }
}

fn scan_block(
    scan: &mut LifecycleScan,
    source: &str,
    function: &str,
    unit: &str,
    block: &SourceBlock,
) {
    for (position, statement) in block.statements.iter().enumerate() {
        if !statement.value.nested_blocks().is_empty() {
            scan.unsupported_syntax(format!(
                "`{function}` contains structured control flow; the evidence-v2.0 \
                 schedule bound is a single path per function"
            ));
            continue;
        }
        let statement_text = source[statement.span.start..statement.span.end].trim();
        if statement_text == "return;" {
            if position + 1 != block.statements.len() {
                scan.unsupported_syntax(format!("`{function}` has operations after a return"));
            }
            break;
        }
        let Some(calls) = statement_calls(source, statement.span) else {
            scan.unsupported_syntax(format!(
                "`{function}` has a statement whose call structure could not be recovered"
            ));
            continue;
        };
        let binding = match &statement.value {
            StatementKind::Let {
                binding: Some(name),
            }
            | StatementKind::Discharge {
                binding: Some(name),
            } => Some(name.clone()),
            _ => None,
        };
        for call in calls {
            let Some(shape) = lifecycle_shape(&call.callee) else {
                scan.unsupported_syntax(format!(
                    "`{function}` has unknown call effect `{}`",
                    call.callee
                ));
                continue;
            };
            let arity = match shape {
                ApiShape::CreateContext => 0,
                ApiShape::OpenChunk => 2,
                _ => 1,
            };
            if call.arguments.len() != arity {
                scan.unsupported_syntax(format!(
                    "`{function}` has unsupported model-call arity for `{}`",
                    call.callee
                ));
                continue;
            }
            if binding
                .as_ref()
                .is_some_and(|name| scan.bindings.contains_key(name))
            {
                scan.unsupported_syntax(format!("`{function}` shadows a lifecycle identity"));
                continue;
            }
            let template = classify(scan, shape, &call, binding.as_ref(), function);
            let Some(template) = template else {
                continue;
            };
            let argument_identity = match template {
                OperationTemplate::CreateContext { context }
                | OperationTemplate::DestroyContext { context } => context,
                OperationTemplate::CreateStream { stream, .. }
                | OperationTemplate::DestroyStream { stream } => stream,
                OperationTemplate::OpenChunk { chunk, .. }
                | OperationTemplate::SetResidencyUp { chunk }
                | OperationTemplate::SetResidencyDown { chunk }
                | OperationTemplate::Lock { chunk }
                | OperationTemplate::Unlock { chunk }
                | OperationTemplate::BeginTransaction { chunk }
                | OperationTemplate::CommitTransaction { chunk }
                | OperationTemplate::RollbackTransaction { chunk }
                | OperationTemplate::CloseChunk { chunk } => chunk,
            };
            scan.inventory.sites.push(LifecycleSite {
                site: SourceSite {
                    file: unit.to_owned(),
                    span_start: call.span.start as u64,
                    span_end: call.span.end as u64,
                    callee: call.callee.clone(),
                    argument_identity,
                },
                function: function.to_owned(),
                template,
            });
        }
    }
}

fn classify(
    scan: &mut LifecycleScan,
    shape: ApiShape,
    call: &StatementCall,
    binding: Option<&String>,
    function: &str,
) -> Option<OperationTemplate> {
    let unresolved = |scan: &mut LifecycleScan, detail: &str| {
        scan.unsupported_syntax(format!(
            "`{function}` calls `{}`: {detail}",
            call.callee.clone()
        ));
        None::<OperationTemplate>
    };
    match shape {
        ApiShape::CreateContext => {
            let Some(binding) = binding else {
                return unresolved(scan, "a created context is not bound to a name");
            };
            let id = scan.inventory.contexts.len() as u32;
            scan.inventory.contexts.insert(id);
            scan.bindings
                .insert(binding.clone(), (EntityKind::Context, id));
            Some(OperationTemplate::CreateContext { context: id })
        }
        ApiShape::CreateStream => {
            let Some(binding) = binding else {
                return unresolved(scan, "a created stream is not bound to a name");
            };
            let Some(context) = scan.resolve(call.arguments.first(), EntityKind::Context) else {
                return unresolved(scan, "its context argument does not name a created context");
            };
            let id = scan.inventory.streams.len() as u32;
            scan.inventory.streams.insert(id);
            scan.bindings
                .insert(binding.clone(), (EntityKind::Stream, id));
            scan.parents.insert(id, (context, id));
            Some(OperationTemplate::CreateStream {
                context,
                stream: id,
            })
        }
        ApiShape::OpenChunk => {
            let Some(binding) = binding else {
                return unresolved(scan, "an opened chunk is not bound to a name");
            };
            let Some(context) = scan.resolve(call.arguments.first(), EntityKind::Context) else {
                return unresolved(scan, "its context argument does not name a created context");
            };
            let Some(stream) = scan.resolve(call.arguments.get(1), EntityKind::Stream) else {
                return unresolved(scan, "its stream argument does not name a created stream");
            };
            let id = scan.inventory.chunks.len() as u32;
            scan.inventory.chunks.insert(id);
            scan.bindings
                .insert(binding.clone(), (EntityKind::Chunk, id));
            Some(OperationTemplate::OpenChunk {
                context,
                stream,
                chunk: id,
            })
        }
        ApiShape::DestroyContext => match scan.resolve(call.arguments.first(), EntityKind::Context)
        {
            Some(context) => Some(OperationTemplate::DestroyContext { context }),
            None => unresolved(scan, "its argument does not name a created context"),
        },
        ApiShape::DestroyStream => match scan.resolve(call.arguments.first(), EntityKind::Stream) {
            Some(stream) => Some(OperationTemplate::DestroyStream { stream }),
            None => unresolved(scan, "its argument does not name a created stream"),
        },
        _ => {
            let Some(chunk) = scan.resolve(call.arguments.first(), EntityKind::Chunk) else {
                return unresolved(scan, "its argument does not name an opened chunk");
            };
            Some(match shape {
                ApiShape::ResidencyUp => OperationTemplate::SetResidencyUp { chunk },
                ApiShape::ResidencyDown => OperationTemplate::SetResidencyDown { chunk },
                ApiShape::Lock => OperationTemplate::Lock { chunk },
                ApiShape::Unlock => OperationTemplate::Unlock { chunk },
                ApiShape::BeginTransaction => {
                    scan.inventory.transacted_chunks.insert(chunk);
                    OperationTemplate::BeginTransaction { chunk }
                }
                ApiShape::CommitTransaction => OperationTemplate::CommitTransaction { chunk },
                ApiShape::RollbackTransaction => OperationTemplate::RollbackTransaction { chunk },
                ApiShape::CloseChunk => OperationTemplate::CloseChunk { chunk },
                _ => unreachable!("context/stream shapes handled above"),
            })
        }
    }
}

fn derive_chunk_lifecycle(
    inputs: &DerivationInputs<'_>,
    task: &mut VerificationTask,
    replay: &mut CoreReplayInputs,
) {
    let capability = VerifierCapability::ChunkLifecycle;
    let source = inputs.program.source();
    let mut scan = LifecycleScan {
        inventory: LifecycleInventory::default(),
        incomplete: Vec::new(),
        bindings: BTreeMap::new(),
        parents: BTreeMap::new(),
    };
    let mut functions_with_sites = BTreeSet::new();
    for declaration in &inputs.program.declarations {
        if declaration.value.kind != DeclarationKind::Function {
            continue;
        }
        let Some(body) = &declaration.value.body else {
            continue;
        };
        scan.bindings.clear();
        let before = scan.inventory.sites.len();
        scan_block(
            &mut scan,
            source,
            &declaration.value.name,
            inputs.translation_unit,
            body,
        );
        if scan.inventory.sites.len() != before {
            functions_with_sites.insert(declaration.value.name.clone());
        }
    }
    if functions_with_sites.len() > 1 {
        // Cross-function order admissibility needs the inlined call-structure
        // linearization; the v2.0 bound is a single analyzed function.
        scan.incomplete_coverage(format!(
            "lifecycle call sites appear in {} functions; the evidence-v2.0 order bound \
             covers a single analyzed function",
            functions_with_sites.len()
        ));
    }
    // This syntax inventory is useful for replay conformance, but spelling
    // alone does not import a real API's fallibility, backend, residency,
    // deletion flags or symbol binding. Do not mint source-safety certificates.
    scan.incomplete_coverage("source lifecycle effect/ABI import is unavailable; model-call spelling does not establish runtime outcomes or provider-chosen parameters".into());
    for (category, reason) in &scan.incomplete {
        task.derivation_incomplete.push(DerivationIncomplete {
            capability,
            category: *category,
            reason: reason.clone(),
        });
    }
    let scope = if functions_with_sites.is_empty() {
        ObligationScope::WholeUnit
    } else {
        ObligationScope::Functions {
            symbols: functions_with_sites.clone(),
        }
    };
    let bounds = TaskBounds::single_path();
    for &context in &scan.inventory.contexts {
        push_obligation(
            task,
            capability,
            ObligationProperty::ContextDischarged,
            ObligationSubject::ChunkEntity {
                entity: ChunkEntityRef::Context { id: context },
            },
            scope.clone(),
            bounds.clone(),
        );
    }
    for &stream in &scan.inventory.streams {
        push_obligation(
            task,
            capability,
            ObligationProperty::StreamDischarged,
            ObligationSubject::ChunkEntity {
                entity: ChunkEntityRef::Stream { id: stream },
            },
            scope.clone(),
            bounds.clone(),
        );
    }
    for &chunk in &scan.inventory.chunks {
        push_obligation(
            task,
            capability,
            ObligationProperty::ChunkDischarged,
            ObligationSubject::ChunkEntity {
                entity: ChunkEntityRef::Chunk { id: chunk },
            },
            scope.clone(),
            bounds.clone(),
        );
    }
    for &chunk in &scan.inventory.transacted_chunks {
        push_obligation(
            task,
            capability,
            ObligationProperty::TransactionSettled,
            ObligationSubject::ChunkEntity {
                entity: ChunkEntityRef::Chunk { id: chunk },
            },
            scope.clone(),
            bounds.clone(),
        );
    }
    if !scan.inventory.sites.is_empty() {
        push_obligation(
            task,
            capability,
            ObligationProperty::LifecycleProgramComplete,
            ObligationSubject::Unit,
            scope,
            bounds,
        );
    }
    replay.lifecycle = Some(scan.inventory);
}

// ---------------------------------------------------------------------------
// Contract observations (design §1.4.3)
// ---------------------------------------------------------------------------

fn derive_observation_hooks(
    inputs: &DerivationInputs<'_>,
    task: &mut VerificationTask,
    replay: &mut CoreReplayInputs,
) {
    let capability = VerifierCapability::ObservationHooks;
    // Recorded observations are a declared trusted assumption (plan §2.6), so
    // the assumption is named on the task and echoed by every certificate
    // rather than being laundered into an unqualified `Safe`.
    // An omitted flag requests the design's conditional observation claim;
    // an explicit refusal must never be rewritten into permission to trust.
    let trusted = *task
        .assumptions
        .entry("observations_trusted".into())
        .or_insert(true);
    if !trusted {
        task.derivation_incomplete.push(DerivationIncomplete {
            capability,
            // A declared premise the analysis requires was refused by the
            // caller: §L21.1 `missing-premise`, not a statement about source.
            category: ReasonCategory::MissingPremise,
            reason: "caller refuses the observations_trusted assumption required for recorded observation replay".into(),
        });
    }
    task.bounds = TaskBounds::observed_runs();
    let mut inventory = ObservationInventory {
        contract_sources: inputs.contract_sources.clone(),
        required_hooks: BTreeMap::new(),
    };
    for (path, text) in &inputs.contract_sources {
        task.contract_sources
            .insert(path.clone(), sha256_hex(text.as_bytes()));
    }
    // The hook name of an `extern fn` contract is its function name: it is the
    // identity the core evaluates predicates for, so a provider cannot rename
    // a hook onto a weaker contract.
    let mut declared = BTreeMap::<String, String>::new();
    for (path, text) in &inputs.contract_sources {
        let Ok(module) = qcore_frontend::frontend::parse_source_program(text) else {
            task.derivation_incomplete.push(DerivationIncomplete {
                capability,
                // The unparseable text is a *trusted contract input* of this
                // task, not the analysed source: §L21.1 `missing-premise`.
                category: ReasonCategory::MissingPremise,
                reason: format!("contract module `{path}` does not parse"),
            });
            continue;
        };
        for declaration in &module.declarations {
            if declaration.value.kind == DeclarationKind::ExternFunction
                && declaration.value.function_contract.is_some()
                && declared
                    .insert(declaration.value.name.clone(), path.clone())
                    .is_some()
            {
                task.derivation_incomplete.push(DerivationIncomplete {
                    capability,
                    // Again the declared contract, not the analysed source.
                    category: ReasonCategory::MissingPremise,
                    reason: format!(
                        "ambiguous observation contract `{}`",
                        declaration.value.name
                    ),
                });
            }
        }
    }
    for hook in &inputs.required_observation_hooks {
        let Some(module) = declared.get(hook) else {
            task.derivation_incomplete.push(DerivationIncomplete {
                capability,
                // §L21.1 lists this reason verbatim under `missing-premise`.
                category: ReasonCategory::MissingPremise,
                reason: format!(
                    "required observation hook `{hook}` has no `extern fn` contract in the \
                     unit's trusted contract sources"
                ),
            });
            continue;
        };
        inventory
            .required_hooks
            .insert(hook.clone(), (module.clone(), hook.clone()));
        push_obligation(
            task,
            capability,
            ObligationProperty::ObservationChecked,
            ObligationSubject::ObservedContract {
                function: hook.clone(),
                contract_module: module.clone(),
            },
            // Empty is the v2 deferred-run scope marker; exact checked run
            // identities remain in the certificate's replayed witness.
            ObligationScope::ObservedRuns {
                run_ids: BTreeSet::new(),
            },
            TaskBounds::observed_runs(),
        );
    }
    if !inventory.required_hooks.is_empty() {
        // E03-B3. The claim is "every hook of *this* denominator was checked",
        // so the identifier is keyed by the provider and by the resolved hook
        // set the replay decides it against — the same `required_hooks` map
        // `ContractObservationReplay` quantifies over. Two providers over one
        // unit now derive two obligations, and neither certificate can
        // discharge the other's.
        let denominator = format!(
            "observation-coverage/v1\u{1f}{}\u{1f}{}",
            inputs.verifier.as_deref().unwrap_or(""),
            inventory
                .required_hooks
                .keys()
                .cloned()
                .collect::<Vec<_>>()
                .join("\u{1e}")
        );
        let subject = ObligationSubject::Unit;
        let obligation_id = scoped_obligation_id(
            task,
            capability,
            ObligationProperty::ObservationCoverageComplete,
            &subject,
            &denominator,
        );
        push_identified_obligation(
            task,
            obligation_id,
            capability,
            ObligationProperty::ObservationCoverageComplete,
            subject,
            ObligationScope::ObservedRuns {
                run_ids: BTreeSet::new(),
            },
            TaskBounds::observed_runs(),
        );
    }
    replay.observation = Some(inventory);
}

// ---------------------------------------------------------------------------
// Resource contracts and projection (design §1.4.1 / §1.4.5)
// ---------------------------------------------------------------------------

fn derive_resource_contracts(inputs: &DerivationInputs<'_>, task: &mut VerificationTask) {
    let capability = VerifierCapability::ResourceContracts;
    let unit = inputs.translation_unit.to_owned();
    for declaration in &inputs.program.declarations {
        if declaration.value.kind != DeclarationKind::Function {
            continue;
        }
        push_obligation(
            task,
            capability,
            ObligationProperty::NoUnknownCallEffect,
            ObligationSubject::Function {
                symbol: declaration.value.name.clone(),
                translation_unit: unit.clone(),
            },
            ObligationScope::Functions {
                symbols: BTreeSet::from([declaration.value.name.clone()]),
            },
            TaskBounds::single_path(),
        );
        for (index, parameter) in declaration.value.parameters.iter().enumerate() {
            if parameter.usage.is_none() {
                continue;
            }
            push_obligation(
                task,
                capability,
                ObligationProperty::ParameterModeRespected,
                ObligationSubject::AllocationSite {
                    function: declaration.value.name.clone(),
                    site_index: index as u32,
                    resource_kind: parameter.type_source.clone(),
                },
                ObligationScope::Functions {
                    symbols: BTreeSet::from([declaration.value.name.clone()]),
                },
                TaskBounds::single_path(),
            );
        }
    }
    // Contracted acquire sites are derived by the unrestricted-C gateway over
    // normalized CFGs, which the QCore verify-scope path does not run. Record
    // the gap: the inventory is honest, but it is not complete, so this
    // capability can never reach `Safe` through it. Discharge is W4 work.
    task.derivation_incomplete.push(DerivationIncomplete {
        capability,
        // §L21.1 lists this reason verbatim under
        // `incomplete-obligation-coverage`.
        category: ReasonCategory::IncompleteObligationCoverage,
        reason: "per-allocation-site custody derivation runs in the unrestricted-C gateway; \
                 the QCore verify-scope path derives call-effect and parameter-mode \
                 obligations only (gateway summaries are not imported here)"
            .into(),
    });
}

fn derive_projection(inputs: &DerivationInputs<'_>, task: &mut VerificationTask) {
    // Projection coverage is not safety coverage: discharging this obligation
    // moves nothing out of the `Unknown` denominator.
    push_obligation(
        task,
        VerifierCapability::TranslationUnitProjection,
        ObligationProperty::ProjectionAccounted,
        ObligationSubject::Unit,
        ObligationScope::Functions {
            symbols: inputs
                .program
                .declarations
                .iter()
                .filter(|declaration| declaration.value.kind == DeclarationKind::Function)
                .map(|declaration| declaration.value.name.clone())
                .collect(),
        },
        TaskBounds::single_path(),
    );
}

#[cfg(test)]
#[path = "../tests/unit/obligation_derivation.rs"]
mod tests;
