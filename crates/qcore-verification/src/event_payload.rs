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

//! Source-derived `cb_event` payload footprints joined to bounded observations.
//!
//! The source half consumes only constructor-private [`SourceSummary`] values
//! produced by the typed C importer.  It derives identities and write targets
//! from every selected path and hashes the sealed summary.  The observation
//! half rechecks the exact payload hashes held by a validated Cycle 18 context,
//! derives storage extents from complete callback snapshots, and checks writes
//! against an explicit contextual authority declaration.  That declaration,
//! snapshot completeness, native producer truth, C layout, and exclusive
//! access remain named premises.  Success is a checked conditional boundary,
//! never a universal `Safe` verdict.

use std::collections::{BTreeMap, BTreeSet};

use qcore_c_gateway::typed_source::{
    SourceExpression, SourceOperation, SourcePath, SourceSummary, SourceValue,
};
use qcore_contract::contract_observation::{ContractFact, check_typed_contract_facts};
use serde::{Deserialize, Serialize};

use crate::event_composition_context::ValidatedEventCompositionContext;
use qcore_core::event_reactor::{CallbackSummary, PayloadId};

pub const EVENT_PAYLOAD_EVIDENCE_VERSION: &str = "qcore.cycle18.cb-event-payload.v1";
pub const EVENT_PAYLOAD_PROPERTY: &str = "cb-event-payload-boundary";
pub const EVENT_PAYLOAD_ALLOCATION_SCHEMA_VERSION: u16 = 1;
pub const EVENT_PAYLOAD_ALLOCATION_EVIDENCE_ASSUMPTION: &str =
    "event_payload_allocation_evidence_sha256";
pub const EVENT_PAYLOAD_CONTEXT_ASSUMPTIONS: [&str; 6] = [
    "event_payload_source_summary_sha256",
    "event_payload_allocation_summary_sha256",
    "event_payload_contract_sha256",
    "event_payload_authority_sha256",
    "event_payload_callback_summary_sha256",
    EVENT_PAYLOAD_ALLOCATION_EVIDENCE_ASSUMPTION,
];
pub const EVENT_PAYLOAD_CONTRACT: &str =
    include_str!("../../../providers/qcore-provider-libevent/contracts/monkey/event_payload.qv");

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Identity {
    Parameter(usize),
    LocalStorage(usize),
    Integer(String),
    Load(Box<Identity>),
    Field(Box<Identity>, String),
    Index(Box<Identity>, Box<Identity>),
    Unary(String, Box<Identity>),
    Binary(String, Box<Identity>, Box<Identity>),
}

impl Identity {
    fn canonical(&self) -> String {
        match self {
            Self::Parameter(index) => format!("param[{index}]"),
            Self::LocalStorage(local) => format!("local[{local}]"),
            Self::Integer(value) => value.clone(),
            Self::Load(place) => format!("load({})", place.canonical()),
            Self::Field(base, name) => format!("{}.{}", base.canonical(), name),
            Self::Index(base, index) => {
                format!("{}[{}]", base.canonical(), index.canonical())
            }
            Self::Unary(operator, value) => format!("{operator}({})", value.canonical()),
            Self::Binary(operator, left, right) => {
                format!("({} {operator} {})", left.canonical(), right.canonical())
            }
        }
    }
}

#[derive(Default)]
struct PathState {
    locals: BTreeMap<usize, Identity>,
    symbols: BTreeMap<usize, Identity>,
}

fn identity(value: &SourceValue, state: &PathState) -> Result<Identity, String> {
    match &value.expression {
        SourceExpression::Parameter { index } => Ok(Identity::Parameter(*index)),
        SourceExpression::LocalStorage { local } => Ok(Identity::LocalStorage(*local)),
        SourceExpression::Integer { value } => Ok(Identity::Integer(value.clone())),
        SourceExpression::Symbol { operation } => state
            .symbols
            .get(operation)
            .cloned()
            .ok_or_else(|| format!("symbol {operation} has no prior source producer")),
        SourceExpression::Cast { value, .. } => identity(value, state),
        SourceExpression::Unary { operator, value } if operator == "&" => identity(value, state),
        SourceExpression::Unary { operator, value } => Ok(Identity::Unary(
            operator.clone(),
            Box::new(identity(value, state)?),
        )),
        SourceExpression::Binary {
            operator,
            left,
            right,
        } => Ok(Identity::Binary(
            operator.clone(),
            Box::new(identity(left, state)?),
            Box::new(identity(right, state)?),
        )),
        SourceExpression::Field { base, name } => Ok(Identity::Field(
            Box::new(identity(base, state)?),
            name.clone(),
        )),
        SourceExpression::Index { base, index } => Ok(Identity::Index(
            Box::new(identity(base, state)?),
            Box::new(identity(index, state)?),
        )),
        _ => Err("unsupported identity expression in cb_event payload path".into()),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct DerivedPath {
    guards: Vec<(i64, bool)>,
    writes: BTreeMap<String, String>,
    event: String,
    map: String,
    context: String,
    fired_base: String,
    fired_index: String,
    mask_value: i64,
}

fn integer_value(value: &Identity) -> Result<i64, String> {
    match value {
        Identity::Integer(value) => value
            .parse()
            .map_err(|_| "source integer is outside i64".into()),
        Identity::Binary(operator, left, right) if operator == "|" => {
            Ok(integer_value(left)? | integer_value(right)?)
        }
        _ => Err("cb_event mask is not a bounded source integer expression".into()),
    }
}

fn guard_mask(value: &SourceValue) -> Result<i64, String> {
    let SourceExpression::Binary {
        operator,
        left,
        right,
    } = &value.expression
    else {
        return Err("cb_event guard is not a binary mask test".into());
    };
    if operator != "&" {
        return Err("cb_event guard does not use bitwise-and".into());
    }
    fn parameter_one(value: &SourceValue) -> bool {
        match &value.expression {
            SourceExpression::Parameter { index } => *index == 1,
            SourceExpression::Cast { value, .. } => parameter_one(value),
            _ => false,
        }
    }
    if !parameter_one(left) {
        return Err("cb_event guard is not derived from the flags parameter".into());
    }
    let SourceExpression::Integer { value } = &right.expression else {
        return Err("cb_event guard mask is not a source integer".into());
    };
    value
        .parse()
        .map_err(|_| "cb_event guard mask is outside i64".into())
}

fn derive_path(path: &SourcePath) -> Result<DerivedPath, String> {
    let mut state = PathState::default();
    let mut write_identities = BTreeMap::new();
    let mut guards = Vec::new();
    for (operation_index, operation) in path.operations.iter().enumerate() {
        match operation {
            SourceOperation::Read { place } => {
                let place = identity(place, &state)?;
                state
                    .symbols
                    .insert(operation_index, Identity::Load(Box::new(place)));
            }
            SourceOperation::LocalWrite { local, value } => {
                let value = identity(value, &state)?;
                state.locals.insert(*local, value.clone());
                state.symbols.insert(operation_index, value);
            }
            SourceOperation::Write { place, value } => {
                let target = identity(place, &state)?.canonical();
                let value = identity(value, &state)?;
                if write_identities.insert(target, value).is_some() {
                    return Err("cb_event writes one payload target more than once".into());
                }
            }
            SourceOperation::Guard { condition, taken } => {
                guards.push((guard_mask(condition)?, *taken));
            }
            SourceOperation::LocalEnd { local } => {
                state.locals.remove(local);
            }
            SourceOperation::Return => {}
            _ => return Err("unsupported operation in cb_event payload path".into()),
        }
    }

    let event = Identity::Parameter(2);
    let map = Identity::Load(Box::new(Identity::Field(
        Box::new(event.clone()),
        "data".into(),
    )));
    let context = Identity::Load(Box::new(Identity::Field(
        Box::new(map.clone()),
        "ctx".into(),
    )));
    let fired_base = Identity::Load(Box::new(Identity::Field(
        Box::new(context.clone()),
        "fired".into(),
    )));
    let fired_index = Identity::Load(Box::new(Identity::Field(
        Box::new(context.clone()),
        "fired_count".into(),
    )));
    let fired = Identity::Index(Box::new(fired_base.clone()), Box::new(fired_index.clone()));
    let writes = write_identities
        .iter()
        .map(|(target, value)| (target.clone(), value.canonical()))
        .collect::<BTreeMap<_, _>>();
    let expected = ["fd", "mask", "data"]
        .into_iter()
        .map(|field| Identity::Field(Box::new(fired.clone()), field.into()).canonical())
        .chain(std::iter::once(
            Identity::Field(Box::new(context.clone()), "fired_count".into()).canonical(),
        ))
        .collect::<BTreeSet<_>>();
    if writes.keys().cloned().collect::<BTreeSet<_>>() != expected {
        return Err("cb_event source path has a substituted or incomplete write footprint".into());
    }
    let fired_data = Identity::Field(Box::new(fired.clone()), "data".into()).canonical();
    let fired_fd = Identity::Field(Box::new(fired.clone()), "fd".into()).canonical();
    let fired_mask = Identity::Field(Box::new(fired.clone()), "mask".into()).canonical();
    let count = Identity::Field(Box::new(context.clone()), "fired_count".into()).canonical();
    let expected_fd = Identity::Load(Box::new(Identity::Field(
        Box::new(event.clone()),
        "fd".into(),
    )))
    .canonical();
    let expected_count = Identity::Binary(
        "+".into(),
        Box::new(fired_index.clone()),
        Box::new(Identity::Integer("1".into())),
    )
    .canonical();
    if writes.get(&fired_data) != Some(&event.canonical())
        || writes.get(&fired_fd) != Some(&expected_fd)
        || writes.get(&count) != Some(&expected_count)
    {
        return Err("cb_event source path writes substituted payload values".into());
    }
    let mask_value = integer_value(
        write_identities
            .get(&fired_mask)
            .ok_or("cb_event source path lacks the fired mask value")?,
    )?;
    Ok(DerivedPath {
        guards,
        writes,
        event: event.canonical(),
        map: map.canonical(),
        context: context.canonical(),
        fired_base: fired_base.canonical(),
        fired_index: fired_index.canonical(),
        mask_value,
    })
}

/// Explicit contextual premise for one callback interval.  It names a region
/// and fields; it is checked against source targets and observed snapshots but
/// is not claimed to be source-derived allocation provenance.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventPayloadWriteAuthority {
    /// Mapping from the observed callback argument to the event model namespace.
    pub callback_argument: u32,
    pub context_object: u64,
    pub fired_base: u64,
    pub writable_fired_fields: BTreeSet<String>,
    pub writable_context_fields: BTreeSet<String>,
    /// Mapping from the native context object to the event model namespace.
    pub context_payload: PayloadId,
    /// Mapping from the native fired allocation to the event model namespace.
    pub fired_payload: PayloadId,
}

/// Build the narrow callback model from the caller's declared payload authority.
/// This records a premise; the later boundary still checks both authority and
/// observed source effects. Audit orchestration does not construct core IR.
#[must_use]
pub fn declared_event_payload_callback(authority: &EventPayloadWriteAuthority) -> CallbackSummary {
    use qcore_core::event_reactor::{ActivationReason, ArgumentMode, PayloadAccessKind};
    CallbackSummary {
        argument: authority.callback_argument,
        argument_mode: ArgumentMode::Borrowed,
        trigger_reasons: BTreeSet::from([ActivationReason::Read, ActivationReason::Write]),
        payload_authority: BTreeMap::from([
            (authority.context_payload, PayloadAccessKind::Write),
            (authority.fired_payload, PayloadAccessKind::Write),
        ]),
        may_defer: false,
        may_spawn_tasks: false,
    }
}

/// Constructor-private source projection.  Callers cannot build source
/// authority from serialized or hand-authored operation rows.
#[derive(Clone, Debug, Serialize)]
pub struct EventPayloadProjection {
    function: String,
    signature: String,
    source_revision: String,
    source_summary_sha256: String,
    allocation_summary_sha256: String,
    selected_paths: usize,
    writes_per_path: Vec<usize>,
    derived_identity_chain: BTreeMap<String, String>,
    #[serde(skip)]
    paths: Vec<DerivedPath>,
}

impl EventPayloadProjection {
    #[must_use]
    pub fn source_revision(&self) -> &str {
        &self.source_revision
    }

    #[must_use]
    pub fn source_summary_sha256(&self) -> &str {
        &self.source_summary_sha256
    }

    #[must_use]
    pub fn selected_paths(&self) -> usize {
        self.selected_paths
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum EventPayloadProjectionOutcome {
    Projected(EventPayloadProjection),
    Unknown { reason: String },
}

/// Canonical identities a caller must place in the independently prepared
/// event-composition context before the contextual join can accept them.
pub fn event_payload_context_bindings(
    projection: &EventPayloadProjection,
    authority: &EventPayloadWriteAuthority,
    callback: &CallbackSummary,
) -> BTreeMap<String, String> {
    let authority_bytes = serde_json::to_vec(authority)
        .expect("EventPayloadWriteAuthority serialization is infallible");
    let callback_bytes = serde_json::to_vec(&serde_json::json!({
        "argument": callback.argument,
        "argument_mode": format!("{:?}", callback.argument_mode),
        "trigger_reasons": callback.trigger_reasons.iter().map(|v| format!("{v:?}")).collect::<Vec<_>>(),
        "payload_authority": callback.payload_authority.iter().map(|(id, kind)| (*id, format!("{kind:?}"))).collect::<Vec<_>>(),
        "may_defer": callback.may_defer,
        "may_spawn_tasks": callback.may_spawn_tasks,
    }))
    .expect("callback identity serialization is infallible");
    BTreeMap::from([
        (
            "event_payload_source_summary_sha256".into(),
            projection.source_summary_sha256.clone(),
        ),
        (
            "event_payload_allocation_summary_sha256".into(),
            projection.allocation_summary_sha256.clone(),
        ),
        (
            "event_payload_contract_sha256".into(),
            qcore_schemas::sha256_hex(EVENT_PAYLOAD_CONTRACT.as_bytes()),
        ),
        (
            "event_payload_authority_sha256".into(),
            qcore_schemas::sha256_hex(&authority_bytes),
        ),
        (
            "event_payload_callback_summary_sha256".into(),
            qcore_schemas::sha256_hex(&callback_bytes),
        ),
    ])
}

#[derive(Clone, Copy)]
pub struct EventPayloadEvidence<'a> {
    pub observations: &'a [u8],
    pub bindings: &'a [u8],
    /// Dedicated native allocation/layout record for the selected event loop.
    pub allocation: &'a [u8],
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct EventPayloadBoundaryReport {
    pub artifact: String,
    pub property: String,
    pub semantic_profile: String,
    pub scope: BTreeSet<String>,
    pub bounds: BTreeMap<String, u64>,
    pub assumptions: BTreeMap<String, String>,
    pub evidence_version: String,
    pub source_revision: String,
    pub source_summary_sha256: String,
    pub allocation_summary_sha256: String,
    pub contract_sha256: String,
    pub caller_prepared_context_sha256: String,
    pub allocation_evidence_sha256: String,
    pub selected_paths: usize,
    pub checked_executions: usize,
    pub accounted_non_cb_event_bindings: usize,
    pub accounted_observation_records: usize,
    pub checked_path_indices: BTreeSet<usize>,
    pub derived_identity_chain: BTreeMap<String, String>,
    pub writes_per_path: Vec<usize>,
    pub contextual_capacity: usize,
    pub contextual_element_size: u64,
    pub universal_safety_certificate: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum EventPayloadBoundary {
    Checked(Box<EventPayloadBoundaryReport>),
    Unknown { reason: String },
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum BindingRow {
    Registration {
        schema_version: u16,
        sequence: u64,
        event: u64,
        argument: u64,
        event_pointer: u64,
    },
    Callback(CallbackRow),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CallbackRow {
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
    calls: Vec<serde_json::Value>,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Cell {
    object: u64,
    field: String,
    c_type: String,
    value: i64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ObservationRow {
    schema_version: u16,
    sequence: u64,
    base: u64,
    dispatcher: u64,
    thread: u64,
    action: serde_json::Value,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventPayloadAllocationEvidence {
    pub schema_version: u16,
    pub context_object: u64,
    pub fired_base: u64,
    pub fired_capacity: usize,
    pub fired_element_size: u64,
    pub fired_allocation_bytes: u64,
    pub loop_size_argument: usize,
    pub context_allocation_bytes: u64,
    pub sizeof_context: u64,
}

fn json_lines<T: for<'de> Deserialize<'de>>(bytes: &[u8]) -> Result<Vec<T>, String> {
    let text = std::str::from_utf8(bytes).map_err(|_| "evidence is not UTF-8")?;
    if text.is_empty() {
        return Err("evidence payload is empty".into());
    }
    text.lines()
        .map(|line| serde_json::from_str(line).map_err(|e| e.to_string()))
        .collect()
}

type CellKey = (u64, String);
type CellValue<'a> = (&'a str, i64);
type CellMap<'a> = BTreeMap<CellKey, CellValue<'a>>;

fn cells(rows: &[Cell]) -> Result<CellMap<'_>, String> {
    let mut result = BTreeMap::new();
    for cell in rows {
        if cell.object == 0
            || cell.field.is_empty()
            || cell.c_type.is_empty()
            || result
                .insert(
                    (cell.object, cell.field.clone()),
                    (cell.c_type.as_str(), cell.value),
                )
                .is_some()
        {
            return Err("callback snapshot has a null or duplicate cell".into());
        }
    }
    Ok(result)
}

fn path_for_flags(paths: &[DerivedPath], flags: i16) -> Result<usize, String> {
    let matches = paths
        .iter()
        .enumerate()
        .filter(|(_, path)| {
            path.guards
                .iter()
                .all(|(mask, taken)| ((i64::from(flags) & mask) != 0) == *taken)
        })
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [index] => Ok(*index),
        _ => Err("callback flags do not select exactly one source path".into()),
    }
}

fn check_observation_interval(
    observations: &[ObservationRow],
    callback: &CallbackRow,
    callback_name: &str,
    run: (u64, u64, u64),
) -> Result<(), String> {
    let matches = |row: &ObservationRow, operation: &str, sequence: u64| {
        row.schema_version == 1
            && row.sequence == sequence
            && (row.base, row.dispatcher, row.thread) == run
            && row.action["kind"] == "event"
            && row.action["operation"] == operation
            && row.action["event"] == callback.event
            && row.action["callback"] == callback_name
            && (callback_name != "cb_event" || row.action["descriptor"]["fd"] == callback.fd)
    };
    if observations
        .iter()
        .filter(|row| matches(row, "callback_enter", callback.enter))
        .count()
        != 1
        || observations
            .iter()
            .filter(|row| matches(row, "callback_exit", callback.exit))
            .count()
            != 1
        || callback.enter >= callback.exit
    {
        return Err("callback binding lacks an exact observation enter/exit interval".into());
    }
    Ok(())
}

fn registered_callback(
    observations: &[ObservationRow],
    event: u64,
    sequence: u64,
) -> Result<(&str, (u64, u64, u64)), String> {
    let found = observations
        .iter()
        .filter(|row| {
            row.schema_version == 1
                && row.sequence == sequence
                && row.action["kind"] == "event"
                && row.action["operation"] == "event_new"
                && row.action["event"] == event
        })
        .collect::<Vec<_>>();
    let [row] = found.as_slice() else {
        return Err("registration lacks one exact event_new observation".into());
    };
    let name = row.action["callback"]
        .as_str()
        .filter(|name| !name.is_empty())
        .ok_or("event_new observation lacks callback identity")?;
    if row.base == 0 || row.dispatcher == 0 || row.thread == 0 {
        return Err("event_new observation lacks run identity".into());
    }
    Ok((name, (row.base, row.dispatcher, row.thread)))
}

fn checked(
    projection: &EventPayloadProjection,
    context: &ValidatedEventCompositionContext,
    evidence: EventPayloadEvidence<'_>,
    authority: &EventPayloadWriteAuthority,
    callback_summary: &CallbackSummary,
) -> Result<EventPayloadBoundaryReport, String> {
    let context_record = context.context();
    if qcore_schemas::sha256_hex(evidence.observations) != context_record.observations_sha256()
        || qcore_schemas::sha256_hex(evidence.bindings) != context_record.bindings_sha256()
    {
        return Err("payload bytes do not match the validated event context".into());
    }
    let allocation_sha256 = qcore_schemas::sha256_hex(evidence.allocation);
    if context_record
        .assumptions()
        .get(EVENT_PAYLOAD_ALLOCATION_EVIDENCE_ASSUMPTION)
        != Some(&allocation_sha256)
    {
        return Err("validated event context does not bind allocation evidence".into());
    }
    let allocation: EventPayloadAllocationEvidence =
        serde_json::from_slice(evidence.allocation).map_err(|e| e.to_string())?;
    let capacity_u64 = u64::try_from(allocation.fired_capacity)
        .map_err(|_| "allocation capacity is outside u64")?;
    capacity_u64
        .checked_mul(allocation.fired_element_size)
        .ok_or("native allocation extent overflow")?;
    check_typed_contract_facts(
        EVENT_PAYLOAD_CONTRACT,
        "native_allocation",
        &[
            (
                "schema_version",
                ContractFact::Integer(i128::from(allocation.schema_version)),
            ),
            (
                "expected_schema_version",
                ContractFact::Integer(i128::from(EVENT_PAYLOAD_ALLOCATION_SCHEMA_VERSION)),
            ),
            (
                "context_object",
                ContractFact::Integer(i128::from(allocation.context_object)),
            ),
            (
                "fired_base",
                ContractFact::Integer(i128::from(allocation.fired_base)),
            ),
            (
                "capacity",
                ContractFact::Integer(allocation.fired_capacity as i128),
            ),
            (
                "element_size",
                ContractFact::Integer(i128::from(allocation.fired_element_size)),
            ),
            (
                "allocation_bytes",
                ContractFact::Integer(i128::from(allocation.fired_allocation_bytes)),
            ),
            (
                "loop_size",
                ContractFact::Integer(allocation.loop_size_argument as i128),
            ),
            (
                "context_allocation_bytes",
                ContractFact::Integer(i128::from(allocation.context_allocation_bytes)),
            ),
            (
                "sizeof_context",
                ContractFact::Integer(i128::from(allocation.sizeof_context)),
            ),
        ],
    )?;
    if projection.function != "cb_event"
        || projection.source_revision != context_record.fluent_bit_revision()
        || !context_record.scope().contains("cb_event")
        || projection.selected_paths != 4
    {
        return Err("sealed source summary does not match the selected cb_event context".into());
    }
    for (name, expected) in event_payload_context_bindings(projection, authority, callback_summary)
    {
        if context_record.assumptions().get(&name) != Some(&expected) {
            return Err(format!("validated event context does not bind `{name}`"));
        }
    }
    let paths = &projection.paths;

    let observations: Vec<ObservationRow> = json_lines(evidence.observations)?;
    let bindings: Vec<BindingRow> = json_lines(evidence.bindings)?;
    let mut registrations = BTreeMap::new();
    let mut callbacks = Vec::new();
    for binding in bindings {
        match binding {
            BindingRow::Registration {
                schema_version,
                sequence,
                event,
                argument,
                event_pointer,
            } => {
                if schema_version != 1
                    || sequence == 0
                    || event == 0
                    || argument == 0
                    || event_pointer == 0
                    || registrations
                        .insert(event, (sequence, argument, event_pointer))
                        .is_some()
                {
                    return Err("invalid or duplicate callback registration".into());
                }
            }
            BindingRow::Callback(callback) => callbacks.push(callback),
        }
    }
    let mut observation_sequences = BTreeSet::new();
    if observations.iter().any(|row| {
        row.schema_version != 1 || row.sequence == 0 || !observation_sequences.insert(row.sequence)
    }) {
        return Err("observation stream has an invalid schema or duplicate sequence".into());
    }
    for (&event, &(sequence, _, _)) in &registrations {
        registered_callback(&observations, event, sequence)?;
    }
    let observed_intervals = observations
        .iter()
        .filter(|row| {
            row.action["kind"] == "event"
                && matches!(
                    row.action["operation"].as_str(),
                    Some("callback_enter" | "callback_exit")
                )
        })
        .count();
    if observed_intervals != callbacks.len() * 2 {
        return Err("callback bindings do not account for every observed interval".into());
    }
    let access = |payload| {
        callback_summary
            .payload_authority
            .get(&payload)
            .map(|kind| format!("{kind:?}"))
            .unwrap_or_else(|| "Missing".into())
    };
    check_typed_contract_facts(
        EVENT_PAYLOAD_CONTRACT,
        "contextual_authority",
        &[
            (
                "authority_context",
                ContractFact::Integer(i128::from(authority.context_object)),
            ),
            (
                "authority_fired_base",
                ContractFact::Integer(i128::from(authority.fired_base)),
            ),
            (
                "fired_fields",
                ContractFact::Text(
                    authority
                        .writable_fired_fields
                        .iter()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(","),
                ),
            ),
            (
                "context_fields",
                ContractFact::Text(
                    authority
                        .writable_context_fields
                        .iter()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(","),
                ),
            ),
            (
                "payload_count",
                ContractFact::Integer(callback_summary.payload_authority.len() as i128),
            ),
            (
                "context_payload",
                ContractFact::Integer(i128::from(authority.context_payload)),
            ),
            (
                "fired_payload",
                ContractFact::Integer(i128::from(authority.fired_payload)),
            ),
            (
                "context_access",
                ContractFact::Text(access(authority.context_payload)),
            ),
            (
                "fired_access",
                ContractFact::Text(access(authority.fired_payload)),
            ),
            (
                "model_argument",
                ContractFact::Integer(i128::from(callback_summary.argument)),
            ),
            (
                "authority_argument",
                ContractFact::Integer(i128::from(authority.callback_argument)),
            ),
            (
                "argument_mode",
                ContractFact::Text(format!("{:?}", callback_summary.argument_mode)),
            ),
            (
                "trigger_reasons",
                ContractFact::Text(
                    callback_summary
                        .trigger_reasons
                        .iter()
                        .map(|reason| format!("{reason:?}"))
                        .collect::<Vec<_>>()
                        .join(","),
                ),
            ),
            (
                "may_defer",
                ContractFact::Integer(i128::from(callback_summary.may_defer as u8)),
            ),
            (
                "may_spawn_tasks",
                ContractFact::Integer(i128::from(callback_summary.may_spawn_tasks as u8)),
            ),
        ],
    )?;

    let mut checked_paths = BTreeSet::new();
    let mut checked_executions = 0usize;
    let mut accounted_non_cb_event_bindings = 0usize;
    let mut callback_intervals = BTreeSet::new();
    let mut checked_layout = None;
    for callback in callbacks {
        let Some(&(registration_sequence, registered_argument, _event_pointer)) =
            registrations.get(&callback.event)
        else {
            return Err("callback binding has no registration".into());
        };
        if registered_argument != callback.argument {
            return Err("callback identity does not match its registration".into());
        }
        if !callback_intervals.insert((callback.event, callback.enter, callback.exit)) {
            return Err("duplicate callback binding interval".into());
        }
        let (callback_name, run) =
            registered_callback(&observations, callback.event, registration_sequence)?;
        check_observation_interval(&observations, &callback, callback_name, run)?;
        if callback_name != "cb_event" {
            accounted_non_cb_event_bindings += 1;
            continue;
        }
        if callback.schema_version != 1 || !callback.calls.is_empty() {
            return Err("cb_event callback binding has unsupported calls or schema".into());
        }
        let before = cells(&callback.before)?;
        let after = cells(&callback.after)?;
        if before.keys().ne(after.keys()) {
            return Err("callback snapshots do not cover the same storage cells".into());
        }
        let event = callback.argument;
        let path_index = path_for_flags(paths, callback.flags)?;
        let source_path = &paths[path_index];
        let map = before
            .get(&(event, "data".into()))
            .filter(|(ty, _)| *ty == "void *")
            .map(|(_, value)| *value)
            .and_then(|value| u64::try_from(value).ok())
            .ok_or("callback snapshot lacks the event-to-map identity")?;
        let context_object = before
            .get(&(map, "ctx".into()))
            .filter(|(ty, _)| *ty == "struct mk_event_ctx *")
            .map(|(_, value)| *value)
            .and_then(|value| u64::try_from(value).ok())
            .ok_or("callback snapshot lacks the map-to-context identity")?;
        let fired_base = before
            .get(&(context_object, "fired".into()))
            .filter(|(ty, _)| *ty == "struct mk_event *")
            .map(|(_, value)| *value)
            .and_then(|value| u64::try_from(value).ok())
            .ok_or("callback snapshot lacks the context-to-fired identity")?;
        let fired_index = before
            .get(&(context_object, "fired_count".into()))
            .filter(|(ty, _)| *ty == "int")
            .map(|(_, value)| *value)
            .ok_or("callback snapshot lacks the fired index")?;
        let queue_size = before
            .get(&(context_object, "queue_size".into()))
            .filter(|(ty, _)| matches!(*ty, "unsigned long" | "unsigned long long"))
            .map(|(_, value)| *value)
            .and_then(|value| usize::try_from(value).ok())
            .ok_or("callback snapshot lacks the source-initialized queue_size")?;
        if queue_size < 2 {
            return Err("observed fired extent is too small to derive a native stride".into());
        }
        let mut object_fields = BTreeMap::<u64, BTreeSet<String>>::new();
        for (object, field) in before.keys() {
            if authority.writable_fired_fields.contains(field) {
                object_fields
                    .entry(*object)
                    .or_default()
                    .insert(field.clone());
            }
        }
        let fired_objects = object_fields
            .into_iter()
            .filter_map(|(object, fields)| {
                (fields == authority.writable_fired_fields).then_some(object)
            })
            .collect::<BTreeSet<_>>();
        if fired_objects.len() != queue_size || fired_objects.first() != Some(&fired_base) {
            return Err(
                "callback snapshot does not expose the complete source-sized fired extent".into(),
            );
        }
        let fired_objects = fired_objects.into_iter().collect::<Vec<_>>();
        let stride = fired_objects[1]
            .checked_sub(fired_objects[0])
            .filter(|stride| *stride > 0)
            .ok_or("native fired layout has an invalid stride")?;
        if fired_objects
            .windows(2)
            .any(|pair| pair[1].checked_sub(pair[0]) != Some(stride))
            || callback.event_size != stride
        {
            return Err("native fired layout is not contiguous at sizeof(struct mk_event)".into());
        }
        if allocation.context_object != context_object
            || allocation.fired_base != fired_base
            || allocation.fired_capacity != queue_size
            || allocation.fired_element_size != stride
        {
            return Err(
                "native allocation record does not match callback storage identities".into(),
            );
        }
        if checked_layout
            .replace((queue_size, stride))
            .is_some_and(|prior| prior != (queue_size, stride))
        {
            return Err("callback executions disagree on the native fired extent".into());
        }
        let fired_index_i32 = i32::try_from(fired_index)
            .map_err(|_| "fired index is outside the source int domain")?;
        if context_object != authority.context_object
            || fired_base != authority.fired_base
            || fired_index_i32 < 0
            || fired_index_i32 == i32::MAX
            || usize::try_from(fired_index_i32).map_or(true, |index| index >= queue_size)
        {
            return Err(
                "callback identity, extent, or index is outside contextual authority".into(),
            );
        }
        for slot in 0..queue_size {
            let object = authority
                .fired_base
                .checked_add(
                    u64::try_from(slot)
                        .map_err(|_| "fired capacity is outside u64")?
                        .checked_mul(stride)
                        .ok_or("declared fired extent overflows")?,
                )
                .ok_or("declared fired extent address overflows")?;
            for field in &authority.writable_fired_fields {
                if !before.contains_key(&(object, field.clone())) {
                    return Err(
                        "callback snapshot is incomplete for the declared fired extent".into(),
                    );
                }
            }
        }
        let index = u64::try_from(fired_index_i32).map_err(|_| "negative fired index")?;
        let target = fired_base
            .checked_add(
                index
                    .checked_mul(stride)
                    .ok_or("fired slot offset overflow")?,
            )
            .ok_or("fired slot address overflow")?;
        let expected = BTreeSet::from([
            (target, "fd".into()),
            (target, "mask".into()),
            (target, "data".into()),
            (context_object, "fired_count".into()),
        ]);
        if expected.iter().any(|cell| !before.contains_key(cell)) {
            return Err("contextual extent does not contain every source-derived target".into());
        }
        if before.get(&(event, "fd".into())) != Some(&("int", i64::from(callback.fd)))
            || after.get(&(target, "fd".into())) != Some(&("int", i64::from(callback.fd)))
            || after.get(&(target, "mask".into()))
                != Some(&("unsigned int", source_path.mask_value))
            || after.get(&(target, "data".into()))
                != Some(&(
                    "void *",
                    i64::try_from(event).map_err(|_| "native event identity is outside i64")?,
                ))
        {
            return Err("callback post-state does not match source-derived payload values".into());
        }
        for (cell, prior) in &before {
            if !expected.contains(cell) && after.get(cell) != Some(prior) {
                return Err("callback changed a cell outside its declared write authority".into());
            }
        }
        let after_count = after
            .get(&(context_object, "fired_count".into()))
            .map(|(_, value)| *value)
            .ok_or("callback post-state lacks fired_count")?;
        if after_count != i64::from(fired_index_i32 + 1) {
            return Err("callback post-state does not increment fired_count once".into());
        }
        checked_paths.insert(path_index);
        checked_executions += 1;
        let offset = index.checked_mul(stride).ok_or("fired offset overflow")?;
        let extent = u64::try_from(queue_size)
            .map_err(|_| "fired capacity is outside u64")?
            .checked_mul(stride)
            .ok_or("fired extent overflow")?;
        check_typed_contract_facts(
            EVENT_PAYLOAD_CONTRACT,
            "contextual_payload_access",
            &[
                ("index", ContractFact::Integer(i128::from(fired_index_i32))),
                ("capacity", ContractFact::Integer(queue_size as i128)),
                ("signed_max", ContractFact::Integer(i128::from(i32::MAX))),
                ("element_size", ContractFact::Integer(i128::from(stride))),
                ("offset", ContractFact::Integer(i128::from(offset))),
                ("extent", ContractFact::Integer(i128::from(extent))),
                ("fired_base", ContractFact::Integer(i128::from(fired_base))),
                ("target", ContractFact::Integer(i128::from(target))),
                (
                    "context_identity",
                    ContractFact::Integer(i128::from(context_object)),
                ),
                (
                    "authority_context",
                    ContractFact::Integer(i128::from(authority.context_object)),
                ),
                (
                    "authority_fired_base",
                    ContractFact::Integer(i128::from(authority.fired_base)),
                ),
                (
                    "fired_fields",
                    ContractFact::Text(
                        authority
                            .writable_fired_fields
                            .iter()
                            .cloned()
                            .collect::<Vec<_>>()
                            .join(","),
                    ),
                ),
                (
                    "context_fields",
                    ContractFact::Text(
                        authority
                            .writable_context_fields
                            .iter()
                            .cloned()
                            .collect::<Vec<_>>()
                            .join(","),
                    ),
                ),
            ],
        )?;
    }
    if checked_paths.len() != paths.len() {
        return Err(
            "native event/context evidence does not cover every selected cb_event path".into(),
        );
    }

    let facts = [
        (
            "checked_paths",
            ContractFact::Integer(checked_paths.len() as i128),
        ),
        ("selected_paths", ContractFact::Integer(paths.len() as i128)),
    ];
    check_typed_contract_facts(
        EVENT_PAYLOAD_CONTRACT,
        "contextual_payload_boundary",
        &facts,
    )?;
    let mut assumptions = context_record.assumptions().clone();
    assumptions.insert(
        "payload_storage_extent".into(),
        "source-sized native allocation record joined to complete callback snapshots".into(),
    );
    assumptions.insert(
        "payload_write_authority".into(),
        "caller-declared exclusive callback-interval field authority".into(),
    );
    assumptions.insert(
        "source_import".into(),
        "sealed typed C importer; faithful Clang/C extraction remains a premise".into(),
    );
    let (contextual_capacity, contextual_element_size) =
        checked_layout.ok_or("no cb_event executions were checked")?;
    Ok(EventPayloadBoundaryReport {
        artifact: context_record.artifact().into(),
        property: EVENT_PAYLOAD_PROPERTY.into(),
        semantic_profile: context_record.semantic_profile().into(),
        scope: BTreeSet::from(["cb_event".into()]),
        bounds: BTreeMap::from([
            ("selected_paths".into(), paths.len() as u64),
            ("checked_executions".into(), checked_executions as u64),
            ("fired_capacity".into(), contextual_capacity as u64),
            ("fired_element_size".into(), contextual_element_size),
        ]),
        assumptions,
        evidence_version: EVENT_PAYLOAD_EVIDENCE_VERSION.into(),
        source_revision: projection.source_revision.clone(),
        source_summary_sha256: projection.source_summary_sha256.clone(),
        allocation_summary_sha256: projection.allocation_summary_sha256.clone(),
        contract_sha256: qcore_schemas::sha256_hex(EVENT_PAYLOAD_CONTRACT.as_bytes()),
        caller_prepared_context_sha256: context.context_sha256(),
        allocation_evidence_sha256: allocation_sha256,
        selected_paths: projection.selected_paths,
        checked_executions,
        accounted_non_cb_event_bindings,
        accounted_observation_records: observations.len(),
        checked_path_indices: checked_paths,
        derived_identity_chain: projection.derived_identity_chain.clone(),
        writes_per_path: projection.writes_per_path.clone(),
        contextual_capacity,
        contextual_element_size,
        universal_safety_certificate: false,
    })
}

/// Check the bounded payload property, retaining every unsupported obligation
/// as `Unknown`.  This API never emits `Safe` or an event-program certificate.
pub fn check_event_payload_boundary(
    projection: &EventPayloadProjection,
    context: &ValidatedEventCompositionContext,
    evidence: EventPayloadEvidence<'_>,
    authority: &EventPayloadWriteAuthority,
    callback_summary: &CallbackSummary,
) -> EventPayloadBoundary {
    match checked(projection, context, evidence, authority, callback_summary) {
        Ok(report) => EventPayloadBoundary::Checked(Box::new(report)),
        Err(reason) => EventPayloadBoundary::Unknown { reason },
    }
}

struct DerivedAllocationShape {
    allocation_function: String,
    allocation_argument: String,
    fired_store: String,
    queue_size_store: String,
}

fn allocation_shape(summary: &SourceSummary) -> Result<DerivedAllocationShape, String> {
    if summary.function() != "_mk_event_loop_create"
        || summary.signature() != "void *(int)"
        || summary.paths().len() != 4
    {
        return Err("sealed allocation summary is not _mk_event_loop_create".into());
    }
    let mut accepted = 0usize;
    for path in summary.paths() {
        let mut allocation = None;
        let mut fired_write = false;
        let mut queue_write = false;
        let mut returns_context = false;
        for (index, operation) in path.operations.iter().enumerate() {
            match operation {
                SourceOperation::Call {
                    function,
                    arguments,
                    ..
                } if function == "mk_mem_alloc_z" && arguments.len() == 1 => {
                    let SourceExpression::Binary {
                        operator,
                        left,
                        right,
                    } = &arguments[0].expression
                    else {
                        continue;
                    };
                    let source_size = matches!(
                        left.expression,
                        SourceExpression::SizeOf { ref argument_type }
                            if argument_type == "struct mk_event"
                    );
                    fn parameter_zero(value: &SourceValue) -> bool {
                        match &value.expression {
                            SourceExpression::Parameter { index } => *index == 0,
                            SourceExpression::Cast { value, .. } => parameter_zero(value),
                            _ => false,
                        }
                    }
                    if operator == "*" && source_size && parameter_zero(right) {
                        allocation = Some(index);
                    }
                }
                SourceOperation::Write { place, value } if matches!(&place.expression, SourceExpression::Field { name, .. } if name == "fired") =>
                {
                    fired_write = matches!(
                        &value.expression,
                        SourceExpression::Cast { value, .. }
                            if matches!(value.expression, SourceExpression::Symbol { operation } if Some(operation) == allocation)
                    );
                }
                SourceOperation::Write { place, value } if matches!(&place.expression, SourceExpression::Field { name, .. } if name == "queue_size") =>
                {
                    fn parameter_zero(value: &SourceValue) -> bool {
                        match &value.expression {
                            SourceExpression::Parameter { index } => *index == 0,
                            SourceExpression::Cast { value, .. } => parameter_zero(value),
                            _ => false,
                        }
                    }
                    queue_write = parameter_zero(value);
                }
                SourceOperation::ReturnValue { value } => {
                    returns_context = !matches!(
                        value.expression,
                        SourceExpression::Integer { ref value } if value == "0"
                    );
                }
                _ => {}
            }
        }
        if allocation.is_some() && fired_write && queue_write && returns_context {
            accepted += 1;
        }
    }
    if accepted != 1 {
        return Err("allocation summary lacks one exact successful fired extent path".into());
    }
    Ok(DerivedAllocationShape {
        allocation_function: "mk_mem_alloc_z".into(),
        allocation_argument: "sizeof(struct mk_event)*param[0]".into(),
        fired_store: "ctx.fired=allocation".into(),
        queue_size_store: "ctx.queue_size=param[0]".into(),
    })
}

fn projected(
    summary: &SourceSummary,
    allocation_summary: &SourceSummary,
) -> Result<EventPayloadProjection, String> {
    if summary.function() != "cb_event"
        || summary.paths().len() != 4
        || !matches!(
            summary.signature(),
            "void (int, short, void *)" | "void (evutil_socket_t, short, void *)"
        )
    {
        return Err("sealed source summary is not the bounded four-path cb_event".into());
    }
    if summary
        .profile_assumptions()
        .get("library")
        .map(String::as_str)
        != Some("monkey")
        || summary
            .profile_assumptions()
            .get("typed_import_role")
            .map(String::as_str)
            != Some("Callback")
        || allocation_summary
            .profile_assumptions()
            .get("library")
            .map(String::as_str)
            != Some("monkey")
        || allocation_summary
            .profile_assumptions()
            .get("typed_import_role")
            .map(String::as_str)
            != Some("Wrapper")
    {
        return Err("sealed source summary has the wrong import profile".into());
    }
    if allocation_summary.source_revision() != summary.source_revision() {
        return Err("callback and allocation summaries have different source revisions".into());
    }
    let allocation = allocation_shape(allocation_summary)?;
    let paths = summary
        .paths()
        .iter()
        .map(derive_path)
        .collect::<Result<Vec<_>, _>>()?;
    if paths
        .iter()
        .map(|path| &path.guards)
        .collect::<BTreeSet<_>>()
        .len()
        != paths.len()
    {
        return Err("cb_event source paths have duplicate guard alternatives".into());
    }
    let first = &paths[0];
    if paths.iter().any(|path| {
        path.writes.keys().ne(first.writes.keys())
            || path.event != first.event
            || path.map != first.map
            || path.context != first.context
            || path.fired_base != first.fired_base
            || path.fired_index != first.fired_index
    }) {
        return Err("cb_event source paths disagree on payload identities or writes".into());
    }
    let writes_per_path = paths
        .iter()
        .map(|path| path.writes.len())
        .collect::<Vec<_>>();
    check_typed_contract_facts(
        EVENT_PAYLOAD_CONTRACT,
        "cb_event_source_projection",
        &[
            ("function", ContractFact::Text(summary.function().into())),
            ("path_count", ContractFact::Integer(paths.len() as i128)),
            (
                "writes_per_path",
                ContractFact::Text(
                    writes_per_path
                        .iter()
                        .map(usize::to_string)
                        .collect::<Vec<_>>()
                        .join(","),
                ),
            ),
            ("event_identity", ContractFact::Text(first.event.clone())),
            ("map_identity", ContractFact::Text(first.map.clone())),
            (
                "context_identity",
                ContractFact::Text(first.context.clone()),
            ),
            (
                "fired_base_identity",
                ContractFact::Text(first.fired_base.clone()),
            ),
            (
                "fired_index_identity",
                ContractFact::Text(first.fired_index.clone()),
            ),
            (
                "allocation_function",
                ContractFact::Text(allocation.allocation_function),
            ),
            (
                "allocation_argument",
                ContractFact::Text(allocation.allocation_argument),
            ),
            ("fired_store", ContractFact::Text(allocation.fired_store)),
            (
                "queue_size_store",
                ContractFact::Text(allocation.queue_size_store),
            ),
        ],
    )?;
    let source_summary_sha256 =
        qcore_schemas::sha256_hex(&serde_json::to_vec(summary).map_err(|e| e.to_string())?);
    let allocation_summary_sha256 = qcore_schemas::sha256_hex(
        &serde_json::to_vec(allocation_summary).map_err(|e| e.to_string())?,
    );
    Ok(EventPayloadProjection {
        function: summary.function().into(),
        signature: summary.signature().into(),
        source_revision: summary.source_revision().into(),
        source_summary_sha256,
        allocation_summary_sha256,
        selected_paths: paths.len(),
        writes_per_path,
        derived_identity_chain: BTreeMap::from([
            ("event".into(), first.event.clone()),
            ("map".into(), first.map.clone()),
            ("context".into(), first.context.clone()),
            ("fired_base".into(), first.fired_base.clone()),
            ("fired_index".into(), first.fired_index.clone()),
        ]),
        paths,
    })
}

/// Derive the source-only footprint from a sealed typed-C summary.
pub fn project_event_payload(
    summary: &SourceSummary,
    allocation_summary: &SourceSummary,
) -> EventPayloadProjectionOutcome {
    match projected(summary, allocation_summary) {
        Ok(projection) => EventPayloadProjectionOutcome::Projected(projection),
        Err(reason) => EventPayloadProjectionOutcome::Unknown { reason },
    }
}
