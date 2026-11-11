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

//! Source derivation for concurrent writes to explicitly laid-out record fields.
//!
//! This route composes syntax the ordinary frontend already owns: `type NAME
//! { ... }` declarations, structured `thread scope` blocks, and the task
//! parser's `write OBJECT.FIELD` captures. It adds no parser bypass and no
//! reserved word. A matching module is total-or-reject: every layout-bearing
//! record declaration and every write capture in the selected scope is read,
//! and unsupported syntax is diagnosed rather than omitted.
//!
//! Layout field syntax is deliberately explicit:
//!
//! ```text
//! type Flags {
//!   left:  bits<0, 0, 3, 0, 4>, // unit, bit offset/width, byte offset/size
//!   right: bits<0, 3, 5, 0, 4>
//! }
//! type Pair { left: field<0, 4>, right: field<4, 4> }
//! ```
//!
//! The declared numbers remain a target-layout premise. The source derivation
//! does not encode bit-fields as byte leases: it constructs
//! [`FieldStorage::BitfieldUnit`], whose bit ranges never enter disjointness.

use std::collections::{BTreeMap, BTreeSet};

use crate::frontend::{
    DeclarationKind, SemanticPass, SourceBlock, SourceCaptureMode, SourceDiagnostic, SourcePlace,
    SourceProgram, SourceTaskExpressionKind, StatementKind,
};
use crate::lexer::{Span, Symbol, Token, TokenKind};
use qcore_core::disjoint_footprint::{
    AllocationId, ElementShape, FieldId, FieldLayout, FieldStorage, FootprintEvent,
    FootprintProgram, Generation, HandleId, LayoutContract, RecordId,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConcurrentFieldWrite {
    pub worker: String,
    pub object: String,
    pub field: String,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoweredBitfieldTaskModule {
    pub program: FootprintProgram,
    pub writes: Vec<ConcurrentFieldWrite>,
    pub scope_span: Span,
}

fn diagnostic(span: Span, message: impl Into<String>) -> SourceDiagnostic {
    SourceDiagnostic {
        code: "Q600",
        pass: SemanticPass::Concurrency,
        span,
        message: message.into(),
    }
}

fn tokens_for(source: &SourceProgram, span: Span) -> Vec<Token> {
    source
        .lexical_tokens()
        .iter()
        .filter(|token| token.span.start >= span.start && token.span.end <= span.end)
        .cloned()
        .collect()
}

fn word(token: Option<&Token>, expected: &str) -> bool {
    matches!(token.map(|token| &token.kind), Some(TokenKind::Word(word)) if word == expected)
}

fn symbol(token: Option<&Token>, expected: Symbol) -> bool {
    matches!(token.map(|token| &token.kind), Some(TokenKind::Symbol(symbol)) if *symbol == expected)
}

fn name(token: Option<&Token>) -> Option<&str> {
    match &token?.kind {
        TokenKind::Word(value) => Some(value),
        _ => None,
    }
}

fn integer(token: Option<&Token>) -> Option<u32> {
    match &token?.kind {
        TokenKind::Integer(value) => value.parse().ok(),
        _ => None,
    }
}

/// Select only the new layout-bearing task shape. A malformed declaration that
/// still spells `bits<` or `field<` stays selected and is refused by this
/// module; it cannot fall through to an older route.
pub fn has_source_bitfield_tasks(source: &SourceProgram) -> bool {
    let layout_record = source.declarations.iter().any(|declaration| {
        declaration.value.kind == DeclarationKind::Record
            && declaration.value.tokens.windows(2).any(|tokens| {
                matches!(&tokens[0], TokenKind::Word(word) if word == "bits" || word == "field")
                    && tokens[1] == TokenKind::Symbol(Symbol::Less)
            })
    });
    let field_write = source.task_expressions().iter().any(|expression| {
        matches!(
            &expression.kind,
            SourceTaskExpressionKind::Spawn { captures, .. }
                if captures.iter().any(|capture| {
                    capture.value.mode == SourceCaptureMode::Write
                        && matches!(capture.value.place.value, SourcePlace::Field { .. })
                })
        )
    });
    layout_record && field_write
}

fn parse_field_layout(
    tokens: &[Token],
    cursor: &mut usize,
    field: FieldId,
) -> Option<(String, FieldLayout)> {
    let field_name = name(tokens.get(*cursor))?.to_owned();
    *cursor += 1;
    if !symbol(tokens.get(*cursor), Symbol::Colon) {
        return None;
    }
    *cursor += 1;
    let storage_name = name(tokens.get(*cursor))?;
    *cursor += 1;
    if !symbol(tokens.get(*cursor), Symbol::Less) {
        return None;
    }
    *cursor += 1;
    let values_needed = if storage_name == "bits" {
        5
    } else if storage_name == "field" {
        2
    } else {
        return None;
    };
    let mut values = Vec::with_capacity(values_needed);
    for index in 0..values_needed {
        values.push(integer(tokens.get(*cursor))?);
        *cursor += 1;
        if index + 1 != values_needed {
            if !symbol(tokens.get(*cursor), Symbol::Comma) {
                return None;
            }
            *cursor += 1;
        }
    }
    if !symbol(tokens.get(*cursor), Symbol::Greater) {
        return None;
    }
    *cursor += 1;
    let (byte_offset, byte_size, storage) = if storage_name == "bits" {
        (
            values[3],
            values[4],
            FieldStorage::BitfieldUnit {
                unit: values[0],
                bit_offset: values[1],
                bit_width: values[2],
            },
        )
    } else {
        (values[0], values[1], FieldStorage::Addressable)
    };
    Some((
        field_name,
        FieldLayout {
            field,
            byte_offset,
            byte_size,
            storage,
        },
    ))
}

#[derive(Clone, Debug)]
struct DeclaredRecord {
    name: String,
    contract: LayoutContract,
    fields: BTreeMap<String, FieldId>,
}

fn statement_is(source: &SourceProgram, span: Span, expected: &[TokenKind]) -> bool {
    tokens_for(source, span)
        .into_iter()
        .map(|token| token.kind)
        .eq(expected.iter().cloned())
}

fn declared_effects(
    source: &SourceProgram,
    declaration: &crate::frontend::Located<crate::frontend::SourceDeclaration>,
) -> Option<BTreeSet<String>> {
    let tokens = tokens_for(source, declaration.span);
    let effects = tokens
        .iter()
        .position(|token| word(Some(token), "effects"))?;
    if !symbol(tokens.get(effects + 1), Symbol::LeftBrace) {
        return None;
    }
    let mut cursor = effects + 2;
    let mut result = BTreeSet::new();
    while !symbol(tokens.get(cursor), Symbol::RightBrace) {
        result.insert(name(tokens.get(cursor))?.to_owned());
        cursor += 1;
        if symbol(tokens.get(cursor), Symbol::Comma) {
            cursor += 1;
        } else if !symbol(tokens.get(cursor), Symbol::RightBrace) {
            return None;
        }
    }
    Some(result)
}

fn validate_worker(source: &SourceProgram, worker_name: &str) -> Result<(), SourceDiagnostic> {
    let Some(worker) = source.declarations.iter().find(|declaration| {
        declaration.value.kind == DeclarationKind::Function && declaration.value.name == worker_name
    }) else {
        return Err(diagnostic(
            Span { start: 0, end: 0 },
            format!("field-writing worker `{worker_name}` is not declared"),
        ));
    };
    if worker.value.parameters.len() != 1
        || worker.value.parameters[0].type_source.trim() != "i32"
        || worker
            .value
            .result_type
            .as_ref()
            .and_then(|result| result.executable_type())
            != Some(crate::frontend::ExecutableResultType::Unit)
    {
        return Err(diagnostic(
            worker.span,
            "a field-writing worker must have exactly one `i32` parameter and return `unit`",
        ));
    }
    if declared_effects(source, worker) != Some(BTreeSet::from(["mut".to_owned()])) {
        return Err(diagnostic(
            worker.span,
            "a field-writing worker must declare exactly `effects { mut }`",
        ));
    }
    let parameter = worker.value.parameters[0].name.value.clone();
    let Some(body) = &worker.value.body else {
        return Err(diagnostic(
            worker.span,
            "field-writing worker requires a body",
        ));
    };
    let expected_write = [
        TokenKind::Word("write".to_owned()),
        TokenKind::Word(parameter),
        TokenKind::Symbol(Symbol::Semicolon),
    ];
    let expected_return = [
        TokenKind::Word("return".to_owned()),
        TokenKind::Symbol(Symbol::Semicolon),
    ];
    if body.statements.len() != 2
        || !statement_is(source, body.statements[0].span, &expected_write)
        || !statement_is(source, body.statements[1].span, &expected_return)
    {
        return Err(diagnostic(
            body.span,
            "a field-writing worker body is exactly `write PARAM; return;`",
        ));
    }
    Ok(())
}

fn parse_record(
    source: &SourceProgram,
    declaration: &crate::frontend::Located<crate::frontend::SourceDeclaration>,
    record: RecordId,
) -> Result<Option<DeclaredRecord>, SourceDiagnostic> {
    let tokens = tokens_for(source, declaration.span);
    let layout_bearing = tokens.iter().any(
        |token| matches!(&token.kind, TokenKind::Word(word) if word == "bits" || word == "field"),
    );
    if !layout_bearing {
        return Ok(None);
    }
    let mut cursor = 0usize;
    if !word(tokens.get(cursor), "type") || name(tokens.get(cursor + 1)).is_none() {
        return Err(diagnostic(
            declaration.span,
            "malformed layout-bearing record head",
        ));
    }
    cursor += 2;
    if !symbol(tokens.get(cursor), Symbol::LeftBrace) {
        return Err(diagnostic(
            declaration.span,
            "layout-bearing record requires `{`",
        ));
    }
    cursor += 1;
    let mut layouts = Vec::new();
    let mut fields = BTreeMap::new();
    while !symbol(tokens.get(cursor), Symbol::RightBrace) {
        let field = layouts.len() as FieldId;
        let Some((field_name, layout)) = parse_field_layout(&tokens, &mut cursor, field) else {
            return Err(diagnostic(
                declaration.span,
                "layout fields must be `FIELD: field<BYTE_OFFSET, BYTE_SIZE>` or `FIELD: bits<UNIT, BIT_OFFSET, BIT_WIDTH, BYTE_OFFSET, BYTE_SIZE>`",
            ));
        };
        if fields.insert(field_name.clone(), field).is_some() {
            return Err(diagnostic(
                declaration.span,
                format!("layout record declares `{field_name}` twice"),
            ));
        }
        layouts.push(layout);
        if symbol(tokens.get(cursor), Symbol::Comma) {
            cursor += 1;
        } else if !symbol(tokens.get(cursor), Symbol::RightBrace) {
            return Err(diagnostic(
                declaration.span,
                "layout fields require a comma",
            ));
        }
    }
    cursor += 1;
    if cursor != tokens.len() || layouts.is_empty() {
        return Err(diagnostic(
            declaration.span,
            "layout-bearing record must contain only one nonempty explicit field list",
        ));
    }
    let size = layouts
        .iter()
        .map(|layout| u64::from(layout.byte_offset) + u64::from(layout.byte_size))
        .max()
        .unwrap_or(0);
    if size == 0 || size > u64::from(u32::MAX) {
        return Err(diagnostic(
            declaration.span,
            "record layout size is not representable",
        ));
    }
    Ok(Some(DeclaredRecord {
        name: declaration.value.name.clone(),
        contract: LayoutContract {
            record,
            size: size as u32,
            fields: layouts,
        },
        fields,
    }))
}

fn let_object(source: &SourceProgram, span: Span) -> Option<(String, String)> {
    let tokens = tokens_for(source, span);
    match tokens.as_slice() {
        [let_, object, equal, type_name, semicolon]
            if word(Some(let_), "let")
                && symbol(Some(equal), Symbol::Equal)
                && symbol(Some(semicolon), Symbol::Semicolon) =>
        {
            Some((
                name(Some(object))?.to_owned(),
                name(Some(type_name))?.to_owned(),
            ))
        }
        _ => None,
    }
}

fn field_place(place: &SourcePlace) -> Option<(&str, &str)> {
    match place {
        SourcePlace::Field { base, field } => match &base.value {
            SourcePlace::Name(object) => Some((&object.value, &field.value)),
            _ => None,
        },
        _ => None,
    }
}

fn collect_scope_writes(
    source: &SourceProgram,
    block: &SourceBlock,
) -> Result<Option<(Span, Vec<ConcurrentFieldWrite>)>, SourceDiagnostic> {
    let mut selected = None;
    for statement in &block.statements {
        if let StatementKind::ThreadScope {
            name: scope_name,
            body,
        } = &statement.value
        {
            let mut writes = Vec::new();
            let expressions = source
                .task_expressions()
                .iter()
                .filter(|expression| {
                    body.span.start <= expression.statement_span.start
                        && expression.statement_span.end <= body.span.end
                })
                .collect::<Vec<_>>();
            if expressions.len() != body.statements.len() {
                return Err(diagnostic(
                    body.span,
                    "every statement in the writing thread scope must be one spawn or join expression",
                ));
            }
            let mut spawned = Vec::<String>::new();
            let mut joined = Vec::<String>::new();
            let mut saw_join = false;
            for expression in expressions {
                let Some(statement) = body
                    .statements
                    .iter()
                    .find(|statement| statement.span == expression.statement_span)
                else {
                    return Err(diagnostic(
                        expression.span,
                        "task expression has no owning statement",
                    ));
                };
                let StatementKind::Let {
                    binding: Some(binding),
                } = &statement.value
                else {
                    return Err(diagnostic(
                        statement.span,
                        "every spawn and join in this bounded route must bind its result",
                    ));
                };
                if !expression.propagates_error {
                    return Err(diagnostic(
                        expression.span,
                        "every spawn and join in this bounded route must propagate failure with `?`",
                    ));
                }
                if let SourceTaskExpressionKind::Spawn {
                    scope,
                    worker,
                    captures,
                } = &expression.kind
                {
                    if saw_join {
                        return Err(diagnostic(
                            expression.span,
                            "all field writers must be spawned before any is joined",
                        ));
                    }
                    if scope.value != *scope_name {
                        return Err(diagnostic(
                            expression.span,
                            "field writer must spawn into its enclosing lexical thread scope",
                        ));
                    }
                    let write_captures = captures
                        .iter()
                        .filter(|capture| capture.value.mode == SourceCaptureMode::Write)
                        .collect::<Vec<_>>();
                    if write_captures.len() != 1 || captures.len() != 1 {
                        return Err(diagnostic(
                            expression.span,
                            "each layout-writing spawn must carry exactly one `write OBJECT.FIELD` capture",
                        ));
                    }
                    let Some((object, field)) = field_place(&write_captures[0].value.place.value)
                    else {
                        return Err(diagnostic(
                            write_captures[0].span,
                            "layout write capture must name one direct record field",
                        ));
                    };
                    writes.push(ConcurrentFieldWrite {
                        worker: worker.value.clone(),
                        object: object.to_owned(),
                        field: field.to_owned(),
                        span: expression.span,
                    });
                    spawned.push(binding.clone());
                } else if let SourceTaskExpressionKind::Join { handle } = &expression.kind {
                    saw_join = true;
                    let SourcePlace::Name(handle) = &handle.value else {
                        return Err(diagnostic(
                            expression.span,
                            "join must consume the direct handle bound by its spawn",
                        ));
                    };
                    joined.push(handle.value.clone());
                }
            }
            if spawned.len() < 2 || joined != spawned {
                return Err(diagnostic(
                    body.span,
                    "the scope must spawn at least two field writers, then join each handle once in spawn order",
                ));
            }
            if writes.len() >= 2 {
                if selected.is_some() {
                    return Err(diagnostic(
                        statement.span,
                        "source.bitfield-concurrency.v1 admits exactly one writing thread scope",
                    ));
                }
                selected = Some((statement.span, writes));
            }
        }
    }
    Ok(selected)
}

/// Derive one `DisjointFootprintsV1` program from actual simultaneous task
/// captures. Pairwise `AssertDisjoint` events are the admission condition for
/// granting those workers independent write authority.
pub fn lower_source_bitfield_tasks(
    source: &SourceProgram,
    entry: &str,
) -> Result<LoweredBitfieldTaskModule, Vec<SourceDiagnostic>> {
    let mut errors = Vec::new();
    if source.imports().len() != 1 || source.imports()[0].value != "std.concurrency" {
        errors.push(diagnostic(
            Span { start: 0, end: 0 },
            "source.bitfield-concurrency.v1 imports exactly `std.concurrency`",
        ));
    }
    let concurrency_profiles = source
        .declarations
        .iter()
        .filter(|declaration| declaration.value.kind == DeclarationKind::ConcurrencyProfile)
        .count();
    if concurrency_profiles != 1 {
        errors.push(diagnostic(
            Span { start: 0, end: 0 },
            "source.bitfield-concurrency.v1 requires exactly one `concurrency checked;` declaration",
        ));
    }
    let mut records = BTreeMap::<String, DeclaredRecord>::new();
    let mut contracts = Vec::new();
    for declaration in source
        .declarations
        .iter()
        .filter(|declaration| declaration.value.kind == DeclarationKind::Record)
    {
        match parse_record(source, declaration, contracts.len() as RecordId) {
            Ok(Some(record)) => {
                contracts.push(record.contract.clone());
                if records.insert(record.name.clone(), record).is_some() {
                    errors.push(diagnostic(
                        declaration.span,
                        "layout record name is not unique",
                    ));
                }
            }
            Ok(None) => errors.push(diagnostic(
                declaration.span,
                "every record on source.bitfield-concurrency.v1 requires an explicit layout",
            )),
            Err(error) => errors.push(error),
        }
    }
    let Some(function) = source.declarations.iter().find(|declaration| {
        declaration.value.kind == DeclarationKind::Function && declaration.value.name == entry
    }) else {
        errors.push(diagnostic(
            Span { start: 0, end: 0 },
            "entry function was not found",
        ));
        return Err(errors);
    };
    let Some(body) = &function.value.body else {
        errors.push(diagnostic(function.span, "entry function requires a body"));
        return Err(errors);
    };
    if declared_effects(source, function)
        != Some(BTreeSet::from([
            "join".to_owned(),
            "mut".to_owned(),
            "spawn".to_owned(),
        ]))
    {
        errors.push(diagnostic(
            function.span,
            "the entry must declare exactly `effects { spawn, join, mut }`",
        ));
    }
    if function
        .value
        .result_type
        .as_ref()
        .and_then(|result| result.executable_type())
        != Some(crate::frontend::ExecutableResultType::Unit)
    {
        errors.push(diagnostic(function.span, "the entry must return `unit`"));
    }
    if !function.value.parameters.is_empty() {
        errors.push(diagnostic(
            function.span,
            "the bounded entry takes no parameters",
        ));
    }
    if body.statements.len() != 3
        || let_object(source, body.statements[0].span).is_none()
        || !matches!(body.statements[1].value, StatementKind::ThreadScope { .. })
        || !statement_is(
            source,
            body.statements[2].span,
            &[
                TokenKind::Word("return".to_owned()),
                TokenKind::Symbol(Symbol::Semicolon),
            ],
        )
    {
        errors.push(diagnostic(
            body.span,
            "the bounded entry is exactly one object declaration, one writing thread scope, and `return;`",
        ));
    }
    let objects = body
        .statements
        .iter()
        .filter_map(|statement| let_object(source, statement.span))
        .collect::<BTreeMap<_, _>>();
    let selected = match collect_scope_writes(source, body) {
        Ok(Some(selected)) => selected,
        Ok(None) => {
            errors.push(diagnostic(
                body.span,
                "source.bitfield-concurrency.v1 requires at least two field-writing spawns in one thread scope",
            ));
            return Err(errors);
        }
        Err(error) => {
            errors.push(error);
            return Err(errors);
        }
    };
    let allowed_declarations = 1
        + records.len()
        + 1
        + selected
            .1
            .iter()
            .map(|write| write.worker.as_str())
            .collect::<BTreeSet<_>>()
            .len();
    if source.declarations.len() != allowed_declarations {
        errors.push(diagnostic(
            function.span,
            "source.bitfield-concurrency.v1 refuses every unrelated declaration",
        ));
    }
    for worker in selected
        .1
        .iter()
        .map(|write| write.worker.as_str())
        .collect::<BTreeSet<_>>()
    {
        if let Err(error) = validate_worker(source, worker) {
            errors.push(error);
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }

    let mut events = Vec::new();
    let mut allocations = BTreeMap::<String, (AllocationId, RecordId)>::new();
    let mut used_objects = BTreeSet::new();
    for write in &selected.1 {
        used_objects.insert(write.object.clone());
    }
    for (index, object) in used_objects.iter().enumerate() {
        let Some(type_name) = objects.get(object) else {
            errors.push(diagnostic(
                selected.0,
                format!("write capture object `{object}` must be declared as `let {object} = TYPE;` in the entry"),
            ));
            continue;
        };
        let Some(record) = records.get(type_name) else {
            errors.push(diagnostic(
                selected.0,
                format!(
                    "object `{object}` names `{type_name}`, which has no explicit layout contract"
                ),
            ));
            continue;
        };
        let alloc = (index + 1) as AllocationId;
        allocations.insert(object.clone(), (alloc, record.contract.record));
        events.push(FootprintEvent::Allocate {
            alloc,
            generation: 1 as Generation,
            shape: ElementShape::Record {
                record: record.contract.record,
            },
            count: 1,
        });
    }
    if !errors.is_empty() {
        return Err(errors);
    }

    let mut handles = Vec::<HandleId>::new();
    for (index, write) in selected.1.iter().enumerate() {
        let (alloc, record_id) = allocations[&write.object];
        let record = records
            .values()
            .find(|record| record.contract.record == record_id)
            .expect("allocation record came from records");
        let Some(field) = record.fields.get(&write.field).copied() else {
            errors.push(diagnostic(
                write.span,
                format!(
                    "layout record `{}` declares no field `{}`",
                    record.name, write.field
                ),
            ));
            continue;
        };
        let base = (index * 2 + 1) as HandleId;
        let derived = base + 1;
        events.push(FootprintEvent::DeriveSlice {
            handle: base,
            alloc,
            generation: 1,
            elem_offset: 0,
            elem_count: 1,
            declared_bytes: None,
        });
        events.push(FootprintEvent::DeriveField {
            handle: derived,
            base,
            field,
        });
        handles.push(derived);
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    for left in 0..handles.len() {
        for right in left + 1..handles.len() {
            events.push(FootprintEvent::AssertDisjoint {
                left: handles[left],
                right: handles[right],
            });
        }
    }
    Ok(LoweredBitfieldTaskModule {
        program: FootprintProgram {
            layout_contracts: contracts,
            events,
        },
        writes: selected.1,
        scope_span: selected.0,
    })
}
