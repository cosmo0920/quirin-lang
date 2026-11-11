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

//! Bind recorded native descriptor executions to the declared libc resource
//! contracts (card W43-DESCRIPTOR-NATIVE-CHECK, Phase B).
//!
//! Input: one run's JSONL records (the schema of
//! [`crate::descriptor_observation`]) and a loaded [`ResourceContracts`]
//! environment, typically the `quirin-provider-libc` modules. Every recorded
//! call whose name has a loaded contract is checked against **that contract's
//! loaded record**, never against a per-callee table:
//!
//! * an owned result of kind `K` (`resource_return_owned` +
//!   `resource_return_kind`): a failing execution returns the kind's failure
//!   value (`-1` for the integer `descriptor` kind, NULL for a pointer kind),
//!   is allowed only when the contract says the result may fail
//!   (`sentinel`/`nullable`), and acquires nothing; every other execution
//!   acquires exactly one identity, of kind `K`, equal to the result, which
//!   was not live before, and (for the integer kind) is nonnegative;
//! * a released parameter of kind `K` (`resource_released` +
//!   `resource_kind`): the record releases exactly one identity, of kind `K`.
//!   Whether the caller met the precondition (`valid`/`nonnull`, live, not
//!   owned by another resource) is recorded as a **caller** observation, not
//!   as a contract contradiction;
//! * every other position is borrowed and the result is `none`: the record
//!   acquires, adopts, derives, replaces and releases nothing.
//!
//! Any failure is a **contradiction** of the contract by a recorded execution.
//! A contract shape this binding cannot check (more than one released
//! parameter, a heap-kind owned result, a reallocated or derived result, an
//! out/stored/consumed parameter) is an error, never a silent pass.
//!
//! Calls with no loaded contract get a descriptive **shape profile** (result
//! classes, acquisitions and releases by kind, reacquisitions of a live
//! identity) so that a refused callee's recorded behaviour can be compared
//! with its refusal ground. A profile is evidence only; it declares nothing.
//!
//! Injected failures are counted separately and never bound: the harness, not
//! libc, produced them.

use crate::contract_loader::{DeclaredLiteralArgument, declared_literal_argument_premises};
use crate::descriptor_observation::{Kind, Record, Role};
use qcore_schemas::contract::{
    ResourceContracts, ResourceKind, ResourceParameterMode, ResourceReturnMode,
};
use serde::Serialize;
use std::collections::BTreeMap;

const MAX_EXAMPLES: usize = 12;
const MAX_ARGUMENT_VALUES: usize = 16;

/// The observed kind a declared resource kind binds to. Heap has none, and
/// neither has a kind the native descriptor harness does not record (card
/// CG25-RESOURCE-KINDS: `dynamic_library`, `regex`; card REFCOUNT-RESOURCE-KIND:
/// `reference`); binding one is an error.
#[must_use]
pub fn observed_kind(kind: ResourceKind) -> Option<Kind> {
    match kind {
        ResourceKind::Heap
        | ResourceKind::DynamicLibrary
        | ResourceKind::Regex
        | ResourceKind::Reference => None,
        ResourceKind::Descriptor => Some(Kind::Fd),
        ResourceKind::Stream => Some(Kind::File),
        ResourceKind::Directory => Some(Kind::Dir),
    }
}

/// One recorded execution that contradicts, or whose caller broke, a contract.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct BindingExample {
    pub sequence: u64,
    pub site_module: String,
    pub site_offset: u64,
    pub result: i64,
    pub errno: i64,
    pub detail: String,
}

/// Binding of every recorded execution of one declared callee.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct CalleeBinding {
    /// The loaded contract, rendered.
    pub contract: String,
    pub executions: usize,
    pub injected: usize,
    pub successes: usize,
    pub failures: usize,
    /// Successful executions of an integer-kind owned result that returned 0.
    pub zero_results: usize,
    pub contradictions: usize,
    pub contradiction_examples: Vec<BindingExample>,
    /// Caller-side precondition observations, by class.
    pub caller_observations: BTreeMap<String, usize>,
    pub caller_examples: Vec<BindingExample>,
    /// Releases that returned an error but still ended the identity.
    pub released_despite_error: usize,
    /// Other recorded facts, by name (for example a stream's derived descriptor).
    pub facts: BTreeMap<String, usize>,
}

/// Recorded shape of one call that has no loaded contract. Evidence only.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct CallShape {
    pub executions: usize,
    pub injected: usize,
    pub result_minus_one: usize,
    pub result_zero: usize,
    pub result_positive: usize,
    pub result_other_negative: usize,
    /// Executions that acquired at least one identity, by result class.
    pub acquiring_by_result: BTreeMap<String, usize>,
    pub acquisitions: BTreeMap<String, usize>,
    pub releases: BTreeMap<String, usize>,
    /// Acquired identity equal to the call's result.
    pub acquired_equals_result: usize,
    /// An identity acquired while already live (a reference, not fresh custody).
    pub reacquired_while_live: usize,
    /// Values of the second recorded argument (for example an `fcntl`
    /// command), capped at `MAX_ARGUMENT_VALUES` distinct values.
    pub argument_1_values: BTreeMap<i64, usize>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct BindingReport {
    pub records: usize,
    pub bound: BTreeMap<String, CalleeBinding>,
    pub unbound: BTreeMap<String, CallShape>,
    pub contradictions: usize,
    pub static_certificate: bool,
}

#[derive(Clone, Copy, Debug)]
struct Live {
    kind: Kind,
    generation: u32,
    /// The stream or directory that owns this descriptor, if any.
    owned: bool,
}

/// Identity spaces: descriptor numbers, and one shared pointer space.
/// Whether an execution's recorded arguments meet every declared literal
/// premise. A literal is canonical decimal token text (`2`, `-` `1`); one that
/// does not read as an `i64` is an error, never a match.
fn premises_hold(premises: &[DeclaredLiteralArgument], args: &[i64]) -> Result<bool, String> {
    for premise in premises {
        let text = premise.literal.concat();
        let value: i64 = text
            .parse()
            .map_err(|_| format!("literal premise `{text}` is not an integer"))?;
        if args.get(premise.parameter) != Some(&value) {
            return Ok(false);
        }
    }
    Ok(true)
}

fn key(kind: Kind, id: i64) -> (bool, i64) {
    (kind == Kind::Fd, id)
}

fn render(contract: &qcore_schemas::contract::ResourceFunctionContract) -> String {
    format!("{contract:?}")
}

struct Binder<'a> {
    contracts: &'a ResourceContracts,
    /// Card W46-EXTERNAL-CALLEE-CONTRACTS: the literal-argument premises
    /// (contract spec C04.5) rederived from the environment's trusted texts.
    premises: BTreeMap<String, Vec<DeclaredLiteralArgument>>,
    live: BTreeMap<(bool, i64), Live>,
    report: BindingReport,
}

fn example(row: &Record, detail: String) -> BindingExample {
    BindingExample {
        sequence: row.sequence,
        site_module: row.site.module.clone(),
        site_offset: row.site.offset,
        result: row.result,
        errno: row.errno,
        detail,
    }
}

impl CalleeBinding {
    fn contradict(&mut self, row: &Record, detail: String) {
        self.contradictions += 1;
        if self.contradiction_examples.len() < MAX_EXAMPLES {
            self.contradiction_examples.push(example(row, detail));
        }
    }

    fn caller(&mut self, row: &Record, class: &str, detail: String) {
        *self.caller_observations.entry(class.into()).or_default() += 1;
        if self.caller_examples.len() < MAX_EXAMPLES {
            self.caller_examples.push(example(row, detail));
        }
    }
}

impl<'a> Binder<'a> {
    fn bind(&mut self, row: &Record) -> Result<(), String> {
        let Some(contract) = self.contracts.contract(&row.call) else {
            self.profile(row);
            return Ok(());
        };
        // A declaration with a literal-argument premise applies only to an
        // execution whose recorded arguments are exactly its literals; every
        // other execution (including one whose argument was not recorded) is
        // of an undeclared callee and is profiled, never bound (C04.5).
        if let Some(premises) = self.premises.get(&row.call)
            && !premises_hold(premises, &row.args)?
        {
            self.profile(row);
            return Ok(());
        }
        let entry = self.report.bound.entry(row.call.clone()).or_default();
        if entry.contract.is_empty() {
            entry.contract = render(contract);
        }
        entry.executions += 1;
        if row.injected {
            entry.injected += 1;
            return Ok(());
        }
        // The contract's released parameters, with their kinds.
        let mut released = Vec::new();
        for (index, mode) in contract.parameters.iter().enumerate() {
            match mode {
                ResourceParameterMode::Borrowed => {}
                ResourceParameterMode::Released { nullable } => {
                    let kind = contract
                        .parameter_kinds
                        .get(&index)
                        .copied()
                        .unwrap_or_default();
                    let observed = observed_kind(kind).ok_or_else(|| {
                        format!(
                            "{}: a {} release cannot be bound to descriptor records",
                            row.call,
                            kind.spelling()
                        )
                    })?;
                    released.push((observed, *nullable));
                }
                other => {
                    return Err(format!(
                        "{}: parameter {index} mode {other:?} is outside the binding",
                        row.call
                    ));
                }
            }
        }
        if released.len() > 1 {
            return Err(format!("{}: more than one released parameter", row.call));
        }
        let owned = match contract.result {
            ResourceReturnMode::None => None,
            ResourceReturnMode::Owned { nullable } => {
                let observed = observed_kind(contract.result_kind).ok_or_else(|| {
                    format!(
                        "{}: a {} owned result cannot be bound to descriptor records",
                        row.call,
                        contract.result_kind.spelling()
                    )
                })?;
                Some((observed, nullable))
            }
            other => {
                return Err(format!(
                    "{}: result mode {other:?} is outside the binding",
                    row.call
                ));
            }
        };
        let acquired: Vec<_> = row
            .operands
            .iter()
            .filter(|operand| operand.role == Role::Acquired)
            .collect();
        let custody_other: Vec<_> = row
            .operands
            .iter()
            .filter(|operand| {
                matches!(
                    operand.role,
                    Role::Adopted | Role::Replaced | Role::ReleasedChild | Role::Derived
                )
            })
            .collect();
        let releases: Vec<_> = row
            .operands
            .iter()
            .filter(|operand| operand.role == Role::Released)
            .collect();

        match owned {
            Some((kind, may_fail)) => {
                let failure_value = if kind == Kind::Fd { -1 } else { 0 };
                let failed = row.result == failure_value;
                if failed {
                    entry.failures += 1;
                    if !may_fail {
                        entry.contradict(
                            row,
                            "a result declared never to fail returned the failure value".into(),
                        );
                    }
                    if !acquired.is_empty() {
                        entry.contradict(row, "a failing execution acquired an identity".into());
                    }
                } else {
                    entry.successes += 1;
                    if kind == Kind::Fd && row.result == 0 {
                        entry.zero_results += 1;
                    }
                    if kind == Kind::Fd && row.result < 0 {
                        entry.contradict(
                            row,
                            format!("descriptor result {} is neither -1 nor valid", row.result),
                        );
                    }
                    match acquired.as_slice() {
                        [one] if one.kind == kind && one.id == row.result => {
                            if self.live.contains_key(&key(one.kind, one.id)) {
                                entry.contradict(
                                    row,
                                    format!("owned result {} was already live", one.id),
                                );
                            }
                        }
                        _ => entry.contradict(
                            row,
                            format!(
                                "a successful execution must acquire exactly one {kind:?} equal to the result; recorded {}",
                                acquired.len()
                            ),
                        ),
                    }
                }
            }
            None => {
                if !acquired.is_empty() {
                    entry.contradict(
                        row,
                        "a call whose result owns nothing acquired an identity".into(),
                    );
                }
            }
        }
        // Derived descriptors of a new stream or directory are the one
        // custody-other role an owned stream/directory result may carry.
        for operand in &custody_other {
            let derived = operand.role == Role::Derived
                && operand.kind == Kind::Fd
                && matches!(owned, Some((Kind::File | Kind::Dir, _)));
            let ended_child = operand.role == Role::ReleasedChild
                && operand.kind == Kind::Fd
                && matches!(released.first(), Some((Kind::File | Kind::Dir, _)));
            let allowed = derived || ended_child;
            if allowed {
                let fact = if operand.role == Role::Derived {
                    "owned_result_derives_its_descriptor"
                } else {
                    "release_ends_owned_descriptor"
                };
                *entry.facts.entry(fact.into()).or_default() += 1;
            } else {
                entry.contradict(
                    row,
                    format!(
                        "{:?} {:?} {} is not stated by the contract",
                        operand.role, operand.kind, operand.id
                    ),
                );
            }
        }
        match (released.first(), releases.as_slice()) {
            (None, []) => {}
            (None, _) => entry.contradict(
                row,
                "a call that releases nothing released an identity".into(),
            ),
            (Some((kind, _)), [one]) if one.kind == *kind => {
                let state = self.live.get(&key(one.kind, one.id)).copied();
                let invalid = if *kind == Kind::Fd {
                    one.id < 0
                } else {
                    one.id == 0
                };
                if invalid {
                    entry.caller(
                        row,
                        "invalid_argument",
                        format!("released the failure value {}", one.id),
                    );
                } else {
                    match state {
                        None => entry.caller(
                            row,
                            "not_live",
                            format!(
                                "{:?} {} generation {} was not live",
                                one.kind, one.id, one.generation
                            ),
                        ),
                        Some(live) if live.generation != one.generation => entry.caller(
                            row,
                            "stale_generation",
                            format!(
                                "{:?} {} generation {} is not the live one",
                                one.kind, one.id, one.generation
                            ),
                        ),
                        Some(live) if live.owned => entry.caller(
                            row,
                            "owned_by_another_resource",
                            format!(
                                "{:?} {} is owned by a stream or directory",
                                one.kind, one.id
                            ),
                        ),
                        Some(_) => {
                            if row.result != 0 {
                                entry.released_despite_error += 1;
                            }
                        }
                    }
                }
            }
            (Some((kind, _)), recorded) => entry.contradict(
                row,
                format!(
                    "a release of one {kind:?} recorded {} releases of other shapes",
                    recorded.len()
                ),
            ),
        }
        // Borrowed positions: a use of a non-live identity is a caller fact.
        for operand in row.operands.iter().filter(|o| o.role == Role::Used) {
            let state = self.live.get(&key(operand.kind, operand.id));
            if state.is_none_or(|live| live.generation != operand.generation) {
                entry.caller(
                    row,
                    "use_not_live",
                    format!("{:?} {} used while not live", operand.kind, operand.id),
                );
            }
        }
        Ok(())
    }

    fn profile(&mut self, row: &Record) {
        if matches!(row.call.as_str(), "start" | "exit") {
            return;
        }
        let shape = self.report.unbound.entry(row.call.clone()).or_default();
        shape.executions += 1;
        if row.injected {
            shape.injected += 1;
            return;
        }
        let class = match row.result {
            -1 => "minus_one",
            0 => "zero",
            r if r > 0 => "positive",
            _ => "other_negative",
        };
        match class {
            "minus_one" => shape.result_minus_one += 1,
            "zero" => shape.result_zero += 1,
            "positive" => shape.result_positive += 1,
            _ => shape.result_other_negative += 1,
        }
        let mut acquired_any = false;
        for operand in &row.operands {
            match operand.role {
                Role::Acquired => {
                    acquired_any = true;
                    *shape
                        .acquisitions
                        .entry(format!("{:?}", operand.kind))
                        .or_default() += 1;
                    if operand.id == row.result {
                        shape.acquired_equals_result += 1;
                    }
                    // `dup2` onto a live number replaces it in the same record.
                    let replaced_here = row.operands.iter().any(|other| {
                        other.role == Role::Replaced
                            && key(other.kind, other.id) == key(operand.kind, operand.id)
                    });
                    if !replaced_here && self.live.contains_key(&key(operand.kind, operand.id)) {
                        shape.reacquired_while_live += 1;
                    }
                }
                Role::Released => {
                    *shape
                        .releases
                        .entry(format!("{:?}", operand.kind))
                        .or_default() += 1;
                }
                _ => {}
            }
        }
        if acquired_any {
            *shape.acquiring_by_result.entry(class.into()).or_default() += 1;
        }
        if let Some(value) = row.args.get(1)
            && (shape.argument_1_values.len() < MAX_ARGUMENT_VALUES
                || shape.argument_1_values.contains_key(value))
        {
            *shape.argument_1_values.entry(*value).or_default() += 1;
        }
    }

    /// Liveness after the record, independent of the replay's own state.
    fn advance(&mut self, row: &Record) {
        for operand in &row.operands {
            let slot = key(operand.kind, operand.id);
            match operand.role {
                Role::Inherited | Role::Acquired => {
                    // A descriptor an inherited stdio stream owns is marked by
                    // that stream's `child` below, after its own operand.
                    let owned = self
                        .live
                        .get(&slot)
                        .is_some_and(|live| live.owned && live.generation == operand.generation);
                    self.live.insert(
                        slot,
                        Live {
                            kind: operand.kind,
                            generation: operand.generation,
                            owned,
                        },
                    );
                    // An inherited stdio stream owns its descriptor only if
                    // that descriptor is itself live (a process started with
                    // descriptor 0 closed still has a `stdin` object).
                    if let Some(child) = operand.child
                        && let Some(live) = self.live.get_mut(&key(Kind::Fd, child))
                    {
                        live.owned = true;
                    }
                }
                Role::Derived | Role::Adopted => {
                    self.live.insert(
                        slot,
                        Live {
                            kind: operand.kind,
                            generation: operand.generation,
                            owned: true,
                        },
                    );
                }
                Role::Released | Role::ReleasedChild | Role::Replaced => {
                    if self.live.get(&slot).is_some_and(|live| {
                        live.generation == operand.generation && live.kind == operand.kind
                    }) {
                        self.live.remove(&slot);
                    }
                }
                Role::Used | Role::Related | Role::Live => {}
            }
        }
    }
}

/// Bind one run's records to `contracts`. Errors mean the records are not
/// JSONL records of the observation schema, or a loaded contract has a shape
/// the binding cannot check; they never mean a contradiction.
pub fn bind_descriptor_contracts(
    source: &str,
    contracts: &ResourceContracts,
) -> Result<BindingReport, String> {
    let premises = declared_literal_argument_premises(contracts)?;
    let mut binder = Binder {
        contracts,
        premises,
        live: BTreeMap::new(),
        report: BindingReport::default(),
    };
    for (index, line) in source.lines().enumerate() {
        let row: Record =
            serde_json::from_str(line).map_err(|error| format!("record {}: {error}", index + 1))?;
        if row.sequence != index as u64 + 1 {
            return Err(format!("record {}: noncontiguous sequence", index + 1));
        }
        binder
            .bind(&row)
            .map_err(|error| format!("record {}: {error}", index + 1))?;
        binder.advance(&row);
        binder.report.records += 1;
    }
    binder.report.contradictions = binder
        .report
        .bound
        .values()
        .map(|binding| binding.contradictions)
        .sum();
    Ok(binder.report)
}

#[cfg(test)]
#[path = "../tests/unit/descriptor_contract_binding.rs"]
mod tests;
