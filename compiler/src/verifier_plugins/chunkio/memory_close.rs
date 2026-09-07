//! Bounded source composition for null and memory-backed cio_chunk_close.
//! Filesystem/other backend paths remain explicit uncovered paths. Dependency
//! contracts are rechecked against their private source summaries before use.
use super::{
    ChunkFrameSummary, PINNED_FLUENT_BIT_REVISION,
    teardown_source::{COUNTER_CONTRACT, LIST_UNLINK_CONTRACT, MEMORY_BACKEND_CONTRACT},
};
use crate::counter_contract::CounterCertificate;
use crate::field_destructor::FieldDestructorCertificate;
use crate::frontend::{DeclarationKind, SourceContractClauseKind, parse_source_program};
use crate::legacy_c::C11TranslationUnit;
use crate::lexer::{Symbol, TokenKind, lex};
use crate::list_unlink_contract::ListUnlinkCertificate;
use crate::typed_source::{
    SourceExpression as Expr, SourceImportProfile, SourceOperation as Op, SourceResult,
    SourceSummary, SourceValue, import_source_functions,
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

pub const CONTRACT: &str = include_str!("../../../../contract/qcore/chunkio/memory_close.qcore");
const PROBE: &str = "\nint qcore_cio_memory_store_value(void) { return CIO_STORE_MEM; }\n";
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct MemoryCloseDependencies {
    pub reset: ChunkFrameSummary,
    pub backend: FieldDestructorCertificate,
    pub unlink: ListUnlinkCertificate,
    pub counter: CounterCertificate,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct MemoryCloseCertificate {
    source: SourceSummary,
    constant_probe: SourceSummary,
    probe_source: String,
    contract_source: String,
    dependencies: MemoryCloseDependencies,
    checked_paths: Vec<usize>,
    uncovered_paths: Vec<usize>,
}
impl MemoryCloseCertificate {
    pub(super) fn source(&self) -> &SourceSummary {
        &self.source
    }
    pub(super) fn memory_kind(&self) -> Result<i32, String> {
        probe_literal(&self.constant_probe)
    }
    pub(super) fn recheck(&self) -> Result<Self, String> {
        check(
            &self.source,
            &self.constant_probe,
            &self.dependencies,
            CONTRACT,
        )
    }
    pub fn checked_paths(&self) -> &[usize] {
        &self.checked_paths
    }
    pub fn uncovered_paths(&self) -> &[usize] {
        &self.uncovered_paths
    }
    /// Bind the checked source parameter to a concrete live model identity.
    /// Caller establishes C pointer provenance, exclusive access and source/ABI
    /// equality. A null argument yields no operation. This is not terminal
    /// context/stream teardown; the returned operation still uses the core checker.
    pub fn instantiate(
        &self,
        state: &crate::chunk_lifecycle::ChunkState,
        chunk: Option<u32>,
        delete: bool,
    ) -> Result<Vec<crate::chunk_lifecycle::ChunkOperation>, String> {
        use crate::chunk_lifecycle::{ChunkBackend, ChunkOperation};
        let Some(chunk) = chunk else {
            return Ok(vec![]);
        };
        let record = state
            .chunks
            .get(&chunk)
            .filter(|r| r.live && r.backend == ChunkBackend::Memory)
            .ok_or("memory close requires a live memory-backed chunk")?;
        if record.file_mapping_live
            || record.file_descriptor_live
            || record.file_path_live
            || (!record.backend_object_live
                && (record.memory_buffer_live || record.metadata_buffer_live))
            || record.locked
            || record.busy
            || record.task.is_some()
            || !record.routes.is_empty()
            || record.transaction_active
            || !record.primary_linked
            || !record.residency_linked
        {
            return Err("memory close has outstanding wrapper or missing link obligations".into());
        }
        let context = state
            .contexts
            .get(&record.context)
            .filter(|r| r.live)
            .ok_or("memory close context is not live")?;
        let stream = state
            .streams
            .get(&record.stream)
            .filter(|r| r.live && r.context == record.context && r.chunks.contains(&chunk))
            .ok_or("memory close stream identity/membership mismatch")?;
        let _ = stream;
        if !context.streams.contains(&record.stream) {
            return Err("memory close context lacks stream membership".into());
        }
        self.dependencies
            .counter
            .instantiate(context.chunk_total as u128, usize::MAX as u128)?;
        Ok(vec![ChunkOperation::CloseChunk { chunk, delete }])
    }
}

pub fn check_memory_close_source(
    unit: &C11TranslationUnit<'_>,
    revision: &str,
    dependencies: &MemoryCloseDependencies,
) -> Result<MemoryCloseCertificate, String> {
    if revision != PINNED_FLUENT_BIT_REVISION {
        return Err("memory close revision mismatch".into());
    }
    let combined = format!("{}{PROBE}", unit.source);
    let probe_unit = C11TranslationUnit {
        source: &combined,
        include_paths: unit.include_paths.clone(),
        definitions: unit.definitions.clone(),
        preprocessor_options: unit.preprocessor_options.clone(),
        dialect: unit.dialect,
        function_scope: unit.function_scope,
        // The probe text is not the declared file, so it keeps no file identity.
        input: unit.input.for_derived_text(),
    };
    let summaries = import_source_functions(
        &probe_unit,
        revision,
        &["cio_chunk_close", "qcore_cio_memory_store_value"],
        &SourceImportProfile {
            records: &["cio_chunk", "cio_ctx", "cio_stream"],
            sizeof_types: &[],
            signature_constraints: &[],
            restricted_calls: &[],
        },
    )?;
    let [SourceResult::Summary(source), SourceResult::Summary(probe)] = summaries.as_slice() else {
        return Err(format!(
            "memory close source/probe incomplete: {summaries:?}"
        ));
    };
    check(source, probe, dependencies, CONTRACT)
}
fn check(
    source: &SourceSummary,
    probe: &SourceSummary,
    deps: &MemoryCloseDependencies,
    contract: &str,
) -> Result<MemoryCloseCertificate, String> {
    if source.signature() != "void (struct cio_chunk *, int)"
        || probe.signature() != "int (void)"
        || source.source_revision() != PINNED_FLUENT_BIT_REVISION
        || probe.source_revision() != source.source_revision()
    {
        return Err("memory close source signature/revision mismatch".into());
    }
    let [probe_path] = probe.paths() else {
        return Err("memory kind probe has multiple paths".into());
    };
    let [
        Op::ReturnValue {
            value:
                SourceValue {
                    expression: Expr::Integer { value },
                    ..
                },
        },
    ] = probe_path.operations.as_slice()
    else {
        return Err("memory kind probe must return an int literal".into());
    };
    let memory_kind = value
        .parse::<i32>()
        .map_err(|_| "memory kind outside int profile")?;
    if deps.backend.source().source_revision() != source.source_revision()
        || deps.unlink.source().source_revision() != source.source_revision()
        || deps.counter.source().source_revision() != source.source_revision()
    {
        return Err("memory close dependency revision mismatch".into());
    }
    crate::chunk_contract::check_chunk_frame_contract(
        &deps.reset,
        include_str!("../../../../contract/qcore/chunkio/error_state.qcore"),
    )?;
    crate::field_destructor::check_field_destructor(
        deps.backend.source(),
        MEMORY_BACKEND_CONTRACT,
        "struct cio_chunk *",
        "Chunk",
    )?;
    crate::list_unlink_contract::check_list_unlink(
        deps.unlink.source(),
        deps.unlink.helper_source(),
        LIST_UNLINK_CONTRACT,
        "struct mk_list *",
        "ListNode",
    )?;
    crate::counter_contract::check_counter_contract(
        deps.counter.source(),
        COUNTER_CONTRACT,
        "struct cio_ctx *",
        "ChunkContext",
    )?;
    if deps.counter.field() != "total_chunks" || deps.counter.delta() != -1 {
        return Err("close requires the total-chunk decrement dependency".into());
    }
    let mut checked_paths = BTreeSet::new();
    for nonnull in [false, true] {
        let mut accepted = Vec::new();
        for (index, path) in source.paths().iter().enumerate() {
            let mut replay = Replay {
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
                deps,
            };
            if replay.run(&path.operations)? {
                check_contract(contract, source.function(), &replay)?;
                accepted.push(index);
            }
        }
        let [index] = accepted.as_slice() else {
            return Err("each null/memory entry must select exactly one complete path".into());
        };
        checked_paths.insert(*index);
    }
    Ok(MemoryCloseCertificate {
        source: source.clone(),
        constant_probe: probe.clone(),
        probe_source: PROBE.into(),
        contract_source: contract.into(),
        dependencies: deps.clone(),
        uncovered_paths: (0..source.paths().len())
            .filter(|i| !checked_paths.contains(i))
            .collect(),
        checked_paths: checked_paths.into_iter().collect(),
    })
}
fn tokens(source: &str) -> Result<Vec<TokenKind>, String> {
    Ok(lex(source)
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|t| t.kind)
        .filter(|t| *t != TokenKind::Eof)
        .collect())
}
fn check_contract(source: &str, function: &str, state: &Replay<'_>) -> Result<(), String> {
    let parsed = parse_source_program(source).map_err(|e| e.to_string())?;
    let declarations: Vec<_> = parsed
        .declarations
        .iter()
        .filter(|d| d.value.name == function)
        .collect();
    let [decl] = declarations.as_slice() else {
        return Err("expected one memory close contract".into());
    };
    let decl = &decl.value;
    if decl.kind != DeclarationKind::ExternFunction
        || decl.parameters.len() != 2
        || tokens(&decl.parameters[0].type_source)? != tokens("ptr<mut, Chunk>")?
        || tokens(&decl.parameters[1].type_source)? != tokens("i32")?
        || decl.parameters.iter().any(|p| p.usage.is_some())
    {
        return Err(
            "memory close contract requires nullable raw Chunk pointer and int delete flag".into(),
        );
    }
    let arrow = decl
        .tokens
        .iter()
        .position(|t| *t == TokenKind::Symbol(Symbol::Arrow))
        .ok_or("missing close result")?;
    if decl.tokens.get(arrow + 1) != Some(&TokenKind::Word("unit".into()))
        || decl.tokens.get(arrow + 2) != Some(&TokenKind::Word("contract".into()))
    {
        return Err("memory close contract requires unit without extra effects".into());
    }
    let mut premises = BTreeSet::new();
    for clause in &decl
        .function_contract
        .as_ref()
        .ok_or("missing close clauses")?
        .clauses
    {
        if clause.value.kind == SourceContractClauseKind::Requires {
            let ts = tokens(&clause.value.expression_source)?;
            let [TokenKind::Word(name)] = ts.as_slice() else {
                return Err("unsupported close premise".into());
            };
            if !premises.insert(name.clone()) {
                return Err("duplicate close premise".into());
            }
        }
    }
    let required: BTreeSet<_> = [
        "nullable_owned_chunk",
        "nonnull_chunk_has_memory_backend",
        "backend_and_name_have_disjoint_owned_provenance",
        "both_links_are_live_reciprocal_memberships",
        "embedded_links_have_distinct_stream_memberships",
        "context_and_stream_outlive_chunk",
        "context_counter_covers_live_chunk",
        "no_lock_busy_task_route_or_transaction_obligations",
        "exclusive_access",
        "cross_unit_source_and_abi_bindings",
        "free_resolves_to_libc",
    ]
    .into_iter()
    .map(str::to_string)
    .collect();
    if premises != required {
        return Err("memory close lacks exact custody/domain/dependency premises".into());
    }
    let mut facts: Vec<_> = premises.iter().map(|p| (p.as_str(), 1)).collect();
    facts.extend([
        ("chunk_was_nonnull", i128::from(state.nonnull)),
        ("backend_discharged", i128::from(state.backend)),
        ("primary_unlinked", i128::from(state.primary)),
        ("state_unlinked", i128::from(state.state_link)),
        ("name_released", i128::from(state.name)),
        ("chunk_released", i128::from(state.freed)),
        (
            "saved_context_counter_decremented",
            i128::from(state.counter),
        ),
        ("call_count", state.calls as i128),
    ]);
    crate::contract_observation::check_contract_facts(source, function, &facts)
}
#[derive(Clone, Debug, PartialEq, Eq)]
enum Value {
    Null,
    Chunk,
    Context,
    Stream,
    Name,
    PrimaryLink,
    StateLink,
    Integer(i32),
    /// Any int except the two source-derived backend constants. Comparing this
    /// class with any other integer must reject, rather than sample one value.
    OtherBackend(i32, i32),
    Boolean(bool),
}
struct Replay<'a> {
    nonnull: bool,
    memory_kind: i32,
    filesystem: bool,
    other_kind: Option<(i32, i32)>,
    delete: bool,
    freed: bool,
    backend: bool,
    primary: bool,
    state_link: bool,
    name: bool,
    counter: bool,
    calls: usize,
    symbols: BTreeMap<usize, Value>,
    deps: &'a MemoryCloseDependencies,
}
impl Replay<'_> {
    fn truth(value: Value) -> Result<bool, String> {
        match value {
            Value::OtherBackend(memory, file) if memory != 0 && file != 0 => {
                Err("truth test splits the symbolic unsupported backend domain".into())
            }
            Value::Null | Value::Boolean(false) | Value::Integer(0) => Ok(false),
            _ => Ok(true),
        }
    }
    fn value(&self, value: &SourceValue) -> Result<Value, String> {
        match &value.expression {
            Expr::Parameter { index: 0 } => Ok(if self.nonnull {
                Value::Chunk
            } else {
                Value::Null
            }),
            Expr::Parameter { index: 1 } if self.filesystem || self.other_kind.is_some() => {
                Ok(Value::Integer(i32::from(self.delete)))
            }
            Expr::Symbol { operation } => self
                .symbols
                .get(operation)
                .cloned()
                .ok_or("unbound close producer".into()),
            Expr::Integer { value } => value
                .parse()
                .map(Value::Integer)
                .map_err(|_| "close integer out of range".into()),
            Expr::Cast { cast, value } if matches!(cast.as_str(), "BitCast" | "NoOp") => {
                self.value(value)
            }
            Expr::Unary { operator, value } if operator == "!" => {
                Ok(Value::Boolean(!Self::truth(self.value(value)?)?))
            }
            Expr::Binary {
                operator,
                left,
                right,
            } if operator == "==" => {
                // C logical expressions have integer results. Preserve that
                // equality when a guard compares !pointer with literal zero.
                let integer_bool = |value| match value {
                    Value::Boolean(value) => Value::Integer(i32::from(value)),
                    other => other,
                };
                let left = integer_bool(self.value(left)?);
                let right = integer_bool(self.value(right)?);
                if let (Value::OtherBackend(memory, file), Value::Integer(value))
                | (Value::Integer(value), Value::OtherBackend(memory, file)) = (&left, &right)
                {
                    return if value == memory || value == file {
                        Ok(Value::Boolean(false))
                    } else {
                        Err("backend comparison splits the symbolic unsupported domain".into())
                    };
                }
                if matches!(left, Value::OtherBackend(..))
                    || matches!(right, Value::OtherBackend(..))
                {
                    return Err("unmodeled symbolic backend comparison".into());
                }
                if matches!(&left, Value::Integer(_)) != matches!(&right, Value::Integer(_)) {
                    return Err("mixed pointer/integer close comparison".into());
                }
                Ok(Value::Boolean(left == right))
            }
            Expr::Unary { operator, value } if operator == "&" => {
                let Expr::Field { base, name } = &value.expression else {
                    return Err("close address is not an embedded link".into());
                };
                if self.value(base)? != Value::Chunk
                    || self.freed
                    || value.c_type != "struct mk_list"
                {
                    return Err("close link lacks live chunk identity".into());
                }
                match name.as_str() {
                    "_head" => Ok(Value::PrimaryLink),
                    "_state_head" => Ok(Value::StateLink),
                    _ => Err("unknown embedded close link".into()),
                }
            }
            _ => Err("unmodeled value in memory close path".into()),
        }
    }
    fn run(&mut self, ops: &[Op]) -> Result<bool, String> {
        for (index, op) in ops.iter().enumerate() {
            match op {
                Op::Guard { condition, taken } => {
                    let value = self.value(condition)?;
                    let truth = Self::truth(value)?;
                    if truth != *taken {
                        return Ok(false);
                    }
                }
                Op::Read { place } => {
                    let Expr::Field { base, name } = &place.expression else {
                        return Err("nonfield close read".into());
                    };
                    let v = match (self.value(base)?, name.as_str()) {
                        (Value::Chunk, _) if self.freed => {
                            return Err("close reads freed chunk storage".into());
                        }
                        (Value::Chunk, "ctx") if place.c_type == "struct cio_ctx *" => {
                            Value::Context
                        }
                        (Value::Chunk, "st") if place.c_type == "struct cio_stream *" => {
                            Value::Stream
                        }
                        (Value::Chunk, "name") if !self.name && place.c_type == "char *" => {
                            Value::Name
                        }
                        (Value::Stream, "type") if place.c_type == "int" => self
                            .other_kind
                            .map_or(Value::Integer(self.memory_kind), |(m, f)| {
                                Value::OtherBackend(m, f)
                            }),
                        _ => return Err("close read lacks modeled provenance".into()),
                    };
                    self.symbols.insert(index, v);
                }
                Op::LocalWrite { value, .. } => {
                    self.value(value)?;
                }
                Op::LocalEnd { .. } => (),
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
                    self.calls += 1;
                    if function == self.deps.reset.function()
                        && signature == "void (struct cio_chunk *)"
                        && result_type == "void"
                        && args == [Value::Chunk]
                        && !self.freed
                        && self.calls == 1
                    {
                    } else if !self.freed
                        && !self.backend
                        && self.other_kind.is_none()
                        && if self.filesystem {
                            function == "cio_file_close"
                                && signature == "void (struct cio_chunk *, int)"
                                && result_type == "void"
                                && args == [Value::Chunk, Value::Integer(i32::from(self.delete))]
                                && self.calls == 2
                        } else {
                            function == self.deps.backend.source().function()
                                && signature == self.deps.backend.source().signature()
                                && args == [Value::Chunk]
                        }
                    {
                        self.backend = true;
                    } else if function == self.deps.unlink.source().function()
                        && signature == self.deps.unlink.source().signature()
                        && !self.freed
                        && (self.backend || self.other_kind.is_some())
                    {
                        match args.as_slice() {
                            [Value::PrimaryLink] if !self.primary => self.primary = true,
                            [Value::StateLink] if !self.state_link => self.state_link = true,
                            _ => return Err("duplicate/wrong embedded unlink identity".into()),
                        }
                    } else if function == "free"
                        && signature == "void (void *)"
                        && result_type == "void"
                        && (self.backend || self.other_kind.is_some())
                        && self.primary
                        && self.state_link
                    {
                        match args.as_slice() {
                            [Value::Name] if !self.name && !self.freed => self.name = true,
                            [Value::Chunk] if self.name && !self.freed => self.freed = true,
                            _ => return Err("duplicate/wrong close free identity".into()),
                        }
                    } else if function == self.deps.counter.source().function()
                        && result_type
                            == self
                                .deps
                                .counter
                                .source()
                                .signature()
                                .split('(')
                                .next()
                                .unwrap()
                                .trim()
                        && signature.split_once('(').map(|(_, args)| args)
                            == Some("struct cio_ctx *)")
                        && args == [Value::Context]
                        && self.freed
                        && !self.counter
                    {
                        self.counter = true;
                    } else {
                        return Err(format!(
                            "unbound or out-of-order memory close call {function}"
                        ));
                    }
                }
                Op::Return if index + 1 == ops.len() => return Ok(true),
                _ => return Err("unsupported operation in memory close path".into()),
            }
        }
        Err("memory close lacks complete return".into())
    }
}

pub(super) fn probe_literal(probe: &SourceSummary) -> Result<i32, String> {
    if probe.signature() != "int (void)" {
        return Err("backend kind probe requires int(void)".into());
    }
    let [path] = probe.paths() else {
        return Err("backend kind probe requires one path".into());
    };
    let [
        Op::ReturnValue {
            value:
                SourceValue {
                    expression: Expr::Integer { value },
                    ..
                },
        },
    ] = path.operations.as_slice()
    else {
        return Err("backend kind probe must return a literal".into());
    };
    value
        .parse()
        .map_err(|_| "backend kind outside int profile".into())
}

/// Reuse the close identity/order interpreter with the filesystem dispatch arm.
/// The caller must bind and recheck the file dependency before using this trace.
pub(super) fn filesystem_path(
    memory: &MemoryCloseCertificate,
    kind: i32,
    nonnull: bool,
    delete: bool,
) -> Result<usize, String> {
    let mut selected = Vec::new();
    for (index, path) in memory.source.paths().iter().enumerate() {
        let mut replay = Replay {
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
            deps: &memory.dependencies,
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
            .any(|v| *v != nonnull)
                || replay.calls != if nonnull { 7 } else { 0 }
            {
                return Err("filesystem chunk close lacks the complete release chain".into());
            }
            selected.push(index);
        }
    }
    let [index] = selected.as_slice() else {
        return Err("filesystem chunk entry must select exactly one complete source path".into());
    };
    Ok(*index)
}

/// The symbolic class excludes exactly the source-probed memory/file values.
/// No representative integer stands in for arbitrary unmodeled discriminants.
pub(super) fn unsupported_path(
    memory: &MemoryCloseCertificate,
    filesystem_kind: i32,
    delete: bool,
) -> Result<usize, String> {
    let mut selected = Vec::new();
    for (index, path) in memory.source.paths().iter().enumerate() {
        let mut replay = Replay {
            nonnull: true,
            memory_kind: memory.memory_kind()?,
            filesystem: false,
            other_kind: Some((memory.memory_kind()?, filesystem_kind)),
            delete,
            freed: false,
            backend: false,
            primary: false,
            state_link: false,
            name: false,
            counter: false,
            calls: 0,
            symbols: BTreeMap::new(),
            deps: &memory.dependencies,
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
                return Err(
                    "unsupported backend path lacks the exact partial release chain".into(),
                );
            }
            selected.push(index);
        }
    }
    let [index] = selected.as_slice() else {
        return Err("unsupported backend class must select one complete source path".into());
    };
    Ok(*index)
}

#[cfg(test)]
#[path = "../../../tests/unit/memory_close.rs"]
pub(crate) mod tests;
