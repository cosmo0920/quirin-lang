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

//! The bounded ChunkIO **unmap-wrapper composition**: a reading over other
//! capabilities' certificates rather than over one body alone.
//!
//! Relocated verbatim from `qcore_compiler::verifier_plugins::chunkio::
//! unmap_wrapper` (card P04-COMPOSE) so the retained in-process adapter and the
//! moved process route call **one** decision procedure and cannot drift. The
//! only change to the moved body is that the constants it read from its former
//! module scope — the pinned revision, the prerequisite's contract and record
//! profile, and the counter contract with its declared root types — are now
//! explicit parameters ([`UnmapCompositionContracts`]). The published
//! certificate's serialized shape is byte-identical, which the retained
//! adapter's archived corpus artifacts depend on.
//!
//! Both prerequisite certificates handed in are **re-evaluated here** from
//! their own retained summaries against their own pinned contracts, so a
//! certificate a caller supplies is never taken on trust.
//!
//! Nothing here observes or claims OS behaviour. A recorded `munmap` call is a
//! recorded call; whether a mapping was torn down is outside every claim. The
//! sync and logging frames remain explicit unproved external-call premises of
//! the pinned contract.

use qcore_c_gateway::typed_source::{
    SourceExpression as Expr, SourceOperation as Op, SourceSummary, SourceValue,
};
use qcore_contract::contract_observation::check_contract_facts;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

use crate::cleanup_outcome::{
    CleanupCertificate, CleanupEntry, CleanupProfile, check_cleanup_outcomes,
};
use crate::counter_contract::{CounterCertificate, check_counter_contract};

/// The pinned inputs the composition reads, which the ChunkIO adapter supplies.
/// None of them is inferred from a proposal: on the moved process route the
/// trusted core supplies its own contract texts and its own contract-derived
/// record profile.
pub struct UnmapCompositionContracts<'a> {
    /// The revision every participating summary must declare.
    pub pinned_revision: &'a str,
    /// The prerequisite cleanup body the wrapper calls.
    pub native_function: &'a str,
    /// The record shape the prerequisite replay evolves.
    pub native_profile: &'a CleanupProfile,
    /// The prerequisite's own pinned contract.
    pub native_contract: &'a str,
    /// The counter prerequisite's pinned contract and declared root types.
    pub counter_contract: &'a str,
    pub counter_root_c_type: &'a str,
    pub counter_root_qcore_type: &'a str,
    /// The composition's own pinned contract.
    pub contract: &'a str,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct UnmapCase {
    pub source_path: usize,
    pub backend_present: bool,
    pub initially_mapped: bool,
    pub initially_synced: bool,
    pub sync_result: Option<i32>,
    pub native_result: Option<i32>,
    pub native_source_path: Option<usize>,
    pub returned: i32,
    pub counter_decrements: usize,
    pub wrapper_writes: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct UnmapCertificate {
    source: SourceSummary,
    native: CleanupCertificate,
    counter: CounterCertificate,
    contract_source: String,
    cases: Vec<UnmapCase>,
}
impl UnmapCertificate {
    pub fn cases(&self) -> &[UnmapCase] {
        &self.cases
    }
    pub fn source(&self) -> &SourceSummary {
        &self.source
    }
    /// Re-run the whole composition from this certificate's own retained
    /// summaries. The caller supplies the pinned contracts; nothing is read out
    /// of the certificate's own recorded contract text.
    pub fn recheck(&self, contracts: &UnmapCompositionContracts<'_>) -> Result<Self, String> {
        check_unmap_composition(&self.source, &self.native, &self.counter, contracts)
    }
    /// Conditional counter projection, not a C pointer/provenance binding.
    pub fn counter_after(&self, case: usize, before: u128, maximum: u128) -> Result<u128, String> {
        let case = self
            .cases
            .get(case)
            .ok_or("unmap case index out of range")?;
        if before > maximum {
            return Err("counter outside supplied target range".into());
        }
        if case.counter_decrements == 1 {
            self.counter.instantiate(before, maximum)
        } else {
            Ok(before)
        }
    }
}
/// Compose the wrapper body with its two already-checked prerequisite
/// certificates.
pub fn check_unmap_composition(
    source: &SourceSummary,
    native: &CleanupCertificate,
    counter: &CounterCertificate,
    contracts: &UnmapCompositionContracts<'_>,
) -> Result<UnmapCertificate, String> {
    let UnmapCompositionContracts {
        pinned_revision,
        native_function,
        native_profile,
        native_contract,
        counter_contract,
        counter_root_c_type,
        counter_root_qcore_type,
        contract,
    } = contracts;
    if source.function() != "munmap_file"
        || source.signature() != "int (struct cio_ctx *, struct cio_chunk *)"
        || source.source_revision() != *pinned_revision
        || native.source().source_revision() != source.source_revision()
        || counter.source().source_revision() != source.source_revision()
        || native.source().function() != *native_function
    {
        return Err("unmap wrapper dependency source/signature/revision mismatch".into());
    }
    let native = check_cleanup_outcomes(native.source(), native_profile, native_contract)?;
    let counter = check_counter_contract(
        counter.source(),
        counter_contract,
        counter_root_c_type,
        counter_root_qcore_type,
    )?;
    if counter.field() != "total_chunks_up" || counter.delta() != -1 {
        return Err("unmap requires up-counter decrement".into());
    }
    let mut entries = vec![
        (false, false, false, None, None),
        (true, false, false, None, None),
    ];
    for (synced, sync) in [(false, Some(-1)), (false, Some(0)), (true, None)] {
        for outcome in [-1, 0] {
            entries.push((true, true, synced, sync, Some(outcome)));
        }
    }
    let mut cases = Vec::new();
    let mut covered = BTreeSet::new();
    for (backend, mapped, synced, sync, outcome) in entries {
        let mut selected = Vec::new();
        for (index, path) in source.paths().iter().enumerate() {
            let mut replay = Replay {
                backend,
                mapped,
                synced,
                sync,
                outcome,
                symbols: BTreeMap::new(),
                sync_calls: 0,
                logs: 0,
                native_calls: 0,
                counter_calls: 0,
                writes: Vec::new(),
            };
            if let Some(returned) = replay.run(&path.operations)? {
                let success = outcome == Some(0);
                if replay.sync_calls != usize::from(sync.is_some())
                    || replay.logs != usize::from(sync == Some(-1))
                    || replay.native_calls != usize::from(outcome.is_some())
                    || replay.counter_calls != usize::from(success)
                    || returned != if success { 0 } else { -1 }
                    || replay.writes
                        != if success {
                            vec!["data_size".to_string(), "alloc_size".to_string()]
                        } else {
                            vec![]
                        }
                {
                    return Err("unmap wrapper violates cleanup/counter outcome transition".into());
                }
                let native_path = if let Some(outcome) = outcome {
                    Some(
                        native
                            .cases()
                            .iter()
                            .find(|c| {
                                c.entry == CleanupEntry::Present
                                    && c.syscall_result == Some(outcome)
                                    && c.returned == outcome
                            })
                            .ok_or("native outcome dependency is incomplete")?
                            .source_path,
                    )
                } else {
                    None
                };
                check_contract_facts(
                    contract,
                    "unmap_file_outcomes",
                    &[
                        ("live_exclusive_chunk_and_nullable_owned_file_backend", 1),
                        ("parameter_context_is_live_chunk_context", 1),
                        ("chunk_backend_context_and_stream_have_disjoint_storage", 1),
                        ("sync_flag_is_boolean", 1),
                        (
                            "sync_returns_zero_or_minus_one_and_preserves_backend_and_mappedness",
                            1,
                        ),
                        (
                            "sync_and_logging_preserve_chunk_identities_mappedness_counter_and_lifetimes",
                            1,
                        ),
                        ("native_unmap_source_and_external_frames_are_bound", 1),
                        (
                            "up_counter_positive_and_in_target_range_when_decremented",
                            1,
                        ),
                        ("cross_unit_source_and_abi_bindings", 1),
                        ("was_mapped", i128::from(mapped)),
                        ("sync_failed", i128::from(sync == Some(-1))),
                        ("native_succeeded", i128::from(success)),
                        ("native_calls", replay.native_calls as i128),
                        ("diagnostic_calls", replay.logs as i128),
                        ("counter_decrements", replay.counter_calls as i128),
                        ("wrapper_writes", replay.writes.len() as i128),
                        ("returned", i128::from(returned)),
                        ("root_and_backend_preserved", 1),
                        ("saved_parameter_context_used", 1),
                    ],
                )?;
                selected.push(UnmapCase {
                    source_path: index,
                    backend_present: backend,
                    initially_mapped: mapped,
                    initially_synced: synced,
                    sync_result: sync,
                    native_result: outcome,
                    native_source_path: native_path,
                    returned,
                    counter_decrements: replay.counter_calls,
                    wrapper_writes: replay.writes,
                });
            }
        }
        if selected.len() != 1 {
            return Err("unmap entry must select exactly one complete path".into());
        }
        let case = selected.remove(0);
        covered.insert(case.source_path);
        cases.push(case);
    }
    if covered.len() != source.paths().len() {
        return Err("unmap source has uncovered paths".into());
    }
    Ok(UnmapCertificate {
        source: source.clone(),
        native,
        counter,
        contract_source: (*contract).into(),
        cases,
    })
}
#[derive(Clone, Debug, PartialEq, Eq)]
enum Value {
    Null,
    Chunk,
    Context,
    Stream,
    Backend,
    Mapping,
    ChunkName,
    StreamName,
    Integer(i32),
    Text(String),
    CounterResult,
    Void,
}
struct Replay {
    backend: bool,
    mapped: bool,
    synced: bool,
    sync: Option<i32>,
    outcome: Option<i32>,
    symbols: BTreeMap<usize, Value>,
    sync_calls: usize,
    logs: usize,
    native_calls: usize,
    counter_calls: usize,
    writes: Vec<String>,
}
impl Replay {
    fn value(&self, v: &SourceValue) -> Result<Value, String> {
        match &v.expression {
            Expr::Parameter { index: 0 } if v.c_type == "struct cio_ctx *" => Ok(Value::Context),
            Expr::Parameter { index: 1 } if v.c_type == "struct cio_chunk *" => Ok(Value::Chunk),
            Expr::Symbol { operation } => self
                .symbols
                .get(operation)
                .cloned()
                .ok_or("unmap missing producer".into()),
            Expr::Integer { value } => Ok(Value::Integer(
                value.parse().map_err(|_| "unmap integer out of range")?,
            )),
            Expr::String { value } => Ok(Value::Text(value.clone())),
            Expr::Cast { cast, value } => {
                let result = self.value(value)?;
                match cast.as_str() {
                    "NullToPointer"
                        if matches!(result, Value::Null | Value::Integer(0))
                            && v.c_type.ends_with('*') =>
                    {
                        Ok(Value::Null)
                    }
                    "BitCast" if value.c_type.ends_with('*') && v.c_type.ends_with('*') => {
                        Ok(result)
                    }
                    "NoOp"
                        if value.c_type == v.c_type
                            || (value.c_type.ends_with('*') && v.c_type.ends_with('*')) =>
                    {
                        Ok(result)
                    }
                    "ArrayToPointerDecay"
                        if matches!(result, Value::Text(_)) && v.c_type.ends_with('*') =>
                    {
                        Ok(result)
                    }
                    "IntegralCast"
                        if result == Value::Integer(0)
                            && matches!(
                                v.c_type.as_str(),
                                "unsigned long" | "unsigned long long"
                            ) =>
                    {
                        Ok(result)
                    }
                    _ => Err("unmap cast outside identity-preserving profile".into()),
                }
            }
            Expr::Unary { operator, value } if operator == "!" => {
                Ok(Value::Integer(i32::from(!Self::truth(self.value(value)?)?)))
            }
            Expr::Unary { operator, value } if operator == "-" => {
                let Value::Integer(n) = self.value(value)? else {
                    return Err("unmap symbolic negation".into());
                };
                Ok(Value::Integer(
                    n.checked_neg().ok_or("unmap integer overflow")?,
                ))
            }
            Expr::Binary {
                operator,
                left,
                right,
            } if operator == "==" || operator == "!=" => {
                let a = self.value(left)?;
                let b = self.value(right)?;
                let equal = match (&a, &b) {
                    (Value::Integer(a), Value::Integer(b)) => a == b,
                    (Value::Null, Value::Null) => true,
                    (Value::Null, Value::Backend | Value::Mapping)
                    | (Value::Backend | Value::Mapping, Value::Null) => false,
                    _ => return Err("unmap comparison outside Boolean/null profile".into()),
                };
                Ok(Value::Integer(i32::from(if operator == "==" {
                    equal
                } else {
                    !equal
                })))
            }
            _ => Err("unmap expression outside source profile".into()),
        }
    }
    fn truth(v: Value) -> Result<bool, String> {
        match v {
            Value::Null => Ok(false),
            Value::Backend | Value::Mapping => Ok(true),
            Value::Integer(n) => Ok(n != 0),
            _ => Err("unmap unresolved truth".into()),
        }
    }
    fn read(&self, place: &SourceValue) -> Result<Value, String> {
        let Expr::Field { base, name } = &place.expression else {
            return Err("unmap requires direct typed field read".into());
        };
        let value = match (
            self.value(base)?,
            base.c_type.as_str(),
            name.as_str(),
            place.c_type.as_str(),
        ) {
            (Value::Chunk, "struct cio_chunk *", "backend", "void *") => {
                if self.backend {
                    Value::Backend
                } else {
                    Value::Null
                }
            }
            (Value::Chunk, "struct cio_chunk *", "ctx", "struct cio_ctx *") => Value::Context,
            (Value::Chunk, "struct cio_chunk *", "st", "struct cio_stream *") => Value::Stream,
            (Value::Chunk, "struct cio_chunk *", "name", "char *") => Value::ChunkName,
            (Value::Stream, "struct cio_stream *", "name", "char *") => Value::StreamName,
            (Value::Backend, "struct cio_file *", "map", "char *") if self.native_calls == 0 => {
                if self.mapped {
                    Value::Mapping
                } else {
                    Value::Null
                }
            }
            (Value::Backend, "struct cio_file *", "synced", "int") if self.sync_calls == 0 => {
                Value::Integer(i32::from(self.synced))
            }
            _ => return Err("unmap unresolved field identity/type or post-call reload".into()),
        };
        Ok(value)
    }
    fn run(&mut self, ops: &[Op]) -> Result<Option<i32>, String> {
        for (i, op) in ops.iter().enumerate() {
            match op {
                Op::Read { place } => {
                    let v = self.read(place)?;
                    self.symbols.insert(i, v);
                }
                Op::LocalWrite { value, .. } => {
                    self.value(value)?;
                }
                Op::LocalStart { .. } | Op::LocalEnd { .. } => (),
                Op::Guard { condition, taken } => {
                    if Self::truth(self.value(condition)?)? != *taken {
                        return Ok(None);
                    }
                }
                Op::Write { place, value } => {
                    let Expr::Field { base, name } = &place.expression else {
                        return Err("unmap write outside direct backend".into());
                    };
                    if self.native_calls != 1
                        || self.outcome != Some(0)
                        || self.counter_calls != 0
                        || self.value(base)? != Value::Backend
                        || base.c_type != "struct cio_file *"
                        || !matches!(
                            place.c_type.as_str(),
                            "unsigned long" | "unsigned long long"
                        )
                        || !matches!(name.as_str(), "data_size" | "alloc_size")
                        || self.value(value)? != Value::Integer(0)
                    {
                        return Err("unmap write violates successful native outcome/frame".into());
                    }
                    self.writes.push(name.clone());
                }
                Op::Call {
                    function,
                    signature,
                    arguments,
                    result_type,
                } => {
                    let args = arguments
                        .iter()
                        .map(|v| self.value(v))
                        .collect::<Result<Vec<_>, _>>()?;
                    let result = match (function.as_str(), signature.as_str(), result_type.as_str())
                    {
                        ("cio_file_sync", "int (struct cio_chunk *)", "int")
                            if args == [Value::Chunk]
                                && self.sync.is_some()
                                && self.sync_calls == 0
                                && self.native_calls == 0 =>
                        {
                            self.sync_calls += 1;
                            Value::Integer(self.sync.unwrap())
                        }
                        (
                            "cio_log_print",
                            "void (void *, int, const char *, int, const char *, ...)",
                            "void",
                        ) if self.sync_calls == 1
                            && self.sync == Some(-1)
                            && self.logs == 0
                            && self.native_calls == 0
                            && matches!(args.as_slice(),[Value::Context,Value::Integer(1),Value::Text(_),Value::Integer(line),Value::Text(format),Value::StreamName,Value::ChunkName] if *line>0 && format == "\"[cio file] error syncing file at %s:%s\"") =>
                        {
                            self.logs += 1;
                            Value::Void
                        }
                        ("cio_file_native_unmap", "int (struct cio_file *)", "int")
                            if args == [Value::Backend]
                                && self.outcome.is_some()
                                && self.native_calls == 0
                                && self.sync_calls == usize::from(self.sync.is_some())
                                && self.logs == usize::from(self.sync == Some(-1)) =>
                        {
                            self.native_calls += 1;
                            Value::Integer(self.outcome.unwrap())
                        }
                        (
                            "cio_chunk_counter_total_up_sub",
                            "size_t (struct cio_ctx *)",
                            "unsigned long" | "unsigned long long",
                        ) if args == [Value::Context]
                            && self.native_calls == 1
                            && self.outcome == Some(0)
                            && self.counter_calls == 0
                            && self.writes == ["data_size", "alloc_size"] =>
                        {
                            self.counter_calls += 1;
                            Value::CounterResult
                        }
                        _ => {
                            return Err(format!(
                                "unmap call dependency/identity/order mismatch: {function}"
                            ));
                        }
                    };
                    self.symbols.insert(i, result);
                }
                Op::ReturnValue { value } if i + 1 == ops.len() && value.c_type == "int" => {
                    let Value::Integer(n) = self.value(value)? else {
                        return Err("unmap return unresolved".into());
                    };
                    return Ok(Some(n));
                }
                _ => return Err("unmap operation outside complete flat source profile".into()),
            }
        }
        Err("unmap path missing return".into())
    }
}
