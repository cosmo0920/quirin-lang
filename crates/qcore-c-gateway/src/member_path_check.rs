//! Card **OP-CALLBACK-MEMBER-PATH-RELEASE** (gateway rule 30; contract spec
//! §C04.19.6): the body check of a declaration's member-path states, in a
//! unit that defines the declared function.
//!
//! Over the Clang body:
//!
//! * the stated formal's pointee (through the unit's typedefs) is `struct
//!   TAG` for every stated tag; an unread pointee leaves an assumption;
//! * every store of a pointer value into a member path of a stated formal is
//!   a null pointer constant into a path stated `#released`, `#moved`, or a
//!   set of one of them; a store of a record holding a stated path, any other
//!   pointer store, and a store into or the address of a path stated `#stale`
//!   or `#kept` are **contradictions** (the declaration is withdrawn in that
//!   unit);
//! * a path stated `#released` / `#moved` is **validated** when a null store
//!   into it is a statement of the body's top-level block before every
//!   `return`, in a body with no `goto` or label; a set needs one null store;
//!   a `#stale` path is validated by the absence of stores.
//!
//! Anything the body leaves to its callees (a static helper that clears the
//! member) is neither: the clause stays a declared assumption (rules
//! 13i–13k). The check establishes the member's reading after the call; that
//! the object it held was released or handed on stays a declared assumption,
//! as for the arms (rule 12o).

use qcore_schemas::member_path::{Fate, MemberPathState};
use serde_json::Value;

use crate::typed_c_ast::{children, kind};

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

fn null_constant(node: &Value) -> bool {
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

/// The member path of `node` below the parameter whose declaration id is
/// `parameter` (`P->a.b.c` is `a.b.c`), `None` for anything else.
fn path_of(node: &Value, parameter: &str) -> Option<String> {
    let mut segments = Vec::new();
    let mut node = strip(node);
    loop {
        if kind(node) != "MemberExpr" {
            return None;
        }
        segments.push(node["name"].as_str()?.to_owned());
        let base = strip(children(node).first()?);
        if node["isArrow"].as_bool() == Some(true) {
            if kind(base) == "DeclRefExpr"
                && base["referencedDecl"]["id"].as_str() == Some(parameter)
            {
                segments.reverse();
                return Some(segments.join("."));
            }
            return None;
        }
        node = base;
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

fn walk<'a>(node: &'a Value, out: &mut Vec<&'a Value>) {
    out.push(node);
    for child in children(node) {
        walk(child, out);
    }
}

fn contains(node: &Value, wanted: &[&str]) -> bool {
    let mut all = Vec::new();
    walk(node, &mut all);
    all.iter().any(|node| wanted.contains(&kind(node)))
}

/// Whether `node` (a top-level statement) is a null store into `path` below
/// `parameter`.
fn null_store(node: &Value, parameter: &str, path: &str) -> bool {
    let node = strip(node);
    if kind(node) != "BinaryOperator" || node["opcode"].as_str() != Some("=") {
        return false;
    }
    let [left, right] = children(node) else {
        return false;
    };
    path_of(left, parameter).as_deref() == Some(path) && null_constant(right)
}

/// Rule 30's body check: `Ok(true)` validated, `Ok(false)` a declared
/// assumption, `Err` a contradiction.
///
/// # Errors
/// A contradiction, naming it.
pub fn validates(
    function: &Value,
    states: &[MemberPathState],
    pointee: &dyn Fn(&Value) -> Option<String>,
) -> Result<bool, String> {
    let parameters = children(function)
        .iter()
        .filter(|child| kind(child) == "ParmVarDecl")
        .collect::<Vec<_>>();
    let body = children(function)
        .iter()
        .rev()
        .find(|child| kind(child) == "CompoundStmt")
        .ok_or("the body is not read")?;
    let mut nodes = Vec::new();
    walk(body, &mut nodes);
    let mut validated = !contains(body, &["GotoStmt", "LabelStmt", "IndirectGotoStmt"]);
    let formals = states
        .iter()
        .map(|state| state.parameter)
        .collect::<std::collections::BTreeSet<_>>();
    for formal in formals {
        let parameter = parameters
            .get(formal)
            .ok_or_else(|| format!("parameter {formal} is not read"))?;
        let id = parameter["id"]
            .as_str()
            .ok_or_else(|| format!("parameter {formal} has no id"))?;
        let stated = states
            .iter()
            .filter(|state| state.parameter == formal)
            .collect::<Vec<_>>();
        // The formal's pointee record, through the unit's typedefs (Clang's
        // JSON gives no desugared spelling for a pointer to a typedef): a
        // different record contradicts the stated tag; an unread one leaves
        // the clause a declared assumption.
        match pointee(parameter) {
            Some(record) => {
                if let Some(state) = stated.iter().find(|state| state.member.tag != record) {
                    return Err(format!(
                        "parameter {formal}'s pointee is `{record}`, not `{}`",
                        state.member.tag
                    ));
                }
            }
            None => validated = false,
        }
        let state_of = |path: &str| stated.iter().find(|state| state.member.path == path);
        let below_a_stated = |path: &str| {
            stated
                .iter()
                .any(|state| state.member.path.starts_with(&format!("{path}.")))
        };
        let mut stored = std::collections::BTreeSet::new();
        for node in &nodes {
            match kind(node) {
                "BinaryOperator" if node["opcode"].as_str() == Some("=") => {
                    let [left, right] = children(node) else {
                        continue;
                    };
                    let Some(path) = path_of(left, id) else {
                        continue;
                    };
                    if below_a_stated(&path) {
                        return Err(format!(
                            "a store of the record `{path}` holding a stated path"
                        ));
                    }
                    if !is_pointer(left) {
                        continue;
                    }
                    match state_of(&path) {
                        Some(state)
                            if matches!(state.state.fate, Fate::Released | Fate::Moved)
                                && null_constant(right) =>
                        {
                            stored.insert(path);
                        }
                        Some(state) => {
                            return Err(format!(
                                "a store into `{path}`, stated `{}`, that is not a null pointer \
                                 constant into a taken path",
                                state.state.spelling()
                            ));
                        }
                        None => {
                            return Err(format!("a pointer store into the unstated path `{path}`"));
                        }
                    }
                }
                "UnaryOperator" if node["opcode"].as_str() == Some("&") => {
                    let Some(inner) = children(node).first() else {
                        continue;
                    };
                    if let Some(path) = path_of(inner, id)
                        && let Some(state) = state_of(&path)
                        && matches!(state.state.fate, Fate::Stale | Fate::Kept)
                    {
                        return Err(format!(
                            "the address of `{path}`, stated `{}`, is taken",
                            state.state.spelling()
                        ));
                    }
                }
                _ => {}
            }
        }
        for state in &stated {
            let path = state.member.path.as_str();
            let holds = match (state.state.fate, state.state.maybe) {
                (Fate::Stale | Fate::Kept, _) => true,
                (Fate::Released | Fate::Moved, true) => stored.contains(path),
                (Fate::Released | Fate::Moved, false) => {
                    let mut before_return = true;
                    let mut found = false;
                    for statement in children(body) {
                        if before_return && null_store(statement, id, path) {
                            found = true;
                            break;
                        }
                        if contains(statement, &["ReturnStmt"]) {
                            before_return = false;
                        }
                    }
                    found
                }
            };
            validated &= holds;
        }
    }
    Ok(validated)
}

#[cfg(test)]
mod tests {
    use super::*;
    use qcore_schemas::callback::CallbackMember;
    use qcore_schemas::member_path::PathState;
    use serde_json::json;

    fn member(base: &str, path: &[&str]) -> Value {
        let mut node = json!({"kind": "DeclRefExpr", "referencedDecl": {"id": base}});
        for (index, segment) in path.iter().enumerate() {
            node = json!({
                "kind": "MemberExpr",
                "name": segment,
                "isArrow": index == 0,
                "type": {"qualType": "void *"},
                "inner": [node],
            });
        }
        node
    }

    fn store(path: &[&str], value: Value) -> Value {
        json!({"kind": "BinaryOperator", "opcode": "=", "inner": [member("p", path), value]})
    }

    fn null() -> Value {
        json!({"kind": "ImplicitCastExpr", "castKind": "NullToPointer",
               "inner": [{"kind": "IntegerLiteral", "value": "0"}]})
    }

    fn function(statements: Vec<Value>) -> Value {
        json!({"kind": "FunctionDecl", "inner": [
            {"kind": "ParmVarDecl", "id": "p", "type": {"qualType": "struct op_s *"}},
            {"kind": "CompoundStmt", "inner": statements},
        ]})
    }

    /// The check with the pointee read from the spelled `struct TAG *`.
    fn validates_spelled(function: &Value, states: &[MemberPathState]) -> Result<bool, String> {
        validates(function, states, &|parameter| {
            parameter["type"]["qualType"]
                .as_str()?
                .strip_prefix("struct ")?
                .split_whitespace()
                .next()
                .map(str::to_owned)
        })
    }

    fn state(path: &str, fate: Fate, maybe: bool) -> MemberPathState {
        MemberPathState {
            parameter: 0,
            member: CallbackMember {
                tag: "op_s".into(),
                path: path.into(),
            },
            state: PathState { fate, maybe },
            guard: None,
        }
    }

    #[test]
    fn a_top_level_null_store_before_every_return_validates() {
        let body = function(vec![
            store(&["u", "a"], null()),
            json!({"kind": "ReturnStmt"}),
        ]);
        assert_eq!(
            validates_spelled(&body, &[state("u.a", Fate::Moved, false)]),
            Ok(true)
        );
    }

    #[test]
    fn a_store_after_a_return_is_an_assumption_and_a_set_needs_one_store() {
        let body = function(vec![
            json!({"kind": "IfStmt", "inner": [{"kind": "ReturnStmt"}]}),
            store(&["u", "a"], null()),
        ]);
        assert_eq!(
            validates_spelled(&body, &[state("u.a", Fate::Released, false)]),
            Ok(false)
        );
        assert_eq!(
            validates_spelled(&body, &[state("u.a", Fate::Released, true)]),
            Ok(true)
        );
        assert_eq!(
            validates_spelled(&function(vec![]), &[state("u.a", Fate::Moved, true)]),
            Ok(false)
        );
    }

    #[test]
    fn contradictions_withdraw() {
        let other = function(vec![store(&["u", "b"], null())]);
        assert!(validates_spelled(&other, &[state("u.a", Fate::Moved, false)]).is_err());
        let value = function(vec![store(
            &["u", "a"],
            json!({"kind": "DeclRefExpr", "referencedDecl": {"id": "q"}}),
        )]);
        assert!(validates_spelled(&value, &[state("u.a", Fate::Moved, false)]).is_err());
        let stale = function(vec![store(&["u", "a"], null())]);
        assert!(validates_spelled(&stale, &[state("u.a", Fate::Stale, false)]).is_err());
        let wrong_tag = json!({"kind": "FunctionDecl", "inner": [
            {"kind": "ParmVarDecl", "id": "p", "type": {"qualType": "struct other *"}},
            {"kind": "CompoundStmt", "inner": []},
        ]});
        assert!(validates_spelled(&wrong_tag, &[state("u.a", Fate::Stale, false)]).is_err());
    }

    #[test]
    fn an_unstored_stale_path_validates() {
        let body = function(vec![json!({"kind": "ReturnStmt"})]);
        assert_eq!(
            validates_spelled(&body, &[state("u.a", Fate::Stale, false)]),
            Ok(true)
        );
    }
}
