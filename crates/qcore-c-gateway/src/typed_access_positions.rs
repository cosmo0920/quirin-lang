//! Card **W47-ACCESS-PATH-POSITIONS** (plan P3): the Clang-typed access-path
//! reader of [`crate::typed_access_path`] in three more positions — a
//! controlling-expression operand, a `return` expression and a call actual.
//!
//! The per-hop reader is P2's, unchanged, with all of its refusals (unions,
//! bitfields, casts within a path, volatile/atomic, pointer arithmetic, a
//! tracked mixed embedded/indexed path, pointer-valued stores, calls, and
//! missing or ambiguous AST facts). This module adds only the matching of a
//! sub-expression to its unique Clang node and the value forms each position
//! reads:
//!
//! * **Condition operand** ([`StatementIndex::condition`]): a scalar value,
//!   the truthiness of a pointer-valued path read, or a comparison of
//!   pointer operands (a path read, a plain pointer name, a null constant).
//!   No connective is read here: `&&`, `||` and `?:` stay with the condition
//!   desugaring, which applies this reader per operand on the edges where C
//!   evaluates it. The read binds nothing and refines nothing.
//! * **Assignment operand** ([`StatementIndex::assignment`]): `NAME = PATH`
//!   read exactly as P2 reads the statement (a borrowed target-free alias for
//!   a pointer, a forgotten scalar otherwise).
//! * **Return** ([`StatementIndex::returned`]): one scalar value, or one
//!   pointer-valued path read whose every root the body never binds (the
//!   loaded pointer is then outside the tracked graph) and which the
//!   caller's `pointer_root` admits (constraint 5b's `may_bind` premise). An array member decaying to its address is an interior
//!   address — a derived result (rule 5j) — and refuses.
//! * **Call actual** ([`StatementIndex::argument`]): the actual at `index` of
//!   the unique call of `callee` in the statement, read as a scalar value or,
//!   with every root never bound, as a pointer-valued path read.
//!
//! A node is matched by its internal line and its exact token text and must
//! be unique; implicit-cast wrappers share their operand's range and are not
//! candidates of their own.

use serde_json::Value;

use crate::typed_access_path::{Reader, StatementIndex, StatementPlan, is_pointer};
use crate::typed_c_ast::{children, kind};

/// What a call actual turned out to be.
#[derive(Clone, Debug)]
pub(crate) struct ArgumentPlan {
    pub(crate) plan: StatementPlan,
    /// The value handed to the callee is a pointer read from memory.
    pub(crate) pointer_valued: bool,
}

/// Every node kind that is an expression the matcher may name. Statement
/// nodes, declarations and wrappers sharing their operand's range are not.
fn candidate(node: &Value) -> bool {
    let kind = kind(node);
    !matches!(
        kind,
        "ImplicitCastExpr" | "ConstantExpr" | "FullExpr" | "ExprWithCleanups"
    ) && (kind.ends_with("Expr") || kind.ends_with("Operator") || kind.ends_with("Literal"))
}

fn expressions<'a>(node: &'a Value, out: &mut Vec<&'a Value>) {
    if candidate(node) {
        out.push(node);
    }
    for child in children(node) {
        expressions(child, out);
    }
}

/// Every `CallExpr` under `node` whose callee is the plain name `callee`.
fn calls<'a>(node: &'a Value, callee: &str, out: &mut Vec<&'a Value>) {
    if kind(node) == "CallExpr"
        && let Some(target) = children(node).first()
        && let Some(target) = strip_callee(target)
        && kind(target) == "DeclRefExpr"
        && target["referencedDecl"]["name"].as_str() == Some(callee)
        && target["referencedDecl"]["kind"].as_str() == Some("FunctionDecl")
    {
        out.push(node);
    }
    for child in children(node) {
        calls(child, callee, out);
    }
}

fn strip_callee(mut node: &Value) -> Option<&Value> {
    while kind(node) == "ImplicitCastExpr" {
        if node["castKind"].as_str()? != "FunctionToPointerDecay" {
            return None;
        }
        node = children(node).first()?;
    }
    Some(node)
}

/// A null pointer constant through parentheses and null/bit/no-op casts.
fn null_constant(mut node: &Value) -> bool {
    loop {
        match kind(node) {
            "ParenExpr" => {}
            "ImplicitCastExpr" | "CStyleCastExpr"
                if matches!(
                    node["castKind"].as_str(),
                    Some("NullToPointer") | Some("BitCast") | Some("NoOp")
                ) => {}
            "IntegerLiteral" => return node["value"].as_str() == Some("0"),
            "GNUNullExpr" => return true,
            _ => return false,
        }
        let Some(inner) = children(node).first() else {
            return false;
        };
        node = inner;
    }
}

/// A pointer value through parentheses and value-preserving implicit casts.
/// `ArrayToPointerDecay` is excluded: it computes an address, it reads none.
fn strip_pointer_value(mut node: &Value) -> Option<&Value> {
    loop {
        match kind(node) {
            "ParenExpr" => {}
            "ImplicitCastExpr"
                if matches!(
                    node["castKind"].as_str()?,
                    "LValueToRValue" | "NoOp" | "BitCast"
                ) => {}
            _ => return Some(node),
        }
        node = children(node).first()?;
    }
}

/// The root names of every path the plan read, for the never-bound check a
/// pointer-valued read requires.
fn roots(plan: &StatementPlan) -> Vec<String> {
    let mut names = plan.scalar.bases.clone();
    for chain in &plan.chains {
        if !names.contains(&chain.root) {
            names.push(chain.root.clone());
        }
    }
    names
}

impl Reader<'_> {
    /// One condition operand: a scalar value, the truthiness of a pointer
    /// read, or a comparison of pointer operands.
    fn condition(&mut self, node: &Value) -> Option<()> {
        match kind(node) {
            "ParenExpr" => self.condition(children(node).first()?),
            "UnaryOperator" if node["opcode"].as_str() == Some("!") => {
                self.condition(children(node).first()?)
            }
            "BinaryOperator"
                if matches!(
                    node["opcode"].as_str(),
                    Some("==") | Some("!=") | Some("<") | Some(">") | Some("<=") | Some(">=")
                ) =>
            {
                let pair = children(node);
                if pair.len() != 2 {
                    return None;
                }
                if self.scalar(&pair[0]) && self.scalar(&pair[1]) {
                    return self.expression(node);
                }
                self.pointer_operand(&pair[0])?;
                self.pointer_operand(&pair[1])
            }
            _ if self.scalar(node) => self.expression(node),
            _ => self.pointer_operand(node),
        }
    }

    /// A pointer operand of a condition: a null constant, a plain pointer
    /// name (its value, no access), or a pointer-valued path read.
    fn pointer_operand(&mut self, node: &Value) -> Option<()> {
        if null_constant(node) {
            return Some(());
        }
        let value = strip_pointer_value(node)?;
        match kind(value) {
            "DeclRefExpr"
                if is_pointer(value)
                    && matches!(
                        value["referencedDecl"]["kind"].as_str(),
                        Some("VarDecl") | Some("ParmVarDecl")
                    ) =>
            {
                Some(())
            }
            "MemberExpr" | "ArraySubscriptExpr" => self.path(value, false, true),
            "UnaryOperator" if value["opcode"].as_str() == Some("*") => {
                self.path(value, false, true)
            }
            _ => None,
        }
    }

    /// A whole value in a `return` or at a call actual: a scalar value, or a
    /// pointer read from memory (`Some(true)`), whose roots the caller checks.
    fn value(&mut self, node: &Value) -> Option<bool> {
        if self.scalar(node) {
            self.expression(node)?;
            return Some(false);
        }
        let value = strip_pointer_value(node)?;
        if !is_pointer(value) {
            return None;
        }
        match kind(value) {
            "MemberExpr" | "ArraySubscriptExpr" => self.path(value, false, true)?,
            "UnaryOperator" if value["opcode"].as_str() == Some("*") => {
                self.path(value, false, true)?
            }
            _ => return None,
        }
        Some(true)
    }

    /// Every root of the plan satisfies the pointer-return premise.
    fn roots_admit(&self, pointer_root: &dyn Fn(&str) -> bool) -> bool {
        roots(&self.plan).iter().all(|root| pointer_root(root))
    }
}

impl StatementIndex {
    /// The unique expression node on `line` whose tokens are exactly `texts`.
    fn expression_node(&self, texts: &[&str], line: usize) -> Option<&Value> {
        let body = children(&self.function)
            .iter()
            .find(|node| kind(node) == "CompoundStmt")?;
        let mut all = Vec::new();
        expressions(body, &mut all);
        let mut matched = all.into_iter().filter(|node| {
            self.node_line(node) == Some(line)
                && self.node_tokens(node).is_some_and(|found| found == texts)
        });
        let node = matched.next()?;
        matched.next().is_none().then_some(node)
    }

    /// A controlling-expression operand read as a typed condition.
    pub(crate) fn condition(
        &self,
        texts: &[&str],
        line: usize,
        never_bound: impl Fn(&str) -> bool,
    ) -> Option<StatementPlan> {
        let node = self.expression_node(texts, line)?;
        let mut reader = self.reader(&never_bound);
        reader.condition(node)?;
        Some(reader.plan)
    }

    /// `NAME = PATH` inside a controlling expression, read as P2's statement.
    pub(crate) fn assignment(
        &self,
        texts: &[&str],
        line: usize,
        never_bound: impl Fn(&str) -> bool,
    ) -> Option<StatementPlan> {
        let node = self.expression_node(texts, line)?;
        if kind(node) != "BinaryOperator" || node["opcode"].as_str() != Some("=") {
            return None;
        }
        let mut reader = self.reader(&never_bound);
        reader.statement(node)?;
        Some(reader.plan)
    }

    /// A `return` expression: one scalar value, or one pointer read whose
    /// every root satisfies `pointer_root` (the caller passes: never bound by
    /// the custody closure, and not `may_bind`, constraint 5b's premise).
    pub(crate) fn returned(
        &self,
        texts: &[&str],
        line: usize,
        never_bound: impl Fn(&str) -> bool,
        pointer_root: impl Fn(&str) -> bool,
    ) -> Option<StatementPlan> {
        let node = self.expression_node(texts, line)?;
        let mut reader = self.reader(&never_bound);
        let pointer = reader.value(node)?;
        if pointer && !reader.roots_admit(&pointer_root) {
            return None;
        }
        Some(reader.plan)
    }

    /// The actual at `index` of the one call of `callee` whose actual there is
    /// exactly `texts` on `line`.
    pub(crate) fn argument(
        &self,
        callee: &str,
        index: usize,
        arity: usize,
        texts: &[&str],
        line: usize,
        never_bound: impl Fn(&str) -> bool,
        pointer_root: impl Fn(&str) -> bool,
    ) -> Option<ArgumentPlan> {
        let body = children(&self.function)
            .iter()
            .find(|node| kind(node) == "CompoundStmt")?;
        let mut found = Vec::new();
        calls(body, callee, &mut found);
        let mut matched = found.into_iter().filter_map(|call| {
            let actuals = children(call).get(1..)?;
            if actuals.len() != arity {
                return None;
            }
            let actual = actuals.get(index)?;
            (self.node_line(actual) == Some(line)
                && self.node_tokens(actual).is_some_and(|found| found == texts))
            .then_some(actual)
        });
        let actual = matched.next()?;
        if matched.next().is_some() {
            return None;
        }
        let mut reader = self.reader(&never_bound);
        let pointer_valued = reader.value(actual)?;
        if pointer_valued && !reader.roots_admit(&pointer_root) {
            return None;
        }
        Some(ArgumentPlan {
            plan: reader.plan,
            pointer_valued,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::legacy_c::{CCompilerDialect, tokenize};
    use crate::typed_access_path::enter_source;

    fn texts(text: &str) -> Vec<String> {
        tokenize(text).into_iter().map(|token| token.text).collect()
    }

    fn with_index<T>(source: &str, body: impl FnOnce(&StatementIndex) -> T) -> T {
        let _scope = enter_source(source, CCompilerDialect::C11, None);
        let index = StatementIndex::read("f").expect("typed function");
        body(&index)
    }

    fn condition(source: &str, operand: &str) -> Option<StatementPlan> {
        with_index(source, |index| {
            let wanted = texts(operand);
            let wanted = wanted.iter().map(String::as_str).collect::<Vec<_>>();
            index.condition(&wanted, 1, |_| true)
        })
    }

    const S: &str =
        "struct S { struct S *next; int value; union { int a; float b; } u; unsigned bits : 3; }; ";

    #[test]
    fn scalar_and_pointer_condition_operands_read_their_bases() {
        let source = format!(
            "{S}int f(struct S *p) {{ if (p->value > 1) return 1; if (p->next == ((void *)0)) return 2; if (!p->next) return 3; return 0; }}"
        );
        let plan = condition(&source, "p->value > 1").expect("scalar comparison");
        assert_eq!(plan.scalar.bases, ["p"]);
        assert!(condition(&source, "p->next == ((void *)0)").is_some());
        assert!(condition(&source, "!p->next").is_some());
    }

    #[test]
    fn a_connective_is_never_one_operand() {
        let source = format!("{S}int f(struct S *p) {{ if (p && p->value) return 1; return 0; }}");
        assert!(condition(&source, "p && p->value").is_none());
        assert!(condition(&source, "p->value").is_some());
    }

    #[test]
    fn union_bitfield_cast_and_call_operands_refuse() {
        let source = format!(
            "{S}int g(int); int f(struct S *p) {{ if (p->u.a) return 1; if (p->bits) return 2; if (*(int *)p->next) return 3; if (g(p->value)) return 4; return 0; }}"
        );
        for operand in ["p->u.a", "p->bits", "*(int *)p->next", "g(p->value)"] {
            assert!(condition(&source, operand).is_none(), "{operand}");
        }
        let hidden =
            "typedef union U { int x; int y; } U; int f(U *u) { if (u->x) return 1; return 0; }";
        assert!(condition(hidden, "u->x").is_none());
    }

    #[test]
    fn an_assignment_operand_binds_a_borrowed_alias() {
        let source = format!(
            "{S}int f(struct S *p) {{ struct S *r; if (!(r = p->next)) return 1; return r->value; }}"
        );
        with_index(&source, |index| {
            let wanted = texts("r = p->next");
            let wanted = wanted.iter().map(String::as_str).collect::<Vec<_>>();
            let plan = index
                .assignment(&wanted, 1, |_| true)
                .expect("typed assignment");
            assert!(plan.borrowed_destination);
            assert_eq!(plan.scalar.destination.as_deref(), Some("r"));
        });
    }

    #[test]
    fn a_returned_pointer_needs_never_bound_roots_and_an_array_member_refuses() {
        let source = "struct S { struct S *next; char name[8]; int value; }; int f(struct S *p) { return p->next ? 1 : 0; }";
        with_index(source, |index| {
            let ternary = texts("p->next ? 1 : 0");
            let ternary = ternary.iter().map(String::as_str).collect::<Vec<_>>();
            assert!(index.returned(&ternary, 1, |_| true, |_| true).is_none());
        });
        let source = "struct S { struct S *next; char name[8]; int value; }; struct S *f(struct S *p) { return p->next; }";
        with_index(source, |index| {
            let wanted = texts("p->next");
            let wanted = wanted.iter().map(String::as_str).collect::<Vec<_>>();
            assert!(index.returned(&wanted, 1, |_| true, |_| true).is_some());
            assert!(
                index
                    .returned(&wanted, 1, |_| true, |name| name != "p")
                    .is_none()
            );
        });
        let source = "struct S { char name[8]; }; char *f(struct S *p) { return p->name; }";
        with_index(source, |index| {
            let wanted = texts("p->name");
            let wanted = wanted.iter().map(String::as_str).collect::<Vec<_>>();
            assert!(index.returned(&wanted, 1, |_| true, |_| true).is_none());
        });
    }

    #[test]
    fn a_call_actual_is_matched_by_callee_and_index() {
        let source = "struct S { int value; struct S *next; }; void g(int, struct S *); int f(struct S *p) { g(p->value, p->next); return 0; }";
        with_index(source, |index| {
            let scalar = texts("p->value");
            let scalar = scalar.iter().map(String::as_str).collect::<Vec<_>>();
            let read = index
                .argument("g", 0, 2, &scalar, 1, |_| true, |_| true)
                .expect("scalar actual");
            assert!(!read.pointer_valued);
            let pointer = texts("p->next");
            let pointer = pointer.iter().map(String::as_str).collect::<Vec<_>>();
            let open = |_: &str| true;
            assert!(
                index
                    .argument("g", 1, 2, &pointer, 1, open, open)
                    .unwrap()
                    .pointer_valued
            );
            assert!(
                index
                    .argument("g", 1, 2, &pointer, 1, open, |name| name != "p")
                    .is_none()
            );
            assert!(index.argument("g", 0, 2, &pointer, 1, open, open).is_none());
            assert!(index.argument("h", 0, 2, &scalar, 1, open, open).is_none());
        });
    }
}
