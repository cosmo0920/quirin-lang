//! Card **CG02-ELABORATION-CORE** (gateway rule 10s): the common expression
//! elaborator of the Clang gap analysis (§7), over Clang's typed AST.
//!
//! [`elaborate`] reads one full expression (or a one-declarator declaration)
//! and yields:
//!
//! * the **result**, a value or a place ([`Operand`]); a value produced by an
//!   increment keeps its old/new origin apart ([`ValueSource`]);
//! * an **evaluation graph** ([`EvalGraph`]): the events in one permitted
//!   serialisation, the explicit sequenced-before edges, and the explicitly
//!   unsequenced operand groups C leaves unordered;
//! * the **events** themselves ([`Event`]: read, write, call, branch, merge,
//!   lifetime start/end);
//! * the **obligations and assumptions** a route must discharge or state.
//!
//! # The order model (gap entry E12, its order part)
//!
//! Sequenced-before is explicit. The operands of an arithmetic operator, of an
//! assignment (its place and its value) and of a call are *unsequenced*; the
//! side effect of an assignment or increment is sequenced after its operands'
//! value computations only; a comma and a call's sequence point order every
//! event of what precedes them. [`EvalGraph::check`] then refuses the graph
//! unless **every pair of events the closure leaves unordered commutes**:
//! two reads, or a read/write or write/write pair on provably disjoint places
//! ([`Place::disjoint`]). A same-place write/write or write/read pair is C
//! undefined behaviour and is refused, never claimed safe; a call unordered with
//! any other event is refused (card CG05 owns calls in unordered groups). No
//! left-to-right order is ever chosen: a route may lower the events in
//! [`EvalGraph::events`]' order only because an admitted graph reaches the same
//! state in every permitted order (`QCore.ElaborationCore.interleave_run`).
//!
//! # Places
//!
//! A named object is read here from its `DeclRefExpr`. Every other place (a
//! member, a subscript, a dereference) is asked of the route's
//! [`Context::place`], the narrow interface card CG10's common typed place is
//! meant to implement. A place's own facts (the route's dereference
//! obligations) travel with its events in [`Elaboration::facts`].
//!
//! # Refusals
//!
//! Every form outside [`FORM_TABLE`]'s admitted rows is refused **by name**,
//! with the card that owns it ([`Refusal`]): value `&&`/`||`/`?:` (CG06),
//! nested and unordered calls (CG05), calls whose actuals carry effects
//! (CG03), pointer-valued stores (CG11), pointer arithmetic (CG16), casts and
//! type queries (CG08), `_Generic` and statement expressions (CG07), aggregate
//! values (CG14), volatile and atomic places (CG15/CG19), indirect callees
//! (CG22). The Lean model is `QCore.ElaborationCore`, bound to [`FORM_TABLE`]
//! and [`commutes`] by the shared-vector test in `tests/unit/elaboration.rs`.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::typed_c_ast::{children, kind};

/// An event's index in [`EvalGraph::events`].
pub(crate) type EventId = usize;

/// A storage location an event touches.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Place {
    /// A named object, by its Clang declaration id.
    Object { decl: String, name: String },
    /// A member, subscript or dereference the route's place reader admitted.
    /// `root` is the declaration id of the object the path starts from;
    /// `indirect` says the path leaves that object through a pointer, so its
    /// storage is not part of `root`.
    Path {
        spelling: String,
        root: String,
        indirect: bool,
    },
}

impl Place {
    /// Whether two places provably share no storage. Distinct named objects
    /// never overlap; a path inside an object overlaps only that object; a
    /// path through a pointer can reach only an object whose address is taken;
    /// two paths are never proved disjoint here (CG10/CG13 own footprints).
    pub(crate) fn disjoint(&self, other: &Place, unaliased: &dyn Fn(&str) -> bool) -> bool {
        match (self, other) {
            (Place::Object { decl: a, .. }, Place::Object { decl: b, .. }) => a != b,
            (Place::Object { decl, .. }, Place::Path { root, indirect, .. })
            | (Place::Path { root, indirect, .. }, Place::Object { decl, .. }) => {
                if *indirect {
                    unaliased(decl)
                } else {
                    root != decl
                }
            }
            (Place::Path { .. }, Place::Path { .. }) => false,
        }
    }

    pub(crate) fn spelling(&self) -> &str {
        match self {
            Place::Object { name, .. } => name,
            Place::Path { spelling, .. } => spelling,
        }
    }
}

/// One effect or custody event of an evaluation.
#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(
    dead_code,
    reason = "Branch, Merge and the lifetime events are the vocabulary cards CG06, CG07 and CG14 emit"
)]
pub(crate) enum Event {
    /// A value computation that reads a place.
    Read(Place),
    /// A side effect that stores into a place; `pointer` is the stored value's
    /// class (a pointer-valued store is card CG11's).
    Write { place: Place, pointer: bool },
    /// A direct call of a named function; its contract is applied by the
    /// route's existing contract application (card CG03 owns the single
    /// application point).
    Call { callee: String },
    /// A control split and join (value `&&`, `||`, `?:`: card CG06). Not
    /// emitted by this card's forms, which refuse those operators by name.
    Branch,
    Merge,
    /// An object's lifetime (compound literals, statement expressions: cards
    /// CG07/CG14). Not emitted by this card's forms.
    LifetimeStart(Place),
    LifetimeEnd(Place),
}

/// Whether two unordered events commute (`QCore.ElaborationCore.commutes`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Commutation {
    Commutes,
    /// A same-place write/write or write/read pair: C undefined behaviour.
    Conflict(String),
    /// A call unordered with another event (card CG05).
    Call(String),
    /// An event kind this card never leaves unordered.
    Unlisted,
}

/// The commutation decision on one unordered pair.
pub(crate) fn commutes(a: &Event, b: &Event, unaliased: &dyn Fn(&str) -> bool) -> Commutation {
    use Event::*;
    match (a, b) {
        (Read(_), Read(_)) => Commutation::Commutes,
        (Read(p) | Write { place: p, .. }, Read(q) | Write { place: q, .. }) => {
            if p.disjoint(q, unaliased) {
                Commutation::Commutes
            } else {
                Commutation::Conflict(p.spelling().to_owned())
            }
        }
        (Call { callee }, _) | (_, Call { callee }) => Commutation::Call(callee.clone()),
        _ => Commutation::Unlisted,
    }
}

/// The evaluation graph of one full expression.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct EvalGraph {
    /// Every event, in one permitted serialisation (every edge goes forward).
    pub(crate) events: Vec<Event>,
    /// `(a, b)`: `a` is sequenced before `b`.
    pub(crate) before: Vec<(EventId, EventId)>,
    /// Each group lists operands (event sets) C leaves mutually unsequenced.
    pub(crate) unsequenced: Vec<Vec<Vec<EventId>>>,
}

impl EvalGraph {
    fn push(&mut self, event: Event) -> EventId {
        self.events.push(event);
        self.events.len() - 1
    }

    fn order(&mut self, from: &[EventId], to: EventId) {
        for &event in from {
            self.before.push((event, to));
        }
    }

    /// Whether `a` is sequenced before `b` (transitively).
    pub(crate) fn sequenced(&self, a: EventId, b: EventId) -> bool {
        let mut reach = vec![false; self.events.len()];
        let mut work = vec![a];
        while let Some(at) = work.pop() {
            for &(from, to) in &self.before {
                if from == at && !reach[to] {
                    reach[to] = true;
                    work.push(to);
                }
            }
        }
        reach.get(b).copied().unwrap_or(false)
    }

    /// Refuse the graph unless every pair the closure leaves unordered
    /// commutes. A pair is checked once, in event order.
    pub(crate) fn check(&self, unaliased: &dyn Fn(&str) -> bool) -> Result<(), Refusal> {
        debug_assert!(self.before.iter().all(|(a, b)| a < b));
        for a in 0..self.events.len() {
            for b in a + 1..self.events.len() {
                if self.sequenced(a, b) {
                    continue;
                }
                match commutes(&self.events[a], &self.events[b], unaliased) {
                    Commutation::Commutes => {}
                    Commutation::Conflict(place) => {
                        return Err(Refusal::UnsequencedConflict(place));
                    }
                    Commutation::Call(callee) => return Err(Refusal::UnorderedCall(callee)),
                    Commutation::Unlisted => return Err(Refusal::Unlisted("unordered event")),
                }
            }
        }
        Ok(())
    }
}

/// Where a produced value comes from. An increment's value is the place's
/// old or new value, never both.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ValueSource {
    /// A value computed from reads and literals.
    Computed,
    /// `P++` / `P--`: the value the place held before the update.
    Old(Place),
    /// `++P` / `--P`, `P = E`, `P op= E`: the value the place holds after it.
    New(Place),
    /// A call's result.
    Call(String),
}

/// The result of an elaborated expression.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Operand {
    Value { pointer: bool, source: ValueSource },
    Void,
}

/// What a route must discharge before an admission stands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Obligation {
    /// The route's dereference obligation of each path access (in G: the
    /// base checks of `IndirectScalarAccess`).
    Access(Place),
    /// The call's contract, applied once by the route (card CG03).
    Contract(String),
}

/// What an admission rests on without proving it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Assumption {
    /// Arithmetic results are custody-neutral values; no numeric result, and
    /// no absence of a numeric fault, is claimed (card CG20).
    NoNumericClaim,
}

/// One elaborated full expression.
#[derive(Clone, Debug)]
pub(crate) struct Elaboration<F> {
    pub(crate) result: Operand,
    pub(crate) graph: EvalGraph,
    /// The place reader's facts of every path event.
    pub(crate) facts: BTreeMap<EventId, F>,
    /// Every event of each call's designator and actuals, by call event.
    pub(crate) actuals: BTreeMap<EventId, Vec<EventId>>,
    /// Route G discharges each `Access` through the instruction it lowers
    /// the event to and each `Contract` through the existing classifier of the
    /// piece; the list is the interface CG03's single application point reads.
    #[allow(dead_code, reason = "read by the tests; CG03 reads it")]
    pub(crate) obligations: Vec<Obligation>,
    #[allow(dead_code, reason = "read by the tests; CG20 reads it")]
    pub(crate) assumptions: Vec<Assumption>,
}

/// A named refusal, with the card that owns the form.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Refusal {
    /// A form [`FORM_TABLE`] refuses, with the card that owns it.
    Form(&'static str, &'static str),
    /// A call unordered with another event (card CG05).
    UnorderedCall(String),
    /// A call whose actuals carry side effects, outside a sequenced piece
    /// (card CG03 owns applying its contract).
    EffectfulActual(String),
    /// A same-place write/write or write/read pair left unordered.
    UnsequencedConflict(String),
    /// A cast kind outside the scalar set (card CG08).
    Cast(String),
    /// The route's place reader refused the place (card CG10).
    Place(String),
    Declaration(&'static str),
    Unlisted(&'static str),
    UnlistedKind(String),
}

/// The refusal of a form [`FORM_TABLE`] lists as refused.
pub(crate) fn refused(form: &'static str) -> Refusal {
    match disposition(form) {
        Some(Disposition::Refused(card)) => Refusal::Form(form, card),
        _ => Refusal::UnlistedKind(form.to_owned()),
    }
}

impl Refusal {
    /// The refusal's text: the form, then the owning card.
    pub(crate) fn reason(&self) -> String {
        match self {
            Refusal::Form(form, card) => format!("{form} ({card})"),
            Refusal::UnorderedCall(callee) => {
                format!("call `{callee}` unsequenced with another event (CG05)")
            }
            Refusal::EffectfulActual(callee) => {
                format!("call `{callee}` whose actuals carry effects (CG03)")
            }
            Refusal::UnsequencedConflict(place) => format!(
                "unsequenced write/write or write/read on `{place}` (C undefined behaviour)"
            ),
            Refusal::Cast(cast) => format!("cast `{cast}` (CG08)"),
            Refusal::Place(reason) => format!("place: {reason} (CG10)"),
            Refusal::Declaration(reason) => format!("declaration: {reason}"),
            Refusal::Unlisted(form) => format!("unlisted form: {form}"),
            Refusal::UnlistedKind(kind) => format!("unlisted form `{kind}`"),
        }
    }
}

/// How the elaborator treats one form (`QCore.ElaborationCore.formTable`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Disposition {
    /// Elaborated into events.
    Admitted,
    /// Elaborated only where every event of its actuals precedes it and it is
    /// unordered with no other event.
    Sequenced,
    /// Refused by name; the owning card.
    Refused(&'static str),
}

/// Every form the elaborator names, with its disposition. A form not listed is
/// refused as unlisted.
pub(crate) const FORM_TABLE: [(&str, Disposition); 28] = [
    ("literal", Disposition::Admitted),
    ("object read", Disposition::Admitted),
    ("path read", Disposition::Admitted),
    ("scalar unary", Disposition::Admitted),
    ("scalar binary", Disposition::Admitted),
    ("scalar cast", Disposition::Admitted),
    ("assignment value", Disposition::Admitted),
    ("compound assignment value", Disposition::Admitted),
    ("pre increment", Disposition::Admitted),
    ("post increment", Disposition::Admitted),
    ("pre decrement", Disposition::Admitted),
    ("post decrement", Disposition::Admitted),
    ("comma", Disposition::Admitted),
    ("declaration", Disposition::Admitted),
    ("direct call", Disposition::Sequenced),
    ("value &&", Disposition::Refused("CG06")),
    ("value ||", Disposition::Refused("CG06")),
    ("conditional", Disposition::Refused("CG06")),
    ("nested call", Disposition::Refused("CG05")),
    ("indirect call", Disposition::Refused("CG22")),
    ("pointer store", Disposition::Refused("CG11")),
    ("pointer arithmetic", Disposition::Refused("CG16")),
    ("type query operand", Disposition::Refused("CG08")),
    ("generic selection", Disposition::Refused("CG07")),
    ("statement expression", Disposition::Refused("CG07")),
    ("compound literal", Disposition::Refused("CG14")),
    ("volatile access", Disposition::Refused("CG15")),
    ("atomic access", Disposition::Refused("CG19")),
];

/// The disposition of a form, `None` for an unlisted one.
pub(crate) fn disposition(form: &str) -> Option<Disposition> {
    FORM_TABLE
        .iter()
        .find(|(name, _)| *name == form)
        .map(|(_, disposition)| *disposition)
}

/// The value form a node is, when it is one of the four this card adds; route
/// T names them in its refusal (see `typed_source.rs`).
pub(crate) fn value_form(node: &Value) -> Option<&'static str> {
    match (kind(node), node["opcode"].as_str()) {
        ("BinaryOperator", Some("=")) => Some("assignment value"),
        ("BinaryOperator", Some(",")) => Some("comma"),
        ("CompoundAssignOperator", _) => Some("compound assignment value"),
        ("UnaryOperator", Some("++")) if node["isPostfix"].as_bool() == Some(true) => {
            Some("post increment")
        }
        ("UnaryOperator", Some("++")) => Some("pre increment"),
        ("UnaryOperator", Some("--")) if node["isPostfix"].as_bool() == Some(true) => {
            Some("post decrement")
        }
        ("UnaryOperator", Some("--")) => Some("pre decrement"),
        _ => None,
    }
}

/// Route T's refusal of a value form: no consumer that imports through T
/// decides one (card CG01's census of every `SourceExpression` reader), so T
/// reads none and names the form.
pub(crate) fn t_refusal(form: &str) -> String {
    format!("typed-source value form `{form}` is decided by no typed-source consumer (CG02)")
}

/// What the elaborator asks of the route.
pub(crate) trait Context {
    /// The route's facts of one admitted path place.
    type Facts: Clone + std::fmt::Debug;
    /// Whether a node's type is a scalar the route reads (not a pointer).
    fn scalar(&self, node: &Value) -> bool;
    /// Whether a node's type is an object pointer the route reads.
    fn pointer(&self, node: &Value) -> bool;
    /// The route's facts of one member, subscript or dereference lvalue, or a
    /// refusal. The elaborator computes the place itself ([`path_place`]) and
    /// refuses a side effect inside its address.
    fn place(&self, node: &Value, store: bool) -> Result<Self::Facts, Refusal>;
    /// Whether no lvalue but the object's own name designates the object with
    /// this declaration id: an automatic local or formal whose address is
    /// never taken.
    fn unaliased(&self, decl: &str) -> bool;
}

/// Elaborate one full expression or one-declarator declaration.
pub(crate) fn elaborate<C: Context>(node: &Value, context: &C) -> Result<Elaboration<C::Facts>, Refusal> {
    let mut elaborator = Elaborator {
        context,
        graph: EvalGraph::default(),
        facts: BTreeMap::new(),
        obligations: Vec::new(),
        in_actual: false,
        actuals: BTreeMap::new(),
    };
    let part = if kind(node) == "DeclStmt" {
        elaborator.declaration(node)?
    } else {
        elaborator.expression(node)?
    };
    elaborator
        .graph
        .check(&|decl| context.unaliased(decl))?;
    let has_arithmetic = part.arithmetic;
    Ok(Elaboration {
        result: part.result,
        graph: elaborator.graph,
        facts: elaborator.facts,
        actuals: elaborator.actuals,
        obligations: elaborator.obligations,
        assumptions: if has_arithmetic {
            vec![Assumption::NoNumericClaim]
        } else {
            Vec::new()
        },
    })
}

/// The events of one sub-expression.
struct Part {
    /// Every event, value computations and side effects.
    all: Vec<EventId>,
    /// The value computation's events: what a consumer of the value is
    /// sequenced after. Side effects are not among them.
    values: Vec<EventId>,
    result: Operand,
    arithmetic: bool,
}

impl Part {
    fn empty(result: Operand) -> Self {
        Part {
            all: Vec::new(),
            values: Vec::new(),
            result,
            arithmetic: false,
        }
    }

    fn pointer(&self) -> bool {
        matches!(self.result, Operand::Value { pointer: true, .. })
    }
}

struct Elaborator<'a, C: Context> {
    context: &'a C,
    graph: EvalGraph,
    facts: BTreeMap<EventId, C::Facts>,
    obligations: Vec<Obligation>,
    /// Inside a call's actual: a further call is nested (CG05).
    in_actual: bool,
    actuals: BTreeMap<EventId, Vec<EventId>>,
}

fn qualified(node: &Value) -> Option<&'static str> {
    let text = crate::typed_access_path::qual(node).unwrap_or("");
    if text.contains("volatile") {
        Some("volatile access")
    } else if text.contains("_Atomic") {
        Some("atomic access")
    } else {
        None
    }
}

impl<C: Context> Elaborator<'_, C> {
    fn value(&self, node: &Value) -> Result<bool, Refusal> {
        if let Some(qualifier) = qualified(node) {
            return Err(refused(qualifier));
        }
        if self.context.scalar(node) {
            Ok(false)
        } else if self.context.pointer(node) {
            Ok(true)
        } else {
            Err(Refusal::Unlisted("value of an unread type"))
        }
    }

    fn computed(&self, node: &Value, all: Vec<EventId>, values: Vec<EventId>) -> Result<Part, Refusal> {
        Ok(Part {
            all,
            values,
            result: Operand::Value {
                pointer: self.value(node)?,
                source: ValueSource::Computed,
            },
            arithmetic: true,
        })
    }

    /// The named object a `DeclRefExpr` designates.
    fn object(node: &Value) -> Result<Place, Refusal> {
        let reference = &node["referencedDecl"];
        match reference["kind"].as_str() {
            Some("VarDecl" | "ParmVarDecl") => Ok(Place::Object {
                decl: reference["id"]
                    .as_str()
                    .ok_or(Refusal::Unlisted("reference without an id"))?
                    .to_owned(),
                name: reference["name"]
                    .as_str()
                    .ok_or(Refusal::Unlisted("reference without a name"))?
                    .to_owned(),
            }),
            _ => Err(Refusal::Unlisted("reference to a non-object")),
        }
    }

    /// An lvalue: a named object (no address events) or a path the context
    /// admits, with the reads of its address computation.
    fn lvalue(&mut self, node: &Value, store: bool) -> Result<(Place, Vec<EventId>, Option<C::Facts>), Refusal> {
        let bare = strip_parens(node);
        if let Some(qualifier) = qualified(bare) {
            return Err(refused(qualifier));
        }
        match kind(bare) {
            "DeclRefExpr" => Ok((Self::object(bare)?, Vec::new(), None)),
            "MemberExpr" | "ArraySubscriptExpr" => self.path(bare, store),
            "UnaryOperator" if bare["opcode"].as_str() == Some("*") => self.path(bare, store),
            other => Err(Refusal::UnlistedKind(format!("lvalue {other}"))),
        }
    }

    fn path(&mut self, node: &Value, store: bool) -> Result<(Place, Vec<EventId>, Option<C::Facts>), Refusal> {
        let place = path_place(node)?;
        let facts = self.context.place(node, store)?;
        let mut reads = Vec::new();
        address_reads(node, true, &mut reads)?;
        let mut events = Vec::new();
        for read in reads {
            let read = strip_parens(read);
            let place = if kind(read) == "DeclRefExpr" {
                Self::object(read)?
            } else {
                path_place(read)?
            };
            events.push(self.graph.push(Event::Read(place)));
        }
        self.obligations.push(Obligation::Access(place.clone()));
        Ok((place, events, Some(facts)))
    }

    fn record(&mut self, event: EventId, facts: &Option<C::Facts>) {
        if let Some(facts) = facts {
            self.facts.insert(event, facts.clone());
        }
    }

    fn declaration(&mut self, node: &Value) -> Result<Part, Refusal> {
        let [declaration] = children(node) else {
            return Err(Refusal::Declaration("not exactly one declarator"));
        };
        if kind(declaration) != "VarDecl" {
            return Err(Refusal::Declaration("not a variable"));
        }
        if declaration["storageClass"].as_str().is_some() {
            return Err(Refusal::Declaration("storage class"));
        }
        let pointer = self.value(declaration)?;
        let place = Place::Object {
            decl: declaration["id"]
                .as_str()
                .ok_or(Refusal::Declaration("no id"))?
                .to_owned(),
            name: declaration["name"]
                .as_str()
                .ok_or(Refusal::Declaration("no name"))?
                .to_owned(),
        };
        let [initializer] = children(declaration) else {
            return Err(Refusal::Declaration("no single initializer"));
        };
        let value = self.expression(initializer)?;
        if value.pointer() != pointer {
            return Err(Refusal::Unlisted("initializer of another class"));
        }
        let write = self.graph.push(Event::Write {
            place: place.clone(),
            pointer,
        });
        self.graph.order(&value.values, write);
        let mut all = value.all;
        all.push(write);
        Ok(Part {
            all,
            values: Vec::new(),
            result: Operand::Void,
            arithmetic: value.arithmetic,
        })
    }

    fn expression(&mut self, node: &Value) -> Result<Part, Refusal> {
        match kind(node) {
            "ParenExpr" => self.expression(children(node).first().ok_or(Refusal::Unlisted("empty parentheses"))?),
            "IntegerLiteral" | "CharacterLiteral" | "FloatingLiteral" => Ok(Part::empty(Operand::Value {
                pointer: false,
                source: ValueSource::Computed,
            })),
            "DeclRefExpr" if node["referencedDecl"]["kind"].as_str() == Some("EnumConstantDecl") => {
                Ok(Part::empty(Operand::Value {
                    pointer: false,
                    source: ValueSource::Computed,
                }))
            }
            "ImplicitCastExpr" => self.implicit_cast(node),
            "CStyleCastExpr" => {
                let inner = children(node).first().ok_or(Refusal::Unlisted("empty cast"))?;
                if self.context.scalar(node) && self.context.scalar(inner) {
                    self.expression(inner)
                } else {
                    Err(Refusal::Cast(node["castKind"].as_str().unwrap_or("?").to_owned()))
                }
            }
            "UnaryExprOrTypeTraitExpr" => {
                if children(node).is_empty() && self.context.scalar(node) {
                    Ok(Part::empty(Operand::Value {
                        pointer: false,
                        source: ValueSource::Computed,
                    }))
                } else {
                    Err(refused("type query operand"))
                }
            }
            "UnaryOperator" => self.unary(node),
            "BinaryOperator" => self.binary(node),
            "CompoundAssignOperator" => self.compound(node),
            "CallExpr" => self.call(node),
            "ConditionalOperator" | "BinaryConditionalOperator" => Err(refused("conditional")),
            "GenericSelectionExpr" => Err(refused("generic selection")),
            "StmtExpr" => Err(refused("statement expression")),
            "CompoundLiteralExpr" => Err(refused("compound literal")),
            "InitListExpr" => Err(refused("compound literal")),
            other => Err(Refusal::UnlistedKind(other.to_owned())),
        }
    }

    fn implicit_cast(&mut self, node: &Value) -> Result<Part, Refusal> {
        let inner = children(node).first().ok_or(Refusal::Unlisted("empty cast"))?;
        match node["castKind"].as_str().unwrap_or("") {
            "LValueToRValue" => {
                let pointer = self.value(node)?;
                let (place, mut events, facts) = self.lvalue(inner, false)?;
                let read = self.graph.push(Event::Read(place));
                self.record(read, &facts);
                self.graph.order(&events, read);
                events.push(read);
                Ok(Part {
                    all: events,
                    values: vec![read],
                    result: Operand::Value {
                        pointer,
                        source: ValueSource::Computed,
                    },
                    arithmetic: false,
                })
            }
            "NoOp" | "IntegralCast" => {
                if !self.context.scalar(node) {
                    return Err(Refusal::Cast(node["castKind"].as_str().unwrap_or("?").to_owned()));
                }
                self.expression(inner)
            }
            "FunctionToPointerDecay" => Err(refused("indirect call")),
            other => Err(Refusal::Cast(other.to_owned())),
        }
    }

    fn unary(&mut self, node: &Value) -> Result<Part, Refusal> {
        let operand = children(node).first().ok_or(Refusal::Unlisted("empty unary"))?;
        match node["opcode"].as_str().unwrap_or("") {
            "+" | "-" | "!" | "~" => {
                let part = self.expression(operand)?;
                if part.pointer() {
                    return Err(refused("pointer arithmetic"));
                }
                self.computed(node, part.all, part.values)
            }
            "++" | "--" => {
                let postfix = node["isPostfix"].as_bool() == Some(true);
                let (place, mut events, facts) = self.lvalue(operand, true)?;
                let pointer = self.value(operand)?;
                if pointer {
                    return Err(refused("pointer arithmetic"));
                }
                let read = self.graph.push(Event::Read(place.clone()));
                self.record(read, &facts);
                self.graph.order(&events, read);
                let write = self.graph.push(Event::Write {
                    place: place.clone(),
                    pointer: false,
                });
                self.record(write, &facts);
                self.graph.order(&[read], write);
                let mut values = events.clone();
                values.push(read);
                events.extend([read, write]);
                Ok(Part {
                    all: events,
                    values,
                    result: Operand::Value {
                        pointer: false,
                        source: if postfix {
                            ValueSource::Old(place)
                        } else {
                            ValueSource::New(place)
                        },
                    },
                    arithmetic: true,
                })
            }
            // A scalar `*p` is always read through an lvalue conversion, which
            // `implicit_cast` handles; a bare one is a designator.
            "*" => Err(Refusal::Unlisted("dereference outside a read")),
            "&" => Err(Refusal::Unlisted("address-of")),
            other => Err(Refusal::UnlistedKind(format!("unary {other}"))),
        }
    }

    fn binary(&mut self, node: &Value) -> Result<Part, Refusal> {
        let [left, right] = children(node) else {
            return Err(Refusal::Unlisted("binary arity"));
        };
        match node["opcode"].as_str().unwrap_or("") {
            "," => {
                let first = self.expression(left)?;
                let second = self.expression(right)?;
                // The left operand completes, then the right one runs.
                for &event in &second.all {
                    self.graph.order(&first.all, event);
                }
                // The comma's value is available only after its left operand
                // completed: with no value event on the right (a literal),
                // the left operand's events stand for that point.
                let values = if second.values.is_empty() {
                    first.all.clone()
                } else {
                    second.values
                };
                let mut all = first.all;
                all.extend(second.all);
                Ok(Part {
                    all,
                    values,
                    result: second.result,
                    arithmetic: first.arithmetic || second.arithmetic,
                })
            }
            "=" => {
                let (place, mut events, facts) = self.lvalue(left, true)?;
                let pointer = self.value(left)?;
                let value = self.expression(right)?;
                if value.pointer() != pointer {
                    return Err(Refusal::Unlisted("assignment of another class"));
                }
                self.graph
                    .unsequenced
                    .push(vec![events.clone(), value.all.clone()]);
                let write = self.graph.push(Event::Write {
                    place: place.clone(),
                    pointer,
                });
                self.record(write, &facts);
                self.graph.order(&events, write);
                self.graph.order(&value.values, write);
                let mut values = events.clone();
                values.extend(value.values.iter().copied());
                events.extend(value.all);
                events.push(write);
                Ok(Part {
                    all: events,
                    values,
                    result: Operand::Value {
                        pointer,
                        source: ValueSource::New(place),
                    },
                    arithmetic: value.arithmetic,
                })
            }
            "&&" => Err(refused("value &&")),
            "||" => Err(refused("value ||")),
            "+" | "-" | "*" | "/" | "%" | "<<" | ">>" | "&" | "|" | "^" | "==" | "!=" | "<"
            | ">" | "<=" | ">=" => {
                let first = self.expression(left)?;
                let second = self.expression(right)?;
                if first.pointer() || second.pointer() {
                    return Err(refused("pointer arithmetic"));
                }
                self.graph
                    .unsequenced
                    .push(vec![first.all.clone(), second.all.clone()]);
                let mut all = first.all;
                all.extend(second.all);
                let mut values = first.values;
                values.extend(second.values);
                self.computed(node, all, values)
            }
            other => Err(Refusal::UnlistedKind(format!("binary {other}"))),
        }
    }

    fn compound(&mut self, node: &Value) -> Result<Part, Refusal> {
        let [left, right] = children(node) else {
            return Err(Refusal::Unlisted("compound arity"));
        };
        let (place, mut events, facts) = self.lvalue(left, true)?;
        if self.value(left)? {
            return Err(refused("pointer arithmetic"));
        }
        // The place is evaluated once: one read of it, then (after the right
        // operand's value) one write back.
        let read = self.graph.push(Event::Read(place.clone()));
        self.record(read, &facts);
        self.graph.order(&events, read);
        let value = self.expression(right)?;
        if value.pointer() {
            return Err(refused("pointer arithmetic"));
        }
        let mut left_all = events.clone();
        left_all.push(read);
        self.graph
            .unsequenced
            .push(vec![left_all, value.all.clone()]);
        let write = self.graph.push(Event::Write {
            place: place.clone(),
            pointer: false,
        });
        self.record(write, &facts);
        self.graph.order(&[read], write);
        self.graph.order(&value.values, write);
        let mut values = events.clone();
        values.push(read);
        values.extend(value.values.iter().copied());
        events.push(read);
        events.extend(value.all);
        events.push(write);
        Ok(Part {
            all: events,
            values,
            result: Operand::Value {
                pointer: false,
                source: ValueSource::New(place),
            },
            arithmetic: true,
        })
    }

    fn call(&mut self, node: &Value) -> Result<Part, Refusal> {
        if self.in_actual {
            return Err(refused("nested call"));
        }
        let (callee, actuals) = children(node)
            .split_first()
            .ok_or(Refusal::Unlisted("call without a callee"))?;
        let name = direct_callee(callee).ok_or_else(|| refused("indirect call"))?;
        self.in_actual = true;
        let mut operands = Vec::new();
        let mut all = Vec::new();
        for actual in actuals {
            let part = match self.expression(actual) {
                Ok(part) => part,
                Err(refusal) => {
                    self.in_actual = false;
                    return Err(refusal);
                }
            };
            operands.push(part.all.clone());
            all.extend(part.all);
        }
        self.in_actual = false;
        if operands.len() > 1 {
            self.graph.unsequenced.push(operands);
        }
        // The sequence point before the call: every event of the designator
        // and the actuals, side effects included, precedes it.
        let call = self.graph.push(Event::Call {
            callee: name.to_owned(),
        });
        self.graph.order(&all, call);
        self.actuals.insert(call, all.clone());
        self.obligations.push(Obligation::Contract(name.to_owned()));
        all.push(call);
        let pointer = if kind(node) == "CallExpr" && crate::typed_access_path::qual(node) == Some("void") {
            None
        } else {
            Some(self.value(node)?)
        };
        Ok(Part {
            all,
            values: vec![call],
            result: match pointer {
                None => Operand::Void,
                Some(pointer) => Operand::Value {
                    pointer,
                    source: ValueSource::Call(name.to_owned()),
                },
            },
            arithmetic: false,
        })
    }
}

/// Parentheses carry no evaluation.
pub(crate) fn strip_parens(mut node: &Value) -> &Value {
    while kind(node) == "ParenExpr" {
        match children(node).first() {
            Some(inner) => node = inner,
            None => break,
        }
    }
    node
}

/// The named function a callee designates through its function-to-pointer
/// decay, when it is one.
pub(crate) fn direct_callee(callee: &Value) -> Option<&str> {
    let mut node = callee;
    loop {
        match kind(node) {
            "ParenExpr" => node = children(node).first()?,
            "ImplicitCastExpr" if node["castKind"].as_str() == Some("FunctionToPointerDecay") => {
                node = children(node).first()?;
            }
            "DeclRefExpr" if node["referencedDecl"]["kind"].as_str() == Some("FunctionDecl") => {
                return node["referencedDecl"]["name"].as_str();
            }
            _ => return None,
        }
    }
}

/// The reads of a path's address computation: every operand of an
/// lvalue-to-rvalue conversion inside it (a pointer root, an index, a pointer
/// member read on the way). `outer` marks the place node itself, whose own
/// access is the caller's event. A side effect inside an address refuses.
fn address_reads<'a>(node: &'a Value, outer: bool, out: &mut Vec<&'a Value>) -> Result<(), Refusal> {
    match kind(node) {
        "CallExpr" | "StmtExpr" | "CompoundAssignOperator" => {
            return Err(Refusal::Place("effectful address".into()));
        }
        "UnaryOperator" if matches!(node["opcode"].as_str(), Some("++" | "--")) => {
            return Err(Refusal::Place("effectful address".into()));
        }
        "BinaryOperator" if matches!(node["opcode"].as_str(), Some("=" | ",")) => {
            return Err(Refusal::Place("effectful address".into()));
        }
        "ImplicitCastExpr" if !outer && node["castKind"].as_str() == Some("LValueToRValue") => {
            if let Some(inner) = children(node).first() {
                out.push(inner);
            }
        }
        _ => {}
    }
    for child in children(node) {
        address_reads(child, false, out)?;
    }
    Ok(())
}

/// The storage a member, subscript or dereference lvalue designates: its
/// root object, and whether the path leaves that object through a pointer.
pub(crate) fn path_place(node: &Value) -> Result<Place, Refusal> {
    let empty = || Refusal::Place("incomplete path".into());
    let mut spelling = Vec::new();
    let mut indirect = false;
    let mut cursor = node;
    loop {
        match kind(cursor) {
            "ParenExpr" => cursor = children(cursor).first().ok_or_else(empty)?,
            "ImplicitCastExpr" => {
                if cursor["castKind"].as_str() != Some("ArrayToPointerDecay") {
                    // A converted pointer value: the path continues through it.
                    indirect = true;
                }
                cursor = children(cursor).first().ok_or_else(empty)?;
            }
            "MemberExpr" => {
                let arrow = cursor["isArrow"].as_bool() == Some(true);
                indirect |= arrow;
                spelling.push(format!(
                    "{}{}",
                    if arrow { "->" } else { "." },
                    cursor["name"].as_str().unwrap_or("?")
                ));
                cursor = children(cursor).first().ok_or_else(empty)?;
            }
            "ArraySubscriptExpr" => {
                spelling.push("[]".to_owned());
                cursor = children(cursor).first().ok_or_else(empty)?;
            }
            "UnaryOperator" if cursor["opcode"].as_str() == Some("*") => {
                indirect = true;
                spelling.push("*".to_owned());
                cursor = children(cursor).first().ok_or_else(empty)?;
            }
            "DeclRefExpr" => {
                let reference = &cursor["referencedDecl"];
                if !matches!(reference["kind"].as_str(), Some("VarDecl" | "ParmVarDecl")) {
                    return Err(Refusal::Place("path not rooted at an object".into()));
                }
                let name = reference["name"].as_str().unwrap_or("?");
                spelling.reverse();
                return Ok(Place::Path {
                    spelling: format!("{name}{}", spelling.concat()),
                    root: reference["id"].as_str().ok_or_else(empty)?.to_owned(),
                    indirect,
                });
            }
            other => return Err(Refusal::Place(format!("path through {other}"))),
        }
    }
}

/// The call events of a graph (tests).
#[cfg(test)]
pub(crate) fn calls(graph: &EvalGraph) -> Vec<(EventId, &str)> {
    graph
        .events
        .iter()
        .enumerate()
        .filter_map(|(id, event)| match event {
            Event::Call { callee } => Some((id, callee.as_str())),
            _ => None,
        })
        .collect()
}

/// The unordered pairs of a graph (tests).
#[cfg(test)]
pub(crate) fn unordered_pairs(graph: &EvalGraph) -> Vec<(EventId, EventId)> {
    let mut pairs = Vec::new();
    for a in 0..graph.events.len() {
        for b in a + 1..graph.events.len() {
            if !graph.sequenced(a, b) {
                pairs.push((a, b));
            }
        }
    }
    pairs
}

#[cfg(test)]
#[path = "../tests/unit/elaboration.rs"]
mod tests;
