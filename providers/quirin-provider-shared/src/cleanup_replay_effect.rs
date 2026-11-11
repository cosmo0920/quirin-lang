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

//! A schemas-only proposal producer for the two bounded ChunkIO
//! cleanup-outcome families.
//!
//! This process reads the supplied typed paths and the supplied record profile
//! independently and runs its **own** symbolic replay over the closed
//! nine-constructor value lattice: for each sampled prestate of the family it
//! builds the initial field state, walks every complete path, decides which
//! single path that prestate selects, and reports the return value, the call
//! counts and the record's field transition. It evaluates no contract, reads no
//! policy, is handed no expected result and cannot see the prerequisite's
//! conclusion: the request carries paths, a profile and an identity triple, and
//! nothing else.
//!
//! The receiver recomputes the same replay from its own import with its own
//! unedited checker, builds its own profile from its own pinned contract, and
//! re-establishes the prerequisite itself, before anything is admitted.
//!
//! Nothing here observes or claims OS behaviour. A `close`, `munmap` or
//! `unlink` operation in a path is a recorded call; whether a descriptor was
//! released, a mapping torn down or a file deleted is not a question this
//! process can even express.
use qcore_schemas::SemanticWitness;
use qcore_schemas::cleanup_replay_summary::{
    CLEANUP_REPLAY_SEMANTIC_PROFILE, CleanupEntryRecord, CleanupFieldRecord, CleanupLatticeValue,
    CleanupPrestateRecord, CleanupProfileRecord, CleanupReplayCaseRecord,
    CleanupReplayExpressionRecord, CleanupReplayFamilyInput, CleanupReplayFunctionRecord,
    CleanupReplayOperationRecord, CleanupReplayProposalRecord, CleanupReplayRecord,
    CleanupReplaySourceInput, CleanupReplayValueRecord, CleanupReplayWitness, CleanupResetRecord,
};
use qcore_schemas::plugin::*;
use std::collections::{BTreeMap, BTreeSet};

/// Bounds applied to a supplied projection before any replay. These apply to
/// direct Rust callers as well as to transported JSON.
const MAX_PATHS: usize = 64;
const MAX_OPERATIONS: usize = 512;
const MAX_FIELDS: usize = 64;
/// Depth cap on the expression tree the replay walks. The importer's own
/// vocabulary is shallow; a deeper tree is declined rather than recursed into,
/// so a hostile projection cannot exhaust the stack.
const MAX_EXPRESSION_DEPTH: u32 = 32;

fn bounded(body: &CleanupReplayFunctionRecord, profile: &CleanupProfileRecord) -> bool {
    let names: BTreeSet<&str> = profile.fields.iter().map(|f| f.name.as_str()).collect();
    !body.function.is_empty()
        && !body.source_revision.is_empty()
        && !body.signature.is_empty()
        && !body.paths.is_empty()
        && body.paths.len() <= MAX_PATHS
        && body
            .paths
            .iter()
            .all(|path| !path.operations.is_empty() && path.operations.len() <= MAX_OPERATIONS)
        && !profile.fields.is_empty()
        && profile.fields.len() <= MAX_FIELDS
        && names.len() == profile.fields.len()
        && names.contains(profile.presence_field.as_str())
        && !profile.argument_fields.is_empty()
        && profile
            .argument_fields
            .iter()
            .all(|f| names.contains(f.as_str()))
        && body.function == profile.function
        && body.signature == format!("int ({})", profile.root_type)
}

fn reset_value(reset: CleanupResetRecord) -> CleanupLatticeValue {
    match reset {
        CleanupResetRecord::Null => CleanupLatticeValue::Null,
        CleanupResetRecord::Integer { value } => CleanupLatticeValue::Integer { value },
    }
}

/// One prestate's replay of one complete path.
struct Replay<'a> {
    profile: &'a CleanupProfileRecord,
    entry: CleanupEntryRecord,
    outcome: Option<i32>,
    initial: BTreeMap<String, CleanupLatticeValue>,
    fields: BTreeMap<String, CleanupLatticeValue>,
    symbols: BTreeMap<u32, CleanupLatticeValue>,
    calls: u32,
    errno_calls: u32,
    reports: u32,
    writes: Vec<String>,
}

impl<'a> Replay<'a> {
    fn new(
        profile: &'a CleanupProfileRecord,
        entry: CleanupEntryRecord,
        outcome: Option<i32>,
    ) -> Self {
        let fields: BTreeMap<_, _> = profile
            .fields
            .iter()
            .map(|f| {
                let value =
                    if f.name == profile.presence_field && entry == CleanupEntryRecord::Absent {
                        reset_value(f.reset)
                    } else if f.c_type.ends_with('*') {
                        CleanupLatticeValue::Pointer {
                            field: f.name.clone(),
                        }
                    } else {
                        CleanupLatticeValue::Symbol {
                            field: f.name.clone(),
                        }
                    };
                (f.name.clone(), value)
            })
            .collect();
        Self {
            profile,
            entry,
            outcome,
            initial: fields.clone(),
            fields,
            symbols: BTreeMap::new(),
            calls: 0,
            errno_calls: 0,
            reports: 0,
            writes: Vec::new(),
        }
    }

    fn field(&self, place: &CleanupReplayValueRecord) -> Option<String> {
        let CleanupReplayExpressionRecord::Field { base, name } = &place.expression else {
            return None;
        };
        if self.value(base, 0)? != CleanupLatticeValue::Root
            || base.c_type != self.profile.root_type
            || self.entry == CleanupEntryRecord::Null
        {
            return None;
        }
        self.profile
            .fields
            .iter()
            .any(|f: &CleanupFieldRecord| f.name == *name && f.c_type == place.c_type)
            .then(|| name.clone())
    }

    fn truth(&self, value: &CleanupLatticeValue) -> Option<bool> {
        match value {
            CleanupLatticeValue::Null => Some(false),
            CleanupLatticeValue::Root | CleanupLatticeValue::Pointer { .. } => Some(true),
            CleanupLatticeValue::Integer { value } => Some(*value != 0),
            _ => None,
        }
    }

    /// The symbolic equality partition. Only pairs the lattice decides are
    /// answered; everything else declines the whole projection.
    fn equal(&self, a: &CleanupLatticeValue, b: &CleanupLatticeValue) -> Option<bool> {
        if a == b {
            return Some(true);
        }
        match (a, b) {
            (CleanupLatticeValue::Integer { .. }, CleanupLatticeValue::Integer { .. }) => {
                Some(false)
            }
            (
                CleanupLatticeValue::Null,
                CleanupLatticeValue::Root | CleanupLatticeValue::Pointer { .. },
            )
            | (
                CleanupLatticeValue::Root | CleanupLatticeValue::Pointer { .. },
                CleanupLatticeValue::Null,
            ) => Some(false),
            (CleanupLatticeValue::Symbol { field }, CleanupLatticeValue::Integer { value: n })
            | (CleanupLatticeValue::Integer { value: n }, CleanupLatticeValue::Symbol { field })
                if *field == self.profile.presence_field
                    && self.profile.fields.iter().any(|f| {
                        f.name == *field && f.reset == CleanupResetRecord::Integer { value: *n }
                    }) =>
            {
                Some(false)
            }
            _ => None,
        }
    }

    fn value(&self, value: &CleanupReplayValueRecord, depth: u32) -> Option<CleanupLatticeValue> {
        if depth > MAX_EXPRESSION_DEPTH {
            return None;
        }
        match &value.expression {
            CleanupReplayExpressionRecord::Parameter { index: 0 }
                if value.c_type == self.profile.root_type =>
            {
                Some(if self.entry == CleanupEntryRecord::Null {
                    CleanupLatticeValue::Null
                } else {
                    CleanupLatticeValue::Root
                })
            }
            CleanupReplayExpressionRecord::Integer { value } => value
                .parse()
                .ok()
                .map(|value| CleanupLatticeValue::Integer { value }),
            CleanupReplayExpressionRecord::String { .. } => Some(CleanupLatticeValue::Text),
            CleanupReplayExpressionRecord::Symbol { operation } => {
                self.symbols.get(operation).cloned()
            }
            CleanupReplayExpressionRecord::Cast { cast, value: inner } => {
                let v = self.value(inner, depth + 1)?;
                match cast.as_str() {
                    "NullToPointer"
                        if matches!(
                            v,
                            CleanupLatticeValue::Null | CleanupLatticeValue::Integer { value: 0 }
                        ) && value.c_type.ends_with('*') =>
                    {
                        Some(CleanupLatticeValue::Null)
                    }
                    "BitCast" if inner.c_type.ends_with('*') && value.c_type.ends_with('*') => {
                        Some(v)
                    }
                    "NoOp"
                        if inner.c_type == value.c_type
                            || (inner.c_type.ends_with('*') && value.c_type.ends_with('*')) =>
                    {
                        Some(v)
                    }
                    "ArrayToPointerDecay"
                        if v == CleanupLatticeValue::Text && value.c_type.ends_with('*') =>
                    {
                        Some(v)
                    }
                    "IntegralCast"
                        if v == CleanupLatticeValue::Integer { value: 0 }
                            && matches!(
                                value.c_type.as_str(),
                                "unsigned long" | "unsigned int" | "int"
                            ) =>
                    {
                        Some(v)
                    }
                    _ => None,
                }
            }
            CleanupReplayExpressionRecord::Unary { operator, value } if operator == "!" => {
                let inner = self.value(value, depth + 1)?;
                Some(CleanupLatticeValue::Integer {
                    value: i32::from(!self.truth(&inner)?),
                })
            }
            CleanupReplayExpressionRecord::Unary { operator, value } if operator == "-" => {
                let CleanupLatticeValue::Integer { value: n } = self.value(value, depth + 1)?
                else {
                    return None;
                };
                n.checked_neg()
                    .map(|value| CleanupLatticeValue::Integer { value })
            }
            CleanupReplayExpressionRecord::Binary {
                operator,
                left,
                right,
            } if operator == "==" || operator == "!=" => {
                let equal = self.equal(
                    &self.value(left, depth + 1)?,
                    &self.value(right, depth + 1)?,
                )?;
                Some(CleanupLatticeValue::Integer {
                    value: i32::from(if operator == "==" { equal } else { !equal }),
                })
            }
            _ => None,
        }
    }

    /// Walk one complete path. `Some(None)` is "this prestate does not select
    /// this path"; `None` is "this projection is outside the vocabulary".
    fn run(&mut self, ops: &[CleanupReplayOperationRecord]) -> Option<Option<i32>> {
        for (index, op) in ops.iter().enumerate() {
            let index = u32::try_from(index).ok()?;
            match op {
                CleanupReplayOperationRecord::Read { place } => {
                    let v = if let CleanupReplayExpressionRecord::Unary { operator, value } =
                        &place.expression
                    {
                        if operator != "*"
                            || place.c_type != "int"
                            || self.value(value, 0)? != CleanupLatticeValue::ErrnoPointer
                        {
                            return None;
                        }
                        CleanupLatticeValue::Errno
                    } else {
                        self.fields.get(&self.field(place)?)?.clone()
                    };
                    self.symbols.insert(index, v);
                }
                CleanupReplayOperationRecord::Write { place, value } => {
                    if self.calls != 1 || self.outcome != Some(0) {
                        return None;
                    }
                    let field = self.field(place)?;
                    let value = self.value(value, 0)?;
                    self.fields.insert(field.clone(), value);
                    self.writes.push(field);
                }
                CleanupReplayOperationRecord::Guard { condition, taken } => {
                    let condition = self.value(condition, 0)?;
                    if self.truth(&condition)? != *taken {
                        return Some(None);
                    }
                }
                CleanupReplayOperationRecord::LocalWrite { value, .. } => {
                    self.value(value, 0)?;
                }
                CleanupReplayOperationRecord::LocalStart { .. }
                | CleanupReplayOperationRecord::LocalEnd { .. } => (),
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
                    let result = if *function == self.profile.syscall {
                        let expected = self
                            .profile
                            .argument_fields
                            .iter()
                            .map(|f| self.initial.get(f).cloned())
                            .collect::<Option<Vec<_>>>()?;
                        if *signature != self.profile.syscall_signature
                            || result_type != "int"
                            || self.calls != 0
                            || self.outcome.is_none()
                            || args != expected
                        {
                            return None;
                        }
                        self.calls += 1;
                        CleanupLatticeValue::Integer {
                            value: self.outcome?,
                        }
                    } else if *function == self.profile.errno_function
                        && signature == "int *(void)"
                        && result_type == "int *"
                        && args.is_empty()
                        && self.calls == 1
                        && self.outcome == Some(-1)
                        && self.errno_calls == 0
                    {
                        self.errno_calls += 1;
                        CleanupLatticeValue::ErrnoPointer
                    } else if *function == self.profile.report_function
                        && signature == "int (int, const char *, int)"
                        && result_type == "int"
                        && matches!(
                            args.as_slice(),
                            [
                                CleanupLatticeValue::Errno,
                                CleanupLatticeValue::Text,
                                CleanupLatticeValue::Integer { value }
                            ] if *value > 0
                        )
                        && self.errno_calls == 1
                        && self.reports == 0
                    {
                        self.reports += 1;
                        CleanupLatticeValue::ReportResult
                    } else {
                        return None;
                    };
                    self.symbols.insert(index, result);
                }
                CleanupReplayOperationRecord::ReturnValue { value }
                    if usize::try_from(index).ok()? + 1 == ops.len() && value.c_type == "int" =>
                {
                    let CleanupLatticeValue::Integer { value } = self.value(value, 0)? else {
                        return None;
                    };
                    return Some(Some(value));
                }
                CleanupReplayOperationRecord::ReturnValue { .. } => return None,
                // The value-free return exists on the shared record enum for
                // the composition route's `void` wrapper only. This family's
                // bodies all return a value, so it is declined by name here
                // exactly as the gateway projector refuses it on the way out
                // (`P04-COMPOSE-SHARED-PATHS`).
                CleanupReplayOperationRecord::Return => return None,
            }
        }
        None
    }
}

/// The native cleanup outcome family: four sampled prestates over one
/// eligibility field the successful arm resets.
/// Public so the composition producer can recompute this prerequisite reading
/// itself rather than being handed one. It is the same computation this
/// capability performs standalone: a composition producer that could not
/// recompute it would have to be *told* the prerequisite's conclusion, which
/// the composition vocabulary deliberately cannot express.
pub fn replay_native_cleanup_outcome(
    body: &CleanupReplayFunctionRecord,
    profile: &CleanupProfileRecord,
) -> Option<CleanupReplayRecord> {
    if !bounded(body, profile) {
        return None;
    }
    let mut cases = Vec::new();
    let mut covered = BTreeSet::new();
    for prestate in CleanupPrestateRecord::native_cleanup_outcome_domain() {
        let CleanupPrestateRecord::NativeCleanupOutcome {
            entry,
            syscall_result,
        } = prestate
        else {
            return None;
        };
        let mut selected = Vec::new();
        for (index, path) in body.paths.iter().enumerate() {
            let mut replay = Replay::new(profile, entry, syscall_result);
            let initial_fields = replay.initial.clone();
            let Some(returned) = replay.run(&path.operations)? else {
                continue;
            };
            let success = syscall_result == Some(0);
            let fields_preserved = replay.fields == replay.initial;
            let fields_reset = profile
                .fields
                .iter()
                .all(|f| replay.fields.get(&f.name) == Some(&reset_value(f.reset)));
            // The same structural transition the trusted checker requires. A
            // body that violates it is declined here and refused there.
            if replay.calls != u32::from(syscall_result.is_some())
                || replay.writes.len() != if success { profile.fields.len() } else { 0 }
                || (success && !fields_reset)
                || (!success && !fields_preserved)
                || returned
                    != if entry == CleanupEntryRecord::Null || syscall_result == Some(-1) {
                        -1
                    } else {
                        0
                    }
                || replay.errno_calls != u32::from(syscall_result == Some(-1))
                || replay.reports != replay.errno_calls
            {
                return None;
            }
            selected.push(CleanupReplayCaseRecord::NativeCleanupOutcome {
                entry,
                syscall_result,
                initial_fields,
                source_path: u32::try_from(index).ok()?,
                returned,
                cleanup_calls: replay.calls,
                field_writes: replay.writes.clone(),
                fields_preserved,
                fields_reset,
            });
        }
        let [case] = selected.as_slice() else {
            return None;
        };
        let CleanupReplayCaseRecord::NativeCleanupOutcome { source_path, .. } = case else {
            return None;
        };
        covered.insert(*source_path);
        cases.push(case.clone());
    }
    (covered.len() == body.paths.len()).then(|| record(body, cases))
}

/// The guarded delete family: five sampled prestates over two eligibility
/// fields, where every arm preserves the record.
///
/// Public for the same reason as the outcome family above: the deleting
/// `file_close` composition producer must recompute this prerequisite reading
/// itself, because the composition vocabulary cannot carry a prerequisite's
/// conclusion.
pub fn replay_guarded_delete(
    body: &CleanupReplayFunctionRecord,
    profile: &CleanupProfileRecord,
    mapping_field: &str,
) -> Option<CleanupReplayRecord> {
    if !bounded(body, profile) {
        return None;
    }
    let [path_field] = profile.argument_fields.as_slice() else {
        return None;
    };
    let field = |name: &str| profile.fields.iter().find(|f| f.name == name);
    if profile.fields.len() != 3
        || path_field == mapping_field
        || path_field == &profile.presence_field
        || mapping_field == profile.presence_field
        || !field(&profile.presence_field).is_some_and(|f| {
            f.c_type == "int" && f.reset == CleanupResetRecord::Integer { value: -1 }
        })
        || !field(mapping_field)
            .is_some_and(|f| f.c_type == "char *" && f.reset == CleanupResetRecord::Null)
        || !field(path_field)
            .is_some_and(|f| f.c_type == "char *" && f.reset == CleanupResetRecord::Null)
    {
        return None;
    }
    let mut cases = Vec::new();
    let mut covered = BTreeSet::new();
    for prestate in CleanupPrestateRecord::guarded_delete_domain() {
        let CleanupPrestateRecord::GuardedDelete {
            descriptor_field_active,
            mapping_field_active,
            syscall_result,
        } = prestate
        else {
            return None;
        };
        let mut selected = Vec::new();
        for (index, path) in body.paths.iter().enumerate() {
            let mut replay = Replay::new(profile, CleanupEntryRecord::Present, syscall_result);
            if !descriptor_field_active {
                replay.fields.insert(
                    profile.presence_field.clone(),
                    CleanupLatticeValue::Integer { value: -1 },
                );
            }
            if !mapping_field_active {
                replay
                    .fields
                    .insert(mapping_field.to_owned(), CleanupLatticeValue::Null);
            }
            replay.initial = replay.fields.clone();
            let initial_fields = replay.initial.clone();
            let Some(returned) = replay.run(&path.operations)? else {
                continue;
            };
            let preserved = replay.fields == replay.initial && replay.writes.is_empty();
            let success = syscall_result == Some(0);
            if !preserved
                || returned != if success { 0 } else { -1 }
                || replay.calls != u32::from(syscall_result.is_some())
                || replay.errno_calls != u32::from(syscall_result == Some(-1))
                || replay.reports != replay.errno_calls
            {
                return None;
            }
            selected.push(CleanupReplayCaseRecord::GuardedDelete {
                descriptor_field_active,
                mapping_field_active,
                syscall_result,
                initial_fields,
                source_path: u32::try_from(index).ok()?,
                returned,
                syscall_calls: replay.calls,
                diagnostic_calls: replay.reports,
                fields_preserved: preserved,
            });
        }
        let [case] = selected.as_slice() else {
            return None;
        };
        let CleanupReplayCaseRecord::GuardedDelete { source_path, .. } = case else {
            return None;
        };
        covered.insert(*source_path);
        cases.push(case.clone());
    }
    (covered.len() == body.paths.len()).then(|| record(body, cases))
}

fn record(
    body: &CleanupReplayFunctionRecord,
    cases: Vec<CleanupReplayCaseRecord>,
) -> CleanupReplayRecord {
    CleanupReplayRecord {
        function: body.function.clone(),
        source_revision: body.source_revision.clone(),
        signature: body.signature.clone(),
        cases,
        profile_assumptions: body.profile_assumptions.clone(),
    }
}

fn derive(source: &CleanupReplaySourceInput) -> Option<CleanupReplayProposalRecord> {
    if !source.version_matches() || !source.prerequisite.is_complete() {
        return None;
    }
    match &source.family {
        CleanupReplayFamilyInput::NativeCleanupOutcome { body, profile } => {
            replay_native_cleanup_outcome(body, profile)
                .map(CleanupReplayProposalRecord::NativeCleanupOutcome)
        }
        CleanupReplayFamilyInput::GuardedDelete {
            body,
            profile,
            mapping_field,
        } => replay_guarded_delete(body, profile, mapping_field)
            .map(CleanupReplayProposalRecord::GuardedDelete),
    }
}

pub fn propose(
    request: &VerificationRequestV2,
    manifest: &PluginManifest,
) -> Option<VerificationResponseV2> {
    if request.capability != VerifierCapability::ResourceContracts
        || request.target.profile != CLEANUP_REPLAY_SEMANTIC_PROFILE
        || !matches!(
            request.target.dialect.as_str(),
            "c11" | "gnu99" | "gnu11" | "gnu17"
        )
    {
        return None;
    }
    let summary = derive(request.cleanup_replay_source.as_ref()?)?;
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
            witness: SemanticWitness::CleanupReplay(Box::new(CleanupReplayWitness {
                task_digest: request.task_digest.clone(),
                summary,
            })),
        }),
    })
}

#[cfg(test)]
mod tests;
