//! Bounded symbolic access paths from the selected Monkey callback definitions.
//!
//! These are source projections, NOT callback-safety or custody certificates.
//! Reads and calls yield fresh symbols; pointer aliases retain those symbols.
//! Calls remain opaque (in particular, a recorded close call is not discharge).
//! All branches are retained, including infeasible alternatives. Consumers must
//! establish guards, pointer provenance, live/bounded storage, arithmetic, call
//! contracts, and synchronization before interpreting a path as safe.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use serde_json::Value;

use crate::external_integration::PINNED_FLUENT_BIT_REVISION;
use crate::legacy_c::{C11TranslationUnit, validate_unrestricted_c11};
use crate::typed_c_ast::{ast_declarations, children, function_ast, kind};

const CALLBACKS: [&str; 3] = ["cb_event", "cb_timeout", "cb_wait_2_timeout"];
const WRAPPERS: [&str; 15] = [
    "mk_event_libevent_socketpair",
    "_mk_event_init",
    "_mk_event_loop_create",
    "_mk_event_loop_destroy",
    "_mk_event_update",
    "_mk_event_add",
    "_mk_event_del",
    "_mk_event_timeout_create",
    "_mk_event_timeout_destroy",
    "_mk_event_channel_create",
    "_mk_event_channel_destroy",
    "_mk_event_inject",
    "_mk_event_wait_with_flags",
    "_mk_event_wait_2",
    "_mk_event_backend",
];
const MAX_PATHS: usize = 64;
const MAX_OPERATIONS: usize = 512;

/// Typed symbolic values. A symbol refers to an earlier Read or Call in this
/// same path. Integer casts and sizeof remain explicit, never host-evaluated.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SourceValue {
    pub c_type: String,
    pub expression: SourceExpression,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SourceExpression {
    Function {
        name: String,
        signature: String,
    },
    Parameter {
        index: usize,
    },
    LocalStorage {
        local: usize,
    },
    Integer {
        value: String,
    },
    String {
        value: String,
    },
    SizeOf {
        argument_type: String,
    },
    Symbol {
        operation: usize,
    },
    Cast {
        cast: String,
        value: Box<SourceValue>,
    },
    Unary {
        operator: String,
        value: Box<SourceValue>,
    },
    Binary {
        operator: String,
        left: Box<SourceValue>,
        right: Box<SourceValue>,
    },
    Field {
        base: Box<SourceValue>,
        name: String,
    },
    Index {
        base: Box<SourceValue>,
        index: Box<SourceValue>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SourceOperation {
    /// Storage type is carried by place; this operation yields that type.
    Read {
        place: SourceValue,
    },
    Write {
        place: SourceValue,
        value: SourceValue,
    },
    /// Stack values are retained for address-taking (e.g. send's &val).
    LocalWrite {
        local: usize,
        value: SourceValue,
    },
    /// Signature is the source declaration's type, not an assumed API contract.
    Call {
        function: String,
        signature: String,
        arguments: Vec<SourceValue>,
        result_type: String,
    },
    Guard {
        condition: SourceValue,
        taken: bool,
    },
    LocalEnd {
        local: usize,
    },
    ReturnValue {
        value: SourceValue,
    },
    Return,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct MonkeySourcePath {
    pub operations: Vec<SourceOperation>,
}

/// Construction is private: every definition node must pass the importer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct MonkeySourceSummary {
    function: String,
    source_revision: String,
    signature: String,
    paths: Vec<MonkeySourcePath>,
}

impl MonkeySourceSummary {
    pub fn function(&self) -> &str {
        &self.function
    }
    pub fn source_revision(&self) -> &str {
        &self.source_revision
    }
    pub fn paths(&self) -> &[MonkeySourcePath] {
        &self.paths
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum MonkeySourceResult {
    Summary(MonkeySourceSummary),
    Unknown { function: String, reason: String },
}

/// Import the three exact callbacks, even though mk_event.c includes their
/// definitions from its selected adapter. Revision is a caller assertion; the
/// pinned external audit separately checks Git/source/build identity.
pub fn import_monkey_callbacks(
    unit: &C11TranslationUnit<'_>,
    source_revision: &str,
) -> Result<Vec<MonkeySourceResult>, String> {
    import_functions(unit, source_revision, &CALLBACKS)
}

/// Import every selected adapter wrapper with an explicit Unknown for bodies
/// outside the bounded grammar. Calls remain opaque; returned values and every
/// imported failure guard are retained, without assuming cleanup or success.
pub fn import_monkey_wrappers(
    unit: &C11TranslationUnit<'_>,
    source_revision: &str,
) -> Result<Vec<MonkeySourceResult>, String> {
    import_functions(unit, source_revision, &WRAPPERS)
}

fn import_functions(
    unit: &C11TranslationUnit<'_>,
    source_revision: &str,
    functions: &[&str],
) -> Result<Vec<MonkeySourceResult>, String> {
    if source_revision != PINNED_FLUENT_BIT_REVISION {
        return Err("Monkey source revision does not match the pinned revision".into());
    }
    let source = validate_unrestricted_c11(unit).map_err(|error| error.to_string())?;
    // Field IDs from independent Clang invocations are not stable. Validate the
    // selected complete records by name/type, then bind each MemberExpr to its
    // typed base and direct field (anonymous/union/bitfield access is rejected).
    let mut fields = BTreeMap::new();
    for record in ["mk_event", "mk_event_ctx", "ev_map", "mk_event_loop"] {
        let declarations = ast_declarations(&source, unit, record)?;
        let definitions: Vec<_> = declarations
            .iter()
            .filter(|node| {
                kind(node) == "RecordDecl"
                    && node["name"] == record
                    && node["completeDefinition"] == true
                    && node["tagUsed"] == "struct"
            })
            .collect();
        if let [definition] = definitions.as_slice() {
            for field in children(definition)
                .iter()
                .filter(|node| kind(node) == "FieldDecl")
            {
                if field["isBitfield"] != true
                    && children(field).is_empty()
                    && let (Some(name), Ok(ty)) = (field["name"].as_str(), c_type(field))
                {
                    fields.insert((format!("struct {record} *"), name.to_owned()), ty);
                }
            }
        }
    }
    Ok(functions
        .iter()
        .map(|function| {
            let result = function_ast(&source, unit, function)
                .and_then(|ast| derive(&ast, function, source_revision, &fields));
            match result {
                Ok(summary) => MonkeySourceResult::Summary(summary),
                Err(reason) => MonkeySourceResult::Unknown {
                    function: (*function).into(),
                    reason,
                },
            }
        })
        .collect())
}

fn c_type(node: &Value) -> Result<String, String> {
    let ty = node["type"]["desugaredQualType"]
        .as_str()
        .or_else(|| node["type"]["qualType"].as_str())
        .ok_or("missing C type")?;
    if ty.contains("volatile") || ty.contains("_Atomic") || ty.contains("restrict") {
        return Err("qualified access requires additional synchronization/alias evidence".into());
    }
    Ok(ty.into())
}

fn value(ty: String, expression: SourceExpression) -> SourceValue {
    SourceValue {
        c_type: ty,
        expression,
    }
}

#[derive(Clone)]
enum Expr {
    Local(String, String),
    Literal(SourceValue),
    Cast(String, String, Box<Self>),
    Unary(String, String, Box<Self>),
    Binary(String, String, Box<Self>, Box<Self>),
    Field(String, Box<Self>, String),
    Index(String, Box<Self>, Box<Self>),
    Read(String, Box<Self>),
    Call(String, String, String, Vec<Self>),
}

#[derive(Clone)]
enum Stmt {
    Block(Vec<Self>),
    Declare(String, String, Option<Expr>),
    Assign(Expr, Expr),
    OrAssign(Expr, Expr),
    Increment(Expr),
    Call(Expr),
    If(Expr, Box<Self>, Option<Box<Self>>),
    Return(Option<Expr>),
}

struct Importer<'a> {
    fields: &'a BTreeMap<(String, String), String>,
    locals: BTreeSet<String>,
    nodes: usize,
}

impl Importer<'_> {
    fn budget(&mut self, depth: usize) -> Result<(), String> {
        self.nodes += 1;
        if depth > 64 || self.nodes > 2048 {
            return Err("callback exceeds bounded source import size/depth".into());
        }
        Ok(())
    }

    fn expression(&mut self, node: &Value, depth: usize) -> Result<Expr, String> {
        self.budget(depth)?;
        let ty = c_type(node)?;
        let inner = children(node);
        match (kind(node), inner) {
            ("DeclRefExpr", [])
                if matches!(
                    node["referencedDecl"]["kind"].as_str(),
                    Some("VarDecl" | "ParmVarDecl")
                ) =>
            {
                let id = node["referencedDecl"]["id"]
                    .as_str()
                    .ok_or("missing local ID")?;
                if !self.locals.contains(id) {
                    return Err("global or unresolved local reference".into());
                }
                Ok(Expr::Local(id.into(), ty))
            }
            ("DeclRefExpr", []) if node["referencedDecl"]["kind"] == "FunctionDecl" => {
                Ok(Expr::Literal(value(
                    ty.clone(),
                    SourceExpression::Function {
                        name: node["referencedDecl"]["name"]
                            .as_str()
                            .ok_or("missing function identity")?
                            .into(),
                        signature: ty,
                    },
                )))
            }
            ("IntegerLiteral", []) | ("StringLiteral", []) => {
                let literal = node["value"].as_str().ok_or("missing literal")?.to_owned();
                Ok(Expr::Literal(value(
                    ty,
                    if kind(node) == "IntegerLiteral" {
                        SourceExpression::Integer { value: literal }
                    } else {
                        SourceExpression::String { value: literal }
                    },
                )))
            }
            ("UnaryExprOrTypeTraitExpr", []) if node["name"] == "sizeof" => {
                let argument_type = node["argType"]["desugaredQualType"]
                    .as_str()
                    .or_else(|| node["argType"]["qualType"].as_str())
                    .ok_or("unsupported sizeof operand")?;
                // Reject variably modified sizeof, whose operand can execute.
                if !matches!(
                    argument_type,
                    "unsigned long"
                        | "unsigned long long"
                        | "int"
                        | "short"
                        | "struct mk_event_ctx"
                        | "struct mk_event"
                        | "struct ev_map"
                ) {
                    return Err("sizeof outside fixed scalar type profile".into());
                }
                Ok(Expr::Literal(value(
                    ty,
                    SourceExpression::SizeOf {
                        argument_type: argument_type.into(),
                    },
                )))
            }
            ("ParenExpr", [child]) => self.expression(child, depth + 1),
            ("ImplicitCastExpr", [child]) if node["castKind"] == "LValueToRValue" => {
                Ok(Expr::Read(ty, Box::new(self.expression(child, depth + 1)?)))
            }
            ("ImplicitCastExpr" | "CStyleCastExpr", [child])
                if matches!(
                    node["castKind"].as_str(),
                    Some(
                        "BitCast"
                            | "IntegralCast"
                            | "NoOp"
                            | "ArrayToPointerDecay"
                            | "NullToPointer"
                            | "FunctionToPointerDecay"
                    )
                ) =>
            {
                Ok(Expr::Cast(
                    ty,
                    node["castKind"].as_str().unwrap().into(),
                    Box::new(self.expression(child, depth + 1)?),
                ))
            }
            ("MemberExpr", [base]) if node["isArrow"] == true => {
                let name = node["name"].as_str().ok_or("missing field name")?;
                if self.fields.get(&(c_type(base)?, name.into())) != Some(&ty) {
                    return Err("field is not a direct ordinary field of a selected record".into());
                }
                Ok(Expr::Field(
                    ty,
                    Box::new(self.expression(base, depth + 1)?),
                    name.into(),
                ))
            }
            ("ArraySubscriptExpr", [base, index]) => Ok(Expr::Index(
                ty,
                Box::new(self.expression(base, depth + 1)?),
                Box::new(self.expression(index, depth + 1)?),
            )),
            ("UnaryOperator", [child])
                if matches!(node["opcode"].as_str(), Some("&" | "*" | "-" | "!")) =>
            {
                Ok(Expr::Unary(
                    ty,
                    node["opcode"].as_str().unwrap().into(),
                    Box::new(self.expression(child, depth + 1)?),
                ))
            }
            ("BinaryOperator", [left, right])
                if matches!(
                    node["opcode"].as_str(),
                    Some("&" | "|" | "==" | "!=" | "<" | ">" | "+" | "*" | "/")
                ) =>
            {
                Ok(Expr::Binary(
                    ty,
                    node["opcode"].as_str().unwrap().into(),
                    Box::new(self.expression(left, depth + 1)?),
                    Box::new(self.expression(right, depth + 1)?),
                ))
            }
            ("CallExpr", [callee, arguments @ ..]) => {
                let [reference] = children(callee) else {
                    return Err("indirect or unmodeled call".into());
                };
                if kind(callee) != "ImplicitCastExpr"
                    || callee["castKind"] != "FunctionToPointerDecay"
                    || kind(reference) != "DeclRefExpr"
                    || reference["referencedDecl"]["kind"] != "FunctionDecl"
                {
                    return Err("indirect or unmodeled call".into());
                }
                let name = reference["referencedDecl"]["name"]
                    .as_str()
                    .ok_or("missing call name")?;
                // This list bounds syntax only. It does not assume these calls
                // succeed or have a particular custody/destruction effect.
                if !matches!(
                    name,
                    "send"
                        | "__errno_location"
                        | "evutil_socket_geterror"
                        | "perror"
                        | "evutil_closesocket"
                        | "event_del"
                        | "event_free"
                        | "mk_mem_free"
                        | "mk_mem_alloc_z"
                        | "event_base_new"
                        | "event_base_free"
                        | "event_new"
                        | "event_add"
                        | "event_base_loop"
                        | "evutil_socketpair"
                        | "_mk_event_init"
                        | "_mk_event_update"
                        | "_mk_event_add"
                        | "_mk_event_del"
                        | "mk_event_libevent_socketpair"
                        | "_mk_event_wait_with_flags"
                ) {
                    return Err(format!(
                        "call {name} is outside the callback source profile"
                    ));
                }
                Ok(Expr::Call(
                    ty,
                    name.into(),
                    c_type(reference)?,
                    arguments
                        .iter()
                        .map(|arg| self.expression(arg, depth + 1))
                        .collect::<Result<_, _>>()?,
                ))
            }
            _ => Err(format!("unmodeled callback expression {}", kind(node))),
        }
    }

    fn statement(&mut self, node: &Value, depth: usize) -> Result<Stmt, String> {
        self.budget(depth)?;
        match (kind(node), children(node)) {
            ("CompoundStmt", statements) => Ok(Stmt::Block(
                statements
                    .iter()
                    .map(|stmt| self.statement(stmt, depth + 1))
                    .collect::<Result<_, _>>()?,
            )),
            ("DeclStmt", [decl])
                if kind(decl) == "VarDecl" && decl.get("storageClass").is_none() =>
            {
                let id = decl["id"]
                    .as_str()
                    .ok_or("missing declaration ID")?
                    .to_owned();
                let ty = c_type(decl)?;
                // Fixed scalar and pointer locals only: no VLA or destructor attrs.
                if !(ty.ends_with('*')
                    || matches!(
                        ty.as_str(),
                        "int" | "short" | "unsigned int" | "unsigned long" | "unsigned long long"
                    ))
                {
                    return Err("local outside scalar/pointer callback profile".into());
                }
                let initial = match children(decl) {
                    [] => None,
                    [expr] => Some(self.expression(expr, depth + 1)?),
                    _ => return Err("unmodeled local initialization/attributes".into()),
                };
                self.locals.insert(id.clone());
                Ok(Stmt::Declare(id, ty, initial))
            }
            ("BinaryOperator", [left, right]) if node["opcode"] == "=" => Ok(Stmt::Assign(
                self.expression(left, depth + 1)?,
                self.expression(right, depth + 1)?,
            )),
            ("CompoundAssignOperator", [left, right]) if node["opcode"] == "|=" => {
                if c_type(left)? != "int" || c_type(right)? != "int" {
                    return Err("compound assignment requires int operands".into());
                }
                Ok(Stmt::OrAssign(
                    self.expression(left, depth + 1)?,
                    self.expression(right, depth + 1)?,
                ))
            }
            ("UnaryOperator", [place]) if node["opcode"] == "++" && c_type(node)? == "int" => {
                Ok(Stmt::Increment(self.expression(place, depth + 1)?))
            }
            ("IfStmt", [condition, yes]) if node["hasElse"] != true => Ok(Stmt::If(
                self.expression(condition, depth + 1)?,
                Box::new(self.statement(yes, depth + 1)?),
                None,
            )),
            ("IfStmt", [condition, yes, no]) if node["hasElse"] == true => Ok(Stmt::If(
                self.expression(condition, depth + 1)?,
                Box::new(self.statement(yes, depth + 1)?),
                Some(Box::new(self.statement(no, depth + 1)?)),
            )),
            ("CallExpr", _) => Ok(Stmt::Call(self.expression(node, depth + 1)?)),
            ("ReturnStmt", []) => Ok(Stmt::Return(None)),
            ("ReturnStmt", [result]) => Ok(Stmt::Return(Some(self.expression(result, depth + 1)?))),
            _ => Err(format!("unmodeled callback statement {}", kind(node))),
        }
    }
}

#[derive(Clone, Default)]
struct Path {
    locals: BTreeMap<String, (usize, String, Option<SourceValue>)>,
    addressed: BTreeSet<String>,
    operations: Vec<SourceOperation>,
    returned: bool,
    return_value: Option<SourceValue>,
    next_local: usize,
}

impl Path {
    fn push(&mut self, operation: SourceOperation) -> Result<usize, String> {
        if self.operations.len() >= MAX_OPERATIONS {
            return Err("callback operation limit exceeded".into());
        }
        let index = self.operations.len();
        self.operations.push(operation);
        Ok(index)
    }

    fn read(&mut self, expression: &Expr, ty: &str) -> Result<SourceValue, String> {
        if let Expr::Local(id, _) = expression {
            return self
                .locals
                .get(id)
                .and_then(|local| local.2.clone())
                .ok_or_else(|| "uninitialized, escaped, or out-of-scope local read".into());
        }
        let place = self.evaluate(expression)?;
        let operation = self.push(SourceOperation::Read { place })?;
        Ok(value(ty.into(), SourceExpression::Symbol { operation }))
    }

    fn evaluate(&mut self, expression: &Expr) -> Result<SourceValue, String> {
        let result = match expression {
            Expr::Local(id, ty) => {
                let local = self.locals.get(id).ok_or("local outside its scope")?;
                value(
                    ty.clone(),
                    SourceExpression::LocalStorage { local: local.0 },
                )
            }
            Expr::Literal(value) => value.clone(),
            Expr::Read(ty, place) => self.read(place, ty)?,
            Expr::Cast(ty, cast, child) => value(
                ty.clone(),
                SourceExpression::Cast {
                    cast: cast.clone(),
                    value: Box::new(self.evaluate(child)?),
                },
            ),
            Expr::Unary(ty, operator, child) => {
                if operator == "&"
                    && let Expr::Local(id, _) = child.as_ref()
                {
                    self.addressed.insert(id.clone());
                }
                value(
                    ty.clone(),
                    SourceExpression::Unary {
                        operator: operator.clone(),
                        value: Box::new(self.evaluate(child)?),
                    },
                )
            }
            Expr::Binary(ty, operator, left, right) => {
                reject_unsequenced(&[left, right])?;
                value(
                    ty.clone(),
                    SourceExpression::Binary {
                        operator: operator.clone(),
                        left: Box::new(self.evaluate(left)?),
                        right: Box::new(self.evaluate(right)?),
                    },
                )
            }
            Expr::Field(ty, base, name) => value(
                ty.clone(),
                SourceExpression::Field {
                    base: Box::new(self.evaluate(base)?),
                    name: name.clone(),
                },
            ),
            Expr::Index(ty, base, index) => {
                reject_unsequenced(&[base, index])?;
                value(
                    ty.clone(),
                    SourceExpression::Index {
                        base: Box::new(self.evaluate(base)?),
                        index: Box::new(self.evaluate(index)?),
                    },
                )
            }
            Expr::Call(ty, function, signature, arguments) => {
                reject_unsequenced(&arguments.iter().collect::<Vec<_>>())?;
                let arguments = arguments
                    .iter()
                    .map(|arg| self.evaluate(arg))
                    .collect::<Result<_, _>>()?;
                let operation = self.push(SourceOperation::Call {
                    function: function.clone(),
                    signature: signature.clone(),
                    arguments,
                    result_type: ty.clone(),
                })?;
                // Opaque calls can change any escaped local, even if the
                // callee's declaration uses const. Do not reuse its old value.
                self.invalidate_addressed();
                value(ty.clone(), SourceExpression::Symbol { operation })
            }
        };
        check_value_size(&result)?;
        Ok(result)
    }

    fn update(&mut self, place: &Expr, operator: &str, right: SourceValue) -> Result<(), String> {
        // C evaluates a compound-assignment lvalue only once. Preserve the
        // resolved address rather than introducing a second nested field read.
        let resolved = if matches!(place, Expr::Local(..)) {
            None
        } else {
            Some(self.evaluate(place)?)
        };
        let left = if let Some(place) = &resolved {
            let operation = self.push(SourceOperation::Read {
                place: place.clone(),
            })?;
            value("int".into(), SourceExpression::Symbol { operation })
        } else {
            self.read(place, "int")?
        };
        let result = value(
            "int".into(),
            SourceExpression::Binary {
                operator: operator.into(),
                left: Box::new(left),
                right: Box::new(right),
            },
        );
        check_value_size(&result)?;
        if let Some(place) = resolved {
            self.push(SourceOperation::Write {
                place,
                value: result,
            })?;
            self.invalidate_addressed();
            Ok(())
        } else {
            self.write(place, result)
        }
    }

    fn invalidate_addressed(&mut self) {
        for id in &self.addressed {
            if let Some(local) = self.locals.get_mut(id) {
                local.2 = None;
            }
        }
    }

    fn write(&mut self, place: &Expr, result: SourceValue) -> Result<(), String> {
        check_value_size(&result)?;
        if let Expr::Local(id, _) = place {
            let local = self
                .locals
                .get_mut(id)
                .ok_or("assignment outside local scope")?;
            local.2 = Some(result.clone());
            let index = local.0;
            self.push(SourceOperation::LocalWrite {
                local: index,
                value: result,
            })?;
        } else {
            let place = self.evaluate(place)?;
            self.push(SourceOperation::Write {
                place,
                value: result,
            })?;
            // An indirect write may alias any address-taken local.
            self.invalidate_addressed();
        }
        Ok(())
    }
}

// Bound alias substitution as well as the input AST: repeated x = x | x
// must not allocate an exponentially growing symbolic report.
fn visit_value(
    value: &SourceValue,
    mut visit: impl FnMut(&SourceValue) -> Result<(), String>,
) -> Result<(), String> {
    let mut pending = vec![(value, 0)];
    let mut count = 0;
    while let Some((value, depth)) = pending.pop() {
        count += 1;
        if count > 2048 || depth > 128 {
            return Err("symbolic expression limit exceeded".into());
        }
        visit(value)?;
        match &value.expression {
            SourceExpression::Cast { value, .. } | SourceExpression::Unary { value, .. } => {
                pending.push((value, depth + 1))
            }
            SourceExpression::Field { base, .. } => pending.push((base, depth + 1)),
            SourceExpression::Binary { left, right, .. } => {
                pending.push((left, depth + 1));
                pending.push((right, depth + 1));
            }
            SourceExpression::Index { base, index } => {
                pending.push((base, depth + 1));
                pending.push((index, depth + 1));
            }
            _ => {}
        }
    }
    Ok(())
}

fn check_value_size(value: &SourceValue) -> Result<(), String> {
    visit_value(value, |_| Ok(()))
}

// Validate the symbolic-reference projection independently of construction.
// This establishes only prior typed producers, not memory/alias/call safety.
fn check_path_symbols(path: &MonkeySourcePath) -> Result<(), String> {
    let mut producers: Vec<Option<&str>> = Vec::new();
    for operation in &path.operations {
        let (values, produced): (Vec<&SourceValue>, Option<&str>) = match operation {
            SourceOperation::Read { place } => (vec![place], Some(&place.c_type)),
            SourceOperation::Write { place, value } => (vec![place, value], None),
            SourceOperation::LocalWrite { value, .. } | SourceOperation::ReturnValue { value } => {
                (vec![value], None)
            }
            SourceOperation::Call {
                arguments,
                result_type,
                ..
            } => (arguments.iter().collect(), Some(result_type)),
            SourceOperation::Guard { condition, .. } => (vec![condition], None),
            SourceOperation::LocalEnd { .. } | SourceOperation::Return => (vec![], None),
        };
        for value in values {
            visit_value(value, |value| {
                if let SourceExpression::Symbol { operation } = value.expression
                    && producers.get(operation).copied().flatten() != Some(value.c_type.as_str())
                {
                    return Err("symbol has no prior producer with its exact C type".into());
                }
                Ok(())
            })?;
        }
        producers.push(produced);
    }
    Ok(())
}

// C does not sequence operands/arguments. Pure ordinary reads are projected in
// AST order under the exclusive-access boundary; an opaque call plus another
// read/call cannot be assigned an invented order.
fn effects(expr: &Expr) -> (bool, bool) {
    let combine = |a: (bool, bool), b: (bool, bool)| (a.0 || b.0, a.1 || b.1);
    match expr {
        Expr::Local(..) | Expr::Literal(..) => (false, false),
        Expr::Read(_, child) => combine((true, false), effects(child)),
        Expr::Cast(_, _, child) | Expr::Unary(_, _, child) | Expr::Field(_, child, _) => {
            effects(child)
        }
        Expr::Binary(_, _, left, right) | Expr::Index(_, left, right) => {
            combine(effects(left), effects(right))
        }
        Expr::Call(..) => (true, true),
    }
}

fn reject_unsequenced(expressions: &[&Expr]) -> Result<(), String> {
    let effects: Vec<_> = expressions.iter().map(|expr| effects(expr)).collect();
    if effects.iter().any(|effect| effect.1) && effects.iter().filter(|effect| effect.0).count() > 1
    {
        Err("unsequenced call/read operands require an explicit ordering model".into())
    } else {
        Ok(())
    }
}

fn execute(statement: &Stmt, paths: Vec<Path>) -> Result<Vec<Path>, String> {
    if let Stmt::Block(statements) = statement {
        let declared: Vec<_> = statements
            .iter()
            .filter_map(|statement| {
                if let Stmt::Declare(id, _, _) = statement {
                    Some(id.clone())
                } else {
                    None
                }
            })
            .collect();
        let mut paths = paths;
        for statement in statements {
            paths = execute(statement, paths)?;
        }
        for path in &mut paths {
            // Remove block storage even on an early return. The final Return
            // operation is emitted after these lexical lifetime exits.
            for id in declared.iter().rev() {
                if let Some((local, _, _)) = path.locals.remove(id) {
                    path.push(SourceOperation::LocalEnd { local })?;
                }
            }
        }
        return Ok(paths);
    }
    let mut output = Vec::new();
    for mut path in paths {
        if path.returned {
            output.push(path);
            continue;
        }
        match statement {
            Stmt::Declare(id, ty, initial) => {
                let index = path.next_local;
                path.next_local += 1;
                let result = initial
                    .as_ref()
                    .map(|expr| path.evaluate(expr))
                    .transpose()?;
                path.locals
                    .insert(id.clone(), (index, ty.clone(), result.clone()));
                if let Some(value) = result {
                    path.push(SourceOperation::LocalWrite {
                        local: index,
                        value,
                    })?;
                }
            }
            Stmt::Assign(place, expr) => {
                // A direct field base read from an unescaped pointer local
                // cannot be changed by the RHS call. Other addresses retain
                // the conservative call/read ordering gate.
                let stable_base = matches!(place, Expr::Field(_, base, _) if matches!(base.as_ref(), Expr::Read(_, local) if matches!(local.as_ref(), Expr::Local(id, _) if !path.addressed.contains(id))));
                if !stable_base {
                    reject_unsequenced(&[place, expr])?;
                }
                let result = path.evaluate(expr)?;
                path.write(place, result)?;
            }
            Stmt::OrAssign(place, expr) => {
                reject_unsequenced(&[place, expr])?;
                // Include the old lvalue read in ordering checks: an RHS call
                // could change an address-taken local before it is read.
                if effects(expr).1 {
                    return Err("compound assignment with an opaque call".into());
                }
                let right = path.evaluate(expr)?;
                path.update(place, "|", right)?;
            }
            Stmt::Increment(place) => {
                if effects(place).1 {
                    return Err("increment with an effectful address".into());
                }
                path.update(
                    place,
                    "+",
                    value(
                        "int".into(),
                        SourceExpression::Integer { value: "1".into() },
                    ),
                )?;
            }
            Stmt::Call(expr) => {
                path.evaluate(expr)?;
            }
            Stmt::If(condition, yes, no) => {
                let condition = path.evaluate(condition)?;
                let mut other = path.clone();
                path.push(SourceOperation::Guard {
                    condition: condition.clone(),
                    taken: true,
                })?;
                other.push(SourceOperation::Guard {
                    condition,
                    taken: false,
                })?;
                output.extend(execute(yes, vec![path])?);
                output.extend(if let Some(no) = no {
                    execute(no, vec![other])?
                } else {
                    vec![other]
                });
                if output.len() > MAX_PATHS {
                    return Err("callback path limit exceeded".into());
                }
                continue;
            }
            Stmt::Return(result) => {
                path.return_value = result
                    .as_ref()
                    .map(|value| path.evaluate(value))
                    .transpose()?;
                path.returned = true;
            }
            Stmt::Block(_) => unreachable!(),
        }
        output.push(path);
        if output.len() > MAX_PATHS {
            return Err("callback path limit exceeded".into());
        }
    }
    Ok(output)
}

fn derive(
    ast: &Value,
    function: &str,
    revision: &str,
    fields: &BTreeMap<(String, String), String>,
) -> Result<MonkeySourceSummary, String> {
    let callback = CALLBACKS.contains(&function);
    let signature = c_type(ast)?;
    let return_type = signature
        .split('(')
        .next()
        .ok_or("missing return type")?
        .trim();
    if ast["variadic"] == true || (callback && return_type != "void") {
        return Err("function signature is outside the selected Unix profile".into());
    }
    if !matches!(return_type, "void" | "void *" | "int" | "char *") {
        return Err("wrapper return type outside bounded profile".into());
    }
    let Some((body, parameters)) = children(ast).split_last() else {
        return Err("missing function body".into());
    };
    if kind(body) != "CompoundStmt" || (callback && parameters.len() != 3) {
        return Err("unmodeled function parameters/attributes".into());
    }
    let mut importer = Importer {
        fields,
        locals: BTreeSet::new(),
        nodes: 0,
    };
    let mut path = Path {
        next_local: parameters.len(),
        ..Path::default()
    };
    for (index, parameter) in parameters.iter().enumerate() {
        if kind(parameter) != "ParmVarDecl" || !children(parameter).is_empty() {
            return Err("unmodeled callback parameter".into());
        }
        let id = parameter["id"]
            .as_str()
            .ok_or("missing parameter ID")?
            .to_owned();
        let ty = c_type(parameter)?;
        if (callback && ty != ["int", "short", "void *"][index])
            || (!callback
                && !(ty.ends_with('*') || matches!(ty.as_str(), "int" | "unsigned int" | "long")))
        {
            return Err("callback parameter type outside selected Unix profile".into());
        }
        importer.locals.insert(id.clone());
        path.locals.insert(
            id,
            (
                index,
                ty.clone(),
                Some(value(ty, SourceExpression::Parameter { index })),
            ),
        );
    }
    // Parse all statements before executing any path; unreachable unsupported
    // operations still make the entire callback Unknown.
    let statement = importer.statement(body, 0)?;
    let paths = execute(&statement, vec![path])?
        .into_iter()
        .map(|mut path| {
            if return_type == "void" {
                if path.return_value.is_some() {
                    return Err("value returned from void function".into());
                }
                path.push(SourceOperation::Return)?;
            } else {
                let value = path
                    .return_value
                    .take()
                    .ok_or("missing nonvoid return on a source path")?;
                if value.c_type != return_type {
                    return Err("returned value type differs from signature".into());
                }
                path.push(SourceOperation::ReturnValue { value })?;
            }
            let path = MonkeySourcePath {
                operations: path.operations,
            };
            check_path_symbols(&path)?;
            Ok(path)
        })
        .collect::<Result<_, String>>()?;
    Ok(MonkeySourceSummary {
        function: function.into(),
        source_revision: revision.into(),
        signature,
        paths,
    })
}

#[cfg(test)]
#[path = "../../../tests/unit/monkey_source.rs"]
mod tests;
