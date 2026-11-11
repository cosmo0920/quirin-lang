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

//! Card `W48-QR-TASK-RECORD-CAPTURES`: records on the scoped-task route.
//!
//! # The representation
//!
//! A record binding on this route is its **leaves**: one task slot per scalar
//! or owned-`Buffer<T>` leaf, depth-first in declaration order, a nested
//! record contributing its own leaves under its field's name — card
//! `W48-QR-USABLE-RECORDS`' leaf order. So every custody question about a
//! record is the existing per-resource question about an owned leaf, asked of
//! the unedited checker and ledger: a construction moves a Buffer into a leaf
//! slot, a field read copies a scalar leaf or moves an owned one out (leaving a
//! hole), a field write binds a fresh leaf slot (slots are single-assignment),
//! a refill is the core's `RefillField`, admitted exactly into a hole, and a
//! record capture is its leaves' captures. The core sees a record whole in one
//! place only — a worker's record **result**, carried through completion and
//! `join` as one outcome and unpacked into leaf slots by `UnpackRecord`.
//!
//! # What the lowering itself tracks
//!
//! The core decides custody per path. What it cannot see is that a record's
//! **scalar** leaves belong to a record that has moved: a scalar leaf is a copy
//! the parent still holds. So the lowering marks a whole-record move — a
//! construction of an outer record, a `move` capture — and refuses any later
//! use of the moved record by name. Where two arms of a `match` leave a record
//! in different states (moved on one, written on one), the record is
//! **unsettled** after the match and every later use refuses
//! (`W48-QR-TASK-RECORD-ARM-MERGE`): the lowering has no join of two slot
//! maps, and refusing is the fail-closed answer.
use super::*;
use qcore_core::source_record::{MAX_FIELDS, MAX_LEAVES, MAX_NESTING};

/// A field type a task record declares.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum FieldKind {
    /// A copy scalar (`i32` or a §G02t type) or an owned `Buffer<T>`.
    Scalar(Scalar),
    /// Another record type this module declares, held by value.
    Record(String),
}

/// One leaf of a record type: its field path and its kind — a copy scalar or
/// an owned Buffer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Leaf {
    pub(super) path: Vec<String>,
    pub(super) kind: Scalar,
}

/// A record type this module declares, with its flattened leaves.
#[derive(Clone, Debug)]
pub(super) struct RecordType {
    /// The key a [`ScalarType::Record`] result carries: the declaration's
    /// position among this module's record declarations.
    pub(super) key: usize,
    pub(super) name: String,
    pub(super) fields: Vec<(String, FieldKind)>,
    pub(super) leaves: Vec<Leaf>,
}

impl RecordType {
    /// Whether some leaf is an owned Buffer: a record that carries custody,
    /// crosses by `move` and returns by `return move`.
    pub(super) fn owns(&self) -> bool {
        self.leaves
            .iter()
            .any(|leaf| matches!(leaf.kind, Scalar::OwnedBuffer(_)))
    }

    /// The core's record declaration.
    pub(super) fn core(&self) -> qcore_core::task_execution::TaskRecord {
        qcore_core::task_execution::TaskRecord {
            name: self.name.clone(),
            leaves: self
                .leaves
                .iter()
                .map(|leaf| qcore_core::task_execution::TaskRecordLeaf {
                    path: leaf.path.join("."),
                    ty: leaf.kind.core(),
                })
                .collect(),
        }
    }
}

/// The record types of this module, by name.
pub(super) type Records = BTreeMap<String, RecordType>;

fn declaration_refusal(span: Span, pass: SemanticPass, message: String) -> SourceDiagnostic {
    failure(span, pass, message)
}

/// Read every record type declaration of this module for the task route.
///
/// A field is a §G02t scalar, an owned Buffer (`own<Buffer>` or
/// `own<Buffer<T>>`, the task route's own spelling of an owned resource), or
/// another record type the module declares. A `bytes` field is the buffer
/// route's byte buffer, which crosses no spawn on this route (`Q500`,
/// `WRITE-LEND-TASKS-EXTENT`); a loan, a lease and every other spelling
/// refuse at `Q020`.
pub(super) fn task_records(source: &SourceProgram) -> Result<Records, SourceDiagnostic> {
    let declared: BTreeSet<String> = source
        .declarations
        .iter()
        .filter(|declaration| declaration.value.kind == DeclarationKind::Record)
        .map(|declaration| declaration.value.name.clone())
        .collect();
    let mut shapes = Vec::new();
    for declaration in source
        .declarations
        .iter()
        .filter(|declaration| declaration.value.kind == DeclarationKind::Record)
    {
        let span = declaration.span;
        let name = declaration.value.name.clone();
        let tokens = tokens_for(source, span);
        let bad = |message: String| declaration_refusal(span, SemanticPass::OrdinaryTypes, message);
        let open = tokens
            .iter()
            .position(|token| symbol(Some(token), Symbol::LeftBrace))
            .ok_or_else(|| bad(format!("record type `{name}` has no field list")))?;
        let close = tokens.len().saturating_sub(1);
        if close <= open || !symbol(tokens.get(close), Symbol::RightBrace) {
            return Err(bad(format!("record type `{name}` is not closed")));
        }
        let mut fields: Vec<(String, FieldKind)> = Vec::new();
        let mut cursor = open + 1;
        while cursor < close {
            let field = name_of(tokens.get(cursor)).ok_or_else(|| {
                bad(format!(
                    "`{name}` expects a field name; a task record declares `FIELD: TYPE`"
                ))
            })?;
            if !symbol(tokens.get(cursor + 1), Symbol::Colon) {
                return Err(bad(format!("`{name}.{field}` is declared without its `:`")));
            }
            let start = cursor + 2;
            let mut end = start;
            let mut depth = 0usize;
            while end < close {
                if symbol(tokens.get(end), Symbol::Less) {
                    depth += 1;
                } else if symbol(tokens.get(end), Symbol::Greater) {
                    depth = depth.saturating_sub(1);
                } else if depth == 0 && symbol(tokens.get(end), Symbol::Comma) {
                    break;
                }
                end += 1;
            }
            let spelled = &tokens[start..end];
            let kind = match spelled {
                [word] if super::word(Some(word), "bytes") => {
                    return Err(declaration_refusal(
                        span,
                        SemanticPass::CEmission,
                        format!(
                            "`{name}.{field}: bytes` is `source.buffer.v1`'s byte buffer, which \
                             crosses no spawn on the task route; a task record owns an \
                             `own<Buffer<T>>` (a byte-extent buffer crossing a spawn is \
                             `WRITE-LEND-TASKS-EXTENT`)"
                        ),
                    ));
                }
                [word] => match name_of(Some(word)) {
                    Some(spelling) if table_scalar(spelling).is_some() => {
                        FieldKind::Scalar(table_scalar(spelling).expect("checked"))
                    }
                    Some(spelling) if declared.contains(spelling) => {
                        FieldKind::Record(spelling.to_owned())
                    }
                    _ => {
                        return Err(bad(format!(
                            "`{name}.{field}` has no task field type: a field is a §G02t scalar, \
                             `own<Buffer<T>>` or a record type this module declares"
                        )));
                    }
                },
                other => match scalar(other) {
                    Some(owned @ Scalar::OwnedBuffer(_)) => FieldKind::Scalar(owned),
                    Some(Scalar::ReadLoan(_) | Scalar::WriteLease(_)) => {
                        return Err(bad(format!(
                            "`{name}.{field}` is a loan or a lease, which is the product of a \
                             crossing and never a value a record holds"
                        )));
                    }
                    _ => {
                        return Err(bad(format!(
                            "`{name}.{field}` has no task field type: a field is a §G02t scalar, \
                             `own<Buffer<T>>` or a record type this module declares"
                        )));
                    }
                },
            };
            if fields.iter().any(|(existing, _)| existing == field) {
                return Err(bad(format!("`{name}` declares the field `{field}` twice")));
            }
            fields.push((field.to_owned(), kind));
            cursor = if end < close { end + 1 } else { end };
        }
        if fields.is_empty() || fields.len() > MAX_FIELDS {
            return Err(bad(format!(
                "record type `{name}` declares {} fields; a record declares 1 to {MAX_FIELDS}",
                fields.len()
            )));
        }
        shapes.push((span, name, fields));
    }
    let by_name: BTreeMap<String, Vec<(String, FieldKind)>> = shapes
        .iter()
        .map(|(_, name, fields)| (name.clone(), fields.clone()))
        .collect();
    let mut records = Records::new();
    for (key, (span, name, fields)) in shapes.into_iter().enumerate() {
        let mut leaves = Vec::new();
        let mut stack = vec![name.clone()];
        flatten(&by_name, &fields, &mut Vec::new(), &mut stack, &mut leaves)
            .map_err(|message| declaration_refusal(span, SemanticPass::OrdinaryTypes, message))?;
        records.insert(
            name.clone(),
            RecordType {
                key,
                name,
                fields,
                leaves,
            },
        );
    }
    Ok(records)
}

/// The leaves of a field list, depth-first in declaration order, refusing a
/// cycle and the depth and leaf bounds card `W48-QR-USABLE-RECORDS` set.
fn flatten(
    types: &BTreeMap<String, Vec<(String, FieldKind)>>,
    fields: &[(String, FieldKind)],
    prefix: &mut Vec<String>,
    stack: &mut Vec<String>,
    leaves: &mut Vec<Leaf>,
) -> Result<(), String> {
    for (field, kind) in fields {
        prefix.push(field.clone());
        match kind {
            FieldKind::Scalar(scalar) => {
                leaves.push(Leaf {
                    path: prefix.clone(),
                    kind: *scalar,
                });
                if leaves.len() > MAX_LEAVES {
                    return Err(format!(
                        "record type `{}` has more than {MAX_LEAVES} leaves",
                        stack[0]
                    ));
                }
            }
            FieldKind::Record(inner) => {
                if stack.contains(inner) {
                    return Err(format!(
                        "record type `{}` contains itself through `{}`",
                        stack[0],
                        prefix.join(".")
                    ));
                }
                if stack.len() >= MAX_NESTING {
                    return Err(format!(
                        "record type `{}` nests deeper than {MAX_NESTING} levels",
                        stack[0]
                    ));
                }
                let inner_fields = types.get(inner).cloned().unwrap_or_default();
                stack.push(inner.clone());
                flatten(types, &inner_fields, prefix, stack, leaves)?;
                stack.pop();
            }
        }
        prefix.pop();
    }
    Ok(())
}

fn name_of(token: Option<&Token>) -> Option<&str> {
    super::name(token)
}

/// A record binding in the lowering's environment: the record type, a slot
/// per leaf, and whether the binding may still be used.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct RecordValue {
    pub(super) ty: String,
    pub(super) leaves: Vec<LeafSlot>,
    pub(super) state: RecordState,
}

/// Whether a record binding may be used.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RecordState {
    Live,
    /// Moved whole — into an outer record, or to a worker by a `move`
    /// capture whose creation succeeded on this path.
    Moved,
    /// Left in different states by two arms of a `match`.
    Unsettled,
}

/// One leaf of a record binding: its path, its current slot and its kind.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct LeafSlot {
    pub(super) path: Vec<String>,
    pub(super) slot: usize,
    pub(super) kind: Scalar,
}

/// What a field path names: one leaf, or a sub-record (its leaves).
pub(super) enum PathTarget {
    Leaf(LeafSlot),
    Record { ty: String, leaves: Vec<LeafSlot> },
}

/// The field path a capture place spells: its root name and the fields after
/// it, or `None` for a place that is not a name followed by fields.
pub(super) fn place_path(place: &SourcePlace) -> Option<(Located<String>, Vec<String>)> {
    match place {
        SourcePlace::Name(name) => Some((name.clone(), Vec::new())),
        SourcePlace::Field { base, field } => {
            let (root, mut path) = place_path(&base.value)?;
            path.push(field.value.clone());
            Some((root, path))
        }
        SourcePlace::Dereference(_) | SourcePlace::Index { .. } => None,
    }
}

/// A place spelled as tokens `NAME . FIELD { . FIELD }` starting at `at`: the
/// root, the path and the index just past it.
pub(super) fn token_path(tokens: &[Token], at: usize) -> Option<(String, Vec<String>, usize)> {
    let root = name_of(tokens.get(at))?.to_owned();
    let mut path = Vec::new();
    let mut cursor = at + 1;
    while symbol(tokens.get(cursor), Symbol::Dot) {
        path.push(name_of(tokens.get(cursor + 1))?.to_owned());
        cursor += 2;
    }
    Some((root, path, cursor))
}

impl Builder<'_> {
    /// The record type `name` declares, if this module declares one.
    pub(super) fn record_type(&self, name: &str) -> Option<&RecordType> {
        self.records.get(name)
    }

    /// Fresh leaf slots for a value of `ty`.
    pub(super) fn fresh_leaves(
        &mut self,
        ty: &str,
        span: Span,
    ) -> Result<Vec<LeafSlot>, SourceDiagnostic> {
        let leaves = self
            .record_type(ty)
            .ok_or_else(|| self.bad(span, format!("`{ty}` is not a record type")))?
            .leaves
            .clone();
        leaves
            .into_iter()
            .map(|leaf| {
                Ok(LeafSlot {
                    path: leaf.path,
                    slot: self.temporary(span)?,
                    kind: leaf.kind,
                })
            })
            .collect()
    }

    /// A live record binding, or the refusal naming why it may not be used.
    pub(super) fn live_record(
        &self,
        name: &str,
        span: Span,
    ) -> Result<(SymbolId, RecordValue), SourceDiagnostic> {
        let (id, kind) = self.lookup(name, span)?;
        let ValueType::Record(record) = kind else {
            return Err(failure(
                span,
                SemanticPass::OrdinaryTypes,
                format!("`{name}` is not a record"),
            ));
        };
        match record.state {
            RecordState::Live => Ok((id, record)),
            RecordState::Moved => Err(failure(
                span,
                SemanticPass::Concurrency,
                format!(
                    "record `{name}` was moved whole on this path — into another record or to a \
                     worker — and is not used after it moved"
                ),
            )),
            RecordState::Unsettled => Err(failure(
                span,
                SemanticPass::Concurrency,
                format!(
                    "record `{name}` is left in different states by the arms of a `match` that \
                     both continue, and the task route has no join of the two; finish the \
                     record's changes on each arm (W48-QR-TASK-RECORD-ARM-MERGE)"
                ),
            )),
        }
    }

    /// What `root.path` names in a live record binding.
    pub(super) fn resolve_path(
        &self,
        root: &str,
        path: &[String],
        span: Span,
    ) -> Result<(SymbolId, RecordValue, PathTarget), SourceDiagnostic> {
        let (id, record) = self.live_record(root, span)?;
        if path.is_empty() {
            let target = PathTarget::Record {
                ty: record.ty.clone(),
                leaves: record.leaves.clone(),
            };
            return Ok((id, record, target));
        }
        if let Some(leaf) = record.leaves.iter().find(|leaf| leaf.path == path) {
            let target = PathTarget::Leaf(leaf.clone());
            return Ok((id, record, target));
        }
        let under: Vec<LeafSlot> = record
            .leaves
            .iter()
            .filter(|leaf| leaf.path.len() > path.len() && leaf.path[..path.len()] == *path)
            .cloned()
            .collect();
        if under.is_empty() {
            return Err(failure(
                span,
                SemanticPass::OrdinaryTypes,
                format!(
                    "`{root}.{}` names no field of the record type `{}`",
                    path.join("."),
                    record.ty
                ),
            ));
        }
        // The sub-record's type: walk the declared fields.
        let mut ty = record.ty.clone();
        for field in path {
            let declared = self
                .record_type(&ty)
                .expect("a live record's type is declared");
            match declared.fields.iter().find(|(name, _)| name == field) {
                Some((_, FieldKind::Record(inner))) => ty = inner.clone(),
                _ => {
                    return Err(failure(
                        span,
                        SemanticPass::OrdinaryTypes,
                        format!(
                            "`{root}.{}` passes through a field that is not a record",
                            path.join(".")
                        ),
                    ));
                }
            }
        }
        let leaves = under
            .into_iter()
            .map(|mut leaf| {
                leaf.path.drain(..path.len());
                leaf
            })
            .collect();
        Ok((id, record, PathTarget::Record { ty, leaves }))
    }

    /// Replace the binding `name` with `record`, in the innermost scope that
    /// holds it.
    pub(super) fn rebind_record(&mut self, name: &str, record: RecordValue) {
        self.rebind(name, ValueType::Record(record));
    }

    /// Whether this function's row declares `mut`, which a field write needs.
    fn declares_mut(&self) -> bool {
        self.signature.effects.mutates
    }

    /// `let NAME = TYPE { FIELD: VALUE, … };` — every declared field once, by
    /// name. A scalar field takes a literal of its type (range-checked) or a
    /// name of exactly its type; an owned field moves a Buffer of its payload
    /// type in; a record field takes a record binding of its type, copied for
    /// a record of scalars and moved for one that owns.
    pub(super) fn construct_record(
        &mut self,
        tokens: &[Token],
        span: Span,
        ty: &str,
    ) -> Result<RecordValue, SourceDiagnostic> {
        let window: Vec<&Token> = tokens.iter().collect();
        // `tokens` is the right-hand side: `TYPE { … }`. Reuse card 2's reader
        // over a synthesized `let _ = …;` window by position: the construction
        // reader returns token ranges relative to the window it was given.
        let Some(open) = tokens
            .iter()
            .position(|token| symbol(Some(token), Symbol::LeftBrace))
        else {
            return Err(self.bad(span, "a record construction is `TYPE { FIELD: VALUE, … }`"));
        };
        let close = tokens.len().saturating_sub(1);
        if open != 1 || !symbol(window.get(close).copied(), Symbol::RightBrace) {
            return Err(self.bad(span, "a record construction is `TYPE { FIELD: VALUE, … }`"));
        }
        let declared = self
            .record_type(ty)
            .ok_or_else(|| self.bad(span, format!("`{ty}` is not a record type")))?
            .clone();
        let mut supplied: BTreeMap<String, Vec<Token>> = BTreeMap::new();
        let mut cursor = open + 1;
        while cursor < close {
            let field = name_of(tokens.get(cursor))
                .ok_or_else(|| {
                    failure(
                        span,
                        SemanticPass::OrdinaryTypes,
                        format!("this `{ty}` construction expects a field name"),
                    )
                })?
                .to_owned();
            if !symbol(tokens.get(cursor + 1), Symbol::Colon) {
                return Err(failure(
                    span,
                    SemanticPass::OrdinaryTypes,
                    format!("`{field}` is written without its `:`"),
                ));
            }
            let start = cursor + 2;
            let mut end = start;
            while end < close && !symbol(tokens.get(end), Symbol::Comma) {
                end += 1;
            }
            if end == start {
                return Err(failure(
                    span,
                    SemanticPass::OrdinaryTypes,
                    format!("`{field}` is supplied no value"),
                ));
            }
            if supplied
                .insert(field.clone(), tokens[start..end].to_vec())
                .is_some()
            {
                return Err(failure(
                    span,
                    SemanticPass::OrdinaryTypes,
                    format!("this `{ty}` construction supplies `{field}` twice"),
                ));
            }
            cursor = if end < close { end + 1 } else { end };
        }
        for field in supplied.keys() {
            if !declared.fields.iter().any(|(name, _)| name == field) {
                return Err(failure(
                    span,
                    SemanticPass::OrdinaryTypes,
                    format!("`{ty}` declares no field `{field}`"),
                ));
            }
        }
        let mut leaves = Vec::new();
        let mut moved_records = Vec::new();
        for (field, kind) in &declared.fields {
            let value = supplied.get(field).ok_or_else(|| {
                failure(
                    span,
                    SemanticPass::OrdinaryTypes,
                    format!(
                        "this `{ty}` construction does not supply `{field}`; a construction \
                         supplies every declared field once"
                    ),
                )
            })?;
            match kind {
                FieldKind::Scalar(Scalar::OwnedBuffer(payload)) => {
                    let [token] = value.as_slice() else {
                        return Err(failure(
                            span,
                            SemanticPass::OrdinaryTypes,
                            format!("the owned field `{field}` takes one Buffer name, moved in"),
                        ));
                    };
                    let buffer = name_of(Some(token)).ok_or_else(|| {
                        failure(
                            span,
                            SemanticPass::OrdinaryTypes,
                            format!("the owned field `{field}` takes one Buffer name, moved in"),
                        )
                    })?;
                    let (id, kind) = self.lookup(buffer, span)?;
                    if kind != ValueType::Scalar(Scalar::OwnedBuffer(*payload)) {
                        return Err(failure(
                            span,
                            SemanticPass::OrdinaryTypes,
                            format!(
                                "the owned field `{field}` holds {}, and `{buffer}` is not one",
                                scalar_spelling(Scalar::OwnedBuffer(*payload))
                            ),
                        ));
                    }
                    let slot = self.temporary(span)?;
                    self.instruction(
                        span,
                        TaskInstructionKind::BindScalar {
                            slot,
                            value: ScalarOperand::Move(id.0 as usize),
                        },
                    );
                    leaves.push(LeafSlot {
                        path: vec![field.clone()],
                        slot,
                        kind: Scalar::OwnedBuffer(*payload),
                    });
                }
                FieldKind::Scalar(scalar) => {
                    let slot = self.temporary(span)?;
                    let bound = self.typed_scalar_value(value, *scalar, span, field)?;
                    self.bind_leaf_value(slot, bound, span);
                    leaves.push(LeafSlot {
                        path: vec![field.clone()],
                        slot,
                        kind: *scalar,
                    });
                }
                FieldKind::Record(inner) => {
                    let [token] = value.as_slice() else {
                        return Err(failure(
                            span,
                            SemanticPass::OrdinaryTypes,
                            format!("the record field `{field}` takes one `{inner}` binding"),
                        ));
                    };
                    let source = name_of(Some(token)).ok_or_else(|| {
                        failure(
                            span,
                            SemanticPass::OrdinaryTypes,
                            format!("the record field `{field}` takes one `{inner}` binding"),
                        )
                    })?;
                    let (_, record) = self.live_record(source, span)?;
                    if record.ty != *inner {
                        return Err(failure(
                            span,
                            SemanticPass::OrdinaryTypes,
                            format!(
                                "the record field `{field}` holds a `{inner}`, and `{source}` is \
                                 a `{}`",
                                record.ty
                            ),
                        ));
                    }
                    let owns = self.record_type(inner).is_some_and(RecordType::owns);
                    for leaf in &record.leaves {
                        let slot = self.temporary(span)?;
                        self.instruction(
                            span,
                            TaskInstructionKind::BindScalar {
                                slot,
                                value: if matches!(leaf.kind, Scalar::OwnedBuffer(_)) {
                                    ScalarOperand::Move(leaf.slot)
                                } else {
                                    ScalarOperand::Slot(leaf.slot)
                                },
                            },
                        );
                        self.copy_known(leaf.slot, slot);
                        let mut path = vec![field.clone()];
                        path.extend(leaf.path.iter().cloned());
                        leaves.push(LeafSlot {
                            path,
                            slot,
                            kind: leaf.kind,
                        });
                    }
                    if owns {
                        moved_records.push((source.to_owned(), record));
                    }
                }
            }
        }
        // A held record that owns moved in whole: it is used no more.
        for (name, mut record) in moved_records {
            record.state = RecordState::Moved;
            self.rebind_record(&name, record);
        }
        Ok(RecordValue {
            ty: ty.to_owned(),
            leaves,
            state: RecordState::Live,
        })
    }

    /// A value written at a scalar leaf of type `scalar`: a literal of that
    /// type (range-checked) or a name bound to exactly that type.
    fn typed_scalar_value(
        &self,
        tokens: &[Token],
        scalar: Scalar,
        span: Span,
        field: &str,
    ) -> Result<ScalarOperand, SourceDiagnostic> {
        if let Scalar::Fixed(ty) = scalar
            && let Some(literal) = self.fixed_literal(tokens, ty, span)
        {
            return literal;
        }
        let (value, kind) = self.scalar_operand(tokens, span).map_err(|error| {
            failure(
                span,
                SemanticPass::OrdinaryTypes,
                format!(
                    "the field `{field}` takes a value of its type: {}",
                    error.message
                ),
            )
        })?;
        if kind != scalar {
            return Err(failure(
                span,
                SemanticPass::OrdinaryTypes,
                format!(
                    "the field `{field}` holds {}, and this value is {}; nothing converts \
                     implicitly",
                    scalar_spelling(scalar),
                    scalar_spelling(kind)
                ),
            ));
        }
        Ok(value)
    }

    /// Bind a fresh leaf slot to a scalar operand, propagating a known value.
    fn bind_leaf_value(&mut self, slot: usize, value: ScalarOperand, span: Span) {
        let known = match &value {
            ScalarOperand::I32(value) => Some(i128::from(*value)),
            ScalarOperand::Fixed { value, .. } => Some(*value),
            ScalarOperand::Slot(source) => self.known_slot(*source),
            _ => None,
        };
        if let Some(known) = known {
            self.known.insert(SymbolId(slot as u32), known);
        }
        self.instruction(span, TaskInstructionKind::BindScalar { slot, value });
    }

    fn known_slot(&self, slot: usize) -> Option<i128> {
        self.known
            .iter()
            .find(|(symbol, _)| symbol.0 as usize == slot)
            .map(|(_, value)| *value)
    }

    fn copy_known(&mut self, from: usize, to: usize) {
        if let Some(known) = self.known_slot(from) {
            self.known.insert(SymbolId(to as u32), known);
        }
    }

    /// `let NAME = ROOT . PATH ;` — a scalar leaf copied, an owned leaf moved
    /// out (leaving a hole the core tracks), or a sub-record of scalars copied.
    /// A whole read of a sub-record that owns refuses by name.
    pub(super) fn field_read(
        &mut self,
        root: &str,
        path: &[String],
        span: Span,
        binding: SymbolId,
    ) -> Result<ValueType, SourceDiagnostic> {
        let (_, _, target) = self.resolve_path(root, path, span)?;
        match target {
            PathTarget::Leaf(leaf) => {
                let value = if matches!(leaf.kind, Scalar::OwnedBuffer(_)) {
                    ScalarOperand::Move(leaf.slot)
                } else {
                    ScalarOperand::Slot(leaf.slot)
                };
                self.copy_known(leaf.slot, binding.0 as usize);
                self.instruction(
                    span,
                    TaskInstructionKind::BindScalar {
                        slot: binding.0 as usize,
                        value,
                    },
                );
                Ok(ValueType::Scalar(leaf.kind))
            }
            PathTarget::Record { ty, leaves } => {
                if leaves
                    .iter()
                    .any(|leaf| matches!(leaf.kind, Scalar::OwnedBuffer(_)))
                {
                    return Err(failure(
                        span,
                        SemanticPass::Concurrency,
                        format!(
                            "`{root}.{}` is a sub-record that owns a Buffer; a whole read of it \
                             would move an aggregate out of another, and is not admitted \
                             (W48-QR-NESTED-OWNED-SUBRECORD-MOVE)",
                            path.join(".")
                        ),
                    ));
                }
                let copied = self.copy_leaves(&leaves, span)?;
                Ok(ValueType::Record(RecordValue {
                    ty,
                    leaves: copied,
                    state: RecordState::Live,
                }))
            }
        }
    }

    /// Fresh slots holding copies of scalar leaves.
    fn copy_leaves(
        &mut self,
        leaves: &[LeafSlot],
        span: Span,
    ) -> Result<Vec<LeafSlot>, SourceDiagnostic> {
        let mut copied = Vec::with_capacity(leaves.len());
        for leaf in leaves {
            let slot = self.temporary(span)?;
            self.bind_leaf_value(slot, ScalarOperand::Slot(leaf.slot), span);
            copied.push(LeafSlot {
                path: leaf.path.clone(),
                slot,
                kind: leaf.kind,
            });
        }
        Ok(copied)
    }

    /// `let NAME = RECORD ;` — a copy of a record of scalars. A record that
    /// owns has no whole-record move spelling (W48-QR-RECORD-WHOLE-MOVE).
    pub(super) fn record_copy(
        &mut self,
        source: &str,
        span: Span,
    ) -> Result<ValueType, SourceDiagnostic> {
        let (_, record) = self.live_record(source, span)?;
        if record
            .leaves
            .iter()
            .any(|leaf| matches!(leaf.kind, Scalar::OwnedBuffer(_)))
        {
            return Err(failure(
                span,
                SemanticPass::Concurrency,
                format!(
                    "`{source}` owns a Buffer, so binding it whole to another name would move \
                     it, and a record has no whole-record move spelling on this route \
                     (W48-QR-RECORD-WHOLE-MOVE)"
                ),
            ));
        }
        let leaves = self.copy_leaves(&record.leaves, span)?;
        Ok(ValueType::Record(RecordValue {
            ty: record.ty,
            leaves,
            state: RecordState::Live,
        }))
    }

    /// `ROOT . PATH = VALUE ;` — a scalar leaf rebound to a fresh slot holding
    /// a value of exactly its type, or an owned leaf **refilled** (the core's
    /// `RefillField`, admitted exactly into a hole). A write needs `mut` in the
    /// function's row (card `W48-QR-USABLE-RECORDS`: a write derives `mut`).
    pub(super) fn field_write(
        &mut self,
        root: &str,
        path: &[String],
        value: &[Token],
        span: Span,
    ) -> Result<(), SourceDiagnostic> {
        if !self.declares_mut() {
            return Err(failure(
                span,
                SemanticPass::TypedErrors,
                format!(
                    "`{root}.{} = …` writes a field, and a write derives `mut`, which this \
                     function's row does not declare",
                    path.join(".")
                ),
            ));
        }
        let (_, mut record, target) = self.resolve_path(root, path, span)?;
        let PathTarget::Leaf(leaf) = target else {
            return Err(failure(
                span,
                SemanticPass::OrdinaryTypes,
                format!(
                    "`{root}.{}` is a sub-record; a field is written field by field \
                     (W48-QR-RECORD-FIELD-WHOLE-WRITE)",
                    path.join(".")
                ),
            ));
        };
        let slot = self.temporary(span)?;
        if let Scalar::OwnedBuffer(payload) = leaf.kind {
            let [token] = value else {
                return Err(failure(
                    span,
                    SemanticPass::OrdinaryTypes,
                    "a refill of an owned field moves one Buffer name in",
                ));
            };
            let buffer = name_of(Some(token)).ok_or_else(|| {
                failure(
                    span,
                    SemanticPass::OrdinaryTypes,
                    "a refill of an owned field moves one Buffer name in",
                )
            })?;
            let (id, kind) = self.lookup(buffer, span)?;
            if kind != ValueType::Scalar(Scalar::OwnedBuffer(payload)) {
                return Err(failure(
                    span,
                    SemanticPass::OrdinaryTypes,
                    format!(
                        "the owned field `{root}.{}` holds {}, and `{buffer}` is not one",
                        path.join("."),
                        scalar_spelling(Scalar::OwnedBuffer(payload))
                    ),
                ));
            }
            self.instruction(
                span,
                TaskInstructionKind::RefillField {
                    hole: leaf.slot,
                    slot,
                    value: ScalarOperand::Move(id.0 as usize),
                },
            );
        } else {
            let field = path.join(".");
            let bound = self.typed_scalar_value(value, leaf.kind, span, &field)?;
            self.bind_leaf_value(slot, bound, span);
        }
        for held in &mut record.leaves {
            if held.path == leaf.path {
                held.slot = slot;
            }
        }
        self.rebind_record(root, record);
        Ok(())
    }

    /// `drop ROOT . PATH ;` releases one owned leaf; `drop RECORD ;` releases
    /// every owned leaf in leaf order, each of which must be live on the path —
    /// a hole refuses, so refill it or release the others one by one.
    pub(super) fn drop_record_path(
        &mut self,
        root: &str,
        path: &[String],
        span: Span,
    ) -> Result<(), SourceDiagnostic> {
        let (_, _, target) = self.resolve_path(root, path, span)?;
        let leaves = match target {
            PathTarget::Leaf(leaf) => vec![leaf],
            PathTarget::Record { leaves, .. } => leaves,
        };
        let owned: Vec<LeafSlot> = leaves
            .into_iter()
            .filter(|leaf| matches!(leaf.kind, Scalar::OwnedBuffer(_)))
            .collect();
        if owned.is_empty() {
            return Err(self.bad(
                span,
                "drop requires an owned buffer; this record path owns none",
            ));
        }
        for leaf in owned {
            self.instruction(span, TaskInstructionKind::DropBuffer { buffer: leaf.slot });
        }
        Ok(())
    }

    /// Unpack a collected record — the value slot a `join` outcome bound —
    /// into fresh leaf slots, at the current position.
    pub(super) fn unpack_record(
        &mut self,
        ty: &str,
        record_slot: usize,
        span: Span,
    ) -> Result<RecordValue, SourceDiagnostic> {
        let leaves = self.fresh_leaves(ty, span)?;
        self.instruction(
            span,
            TaskInstructionKind::UnpackRecord {
                record: record_slot,
                leaves: leaves.iter().map(|leaf| leaf.slot).collect(),
            },
        );
        Ok(RecordValue {
            ty: ty.to_owned(),
            leaves,
            state: RecordState::Live,
        })
    }

    /// `return RECORD;` of a record of scalars, or `return move RECORD;` of a
    /// record that owns — a record crosses `join` the way it crosses a spawn.
    pub(super) fn record_return(
        &mut self,
        tokens: &[Token],
        span: Span,
        ty: &str,
    ) -> Result<ScalarOperand, SourceDiagnostic> {
        let (moved, name) = match tokens {
            [_, token, _] => (false, name_of(Some(token))),
            [_, marker, token, _] if super::word(Some(marker), "move") => {
                (true, name_of(Some(token)))
            }
            _ => (false, None),
        };
        let name = name.ok_or_else(|| {
            failure(
                span,
                SemanticPass::OrdinaryTypes,
                format!("a `{ty}` result returns one record binding"),
            )
        })?;
        let (_, record) = self.live_record(name, span)?;
        if record.ty != ty {
            return Err(failure(
                span,
                SemanticPass::OrdinaryTypes,
                format!(
                    "return value does not match the declared task function result: `{name}` is \
                     a `{}` and the result is a `{ty}`",
                    record.ty
                ),
            ));
        }
        let owns = record
            .leaves
            .iter()
            .any(|leaf| matches!(leaf.kind, Scalar::OwnedBuffer(_)));
        if owns != moved {
            return Err(failure(
                span,
                SemanticPass::Concurrency,
                if owns {
                    format!(
                        "`{name}` owns a Buffer, so it returns by custody transfer: \
                         `return move {name};`"
                    )
                } else {
                    format!(
                        "`{name}` is a record of scalars, which returns by copy: `return {name};` \
                         — a `move` has no custody to carry"
                    )
                },
            ));
        }
        Ok(ScalarOperand::Record(
            record
                .leaves
                .iter()
                .map(|leaf| {
                    if matches!(leaf.kind, Scalar::OwnedBuffer(_)) {
                        ScalarOperand::Move(leaf.slot)
                    } else {
                        ScalarOperand::Slot(leaf.slot)
                    }
                })
                .collect(),
        ))
    }

    /// A whole-record capture against a record parameter: the aggregate
    /// judgment, the enumeration beside it, and the leaves' operands.
    pub(super) fn record_capture(
        &self,
        capture: &ResolvedTaskCapture,
        root: &str,
        path: &[String],
        parameter: &str,
    ) -> Result<(Vec<ScalarOperand>, Option<String>), SourceDiagnostic> {
        use qcore_core::transfer::{AdmittedTransfer, AggregateRefusal};
        let span = capture.span;
        let (_, _, target) = self.resolve_path(root, path, span)?;
        let PathTarget::Record { ty, leaves } = target else {
            return Err(failure(
                span,
                SemanticPass::OrdinaryTypes,
                "capture type does not match worker parameter",
            ));
        };
        let kinds: Vec<_> = leaves.iter().map(|leaf| transfer_kind(leaf.kind)).collect();
        // A sub-record that owns crosses field by field: moving it whole would
        // leave its scalar fields readable in a record that no longer holds
        // them, and the lowering tracks a whole move per binding only.
        if !path.is_empty()
            && kinds.contains(&qcore_core::transfer::TransferKind::OwnedBuffer)
            && matches!(capture.mode, SourceCaptureMode::Move)
        {
            return Err(failure(
                span,
                SemanticPass::Concurrency,
                format!(
                    "`{root}.{}` is a sub-record that owns a Buffer; it crosses field by field, and \
                     a whole move of it out of `{root}` is not admitted \
                     (W48-QR-NESTED-OWNED-SUBRECORD-MOVE)",
                    path.join(".")
                ),
            ));
        }
        let derived = qcore_core::transfer::admits_aggregate_capture(
            &ty,
            &kinds,
            transfer_mode(&capture.mode),
            &parameter.to_owned(),
        );
        let owns = kinds.contains(&qcore_core::transfer::TransferKind::OwnedBuffer);
        // The enumeration: a record of scalars under `copy`, a record that owns
        // under `move`, into a parameter of its own type.
        let enumerated = match (&capture.mode, owns, ty == parameter) {
            (SourceCaptureMode::Copy, false, true) => Some(AdmittedTransfer::Duplicate),
            (SourceCaptureMode::Move, true, true) => Some(AdmittedTransfer::MoveCustody),
            _ => None,
        };
        let agrees = match (&derived, enumerated) {
            (Ok(admitted), Some(form)) => *admitted == form,
            (Err(_), None) => true,
            _ => false,
        };
        if !agrees {
            return Err(self.bad(
                span,
                "the derived transferability judgment and the capture enumeration disagree \
                 about this record capture",
            ));
        }
        let spelled = if path.is_empty() {
            root.to_owned()
        } else {
            format!("{root}.{}", path.join("."))
        };
        match derived {
            Ok(admitted) => {
                let operands = leaves
                    .iter()
                    .map(|leaf| {
                        if matches!(leaf.kind, Scalar::OwnedBuffer(_)) {
                            ScalarOperand::Move(leaf.slot)
                        } else {
                            ScalarOperand::Slot(leaf.slot)
                        }
                    })
                    .collect();
                // A whole record moved to a worker is used no more on the
                // path on which the creation succeeded.
                let moved = (admitted == AdmittedTransfer::MoveCustody && path.is_empty())
                    .then(|| root.to_owned());
                Ok((operands, moved))
            }
            Err(refusal) => {
                let asked = mode_spelling(&capture.mode);
                let message = match refusal {
                    AggregateRefusal::WriteCaptureUnimplemented => format!(
                        "the transferability judgment refuses this capture: a {asked} capture \
                         names no transfer or share contract — no kind crosses a spawn boundary \
                         under it, and a record is asked nothing before that; `{spelled}` (a \
                         `{ty}`) crosses only under {}",
                        record_contracts(owns)
                    ),
                    AggregateRefusal::Leaf { index, refusal } => {
                        let leaf = &leaves[index];
                        let leaf_name = format!("{spelled}.{}", leaf.path.join("."));
                        let leaf_kind = scalar_spelling(leaf.kind);
                        let reason = match refusal {
                            qcore_core::transfer::TransferRefusal::ModeNotAdmittedForKind => {
                                format!(
                                    "no transfer or share contract admits {asked} of \
                                     {leaf_kind} across a spawn boundary; {leaf_kind} crosses \
                                     only under {}",
                                    admitted_contracts(leaf.kind)
                                )
                            }
                            _ => format!(
                                "{leaf_kind} carries no transfer or share contract for this \
                                 crossing"
                            ),
                        };
                        format!(
                            "the transferability judgment refuses this capture: {asked} of the \
                             record `{spelled}` (a `{ty}`) asks it of every leaf, and the leaf \
                             `{leaf_name}` does not cross: {reason}; `{spelled}` crosses only \
                             under {}",
                            record_contracts(owns)
                        )
                    }
                    AggregateRefusal::ModeNotAdmittedForRecord => format!(
                        "the transferability judgment refuses this capture: `{spelled}` (a \
                         `{ty}`) owns no Buffer, so it crosses by duplication and {asked} has no \
                         custody to carry; it crosses only under {}",
                        record_contracts(false)
                    ),
                    AggregateRefusal::NoWholeRecordProtocol => format!(
                        "the transferability judgment refuses this capture: {asked} names a \
                         sharing protocol, and a record shares none whole — an owned field is \
                         shared by its own field capture (`read {spelled}.FIELD share N`, `lend \
                         {spelled}.FIELD LOW..HIGH`); `{spelled}` crosses only under {}",
                        record_contracts(owns)
                    ),
                    AggregateRefusal::ReceivedKindMismatch => format!(
                        "the transferability judgment refuses this capture: {asked} of \
                         `{spelled}` is an admitted transfer, but it delivers a `{ty}` and the \
                         worker declares it receives a `{parameter}`; record types are nominal"
                    ),
                };
                Err(failure(span, SemanticPass::Concurrency, message))
            }
        }
    }

    /// `print RENDERING ROOT.PATH;` or `log LEVEL RENDERING ROOT.PATH;` of a
    /// scalar leaf, on the composed route (§G02j). `None` when the statement
    /// is not that shape.
    pub(super) fn record_line(
        &mut self,
        tokens: &[Token],
        span: Span,
    ) -> Option<Result<(), SourceDiagnostic>> {
        use qcore_core::std_io::{Level, Rendering, StdIoEvent, StdIoOperand};
        let (level, rendering_at) = if super::word(tokens.first(), "print") {
            (None, 1)
        } else if super::word(tokens.first(), "log") {
            (Some(Level::parse(name_of(tokens.get(1))?)?), 2)
        } else {
            return None;
        };
        let rendering = Rendering::parse(name_of(tokens.get(rendering_at))?)?;
        let (root, path, after) = token_path(tokens, rendering_at + 1)?;
        if path.is_empty()
            || after + 1 != tokens.len()
            || !symbol(tokens.get(after), Symbol::Semicolon)
            || !matches!(self.lookup(&root, span), Ok((_, ValueType::Record(_))))
        {
            return None;
        }
        Some((|| {
            let (_, _, target) = self.resolve_path(&root, &path, span)?;
            let spelled = format!("{root}.{}", path.join("."));
            let PathTarget::Leaf(leaf) = target else {
                return Err(failure(
                    span,
                    SemanticPass::ComposedTasks,
                    format!("`{spelled}` is a sub-record, which renders field by field"),
                ));
            };
            if !matches!(leaf.kind, Scalar::I32 | Scalar::Fixed(_)) {
                return Err(failure(
                    span,
                    SemanticPass::ComposedTasks,
                    format!(
                        "`{spelled}` is an owned field, which renders no number; read it with \
                         `qcore_buffer_read({spelled})` first"
                    ),
                ));
            }
            if rendering == Rendering::Hex && leaf.kind == Scalar::Fixed(FixedType::Bool) {
                return Err(failure(
                    span,
                    SemanticPass::ComposedTasks,
                    format!(
                        "`{spelled}` is a `bool`, which renders `true`/`false` under `dec` and \
                         has no `hex` rendering"
                    ),
                ));
            }
            let operand = StdIoOperand::Scalar(spelled);
            let event = match level {
                None => StdIoEvent::Print { rendering, operand },
                Some(level) => StdIoEvent::Log {
                    level,
                    rendering,
                    operand,
                },
            };
            self.sites.push(TaskIoSite {
                function: self.function,
                block: self.current,
                index: self.instructions.len(),
                event,
                operand: Some(leaf.slot),
                span: core_span(span),
            });
            Ok(())
        })())
    }
}

/// The contracts a record crosses under, for the judgment's messages.
fn record_contracts(owns: bool) -> &'static str {
    if owns {
        "`move` (custody of every owned field transferred to the worker, its scalars \
         duplicated), with each owned field lent or leased by its own field capture"
    } else {
        "`copy` (an independent duplicate of every field; no custody crosses)"
    }
}
