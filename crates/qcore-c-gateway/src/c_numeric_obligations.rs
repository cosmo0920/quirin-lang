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

//! The C integer arithmetic property (card **CG20-NUMERIC-DOMAIN**, gateway
//! rule 11q): every integer operation and conversion of a function body, with
//! the fault cases of [`crate::c_numeric`] as obligations.
//!
//! This is a **new property beside custody**, published separately: a
//! function's custody verdict neither reads nor changes this one, and this
//! verdict is never folded into the custody denominator. Its scope:
//!
//! * **In.** The function's own integer `+ - * / % << >> & | ^`, unary `- ~ +`,
//!   `++`/`--`, compound assignments, every integer conversion Clang
//!   recorded (implicit or written), bit-field stores, and the
//!   integer/floating/pointer conversions' range obligations.
//! * **Out, by statement.** Pointer arithmetic and subscripts (the bounds
//!   property, CG16), floating arithmetic itself (N09/N10, guarantee-gated),
//!   and a callee's own behaviour (a call is a value of its result type).
//!   Floating arithmetic makes the verdict `Unknown` by name rather than
//!   being skipped.
//!
//! The walker reads Clang's typed AST for the unit, with the unit's frontend
//! flags and the declared target (see [`NumericTarget`]); every width comes
//! from the [`TargetIntegerModel`] Clang reports for the same flags, and
//! every conversion is the one Clang recorded. A value is an [`Interval`]
//! within its type: a literal or Clang-evaluated constant is a point, an
//! enumerator a point when Clang evaluated it, anything read from storage
//! or a call is its type's full range. No flow, path or loop fact is used:
//! switch-value pruning and branch correlation (C04, C05) are not done.
//!
//! **Verdict.** `Safe` only when every obligation of every site is
//! [`Status::Discharged`] and nothing was refused. A [`Status::Violated`]
//! obligation is a definite fault *if its site is reached*; reachability is
//! not decided here, so it makes the function `Unknown` with the site named,
//! never `Unsafe`. Implementation-defined cases are obligations too (no
//! implementation premise is declared).

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::process::Stdio;

use serde::Serialize;
use serde_json::Value;

use crate::c_numeric::{
    self, BinOp, CType, Fault, IntKind, Interval, Obligation, Status, TargetIntegerModel, UnOp,
};
use crate::legacy_c::C11TranslationUnit;

/// The target the property is decided for. `Default` is the target the
/// analysing Clang uses for the unit's own flags (its default triple, or the
/// one an `-arch` flag selects); `Declared` adds `--target=TRIPLE` to both
/// the model query and the AST run, and is a declaration of this property
/// only (the custody gateway transports no `--target`; CG30).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumericTarget<'a> {
    Default,
    Declared(&'a str),
}

impl NumericTarget<'_> {
    fn flag(self) -> Option<String> {
        match self {
            Self::Default => None,
            Self::Declared(triple) => Some(format!("--target={triple}")),
        }
    }
}

/// The property's verdict for one function. There is no `Unsafe`: a definite
/// fault is reported as a violated obligation of an `Unknown` function.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum NumericVerdict {
    Safe,
    Unknown,
}

/// One operation or conversion and its obligations.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Site {
    pub line: Option<u64>,
    /// `binary +`, `compound <<=`, `unary -`, `increment`, `conversion`,
    /// `bit-field store`, `cast FloatingToIntegral`, ….
    pub operation: String,
    /// The computation (or target) type, as Clang spells it.
    pub c_type: String,
    pub obligations: Vec<Obligation>,
}

/// A defined but value-changing event: an unsigned wrap that may happen, or
/// a comparison whose operand a usual conversion may change (`u < -1`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DefinedEvent {
    pub line: Option<u64>,
    pub kind: &'static str,
    pub detail: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FunctionNumeric {
    pub name: String,
    pub verdict: NumericVerdict,
    pub sites: Vec<Site>,
    pub events: Vec<DefinedEvent>,
    /// Named refusals; any makes the verdict `Unknown`.
    pub refused: Vec<String>,
    /// Pointer-arithmetic sites left to the bounds property (not obligations
    /// of this one; counted so the scope is visible).
    pub outside: usize,
}

impl FunctionNumeric {
    /// Every obligation with the given status.
    pub fn with_status(&self, status: Status) -> Vec<(&Site, Fault)> {
        self.sites
            .iter()
            .flat_map(|site| {
                site.obligations
                    .iter()
                    .filter(move |o| o.status == status)
                    .map(move |o| (site, o.fault))
            })
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct NumericReport {
    pub target: TargetIntegerModel,
    pub data_model: c_numeric::DataModel,
    pub functions: Vec<FunctionNumeric>,
}

impl NumericReport {
    pub fn function(&self, name: &str) -> Option<&FunctionNumeric> {
        self.functions.iter().find(|function| function.name == name)
    }
}

/// Query the target model the property uses for this unit and target.
pub fn target_model(
    unit: &C11TranslationUnit<'_>,
    target: NumericTarget<'_>,
) -> Result<TargetIntegerModel, String> {
    let mut command = crate::clang::command()?;
    unit.append_frontend_flags(&mut command);
    let declared = match target {
        NumericTarget::Default => None,
        NumericTarget::Declared(triple) => Some(triple),
    };
    TargetIntegerModel::query(command, declared)
}

/// Decide the property for every function defined in the unit.
pub fn analyze_numeric_obligations(
    unit: &C11TranslationUnit<'_>,
    target: NumericTarget<'_>,
) -> Result<NumericReport, String> {
    let model = target_model(unit, target)?;
    let ast = typed_ast(unit, target)?;
    Ok(report(model, &ast))
}

/// Decide the property over an AST already read for `model`'s target.
pub fn report(model: TargetIntegerModel, ast: &Value) -> NumericReport {
    let mut tables = Tables::default();
    tables.collect(ast);
    let mut functions = Vec::new();
    let mut line = None;
    for declaration in ast["inner"].as_array().into_iter().flatten() {
        track_line(declaration, &mut line);
        if declaration["kind"] != "FunctionDecl" {
            continue;
        }
        let Some(body) = declaration["inner"]
            .as_array()
            .and_then(|inner| inner.iter().find(|node| node["kind"] == "CompoundStmt"))
        else {
            continue;
        };
        let mut walker = Walker {
            model: &model,
            tables: &tables,
            line,
            sites: Vec::new(),
            events: Vec::new(),
            refused: BTreeSet::new(),
            outside: 0,
        };
        walker.value(body);
        line = walker.line;
        let verdict = if walker.refused.is_empty()
            && walker.sites.iter().all(|site| {
                site.obligations
                    .iter()
                    .all(|o| o.status == Status::Discharged)
            }) {
            NumericVerdict::Safe
        } else {
            NumericVerdict::Unknown
        };
        functions.push(FunctionNumeric {
            name: declaration["name"].as_str().unwrap_or_default().to_owned(),
            verdict,
            sites: walker.sites,
            events: walker.events,
            refused: walker.refused.into_iter().collect(),
            outside: walker.outside,
        });
    }
    NumericReport {
        data_model: model.data_model(),
        target: model,
        functions,
    }
}

/// Clang's typed AST of the unit for the property's target (raw transport).
pub fn typed_ast(
    unit: &C11TranslationUnit<'_>,
    target: NumericTarget<'_>,
) -> Result<Value, String> {
    let mut command = crate::clang::command()?;
    unit.append_frontend_flags(&mut command);
    if let Some(flag) = target.flag() {
        command.arg(flag);
    }
    let dialect = unit.dialect.clang_invocation_argument()?;
    let mut child = command
        .args([
            "-x",
            "c",
            dialect,
            "-fsyntax-only",
            "-Xclang",
            "-ast-dump=json",
            "-",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("Clang AST unavailable: {error}"))?;
    let mut stdin = child.stdin.take().ok_or("Clang stdin unavailable")?;
    let source = unit.source;
    let output = std::thread::scope(|scope| {
        let writer = scope.spawn(move || stdin.write_all(source.as_bytes()));
        let output = child.wait_with_output();
        writer
            .join()
            .map_err(|_| "Clang input writer panicked".to_owned())?
            .map_err(|error| format!("Clang input failed: {error}"))?;
        output.map_err(|error| format!("Clang AST failed: {error}"))
    })?;
    if !output.status.success() {
        return Err(format!(
            "Clang AST rejected source: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("unsupported Clang AST JSON: {error}"))
}

/// Unit-wide facts: Clang-evaluated enumerator values and bit-field members.
#[derive(Default)]
struct Tables {
    enumerators: BTreeMap<String, i128>,
    bit_fields: BTreeMap<String, String>,
}

impl Tables {
    fn collect(&mut self, node: &Value) {
        match node["kind"].as_str() {
            Some("EnumConstantDecl") => {
                // Only an enumerator Clang evaluated (a `ConstantExpr` with a
                // value) is a point; an implicit one stays unknown.
                let value = node["inner"]
                    .as_array()
                    .and_then(|inner| inner.first())
                    .filter(|first| first["kind"] == "ConstantExpr")
                    .and_then(|first| first["value"].as_str())
                    .and_then(|value| value.parse::<i128>().ok());
                if let (Some(id), Some(value)) = (node["id"].as_str(), value) {
                    self.enumerators.insert(id.to_owned(), value);
                }
            }
            Some("FieldDecl") if node["isBitfield"] == true => {
                if let Some(id) = node["id"].as_str() {
                    self.bit_fields
                        .insert(id.to_owned(), spelling(node).to_owned());
                }
            }
            _ => {}
        }
        for child in node["inner"].as_array().into_iter().flatten() {
            self.collect(child);
        }
    }
}

/// Clang's canonical spelling of a node's type.
fn spelling(node: &Value) -> &str {
    node["type"]["desugaredQualType"]
        .as_str()
        .or_else(|| node["type"]["qualType"].as_str())
        .unwrap_or_default()
}

fn is_pointer(spelling: &str) -> bool {
    spelling.ends_with('*') || spelling.contains("(*)") || spelling.ends_with(']')
}

/// Update the current line from a node's locations, in the order Clang prints
/// them (it omits a line equal to the previous one).
fn track_line(node: &Value, line: &mut Option<u64>) {
    for location in [&node["loc"], &node["range"]["begin"], &node["range"]["end"]] {
        let location = if location["expansionLoc"].is_object() {
            &location["expansionLoc"]
        } else {
            location
        };
        if let Some(found) = location["line"].as_u64() {
            *line = Some(found);
        }
    }
}

/// The mathematical range of an unsigned `+ - * <<` before C's wrap, for the
/// defined-wrap event only (never an obligation).
fn exact_interval(op: BinOp, a: Interval, b: Interval) -> Option<Interval> {
    let corners = |f: fn(i128, i128) -> Option<i128>| -> Option<Interval> {
        let values = [
            f(a.lo, b.lo)?,
            f(a.lo, b.hi)?,
            f(a.hi, b.lo)?,
            f(a.hi, b.hi)?,
        ];
        Some(Interval {
            lo: *values.iter().min()?,
            hi: *values.iter().max()?,
        })
    };
    match op {
        BinOp::Add => corners(i128::checked_add),
        BinOp::Sub => corners(i128::checked_sub),
        BinOp::Mul => corners(i128::checked_mul),
        BinOp::Shl if a.lo >= 0 && b.lo >= 0 && b.hi < 64 => Some(Interval {
            lo: a.lo << b.lo,
            hi: a.hi << b.hi,
        }),
        _ => None,
    }
}

struct Walker<'a> {
    model: &'a TargetIntegerModel,
    tables: &'a Tables,
    line: Option<u64>,
    sites: Vec<Site>,
    events: Vec<DefinedEvent>,
    refused: BTreeSet<String>,
    outside: usize,
}

/// Cast kinds that perform no integer or floating value conversion.
const NEUTRAL_CASTS: [&str; 14] = [
    "LValueToRValue",
    "NoOp",
    "BitCast",
    "ArrayToPointerDecay",
    "FunctionToPointerDecay",
    "NullToPointer",
    "ToVoid",
    "BuiltinFnToFnPtr",
    "AtomicToNonAtomic",
    "NonAtomicToAtomic",
    "PointerToBoolean",
    "FloatingToBoolean",
    "IntegralToBoolean",
    "LValueBitCast",
];

impl Walker<'_> {
    fn integer(&self, node: &Value) -> Option<(IntKind, CType)> {
        IntKind::parse(spelling(node)).map(|kind| (kind, self.model.ctype(kind)))
    }

    fn site(&mut self, operation: String, node: &Value, obligations: Vec<Obligation>) {
        if !obligations.is_empty() {
            self.sites.push(Site {
                line: self.line,
                operation,
                c_type: spelling(node).to_owned(),
                obligations,
            });
        }
    }

    fn open(&mut self, operation: String, node: &Value, fault: Fault) {
        self.site(
            operation,
            node,
            vec![Obligation {
                fault,
                status: Status::Open,
            }],
        );
    }

    fn event(&mut self, kind: &'static str, detail: String) {
        self.events.push(DefinedEvent {
            line: self.line,
            kind,
            detail,
        });
    }

    fn children(&mut self, node: &Value) -> Vec<Option<Interval>> {
        node["inner"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|child| self.value(child))
            .collect()
    }

    /// The full range of an integer-typed node, or `None`.
    fn full(&self, node: &Value) -> Option<Interval> {
        self.integer(node).map(|(_, ty)| ty.full())
    }

    /// Visit a node, record its sites, and return its value range if it is
    /// an integer expression.
    fn value(&mut self, node: &Value) -> Option<Interval> {
        track_line(node, &mut self.line);
        let kind = node["kind"].as_str().unwrap_or_default();
        match kind {
            "IntegerLiteral" | "CharacterLiteral" => {
                let value = node["value"]
                    .as_str()
                    .and_then(|v| v.parse::<i128>().ok())
                    .or_else(|| node["value"].as_i64().map(i128::from));
                let (_, ty) = self.integer(node)?;
                match value {
                    Some(value) if ty.contains(value) => Some(Interval::point(value)),
                    _ => Some(ty.full()),
                }
            }
            "ConstantExpr" => {
                let inner = self.children(node).into_iter().next().flatten();
                let value = node["value"].as_str().and_then(|v| v.parse::<i128>().ok());
                match (self.integer(node), value) {
                    (Some((_, ty)), Some(value)) if ty.contains(value) => {
                        Some(Interval::point(value))
                    }
                    _ => inner.or_else(|| self.full(node)),
                }
            }
            "ParenExpr" => self.children(node).into_iter().next().flatten(),
            "DeclRefExpr" => {
                let decl = &node["referencedDecl"];
                if decl["kind"] == "EnumConstantDecl"
                    && let Some(value) = decl["id"]
                        .as_str()
                        .and_then(|id| self.tables.enumerators.get(id))
                {
                    return Some(Interval::point(*value));
                }
                self.full(node)
            }
            "ImplicitCastExpr" | "CStyleCastExpr" => self.cast(node),
            "BinaryOperator" => self.binary(node),
            "CompoundAssignOperator" => self.compound(node),
            "UnaryOperator" => self.unary(node),
            "ConditionalOperator" => {
                let values = self.children(node);
                match (
                    values.get(1).copied().flatten(),
                    values.get(2).copied().flatten(),
                ) {
                    (Some(a), Some(b)) => Some(a.hull(b)),
                    _ => self.full(node),
                }
            }
            _ => {
                self.children(node);
                self.full(node)
            }
        }
    }

    fn cast(&mut self, node: &Value) -> Option<Interval> {
        let cast = node["castKind"].as_str().unwrap_or_default().to_owned();
        let child = node["inner"].as_array().and_then(|inner| inner.first())?;
        let value = self.value(child);
        let to = spelling(node).to_owned();
        let from = spelling(child).to_owned();
        match cast.as_str() {
            "LValueToRValue" | "NoOp" => value.or_else(|| self.full(node)),
            "IntegralCast" => {
                let (Some((_, from_ty)), Some((to_kind, to_ty))) =
                    (self.integer(child), self.integer(node))
                else {
                    self.refused.insert(format!(
                        "integral conversion `{from}` -> `{to}` outside the standard integer types"
                    ));
                    return self.full(node);
                };
                let a = value.unwrap_or(from_ty.full());
                let (result, obligation, changes) =
                    c_numeric::convert_interval(to_ty, a, to_kind == IntKind::Bool);
                if let Some(obligation) = obligation {
                    self.site(format!("conversion {from} -> {to}"), node, vec![obligation]);
                }
                if changes && !to_ty.signed {
                    self.event("wrapped-conversion", format!("{from} -> {to}"));
                }
                Some(result)
            }
            "IntegralToBoolean" | "FloatingToBoolean" | "PointerToBoolean" => {
                Some(Interval { lo: 0, hi: 1 })
            }
            "FloatingToIntegral" => {
                self.open(
                    format!("cast {cast} {from} -> {to}"),
                    node,
                    Fault::FloatingToInteger,
                );
                self.full(node)
            }
            "IntegralToFloating" => {
                let precision = self.model.floating_precision(&to);
                match (self.integer(child), precision) {
                    (Some((_, from_ty)), Some(precision)) => {
                        let a = value.unwrap_or(from_ty.full());
                        let obligation = c_numeric::integer_to_floating(a, precision);
                        self.site(
                            format!("cast {cast} {from} -> {to}"),
                            node,
                            vec![obligation],
                        );
                    }
                    _ => {
                        self.refused
                            .insert(format!("integer-to-floating conversion `{from}` -> `{to}`"));
                    }
                }
                None
            }
            "FloatingCast" => {
                match (
                    self.model.floating_precision(&from),
                    self.model.floating_precision(&to),
                ) {
                    (Some(a), Some(b)) if b >= a => {}
                    (Some(_), Some(_)) => self.open(
                        format!("cast {cast} {from} -> {to}"),
                        node,
                        Fault::FloatingNarrowing,
                    ),
                    _ => {
                        self.refused
                            .insert(format!("floating conversion `{from}` -> `{to}`"));
                    }
                }
                None
            }
            "PointerToIntegral" => {
                self.open(
                    format!("cast {cast} -> {to}"),
                    node,
                    Fault::PointerToInteger,
                );
                self.full(node)
            }
            "IntegralToPointer" => {
                self.open(
                    format!("cast {cast} {from} ->"),
                    node,
                    Fault::IntegerToPointer,
                );
                None
            }
            kind if NEUTRAL_CASTS.contains(&kind) => self.full(node),
            other => {
                self.refused.insert(format!("cast kind {other}"));
                self.full(node)
            }
        }
    }

    fn binary(&mut self, node: &Value) -> Option<Interval> {
        let opcode = node["opcode"].as_str().unwrap_or_default().to_owned();
        let inner = node["inner"].as_array().cloned().unwrap_or_default();
        let (Some(left), Some(right)) = (inner.first(), inner.get(1)) else {
            self.refused
                .insert(format!("binary {opcode} without two operands"));
            return None;
        };
        if matches!(opcode.as_str(), "<" | ">" | "<=" | ">=" | "==" | "!=") {
            for operand in [left, right] {
                self.mixed_signedness(&opcode, operand);
            }
        }
        let a = self.value(left);
        let b = self.value(right);
        match opcode.as_str() {
            "<" | ">" | "<=" | ">=" | "==" | "!=" => match (
                a.and_then(Interval::constant),
                b.and_then(Interval::constant),
                self.integer(left),
            ) {
                (Some(x), Some(y), Some((_, ty))) => c_numeric::compare(&opcode, ty, x, y)
                    .map(|truth| Interval::point(i128::from(truth))),
                _ => Some(Interval { lo: 0, hi: 1 }),
            }
            .or(Some(Interval { lo: 0, hi: 1 })),
            "&&" | "||" => Some(Interval { lo: 0, hi: 1 }),
            "," => b,
            "=" => {
                self.bit_field_store(left, &format!("assignment {}", spelling(left)));
                b
            }
            _ => {
                let Some(op) = BinOp::parse(&opcode) else {
                    self.refused.insert(format!("binary operator {opcode}"));
                    return self.full(node);
                };
                let result = spelling(node);
                if is_pointer(spelling(left)) || is_pointer(spelling(right)) || is_pointer(result) {
                    self.outside += 1;
                    return self.full(node);
                }
                if self.model.floating_precision(result).is_some() {
                    self.refused
                        .insert(format!("floating arithmetic `{opcode}` (N09/N10)"));
                    return None;
                }
                let (Some((_, ty)), Some((_, left_ty)), Some((_, right_ty))) =
                    (self.integer(node), self.integer(left), self.integer(right))
                else {
                    self.refused.insert(format!(
                        "binary {opcode} on `{}`, `{}` -> `{result}`",
                        spelling(left),
                        spelling(right)
                    ));
                    return self.full(node);
                };
                let shift = matches!(op, BinOp::Shl | BinOp::Shr);
                if left_ty != ty || (!shift && right_ty != ty) {
                    self.refused.insert(format!(
                        "binary {opcode}: operand types differ from the computation type `{result}`"
                    ));
                    return self.full(node);
                }
                let (value, obligations) = c_numeric::binary_interval(
                    op,
                    ty,
                    a.unwrap_or(ty.full()),
                    b.unwrap_or(right_ty.full()),
                );
                if !ty.signed && matches!(op, BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Shl) {
                    let exact = a.zip(b).and_then(|(a, b)| exact_interval(op, a, b));
                    if exact.is_none_or(|exact| !exact.within(ty)) {
                        self.event("unsigned-wrap", format!("{opcode} at `{result}`"));
                    }
                }
                self.site(format!("binary {opcode}"), node, obligations);
                Some(value)
            }
        }
    }

    /// N07: a comparison operand a usual conversion makes unsigned from a
    /// signed type (`u < -1`) is a defined, value-changing event.
    fn mixed_signedness(&mut self, opcode: &str, operand: &Value) {
        if operand["kind"] != "ImplicitCastExpr" || operand["castKind"] != "IntegralCast" {
            return;
        }
        let Some(child) = operand["inner"].as_array().and_then(|inner| inner.first()) else {
            return;
        };
        if let (Some((_, from)), Some((_, to))) = (self.integer(child), self.integer(operand))
            && from.signed
            && !to.signed
        {
            self.event(
                "mixed-signedness-comparison",
                format!(
                    "`{opcode}`: `{}` converted to `{}`",
                    spelling(child),
                    spelling(operand)
                ),
            );
        }
    }

    /// A store to a bit-field narrows to the field's width: defined for an
    /// unsigned field, implementation-defined for a signed one (an open
    /// obligation, since the stored range is not tracked against the width).
    fn bit_field_store(&mut self, place: &Value, operation: &str) {
        let mut place = place;
        while place["kind"] == "ParenExpr" {
            match place["inner"].as_array().and_then(|inner| inner.first()) {
                Some(inner) => place = inner,
                None => return,
            }
        }
        if place["kind"] != "MemberExpr" {
            return;
        }
        let Some(field) = place["referencedMemberDecl"]
            .as_str()
            .and_then(|id| self.tables.bit_fields.get(id))
        else {
            return;
        };
        match IntKind::parse(field).map(|kind| self.model.ctype(kind)) {
            Some(ty) if !ty.signed => {}
            Some(_) => self.open(
                format!("bit-field store ({operation})"),
                place,
                Fault::SignedConversion,
            ),
            None => {
                self.refused.insert(format!("bit-field of type `{field}`"));
            }
        }
    }

    fn compound(&mut self, node: &Value) -> Option<Interval> {
        let opcode = node["opcode"].as_str().unwrap_or_default().to_owned();
        let inner = node["inner"].as_array().cloned().unwrap_or_default();
        let (Some(left), Some(right)) = (inner.first(), inner.get(1)) else {
            self.refused
                .insert(format!("compound {opcode} without two operands"));
            return None;
        };
        self.value(left);
        let b = self.value(right);
        let place = spelling(left).to_owned();
        let compute = |key: &str| {
            node[key]["desugaredQualType"]
                .as_str()
                .or_else(|| node[key]["qualType"].as_str())
                .unwrap_or_default()
                .to_owned()
        };
        let (lhs, result) = (compute("computeLHSType"), compute("computeResultType"));
        if is_pointer(&place) {
            self.outside += 1;
            return self.full(node);
        }
        let Some(op) = BinOp::parse(&opcode) else {
            self.refused.insert(format!("compound operator {opcode}"));
            return self.full(node);
        };
        if self.model.floating_precision(&result).is_some() {
            self.refused
                .insert(format!("floating compound `{opcode}` (N09/N10)"));
            return None;
        }
        let parse = |text: &str| IntKind::parse(text).map(|kind| (kind, self.model.ctype(kind)));
        let (Some((_, place_ty)), Some((_, lhs_ty)), Some((_, result_ty)), Some((_, right_ty))) = (
            parse(&place),
            parse(&lhs),
            parse(&result),
            self.integer(right),
        ) else {
            self.refused.insert(format!(
                "compound {opcode} on `{place}` ({lhs} -> {result})"
            ));
            return self.full(node);
        };
        let shift = matches!(op, BinOp::Shl | BinOp::Shr);
        if lhs_ty != result_ty || (!shift && right_ty != result_ty) {
            self.refused.insert(format!(
                "compound {opcode}: operand types differ from the computation type `{result}`"
            ));
            return self.full(node);
        }
        // The place's value converted to the computation type, the
        // operation, and the result converted back to the place's type.
        let (a, to_lhs, _) = c_numeric::convert_interval(lhs_ty, place_ty.full(), false);
        let (value, mut obligations) =
            c_numeric::binary_interval(op, result_ty, a, b.unwrap_or(right_ty.full()));
        obligations.extend(to_lhs);
        let (stored, back, _) =
            c_numeric::convert_interval(place_ty, value, place_ty.width == 1 && !place_ty.signed);
        obligations.extend(back);
        self.bit_field_store(left, &format!("compound {opcode}"));
        self.site(format!("compound {opcode}"), node, obligations);
        Some(stored)
    }

    fn unary(&mut self, node: &Value) -> Option<Interval> {
        let opcode = node["opcode"].as_str().unwrap_or_default().to_owned();
        let child = node["inner"].as_array().and_then(|inner| inner.first())?;
        let a = self.value(child);
        match opcode.as_str() {
            "!" => Some(Interval { lo: 0, hi: 1 }),
            "-" | "~" | "+" => {
                if self.model.floating_precision(spelling(node)).is_some() {
                    // Negating or keeping a floating value is exact.
                    return None;
                }
                let Some((_, ty)) = self.integer(node) else {
                    self.refused
                        .insert(format!("unary {opcode} on `{}`", spelling(node)));
                    return None;
                };
                let op = match opcode.as_str() {
                    "-" => UnOp::Neg,
                    "~" => UnOp::Not,
                    _ => UnOp::Plus,
                };
                let (value, obligations) =
                    c_numeric::unary_interval(op, ty, a.unwrap_or(ty.full()));
                self.site(format!("unary {opcode}"), node, obligations);
                Some(value)
            }
            "++" | "--" => {
                let place = spelling(child).to_owned();
                if is_pointer(&place) {
                    self.outside += 1;
                    return self.full(node);
                }
                if self.model.floating_precision(&place).is_some() {
                    self.refused
                        .insert(format!("floating `{opcode}` (N09/N10)"));
                    return None;
                }
                let Some((kind, ty)) = self.integer(child) else {
                    self.refused.insert(format!("`{opcode}` on `{place}`"));
                    return None;
                };
                self.bit_field_store(child, &format!("`{opcode}`"));
                // `E++` is `E += 1`: a signed type of `int`'s width or more
                // overflows in its own type; a narrower one is computed in
                // its promoted type and converted back. The stored range is
                // not tracked, so a signed place is always an open obligation.
                if ty.signed && kind != IntKind::Bool {
                    let fault = if ty.width >= self.model.int_width {
                        Fault::SignedOverflow
                    } else {
                        Fault::SignedConversion
                    };
                    self.open(format!("increment {opcode}"), node, fault);
                } else if kind != IntKind::Bool {
                    self.event("unsigned-wrap", format!("`{opcode}` at `{place}`"));
                }
                Some(ty.full())
            }
            "__extension__" => a,
            _ => self.full(node),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lp64() -> TargetIntegerModel {
        TargetIntegerModel {
            triple: "x86_64-pc-linux-gnu".into(),
            char_bit: 8,
            char_signed: true,
            short_width: 16,
            int_width: 32,
            long_width: 64,
            long_long_width: 64,
            pointer_width: 64,
            float_precision: [24, 53, 64],
        }
    }

    #[test]
    fn an_empty_unit_has_no_functions() {
        let report = report(
            lp64(),
            &serde_json::json!({"kind": "TranslationUnitDecl", "inner": []}),
        );
        assert!(report.functions.is_empty());
        assert_eq!(report.data_model, c_numeric::DataModel::Lp64);
    }
}
