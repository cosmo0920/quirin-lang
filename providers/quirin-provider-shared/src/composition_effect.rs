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

//! A schemas-only proposal producer for the bounded ChunkIO **composition**
//! capability.
//!
//! This process is handed no composed result and no prerequisite conclusion —
//! the composition vocabulary cannot express either. It reads the supplied
//! wrapper paths, the supplied prerequisite paths and the supplied record
//! profile, **recomputes the prerequisite reading itself** with the same
//! symbolic replay this package performs for that capability standalone, and
//! then runs its own wrapper replay over the family's eight sampled entry
//! states: for each one it walks every complete wrapper path, decides which
//! single path that entry selects, and reports the return value, the counter
//! decrement, the wrapper's field writes and — the one figure that makes this a
//! composition — the *prerequisite's* own selected path for that outcome.
//!
//! It evaluates no contract, reads no policy, and cannot see any
//! prerequisite's verdict: the request carries paths, a profile and three
//! identity triples, and nothing else.
//!
//! The receiver re-derives every prerequisite task from its own import, refuses
//! unless each claimed triple is the identity it derives, re-runs each
//! prerequisite's own contract evaluation, builds its own record profile from
//! its own pinned contract, and re-runs its own unedited composition, before
//! anything is admitted.
//!
//! Nothing here observes or claims OS behaviour. A `munmap` frame in a path is
//! a recorded call; whether a mapping was torn down is not a question this
//! process can express.
use qcore_schemas::SemanticWitness;
use qcore_schemas::cleanup_replay_summary::{
    CleanupEntryRecord, CleanupProfileRecord, CleanupReplayCaseRecord,
    CleanupReplayExpressionRecord, CleanupReplayFunctionRecord, CleanupReplayOperationRecord,
    CleanupReplayRecord, CleanupReplayValueRecord,
};
use qcore_schemas::composition_summary::{
    CalleeIdentityRecord, CompositionFamilyInput, CompositionProposalRecord, CompositionRecord,
    CompositionSourceInput, CompositionWitness, DeletingCloseCaseRecord, DeletionOutcomeRecord,
    DerivedUnitRecord, FILE_CLOSE_DELETE_SEMANTIC_PROFILE, FILE_CLOSE_SEMANTIC_PROFILE,
    FILESYSTEM_CHUNK_CLOSE_SEMANTIC_PROFILE, FileCloseCaseRecord, FileCloseCompositionRecord,
    FileCloseDeleteInput, FilesystemChunkCloseCaseRecord, FilesystemChunkCloseCompositionRecord,
    MEMORY_CLOSE_SEMANTIC_PROFILE, MemoryCloseCalleeRecord, MemoryCloseCaseRecord,
    MemoryCloseCompositionRecord, ResidualCustodyRecord, UNMAP_WRAPPER_SEMANTIC_PROFILE,
    UNSUPPORTED_CLOSE_SEMANTIC_PROFILE, UnmapCaseRecord, UnsupportedCloseCompositionRecord,
};
use qcore_schemas::plugin::*;
use std::collections::{BTreeMap, BTreeSet};

/// Bounds applied to a supplied projection before any replay. These apply to
/// direct Rust callers as well as to transported JSON.
const MAX_PATHS: usize = 64;
const MAX_OPERATIONS: usize = 512;
/// Depth cap on the expression tree the replay walks. The importer's own
/// vocabulary is shallow; a deeper tree is declined rather than recursed into,
/// so a hostile projection cannot exhaust the stack.
const MAX_EXPRESSION_DEPTH: u32 = 32;

/// The wrapper's symbolic value domain, constructor for constructor with the
/// trusted composition's own private domain and in the same order. There is no
/// `Unknown`/`Other` constructor: an unresolved value is a decline, never an
/// opaque token.
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

/// One sampled entry state: the backend's presence, its mappedness, the prior
/// sync flag, the sync outcome and the native cleanup outcome.
type EntryState = (bool, bool, bool, Option<i32>, Option<i32>);

/// The eight sampled entry states this family answers over, in the exact order
/// the trusted composition enumerates them.
fn entry_states() -> Vec<EntryState> {
    let mut entries = vec![
        (false, false, false, None, None),
        (true, false, false, None, None),
    ];
    for (synced, sync) in [(false, Some(-1)), (false, Some(0)), (true, None)] {
        for outcome in [-1, 0] {
            entries.push((true, true, synced, sync, Some(outcome)));
        }
    }
    entries
}

fn bounded(body: &CleanupReplayFunctionRecord) -> bool {
    !body.function.is_empty()
        && !body.source_revision.is_empty()
        && !body.signature.is_empty()
        && !body.paths.is_empty()
        && body.paths.len() <= MAX_PATHS
        && body
            .paths
            .iter()
            .all(|path| !path.operations.is_empty() && path.operations.len() <= MAX_OPERATIONS)
}

struct Replay {
    backend: bool,
    mapped: bool,
    synced: bool,
    sync: Option<i32>,
    outcome: Option<i32>,
    symbols: BTreeMap<u32, Value>,
    sync_calls: u32,
    logs: u32,
    native_calls: u32,
    counter_calls: u32,
    writes: Vec<String>,
}

impl Replay {
    fn value(&self, v: &CleanupReplayValueRecord, depth: u32) -> Option<Value> {
        if depth > MAX_EXPRESSION_DEPTH {
            return None;
        }
        match &v.expression {
            CleanupReplayExpressionRecord::Parameter { index: 0 }
                if v.c_type == "struct cio_ctx *" =>
            {
                Some(Value::Context)
            }
            CleanupReplayExpressionRecord::Parameter { index: 1 }
                if v.c_type == "struct cio_chunk *" =>
            {
                Some(Value::Chunk)
            }
            CleanupReplayExpressionRecord::Symbol { operation } => {
                self.symbols.get(operation).cloned()
            }
            CleanupReplayExpressionRecord::Integer { value } => {
                Some(Value::Integer(value.parse().ok()?))
            }
            CleanupReplayExpressionRecord::String { value } => Some(Value::Text(value.clone())),
            CleanupReplayExpressionRecord::Cast { cast, value } => {
                let result = self.value(value, depth + 1)?;
                match cast.as_str() {
                    "NullToPointer"
                        if matches!(result, Value::Null | Value::Integer(0))
                            && v.c_type.ends_with('*') =>
                    {
                        Some(Value::Null)
                    }
                    "BitCast" if value.c_type.ends_with('*') && v.c_type.ends_with('*') => {
                        Some(result)
                    }
                    "NoOp"
                        if value.c_type == v.c_type
                            || (value.c_type.ends_with('*') && v.c_type.ends_with('*')) =>
                    {
                        Some(result)
                    }
                    "ArrayToPointerDecay"
                        if matches!(result, Value::Text(_)) && v.c_type.ends_with('*') =>
                    {
                        Some(result)
                    }
                    "IntegralCast"
                        if result == Value::Integer(0) && v.c_type == "unsigned long" =>
                    {
                        Some(result)
                    }
                    _ => None,
                }
            }
            CleanupReplayExpressionRecord::Unary { operator, value } if operator == "!" => Some(
                Value::Integer(i32::from(!Self::truth(self.value(value, depth + 1)?)?)),
            ),
            CleanupReplayExpressionRecord::Unary { operator, value } if operator == "-" => {
                let Value::Integer(n) = self.value(value, depth + 1)? else {
                    return None;
                };
                Some(Value::Integer(n.checked_neg()?))
            }
            CleanupReplayExpressionRecord::Binary {
                operator,
                left,
                right,
            } if operator == "==" || operator == "!=" => {
                let a = self.value(left, depth + 1)?;
                let b = self.value(right, depth + 1)?;
                let equal = match (&a, &b) {
                    (Value::Integer(a), Value::Integer(b)) => a == b,
                    (Value::Null, Value::Null) => true,
                    (Value::Null, Value::Backend | Value::Mapping)
                    | (Value::Backend | Value::Mapping, Value::Null) => false,
                    _ => return None,
                };
                Some(Value::Integer(i32::from(if operator == "==" {
                    equal
                } else {
                    !equal
                })))
            }
            _ => None,
        }
    }

    fn truth(v: Value) -> Option<bool> {
        match v {
            Value::Null => Some(false),
            Value::Backend | Value::Mapping => Some(true),
            Value::Integer(n) => Some(n != 0),
            _ => None,
        }
    }

    fn read(&self, place: &CleanupReplayValueRecord) -> Option<Value> {
        let CleanupReplayExpressionRecord::Field { base, name } = &place.expression else {
            return None;
        };
        let value = match (
            self.value(base, 0)?,
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
            _ => return None,
        };
        Some(value)
    }

    /// `Some(Some(n))`: this path completed and returned `n`. `Some(None)`: a
    /// guard did not hold, so this entry state does not select this path.
    /// `None`: the projection is outside the vocabulary, so decline entirely.
    fn run(&mut self, ops: &[CleanupReplayOperationRecord]) -> Option<Option<i32>> {
        for (i, op) in ops.iter().enumerate() {
            let i = u32::try_from(i).ok()?;
            match op {
                CleanupReplayOperationRecord::Read { place } => {
                    let v = self.read(place)?;
                    self.symbols.insert(i, v);
                }
                CleanupReplayOperationRecord::LocalWrite { value, .. } => {
                    self.value(value, 0)?;
                }
                CleanupReplayOperationRecord::LocalStart { .. }
                | CleanupReplayOperationRecord::LocalEnd { .. } => (),
                CleanupReplayOperationRecord::Guard { condition, taken } => {
                    if Self::truth(self.value(condition, 0)?)? != *taken {
                        return Some(None);
                    }
                }
                CleanupReplayOperationRecord::Write { place, value } => {
                    let CleanupReplayExpressionRecord::Field { base, name } = &place.expression
                    else {
                        return None;
                    };
                    if self.native_calls != 1
                        || self.outcome != Some(0)
                        || self.counter_calls != 0
                        || self.value(base, 0)? != Value::Backend
                        || base.c_type != "struct cio_file *"
                        || place.c_type != "unsigned long"
                        || !matches!(name.as_str(), "data_size" | "alloc_size")
                        || self.value(value, 0)? != Value::Integer(0)
                    {
                        return None;
                    }
                    self.writes.push(name.clone());
                }
                CleanupReplayOperationRecord::Call {
                    function,
                    signature,
                    arguments,
                    result_type,
                } => {
                    let args = arguments
                        .iter()
                        .map(|v| self.value(v, 0))
                        .collect::<Option<Vec<_>>>()?;
                    let result = match (function.as_str(), signature.as_str(), result_type.as_str())
                    {
                        ("cio_file_sync", "int (struct cio_chunk *)", "int")
                            if args == [Value::Chunk]
                                && self.sync.is_some()
                                && self.sync_calls == 0
                                && self.native_calls == 0 =>
                        {
                            self.sync_calls += 1;
                            Value::Integer(self.sync?)
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
                                && self.sync_calls == u32::from(self.sync.is_some())
                                && self.logs == u32::from(self.sync == Some(-1)) =>
                        {
                            self.native_calls += 1;
                            Value::Integer(self.outcome?)
                        }
                        (
                            "cio_chunk_counter_total_up_sub",
                            "size_t (struct cio_ctx *)",
                            "unsigned long",
                        ) if args == [Value::Context]
                            && self.native_calls == 1
                            && self.outcome == Some(0)
                            && self.counter_calls == 0
                            && self.writes == ["data_size", "alloc_size"] =>
                        {
                            self.counter_calls += 1;
                            Value::CounterResult
                        }
                        _ => return None,
                    };
                    self.symbols.insert(i, result);
                }
                CleanupReplayOperationRecord::ReturnValue { value }
                    if usize::try_from(i).ok()? + 1 == ops.len() && value.c_type == "int" =>
                {
                    let Value::Integer(n) = self.value(value, 0)? else {
                        return None;
                    };
                    return Some(Some(n));
                }
                CleanupReplayOperationRecord::ReturnValue { .. } => return None,
                // The leaf family's wrapper returns a value on every complete
                // path; a value-free return belongs to the `file_close` wrapper
                // alone and is declined by name here.
                CleanupReplayOperationRecord::Return => return None,
            }
        }
        None
    }
}

/// The prerequisite's own selected path for one native outcome, recomputed from
/// the prerequisite reading this process derived itself.
fn native_source_path(cases: &[CleanupReplayCaseRecord], outcome: i32) -> Option<u32> {
    let mut found = None;
    for case in cases {
        let CleanupReplayCaseRecord::NativeCleanupOutcome {
            entry,
            syscall_result,
            source_path,
            returned,
            ..
        } = case
        else {
            return None;
        };
        if *entry == CleanupEntryRecord::Present
            && *syscall_result == Some(outcome)
            && *returned == outcome
        {
            if found.is_some() {
                return None;
            }
            found = Some(*source_path);
        }
    }
    found
}

fn compose_unmap_wrapper(
    wrapper: &CleanupReplayFunctionRecord,
    native: &CleanupReplayFunctionRecord,
    native_profile: &CleanupProfileRecord,
) -> Option<CompositionRecord> {
    if !bounded(wrapper) || !bounded(native) {
        return None;
    }
    if wrapper.function == native.function
        || wrapper.source_revision != native.source_revision
        || wrapper.signature != "int (struct cio_ctx *, struct cio_chunk *)"
    {
        return None;
    }
    // The prerequisite reading is recomputed here, from the prerequisite's own
    // paths and profile. It is never supplied.
    let native_reading =
        crate::cleanup_replay_effect::replay_native_cleanup_outcome(native, native_profile)?;
    let mut cases = Vec::new();
    let mut covered = BTreeSet::new();
    for (backend, mapped, synced, sync, outcome) in entry_states() {
        let mut selected = Vec::new();
        for (index, path) in wrapper.paths.iter().enumerate() {
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
            let Some(returned) = replay.run(&path.operations)? else {
                continue;
            };
            let success = outcome == Some(0);
            if replay.sync_calls != u32::from(sync.is_some())
                || replay.logs != u32::from(sync == Some(-1))
                || replay.native_calls != u32::from(outcome.is_some())
                || replay.counter_calls != u32::from(success)
                || returned != if success { 0 } else { -1 }
                || replay.writes
                    != if success {
                        vec!["data_size".to_string(), "alloc_size".to_string()]
                    } else {
                        Vec::<String>::new()
                    }
            {
                return None;
            }
            let native_path = match outcome {
                Some(outcome) => Some(native_source_path(&native_reading.cases, outcome)?),
                None => None,
            };
            selected.push(UnmapCaseRecord {
                source_path: u32::try_from(index).ok()?,
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
        let [case] = selected.as_slice() else {
            return None;
        };
        covered.insert(case.source_path);
        cases.push(case.clone());
    }
    if covered.len() != wrapper.paths.len() {
        return None;
    }
    Some(CompositionRecord {
        function: wrapper.function.clone(),
        source_revision: wrapper.source_revision.clone(),
        signature: wrapper.signature.clone(),
        cases,
        profile_assumptions: wrapper.profile_assumptions.clone(),
    })
}

/// The `cio_file_close` wrapper's symbolic value domain, constructor for
/// constructor with the trusted composition's own private domain and in the same
/// order. There is no `Unknown`/`Other` constructor: an unresolved value is a
/// decline, never an opaque token.
#[derive(Clone, Debug, PartialEq, Eq)]
enum CloseValue {
    Null,
    Chunk,
    Context,
    Backend,
    Path,
    Stream,
    StreamName,
    ChunkName,
    Text(String),
    Integer(i32),
    IgnoredResult,
}

struct CloseReplay {
    backend: bool,
    path_live: bool,
    phase: u32,
    deleting: bool,
    delete_result: Option<i32>,
    delete_called: bool,
    logged: bool,
    symbols: BTreeMap<u32, CloseValue>,
}

impl CloseReplay {
    fn value(&self, v: &CleanupReplayValueRecord, depth: u32) -> Option<CloseValue> {
        if depth > MAX_EXPRESSION_DEPTH {
            return None;
        }
        match &v.expression {
            CleanupReplayExpressionRecord::Parameter { index: 0 }
                if v.c_type == "struct cio_chunk *" =>
            {
                Some(CloseValue::Chunk)
            }
            CleanupReplayExpressionRecord::Parameter { index: 1 } if v.c_type == "int" => {
                Some(CloseValue::Integer(i32::from(self.deleting)))
            }
            CleanupReplayExpressionRecord::String { value } => {
                Some(CloseValue::Text(value.clone()))
            }
            CleanupReplayExpressionRecord::Integer { value } => {
                Some(CloseValue::Integer(value.parse().ok()?))
            }
            CleanupReplayExpressionRecord::Symbol { operation } => self
                .symbols
                .get(operation)
                .filter(|v| **v != CloseValue::IgnoredResult)
                .cloned(),
            CleanupReplayExpressionRecord::Cast { cast, value } => {
                let result = self.value(value, depth + 1)?;
                match cast.as_str() {
                    "NullToPointer"
                        if matches!(result, CloseValue::Null | CloseValue::Integer(0))
                            && v.c_type.ends_with('*') =>
                    {
                        Some(CloseValue::Null)
                    }
                    "ArrayToPointerDecay"
                        if matches!(result, CloseValue::Text(_)) && v.c_type.ends_with('*') =>
                    {
                        Some(result)
                    }
                    "BitCast" | "NoOp"
                        if value.c_type.ends_with('*') && v.c_type.ends_with('*') =>
                    {
                        Some(result)
                    }
                    _ => None,
                }
            }
            CleanupReplayExpressionRecord::Unary { operator, value } if operator == "-" => {
                let CloseValue::Integer(n) = self.value(value, depth + 1)? else {
                    return None;
                };
                Some(CloseValue::Integer(n.checked_neg()?))
            }
            CleanupReplayExpressionRecord::Unary { operator, value } if operator == "!" => Some(
                CloseValue::Integer(i32::from(!Self::truth(self.value(value, depth + 1)?)?)),
            ),
            CleanupReplayExpressionRecord::Binary {
                operator,
                left,
                right,
            } if operator == "==" || operator == "!=" => {
                let a = self.value(left, depth + 1)?;
                let b = self.value(right, depth + 1)?;
                let eq = match (&a, &b) {
                    (CloseValue::Integer(a), CloseValue::Integer(b)) => a == b,
                    (CloseValue::Null, CloseValue::Null) => true,
                    (CloseValue::Backend | CloseValue::Path, CloseValue::Null)
                    | (CloseValue::Null, CloseValue::Backend | CloseValue::Path) => false,
                    _ => return None,
                };
                Some(CloseValue::Integer(i32::from(if operator == "==" {
                    eq
                } else {
                    !eq
                })))
            }
            _ => None,
        }
    }

    fn truth(v: CloseValue) -> Option<bool> {
        match v {
            CloseValue::Null => Some(false),
            CloseValue::Backend | CloseValue::Path => Some(true),
            CloseValue::Integer(n) => Some(n != 0),
            _ => None,
        }
    }

    fn reporting(&self) -> bool {
        self.phase == 2
            && self.deleting
            && self.delete_called
            && self.delete_result == Some(-1)
            && !self.logged
    }

    fn ready_to_free(&self) -> bool {
        self.phase == 2
            && (!self.deleting
                || (self.delete_called && self.logged == (self.delete_result == Some(-1))))
    }

    /// `Some(true)`: this path completed. `Some(false)`: a guard did not hold,
    /// so this entry state does not select this path. `None`: the projection is
    /// outside the vocabulary, so decline entirely.
    fn run(&mut self, ops: &[CleanupReplayOperationRecord]) -> Option<bool> {
        for (i, op) in ops.iter().enumerate() {
            let i = u32::try_from(i).ok()?;
            match op {
                CleanupReplayOperationRecord::Read { place } => {
                    let CleanupReplayExpressionRecord::Field { base, name } = &place.expression
                    else {
                        return None;
                    };
                    let result = match (
                        self.value(base, 0)?,
                        base.c_type.as_str(),
                        name.as_str(),
                        place.c_type.as_str(),
                    ) {
                        (CloseValue::Chunk, "struct cio_chunk *", "backend", "void *")
                            if self.phase == 0 =>
                        {
                            if self.backend {
                                CloseValue::Backend
                            } else {
                                CloseValue::Null
                            }
                        }
                        (CloseValue::Chunk, "struct cio_chunk *", "ctx", "struct cio_ctx *")
                            if self.phase == 0 || self.reporting() =>
                        {
                            CloseValue::Context
                        }
                        (CloseValue::Chunk, "struct cio_chunk *", "st", "struct cio_stream *")
                            if self.reporting() =>
                        {
                            CloseValue::Stream
                        }
                        (CloseValue::Chunk, "struct cio_chunk *", "name", "char *")
                            if self.reporting() =>
                        {
                            CloseValue::ChunkName
                        }
                        (CloseValue::Stream, "struct cio_stream *", "name", "char *")
                            if self.reporting() =>
                        {
                            CloseValue::StreamName
                        }
                        (CloseValue::Backend, "struct cio_file *", "path", "char *")
                            if self.ready_to_free() =>
                        {
                            if self.path_live {
                                CloseValue::Path
                            } else {
                                CloseValue::Null
                            }
                        }
                        _ => return None,
                    };
                    self.symbols.insert(i, result);
                }
                CleanupReplayOperationRecord::LocalWrite { value, .. } => {
                    self.value(value, 0)?;
                }
                CleanupReplayOperationRecord::LocalStart { .. }
                | CleanupReplayOperationRecord::LocalEnd { .. } => (),
                CleanupReplayOperationRecord::Guard { condition, taken } => {
                    if Self::truth(self.value(condition, 0)?)? != *taken {
                        return Some(false);
                    }
                }
                CleanupReplayOperationRecord::Call {
                    function,
                    signature,
                    arguments,
                    result_type,
                } => {
                    let args = arguments
                        .iter()
                        .map(|v| self.value(v, 0))
                        .collect::<Option<Vec<_>>>()?;
                    if function == "cio_file_native_delete" {
                        if self.phase != 2
                            || !self.deleting
                            || self.delete_called
                            || self.delete_result.is_none()
                            || signature != "int (struct cio_file *)"
                            || result_type != "int"
                            || args != [CloseValue::Backend]
                        {
                            return None;
                        }
                        self.delete_called = true;
                        self.symbols
                            .insert(i, CloseValue::Integer(self.delete_result?));
                        continue;
                    }
                    if function == "cio_log_print" {
                        if !self.reporting()
                            || signature
                                != "void (void *, int, const char *, int, const char *, ...)"
                            || result_type != "void"
                            || !matches!(args.as_slice(),[CloseValue::Context,CloseValue::Integer(1),CloseValue::Text(_),CloseValue::Integer(line),CloseValue::Text(format),CloseValue::StreamName,CloseValue::ChunkName] if *line>0 && format=="\"[cio file] error deleting file at close %s:%s\"")
                        {
                            return None;
                        }
                        self.logged = true;
                        self.symbols.insert(i, CloseValue::IgnoredResult);
                        continue;
                    }
                    let valid = match (
                        self.phase,
                        function.as_str(),
                        signature.as_str(),
                        result_type.as_str(),
                    ) {
                        (0, "munmap_file", "int (struct cio_ctx *, struct cio_chunk *)", "int") => {
                            self.backend && args == [CloseValue::Context, CloseValue::Chunk]
                        }
                        (1, "cio_file_native_close", "int (struct cio_file *)", "int") => {
                            args == [CloseValue::Backend]
                        }
                        (2, "free", "void (void *)", "void") => {
                            self.ready_to_free()
                                && args
                                    == [if self.path_live {
                                        CloseValue::Path
                                    } else {
                                        CloseValue::Null
                                    }]
                        }
                        (3, "free", "void (void *)", "void") => args == [CloseValue::Backend],
                        _ => false,
                    };
                    if !valid {
                        return None;
                    }
                    self.phase += 1;
                    self.symbols.insert(i, CloseValue::IgnoredResult);
                }
                CleanupReplayOperationRecord::Return
                    if usize::try_from(i).ok()? + 1 == ops.len() =>
                {
                    return Some(true);
                }
                _ => return None,
            }
        }
        None
    }
}

/// The native close reading's own selected case for one prestate, recomputed
/// from the prerequisite reading this process derived itself.
fn close_cases(record: &CleanupReplayRecord) -> Option<Vec<(u32, Option<i32>, bool)>> {
    let mut cases = Vec::new();
    for (index, case) in record.cases.iter().enumerate() {
        let CleanupReplayCaseRecord::NativeCleanupOutcome {
            entry,
            syscall_result,
            ..
        } = case
        else {
            return None;
        };
        cases.push((
            u32::try_from(index).ok()?,
            *syscall_result,
            *entry != CleanupEntryRecord::Null,
        ));
    }
    Some(cases)
}

/// One guarded-delete case as the `file_close` entry domain reads it: the case
/// index, both eligibility fields, the syscall outcome and the returned value.
type DeleteCase = (u32, bool, bool, Option<i32>, i32);

/// The guarded delete reading's own cases, in the shape the file_close entry
/// domain crosses them with.
fn delete_cases(record: &CleanupReplayRecord) -> Option<Vec<DeleteCase>> {
    let mut cases = Vec::new();
    for (index, case) in record.cases.iter().enumerate() {
        let CleanupReplayCaseRecord::GuardedDelete {
            descriptor_field_active,
            mapping_field_active,
            syscall_result,
            returned,
            ..
        } = case
        else {
            return None;
        };
        cases.push((
            u32::try_from(index).ok()?,
            *descriptor_field_active,
            *mapping_field_active,
            *syscall_result,
            *returned,
        ));
    }
    Some(cases)
}

fn residual(unresolved: bool) -> ResidualCustodyRecord {
    if unresolved {
        ResidualCustodyRecord::Unresolved
    } else {
        ResidualCustodyRecord::Absent
    }
}

/// Recompute the whole two-level `cio_file_close` reading.
///
/// The nested composed reading is recomputed here by running this same process's
/// unmap-wrapper composition over the transported nested question — not read out
/// of anything, because the vocabulary cannot carry it.
fn compose_file_close(
    wrapper: &CleanupReplayFunctionRecord,
    unmap: &CompositionFamilyInput,
    close: &CleanupReplayFunctionRecord,
    close_profile: &CleanupProfileRecord,
    delete: Option<&FileCloseDeleteInput>,
) -> Option<FileCloseCompositionRecord> {
    if !bounded(wrapper) || !bounded(close) {
        return None;
    }
    if wrapper.signature != "void (struct cio_chunk *, int)"
        || wrapper.function == close.function
        || wrapper.source_revision != close.source_revision
    {
        return None;
    }
    // Level two, recomputed rather than received.
    let CompositionFamilyInput::UnmapWrapper {
        wrapper: nested_wrapper,
        native: nested_native,
        native_profile,
    } = unmap
    else {
        return None;
    };
    if nested_wrapper.source_revision != wrapper.source_revision {
        return None;
    }
    let nested = compose_unmap_wrapper(nested_wrapper, nested_native, native_profile)?;
    let close_reading =
        crate::cleanup_replay_effect::replay_native_cleanup_outcome(close, close_profile)?;
    let close_cases = close_cases(&close_reading)?;
    let delete_reading = match delete {
        None => None,
        Some(delete) => {
            if !bounded(&delete.body) || delete.body.source_revision != wrapper.source_revision {
                return None;
            }
            let record = crate::cleanup_replay_effect::replay_guarded_delete(
                &delete.body,
                &delete.profile,
                &delete.mapping_field,
            )?;
            Some(delete_cases(&record)?)
        }
    };

    // The non-deleting arm, over the same entry domain the trusted composition
    // enumerates: the nested composition's backend-present cases crossed with
    // the close reading's non-null prestates and the path allocation.
    // The nested case *index*, not its selected path, is what the trusted
    // reading publishes, so the entry domain is enumerated over indices.
    let mut entries: Vec<(Option<u32>, Option<u32>, bool)> = vec![(None, None, false)];
    for (unmap_index, case) in nested.cases.iter().enumerate() {
        if !case.backend_present {
            continue;
        }
        let unmap_index = u32::try_from(unmap_index).ok()?;
        for (close_index, _, present) in &close_cases {
            if !present {
                continue;
            }
            for path_live in [false, true] {
                entries.push((Some(unmap_index), Some(*close_index), path_live));
            }
        }
    }
    let mut cases = Vec::new();
    let mut covered = BTreeSet::new();
    for (u, c, path_live) in entries {
        let mut selected = Vec::new();
        for (index, path) in wrapper.paths.iter().enumerate() {
            let mut replay = CloseReplay {
                backend: u.is_some(),
                path_live,
                phase: 0,
                deleting: false,
                delete_result: None,
                delete_called: false,
                logged: false,
                symbols: BTreeMap::new(),
            };
            if !replay.run(&path.operations)? {
                continue;
            }
            if replay.phase != if u.is_some() { 4 } else { 0 } {
                return None;
            }
            let mapping = u.is_some_and(|i| {
                nested.cases.get(i as usize).and_then(|c| c.native_result) == Some(-1)
            });
            let descriptor = c.is_some_and(|i| {
                close_cases
                    .iter()
                    .find(|(index, _, _)| *index == i)
                    .and_then(|(_, result, _)| *result)
                    == Some(-1)
            });
            selected.push(FileCloseCaseRecord {
                source_path: u32::try_from(index).ok()?,
                backend_present: u.is_some(),
                path_allocation_present: path_live,
                unmap_case: u,
                close_case: c,
                mapping: residual(mapping),
                descriptor: residual(descriptor),
                path_released: path_live,
                backend_released: u.is_some(),
                up_counter_decrements: match u {
                    None => 0,
                    Some(i) => nested.cases.get(i as usize)?.counter_decrements,
                },
            });
        }
        let [case] = selected.as_slice() else {
            return None;
        };
        covered.insert(case.source_path);
        cases.push(case.clone());
    }
    let uncovered_paths: Vec<u32> = (0..u32::try_from(wrapper.paths.len()).ok()?)
        .filter(|index| !covered.contains(index))
        .collect();

    // The deleting arm, on top of the non-deleting reading.
    let (deleting_cases, deleting_uncovered_paths) = match delete_reading {
        None => (None, None),
        Some(delete_cases) => {
            let mut entries: Vec<(Option<u32>, Option<u32>, Option<u32>)> =
                vec![(None, None, None)];
            for (unmap_index, case) in nested.cases.iter().enumerate() {
                if !case.backend_present {
                    continue;
                }
                let unmap_index = u32::try_from(unmap_index).ok()?;
                let mapped = case.native_result == Some(-1);
                for (close_index, close_result, present) in &close_cases {
                    if !present {
                        continue;
                    }
                    let open = *close_result == Some(-1);
                    for (delete_index, descriptor_active, mapping_active, _, _) in &delete_cases {
                        if *mapping_active == mapped && *descriptor_active == open {
                            entries.push((
                                Some(unmap_index),
                                Some(*close_index),
                                Some(*delete_index),
                            ));
                        }
                    }
                }
            }
            let mut deleting = Vec::new();
            let mut covered: BTreeSet<u32> = cases.iter().map(|case| case.source_path).collect();
            for (u, c, d) in entries {
                let row = match d {
                    None => None,
                    Some(index) => Some(delete_cases.iter().find(|(i, _, _, _, _)| *i == index)?),
                };
                let outcome = row.map(|(_, _, _, _, returned)| *returned);
                let mut selected = Vec::new();
                for (index, path) in wrapper.paths.iter().enumerate() {
                    let mut replay = CloseReplay {
                        backend: u.is_some(),
                        path_live: u.is_some(),
                        phase: 0,
                        deleting: true,
                        delete_result: outcome,
                        delete_called: false,
                        logged: false,
                        symbols: BTreeMap::new(),
                    };
                    if !replay.run(&path.operations)? {
                        continue;
                    }
                    if replay.phase != if u.is_some() { 4 } else { 0 }
                        || replay.delete_called != d.is_some()
                        || replay.logged != (outcome == Some(-1))
                    {
                        return None;
                    }
                    let mapping = u.is_some_and(|i| {
                        nested.cases.get(i as usize).and_then(|c| c.native_result) == Some(-1)
                    });
                    let descriptor = c.is_some_and(|i| {
                        close_cases
                            .iter()
                            .find(|(index, _, _)| *index == i)
                            .and_then(|(_, result, _)| *result)
                            == Some(-1)
                    });
                    let deletion = match row.map(|(_, _, _, syscall, _)| *syscall) {
                        None => DeletionOutcomeRecord::NoBackend,
                        Some(None) => DeletionOutcomeRecord::BlockedByFields,
                        Some(Some(-1)) => DeletionOutcomeRecord::SyscallFailed,
                        Some(Some(0)) => DeletionOutcomeRecord::SyscallSucceeded,
                        _ => return None,
                    };
                    selected.push(DeletingCloseCaseRecord {
                        source_path: u32::try_from(index).ok()?,
                        unmap_case: u,
                        close_case: c,
                        delete_case: d,
                        mapping: residual(mapping),
                        descriptor: residual(descriptor),
                        deletion,
                        backend_released: u.is_some(),
                        path_released: u.is_some(),
                        diagnostic_calls: u32::from(replay.logged),
                        up_counter_decrements: match u {
                            None => 0,
                            Some(i) => nested.cases.get(i as usize)?.counter_decrements,
                        },
                    });
                }
                let [case] = selected.as_slice() else {
                    return None;
                };
                covered.insert(case.source_path);
                deleting.push(case.clone());
            }
            // The deleting profile must cover every path; an uncovered one is a
            // decline, never a published gap.
            if covered.len() != wrapper.paths.len() {
                return None;
            }
            (Some(deleting), Some(Vec::new()))
        }
    };
    Some(FileCloseCompositionRecord {
        function: wrapper.function.clone(),
        source_revision: wrapper.source_revision.clone(),
        signature: wrapper.signature.clone(),
        cases,
        uncovered_paths,
        deleting_cases,
        deleting_uncovered_paths,
        profile_assumptions: wrapper.profile_assumptions.clone(),
    })
}

// ---------------------------------------------------------------------------
// The memory-close family: four prerequisites over a derived unit.
// ---------------------------------------------------------------------------

/// The memory-close wrapper's symbolic value domain, constructor for
/// constructor with the trusted composition's own private domain and in the
/// same order — minus the symbolic unsupported-backend class, which the
/// transported question never asks about. There is no `Unknown`/`Other`
/// constructor: an unresolved value is a decline, never an opaque token.
#[derive(Clone, Debug, PartialEq, Eq)]
enum MemoryValue {
    Null,
    Chunk,
    Context,
    Stream,
    Name,
    PrimaryLink,
    StateLink,
    Integer(i32),
    /// The **symbolic** other-backend class of card `P04-COMPOSE-UNSUPPORTED`:
    /// every `int` except the two source-probed constants it names. It is not a
    /// value and no member of it is ever sampled — every comparison this class
    /// cannot decide for the whole class is a decline, exactly as the trusted
    /// composition's own `Value::OtherBackend` refuses.
    OtherBackend(i32, i32),
    Boolean(bool),
}

struct MemoryReplay<'a> {
    nonnull: bool,
    memory_kind: i32,
    /// Whether this replay is walking the **filesystem** arm of the same
    /// wrapper (card `P04-COMPOSE-FS`). On the memory family it is always
    /// false, and every branch it guards is unreachable there.
    filesystem: bool,
    /// The two excluded constants, when this replay is walking the **symbolic**
    /// arm. `None` on every other arm, and every branch it guards is then
    /// unreachable.
    other_kind: Option<(i32, i32)>,
    delete: bool,
    freed: bool,
    backend: bool,
    primary: bool,
    state_link: bool,
    name: bool,
    counter: bool,
    calls: u32,
    symbols: BTreeMap<u32, MemoryValue>,
    callees: &'a MemoryCloseCalleeRecord,
}

impl MemoryReplay<'_> {
    /// `None` is a decline: the symbolic class splits under this test, because
    /// `0` is inside it and both arms of the branch would be reachable.
    fn truth(value: &MemoryValue) -> Option<bool> {
        if let MemoryValue::OtherBackend(memory, file) = value
            && *memory != 0
            && *file != 0
        {
            return None;
        }
        Some(!matches!(
            value,
            MemoryValue::Null | MemoryValue::Boolean(false) | MemoryValue::Integer(0)
        ))
    }

    fn value(&self, v: &CleanupReplayValueRecord, depth: u32) -> Option<MemoryValue> {
        if depth > MAX_EXPRESSION_DEPTH {
            return None;
        }
        match &v.expression {
            CleanupReplayExpressionRecord::Parameter { index: 0 } => Some(if self.nonnull {
                MemoryValue::Chunk
            } else {
                MemoryValue::Null
            }),
            // The delete flag is only ever read on the filesystem arm; the
            // memory family's entry states do not resolve it at all.
            CleanupReplayExpressionRecord::Parameter { index: 1 }
                if self.filesystem || self.other_kind.is_some() =>
            {
                Some(MemoryValue::Integer(i32::from(self.delete)))
            }
            CleanupReplayExpressionRecord::Symbol { operation } => {
                self.symbols.get(operation).cloned()
            }
            CleanupReplayExpressionRecord::Integer { value } => {
                Some(MemoryValue::Integer(value.parse().ok()?))
            }
            CleanupReplayExpressionRecord::Cast { cast, value }
                if matches!(cast.as_str(), "BitCast" | "NoOp") =>
            {
                self.value(value, depth + 1)
            }
            CleanupReplayExpressionRecord::Unary { operator, value } if operator == "!" => Some(
                MemoryValue::Boolean(!Self::truth(&self.value(value, depth + 1)?)?),
            ),
            CleanupReplayExpressionRecord::Binary {
                operator,
                left,
                right,
            } if operator == "==" => {
                // C logical expressions have integer results. Preserve that
                // equality when a guard compares !pointer with literal zero.
                let integer_bool = |value| match value {
                    MemoryValue::Boolean(value) => MemoryValue::Integer(i32::from(value)),
                    other => other,
                };
                let left = integer_bool(self.value(left, depth + 1)?);
                let right = integer_bool(self.value(right, depth + 1)?);
                // The symbolic class decides an equality only against the two
                // constants it excludes; every other integer is inside it *and*
                // outside it, so the comparison is a decline rather than a
                // sampled answer.
                if let (MemoryValue::OtherBackend(memory, file), MemoryValue::Integer(value))
                | (MemoryValue::Integer(value), MemoryValue::OtherBackend(memory, file)) =
                    (&left, &right)
                {
                    return if value == memory || value == file {
                        Some(MemoryValue::Boolean(false))
                    } else {
                        None
                    };
                }
                if matches!(left, MemoryValue::OtherBackend(..))
                    || matches!(right, MemoryValue::OtherBackend(..))
                {
                    return None;
                }
                if matches!(&left, MemoryValue::Integer(_))
                    != matches!(&right, MemoryValue::Integer(_))
                {
                    return None;
                }
                Some(MemoryValue::Boolean(left == right))
            }
            CleanupReplayExpressionRecord::Unary { operator, value } if operator == "&" => {
                let CleanupReplayExpressionRecord::Field { base, name } = &value.expression else {
                    return None;
                };
                if self.value(base, depth + 1)? != MemoryValue::Chunk
                    || self.freed
                    || value.c_type != "struct mk_list"
                {
                    return None;
                }
                match name.as_str() {
                    "_head" => Some(MemoryValue::PrimaryLink),
                    "_state_head" => Some(MemoryValue::StateLink),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// Walk one complete path. `Some(true)` means this entry selects it,
    /// `Some(false)` that a guard turned the other way, and `None` that the
    /// path is outside what this producer can resolve — a decline.
    fn run(&mut self, operations: &[CleanupReplayOperationRecord]) -> Option<bool> {
        for (index, operation) in operations.iter().enumerate() {
            let index = u32::try_from(index).ok()?;
            match operation {
                CleanupReplayOperationRecord::Guard { condition, taken } => {
                    if Self::truth(&self.value(condition, 0)?)? != *taken {
                        return Some(false);
                    }
                }
                CleanupReplayOperationRecord::Read { place } => {
                    let CleanupReplayExpressionRecord::Field { base, name } = &place.expression
                    else {
                        return None;
                    };
                    let resolved = match (self.value(base, 0)?, name.as_str()) {
                        (MemoryValue::Chunk, _) if self.freed => return None,
                        (MemoryValue::Chunk, "ctx") if place.c_type == "struct cio_ctx *" => {
                            MemoryValue::Context
                        }
                        (MemoryValue::Chunk, "st") if place.c_type == "struct cio_stream *" => {
                            MemoryValue::Stream
                        }
                        (MemoryValue::Chunk, "name") if !self.name && place.c_type == "char *" => {
                            MemoryValue::Name
                        }
                        (MemoryValue::Stream, "type") if place.c_type == "int" => self
                            .other_kind
                            .map_or(MemoryValue::Integer(self.memory_kind), |(m, f)| {
                                MemoryValue::OtherBackend(m, f)
                            }),
                        _ => return None,
                    };
                    self.symbols.insert(index, resolved);
                }
                CleanupReplayOperationRecord::LocalWrite { value, .. } => {
                    self.value(value, 0)?;
                }
                CleanupReplayOperationRecord::LocalEnd { .. } => (),
                CleanupReplayOperationRecord::Call {
                    function,
                    signature,
                    arguments,
                    result_type,
                } => {
                    let mut resolved = Vec::new();
                    for argument in arguments {
                        resolved.push(self.value(argument, 0)?);
                    }
                    self.calls = self.calls.checked_add(1)?;
                    let named = |callee: &CalleeIdentityRecord| {
                        *function == callee.function && *signature == callee.signature
                    };
                    if named(&self.callees.reset)
                        && result_type == "void"
                        && resolved == [MemoryValue::Chunk]
                        && !self.freed
                        && self.calls == 1
                    {
                    } else if !self.freed
                        && !self.backend
                        && self.other_kind.is_none()
                        && if self.filesystem {
                            // The filesystem dispatch frame is not supplied by
                            // any prerequisite of the memory family: it is the
                            // backend composition's own wrapper, and this arm
                            // resolves it by its declared prototype only.
                            function == FILESYSTEM_DISPATCH_CALLEE.0
                                && signature == FILESYSTEM_DISPATCH_CALLEE.1
                                && result_type == "void"
                                && resolved
                                    == [
                                        MemoryValue::Chunk,
                                        MemoryValue::Integer(i32::from(self.delete)),
                                    ]
                                && self.calls == 2
                        } else {
                            *function == self.callees.backend.function
                                && *signature == self.callees.backend.signature
                                && resolved == [MemoryValue::Chunk]
                        }
                    {
                        self.backend = true;
                    } else if named(&self.callees.unlink)
                        && !self.freed
                        && (self.backend || self.other_kind.is_some())
                    {
                        match resolved.as_slice() {
                            [MemoryValue::PrimaryLink] if !self.primary => self.primary = true,
                            [MemoryValue::StateLink] if !self.state_link => {
                                self.state_link = true;
                            }
                            _ => return None,
                        }
                    } else if function == MEMORY_CLOSE_RELEASE_CALLEE.0
                        && signature == MEMORY_CLOSE_RELEASE_CALLEE.1
                        && result_type == "void"
                        && (self.backend || self.other_kind.is_some())
                        && self.primary
                        && self.state_link
                    {
                        match resolved.as_slice() {
                            [MemoryValue::Name] if !self.name && !self.freed => self.name = true,
                            [MemoryValue::Chunk] if self.name && !self.freed => self.freed = true,
                            _ => return None,
                        }
                    } else if *function == self.callees.counter.function
                        && result_type
                            == self
                                .callees
                                .counter
                                .signature
                                .split('(')
                                .next()
                                .unwrap_or_default()
                                .trim()
                        && signature.split_once('(').map(|(_, args)| args)
                            == Some("struct cio_ctx *)")
                        && resolved == [MemoryValue::Context]
                        && self.freed
                        && !self.counter
                    {
                        self.counter = true;
                    } else {
                        return None;
                    }
                }
                CleanupReplayOperationRecord::Return
                    if usize::try_from(index).ok()? + 1 == operations.len() =>
                {
                    return Some(true);
                }
                _ => return None,
            }
        }
        None
    }
}

/// The libc release frame the memory-close wrapper is allowed to resolve
/// without a prerequisite. It is not a transported value: a wrapper that calls
/// anything else under this name has no projection, and the receiver runs its
/// own composition regardless.
const MEMORY_CLOSE_RELEASE_CALLEE: (&str, &str) = ("free", "void (void *)");

/// The filesystem dispatch frame the filesystem arm resolves. It is the backend
/// composition's own wrapper, matched on name and declared prototype; nothing
/// about its *reading* is transported, and the receiver re-establishes that
/// whole composition itself.
const FILESYSTEM_DISPATCH_CALLEE: (&str, &str) =
    ("cio_file_close", "void (struct cio_chunk *, int)");

/// Read the backend discriminant out of the transported probe body. The value
/// is never handed to this process; only the body is.
fn memory_kind(probe: &CleanupReplayFunctionRecord) -> Option<i32> {
    if probe.signature != "int (void)" {
        return None;
    }
    let [path] = probe.paths.as_slice() else {
        return None;
    };
    let [CleanupReplayOperationRecord::ReturnValue { value }] = path.operations.as_slice() else {
        return None;
    };
    let CleanupReplayExpressionRecord::Integer { value } = &value.expression else {
        return None;
    };
    value.parse().ok()
}

/// Recompute the whole memory-close reading from the transported question.
fn compose_memory_close(
    wrapper: &CleanupReplayFunctionRecord,
    probe: &CleanupReplayFunctionRecord,
    derivation: &DerivedUnitRecord,
    callees: &MemoryCloseCalleeRecord,
) -> Option<MemoryCloseCompositionRecord> {
    if !bounded(wrapper) || !bounded(probe) {
        return None;
    }
    if wrapper.signature != "void (struct cio_chunk *, int)"
        || wrapper.source_revision != probe.source_revision
        || wrapper.function == probe.function
    {
        return None;
    }
    // The derivation travels as a claim. This process cannot check it against
    // any text — it has none — so it checks only that the claim is well formed
    // and names the body it was handed. The receiver compares it against its
    // own derivation, which is where the authority is.
    if !derivation.is_complete()
        || !derivation.is_a_derivation()
        || derivation.recipe_function != probe.function
    {
        return None;
    }
    if !callees.is_complete() || !callees.are_distinct() {
        return None;
    }
    let memory_kind = memory_kind(probe)?;
    let mut cases = Vec::new();
    let mut checked = BTreeSet::new();
    for nonnull in [false, true] {
        let mut accepted: Vec<(u32, MemoryCloseCaseRecord)> = Vec::new();
        for (index, path) in wrapper.paths.iter().enumerate() {
            let index = u32::try_from(index).ok()?;
            let mut replay = MemoryReplay {
                nonnull,
                memory_kind,
                filesystem: false,
                other_kind: None,
                delete: false,
                freed: false,
                backend: false,
                primary: false,
                state_link: false,
                name: false,
                counter: false,
                calls: 0,
                symbols: BTreeMap::new(),
                callees,
            };
            if replay.run(&path.operations)? {
                accepted.push((
                    index,
                    MemoryCloseCaseRecord {
                        nonnull,
                        source_path: index,
                        calls: replay.calls,
                        backend_discharged: replay.backend,
                        primary_unlinked: replay.primary,
                        state_unlinked: replay.state_link,
                        name_released: replay.name,
                        chunk_released: replay.freed,
                        counter_decremented: replay.counter,
                    },
                ));
            }
        }
        // Exactly one complete path per entry state, exactly as the trusted
        // composition requires. Two, or none, is a decline.
        let [(index, case)] = accepted.as_slice() else {
            return None;
        };
        checked.insert(*index);
        cases.push(case.clone());
    }
    let uncovered = (0..u32::try_from(wrapper.paths.len()).ok()?)
        .filter(|index| !checked.contains(index))
        .collect();
    Some(MemoryCloseCompositionRecord {
        function: wrapper.function.clone(),
        source_revision: wrapper.source_revision.clone(),
        signature: wrapper.signature.clone(),
        memory_kind,
        cases,
        checked_paths: checked.into_iter().collect(),
        uncovered_paths: uncovered,
        profile_assumptions: wrapper.profile_assumptions.clone(),
    })
}

// ---------------------------------------------------------------------------
// The filesystem family: two composition prerequisites over two derived units.
// ---------------------------------------------------------------------------

/// Read the filesystem discriminant out of the transported probe body. The
/// value is never handed to this process; only the body is.
fn filesystem_kind(probe: &CleanupReplayFunctionRecord) -> Option<i32> {
    memory_kind(probe)
}

/// The single wrapper path one filesystem entry state selects, recomputed by
/// replaying the *nested memory question's* wrapper on the filesystem arm.
///
/// This mirrors the trusted composition's own `filesystem_path`: the entry must
/// select exactly one complete path, and on the nonnull entry that path must
/// have performed every one of the six releases and resolved exactly seven
/// calls. Anything else is a decline.
fn filesystem_path(
    wrapper: &CleanupReplayFunctionRecord,
    callees: &MemoryCloseCalleeRecord,
    kind: i32,
    nonnull: bool,
    delete: bool,
) -> Option<u32> {
    let mut selected: Vec<u32> = Vec::new();
    for (index, path) in wrapper.paths.iter().enumerate() {
        let index = u32::try_from(index).ok()?;
        let mut replay = MemoryReplay {
            nonnull,
            memory_kind: kind,
            filesystem: true,
            other_kind: None,
            delete,
            freed: false,
            backend: false,
            primary: false,
            state_link: false,
            name: false,
            counter: false,
            calls: 0,
            symbols: BTreeMap::new(),
            callees,
        };
        if replay.run(&path.operations)? {
            if [
                replay.freed,
                replay.backend,
                replay.primary,
                replay.state_link,
                replay.name,
                replay.counter,
            ]
            .iter()
            .any(|value| *value != nonnull)
                || replay.calls != if nonnull { 7 } else { 0 }
            {
                return None;
            }
            selected.push(index);
        }
    }
    let [index] = selected.as_slice() else {
        return None;
    };
    Some(*index)
}

/// Recompute the whole three-level filesystem reading from the transported
/// question.
///
/// Both prerequisite readings are recomputed here by running this same
/// process's memory-close and `file_close` compositions over the two
/// transported nested questions — not read out of anything, because the
/// vocabulary cannot carry either.
fn compose_filesystem_chunk_close(
    memory: &CompositionFamilyInput,
    backend: &CompositionFamilyInput,
    probe: &CleanupReplayFunctionRecord,
    derivation: &DerivedUnitRecord,
) -> Option<FilesystemChunkCloseCompositionRecord> {
    if !bounded(probe) {
        return None;
    }
    // This family's own derivation travels as a claim. This process cannot check
    // it against any text — it has none — so it checks only that the claim is
    // well formed and names the body it was handed. The receiver compares it
    // against its own derivation, which is where the authority is.
    if !derivation.is_complete()
        || !derivation.is_a_derivation()
        || derivation.recipe_function != probe.function
    {
        return None;
    }
    let CompositionFamilyInput::MemoryClose {
        wrapper,
        probe: memory_probe,
        derivation: memory_derivation,
        callees,
    } = memory
    else {
        return None;
    };
    // Two derivations over one base under two different recipes, checked here
    // for well-formedness and by the receiver against its own two derivations.
    if memory_derivation.base_source_digest != derivation.base_source_digest
        || memory_derivation.recipe == derivation.recipe
        || memory_derivation.recipe_function == derivation.recipe_function
        || memory_derivation.derived_source_digest == derivation.derived_source_digest
    {
        return None;
    }
    // Level two, recomputed rather than received.
    let memory_reading = compose_memory_close(wrapper, memory_probe, memory_derivation, callees)?;
    let CompositionFamilyInput::FileClose {
        wrapper: backend_wrapper,
        unmap,
        close,
        close_profile,
        delete: Some(delete),
    } = backend
    else {
        return None;
    };
    // Levels two and three of the backend half, also recomputed.
    let backend_reading =
        compose_file_close(backend_wrapper, unmap, close, close_profile, Some(delete))?;
    let deleting = backend_reading.deleting_cases.as_ref()?;
    if backend_reading.source_revision != memory_reading.source_revision
        || backend_reading.signature != memory_reading.signature
    {
        return None;
    }
    let kind = filesystem_kind(probe)?;
    if kind == memory_reading.memory_kind {
        return None;
    }
    let mut cases = Vec::new();
    let mut covered = BTreeSet::new();
    for delete_flag in [false, true] {
        let count = if delete_flag {
            deleting.len()
        } else {
            backend_reading.cases.len()
        };
        for entry in std::iter::once(None).chain((0..count).map(Some)) {
            let nonnull = entry.is_some();
            let source_path = filesystem_path(wrapper, callees, kind, nonnull, delete_flag)?;
            let (mapping, descriptor, deletion, backend_released, path_released, up) = match entry {
                None => (
                    ResidualCustodyRecord::Absent,
                    ResidualCustodyRecord::Absent,
                    None,
                    false,
                    false,
                    0,
                ),
                Some(index) if delete_flag => {
                    let case = deleting.get(index)?;
                    (
                        case.mapping,
                        case.descriptor,
                        Some(case.deletion),
                        case.backend_released,
                        case.path_released,
                        case.up_counter_decrements,
                    )
                }
                Some(index) => {
                    let case = backend_reading.cases.get(index)?;
                    (
                        case.mapping,
                        case.descriptor,
                        None,
                        case.backend_released,
                        case.path_released,
                        case.up_counter_decrements,
                    )
                }
            };
            covered.insert(source_path);
            cases.push(FilesystemChunkCloseCaseRecord {
                source_path,
                chunk_present: nonnull,
                delete: delete_flag,
                backend_case: match entry {
                    None => None,
                    Some(index) => Some(u32::try_from(index).ok()?),
                },
                backend_released,
                path_released,
                mapping,
                descriptor,
                deletion,
                primary_unlinked: nonnull,
                state_unlinked: nonnull,
                name_released: nonnull,
                chunk_released: nonnull,
                total_counter_decrements: u32::from(nonnull),
                up_counter_decrements: up,
            });
        }
    }
    let paths = u32::try_from(wrapper.paths.len()).ok()?;
    let uncovered: Vec<u32> = (0..paths)
        .filter(|index| !covered.contains(index))
        .collect();
    let combined = uncovered
        .iter()
        .filter(|index| !memory_reading.checked_paths.contains(index))
        .copied()
        .collect();
    Some(FilesystemChunkCloseCompositionRecord {
        function: memory_reading.function.clone(),
        source_revision: memory_reading.source_revision.clone(),
        signature: memory_reading.signature.clone(),
        filesystem_kind: kind,
        memory_kind: memory_reading.memory_kind,
        cases,
        checked_paths: covered.into_iter().collect(),
        uncovered_paths: uncovered,
        combined_uncovered_paths: combined,
        profile_assumptions: memory_reading.profile_assumptions.clone(),
    })
}

// ---------------------------------------------------------------------------
// The symbolic family: one prerequisite, no body of its own, a class not a value.
// ---------------------------------------------------------------------------

/// The single wrapper path the **symbolic** other-backend class selects,
/// recomputed by replaying the memory question's wrapper a third time with the
/// stream's discriminant standing for the whole excluded class.
///
/// This mirrors the trusted composition's own `unsupported_path`: the class must
/// select exactly one complete path, that path must perform all five of the
/// wrapper's own releases, must **not** discharge any backend frame, and must
/// resolve exactly six calls. Anything else is a decline.
fn unsupported_path(
    wrapper: &CleanupReplayFunctionRecord,
    callees: &MemoryCloseCalleeRecord,
    memory_kind: i32,
    filesystem_kind: i32,
    delete: bool,
) -> Option<u32> {
    let mut selected: Vec<u32> = Vec::new();
    for (index, path) in wrapper.paths.iter().enumerate() {
        let index = u32::try_from(index).ok()?;
        let mut replay = MemoryReplay {
            nonnull: true,
            memory_kind,
            filesystem: false,
            other_kind: Some((memory_kind, filesystem_kind)),
            delete,
            freed: false,
            backend: false,
            primary: false,
            state_link: false,
            name: false,
            counter: false,
            calls: 0,
            symbols: BTreeMap::new(),
            callees,
        };
        if replay.run(&path.operations)? {
            if !replay.freed
                || !replay.primary
                || !replay.state_link
                || !replay.name
                || !replay.counter
                || replay.backend
                || replay.calls != 6
            {
                return None;
            }
            selected.push(index);
        }
    }
    let [index] = selected.as_slice() else {
        return None;
    };
    Some(*index)
}

/// Recompute the whole four-level symbolic reading from the transported
/// question.
///
/// The prerequisite reading is recomputed here by running this same process's
/// filesystem composition over the transported nested question — which itself
/// recomputes the memory and `file_close` compositions — and only then is the
/// fourth, symbolic replay run. Nothing is read out of anything: the vocabulary
/// cannot carry a composed reading at any level.
fn compose_unsupported_close(
    filesystem: &CompositionFamilyInput,
) -> Option<UnsupportedCloseCompositionRecord> {
    let CompositionFamilyInput::FilesystemChunkClose {
        memory,
        backend,
        probe,
        derivation,
    } = filesystem
    else {
        return None;
    };
    // Levels two, three and four, recomputed rather than received. The symbolic
    // arm is only asked once the filesystem arm this question depends on has
    // been recomputed and accepted here.
    let filesystem_reading = compose_filesystem_chunk_close(memory, backend, probe, derivation)?;
    let CompositionFamilyInput::MemoryClose {
        wrapper, callees, ..
    } = &**memory
    else {
        return None;
    };
    let memory_kind = filesystem_reading.memory_kind;
    let filesystem_kind = filesystem_reading.filesystem_kind;
    if memory_kind == filesystem_kind {
        return None;
    }
    let source_path = unsupported_path(wrapper, callees, memory_kind, filesystem_kind, false)?;
    // The class's reading must not depend on the delete argument: the wrapper
    // never reaches a dispatch frame on this arm, so a path that changed with
    // the flag would mean the replay resolved something it must not have.
    if unsupported_path(wrapper, callees, memory_kind, filesystem_kind, true)? != source_path {
        return None;
    }
    Some(UnsupportedCloseCompositionRecord {
        function: filesystem_reading.function.clone(),
        source_revision: filesystem_reading.source_revision.clone(),
        signature: filesystem_reading.signature.clone(),
        memory_kind,
        filesystem_kind,
        source_path,
        profile_assumptions: filesystem_reading.profile_assumptions.clone(),
    })
}

fn derive(source: &CompositionSourceInput) -> Option<CompositionProposalRecord> {
    if !source.version_matches()
        || !source.prerequisites.is_complete()
        || !source.prerequisites.are_distinct()
        || !source.family.matches(&source.prerequisites)
    {
        return None;
    }
    match &source.family {
        CompositionFamilyInput::UnmapWrapper {
            wrapper,
            native,
            native_profile,
        } => compose_unmap_wrapper(wrapper, native, native_profile)
            .map(CompositionProposalRecord::UnmapWrapper),
        CompositionFamilyInput::FileClose {
            wrapper,
            unmap,
            close,
            close_profile,
            delete,
        } => compose_file_close(wrapper, unmap, close, close_profile, delete.as_deref())
            .map(CompositionProposalRecord::FileClose),
        CompositionFamilyInput::MemoryClose {
            wrapper,
            probe,
            derivation,
            callees,
        } => compose_memory_close(wrapper, probe, derivation, callees)
            .map(CompositionProposalRecord::MemoryClose),
        CompositionFamilyInput::FilesystemChunkClose {
            memory,
            backend,
            probe,
            derivation,
        } => compose_filesystem_chunk_close(memory, backend, probe, derivation)
            .map(CompositionProposalRecord::FilesystemChunkClose),
        CompositionFamilyInput::UnsupportedClose { filesystem } => {
            compose_unsupported_close(filesystem).map(CompositionProposalRecord::UnsupportedClose)
        }
    }
}

pub fn propose(
    request: &VerificationRequestV2,
    manifest: &PluginManifest,
) -> Option<VerificationResponseV2> {
    if request.capability != VerifierCapability::ResourceContracts
        || !matches!(
            request.target.profile.as_str(),
            UNMAP_WRAPPER_SEMANTIC_PROFILE
                | FILE_CLOSE_SEMANTIC_PROFILE
                | FILE_CLOSE_DELETE_SEMANTIC_PROFILE
                | MEMORY_CLOSE_SEMANTIC_PROFILE
                | FILESYSTEM_CHUNK_CLOSE_SEMANTIC_PROFILE
                | UNSUPPORTED_CLOSE_SEMANTIC_PROFILE
        )
        || !matches!(
            request.target.dialect.as_str(),
            "c11" | "gnu99" | "gnu11" | "gnu17"
        )
    {
        return None;
    }
    // The request's declared profile and the transported question's own family
    // must agree, so one family's question can never be answered under another's
    // profile.
    if request
        .composition_source
        .as_ref()?
        .family
        .semantic_profile()
        != request.target.profile
    {
        return None;
    }
    let summary = derive(request.composition_source.as_ref()?)?;
    Some(VerificationResponseV2 {
        protocol_version: 1,
        request_id: request.request_id.clone(),
        plugin_id: manifest.plugin_id.clone(),
        verdict: ProposedVerdict::Safe,
        diagnostics: Vec::new(),
        evidence: Some(ProposedEvidenceV2 {
            evidence_version: 2,
            unit: request.unit.clone(),
            capability: request.capability,
            coverage: request.coverage(),
            assumptions_used: request.assumptions.clone(),
            reproducibility: Reproducibility {
                provider_version: manifest.package_version.clone(),
                package_digest: manifest.package_digest.clone(),
                deterministic: true,
            },
            witness: SemanticWitness::Composition(Box::new(CompositionWitness {
                task_digest: request.task_digest.clone(),
                summary,
            })),
        }),
    })
}

#[cfg(test)]
mod tests;
