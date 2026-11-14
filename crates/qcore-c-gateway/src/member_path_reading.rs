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

//! Card **MEMBER-PATH-STATES-THROUGH-UNIT-HELPERS** (gateway rule 30,
//! amended; contract spec §C04.19.6): the bounded path reading behind rule
//! 30's validation of a declaration's member-path states.
//!
//! The declaration's body is lowered to a control-flow graph (`if`,
//! `switch`/`case`/`default`, `while`/`do`/`for`, `break`/`continue`,
//! `goto` and labels, `return`) and read to a fixpoint over a finite value
//! set per stated path of the formal ([`ORIG`] untouched, [`ORIG_NULL`]
//! untouched and tested null, [`NULL`] a null pointer constant stored or a
//! touched value tested null, [`OTHER`] anything). The sets only grow, so the
//! fixpoint is reached; a node budget and a frame depth bound the reading.
//!
//! * A null-pointer test of a stated path (`P->x`, `!P->x`, `P->x == NULL`,
//!   `P->x != NULL`, under `__builtin_expect`) refines the path on each
//!   branch; an empty set is an infeasible branch.
//! * A direct call to a function the unit defines with storage class
//!   `static` (a header `static inline` included), not already being read and
//!   within the depth bound, that receives a **view** of the formal (the
//!   formal itself, or the address of a member path of it) is **followed**:
//!   its body is read from the caller's state with each parameter bound to
//!   its view (the call summary is the callee's reading, equal to inlining
//!   it). `return f(…)` keeps the callee's outcome per returned value, so a
//!   wrapper returning a static worker's result reads as the worker.
//! * Every other call that receives a view (an indirect, external,
//!   non-static or recursive callee, a variadic position, the bound) leaves
//!   every stated path the view reaches [`OTHER`]: a call that is not
//!   followed never validates what it reaches. A member's **value** passed
//!   to a call reaches nothing of the formal (the frame of a call is its
//!   arguments; a pointer to the formal stored in memory is not followed).
//! * The reading **fails** (the clause stays a declared assumption, never a
//!   withdrawal) on a view copied into a variable, a binding reassigned,
//!   effects in two unsequenced operands, a statement or expression it does
//!   not model, a pointer store into an unstated path inside a followed
//!   helper, or the budget.
//!
//! Card **SCHEDULER-DESTROY-MEMBER-STATES** (gateway rule 50): two
//! additions, both bounded and fail-closed.
//!
//! * A store into a path at or below a **link member** the declaration's own
//!   unlink names (`resource_unpublished(P, LINK)`) inside a followed helper
//!   (`mk_list_del(&mth->_head)` clearing `_head.prev` / `_head.next`) is
//!   that clause's, not an unstated store: it no longer ends the reading.
//! * A **taken** slot per stated path and a **copy** slot per qualified local
//!   ([`TAKEN`], [`UNTAKEN`], [`UNSET`]), after the value slots of the
//!   [`State`]: in the declaration's own frame, a direct call to a `static`
//!   function the unit defines that has a [`Helper`] declaration in force and
//!   receives the **value** of a path stated `#stale` (`P->M`, or a local
//!   whose one write in the body is `v = P->M`, read while `M` is untouched)
//!   at the formal its declaration releases is **composed**: the object `M`
//!   held is taken there, once. Any other call, store or return that may
//!   carry that value (in any frame), a second take, a take on only some
//!   paths, a helper not in force, a member invocation whose run set is not
//!   closed and empty, or a pointee that is not the helper's record leaves
//!   the take a declared assumption ([`taken_exits`]).
//!
//! [`validated_exits`] then checks every exit where a state's guard may hold
//! (an exit returning exactly the guard's enumerator is exempt from a `!=`
//! guard): `#released` / `#moved` read null ([`NULL`] or [`ORIG_NULL`]), a
//! set `{V, #kept}` reads no [`OTHER`] and the reading saw a null store into
//! the path, `#stale` / `#kept` read untouched ([`ORIG`] or [`ORIG_NULL`]).

use std::collections::{BTreeMap, BTreeSet};

use qcore_schemas::handoff::GuardOp;
use qcore_schemas::member_path::{Fate, MemberPathState};
use serde_json::Value;

use crate::typed_c_ast::{children, kind};

/// Untouched since entry.
pub const ORIG: u8 = 1;
/// Untouched since entry, and tested null on this path.
pub const ORIG_NULL: u8 = 2;
/// A null pointer constant stored (or a touched value tested null).
pub const NULL: u8 = 4;
/// Anything: a store of another value, a call that is not followed.
pub const OTHER: u8 = 8;

/// Card SCHEDULER-DESTROY-MEMBER-STATES (rule 50): a taken slot's "some
/// path reaching here has not taken the object" bit.
pub const UNTAKEN: u8 = 1;
/// A taken slot's "some path reaching here has taken the object" bit.
pub const TAKEN: u8 = 2;
/// A copy slot's "some path reaching here has not stored the member's value
/// into the local" bit (zero: every path has, from an untouched member).
pub const UNSET: u8 = 1;

/// Frames the reading follows at most (the declaration's body is the first).
pub const MAX_DEPTH: usize = 8;
/// Control-flow nodes the reading evaluates at most, over every frame.
pub const BUDGET: usize = 50_000;

/// What each stated path may read on one path: a mask of [`ORIG`],
/// [`ORIG_NULL`], [`NULL`], [`OTHER`] per stated path (all zero: no path
/// reaches here).
pub type State = Vec<u8>;

/// What an exit returns: the enumerator named by the returned expression,
/// or anything.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Returned {
    Enumerator(String),
    Unknown,
}

/// A function's exits: the joined state per returned value.
pub type Outcomes = BTreeMap<Returned, State>;

/// A view of the formal a binding holds: the object itself (`None`) or the
/// subobject at a member path of it.
type View = Option<String>;

/// One frame's bindings: parameter declaration id to the view it holds.
type Bindings = BTreeMap<String, View>;

/// Card SCHEDULER-DESTROY-MEMBER-STATES (rule 50): the unit's helper
/// declarations by function name.
pub type HelperLookup<'c> = &'c dyn Fn(&str) -> Option<Helper>;

/// Card SCHEDULER-DESTROY-MEMBER-STATES (rule 50): a unit helper's
/// declaration as the reading composes it at a direct call that hands it a
/// stated member's value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Helper {
    /// The formal its member-path states and member invocations bind.
    pub parameter: usize,
    /// The record its states are stated on (`struct TAG`).
    pub tag: String,
    /// Whether it states `resource_released` of that formal.
    pub releases: bool,
    /// `Ok` when each member invocation it states reads a closed empty set
    /// over the run's member facts (it invokes nothing); why not otherwise.
    pub invokes_nothing: Result<(), String>,
    /// Its member-path states, `(path, value spelling)`.
    pub states: Vec<(String, String)>,
    /// The members it invokes.
    pub invocations: Vec<String>,
}

fn join_into(into: &mut State, from: &State) -> bool {
    let mut grew = false;
    for (slot, value) in into.iter_mut().zip(from) {
        let next = *slot | value;
        grew |= next != *slot;
        *slot = next;
    }
    grew
}

fn strip(mut node: &Value) -> &Value {
    loop {
        match kind(node) {
            "ParenExpr" | "ImplicitCastExpr" | "CStyleCastExpr" | "ConstantExpr"
            | "ExprWithCleanups" => match children(node).first() {
                Some(inner) => node = inner,
                None => return node,
            },
            _ => return node,
        }
    }
}

fn strip_keeping_decay(mut node: &Value) -> &Value {
    loop {
        match kind(node) {
            "ImplicitCastExpr" if node["castKind"].as_str() == Some("ArrayToPointerDecay") => {
                return node;
            }
            "ParenExpr" | "ImplicitCastExpr" | "CStyleCastExpr" | "ConstantExpr"
            | "ExprWithCleanups" => match children(node).first() {
                Some(inner) => node = inner,
                None => return node,
            },
            _ => return node,
        }
    }
}

/// `node` is a null pointer constant (rule 30's base reading).
#[must_use]
pub fn null_constant(node: &Value) -> bool {
    match kind(node) {
        "ImplicitValueInitExpr" | "GNUNullExpr" => true,
        "IntegerLiteral" => node["value"].as_str() == Some("0"),
        "ParenExpr" | "ConstantExpr" => children(node).first().is_some_and(null_constant),
        "ImplicitCastExpr" | "CStyleCastExpr"
            if matches!(
                node["castKind"].as_str(),
                Some("NullToPointer" | "IntegralToPointer" | "NoOp" | "BitCast")
            ) =>
        {
            children(node).first().is_some_and(null_constant)
        }
        _ => false,
    }
}

fn is_pointer(node: &Value) -> bool {
    let ty = &node["type"];
    let spelled = ty["desugaredQualType"]
        .as_str()
        .or_else(|| ty["qualType"].as_str())
        .unwrap_or("");
    spelled.trim_end().ends_with('*')
}

fn related(a: &str, b: &str) -> bool {
    a == b || a.starts_with(&format!("{b}.")) || b.starts_with(&format!("{a}."))
}

fn join_path(view: &View, relative: &str) -> String {
    match view {
        None => relative.to_owned(),
        Some(prefix) if relative.is_empty() => prefix.clone(),
        Some(prefix) => format!("{prefix}.{relative}"),
    }
}

fn label_key(node: &Value) -> Option<String> {
    node["targetLabelDeclId"]
        .as_str()
        .or_else(|| node["declId"].as_str())
        .or_else(|| node["name"].as_str())
        .map(str::to_owned)
}

/// Whether evaluating `node` may change state: a call, an assignment or an
/// update.
#[must_use]
pub fn eventful(node: &Value) -> bool {
    match kind(node) {
        "CallExpr" | "CompoundAssignOperator" | "StmtExpr" => true,
        "BinaryOperator" if node["opcode"].as_str() == Some("=") => true,
        "UnaryOperator" if matches!(node["opcode"].as_str(), Some("++" | "--")) => true,
        _ => children(node).iter().any(eventful),
    }
}

/// Whether `node` holds a statement that leaves it other than by its end
/// (`return`, `goto`, a label, `break` / `continue`).
fn contains_transfer(node: &Value) -> bool {
    matches!(
        kind(node),
        "ReturnStmt" | "GotoStmt" | "IndirectGotoStmt" | "LabelStmt" | "BreakStmt" | "ContinueStmt"
    ) || children(node).iter().any(contains_transfer)
}

fn collect<'v>(node: &'v Value, out: &mut Vec<&'v Value>) {
    out.push(node);
    for child in children(node) {
        collect(child, out);
    }
}

const PENDING: usize = usize::MAX;

enum Node<'a> {
    /// A full expression (or declaration), then `next`.
    Eval(&'a Value, usize),
    /// A condition: true, false.
    Branch(&'a Value, usize, usize),
    /// A switch's condition, then any of the targets.
    Multi(&'a Value, Vec<usize>),
    Return(Option<&'a Value>),
    Goto(usize),
}

#[derive(Default)]
struct Graph<'a> {
    nodes: Vec<Node<'a>>,
    labels: BTreeMap<String, usize>,
    gotos: Vec<(usize, String)>,
    breaks: Vec<usize>,
    continues: Vec<usize>,
    cases: Vec<Vec<usize>>,
    defaults: Vec<Option<usize>>,
    error: Option<String>,
}

impl<'a> Graph<'a> {
    fn push(&mut self, node: Node<'a>) -> usize {
        self.nodes.push(node);
        self.nodes.len() - 1
    }

    fn fail(&mut self, why: String, next: usize) -> usize {
        self.error.get_or_insert(why);
        next
    }

    fn present(node: &Value) -> bool {
        node.as_object().is_some_and(|object| !object.is_empty())
    }

    fn lower(&mut self, statement: &'a Value, next: usize) -> usize {
        let inner = children(statement);
        match kind(statement) {
            "CompoundStmt" => inner
                .iter()
                .rev()
                .fold(next, |next, child| self.lower(child, next)),
            "NullStmt" => next,
            "AttributedStmt" => match inner.last() {
                Some(sub) => self.lower(sub, next),
                None => next,
            },
            "DeclStmt" => self.push(Node::Eval(statement, next)),
            "ReturnStmt" => self.push(Node::Return(inner.first())),
            "IfStmt" => {
                let (Some(condition), Some(then)) = (inner.first(), inner.get(1)) else {
                    return self.fail("an `if` the reading does not read".into(), next);
                };
                if inner.len() > 3 {
                    return self.fail("an `if` with an initializer".into(), next);
                }
                let then = self.lower(then, next);
                let otherwise = match inner.get(2) {
                    Some(otherwise) => self.lower(otherwise, next),
                    None => next,
                };
                self.push(Node::Branch(condition, then, otherwise))
            }
            "WhileStmt" => {
                let ([condition, body] | [_, condition, body]) = inner else {
                    return self.fail("a `while` the reading does not read".into(), next);
                };
                if inner.len() != 2 {
                    return self.fail("a `while` condition variable".into(), next);
                }
                let header = self.push(Node::Branch(condition, PENDING, next));
                self.breaks.push(next);
                self.continues.push(header);
                let body = self.lower(body, header);
                self.breaks.pop();
                self.continues.pop();
                self.nodes[header] = Node::Branch(condition, body, next);
                header
            }
            "DoStmt" => {
                let [body, condition] = inner else {
                    return self.fail("a `do` the reading does not read".into(), next);
                };
                let test = self.push(Node::Branch(condition, PENDING, next));
                self.breaks.push(next);
                self.continues.push(test);
                let body = self.lower(body, test);
                self.breaks.pop();
                self.continues.pop();
                self.nodes[test] = Node::Branch(condition, body, next);
                body
            }
            "ForStmt" => {
                let [init, variable, condition, increment, body] = inner else {
                    return self.fail("a `for` the reading does not read".into(), next);
                };
                if Self::present(variable) {
                    return self.fail("a `for` condition variable".into(), next);
                }
                let tested = Self::present(condition);
                let header = if tested {
                    self.push(Node::Branch(condition, PENDING, next))
                } else {
                    self.push(Node::Goto(PENDING))
                };
                let step = if Self::present(increment) {
                    self.push(Node::Eval(increment, header))
                } else {
                    header
                };
                self.breaks.push(next);
                self.continues.push(step);
                let body = self.lower(body, step);
                self.breaks.pop();
                self.continues.pop();
                self.nodes[header] = if tested {
                    Node::Branch(condition, body, next)
                } else {
                    Node::Goto(body)
                };
                if Self::present(init) {
                    self.lower(init, header)
                } else {
                    header
                }
            }
            "SwitchStmt" => {
                let [condition, body] = inner else {
                    return self.fail("a `switch` the reading does not read".into(), next);
                };
                let switch = self.push(Node::Multi(condition, Vec::new()));
                self.cases.push(Vec::new());
                self.defaults.push(None);
                self.breaks.push(next);
                self.lower(body, next);
                self.breaks.pop();
                let mut targets = self.cases.pop().unwrap_or_default();
                targets.push(self.defaults.pop().flatten().unwrap_or(next));
                self.nodes[switch] = Node::Multi(condition, targets);
                switch
            }
            "CaseStmt" | "DefaultStmt" => {
                let Some(sub) = inner.last() else {
                    return self.fail("a `case` the reading does not read".into(), next);
                };
                let sub = self.lower(sub, next);
                let entry = self.push(Node::Goto(sub));
                let recorded = if kind(statement) == "CaseStmt" {
                    self.cases
                        .last_mut()
                        .map(|cases| cases.push(entry))
                        .is_some()
                } else {
                    self.defaults
                        .last_mut()
                        .map(|default| *default = Some(entry))
                        .is_some()
                };
                if !recorded {
                    return self.fail("a `case` outside a `switch`".into(), next);
                }
                entry
            }
            "BreakStmt" => match self.breaks.last() {
                Some(&target) => target,
                None => self.fail("a `break` outside a loop or `switch`".into(), next),
            },
            "ContinueStmt" => match self.continues.last() {
                Some(&target) => target,
                None => self.fail("a `continue` outside a loop".into(), next),
            },
            "GotoStmt" => {
                let Some(key) = label_key(statement) else {
                    return self.fail("a `goto` without its label".into(), next);
                };
                let id = self.push(Node::Goto(PENDING));
                self.gotos.push((id, key));
                id
            }
            "LabelStmt" => {
                let Some(key) = label_key(statement) else {
                    return self.fail("a label without its name".into(), next);
                };
                let sub = match inner.last() {
                    Some(sub) => self.lower(sub, next),
                    None => next,
                };
                let id = self.push(Node::Goto(sub));
                self.labels.insert(key, id);
                id
            }
            other if other.ends_with("Stmt") => self.fail(
                format!("a statement the reading does not model: {other}"),
                next,
            ),
            _ => self.push(Node::Eval(statement, next)),
        }
    }

    /// The function body's graph and its entry, or why it is not read.
    fn of(body: &'a Value) -> Result<(Self, usize), String> {
        let mut graph = Self::default();
        let end = graph.push(Node::Return(None));
        let entry = graph.lower(body, end);
        if let Some(error) = graph.error.take() {
            return Err(error);
        }
        for (id, key) in std::mem::take(&mut graph.gotos) {
            match graph.labels.get(&key) {
                Some(&target) => graph.nodes[id] = Node::Goto(target),
                None => return Err(format!("a `goto` to an unread label {key}")),
            }
        }
        Ok((graph, entry))
    }
}

/// The reading of one formal's stated paths.
pub struct Reader<'a, 'c> {
    paths: Vec<String>,
    callee: &'c dyn Fn(&str) -> Option<&'a Value>,
    stack: Vec<String>,
    spent: usize,
    /// Stated paths a null store into was read.
    pub stored: BTreeSet<usize>,
    /// The calls that received a view and were not followed.
    pub not_followed: BTreeSet<String>,
    /// Why the reading failed.
    pub failure: Option<String>,
    /// Card SCHEDULER-DESTROY-MEMBER-STATES (rule 50): the link members the
    /// declaration's own unlink names; a store at or below one inside a
    /// followed helper is that clause's.
    linked: Vec<String>,
    /// The unit's helper declarations (rule 50), when the reading composes
    /// them.
    helper: Option<HelperLookup<'c>>,
    /// Whether each stated path is stated `#stale` (a take is read there).
    stale: Vec<bool>,
    /// Qualified locals of the declaration's own frame: declaration id to
    /// (stated path index, copy slot).
    copies: BTreeMap<String, (usize, usize)>,
    /// The width of a state: value slots, taken slots, copy slots.
    width: usize,
    /// What the composed helpers state at the taken objects (rule 50).
    pub composed: BTreeSet<String>,
    /// Why a take was not composed or was spoilt (rule 50).
    pub take_failures: BTreeSet<String>,
}

impl<'a, 'c> Reader<'a, 'c> {
    /// A reading of `paths` (the stated paths of one formal), following the
    /// unit's functions `callee` names.
    #[must_use]
    pub fn new(paths: Vec<String>, callee: &'c dyn Fn(&str) -> Option<&'a Value>) -> Self {
        Self {
            paths,
            callee,
            stack: Vec::new(),
            spent: 0,
            stored: BTreeSet::new(),
            not_followed: BTreeSet::new(),
            failure: None,
            linked: Vec::new(),
            helper: None,
            stale: Vec::new(),
            copies: BTreeMap::new(),
            width: 0,
            composed: BTreeSet::new(),
            take_failures: BTreeSet::new(),
        }
        .sized()
    }

    fn sized(mut self) -> Self {
        let paths = self.paths.len();
        self.stale.resize(paths, false);
        self.width = 2 * paths + self.copies.len();
        self
    }

    /// Card SCHEDULER-DESTROY-MEMBER-STATES (rule 50): the link members the
    /// declaration's own unlink names.
    #[must_use]
    pub fn with_link_members(mut self, linked: Vec<String>) -> Self {
        self.linked = linked;
        self
    }

    /// Card SCHEDULER-DESTROY-MEMBER-STATES (rule 50): compose the unit's
    /// helper declarations at the stated `#stale` paths (`stale`, one flag
    /// per stated path).
    #[must_use]
    pub fn with_helpers(mut self, helper: HelperLookup<'c>, stale: Vec<bool>) -> Self {
        self.helper = Some(helper);
        self.stale = stale;
        self.sized()
    }

    /// The entry state: every stated path untouched and not taken. The
    /// qualified locals' copy slots are added when the declaration's own
    /// frame is read ([`Self::read`]).
    #[must_use]
    pub fn entry(&self) -> State {
        let paths = self.paths.len();
        let mut state = vec![ORIG; paths];
        state.extend(std::iter::repeat_n(UNTAKEN, paths));
        state
    }

    fn fail<T>(&mut self, why: String) -> Option<T> {
        self.failure.get_or_insert(why);
        None
    }

    /// The function's exits from `entry`, its parameter at each position of
    /// `views` bound to that view; `None` when the reading failed.
    pub fn read(
        &mut self,
        name: &str,
        function: &'a Value,
        views: &BTreeMap<usize, View>,
        entry: State,
    ) -> Option<Outcomes> {
        let parameters = children(function)
            .iter()
            .filter(|child| kind(child) == "ParmVarDecl")
            .collect::<Vec<_>>();
        let mut bindings = Bindings::new();
        for (&position, view) in views {
            let Some(id) = parameters
                .get(position)
                .and_then(|parameter| parameter["id"].as_str())
            else {
                return self.fail(format!("{name}'s parameter {position} is not read"));
            };
            bindings.insert(id.to_owned(), view.clone());
        }
        let Some(body) = children(function)
            .iter()
            .rev()
            .find(|child| kind(child) == "CompoundStmt")
        else {
            return self.fail(format!("{name}'s body is not read"));
        };
        let (graph, start) = match Graph::of(body) {
            Ok(graph) => graph,
            Err(why) => return self.fail(format!("{name}: {why}")),
        };
        let mut entry = entry;
        if self.stack.is_empty() {
            if self.helper.is_some() {
                self.find_copies(body, &bindings);
            }
            entry.resize(2 * self.paths.len(), UNTAKEN);
            entry.extend(std::iter::repeat_n(UNSET, self.copies.len()));
            self.width = entry.len();
        }
        self.stack.push(name.to_owned());
        let outcomes = self.fixpoint(&graph, start, entry, &bindings);
        self.stack.pop();
        outcomes
    }

    fn fixpoint(
        &mut self,
        graph: &Graph<'a>,
        start: usize,
        entry: State,
        bindings: &Bindings,
    ) -> Option<Outcomes> {
        let mut states: Vec<Option<State>> = vec![None; graph.nodes.len()];
        states[start] = Some(entry);
        let mut work = BTreeSet::from([start]);
        let mut outcomes = Outcomes::new();
        let flow = |states: &mut Vec<Option<State>>,
                    work: &mut BTreeSet<usize>,
                    target: usize,
                    state: State| {
            let grew = match &mut states[target] {
                Some(present) => join_into(present, &state),
                slot @ None => {
                    *slot = Some(state);
                    true
                }
            };
            if grew {
                work.insert(target);
            }
        };
        while let Some(id) = work.pop_first() {
            self.spent += 1;
            if self.spent > BUDGET {
                return self.fail(format!("the reading's bound of {BUDGET} nodes"));
            }
            let Some(state) = states[id].clone() else {
                continue;
            };
            match &graph.nodes[id] {
                Node::Eval(expression, next) => {
                    let after = self.statement(expression, state, bindings)?;
                    flow(&mut states, &mut work, *next, after);
                }
                Node::Branch(condition, then, otherwise) => {
                    let after = self.walk(condition, state, false, bindings)?;
                    let (yes, no) = self.refine(condition, &after, bindings);
                    if let Some(yes) = yes {
                        flow(&mut states, &mut work, *then, yes);
                    }
                    if let Some(no) = no {
                        flow(&mut states, &mut work, *otherwise, no);
                    }
                }
                Node::Multi(condition, targets) => {
                    let after = self.walk(condition, state, false, bindings)?;
                    for &target in targets {
                        flow(&mut states, &mut work, target, after.clone());
                    }
                }
                Node::Goto(target) => flow(&mut states, &mut work, *target, state),
                Node::Return(expression) => {
                    for (returned, state) in self.returned(*expression, state, bindings)? {
                        match outcomes.get_mut(&returned) {
                            Some(present) => {
                                join_into(present, &state);
                            }
                            None => {
                                outcomes.insert(returned, state);
                            }
                        }
                    }
                }
            }
        }
        Some(outcomes)
    }

    /// A `return`'s outcomes: a followed callee's per returned value, else the
    /// returned enumerator or anything.
    fn returned(
        &mut self,
        expression: Option<&'a Value>,
        state: State,
        bindings: &Bindings,
    ) -> Option<Vec<(Returned, State)>> {
        let Some(expression) = expression else {
            return Some(vec![(Returned::Unknown, state)]);
        };
        let stripped = strip(expression);
        if kind(stripped) == "CallExpr" && !children(stripped).iter().any(eventful) {
            // Rule 50: the call's member values are read before it is
            // followed (a followed call bypasses `call`).
            let composed = self.compose(stripped, state.clone(), false, bindings);
            if let Some(outcomes) = self.followed(stripped, &composed, bindings) {
                return Some(outcomes?.into_iter().collect());
            }
        }
        let after = self.walk(expression, state, false, bindings)?;
        let after = self.escape(expression, after, "a `return`", bindings);
        let returned = if kind(stripped) == "DeclRefExpr"
            && stripped["referencedDecl"]["kind"].as_str() == Some("EnumConstantDecl")
        {
            stripped["referencedDecl"]["name"]
                .as_str()
                .map_or(Returned::Unknown, |name| {
                    Returned::Enumerator(name.to_owned())
                })
        } else {
            Returned::Unknown
        };
        Some(vec![(returned, after)])
    }

    /// The path below a binding that `node` designates (`P->a.b` and
    /// `(*P).a.b` are the binding's view joined with `a.b`), `None` for
    /// anything else.
    fn path_of(&self, node: &Value, bindings: &Bindings) -> Option<String> {
        let mut segments = Vec::new();
        let mut node = strip(node);
        loop {
            if kind(node) != "MemberExpr" {
                return None;
            }
            segments.push(node["name"].as_str()?.to_owned());
            let base = strip(children(node).first()?);
            let root = if node["isArrow"].as_bool() == Some(true) {
                Some(base)
            } else if kind(base) == "UnaryOperator" && base["opcode"].as_str() == Some("*") {
                children(base).first().map(strip)
            } else {
                None
            };
            if let Some(root) = root {
                let view = self.binding(root, bindings)?;
                segments.reverse();
                return Some(join_path(view, &segments.join(".")));
            }
            node = base;
        }
    }

    fn binding<'b>(&self, node: &Value, bindings: &'b Bindings) -> Option<&'b View> {
        if kind(node) != "DeclRefExpr" {
            return None;
        }
        bindings.get(node["referencedDecl"]["id"].as_str()?)
    }

    /// The views of the formal an expression's **value** may hold: a
    /// binding, the address of (or an array decaying from) a member path
    /// below one, pointer arithmetic and `?:` over them, and the value of a
    /// pointer-typed call that received one (a callee may return what it
    /// received). A member's value, an element's value, a comparison or a
    /// test holds none.
    fn views_of(&self, node: &Value, bindings: &Bindings) -> Vec<View> {
        let mut views = Vec::new();
        self.value_views(node, bindings, &mut views);
        views
    }

    fn value_views(&self, node: &Value, bindings: &Bindings, views: &mut Vec<View>) {
        let node = strip_keeping_decay(node);
        if let Some(view) = self.binding(node, bindings) {
            views.push(view.clone());
            return;
        }
        let inner = children(node);
        match kind(node) {
            // An array decaying to its first element's address.
            "ImplicitCastExpr" => {
                if let Some(array) = inner.first() {
                    self.address_views(array, bindings, views);
                }
            }
            "UnaryOperator" => match node["opcode"].as_str() {
                Some("&") => {
                    if let Some(operand) = inner.first() {
                        self.address_views(operand, bindings, views);
                    }
                }
                Some("++" | "--") => {
                    if let Some(operand) = inner.first() {
                        self.value_views(operand, bindings, views);
                    }
                }
                _ => {}
            },
            "BinaryOperator" => match node["opcode"].as_str() {
                Some("+" | "-") => {
                    for operand in inner {
                        self.value_views(operand, bindings, views);
                    }
                }
                Some("," | "=") => {
                    if let Some(right) = inner.last() {
                        self.value_views(right, bindings, views);
                    }
                }
                _ => {}
            },
            "ConditionalOperator" => {
                for arm in inner.iter().skip(1) {
                    self.value_views(arm, bindings, views);
                }
            }
            "CallExpr" => {
                if is_pointer(node) {
                    for argument in inner.iter().skip(1) {
                        self.value_views(argument, bindings, views);
                    }
                }
            }
            "MemberExpr"
            | "ArraySubscriptExpr"
            | "DeclRefExpr"
            | "IntegerLiteral"
            | "CharacterLiteral"
            | "FloatingLiteral"
            | "StringLiteral"
            | "UnaryExprOrTypeTraitExpr"
            | "CompoundAssignOperator"
            | "PredefinedExpr"
            | "OffsetOfExpr" => {}
            _ => {
                for child in inner {
                    self.value_views(child, bindings, views);
                }
            }
        }
    }

    /// The views `&lvalue` holds.
    fn address_views(&self, lvalue: &Value, bindings: &Bindings, views: &mut Vec<View>) {
        if let Some(path) = self.path_of(lvalue, bindings) {
            views.push(Some(path));
            return;
        }
        self.object_views(lvalue, bindings, views);
    }

    /// The views of the object an lvalue that is not an exact path lies in
    /// (`*E`, `E[i]`, `E->m` with `E` holding a view, and a member of such).
    fn object_views(&self, lvalue: &Value, bindings: &Bindings, views: &mut Vec<View>) {
        let lvalue = strip(lvalue);
        let inner = children(lvalue);
        match kind(lvalue) {
            "UnaryOperator" if lvalue["opcode"].as_str() == Some("*") => {
                if let Some(pointer) = inner.first() {
                    self.value_views(pointer, bindings, views);
                }
            }
            "ArraySubscriptExpr" => {
                for operand in inner {
                    self.value_views(operand, bindings, views);
                }
            }
            "MemberExpr" => {
                if let Some(base) = inner.first() {
                    if lvalue["isArrow"].as_bool() == Some(true) {
                        self.value_views(base, bindings, views);
                    } else {
                        self.object_views(base, bindings, views);
                    }
                }
            }
            "CompoundLiteralExpr" => {
                for child in inner {
                    self.value_views(child, bindings, views);
                }
            }
            _ => {}
        }
    }

    /// Whether evaluating `node` may change what a stated path reads: a call
    /// that receives a view, a store into (or into the object of) a binding,
    /// a statement expression. Operands whose effects reach no binding are
    /// unordered freely: no order changes a stated path's reading.
    fn touches(&self, node: &Value, bindings: &Bindings) -> bool {
        let written = |target: &Value| {
            let mut views = Vec::new();
            self.object_views(target, bindings, &mut views);
            self.path_of(target, bindings).is_some()
                || self.binding(strip(target), bindings).is_some()
                || !views.is_empty()
        };
        let here = match kind(node) {
            "StmtExpr" => true,
            "CallExpr" => !self.argument_views(node, bindings).is_empty(),
            "BinaryOperator" if node["opcode"].as_str() == Some("=") => {
                children(node).first().is_some_and(written)
            }
            "CompoundAssignOperator" => children(node).first().is_some_and(written),
            "UnaryOperator" if matches!(node["opcode"].as_str(), Some("++" | "--")) => {
                children(node).first().is_some_and(written)
            }
            _ => false,
        };
        here || children(node)
            .iter()
            .any(|child| self.touches(child, bindings))
    }

    /// Paths a view reaches.
    fn reached(&self, view: &View) -> Vec<usize> {
        (0..self.paths.len())
            .filter(|&index| match view {
                None => true,
                Some(prefix) => related(&self.paths[index], prefix),
            })
            .collect()
    }

    /// A statement node: a declaration (its initializers) or an expression.
    fn statement(&mut self, node: &'a Value, state: State, bindings: &Bindings) -> Option<State> {
        if kind(node) != "DeclStmt" {
            return self.walk(node, state, false, bindings);
        }
        let mut state = state;
        for declaration in children(node) {
            if kind(declaration) != "VarDecl" {
                continue;
            }
            for init in children(declaration) {
                if !self.views_of(init, bindings).is_empty() {
                    return self.fail(format!(
                        "a view of the formal is copied into the variable `{}`",
                        declaration["name"].as_str().unwrap_or("?")
                    ));
                }
                state = self.walk(init, state, false, bindings)?;
                let copy = declaration["id"]
                    .as_str()
                    .and_then(|id| self.copies.get(id).copied())
                    .filter(|_| self.stack.len() == 1);
                state = self.write_copy_or_escape(copy, init, state, false, bindings);
            }
        }
        Some(state)
    }

    /// The state after evaluating `node`; `maybe` when the evaluation may not
    /// happen (the right of `&&` / `||`, an arm of `?:`).
    fn walk(
        &mut self,
        node: &'a Value,
        state: State,
        maybe: bool,
        bindings: &Bindings,
    ) -> Option<State> {
        let inner = children(node);
        match kind(node) {
            // A statement expression (glibc's `assert`) with no transfer out
            // of it: its statements read as a nested graph from this state.
            "StmtExpr" if !contains_transfer(node) => {
                let Some(body) = inner.first() else {
                    return self.fail("a statement expression without its body".into());
                };
                let (graph, start) = match Graph::of(body) {
                    Ok(graph) => graph,
                    Err(why) => return self.fail(format!("a statement expression: {why}")),
                };
                let before = state.clone();
                let outcomes = self.fixpoint(&graph, start, state, bindings)?;
                let mut after = if maybe { before } else { vec![0; self.width] };
                for exit in outcomes.values() {
                    join_into(&mut after, exit);
                }
                Some(after)
            }
            "StmtExpr" | "BinaryConditionalOperator" | "ChooseExpr" | "GenericSelectionExpr" => {
                self.fail(format!(
                    "an expression the reading does not model: {}",
                    kind(node)
                ))
            }
            "UnaryExprOrTypeTraitExpr" => Some(state),
            "BinaryOperator" if matches!(node["opcode"].as_str(), Some("&&" | "||")) => {
                let [left, right] = inner else {
                    return self.fail("an operator the reading does not read".into());
                };
                let state = self.walk(left, state, maybe, bindings)?;
                self.walk(right, state, true, bindings)
            }
            "BinaryOperator" if node["opcode"].as_str() == Some(",") => {
                let [left, right] = inner else {
                    return self.fail("an operator the reading does not read".into());
                };
                let state = self.walk(left, state, maybe, bindings)?;
                self.walk(right, state, maybe, bindings)
            }
            "ConditionalOperator" => {
                let [condition, then, otherwise] = inner else {
                    return self.fail("an operator the reading does not read".into());
                };
                let state = self.walk(condition, state, maybe, bindings)?;
                let state = self.walk(then, state, true, bindings)?;
                self.walk(otherwise, state, true, bindings)
            }
            "BinaryOperator" if node["opcode"].as_str() == Some("=") => {
                let [left, right] = inner else {
                    return self.fail("an assignment the reading does not read".into());
                };
                if self.touches(left, bindings) && self.touches(right, bindings) {
                    return self.fail("effects in two unsequenced operands".into());
                }
                let state = self.walk(right, state, maybe, bindings)?;
                let state = self.walk(left, state, maybe, bindings)?;
                self.store(left, Some(right), state, maybe, bindings)
            }
            "CompoundAssignOperator" => {
                let [left, right] = inner else {
                    return self.fail("an assignment the reading does not read".into());
                };
                if self.touches(left, bindings) && self.touches(right, bindings) {
                    return self.fail("effects in two unsequenced operands".into());
                }
                let state = self.walk(right, state, maybe, bindings)?;
                let state = self.walk(left, state, maybe, bindings)?;
                self.store(left, None, state, maybe, bindings)
            }
            "UnaryOperator" if matches!(node["opcode"].as_str(), Some("++" | "--")) => {
                let Some(operand) = inner.first() else {
                    return self.fail("an operator the reading does not read".into());
                };
                let state = self.walk(operand, state, maybe, bindings)?;
                self.store(operand, None, state, maybe, bindings)
            }
            "CallExpr" => {
                if inner
                    .iter()
                    .filter(|child| self.touches(child, bindings))
                    .count()
                    > 1
                {
                    return self.fail("effects in two unsequenced call operands".into());
                }
                let mut state = state;
                for child in inner {
                    state = self.walk(child, state, maybe, bindings)?;
                }
                self.call(node, state, maybe, bindings)
            }
            _ => {
                if inner
                    .iter()
                    .filter(|child| self.touches(child, bindings))
                    .count()
                    > 1
                {
                    return self.fail("effects in two unsequenced operands".into());
                }
                let mut state = state;
                for child in inner {
                    state = self.walk(child, state, maybe, bindings)?;
                }
                Some(state)
            }
        }
    }

    /// A store into `target` (`value` for `=`, `None` for an update).
    fn store(
        &mut self,
        target: &Value,
        value: Option<&Value>,
        mut state: State,
        maybe: bool,
        bindings: &Bindings,
    ) -> Option<State> {
        let stripped = strip(target);
        if self.binding(stripped, bindings).is_some() {
            return self.fail("a binding of the formal is reassigned".into());
        }
        if let Some(value) = value
            && kind(stripped) == "DeclRefExpr"
            && !self.views_of(value, bindings).is_empty()
        {
            return self.fail(format!(
                "a view of the formal is copied into the variable `{}`",
                stripped["referencedDecl"]["name"].as_str().unwrap_or("?")
            ));
        }
        if let Some(value) = value {
            let copy = (kind(stripped) == "DeclRefExpr")
                .then(|| stripped["referencedDecl"]["id"].as_str())
                .flatten()
                .and_then(|id| self.copies.get(id).copied())
                .filter(|_| self.stack.len() == 1);
            state = self.write_copy_or_escape(copy, value, state, maybe, bindings);
        }
        let null = value.is_some_and(null_constant);
        let mut touched = false;
        let written = if let Some(path) = self.path_of(stripped, bindings) {
            for (index, (stated, value)) in self.paths.iter().zip(state.iter_mut()).enumerate() {
                if *stated == path && null {
                    *value = if maybe { *value | NULL } else { NULL };
                    self.stored.insert(index);
                    touched = true;
                } else if related(stated, &path) {
                    *value |= OTHER;
                    touched = true;
                }
            }
            Some(path)
        } else {
            let mut views = Vec::new();
            self.object_views(stripped, bindings, &mut views);
            for view in &views {
                for index in self.reached(view) {
                    state[index] |= OTHER;
                    touched = true;
                }
            }
            views.first().map(|view| join_path(view, "…"))
        };
        if let Some(path) = written
            && !touched
            && is_pointer(target)
            && self.stack.len() > 1
            && !self
                .linked
                .iter()
                .any(|link| path == *link || path.starts_with(&format!("{link}.")))
        {
            return self.fail(format!(
                "`{}` stores a pointer into the unstated path `{path}`",
                self.stack.last().map_or("a helper", String::as_str)
            ));
        }
        Some(state)
    }

    /// A call's direct callee, when the call names a function.
    fn direct_callee(call: &Value) -> Option<&str> {
        let callee = strip(children(call).first()?);
        if kind(callee) == "DeclRefExpr"
            && callee["referencedDecl"]["kind"].as_str() == Some("FunctionDecl")
        {
            return callee["referencedDecl"]["name"].as_str();
        }
        None
    }

    /// The views a call's arguments pass, by argument position.
    fn argument_views(&self, call: &Value, bindings: &Bindings) -> BTreeMap<usize, Vec<View>> {
        children(call)
            .iter()
            .skip(1)
            .enumerate()
            .filter_map(|(position, argument)| {
                let views = self.views_of(argument, bindings);
                (!views.is_empty()).then_some((position, views))
            })
            .collect()
    }

    /// The followed callee's outcomes, `Some(None)` when its reading failed,
    /// `None` when the call is not followed.
    fn followed(
        &mut self,
        call: &Value,
        state: &State,
        bindings: &Bindings,
    ) -> Option<Option<Outcomes>> {
        let arguments = self.argument_views(call, bindings);
        if arguments.is_empty() {
            return None;
        }
        let name = Self::direct_callee(call)?;
        let function = (self.callee)(name)?;
        if function["storageClass"].as_str() != Some("static")
            || self.stack.iter().any(|frame| frame == name)
            || self.stack.len() >= MAX_DEPTH
        {
            return None;
        }
        let parameters = children(function)
            .iter()
            .filter(|child| kind(child) == "ParmVarDecl")
            .count();
        let mut views = BTreeMap::new();
        for (position, mut passed) in arguments {
            if position >= parameters || passed.len() != 1 {
                return None;
            }
            views.insert(position, passed.pop().flatten());
        }
        Some(self.read(name, function, &views, state.clone()))
    }

    fn call(
        &mut self,
        call: &Value,
        state: State,
        maybe: bool,
        bindings: &Bindings,
    ) -> Option<State> {
        let state = self.compose(call, state, maybe, bindings);
        let arguments = self.argument_views(call, bindings);
        if arguments.is_empty() {
            return Some(state);
        }
        if let Some(outcomes) = self.followed(call, &state, bindings) {
            let mut after = if maybe { state } else { vec![0; self.width] };
            for exit in outcomes?.values() {
                join_into(&mut after, exit);
            }
            return Some(after);
        }
        let name = Self::direct_callee(call)
            .map_or_else(|| "an indirect call".to_owned(), |name| format!("`{name}`"));
        let frame = self.stack.last().cloned().unwrap_or_default();
        let mut state = state;
        for views in arguments.values() {
            for view in views {
                for index in self.reached(view) {
                    state[index] |= OTHER;
                    self.not_followed.insert(format!(
                        "{name} in `{frame}` reaches `{}`",
                        self.paths[index]
                    ));
                }
            }
        }
        Some(state)
    }

    // Card SCHEDULER-DESTROY-MEMBER-STATES (rule 50): takes through a
    // composed helper.

    /// The qualified locals of the declaration's own body: a local
    /// (non-`static`) pointer variable whose only write is one assignment or
    /// initializer of the value of a path stated `#stale`, whose address is
    /// never taken and which is never updated.
    fn find_copies(&mut self, body: &Value, bindings: &Bindings) {
        let mut nodes = Vec::new();
        collect(body, &mut nodes);
        let mut locals = BTreeSet::new();
        let mut writes: BTreeMap<String, Vec<&Value>> = BTreeMap::new();
        let mut spoilt = BTreeSet::new();
        for node in &nodes {
            let target = |node: &Value| {
                children(node)
                    .first()
                    .map(strip)
                    .filter(|target| kind(target) == "DeclRefExpr")
                    .and_then(|target| target["referencedDecl"]["id"].as_str())
                    .map(str::to_owned)
            };
            match kind(node) {
                "VarDecl" => {
                    if let Some(id) = node["id"].as_str()
                        && node["storageClass"].as_str().is_none()
                    {
                        locals.insert(id.to_owned());
                        if let Some(init) = children(node).first() {
                            writes.entry(id.to_owned()).or_default().push(init);
                        }
                    }
                }
                "BinaryOperator" if node["opcode"].as_str() == Some("=") => {
                    if let (Some(id), Some(value)) = (target(node), children(node).get(1)) {
                        writes.entry(id).or_default().push(value);
                    }
                }
                "CompoundAssignOperator" => spoilt.extend(target(node)),
                "UnaryOperator" if matches!(node["opcode"].as_str(), Some("++" | "--" | "&")) => {
                    spoilt.extend(target(node));
                }
                _ => {}
            }
        }
        for (id, values) in writes {
            if !locals.contains(&id) || spoilt.contains(&id) {
                continue;
            }
            let [value] = values.as_slice() else {
                continue;
            };
            let Some(index) = self
                .path_of(value, bindings)
                .and_then(|path| self.paths.iter().position(|stated| *stated == path))
                .filter(|&index| self.stale[index])
            else {
                continue;
            };
            let slot = 2 * self.paths.len() + self.copies.len();
            self.copies.insert(id, (index, slot));
        }
    }

    /// The stated `#stale` paths whose value `node` may carry: the path's
    /// value itself, a qualified local, and any expression built from them
    /// other than through a member access, a dereference or a call (a call
    /// that receives the value is read at the call).
    fn carried(&self, node: &Value, bindings: &Bindings) -> BTreeSet<usize> {
        let node = strip(node);
        let mut out = BTreeSet::new();
        if let Some(index) = self
            .path_of(node, bindings)
            .and_then(|path| self.paths.iter().position(|stated| *stated == path))
            .filter(|&index| self.stale.get(index).copied().unwrap_or(false))
        {
            out.insert(index);
            return out;
        }
        if kind(node) == "DeclRefExpr" {
            if let Some((index, _)) = node["referencedDecl"]["id"]
                .as_str()
                .and_then(|id| self.copies.get(id))
            {
                out.insert(*index);
            }
            return out;
        }
        match kind(node) {
            "MemberExpr" | "ArraySubscriptExpr" | "CallExpr" | "UnaryExprOrTypeTraitExpr" => {}
            "UnaryOperator" if node["opcode"].as_str() == Some("*") => {}
            _ => {
                for child in children(node) {
                    out.extend(self.carried(child, bindings));
                }
            }
        }
        out
    }

    /// Spoil the take at every stated path `node` may carry the value of:
    /// the value reaches something the reading does not compose.
    fn escape(&mut self, node: &Value, mut state: State, what: &str, bindings: &Bindings) -> State {
        if self.helper.is_none() {
            return state;
        }
        let frame = self.stack.last().cloned().unwrap_or_default();
        for index in self.carried(node, bindings) {
            state[self.paths.len() + index] |= UNTAKEN | TAKEN;
            self.take_failures.insert(format!(
                "{what} in `{frame}` carries the value of `{}`",
                self.paths[index]
            ));
        }
        state
    }

    /// A store of `value`: into the qualified local `copy` (its one write),
    /// set from an untouched member; anywhere else, an escape of what it
    /// carries.
    fn write_copy_or_escape(
        &mut self,
        copy: Option<(usize, usize)>,
        value: &Value,
        mut state: State,
        maybe: bool,
        bindings: &Bindings,
    ) -> State {
        if let Some((index, slot)) = copy {
            let from_untouched = self.path_of(value, bindings).as_deref()
                == Some(self.paths[index].as_str())
                && state[index] & !(ORIG | ORIG_NULL) == 0;
            let set = if from_untouched { 0 } else { UNSET };
            state[slot] = if maybe { state[slot] | set } else { set };
            return state;
        }
        self.escape(value, state, "a store", bindings)
    }

    /// The stated `#stale` path whose untouched value `argument` holds, read
    /// directly (`P->M`) or through a qualified local every path set.
    fn member_value(&self, argument: &Value, state: &State, bindings: &Bindings) -> Option<usize> {
        let argument = strip(argument);
        if kind(argument) == "DeclRefExpr" {
            let (index, slot) = *self
                .copies
                .get(argument["referencedDecl"]["id"].as_str()?)?;
            return (state[slot] == 0).then_some(index);
        }
        let index = self
            .path_of(argument, bindings)
            .and_then(|path| self.paths.iter().position(|stated| *stated == path))?;
        (self.stale[index] && state[index] & !(ORIG | ORIG_NULL) == 0).then_some(index)
    }

    /// Rule 50 at a call: a composed take, or an escape of every carried
    /// value.
    fn compose(
        &mut self,
        call: &Value,
        mut state: State,
        maybe: bool,
        bindings: &Bindings,
    ) -> State {
        if self.helper.is_none() {
            return state;
        }
        let arguments = children(call).iter().skip(1).collect::<Vec<_>>();
        let carried = arguments
            .iter()
            .enumerate()
            .filter(|(_, argument)| !self.carried(argument, bindings).is_empty())
            .map(|(position, _)| position)
            .collect::<Vec<_>>();
        if carried.is_empty() {
            return state;
        }
        let name = Self::direct_callee(call).map(str::to_owned);
        let label = name
            .as_deref()
            .map_or_else(|| "an indirect call".to_owned(), |name| format!("`{name}`"));
        let helper = self.admitted_helper(name.as_deref());
        for position in carried {
            let argument = arguments[position];
            let take = match (&helper, self.member_value(argument, &state, bindings)) {
                (Ok(helper), Some(index)) => self
                    .admits(helper, position, argument)
                    .map(|()| (helper.clone(), index)),
                (Ok(_), None) => Err(
                    "the argument is not the untouched value of a stated `#stale` path or of                      its one copy"
                        .to_owned(),
                ),
                (Err(why), _) => Err(why.clone()),
            };
            match take {
                Ok((helper, index)) => {
                    let slot = self.paths.len() + index;
                    if state[slot] & TAKEN != 0 {
                        state[slot] |= UNTAKEN | TAKEN;
                        self.take_failures.insert(format!(
                            "{label} may take `{}`'s object a second time",
                            self.paths[index]
                        ));
                        continue;
                    }
                    state[slot] = if maybe { state[slot] | TAKEN } else { TAKEN };
                    let path = &self.paths[index];
                    self.composed
                        .insert(format!("`{path}`'s object is released by {label}"));
                    for (inner, spelling) in &helper.states {
                        self.composed
                            .insert(format!("`{path}->{inner}` reads {spelling} ({label})"));
                    }
                    for member in &helper.invocations {
                        self.composed.insert(format!(
                            "`{path}->{member}` invokes nothing: its run set is closed and                              empty ({label})"
                        ));
                    }
                }
                Err(why) => {
                    let frame = self.stack.last().cloned().unwrap_or_default();
                    for index in self.carried(argument, bindings) {
                        state[self.paths.len() + index] |= UNTAKEN | TAKEN;
                        self.take_failures.insert(format!(
                            "{label} in `{frame}` receives the value of `{}` and is not composed:                              {why}",
                            self.paths[index]
                        ));
                    }
                }
            }
        }
        state
    }

    /// The call's helper declaration, when its callee may be composed at
    /// all: the declaration's own frame, a `static` function the unit
    /// defines, a helper declaration in force.
    fn admitted_helper(&self, name: Option<&str>) -> Result<Helper, String> {
        let name = name.ok_or("the callee is not named")?;
        if self.stack.len() != 1 {
            return Err("only a call in the declaration's own frame is composed".into());
        }
        let function = (self.callee)(name).ok_or("the unit defines no unique body for it")?;
        if function["storageClass"].as_str() != Some("static") {
            return Err("it is not a `static` function of the unit".into());
        }
        if self.stack.iter().any(|frame| frame == name) {
            return Err("it is being read".into());
        }
        let helper = self.helper.ok_or("no helper is composed")?;
        helper(name).ok_or_else(|| "it has no helper declaration in force in this unit".into())
    }

    /// Whether `helper`'s declaration takes the object at `position`.
    fn admits(&self, helper: &Helper, position: usize, argument: &Value) -> Result<(), String> {
        if position != helper.parameter {
            return Err("its declaration states nothing at that position".into());
        }
        if !helper.releases {
            return Err("its declaration does not release that formal".into());
        }
        helper.invokes_nothing.clone()?;
        let spelled = argument["type"]["desugaredQualType"]
            .as_str()
            .or_else(|| argument["type"]["qualType"].as_str())
            .unwrap_or("");
        let record = spelled
            .trim_end()
            .strip_suffix('*')
            .map(str::trim_end)
            .and_then(|pointee| pointee.strip_prefix("struct "))
            .map(str::trim);
        if record != Some(helper.tag.as_str()) {
            return Err(format!(
                "the argument's pointee `{spelled}` is not `struct {}`",
                helper.tag
            ));
        }
        Ok(())
    }

    /// The states on the true and false branch of `condition`.
    fn refine(
        &self,
        condition: &Value,
        state: &State,
        bindings: &Bindings,
    ) -> (Option<State>, Option<State>) {
        let mut node = strip(condition);
        let mut negated = false;
        loop {
            if kind(node) == "UnaryOperator" && node["opcode"].as_str() == Some("!") {
                match children(node).first() {
                    Some(inner) => {
                        node = strip(inner);
                        negated = !negated;
                    }
                    None => break,
                }
            } else if kind(node) == "CallExpr"
                && Self::direct_callee(node) == Some("__builtin_expect")
                && let Some(inner) = children(node).get(1)
            {
                node = strip(inner);
            } else {
                break;
            }
        }
        let mut tested = None;
        if let Some(path) = self.path_of(node, bindings) {
            tested = Some((path, true));
        } else if kind(node) == "BinaryOperator"
            && let Some(op @ ("==" | "!=")) = node["opcode"].as_str()
            && let [left, right] = children(node)
        {
            let path = if null_constant(right) {
                self.path_of(left, bindings)
            } else if null_constant(left) {
                self.path_of(right, bindings)
            } else {
                None
            };
            tested = path.map(|path| (path, op == "!="));
        }
        let unrefined = || (Some(state.clone()), Some(state.clone()));
        let Some((path, mut nonnull_when_true)) = tested else {
            return unrefined();
        };
        let Some(index) = self.paths.iter().position(|stated| *stated == path) else {
            return unrefined();
        };
        if negated {
            nonnull_when_true = !nonnull_when_true;
        }
        let value = state[index];
        let mut nonnull = state.clone();
        nonnull[index] = value & (ORIG | OTHER);
        let mut null = state.clone();
        null[index] = (if value & (ORIG | ORIG_NULL) == 0 {
            0
        } else {
            ORIG_NULL
        }) | (if value & (NULL | OTHER) == 0 { 0 } else { NULL });
        let nonnull = (nonnull[index] != 0).then_some(nonnull);
        let null = (null[index] != 0).then_some(null);
        if nonnull_when_true {
            (nonnull, null)
        } else {
            (null, nonnull)
        }
    }
}

/// Whether `value` (a stated path's mask at an exit) establishes `state`;
/// `stored` when the reading saw a null store into the path.
#[must_use]
pub fn establishes(state: &MemberPathState, value: u8, stored: bool) -> bool {
    match (state.state.fate, state.state.maybe) {
        (Fate::Released | Fate::Moved, false) => value & !(NULL | ORIG_NULL) == 0,
        (Fate::Released | Fate::Moved, true) => value & OTHER == 0 && stored,
        (Fate::Stale | Fate::Kept, _) => value & !(ORIG | ORIG_NULL) == 0,
    }
}

/// Whether the guard of `state` may hold at an exit returning `returned`:
/// an exit returning exactly the guard's enumerator is exempt from a `!=`
/// guard; every other exit is read.
#[must_use]
pub fn guard_may_hold(state: &MemberPathState, returned: &Returned) -> bool {
    match (&state.guard, returned) {
        (Some(guard), Returned::Enumerator(name)) if *name == guard.constant => {
            guard.op == GuardOp::Eq
        }
        _ => true,
    }
}

/// Every exit where a state's guard may hold establishes it: `Ok(())`, or
/// the first state and exit that does not.
///
/// # Errors
/// The unestablished state, naming its path and the exit's reading.
pub fn validated_exits(
    states: &[&MemberPathState],
    paths: &[String],
    outcomes: &Outcomes,
    stored: &BTreeSet<usize>,
) -> Result<(), String> {
    for state in states {
        let Some(index) = paths.iter().position(|path| *path == state.member.path) else {
            return Err(format!("`{}` is not read", state.member.path));
        };
        for (returned, exit) in outcomes {
            if guard_may_hold(state, returned)
                && !establishes(state, exit[index], stored.contains(&index))
            {
                return Err(format!(
                    "`{}` stated `{}` reads {} at an exit returning {}",
                    state.member.path,
                    state.state.spelling(),
                    spell(exit[index]),
                    match returned {
                        Returned::Enumerator(name) => name.as_str(),
                        Returned::Unknown => "a value the reading does not name",
                    }
                ));
            }
        }
    }
    Ok(())
}

/// What rule 50 establishes for a declaration's `#stale` states.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Taken {
    /// No non-set `#stale` state is stated.
    NotStated,
    /// Every exit where a `#stale` state's guard may hold took the path's
    /// object exactly once, through a composed helper.
    Established,
    /// A declared assumption, with why.
    Assumption(String),
}

/// Card SCHEDULER-DESTROY-MEMBER-STATES (rule 50): whether every exit where
/// a non-set `#stale` state's guard may hold reads its taken slot exactly
/// [`TAKEN`] (every path took the object, none took it twice or let its value
/// escape).
#[must_use]
pub fn taken_exits(states: &[&MemberPathState], paths: &[String], outcomes: &Outcomes) -> Taken {
    let mut stated = false;
    for state in states {
        if state.state.fate != Fate::Stale || state.state.maybe {
            continue;
        }
        stated = true;
        let Some(index) = paths.iter().position(|path| *path == state.member.path) else {
            return Taken::Assumption(format!("`{}` is not read", state.member.path));
        };
        if outcomes.is_empty() {
            return Taken::Assumption("the body has no exit".into());
        }
        for (returned, exit) in outcomes {
            if guard_may_hold(state, returned) && exit.get(paths.len() + index) != Some(&TAKEN) {
                return Taken::Assumption(format!(
                    "`{}`'s object is not taken exactly once by a composed helper on every path                      to an exit returning {}",
                    state.member.path,
                    match returned {
                        Returned::Enumerator(name) => name.as_str(),
                        Returned::Unknown => "a value the reading does not name",
                    }
                ));
            }
        }
    }
    if stated {
        Taken::Established
    } else {
        Taken::NotStated
    }
}

fn spell(value: u8) -> String {
    let words = [
        (ORIG, "untouched"),
        (ORIG_NULL, "untouched-null"),
        (NULL, "null"),
        (OTHER, "other"),
    ];
    let spelled = words
        .iter()
        .filter(|(bit, _)| value & bit != 0)
        .map(|(_, word)| *word)
        .collect::<Vec<_>>();
    format!("{{{}}}", spelled.join(", "))
}
