//! Card **OUT-SLOT-CALL-WRITE-ALIASING** (gateway rule 11r(8), amending rule
//! 4's W4 `Out` summaries and rules 11r(4)/(6)/(7) and 12s(2)): a call that
//! may write caller memory poisons every bound out slot it may reach.
//!
//! # Why
//!
//! `int r(char **buf, size_t *size) { … *buf = data; memcpy(size, &n, sizeof
//! n); return 0; }` validated its out slot: a contracted call whose positions
//! are `Borrowed` writes no binding, so nothing touched `*buf`. But
//! `memcpy`'s copy is character-typed (C11 7.24.2.1, 6.5p7's character
//! exception) and `size` may designate the caller's `char *` object, or a byte
//! of it: the slot then holds no pointer the summary can name. A `Borrowed`
//! mode states that the call takes no custody through the position and
//! transfers no tracked obligation through it (contract specification C04.1,
//! "`resource_borrowed(p)`"); the v1 vocabulary has no read/write distinction
//! (C04.3.1 item 6), and rule 10k's W48 amendment already reads a `Borrowed`
//! mode alone as no read-only frame. So a call reaches whatever its actuals
//! designate, and an out slot is caller memory.
//!
//! # The decision
//!
//! An analysis whose contract sanctions out slots (`*X`, an `Out` formal)
//! carries a [`Frame`] read from the function's Clang AST: every call, the
//! lines of its statement, its callee, and the positions whose actual **may
//! designate caller memory**. At a node that applies a call
//! ([`call_bearing`]) on those lines, after the call's own effect, every
//! sanctioned slot still **bound** is poisoned (`unknown`, possibly null) as
//! the typed scalar store's refused-write frame poisons it: the slot is
//! `Ambiguous` at every exit it reaches, so no `Out` mode, unwritten class or
//! per-class row that needed it validates, and the function's row returns
//! to its uncontracted reading (`store through `*X` without an out-parameter
//! contract`). Nothing is accused.
//!
//! An actual may **not** designate caller memory when, and only when, it is
//! (positively read):
//!
//! * not a pointer: an integer, an enumeration, a floating value (a record or
//!   union passed by value may carry a pointer, so it reaches);
//! * a function pointer, a null pointer constant or a string literal;
//! * a pointer whose pointee Clang types `const` at the position (the
//!   parameter type the actual is converted to; in a variadic tail, the
//!   actual's own): the callee's prototype states it does not write through
//!   it, and C requires a cast to drop the qualifier;
//! * `&NAME`, `&NAME.f`, `&NAME[i]` or the decay of the array `NAME`, where
//!   `NAME` is an automatic local of this function or a formal's own
//!   variable whose declared type holds no pointer (no `*`, no record or
//!   union): an object of this frame, distinct from every caller object, and
//!   holding nothing that could lead the callee to one;
//! * at a position the call's own contract declares `Out`: the call states
//!   the object it writes there (`*actual`), and the slot that object is
//!   keeps the binding the call's effect gave it.
//!
//! Every other actual reaches: another formal (the slot's own formal at a
//! non-`Out` position is already refused, "out-slot pointer passed without a
//! modeled Out effect"), the value of a local pointer, a member or element
//! read, `*X`, a call's result, a file-scope or `static` object, an address of
//! a pointer or record local. The effective-type premise of rule 11r(7)
//! never spares a call: a call's write is untyped.
//!
//! What is not poisoned: a slot no store has bound yet. A `Borrowed`
//! position transfers no tracked obligation, so a write through it passes no
//! custody into the slot, and rule 11r(2)'s consumer already reads the
//! caller's variable as untracked on a class that leaves the slot unwritten.
//! Bindings other than sanctioned slots (members, locals) are not this rule's.
//!
//! When the AST does not read (no source context, no unique body), or no
//! call of the AST lies on a call-bearing node's lines, every position of
//! the node's calls but an `Out` one reaches: no reading is no exclusion.
//!
//! The Lean model is `QCore.OutSlotCallWrite`.

use std::collections::{BTreeMap, BTreeSet};

use qcore_schemas::contract::ResourceParameterMode;
use serde_json::Value;

use crate::legacy_c::LegacyInstruction;

/// One call of the function's body, as the frame keeps it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Call {
    /// The first line of the call's statement and the last line of the call.
    lines: (usize, usize),
    /// The directly named callee, when there is one.
    callee: Option<String>,
    /// The argument positions whose actual may designate caller memory.
    reaching: BTreeSet<usize>,
}

/// What a call may write, over one analysis.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Frame {
    /// The AST read: every call of the body.
    Read(Vec<Call>),
    /// The AST did not read: no actual is read as excluded.
    Unread,
}

impl Frame {
    /// Whether the call-bearing instruction at `line` may write a caller
    /// object: a call on its lines has a reaching position that is not an
    /// `Out` position of the instruction's own contract for that callee.
    /// With no call of the AST on those lines (or no AST), every position
    /// but an `Out` one reaches.
    pub(crate) fn writes(&self, line: usize, instruction: &LegacyInstruction) -> bool {
        let applied = call_bearing(instruction);
        if applied.is_empty() {
            return false;
        }
        let on_line = match self {
            Self::Read(calls) => calls
                .iter()
                .filter(|call| call.lines.0 <= line && line <= call.lines.1)
                .collect::<Vec<_>>(),
            Self::Unread => Vec::new(),
        };
        if on_line.is_empty() {
            return applied.iter().any(|call| {
                (0..call.positions).any(|position| !call.out_positions.contains(&position))
            });
        }
        on_line.iter().any(|call| {
            let out = applied
                .iter()
                .filter(|applied| Some(applied.callee) == call.callee.as_deref())
                .flat_map(|applied| applied.out_positions.iter().copied())
                .collect::<BTreeSet<_>>();
            call.reaching.iter().any(|position| !out.contains(position))
        })
    }
}

/// A call an instruction applies: its callee, its positions and its `Out`
/// positions.
#[derive(Debug)]
pub(crate) struct Applied<'a> {
    callee: &'a str,
    positions: usize,
    out_positions: Vec<usize>,
    /// The places the `Out` positions write (`*X` for a forwarded slot).
    pub(crate) out_places: Vec<&'a str>,
}

impl<'a> Applied<'a> {
    fn borrowed(callee: &'a str, positions: usize) -> Self {
        Self {
            callee,
            positions,
            out_positions: Vec::new(),
            out_places: Vec::new(),
        }
    }
}

/// The calls `instruction` applies; empty for an instruction that applies
/// none. A refusal (`Unknown`) already poisons every slot binding.
pub(crate) fn call_bearing(instruction: &LegacyInstruction) -> Vec<Applied<'_>> {
    let mut found = Vec::new();
    collect(instruction, &mut found);
    found
}

fn collect<'a>(instruction: &'a LegacyInstruction, found: &mut Vec<Applied<'a>>) {
    match instruction {
        LegacyInstruction::ContractCall {
            function,
            arguments,
            contract,
            nested,
            ..
        } => {
            let out = contract
                .parameters
                .iter()
                .enumerate()
                .filter(|(_, mode)| matches!(mode, ResourceParameterMode::Out { .. }))
                .map(|(index, _)| index)
                .collect::<Vec<_>>();
            found.push(Applied {
                callee: function,
                positions: contract.parameters.len().max(arguments.len()),
                out_places: out
                    .iter()
                    .filter_map(|index| arguments.get(*index)?.as_deref())
                    .collect(),
                out_positions: out,
            });
            for call in nested {
                found.push(Applied::borrowed(
                    &call.function,
                    call.contract.parameters.len().max(call.arguments.len()),
                ));
            }
        }
        LegacyInstruction::ElementStore {
            call: Some(call), ..
        } => collect(call, found),
        LegacyInstruction::ElementRelease { callee, .. } => {
            found.push(Applied::borrowed(callee, 1));
        }
        LegacyInstruction::SubObjectStore {
            value: crate::legacy_c::sub_object_store::StoredValue::Call(call),
            ..
        } => collect(call, found),
        LegacyInstruction::SelfInitCall { function, .. }
        | LegacyInstruction::TrackedSelfInit { function, .. } => {
            found.push(Applied::borrowed(function, 1));
        }
        _ => {}
    }
}

/// The frame of `function`'s analysis whose sanctioned out slots are
/// `sanctioned`: `None` when it sanctions none (nothing to poison).
pub(crate) fn frame(function: &str, sanctioned: &BTreeSet<&str>) -> Option<Frame> {
    if sanctioned.is_empty() {
        return None;
    }
    Some(
        crate::typed_access_path::with_indirect_call_unit(|unit| {
            let ast = unit.function(function)?;
            let mut automatic = BTreeMap::new();
            walk(ast, &mut |node| {
                if matches!(node["kind"].as_str(), Some("VarDecl" | "ParmVarDecl"))
                    && let Some(id) = node["id"].as_str()
                {
                    let local = node["kind"].as_str() == Some("ParmVarDecl")
                        || !matches!(node["storageClass"].as_str(), Some("static" | "extern"));
                    automatic.insert(id.to_owned(), local && pointer_free(node));
                }
            });
            let mut calls = Vec::new();
            let line = |node: &Value| unit.node_line(node);
            statements(ast, None, &line, &mut |node, statement| {
                if node["kind"].as_str() != Some("CallExpr") {
                    return;
                }
                let arguments = node["inner"]
                    .as_array()
                    .map_or(&[][..], |inner| inner.get(1..).unwrap_or(&[]));
                let reaching = arguments
                    .iter()
                    .enumerate()
                    .filter(|(_, argument)| may_reach(argument, &automatic))
                    .map(|(index, _)| index)
                    .collect::<BTreeSet<_>>();
                let Some(first) = statement.or_else(|| line(node)) else {
                    calls.push(None);
                    return;
                };
                let mut last = first;
                walk(node, &mut |inner| {
                    if let Some(at) = line(inner) {
                        last = last.max(at);
                    }
                });
                calls.push(Some(Call {
                    lines: (first.min(last), last),
                    callee: callee(node).map(str::to_owned),
                    reaching,
                }));
            });
            // A call whose lines the AST does not give is no exclusion.
            calls.into_iter().collect::<Option<Vec<_>>>()
        })
        .flatten()
        .map_or(Frame::Unread, Frame::Read),
    )
}

/// The directly named callee of a call (`f(…)`, `(f)(…)`).
fn callee(call: &Value) -> Option<&str> {
    let mut node = call["inner"].get(0)?;
    while matches!(
        node["kind"].as_str(),
        Some("ImplicitCastExpr" | "ParenExpr")
    ) {
        node = node["inner"].get(0)?;
    }
    (node["kind"].as_str()? == "DeclRefExpr"
        && node["referencedDecl"]["kind"].as_str()? == "FunctionDecl")
        .then(|| node["referencedDecl"]["name"].as_str())
        .flatten()
}

/// Whether `argument` may designate caller memory (the module's list,
/// read positively; everything unread reaches).
fn may_reach(argument: &Value, automatic: &BTreeMap<String, bool>) -> bool {
    let Some(text) = crate::typed_access_path::qual(argument) else {
        return true;
    };
    if !is_pointer(text) {
        return is_record(text);
    }
    if text.contains("(*)(") || pointee_const(text) {
        return false;
    }
    let mut node = argument;
    loop {
        match node["kind"].as_str() {
            Some("ImplicitCastExpr" | "CStyleCastExpr")
                if node["castKind"].as_str() == Some("NullToPointer") =>
            {
                return false;
            }
            Some("ImplicitCastExpr")
                if node["castKind"].as_str() == Some("ArrayToPointerDecay") =>
            {
                let Some(array) = node["inner"].get(0) else {
                    return true;
                };
                let array = unparenthesised(array);
                if array["kind"].as_str() == Some("StringLiteral") {
                    return false;
                }
                return !frame_object(array, automatic);
            }
            Some("ParenExpr" | "ImplicitCastExpr" | "CStyleCastExpr") => {
                let Some(inner) = node["inner"].get(0) else {
                    return true;
                };
                node = inner;
            }
            Some("UnaryOperator") if node["opcode"].as_str() == Some("&") => {
                return node["inner"]
                    .get(0)
                    .is_none_or(|object| !frame_object(object, automatic));
            }
            _ => return true,
        }
    }
}

fn unparenthesised(mut node: &Value) -> &Value {
    while node["kind"].as_str() == Some("ParenExpr") {
        match node["inner"].get(0) {
            Some(inner) => node = inner,
            None => break,
        }
    }
    node
}

/// Whether the lvalue `object` designates (part of) an automatic local or a
/// formal's own variable whose declared type holds no pointer: `NAME`,
/// `NAME.f`, `NAME[i]` with `NAME` an array.
fn frame_object(object: &Value, automatic: &BTreeMap<String, bool>) -> bool {
    let object = unparenthesised(object);
    match object["kind"].as_str() {
        Some("DeclRefExpr") => object["referencedDecl"]["id"]
            .as_str()
            .and_then(|id| automatic.get(id))
            .copied()
            .unwrap_or(false),
        Some("MemberExpr") => {
            object["isArrow"].as_bool() == Some(false)
                && object["inner"]
                    .get(0)
                    .is_some_and(|base| frame_object(base, automatic))
        }
        Some("ArraySubscriptExpr") => object["inner"].get(0).is_some_and(|base| {
            let base = unparenthesised(base);
            base["kind"].as_str() == Some("ImplicitCastExpr")
                && base["castKind"].as_str() == Some("ArrayToPointerDecay")
                && base["inner"]
                    .get(0)
                    .is_some_and(|array| frame_object(array, automatic))
        }),
        _ => false,
    }
}

/// A declaration whose type (Clang's desugared spelling) holds no pointer:
/// no `*`, no record or union, no function type, no `volatile`/`_Atomic`.
fn pointer_free(declaration: &Value) -> bool {
    crate::typed_access_path::qual(declaration).is_some_and(|text| {
        !text.contains('*')
            && !text.contains('(')
            && !is_record(text)
            && !text.contains("volatile")
            && !text.contains("_Atomic")
    })
}

fn is_record(text: &str) -> bool {
    text.split(|c: char| !c.is_alphanumeric() && c != '_')
        .any(|word| matches!(word, "struct" | "union"))
}

const QUALIFIERS: [&str; 4] = ["const", "volatile", "restrict", "__restrict"];

/// `text` with its top-level qualifiers removed (`char *const` -> `char *`).
fn unqualified(text: &str) -> &str {
    let mut text = text.trim();
    loop {
        let before = text;
        for qualifier in QUALIFIERS {
            if let Some(rest) = text.strip_suffix(qualifier)
                && rest.ends_with([' ', '*'])
            {
                text = rest.trim_end();
            }
        }
        if text == before {
            return text;
        }
    }
}

fn is_pointer(text: &str) -> bool {
    unqualified(text).ends_with('*') || text.contains("(*)")
}

/// Whether the object a pointer of type `text` designates is `const` (a
/// pointer to an array, `T (*)[N]`, is read as not: it reaches).
fn pointee_const(text: &str) -> bool {
    let Some(inner) = unqualified(text).strip_suffix('*') else {
        return false;
    };
    let inner = inner.trim_end();
    if inner.contains('*') {
        // `T *const *`: the pointee is a const pointer; `T **`: a mutable one.
        return unqualified(inner) != inner && inner.ends_with("const");
    }
    inner.split_whitespace().any(|word| word == "const")
}

fn walk<'a>(node: &'a Value, visit: &mut impl FnMut(&'a Value)) {
    visit(node);
    if let Some(children) = node["inner"].as_array() {
        for child in children {
            walk(child, visit);
        }
    }
}

/// Visit every node with the first line of the statement holding it: a
/// compound statement's child starts a statement; everything else inherits.
fn statements<'a>(
    node: &'a Value,
    statement: Option<usize>,
    line: &dyn Fn(&Value) -> Option<usize>,
    visit: &mut dyn FnMut(&'a Value, Option<usize>),
) {
    visit(node, statement);
    let compound = node["kind"].as_str() == Some("CompoundStmt");
    if let Some(children) = node["inner"].as_array() {
        for child in children {
            let inner = if compound { line(child) } else { statement };
            statements(child, inner, line, visit);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qualifier_readings() {
        assert!(is_pointer("const void **"));
        assert!(!pointee_const("const void **"));
        assert!(pointee_const("const void *"));
        assert!(!pointee_const("void *"));
        assert!(is_pointer("char *const"));
        assert!(!pointee_const("char *const"));
        assert!(pointee_const("const char *const"));
        assert!(pointee_const("char *const *"));
        assert!(!is_pointer("unsigned long"));
        assert!(is_pointer("void (*)(int)"));
        assert!(is_record("struct x_chunk"));
        assert!(!is_record("unsigned long"));
    }
}
