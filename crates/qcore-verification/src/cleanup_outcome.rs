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

//! Complete flat-path replay for nullable native cleanup helpers. Symbolic field
//! identities are never replaced by sampled integers. External cleanup outcomes
//! and diagnostic framing are explicit assumptions, not inferred OS semantics.
//!
//! Card `P04-B1`: the `CleanupProfile` is no longer free Rust data. Both entry
//! points require the pinned contract to *declare* the field inventory, the
//! syscall and helper names and the C types they bind
//! ([`crate::cleanup_vocabulary`]), refuse a profile that is not exactly that
//! declaration, and then check the declaration against the pinned typed source
//! by the same replay as before. No accepted input changed: the declaration is
//! a referent, not a new semantic rule.
use crate::cleanup_vocabulary::declared_cleanup_vocabulary;
use qcore_c_gateway::typed_source::{
    SourceExpression as Expr, SourceOperation as Op, SourceSummary, SourceValue,
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CleanupProfile {
    pub function: String,
    pub root_type: String,
    pub syscall: String,
    pub syscall_signature: String,
    pub presence_field: String,
    pub fields: Vec<CleanupField>,
    pub argument_fields: Vec<String>,
    pub policy: String,
    pub errno_function: String,
    pub report_function: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CleanupField {
    pub name: String,
    pub c_type: String,
    pub reset: ResetValue,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum ResetValue {
    Null,
    Integer(i32),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum CleanupEntry {
    Null,
    Absent,
    Present,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CleanupCase {
    pub entry: CleanupEntry,
    pub syscall_result: Option<i32>,
    pub source_path: usize,
    pub returned: i32,
    pub cleanup_calls: usize,
    pub field_writes: Vec<String>,
    pub fields_preserved: bool,
    pub fields_reset: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CleanupCertificate {
    source: SourceSummary,
    profile: CleanupProfile,
    contract_source: String,
    cases: Vec<CleanupCase>,
}
impl CleanupCertificate {
    pub fn source(&self) -> &SourceSummary {
        &self.source
    }
    pub fn cases(&self) -> &[CleanupCase] {
        &self.cases
    }
}

/// Evaluate one policy over the case facts plus the vocabulary its own
/// contract declares. Every declared clause must be supplied or the predicate
/// interpreter reports `missing observation`, so a contract cannot declare a
/// referent this checker does not recognise.
fn check_facts(
    contract: &str,
    policy: &str,
    facts: &[(&str, i128)],
    declared: &[(String, i128)],
) -> Result<(), String> {
    let all: Vec<(&str, i128)> = facts
        .iter()
        .copied()
        .chain(declared.iter().map(|(name, value)| (name.as_str(), *value)))
        .collect();
    qcore_contract::contract_observation::check_contract_facts(contract, policy, &all)
}

pub fn check_cleanup_outcomes(
    source: &SourceSummary,
    profile: &CleanupProfile,
    contract: &str,
) -> Result<CleanupCertificate, String> {
    if source.function() != profile.function
        || source.signature() != format!("int ({})", profile.root_type)
        || source.has_scan_loop()
    {
        return Err("cleanup source signature or flat-path profile mismatch".into());
    }
    let fields: BTreeSet<_> = profile.fields.iter().map(|f| f.name.as_str()).collect();
    if fields.len() != profile.fields.len()
        || fields.is_empty()
        || !fields.contains(profile.presence_field.as_str())
        || profile.argument_fields.is_empty()
        || profile
            .argument_fields
            .iter()
            .any(|f| !fields.contains(f.as_str()))
    {
        return Err("invalid cleanup field profile".into());
    }
    // The profile must be the vocabulary its own contract declares, before any
    // path is replayed against it.
    let vocabulary = declared_cleanup_vocabulary(contract, &profile.policy)?;
    vocabulary.bind(profile, None)?;
    let declared = vocabulary.facts();
    let mut cases = Vec::new();
    let mut covered = BTreeSet::new();
    for (entry, outcome) in [
        (CleanupEntry::Null, None),
        (CleanupEntry::Absent, None),
        (CleanupEntry::Present, Some(-1)),
        (CleanupEntry::Present, Some(0)),
    ] {
        let mut selected = Vec::new();
        for (index, path) in source.paths().iter().enumerate() {
            let mut replay = Replay::new(profile, entry, outcome);
            if let Some(returned) = replay.run(&path.operations)? {
                let case = CleanupCase {
                    entry,
                    syscall_result: outcome,
                    source_path: index,
                    returned,
                    cleanup_calls: replay.calls,
                    field_writes: replay.writes.clone(),
                    fields_preserved: replay.fields == replay.initial,
                    fields_reset: profile
                        .fields
                        .iter()
                        .all(|f| replay.fields.get(&f.name) == Some(&reset(f.reset))),
                };
                let success = outcome == Some(0);
                // Structural coverage and exact source effects remain mandatory even
                // if the caller weakens a declarative predicate.
                if case.cleanup_calls != usize::from(outcome.is_some())
                    || case.field_writes.len() != if success { profile.fields.len() } else { 0 }
                    || (success && !case.fields_reset)
                    || (!success && !case.fields_preserved)
                    || case.returned
                        != if entry == CleanupEntry::Null || outcome == Some(-1) {
                            -1
                        } else {
                            0
                        }
                    || replay.errno_calls != usize::from(outcome == Some(-1))
                    || replay.reports != replay.errno_calls
                {
                    return Err("cleanup source violates outcome/field transition".into());
                }
                check_facts(
                    contract,
                    &profile.policy,
                    &[
                        ("nullable_live_exclusive_record", 1),
                        ("present_field_has_valid_syscall_argument_identity", 1),
                        ("syscall_returns_zero_or_minus_one_and_frames_record", 1),
                        ("diagnostics_return_and_frame_record", 1),
                        ("source_and_abi_bindings", 1),
                        ("was_null", i128::from(entry == CleanupEntry::Null)),
                        ("was_present", i128::from(entry == CleanupEntry::Present)),
                        ("syscall_succeeded", i128::from(success)),
                        ("syscall_failed", i128::from(outcome == Some(-1))),
                        ("returned", i128::from(returned)),
                        ("cleanup_calls", case.cleanup_calls as i128),
                        ("field_writes", case.field_writes.len() as i128),
                        ("reset_field_count", profile.fields.len() as i128),
                        ("fields_preserved", i128::from(case.fields_preserved)),
                        ("fields_reset", i128::from(case.fields_reset)),
                        ("root_preserved", 1),
                        ("other_fields_preserved", 1),
                    ],
                    &declared,
                )?;
                selected.push(case);
            }
        }
        if selected.len() != 1 {
            return Err("cleanup entry/outcome must select exactly one complete path".into());
        }
        let case = selected.remove(0);
        covered.insert(case.source_path);
        cases.push(case);
    }
    if covered.len() != source.paths().len() {
        return Err("cleanup source has uncovered paths".into());
    }
    Ok(CleanupCertificate {
        source: source.clone(),
        profile: profile.clone(),
        contract_source: contract.into(),
        cases,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Value {
    Null,
    Root,
    Pointer(String),
    Integer(i32),
    Symbol(String),
    ErrnoPointer,
    Errno,
    Text,
    ReportResult,
}
fn reset(value: ResetValue) -> Value {
    match value {
        ResetValue::Null => Value::Null,
        ResetValue::Integer(n) => Value::Integer(n),
    }
}
struct Replay<'a> {
    profile: &'a CleanupProfile,
    entry: CleanupEntry,
    outcome: Option<i32>,
    initial: BTreeMap<String, Value>,
    fields: BTreeMap<String, Value>,
    symbols: BTreeMap<usize, Value>,
    calls: usize,
    errno_calls: usize,
    reports: usize,
    writes: Vec<String>,
}
impl<'a> Replay<'a> {
    fn new(profile: &'a CleanupProfile, entry: CleanupEntry, outcome: Option<i32>) -> Self {
        let fields: BTreeMap<_, _> = profile
            .fields
            .iter()
            .map(|f| {
                let value = if f.name == profile.presence_field && entry == CleanupEntry::Absent {
                    reset(f.reset)
                } else if f.c_type.ends_with('*') {
                    Value::Pointer(f.name.clone())
                } else {
                    Value::Symbol(f.name.clone())
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
    fn field(&self, place: &SourceValue) -> Result<String, String> {
        let Expr::Field { base, name } = &place.expression else {
            return Err("cleanup access must be a direct record field".into());
        };
        if self.value(base)? != Value::Root
            || base.c_type != self.profile.root_type
            || self.entry == CleanupEntry::Null
        {
            return Err("cleanup field root is null or has wrong identity/type".into());
        }
        if !self
            .profile
            .fields
            .iter()
            .any(|f| f.name == *name && f.c_type == place.c_type)
        {
            return Err("cleanup access outside typed field profile".into());
        }
        Ok(name.clone())
    }
    fn truth(&self, value: Value) -> Result<bool, String> {
        match value {
            Value::Null => Ok(false),
            Value::Root | Value::Pointer(_) => Ok(true),
            Value::Integer(n) => Ok(n != 0),
            _ => Err("symbolic cleanup truth value is unresolved".into()),
        }
    }
    fn equal(&self, a: Value, b: Value) -> Result<bool, String> {
        if a == b {
            return Ok(true);
        }
        match (&a, &b) {
            (Value::Integer(_), Value::Integer(_)) => Ok(false),
            (Value::Null, Value::Root | Value::Pointer(_))
            | (Value::Root | Value::Pointer(_), Value::Null) => Ok(false),
            (Value::Symbol(name), Value::Integer(n)) | (Value::Integer(n), Value::Symbol(name))
                if name == &self.profile.presence_field
                    && self
                        .profile
                        .fields
                        .iter()
                        .any(|f| f.name == *name && f.reset == ResetValue::Integer(*n)) =>
            {
                Ok(false)
            }
            _ => Err("cleanup equality outside symbolic partition".into()),
        }
    }
    fn value(&self, value: &SourceValue) -> Result<Value, String> {
        match &value.expression {
            Expr::Parameter { index: 0 } if value.c_type == self.profile.root_type => {
                Ok(if self.entry == CleanupEntry::Null {
                    Value::Null
                } else {
                    Value::Root
                })
            }
            Expr::Integer { value } => Ok(Value::Integer(
                value.parse().map_err(|_| "cleanup integer outside i32")?,
            )),
            Expr::String { .. } => Ok(Value::Text),
            Expr::Symbol { operation } => self
                .symbols
                .get(operation)
                .cloned()
                .ok_or("missing cleanup source producer".into()),
            Expr::Cast { cast, value: inner } => {
                let v = self.value(inner)?;
                match cast.as_str() {
                    "NullToPointer"
                        if matches!(v, Value::Null | Value::Integer(0))
                            && value.c_type.ends_with('*') =>
                    {
                        Ok(Value::Null)
                    }
                    "BitCast" if inner.c_type.ends_with('*') && value.c_type.ends_with('*') => {
                        Ok(v)
                    }
                    "NoOp"
                        if inner.c_type == value.c_type
                            || (inner.c_type.ends_with('*') && value.c_type.ends_with('*')) =>
                    {
                        Ok(v)
                    }
                    "ArrayToPointerDecay" if v == Value::Text && value.c_type.ends_with('*') => {
                        Ok(v)
                    }
                    "IntegralCast"
                        if v == Value::Integer(0)
                            && matches!(
                                value.c_type.as_str(),
                                "unsigned long" | "unsigned long long" | "unsigned int" | "int"
                            ) =>
                    {
                        Ok(v)
                    }
                    _ => Err("cleanup cast outside value-preserving profile".into()),
                }
            }
            Expr::Unary { operator, value } if operator == "!" => {
                Ok(Value::Integer(i32::from(!self.truth(self.value(value)?)?)))
            }
            Expr::Unary { operator, value } if operator == "-" => {
                let Value::Integer(n) = self.value(value)? else {
                    return Err("symbolic cleanup negation".into());
                };
                Ok(Value::Integer(
                    n.checked_neg().ok_or("cleanup negation overflow")?,
                ))
            }
            Expr::Binary {
                operator,
                left,
                right,
            } if operator == "==" || operator == "!=" => {
                let equal = self.equal(self.value(left)?, self.value(right)?)?;
                Ok(Value::Integer(i32::from(if operator == "==" {
                    equal
                } else {
                    !equal
                })))
            }
            _ => Err("cleanup expression outside symbolic profile".into()),
        }
    }
    fn run(&mut self, ops: &[Op]) -> Result<Option<i32>, String> {
        for (index, op) in ops.iter().enumerate() {
            match op {
                Op::Read { place } => {
                    let v = if let Expr::Unary { operator, value } = &place.expression {
                        if operator != "*"
                            || place.c_type != "int"
                            || self.value(value)? != Value::ErrnoPointer
                        {
                            return Err("cleanup indirect read is not diagnostic errno".into());
                        }
                        Value::Errno
                    } else {
                        self.fields[&self.field(place)?].clone()
                    };
                    self.symbols.insert(index, v);
                }
                Op::Write { place, value } => {
                    if self.calls != 1 || self.outcome != Some(0) {
                        return Err("cleanup writes before successful syscall".into());
                    }
                    let field = self.field(place)?;
                    let value = self.value(value)?;
                    self.fields.insert(field.clone(), value);
                    self.writes.push(field);
                }
                Op::Guard { condition, taken } => {
                    if self.truth(self.value(condition)?)? != *taken {
                        return Ok(None);
                    }
                }
                Op::LocalWrite { value, .. } => {
                    self.value(value)?;
                }
                Op::LocalStart { .. } | Op::LocalEnd { .. } => (),
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
                    let result = if function == &self.profile.syscall {
                        if signature != &self.profile.syscall_signature
                            || result_type != "int"
                            || self.calls != 0
                            || self.outcome.is_none()
                            || args
                                != self
                                    .profile
                                    .argument_fields
                                    .iter()
                                    .map(|f| self.initial[f].clone())
                                    .collect::<Vec<_>>()
                        {
                            return Err(
                                "cleanup syscall identity/signature/outcome mismatch".into()
                            );
                        }
                        self.calls += 1;
                        Value::Integer(self.outcome.unwrap())
                    } else if function == &self.profile.errno_function
                        && signature == "int *(void)"
                        && result_type == "int *"
                        && args.is_empty()
                        && self.calls == 1
                        && self.outcome == Some(-1)
                        && self.errno_calls == 0
                    {
                        self.errno_calls += 1;
                        Value::ErrnoPointer
                    } else if function == &self.profile.report_function
                        && signature == "int (int, const char *, int)"
                        && result_type == "int"
                        && matches!(args.as_slice(), [Value::Errno, Value::Text, Value::Integer(n)] if *n > 0)
                        && self.errno_calls == 1
                        && self.reports == 0
                    {
                        self.reports += 1;
                        Value::ReportResult
                    } else {
                        return Err(format!("unresolved cleanup call {function}"));
                    };
                    self.symbols.insert(index, result);
                }
                Op::ReturnValue { value } if index + 1 == ops.len() && value.c_type == "int" => {
                    let Value::Integer(n) = self.value(value)? else {
                        return Err("cleanup result remains symbolic".into());
                    };
                    return Ok(Some(n));
                }
                _ => return Err("unsupported or nonterminal cleanup operation".into()),
            }
        }
        Err("cleanup path lacks terminal return".into())
    }
}

/// Conditional deletion of backing content. The record and its path allocation
/// remain live; only a successful external syscall can establish content removal.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct GuardedDeleteCase {
    pub descriptor_field_active: bool,
    pub mapping_field_active: bool,
    pub syscall_result: Option<i32>,
    pub source_path: usize,
    pub returned: i32,
    pub syscall_calls: usize,
    pub diagnostic_calls: usize,
    pub fields_preserved: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct GuardedDeleteCertificate {
    source: SourceSummary,
    profile: CleanupProfile,
    mapping_field: String,
    contract_source: String,
    cases: Vec<GuardedDeleteCase>,
}
impl GuardedDeleteCertificate {
    pub fn source(&self) -> &SourceSummary {
        &self.source
    }
    pub fn cases(&self) -> &[GuardedDeleteCase] {
        &self.cases
    }
}

pub fn check_guarded_delete_outcomes(
    source: &SourceSummary,
    profile: &CleanupProfile,
    mapping_field: &str,
    contract: &str,
) -> Result<GuardedDeleteCertificate, String> {
    let field = |name: &str| profile.fields.iter().find(|f| f.name == name);
    let [path_field] = profile.argument_fields.as_slice() else {
        return Err("delete requires one path argument".into());
    };
    if source.function() != profile.function
        || source.signature() != format!("int ({})", profile.root_type)
        || source.has_scan_loop()
        || profile.fields.len() != 3
        || profile
            .fields
            .iter()
            .map(|f| &f.name)
            .collect::<BTreeSet<_>>()
            .len()
            != 3
        || path_field == mapping_field
        || path_field == &profile.presence_field
        || mapping_field == profile.presence_field
        || !field(&profile.presence_field)
            .is_some_and(|f| f.c_type == "int" && f.reset == ResetValue::Integer(-1))
        || !field(mapping_field)
            .is_some_and(|f| f.c_type == "char *" && f.reset == ResetValue::Null)
        || !field(path_field).is_some_and(|f| f.c_type == "char *" && f.reset == ResetValue::Null)
    {
        return Err("guarded delete source/field profile mismatch".into());
    }
    // The eligibility field is not a `CleanupProfile` member, so it is bound
    // separately: the delete policy must declare it and the close/unmap
    // policies must not.
    let vocabulary = declared_cleanup_vocabulary(contract, &profile.policy)?;
    vocabulary.bind(profile, Some(mapping_field))?;
    let declared = vocabulary.facts();
    let mut cases = Vec::new();
    let mut covered = BTreeSet::new();
    for (open, mapped, outcome) in [
        (true, false, None),
        (true, true, None),
        (false, true, None),
        (false, false, Some(-1)),
        (false, false, Some(0)),
    ] {
        let mut selected = Vec::new();
        for (index, path) in source.paths().iter().enumerate() {
            let mut replay = Replay::new(profile, CleanupEntry::Present, outcome);
            if !open {
                replay
                    .fields
                    .insert(profile.presence_field.clone(), Value::Integer(-1));
            }
            if !mapped {
                replay.fields.insert(mapping_field.into(), Value::Null);
            }
            replay.initial = replay.fields.clone();
            if let Some(returned) = replay.run(&path.operations)? {
                let preserved = replay.fields == replay.initial && replay.writes.is_empty();
                let success = outcome == Some(0);
                if !preserved
                    || returned != if success { 0 } else { -1 }
                    || replay.calls != usize::from(outcome.is_some())
                    || replay.errno_calls != usize::from(outcome == Some(-1))
                    || replay.reports != replay.errno_calls
                {
                    return Err(
                        "guarded delete violates eligibility/outcome/frame transition".into(),
                    );
                }
                check_facts(
                    contract,
                    &profile.policy,
                    &[
                        ("nonnull_live_exclusive_record", 1),
                        ("path_is_live_readable_terminated_string", 1),
                        (
                            "syscall_returns_zero_or_minus_one_and_frames_record_and_path",
                            1,
                        ),
                        ("diagnostics_return_and_frame_record_and_path", 1),
                        ("source_and_abi_bindings", 1),
                        ("descriptor_field_active", i128::from(open)),
                        ("mapping_field_active", i128::from(mapped)),
                        ("syscall_calls", replay.calls as i128),
                        ("syscall_succeeded", i128::from(success)),
                        ("syscall_failed", i128::from(outcome == Some(-1))),
                        ("diagnostic_calls", replay.reports as i128),
                        ("returned", i128::from(returned)),
                        ("fields_preserved", i128::from(preserved)),
                        ("record_and_path_allocations_preserved", 1),
                    ],
                    &declared,
                )?;
                selected.push(GuardedDeleteCase {
                    descriptor_field_active: open,
                    mapping_field_active: mapped,
                    syscall_result: outcome,
                    source_path: index,
                    returned,
                    syscall_calls: replay.calls,
                    diagnostic_calls: replay.reports,
                    fields_preserved: preserved,
                });
            }
        }
        if selected.len() != 1 {
            return Err("delete entry/outcome must select exactly one complete source path".into());
        }
        let case = selected.remove(0);
        covered.insert(case.source_path);
        cases.push(case);
    }
    if covered.len() != source.paths().len() {
        return Err("delete has uncovered source paths".into());
    }
    Ok(GuardedDeleteCertificate {
        source: source.clone(),
        profile: profile.clone(),
        mapping_field: mapping_field.into(),
        contract_source: contract.into(),
        cases,
    })
}
