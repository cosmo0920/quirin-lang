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
    CleanupReplayValueRecord,
};
use qcore_schemas::composition_summary::{
    CompositionFamilyInput, CompositionProposalRecord, CompositionRecord, CompositionSourceInput,
    CompositionWitness, UNMAP_WRAPPER_SEMANTIC_PROFILE, UnmapCaseRecord,
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

fn derive(source: &CompositionSourceInput) -> Option<CompositionProposalRecord> {
    if !source.version_matches()
        || !source.prerequisites.is_complete()
        || !source.prerequisites.are_distinct()
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
    }
}

pub fn propose(
    request: &VerificationRequestV2,
    manifest: &PluginManifest,
) -> Option<VerificationResponseV2> {
    if request.capability != VerifierCapability::ResourceContracts
        || request.target.profile != UNMAP_WRAPPER_SEMANTIC_PROFILE
        || !matches!(
            request.target.dialect.as_str(),
            "c11" | "gnu99" | "gnu11" | "gnu17"
        )
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
#[path = "composition_effect_tests.rs"]
mod tests;
