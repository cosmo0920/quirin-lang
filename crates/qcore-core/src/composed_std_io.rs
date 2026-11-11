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

//! `source.std-io-extern.v1` — the checked composition of
//! `source.extern-call.v1` and `source.std-io.v1` (card `STD-IO-COMPOSE`).
//!
//! # What this model is, stated before anything else
//!
//! It is a **union of two checked models, not a third one**. A composed program
//! carries an [`ExternProgram`] and a [`StdIoProgram`] side by side plus the
//! program order that interleaves them, and [`check_composed_program`] runs
//! [`check_extern_program`] and [`check_std_io_program`] **unmodified** over
//! them before checking anything of its own. Nothing here re-implements a
//! custody rule, an adapter rule, an effect rule or a rendering rule; nothing
//! here can weaken one either, because neither projection is constructed by
//! this module.
//!
//! Concretely, a module on this route gets:
//!
//! * every `Q022`–`Q027` extern refusal, at the same site, from the same
//!   checker, with the mandatory nullable-owned-return adapter and the `Q023`
//!   contract-selection default unmoved;
//! * every `Q028`–`Q031` standard-IO refusal, including the structural effect
//!   discipline — [`StdIoOperand`]'s two *bare* constructors still cannot denote
//!   a resource, so a bare rendering still cannot touch custody, and the third
//!   (`borrow NAME`) denotes only an identity this route proved live;
//! * the established sequential passes over the extern lowering's semantics,
//!   which the composing driver runs exactly as the extern route does;
//! * and the *join* rules below, which are the only thing this module owns.
//!
//! # The join, in full
//!
//! Four rules, each fail-closed, each a refusal under `Q043`:
//!
//! 1. **The order is an interleaving.** [`ComposedProgram::order`] must list
//!    every event of both projections exactly once, in each projection's own
//!    order, ending with the single `Return` both models require. A composed
//!    program whose order lost, duplicated or reordered an event is refused
//!    rather than checked, because either projection would then have been
//!    checked against a program the emitted C does not perform.
//! 2. **A rendered value is bound earlier, and its number is modeled.** The
//!    extern model owns the value namespace, so a rendering operand naming a
//!    scalar resolves to a [`ValueId`] that model bound — never to a resource
//!    identity, which has no operand form — and the composed step that bound it
//!    must precede the step that renders it. A value whose number the source
//!    did not determine (a bound call result with no declared outcome) is not a
//!    renderable operand at all: rendering it would publish a line no model
//!    computed, so it refuses.
//! 3. **A borrowed rendering names a live identity** (card `STD-FMT-BORROW`).
//!    `print RENDERING borrow NAME;` renders the **model identity** of a
//!    resource — the number the emitted C's own trace spells `rN` — and it is
//!    admitted only where the extern model's *own published ledger*
//!    ([`ExternCallCertificate::live_after`]) holds that identity live at that
//!    composed step. This module derives no liveness of its own: it reads the
//!    extern projection's answer, so composing still owns no custody rule. An
//!    identity that was never established, or that departed by consumption,
//!    release, storage or reallocation before the rendering, is refused rather
//!    than rendered.
//!
//!    The rendering itself moves nothing. It opens no obligation, discharges
//!    none, and leaves the identity exactly as live afterwards as before — so
//!    the composed ledger is still, theorem for theorem, the extern
//!    projection's ledger. It is also **not a publication** in the sense
//!    standing constraint 10 refuses: the destination is a byte stream rather
//!    than an object of the custody graph, the bytes carry a copy of the
//!    identity *number* rather than the address, and no operation of any model
//!    here reads a stream back — so no second reachable path to the resource
//!    exists and nothing the caller owes has changed.
//!
//! 4. **Nothing is dropped at the join.** The composed effect row is the
//!    **union** of the two derived rows and must be admitted by the one row the
//!    entry declares; each projection's certificate must be its own, rederived
//!    here rather than believed; and the composed observation must reproduce
//!    both projections' own observations, per stream.
//!
//! # What is not claimed
//!
//! * **No cross-stream interleaving.** Standard output carries this module's
//!   `print` lines *and* the extern route's synthesized call trace, and their
//!   relative order on that one stream is real and checked. The order between
//!   stdout and stderr is not modeled, exactly as `source.std-io.v1` already
//!   stated — C gives no order between two separately buffered streams.
//! * **No new premise.** Every `ExternAssumption` a composed module carries is
//!   one its extern projection derived: `DeclaredContract`, `NoExtentTransport`,
//!   `LinkageSuppliedByBuilder` and `UncheckedLinkage` cross the join
//!   unchanged, and composing adds none.
//! * A declared outcome is a fixture control, not a claim about a production
//!   callee. See [`crate::extern_call::ScalarBinding`].

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::extern_call::{
    AbiType, ExternCallCertificate, ExternEvent, ExternProgram, ValueId, check_extern_program,
    recheck_extern_certificate,
};
use crate::ownership::ResourceId;
use crate::scalar_types::ScalarType;
use crate::std_io::{
    StdIoCertificate, StdIoEvent, StdIoOperand, StdIoProgram, check_std_io_program,
    recheck_std_io_certificate,
};
use crate::typed_errors::EffectRow;

/// One step of the composed program, naming the projection that models it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComposedStep {
    /// Event `usize` of the extern projection.
    Extern(usize),
    /// Event `usize` of the standard-IO projection.
    Io(usize),
    /// The single `return;`, which is the last event of **both** projections.
    /// One source statement, two models, one step.
    Return,
}

/// A complete composed module.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComposedProgram {
    pub entry: String,
    pub externs: ExternProgram,
    pub io: StdIoProgram,
    /// Program order over both projections; see the join rules in the module
    /// header.
    pub order: Vec<ComposedStep>,
    /// The value identity behind each scalar name a rendering may read. Built
    /// by the composing lowering from the extern model's own value namespace,
    /// so a name that means a resource identity is absent by construction.
    pub operand_values: BTreeMap<String, ValueId>,
    /// The declared ABI spelling of each bound value, so a rendering of a width
    /// this profile does not render refuses instead of being emitted with the
    /// wrong conversion.
    pub value_abi: BTreeMap<ValueId, AbiType>,
    /// The custody identity behind each name a **borrowed** rendering reads
    /// (card `STD-FMT-BORROW`). Built by the composing lowering from the extern
    /// model's own resource namespace, which is disjoint from `operand_values`
    /// by construction: a name is in one map or the other and never in both.
    pub operand_identities: BTreeMap<String, ResourceId>,
}

/// The ABI spelling `source.std-io.v1` rendered before card
/// `W48-QR-SCALAR-TYPES-R3`, which widened the composed rendering to every
/// §G02t type through §G02t's own conversion table (`scalar_types::render`
/// and `scalar_types::c_rendering`). Kept as the name of the one type whose
/// rendering is `source.std-io.v1`'s own, byte for byte.
pub const RENDERABLE_ABI: AbiType = AbiType::I32;

/// Refusals this composition owns. Everything else a composed module can be
/// refused for belongs to one of the two projections and keeps that
/// projection's own code.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ComposedError {
    /// `Q043`: the order is not an interleaving of the two projections.
    Order { reason: &'static str },
    /// `Q043`: a rendering names a scalar whose binding this program does not
    /// place before it.
    OperandBoundLater { step: usize, name: String },
    /// `Q043`: a rendering names a scalar whose number no model determined.
    OperandNotDetermined { step: usize, name: String },
    /// `Q043`: a rendering names a value this profile cannot render: since
    /// card `W48-QR-SCALAR-TYPES-R3`, an `isize`/`usize` with no recorded
    /// target (every other §G02t type renders).
    OperandNotRenderable {
        step: usize,
        name: String,
        ty: ScalarType,
    },
    /// `Q043`: a borrowed rendering names something this module binds no
    /// custody identity for (card `STD-FMT-BORROW`).
    BorrowUnknown { step: usize, name: String },
    /// `Q043`: a borrowed rendering names an identity the extern model's own
    /// published ledger does not hold live at that composed step — it was never
    /// established, or it departed before the rendering.
    BorrowNotLive { step: usize, name: String },
    /// `Q043`: the identity the standard-IO projection rendered is not the one
    /// the extern model bound to that name. A cross-check, never a source
    /// shape: reaching it means the two halves disagree about what was
    /// rendered.
    BorrowDisagrees { step: usize, name: String },
    /// `Q043`: the union of the two derived effect rows is not admitted by the
    /// one row the entry declares.
    EffectNotDeclared { effect: String },
    /// `Q043`: a projection's own checker refused. Reported so the composition
    /// never produces a certificate over a program a projection rejected; the
    /// driver reports the projection's own diagnostics beside this.
    Projection { which: &'static str },
}

impl ComposedError {
    /// One code, one pass. Every other refusal a composed module can receive is
    /// the refusal one of the two composed routes already gave it.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        "Q043"
    }
}

impl fmt::Display for ComposedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Order { reason } => write!(
                f,
                "the composed program order does not interleave its two models: {reason}"
            ),
            Self::OperandBoundLater { name, .. } => write!(
                f,
                "`{name}` is rendered before the statement that binds it; a composed rendering \
                 reads a value the same entry already bound"
            ),
            Self::OperandNotDetermined { name, .. } => write!(
                f,
                "`{name}` is bound to a call result this module does not model a number for, so \
                 rendering it would publish a line no model computed; declare the result with \
                 `succeeds INTEGER` or render a value this entry determines"
            ),
            Self::OperandNotRenderable { name, ty, .. } => write!(
                f,
                "`{name}` is a `{ty}`, whose width only a recorded target supplies, and none was \
                 supplied; the rendering is refused rather than guessed"
            ),
            Self::BorrowUnknown { name, .. } => write!(
                f,
                "`borrow {name}` renders the model identity of a live resource, and this module \
                 binds no resource of that name; a borrowed rendering reads the extern model's \
                 custody namespace and never its value namespace"
            ),
            Self::BorrowNotLive { name, .. } => write!(
                f,
                "`{name}` is not live at this rendering, so its identity is not this entry's to \
                 render; a borrowed rendering requires the resource live at that statement, and \
                 an identity that was never established or that has departed is refused rather \
                 than rendered"
            ),
            Self::BorrowDisagrees { name, .. } => write!(
                f,
                "the identity rendered for `{name}` is not the one the extern model bound to it"
            ),
            Self::EffectNotDeclared { effect } => write!(
                f,
                "this entry performs `{effect}` across its two models but its declared \
                 `effects {{ … }}` row does not admit it"
            ),
            Self::Projection { which } => {
                write!(f, "the {which} projection of this module refuses it")
            }
        }
    }
}

impl std::error::Error for ComposedError {}

/// Evidence a checked composed module publishes.
///
/// It carries **both** projections' certificates verbatim. That is the point:
/// a reader of a composed result reads exactly the two results the two routes
/// publish on their own, plus the join fields below, and can check either one
/// against that route's own rules without trusting this module.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComposedCertificate {
    pub entry: String,
    pub externs: ExternCallCertificate,
    pub io: StdIoCertificate,
    /// The interleaving, rederived and compared rather than copied.
    pub order: Vec<ComposedStep>,
    /// The union of the two derived rows.
    pub effects: EffectRow,
    /// Every value a rendering read, with the number it carried, in the order
    /// the renderings read them. A reader can see which numbers crossed from
    /// the extern model into the rendered lines.
    pub rendered_values: Vec<(String, i32)>,
    /// The same, for values of every other §G02t type, with the type (card
    /// `W48-QR-SCALAR-TYPES-R3`). Empty for every program before the card.
    pub typed_rendered_values: Vec<(String, ScalarType, i128)>,
    /// Every borrowed rendering, as `(name, identity)` in composed order, with
    /// the identity the extern model's ledger held live at that step (card
    /// `STD-FMT-BORROW`). Published separately from `rendered_values` because
    /// the two say different things: one names a number that crossed the value
    /// namespace, the other names a **custody identity** the entry still owed a
    /// cleanup for when the line was written.
    pub borrowed_renders: Vec<(String, ResourceId)>,
}

/// Check the interleaving alone. Split out so it can be stated as a property
/// rather than only as a step of the whole check.
fn check_order(program: &ComposedProgram) -> Result<(), ComposedError> {
    let extern_events = program.externs.events.len();
    let io_events = program.io.events.len();
    // The `Return` is the last event of each projection and one composed step,
    // so the order is one shorter than the two event counts together.
    if extern_events == 0 || io_events == 0 {
        return Err(ComposedError::Order {
            reason: "a composed module has events in both models; a module using one model is not \
                     composed and keeps that model's own route",
        });
    }
    if program.externs.events.last() != Some(&ExternEvent::Return)
        || program.io.events.last() != Some(&StdIoEvent::Return)
    {
        return Err(ComposedError::Order {
            reason: "each projection ends in its own `return`",
        });
    }
    if program.order.last() != Some(&ComposedStep::Return) {
        return Err(ComposedError::Order {
            reason: "the composed order ends in the one `return` both models share",
        });
    }
    let mut next_extern = 0;
    let mut next_io = 0;
    for (position, step) in program.order.iter().enumerate() {
        match step {
            ComposedStep::Extern(index) => {
                if *index != next_extern {
                    return Err(ComposedError::Order {
                        reason: "the extern events appear in their own order, each exactly once",
                    });
                }
                next_extern += 1;
            }
            ComposedStep::Io(index) => {
                if *index != next_io {
                    return Err(ComposedError::Order {
                        reason: "the standard-IO events appear in their own order, each exactly \
                                 once",
                    });
                }
                next_io += 1;
            }
            ComposedStep::Return => {
                if position + 1 != program.order.len() {
                    return Err(ComposedError::Order {
                        reason: "nothing follows the composed `return`",
                    });
                }
            }
        }
    }
    // The trailing `Return` of each projection is the composed `Return` step.
    if next_extern != extern_events - 1 || next_io != io_events - 1 {
        return Err(ComposedError::Order {
            reason: "every event of both models is placed exactly once",
        });
    }
    Ok(())
}

/// Which composed position binds each value name, and which composed position
/// each standard-IO event occupies.
fn positions(program: &ComposedProgram) -> (BTreeMap<ValueId, usize>, Vec<usize>) {
    let mut bound_at = BTreeMap::new();
    let mut io_positions = vec![0; program.io.events.len()];
    for (position, step) in program.order.iter().enumerate() {
        match step {
            ComposedStep::Extern(index) => {
                if let Some(ExternEvent::Call(call)) = program.externs.events.get(*index) {
                    if let Some(binding) = call.binding {
                        bound_at.insert(binding.value, position);
                    }
                } else if let Some(ExternEvent::Bind(literal)) = program.externs.events.get(*index)
                {
                    bound_at.insert(literal.value, position);
                } else if let Some(ExternEvent::Compute(statement)) =
                    program.externs.events.get(*index)
                {
                    // Card `W48-QR-SCALAR-TYPES-R3`: a value statement binds
                    // at its own step.
                    bound_at.insert(statement.value, position);
                }
            }
            ComposedStep::Io(index) => {
                if let Some(slot) = io_positions.get_mut(*index) {
                    *slot = position;
                }
            }
            ComposedStep::Return => {}
        }
    }
    (bound_at, io_positions)
}

/// The identities live *before* each composed position, read off the extern
/// certificate's own published ledger (card `STD-FMT-BORROW`).
///
/// The composition owns no custody rule, and this function is where that shows:
/// it re-derives nothing. It walks the interleaving carrying the extern model's
/// own `live_after` entry for the last extern event placed, which is exactly the
/// state the emitted C is in when the next standard-IO statement runs. The empty
/// ledger before the first extern event is the entry's own starting state.
fn live_before(
    program: &ComposedProgram,
    certificate: &ExternCallCertificate,
) -> Vec<Vec<ResourceId>> {
    let mut current: Vec<ResourceId> = Vec::new();
    let mut live = Vec::with_capacity(program.order.len());
    for step in &program.order {
        live.push(current.clone());
        if let ComposedStep::Extern(index) = step
            && let Some(after) = certificate.live_after.get(*index)
        {
            current = after.clone();
        }
    }
    live
}

/// Check one composed module and derive its certificate.
///
/// Runs both projections' own checkers first, then the join. Every refusal
/// found is reported, so a caller sees the whole cascade rather than the first
/// step of it.
pub fn check_composed_program(
    program: &ComposedProgram,
) -> Result<ComposedCertificate, Vec<ComposedError>> {
    let Ok(extern_certificate) = check_extern_program(&program.externs) else {
        return Err(vec![ComposedError::Projection { which: "extern" }]);
    };
    let Ok(io_certificate) = check_std_io_program(&program.io) else {
        return Err(vec![ComposedError::Projection {
            which: "standard-IO",
        }]);
    };
    let mut errors = Vec::new();
    if let Err(error) = check_order(program) {
        // Every later rule reads the order, so a broken interleaving is
        // reported alone rather than followed by its own consequences.
        return Err(vec![error]);
    }
    let (bound_at, io_positions) = positions(program);
    let live = live_before(program, &extern_certificate);
    let mut rendered_values = Vec::new();
    let mut typed_rendered_values = Vec::new();
    let mut borrowed_renders = Vec::new();
    for (index, event) in program.io.events.iter().enumerate() {
        let operand = match event {
            StdIoEvent::Print { operand, .. } | StdIoEvent::Log { operand, .. } => operand,
            StdIoEvent::LogLevel { .. } | StdIoEvent::Flush | StdIoEvent::Return => continue,
        };
        // Card `STD-FMT-BORROW`. The three rules below are the whole custody
        // argument in code: the name must be a resource this module binds, the
        // extern model's own ledger must hold it live at this step, and the
        // number the standard-IO projection rendered must be that identity.
        if let StdIoOperand::Borrowed(name) = operand {
            let step = io_positions.get(index).copied().unwrap_or_default();
            let Some(identity) = program.operand_identities.get(name).copied() else {
                errors.push(ComposedError::BorrowUnknown {
                    step,
                    name: name.clone(),
                });
                continue;
            };
            let live_here = live.get(step).map(Vec::as_slice).unwrap_or_default();
            if !live_here.contains(&identity) {
                errors.push(ComposedError::BorrowNotLive {
                    step,
                    name: name.clone(),
                });
                continue;
            }
            match (
                program.io.identities.get(name).copied(),
                i32::try_from(identity),
            ) {
                (Some(rendered), Ok(expected)) if rendered == expected => {
                    borrowed_renders.push((name.clone(), identity));
                }
                _ => errors.push(ComposedError::BorrowDisagrees {
                    step,
                    name: name.clone(),
                }),
            }
            continue;
        }
        let StdIoOperand::Scalar(name) = operand else {
            continue;
        };
        let step = io_positions.get(index).copied().unwrap_or_default();
        let Some(value) = program.operand_values.get(name).copied() else {
            errors.push(ComposedError::OperandNotDetermined {
                step,
                name: name.clone(),
            });
            continue;
        };
        // Card `W48-QR-SCALAR-TYPES-R3`: every §G02t type renders, read off
        // the extern checker's own typing of its namespace; only a
        // target-width type with no recorded target is refused.
        let ty = match extern_certificate.value_types.get(&value) {
            Some(ty) => *ty,
            None => {
                errors.push(ComposedError::OperandNotDetermined {
                    step,
                    name: name.clone(),
                });
                continue;
            }
        };
        if ty.is_target_width() && program.io.target.is_none() {
            errors.push(ComposedError::OperandNotRenderable {
                step,
                name: name.clone(),
                ty,
            });
            continue;
        }
        match bound_at.get(&value) {
            Some(binding_step) if *binding_step < step => {}
            _ => {
                errors.push(ComposedError::OperandBoundLater {
                    step,
                    name: name.clone(),
                });
                continue;
            }
        }
        // The number itself comes from the standard-IO projection's own scalar
        // environment, which the composing lowering filled from the extern
        // model. Absence here is the "no model computed a number" case and is
        // already a refusal of that projection's checker; it is repeated rather
        // than assumed away.
        if ty == ScalarType::I32 {
            match program.io.scalars.get(name) {
                Some(number) => rendered_values.push((name.clone(), *number)),
                None => errors.push(ComposedError::OperandNotDetermined {
                    step,
                    name: name.clone(),
                }),
            }
        } else {
            match program.io.typed_scalars.get(name) {
                Some((rendered, number)) if *rendered == ty => {
                    typed_rendered_values.push((name.clone(), ty, *number));
                }
                _ => errors.push(ComposedError::OperandNotDetermined {
                    step,
                    name: name.clone(),
                }),
            }
        }
    }
    // The union of the two derived rows, against the one row the entry
    // declares. Neither projection sees the other's effects, so this is the
    // one place the composed row exists at all.
    let mut effects = BTreeSet::new();
    effects.extend(extern_certificate.effects.iter().cloned());
    effects.extend(io_certificate.effects.iter().cloned());
    let effects = EffectRow::new(effects);
    for effect in effects.iter() {
        if !program.io.declared_effects.contains(effect) {
            errors.push(ComposedError::EffectNotDeclared {
                effect: format!("{effect:?}"),
            });
        }
    }
    if errors.is_empty() {
        Ok(ComposedCertificate {
            entry: program.entry.clone(),
            externs: extern_certificate,
            io: io_certificate,
            order: program.order.clone(),
            effects,
            rendered_values,
            typed_rendered_values,
            borrowed_renders,
        })
    } else {
        Err(errors)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ComposedRecheckFailure {
    Entry,
    Extern,
    Io,
    Order,
    Effects,
    RenderedValues,
    BorrowedRenders,
    Refused,
}

impl fmt::Display for ComposedRecheckFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let what = match self {
            Self::Entry => "the entry name",
            Self::Extern => "the extern projection's certificate",
            Self::Io => "the standard-IO projection's certificate",
            Self::Order => "the composed order",
            Self::Effects => "the composed effect row",
            Self::RenderedValues => "the rendered values",
            Self::BorrowedRenders => "the borrowed renders and the identities they name",
            Self::Refused => "the program itself, which the recheck refuses",
        };
        write!(f, "{what} does not survive an independent rederivation")
    }
}

/// Rederive the composed certificate from the program alone and compare it
/// field for field, **including** rechecking each projection's certificate
/// against that projection's own recheck. A composed publication that dropped
/// or edited either sub-certificate is refused here rather than published.
pub fn recheck_composed_certificate(
    program: &ComposedProgram,
    certificate: &ComposedCertificate,
) -> Result<(), ComposedRecheckFailure> {
    recheck_extern_certificate(&program.externs, &certificate.externs)
        .map_err(|_| ComposedRecheckFailure::Extern)?;
    recheck_std_io_certificate(&program.io, &certificate.io)
        .map_err(|_| ComposedRecheckFailure::Io)?;
    let rederived = check_composed_program(program).map_err(|_| ComposedRecheckFailure::Refused)?;
    if rederived.entry != certificate.entry {
        return Err(ComposedRecheckFailure::Entry);
    }
    if rederived.externs != certificate.externs {
        return Err(ComposedRecheckFailure::Extern);
    }
    if rederived.io != certificate.io {
        return Err(ComposedRecheckFailure::Io);
    }
    if rederived.order != certificate.order {
        return Err(ComposedRecheckFailure::Order);
    }
    if rederived.effects != certificate.effects {
        return Err(ComposedRecheckFailure::Effects);
    }
    if rederived.rendered_values != certificate.rendered_values
        || rederived.typed_rendered_values != certificate.typed_rendered_values
    {
        return Err(ComposedRecheckFailure::RenderedValues);
    }
    if rederived.borrowed_renders != certificate.borrowed_renders {
        return Err(ComposedRecheckFailure::BorrowedRenders);
    }
    Ok(())
}
