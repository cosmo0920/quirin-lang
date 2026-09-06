//! Non-deleting cio_file_close source composition. Releasing the backend's C
//! allocation does not erase unresolved mapping/descriptor cleanup obligations.
use super::{
    PINNED_FLUENT_BIT_REVISION, file_attempts::check_file_attempt_sources, file_outcomes,
    unmap_wrapper::UnmapCertificate,
};
use crate::cleanup_outcome::{CleanupCertificate, CleanupEntry, check_cleanup_outcomes};
use crate::legacy_c::C11TranslationUnit;
use crate::typed_source::{
    SourceExpression as Expr, SourceOperation as Op, SourceSummary, SourceValue,
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

pub const CONTRACT: &str = include_str!("../../../../contract/qcore/chunkio/file_close.qcore");
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
pub fn check_file_close_source(
    unit: &C11TranslationUnit<'_>,
    revision: &str,
    unmap: &UnmapCertificate,
    close: &CleanupCertificate,
) -> Result<FileCloseEvidence, String> {
    if revision != PINNED_FLUENT_BIT_REVISION {
        return Err("file-close revision mismatch".into());
    }
    let evidence = check_file_attempt_sources(unit, revision, false)?;
    if evidence
        .iter()
        .find(|e| e.source().function() == "munmap_file")
        .map(|e| e.source())
        != Some(unmap.source())
    {
        return Err("file-close same-unit unmap dependency source mismatch".into());
    }
    let source = evidence
        .iter()
        .find(|e| e.source().function() == "cio_file_close")
        .ok_or("missing file-close source")?
        .source();
    check(source, unmap, close, CONTRACT)
}
fn check(
    source: &SourceSummary,
    unmap: &UnmapCertificate,
    close: &CleanupCertificate,
    contract: &str,
) -> Result<FileCloseEvidence, String> {
    if source.function() != "cio_file_close"
        || source.signature() != "void (struct cio_chunk *, int)"
        || source.source_revision() != PINNED_FLUENT_BIT_REVISION
        || unmap.source().source_revision() != source.source_revision()
        || close.source().source_revision() != source.source_revision()
        || close.source().function() != "cio_file_native_close"
    {
        return Err("file-close source/dependency signature/revision mismatch".into());
    }
    let unmap = unmap.recheck()?;
    let close = check_cleanup_outcomes(
        close.source(),
        &file_outcomes::profile("cio_file_native_close")?,
        file_outcomes::CONTRACT,
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
                crate::contract_observation::check_contract_facts(
                    contract,
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
    Ok(FileCloseEvidence {source:source.clone(),unmap,close,contract_source:contract.into(),cases,uncovered_paths:(0..source.paths().len()).filter(|i|!covered.contains(i)).collect(),status:"checked non-deleting release chain; OS errors retain Unknown custody; deletion paths uncovered".into()})
}
#[derive(Clone, Debug, PartialEq, Eq)]
enum Value {
    Null,
    Chunk,
    Context,
    Backend,
    Path,
    Integer(i32),
    IgnoredResult,
}
struct Replay {
    backend: bool,
    path_live: bool,
    phase: usize,
    symbols: BTreeMap<usize, Value>,
}
impl Replay {
    fn value(&self, v: &SourceValue) -> Result<Value, String> {
        match &v.expression {
            Expr::Parameter { index: 0 } if v.c_type == "struct cio_chunk *" => Ok(Value::Chunk),
            Expr::Parameter { index: 1 } if v.c_type == "int" => Ok(Value::Integer(0)),
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
                    "BitCast" | "NoOp"
                        if value.c_type.ends_with('*') && v.c_type.ends_with('*') =>
                    {
                        Ok(result)
                    }
                    _ => Err("file-close cast outside pointer identity profile".into()),
                }
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
                            if self.phase == 0 =>
                        {
                            Value::Context
                        }
                        (Value::Backend, "struct cio_file *", "path", "char *")
                            if self.phase == 2 =>
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
                            args == [if self.path_live {
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
#[cfg(test)]
#[path = "../../../tests/unit/file_close.rs"]
mod tests;
