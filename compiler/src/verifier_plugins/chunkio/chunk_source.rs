//! Bounded, source-derived contracts for single-chunk Boolean field functions.
//!
//! Clang's typed AST is the input boundary. Every statement and expression must
//! belong to the supported grammar before either initial Boolean state is
//! evaluated. Calls, pointer aliases, loops, other fields, and fallthrough are
//! Unknown. A summary describes a borrowed live chunk, not complete teardown or
//! C memory safety. In particular, Fluent Bit's release_lock clears `busy`; it
//! does not unlock the underlying ChunkIO chunk.

use std::io::Write;
use std::process::{Command, Stdio};

use serde_json::Value;

use super::PINNED_FLUENT_BIT_REVISION;
use crate::chunk_lifecycle::{ChunkId, ChunkOperation, ChunkState};
use crate::legacy_c::{C11TranslationUnit, legacy_c_source_functions, validate_unrestricted_c11};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChunkSourceField {
    Busy,
    Locked,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChunkSourcePath {
    pub initial_value: bool,
    pub final_value: bool,
    pub return_value: i32,
    /// Taken/not-taken decisions in source execution order.
    pub branches: Vec<bool>,
    /// Every field assignment, including idempotent writes, in source order.
    pub writes: Vec<bool>,
}

/// Only the typed source importer can construct this summary. The complete
/// finite pre-state domain is exactly {false, true}, under the documented
/// borrowed-live-chunk and exclusive-field-access preconditions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChunkSourceSummary {
    function: String,
    source_revision: String,
    field: ChunkSourceField,
    paths: [ChunkSourcePath; 2],
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

    /// Bind the source parameter to one live model identity. The caller must
    /// establish that its C pointer denotes this chunk and has exclusive access
    /// to the field. This does not infer pointer provenance or synchronization.
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

#[derive(Clone, Debug, PartialEq, Eq)]
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
    if source_revision != PINNED_FLUENT_BIT_REVISION {
        return Err("ChunkIO source revision does not match the pinned revision".into());
    }
    let source = validate_unrestricted_c11(unit).map_err(|error| error.to_string())?;
    let layouts = [("flb_input_chunk", "busy"), ("cio_chunk", "lock")]
        .map(|(record, field)| validate_field_layout(&source, unit, record, field));
    let mut results = Vec::new();
    for function in legacy_c_source_functions(unit.source) {
        let result = function_ast(&source, unit, &function).and_then(|ast| {
            let summary = derive_summary(&ast, &function, source_revision)?;
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

fn ast_declarations(
    source: &str,
    unit: &C11TranslationUnit<'_>,
    function: &str,
) -> Result<Vec<Value>, String> {
    let mut child = Command::new("clang")
        .args([
            "-x",
            "c",
            unit.dialect.clang_argument(),
            "-fsyntax-only",
            "-Xclang",
            "-ast-dump=json",
            "-Xclang",
            "-ast-dump-filter",
            "-Xclang",
            function,
            "-",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("Clang AST unavailable: {error}"))?;
    // Drain stdout concurrently with feeding input: filtered AST output can be
    // emitted before Clang has consumed a large preprocessed translation unit.
    let mut stdin = child.stdin.take().ok_or("Clang stdin unavailable")?;
    let output = std::thread::scope(|scope| {
        let writer = scope.spawn(move || stdin.write_all(source.as_bytes()));
        let output = child.wait_with_output();
        let written = writer
            .join()
            .map_err(|_| "Clang input writer panicked".to_owned())?;
        written.map_err(|error| format!("Clang input failed: {error}"))?;
        output.map_err(|error| format!("Clang AST failed: {error}"))
    })?;
    if !output.status.success() {
        return Err(format!(
            "Clang AST rejected source: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    serde_json::Deserializer::from_slice(&output.stdout)
        .into_iter::<Value>()
        .map(|value| value.map_err(|error| format!("unsupported Clang AST JSON: {error}")))
        .collect()
}

fn function_ast(
    source: &str,
    unit: &C11TranslationUnit<'_>,
    function: &str,
) -> Result<Value, String> {
    // Clang's name filter is a substring filter. Require one exact definition.
    let mut definitions = ast_declarations(source, unit, function)?
        .into_iter()
        .filter(|value| {
            kind(value) == "FunctionDecl"
                && value["name"].as_str() == Some(function)
                && children(value)
                    .iter()
                    .any(|node| kind(node) == "CompoundStmt")
        })
        .collect::<Vec<_>>();
    if definitions.len() != 1 {
        return Err(format!(
            "expected one selected definition after preprocessing, found {}",
            definitions.len()
        ));
    }
    Ok(definitions.remove(0))
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
        }
    }
}

struct Importer<'a> {
    parameter_id: &'a str,
    field_name: &'a str,
    used_field: bool,
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
    })
}

#[cfg(test)]
#[path = "../../../tests/unit/chunk_source.rs"]
mod tests;
