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

//! Source and bounded-observation boundary for callback task handoff.
//!
//! The current Monkey source supplies three selected callbacks and eight
//! paths.  Their calls are opaque until each resolves to a declared Quirin
//! effect contract.  The accepted policy declares no task-spawn or deferred
//! effect for those callees.  Native records then establish only that the
//! selected callback windows contained no direct task/deferred action.

use std::collections::{BTreeMap, BTreeSet};

use qcore_c_gateway::typed_source::{
    SourceExpression, SourceOperation, SourceSummary, SourceValue,
};
use qcore_contract::contract_observation::{
    ContractFact, check_typed_contract_facts, contract_fact_literal,
};
use qcore_core::callback_task_handoff::{
    ArgumentLifetime, HandoffOperation, check_callback_task_handoff,
};
use qcore_schemas::sha256_hex;
use serde::{Deserialize, Serialize};

pub const CALLBACK_TASK_HANDOFF_POLICY: &str = include_str!(
    "../../../providers/qcore-provider-libevent/contracts/monkey/callback_task_handoff.qv"
);
pub const CALLBACK_TASK_HANDOFF_EVIDENCE_VERSION: &str = "qcore.cycle18.callback-task-handoff.v1";

const EFFECT_DECLARATIONS: [&str; 7] = [
    "effect_errno_location",
    "effect_event_del",
    "effect_event_free",
    "effect_evutil_closesocket",
    "effect_mk_mem_free",
    "effect_perror",
    "effect_send",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CallbackTaskHandoffError {
    Policy(String),
    Source(String),
    Observation(String),
    Model(String),
}

fn literal(function: &str, fact: &str) -> Result<ContractFact, CallbackTaskHandoffError> {
    contract_fact_literal(CALLBACK_TASK_HANDOFF_POLICY, function, fact)
        .map_err(CallbackTaskHandoffError::Policy)
}

fn declared_effect(callee: &str) -> Result<(), CallbackTaskHandoffError> {
    let mut matches = Vec::new();
    for declaration in EFFECT_DECLARATIONS {
        if literal(declaration, "callee")? == ContractFact::Text(callee.into()) {
            matches.push(declaration);
        }
    }
    let [declaration] = matches.as_slice() else {
        return Err(CallbackTaskHandoffError::Source(format!(
            "callee `{callee}` has no unique declared task/defer effect"
        )));
    };
    let spawn = literal(declaration, "task_spawn")?;
    let defer = literal(declaration, "defer")?;
    check_typed_contract_facts(
        CALLBACK_TASK_HANDOFF_POLICY,
        declaration,
        &[
            ("callee", ContractFact::Text(callee.into())),
            ("task_spawn", spawn),
            ("defer", defer),
        ],
    )
    .map_err(CallbackTaskHandoffError::Policy)?;
    if literal(declaration, "task_spawn")? != ContractFact::Integer(0)
        || literal(declaration, "defer")? != ContractFact::Integer(0)
    {
        return Err(CallbackTaskHandoffError::Source(format!(
            "callee `{callee}` can spawn tasks or defer work"
        )));
    }
    Ok(())
}

fn path_calls(operations: &[SourceOperation]) -> Result<Vec<String>, CallbackTaskHandoffError> {
    let mut calls = Vec::new();
    for operation in operations {
        match operation {
            SourceOperation::Call { function, .. } => {
                declared_effect(function)?;
                calls.push(function.clone());
            }
            SourceOperation::Read { .. }
            | SourceOperation::Write { .. }
            | SourceOperation::LocalWrite { .. }
            | SourceOperation::LocalStart { .. }
            | SourceOperation::Guard { .. }
            | SourceOperation::LocalEnd { .. }
            | SourceOperation::ReturnValue { .. }
            | SourceOperation::Return
            | SourceOperation::Terminate => {}
            SourceOperation::ScanLoop { .. } => {
                return Err(CallbackTaskHandoffError::Source(
                    "scan-loop effects are unsupported by callback handoff projection".into(),
                ));
            }
        }
    }
    Ok(calls)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CallbackTaskSourceProjection {
    callbacks: Vec<String>,
    path_counts: Vec<usize>,
    calls_by_callback_path: BTreeMap<String, Vec<Vec<String>>>,
    source_revision: String,
    source_digest: String,
    policy_digest: String,
}

impl CallbackTaskSourceProjection {
    pub fn callbacks(&self) -> &[String] {
        &self.callbacks
    }
    pub fn total_paths(&self) -> usize {
        self.path_counts.iter().sum()
    }
    pub fn source_revision(&self) -> &str {
        &self.source_revision
    }
    pub fn source_digest(&self) -> &str {
        &self.source_digest
    }
    pub fn policy_digest(&self) -> &str {
        &self.policy_digest
    }
}

/// Derive complete no-task/no-defer obligations for all selected callbacks.
pub fn project_callback_task_source(
    summaries: &[SourceSummary],
) -> Result<CallbackTaskSourceProjection, CallbackTaskHandoffError> {
    if summaries.is_empty() {
        return Err(CallbackTaskHandoffError::Source(
            "no selected callback summaries".into(),
        ));
    }
    let mut ordered = summaries.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|summary| summary.function());
    let source_revision = ordered[0].source_revision().to_owned();
    if ordered
        .iter()
        .any(|summary| summary.source_revision() != source_revision)
    {
        return Err(CallbackTaskHandoffError::Source(
            "selected callbacks do not share one source revision".into(),
        ));
    }
    let mut callbacks = Vec::new();
    let mut path_counts = Vec::new();
    let mut calls_by_callback_path = BTreeMap::new();
    for summary in &ordered {
        let assumptions = summary.profile_assumptions();
        check_typed_contract_facts(
            CALLBACK_TASK_HANDOFF_POLICY,
            "selected_callback_identity",
            &[
                ("callback", ContractFact::Text(summary.function().into())),
                ("signature", ContractFact::Text(summary.signature().into())),
                (
                    "library",
                    ContractFact::Text(assumptions.get("library").cloned().unwrap_or_default()),
                ),
                (
                    "source_set",
                    ContractFact::Text(assumptions.get("source_set").cloned().unwrap_or_default()),
                ),
                (
                    "import_role",
                    ContractFact::Text(
                        assumptions
                            .get("typed_import_role")
                            .cloned()
                            .unwrap_or_default(),
                    ),
                ),
                (
                    "profile_digest",
                    ContractFact::Text(
                        assumptions
                            .get("profile_digest")
                            .cloned()
                            .unwrap_or_default(),
                    ),
                ),
            ],
        )
        .map_err(CallbackTaskHandoffError::Source)?;
        let paths = summary
            .paths()
            .iter()
            .map(|path| path_calls(&path.operations))
            .collect::<Result<Vec<_>, _>>()?;
        callbacks.push(summary.function().to_owned());
        path_counts.push(paths.len());
        if calls_by_callback_path
            .insert(summary.function().to_owned(), paths)
            .is_some()
        {
            return Err(CallbackTaskHandoffError::Source(
                "duplicate selected callback".into(),
            ));
        }
    }
    let callback_names = callbacks.join(",");
    let counts = path_counts
        .iter()
        .map(usize::to_string)
        .collect::<Vec<_>>()
        .join(",");
    let total_paths = path_counts.iter().sum::<usize>();
    check_typed_contract_facts(
        CALLBACK_TASK_HANDOFF_POLICY,
        "selected_callbacks",
        &[
            (
                "source_revision",
                ContractFact::Text(source_revision.clone()),
            ),
            ("callback_names", ContractFact::Text(callback_names)),
            ("path_counts", ContractFact::Text(counts)),
            ("total_paths", ContractFact::Integer(total_paths as i128)),
        ],
    )
    .map_err(CallbackTaskHandoffError::Source)?;
    let source_digest = sha256_hex(&serde_json::to_vec(&ordered).map_err(|error| {
        CallbackTaskHandoffError::Source(format!("cannot encode source summaries: {error}"))
    })?);
    Ok(CallbackTaskSourceProjection {
        callbacks,
        path_counts,
        calls_by_callback_path,
        source_revision,
        source_digest,
        policy_digest: sha256_hex(CALLBACK_TASK_HANDOFF_POLICY.as_bytes()),
    })
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum CallbackTaskObservationAction {
    CallbackEnter { callback: String, argument: u32 },
    ExternalCall { callback: String, function: String },
    CallbackExit { callback: String },
    TaskSpawn { callback: String, task: u32 },
    TaskStart { task: u32 },
    TaskComplete { task: u32 },
    TaskCancelRequest { callback: String, task: u32 },
    TaskCancelAck { task: u32 },
    TaskJoin { task: u32 },
    DeferredRegister { callback: String, work: u32 },
    DeferredStart { work: u32 },
    DeferredComplete { work: u32 },
    ArgumentDrop { argument: u32 },
    Teardown { callback: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CallbackTaskObservation {
    pub schema_version: u16,
    pub sequence: u64,
    #[serde(default)]
    pub thread: Option<u64>,
    #[serde(default)]
    pub order: Option<String>,
    pub action: CallbackTaskObservationAction,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CallbackTaskObservationBundle {
    pub schema_version: u16,
    pub producer: String,
    pub capabilities: Vec<String>,
    pub records: Vec<CallbackTaskObservation>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CallbackTaskObservationCertificate {
    pub evidence_version: String,
    pub source_digest: String,
    pub policy_digest: String,
    pub callbacks_observed: Vec<String>,
    pub observation_count: usize,
    pub no_task_or_deferred_action_observed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskfulCallbackSourceProjection {
    operations: Vec<HandoffOperation>,
    callback_name: String,
    source_digest: String,
    policy_digest: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskfulFixtureContract {
    pub artifact: String,
    pub callback: String,
    pub after_function: String,
    pub source_revision: String,
    pub source_sha256: String,
    pub library: String,
    pub source_set: String,
    pub callback_role: String,
    pub after_role: String,
}

fn taskful_text(function: &str, fact: &str) -> Result<String, CallbackTaskHandoffError> {
    match literal(function, fact)? {
        ContractFact::Text(value) => Ok(value),
        ContractFact::Integer(_) => Err(CallbackTaskHandoffError::Policy(format!(
            "taskful policy fact `{fact}` is not text"
        ))),
    }
}

/// Load the fixture and profile identities from the executable Quirin policy.
pub fn taskful_fixture_contract() -> Result<TaskfulFixtureContract, CallbackTaskHandoffError> {
    Ok(TaskfulFixtureContract {
        artifact: taskful_text("taskful_fixture_source", "artifact")?,
        callback: taskful_text("taskful_fixture_source", "callback")?,
        after_function: taskful_text("taskful_fixture_source", "after_function")?,
        source_revision: taskful_text("taskful_fixture_source", "source_revision")?,
        source_sha256: taskful_text("taskful_fixture_source", "source_sha256")?,
        library: taskful_text("taskful_fixture_source", "library")?,
        source_set: taskful_text("taskful_fixture_source", "source_set")?,
        callback_role: taskful_text("taskful_fixture_source", "callback_role")?,
        after_role: taskful_text("taskful_fixture_source", "after_role")?,
    })
}

impl TaskfulCallbackSourceProjection {
    pub fn operations(&self) -> &[HandoffOperation] {
        &self.operations
    }
    pub fn source_digest(&self) -> &str {
        &self.source_digest
    }
    pub fn policy_digest(&self) -> &str {
        &self.policy_digest
    }
}

fn integer_argument(value: &SourceValue) -> Result<u32, CallbackTaskHandoffError> {
    match &value.expression {
        SourceExpression::Integer { value } => value.parse().map_err(|_| {
            CallbackTaskHandoffError::Source("taskful fixture identity is outside u32".into())
        }),
        SourceExpression::Cast { value, .. } => integer_argument(value),
        _ => Err(CallbackTaskHandoffError::Source(
            "taskful fixture identity is not a source integer".into(),
        )),
    }
}

fn exact_call_sequence(
    summary: &SourceSummary,
    ignored_calls: &BTreeSet<String>,
) -> Result<Vec<(String, Vec<u32>)>, CallbackTaskHandoffError> {
    let [path] = summary.paths() else {
        return Err(CallbackTaskHandoffError::Source(
            "taskful fixture function must have exactly one path".into(),
        ));
    };
    let mut calls = Vec::new();
    for operation in &path.operations {
        match operation {
            SourceOperation::Call {
                function,
                arguments,
                ..
            } => {
                if ignored_calls.contains(function) {
                    continue;
                }
                let arguments = arguments
                    .iter()
                    .map(integer_argument)
                    .collect::<Result<Vec<_>, _>>()?;
                calls.push((function.clone(), arguments));
            }
            SourceOperation::LocalStart { .. }
            | SourceOperation::LocalEnd { .. }
            | SourceOperation::Return => {}
            _ => {
                return Err(CallbackTaskHandoffError::Source(
                    "unsupported taskful fixture source operation".into(),
                ));
            }
        }
    }
    Ok(calls)
}

/// Project the separate bounded taskful C fixture into the generic kernel.
pub fn project_taskful_callback_source(
    artifact: &str,
    source_sha256: &str,
    callback: &SourceSummary,
    after_return: &SourceSummary,
) -> Result<TaskfulCallbackSourceProjection, CallbackTaskHandoffError> {
    if after_return.source_revision() != callback.source_revision() {
        return Err(CallbackTaskHandoffError::Source(
            "taskful fixture functions have different source revisions".into(),
        ));
    }
    let primitive = |name| taskful_text("taskful_fixture_primitives", name);
    let spawn = primitive("spawn")?;
    let defer_register = primitive("defer_register")?;
    let task_start = primitive("task_start")?;
    let cancel_request = primitive("cancel_request")?;
    let cancel_ack = primitive("cancel_ack")?;
    let task_join = primitive("task_join")?;
    let deferred_start = primitive("deferred_start")?;
    let deferred_complete = primitive("deferred_complete")?;
    let argument_drop = primitive("argument_drop")?;
    let teardown = primitive("teardown")?;
    let ignored_calls = [primitive("observe_enter")?, primitive("observe_exit")?]
        .into_iter()
        .collect();
    let callback_calls = exact_call_sequence(callback, &ignored_calls)?;
    let after_calls = exact_call_sequence(after_return, &ignored_calls)?;
    let render_calls = |calls: &[(String, Vec<u32>)]| {
        calls
            .iter()
            .map(|(name, arguments)| {
                format!(
                    "{name}({})",
                    arguments
                        .iter()
                        .map(u32::to_string)
                        .collect::<Vec<_>>()
                        .join(",")
                )
            })
            .collect::<Vec<_>>()
            .join(";")
    };
    let callback_assumptions = callback.profile_assumptions();
    let after_assumptions = after_return.profile_assumptions();
    let fact = |assumptions: &BTreeMap<String, String>, name: &str| {
        assumptions.get(name).cloned().ok_or_else(|| {
            CallbackTaskHandoffError::Source(format!("taskful fixture lacks `{name}` identity"))
        })
    };
    let first = callback_calls
        .iter()
        .find(|(name, _)| name == &spawn)
        .ok_or_else(|| {
            CallbackTaskHandoffError::Source("taskful callback has no spawn call".into())
        })?;
    let second = callback_calls
        .iter()
        .find(|(name, _)| name == &defer_register)
        .ok_or_else(|| {
            CallbackTaskHandoffError::Source("taskful callback has no deferred call".into())
        })?;
    let source_integer = |call: &(String, Vec<u32>), index: usize, name: &str| {
        call.1.get(index).copied().ok_or_else(|| {
            CallbackTaskHandoffError::Source(format!("taskful source lacks `{name}` argument"))
        })
    };
    let callback_id = source_integer(first, 0, "callback_id")?;
    let task_id = source_integer(first, 1, "task_id")?;
    let argument_id = source_integer(first, 2, "argument_id")?;
    let retains_argument = source_integer(first, 3, "retains_argument")?;
    let work_id = source_integer(second, 1, "work_id")?;
    check_typed_contract_facts(
        CALLBACK_TASK_HANDOFF_POLICY,
        "taskful_fixture_source",
        &[
            ("artifact", ContractFact::Text(artifact.into())),
            ("callback", ContractFact::Text(callback.function().into())),
            (
                "after_function",
                ContractFact::Text(after_return.function().into()),
            ),
            (
                "source_revision",
                ContractFact::Text(callback.source_revision().into()),
            ),
            ("source_sha256", ContractFact::Text(source_sha256.into())),
            (
                "library",
                ContractFact::Text(fact(callback_assumptions, "library")?),
            ),
            (
                "source_set",
                ContractFact::Text(fact(callback_assumptions, "source_set")?),
            ),
            (
                "callback_role",
                ContractFact::Text(fact(callback_assumptions, "typed_import_role")?),
            ),
            (
                "after_role",
                ContractFact::Text(fact(after_assumptions, "typed_import_role")?),
            ),
            (
                "callback_calls",
                ContractFact::Text(render_calls(&callback_calls)),
            ),
            (
                "after_calls",
                ContractFact::Text(render_calls(&after_calls)),
            ),
            ("callback_id", ContractFact::Integer(callback_id.into())),
            ("task_id", ContractFact::Integer(task_id.into())),
            ("work_id", ContractFact::Integer(work_id.into())),
            ("argument_id", ContractFact::Integer(argument_id.into())),
            (
                "retains_argument",
                ContractFact::Integer(retains_argument.into()),
            ),
        ],
    )
    .map_err(CallbackTaskHandoffError::Source)?;
    let mut operations = vec![HandoffOperation::CallbackEnter {
        callback: callback_id,
        argument: argument_id,
        argument_lifetime: ArgumentLifetime::Owned,
    }];
    let lower = |name: &str,
                 arguments: &[u32]|
     -> Result<HandoffOperation, CallbackTaskHandoffError> {
        match (name, arguments) {
            (name, &[callback, task, _, retains]) if name == spawn => {
                Ok(HandoffOperation::SpawnTask {
                    callback,
                    task,
                    retains_argument: retains == 1,
                })
            }
            (name, &[callback, work, _, retains]) if name == defer_register => {
                Ok(HandoffOperation::RegisterDeferred {
                    callback,
                    work,
                    retains_argument: retains == 1,
                })
            }
            (name, &[task]) if name == task_start => Ok(HandoffOperation::StartTask { task }),
            (name, &[callback, task]) if name == cancel_request => {
                Ok(HandoffOperation::RequestCancel { callback, task })
            }
            (name, &[task]) if name == cancel_ack => {
                Ok(HandoffOperation::AcknowledgeCancel { task })
            }
            (name, &[callback, task]) if name == task_join => {
                Ok(HandoffOperation::JoinTask { callback, task })
            }
            (name, &[work]) if name == deferred_start => {
                Ok(HandoffOperation::StartDeferred { work })
            }
            (name, &[work]) if name == deferred_complete => {
                Ok(HandoffOperation::CompleteDeferred { work })
            }
            (name, &[argument]) if name == argument_drop => {
                Ok(HandoffOperation::DropArgument { argument })
            }
            (name, &[callback]) if name == teardown => Ok(HandoffOperation::Teardown { callback }),
            _ => Err(CallbackTaskHandoffError::Source(format!(
                "unsupported taskful call `{name}`"
            ))),
        }
    };
    operations.extend(
        callback_calls
            .iter()
            .map(|(name, args)| lower(name, args))
            .collect::<Result<Vec<_>, _>>()?,
    );
    operations.push(HandoffOperation::CallbackExit {
        callback: callback_id,
    });
    operations.extend(
        after_calls
            .iter()
            .map(|(name, args)| lower(name, args))
            .collect::<Result<Vec<_>, _>>()?,
    );
    let source_digest = sha256_hex(&serde_json::to_vec(&(callback, after_return)).map_err(
        |error| CallbackTaskHandoffError::Source(format!("cannot encode taskful fixture: {error}")),
    )?);
    Ok(TaskfulCallbackSourceProjection {
        operations,
        callback_name: callback.function().into(),
        source_digest,
        policy_digest: sha256_hex(CALLBACK_TASK_HANDOFF_POLICY.as_bytes()),
    })
}

fn exact_capabilities(bundle: &CallbackTaskObservationBundle) -> bool {
    let capabilities = bundle.capabilities.iter().cloned().collect::<BTreeSet<_>>();
    bundle.schema_version == 1
        && !bundle.producer.is_empty()
        && bundle.capabilities.len() == capabilities.len()
        && capabilities
            == [
                "callback_boundaries".to_owned(),
                "task_lifecycle".to_owned(),
                "deferred_lifecycle".to_owned(),
            ]
            .into_iter()
            .collect()
}

/// Require the native taskful sequence to equal the source-derived operations.
pub fn check_taskful_callback_observations(
    source: &TaskfulCallbackSourceProjection,
    bundle: &CallbackTaskObservationBundle,
) -> Result<CallbackTaskObservationCertificate, CallbackTaskHandoffError> {
    if !exact_capabilities(bundle)
        || bundle.records.len() != source.operations.len()
        || bundle
            .records
            .windows(2)
            .any(|rows| rows[0].sequence >= rows[1].sequence)
    {
        return Err(CallbackTaskHandoffError::Observation(
            "taskful observer identity, capabilities, or ordering mismatch".into(),
        ));
    }
    if bundle
        .records
        .iter()
        .any(|record| record.thread.is_none() || record.order.as_deref() != Some("causal_replay"))
    {
        return Err(CallbackTaskHandoffError::Observation(
            "taskful records lack thread identity or causal-replay labelling".into(),
        ));
    }
    let callback_threads = bundle
        .records
        .iter()
        .filter(|record| {
            !matches!(
                record.action,
                CallbackTaskObservationAction::TaskStart { .. }
                    | CallbackTaskObservationAction::TaskCancelAck { .. }
            )
        })
        .filter_map(|record| record.thread)
        .collect::<BTreeSet<_>>();
    let worker_threads = bundle
        .records
        .iter()
        .filter(|record| {
            matches!(
                record.action,
                CallbackTaskObservationAction::TaskStart { .. }
                    | CallbackTaskObservationAction::TaskCancelAck { .. }
            )
        })
        .filter_map(|record| record.thread)
        .collect::<BTreeSet<_>>();
    if callback_threads.len() != 1
        || worker_threads.len() != 1
        || callback_threads == worker_threads
    {
        return Err(CallbackTaskHandoffError::Observation(
            "taskful worker/callback thread identities do not match causal roles".into(),
        ));
    }
    let mut observed = Vec::new();
    for (row, expected) in bundle.records.iter().zip(&source.operations) {
        if row.schema_version != 1 {
            return Err(CallbackTaskHandoffError::Observation(
                "unsupported taskful observation record".into(),
            ));
        }
        let matches_source = match (&row.action, expected) {
            (
                CallbackTaskObservationAction::CallbackEnter { callback, argument },
                HandoffOperation::CallbackEnter {
                    argument: source_argument,
                    ..
                },
            ) => callback == &source.callback_name && argument == source_argument,
            (
                CallbackTaskObservationAction::TaskSpawn { callback, task },
                HandoffOperation::SpawnTask {
                    task: source_task, ..
                },
            ) => callback == &source.callback_name && task == source_task,
            (
                CallbackTaskObservationAction::DeferredRegister { callback, work },
                HandoffOperation::RegisterDeferred {
                    work: source_work, ..
                },
            ) => callback == &source.callback_name && work == source_work,
            (
                CallbackTaskObservationAction::CallbackExit { callback },
                HandoffOperation::CallbackExit { .. },
            )
            | (
                CallbackTaskObservationAction::Teardown { callback },
                HandoffOperation::Teardown { .. },
            ) => callback == &source.callback_name,
            (
                CallbackTaskObservationAction::TaskStart { task },
                HandoffOperation::StartTask { task: source_task },
            )
            | (
                CallbackTaskObservationAction::TaskCancelAck { task },
                HandoffOperation::AcknowledgeCancel { task: source_task },
            )
            | (
                CallbackTaskObservationAction::TaskJoin { task },
                HandoffOperation::JoinTask {
                    task: source_task, ..
                },
            ) => task == source_task,
            (
                CallbackTaskObservationAction::TaskCancelRequest { callback, task },
                HandoffOperation::RequestCancel {
                    task: source_task, ..
                },
            ) => callback == &source.callback_name && task == source_task,
            (
                CallbackTaskObservationAction::DeferredStart { work },
                HandoffOperation::StartDeferred { work: source_work },
            )
            | (
                CallbackTaskObservationAction::DeferredComplete { work },
                HandoffOperation::CompleteDeferred { work: source_work },
            ) => work == source_work,
            (
                CallbackTaskObservationAction::ArgumentDrop { argument },
                HandoffOperation::DropArgument {
                    argument: source_argument,
                },
            ) => argument == source_argument,
            _ => false,
        };
        if !matches_source {
            return Err(CallbackTaskHandoffError::Observation(
                "taskful observation action differs from source-derived operation".into(),
            ));
        }
        observed.push(expected.clone());
    }
    if observed != source.operations {
        return Err(CallbackTaskHandoffError::Observation(
            "native taskful sequence differs from source-derived operations".into(),
        ));
    }
    let certificate = check_callback_task_handoff(&observed)
        .map_err(|error| CallbackTaskHandoffError::Model(format!("{error:?}")))?;
    if !certificate.remaining_argument_custody.is_empty() {
        return Err(CallbackTaskHandoffError::Model(
            "taskful native sequence leaves argument custody".into(),
        ));
    }
    Ok(CallbackTaskObservationCertificate {
        evidence_version: CALLBACK_TASK_HANDOFF_EVIDENCE_VERSION.into(),
        source_digest: source.source_digest.clone(),
        policy_digest: source.policy_digest.clone(),
        callbacks_observed: vec![source.callback_name.clone()],
        observation_count: bundle.records.len(),
        no_task_or_deferred_action_observed: false,
    })
}

/// Check bounded records for one complete invocation of every selected callback.
pub fn check_callback_task_observations(
    source: &CallbackTaskSourceProjection,
    bundle: &CallbackTaskObservationBundle,
) -> Result<CallbackTaskObservationCertificate, CallbackTaskHandoffError> {
    if !exact_capabilities(bundle) {
        return Err(CallbackTaskHandoffError::Observation(
            "observer lacks the exact callback/task/deferred completeness capabilities".into(),
        ));
    }
    let observations = &bundle.records;
    if observations.is_empty()
        || observations.iter().any(|row| row.schema_version != 1)
        || observations
            .windows(2)
            .any(|rows| rows[0].sequence >= rows[1].sequence)
    {
        return Err(CallbackTaskHandoffError::Observation(
            "observations are empty, unsupported, or not strictly ordered".into(),
        ));
    }
    let selected = source.callbacks.iter().cloned().collect::<BTreeSet<_>>();
    let mut active = BTreeMap::<String, (u32, u32)>::new();
    let mut finished = BTreeSet::new();
    let mut model = Vec::new();
    for row in observations {
        match &row.action {
            CallbackTaskObservationAction::CallbackEnter { callback, argument } => {
                if !selected.contains(callback)
                    || finished.contains(callback)
                    || active.contains_key(callback)
                {
                    return Err(CallbackTaskHandoffError::Observation(
                        "callback enter is unknown or duplicated".into(),
                    ));
                }
                let id = u32::try_from(
                    source
                        .callbacks
                        .iter()
                        .position(|name| name == callback)
                        .expect("selected callback")
                        + 1,
                )
                .map_err(|_| {
                    CallbackTaskHandoffError::Observation("callback id overflow".into())
                })?;
                active.insert(callback.clone(), (id, *argument));
                model.push(HandoffOperation::CallbackEnter {
                    callback: id,
                    argument: *argument,
                    argument_lifetime: ArgumentLifetime::BorrowedFrame,
                });
            }
            CallbackTaskObservationAction::ExternalCall { callback, function } => {
                let callback_calls = source
                    .calls_by_callback_path
                    .get(callback)
                    .into_iter()
                    .flatten()
                    .flatten()
                    .cloned()
                    .collect::<BTreeSet<_>>();
                if !active.contains_key(callback) || !callback_calls.contains(function) {
                    return Err(CallbackTaskHandoffError::Observation(
                        "external call is outside the selected source callback window".into(),
                    ));
                }
                declared_effect(function)?;
            }
            CallbackTaskObservationAction::CallbackExit { callback } => {
                let (id, _) = active.remove(callback).ok_or_else(|| {
                    CallbackTaskHandoffError::Observation(
                        "callback exit has no matching selected enter".into(),
                    )
                })?;
                if !finished.insert(callback.clone()) {
                    return Err(CallbackTaskHandoffError::Observation(
                        "callback exit is duplicated".into(),
                    ));
                }
                model.push(HandoffOperation::CallbackExit { callback: id });
                model.push(HandoffOperation::Teardown { callback: id });
            }
            CallbackTaskObservationAction::TaskSpawn { .. }
            | CallbackTaskObservationAction::TaskStart { .. }
            | CallbackTaskObservationAction::TaskComplete { .. }
            | CallbackTaskObservationAction::TaskCancelRequest { .. }
            | CallbackTaskObservationAction::TaskCancelAck { .. }
            | CallbackTaskObservationAction::TaskJoin { .. }
            | CallbackTaskObservationAction::DeferredRegister { .. }
            | CallbackTaskObservationAction::DeferredStart { .. }
            | CallbackTaskObservationAction::DeferredComplete { .. }
            | CallbackTaskObservationAction::ArgumentDrop { .. }
            | CallbackTaskObservationAction::Teardown { .. } => {
                return Err(CallbackTaskHandoffError::Observation(
                    "selected no-task callback window contains task or deferred work".into(),
                ));
            }
        }
    }
    if !active.is_empty() || finished != selected {
        return Err(CallbackTaskHandoffError::Observation(
            "observations do not cover each selected callback exactly once".into(),
        ));
    }
    check_callback_task_handoff(&model)
        .map_err(|error| CallbackTaskHandoffError::Model(format!("{error:?}")))?;
    check_typed_contract_facts(
        CALLBACK_TASK_HANDOFF_POLICY,
        "bounded_no_task_observation",
        &[
            (
                "selected_callback_count",
                ContractFact::Integer(selected.len() as i128),
            ),
            (
                "observed_callback_count",
                ContractFact::Integer(finished.len() as i128),
            ),
            ("task_action_count", ContractFact::Integer(0)),
            ("deferred_action_count", ContractFact::Integer(0)),
            ("callback_boundaries_complete", ContractFact::Integer(1)),
            ("task_lifecycle_complete", ContractFact::Integer(1)),
            ("deferred_lifecycle_complete", ContractFact::Integer(1)),
        ],
    )
    .map_err(CallbackTaskHandoffError::Observation)?;
    Ok(CallbackTaskObservationCertificate {
        evidence_version: CALLBACK_TASK_HANDOFF_EVIDENCE_VERSION.into(),
        source_digest: source.source_digest.clone(),
        policy_digest: source.policy_digest.clone(),
        callbacks_observed: finished.into_iter().collect(),
        observation_count: observations.len(),
        no_task_or_deferred_action_observed: true,
    })
}
