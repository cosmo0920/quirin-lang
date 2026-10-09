//! Card **CG10-INDIRECT-ACCESS-SOLE-RESIDUALS** round 2 (gateway rules 14a and
//! 14b): the Clang readings of two statement shapes the lexical gate refused
//! and no older route read. The gateway half (the call's lowering, the hook
//! and the parity rows) is `legacy_residual_access.rs`.
//!
//! ## Rule 14a: one contracted scalar call inside a typed scalar statement
//!
//! `T x = E;`, `x = E;` or `P = E;` (`P` a typed scalar place) whose value `E`
//! holds **exactly one** call, as in `int64_t d = rd_atomic64_get(&rk->c) -
//! rk->h.c;` or `rkcg->ts = rd_clock();`. The statement is read by the typed
//! path reader exactly as a call-free scalar statement (every per-hop
//! obligation of each place it reads or stores), with that one call read as
//! an opaque scalar operand ([`crate::typed_access_path::Reader::opaque_call`]);
//! the gateway lowers the call itself through rule W44 and admits the
//! statement only when the call's contract has a `None` result and no
//! custody-taking position, so the call changes no binding the reads depend on
//! and the order C leaves unspecified between the call and the other operands
//! (C11 6.5p2, 6.5.2.2p10) cannot change any decision.
//!
//! ## Rule 14b: an interior pointer into an untracked object
//!
//! A pointer local (declared or assigned by name) given `&PATH` (`&s->iov[i]`,
//! `&msg->u.fetch.rkm`) or `PATH ± n` (`buf->data + buf->off`), `PATH` a
//! member or element path and `n` a scalar value, with **every root of
//! `PATH` never bound**. The value is a pointer into the object `PATH`
//! reaches (C11 6.5.6p8, 6.5.3.2p3), which the never-bound premise puts
//! outside the tracked graph, so the local becomes exactly the target-free
//! borrowed alias a pointer-valued path read (`p = s->iov;`) binds, with the
//! same per-hop obligations; a later release or store through it is decided
//! by its own rules. The address's place type must not be a pointer, an array
//! or a function (rule 10n(1)'s test, read here as a local alias, not handed
//! to a callee).
//!
//! Premises stated, not proved: faithful Clang extraction of the statement's
//! types and node shapes (trust boundary 28) and the never-bound closure the
//! statement route already reads (rule 10o). The Lean model is
//! `QCore.ResidualAccessReadings`.

use serde_json::Value;

use crate::typed_access_path::{Reader, StatementIndex, StatementPlan, is_pointer, strip};
use crate::typed_c_ast::{children, kind};

/// Every `CallExpr` under `node`, outermost first.
fn calls<'a>(node: &'a Value, out: &mut Vec<&'a Value>) {
    if kind(node) == "CallExpr" {
        out.push(node);
    }
    for child in children(node) {
        calls(child, out);
    }
}

/// The callee's name when it is a plain function designator.
pub(crate) fn direct_callee(call: &Value) -> Option<&str> {
    let mut target = children(call).first()?;
    while kind(target) == "ImplicitCastExpr" {
        if target["castKind"].as_str()? != "FunctionToPointerDecay" {
            return None;
        }
        target = children(target).first()?;
    }
    (kind(target) == "DeclRefExpr"
        && target["referencedDecl"]["kind"].as_str() == Some("FunctionDecl"))
    .then(|| target["referencedDecl"]["name"].as_str())
    .flatten()
}

/// The statement shapes rule 14a reads: a single scalar declaration with an
/// initializer, or a plain `=` whose destination is not a pointer.
fn scalar_statement_shape(node: &Value, reader: &Reader<'_>) -> bool {
    match kind(node) {
        "DeclStmt" => match children(node) {
            [decl] => {
                kind(decl) == "VarDecl"
                    && !is_pointer(decl)
                    && reader.scalar(decl)
                    && children(decl).len() == 1
            }
            _ => false,
        },
        "BinaryOperator" => {
            node["opcode"].as_str() == Some("=")
                && children(node).len() == 2
                && !is_pointer(&children(node)[0])
        }
        _ => false,
    }
}

/// Rule 14a's reading.
pub(crate) struct ScalarCallReading<'a> {
    /// The one call, which the caller lowers through rule W44.
    pub(crate) call: &'a Value,
    /// Its callee.
    pub(crate) callee: String,
    /// The statement's reads and store, the call read as an opaque scalar.
    pub(crate) plan: StatementPlan,
}

/// Rule 14b's value forms.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InteriorForm {
    /// `&PATH`.
    Address,
    /// `PATH + n` or `PATH - n`.
    Offset,
}

impl InteriorForm {
    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::Address => "address",
            Self::Offset => "offset",
        }
    }
}

/// Every root a plan reads, for the never-bound premise.
fn roots(plan: &StatementPlan) -> Vec<&str> {
    let mut names = plan
        .scalar
        .bases
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    for chain in &plan.chains {
        if !names.contains(&chain.root.as_str()) {
            names.push(&chain.root);
        }
    }
    names
}

/// A pointer value through parentheses and value-preserving implicit casts.
fn strip_value(mut node: &Value) -> Option<&Value> {
    loop {
        match kind(node) {
            "ParenExpr" => {}
            "ImplicitCastExpr"
                if matches!(node["castKind"].as_str()?, "LValueToRValue" | "NoOp") => {}
            _ => return Some(node),
        }
        node = children(node).first()?;
    }
}

impl StatementIndex {
    /// Rule 14a: `node`'s one call and the statement read with it opaque.
    pub(crate) fn scalar_call_statement<'a>(
        &'a self,
        node: &'a Value,
        never_bound: &'a dyn Fn(&str) -> bool,
        decisions: Option<&'a crate::consultation_decisions::Decisions>,
    ) -> Option<ScalarCallReading<'a>> {
        let mut found = Vec::new();
        calls(node, &mut found);
        let [call] = found.as_slice() else {
            return None;
        };
        let callee = direct_callee(call)?.to_owned();
        let mut reader = self.reader(never_bound).recording(decisions);
        if !scalar_statement_shape(node, &reader) {
            return None;
        }
        reader.opaque_call = Some(call);
        reader.statement(node)?;
        let plan = reader.plan;
        if plan.pointer_store.is_some() || plan.borrowed_destination || plan.readback.is_some() {
            return None;
        }
        Some(ScalarCallReading { call, callee, plan })
    }

    /// Rule 14b: a pointer local given an interior pointer of a path whose
    /// every root is never bound, read as the borrowed alias a pointer read
    /// binds.
    pub(crate) fn interior_pointer_statement(
        &self,
        node: &Value,
        never_bound: &dyn Fn(&str) -> bool,
        decisions: Option<&crate::consultation_decisions::Decisions>,
    ) -> Option<(StatementPlan, InteriorForm)> {
        let mut found = Vec::new();
        calls(node, &mut found);
        if !found.is_empty() {
            return None;
        }
        let (name, value) = match kind(node) {
            "DeclStmt" => {
                let [decl] = children(node) else {
                    return None;
                };
                if kind(decl) != "VarDecl" || !is_pointer(decl) {
                    return None;
                }
                let [value] = children(decl) else {
                    return None;
                };
                (decl["name"].as_str()?, value)
            }
            "BinaryOperator" if node["opcode"].as_str() == Some("=") => {
                let [destination, value] = children(node) else {
                    return None;
                };
                let destination = strip(destination)?;
                if kind(destination) != "DeclRefExpr"
                    || !is_pointer(destination)
                    || !matches!(
                        destination["referencedDecl"]["kind"].as_str(),
                        Some("VarDecl") | Some("ParmVarDecl")
                    )
                {
                    return None;
                }
                (destination["referencedDecl"]["name"].as_str()?, value)
            }
            _ => return None,
        };
        let value = strip_value(value)?;
        let mut reader = self.reader(never_bound).recording(decisions);
        let form = match kind(value) {
            "UnaryOperator" if value["opcode"].as_str() == Some("&") => {
                reader.address(value, false)?;
                InteriorForm::Address
            }
            "BinaryOperator"
                if matches!(value["opcode"].as_str(), Some("+") | Some("-"))
                    && is_pointer(value) =>
            {
                let [pointer, offset] = children(value) else {
                    return None;
                };
                if !is_pointer(pointer) || !reader.scalar(offset) {
                    return None;
                }
                let pointer = strip(pointer)?;
                match kind(pointer) {
                    "MemberExpr" | "ArraySubscriptExpr" => {}
                    "UnaryOperator" if pointer["opcode"].as_str() == Some("*") => {}
                    _ => return None,
                }
                reader.path(pointer, false, true)?;
                reader.expression(offset)?;
                InteriorForm::Offset
            }
            _ => return None,
        };
        let mut plan = reader.plan;
        if plan.accesses == 0
            || plan.pointer_store.is_some()
            || !roots(&plan).iter().all(|root| never_bound(root))
        {
            return None;
        }
        plan.scalar.destination = Some(name.to_owned());
        plan.borrowed_destination = true;
        Some((plan, form))
    }
}

/// The shape table the Lean model mirrors: (statement shape, value form,
/// every root never bound, admitted by rule 14b).
#[cfg(test)]
pub(crate) const INTERIOR_VECTORS: &[(&str, &str, bool, bool)] = &[
    ("pointer-local", "address", true, true),
    ("pointer-local", "offset", true, true),
    ("pointer-local", "address", false, false),
    ("pointer-local", "offset", false, false),
    ("pointer-local", "call", true, false),
    ("pointer-local", "cast", true, false),
    ("member-place", "address", true, false),
    ("scalar-local", "address", true, false),
];

/// Rule 14b's decision on the table's terms.
#[cfg(test)]
pub(crate) fn interior_admitted(shape: &str, form: &str, roots_never_bound: bool) -> bool {
    shape == "pointer-local" && matches!(form, "address" | "offset") && roots_never_bound
}

/// Rule 14a's decision on a call's contract: (calls in the value, result is
/// `None`, every declared position `Borrowed`, admitted).
#[cfg(test)]
pub(crate) const SCALAR_CALL_VECTORS: &[(usize, bool, bool, bool)] = &[
    (1, true, true, true),
    (0, true, true, false),
    (2, true, true, false),
    (1, false, true, false),
    (1, true, false, false),
];

/// Rule 14a's decision on the table's terms.
pub(crate) fn scalar_call_admitted(calls: usize, none_result: bool, all_borrowed: bool) -> bool {
    calls == 1 && none_result && all_borrowed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::legacy_c::{CCompilerDialect, tokenize};
    use crate::typed_access_path::enter_source;

    #[test]
    fn the_decision_vectors_are_the_rust_decisions() {
        for (shape, form, never_bound, admitted) in INTERIOR_VECTORS {
            assert_eq!(interior_admitted(shape, form, *never_bound), *admitted);
        }
        for (calls, none_result, borrowed, admitted) in SCALAR_CALL_VECTORS {
            assert_eq!(
                scalar_call_admitted(*calls, *none_result, *borrowed),
                *admitted
            );
        }
    }

    fn with_node<T>(
        source: &str,
        statement: &str,
        body: impl FnOnce(&StatementIndex, &Value) -> T,
    ) -> T {
        let _scope = enter_source(source, CCompilerDialect::C11, None);
        let index = StatementIndex::read("f").expect("typed function");
        let wanted = tokenize(statement);
        let texts = wanted
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        let node = index
            .statement_node(&texts, 1, &|| None)
            .expect("statement node");
        body(&index, node)
    }

    const S: &str = "struct V { char *base; long len; }; struct S { struct V *v; long n; char *buf; long off; struct V one; long *counts; }; long g(long *p); char *h(char *p);";

    #[test]
    fn rule_14a_reads_one_scalar_call_and_the_rest() {
        let source = format!("{S} long f(struct S *s) {{ long d = g(&s->n) - s->off; return d; }}");
        with_node(&source, "long d = g(&s->n) - s->off", |index, node| {
            let never_bound = |_: &str| true;
            let reading = index
                .scalar_call_statement(node, &never_bound, None)
                .expect("one scalar call");
            assert_eq!(reading.callee, "g");
            assert_eq!(reading.plan.scalar.bases, ["s"]);
            assert_eq!(reading.plan.scalar.destination.as_deref(), Some("d"));
        });
        let source = format!("{S} void f(struct S *s) {{ s->n = g(&s->n) + g(&s->off); }}");
        with_node(&source, "s->n = g(&s->n) + g(&s->off)", |index, node| {
            let never_bound = |_: &str| true;
            assert!(
                index
                    .scalar_call_statement(node, &never_bound, None)
                    .is_none()
            );
        });
        let source = format!("{S} void f(struct S *s) {{ char *p = h(s->buf); (void)p; }}");
        with_node(&source, "char *p = h(s->buf)", |index, node| {
            let never_bound = |_: &str| true;
            assert!(
                index
                    .scalar_call_statement(node, &never_bound, None)
                    .is_none()
            );
        });
    }

    #[test]
    fn rule_14b_reads_an_interior_pointer_of_a_never_bound_root() {
        let source = format!(
            "{S} void f(struct S *s) {{ struct V *v = &s->v[s->n]; char *p = s->buf + s->off; (void)v; (void)p; }}"
        );
        with_node(&source, "struct V *v = &s->v[s->n]", |index, node| {
            let (plan, form) = index
                .interior_pointer_statement(node, &|_| true, None)
                .expect("an interior address");
            assert_eq!(form, InteriorForm::Address);
            assert!(plan.borrowed_destination);
            assert_eq!(plan.scalar.destination.as_deref(), Some("v"));
            assert!(
                index
                    .interior_pointer_statement(node, &|_| false, None)
                    .is_none()
            );
        });
        with_node(&source, "char *p = s->buf + s->off", |index, node| {
            let (_, form) = index
                .interior_pointer_statement(node, &|_| true, None)
                .expect("an interior offset");
            assert_eq!(form, InteriorForm::Offset);
        });
        let source = format!(
            "{S} void f(struct S *s) {{ long *c = &s->counts[0]; struct V *o = &s->one; char **q = &s->buf; (void)c; (void)o; (void)q; }}"
        );
        with_node(&source, "struct V *o = &s->one", |index, node| {
            assert!(
                index
                    .interior_pointer_statement(node, &|_| true, None)
                    .is_some()
            );
        });
        // The address of a pointer member is refused (rule 10n(1)'s type test).
        with_node(&source, "char **q = &s->buf", |index, node| {
            assert!(
                index
                    .interior_pointer_statement(node, &|_| true, None)
                    .is_none()
            );
        });
    }
}
