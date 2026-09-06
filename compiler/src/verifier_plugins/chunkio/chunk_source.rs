//! Bounded, source-derived contracts for single-chunk Boolean field functions.
//!
//! Clang's typed AST is the input boundary. Every statement and expression must
//! belong to the supported grammar before either initial Boolean state is
//! evaluated. Calls need independently source-validated constant-field helper
//! bodies. Other calls, pointer aliases, loops, and fallthrough remain Unknown. A summary describes a borrowed live chunk, not complete teardown or
//! C memory safety. In particular, Fluent Bit's release_lock clears `busy`; it
//! does not unlock the underlying ChunkIO chunk.

use crate::typed_c_ast::{ast_declarations, function_ast};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

use super::PINNED_FLUENT_BIT_REVISION;
use crate::chunk_lifecycle::{ChunkId, ChunkOperation, ChunkState};
use crate::legacy_c::{C11TranslationUnit, legacy_c_source_functions, validate_unrestricted_c11};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum ChunkSourceField {
    Busy,
    Locked,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ChunkSourcePath {
    pub initial_value: bool,
    pub final_value: bool,
    pub return_value: i32,
    /// Taken/not-taken decisions in source execution order.
    pub branches: Vec<bool>,
    /// Every field assignment, including idempotent writes, in source order.
    pub writes: Vec<bool>,
    /// Source-validated helper writes outside the modeled Boolean field.
    pub frame_writes: Vec<(String, i32)>,
}

/// Only the typed source importer can construct this summary. The complete
/// finite pre-state domain is exactly {false, true}, under the documented
/// borrowed-live-chunk and exclusive-field-access preconditions.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ChunkSourceSummary {
    function: String,
    source_revision: String,
    field: ChunkSourceField,
    paths: [ChunkSourcePath; 2],
    dependencies: BTreeMap<String, String>,
}

impl ChunkSourceSummary {
    pub fn function(&self) -> &str {
        &self.function
    }

    pub fn source_revision(&self) -> &str {
        &self.source_revision
    }

    pub fn field(&self) -> ChunkSourceField {
        self.field
    }

    pub fn paths(&self) -> &[ChunkSourcePath; 2] {
        &self.paths
    }

    pub fn dependencies(&self) -> &BTreeMap<String, String> {
        &self.dependencies
    }

    /// Bind the source parameter to one live model identity. The caller must
    /// establish that its C pointer denotes this chunk and has exclusive access
    /// to the field. This does not infer pointer provenance or synchronization.
    /// Non-lifecycle helper writes remain in paths/frame_writes and the native
    /// contract certificate; ChunkState does not model error/metadata storage.
    pub fn instantiate(
        &self,
        state: &ChunkState,
        chunk: ChunkId,
    ) -> Result<(i32, Vec<ChunkOperation>), String> {
        let record = state
            .chunks
            .get(&chunk)
            .filter(|record| record.live)
            .ok_or_else(|| format!("source call requires live chunk {chunk}"))?;
        let value = match self.field {
            ChunkSourceField::Busy => record.busy,
            ChunkSourceField::Locked => record.locked,
        };
        let path = &self.paths[usize::from(value)];
        let mut previous = value;
        let operations = path
            .writes
            .iter()
            .filter(|value| {
                let changed = previous != **value;
                previous = **value;
                changed
            })
            .map(|value| match self.field {
                ChunkSourceField::Busy => ChunkOperation::SetBusy {
                    chunk,
                    busy: *value,
                },
                ChunkSourceField::Locked if *value => ChunkOperation::Lock { chunk },
                ChunkSourceField::Locked => ChunkOperation::Unlock { chunk },
            })
            .collect();
        Ok((path.return_value, operations))
    }
}

/// A checked, nonrecursive helper with one borrowed ChunkIO pointer and only
/// constant writes to ordinary int fields outside lock/busy. Source text is
/// retained; the audit must establish revision, build and cross-unit ABI identity.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ChunkFrameSummary {
    function: String,
    source: String,
    layout: Vec<(String, String)>,
    writes: Vec<(String, i32)>,
}

fn record_layout(
    source: &str,
    unit: &C11TranslationUnit<'_>,
) -> Result<Vec<(String, String)>, String> {
    let declarations = ast_declarations(source, unit, "cio_chunk")?;
    let exact: Vec<_> = declarations
        .iter()
        .filter(|node| {
            kind(node) == "RecordDecl"
                && node["name"] == "cio_chunk"
                && node["completeDefinition"] == true
                && node["tagUsed"] == "struct"
        })
        .collect();
    let [record] = exact.as_slice() else {
        return Err("expected one ordinary ChunkIO record".into());
    };
    children(record)
        .iter()
        .map(|field| {
            if kind(field) != "FieldDecl"
                || field["isBitfield"] == true
                || !children(field).is_empty()
            {
                return Err("chunk dependency layout has unmodeled attributes or fields".into());
            }
            Ok((
                field["name"].as_str().ok_or("unnamed chunk field")?.into(),
                field["type"]["qualType"]
                    .as_str()
                    .ok_or("missing chunk field type")?
                    .into(),
            ))
        })
        .collect()
}

pub fn import_chunk_frame_helper(
    unit: &C11TranslationUnit<'_>,
    revision: &str,
    function: &str,
) -> Result<ChunkFrameSummary, String> {
    if revision != PINNED_FLUENT_BIT_REVISION {
        return Err("chunk helper source revision mismatch".into());
    }
    let source = validate_unrestricted_c11(unit).map_err(|error| error.to_string())?;
    let ast = function_ast(&source, unit, function)?;
    if ast["type"]["qualType"] != "void (struct cio_chunk *)" {
        return Err("helper requires one ChunkIO pointer and void result".into());
    }
    let [parameter, body] = children(&ast) else {
        return Err("unmodeled helper parameters or attributes".into());
    };
    if kind(parameter) != "ParmVarDecl"
        || !children(parameter).is_empty()
        || kind(body) != "CompoundStmt"
    {
        return Err("unmodeled helper definition".into());
    }
    let layout = record_layout(&source, unit)?;
    let id = parameter["id"]
        .as_str()
        .ok_or("missing helper parameter identity")?;
    let mut writes = Vec::new();
    for statement in children(body) {
        let [place, expression] = children(statement) else {
            return Err("helper requires direct constant field assignments".into());
        };
        let field = place["name"]
            .as_str()
            .ok_or("helper requires a named field")?;
        if kind(statement) != "BinaryOperator"
            || statement["opcode"] != "="
            || matches!(field, "busy" | "lock")
            || !layout.contains(&(field.into(), "int".into()))
        {
            return Err(
                "helper must preserve lifecycle fields and write ordinary int fields".into(),
            );
        }
        let mut importer = Importer {
            parameter_id: id,
            field_name: field,
            used_field: false,
            helpers: &BTreeMap::new(),
            dependencies: BTreeMap::new(),
        };
        if !importer.field(place) {
            return Err("helper field is not bound to its borrowed parameter".into());
        }
        let expr = importer.expression(expression, 0)?;
        // Constant syntax only: checking two sample values would not establish
        // independence from arbitrary initial error/metadata fields.
        fn constant(expr: &Expression) -> bool {
            match expr {
                Expression::Integer(_) => true,
                Expression::Negate(value) => constant(value),
                _ => false,
            }
        }
        if !constant(&expr) {
            return Err("helper assignment is not a constant".into());
        }
        writes.push((field.into(), expr.evaluate(false)?));
    }
    if writes.is_empty() {
        return Err("helper has no checked field writes".into());
    }
    Ok(ChunkFrameSummary {
        function: function.into(),
        source,
        layout,
        writes,
    })
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum ChunkSourceResult {
    /// A complete field-effect summary, not a Safe translation-unit verdict.
    Summary(ChunkSourceSummary),
    Unknown {
        function: String,
        reason: String,
    },
}

/// Report every discovered main-source function, including definitions absent
/// after preprocessing. The revision is a caller assertion; the external audit
/// additionally verifies Git identity and a clean source tree.
pub fn import_chunk_source(
    unit: &C11TranslationUnit<'_>,
    source_revision: &str,
) -> Result<Vec<ChunkSourceResult>, String> {
    import_chunk_source_with_helpers(unit, source_revision, &[])
}

pub fn import_chunk_source_with_helpers(
    unit: &C11TranslationUnit<'_>,
    source_revision: &str,
    helpers: &[ChunkFrameSummary],
) -> Result<Vec<ChunkSourceResult>, String> {
    if source_revision != PINNED_FLUENT_BIT_REVISION {
        return Err("ChunkIO source revision does not match the pinned revision".into());
    }
    let source = validate_unrestricted_c11(unit).map_err(|error| error.to_string())?;
    let mut checked_helpers = BTreeMap::new();
    if !helpers.is_empty() {
        let layout = record_layout(&source, unit)?;
        for helper in helpers {
            let declarations = ast_declarations(&source, unit, &helper.function)?;
            let exact: Vec<_> = declarations
                .iter()
                .filter(|node| kind(node) == "FunctionDecl" && node["name"] == helper.function)
                .collect();
            if exact.is_empty()
                || exact.iter().any(|decl| {
                    decl["storageClass"] == "static"
                        || decl["type"]["qualType"] != "void (struct cio_chunk *)"
                        || children(decl)
                            .iter()
                            .any(|child| kind(child) != "ParmVarDecl")
                })
            {
                return Err("frame dependency requires matching external declarations without local definitions".into());
            }
            if helper.layout != layout
                || checked_helpers
                    .insert(helper.function.clone(), helper)
                    .is_some()
            {
                return Err("helper layout mismatch or duplicate helper".into());
            }
        }
    }
    let layouts = [("flb_input_chunk", "busy"), ("cio_chunk", "lock")]
        .map(|(record, field)| validate_field_layout(&source, unit, record, field));
    let mut results = Vec::new();
    for function in legacy_c_source_functions(unit.source) {
        let result = function_ast(&source, unit, &function).and_then(|ast| {
            let summary = derive_summary(&ast, &function, source_revision, &checked_helpers)?;
            layouts[match summary.field {
                ChunkSourceField::Busy => 0,
                ChunkSourceField::Locked => 1,
            }]
            .clone()?;
            Ok(summary)
        });
        results.push(match result {
            Ok(summary) => ChunkSourceResult::Summary(summary),
            Err(reason) => ChunkSourceResult::Unknown { function, reason },
        });
    }
    Ok(results)
}

fn validate_field_layout(
    source: &str,
    unit: &C11TranslationUnit<'_>,
    record: &str,
    field: &str,
) -> Result<(), String> {
    let definitions = ast_declarations(source, unit, record)?;
    let records = definitions
        .iter()
        .filter(|node| {
            kind(node) == "RecordDecl"
                && node["name"].as_str() == Some(record)
                && node["completeDefinition"] == true
                && node["tagUsed"] == "struct"
        })
        .collect::<Vec<_>>();
    let [record] = records.as_slice() else {
        return Err("expected one complete chunk record definition".into());
    };
    let fields = children(record)
        .iter()
        .filter(|node| kind(node) == "FieldDecl" && node["name"].as_str() == Some(field))
        .collect::<Vec<_>>();
    let [field] = fields.as_slice() else {
        return Err("expected one direct lifecycle field".into());
    };
    if field["type"]["qualType"] != "int" || field["isBitfield"] == true {
        return Err("lifecycle field must be an ordinary non-volatile int, not a bitfield".into());
    }
    Ok(())
}

fn kind(node: &Value) -> &str {
    node["kind"].as_str().unwrap_or("")
}

fn children(node: &Value) -> &[Value] {
    node["inner"].as_array().map_or(&[], Vec::as_slice)
}

#[derive(Clone, Debug)]
enum Expression {
    Integer(i32),
    Field,
    Not(Box<Self>),
    Negate(Box<Self>),
    Equal(Box<Self>, Box<Self>),
    NotEqual(Box<Self>, Box<Self>),
}

impl Expression {
    fn evaluate(&self, field: bool) -> Result<i32, String> {
        Ok(match self {
            Self::Integer(value) => *value,
            Self::Field => i32::from(field),
            Self::Not(value) => i32::from(value.evaluate(field)? == 0),
            Self::Negate(value) => value
                .evaluate(field)?
                .checked_neg()
                .ok_or("integer overflow in source expression")?,
            Self::Equal(left, right) => i32::from(left.evaluate(field)? == right.evaluate(field)?),
            Self::NotEqual(left, right) => {
                i32::from(left.evaluate(field)? != right.evaluate(field)?)
            }
        })
    }
}

#[derive(Clone, Debug)]
enum Statement {
    Block(Vec<Self>),
    If(Expression, Box<Self>, Option<Box<Self>>),
    Assign(Expression),
    Return(Expression),
    FrameWrites(Vec<(String, i32)>),
}

impl Statement {
    fn execute(&self, path: &mut ChunkSourcePath) -> Result<Option<i32>, String> {
        match self {
            Self::Block(statements) => {
                for statement in statements {
                    if let Some(value) = statement.execute(path)? {
                        return Ok(Some(value));
                    }
                }
                Ok(None)
            }
            Self::If(condition, yes, no) => {
                let taken = condition.evaluate(path.final_value)? != 0;
                path.branches.push(taken);
                if taken {
                    yes.execute(path)
                } else if let Some(no) = no {
                    no.execute(path)
                } else {
                    Ok(None)
                }
            }
            Self::Assign(value) => {
                let value = value.evaluate(path.final_value)?;
                if !matches!(value, 0 | 1) {
                    return Err("field assignment leaves the Boolean lifecycle domain".into());
                }
                let value = value != 0;
                path.writes.push(value);
                path.final_value = value;
                Ok(None)
            }
            Self::Return(value) => value.evaluate(path.final_value).map(Some),
            Self::FrameWrites(writes) => {
                path.frame_writes.extend(writes.clone());
                Ok(None)
            }
        }
    }
}

struct Importer<'a> {
    parameter_id: &'a str,
    field_name: &'a str,
    used_field: bool,
    helpers: &'a BTreeMap<String, &'a ChunkFrameSummary>,
    dependencies: BTreeMap<String, String>,
}

impl Importer<'_> {
    fn field(&mut self, node: &Value) -> bool {
        let inner = children(node);
        if kind(node) != "MemberExpr"
            || node["name"].as_str() != Some(self.field_name)
            || node["isArrow"].as_bool() != Some(true)
            || node["type"]["qualType"].as_str() != Some("int")
            || inner.len() != 1
        {
            return false;
        }
        let mut base = &inner[0];
        if kind(base) == "ImplicitCastExpr"
            && base["castKind"] == "LValueToRValue"
            && children(base).len() == 1
        {
            base = &children(base)[0];
        }
        let accepted = kind(base) == "DeclRefExpr"
            && base["referencedDecl"]["kind"] == "ParmVarDecl"
            && base["referencedDecl"]["id"].as_str() == Some(self.parameter_id);
        self.used_field |= accepted;
        accepted
    }

    fn expression(&mut self, node: &Value, depth: usize) -> Result<Expression, String> {
        if depth > 64 {
            return Err("source expression exceeds the import depth limit".into());
        }
        let inner = children(node);
        match (kind(node), inner) {
            ("IntegerLiteral", []) if node["type"]["qualType"] == "int" => node["value"]
                .as_str()
                .and_then(|value| value.parse().ok())
                .map(Expression::Integer)
                .ok_or_else(|| "unsupported integer literal".into()),
            ("ImplicitCastExpr", [value]) if node["castKind"] == "LValueToRValue" => {
                self.expression(value, depth + 1)
            }
            ("ParenExpr", [value]) => self.expression(value, depth + 1),
            ("MemberExpr", _) if self.field(node) => Ok(Expression::Field),
            ("UnaryOperator", [value]) if node["opcode"] == "!" => Ok(Expression::Not(Box::new(
                self.expression(value, depth + 1)?,
            ))),
            ("UnaryOperator", [value]) if node["opcode"] == "-" => Ok(Expression::Negate(
                Box::new(self.expression(value, depth + 1)?),
            )),
            ("BinaryOperator", [left, right])
                if node["opcode"] == "==" || node["opcode"] == "!=" =>
            {
                let left = Box::new(self.expression(left, depth + 1)?);
                let right = Box::new(self.expression(right, depth + 1)?);
                Ok(if node["opcode"] == "==" {
                    Expression::Equal(left, right)
                } else {
                    Expression::NotEqual(left, right)
                })
            }
            _ => Err(format!("unmodeled expression {}", kind(node))),
        }
    }

    fn statement(&mut self, node: &Value, depth: usize) -> Result<Statement, String> {
        if depth > 64 {
            return Err("source statement exceeds the import depth limit".into());
        }
        match (kind(node), children(node)) {
            ("CallExpr", [callee, argument]) => {
                let [reference] = children(callee) else {
                    return Err("unmodeled helper callee".into());
                };
                let [parameter] = children(argument) else {
                    return Err("helper requires direct borrowed parameter".into());
                };
                let name = reference["referencedDecl"]["name"]
                    .as_str()
                    .ok_or("missing helper identity")?;
                if kind(callee) != "ImplicitCastExpr"
                    || callee["castKind"] != "FunctionToPointerDecay"
                    || kind(reference) != "DeclRefExpr"
                    || reference["referencedDecl"]["kind"] != "FunctionDecl"
                    || reference["type"]["qualType"] != "void (struct cio_chunk *)"
                    || kind(argument) != "ImplicitCastExpr"
                    || argument["castKind"] != "LValueToRValue"
                    || kind(parameter) != "DeclRefExpr"
                    || parameter["referencedDecl"]["kind"] != "ParmVarDecl"
                    || parameter["referencedDecl"]["id"].as_str() != Some(self.parameter_id)
                    || self.field_name != "lock"
                {
                    return Err("helper signature or parameter binding mismatch".into());
                }
                let helper = self
                    .helpers
                    .get(name)
                    .ok_or("call lacks source-validated frame helper")?;
                self.dependencies.insert(name.into(), helper.source.clone());
                Ok(Statement::FrameWrites(helper.writes.clone()))
            }
            ("CompoundStmt", inner) => inner
                .iter()
                .map(|node| self.statement(node, depth + 1))
                .collect::<Result<_, _>>()
                .map(Statement::Block),
            ("IfStmt", [condition, yes]) if node["hasElse"] != true => Ok(Statement::If(
                self.expression(condition, depth + 1)?,
                Box::new(self.statement(yes, depth + 1)?),
                None,
            )),
            ("IfStmt", [condition, yes, no]) if node["hasElse"] == true => Ok(Statement::If(
                self.expression(condition, depth + 1)?,
                Box::new(self.statement(yes, depth + 1)?),
                Some(Box::new(self.statement(no, depth + 1)?)),
            )),
            ("BinaryOperator", [place, value]) if node["opcode"] == "=" && self.field(place) => {
                Ok(Statement::Assign(self.expression(value, depth + 1)?))
            }
            ("ReturnStmt", [value]) => Ok(Statement::Return(self.expression(value, depth + 1)?)),
            _ => Err(format!("unmodeled statement {}", kind(node))),
        }
    }
}

fn derive_summary(
    ast: &Value,
    function: &str,
    source_revision: &str,
    helpers: &BTreeMap<String, &ChunkFrameSummary>,
) -> Result<ChunkSourceSummary, String> {
    let (field, field_name) = match ast["type"]["qualType"].as_str() {
        Some("int (struct flb_input_chunk *)") => (ChunkSourceField::Busy, "busy"),
        Some("int (struct cio_chunk *)") => (ChunkSourceField::Locked, "lock"),
        _ => return Err("outside single borrowed chunk / int-result source profile".into()),
    };
    // Do not silently discard attributes, nested declarations, extra parameters,
    // or other definition children that can change execution semantics.
    let [parameter, body] = children(ast) else {
        return Err("unmodeled function parameters or attributes".into());
    };
    if kind(parameter) != "ParmVarDecl" || kind(body) != "CompoundStmt" {
        return Err("unmodeled function definition".into());
    }
    let mut importer = Importer {
        parameter_id: parameter["id"]
            .as_str()
            .ok_or("missing parameter identity")?,
        field_name,
        used_field: false,
        helpers,
        dependencies: BTreeMap::new(),
    };
    let statement = importer.statement(body, 0)?;
    if !importer.used_field {
        return Err("function has no modeled chunk field access".into());
    }
    let evaluate = |initial_value| {
        let mut path = ChunkSourcePath {
            initial_value,
            final_value: initial_value,
            return_value: 0,
            branches: Vec::new(),
            writes: Vec::new(),
            frame_writes: Vec::new(),
        };
        path.return_value = statement
            .execute(&mut path)?
            .ok_or("source path falls through without an explicit return")?;
        Ok::<_, String>(path)
    };
    Ok(ChunkSourceSummary {
        function: function.to_owned(),
        source_revision: source_revision.to_owned(),
        field,
        paths: [evaluate(false)?, evaluate(true)?],
        dependencies: importer.dependencies,
    })
}

#[cfg(test)]
#[path = "../../../tests/unit/chunk_source.rs"]
mod tests;
