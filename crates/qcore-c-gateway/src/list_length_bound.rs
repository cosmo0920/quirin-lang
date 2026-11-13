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

//! Card **W45-FB-LIST-FINITENESS-PREMISE** (gateway rule 48): a list-length
//! bound fact for an intrusive-list traversal, under a **declared**
//! list-finiteness premise (contract specification C04.22, grammar G05l)
//! that is **checked where its frame is established**.
//!
//! Like rule 44's counted measure, this is a **fact, not a verdict**: it is
//! recorded beside the loop-measure row, reads no custody state, and no
//! verdict reads it.
//!
//! **The admitted shape.** A `for` loop of rule 5l's header family, read from
//! Clang's typed AST of the preprocessed text:
//! `for (C = (H)->D; C != (H); C = C->D)` (plain) or
//! `for (C = (H)->D, N = C->D; C != (H); C = N, N = C->D)` (safe), `D` one
//! of `next`/`prev` throughout, where
//!
//! * `C` (and `N`) are automatic, non-volatile locals whose address is taken
//!   nowhere in the function and which nothing in the loop's condition or
//!   body writes;
//! * `H` is `&R->M` in both places, `R` the same automatic, non-volatile
//!   local or formal of type `struct TAG *`, never address-taken and not
//!   written in the loop, and `M` a `struct mk_list` or `struct cfl_list`
//!   member: the head's address is fixed for the whole loop;
//! * `TAG.M` is declared by a list-finiteness premise;
//! * the function calls no returns-twice function and holds no inline
//!   assembly or block literal, and the loop holds no label and no `case` of
//!   a `switch` outside it;
//! * the loop's **frame** — its body and every function the body can call,
//!   transitively — is established in the unit: every callee is a function
//!   the unit defines (or a read-only C library function of a fixed table),
//!   no call is indirect, no body holds inline assembly, a block literal or
//!   an atomic expression, and no write in the frame may reach a list link
//!   (an assignment, compound assignment, `++` or `--` whose target is a
//!   record, a list pointer `struct mk_list *`/`struct cfl_list *` or a
//!   character object, other than a plain local or global variable of a
//!   non-record type).
//!
//! **The fact.** Under the premise, the list is a cycle of `n + 1` links
//! through its head, `n` its length at loop entry, and the frame writes no
//! link, so the cursor visits the `n` nodes in order and meets the head after
//! exactly `n` steps: the loop runs **at most `n = |H|` iterations** (fewer
//! when it leaves early), and `n - k` after `k` iterations is a measure that
//! is positive while the test holds and decreases by one per iteration
//! (`QCore.ListLengthBound`).
//!
//! Premises stated, not proved (trust boundary 28): the declared
//! list-finiteness premise itself; Clang's JSON AST is read faithfully; a
//! call to a function the unit defines runs that definition (no symbol
//! interposition); the program obeys C's effective-type rule (C11 6.5p7), so
//! a write through an lvalue that is neither a list pointer, a record nor a
//! character object does not modify a list link; and the table's library
//! functions write no object.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::Serialize;
use serde_json::Value;

use qcore_contract::list_finiteness::ListFinitenessPremises;

/// The fact model's version.
pub const MODEL_VERSION: &str = "qcore-list-length-bound/1";

/// Read-only C library functions a frame may call: each reads only its
/// arguments and writes no object (a stated premise on the C library).
pub const READ_ONLY_LIBRARY: &[&str] = &[
    "__builtin_expect",
    "memcmp",
    "strcasecmp",
    "strcmp",
    "strlen",
    "strncasecmp",
    "strncmp",
    "strnlen",
];

/// Why a loop carries no list-length bound fact. The codes are shared with
/// `QCore.ListLengthBound` (`ListRefusal.code`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub enum ListRefusal {
    /// Not a `for` loop of rule 5l's header family.
    NotAListTraversal,
    /// A cursor is written in the loop, address-taken or not an automatic,
    /// non-volatile local.
    CursorWritten,
    /// The head is not `&R->M` with `R` a local or formal and `M` a list
    /// member of `struct TAG`.
    HeadNotMember,
    /// The head's root `R` is written in the loop, address-taken or volatile.
    HeadRootNotStable,
    /// No list-finiteness premise declares `TAG.M` (or the premise file was
    /// rejected, which declares nothing).
    PremiseNotDeclared,
    /// A returns-twice call, inline assembly, a block literal or an atomic
    /// expression in the function or its frame.
    OpaqueControl,
    /// The loop holds a label or a `case` of a `switch` outside it.
    LabelInLoop,
    /// A write in the frame may reach a list link.
    FrameWritesLinks,
    /// An indirect call in the frame.
    FrameIndirectCall,
    /// A callee in the frame that the unit does not define (and that is not
    /// a read-only library function of the table).
    FrameCalleeUndefined,
}

impl ListRefusal {
    pub fn code(self) -> &'static str {
        match self {
            Self::NotAListTraversal => "not-a-list-traversal",
            Self::CursorWritten => "cursor-written",
            Self::HeadNotMember => "head-not-member",
            Self::HeadRootNotStable => "head-root-not-stable",
            Self::PremiseNotDeclared => "premise-not-declared",
            Self::OpaqueControl => "opaque-control",
            Self::LabelInLoop => "label-in-loop",
            Self::FrameWritesLinks => "frame-writes-links",
            Self::FrameIndirectCall => "frame-indirect-call",
            Self::FrameCalleeUndefined => "frame-callee-undefined",
        }
    }
}

/// The loop, abstracted to what the decision reads. Shared with
/// `QCore.ListLengthBound.ListShape`, field for field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct ListShape {
    pub list_traversal: bool,
    pub cursor_stable: bool,
    pub head_member: bool,
    pub head_root_stable: bool,
    pub premise_declared: bool,
    pub opaque_control: bool,
    pub label_in_loop: bool,
    pub frame_writes_links: bool,
    pub frame_indirect_call: bool,
    pub frame_callee_undefined: bool,
}

/// The decision (`QCore.ListLengthBound.decide`): `None` when admitted,
/// otherwise the first failing gate in this precedence.
pub fn decide(shape: &ListShape) -> Option<ListRefusal> {
    [
        (!shape.list_traversal, ListRefusal::NotAListTraversal),
        (!shape.cursor_stable, ListRefusal::CursorWritten),
        (!shape.head_member, ListRefusal::HeadNotMember),
        (!shape.head_root_stable, ListRefusal::HeadRootNotStable),
        (!shape.premise_declared, ListRefusal::PremiseNotDeclared),
        (shape.opaque_control, ListRefusal::OpaqueControl),
        (shape.label_in_loop, ListRefusal::LabelInLoop),
        (shape.frame_writes_links, ListRefusal::FrameWritesLinks),
        (shape.frame_indirect_call, ListRefusal::FrameIndirectCall),
        (
            shape.frame_callee_undefined,
            ListRefusal::FrameCalleeUndefined,
        ),
    ]
    .into_iter()
    .find_map(|(fails, refusal)| fails.then_some(refusal))
}

/// One shared vector: the shape's gates, in [`ListShape`]'s field order, and
/// the expected code.
pub type ListDecideVector = ([bool; 10], &'static str);

/// Identical to `QCore.ListLengthBound.decideVectors`.
pub const LIST_DECIDE_VECTORS: &[ListDecideVector] = &[
    // A read-only traversal of a declared head (`flb_output_get_instance`).
    (
        [
            true, true, true, true, true, false, false, false, false, false,
        ],
        "admitted",
    ),
    // `sched_timer_coro_get_id`'s `while` loop.
    (
        [
            false, false, false, false, false, false, false, false, false, false,
        ],
        "not-a-list-traversal",
    ),
    // The cursor reassigned in the body.
    (
        [
            true, false, true, true, true, false, false, false, false, false,
        ],
        "cursor-written",
    ),
    // A formal list pointer (`flb_engine_destroy_tasks`' `tasks`).
    (
        [
            true, true, false, false, false, false, false, false, false, false,
        ],
        "head-not-member",
    ),
    // The root rebound in the body.
    (
        [
            true, true, true, false, true, false, false, false, false, false,
        ],
        "head-root-not-stable",
    ),
    // An undeclared head.
    (
        [
            true, true, true, true, false, false, false, false, false, false,
        ],
        "premise-not-declared",
    ),
    (
        [
            true, true, true, true, true, true, false, false, false, false,
        ],
        "opaque-control",
    ),
    (
        [
            true, true, true, true, true, false, true, false, false, false,
        ],
        "label-in-loop",
    ),
    // `mk_list_add` in the frame, beside an undefined callee: the link write
    // is reported.
    (
        [
            true, true, true, true, true, false, false, true, false, true,
        ],
        "frame-writes-links",
    ),
    (
        [
            true, true, true, true, true, false, false, false, true, true,
        ],
        "frame-indirect-call",
    ),
    // `flb_log_print`, defined in another unit.
    (
        [
            true, true, true, true, true, false, false, false, false, true,
        ],
        "frame-callee-undefined",
    ),
];

/// Build the abstract shape of a vector row.
pub fn vector_shape(gates: [bool; 10]) -> ListShape {
    let [
        list_traversal,
        cursor_stable,
        head_member,
        head_root_stable,
        premise_declared,
        opaque_control,
        label_in_loop,
        frame_writes_links,
        frame_indirect_call,
        frame_callee_undefined,
    ] = gates;
    ListShape {
        list_traversal,
        cursor_stable,
        head_member,
        head_root_stable,
        premise_declared,
        opaque_control,
        label_in_loop,
        frame_writes_links,
        frame_indirect_call,
        frame_callee_undefined,
    }
}

/// A list-length bound for one loop.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ListBound {
    /// The declared head, `TAG.MEMBER`.
    pub list: String,
    /// The head as the loop spells it, `&R->M`.
    pub head: String,
    pub cursor: String,
    /// `plain` or `safe`.
    pub traversal: &'static str,
    /// `next` or `prev`.
    pub direction: &'static str,
    /// The bound: the list's length at loop entry.
    pub bound: String,
    /// The measure after `k` iterations.
    pub measure: String,
    /// The premise the fact rests on, as declared.
    pub premise: String,
    /// The functions whose bodies establish the frame, sorted (the loop's own
    /// body is not listed).
    pub frame_functions: Vec<String>,
    /// The read-only library functions the frame calls, sorted.
    pub frame_library: Vec<String>,
}

/// A loop's list-length outcome: a bound, or a named refusal.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ListBoundFact {
    pub model_version: &'static str,
    pub bound: Option<ListBound>,
    pub refusal: Option<&'static str>,
    pub detail: Option<String>,
}

impl ListBoundFact {
    fn refused(refusal: ListRefusal, detail: String) -> Self {
        Self {
            model_version: MODEL_VERSION,
            bound: None,
            refusal: Some(refusal.code()),
            detail: Some(detail),
        }
    }
}

/// What a body (or a loop's body) may do to list links: the summary the
/// frame check composes. Built for every function the unit defines.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FrameSummary {
    /// The first write that may reach a list link, described.
    pub link_write: Option<String>,
    /// The first indirect call, described.
    pub indirect_call: Option<String>,
    /// Inline assembly, a block literal or an atomic expression.
    pub opaque: Option<String>,
    /// Direct callees, by name.
    pub callees: BTreeSet<String>,
}

fn kind(node: &Value) -> &str {
    node["kind"].as_str().unwrap_or("")
}

fn children(node: &Value) -> &[Value] {
    node["inner"].as_array().map_or(&[], Vec::as_slice)
}

fn spelling(node: &Value) -> &str {
    node["type"]["desugaredQualType"]
        .as_str()
        .or_else(|| node["type"]["qualType"].as_str())
        .unwrap_or_default()
}

fn bare(node: &Value) -> &Value {
    let mut node = node;
    while kind(node) == "ParenExpr" {
        match children(node).first() {
            Some(inner) => node = inner,
            None => break,
        }
    }
    node
}

/// Strip parentheses and `LValueToRValue`/`NoOp` conversions.
fn value_of(node: &Value) -> &Value {
    let mut node = bare(node);
    while kind(node) == "ImplicitCastExpr"
        && matches!(node["castKind"].as_str(), Some("LValueToRValue" | "NoOp"))
    {
        match children(node).first() {
            Some(inner) => node = bare(inner),
            None => break,
        }
    }
    node
}

/// The declaration a bare `DeclRefExpr` names.
fn named(node: &Value) -> Option<&str> {
    let node = bare(node);
    (kind(node) == "DeclRefExpr")
        .then(|| node["referencedDecl"]["id"].as_str())
        .flatten()
}

/// A type spelling without its qualifiers.
fn unqualified(spelling: &str) -> String {
    spelling
        .replace('*', " * ")
        .split_whitespace()
        .filter(|word| !matches!(*word, "const" | "volatile" | "restrict" | "_Atomic"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// A list node record, `struct mk_list` or `struct cfl_list`.
pub fn list_node(spelling: &str) -> bool {
    matches!(
        unqualified(spelling).as_str(),
        "struct mk_list" | "struct cfl_list"
    )
}

/// A pointer to a list node.
fn list_pointer(spelling: &str) -> bool {
    let text = unqualified(spelling);
    text.strip_suffix('*')
        .map(str::trim_end)
        .is_some_and(list_node)
}

/// A record or union object (not a pointer, array or function).
fn record(spelling: &str) -> bool {
    let text = unqualified(spelling);
    (text.starts_with("struct ") || text.starts_with("union ")) && !text.contains(['*', '[', '('])
}

fn character(spelling: &str) -> bool {
    matches!(
        unqualified(spelling).as_str(),
        "char" | "signed char" | "unsigned char"
    )
}

/// The `struct TAG` a `struct TAG *` spelling points to.
fn pointee_tag(spelling: &str) -> Option<String> {
    let text = unqualified(spelling);
    let pointee = text.strip_suffix('*')?.trim_end();
    let tag = pointee.strip_prefix("struct ")?;
    (!tag.is_empty() && tag.bytes().all(|b| b == b'_' || b.is_ascii_alphanumeric()))
        .then(|| tag.to_owned())
}

/// Whether a write to `target` may reach a list link, and why.
fn link_write(target: &Value) -> Option<String> {
    let target = bare(target);
    let ty = spelling(target);
    if kind(target) == "DeclRefExpr" {
        // A whole variable: its own storage, a list link only when it is (or
        // holds) a record.
        return record(ty).then(|| {
            format!(
                "a write to the record variable `{}` (`{ty}`)",
                target["referencedDecl"]["name"].as_str().unwrap_or("?")
            )
        });
    }
    let what = if record(ty) {
        "a record"
    } else if list_pointer(ty) {
        "a list pointer"
    } else if character(ty) {
        "a character object"
    } else {
        return None;
    };
    let field = (kind(target) == "MemberExpr")
        .then(|| target["name"].as_str())
        .flatten()
        .map(|name| format!(" `{name}`"))
        .unwrap_or_default();
    Some(format!("a write to {what}{field} (`{ty}`)"))
}

/// The name of a direct callee (through its decay cast), when the callee is
/// a function declaration.
fn direct_callee(call: &Value) -> Option<&str> {
    let mut node = bare(children(call).first()?);
    while kind(node) == "ImplicitCastExpr" {
        node = bare(children(node).first()?);
    }
    (kind(node) == "DeclRefExpr" && node["referencedDecl"]["kind"] == "FunctionDecl")
        .then(|| node["referencedDecl"]["name"].as_str())
        .flatten()
}

impl FrameSummary {
    /// Summarise a body (a function's, or a loop's).
    pub fn of(node: &Value) -> Self {
        let mut summary = Self::default();
        summary.scan(node);
        summary
    }

    fn scan(&mut self, node: &Value) {
        match kind(node) {
            "CallExpr" => match direct_callee(node) {
                Some(name) => {
                    self.callees.insert(name.to_owned());
                }
                None => {
                    self.indirect_call
                        .get_or_insert_with(|| format!("an indirect call (`{}`)", spelling(node)));
                }
            },
            "BinaryOperator" if node["opcode"] == "=" => self.write(children(node).first()),
            "CompoundAssignOperator" => self.write(children(node).first()),
            "UnaryOperator" if matches!(node["opcode"].as_str(), Some("++" | "--")) => {
                self.write(children(node).first());
            }
            "GCCAsmStmt" | "MSAsmStmt" => {
                self.opaque.get_or_insert_with(|| "inline assembly".into());
            }
            "BlockExpr" => {
                self.opaque.get_or_insert_with(|| "a block literal".into());
            }
            "AtomicExpr" => {
                self.opaque
                    .get_or_insert_with(|| "an atomic expression".into());
            }
            _ => {}
        }
        for child in children(node) {
            self.scan(child);
        }
    }

    fn write(&mut self, target: Option<&Value>) {
        if self.link_write.is_some() {
            return;
        }
        if let Some(why) = target.and_then(link_write) {
            self.link_write = Some(why);
        }
    }
}

/// The recognised header of a list traversal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Header {
    pub cursor: String,
    pub cursor_ids: Vec<String>,
    pub safe: bool,
    pub direction: &'static str,
    /// `(root id, root name, root type, member, member type)`, or `None` when
    /// the head is not `&R->M`.
    pub head: Option<Head>,
    /// The head as spelled, for a refusal's detail.
    pub head_text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Head {
    pub root_id: String,
    pub root: String,
    pub tag: String,
    pub member: String,
}

/// `&R->M` with `R` a declaration reference of type `struct TAG *` and `M`
/// a list node member. A head key compares the init's and the condition's
/// heads.
fn head_of(node: &Value) -> Result<Head, String> {
    let node = bare(node);
    let member = (kind(node) == "UnaryOperator" && node["opcode"] == "&")
        .then(|| children(node).first().map(bare))
        .flatten()
        .filter(|member| kind(member) == "MemberExpr" && member["isArrow"] == true);
    let Some(member) = member else {
        return Err(head_text(node));
    };
    let base = children(member).first().map(value_of);
    let (Some(base), Some(name)) = (base, member["name"].as_str()) else {
        return Err(head_text(node));
    };
    let root_id = named(base);
    let tag = pointee_tag(spelling(base));
    match (root_id, tag) {
        (Some(root_id), Some(tag)) if list_node(spelling(member)) => Ok(Head {
            root_id: root_id.to_owned(),
            root: base["referencedDecl"]["name"]
                .as_str()
                .unwrap_or("?")
                .to_owned(),
            tag,
            member: name.to_owned(),
        }),
        _ => Err(head_text(node)),
    }
}

/// A short spelling of a head expression, for details.
fn head_text(node: &Value) -> String {
    let node = value_of(node);
    match kind(node) {
        "DeclRefExpr" => node["referencedDecl"]["name"]
            .as_str()
            .unwrap_or("?")
            .to_owned(),
        "UnaryOperator" if node["opcode"] == "&" => format!(
            "&{}",
            children(node).first().map(head_text).unwrap_or_default()
        ),
        "MemberExpr" => format!(
            "{}{}{}",
            children(node).first().map(head_text).unwrap_or_default(),
            if node["isArrow"] == true { "->" } else { "." },
            node["name"].as_str().unwrap_or("?")
        ),
        other => format!("<{other}>"),
    }
}

/// `C = (H)->D`: the cursor's id, the head node and the direction.
fn first_step(node: &Value) -> Option<(&str, &Value, &'static str)> {
    let node = bare(node);
    if kind(node) != "BinaryOperator" || node["opcode"] != "=" {
        return None;
    }
    let [target, value] = children(node) else {
        return None;
    };
    let cursor = named(target)?;
    let read = value_of(value);
    let direction = direction(read)?;
    let head = children(read).first()?;
    Some((cursor, head, direction))
}

/// `X->next` or `X->prev` over a list node pointer.
fn direction(read: &Value) -> Option<&'static str> {
    if kind(read) != "MemberExpr" || read["isArrow"] != true || !list_pointer(spelling(read)) {
        return None;
    }
    match read["name"].as_str()? {
        "next" => Some("next"),
        "prev" => Some("prev"),
        _ => None,
    }
}

/// `A = B->D` with `A`, `B` named: `(A, B, D)`.
fn successor(node: &Value) -> Option<(&str, &str, &'static str)> {
    let node = bare(node);
    if kind(node) != "BinaryOperator" || node["opcode"] != "=" {
        return None;
    }
    let [target, value] = children(node) else {
        return None;
    };
    let read = value_of(value);
    let direction = direction(read)?;
    Some((
        named(target)?,
        named(value_of(children(read).first()?))?,
        direction,
    ))
}

/// `A = B` with both named.
fn copy(node: &Value) -> Option<(&str, &str)> {
    let node = bare(node);
    if kind(node) != "BinaryOperator" || node["opcode"] != "=" {
        return None;
    }
    let [target, value] = children(node) else {
        return None;
    };
    Some((named(target)?, named(value_of(value))?))
}

fn comma(node: &Value) -> Option<(&Value, &Value)> {
    let node = bare(node);
    if kind(node) != "BinaryOperator" || node["opcode"] != "," {
        return None;
    }
    let [left, right] = children(node) else {
        return None;
    };
    Some((left, right))
}

/// Recognise rule 5l's header family on a `ForStmt`'s init, condition and
/// step. `None` when the loop is not one.
pub fn header(init: &Value, condition: &Value, step: &Value) -> Option<Header> {
    // The init: `C = (H)->D` or `C = (H)->D, N = C->D`.
    let (cursor, head_node, dir, saved) = match comma(init) {
        Some((first, second)) => {
            let (cursor, head, dir) = first_step(first)?;
            let (saved, from, dir2) = successor(second)?;
            (from == cursor && dir2 == dir && saved != cursor).then_some(())?;
            (cursor, head, dir, Some(saved))
        }
        None => {
            let (cursor, head, dir) = first_step(init)?;
            (cursor, head, dir, None)
        }
    };
    // The condition: `C != (H)`, the same head.
    let condition = bare(condition);
    if kind(condition) != "BinaryOperator" || condition["opcode"] != "!=" {
        return None;
    }
    let [left, right] = children(condition) else {
        return None;
    };
    if named(value_of(left)) != Some(cursor) {
        return None;
    }
    let head = head_of(head_node);
    let other = head_of(right);
    match (&head, &other) {
        (Ok(a), Ok(b)) if a == b => {}
        (Err(a), Err(b)) if a == b => {}
        _ => return None,
    }
    // The step: `C = C->D`, or `C = N, N = C->D`.
    match saved {
        None => {
            let (target, from, dir2) = successor(step)?;
            (target == cursor && from == cursor && dir2 == dir).then_some(())?;
        }
        Some(saved) => {
            let (first, second) = comma(step)?;
            let (target, from) = copy(first)?;
            let (target2, from2, dir2) = successor(second)?;
            (target == cursor
                && from == saved
                && target2 == saved
                && from2 == cursor
                && dir2 == dir)
                .then_some(())?;
        }
    }
    let cursor_name = bare(init);
    let cursor_name = match comma(cursor_name) {
        Some((first, _)) => first,
        None => cursor_name,
    };
    let cursor_name = children(bare(cursor_name))
        .first()
        .map(|target| {
            bare(target)["referencedDecl"]["name"]
                .as_str()
                .unwrap_or("?")
        })
        .unwrap_or("?")
        .to_owned();
    let mut cursor_ids = vec![cursor.to_owned()];
    cursor_ids.extend(saved.map(str::to_owned));
    Some(Header {
        cursor: cursor_name,
        cursor_ids,
        safe: saved.is_some(),
        direction: dir,
        head_text: match &head {
            Ok(head) => format!("&{}->{}", head.root, head.member),
            Err(text) => text.clone(),
        },
        head: head.ok(),
    })
}

/// A loop waiting for the unit's frame summaries: everything decided locally,
/// and the loop body's own frame.
#[derive(Clone, Debug)]
pub struct PendingList {
    pub shape: ListShape,
    pub header: Option<Header>,
    pub local: FrameSummary,
    /// The detail of a locally decided refusal.
    pub detail: Option<String>,
}

/// The outcome of composing a frame from the unit's summaries.
struct Frame {
    functions: Vec<String>,
    library: Vec<String>,
    link_write: Option<String>,
    indirect_call: Option<String>,
    opaque: Option<String>,
    undefined: Option<String>,
}

fn compose(local: &FrameSummary, summaries: &BTreeMap<String, FrameSummary>) -> Frame {
    let mut frame = Frame {
        functions: Vec::new(),
        library: Vec::new(),
        link_write: local.link_write.clone().map(|w| format!("{w} in the loop")),
        indirect_call: local
            .indirect_call
            .clone()
            .map(|w| format!("{w} in the loop")),
        opaque: local.opaque.clone().map(|w| format!("{w} in the loop")),
        undefined: None,
    };
    let mut seen = BTreeSet::new();
    let mut queue = local.callees.iter().cloned().collect::<VecDeque<_>>();
    while let Some(callee) = queue.pop_front() {
        if !seen.insert(callee.clone()) {
            continue;
        }
        if let Some(summary) = summaries.get(&callee) {
            frame.functions.push(callee.clone());
            if frame.link_write.is_none() {
                frame.link_write = summary
                    .link_write
                    .as_ref()
                    .map(|w| format!("{w} in `{callee}`"));
            }
            if frame.indirect_call.is_none() {
                frame.indirect_call = summary
                    .indirect_call
                    .as_ref()
                    .map(|w| format!("{w} in `{callee}`"));
            }
            if frame.opaque.is_none() {
                frame.opaque = summary
                    .opaque
                    .as_ref()
                    .map(|w| format!("{w} in `{callee}`"));
            }
            queue.extend(summary.callees.iter().cloned());
        } else if READ_ONLY_LIBRARY.contains(&callee.as_str()) {
            frame.library.push(callee);
        } else if frame.undefined.is_none() {
            frame.undefined = Some(format!(
                "`{callee}` is called in the frame and the unit does not define it"
            ));
        }
    }
    frame.functions.sort();
    frame.library.sort();
    frame
}

/// Decide a pending loop once the unit's summaries are complete.
pub fn resolve(
    pending: PendingList,
    summaries: &BTreeMap<String, FrameSummary>,
    premises: &ListFinitenessPremises,
    rejected: Option<&str>,
) -> ListBoundFact {
    let PendingList {
        mut shape,
        header,
        local,
        detail,
    } = pending;
    let Some(header) = header else {
        return ListBoundFact::refused(
            ListRefusal::NotAListTraversal,
            detail.unwrap_or_else(|| "not a list traversal of rule 5l's header family".into()),
        );
    };
    if let Some(refusal) = decide(&shape)
        && refusal < ListRefusal::PremiseNotDeclared
    {
        return ListBoundFact::refused(refusal, detail.unwrap_or_default());
    }
    let head = header
        .head
        .clone()
        .expect("a head-member shape carries its head");
    shape.premise_declared = rejected.is_none() && premises.declares(&head.tag, &head.member);
    let frame = compose(&local, summaries);
    shape.opaque_control |= frame.opaque.is_some();
    shape.frame_writes_links = frame.link_write.is_some();
    shape.frame_indirect_call = frame.indirect_call.is_some();
    shape.frame_callee_undefined = frame.undefined.is_some();
    let Some(refusal) = decide(&shape) else {
        let length = format!("|{}|", header.head_text.trim_start_matches('&'));
        return ListBoundFact {
            model_version: MODEL_VERSION,
            bound: Some(ListBound {
                list: format!("{}.{}", head.tag, head.member),
                head: header.head_text.clone(),
                cursor: header.cursor.clone(),
                traversal: if header.safe { "safe" } else { "plain" },
                direction: header.direction,
                bound: format!("{length} at loop entry"),
                measure: format!("{length} - k after k iterations"),
                premise: ListFinitenessPremises::spelling(&head.tag, &head.member),
                frame_functions: frame.functions,
                frame_library: frame.library,
            }),
            refusal: None,
            detail: None,
        };
    };
    let detail = match refusal {
        ListRefusal::PremiseNotDeclared => match rejected {
            Some(error) => format!("the premise file was rejected, so it declares nothing: {error}"),
            None => format!(
                "no premise declares `{}.{}`",
                head.tag, head.member
            ),
        },
        ListRefusal::OpaqueControl => frame.opaque.unwrap_or_else(|| {
            "the function calls a returns-twice function or holds inline assembly or a block literal"
                .into()
        }),
        ListRefusal::LabelInLoop => {
            "the loop holds a label or a `case` of a `switch` outside it".into()
        }
        ListRefusal::FrameWritesLinks => frame.link_write.unwrap_or_default(),
        ListRefusal::FrameIndirectCall => frame.indirect_call.unwrap_or_default(),
        ListRefusal::FrameCalleeUndefined => frame.undefined.unwrap_or_default(),
        _ => detail.unwrap_or_default(),
    };
    ListBoundFact::refused(refusal, detail)
}

#[cfg(test)]
#[path = "../tests/unit/list_length_bound.rs"]
mod tests;
