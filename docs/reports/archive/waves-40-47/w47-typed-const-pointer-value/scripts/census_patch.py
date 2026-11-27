#!/usr/bin/env python3
"""Temporary census patch (never committed): at the base, mark every typed
row the reader refuses **only** at the value-type check of a `T *const`
pointer value (bit 1) or at `strip`'s refusal of an implicit `void *` BitCast
in a pointer-destination statement (bit 2), and whose read would otherwise be
admitted by the unchanged API (every other check, root premise included).
The row keeps its refusal; a `W47CP` line goes to stderr from the hook.

Usage: census_patch.py WORKTREE apply|revert   (revert = git checkout of the
three files, run by the caller)
"""
import pathlib
import sys

W = pathlib.Path(sys.argv[1])
TAP = W / "crates/qcore-c-gateway/src/typed_access_path.rs"
POS = W / "crates/qcore-c-gateway/src/typed_access_positions.rs"
HOOK = W / "crates/qcore-c-gateway/src/legacy_typed_positions.rs"
LEG = W / "crates/qcore-c-gateway/src/legacy_c.rs"


def sub(text, old, new, count=1):
    assert text.count(old) == count, (old, text.count(old))
    return text.replace(old, new)


HELPERS = r'''
thread_local! {
    pub(crate) static W47CP: std::cell::Cell<u8> = const { std::cell::Cell::new(0) };
}

pub(crate) fn w47cp_reset() {
    W47CP.with(|cell| cell.set(0));
}

pub(crate) fn w47cp_mark(bit: u8) {
    W47CP.with(|cell| cell.set(cell.get() | bit));
}

/// True when the census marked the read: complete it as a refusal.
pub(crate) fn w47cp_would_admit() -> bool {
    W47CP.with(|cell| {
        if cell.get() & 3 != 0 {
            cell.set(cell.get() | 0x80);
            true
        } else {
            false
        }
    })
}

pub(crate) fn w47cp_census(function: &str, line: usize, texts: &[&str], position: &str) {
    let flag = W47CP.with(|cell| cell.replace(0));
    if flag & 0x80 != 0 {
        eprintln!("W47CP\t{function}\t{line}\t{position}\t{}\t{}", flag & 3, texts.join(" "));
    }
}

/// The candidate rule: Clang's canonical type is an object pointer whose only
/// top-level qualifier is `const`.
pub(crate) fn w47cp_const(node: &Value) -> bool {
    let Some(text) = qual(node) else { return false };
    let Some(star) = text.rfind('*') else { return false };
    let words = text[star + 1..].split_whitespace().collect::<Vec<_>>();
    if words != ["const"] {
        return false;
    }
    let unqualified = &text[..=star];
    !unqualified.contains("union ")
        && !unqualified.contains("volatile")
        && !unqualified.contains("_Atomic")
        && !unqualified.contains('(')
}

/// `ImplicitCastExpr <BitCast>` from `void *` to an object pointer over a
/// value `strip` reads.
pub(crate) fn w47cp_bitcast(node: &Value) -> Option<&Value> {
    if kind(node) != "ImplicitCastExpr" || node["castKind"].as_str()? != "BitCast" {
        return None;
    }
    let target = qual(node)?;
    let pointee = target[..target.rfind('*')?]
        .split_whitespace()
        .filter(|word| !matches!(*word, "const" | "volatile" | "restrict"))
        .collect::<Vec<_>>();
    if !is_pointer(node) || pointee == ["void"] {
        return None;
    }
    let inner = children(node).first()?;
    if qual(inner)? != "void *" {
        return None;
    }
    strip(inner)
}
'''


def apply():
    s = TAP.read_text()
    s = sub(s, "pub(crate) fn strip(mut node: &Value)", HELPERS + "\npub(crate) fn strip(mut node: &Value)")
    s = sub(s, """        if !(self.scalar(node) || (pointer_read && !store && is_pointer(node))) {
            return None;
        }""", """        if !(self.scalar(node) || (pointer_read && !store && is_pointer(node))) {
            if pointer_read && !store && w47cp_const(node) {
                w47cp_mark(1);
            } else {
                return None;
            }
        }""")
    s = sub(s, """                    self.plan.borrowed_destination = true;
                    return self.path(strip(&pair[1])?, false, true);""", """                    self.plan.borrowed_destination = true;
                    let source = match strip(&pair[1]) {
                        Some(source) => source,
                        None => {
                            let source = w47cp_bitcast(&pair[1])?;
                            w47cp_mark(2);
                            source
                        }
                    };
                    return self.path(source, false, true);""")
    s = sub(s, """                    self.plan.borrowed_destination = true;
                    self.path(strip(&children(decl)[0])?, false, true)""", """                    self.plan.borrowed_destination = true;
                    let source = match strip(&children(decl)[0]) {
                        Some(source) => source,
                        None => {
                            let source = w47cp_bitcast(&children(decl)[0])?;
                            w47cp_mark(2);
                            source
                        }
                    };
                    self.path(source, false, true)""")
    s = sub(s, """        let mut reader = self.reader(&never_bound);
        reader.statement(node)?;
        Some(reader.plan)""", """        w47cp_reset();
        let mut reader = self.reader(&never_bound);
        reader.statement(node)?;
        if w47cp_would_admit() {
            return None;
        }
        Some(reader.plan)""")
    TAP.write_text(s)

    s = POS.read_text()
    s = sub(s, "use crate::typed_access_path::{Reader, StatementIndex, StatementPlan, is_pointer};",
            "use crate::typed_access_path::{Reader, StatementIndex, StatementPlan, is_pointer, w47cp_const, w47cp_mark, w47cp_reset, w47cp_would_admit};")
    s = sub(s, """        let value = strip_pointer_value(node)?;
        if !is_pointer(value) {
            return None;
        }""", """        let value = strip_pointer_value(node)?;
        if !is_pointer(value) {
            if w47cp_const(value) {
                w47cp_mark(1);
            } else {
                return None;
            }
        }""")
    s = sub(s, """        let mut reader = self.reader(&never_bound);
        reader.condition(node)?;
        Some(reader.plan)""", """        w47cp_reset();
        let mut reader = self.reader(&never_bound);
        reader.condition(node)?;
        if w47cp_would_admit() {
            return None;
        }
        Some(reader.plan)""")
    s = sub(s, """        let mut reader = self.reader(&never_bound);
        reader.statement(node)?;
        Some(reader.plan)""", """        w47cp_reset();
        let mut reader = self.reader(&never_bound);
        reader.statement(node)?;
        if w47cp_would_admit() {
            return None;
        }
        Some(reader.plan)""")
    s = sub(s, """        if pointer && !reader.roots_admit(&pointer_root) {
            return None;
        }
        Some(reader.plan)""", """        if pointer && !reader.roots_admit(&pointer_root) {
            return None;
        }
        if w47cp_would_admit() {
            return None;
        }
        Some(reader.plan)""")
    s = sub(s, """        let node = self.expression_node(texts, line)?;
        let mut reader = self.reader(&never_bound);
        let pointer = reader.value(node)?;""", """        w47cp_reset();
        let node = self.expression_node(texts, line)?;
        let mut reader = self.reader(&never_bound);
        let pointer = reader.value(node)?;""")
    s = sub(s, """        if pointer_valued && !reader.roots_admit(&never_bound) {
            return None;
        }""", """        if pointer_valued && !reader.roots_admit(&never_bound) {
            return None;
        }
        if w47cp_would_admit() {
            return None;
        }""")
    s = sub(s, """        let mut reader = self.reader(&never_bound);
        let pointer_valued = reader.value(actual)?;""", """        w47cp_reset();
        let mut reader = self.reader(&never_bound);
        let pointer_valued = reader.value(actual)?;""")
    POS.write_text(s)

    s = HOOK.read_text()
    s = sub(s, """        record_position(
            &self.typed_path_function,
            line,
            &texts,
            "condition",
            plan.as_ref(),
        );""", """        crate::typed_access_path::w47cp_census(&self.typed_path_function, line, &texts, "condition");
        record_position(
            &self.typed_path_function,
            line,
            &texts,
            "condition",
            plan.as_ref(),
        );""")
    s = sub(s, """        record_position(
            &self.typed_path_function,
            line,
            &statement_texts,
            "condition-assignment",
            plan.as_ref(),
        );""", """        crate::typed_access_path::w47cp_census(&self.typed_path_function, line, &statement_texts, "condition-assignment");
        record_position(
            &self.typed_path_function,
            line,
            &statement_texts,
            "condition-assignment",
            plan.as_ref(),
        );""")
    s = sub(s, """        record_position(
            &self.typed_path_function,
            line,
            &texts,
            "return",
            plan.as_ref(),
        );""", """        crate::typed_access_path::w47cp_census(&self.typed_path_function, line, &texts, "return");
        record_position(
            &self.typed_path_function,
            line,
            &texts,
            "return",
            plan.as_ref(),
        );""")
    s = sub(s, """        record_position(
            &self.typed_path_function,
            line,
            argument,
            "call-argument",
            plan.as_ref().map(|plan| &plan.plan),
        );""", """        crate::typed_access_path::w47cp_census(&self.typed_path_function, line, argument, &format!("call-argument:{callee}:{index}"));
        record_position(
            &self.typed_path_function,
            line,
            argument,
            "call-argument",
            plan.as_ref().map(|plan| &plan.plan),
        );""")
    s = sub(s, """        let plan = self
            .typed_positions_index()?
            .condition(""", """        crate::typed_access_path::w47cp_reset();
        let plan = self
            .typed_positions_index()?
            .condition(""")
    s = sub(s, """        let plan = self
            .typed_positions_index()?
            .assignment(""", """        crate::typed_access_path::w47cp_reset();
        let plan = self
            .typed_positions_index()?
            .assignment(""")
    s = sub(s, """        let plan = self.typed_positions_index()?.returned(""", """        crate::typed_access_path::w47cp_reset();
        let plan = self.typed_positions_index()?.returned(""")
    s = sub(s, """        let plan =
            self.typed_positions_index()?
                .argument(""", """        crate::typed_access_path::w47cp_reset();
        let plan =
            self.typed_positions_index()?
                .argument(""")
    HOOK.write_text(s)

    s = LEG.read_text()
    s = sub(s, """    fn typed_path_statement(&self, tokens: &[Token]) -> Option<LegacyInstruction> {
        let plan = self.typed_path_plan(tokens)?;""", """    fn typed_path_statement(&self, tokens: &[Token]) -> Option<LegacyInstruction> {
        crate::typed_access_path::w47cp_reset();
        let plan = self.typed_path_plan(tokens);
        {
            let texts = tokens.iter().map(|token| token.text.as_str()).collect::<Vec<_>>();
            crate::typed_access_path::w47cp_census(&self.typed_path_function, tokens.first()?.line, &texts, "statement");
        }
        let plan = plan?;""")
    LEG.write_text(s)
    print("applied")


if sys.argv[2] == "apply":
    apply()
