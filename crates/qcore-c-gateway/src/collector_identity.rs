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

//! Source-checked first-match collector lookup and constructor-local loop equations.
//!
//! These imports prove bounded source shapes, never a stable ambient registry,
//! a caller's configuration equality, nonnullness or lifetime safety. They are
//! deliberately not deserializable authority. A later consumer must establish
//! its own frame and exact actual substitution before using either reading.
use crate::legacy_c::{C11TranslationUnit, validate_unrestricted_c11};
use crate::typed_c_ast::{children, kind};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectorQuery {
    pub function: String,
    pub record: String,
    pub owner_record: String,
    pub list_record: String,
    pub list_member: String,
    pub link_member: String,
    pub key_member: String,
}

/// Exact source/configuration checkpoint. Serialization is a report, not an
/// import path; replay always invokes the source reader again.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SelectionImport {
    query: SelectorQuery,
    source_sha256: String,
    preprocessing_sha256: String,
    configuration_sha256: String,
    compiler: crate::clang::ClangIdentity,
    body_sha256: String,
    key_parameter: usize,
    owner_parameter: usize,
}
impl SelectionImport {
    pub fn query(&self) -> &SelectorQuery {
        &self.query
    }
    pub fn source_sha256(&self) -> &str {
        &self.source_sha256
    }
    /// Selection is the first matching entry of this owner's current list.
    /// Duplicate keys are allowed; neither uniqueness nor cross-call stability
    /// follows from the source reading.
    pub fn parameter_roles(&self) -> (usize, usize) {
        (self.key_parameter, self.owner_parameter)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoopQuery {
    pub function: String,
    pub predicate: String,
    pub record: String,
    pub owner_record: String,
    pub worker_record: String,
    pub config_record: String,
    pub loop_record: String,
    pub instance_member: String,
    pub threaded_member: String,
    pub worker_member: String,
    pub loop_member: String,
}

/// A relation at the join immediately after one checked conditional store.
/// It is not an invariant after an unread call, another write or a flag change.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ConditionalLoopImport {
    query: LoopQuery,
    source_sha256: String,
    preprocessing_sha256: String,
    configuration_sha256: String,
    compiler: crate::clang::ClangIdentity,
    constructor_sha256: String,
    predicate_sha256: String,
    owner_parameter: usize,
    config_parameter: usize,
    record_local: String,
}
impl ConditionalLoopImport {
    pub fn query(&self) -> &LoopQuery {
        &self.query
    }
    pub fn parameter_roles(&self) -> (usize, usize) {
        (self.owner_parameter, self.config_parameter)
    }
    pub fn record_local(&self) -> &str {
        &self.record_local
    }
}

fn tokens(text: &str) -> Vec<String> {
    crate::legacy_c::tokenize(text)
        .into_iter()
        .map(|t| t.text)
        .collect()
}
fn ty(n: &Value) -> &str {
    n["type"]["desugaredQualType"]
        .as_str()
        .or_else(|| n["type"]["qualType"].as_str())
        .unwrap_or("")
}
fn name(n: &Value) -> Result<&str, String> {
    n["name"]
        .as_str()
        .ok_or_else(|| "missing declaration name".into())
}
fn text<'a>(source: &'a str, n: &Value) -> Result<&'a str, String> {
    let a = n["range"]["begin"]["offset"]
        .as_u64()
        .ok_or("missing source begin")?;
    let b = n["range"]["end"]["offset"]
        .as_u64()
        .ok_or("missing source end")?;
    let len = n["range"]["end"]["tokLen"]
        .as_u64()
        .ok_or("missing source token length")?;
    source
        .get(a as usize..b.checked_add(len).ok_or("source span overflow")? as usize)
        .ok_or_else(|| "source span is outside preprocessing snapshot".into())
}
fn definition<'a>(ast: &'a Value, wanted: &str) -> Result<&'a Value, String> {
    let matches = children(ast)
        .iter()
        .filter(|n| {
            kind(n) == "FunctionDecl"
                && n["name"].as_str() == Some(wanted)
                && children(n).iter().any(|c| kind(c) == "CompoundStmt")
        })
        .collect::<Vec<_>>();
    let [one] = matches.as_slice() else {
        return Err("absent or ambiguous function definition".into());
    };
    Ok(one)
}
fn body(n: &Value) -> Result<&Value, String> {
    children(n)
        .iter()
        .find(|n| kind(n) == "CompoundStmt")
        .ok_or_else(|| "missing body".into())
}
fn field(ast: &Value, record: &str, member: &str, expected: &str) -> Result<(), String> {
    let mut defs = Vec::new();
    let mut pending = vec![ast];
    while let Some(n) = pending.pop() {
        if kind(n) == "RecordDecl"
            && n["name"].as_str() == Some(record)
            && n["completeDefinition"].as_bool() == Some(true)
        {
            defs.push(n);
        }
        pending.extend(children(n));
    }
    let [def] = defs.as_slice() else {
        return Err("missing or ambiguous record definition".into());
    };
    if def["tagUsed"].as_str() != Some("struct") {
        return Err("record is not an ordinary struct".into());
    }
    let fields = children(def)
        .iter()
        .filter(|n| kind(n) == "FieldDecl" && n["name"].as_str() == Some(member))
        .collect::<Vec<_>>();
    let [f] = fields.as_slice() else {
        return Err("missing or ambiguous direct field".into());
    };
    if ty(f) != expected || f["isBitfield"].as_bool() == Some(true) {
        return Err(format!(
            "field {record}.{member} is not exact ordinary {expected}"
        ));
    }
    Ok(())
}
fn parameters(n: &Value) -> Vec<&Value> {
    children(n)
        .iter()
        .filter(|n| kind(n) == "ParmVarDecl")
        .collect()
}
fn unique_typed<'a>(decls: &[&'a Value], expected: &str) -> Result<(usize, &'a Value), String> {
    let values = decls
        .iter()
        .enumerate()
        .filter(|(_, n)| ty(n) == expected)
        .collect::<Vec<_>>();
    let [(index, value)] = values.as_slice() else {
        return Err(format!("missing or ambiguous role of type {expected}"));
    };
    Ok((*index, *value))
}
fn locals(n: &Value) -> Vec<&Value> {
    children(n)
        .iter()
        .filter(|n| kind(n) == "DeclStmt")
        .flat_map(children)
        .filter(|n| kind(n) == "VarDecl")
        .collect()
}
fn stable_binding(body: &Value, declaration: &Value, expected_writes: usize) -> bool {
    let Some(id) = declaration["id"].as_str() else {
        return false;
    };
    let mut pending = vec![body];
    let mut writes = 0usize;
    while let Some(n) = pending.pop() {
        if matches!(
            kind(n),
            "GCCAsmStmt" | "MSAsmStmt" | "GotoStmt" | "IndirectGotoStmt" | "LabelStmt"
        ) {
            return false;
        }
        if kind(n) == "UnaryOperator" && children(n).first().is_some_and(|n| reference(n, id)) {
            match n["opcode"].as_str() {
                Some("&") => return false,
                Some("++" | "--") => writes += 1,
                _ => (),
            }
        }
        if (kind(n) == "BinaryOperator" && n["opcode"].as_str() == Some("=")
            || kind(n) == "CompoundAssignOperator")
            && children(n).first().is_some_and(|n| reference(n, id))
        {
            writes += 1;
        }
        pending.extend(children(n));
    }
    writes == expected_writes
}

fn digest(unit: &C11TranslationUnit<'_>) -> String {
    qcore_schemas::sha256_hex(
        format!(
            "{:?}\0{:?}\0{:?}\0{:?}\0{:?}",
            unit.input,
            unit.dialect,
            unit.include_paths,
            unit.definitions,
            unit.preprocessor_options
        )
        .as_bytes(),
    )
}
fn prepared(unit: &C11TranslationUnit<'_>) -> Result<(String, Value), String> {
    let source = validate_unrestricted_c11(unit).map_err(|e| format!("{e:?}"))?;
    let ast = crate::typed_c_ast::translation_unit_ast(&source, unit)?;
    Ok((source, ast))
}

/// Import the complete production GNU container-of first-match loop. No macro
/// spelling is trusted: preprocessing happens first, then the complete typed
/// function body must equal the bounded expansion, including its null result.
pub fn import_selection(
    unit: &C11TranslationUnit<'_>,
    query: &SelectorQuery,
) -> Result<SelectionImport, String> {
    let (source, ast) = prepared(unit)?;
    let function = definition(&ast, &query.function)?;
    if function["storageClass"].as_str() != Some("static") {
        return Err("selector needs internal linkage or a separately replayed home".into());
    }
    if ty(function)
        != format!(
            "struct {} *(int, struct {} *)",
            query.record, query.owner_record
        )
    {
        return Err("selector has an unsupported exact signature".into());
    }
    let params = parameters(function);
    if params.len() != 2 {
        return Err("selector needs exactly two typed parameters".into());
    }
    let (key_parameter, id) = unique_typed(&params, "int")?;
    let (owner_parameter, owner) =
        unique_typed(&params, &format!("struct {} *", query.owner_record))?;
    let b = body(function)?;
    let ls = locals(b);
    if ls.len() != 2 {
        return Err("selector needs exactly two direct locals".into());
    }
    let (_, head) = unique_typed(&ls, &format!("struct {} *", query.list_record))?;
    let (_, coll) = unique_typed(&ls, &format!("struct {} *", query.record))?;
    field(
        &ast,
        &query.owner_record,
        &query.list_member,
        &format!("struct {}", query.list_record),
    )?;
    field(
        &ast,
        &query.record,
        &query.link_member,
        &format!("struct {}", query.list_record),
    )?;
    field(&ast, &query.record, &query.key_member, "int")?;
    field(
        &ast,
        &query.list_record,
        "next",
        &format!("struct {} *", query.list_record),
    )?;
    let (id, owner, head, coll) = (name(id)?, name(owner)?, name(head)?, name(coll)?);
    let r = &query.record;
    let lr = &query.list_record;
    let list = &query.list_member;
    let link = &query.link_member;
    let key = &query.key_member;
    let expected = format!(
        r#"{{ struct {lr} *{head}; struct {r} *{coll};
        for ({head}=(&{owner}->{list})->next; {head}!=(&{owner}->{list}); {head}={head}->next) {{
            {coll}=({{ const typeof(((struct {r} *)0)->{link}) *__mptr=({head});
                (struct {r} *)((char *)__mptr - ((size_t)&((struct {r} *)0)->{link})); }});
            if ({coll}->{key}=={id}) {{ return {coll}; }}
        }} return ((void *)0); }}"#
    );
    let actual = text(&source, b)?;
    let builtin_expected = expected.replace(
        &format!("((size_t)&((struct {r} *)0)->{link})"),
        &format!("__builtin_offsetof(struct {r},{link})"),
    );
    if tokens(actual) != tokens(&expected) && tokens(actual) != tokens(&builtin_expected) {
        return Err("complete selector body is outside the checked first-match expansion".into());
    }
    Ok(SelectionImport {
        query: query.clone(),
        source_sha256: qcore_schemas::sha256_hex(unit.source.as_bytes()),
        preprocessing_sha256: qcore_schemas::sha256_hex(source.as_bytes()),
        configuration_sha256: digest(unit),
        compiler: crate::clang::ClangIdentity::capture()?,
        body_sha256: qcore_schemas::sha256_hex(actual.as_bytes()),
        key_parameter,
        owner_parameter,
    })
}

fn strip(mut n: &Value) -> &Value {
    while matches!(kind(n), "ImplicitCastExpr" | "ParenExpr") {
        let Some(child) = children(n).first() else {
            break;
        };
        n = child;
    }
    n
}
fn reference(n: &Value, id: &str) -> bool {
    let n = strip(n);
    kind(n) == "DeclRefExpr" && n["referencedDecl"]["id"].as_str() == Some(id)
}
fn scalar_value(n: &Value, params: &[&Value]) -> bool {
    let n = strip(n);
    match kind(n) {
        "IntegerLiteral" | "GNUNullExpr" => true,
        "DeclRefExpr" => params.iter().any(|p| p["id"] == n["referencedDecl"]["id"]),
        "CStyleCastExpr" => children(n).len() == 1 && scalar_value(&children(n)[0], params),
        "UnaryOperator" if matches!(n["opcode"].as_str(), Some("-" | "+")) => {
            children(n).len() == 1 && scalar_value(&children(n)[0], params)
        }
        _ => false,
    }
}
// A closed subobject frame: every statement is a direct write inside the
// supplied struct or a bounded internal call of the same checked class.
// Neither a borrowed contract nor a function's spelling supplies this frame.
fn subobject_frame(ast: &Value, function: &Value, depth: usize) -> bool {
    if depth >= 16 || function["storageClass"].as_str() != Some("static") {
        return false;
    }
    let params = parameters(function);
    let Some(first) = params.first() else {
        return false;
    };
    let Some(id) = first["id"].as_str() else {
        return false;
    };
    let Ok(b) = body(function) else { return false };
    if !ty(function).starts_with("void (") {
        return false;
    }
    children(b).iter().all(|statement| {
        if kind(statement) == "BinaryOperator"
            && statement["opcode"].as_str() == Some("=")
            && children(statement).len() == 2
        {
            let lhs = strip(&children(statement)[0]);
            return kind(lhs) == "MemberExpr"
                && lhs["isArrow"].as_bool() == Some(true)
                && children(lhs).first().is_some_and(|n| reference(n, id))
                && scalar_value(&children(statement)[1], &params);
        }
        if kind(statement) == "CallExpr" {
            let args = children(statement);
            let Some(callee) = args.first().map(strip) else {
                return false;
            };
            let Some(callee_name) = callee["referencedDecl"]["name"].as_str() else {
                return false;
            };
            let Ok(called) = definition(ast, callee_name) else {
                return false;
            };
            let called_params = parameters(called);
            return kind(callee) == "DeclRefExpr"
                && callee["referencedDecl"]["kind"].as_str() == Some("FunctionDecl")
                && called_params.len() + 1 == args.len()
                && called_params.first().is_some_and(|p| ty(p) == ty(first))
                && args.get(1).is_some_and(|n| reference(n, id))
                && args.iter().skip(2).all(|n| scalar_value(n, &params))
                && subobject_frame(ast, called, depth + 1);
        }
        false
    })
}
fn preserves_outer(ast: &Value, statement: &Value, coll: &Value, record: &str) -> bool {
    if kind(statement) != "CallExpr" || children(statement).len() != 2 {
        return false;
    }
    let args = children(statement);
    let callee = strip(&args[0]);
    let actual = strip(&args[1]);
    let Some(callee_name) = callee["referencedDecl"]["name"].as_str() else {
        return false;
    };
    if kind(callee) != "DeclRefExpr"
        || callee["referencedDecl"]["kind"].as_str() != Some("FunctionDecl")
        || kind(actual) != "UnaryOperator"
        || actual["opcode"].as_str() != Some("&")
    {
        return false;
    }
    let Some(member) = children(actual).first().map(strip) else {
        return false;
    };
    let Some(id) = coll["id"].as_str() else {
        return false;
    };
    let Some(member_name) = member["name"].as_str() else {
        return false;
    };
    if kind(member) != "MemberExpr"
        || member["isArrow"].as_bool() != Some(true)
        || !children(member).first().is_some_and(|n| reference(n, id))
        || !ty(member).starts_with("struct ")
        || ty(member).contains('*')
        || field(ast, record, member_name, ty(member)).is_err()
    {
        return false;
    }
    let Ok(called) = definition(ast, callee_name) else {
        return false;
    };
    let params = parameters(called);
    params.len() == 1
        && ty(params[0]) == format!("{} *", ty(member))
        && subobject_frame(ast, called, 0)
}

/// Read just the relation established by a constructor's instance
/// store and two-arm loop store, allowing checked subobject-only helper frames
/// between them. Other constructor effects are not validated.
/// A false arm rooted in the config formal stays that formal; it cannot silently
/// become owner->config. Enduring field/flag stability must be proved elsewhere.
pub fn import_conditional_loop(
    unit: &C11TranslationUnit<'_>,
    query: &LoopQuery,
) -> Result<ConditionalLoopImport, String> {
    let (source, ast) = prepared(unit)?;
    let function = definition(&ast, &query.function)?;
    let predicate = definition(&ast, &query.predicate)?;
    if function["storageClass"].as_str() != Some("static")
        || predicate["storageClass"].as_str() != Some("static")
    {
        return Err("constructor and predicate need internally linked definitions".into());
    }
    let params = parameters(function);
    let (owner_parameter, owner) =
        unique_typed(&params, &format!("struct {} *", query.owner_record))?;
    let (config_parameter, config) =
        unique_typed(&params, &format!("struct {} *", query.config_record))?;
    let predicate_params = parameters(predicate);
    let [predicate_owner] = predicate_params.as_slice() else {
        return Err("predicate needs one parameter".into());
    };
    if ty(predicate_owner) != ty(owner)
        || ty(predicate) != format!("int (struct {} *)", query.owner_record)
    {
        return Err("predicate has an unsupported exact signature".into());
    }
    let g = name(predicate_owner)?;
    if tokens(text(&source, body(predicate)?)?)
        != tokens(&format!("{{return {g}->{};}}", query.threaded_member))
    {
        return Err("predicate is not the exact integer member getter".into());
    }
    field(&ast, &query.owner_record, &query.threaded_member, "int")?;
    field(
        &ast,
        &query.record,
        &query.instance_member,
        &format!("struct {} *", query.owner_record),
    )?;
    field(
        &ast,
        &query.owner_record,
        &query.worker_member,
        &format!("struct {} *", query.worker_record),
    )?;
    for record in [&query.record, &query.worker_record, &query.config_record] {
        field(
            &ast,
            record,
            &query.loop_member,
            &format!("struct {} *", query.loop_record),
        )?;
    }
    let b = body(function)?;
    let ls = locals(b);
    let (_, coll) = unique_typed(&ls, &format!("struct {} *", query.record))?;
    let (_, worker) = unique_typed(&ls, &format!("struct {} *", query.worker_record))?;
    let coll_decl = coll;
    let binding_declarations = [(owner, 0), (config, 0), (worker, 1)];
    let (owner, config, coll, worker) = (name(owner)?, name(config)?, name(coll)?, name(worker)?);
    let instance = &query.instance_member;
    let worker_member = &query.worker_member;
    let evl = &query.loop_member;
    let expected_store = tokens(&format!("{coll}->{instance}={owner}"));
    let expected_if = tokens(&format!(
        "if ({}({owner})) {{ {worker}={owner}->{worker_member}; {coll}->{evl}={worker}->{evl}; }} else {{ {coll}->{evl}={config}->{evl}; }}",
        query.predicate
    ));
    let statements = children(b);
    let mut pairs = 0usize;
    for (end, statement) in statements.iter().enumerate() {
        if !text(&source, statement).is_ok_and(|s| tokens(s) == expected_if) {
            continue;
        }
        let Some(condition) = children(statement).first().map(strip) else {
            continue;
        };
        let Some(callee) = children(condition).first().map(strip) else {
            continue;
        };
        if kind(condition) != "CallExpr"
            || kind(callee) != "DeclRefExpr"
            || callee["referencedDecl"]["kind"].as_str() != Some("FunctionDecl")
            || callee["referencedDecl"]["name"].as_str() != Some(query.predicate.as_str())
        {
            continue;
        }
        for start in (0..end).rev() {
            if text(&source, &statements[start]).is_ok_and(|s| tokens(s) == expected_store) {
                pairs += 1;
                break;
            }
            if !preserves_outer(&ast, &statements[start], coll_decl, &query.record) {
                break;
            }
        }
    }
    if pairs != 1 {
        return Err(
            "missing or ambiguous instance/conditional store with a closed intervening frame"
                .into(),
        );
    }
    // This is a snapshot at the conditional's join, not a whole-body invariant.
    // Local/formal rebinding anywhere invalidates spelling-based substitution.
    for (declaration, writes) in binding_declarations {
        if !stable_binding(b, declaration, writes) {
            return Err(
                "formal or worker rebinding/escape prevents exact instance identity".into(),
            );
        }
    }
    Ok(ConditionalLoopImport {
        query: query.clone(),
        source_sha256: qcore_schemas::sha256_hex(unit.source.as_bytes()),
        preprocessing_sha256: qcore_schemas::sha256_hex(source.as_bytes()),
        configuration_sha256: digest(unit),
        compiler: crate::clang::ClangIdentity::capture()?,
        constructor_sha256: qcore_schemas::sha256_hex(text(&source, function)?.as_bytes()),
        predicate_sha256: qcore_schemas::sha256_hex(text(&source, predicate)?.as_bytes()),
        owner_parameter,
        config_parameter,
        record_local: coll.into(),
    })
}

/// Independent replay requires the retained original source/configuration and
/// the complete expected import. A serialized report cannot create authority.
pub fn replay_selection(
    unit: &C11TranslationUnit<'_>,
    expected: &SelectionImport,
) -> Result<(), String> {
    (import_selection(unit, &expected.query)? == *expected)
        .then_some(())
        .ok_or_else(|| "selection import differs on replay".into())
}
pub fn replay_conditional_loop(
    unit: &C11TranslationUnit<'_>,
    expected: &ConditionalLoopImport,
) -> Result<(), String> {
    (import_conditional_loop(unit, &expected.query)? == *expected)
        .then_some(())
        .ok_or_else(|| "conditional loop import differs on replay".into())
}

#[cfg(test)]
mod stack_tests {
    use super::*;
    use crate::deep_ast_transports::stack::high_water;
    use crate::deep_unit_ast::{ADMITTED_DEPTH, depth};
    use crate::legacy_c::CCompilerDialect;
    const FIXTURE: &str = include_str!("../tests/fixtures/collector_identity.c");
    fn unit(source: &str) -> C11TranslationUnit<'_> {
        C11TranslationUnit {
            source,
            dialect: CCompilerDialect::Gnu17,
            ..Default::default()
        }
    }
    fn selector() -> SelectorQuery {
        SelectorQuery {
            function: "get_collector".into(),
            record: "collector".into(),
            owner_record: "instance".into(),
            list_record: "mk_list".into(),
            list_member: "collectors".into(),
            link_member: "_head".into(),
            key_member: "id".into(),
        }
    }
    fn loops() -> LoopQuery {
        LoopQuery {
            function: "create".into(),
            predicate: "threaded".into(),
            record: "collector".into(),
            owner_record: "instance".into(),
            worker_record: "worker".into(),
            config_record: "config".into(),
            loop_record: "loop".into(),
            instance_member: "instance".into(),
            threaded_member: "is_threaded".into(),
            worker_member: "thi".into(),
            loop_member: "evl".into(),
        }
    }
    fn source(arms: usize, helper: usize) -> String {
        let mut helpers = String::from(
            "struct event { int fd; }; static void frame0(struct event *e) { e->fd=0; }\n",
        );
        for index in 1..=16 {
            helpers.push_str(&format!(
                "static void frame{index}(struct event *e) {{ frame{}(e); }}\n",
                index - 1
            ));
        }
        let base = FIXTURE
            .replace(
                "struct collector {",
                &format!("{helpers} struct collector {{ struct event event;"),
            )
            .replace(
                "coll->instance=ins;",
                &format!("coll->instance=ins; frame{helper}(&coll->event);"),
            );
        let mut chain = String::from("int deep_chain(int x) { if (x==0) return 0;\n");
        for arm in 1..arms {
            chain.push_str(&format!("else if (x=={arm}) return {arm};\n"));
        }
        chain.push_str("return -1; }\n");
        base + &chain
    }
    fn source_depth(source: &str) -> usize {
        crate::typed_c_ast::translation_unit_ast_read(source, &unit(source), |reader| {
            let mut bytes = Vec::new();
            std::io::Read::read_to_end(reader, &mut bytes).map_err(serde_json::Error::io)?;
            Ok::<_, serde_json::Error>(depth(&bytes))
        })
        .unwrap()
    }
    #[test]
    fn complete_collector_readers_fit_the_existing_ast_stack_budget() {
        // Pin the actual JSON depth of this Clang's whole-unit document, not
        // an assumed relation between C nesting and JSON nesting.
        let mut arms = 100;
        while source_depth(&source(arms + 1, 15)) <= ADMITTED_DEPTH {
            arms += 1;
        }
        let deep = source(arms, 15);
        let observed = source_depth(&deep);
        assert!(observed > 128 && observed <= ADMITTED_DEPTH);
        assert!(source_depth(&source(arms + 3, 15)) > ADMITTED_DEPTH);
        // Also put the deep expression on the reader's recursive semantic
        // path, at the bottom of the maximum admitted helper chain.
        let value_source =
            |n: usize| source(1, 15).replace("e->fd=0", &format!("e->fd={}0", "+ ".repeat(n)));
        let mut unary = 100;
        while source_depth(&value_source(unary + 1)) <= ADMITTED_DEPTH {
            unary += 1;
        }
        let deep_value = value_source(unary);
        let value_depth = source_depth(&deep_value);
        assert!(value_depth > 128 && value_depth <= ADMITTED_DEPTH);
        let over_value = value_source(unary + 1);
        assert!(source_depth(&over_value) > ADMITTED_DEPTH);
        let ((selected, conditional, value_selected, value_conditional), kib) =
            high_water(move || {
                (
                    import_selection(&unit(&deep), &selector()),
                    import_conditional_loop(&unit(&deep), &loops()),
                    import_selection(&unit(&deep_value), &selector()),
                    import_conditional_loop(&unit(&deep_value), &loops()),
                )
            });
        selected.unwrap();
        conditional.unwrap();
        value_selected.unwrap();
        value_conditional.unwrap();
        #[cfg(target_os = "linux")]
        {
            let measured = kib.expect("Linux stack high-water mark must be available");
            eprintln!(
                "collector readers: JSON depths {observed}/{value_depth}, helper depth 15, stack {measured} KiB"
            );
            assert!(
                measured <= 1024,
                "collector readers exceed the existing 1 MiB budget: {measured} KiB"
            );
        }
        #[cfg(not(target_os = "linux"))]
        {
            assert_eq!(kib, None);
            eprintln!("collector stack measurement unavailable off Linux");
        }
        assert!(import_selection(&unit(&source(arms + 3, 15)), &selector()).is_err());
        assert!(import_conditional_loop(&unit(&over_value), &loops()).is_err());
        assert!(import_conditional_loop(&unit(&source(arms, 16)), &loops()).is_err());
    }
}
