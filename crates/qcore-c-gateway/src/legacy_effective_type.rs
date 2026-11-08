//! Card **OUT-PARAMETER-SCALAR-FRAME-PREMISE** (gateway rule 11r(7), user
//! decision 2026-11-07, "A: effective-type"): rule 11r(6)'s per-class body
//! summary may keep a pointer-typed out slot across a scalar store through a
//! *second* out formal, under the listed premise `effective-type:`.
//!
//! # Why
//!
//! `int f(…, char **buf, size_t *size) { … *buf = data; *size = n; return 0; }`
//! stores the resource and then its size. The scalar store's refused-write
//! frame may write any object `size` points to, so it poisons the binding of
//! the slot `*buf` (a slot is never spared, `RefusedWrites::Sparing`
//! premise 1), the slot is `Ambiguous` at the exit, and the class does not
//! validate. Under C11 6.5p7 an lvalue of a non-character integer or
//! enumeration type cannot access, so cannot modify, an object whose
//! effective type is a pointer type: a program where it did would have
//! undefined behaviour. The premise is that reading, stated in the row's
//! assumptions, never silent.
//!
//! # Where it applies
//!
//! Only inside [`with_premise`], which rule 11r(6)'s routes open for one
//! function when every older reading failed (the per-value inference of an
//! inferred relation and its per-class row). There, and only when the unit's
//! build leaves the rule in force (no `-fno-strict-aliasing`,
//! [`crate::typed_access_path::strict_aliasing_in_force`]), an analysis
//! carries a [`Frame`]: the sanctioned out slots `*X` whose formal Clang types
//! as a pointer to an object pointer, and the statements `*Y = V` (or `*Y op=
//! V`) whose left operand Clang types as a non-character integer or an
//! enumeration, `Y` a formal other than every such `X`, keyed by line and `Y`
//! (a line holding another store through `Y` of any other type is not keyed).
//! A typed scalar store at such a node then spares exactly those slots from
//! its frame, and the analysis notes that it relied on the premise.
//!
//! What still poisons: a character store (`char`, `signed char`, `unsigned
//! char`, which 6.5p7 lets alias anything), a floating store, a store whose
//! lvalue type the AST does not give, a store through a base that is not a
//! formal, every call (`memcpy`, `memmove`, `memset` included), and every
//! store in a unit built with `-fno-strict-aliasing`. Every other refusal of
//! rule 11r(6) is unchanged: undecided paths, a slot written on some exits
//! only, a store after release, two slots given one site.
//!
//! The Lean model is `QCore.OutParameterScalarFramePremise`.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

/// The premise a row resting on this reading lists among its assumptions.
pub(crate) const PREMISE: &str = "effective-type: a non-character integer or enumeration store \
     through another out formal does not modify a pointer out slot (C11 6.5p7)";

/// The rule the premise is carried under.
pub(crate) const RULE: &str = "11r";

#[derive(Debug)]
struct Scope {
    function: String,
    used: bool,
}

thread_local! {
    static SCOPES: RefCell<Vec<Scope>> = const { RefCell::new(Vec::new()) };
    /// Functions one of whose classes the current inference read under the
    /// premise; taken (and cleared) by the route that publishes the relation.
    static PREMISED: RefCell<BTreeSet<String>> = const { RefCell::new(BTreeSet::new()) };
}

/// Note that a class of `function`'s relation rests on the premise.
pub(crate) fn note_premised(function: &str) {
    PREMISED.with(|premised| {
        premised.borrow_mut().insert(function.to_owned());
    });
}

/// Whether a class of `function`'s relation rests on the premise; clears it.
pub(crate) fn take_premised(function: &str) -> bool {
    PREMISED.with(|premised| premised.borrow_mut().remove(function))
}

/// Run `work` with the premise available to `function`'s analyses, and
/// return whether any of them relied on it.
pub(crate) fn with_premise<R>(function: &str, work: impl FnOnce() -> R) -> (R, bool) {
    struct Pop;
    impl Drop for Pop {
        fn drop(&mut self) {
            SCOPES.with(|scopes| {
                scopes.borrow_mut().pop();
            });
        }
    }
    SCOPES.with(|scopes| {
        scopes.borrow_mut().push(Scope {
            function: function.to_owned(),
            used: false,
        });
    });
    let pop = Pop;
    let result = work();
    let used = SCOPES.with(|scopes| scopes.borrow().last().is_some_and(|scope| scope.used));
    drop(pop);
    (result, used)
}

fn active_for(function: &str) -> bool {
    SCOPES.with(|scopes| {
        scopes
            .borrow()
            .last()
            .is_some_and(|scope| scope.function == function)
    })
}

/// Note that the innermost scope's analysis relied on the premise.
pub(crate) fn note_used() {
    SCOPES.with(|scopes| {
        if let Some(scope) = scopes.borrow_mut().last_mut() {
            scope.used = true;
        }
    });
}

/// What one analysis may spare under the premise.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Frame {
    /// Sanctioned out slots `*X` whose formal is a pointer to an object pointer.
    slots: BTreeSet<String>,
    /// `(line, Y)`: the only write on that line is a non-character integer
    /// or enumeration store through `*Y`, `Y` a formal and no slot's formal.
    stores: BTreeSet<(usize, String)>,
}

impl Frame {
    /// Whether the typed scalar store at `line` (with no scalar destination),
    /// dereferencing `bases`, spares `place`: the line's one write is a keyed
    /// store through one of `bases`.
    pub(crate) fn spares(&self, line: usize, bases: &[String], place: &str) -> bool {
        self.slots.contains(place)
            && self
                .stores
                .iter()
                .any(|(at, base)| *at == line && bases.contains(base))
    }
}

/// The frame of `function`'s analysis whose sanctioned out slots are
/// `sanctioned`, or `None` outside [`with_premise`], in a unit built without
/// the effective-type rule, or where the AST does not read.
pub(crate) fn frame(function: &str, sanctioned: &BTreeSet<&str>) -> Option<Frame> {
    if sanctioned.is_empty()
        || !active_for(function)
        || !crate::typed_access_path::strict_aliasing_in_force()
    {
        return None;
    }
    crate::typed_access_path::with_indirect_call_unit(|unit| {
        let ast = unit.function(function)?;
        let formals = ast["inner"]
            .as_array()?
            .iter()
            .filter(|node| node["kind"].as_str() == Some("ParmVarDecl"))
            .filter_map(|node| Some((node["name"].as_str()?.to_owned(), node)))
            .collect::<Vec<_>>();
        let slots = sanctioned
            .iter()
            .filter_map(|place| {
                let name = place.strip_prefix('*')?;
                let (_, formal) = formals.iter().find(|(formal, _)| formal == name)?;
                pointer_to_object_pointer(formal).then(|| (*place).to_owned())
            })
            .collect::<BTreeSet<_>>();
        if slots.is_empty() {
            return None;
        }
        let slot_formals = slots
            .iter()
            .filter_map(|slot| slot.strip_prefix('*'))
            .collect::<BTreeSet<_>>();
        let formal_names = formals
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<BTreeSet<_>>();
        // Every write on each line (an assignment, compound assignment or
        // increment), and the lines whose one write is a keyed store.
        let mut writes: BTreeMap<usize, usize> = BTreeMap::new();
        let mut wide = BTreeSet::new();
        walk(ast, &mut |node| {
            if !is_write(node) {
                return;
            }
            let Some(line) = unit.node_line(node) else {
                return;
            };
            *writes.entry(line).or_default() += 1;
            let Some(target) = store_through(node) else {
                return;
            };
            if formal_names.contains(target)
                && !slot_formals.contains(target)
                && wide_integer_or_enum(unit, &node["inner"][0])
            {
                wide.insert((line, target.to_owned()));
            }
        });
        let stores = wide
            .into_iter()
            .filter(|(line, _)| writes.get(line) == Some(&1))
            .collect::<BTreeSet<_>>();
        Some(Frame { slots, stores })
    })
    .flatten()
}

/// An assignment, a compound assignment or an increment/decrement.
fn is_write(node: &Value) -> bool {
    match node["kind"].as_str() {
        Some("BinaryOperator") => node["opcode"].as_str() == Some("="),
        Some("CompoundAssignOperator") => true,
        Some("UnaryOperator") => matches!(node["opcode"].as_str(), Some("++" | "--")),
        _ => false,
    }
}

/// The name `Y` of an assignment `*Y = V` or `*Y op= V` (parentheses and
/// implicit conversions around `Y` read through).
fn store_through(node: &Value) -> Option<&str> {
    let assigns = match node["kind"].as_str()? {
        "BinaryOperator" => node["opcode"].as_str() == Some("="),
        "CompoundAssignOperator" => true,
        _ => false,
    };
    if !assigns {
        return None;
    }
    let mut lvalue = node["inner"].get(0)?;
    while lvalue["kind"].as_str() == Some("ParenExpr") {
        lvalue = lvalue["inner"].get(0)?;
    }
    if lvalue["kind"].as_str()? != "UnaryOperator" || lvalue["opcode"].as_str()? != "*" {
        return None;
    }
    let mut base = lvalue["inner"].get(0)?;
    while matches!(
        base["kind"].as_str(),
        Some("ImplicitCastExpr" | "ParenExpr")
    ) {
        base = base["inner"].get(0)?;
    }
    if base["kind"].as_str()? != "DeclRefExpr"
        || base["referencedDecl"]["kind"].as_str()? != "ParmVarDecl"
    {
        return None;
    }
    base["referencedDecl"]["name"].as_str()
}

/// A non-character integer or an enumeration, positively read from Clang's
/// type of `node`; `volatile`, `_Atomic`, floating and unread types are not.
fn wide_integer_or_enum(
    unit: &crate::typed_access_path::IndirectCallUnit<'_>,
    node: &Value,
) -> bool {
    if unit.is_enum(node) {
        return true;
    }
    let Some(text) = crate::typed_access_path::qual(node) else {
        return false;
    };
    let mut base = false;
    for word in text.split_whitespace() {
        match word {
            "const" => {}
            "signed" | "unsigned" | "short" | "long" | "int" | "_Bool" | "bool" | "__int128" => {
                base = true;
            }
            _ => return false,
        }
    }
    base
}

/// A formal of type `T **` with `T *` an object pointer, positively read.
fn pointer_to_object_pointer(formal: &Value) -> bool {
    let Some(text) = crate::typed_access_path::qual(formal) else {
        return false;
    };
    let text = text.trim_end();
    let Some(inner) = text.strip_suffix('*') else {
        return false;
    };
    let inner = inner.trim_end();
    inner.ends_with('*')
        && !text.contains('(')
        && !text.contains("volatile")
        && !text.contains("_Atomic")
        && !text.contains('[')
}

fn walk<'a>(node: &'a Value, visit: &mut impl FnMut(&'a Value)) {
    visit(node);
    if let Some(children) = node["inner"].as_array() {
        for child in children {
            walk(child, visit);
        }
    }
}
