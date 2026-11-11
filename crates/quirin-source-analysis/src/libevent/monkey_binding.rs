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

//! Replay imported callback paths against supplied native snapshots and call hooks.
//!
//! This is bounded execution refinement, not a static or plugin certificate.
//! The native producer is trusted for object liveness, layout, ordinary memory
//! snapshots, and exclusive access. Addresses are scoped to a registered event
//! and callback interval; they are not allocation provenance certificates.

use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;
use serde::Serialize;

use super::monkey_observation::{
    EventAction, MonkeyObservation, ObservationAction, check_monkey_observations,
};
use super::monkey_source::{
    MonkeySourcePath, MonkeySourceResult, SourceExpression as E, SourceOperation as O, SourceValue,
};
use crate::contract_observation::{
    ObservationVerdict, check_contract_facts as rule, check_contract_observation,
};
use crate::external_integration::PINNED_FLUENT_BIT_REVISION;
use qcore_schemas::profile::ImportRole;
pub use qcore_verification::callback_coverage::{
    CallbackCoverage as MonkeyCallbackCoverage,
    CallbackCoverageReport as MonkeyCallbackCoverageReport, Cycle18CallbackStatus,
};
use qcore_verification::callback_coverage::{
    CallbackExecutionEvidence, CallbackSourceInventory, account_callback_coverage,
};

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Binding {
    Registration {
        schema_version: u16,
        sequence: u64,
        event: u64,
        argument: u64,
        event_pointer: u64,
    },
    Callback(Callback),
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Callback {
    schema_version: u16,
    enter: u64,
    exit: u64,
    event: u64,
    fd: i32,
    flags: i16,
    argument: u64,
    event_size: u64,
    before: Vec<Cell>,
    after: Vec<Cell>,
    calls: Vec<Call>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Cell {
    object: u64,
    field: String,
    c_type: String,
    value: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
enum Argument {
    Integer(i64),
    String(String),
    Buffer(Buffer),
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct Buffer {
    value: u64,
    size: u64,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum CallOrigin {
    #[default]
    Native,
    InjectedFailure,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Call {
    #[serde(default)]
    origin: CallOrigin,
    function: String,
    arguments: Vec<Argument>,
    result: i64,
    sequence: u64,
    memory_value: i64,
}

fn is_errno_location(function: &str) -> bool {
    matches!(function, "__errno_location" | "__error")
}

fn same_source_call(imported: &str, observed: &str) -> bool {
    imported == observed || (is_errno_location(imported) && is_errno_location(observed))
}

#[derive(Debug, Serialize)]
pub struct CallbackBindingReport {
    pub function: String,
    pub checked_executions: usize,
    pub checked_paths: BTreeSet<usize>,
    pub native_paths: BTreeSet<usize>,
    pub injected_paths: BTreeSet<usize>,
    pub injected_executions: usize,
    pub unknown_paths: Vec<usize>,
    pub unknown_reasons: Vec<String>,
    pub checked_postconditions: usize,
}
#[derive(Debug, Serialize)]
pub struct MonkeyBindingReport {
    pub callbacks: Vec<CallbackBindingReport>,
    pub unknown: Vec<String>,
    pub static_certificate: bool,
}

fn callback_inventory() -> Result<(BTreeSet<String>, BTreeMap<String, String>), String> {
    let profile =
        crate::source_profiles::load_profile(crate::source_profiles::FLUENT_BIT_SOURCE_SET)
            .map_err(|error| error.to_string())?;
    let library = profile
        .library("monkey")
        .ok_or("source profile has no Monkey library")?;
    let typed = library
        .typed_import
        .as_ref()
        .ok_or("Monkey library has no typed import profile")?;
    let selected = typed
        .functions
        .iter()
        .filter_map(|(name, role)| (*role == ImportRole::Callback).then_some(name.clone()))
        .collect::<BTreeSet<_>>();
    let mut assumptions = profile.assumptions();
    assumptions.insert("library".into(), "monkey".into());
    Ok((selected, assumptions))
}

/// Bind the profile-derived callback denominator to source import and optional
/// native replay evidence. This never promotes replay to a Cycle 18
/// certificate; the missing source-to-event-program composition is retained on
/// every selected callback.
pub fn account_monkey_callbacks(
    summaries: &[MonkeySourceResult],
    binding: Option<&MonkeyBindingReport>,
) -> Result<MonkeyCallbackCoverageReport, String> {
    let (selected, assumptions) = callback_inventory()?;
    let inventory = summaries
        .iter()
        .map(|source| match source {
            MonkeySourceResult::Summary(summary) => CallbackSourceInventory {
                function: summary.function().to_owned(),
                source_revision: Some(summary.source_revision().to_owned()),
                selected_paths: Some(summary.paths().len()),
                unknown_reason: None,
            },
            MonkeySourceResult::Unknown { function, reason } => CallbackSourceInventory {
                function: function.clone(),
                source_revision: None,
                selected_paths: None,
                unknown_reason: Some(reason.clone()),
            },
        })
        .collect();
    let evidence = binding.map(|binding| {
        binding
            .callbacks
            .iter()
            .map(|row| CallbackExecutionEvidence {
                function: row.function.clone(),
                checked_paths: row.checked_paths.clone(),
                checked_executions: row.checked_executions,
            })
            .collect()
    });
    account_callback_coverage(&selected, assumptions, inventory, evidence)
}

/// Native QCore policy for the replay boundary, separate from callback postconditions.
pub const CALLBACK_BINDING_CONTRACTS: &str = include_str!(
    "../../../../providers/qcore-provider-libevent/contracts/monkey/callback_binding.qv"
);

type Memory = BTreeMap<(u64, String), (String, i64)>;

fn memory(cells: &[Cell]) -> Result<Memory, String> {
    let mut result = Memory::new();
    if cells.len() > 256 {
        return Err("snapshot exceeds bounded memory profile".into());
    }
    for cell in cells {
        if cell.object == 0
            || result
                .insert(
                    (cell.object, cell.field.clone()),
                    (cell.c_type.clone(), cell.value),
                )
                .is_some()
        {
            return Err("null or duplicate snapshot cell".into());
        }
        scalar(cell.value, &cell.c_type)?;
    }
    Ok(result)
}

// LP64 profile. Linux and macOS use different underlying unsigned types for
// `uint64_t`; both spellings remain subject to the same nonnegative i64 bound.
// Narrowing outside the representable range remains Unknown; implementation-
// defined conversions and modular arithmetic are not guessed.
fn scalar(value: i64, ty: &str) -> Result<i64, String> {
    let valid = match ty {
        "int" => i32::try_from(value).is_ok(),
        "short" => i16::try_from(value).is_ok(),
        "unsigned int" => u32::try_from(value).is_ok(),
        "long" => true,
        "unsigned long" | "unsigned long long" => value >= 0,
        _ if ty.ends_with('*') => value >= 0,
        _ => false,
    };
    if valid {
        Ok(value)
    } else {
        Err(format!("unsupported type or out-of-range value: {ty}"))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Value {
    Scalar(i64),
    Place(u64, String),
    Local(usize),
    Text(String),
}
impl Value {
    fn number(&self) -> Result<i64, String> {
        if let Self::Scalar(value) = self {
            Ok(*value)
        } else {
            Err("expected scalar value".into())
        }
    }
}

struct Replay<'a> {
    rules: &'a str,
    binding: &'a Callback,
    memory: Memory,
    symbols: BTreeMap<usize, (String, Value)>,
    locals: BTreeMap<usize, (String, Value)>,
    call_index: usize,
    writes: BTreeSet<(u64, String)>,
    releases: Vec<u64>,
    closed: usize,
}
impl Replay<'_> {
    fn eval(&self, value: &SourceValue) -> Result<Value, String> {
        let result = match &value.expression {
            E::LoopIndex { .. } => {
                return Err("loop induction value outside callback replay profile".into());
            }
            E::Aggregate { .. } => {
                return Err("aggregate initializer outside callback replay profile".into());
            }
            E::Function { .. } => {
                return Err("function pointer outside callback replay profile".into());
            }
            // CG01 literals (rule 10r): the callbacks import through the
            // legacy admission, so they cannot reach this replay; refused.
            E::Floating { .. } | E::Character { .. } => {
                return Err(
                    "CG01 floating/character literal outside callback replay profile".into(),
                );
            }
            E::Parameter { index } => Value::Scalar(match index {
                0 => self.binding.fd.into(),
                1 => self.binding.flags.into(),
                2 => self
                    .binding
                    .argument
                    .try_into()
                    .map_err(|_| "pointer out of range")?,
                _ => return Err("unknown parameter".into()),
            }),
            E::Integer { value } => Value::Scalar(value.parse().map_err(|_| "invalid integer")?),
            E::String { value } => Value::Text(value.clone()),
            E::LocalStorage { local } => {
                if !self.locals.contains_key(local) {
                    return Err("dead or uninitialized local".into());
                }
                Value::Local(*local)
            }
            E::SizeOf { argument_type } => Value::Scalar(match argument_type.as_str() {
                "unsigned long" | "unsigned long long" | "long" => 8,
                "int" | "unsigned int" => 4,
                "short" => 2,
                _ => return Err("sizeof outside LP64 scalar profile".into()),
            }),
            E::Symbol { operation } => {
                let producer = self.symbols.get(operation);
                rule(
                    self.rules,
                    "binding_symbol",
                    &[
                        ("producer_present", i128::from(producer.is_some())),
                        (
                            "producer_type_matches",
                            i128::from(producer.is_some_and(|(ty, _)| ty == &value.c_type)),
                        ),
                    ],
                )?;
                let (_, result) = producer.ok_or("absent prior symbol")?;
                result.clone()
            }
            E::Cast { cast, value: inner } => {
                let result = self.eval(inner)?;
                match cast.as_str() {
                    "IntegralCast" => Value::Scalar(scalar(result.number()?, &value.c_type)?),
                    "NoOp" if inner.c_type == value.c_type.trim_start_matches("const ") => result,
                    "BitCast" if inner.c_type.ends_with('*') && value.c_type.ends_with('*') => {
                        result
                    }
                    "ArrayToPointerDecay"
                        if matches!(result, Value::Place(..) | Value::Text(_)) =>
                    {
                        result
                    }
                    _ => return Err(format!("unmodeled cast: {cast}")),
                }
            }
            E::Unary {
                operator,
                value: inner,
            } => {
                let result = self.eval(inner)?;
                match operator.as_str() {
                    "&" => result,
                    "*" => Value::Place(
                        result
                            .number()?
                            .try_into()
                            .map_err(|_| "negative pointer")?,
                        String::new(),
                    ),
                    "-" => Value::Scalar(result.number()?.checked_neg().ok_or("integer overflow")?),
                    "!" => Value::Scalar(i64::from(result.number()? == 0)),
                    _ => return Err(format!("unmodeled unary operator: {operator}")),
                }
            }
            E::Binary {
                operator,
                left,
                right,
            } => {
                let l = self.eval(left)?.number()?;
                let r = self.eval(right)?.number()?;
                Value::Scalar(match operator.as_str() {
                    "+" => l.checked_add(r).ok_or("integer overflow")?,
                    "&" => l & r,
                    "|" => l | r,
                    "==" => i64::from(l == r),
                    "!=" => i64::from(l != r),
                    _ => return Err(format!("unmodeled binary operator: {operator}")),
                })
            }
            E::Field { base, name } => {
                let base = self
                    .eval(base)?
                    .number()?
                    .try_into()
                    .map_err(|_| "negative object")?;
                Value::Place(base, name.clone())
            }
            E::Index { base, index } => {
                let index = self.eval(index)?.number()?;
                if !(0..64).contains(&index) {
                    return Err("index outside bounded storage".into());
                }
                match self.eval(base)? {
                    Value::Place(object, field) if base.c_type == "int *" => {
                        Value::Place(object, format!("{field}[{index}]"))
                    }
                    Value::Scalar(address) if base.c_type == "struct mk_event *" => {
                        let stride =
                            i64::try_from(self.binding.event_size).map_err(|_| "invalid layout")?;
                        let offset = index.checked_mul(stride).ok_or("pointer overflow")?;
                        Value::Scalar(address.checked_add(offset).ok_or("pointer overflow")?)
                    }
                    _ => return Err("unmodeled array base".into()),
                }
            }
        };
        // A scalar expression's C range is checked at each intermediate step.
        if let Value::Scalar(number) = &result
            && value.c_type != "struct mk_event"
        {
            scalar(*number, &value.c_type)?;
        }
        Ok(result)
    }

    fn place(&self, value: &SourceValue) -> Result<(u64, String), String> {
        let Value::Place(object, field) = self.eval(value)? else {
            return Err("unmodeled memory place".into());
        };
        let cell = self.memory.get(&(object, field.clone()));
        rule(
            self.rules,
            "binding_memory",
            &[
                ("object", object.into()),
                ("cell_present", i128::from(cell.is_some())),
                (
                    "cell_type_matches",
                    i128::from(cell.is_some_and(|(ty, _)| ty == &value.c_type)),
                ),
            ],
        )?;
        // The interpreter still requires concrete typed storage even if a
        // caller supplies a weaker policy. Contracts cannot create cells.
        if cell.is_none_or(|(ty, _)| ty != &value.c_type) {
            return Err("absent or differently typed memory cell".into());
        }
        Ok((object, field))
    }

    fn argument(&self, value: &SourceValue) -> Result<Argument, String> {
        Ok(match self.eval(value)? {
            Value::Scalar(value) => Argument::Integer(value),
            Value::Text(value) => Argument::String(value.trim_matches('"').to_owned()),
            Value::Local(local) => {
                let (ty, value) = self.locals.get(&local).ok_or("dead call buffer")?;
                if !matches!(ty.as_str(), "unsigned long" | "unsigned long long") {
                    return Err("unmodeled call buffer type".into());
                }
                Argument::Buffer(Buffer {
                    value: value.number()?.try_into().map_err(|_| "negative buffer")?,
                    size: 8,
                })
            }
            _ => return Err("unmodeled call argument".into()),
        })
    }

    fn run(&mut self, path: &MonkeySourcePath) -> Result<(), String> {
        let mut returned = false;
        for (index, operation) in path.operations.iter().enumerate() {
            if returned {
                return Err("operation after return".into());
            }
            match operation {
                O::ScanLoop { .. } => {
                    return Err("scan loop outside callback replay profile".into());
                }
                O::Terminate => {
                    return Err("nonreturning path outside callback replay profile".into());
                }
                O::LocalStart { .. } => {
                    return Err("aggregate storage outside callback replay profile".into());
                }
                O::Read { place } => {
                    let key = self.place(place)?;
                    self.symbols.insert(
                        index,
                        (place.c_type.clone(), Value::Scalar(self.memory[&key].1)),
                    );
                }
                O::Write { place, value } => {
                    let key = self.place(place)?;
                    let result = scalar(self.eval(value)?.number()?, &place.c_type)?;
                    self.memory.get_mut(&key).expect("checked cell").1 = result;
                    self.writes.insert(key);
                }
                O::LocalWrite { local, value } => {
                    self.locals
                        .insert(*local, (value.c_type.clone(), self.eval(value)?));
                }
                O::LocalEnd { local } => {
                    self.locals.remove(local);
                }
                O::Guard { condition, taken } => {
                    if (self.eval(condition)?.number()? != 0) != *taken {
                        return Err("source guard differs from execution".into());
                    }
                }
                O::Call {
                    function,
                    arguments,
                    result_type,
                    ..
                } => {
                    let call = self
                        .binding
                        .calls
                        .get(self.call_index)
                        .ok_or("missing observed source call")?;
                    let arguments = arguments
                        .iter()
                        .map(|v| self.argument(v))
                        .collect::<Result<Vec<_>, _>>()?;
                    rule(
                        self.rules,
                        "binding_call_arguments",
                        &[
                            (
                                "function_matches",
                                i128::from(same_source_call(function, &call.function)),
                            ),
                            ("arguments_match", i128::from(arguments == call.arguments)),
                        ],
                    )?;
                    match function.as_str() {
                        "__errno_location" | "__error" => {
                            let address =
                                u64::try_from(call.result).map_err(|_| "invalid errno pointer")?;
                            rule(
                                self.rules,
                                "binding_errno_location",
                                &[
                                    ("result", call.result.into()),
                                    (
                                        "aliases_observed_object",
                                        i128::from(
                                            self.memory
                                                .keys()
                                                .any(|(object, _)| *object == address),
                                        ),
                                    ),
                                ],
                            )?;
                            scalar(call.memory_value, "int")?;
                            self.memory.insert(
                                (address, String::new()),
                                ("int".into(), call.memory_value),
                            );
                        }
                        "send" => {
                            let [
                                Argument::Integer(_),
                                Argument::Buffer(buffer),
                                Argument::Integer(length),
                                Argument::Integer(flags),
                            ] = call.arguments.as_slice()
                            else {
                                return Err(
                                    "send requires a descriptor, typed buffer, length and flags"
                                        .into(),
                                );
                            };
                            rule(
                                self.rules,
                                "binding_send",
                                &[
                                    ("buffer_size", buffer.size.into()),
                                    ("length", (*length).into()),
                                    ("flags", (*flags).into()),
                                    ("result", call.result.into()),
                                ],
                            )?;
                        }
                        "perror" | "evutil_closesocket" | "event_del" | "event_free" => {
                            rule(
                                self.rules,
                                &format!("binding_{function}"),
                                &[("result", call.result.into())],
                            )?;
                            self.closed += usize::from(function == "evutil_closesocket");
                        }
                        "mk_mem_free" => {
                            let [Argument::Integer(pointer)] = arguments.as_slice() else {
                                return Err("map release requires one pointer".into());
                            };
                            rule(
                                self.rules,
                                "binding_mk_mem_free",
                                &[
                                    ("pointer", (*pointer).into()),
                                    ("registered_argument", self.binding.argument.into()),
                                    ("result", call.result.into()),
                                    ("prior_releases", self.releases.len() as i128),
                                ],
                            )?;
                            let pointer =
                                u64::try_from(*pointer).map_err(|_| "invalid released pointer")?;
                            self.memory.retain(|(object, _), _| *object != pointer);
                            self.releases.push(pointer);
                        }
                        _ => return Err("unmodeled call effects".into()),
                    }
                    let result = if result_type == "void" {
                        Value::Scalar(0)
                    } else {
                        Value::Scalar(scalar(call.result, result_type)?)
                    };
                    self.symbols.insert(index, (result_type.clone(), result));
                    self.call_index += 1;
                }
                O::Return => returned = true,
                O::ReturnValue { .. } => {
                    return Err("value return outside void callback profile".into());
                }
            }
        }
        // errno belongs to the thread, outside the callback snapshot footprint.
        self.memory.retain(|(_, field), _| !field.is_empty());
        rule(
            self.rules,
            "binding_execution",
            &[
                ("returned", i128::from(returned)),
                ("consumed_calls", self.call_index as i128),
                ("observed_calls", self.binding.calls.len() as i128),
                (
                    "post_state_matches",
                    i128::from(self.memory == memory(&self.binding.after)?),
                ),
            ],
        )
    }
}

fn row(rows: &[MonkeyObservation], sequence: u64) -> Result<&MonkeyObservation, String> {
    rows.get(sequence.checked_sub(1).ok_or("zero sequence")? as usize)
        .ok_or_else(|| "binding refers outside observation".into())
}

// Every runtime effect inside a callback must have exactly one corresponding
// intercepted source call. The accepted v1 checker supplies generation/base/
// dispatcher/thread and active-event checks; no second observation is invented.
fn effects(
    binding: &Callback,
    rows: &[MonkeyObservation],
    event_pointer: u64,
    rules: &str,
) -> Result<(), String> {
    let mut consumed = BTreeSet::new();
    let mut prior = binding.enter;
    for call in &binding.calls {
        rule(
            rules,
            "binding_call_interval",
            &[
                ("sequence", call.sequence.into()),
                ("prior_sequence", prior.into()),
                ("callback_exit", binding.exit.into()),
                (
                    "injected",
                    i128::from(call.origin == CallOrigin::InjectedFailure),
                ),
                ("is_send", i128::from(call.function == "send")),
                ("result", call.result.into()),
            ],
        )?;
        prior = call.sequence;
        let observed = row(rows, call.sequence)?;
        let arg = |n| match call.arguments.get(n) {
            Some(Argument::Integer(value)) => Ok(*value),
            _ => Err("invalid effect argument"),
        };
        let runtime = match call.function.as_str() {
            "send" => {
                matches!(&observed.action, ObservationAction::DescriptorUse { descriptor, result } if i64::from(descriptor.fd) == arg(0)? && *result == call.result)
            }
            "evutil_closesocket" => {
                matches!(&observed.action, ObservationAction::DescriptorClose { descriptor, result } if i64::from(descriptor.fd) == arg(0)? && i64::from(*result) == call.result)
            }
            "event_del" | "event_free" => {
                if arg(0)? != event_pointer as i64 {
                    return Err("callback cleanup targets another event".into());
                }
                let expected = if call.function == "event_del" {
                    EventAction::EventDel
                } else {
                    EventAction::EventFree
                };
                matches!(&observed.action, ObservationAction::Event { operation, event, result, .. } if *operation == expected && *event == binding.event && i64::from(*result) == call.result)
            }
            "__errno_location" | "__error" | "perror" | "mk_mem_free" => continue,
            _ => return Err("unknown effect call".into()),
        };
        rule(
            rules,
            "binding_runtime_effect",
            &[
                ("effect_matches", i128::from(runtime)),
                (
                    "previously_consumed",
                    i128::from(consumed.contains(&call.sequence)),
                ),
            ],
        )?;
        consumed.insert(call.sequence);
    }
    for sequence in binding.enter + 1..binding.exit {
        if !consumed.contains(&sequence) {
            return Err("unbound runtime effect inside callback".into());
        }
    }
    Ok(())
}

/// Supply private, freshly imported summaries; serialized summaries cannot be
/// deserialized into this interface. Contracts are parsed/evaluated at runtime.
pub fn check_monkey_bindings(
    summaries: &[MonkeySourceResult],
    observations: &str,
    bindings: &str,
    contracts: &str,
) -> Result<MonkeyBindingReport, String> {
    check_monkey_bindings_with_rules(
        summaries,
        observations,
        bindings,
        contracts,
        CALLBACK_BINDING_CONTRACTS,
    )
}

/// Evaluate a supplied native rule source. This is an execution report, never
/// a static certificate; altered policy must be recorded by the evidence producer.
pub fn check_monkey_bindings_with_rules(
    summaries: &[MonkeySourceResult],
    observations: &str,
    bindings: &str,
    contracts: &str,
    rules: &str,
) -> Result<MonkeyBindingReport, String> {
    check_monkey_observations(observations)?;
    let rows = observations
        .lines()
        .map(serde_json::from_str)
        .collect::<Result<Vec<MonkeyObservation>, _>>()
        .map_err(|e| e.to_string())?;
    let mut registrations = BTreeMap::new();
    let mut executions = BTreeMap::new();
    for line in bindings.lines() {
        let binding: Binding = serde_json::from_str(line).map_err(|e| e.to_string())?;
        match binding {
            Binding::Registration {
                schema_version,
                sequence,
                event,
                argument,
                event_pointer,
            } => {
                if schema_version != 1
                    || event_pointer == 0
                    || argument == 0
                    || event_pointer > i64::MAX as u64
                    || argument > i64::MAX as u64
                    || !matches!(&row(&rows, sequence)?.action, ObservationAction::Event { operation: EventAction::EventNew, event: id, .. } if *id == event)
                    || registrations
                        .insert(event, (sequence, argument, event_pointer))
                        .is_some()
                {
                    return Err("invalid or duplicate registration binding".into());
                }
            }
            Binding::Callback(binding) => {
                if binding.schema_version != 1
                    || binding.enter >= binding.exit
                    || binding.event_size == 0
                    || binding.event_size > 4096
                    || binding.calls.len() > 32
                    || executions.insert(binding.enter, binding).is_some()
                {
                    return Err("invalid or duplicate callback binding".into());
                }
            }
        }
    }
    let mut by_function: BTreeMap<String, Vec<&Callback>> = BTreeMap::new();
    for observed in &rows {
        let ObservationAction::Event {
            operation,
            event,
            callback,
            descriptor,
            ..
        } = &observed.action
        else {
            continue;
        };
        if *operation == EventAction::EventNew && !registrations.contains_key(event) {
            return Err("missing registration payload binding".into());
        }
        if *operation != EventAction::CallbackEnter {
            continue;
        }
        let binding = executions
            .get(&observed.sequence)
            .ok_or("missing callback payload binding")?;
        let (created, argument, event_pointer) = registrations
            .get(event)
            .ok_or("absent callback registration")?;
        let exit = row(&rows, binding.exit)?;
        rule(
            rules,
            "binding_scope",
            &[
                ("created", (*created).into()),
                ("enter", binding.enter.into()),
                ("event", binding.event.into()),
                ("registered_event", (*event).into()),
                ("argument", binding.argument.into()),
                ("registered_argument", (*argument).into()),
                ("fd", binding.fd.into()),
                ("registered_fd", descriptor.map_or(-1, |key| key.fd).into()),
                ("exit_base", exit.base.into()),
                ("base", observed.base.into()),
                ("exit_dispatcher", exit.dispatcher.into()),
                ("dispatcher", observed.dispatcher.into()),
                ("exit_thread", exit.thread.into()),
                ("thread", observed.thread.into()),
                (
                    "exit_matches_event",
                    i128::from(
                        matches!(&exit.action, ObservationAction::Event { operation: EventAction::CallbackExit, event: id, .. } if id == event),
                    ),
                ),
            ],
        )?;
        effects(binding, &rows, *event_pointer, rules)?;
        by_function
            .entry(callback.clone())
            .or_default()
            .push(binding);
    }
    if by_function.values().map(Vec::len).sum::<usize>() != executions.len() {
        return Err("orphan callback binding".into());
    }
    let mut report = MonkeyBindingReport {
        callbacks: Vec::new(),
        unknown: vec![
            "allocation and registration failure source imports are not covered".into(),
            "allocation provenance and complete wrapper/map custody remain unproved".into(),
            "observed backend/dispatcher is checked by the runtime checker; no Cycle 18 event-program composition certificate is issued".into(),
        ],
        static_certificate: false,
    };
    let (selected_callbacks, _) = callback_inventory()?;
    let mut names = BTreeSet::new();
    for source in summaries {
        let (name, summary, reason) = match source {
            MonkeySourceResult::Summary(summary) => (summary.function(), Some(summary), None),
            MonkeySourceResult::Unknown { function, reason } => {
                (function.as_str(), None, Some(reason.clone()))
            }
        };
        if !selected_callbacks.contains(name) || !names.insert(name) {
            return Err("invalid callback source set".into());
        }
        let mut result = CallbackBindingReport {
            function: name.into(),
            checked_executions: 0,
            checked_paths: BTreeSet::new(),
            native_paths: BTreeSet::new(),
            injected_paths: BTreeSet::new(),
            injected_executions: 0,
            unknown_paths: Vec::new(),
            unknown_reasons: reason.into_iter().collect(),
            checked_postconditions: 0,
        };
        if let Some(summary) = summary {
            if summary.source_revision() != PINNED_FLUENT_BIT_REVISION {
                return Err("source revision mismatch".into());
            }
            for binding in by_function.get(name).into_iter().flatten() {
                let mut matches = Vec::new();
                let mut failures = BTreeSet::new();
                for (index, path) in summary.paths().iter().enumerate() {
                    let mut replay = Replay {
                        rules,
                        binding,
                        memory: memory(&binding.before)?,
                        symbols: BTreeMap::new(),
                        locals: BTreeMap::new(),
                        call_index: 0,
                        writes: BTreeSet::new(),
                        releases: Vec::new(),
                        closed: 0,
                    };
                    match replay.run(path) {
                        Ok(()) => matches.push((index, replay)),
                        Err(reason) => {
                            failures.insert(reason);
                        }
                    }
                }
                if let Err(reason) = rule(
                    rules,
                    "binding_select_path",
                    &[("refining_paths", matches.len() as i128)],
                ) {
                    result.unknown_reasons.push(format!(
                        "execution {}: {reason}; {}",
                        binding.enter,
                        failures.into_iter().collect::<Vec<_>>().join("; ")
                    ));
                    continue;
                }
                let [(index, replay)] = matches.as_slice() else {
                    result
                        .unknown_reasons
                        .push("ambiguous or absent replay cannot be interpreted".into());
                    continue;
                };
                let values = BTreeMap::from([
                    ("callback_argument_is_borrowed".into(), 1),
                    (
                        "fired_event_array_is_written".into(),
                        i128::from(
                            replay.writes.iter().any(|(_, field)| field == "fd")
                                && replay.writes.iter().any(|(_, field)| field == "mask")
                                && replay.writes.iter().any(|(_, field)| field == "data"),
                        ),
                    ),
                    (
                        "callback_argument_custody_is_preserved".into(),
                        i128::from(replay.releases.is_empty()),
                    ),
                    (
                        "descriptor_may_be_closed".into(),
                        i128::from(replay.closed <= 1),
                    ),
                    (
                        "event_map_may_be_released".into(),
                        i128::from(replay.releases.len() <= 1),
                    ),
                    (
                        "every_release_is_reflected_in_custody".into(),
                        i128::from(
                            replay
                                .releases
                                .iter()
                                .all(|object| !replay.memory.keys().any(|(key, _)| key == object)),
                        ),
                    ),
                    (
                        "callback_has_no_resource_effect".into(),
                        i128::from(binding.calls.is_empty() && replay.writes.is_empty()),
                    ),
                ]);
                match check_contract_observation(contracts, name, &values) {
                    Ok(ObservationVerdict::Checked { postconditions }) => {
                        result.checked_executions += 1;
                        result.checked_paths.insert(*index);
                        if binding
                            .calls
                            .iter()
                            .any(|call| call.origin == CallOrigin::InjectedFailure)
                        {
                            result.injected_executions += 1;
                            result.injected_paths.insert(*index);
                        } else {
                            result.native_paths.insert(*index);
                        }
                        result.checked_postconditions += postconditions;
                    }
                    other => result.unknown_reasons.push(format!(
                        "execution {}: native callback contract not checked: {other:?}",
                        binding.enter
                    )),
                }
            }
            result.unknown_paths = (0..summary.paths().len())
                .filter(|index| !result.checked_paths.contains(index))
                .collect();
            if !result.unknown_paths.is_empty() {
                result
                    .unknown_reasons
                    .push("unobserved source paths remain Unknown".into());
            }
        }
        report.callbacks.push(result);
    }
    if names.len() != selected_callbacks.len() {
        return Err("missing callback source result".into());
    }
    Ok(report)
}

#[cfg(test)]
#[path = "../../tests/unit/monkey_binding.rs"]
mod tests;
