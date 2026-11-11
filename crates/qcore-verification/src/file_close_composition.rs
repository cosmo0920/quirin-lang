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

//! The bounded ChunkIO **`cio_file_close` compositions**: readings over other
//! capabilities' certificates, one of which is *itself* a composition.
//!
//! Relocated verbatim from `qcore_compiler::verifier_plugins::chunkio::
//! file_close` (card `P04-COMPOSE-A`) so the retained in-process adapter and the
//! moved process route call **one** decision procedure and cannot drift. The
//! only change to the moved body is that the constants it read from its former
//! module scope — the pinned revision, the nested unmap composition's contracts
//! and record profile, the native close contract and profile, and the
//! guarded-delete contract, profile and declared eligibility field — are now
//! explicit parameters ([`FileCloseContracts`], [`FileCloseDeleteContracts`]).
//! The published certificates' serialized shapes are byte-identical, which the
//! retained adapter's archived corpus artifacts depend on.
//!
//! Every prerequisite certificate handed in is **re-evaluated here** from its
//! own retained summaries against its own pinned contracts, and the nested unmap
//! composition re-establishes *its* three prerequisites in turn, so a chain is
//! never admitted by its weakest link and a certificate a caller supplies is
//! never taken on trust.
//!
//! Nothing here observes or claims OS behaviour. Releasing the backend's C
//! allocation does not erase unresolved mapping or descriptor cleanup
//! obligations, a recorded `close`, `munmap` or `unlink` call is a recorded
//! call, and freeing the C path storage is never accepted as successful backing
//! deletion.

use qcore_c_gateway::typed_source::{
    SourceExpression as Expr, SourceOperation as Op, SourceSummary, SourceValue,
};
use qcore_contract::contract_observation::check_contract_facts;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

use crate::cleanup_outcome::{
    CleanupCertificate, CleanupEntry, CleanupProfile, GuardedDeleteCertificate,
    check_cleanup_outcomes, check_guarded_delete_outcomes,
};
use crate::unmap_composition::{UnmapCertificate, UnmapCompositionContracts};

/// The pinned inputs the non-deleting composition reads. None of them is
/// inferred from a proposal: on the moved process route the trusted core
/// supplies its own contract texts and its own contract-derived record profiles.
pub struct FileCloseContracts<'a> {
    /// The revision every participating summary must declare.
    pub pinned_revision: &'a str,
    /// The nested unmap composition's own pinned inputs, used to re-run it.
    pub unmap: &'a UnmapCompositionContracts<'a>,
    /// The native close prerequisite: the record shape its replay evolves and
    /// its own pinned contract.
    pub close_function: &'a str,
    pub close_profile: &'a CleanupProfile,
    pub close_contract: &'a str,
    /// The composition's own pinned contract.
    pub contract: &'a str,
}

/// The pinned inputs the deleting arm additionally reads.
pub struct FileCloseDeleteContracts<'a> {
    pub delete_function: &'a str,
    pub delete_profile: &'a CleanupProfile,
    /// The declared eligibility field, which is not a profile member.
    pub delete_mapping_field: &'a str,
    pub delete_contract: &'a str,
    /// The deleting arm's own pinned contract.
    pub contract: &'a str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum ResidualCustody {
    Absent,
    Unresolved,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FileCloseCase {
    pub source_path: usize,
    pub backend_present: bool,
    pub path_allocation_present: bool,
    pub unmap_case: Option<usize>,
    pub close_case: Option<usize>,
    pub mapping: ResidualCustody,
    pub descriptor: ResidualCustody,
    pub path_released: bool,
    pub backend_released: bool,
    pub up_counter_decrements: usize,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FileCloseEvidence {
    source: SourceSummary,
    unmap: UnmapCertificate,
    close: CleanupCertificate,
    contract_source: String,
    cases: Vec<FileCloseCase>,
    uncovered_paths: Vec<usize>,
    status: String,
}
impl FileCloseEvidence {
    pub fn cases(&self) -> &[FileCloseCase] {
        &self.cases
    }
    pub fn uncovered_paths(&self) -> &[usize] {
        &self.uncovered_paths
    }
    /// The imported wrapper body, so a dependent can require that the body it
    /// composes over is the one this reading was taken from.
    pub fn source(&self) -> &SourceSummary {
        &self.source
    }
    /// The nested composition's certificate, re-established by this reading.
    pub fn unmap(&self) -> &UnmapCertificate {
        &self.unmap
    }
    pub fn close(&self) -> &CleanupCertificate {
        &self.close
    }
    /// Only a conditional nested-resource gate; never a whole-chunk or whole-unit
    /// safety verdict. OS outcomes and caller provenance still need binding.
    pub fn require_nested_discharge(&self, case: usize) -> Result<(), String> {
        let c = self.cases.get(case).ok_or("file-close case out of range")?;
        if c.mapping == ResidualCustody::Unresolved || c.descriptor == ResidualCustody::Unresolved {
            return Err("Unknown: backend release retains unresolved OS cleanup custody".into());
        }
        Ok(())
    }
}

/// Compose the `cio_file_close` body with its re-established prerequisites.
pub fn check_file_close_composition(
    source: &SourceSummary,
    unmap: &UnmapCertificate,
    close: &CleanupCertificate,
    contracts: &FileCloseContracts<'_>,
) -> Result<FileCloseEvidence, String> {
    if source.function() != "cio_file_close"
        || source.signature() != "void (struct cio_chunk *, int)"
        || source.source_revision() != contracts.pinned_revision
        || unmap.source().source_revision() != source.source_revision()
        || close.source().source_revision() != source.source_revision()
        || close.source().function() != contracts.close_function
    {
        return Err("file-close source/dependency signature/revision mismatch".into());
    }
    // The nested composition is re-run from its own retained summaries against
    // its own pinned contracts; it in turn re-evaluates its own two
    // prerequisites. Nothing a caller supplies is taken on trust.
    let unmap = unmap.recheck(contracts.unmap)?;
    let close = check_cleanup_outcomes(
        close.source(),
        contracts.close_profile,
        contracts.close_contract,
    )?;
    let mut entries = vec![(None, None, false)];
    for (u, _) in unmap
        .cases()
        .iter()
        .enumerate()
        .filter(|(_, c)| c.backend_present)
    {
        for (c, _) in close
            .cases()
            .iter()
            .enumerate()
            .filter(|(_, c)| c.entry != CleanupEntry::Null)
        {
            for path_live in [false, true] {
                entries.push((Some(u), Some(c), path_live));
            }
        }
    }
    let mut cases = Vec::new();
    let mut covered = BTreeSet::new();
    for (u, c, path_live) in entries {
        let mut selected = Vec::new();
        for (index, path) in source.paths().iter().enumerate() {
            let mut replay = Replay {
                backend: u.is_some(),
                path_live,
                phase: 0,
                deleting: false,
                delete_result: None,
                delete_called: false,
                logged: false,
                symbols: BTreeMap::new(),
            };
            if replay.run(&path.operations)? {
                if replay.phase != if u.is_some() { 4 } else { 0 } {
                    return Err("file close lacks complete non-deleting release chain".into());
                }
                let mapping = if u.is_some_and(|i| unmap.cases()[i].native_result == Some(-1)) {
                    ResidualCustody::Unresolved
                } else {
                    ResidualCustody::Absent
                };
                let descriptor = if c.is_some_and(|i| close.cases()[i].syscall_result == Some(-1)) {
                    ResidualCustody::Unresolved
                } else {
                    ResidualCustody::Absent
                };
                let case = FileCloseCase {
                    source_path: index,
                    backend_present: u.is_some(),
                    path_allocation_present: path_live,
                    unmap_case: u,
                    close_case: c,
                    mapping,
                    descriptor,
                    path_released: path_live,
                    backend_released: u.is_some(),
                    up_counter_decrements: u.map_or(0, |i| unmap.cases()[i].counter_decrements),
                };
                check_contract_facts(
                    contracts.contract,
                    "file_close_no_delete",
                    &[
                        ("live_exclusive_chunk_with_nullable_owned_file_backend", 1),
                        ("backend_and_nullable_path_disjoint_owned_provenance", 1),
                        ("backend_and_path_are_disjoint_from_chunk_root", 1),
                        ("delete_argument_is_zero", 1),
                        ("chunk_context_stream_and_names_outlive_calls", 1),
                        ("unmap_dependency_premises_and_counter_bounds", 1),
                        ("native_close_frames_mapping_and_record_lifetimes", 1),
                        ("unmap_frames_descriptor_and_owned_path", 1),
                        ("successful_syscalls_discharge_the_bound_os_resource", 1),
                        ("cross_unit_source_abi_and_libc_free_bindings", 1),
                        ("was_nonnull", i128::from(case.backend_present)),
                        ("path_was_live", i128::from(path_live)),
                        ("path_released", i128::from(case.path_released)),
                        ("backend_released", i128::from(case.backend_released)),
                        ("cleanup_results_ignored", 1),
                        (
                            "mapping_failure",
                            i128::from(mapping == ResidualCustody::Unresolved),
                        ),
                        (
                            "descriptor_failure",
                            i128::from(descriptor == ResidualCustody::Unresolved),
                        ),
                        (
                            "mapping_unresolved",
                            i128::from(mapping == ResidualCustody::Unresolved),
                        ),
                        (
                            "descriptor_unresolved",
                            i128::from(descriptor == ResidualCustody::Unresolved),
                        ),
                        (
                            "nested_discharge_permitted",
                            i128::from(
                                mapping == ResidualCustody::Absent
                                    && descriptor == ResidualCustody::Absent,
                            ),
                        ),
                        ("chunk_root_preserved", 1),
                    ],
                )?;
                selected.push(case);
            }
        }
        if selected.len() != 1 {
            return Err("file-close no-delete case must select exactly one source path".into());
        }
        let case = selected.remove(0);
        covered.insert(case.source_path);
        cases.push(case);
    }
    Ok(FileCloseEvidence {source:source.clone(),unmap,close,contract_source:contracts.contract.into(),cases,uncovered_paths:(0..source.paths().len()).filter(|i|!covered.contains(i)).collect(),status:"checked non-deleting release chain; OS errors retain Unknown custody; deletion paths uncovered".into()})
}
#[derive(Clone, Debug, PartialEq, Eq)]
enum Value {
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
struct Replay {
    backend: bool,
    path_live: bool,
    phase: usize,
    deleting: bool,
    delete_result: Option<i32>,
    delete_called: bool,
    logged: bool,
    symbols: BTreeMap<usize, Value>,
}
impl Replay {
    fn value(&self, v: &SourceValue) -> Result<Value, String> {
        match &v.expression {
            Expr::Parameter { index: 0 } if v.c_type == "struct cio_chunk *" => Ok(Value::Chunk),
            Expr::Parameter { index: 1 } if v.c_type == "int" => {
                Ok(Value::Integer(i32::from(self.deleting)))
            }
            Expr::String { value } => Ok(Value::Text(value.clone())),
            Expr::Integer { value } => Ok(Value::Integer(
                value
                    .parse()
                    .map_err(|_| "file-close integer out of range")?,
            )),
            Expr::Symbol { operation } => self
                .symbols
                .get(operation)
                .filter(|v| **v != Value::IgnoredResult)
                .cloned()
                .ok_or("file-close cleanup result is used or producer missing".into()),
            Expr::Cast { cast, value } => {
                let result = self.value(value)?;
                match cast.as_str() {
                    "NullToPointer"
                        if matches!(result, Value::Null | Value::Integer(0))
                            && v.c_type.ends_with('*') =>
                    {
                        Ok(Value::Null)
                    }
                    "ArrayToPointerDecay"
                        if matches!(result, Value::Text(_)) && v.c_type.ends_with('*') =>
                    {
                        Ok(result)
                    }
                    "BitCast" | "NoOp"
                        if value.c_type.ends_with('*') && v.c_type.ends_with('*') =>
                    {
                        Ok(result)
                    }
                    _ => Err("file-close cast outside pointer identity profile".into()),
                }
            }
            Expr::Unary { operator, value } if operator == "-" => {
                let Value::Integer(n) = self.value(value)? else {
                    return Err("file-close symbolic negation".into());
                };
                Ok(Value::Integer(
                    n.checked_neg().ok_or("file-close integer overflow")?,
                ))
            }
            Expr::Unary { operator, value } if operator == "!" => {
                Ok(Value::Integer(i32::from(!Self::truth(self.value(value)?)?)))
            }
            Expr::Binary {
                operator,
                left,
                right,
            } if operator == "==" || operator == "!=" => {
                let a = self.value(left)?;
                let b = self.value(right)?;
                let eq = match (&a, &b) {
                    (Value::Integer(a), Value::Integer(b)) => a == b,
                    (Value::Null, Value::Null) => true,
                    (Value::Backend | Value::Path, Value::Null)
                    | (Value::Null, Value::Backend | Value::Path) => false,
                    _ => return Err("file-close comparison outside null/zero profile".into()),
                };
                Ok(Value::Integer(i32::from(if operator == "==" {
                    eq
                } else {
                    !eq
                })))
            }
            _ => Err("file-close expression outside bounded profile".into()),
        }
    }
    fn truth(v: Value) -> Result<bool, String> {
        match v {
            Value::Null => Ok(false),
            Value::Backend | Value::Path => Ok(true),
            Value::Integer(n) => Ok(n != 0),
            _ => Err("file-close unresolved truth".into()),
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
    fn run(&mut self, ops: &[Op]) -> Result<bool, String> {
        for (i, op) in ops.iter().enumerate() {
            match op {
                Op::Read { place } => {
                    let Expr::Field { base, name } = &place.expression else {
                        return Err("file-close requires direct field read".into());
                    };
                    let result = match (
                        self.value(base)?,
                        base.c_type.as_str(),
                        name.as_str(),
                        place.c_type.as_str(),
                    ) {
                        (Value::Chunk, "struct cio_chunk *", "backend", "void *")
                            if self.phase == 0 =>
                        {
                            if self.backend {
                                Value::Backend
                            } else {
                                Value::Null
                            }
                        }
                        (Value::Chunk, "struct cio_chunk *", "ctx", "struct cio_ctx *")
                            if self.phase == 0 || self.reporting() =>
                        {
                            Value::Context
                        }
                        (Value::Chunk, "struct cio_chunk *", "st", "struct cio_stream *")
                            if self.reporting() =>
                        {
                            Value::Stream
                        }
                        (Value::Chunk, "struct cio_chunk *", "name", "char *")
                            if self.reporting() =>
                        {
                            Value::ChunkName
                        }
                        (Value::Stream, "struct cio_stream *", "name", "char *")
                            if self.reporting() =>
                        {
                            Value::StreamName
                        }
                        (Value::Backend, "struct cio_file *", "path", "char *")
                            if self.ready_to_free() =>
                        {
                            if self.path_live {
                                Value::Path
                            } else {
                                Value::Null
                            }
                        }
                        _ => {
                            return Err(
                                "file-close read has wrong identity/type or released lifetime"
                                    .into(),
                            );
                        }
                    };
                    self.symbols.insert(i, result);
                }
                Op::LocalWrite { value, .. } => {
                    self.value(value)?;
                }
                Op::LocalStart { .. } | Op::LocalEnd { .. } => (),
                Op::Guard { condition, taken } => {
                    if Self::truth(self.value(condition)?)? != *taken {
                        return Ok(false);
                    }
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
                    if function == "cio_file_native_delete" {
                        if self.phase != 2
                            || !self.deleting
                            || self.delete_called
                            || self.delete_result.is_none()
                            || signature != "int (struct cio_file *)"
                            || result_type != "int"
                            || args != [Value::Backend]
                        {
                            return Err(
                                "file-close deletion dependency identity/order mismatch".into()
                            );
                        }
                        self.delete_called = true;
                        self.symbols
                            .insert(i, Value::Integer(self.delete_result.unwrap()));
                        continue;
                    }
                    if function == "cio_log_print" {
                        if !self.reporting()
                            || signature
                                != "void (void *, int, const char *, int, const char *, ...)"
                            || result_type != "void"
                            || !matches!(args.as_slice(),[Value::Context,Value::Integer(1),Value::Text(_),Value::Integer(line),Value::Text(format),Value::StreamName,Value::ChunkName] if *line>0 && format=="\"[cio file] error deleting file at close %s:%s\"")
                        {
                            return Err(
                                "file-close deletion diagnostic identity/order mismatch".into()
                            );
                        }
                        self.logged = true;
                        self.symbols.insert(i, Value::IgnoredResult);
                        continue;
                    }
                    let valid = match (
                        self.phase,
                        function.as_str(),
                        signature.as_str(),
                        result_type.as_str(),
                    ) {
                        (0, "munmap_file", "int (struct cio_ctx *, struct cio_chunk *)", "int") => {
                            self.backend && args == [Value::Context, Value::Chunk]
                        }
                        (1, "cio_file_native_close", "int (struct cio_file *)", "int") => {
                            args == [Value::Backend]
                        }
                        (2, "free", "void (void *)", "void") => {
                            self.ready_to_free()
                                && args
                                    == [if self.path_live {
                                        Value::Path
                                    } else {
                                        Value::Null
                                    }]
                        }
                        (3, "free", "void (void *)", "void") => args == [Value::Backend],
                        _ => false,
                    };
                    if !valid {
                        return Err(format!(
                            "file-close call identity/order mismatch: {function}"
                        ));
                    }
                    self.phase += 1;
                    self.symbols.insert(i, Value::IgnoredResult);
                }
                Op::Return if i + 1 == ops.len() => return Ok(true),
                _ => return Err("file-close unsupported operation on no-delete path".into()),
            }
        }
        Err("file-close lacks terminal return".into())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum DeletionOutcome {
    NoBackend,
    BlockedByFields,
    SyscallFailed,
    SyscallSucceeded,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DeletingCloseCase {
    pub source_path: usize,
    pub unmap_case: Option<usize>,
    pub close_case: Option<usize>,
    pub delete_case: Option<usize>,
    pub mapping: ResidualCustody,
    pub descriptor: ResidualCustody,
    pub deletion: DeletionOutcome,
    pub backend_released: bool,
    pub path_released: bool,
    pub diagnostic_calls: usize,
    pub up_counter_decrements: usize,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FileCloseDeletionEvidence {
    no_delete: FileCloseEvidence,
    native_delete: GuardedDeleteCertificate,
    deleting_cases: Vec<DeletingCloseCase>,
    contract_source: String,
    uncovered_paths: Vec<usize>,
}
impl FileCloseDeletionEvidence {
    pub fn source(&self) -> &SourceSummary {
        &self.no_delete.source
    }
    /// The non-deleting arm this reading embeds. The deleting entry point
    /// answers a strictly larger question, so it publishes both.
    pub fn no_delete(&self) -> &FileCloseEvidence {
        &self.no_delete
    }
    pub fn native_delete(&self) -> &GuardedDeleteCertificate {
        &self.native_delete
    }
    pub fn non_deleting_cases(&self) -> &[FileCloseCase] {
        &self.no_delete.cases
    }
    pub fn recheck(
        &self,
        contracts: &FileCloseContracts<'_>,
        delete: &FileCloseDeleteContracts<'_>,
    ) -> Result<Self, String> {
        let no_delete = check_file_close_composition(
            &self.no_delete.source,
            &self.no_delete.unmap,
            &self.no_delete.close,
            contracts,
        )?;
        check_file_close_deletion(no_delete, &self.native_delete, contracts, delete)
    }
    pub fn deleting_cases(&self) -> &[DeletingCloseCase] {
        &self.deleting_cases
    }
    pub fn uncovered_paths(&self) -> &[usize] {
        &self.uncovered_paths
    }
    /// Conditional outcome gate. A null backend makes no deletion attempt;
    /// freeing C path storage is never accepted as successful backing deletion.
    pub fn require_successful_deletion(&self, case: usize) -> Result<(), String> {
        let c = self
            .deleting_cases
            .get(case)
            .ok_or("deleting-close case out of range")?;
        if c.deletion != DeletionOutcome::SyscallSucceeded
            || c.mapping != ResidualCustody::Absent
            || c.descriptor != ResidualCustody::Absent
        {
            return Err("Unknown: successful deletion is not established".into());
        }
        Ok(())
    }
}

/// Compose the deleting arm on top of an already-checked non-deleting reading.
pub fn check_file_close_deletion(
    no_delete: FileCloseEvidence,
    delete: &GuardedDeleteCertificate,
    contracts: &FileCloseContracts<'_>,
    delete_contracts: &FileCloseDeleteContracts<'_>,
) -> Result<FileCloseDeletionEvidence, String> {
    if delete.source().source_revision() != contracts.pinned_revision
        || delete.source().function() != delete_contracts.delete_function
    {
        return Err("native delete dependency revision/function mismatch".into());
    }
    let delete = check_guarded_delete_outcomes(
        delete.source(),
        delete_contracts.delete_profile,
        delete_contracts.delete_mapping_field,
        delete_contracts.delete_contract,
    )?;
    if delete.source().source_revision() != no_delete.source.source_revision() {
        return Err("deleting-close source revision mismatch".into());
    }
    let contract = delete_contracts.contract;
    let mut entries = vec![(None, None, None)];
    for (u, uc) in no_delete
        .unmap
        .cases()
        .iter()
        .enumerate()
        .filter(|(_, c)| c.backend_present)
    {
        for (c, cc) in no_delete
            .close
            .cases()
            .iter()
            .enumerate()
            .filter(|(_, c)| c.entry != CleanupEntry::Null)
        {
            // These are the source-proved field states after cleanup, not claims
            // that a syscall failure necessarily retained actual OS resources.
            let mapped = uc.native_result == Some(-1);
            let open = cc.syscall_result == Some(-1);
            for (d, _) in delete.cases().iter().enumerate().filter(|(_, d)| {
                d.mapping_field_active == mapped && d.descriptor_field_active == open
            }) {
                entries.push((Some(u), Some(c), Some(d)));
            }
        }
    }
    let mut cases = Vec::new();
    let mut covered: BTreeSet<_> = no_delete.cases.iter().map(|c| c.source_path).collect();
    for (u, c, d) in entries {
        let outcome = d.map(|i| delete.cases()[i].returned);
        let mut selected = Vec::new();
        for (index, path) in no_delete.source.paths().iter().enumerate() {
            let mut replay = Replay {
                backend: u.is_some(),
                path_live: u.is_some(),
                phase: 0,
                deleting: true,
                delete_result: outcome,
                delete_called: false,
                logged: false,
                symbols: BTreeMap::new(),
            };
            if replay.run(&path.operations)? {
                if replay.phase != if u.is_some() { 4 } else { 0 }
                    || replay.delete_called != d.is_some()
                    || replay.logged != (outcome == Some(-1))
                {
                    return Err("incomplete deleting-close release/diagnostic sequence".into());
                }
                let mapping =
                    if u.is_some_and(|i| no_delete.unmap.cases()[i].native_result == Some(-1)) {
                        ResidualCustody::Unresolved
                    } else {
                        ResidualCustody::Absent
                    };
                let descriptor =
                    if c.is_some_and(|i| no_delete.close.cases()[i].syscall_result == Some(-1)) {
                        ResidualCustody::Unresolved
                    } else {
                        ResidualCustody::Absent
                    };
                let deletion = match d.map(|i| delete.cases()[i].syscall_result) {
                    None => DeletionOutcome::NoBackend,
                    Some(None) => DeletionOutcome::BlockedByFields,
                    Some(Some(-1)) => DeletionOutcome::SyscallFailed,
                    Some(Some(0)) => DeletionOutcome::SyscallSucceeded,
                    _ => return Err("unsupported delete syscall result".into()),
                };
                check_contract_facts(
                    contract,
                    "file_close_delete",
                    &[
                        ("inherited_no_delete_custody_frame_and_abi_premises", 1),
                        ("delete_argument_is_one", 1),
                        ("nonnull_backend_has_live_owned_terminated_path", 1),
                        (
                            "native_delete_and_logging_return_and_preserve_all_record_identities",
                            1,
                        ),
                        ("successful_unlink_removes_the_bound_backing_path", 1),
                        ("was_nonnull", i128::from(u.is_some())),
                        ("delete_calls", i128::from(replay.delete_called)),
                        ("delete_failed", i128::from(outcome == Some(-1))),
                        ("diagnostic_calls", i128::from(replay.logged)),
                        ("path_released", i128::from(replay.phase == 4)),
                        ("backend_released", i128::from(replay.phase == 4)),
                        ("cleanup_results_ignored", 1),
                        (
                            "deletion_succeeded",
                            i128::from(deletion == DeletionOutcome::SyscallSucceeded),
                        ),
                        (
                            "mapping_unresolved",
                            i128::from(mapping == ResidualCustody::Unresolved),
                        ),
                        (
                            "descriptor_unresolved",
                            i128::from(descriptor == ResidualCustody::Unresolved),
                        ),
                        ("chunk_root_preserved", 1),
                    ],
                )?;
                selected.push(DeletingCloseCase {
                    source_path: index,
                    unmap_case: u,
                    close_case: c,
                    delete_case: d,
                    mapping,
                    descriptor,
                    deletion,
                    backend_released: u.is_some(),
                    path_released: u.is_some(),
                    diagnostic_calls: usize::from(replay.logged),
                    up_counter_decrements: u
                        .map_or(0, |i| no_delete.unmap.cases()[i].counter_decrements),
                });
            }
        }
        if selected.len() != 1 {
            return Err("deleting-close case must select exactly one source path".into());
        }
        let case = selected.remove(0);
        covered.insert(case.source_path);
        cases.push(case);
    }
    let uncovered_paths: Vec<_> = (0..no_delete.source.paths().len())
        .filter(|i| !covered.contains(i))
        .collect();
    if !uncovered_paths.is_empty() {
        return Err("file-close Boolean-delete profile has uncovered paths".into());
    }
    Ok(FileCloseDeletionEvidence {
        no_delete,
        native_delete: delete,
        deleting_cases: cases,
        contract_source: contract.into(),
        uncovered_paths,
    })
}
