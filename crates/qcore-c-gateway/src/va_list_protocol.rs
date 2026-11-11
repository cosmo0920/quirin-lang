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

//! Card **CG26-VARIADICS** (gateway rule 11w): the `va_list` protocol of one
//! function, read from Clang's typed AST.
//!
//! C11 7.16 gives a variable argument list a protocol, not a custody: a list is
//! initialised by `va_start` (or `va_copy` into it), read by `va_arg`, handed
//! whole to a callee such as `vsnprintf` (after which "the value of `ap` in the
//! calling function is indeterminate and shall be passed to the `va_end` macro
//! prior to any further reference", 7.16p3), and ended by `va_end` before the
//! function returns (7.16.1.3p2, 7.16.1.4p3). Card W44-LIBC-RESIDUALS measured
//! 127 `call to uncontracted function __builtin_va_start/__builtin_va_end`
//! leaves; they are compiler builtins with no C prototype, so no provider
//! declaration can state them.
//!
//! This module is the dedicated recognised form for the four builtins (card
//! CG21 owns builtins' common opaque boundary; these are never routed through
//! it). It decides, for one function, whether **every** path from entry keeps
//! every list's protocol, over an explicit control-flow graph built from the
//! AST: sequence, `if`, `while`, `do`, `for`, `switch` with its `case` and
//! `default` labels, `break`, `continue`, `return`, `goto` and labels. The
//! per-list abstract state is a set of [`ListState`]s, joined over edges and
//! iterated to a fixed point; [`step`] is the transition table and
//! [`exit_ok`] the exit predicate (`QCore.Variadics` proves that an accepted
//! graph has no violating path).
//!
//! A list is either a **local** of `va_list` type declared in the function
//! body's outermost block (initially uninitialised, and owing `va_end` on
//! every exit), or a **parameter** of `va_list` type (initially active and
//! owing nothing, since its `va_end` is the caller's; `va_start`, `va_copy`
//! into it and `va_end` on it are refused).
//!
//! What a list event is, read from the AST only:
//!
//! * `__builtin_va_start(L, P)` on a local of a variadic function, `P` its
//!   last named parameter — [`Operation::Start`];
//! * `__builtin_va_end(L)` — [`Operation::End`];
//! * `__builtin_va_copy(D, S)` — [`Operation::CopyFrom`] on `S` then
//!   [`Operation::Start`] on `D` (one full-expression event);
//! * `VAArgExpr` on `L` — [`Operation::Arg`];
//! * `L` as one whole argument of any other call — [`Operation::Pass`].
//!
//! Every other mention of a list (its address, `sizeof`, an assignment, a
//! comparison, a cast to anything but its own decay), two list events in one
//! full expression, a list event under `&&`, `||`, `?:` or a statement
//! expression, a computed `goto`, a `longjmp`, and a statement form this
//! reader does not know, refuses the function's protocol by name. The
//! protocol says nothing about which arguments a `va_arg` reads or their
//! types (format safety, gap entry K10, is not claimed).

use std::collections::BTreeMap;

use serde_json::Value;

use crate::typed_c_ast::{children, kind};

/// One abstract state of one list.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ListState {
    /// Declared, not yet started (or a local before its declaration).
    Uninitialised,
    /// Started by `va_start`/`va_copy`, or a parameter as received.
    Active,
    /// Handed whole to a callee: indeterminate until `va_end` (7.16p3).
    Passed,
    /// Ended by `va_end`.
    Ended,
}

impl ListState {
    pub(crate) const ALL: [Self; 4] =
        [Self::Uninitialised, Self::Active, Self::Passed, Self::Ended];

    const fn bit(self) -> u8 {
        match self {
            Self::Uninitialised => 1,
            Self::Active => 2,
            Self::Passed => 4,
            Self::Ended => 8,
        }
    }
}

/// A set of [`ListState`]s, as a bit mask (bit order of [`ListState::ALL`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct States(pub(crate) u8);

impl States {
    pub(crate) const EMPTY: Self = Self(0);

    pub(crate) const fn only(state: ListState) -> Self {
        Self(state.bit())
    }

    pub(crate) const fn contains(self, state: ListState) -> bool {
        self.0 & state.bit() != 0
    }

    pub(crate) const fn join(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// `self ⊆ other`.
    pub(crate) const fn within_set(self, other: Self) -> bool {
        self.0 & !other.0 == 0
    }

    /// Whether every member is one of `allowed`.
    fn within(self, allowed: &[ListState]) -> bool {
        ListState::ALL
            .iter()
            .all(|state| !self.contains(*state) || allowed.contains(state))
    }
}

/// One operation on one list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Operation {
    /// `va_start(L, P)`, or the destination half of `va_copy(L, S)`.
    Start,
    /// `va_arg(L, T)`.
    Arg,
    /// `L` handed whole to a callee.
    Pass,
    /// The source half of `va_copy(D, L)`: a read of an active list.
    CopyFrom,
    /// `va_end(L)`.
    End,
}

/// Whether a list is the function's own local or a parameter it received.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Origin {
    Local,
    Parameter,
}

/// Why one operation is refused at one node (the protocol violation the
/// abstract state cannot exclude).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Violation {
    /// `va_start`/`va_copy` into a list that may still be active or passed.
    StartWhileStarted,
    /// `va_arg`, a pass or a `va_copy` read of a list that may not be active.
    UseWhenNotActive,
    /// `va_end` on a list that may never have been started or is already ended.
    EndWhenNotStarted,
    /// `va_start`, `va_copy` into or `va_end` on a parameter list.
    ParameterOwnedByCaller,
    /// The fixed point did not close (never expected: the worklist joins
    /// until it does); the certificate check refuses rather than trusting it.
    Unclosed,
}

/// The transition table. `Err` when some state in `states` makes `operation`
/// a protocol violation; otherwise the set of states after it.
pub(crate) fn step(
    origin: Origin,
    states: States,
    operation: Operation,
) -> Result<States, Violation> {
    use ListState::{Active, Ended, Passed, Uninitialised};
    match operation {
        Operation::Start | Operation::End if origin == Origin::Parameter => {
            Err(Violation::ParameterOwnedByCaller)
        }
        Operation::Start if states.within(&[Uninitialised, Ended]) => Ok(States::only(Active)),
        Operation::Start => Err(Violation::StartWhileStarted),
        Operation::Arg | Operation::CopyFrom if states.within(&[Active]) => {
            Ok(States::only(Active))
        }
        Operation::Pass if states.within(&[Active]) => Ok(States::only(Passed)),
        Operation::Arg | Operation::CopyFrom | Operation::Pass => Err(Violation::UseWhenNotActive),
        Operation::End if states.within(&[Active, Passed]) => Ok(States::only(Ended)),
        Operation::End => Err(Violation::EndWhenNotStarted),
    }
}

/// Whether a list may be in `states` at a function exit: a local list must be
/// uninitialised or ended (7.16.1.3p2); a parameter list owes nothing here.
pub(crate) fn exit_ok(origin: Origin, states: States) -> bool {
    origin == Origin::Parameter || states.within(&[ListState::Uninitialised, ListState::Ended])
}

/// The initial state of a list at function entry.
pub(crate) const fn initial(origin: Origin) -> States {
    match origin {
        Origin::Local => States::only(ListState::Uninitialised),
        Origin::Parameter => States::only(ListState::Active),
    }
}

/// The transition table as data, for the shared-vector binding with
/// `proof/QCore/Variadics.lean`: origin, incoming states (mask), operation,
/// and the outgoing mask or `None` for a violation. Every one of the
/// 2 × 16 × 5 rows is checked against [`step`] by a unit test, and these rows
/// are the ones the Lean file decides.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) const STEP_VECTORS: &[(Origin, u8, Operation, Option<u8>)] = &[
    (Origin::Local, 1, Operation::Start, Some(2)),
    (Origin::Local, 8, Operation::Start, Some(2)),
    (Origin::Local, 9, Operation::Start, Some(2)),
    (Origin::Local, 2, Operation::Start, None),
    (Origin::Local, 3, Operation::Start, None),
    (Origin::Local, 2, Operation::Arg, Some(2)),
    (Origin::Local, 3, Operation::Arg, None),
    (Origin::Local, 4, Operation::Arg, None),
    (Origin::Local, 8, Operation::Arg, None),
    (Origin::Local, 2, Operation::Pass, Some(4)),
    (Origin::Local, 4, Operation::Pass, None),
    (Origin::Local, 2, Operation::CopyFrom, Some(2)),
    (Origin::Local, 10, Operation::CopyFrom, None),
    (Origin::Local, 2, Operation::End, Some(8)),
    (Origin::Local, 4, Operation::End, Some(8)),
    (Origin::Local, 6, Operation::End, Some(8)),
    (Origin::Local, 1, Operation::End, None),
    (Origin::Local, 8, Operation::End, None),
    (Origin::Local, 10, Operation::End, None),
    (Origin::Parameter, 2, Operation::Arg, Some(2)),
    (Origin::Parameter, 2, Operation::Pass, Some(4)),
    (Origin::Parameter, 4, Operation::Arg, None),
    (Origin::Parameter, 2, Operation::CopyFrom, Some(2)),
    (Origin::Parameter, 2, Operation::Start, None),
    (Origin::Parameter, 2, Operation::End, None),
];

/// One list the function reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct List {
    pub(crate) name: String,
    pub(crate) origin: Origin,
}

/// Why a function's protocol was not established. Every variant keeps the
/// function's `va_*` statements refused; the wording replaces the
/// uncontracted-builtin refusal so that a report names the reason.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Refusal {
    /// A step of the transition table failed at some reachable node.
    Violation { list: String, violation: Violation },
    /// A local list may reach a function exit not ended.
    MissingEnd { list: String },
    /// A mention of a list that is not one of the five events.
    UnreadUse { list: String },
    /// `va_start` whose second operand is not the last named parameter, or in
    /// a function that is not variadic.
    StartOperand,
    /// A list event under a conditional operator, a statement expression or an
    /// unevaluated operand.
    ConditionalEvent,
    /// Two list events in one full expression.
    TwoEvents,
    /// A local list declared outside the outermost block, `static`, or with an
    /// initialiser.
    Declaration { list: String },
    /// A statement or control form the graph reader does not read, in a
    /// function that has a list.
    Form(String),
    /// A `goto` to a label the reader did not find.
    Label,
}

impl Refusal {
    pub(crate) fn description(&self) -> String {
        let reason = match self {
            Self::Violation { list, violation } => match violation {
                Violation::StartWhileStarted => {
                    format!("`va_start`/`va_copy` into `{list}`, which may already be started")
                }
                Violation::UseWhenNotActive => format!(
                    "`{list}` may be read after `va_end`, after being passed to a callee, or before `va_start`"
                ),
                Violation::EndWhenNotStarted => {
                    format!("`va_end` on `{list}`, which may not be started")
                }
                Violation::ParameterOwnedByCaller => {
                    format!("`va_start`, `va_copy` into or `va_end` on the parameter list `{list}`")
                }
                Violation::Unclosed => {
                    format!("the states of `{list}` did not reach a fixed point")
                }
            },
            Self::MissingEnd { list } => {
                format!("`{list}` may reach a function exit without `va_end`")
            }
            Self::UnreadUse { list } => format!(
                "`{list}` is used outside `va_start`, `va_arg`, `va_copy`, `va_end` or a whole call argument"
            ),
            Self::StartOperand => {
                "`va_start` does not name the last parameter of a variadic function".to_owned()
            }
            Self::ConditionalEvent => {
                "a `va_list` operation under a conditional operator or an unevaluated operand"
                    .to_owned()
            }
            Self::TwoEvents => "two `va_list` operations in one full expression".to_owned(),
            Self::Declaration { list } => {
                format!("`{list}` is not a plain local of the function's outermost block")
            }
            Self::Form(form) => format!("unread control form `{form}` beside a `va_list`"),
            Self::Label => "a `goto` whose label was not read".to_owned(),
        };
        format!("va_list protocol refused: {reason}")
    }
}

/// The established protocol of one function: its lists, each of whose every
/// path keeps [`step`] and [`exit_ok`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Protocol {
    pub(crate) lists: Vec<List>,
}

/// The builtins this module recognises.
pub(crate) const START: &str = "__builtin_va_start";
pub(crate) const END: &str = "__builtin_va_end";
pub(crate) const COPY: &str = "__builtin_va_copy";
/// The token spelling of `va_arg` after preprocessing (a `VAArgExpr` node).
pub(crate) const ARG: &str = "__builtin_va_arg";

/// Whether `name` is one of the four `va_*` builtin spellings.
pub(crate) fn names_va_builtin(name: &str) -> bool {
    matches!(name, START | END | COPY | ARG)
}

/// Whether a declared type is `va_list` (every spelling the audited targets
/// give it, local or adjusted parameter).
fn is_va_list_type(node: &Value) -> bool {
    let qual = node["type"]["qualType"].as_str().unwrap_or("");
    let desugared = node["type"]["desugaredQualType"].as_str().unwrap_or("");
    matches!(qual, "va_list" | "__gnuc_va_list" | "__builtin_va_list")
        || matches!(
            desugared,
            "struct __va_list_tag[1]" | "__builtin_va_list" | "struct __va_list_tag *"
        )
        || qual == "struct __va_list_tag *"
}

fn decl_id(node: &Value) -> Option<&str> {
    node["id"].as_str()
}

fn referenced_id(node: &Value) -> Option<&str> {
    node["referencedDecl"]["id"].as_str()
}

/// The direct callee name of a call, through Clang's function decays.
fn callee_name(call: &Value) -> Option<&str> {
    let mut callee = children(call).first()?;
    while matches!(kind(callee), "ImplicitCastExpr" | "ParenExpr") {
        callee = children(callee).first()?;
    }
    (kind(callee) == "DeclRefExpr"
        && callee["referencedDecl"]["kind"].as_str() == Some("FunctionDecl"))
    .then(|| callee["referencedDecl"]["name"].as_str())
    .flatten()
}

/// Facts the reader carries across one function.
struct Reader<'a> {
    /// Declaration id → list index.
    lists: BTreeMap<String, usize>,
    names: Vec<List>,
    /// The function's last named parameter's declaration id, when variadic.
    last_parameter: Option<&'a str>,
}

/// One list event of one full expression.
type Event = Vec<(usize, Operation)>;

impl Reader<'_> {
    fn list_of(&self, node: &Value) -> Option<usize> {
        let mut node = node;
        loop {
            match kind(node) {
                "ParenExpr" => node = children(node).first()?,
                "ImplicitCastExpr"
                    if matches!(
                        node["castKind"].as_str(),
                        Some("ArrayToPointerDecay" | "LValueToRValue" | "NoOp")
                    ) =>
                {
                    node = children(node).first()?;
                }
                "DeclRefExpr" => return self.lists.get(referenced_id(node)?).copied(),
                _ => return None,
            }
        }
    }

    /// Whether `node`'s subtree mentions a list at all.
    fn mentions(&self, node: &Value) -> Option<usize> {
        if kind(node) == "DeclRefExpr"
            && let Some(id) = referenced_id(node)
            && let Some(list) = self.lists.get(id)
        {
            return Some(*list);
        }
        children(node).iter().find_map(|child| self.mentions(child))
    }

    /// The list events of one full expression, in no order (at most one event
    /// is admitted, `va_copy`'s two halves being one).
    fn expression(&self, node: &Value, out: &mut Vec<Event>) -> Result<(), Refusal> {
        match kind(node) {
            "" => Ok(()),
            "CallExpr" => {
                let arguments = children(node).get(1..).unwrap_or(&[]);
                match callee_name(node) {
                    Some(START) => {
                        let [list, parameter] = arguments else {
                            return Err(Refusal::StartOperand);
                        };
                        let list = self.list_of(list).ok_or_else(|| self.unread(node))?;
                        let parameter = strip_reads(parameter);
                        if kind(parameter) != "DeclRefExpr"
                            || self.last_parameter.is_none()
                            || referenced_id(parameter) != self.last_parameter
                        {
                            return Err(Refusal::StartOperand);
                        }
                        out.push(vec![(list, Operation::Start)]);
                        Ok(())
                    }
                    Some(END) => {
                        let [list] = arguments else {
                            return Err(self.unread(node));
                        };
                        let list = self.list_of(list).ok_or_else(|| self.unread(node))?;
                        out.push(vec![(list, Operation::End)]);
                        Ok(())
                    }
                    Some(COPY) => {
                        let [destination, source] = arguments else {
                            return Err(self.unread(node));
                        };
                        let destination =
                            self.list_of(destination).ok_or_else(|| self.unread(node))?;
                        let source = self.list_of(source).ok_or_else(|| self.unread(node))?;
                        out.push(vec![
                            (source, Operation::CopyFrom),
                            (destination, Operation::Start),
                        ]);
                        Ok(())
                    }
                    Some(name) if name.starts_with("__builtin_va_") => {
                        Err(Refusal::Form(name.to_owned()))
                    }
                    Some(
                        "longjmp" | "siglongjmp" | "_longjmp" | "__builtin_longjmp"
                        | "__longjmp_chk",
                    ) => Err(Refusal::Form("longjmp".to_owned())),
                    _ => {
                        // The callee expression itself, then every actual: a
                        // whole list actual is a pass, the address of a whole
                        // local list a lend, any other is walked.
                        self.expression(&children(node)[0], out)?;
                        for argument in arguments {
                            if let Some(list) = self.list_of(argument) {
                                out.push(vec![(list, Operation::Pass)]);
                            } else if let Some(list) = self.lent_list(argument) {
                                out.push(vec![(list, Operation::Arg)]);
                            } else {
                                self.expression(argument, out)?;
                            }
                        }
                        Ok(())
                    }
                }
            }
            "VAArgExpr" => {
                let [list] = children(node) else {
                    return Err(self.unread(node));
                };
                let list = self.list_of(list).ok_or_else(|| self.unread(node))?;
                out.push(vec![(list, Operation::Arg)]);
                Ok(())
            }
            "ConditionalOperator"
            | "BinaryConditionalOperator"
            | "StmtExpr"
            | "UnaryExprOrTypeTraitExpr"
            | "GenericSelectionExpr" => self.inert(node),
            "BinaryOperator" if matches!(node["opcode"].as_str(), Some("&&" | "||")) => {
                self.inert(node)
            }
            "DeclRefExpr" => match referenced_id(node).and_then(|id| self.lists.get(id)) {
                Some(list) => Err(Refusal::UnreadUse {
                    list: self.names[*list].name.clone(),
                }),
                None => Ok(()),
            },
            _ => {
                for child in children(node) {
                    self.expression(child, out)?;
                }
                Ok(())
            }
        }
    }

    /// A subexpression C may evaluate conditionally, never, or as statements:
    /// admitted only when it mentions no list and holds no statement
    /// expression with a jump (a `return` inside `({ … })` leaves the function
    /// from inside an expression, which this graph does not draw).
    fn inert(&self, node: &Value) -> Result<(), Refusal> {
        if self.mentions(node).is_some() {
            return Err(Refusal::ConditionalEvent);
        }
        if contains_jump(node) {
            return Err(Refusal::Form("StmtExpr".to_owned()));
        }
        Ok(())
    }

    /// `&L` for a whole local list `L`: C11 7.16p3's footnote permits handing
    /// a callee a pointer to a list, after which the caller may use the list
    /// further. It is read as [`Operation::Arg`]: the list must be active and
    /// stays active. A parameter's address is not a pointer to a `va_list` (its
    /// type was adjusted), so it is not read here.
    fn lent_list(&self, node: &Value) -> Option<usize> {
        let node = strip_parens(node);
        if kind(node) != "UnaryOperator" || node["opcode"].as_str() != Some("&") {
            return None;
        }
        let [operand] = children(node) else {
            return None;
        };
        let operand = strip_parens(operand);
        if kind(operand) != "DeclRefExpr" {
            return None;
        }
        let list = *self.lists.get(referenced_id(operand)?)?;
        (self.names[list].origin == Origin::Local).then_some(list)
    }

    fn unread(&self, node: &Value) -> Refusal {
        match self.mentions(node) {
            Some(list) => Refusal::UnreadUse {
                list: self.names[list].name.clone(),
            },
            None => Refusal::Form(kind(node).to_owned()),
        }
    }

    /// The one event of a full expression, or none.
    fn full_expression(&self, node: &Value) -> Result<Option<Event>, Refusal> {
        let mut events = Vec::new();
        self.expression(node, &mut events)?;
        match events.len() {
            0 => Ok(None),
            1 => Ok(events.pop()),
            _ => Err(Refusal::TwoEvents),
        }
    }
}

fn strip_parens(mut node: &Value) -> &Value {
    while kind(node) == "ParenExpr" {
        let Some(child) = children(node).first() else {
            break;
        };
        node = child;
    }
    node
}

/// Whether a statement expression inside `node` holds a jump statement.
fn contains_jump(node: &Value) -> bool {
    matches!(
        kind(node),
        "ReturnStmt" | "GotoStmt" | "IndirectGotoStmt" | "BreakStmt" | "ContinueStmt" | "LabelStmt"
    ) || children(node).iter().any(contains_jump)
}

fn strip_reads(mut node: &Value) -> &Value {
    while matches!(kind(node), "ParenExpr" | "ImplicitCastExpr") {
        let Some(child) = children(node).first() else {
            break;
        };
        node = child;
    }
    node
}

/// One node of the protocol graph: at most one event, then its successors.
#[derive(Clone, Debug, Default)]
struct Node {
    event: Option<Event>,
    successors: Vec<usize>,
}

/// The node every `return` and the end of the body reach.
const EXIT: usize = 0;

struct Graph<'r, 'a> {
    reader: &'r Reader<'a>,
    nodes: Vec<Node>,
    /// One frame per enclosing loop or `switch`: the nodes that `break` leaves
    /// from, and (loops only) those `continue` leaves from.
    breaks: Vec<Vec<usize>>,
    continues: Vec<Option<Vec<usize>>>,
    /// The innermost `switch`'s head node, and whether it has a `default`.
    switches: Vec<(usize, bool)>,
    labels: BTreeMap<String, usize>,
    gotos: Vec<(usize, String)>,
}

impl Graph<'_, '_> {
    fn node(&mut self, predecessors: &[usize], event: Option<Event>) -> usize {
        let index = self.nodes.len();
        self.nodes.push(Node {
            event,
            successors: Vec::new(),
        });
        for &predecessor in predecessors {
            self.nodes[predecessor].successors.push(index);
        }
        index
    }

    fn edge(&mut self, from: &[usize], to: usize) {
        for &predecessor in from {
            self.nodes[predecessor].successors.push(to);
        }
    }

    fn expression_node(
        &mut self,
        predecessors: &[usize],
        expression: &Value,
    ) -> Result<Vec<usize>, Refusal> {
        let event = self.reader.full_expression(expression)?;
        if event.is_none() && predecessors.len() == 1 {
            return Ok(predecessors.to_vec());
        }
        Ok(vec![self.node(predecessors, event)])
    }

    /// Lower `statement` reached from `predecessors`; the result is the set of
    /// nodes control falls through from.
    fn statement(
        &mut self,
        statement: &Value,
        predecessors: Vec<usize>,
    ) -> Result<Vec<usize>, Refusal> {
        match kind(statement) {
            "" | "NullStmt" => Ok(predecessors),
            "CompoundStmt" => {
                let mut current = predecessors;
                for child in children(statement) {
                    current = self.statement(child, current)?;
                }
                Ok(current)
            }
            "DeclStmt" => {
                let mut current = predecessors;
                for declaration in children(statement) {
                    match kind(declaration) {
                        "VarDecl" => {
                            for initializer in children(declaration) {
                                current = self.expression_node(&current, initializer)?;
                            }
                        }
                        "TypedefDecl" | "RecordDecl" | "EnumDecl" | "FunctionDecl"
                        | "StaticAssertDecl" => {
                            if self.reader.mentions(declaration).is_some() {
                                return Err(self.reader.unread(declaration));
                            }
                        }
                        other => return Err(Refusal::Form(other.to_owned())),
                    }
                }
                Ok(current)
            }
            "IfStmt" => {
                if statement["hasInit"].as_bool() == Some(true)
                    || statement["hasVar"].as_bool() == Some(true)
                {
                    return Err(Refusal::Form("IfStmt".to_owned()));
                }
                let parts = children(statement);
                let (Some(condition), Some(then)) = (parts.first(), parts.get(1)) else {
                    return Err(Refusal::Form("IfStmt".to_owned()));
                };
                let head = self.expression_node(&predecessors, condition)?;
                let mut after = self.statement(then, head.clone())?;
                match parts.get(2) {
                    Some(otherwise) if statement["hasElse"].as_bool() == Some(true) => {
                        after.extend(self.statement(otherwise, head)?);
                    }
                    _ => after.extend(head),
                }
                Ok(after)
            }
            "WhileStmt" => {
                let parts = children(statement);
                let (Some(condition), Some(body)) = (parts.first(), parts.get(1)) else {
                    return Err(Refusal::Form("WhileStmt".to_owned()));
                };
                let event = self.reader.full_expression(condition)?;
                let head = self.node(&predecessors, event);
                self.breaks.push(Vec::new());
                self.continues.push(Some(Vec::new()));
                let end = self.statement(body, vec![head])?;
                let breaks = self.breaks.pop().unwrap_or_default();
                let continues = self.continues.pop().flatten().unwrap_or_default();
                self.edge(&end, head);
                self.edge(&continues, head);
                let mut after = vec![head];
                after.extend(breaks);
                Ok(after)
            }
            "DoStmt" => {
                let parts = children(statement);
                let (Some(body), Some(condition)) = (parts.first(), parts.get(1)) else {
                    return Err(Refusal::Form("DoStmt".to_owned()));
                };
                let entry = self.node(&predecessors, None);
                self.breaks.push(Vec::new());
                self.continues.push(Some(Vec::new()));
                let mut end = self.statement(body, vec![entry])?;
                let breaks = self.breaks.pop().unwrap_or_default();
                end.extend(self.continues.pop().flatten().unwrap_or_default());
                let event = self.reader.full_expression(condition)?;
                let test = self.node(&end, event);
                self.edge(&[test], entry);
                let mut after = vec![test];
                after.extend(breaks);
                Ok(after)
            }
            "ForStmt" => {
                let parts = children(statement);
                let [init, variable, condition, increment, body] = parts else {
                    return Err(Refusal::Form("ForStmt".to_owned()));
                };
                if !kind(variable).is_empty() {
                    return Err(Refusal::Form("ForStmt".to_owned()));
                }
                let current = self.statement(init, predecessors)?;
                let event = if kind(condition).is_empty() {
                    None
                } else {
                    self.reader.full_expression(condition)?
                };
                let head = self.node(&current, event);
                self.breaks.push(Vec::new());
                self.continues.push(Some(Vec::new()));
                let mut end = self.statement(body, vec![head])?;
                let breaks = self.breaks.pop().unwrap_or_default();
                end.extend(self.continues.pop().flatten().unwrap_or_default());
                let stepped = if kind(increment).is_empty() {
                    end
                } else {
                    self.expression_node(&end, increment)?
                };
                self.edge(&stepped, head);
                let mut after = if kind(condition).is_empty() {
                    Vec::new()
                } else {
                    vec![head]
                };
                after.extend(breaks);
                Ok(after)
            }
            "SwitchStmt" => {
                if statement["hasInit"].as_bool() == Some(true)
                    || statement["hasVar"].as_bool() == Some(true)
                {
                    return Err(Refusal::Form("SwitchStmt".to_owned()));
                }
                let parts = children(statement);
                let (Some(condition), Some(body)) = (parts.first(), parts.get(1)) else {
                    return Err(Refusal::Form("SwitchStmt".to_owned()));
                };
                let event = self.reader.full_expression(condition)?;
                let head = self.node(&predecessors, event);
                self.breaks.push(Vec::new());
                self.continues.push(None);
                self.switches.push((head, false));
                // The body is entered only at its labels.
                let end = self.statement(body, Vec::new())?;
                let breaks = self.breaks.pop().unwrap_or_default();
                self.continues.pop();
                let (_, has_default) = self.switches.pop().unwrap_or((head, false));
                let mut after = end;
                after.extend(breaks);
                if !has_default {
                    after.push(head);
                }
                Ok(after)
            }
            "CaseStmt" | "DefaultStmt" => {
                let Some(&(head, _)) = self.switches.last() else {
                    return Err(Refusal::Form(kind(statement).to_owned()));
                };
                if kind(statement) == "DefaultStmt"
                    && let Some(frame) = self.switches.last_mut()
                {
                    frame.1 = true;
                }
                let parts = children(statement);
                // `case V:` / `case V ... W:` and its statement (the last child).
                let Some(body) = parts.last() else {
                    return Err(Refusal::Form(kind(statement).to_owned()));
                };
                if kind(statement) == "CaseStmt" {
                    for value in &parts[..parts.len() - 1] {
                        if self.reader.mentions(value).is_some() {
                            return Err(self.reader.unread(value));
                        }
                    }
                }
                let mut entry = predecessors;
                entry.push(head);
                let label = self.node(&entry, None);
                self.statement(body, vec![label])
            }
            "BreakStmt" => {
                let Some(frame) = self.breaks.last_mut() else {
                    return Err(Refusal::Form("BreakStmt".to_owned()));
                };
                frame.extend(predecessors);
                Ok(Vec::new())
            }
            "ContinueStmt" => {
                let Some(frame) = self.continues.iter_mut().rev().find_map(Option::as_mut) else {
                    return Err(Refusal::Form("ContinueStmt".to_owned()));
                };
                frame.extend(predecessors);
                Ok(Vec::new())
            }
            "ReturnStmt" => {
                let current = match children(statement).first() {
                    Some(value) => self.expression_node(&predecessors, value)?,
                    None => predecessors,
                };
                self.edge(&current, EXIT);
                Ok(Vec::new())
            }
            "LabelStmt" => {
                let Some(id) = statement["declId"].as_str() else {
                    return Err(Refusal::Form("LabelStmt".to_owned()));
                };
                let label = self.node(&predecessors, None);
                if self.labels.insert(id.to_owned(), label).is_some() {
                    return Err(Refusal::Label);
                }
                match children(statement).first() {
                    Some(body) => self.statement(body, vec![label]),
                    None => Ok(vec![label]),
                }
            }
            "GotoStmt" => {
                let Some(target) = statement["targetLabelDeclId"].as_str() else {
                    return Err(Refusal::Form("GotoStmt".to_owned()));
                };
                let from = self.node(&predecessors, None);
                self.gotos.push((from, target.to_owned()));
                Ok(Vec::new())
            }
            "AttributedStmt" => {
                let mut current = predecessors;
                for child in children(statement) {
                    if kind(child).ends_with("Attr") {
                        continue;
                    }
                    current = self.statement(child, current)?;
                }
                Ok(current)
            }
            "GCCAsmStmt" | "MSAsmStmt" => {
                if self.reader.mentions(statement).is_some() {
                    return Err(self.reader.unread(statement));
                }
                Ok(predecessors)
            }
            other
                if other.ends_with("Expr")
                    || other.ends_with("Operator")
                    || other.ends_with("Literal") =>
            {
                self.expression_node(&predecessors, statement)
            }
            other => Err(Refusal::Form(other.to_owned())),
        }
    }
}

/// The lists of one function: its `va_list` parameters, and the `va_list`
/// locals of its outermost block. A `va_list` declared anywhere else, `static`,
/// or initialised refuses.
fn read_lists(function: &Value, body: &Value) -> Result<Vec<(String, List)>, Refusal> {
    let mut lists = Vec::new();
    for parameter in children(function)
        .iter()
        .filter(|node| kind(node) == "ParmVarDecl")
    {
        if is_va_list_type(parameter)
            && let (Some(id), Some(name)) = (decl_id(parameter), parameter["name"].as_str())
        {
            lists.push((
                id.to_owned(),
                List {
                    name: name.to_owned(),
                    origin: Origin::Parameter,
                },
            ));
        }
    }
    let outermost = children(body)
        .iter()
        .filter(|node| kind(node) == "DeclStmt")
        .flat_map(children)
        .filter_map(|node| decl_id(node))
        .collect::<Vec<_>>();
    let mut failure = None;
    walk_declarations(body, &mut |declaration| {
        if failure.is_some() || !is_va_list_type(declaration) {
            return;
        }
        let name = declaration["name"].as_str().unwrap_or("").to_owned();
        let Some(id) = decl_id(declaration) else {
            failure = Some(Refusal::Declaration { list: name });
            return;
        };
        if !outermost.contains(&id)
            || declaration["storageClass"].is_string()
            || !children(declaration).is_empty()
        {
            failure = Some(Refusal::Declaration { list: name });
            return;
        }
        lists.push((
            id.to_owned(),
            List {
                name,
                origin: Origin::Local,
            },
        ));
    });
    match failure {
        Some(refusal) => Err(refusal),
        None => Ok(lists),
    }
}

fn walk_declarations(node: &Value, visit: &mut dyn FnMut(&Value)) {
    if kind(node) == "VarDecl" {
        visit(node);
    }
    for child in children(node) {
        walk_declarations(child, visit);
    }
}

/// The function's body (its one `CompoundStmt` child).
fn body_of(function: &Value) -> Option<&Value> {
    children(function)
        .iter()
        .find(|node| kind(node) == "CompoundStmt")
}

/// Decide one function's `va_list` protocol. `Ok` with no lists for a function
/// that reads none; `Err` names the first reason a list's protocol is not
/// established on every path.
pub(crate) fn check(function: &Value) -> Result<Protocol, Refusal> {
    let Some(body) = body_of(function) else {
        return Err(Refusal::Form("FunctionDecl".to_owned()));
    };
    let lists = read_lists(function, body)?;
    if lists.is_empty() {
        return Ok(Protocol::default());
    }
    let parameters = children(function)
        .iter()
        .filter(|node| kind(node) == "ParmVarDecl")
        .collect::<Vec<_>>();
    let last_parameter = (function["variadic"].as_bool() == Some(true))
        .then(|| parameters.last().and_then(|parameter| decl_id(parameter)))
        .flatten();
    let reader = Reader {
        lists: lists
            .iter()
            .enumerate()
            .map(|(index, (id, _))| (id.clone(), index))
            .collect(),
        names: lists.iter().map(|(_, list)| list.clone()).collect(),
        last_parameter,
    };
    let mut graph = Graph {
        reader: &reader,
        nodes: vec![Node::default()],
        breaks: Vec::new(),
        continues: Vec::new(),
        switches: Vec::new(),
        labels: BTreeMap::new(),
        gotos: Vec::new(),
    };
    let entry = graph.node(&[], None);
    let end = graph.statement(body, vec![entry])?;
    graph.edge(&end, EXIT);
    for (from, target) in std::mem::take(&mut graph.gotos) {
        let Some(&to) = graph.labels.get(&target) else {
            return Err(Refusal::Label);
        };
        graph.nodes[from].successors.push(to);
    }
    let origins = reader
        .names
        .iter()
        .map(|list| list.origin)
        .collect::<Vec<_>>();
    let initial_states = origins
        .iter()
        .map(|origin| initial(*origin))
        .collect::<Vec<_>>();
    decide(&graph.nodes, entry, &origins, &initial_states).map_err(
        |(list, refusal)| match refusal {
            Some(violation) => Refusal::Violation {
                list: reader.names[list].name.clone(),
                violation,
            },
            None => Refusal::MissingEnd {
                list: reader.names[list].name.clone(),
            },
        },
    )?;
    Ok(Protocol {
        lists: reader.names,
    })
}

/// The fixed point: the abstract states *before* each node, per list, from
/// `entry`; then every node's event must step and the exit's states must be
/// accepted. `Err((list, Some(v)))` for a failed step, `Err((list, None))` for
/// an exit a local list may reach not ended.
fn decide(
    nodes: &[Node],
    entry: usize,
    origins: &[Origin],
    initial_states: &[States],
) -> Result<(), (usize, Option<Violation>)> {
    let mut before = vec![vec![States::EMPTY; origins.len()]; nodes.len()];
    before[entry] = initial_states.to_vec();
    let mut work = vec![entry];
    while let Some(index) = work.pop() {
        let mut after = before[index].clone();
        if let Some(event) = &nodes[index].event {
            for &(list, operation) in event {
                // A failed step is reported by the check below; the fixed
                // point continues from the state it would have reached.
                after[list] = step(origins[list], after[list], operation).unwrap_or(States::EMPTY);
            }
        }
        for &successor in &nodes[index].successors {
            let mut changed = false;
            for (list, states) in after.iter().enumerate() {
                let joined = before[successor][list].join(*states);
                if joined != before[successor][list] {
                    before[successor][list] = joined;
                    changed = true;
                }
            }
            if changed {
                work.push(successor);
            }
        }
    }
    // The certificate `QCore.Variadics.certify` checks, list by list: the
    // entry holds the initial state; every node whose set is non-empty steps
    // its operations on the list and the result is contained in every
    // successor's set; the exit's set is accepted.
    for (list, origin) in origins.iter().enumerate() {
        if !initial_states[list].within_set(before[entry][list]) {
            return Err((list, Some(Violation::Unclosed)));
        }
        for (index, node) in nodes.iter().enumerate() {
            let mut states = before[index][list];
            if states == States::EMPTY {
                // Unreachable from entry on this list's projection.
                continue;
            }
            for &(event_list, operation) in node.event.iter().flatten() {
                if event_list == list {
                    states = step(*origin, states, operation)
                        .map_err(|violation| (list, Some(violation)))?;
                }
            }
            if node
                .successors
                .iter()
                .any(|successor| !states.within_set(before[*successor][list]))
            {
                return Err((list, Some(Violation::Unclosed)));
            }
        }
        if !exit_ok(*origin, before[EXIT][list]) {
            return Err((list, None));
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "../tests/unit/va_list_protocol.rs"]
mod tests;
