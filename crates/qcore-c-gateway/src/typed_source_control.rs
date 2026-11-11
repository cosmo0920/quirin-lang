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

//! Route T's control statements (card CG09-TYPED-SOURCE-CONTROL, gateway rule
//! 11l): `switch` / `case` / `default` with fallthrough, `break`, `continue`,
//! forward `goto` and labels, and general `while` / `for` / `do` loops.
//!
//! **Reading is not admission**, as for CG01's nodes (rule 10r). Every importer
//! call names a [`ControlAdmission`], the consumer's own list of the control
//! forms it decides. A form the consumer does not list is refused by name and
//! the function stays `Unknown`. The legacy entry points pass
//! [`ControlAdmission::LEGACY`], which lists nothing.
//!
//! **Acyclic forms are lowered, not modelled anew.** A `switch`, a `break`, a
//! `continue` out of `do … while (0)` and a forward `goto` become paths of the
//! existing vocabulary: guards (`selector == constant`, taken or not) and the
//! statements that run. The selector is evaluated once. Every path that leaves
//! a block, by falling off its end, by `return` or by a jump, ends that block's
//! locals (`LocalEnd`) on the way out. Under a consumer that lists any control
//! form, a use of a local's storage after its `LocalEnd` is refused on every
//! path ([`ControlAdmission::check_local_lifetimes`]); the legacy admission
//! keeps T's pre-CG09 projection of ended storage.
//!
//! **A back-edge needs an invariant or a summary.** A general loop or a
//! backward `goto` closes a cycle. T's product is a finite set of paths, so a
//! cycle can be admitted only where the consumer supplies an invariant or a
//! summary for it. Route T can check none yet (card CG27 owns them), so no
//! admission may list a back-edge form, and every consumer refuses one by
//! name. The budgets are unchanged, and exceeding any of them is a refusal.
//!
//! `proof/QCore/TypedSourceControl.lean` models the table, the admission, the
//! lifetime ends, the `switch` lowering and the budgets. The test
//! `tests/unit/source_control.rs` binds the two by reading the Lean vectors.

use serde_json::Value;

use super::{
    Expr, Importer, MAX_PATHS, Path, SourceExpression, SourceOperation, SourcePath, SourceValue,
    Stmt, c_type, check_value_size, effects, execute, value, visit_value,
};
use crate::typed_c_ast::{children, kind};
use crate::typed_source_nodes::OPERAND_TYPES;

/// One control form route T reads, and whether it closes a back-edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ControlFacts {
    pub spelling: &'static str,
    /// The form closes a cycle in the control-flow graph.
    pub back_edge: bool,
}

const fn form(spelling: &'static str, back_edge: bool) -> ControlFacts {
    ControlFacts {
        spelling,
        back_edge,
    }
}

/// Every control form CG09 reads, with its intrinsic fact. `continue` is
/// acyclic here because the only statement route T lets it target is
/// `do … while (0)`, whose condition is the literal zero: every general loop
/// is refused at its own node first. The Lean model carries the same rows
/// (`TypedSourceControl.controlTable`).
pub const CONTROL_TABLE: [ControlFacts; 8] = [
    form("switch", false),
    form("break", false),
    form("continue", false),
    form("goto", false),
    form("goto backward", true),
    form("loop while", true),
    form("loop for", true),
    form("loop do", true),
];

/// Clang statement kinds T read before CG09 (`for` only as a canonical scan,
/// `do` only with a literal zero condition).
pub const LEGACY_STATEMENTS: [&str; 11] = [
    "CompoundStmt",
    "NullStmt",
    "ForStmt",
    "DoStmt",
    "DeclStmt",
    "BinaryOperator",
    "CompoundAssignOperator",
    "UnaryOperator",
    "IfStmt",
    "CallExpr",
    "ReturnStmt",
];

/// The control form each Clang statement kind is read as. A `for` or `do`
/// takes its row only where it is not the legacy shape; a `goto` whose label
/// was already read is `goto backward`.
pub const CONTROL_STATEMENTS: [(&str, &str); 8] = [
    ("SwitchStmt", "switch"),
    ("BreakStmt", "break"),
    ("ContinueStmt", "continue"),
    ("GotoStmt", "goto"),
    ("LabelStmt", "goto"),
    ("WhileStmt", "loop while"),
    ("ForStmt", "loop for"),
    ("DoStmt", "loop do"),
];

/// The intrinsic facts of a control form, by spelling.
pub fn control_facts(spelling: &str) -> Option<&'static ControlFacts> {
    CONTROL_TABLE
        .iter()
        .find(|facts| facts.spelling == spelling)
}

/// One consumer's list of the control forms it decides. Validated before any
/// function is read: an unknown spelling, a duplicate row or a back-edge form
/// fails the whole import.
#[derive(Clone, Copy, Debug)]
pub struct ControlAdmission {
    consumer: &'static str,
    rows: &'static [&'static str],
}

impl ControlAdmission {
    /// The pre-CG09 grammar: no control form is admitted.
    pub const LEGACY: Self = Self {
        consumer: "legacy typed-source consumer",
        rows: &[],
    };

    /// A consumer's admission, checked by [`Self::validate`].
    pub const fn new(consumer: &'static str, rows: &'static [&'static str]) -> Self {
        Self { consumer, rows }
    }

    pub fn consumer(&self) -> &'static str {
        self.consumer
    }

    pub fn rows(&self) -> &'static [&'static str] {
        self.rows
    }

    /// Check the rows against the control table. A back-edge form cannot be
    /// listed: route T has no invariant or summary to carry it (CG27).
    pub fn validate(&self) -> Result<(), String> {
        for (index, spelling) in self.rows.iter().enumerate() {
            let facts = control_facts(spelling).ok_or_else(|| {
                format!("{}: `{spelling}` is not a CG09 control form", self.consumer)
            })?;
            if self.rows[..index].contains(spelling) {
                return Err(format!("{}: `{spelling}` is listed twice", self.consumer));
            }
            if facts.back_edge {
                return Err(format!(
                    "{}: `{spelling}` has a back-edge, and route T checks no invariant or \
                     summary for one (CG27)",
                    self.consumer
                ));
            }
        }
        Ok(())
    }

    /// Admit one form, or refuse it by name.
    pub fn admit(&self, spelling: &str) -> Result<(), String> {
        if control_facts(spelling).is_some_and(|facts| facts.back_edge) {
            return Err(self.back_edge_refusal(spelling, None));
        }
        if self.rows.contains(&spelling) {
            Ok(())
        } else {
            Err(format!(
                "typed-source control `{spelling}` is not admitted by the {}",
                self.consumer
            ))
        }
    }

    /// The refusal of a back-edge form: no consumer supplies an invariant or
    /// summary for it. `shape` is the legacy shape's own reason, where the
    /// statement also has one (a `for` that is not a canonical scan).
    pub fn back_edge_refusal(&self, spelling: &str, shape: Option<&str>) -> String {
        let refusal = format!(
            "typed-source control `{spelling}` has a back-edge, and the {} supplies no \
             invariant or summary for it (CG27)",
            self.consumer
        );
        match shape {
            Some(shape) => format!("{refusal}; not a canonical scan: {shape}"),
            None => refusal,
        }
    }
}

/// A pending jump on one path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Jump {
    Break,
    Continue,
    /// A forward `goto`, by the target label's Clang declaration ID.
    Goto(String),
}

/// The top level of a `switch` body, flattened: each label is its own item,
/// in source order, followed by the statement it labels.
#[derive(Clone)]
pub(super) enum SwitchItem {
    Case(Expr),
    Default,
    Statement(Box<Stmt>),
}

impl Importer<'_> {
    /// Every statement kind outside the pre-CG09 grammar: a control form is
    /// admitted or refused by name; anything else keeps the generic refusal.
    pub(super) fn control_statement(&mut self, node: &Value, depth: usize) -> Result<Stmt, String> {
        match (kind(node), children(node)) {
            ("SwitchStmt", parts) => {
                self.control.admit("switch")?;
                let [condition, body] = parts else {
                    return Err("switch with an initializer or condition variable".into());
                };
                self.switch(condition, body, depth)
            }
            ("BreakStmt", []) => {
                self.control.admit("break")?;
                Ok(Stmt::Jump(Jump::Break))
            }
            ("ContinueStmt", []) => {
                self.control.admit("continue")?;
                Ok(Stmt::Jump(Jump::Continue))
            }
            ("GotoStmt", []) => {
                let target = node["targetLabelDeclId"]
                    .as_str()
                    .ok_or("missing goto target")?;
                if self.labels.contains(target) {
                    return Err(self.control.back_edge_refusal("goto backward", None));
                }
                self.control.admit("goto")?;
                Ok(Stmt::Jump(Jump::Goto(target.into())))
            }
            ("LabelStmt", [inner]) => {
                self.control.admit("goto")?;
                let label = node["declId"].as_str().ok_or("missing label declaration")?;
                // Recorded before its statement: a `goto` inside it is backward.
                self.labels.insert(label.into());
                let inner = self.statement(inner, depth + 1)?;
                if matches!(inner, Stmt::Declare(..)) {
                    return Err("label on a declaration".into());
                }
                Ok(Stmt::Label(label.into(), Box::new(inner)))
            }
            ("WhileStmt", _) => Err(self.control.back_edge_refusal("loop while", None)),
            ("CaseStmt" | "DefaultStmt", _) => {
                Err("case label outside its switch body's top level".into())
            }
            _ => Err(format!("unmodeled callback statement {}", kind(node))),
        }
    }

    // The selector must be a promoted integer; the body a compound statement
    // whose top level holds labels and statements, but no declaration and no
    // ordinary label (neither could be scoped or reached soundly).
    fn switch(&mut self, condition: &Value, body: &Value, depth: usize) -> Result<Stmt, String> {
        let ty = c_type(condition)?;
        if !OPERAND_TYPES.contains(&ty.as_str()) {
            return Err(format!(
                "switch controlling type {ty} is not a promoted integer type"
            ));
        }
        let selector = self.expression(condition, depth + 1)?;
        if kind(body) != "CompoundStmt" {
            return Err("switch body is not a compound statement".into());
        }
        let mut items = Vec::new();
        for child in children(body) {
            self.switch_item(child, &ty, &mut items, depth + 1)?;
        }
        Ok(Stmt::Switch(selector, items))
    }

    fn switch_item(
        &mut self,
        node: &Value,
        ty: &str,
        items: &mut Vec<SwitchItem>,
        depth: usize,
    ) -> Result<(), String> {
        self.budget(depth)?;
        match (kind(node), children(node)) {
            ("CaseStmt", [constant, inner]) => {
                if c_type(constant)? != ty {
                    return Err(
                        "switch case constant type differs from its controlling type".into(),
                    );
                }
                let constant = self.case_constant(constant, depth + 1)?;
                if effects(&constant) != (false, false) {
                    return Err("switch case label reads or calls".into());
                }
                items.push(SwitchItem::Case(constant));
                self.switch_item(inner, ty, items, depth + 1)
            }
            ("CaseStmt", _) => Err("GNU case range is outside route T".into()),
            ("DefaultStmt", [inner]) => {
                items.push(SwitchItem::Default);
                self.switch_item(inner, ty, items, depth + 1)
            }
            ("DeclStmt", _) => Err("declaration at a switch body's top level".into()),
            ("LabelStmt", _) => Err("label at a switch body's top level".into()),
            _ => {
                items.push(SwitchItem::Statement(Box::new(
                    self.statement(node, depth)?,
                )));
                Ok(())
            }
        }
    }

    // Clang's case label: its constant expression, under Clang's own integral
    // conversion to the promoted controlling type. The value is never
    // evaluated on the host.
    fn case_constant(&mut self, node: &Value, depth: usize) -> Result<Expr, String> {
        match (kind(node), children(node)) {
            ("ConstantExpr", [inner]) => self.expression(inner, depth + 1),
            ("ImplicitCastExpr", [inner]) if node["castKind"] == "IntegralCast" => Ok(Expr::Cast(
                c_type(node)?,
                "IntegralCast".into(),
                Box::new(self.case_constant(inner, depth + 1)?),
            )),
            _ => Err("switch case label is not a constant expression".into()),
        }
    }
}

/// Before `statements[index]` runs in its block: a path whose forward `goto`
/// targets this label resumes, and a path that would jump past a declaration
/// to a label later in the same block (into the declared local's scope, where
/// it was never initialised) is refused.
pub(super) fn enter_statement(
    statements: &[Stmt],
    index: usize,
    paths: &mut [Path],
) -> Result<(), String> {
    let statement = &statements[index];
    let mut labelled = statement;
    while let Stmt::Label(label, inner) = labelled {
        for path in paths.iter_mut() {
            if path.jump.as_ref() == Some(&Jump::Goto(label.clone())) {
                path.jump = None;
            }
        }
        labelled = inner;
    }
    if matches!(statement, Stmt::Declare(..)) {
        for path in paths.iter() {
            if let Some(Jump::Goto(label)) = &path.jump
                && statements[index + 1..]
                    .iter()
                    .any(|later| contains_label(later, label))
            {
                return Err("goto skips a declaration in its target scope".into());
            }
        }
    }
    Ok(())
}

fn contains_label(statement: &Stmt, label: &str) -> bool {
    match statement {
        Stmt::Label(found, inner) => found == label || contains_label(inner, label),
        Stmt::Block(statements) => statements.iter().any(|inner| contains_label(inner, label)),
        Stmt::If(_, yes, no) => {
            contains_label(yes, label) || no.as_ref().is_some_and(|no| contains_label(no, label))
        }
        Stmt::DoOnce(body, _) | Stmt::ScanLoop(body, ..) => contains_label(body, label),
        Stmt::Switch(_, items) => items.iter().any(
            |item| matches!(item, SwitchItem::Statement(inner) if contains_label(inner, label)),
        ),
        _ => false,
    }
}

/// After `do … while (0)`'s body: whether this path evaluates the condition.
/// `break` leaves before it and `continue` evaluates it (both are taken
/// here); a `goto`, a `return` and a termination pass through.
pub(super) fn do_once_reaches_condition(path: &mut Path) -> bool {
    match path.jump.take() {
        Some(Jump::Break) => false,
        Some(Jump::Continue) => true,
        Some(other) => {
            path.jump = Some(other);
            false
        }
        None => !path.returned && !path.terminated,
    }
}

/// Lower one `switch`: the selector is evaluated once; each label is an entry
/// whose path carries `selector == constant` taken (a `case`) or every case's
/// guard not taken (`default`), then runs every statement from that label on
/// (fallthrough) until a `break`, which the `switch` takes. With no `default`,
/// one more path carries every case guard not taken and runs nothing.
pub(super) fn switch(
    mut path: Path,
    selector: &Expr,
    items: &[SwitchItem],
) -> Result<Vec<Path>, String> {
    let selector = path.evaluate(selector)?;
    let constants = items
        .iter()
        .filter_map(|item| match item {
            SwitchItem::Case(constant) => Some(constant),
            _ => None,
        })
        .collect::<Vec<_>>();
    let mut output = Vec::new();
    let mut has_default = false;
    for (start, item) in items.iter().enumerate() {
        let mut entry = path.clone();
        match item {
            SwitchItem::Case(constant) => {
                let constant = entry.evaluate(constant)?;
                entry.push(guard(&selector, constant, true)?)?;
            }
            SwitchItem::Default => {
                has_default = true;
                for constant in &constants {
                    let constant = entry.evaluate(constant)?;
                    entry.push(guard(&selector, constant, false)?)?;
                }
            }
            SwitchItem::Statement(_) => continue,
        }
        output.extend(run_from(entry, &items[start..])?);
        if output.len() > MAX_PATHS {
            return Err("callback path limit exceeded".into());
        }
    }
    if !has_default {
        for constant in &constants {
            let constant = path.evaluate(constant)?;
            path.push(guard(&selector, constant, false)?)?;
        }
        output.push(path);
    }
    if output.len() > MAX_PATHS {
        return Err("callback path limit exceeded".into());
    }
    Ok(output)
}

fn guard(
    selector: &SourceValue,
    constant: SourceValue,
    taken: bool,
) -> Result<SourceOperation, String> {
    let condition = value(
        "int".into(),
        SourceExpression::Binary {
            operator: "==".into(),
            left: Box::new(selector.clone()),
            right: Box::new(constant),
        },
    );
    check_value_size(&condition)?;
    Ok(SourceOperation::Guard { condition, taken })
}

// Run every statement from an entry label to the end of the body; the switch
// takes each path's `break`.
fn run_from(entry: Path, items: &[SwitchItem]) -> Result<Vec<Path>, String> {
    let mut paths = vec![entry];
    for item in items {
        if let SwitchItem::Statement(statement) = item {
            paths = execute(statement, paths)?;
        }
    }
    for path in &mut paths {
        if path.jump == Some(Jump::Break) {
            path.jump = None;
        }
    }
    Ok(paths)
}

/// The refusal of a jump that no statement took by the end of the function.
pub(super) fn untaken(jump: &Jump) -> String {
    match jump {
        Jump::Goto(_) => "goto target is not a later statement of an enclosing block".into(),
        Jump::Break | Jump::Continue => "break or continue outside its target".into(),
    }
}

impl ControlAdmission {
    /// Under a consumer that lists any control form, refuse a path that uses
    /// a local's storage after that local's `LocalEnd`: a jump (or a block's
    /// end) closed the scope, so any later address of it dangles. Checked on
    /// every path, jump or not. The legacy admission keeps route T's
    /// pre-CG09 projection, which retains ended storage for the consumer to
    /// decide (pinned by the Monkey callback tests).
    pub(super) fn check_local_lifetimes(&self, path: &SourcePath) -> Result<(), String> {
        if self.rows.is_empty() {
            return Ok(());
        }
        local_lifetimes(path)
    }
}

fn local_lifetimes(path: &SourcePath) -> Result<(), String> {
    let mut ended = std::collections::BTreeSet::new();
    for operation in &path.operations {
        if let SourceOperation::LocalEnd { local } = operation {
            ended.insert(*local);
            continue;
        }
        for value in operation_values(operation) {
            visit_value(value, |value| match value.expression {
                SourceExpression::LocalStorage { local } if ended.contains(&local) => {
                    Err("local storage used after its lifetime ended".into())
                }
                _ => Ok(()),
            })?;
        }
        if let SourceOperation::LocalWrite { local, .. } | SourceOperation::LocalStart { local, .. } =
            operation
            && ended.contains(local)
        {
            return Err("local storage used after its lifetime ended".into());
        }
    }
    Ok(())
}

fn operation_values(operation: &SourceOperation) -> Vec<&SourceValue> {
    match operation {
        SourceOperation::ScanLoop {
            initial,
            header,
            matches,
            early_return,
            increment,
            ..
        } => {
            let mut values = vec![initial, increment];
            for region in [header, matches, early_return] {
                for operation in &region.operations {
                    values.extend(operation_values(operation));
                }
                values.push(&region.result);
            }
            values
        }
        SourceOperation::Read { place } => vec![place],
        SourceOperation::Write { place, value } => vec![place, value],
        SourceOperation::LocalWrite { value, .. } | SourceOperation::ReturnValue { value } => {
            vec![value]
        }
        SourceOperation::Call { arguments, .. } => arguments.iter().collect(),
        SourceOperation::Guard { condition, .. } => vec![condition],
        SourceOperation::LocalStart { .. }
        | SourceOperation::LocalEnd { .. }
        | SourceOperation::Return
        | SourceOperation::Terminate => vec![],
    }
}
