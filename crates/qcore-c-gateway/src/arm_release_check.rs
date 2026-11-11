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

//! Card **CG22-OP-CALLBACKS-AND-SERVED-QUEUES** (gateway rule 12o; contract
//! spec §C04.19.2): the body check of a type-switched release.
//!
//! In a unit that defines the declared function, the arms stand only when its
//! Clang body has exactly one `switch` whose condition reads the declared
//! discriminant of the declared parameter (`P->FIELD`, or `P->FIELD & E`
//! with `E` folding to `~MASK`), and, read case group by case group (stacked
//! labels and fall-through accumulate; `break` and `return` end a group),
//! each value's **payload paths** are exactly the declared ones: a payload
//! path is a member path of `P` (through `.`, `->` of `P` only, `&`, casts
//! and subscripts) handed as an actual to a call inside the group, whose
//! first segment is the first segment of some declared arm path (the union
//! the arms name, `rko_u`). A value no label names takes `default:` (or
//! nothing); `otherwise` is checked against it. A `break` the check cannot
//! place (under an `if` outside a loop) refuses. The check establishes which
//! members each arm hands to a call; that the call releases them stays a
//! declared assumption.

use std::collections::{BTreeMap, BTreeSet};

use qcore_schemas::arm_release::{ArmClause, ArmEffect, ArmGuard, Discriminant};
use serde_json::Value;

use crate::typed_c_ast::{children, kind};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Label {
    Value(i64),
    Default,
}

fn strip(mut node: &Value) -> &Value {
    loop {
        match kind(node) {
            "ParenExpr" | "ImplicitCastExpr" | "CStyleCastExpr" | "ConstantExpr" => {
                match children(node).first() {
                    Some(inner) => node = inner,
                    None => return node,
                }
            }
            _ => return node,
        }
    }
}

/// Fold an integer constant expression of literals, casts, enumerators (by
/// `enumerators`, declaration id to value), `~`, `-`, `|`, `&`, `<<`, `>>`
/// and `+`.
pub(crate) fn fold_with(node: &Value, enumerators: &BTreeMap<String, i64>) -> Option<i64> {
    let fold = |node: &Value| fold_with(node, enumerators);
    if kind(node) == "ConstantExpr"
        && let Some(value) = node["value"].as_str()
    {
        return value.parse().ok();
    }
    let node = strip(node);
    match kind(node) {
        "IntegerLiteral" => node["value"].as_str()?.parse().ok(),
        "DeclRefExpr" if node["referencedDecl"]["kind"].as_str() == Some("EnumConstantDecl") => {
            enumerators
                .get(node["referencedDecl"]["id"].as_str()?)
                .copied()
        }
        "UnaryOperator" => {
            let inner = fold(children(node).first()?)?;
            match node["opcode"].as_str()? {
                "~" => Some(!inner),
                "-" => inner.checked_neg(),
                "+" => Some(inner),
                _ => None,
            }
        }
        "BinaryOperator" => {
            let [left, right] = children(node) else {
                return None;
            };
            let (left, right) = (fold(left)?, fold(right)?);
            match node["opcode"].as_str()? {
                "|" => Some(left | right),
                "&" => Some(left & right),
                "<<" => left.checked_shl(u32::try_from(right).ok()?),
                ">>" => left.checked_shr(u32::try_from(right).ok()?),
                "+" => left.checked_add(right),
                _ => None,
            }
        }
        _ => None,
    }
}

/// The member path of `node` from the parameter whose declaration id is
/// `parameter`, `None` for anything else.
fn path_of(node: &Value, parameter: &str) -> Option<String> {
    let mut segments = Vec::new();
    let mut node = strip(node);
    loop {
        match kind(node) {
            "MemberExpr" => {
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
            _ => return None,
        }
    }
}

/// The payload path an actual names: through `&`, casts, parentheses and a
/// subscript's base.
fn actual_path(node: &Value, parameter: &str) -> Option<String> {
    let mut node = strip(node);
    loop {
        match kind(node) {
            "UnaryOperator" if node["opcode"].as_str() == Some("&") => {
                node = strip(children(node).first()?);
            }
            "ArraySubscriptExpr" => node = strip(children(node).first()?),
            _ => return path_of(node, parameter),
        }
    }
}

fn collect_paths(
    node: &Value,
    parameter: &str,
    roots: &BTreeSet<&str>,
    out: &mut BTreeSet<String>,
) {
    if kind(node) == "CallExpr" {
        for argument in children(node).iter().skip(1) {
            if let Some(path) = actual_path(argument, parameter)
                && roots.contains(path.split('.').next().unwrap_or(""))
            {
                out.insert(path);
            }
        }
    }
    for child in children(node) {
        collect_paths(child, parameter, roots, out);
    }
}

/// Whether `node` holds a `break` that leaves the enclosing switch from
/// somewhere the check cannot place (not inside a loop or a nested switch).
fn stray_break(node: &Value) -> bool {
    match kind(node) {
        "BreakStmt" => true,
        "ForStmt" | "WhileStmt" | "DoStmt" | "SwitchStmt" => false,
        _ => children(node).iter().any(stray_break),
    }
}

struct Groups<'a> {
    enumerators: &'a BTreeMap<String, i64>,
    parameter: &'a str,
    roots: BTreeSet<&'a str>,
    active: BTreeSet<Label>,
    paths: BTreeMap<Label, BTreeSet<String>>,
}

impl Groups<'_> {
    fn statement(&mut self, node: &Value) -> Result<(), String> {
        match kind(node) {
            "CaseStmt" => {
                let value = children(node)
                    .first()
                    .and_then(|label| fold_with(label, self.enumerators))
                    .ok_or("a case label the check does not fold")?;
                self.label(Label::Value(value));
                if let Some(body) = children(node).get(1) {
                    self.statement(body)?;
                }
                Ok(())
            }
            "DefaultStmt" => {
                self.label(Label::Default);
                if let Some(body) = children(node).first() {
                    self.statement(body)?;
                }
                Ok(())
            }
            "BreakStmt" => {
                self.active.clear();
                Ok(())
            }
            "CompoundStmt" => {
                for child in children(node) {
                    self.statement(child)?;
                }
                Ok(())
            }
            "ReturnStmt" => {
                self.add(node);
                self.active.clear();
                Ok(())
            }
            _ => {
                if stray_break(node) {
                    return Err("a `break` the check cannot place".into());
                }
                self.add(node);
                Ok(())
            }
        }
    }

    fn label(&mut self, label: Label) {
        self.active.insert(label);
        self.paths.entry(label).or_default();
    }

    fn add(&mut self, node: &Value) {
        let mut found = BTreeSet::new();
        collect_paths(node, self.parameter, &self.roots, &mut found);
        for label in &self.active {
            self.paths
                .entry(*label)
                .or_default()
                .extend(found.iter().cloned());
        }
    }
}

fn switches<'a>(node: &'a Value, out: &mut Vec<&'a Value>) {
    if kind(node) == "SwitchStmt" {
        out.push(node);
    }
    for child in children(node) {
        switches(child, out);
    }
}

/// Whether a switch condition reads the declared discriminant.
fn reads_discriminant(
    condition: &Value,
    parameter: &str,
    discriminant: &Discriminant,
    enumerators: &BTreeMap<String, i64>,
) -> bool {
    let fold = |node: &Value| fold_with(node, enumerators);
    let condition = strip(condition);
    if discriminant.mask == 0 {
        return path_of(condition, parameter).as_deref() == Some(discriminant.field.as_str());
    }
    if kind(condition) != "BinaryOperator" || condition["opcode"].as_str() != Some("&") {
        return false;
    }
    let [left, right] = children(condition) else {
        return false;
    };
    let cleared = i64::try_from(discriminant.mask).ok().map(|mask| !mask);
    path_of(left, parameter).as_deref() == Some(discriminant.field.as_str())
        && fold(right).is_some()
        && fold(right).map(|value| value as i32 as i64) == cleared.map(|value| value as i32 as i64)
}

/// Rule 12o: whether `function`'s Clang body validates the arms.
///
/// # Errors
///
/// Why the body does not validate them.
pub fn validates(
    function: &Value,
    discriminant: &Discriminant,
    arms: &[ArmClause],
    enumerators: &BTreeMap<String, i64>,
) -> Result<(), String> {
    let parameter = children(function)
        .iter()
        .filter(|child| kind(child) == "ParmVarDecl")
        .nth(discriminant.parameter)
        .and_then(|parameter| parameter["id"].as_str())
        .ok_or("the discriminated parameter is not read")?;
    let mut found = Vec::new();
    switches(function, &mut found);
    let selected = found
        .into_iter()
        .filter(|switch| {
            children(switch).first().is_some_and(|condition| {
                reads_discriminant(condition, parameter, discriminant, enumerators)
            })
        })
        .collect::<Vec<_>>();
    let [switch] = selected.as_slice() else {
        return Err(format!(
            "{} switches read the discriminant `{}`",
            selected.len(),
            discriminant.field
        ));
    };
    let roots = arms
        .iter()
        .filter_map(|arm| match &arm.effect {
            ArmEffect::Released { path } => path.split('.').next(),
            ArmEffect::Empty => None,
        })
        .collect::<BTreeSet<_>>();
    let mut groups = Groups {
        enumerators,
        parameter,
        roots,
        active: BTreeSet::new(),
        paths: BTreeMap::new(),
    };
    let body = children(switch).last().ok_or("a switch with no body")?;
    groups.statement(body)?;
    let mut declared: BTreeMap<ArmGuard, BTreeSet<String>> = BTreeMap::new();
    for arm in arms {
        let entry = declared.entry(arm.guard).or_default();
        if let ArmEffect::Released { path } = &arm.effect {
            entry.insert(path.clone());
        }
    }
    let body_default = groups
        .paths
        .get(&Label::Default)
        .cloned()
        .unwrap_or_default();
    let declared_otherwise = declared
        .get(&ArmGuard::Otherwise)
        .cloned()
        .unwrap_or_default();
    if body_default != declared_otherwise {
        return Err("the default arm hands other members to a call than `otherwise` names".into());
    }
    let mut values = groups
        .paths
        .keys()
        .filter_map(|label| match label {
            Label::Value(value) => Some(*value),
            Label::Default => None,
        })
        .collect::<BTreeSet<_>>();
    values.extend(declared.keys().filter_map(|guard| match guard {
        ArmGuard::Equals { value } => Some(*value),
        ArmGuard::Otherwise => None,
    }));
    for value in values {
        let body = groups
            .paths
            .get(&Label::Value(value))
            .unwrap_or(&body_default);
        let stated = declared
            .get(&ArmGuard::Equals { value })
            .unwrap_or(&declared_otherwise);
        if body != stated {
            return Err(format!(
                "the arm {value} hands {body:?} to a call, the declaration names {stated:?}"
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn member(path: &[&str]) -> Value {
        // rko->a.b.c as nested MemberExpr nodes.
        let mut node = serde_json::json!({
            "kind": "DeclRefExpr",
            "referencedDecl": {"kind": "ParmVarDecl", "id": "0xP", "name": "rko"}
        });
        for (at, name) in path.iter().enumerate() {
            node = serde_json::json!({
                "kind": "MemberExpr", "name": name, "isArrow": at == 0, "inner": [node]
            });
        }
        node
    }

    fn call(argument: Value) -> Value {
        serde_json::json!({"kind": "CallExpr", "inner": [
            {"kind": "DeclRefExpr", "referencedDecl": {"kind": "FunctionDecl", "name": "free"}},
            argument
        ]})
    }

    fn case(value: i64, body: Value) -> Value {
        serde_json::json!({"kind": "CaseStmt", "inner": [
            {"kind": "ConstantExpr", "value": value.to_string()}, body
        ]})
    }

    fn function(statements: Vec<Value>) -> Value {
        serde_json::json!({"kind": "FunctionDecl", "inner": [
            {"kind": "ParmVarDecl", "id": "0xP", "name": "rko"},
            {"kind": "CompoundStmt", "inner": [
                {"kind": "SwitchStmt", "inner": [
                    member(&["type"]),
                    {"kind": "CompoundStmt", "inner": statements}
                ]}
            ]}
        ]})
    }

    fn arms(list: &[(Option<i64>, Option<&str>)]) -> Vec<ArmClause> {
        list.iter()
            .map(|(value, path)| ArmClause {
                guard: value.map_or(ArmGuard::Otherwise, |value| ArmGuard::Equals { value }),
                effect: path.map_or(ArmEffect::Empty, |path| ArmEffect::Released {
                    path: path.to_owned(),
                }),
            })
            .collect()
    }

    const DISCRIMINANT: Discriminant = Discriminant {
        parameter: 0,
        field: String::new(),
        mask: 0,
    };

    fn discriminant() -> Discriminant {
        Discriminant {
            field: "type".to_owned(),
            ..DISCRIMINANT
        }
    }

    #[test]
    fn groups_with_stacked_labels_and_fall_through_validate() {
        let body = function(vec![
            case(1, call(member(&["u", "a", "x"]))),
            serde_json::json!({"kind": "BreakStmt"}),
            case(2, case(3, call(member(&["u", "b", "y"])))),
            // 2 and 3 fall through into 4.
            case(4, call(member(&["u", "c", "z"]))),
            serde_json::json!({"kind": "BreakStmt"}),
            serde_json::json!({"kind": "DefaultStmt", "inner": [{"kind": "BreakStmt"}]}),
        ]);
        let declared = arms(&[
            (Some(1), Some("u.a.x")),
            (Some(2), Some("u.b.y")),
            (Some(2), Some("u.c.z")),
            (Some(3), Some("u.b.y")),
            (Some(3), Some("u.c.z")),
            (Some(4), Some("u.c.z")),
            (None, None),
        ]);
        assert_eq!(
            validates(&body, &discriminant(), &declared, &BTreeMap::new()),
            Ok(())
        );
        // A missing release, or a declared one the body does not hand on.
        let missing = arms(&[
            (Some(1), Some("u.a.x")),
            (Some(2), Some("u.b.y")),
            (Some(3), Some("u.b.y")),
            (Some(3), Some("u.c.z")),
            (Some(4), Some("u.c.z")),
            (None, None),
        ]);
        assert!(validates(&body, &discriminant(), &missing, &BTreeMap::new()).is_err());
        let extra = arms(&[(Some(9), Some("u.a.x")), (None, None)]);
        assert!(validates(&body, &discriminant(), &extra, &BTreeMap::new()).is_err());
    }

    #[test]
    fn a_value_with_no_label_takes_the_default() {
        let body = function(vec![
            case(1, call(member(&["u", "a", "x"]))),
            serde_json::json!({"kind": "BreakStmt"}),
            serde_json::json!({"kind": "DefaultStmt", "inner": [{"kind": "BreakStmt"}]}),
        ]);
        // A declared arm for 7 releasing nothing agrees with the default.
        let declared = arms(&[(Some(1), Some("u.a.x")), (Some(7), None), (None, None)]);
        assert_eq!(
            validates(&body, &discriminant(), &declared, &BTreeMap::new()),
            Ok(())
        );
        // `otherwise` releasing something the default does not.
        let wrong = arms(&[(Some(1), Some("u.a.x")), (None, Some("u.a.x"))]);
        assert!(validates(&body, &discriminant(), &wrong, &BTreeMap::new()).is_err());
    }

    #[test]
    fn a_conditional_break_is_refused() {
        let body = function(vec![
            case(
                1,
                serde_json::json!({"kind": "IfStmt", "inner": [
                    member(&["type"]), {"kind": "BreakStmt"}
                ]}),
            ),
            call(member(&["u", "a", "x"])),
            serde_json::json!({"kind": "BreakStmt"}),
        ]);
        let declared = arms(&[(Some(1), Some("u.a.x")), (None, None)]);
        assert!(validates(&body, &discriminant(), &declared, &BTreeMap::new()).is_err());
    }
}
