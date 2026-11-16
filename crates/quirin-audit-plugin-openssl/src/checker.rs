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

//! The class rules: what each checked contract class requires of a
//! definition's Clang AST. Every rule is a closed whitelist — a node kind,
//! operator or cast it does not name fails it — so a body the rule was not
//! written for is refused, never guessed. A refusal is an undischarged
//! obligation, and the binding is `Unknown`.
//!
//! * **no-pointer-interface**: no parameter, and a result that is `void` or a
//!   builtin integer, spelled so (no typedef can hide a pointer).
//! * **static-object-return**: no parameter, and the body is exactly
//!   `return &V;` for the contract's `V`, a `static const` object at file
//!   scope of the defining file (the object dump has exactly one declaration
//!   of that name).
//! * **pure-reader**: every node of the body is one [`pure_node`] admits:
//!   no call, no assignment, no increment or decrement, no loop, no `goto`, no
//!   `static` local, and every name read a parameter, a local of the body or
//!   an enumerator. Such a body terminates, writes nothing outside its own
//!   locals, and allocates and releases nothing; its result is computed from
//!   its arguments' reachable memory and constants.
//! * **member-store**: every statement is a store `p->m = e` or `p->m op= e`
//!   (alone, or as the returned expression) with `p` a parameter and `e` a
//!   pure expression over parameters; the stores and the escaping pointer
//!   parameters must equal the contract's exactly.
//!
//! `QCore.OpensslContractPlugin` models these rules (the whitelist and its
//! consequence, the escape computation) and the verdict a binding gets.

use std::collections::BTreeSet;

use serde_json::Value;

use crate::ast::{body, children, declaration_file, kind, parameters, qual_type, reaches_memory};
use crate::contracts::Store;

/// The spelled prototypes `no-pointer-interface` admits.
pub const NO_POINTER_PROTOTYPES: [&str; 5] = [
    "void (void)",
    "int (void)",
    "unsigned int (void)",
    "long (void)",
    "unsigned long (void)",
];

/// Cast kinds that only read or reinterpret a value.
pub const PURE_CASTS: [&str; 8] = [
    "LValueToRValue",
    "NoOp",
    "BitCast",
    "NullToPointer",
    "IntegralCast",
    "IntegralToBoolean",
    "PointerToBoolean",
    "ArrayToPointerDecay",
];

/// Binary operators that compute a value and assign nothing.
pub const PURE_BINARY: [&str; 19] = [
    "==", "!=", "<", ">", "<=", ">=", "&&", "||", "+", "-", "*", "/", "%", "&", "|", "^", "<<", ">>",
    ",",
];

/// Unary operators that compute a value and assign nothing (`++`/`--` are
/// not among them).
pub const PURE_UNARY: [&str; 6] = ["!", "-", "+", "~", "*", "&"];

/// The names a pure expression may read.
#[derive(Debug, Default)]
pub struct Scope {
    /// Parameter declaration ids, in order.
    pub parameters: Vec<String>,
    /// Local variable declaration ids of the body.
    pub locals: BTreeSet<String>,
}

impl Scope {
    fn of(function: &Value) -> Self {
        Self {
            parameters: parameters(function)
                .iter()
                .map(|parameter| parameter["id"].as_str().unwrap_or("").to_owned())
                .collect(),
            locals: BTreeSet::new(),
        }
    }
}

fn node_name(node: &Value) -> String {
    let mut name = kind(node).to_owned();
    for key in ["opcode", "castKind"] {
        if let Some(value) = node[key].as_str() {
            name.push_str(&format!(" `{value}`"));
        }
    }
    name
}

/// Whether `node` and everything below it is pure (see the module
/// documentation), recording the parameters it reads by index.
///
/// # Errors
///
/// The first node the whitelist does not admit.
pub fn pure_node(node: &Value, scope: &Scope, read: &mut BTreeSet<usize>) -> Result<(), String> {
    let admitted = match kind(node) {
        "CompoundStmt" | "ReturnStmt" | "IfStmt" | "NullStmt" | "DeclStmt" | "ParenExpr"
        | "MemberExpr" | "ArraySubscriptExpr" | "IntegerLiteral" | "CharacterLiteral"
        | "ConditionalOperator" | "ConstantExpr" | "UnaryExprOrTypeTraitExpr" => true,
        "VarDecl" => {
            node["storageClass"].is_null()
                && scope.locals.contains(node["id"].as_str().unwrap_or(""))
        }
        "ImplicitCastExpr" | "CStyleCastExpr" => node["castKind"]
            .as_str()
            .is_some_and(|cast| PURE_CASTS.contains(&cast)),
        "BinaryOperator" => node["opcode"]
            .as_str()
            .is_some_and(|operator| PURE_BINARY.contains(&operator)),
        "UnaryOperator" => node["opcode"]
            .as_str()
            .is_some_and(|operator| PURE_UNARY.contains(&operator)),
        "DeclRefExpr" => {
            let referenced = &node["referencedDecl"];
            let id = referenced["id"].as_str().unwrap_or("");
            match referenced["kind"].as_str() {
                Some("ParmVarDecl") => match scope.parameters.iter().position(|p| p == id) {
                    Some(index) => {
                        read.insert(index);
                        true
                    }
                    None => false,
                },
                Some("VarDecl") => scope.locals.contains(id),
                Some("EnumConstantDecl") => true,
                _ => false,
            }
        }
        _ => false,
    };
    if !admitted {
        return Err(format!(
            "the body holds {}{}, which the rule does not admit",
            node_name(node),
            node["referencedDecl"]["name"]
                .as_str()
                .map(|name| format!(" (`{name}`)"))
                .unwrap_or_default()
        ));
    }
    for child in children(node) {
        pure_node(child, scope, read)?;
    }
    Ok(())
}

fn collect_locals(node: &Value, locals: &mut BTreeSet<String>) {
    if kind(node) == "VarDecl" {
        if let Some(id) = node["id"].as_str() {
            locals.insert(id.to_owned());
        }
    }
    for child in children(node) {
        collect_locals(child, locals);
    }
}

fn strip<'a>(mut node: &'a Value, kinds: &[&str]) -> &'a Value {
    while kinds.contains(&kind(node)) && children(node).len() == 1 {
        node = &children(node)[0];
    }
    node
}

fn definition_body(function: &Value) -> Result<&Value, String> {
    body(function).ok_or_else(|| "the declaration is not a definition".to_owned())
}

/// `no-pointer-interface`.
///
/// # Errors
///
/// A parameter, or a prototype outside [`NO_POINTER_PROTOTYPES`].
pub fn no_pointer_interface(function: &Value) -> Result<String, String> {
    definition_body(function)?;
    if !parameters(function).is_empty() {
        return Err("the definition has parameters".into());
    }
    let spelled = qual_type(function);
    if !NO_POINTER_PROTOTYPES.contains(&spelled) {
        return Err(format!(
            "the prototype `{spelled}` is not a void or builtin-integer function of no \
             parameter"
        ));
    }
    Ok(format!(
        "`{spelled}`: no parameter and a {} result, so no caller resource is passed in or \
         returned",
        if spelled.starts_with("void") {
            "void"
        } else {
            "builtin-integer"
        }
    ))
}

/// `static-object-return`: `objects` is the dump of the declarations named
/// `returns` in the defining file `file`.
///
/// # Errors
///
/// A body other than `return &returns;`, or an object that is not one
/// `static const` file-scope declaration of the defining file.
pub fn static_object_return(
    function: &Value,
    returns: &str,
    objects: &[Value],
    file: &str,
) -> Result<String, String> {
    let body = definition_body(function)?;
    if !parameters(function).is_empty() {
        return Err("the definition has parameters".into());
    }
    let [statement] = children(body) else {
        return Err("the body is not a single statement".into());
    };
    if kind(statement) != "ReturnStmt" {
        return Err("the body's statement is not a return".into());
    }
    let [value] = children(statement) else {
        return Err("the return has no value".into());
    };
    let address = strip(value, &["ParenExpr", "ImplicitCastExpr"]);
    if kind(address) != "UnaryOperator" || address["opcode"].as_str() != Some("&") {
        return Err("the returned value is not an address-of expression".into());
    }
    let [operand] = children(address) else {
        return Err("the address-of has no operand".into());
    };
    let reference = strip(operand, &["ParenExpr"]);
    let named = &reference["referencedDecl"];
    if kind(reference) != "DeclRefExpr"
        || named["kind"].as_str() != Some("VarDecl")
        || named["name"].as_str() != Some(returns)
    {
        return Err(format!("the returned address is not that of `{returns}`"));
    }
    let declarations: Vec<&Value> = objects
        .iter()
        .filter(|object| kind(object) == "VarDecl" && object["name"].as_str() == Some(returns))
        .collect();
    let [object] = declarations.as_slice() else {
        return Err(format!(
            "`{returns}` is declared {} times in the defining file's dump, not once",
            declarations.len()
        ));
    };
    if object["storageClass"].as_str() != Some("static")
        || !qual_type(object).starts_with("const ")
        || declaration_file(object) != Some(file)
    {
        return Err(format!(
            "`{returns}` is not a static const object of {file} (`{}`, storage {})",
            qual_type(object),
            object["storageClass"].as_str().unwrap_or("none")
        ));
    }
    Ok(format!(
        "the body is `return &{returns};` and `{returns}` is a static `{}` of {file}: \
         library-owned static storage",
        qual_type(object)
    ))
}

/// `pure-reader`.
///
/// # Errors
///
/// The first node of the body the whitelist does not admit.
pub fn pure_reader(function: &Value) -> Result<String, String> {
    let body = definition_body(function)?;
    let mut scope = Scope::of(function);
    collect_locals(body, &mut scope.locals);
    let mut read = BTreeSet::new();
    pure_node(body, &scope, &mut read)?;
    Ok(format!(
        "the body is loop-free and call-free with no assignment, increment or static local; \
         it reads parameter(s) {read:?} and {} local(s)",
        scope.locals.len()
    ))
}

/// The stores of a `member-store` body, or the first statement that is not
/// one.
///
/// # Errors
///
/// A statement that is not a store of the admitted shape.
pub fn stores(function: &Value) -> Result<Vec<Store>, String> {
    let body = definition_body(function)?;
    let scope = Scope::of(function);
    let mut found = Vec::new();
    for statement in children(body) {
        let expression = if kind(statement) == "ReturnStmt" {
            match children(statement) {
                [value] => value,
                _ => return Err("a return has no value".into()),
            }
        } else {
            statement
        };
        let expression = strip(expression, &["ParenExpr"]);
        let operator = match kind(expression) {
            "BinaryOperator" if expression["opcode"].as_str() == Some("=") => "=",
            "CompoundAssignOperator" => expression["opcode"].as_str().unwrap_or(""),
            _ => {
                return Err(format!(
                    "a statement is {}, not a member store",
                    node_name(expression)
                ));
            }
        };
        let [target, value] = children(expression) else {
            return Err("a store does not have two operands".into());
        };
        let target = strip(target, &["ParenExpr"]);
        if kind(target) != "MemberExpr" || target["isArrow"].as_bool() != Some(true) {
            return Err("a store's target is not `parameter->member`".into());
        }
        let member = target["name"].as_str().unwrap_or("").to_owned();
        let [object] = children(target) else {
            return Err("a member access has no object".into());
        };
        let object = strip(object, &["ParenExpr", "ImplicitCastExpr"]);
        let id = object["referencedDecl"]["id"].as_str().unwrap_or("");
        let base = match (kind(object), object["referencedDecl"]["kind"].as_str()) {
            ("DeclRefExpr", Some("ParmVarDecl")) => scope.parameters.iter().position(|p| p == id),
            _ => None,
        }
        .ok_or("a store's target is not a member of a parameter")?;
        let mut read = BTreeSet::new();
        pure_node(value, &scope, &mut read)?;
        found.push(Store {
            base,
            member,
            operator: operator.to_owned(),
            parameters: read.into_iter().collect(),
        });
    }
    Ok(found)
}

/// The pointer parameters `stores` read: they escape into the base object.
#[must_use]
pub fn escapes(function: &Value, stores: &[Store]) -> Vec<usize> {
    let parameters = parameters(function);
    stores
        .iter()
        .flat_map(|store| store.parameters.iter().copied())
        .filter(|index| parameters.get(*index).is_some_and(|p| reaches_memory(p)))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

/// `member-store` against the contract's stores and escapes.
///
/// # Errors
///
/// A body that is not stores only, or stores or escapes other than the
/// contract's.
pub fn member_store(
    function: &Value,
    expected: &[Store],
    expected_escapes: &[usize],
) -> Result<String, String> {
    let found = stores(function)?;
    if found != expected {
        return Err(format!(
            "the body's stores {found:?} are not the contract's {expected:?}"
        ));
    }
    let escaping = escapes(function, &found);
    if escaping != expected_escapes {
        return Err(format!(
            "the escaping parameters {escaping:?} are not the contract's {expected_escapes:?}"
        ));
    }
    Ok(format!(
        "the body is exactly the stores {}; escaping parameter(s) {escaping:?}",
        found
            .iter()
            .map(|store| format!(
                "p{}->{} {} f(p{:?})",
                store.base, store.member, store.operator, store.parameters
            ))
            .collect::<Vec<_>>()
            .join(", ")
    ))
}
