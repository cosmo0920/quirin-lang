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

//! Card **FB-BACKLOG-ACCOUNTING** (gateway rule 92): checked saturating
//! ledger transitions.
//!
//! A *ledger* is an unsigned byte or count field that tracks a sum (Fluent
//! Bit's per-output `fs_backlog_chunks_size` and `fs_chunks_size`). This
//! module is a consumer of the typed-source route (T): it imports a function
//! under its own node admission ([`LEDGER_ADMISSION`], which keeps `binary -`
//! and the two additive compound assignments as obligations) and then reads
//! every path. Each write to the named ledger field must be exactly one of
//! four transitions:
//!
//! * [`LedgerTransition::Discharge`]: `L = L - d` on the path where the guard
//!   `d > L` was **not** taken, with no write, call or local store between the
//!   guard's reads and the update, so the update cannot wrap;
//! * [`LedgerTransition::UnderflowClamp`]: `L = 0` on the path where that
//!   guard **was** taken;
//! * [`LedgerTransition::Charge`]: `L = L + d` where `d > MAX - L` was not
//!   taken (`MAX` the ledger type's maximum in the selected ABI, spelled as
//!   Clang spells the literal);
//! * [`LedgerTransition::OverflowClamp`]: `L = MAX` where it was taken.
//!
//! Every other write (a bare `L -= d`, a guard on another operand, a guard of
//! another shape, a store between guard and update, a clamp to another value)
//! is refused, and so is a function with a scan loop. With a
//! [`LedgerPairing`], every path must carry exactly as many charges (with
//! their clamps) as link calls and as many discharges (with their clamps) as
//! unlink calls, so a missing or doubled update on any path refuses;
//! without one, a path may change the ledger at most once.
//!
//! What a reading certifies is arithmetic: no transition wraps, and each
//! clamp arm is named. A clamp keeps the ledger in range but makes it
//! diverge from the sum it tracks; `QCore.BacklogLedger` proves that under
//! the conservation invariant (ledger = sum of the linked charges) the
//! underflow clamp is unreachable, and the overflow clamp is unreachable
//! under the declared physical bound. Premises, stated and not proved:
//! exclusive access to the ledger while the function runs (P-LEDGER-EXCLUSIVE),
//! callees do not write the ledger (P-LEDGER-CALLEES), and the selected ABI's
//! maximum is the one the caller supplies (P-LEDGER-ABI). The reading is not
//! a custody verdict and the custody gateway does not consult it.

use serde::Serialize;

use crate::typed_source::{
    SourceExpression as Expr, SourceOperation as Op, SourceSummary, SourceValue,
};
use crate::typed_source_nodes::{Classification, NodeAdmission};

/// The typed-source nodes this consumer admits. Each is obligation-bearing:
/// the checker below discharges the unsigned-wrap case only under the
/// dominating guard and refuses every other occurrence that writes a ledger.
pub const LEDGER_ADMISSION: NodeAdmission = NodeAdmission::new(
    "rule 92 ledger consumer",
    &[
        ("binary -", Classification::ObligationBearing),
        ("compound +=", Classification::ObligationBearing),
        ("compound -=", Classification::ObligationBearing),
    ],
);

/// The ledger a function is checked against.
#[derive(Clone, Copy, Debug)]
pub struct LedgerSpec<'a> {
    /// The field name, e.g. `fs_backlog_chunks_size`.
    pub field: &'a str,
    /// The field's C type as Clang spells it, e.g. `unsigned long`.
    pub ledger_type: &'a str,
    /// The type's maximum in the selected ABI, as Clang spells the literal.
    pub maximum: &'a str,
    /// The list operations each transition must pair with, if any.
    pub pairing: Option<LedgerPairing<'a>>,
}

/// Charges pair with `link` calls and discharges with `unlink` calls.
#[derive(Clone, Copy, Debug)]
pub struct LedgerPairing<'a> {
    pub link: &'a str,
    pub unlink: &'a str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LedgerTransition {
    Discharge,
    UnderflowClamp,
    Charge,
    OverflowClamp,
}

impl LedgerTransition {
    /// Whether the transition is on the charge side (pairs with a link).
    pub fn is_charge_side(self) -> bool {
        matches!(self, Self::Charge | Self::OverflowClamp)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LedgerSite {
    pub path: usize,
    pub operation: usize,
    pub transition: LedgerTransition,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LedgerReading {
    pub function: String,
    pub field: String,
    pub sites: Vec<LedgerSite>,
}

/// The guard a transition needs, read from its condition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GuardShape {
    /// `d > L`
    Underflow,
    /// `d > MAX - L`
    Overflow,
}

/// The decision table of rule 92, shared with `QCore.BacklogLedger.decide`:
/// the write's form, the shape of the nearest guard on the ledger, whether
/// that guard was taken, and whether the window between them is clean.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WriteForm {
    Subtract,
    Add,
    Zero,
    Maximum,
    Other,
}

pub fn decide(
    form: WriteForm,
    guard: Option<(bool, bool)>,
    clean_window: bool,
) -> Option<LedgerTransition> {
    // guard = (is_overflow_shape, taken)
    match (form, guard) {
        (WriteForm::Subtract, Some((false, false))) if clean_window => {
            Some(LedgerTransition::Discharge)
        }
        (WriteForm::Zero, Some((false, true))) => Some(LedgerTransition::UnderflowClamp),
        (WriteForm::Add, Some((true, false))) if clean_window => Some(LedgerTransition::Charge),
        (WriteForm::Maximum, Some((true, true))) => Some(LedgerTransition::OverflowClamp),
        _ => None,
    }
}

fn refusal(spec: &LedgerSpec<'_>, what: &str) -> String {
    format!("rule 92: ledger `{}` {what}", spec.field)
}

/// Replace every symbol produced by a `Read` with the place it read, so two
/// reads of one place compare equal. Only sound inside a window with no
/// intervening store or call, which the caller checks.
fn normal(value: &SourceValue, operations: &[Op]) -> SourceValue {
    let expression = match &value.expression {
        Expr::Symbol { operation } => match operations.get(*operation) {
            Some(Op::Read { place }) => Expr::Unary {
                operator: "read".into(),
                value: Box::new(normal(place, operations)),
            },
            _ => value.expression.clone(),
        },
        Expr::Cast { cast, value: inner } => Expr::Cast {
            cast: cast.clone(),
            value: Box::new(normal(inner, operations)),
        },
        Expr::Unary {
            operator,
            value: inner,
        } => Expr::Unary {
            operator: operator.clone(),
            value: Box::new(normal(inner, operations)),
        },
        Expr::Binary {
            operator,
            left,
            right,
        } => Expr::Binary {
            operator: operator.clone(),
            left: Box::new(normal(left, operations)),
            right: Box::new(normal(right, operations)),
        },
        Expr::Field { base, name } => Expr::Field {
            base: Box::new(normal(base, operations)),
            name: name.clone(),
        },
        Expr::Index { base, index } => Expr::Index {
            base: Box::new(normal(base, operations)),
            index: Box::new(normal(index, operations)),
        },
        other => other.clone(),
    };
    SourceValue {
        c_type: value.c_type.clone(),
        expression,
    }
}

/// The smallest operation index any symbol in `value` refers to.
fn earliest_symbol(value: &SourceValue, operations: &[Op]) -> Option<usize> {
    let mut least: Option<usize> = None;
    let mut visit = |index: usize| least = Some(least.map_or(index, |l| l.min(index)));
    fn walk(value: &SourceValue, operations: &[Op], visit: &mut dyn FnMut(usize)) {
        match &value.expression {
            Expr::Symbol { operation } => {
                visit(*operation);
                if let Some(Op::Read { place }) = operations.get(*operation) {
                    walk(place, operations, visit);
                }
            }
            Expr::Cast { value, .. } | Expr::Unary { value, .. } => walk(value, operations, visit),
            Expr::Binary { left, right, .. } => {
                walk(left, operations, visit);
                walk(right, operations, visit);
            }
            Expr::Field { base, .. } => walk(base, operations, visit),
            Expr::Index { base, index } => {
                walk(base, operations, visit);
                walk(index, operations, visit);
            }
            _ => {}
        }
    }
    walk(value, operations, &mut visit);
    least
}

/// A read of the ledger place, in normal form.
fn ledger_read(place: &SourceValue, operations: &[Op]) -> SourceValue {
    SourceValue {
        c_type: place.c_type.clone(),
        expression: Expr::Unary {
            operator: "read".into(),
            value: Box::new(normal(place, operations)),
        },
    }
}

fn mentions(value: &SourceValue, target: &SourceValue) -> bool {
    if value == target {
        return true;
    }
    match &value.expression {
        Expr::Cast { value, .. } | Expr::Unary { value, .. } => mentions(value, target),
        Expr::Binary { left, right, .. } => mentions(left, target) || mentions(right, target),
        Expr::Field { base, .. } => mentions(base, target),
        Expr::Index { base, index } => mentions(base, target) || mentions(index, target),
        _ => false,
    }
}

/// The guard's shape and its `d`, when the normalised condition is exactly
/// `d > L` or `d > MAX - L` over the ledger read `read`.
fn guard_shape(
    condition: &SourceValue,
    read: &SourceValue,
    spec: &LedgerSpec<'_>,
) -> Option<(GuardShape, SourceValue)> {
    let Expr::Binary {
        operator,
        left,
        right,
    } = &condition.expression
    else {
        return None;
    };
    if operator != ">" || left.c_type != spec.ledger_type {
        return None;
    }
    if **right == *read {
        return Some((GuardShape::Underflow, (**left).clone()));
    }
    if let Expr::Binary {
        operator: minus,
        left: maximum,
        right: subtrahend,
    } = &right.expression
        && minus == "-"
        && right.c_type == spec.ledger_type
        && maximum.c_type == spec.ledger_type
        && maximum.expression
            == (Expr::Integer {
                value: spec.maximum.into(),
            })
        && **subtrahend == *read
    {
        return Some((GuardShape::Overflow, (**left).clone()));
    }
    None
}

fn is_zero(value: &SourceValue, spec: &LedgerSpec<'_>) -> bool {
    if value.c_type != spec.ledger_type {
        return false;
    }
    let inner = match &value.expression {
        Expr::Cast { cast, value } if cast == "IntegralCast" => value.as_ref(),
        _ => value,
    };
    inner.expression == (Expr::Integer { value: "0".into() })
}

/// Check every path of `summary` against `spec`. `Err` is a refusal: the
/// ledger property stays `Unknown` for this function.
pub fn check_ledger_transitions(
    summary: &SourceSummary,
    spec: &LedgerSpec<'_>,
) -> Result<LedgerReading, String> {
    if summary.has_scan_loop() {
        return Err(refusal(spec, "is not read through a scan loop"));
    }
    let mut sites = Vec::new();
    for (path_index, path) in summary.paths().iter().enumerate() {
        let operations = path.operations.as_slice();
        let mut on_path = Vec::new();
        for (index, operation) in operations.iter().enumerate() {
            let Op::Write { place, value } = operation else {
                continue;
            };
            let Expr::Field { name, .. } = &place.expression else {
                continue;
            };
            if name != spec.field {
                continue;
            }
            if place.c_type != spec.ledger_type {
                return Err(refusal(
                    spec,
                    "has a type other than the declared ledger type",
                ));
            }
            let read = ledger_read(place, operations);
            // The write's form, and for an update the operand and its read.
            let mut operand = None;
            let mut update_read = None;
            let form = match &value.expression {
                Expr::Binary {
                    operator,
                    left,
                    right,
                } if value.c_type == spec.ledger_type
                    && right.c_type == spec.ledger_type
                    && normal(left, operations) == read
                    && matches!(operator.as_str(), "-" | "+") =>
                {
                    operand = Some(normal(right, operations));
                    update_read = earliest_symbol(left, operations);
                    if operator == "-" {
                        WriteForm::Subtract
                    } else {
                        WriteForm::Add
                    }
                }
                Expr::Integer { value: literal }
                    if value.c_type == spec.ledger_type && literal == spec.maximum =>
                {
                    WriteForm::Maximum
                }
                _ if is_zero(value, spec) => WriteForm::Zero,
                _ => WriteForm::Other,
            };
            let limit = update_read.unwrap_or(index);
            // The nearest guard before the write (before its read, for an
            // update) whose condition reads the ledger.
            let guard = operations[..limit]
                .iter()
                .enumerate()
                .rev()
                .find_map(|(at, operation)| match operation {
                    Op::Guard { condition, taken } => {
                        let normalised = normal(condition, operations);
                        mentions(&normalised, &read).then_some((at, condition, normalised, *taken))
                    }
                    _ => None,
                });
            let mut shape = None;
            let mut clean = false;
            if let Some((at, raw, condition, taken)) = &guard
                && let Some((found, d)) = guard_shape(condition, &read, spec)
            {
                shape = Some((found == GuardShape::Overflow, *taken));
                if let Some(operand) = &operand {
                    // The window: from the earliest read the guard or the
                    // update uses to the write, only reads, guards and local
                    // scopes.
                    let start = [
                        earliest_symbol(raw, operations),
                        earliest_symbol(value, operations),
                    ]
                    .into_iter()
                    .flatten()
                    .min()
                    .unwrap_or(*at)
                    .min(*at);
                    clean = d == *operand
                        && operations[start..index].iter().all(|operation| {
                            matches!(
                                operation,
                                Op::Read { .. }
                                    | Op::Guard { .. }
                                    | Op::LocalStart { .. }
                                    | Op::LocalEnd { .. }
                            )
                        });
                }
            }
            let Some(transition) = decide(form, shape, clean) else {
                return Err(refusal(
                    spec,
                    "write is not an admitted saturating transition",
                ));
            };
            on_path.push(LedgerSite {
                path: path_index,
                operation: index,
                transition,
            });
        }
        match spec.pairing {
            None if on_path.len() > 1 => {
                return Err(refusal(spec, "changes more than once on one path"));
            }
            None => {}
            Some(pairing) => {
                let calls = |name: &str| {
                    operations
                        .iter()
                        .filter(|operation| matches!(operation, Op::Call { function, .. } if function == name))
                        .count()
                };
                let charges = on_path
                    .iter()
                    .filter(|site| site.transition.is_charge_side())
                    .count();
                let discharges = on_path.len() - charges;
                if charges != calls(pairing.link) || discharges != calls(pairing.unlink) {
                    return Err(refusal(
                        spec,
                        "transitions are not paired one-to-one with the list operations on a path",
                    ));
                }
            }
        }
        sites.extend(on_path);
    }
    Ok(LedgerReading {
        function: summary.function().into(),
        field: spec.field.into(),
        sites,
    })
}

#[cfg(test)]
#[path = "../tests/unit/ledger_transition.rs"]
mod tests;
