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

//! Card **W48-RK-LOG-CALLER-FRAME** (gateway rule 32): the caller-frame
//! projection of a reviewed `logger-effects-v1` model, and the check that a
//! caller-facing declaration states exactly the premises it derives.
//!
//! A caller-frame declaration borrows every formal (and its variadic tail),
//! returns nothing and has no other clause, so a caller reads the call as
//! the identity on its own custody. That reading is sound only if every
//! effect the callee may have on a returning trace is invisible from the
//! caller's frame, or is covered by a premise the declaration names. The
//! projection document says, for each event of the model and for each
//! obligation that event raises, which of the two it is:
//!
//! * `internal`, with the argument that keeps it out of the caller's frame
//!   ([`Argument`]): the object is fresh, the read is through a borrowed
//!   formal, the state is the library's own, the queue is reached only
//!   through a borrowed formal, the effect happens after the frame ends, only
//!   non-returning traces are affected, or it is trust boundary 28. The
//!   obligation stays **Open**: nothing here proves it, it is only not the
//!   caller's.
//! * `premise`, naming the premise that covers a caller-visible effect.
//!
//! Each path's standing is **derived**, never trusted: a path holding an
//! event that a fresh object makes infeasible is `infeasible`; a path holding
//! the frame boundary event is `outside-frame` and must name the in-frame path
//! its prefix equals; every other path is `proved` when its in-frame events
//! (and the frame's own obligations) need no premise and `premised` with
//! exactly the premises they name. The document states each standing for
//! review, and a stated standing that differs from the derived one refuses.
//! The caller premise set is the union over the in-frame paths, and every
//! declaration the projection names must state exactly that set, by name, no
//! more and no fewer ([`check_declarations`]).
//!
//! Total-or-reject: a model or projection whose digest is not the
//! independently held one, an unclassified or invented obligation or event,
//! a path the model does not have (or one it has that the projection omits),
//! an internal argument outside [`Argument`], an outside-frame path whose
//! prefix is not an in-frame path, and a declaration that is not
//! caller-frame shaped all refuse. No obligation is ever discharged:
//! [`CheckedCallerFrame::open_obligations`] is the model's whole set. Lean:
//! `QCore.RkLogCallerFrame`.
use crate::logger_effects::{Event, Plan};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// The projection document's version.
pub const VERSION: &str = "caller-frame-v1";

/// Why an obligation is not visible from the caller's frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Argument {
    /// The object was created by the call itself (zero-initialised): the caller
    /// cannot hold it.
    FreshObject,
    /// A read through a formal the declaration borrows; its liveness is the
    /// caller's own borrowed-position obligation, checked at the call site.
    BorrowedRead,
    /// The library's own state (thread-local or static), never a caller's.
    LibraryState,
    /// Queue state reached only through a borrowed formal; no caller custody
    /// rests on it.
    QueueInternal,
    /// The effect happens after the call has returned (a later consumer).
    AfterFrame,
    /// Only a non-returning trace is affected; a normal exit carries no claim
    /// about it.
    NormalExitOnly,
    /// Trust boundary 28 (faithful extraction), which every verdict rests on.
    TrustBoundary,
}

/// An internal obligation: not caller-visible, for `internal`'s reason.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Internal {
    pub internal: Argument,
    pub reason: String,
}

/// A caller-visible obligation, covered by the named premise.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Premised {
    pub premise: String,
    pub reason: String,
}

/// One obligation's class at one event (or in the frame).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Class {
    /// Not caller-visible.
    Internal(Internal),
    /// Caller-visible, covered by the named premise.
    Premise(Premised),
}

/// Where an event stands relative to the caller's frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Frame {
    /// The event happens during the call.
    In,
    /// The frame ends here: this event and every later one are a consumer's.
    Boundary,
    /// A fresh object makes the event impossible during the call.
    Infeasible,
}

/// One event kind's projection.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventProjection {
    pub frame: Frame,
    pub reason: String,
    pub obligations: BTreeMap<String, Class>,
}

/// A path's standing, as the document states it and as it is derived.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields, tag = "standing", rename_all = "kebab-case")]
pub enum Standing {
    /// Every in-frame obligation is internal.
    Proved,
    /// Caller-visible effects, each covered by one of these premises.
    Premised { premises: BTreeSet<String> },
    /// Impossible for the call's fresh objects.
    Infeasible,
    /// The prefix before the boundary event is this in-frame path.
    OutsideFrame { prefix: String },
}

/// The projection document (`caller-frame-v1`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Projection {
    pub version: String,
    pub model_sha256: String,
    pub root: String,
    pub declarations: Vec<String>,
    /// Obligations every returning trace raises before any event (the root's
    /// own formatting, configuration and trust boundary).
    pub frame: BTreeMap<String, Class>,
    pub events: BTreeMap<String, EventProjection>,
    pub paths: BTreeMap<String, Standing>,
}

/// The checked projection. No `Deserialize`: only [`check`] makes one.
#[derive(Debug, PartialEq, Eq)]
pub struct CheckedCallerFrame {
    model_digest: String,
    declarations: Vec<String>,
    premises: BTreeSet<String>,
    paths: BTreeMap<String, Standing>,
    open_obligations: BTreeSet<String>,
}

impl CheckedCallerFrame {
    /// The digest of the model the projection was checked against.
    #[must_use]
    pub fn model_digest(&self) -> &str {
        &self.model_digest
    }
    /// The declarations that must state [`Self::premises`].
    #[must_use]
    pub fn declarations(&self) -> &[String] {
        &self.declarations
    }
    /// The caller premise set: the union over the in-frame paths.
    #[must_use]
    pub fn premises(&self) -> &BTreeSet<String> {
        &self.premises
    }
    /// Each path's derived standing.
    #[must_use]
    pub fn paths(&self) -> &BTreeMap<String, Standing> {
        &self.paths
    }
    /// Every obligation of the model: none is discharged here.
    #[must_use]
    pub fn open_obligations(&self) -> &BTreeSet<String> {
        &self.open_obligations
    }
}

fn event_name(event: Event) -> &'static str {
    match event {
        Event::CreateOp => "create-op",
        Event::CreateText => "create-text",
        Event::Forward => "forward",
        Event::Reply => "reply",
        Event::Transfer => "transfer",
        Event::Take => "take",
        Event::Callback => "callback",
        Event::DestroyText => "destroy-text",
        Event::DestroyOp => "destroy-op",
    }
}

fn identifier(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
}

fn reason(text: &str) -> bool {
    !text.trim().is_empty() && text.len() <= 4096 && !text.chars().any(char::is_control)
}

fn class_premise(class: &Class) -> Result<Option<&str>, String> {
    match class {
        Class::Internal(Internal { reason: why, .. }) if reason(why) => Ok(None),
        Class::Premise(Premised {
            premise,
            reason: why,
        }) if identifier(premise) && reason(why) => Ok(Some(premise)),
        _ => Err("caller-frame: malformed obligation class".into()),
    }
}

/// Checks `projection` against `model`, each against its independently held
/// digest. The model is parsed exactly as `logger_effects` reads it; its
/// source binding is that module's (and its replay example's) concern, so the
/// projection is bound to the model only through `model_sha256`.
pub fn check(
    model: &[u8],
    expected_model: &str,
    projection: &[u8],
    expected_projection: &str,
) -> Result<CheckedCallerFrame, String> {
    if model.len() > 131_072 || qcore_schemas::sha256_hex(model) != expected_model {
        return Err("caller-frame: model digest or byte bound".into());
    }
    if projection.len() > 131_072 || qcore_schemas::sha256_hex(projection) != expected_projection {
        return Err("caller-frame: projection digest or byte bound".into());
    }
    let plan: Plan =
        serde_json::from_slice(model).map_err(|e| format!("caller-frame: model: {e}"))?;
    let doc: Projection =
        serde_json::from_slice(projection).map_err(|e| format!("caller-frame: {e}"))?;
    if plan.version != "logger-effects-v1" {
        return Err("caller-frame: model version".into());
    }
    if doc.version != VERSION || doc.model_sha256 != expected_model {
        return Err("caller-frame: projection version or model binding".into());
    }
    if doc.root != plan.root || !doc.declarations.contains(&doc.root) {
        return Err("caller-frame: root".into());
    }
    let named: BTreeSet<&String> = doc.declarations.iter().collect();
    if named.len() != doc.declarations.len()
        || doc.declarations.is_empty()
        || doc.declarations.len() > 16
        || doc.declarations.iter().any(|d| !identifier(d))
    {
        return Err("caller-frame: declarations".into());
    }
    // The events the model's paths use, exactly.
    let used: BTreeSet<&'static str> = plan
        .paths
        .values()
        .flat_map(|p| p.events.iter().map(|e| event_name(*e)))
        .collect();
    if doc
        .events
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>()
        != used
    {
        return Err("caller-frame: classified events differ from the model's".into());
    }
    if doc.paths.keys().ne(plan.paths.keys()) {
        return Err("caller-frame: paths differ from the model's".into());
    }
    // Every model obligation is projected somewhere; none is invented.
    let mut projected = BTreeSet::new();
    let mut frame_premises = BTreeSet::new();
    for (obligation, class) in &doc.frame {
        projected.insert(obligation.clone());
        if let Some(p) = class_premise(class)? {
            frame_premises.insert(p.to_owned());
        }
    }
    let mut event_premises: BTreeMap<&str, BTreeSet<String>> = BTreeMap::new();
    for (event, projection) in &doc.events {
        if !reason(&projection.reason) || projection.obligations.is_empty() {
            return Err(format!("caller-frame: event `{event}` unexplained"));
        }
        let mut premises = BTreeSet::new();
        for (obligation, class) in &projection.obligations {
            projected.insert(obligation.clone());
            let premise = class_premise(class)?;
            match (projection.frame, premise, class) {
                (Frame::In, Some(p), _) => {
                    premises.insert(p.to_owned());
                }
                (Frame::In, None, _) => {}
                // A boundary event's obligations are all after the frame.
                (
                    Frame::Boundary,
                    None,
                    Class::Internal(Internal {
                        internal: Argument::AfterFrame,
                        ..
                    }),
                ) => {}
                // An infeasible event is impossible for a fresh object.
                (
                    Frame::Infeasible,
                    None,
                    Class::Internal(Internal {
                        internal: Argument::FreshObject,
                        ..
                    }),
                ) => {}
                _ => {
                    return Err(format!(
                        "caller-frame: event `{event}` class does not fit its frame standing"
                    ));
                }
            }
        }
        event_premises.insert(event.as_str(), premises);
    }
    if projected.iter().ne(plan.obligations.keys()) {
        return Err("caller-frame: projected obligations differ from the model's".into());
    }
    let frame_of = |event: Event| doc.events[event_name(event)].frame;
    // Derive every in-frame path's standing first; outside-frame ones then
    // name one of them.
    let mut derived = BTreeMap::new();
    let mut in_frame_events = BTreeMap::new();
    for (name, path) in &plan.paths {
        if path
            .events
            .iter()
            .any(|e| frame_of(*e) == Frame::Infeasible)
        {
            derived.insert(name.clone(), Standing::Infeasible);
            continue;
        }
        let cut = path
            .events
            .iter()
            .position(|e| frame_of(*e) == Frame::Boundary);
        let inside = &path.events[..cut.unwrap_or(path.events.len())];
        if cut.is_some() {
            in_frame_events.insert(name.clone(), (inside.to_vec(), true));
            continue;
        }
        in_frame_events.insert(name.clone(), (inside.to_vec(), false));
        let mut premises = frame_premises.clone();
        for event in inside {
            premises.extend(event_premises[event_name(*event)].iter().cloned());
        }
        derived.insert(
            name.clone(),
            if premises.is_empty() {
                Standing::Proved
            } else {
                Standing::Premised { premises }
            },
        );
    }
    for (name, stated) in &doc.paths {
        let Some((inside, true)) = in_frame_events.get(name) else {
            continue;
        };
        let Standing::OutsideFrame { prefix } = stated else {
            return Err(format!("caller-frame: path `{name}` leaves the frame"));
        };
        match (in_frame_events.get(prefix), derived.get(prefix)) {
            (Some((events, false)), Some(Standing::Proved | Standing::Premised { .. }))
                if events == inside => {}
            _ => {
                return Err(format!(
                    "caller-frame: path `{name}`'s in-frame prefix is not path `{prefix}`"
                ));
            }
        }
        derived.insert(name.clone(), stated.clone());
    }
    if derived != doc.paths {
        return Err("caller-frame: a stated path standing differs from the derived one".into());
    }
    let premises = derived
        .values()
        .filter_map(|s| match s {
            Standing::Premised { premises } => Some(premises.iter().cloned()),
            _ => None,
        })
        .flatten()
        .chain(frame_premises)
        .collect();
    Ok(CheckedCallerFrame {
        model_digest: expected_model.into(),
        declarations: doc.declarations,
        premises,
        paths: derived,
        open_obligations: plan.obligations.keys().cloned().collect(),
    })
}

/// The `requires` clauses a caller-frame declaration may carry besides its
/// premises: the vocabulary marker and the premise-decomposition metadata
/// (which states hypotheses about a premise, never a custody effect).
const METADATA_REQUIRES: [&str; 4] = [
    "resource_hypothesis(",
    "resource_lifecycle_v1(",
    "resource_premise_decomposition(",
    "resource_premise_flow_v1(",
];

/// Checks that every declaration `checked` names is in `source` (one
/// `module contract;` text), is caller-frame shaped, and states exactly
/// `checked.premises()` as its declared premises. Shape: in the loaded
/// resource environment every parameter is `Borrowed`, the result is `None`,
/// with no kind, container effect or counted user, and no side table
/// (retention, relation) holds it; in the text, every `ensures normal` clause
/// is `resource_borrowed(FORMAL)` for each formal once, at most one
/// `resource_variadic_borrowed_tail()` and exactly one `resource_return_none()`,
/// and every `requires` clause is `resource_contract_v1`, a
/// `resource_premise`, or premise-decomposition metadata. Any other clause,
/// exit selector or `quant` refuses.
pub fn check_declarations(checked: &CheckedCallerFrame, source: &str) -> Result<(), String> {
    use qcore_frontend::frontend::{SourceContractClauseKind, SourceContractExitSelector};
    use qcore_schemas::contract::{ResourceParameterMode, ResourceReturnMode};
    let parsed = qcore_frontend::frontend::parse_source_program(source)
        .map_err(|e| format!("caller-frame: {e}"))?;
    let loaded = crate::contract_loader::load_resource_contract_sources(&BTreeMap::from([(
        "caller-frame.qv".to_owned(),
        source.to_owned(),
    )]))?;
    let declared = crate::contract_loader::module_premises(source)?;
    for name in checked.declarations() {
        let declaration = parsed
            .declarations
            .iter()
            .find(|d| &d.value.name == name)
            .ok_or_else(|| format!("caller-frame: `{name}` is not declared"))?;
        let contract = declaration
            .value
            .function_contract
            .as_ref()
            .ok_or_else(|| format!("caller-frame: `{name}` has no contract"))?;
        let formals: Vec<&str> = declaration
            .value
            .parameters
            .iter()
            .map(|p| p.name.value.as_str())
            .collect();
        let mut borrowed = BTreeSet::new();
        let (mut tails, mut nones) = (0, 0);
        for clause in &contract.clauses {
            let text = clause.value.expression_source.trim();
            let refuse = || format!("caller-frame: `{name}` clause `{text}` is not caller-frame");
            match &clause.value.kind {
                SourceContractClauseKind::Requires => {
                    if text != "resource_contract_v1"
                        && !text.starts_with("resource_premise(")
                        && !METADATA_REQUIRES.iter().any(|m| text.starts_with(m))
                    {
                        return Err(refuse());
                    }
                }
                SourceContractClauseKind::Ensures(SourceContractExitSelector::Normal) => {
                    if text == "resource_return_none()" {
                        nones += 1;
                    } else if text == "resource_variadic_borrowed_tail()" {
                        tails += 1;
                    } else if let Some(formal) = text
                        .strip_prefix("resource_borrowed(")
                        .and_then(|t| t.strip_suffix(')'))
                        .map(str::trim)
                        .filter(|f| formals.contains(f))
                    {
                        if !borrowed.insert(formal.to_owned()) {
                            return Err(refuse());
                        }
                    } else {
                        return Err(refuse());
                    }
                }
                _ => return Err(refuse()),
            }
        }
        if nones != 1 || tails > 1 || borrowed.len() != formals.len() {
            return Err(format!(
                "caller-frame: `{name}` does not borrow every formal and return nothing"
            ));
        }
        let function = loaded
            .functions()
            .get(name)
            .ok_or_else(|| format!("caller-frame: `{name}` is not in the function map"))?;
        if function.result != ResourceReturnMode::None
            || function.parameters.len() != formals.len()
            || function
                .parameters
                .iter()
                .any(|m| *m != ResourceParameterMode::Borrowed)
            || !function.parameter_kinds.is_empty()
            || !function.container_effects.is_empty()
            || !function.counted_users.is_empty()
            || loaded.retaining_contract(name).is_some()
            || loaded.relational_contract(name).is_some()
        {
            return Err(format!(
                "caller-frame: `{name}` loads with a custody effect"
            ));
        }
        let stated: BTreeSet<String> = declared
            .get(name)
            .map(|ps| ps.iter().map(|p| p.name.clone()).collect())
            .unwrap_or_default();
        if &stated != checked.premises() {
            return Err(format!(
                "caller-frame: `{name}` states premises {stated:?}, the projection derives {:?}",
                checked.premises()
            ));
        }
    }
    Ok(())
}

/// Rule 32a: the members of each reviewed caller-frame projection, one slice
/// per projection: exactly its `declarations` (the test
/// `crates/qcore-contract/tests/caller_frame.rs` holds each slice equal to the
/// checked projection's [`CheckedCallerFrame::declarations`]).
///
/// A caller-frame declaration serves only callers **outside** its frame. The
/// frame's own bodies are the body the projection's model describes, and
/// that model's obligations are all still Open: a member's row that read a
/// member's declaration (its own, or a sibling's) would rest on the very
/// obligations the projection only classifies as not the caller's. So the
/// gateway holds every call from a member's body to a member
/// (`qcore_c_gateway::legacy_c::caller_frame_hold`), and each member's row is
/// decided by its body. Lean: `QCore.RkLogCallerFrame`
/// (`a_member_never_reads_a_member_declaration`).
pub const FRAMES: &[&[&str]] = &[
    // `logger-effects-v1` (`docs/reports/w48-rk-log-caller-frame/caller-frame.json`).
    &["rd_kafka_log_buf", "rd_kafka_log0"],
];

/// Whether `caller`'s body calling `callee` is a call inside one caller-frame
/// (both members of the same reviewed projection, `caller == callee`
/// included): such a call never reads the frame's declaration.
#[must_use]
pub fn within_one_frame(caller: &str, callee: &str) -> bool {
    FRAMES
        .iter()
        .any(|members| members.contains(&caller) && members.contains(&callee))
}

/// Whether `name` is a member of any reviewed caller-frame projection.
#[must_use]
pub fn is_frame_member(name: &str) -> bool {
    FRAMES.iter().any(|members| members.contains(&name))
}
