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

//! S3 source facts. Typed paths keep their original root and every operation
//! remains in the census, including operations this reader cannot resolve.
//! No path spelling, allocation-site overlap or local condition declares an
//! equality. Unsupported paths become explicit open evidence.
use crate::typed_c_ast::{children, kind};
use qcore_schemas::contract::{ResourceContracts, ResourceParameterMode};
use qcore_schemas::retained_by::RetentionHolder;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Path {
    pub root: String,
    pub record: String,
    pub fields: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperationKind {
    Register,
    Remove,
    Destroy,
    Release,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Operation {
    pub kind: OperationKind,
    pub callee: String,
    pub object: Option<Path>,
    pub holder: Option<Path>,
    pub conditional: bool,
    pub offset: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConditionFact {
    pub path: Path,
    pub value: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallSite {
    pub callee: String,
    pub actuals: Vec<Option<Path>>,
    /// Typed guard interpretation never follows integer narrowing or pointer
    /// retyping, even when may-effects retain the physical pointer identity.
    pub condition_actuals: Vec<Option<Path>>,
    /// Field interpretation stays typed; an outer conversion of an already
    /// grounded physical address to void* does not reinterpret its fields.
    pub typed_effect_actuals: Vec<Option<Path>>,
    pub conditions: Vec<ConditionFact>,
    pub pure_operands: bool,
    pub conditional: bool,
    pub offset: u64,
    pub end: u64,
    pub line: usize,
    pub tokens: Vec<String>,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionFacts {
    #[serde(default)]
    pub immutable_aliases: BTreeMap<String, Path>,
    #[serde(default)]
    pub internal_linkage: bool,
    pub tokens_sha256: String,
    pub complete_straight_line: bool,
    pub parameters: Vec<String>,
    pub operations: Vec<Operation>,
    pub stores: Vec<(Option<Path>, Option<Path>)>,
    pub calls: Vec<CallSite>,
    pub returned: Vec<Option<Path>>,
    pub open: BTreeSet<String>,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceFacts {
    pub identity: String,
    pub source_sha256: String,
    pub fields: BTreeMap<String, String>,
    pub functions: BTreeMap<String, FunctionFacts>,
    pub open: BTreeSet<String>,
}
impl SourceFacts {
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty() && self.functions.is_empty() && self.open.is_empty()
    }
}
fn ty(node: &Value) -> &str {
    node["type"]["desugaredQualType"]
        .as_str()
        .or_else(|| node["type"]["qualType"].as_str())
        .unwrap_or("")
}
fn record(text: &str) -> Option<String> {
    let words = text.split_whitespace().collect::<Vec<_>>();
    let at = words.iter().position(|w| *w == "struct")?;
    Some(words.get(at + 1)?.trim_end_matches('*').to_string())
}
fn strip(mut node: &Value) -> &Value {
    while matches!(kind(node), "ImplicitCastExpr" | "ParenExpr") {
        let Some(inner) = children(node).first() else {
            break;
        };
        node = inner;
    }
    node
}
fn direct(node: &Value) -> Option<&str> {
    let callee = strip(children(node).first()?);
    (kind(callee) == "DeclRefExpr"
        && callee["referencedDecl"]["kind"].as_str() == Some("FunctionDecl"))
    .then(|| callee["referencedDecl"]["name"].as_str())
    .flatten()
}
fn path(node: &Value, aliases: &BTreeMap<String, Option<Path>>) -> Option<Path> {
    let node = strip(node);
    match kind(node) {
        "DeclRefExpr" => {
            let id = node["referencedDecl"]["id"].as_str()?.to_owned();
            if let Some(alias) = aliases.get(&id) {
                return alias.clone();
            }
            Some(Path {
                root: id,
                record: record(ty(node))?,
                fields: Vec::new(),
            })
        }
        "MemberExpr" => {
            let mut base = path(children(node).first()?, aliases)?;
            base.fields.push(node["name"].as_str()?.into());
            Some(base)
        }
        "UnaryOperator" if node["opcode"].as_str() == Some("&") => {
            path(children(node).first()?, aliases)
        }
        _ => None,
    }
}
fn walk(node: &Value, action: &mut impl FnMut(&Value)) {
    action(node);
    for child in children(node) {
        walk(child, action)
    }
}
fn assignments(body: &Value) -> BTreeMap<String, Vec<Value>> {
    let mut writes: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    walk(body, &mut |n| {
        if kind(n) == "VarDecl"
            && ty(n).contains('*')
            && let Some(init) = children(n).last()
            && let Some(id) = n["id"].as_str()
        {
            writes.entry(id.into()).or_default().push(init.clone());
        }
        if kind(n) == "BinaryOperator"
            && n["opcode"].as_str() == Some("=")
            && children(n).len() == 2
        {
            let lhs = strip(&children(n)[0]);
            if kind(lhs) == "DeclRefExpr"
                && let Some(id) = lhs["referencedDecl"]["id"].as_str()
            {
                writes
                    .entry(id.into())
                    .or_default()
                    .push(children(n)[1].clone());
            }
        }
    });
    writes
}
fn function(
    node: &Value,
    contracts: &ResourceContracts,
    holder_types: &BTreeSet<String>,
    getters: &BTreeMap<String, (usize, Vec<String>)>,
    identity: &str,
) -> Option<(String, FunctionFacts)> {
    if kind(node) != "FunctionDecl" {
        return None;
    }
    let body = children(node).iter().find(|n| kind(n) == "CompoundStmt")?;
    let name = node["name"].as_str()?.to_owned();
    let parameters = children(node)
        .iter()
        .filter(|n| kind(n) == "ParmVarDecl")
        .filter_map(|n| n["id"].as_str().map(str::to_owned))
        .collect::<Vec<_>>();
    let writes = assignments(body);
    let mut aliases = BTreeMap::new();
    // Resolve only single assignments. Unresolved values retain their own
    // typed object identity, but never acquire an alias to another object.
    for _ in 0..writes.len().saturating_add(1) {
        let before = aliases.clone();
        for (id, values) in &writes {
            // A formal can be reassigned after an earlier call. A whole-body
            // singleton store is not a dominance proof for that earlier use.
            if parameters.contains(id) {
                continue;
            }
            if values.len() != 1 {
                aliases.insert(id.clone(), None);
                continue;
            }
            if let Some(p) = path(&values[0], &aliases)
                && p.root != *id
            {
                aliases.insert(id.clone(), Some(p));
            }
        }
        if aliases == before {
            break;
        }
    }
    // A may-effect alias may preserve an erased physical pointer. A typed
    // condition needs its own stricter alias closure, so an earlier void*/
    // incompatible-pointer bridge cannot be hidden behind a later plain name.
    let mut initialized = BTreeSet::new();
    let mut addressed = BTreeSet::new();
    walk(body, &mut |node| {
        if kind(node) == "VarDecl"
            && ty(node).contains('*')
            && !children(node).is_empty()
            && let Some(id) = node["id"].as_str()
        {
            initialized.insert(id.to_owned());
        }
        if kind(node) == "UnaryOperator"
            && node["opcode"].as_str() == Some("&")
            && let Some(value) = children(node).first().map(strip)
            && kind(value) == "DeclRefExpr"
            && let Some(id) = value["referencedDecl"]["id"].as_str()
        {
            addressed.insert(id.to_owned());
        }
    });
    let mut condition_aliases = BTreeMap::new();
    for _ in 0..writes.len().saturating_add(1) {
        let before = condition_aliases.clone();
        for (id, values) in &writes {
            if parameters.contains(id) {
                continue;
            }
            if values.len() != 1
                || !initialized.contains(id)
                || addressed.contains(id)
                || !condition_casts_preserve(&values[0])
            {
                condition_aliases.insert(id.clone(), None);
            } else if let Some(p) = path(&values[0], &condition_aliases)
                && p.root != *id
            {
                condition_aliases.insert(id.clone(), Some(p));
            }
        }
        if before == condition_aliases {
            break;
        }
    }
    let mut complete_straight_line = true;
    walk(body, &mut |n| {
        if matches!(
            kind(n),
            "IfStmt"
                | "ForStmt"
                | "WhileStmt"
                | "DoStmt"
                | "SwitchStmt"
                | "ConditionalOperator"
                | "GotoStmt"
                | "IndirectGotoStmt"
                | "LabelStmt"
                | "CaseStmt"
                | "DefaultStmt"
                | "GCCAsmStmt"
                | "MSAsmStmt"
        ) {
            complete_straight_line = false;
        }
    });
    let direct_returns = children(body)
        .iter()
        .enumerate()
        .filter(|(_, n)| kind(n) == "ReturnStmt")
        .collect::<Vec<_>>();
    if direct_returns
        .iter()
        .any(|(i, _)| i + 1 != children(body).len())
    {
        complete_straight_line = false;
    }
    let mut out = FunctionFacts {
        internal_linkage: node["storageClass"].as_str() == Some("static")
            && !name.starts_with("__")
            && !name
                .strip_prefix('_')
                .is_some_and(|s| s.starts_with(|c: char| c.is_ascii_uppercase())),
        parameters,
        complete_straight_line,
        ..Default::default()
    };
    if writes.keys().any(|id| out.parameters.contains(id)) {
        out.open
            .insert("formal reassignment prevents entry snapshot correspondence".into());
    }
    fn visit(
        node: &Value,
        conditional: bool,
        aliases: &BTreeMap<String, Option<Path>>,
        condition_aliases: &BTreeMap<String, Option<Path>>,
        contracts: &ResourceContracts,
        holder_types: &BTreeSet<String>,
        out: &mut FunctionFacts,
    ) {
        let conditional = conditional
            || matches!(
                kind(node),
                "IfStmt"
                    | "ForStmt"
                    | "WhileStmt"
                    | "DoStmt"
                    | "SwitchStmt"
                    | "ConditionalOperator"
            );
        if kind(node) == "BinaryOperator"
            && node["opcode"].as_str() == Some("=")
            && children(node).len() == 2
        {
            let lhs = strip(&children(node)[0]);
            if kind(lhs) == "MemberExpr" {
                out.stores
                    .push((path(lhs, aliases), path(&children(node)[1], aliases)));
            }
        }
        if kind(node) == "ReturnStmt" {
            out.returned
                .push(children(node).first().and_then(|n| path(n, aliases)));
        }
        if kind(node) == "CallExpr" {
            if let Some(name) = direct(node) {
                let args = children(node)
                    .iter()
                    .skip(1)
                    .map(|n| path(n, aliases))
                    .collect::<Vec<_>>();
                let arg = |i: usize| args.get(i).cloned().flatten();
                let typed_args = children(node)
                    .iter()
                    .skip(1)
                    .map(|arg| typed_effect_path(arg, condition_aliases))
                    .collect::<Vec<_>>();
                let typed_arg = |i: usize| typed_args.get(i).cloned().flatten();
                let offset = node["range"]["begin"]["offset"].as_u64().unwrap_or(0);
                let mut classified = false;
                if let Some(declared) = contracts.retaining_contract(name) {
                    for retained in &declared.retentions {
                        if let RetentionHolder::Parameter { index } = retained.holder {
                            out.operations.push(Operation {
                                kind: OperationKind::Register,
                                callee: name.into(),
                                object: arg(retained.retained),
                                holder: arg(index),
                                conditional,
                                offset,
                            });
                            classified = true;
                        }
                    }
                    for removed in &declared.removals {
                        out.operations.push(Operation {
                            kind: OperationKind::Remove,
                            callee: name.into(),
                            object: typed_arg(removed.retained),
                            holder: typed_arg(removed.holder),
                            conditional,
                            offset,
                        });
                        classified = true;
                    }
                }
                if let Some(declared) = contracts.contract(name) {
                    for (index, mode) in declared.parameters.iter().enumerate() {
                        if matches!(mode, ResourceParameterMode::Released { .. }) {
                            let value = arg(index);
                            let destroy =
                                record(ty(strip(children(node).get(index + 1).unwrap_or(node))))
                                    .is_some_and(|r| holder_types.contains(&r))
                                    && value.is_some()
                                    && typed_arg(index) == value;
                            out.operations.push(Operation {
                                kind: if destroy {
                                    OperationKind::Destroy
                                } else {
                                    OperationKind::Release
                                },
                                callee: name.into(),
                                object: if destroy { None } else { value.clone() },
                                holder: if destroy { value } else { None },
                                conditional,
                                offset,
                            });
                            classified = true;
                        }
                    }
                }
                out.calls.push(CallSite {
                    callee: name.into(),
                    condition_actuals: children(node)
                        .iter()
                        .skip(1)
                        .map(|arg| {
                            condition_casts_preserve(arg)
                                .then(|| path(arg, condition_aliases))
                                .flatten()
                        })
                        .collect(),
                    typed_effect_actuals: typed_args,
                    actuals: args,
                    conditions: Vec::new(),
                    pure_operands: children(node).iter().all(pure_call_expression),
                    conditional,
                    offset,
                    end: node["range"]["end"]["offset"]
                        .as_u64()
                        .zip(node["range"]["end"]["tokLen"].as_u64())
                        .and_then(|(x, y)| x.checked_add(y))
                        .unwrap_or(0),
                    line: 0,
                    tokens: Vec::new(),
                });
                // The call remains in the graph even when not a primitive.
                // Its summary must be available before it may decide anything.
                let _ = classified;
            } else {
                out.open
                    .insert("indirect call has no lifetime summary".into());
            }
        }
        if matches!(
            kind(node),
            "CStyleCastExpr" | "CompoundAssignOperator" | "GCCAsmStmt" | "MSAsmStmt"
        ) || (kind(node) == "UnaryOperator"
            && matches!(node["opcode"].as_str(), Some("++" | "--")))
        {
            out.open
                .insert(format!("unresolved {} in lifetime body", kind(node)));
        }
        for child in children(node) {
            visit(
                child,
                conditional,
                aliases,
                condition_aliases,
                contracts,
                holder_types,
                out,
            )
        }
    }
    visit(
        body,
        false,
        &aliases,
        &condition_aliases,
        contracts,
        holder_types,
        &mut out,
    );
    // Clang declaration ids are process addresses. Canonical roots use only
    // source coordinates and positions, never unstable AST pointer spellings.
    let mut roots = out
        .parameters
        .iter()
        .enumerate()
        .map(|(i, id)| (id.clone(), format!("{identity}:{name}#formal:{i}")))
        .collect::<BTreeMap<_, _>>();
    walk(body, &mut |n| {
        if kind(n) == "VarDecl"
            && let (Some(id), Some(offset)) = (n["id"].as_str(), n["loc"]["offset"].as_u64())
        {
            roots.insert(
                id.into(),
                format!(
                    "{identity}:{name}#local:{}@{offset}",
                    n["name"].as_str().unwrap_or("?")
                ),
            );
        }
    });
    let canonical = |value: &mut Option<Path>| {
        if let Some(path) = value {
            if let Some(root) = roots.get(&path.root) {
                path.root = root.clone();
            } else {
                *value = None;
            }
        }
    };
    // Only initialized, unaddressed, single-write locals with type-preserving
    // initializers receive an exact identity. No id-search or type-wide alias.
    if out.complete_straight_line {
        let mut names = BTreeMap::<String, usize>::new();
        for parameter in children(node).iter().filter(|p| kind(p) == "ParmVarDecl") {
            if let Some(name) = parameter["name"].as_str() {
                *names.entry(name.into()).or_default() += 1;
            }
        }
        walk(body, &mut |n| {
            if kind(n) == "VarDecl"
                && let Some(name) = n["name"].as_str()
            {
                *names.entry(name.into()).or_default() += 1;
            }
        });
        walk(body, &mut |n| {
            if kind(n) == "VarDecl"
                && let (Some(id), Some(name)) = (n["id"].as_str(), n["name"].as_str())
                && names.get(name) == Some(&1)
                && !ty(n).contains("volatile")
                && !ty(n).contains("_Atomic")
                && let Some(Some(alias)) = condition_aliases.get(id)
                && writes.get(id).is_some_and(|values| {
                    let mut address = false;
                    walk(&values[0], &mut |n| {
                        address |= kind(n) == "UnaryOperator" && n["opcode"].as_str() == Some("&");
                    });
                    !address
                })
            {
                let mut value = Some(alias.clone());
                canonical(&mut value);
                if let Some(path) = value {
                    out.immutable_aliases.insert(name.into(), path);
                }
            }
        });
    }
    for op in &mut out.operations {
        canonical(&mut op.object);
        canonical(&mut op.holder);
    }
    for (lhs, rhs) in &mut out.stores {
        canonical(lhs);
        canonical(rhs);
    }
    let evidence = condition_calls(body, &condition_aliases, getters);
    for call in &mut out.calls {
        for actual in &mut call.actuals {
            canonical(actual);
        }
        for actual in &mut call.condition_actuals {
            canonical(actual);
        }
        for actual in &mut call.typed_effect_actuals {
            canonical(actual);
        }
        call.conditions = evidence
            .get(&call.offset)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|mut fact| {
                let mut value = Some(fact.path);
                canonical(&mut value);
                fact.path = value?;
                Some(fact)
            })
            .collect();
    }
    for value in &mut out.returned {
        canonical(value);
    }
    out.parameters = out
        .parameters
        .into_iter()
        .filter_map(|id| roots.get(&id).cloned())
        .collect();
    Some((name, out))
}
/// Read typed source facts under the exact analyzing translation-unit command.
pub fn read_unit(
    source: &str,
    unit: &crate::legacy_c::C11TranslationUnit<'_>,
    contracts: &ResourceContracts,
) -> Result<SourceFacts, String> {
    crate::typed_c_ast::translation_unit_ast_read(source, unit, |reader| {
        let ast = crate::deep_unit_ast::from_reader::<Value>(reader)?;
        // Recursive source walkers run only after an iterative resource-bound
        // check. Truncation is never treated as an empty/closed census.
        let mut pending = vec![(&ast, 0usize)];
        let mut visited = 0usize;
        while let Some((node, depth)) = pending.pop() {
            visited += 1;
            if depth > 512 || visited > 2_000_000 {
                return Err(serde_json::Error::io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "lifetime-source-bound: AST exceeds depth/node budget",
                )));
            }
            pending.extend(children(node).iter().map(|child| (child, depth + 1)));
        }
        let mut descriptor = format!(
            "{:?}\0{:?}\0{:?}\0{:?}\0{:?}\0",
            unit.input,
            unit.dialect,
            unit.include_paths,
            unit.definitions,
            unit.preprocessor_options
        )
        .into_bytes();
        descriptor.extend_from_slice(source.as_bytes());
        let identity = qcore_schemas::sha256_hex(&descriptor);
        let mut out = SourceFacts {
            identity: identity.clone(),
            source_sha256: qcore_schemas::sha256_hex(source.as_bytes()),
            ..Default::default()
        };
        // Tag spelling alone cannot distinguish block-scoped declarations.
        // Refuse this bounded reader's typed interpretations when multiple
        // complete declarations share a tag, even if field types coincide.
        let mut definitions = BTreeMap::<String, usize>::new();
        walk(&ast, &mut |node| {
            if kind(node) == "RecordDecl"
                && node["completeDefinition"].as_bool() == Some(true)
                && let Some(tag) = node["name"].as_str()
            {
                *definitions.entry(tag.into()).or_default() += 1;
            }
        });
        for (tag, count) in definitions {
            if count > 1 {
                out.open
                    .insert(format!("ambiguous scoped record declaration {tag}"));
            }
        }
        let mut holder_types = BTreeSet::new();
        for node in children(&ast).iter().filter(|n| kind(n) == "FunctionDecl") {
            if let Some(declared) = node["name"]
                .as_str()
                .and_then(|name| contracts.retaining_contract(name))
            {
                let parameters = children(node)
                    .iter()
                    .filter(|n| kind(n) == "ParmVarDecl")
                    .collect::<Vec<_>>();
                for clause in &declared.retentions {
                    if let RetentionHolder::Parameter { index } = clause.holder
                        && let Some(tag) = parameters.get(index).and_then(|p| record(ty(p)))
                    {
                        holder_types.insert(tag);
                    }
                }
            }
        }
        let getters = getter_definitions(&ast);
        for node in children(&ast) {
            if kind(node) == "RecordDecl"
                && let Some(tag) = node["name"].as_str()
            {
                for field in children(node).iter().filter(|f| kind(f) == "FieldDecl") {
                    if let Some(name) = field["name"].as_str() {
                        let key = format!("{tag}.{name}");
                        let value = ty(field).to_owned();
                        if out
                            .fields
                            .insert(key.clone(), value.clone())
                            .is_some_and(|old| old != value)
                        {
                            out.open.insert(format!("conflicting field type {key}"));
                        }
                    }
                }
            }
            if let Some((name, mut facts)) =
                function(node, contracts, &holder_types, &getters, &identity)
            {
                let span = node["range"]["begin"]["offset"].as_u64().zip(
                    node["range"]["end"]["offset"]
                        .as_u64()
                        .zip(node["range"]["end"]["tokLen"].as_u64()),
                );
                if let Some((start, (end, len))) = span
                    && let Some(text) = end
                        .checked_add(len)
                        .and_then(|end| source.get(start as usize..end as usize))
                {
                    facts.tokens_sha256 = tokens_digest(
                        &crate::legacy_c::tokenize(text)
                            .into_iter()
                            .map(|t| t.text)
                            .collect::<Vec<_>>(),
                    );
                }
                if facts.tokens_sha256.is_empty() {
                    facts
                        .open
                        .insert("source body has no bounded token identity".into());
                }
                for call in &mut facts.calls {
                    if let Some(text) = source.get(call.offset as usize..call.end as usize) {
                        call.tokens = crate::legacy_c::tokenize(text)
                            .into_iter()
                            .map(|t| t.text)
                            .collect();
                        call.line = source
                            .as_bytes()
                            .get(..call.offset as usize)
                            .map(|s| s.iter().filter(|c| **c == b'\n').count() + 1)
                            .unwrap_or(0);
                    }
                    if call.tokens.is_empty() || call.line == 0 {
                        facts
                            .open
                            .insert("source call has no bounded token identity".into());
                    }
                }
                if out
                    .functions
                    .insert(name.clone(), facts.clone())
                    .is_some_and(|old| old != facts)
                {
                    out.open.insert(format!("conflicting function body {name}"));
                }
            }
        }
        Ok(out)
    })
}

fn getter_definitions(ast: &Value) -> BTreeMap<String, (usize, Vec<String>)> {
    let mut getters = BTreeMap::new();
    for node in children(ast).iter().filter(|n| kind(n) == "FunctionDecl") {
        // A read of this exact body is not a proof about an interposable
        // external symbol. The initial getter route is static source only.
        if node["storageClass"].as_str() != Some("static") {
            continue;
        }
        let Some(body) = children(node).iter().find(|n| kind(n) == "CompoundStmt") else {
            continue;
        };
        let [returned] = children(body) else { continue };
        if kind(returned) != "ReturnStmt" {
            continue;
        }
        let [value] = children(returned) else {
            continue;
        };
        if !condition_casts_preserve(value) || !integer_value_type(value) {
            continue;
        }
        let Some(value) = path(value, &BTreeMap::new()) else {
            continue;
        };
        if value.fields.is_empty() {
            continue;
        }
        let formals = children(node)
            .iter()
            .filter(|n| kind(n) == "ParmVarDecl")
            .collect::<Vec<_>>();
        if let Some(index) = formals
            .iter()
            .position(|p| p["id"].as_str() == Some(value.root.as_str()))
            && let Some(name) = node["name"].as_str()
        {
            getters.insert(name.into(), (index, value.fields));
        }
    }
    getters
}
/// Strict zero/nonzero and typed-object correspondence. A may-effect can
/// survive void* erasure, but a guard cannot interpret fields through it.
fn integer_value_type(node: &Value) -> bool {
    let words = ty(node).split_whitespace().collect::<Vec<_>>();
    !words.is_empty()
        && words.iter().all(|word| {
            matches!(
                *word,
                "const" | "signed" | "unsigned" | "char" | "short" | "int" | "long" | "_Bool"
            )
        })
}
fn condition_casts_preserve(node: &Value) -> bool {
    if ty(node).contains("volatile") || ty(node).contains("_Atomic") {
        return false;
    }
    match kind(node) {
        "CStyleCastExpr" => false,
        "ImplicitCastExpr"
            if !matches!(
                node["castKind"].as_str(),
                Some("LValueToRValue" | "NoOp" | "FunctionToPointerDecay")
            ) =>
        {
            false
        }
        _ => children(node).iter().all(condition_casts_preserve),
    }
}
fn typed_effect_path(mut node: &Value, aliases: &BTreeMap<String, Option<Path>>) -> Option<Path> {
    // A C API may erase an address to void*. This outer conversion preserves
    // the physical address, but no void/retyping alias can supply field layout.
    while kind(node) == "ParenExpr" {
        node = children(node).first()?;
    }
    if kind(node) == "ImplicitCastExpr"
        && node["castKind"].as_str() == Some("BitCast")
        && ty(node).split_whitespace().collect::<String>() == "void*"
    {
        node = children(node).first()?;
    }
    condition_casts_preserve(node)
        .then(|| path(node, aliases))
        .flatten()
}
fn condition(
    node: &Value,
    aliases: &BTreeMap<String, Option<Path>>,
    getters: &BTreeMap<String, (usize, Vec<String>)>,
) -> Option<ConditionFact> {
    if !condition_casts_preserve(node) || !integer_value_type(node) {
        return None;
    }
    let node = strip(node);
    if kind(node) == "UnaryOperator" && node["opcode"].as_str() == Some("!") {
        let mut c = condition(children(node).first()?, aliases, getters)?;
        c.value = !c.value;
        return Some(c);
    }
    if kind(node) == "BinaryOperator" && matches!(node["opcode"].as_str(), Some("==" | "!=")) {
        let [left, right] = children(node) else {
            return None;
        };
        let value = if kind(strip(right)) == "IntegerLiteral"
            && strip(right)["value"].as_str() == Some("0")
        {
            left
        } else if kind(strip(left)) == "IntegerLiteral"
            && strip(left)["value"].as_str() == Some("0")
        {
            right
        } else {
            return None;
        };
        let mut c = condition(value, aliases, getters)?;
        if node["opcode"].as_str() == Some("==") {
            c.value = !c.value
        }
        return Some(c);
    }
    let value = if kind(node) == "CallExpr" {
        // The checked getter body is pure, but C evaluates its actuals first.
        // Refuse the whole condition when any actual or designator can mutate
        // a fact retained from an enclosing branch.
        if !children(node).iter().all(pure_call_expression) {
            return None;
        }
        let (index, fields) = getters.get(direct(node)?)?;
        let mut actual = path(children(node).get(index + 1)?, aliases)?;
        actual.fields.extend(fields.iter().cloned());
        actual
    } else {
        path(node, aliases)?
    };
    (!value.fields.is_empty()).then_some(ConditionFact {
        path: value,
        value: true,
    })
}
/// Ancestor branch evidence is local. Every intervening call or assignment
/// invalidates it; no purity is inferred from a callee's borrowed parameters.
fn pure_call_expression(node: &Value) -> bool {
    match kind(node) {
        "DeclRefExpr" | "IntegerLiteral" | "FloatingLiteral" | "StringLiteral"
        | "CharacterLiteral" | "GNUNullExpr" => true,
        "ImplicitCastExpr"
        | "CStyleCastExpr"
        | "ParenExpr"
        | "MemberExpr"
        | "ArraySubscriptExpr"
        | "ConditionalOperator" => children(node).iter().all(pure_call_expression),
        "UnaryOperator"
            if matches!(
                node["opcode"].as_str(),
                Some("&" | "*" | "+" | "-" | "~" | "!")
            ) =>
        {
            children(node).iter().all(pure_call_expression)
        }
        "BinaryOperator" if !matches!(node["opcode"].as_str(), Some("=")) => {
            children(node).iter().all(pure_call_expression)
        }
        _ => false,
    }
}
fn condition_calls(
    body: &Value,
    aliases: &BTreeMap<String, Option<Path>>,
    getters: &BTreeMap<String, (usize, Vec<String>)>,
) -> BTreeMap<u64, Vec<ConditionFact>> {
    fn visit(
        node: &Value,
        known: Vec<ConditionFact>,
        aliases: &BTreeMap<String, Option<Path>>,
        getters: &BTreeMap<String, (usize, Vec<String>)>,
        out: &mut BTreeMap<u64, Vec<ConditionFact>>,
    ) -> Vec<ConditionFact> {
        match kind(node) {
            "CompoundStmt" => {
                let mut current = known;
                for child in children(node) {
                    current = visit(child, current, aliases, getters, out)
                }
                current
            }
            "IfStmt" => {
                let parts = children(node);
                let expected = if node["hasElse"].as_bool() == Some(true) {
                    3
                } else {
                    2
                };
                if parts.len() != expected {
                    for child in parts {
                        visit(child, Vec::new(), aliases, getters, out);
                    }
                    return Vec::new();
                }
                let Some(fact) = condition(&parts[0], aliases, getters) else {
                    for child in parts {
                        visit(child, Vec::new(), aliases, getters, out);
                    }
                    return Vec::new();
                };
                let mut positive = known.clone();
                positive.push(fact.clone());
                let mut negative = known;
                negative.push(ConditionFact {
                    value: !fact.value,
                    ..fact
                });
                let positive = visit(&parts[1], positive, aliases, getters, out);
                let negative = if parts.len() == 3 {
                    visit(&parts[2], negative, aliases, getters, out)
                } else {
                    negative
                };
                positive
                    .into_iter()
                    .filter(|f| negative.contains(f))
                    .collect()
            }
            "CallExpr" => {
                // C may evaluate every argument before entering the callee;
                // an assignment in a comma expression invalidates an ancestor
                // fact just as a nested call does. Unknown evaluation order
                // never chooses the optimistic order.
                let facts = if children(node).iter().all(pure_call_expression) {
                    known
                } else {
                    Vec::new()
                };
                if let Some(offset) = node["range"]["begin"]["offset"].as_u64() {
                    out.insert(offset, facts);
                }
                for child in children(node) {
                    visit(child, Vec::new(), aliases, getters, out);
                }
                Vec::new()
            }
            "ForStmt" | "WhileStmt" | "DoStmt" | "SwitchStmt" | "ConditionalOperator" => {
                for child in children(node) {
                    visit(child, Vec::new(), aliases, getters, out);
                }
                Vec::new()
            }
            _ => {
                let mut current = known;
                for child in children(node) {
                    current = visit(child, current, aliases, getters, out)
                }
                if (kind(node) == "BinaryOperator" && node["opcode"].as_str() == Some("="))
                    || kind(node) == "CompoundAssignOperator"
                    || (kind(node) == "UnaryOperator"
                        && matches!(node["opcode"].as_str(), Some("++" | "--")))
                {
                    Vec::new()
                } else {
                    current
                }
            }
        }
    }
    let mut out = BTreeMap::new();
    let mut nonlocal = false;
    walk(body, &mut |node| {
        if matches!(
            kind(node),
            "GotoStmt"
                | "IndirectGotoStmt"
                | "LabelStmt"
                | "CaseStmt"
                | "DefaultStmt"
                | "GCCAsmStmt"
                | "MSAsmStmt"
        ) || (kind(node) == "CallExpr"
            && matches!(
                direct(node),
                Some("setjmp" | "_setjmp" | "sigsetjmp" | "longjmp" | "_longjmp" | "siglongjmp")
            ))
        {
            nonlocal = true;
        }
    });
    if nonlocal {
        return out;
    }
    visit(body, Vec::new(), aliases, getters, &mut out);
    out
}
/// Read a bound member on this exact call instance. A binding is a read of
/// an integer, not a Boolean assumption; unmatched roots and unread types
/// remain unknown. Branch facts were invalidated by intervening mutation.
pub fn condition_at(
    binding: &qcore_schemas::loop_lifetime::ConditionBinding,
    call: &CallSite,
    units: &[&SourceFacts],
) -> qcore_core::local_condition::Truth {
    use qcore_core::local_condition::Truth;
    if units.iter().any(|unit| !unit.open.is_empty()) {
        return Truth::Unknown;
    }
    let Some(Some(actual)) = call.condition_actuals.get(binding.parameter) else {
        return Truth::Unknown;
    };
    let mut target = actual.clone();
    target.fields.extend(binding.members.iter().cloned());
    let mut tag = target.record.clone();
    let mut last = None;
    for (index, field) in target.fields.iter().enumerate() {
        let key = format!("{tag}.{field}");
        let values = units
            .iter()
            .filter_map(|u| u.fields.get(&key))
            .collect::<BTreeSet<_>>();
        if values.len() != 1 {
            return Truth::Unknown;
        }
        let value = *values.first().unwrap();
        if value.contains("volatile") || value.contains("_Atomic") {
            return Truth::Unknown;
        }
        if index + 1 == target.fields.len() {
            last = Some(value.as_str());
        } else if let Some(next) = record(value) {
            tag = next
        } else {
            return Truth::Unknown;
        }
    }
    let Some(last) = last else {
        return Truth::Unknown;
    };
    if last.contains('*')
        || !last.split_whitespace().all(|w| {
            matches!(
                w,
                "const" | "signed" | "unsigned" | "char" | "short" | "int" | "long" | "_Bool"
            )
        })
    {
        return Truth::Unknown;
    }
    let values = call
        .conditions
        .iter()
        .filter(|f| f.path == target)
        .map(|f| f.value)
        .collect::<BTreeSet<_>>();
    match (values.contains(&true), values.contains(&false)) {
        (true, false) => Truth::Yes,
        (false, true) => Truth::No,
        _ => Truth::Unknown,
    }
}

pub(crate) fn tokens_digest(tokens: &[String]) -> String {
    let mut bytes = b"qcore.s3.function-tokens.v1\0".to_vec();
    for token in tokens {
        bytes.extend_from_slice(&(token.len() as u64).to_be_bytes());
        bytes.extend_from_slice(token.as_bytes());
    }
    qcore_schemas::sha256_hex(&bytes)
}
fn binding_members(
    binding: &qcore_schemas::loop_lifetime::ConditionBinding,
    call: &CallSite,
    units: &[&SourceFacts],
) -> Result<BTreeSet<String>, String> {
    if units.iter().any(|unit| !unit.open.is_empty()) {
        return Err("condition source/type declarations are open".into());
    }
    let Some(Some(actual)) = call.condition_actuals.get(binding.parameter) else {
        return Err("condition actual root is unresolved".into());
    };
    let mut tag = actual.record.clone();
    let fields = actual
        .fields
        .iter()
        .chain(&binding.members)
        .collect::<Vec<_>>();
    let mut out = BTreeSet::new();
    for (index, field) in fields.iter().enumerate() {
        let key = format!("{tag}.{field}");
        out.insert(key.clone());
        let values = units
            .iter()
            .filter_map(|u| u.fields.get(&key))
            .collect::<BTreeSet<_>>();
        if values.len() != 1 {
            return Err("condition member type is unread or inconsistent".into());
        }
        let ty = *values.first().unwrap();
        if ty.contains("volatile") || ty.contains("_Atomic") {
            return Err("condition member is volatile or atomic".into());
        }
        if index + 1 < fields.len() {
            tag = record(ty).ok_or("condition member path has an unread record type")?;
        } else if ty.contains('*')
            || !ty.split_whitespace().all(|w| {
                matches!(
                    w,
                    "const" | "signed" | "unsigned" | "char" | "short" | "int" | "long" | "_Bool"
                )
            })
        {
            return Err("condition member is not a checked integer".into());
        }
    }
    Ok(out)
}
/// A body may use its own declared conditional precondition. This entry
/// context carries no ensures/postconditions and cannot establish the condition
/// at an external caller. Production entry/call correspondence is deliberately
/// restricted to unchanged exact formal member paths before the first call.
pub struct EntryContext<'a> {
    pub lifetime: &'a qcore_schemas::loop_lifetime::LoopLifetimeContract,
    pub body: &'a FunctionFacts,
}
fn entry_discharges(
    text: &str,
    binding: &qcore_schemas::loop_lifetime::ConditionBinding,
    guard: &qcore_schemas::loop_lifetime::ConditionGuard,
    call: &CallSite,
    entry: Option<&EntryContext<'_>>,
) -> bool {
    use qcore_schemas::loop_lifetime::LocalClause;
    let Some(wanted) = qcore_contract::local_condition::named_prerequisite(text) else {
        return false;
    };
    let Some(entry) = entry else { return false };
    // Stores anywhere in the body are rejected rather than presumed to occur
    // after the call; unknown roots/aliasing cannot preserve an entry snapshot.
    if !entry.lifetime.entry_snapshot_stable
        || !call.pure_operands
        || !entry.body.open.is_empty()
        || !entry.body.stores.is_empty()
        || entry.body.calls.iter().any(|c| c.offset < call.offset)
    {
        return false;
    }
    let Some(Some(actual)) = call.condition_actuals.get(binding.parameter) else {
        return false;
    };
    let full_members = actual
        .fields
        .iter()
        .chain(&binding.members)
        .cloned()
        .collect::<Vec<_>>();
    entry.lifetime.blocks.iter().any(|block| {
        if block.guard.negated != guard.negated { return false; }
        let Some(own_binding) = entry.lifetime.bindings.iter().find(|b| b.name == block.guard.condition) else { return false };
        if entry.body.parameters.get(own_binding.parameter) != Some(&actual.root)
            || own_binding.members != full_members
        { return false; }
        block.clauses.iter().any(|clause| matches!(clause,
            LocalClause::Prerequisite(own) if qcore_contract::local_condition::named_prerequisite(own).as_ref() == Some(&wanted)))
    })
}
/// Decide only facts established at this exact call. Active prerequisites are
/// not assumed; unsupported source obligations and per-record correlations
/// remain explicit refusals. Unknown guards retain their active alternative.
pub fn call_ready(
    lifetime: &qcore_schemas::loop_lifetime::LoopLifetimeContract,
    call: &CallSite,
    units: &[&SourceFacts],
    run: &crate::member_run_facts::RunMemberFacts,
    entry: Option<&EntryContext<'_>>,
) -> Result<BTreeSet<String>, String> {
    use qcore_core::local_condition::Truth;
    let mut members = BTreeSet::new();
    for protocol in &lifetime.registered {
        if matches!(
            crate::loop_lifetime_summary::standing(protocol, units, &run.unread),
            crate::loop_lifetime_summary::Standing::Withdrawn { .. }
        ) {
            return Err("registered footprint is contradicted and withdrawn".into());
        }
        members.insert(format!(
            "{}.{}",
            protocol.record,
            protocol.interior.join(".")
        ));
        members.insert(format!("{}.{}", protocol.record, protocol.holder.join(".")));
    }
    for block in &lifetime.blocks {
        let binding = lifetime
            .bindings
            .iter()
            .find(|b| b.name == block.guard.condition)
            .ok_or("condition alias is unbound")?;
        let reading = condition_at(binding, call, units);
        let inactive = matches!(
            (reading, block.guard.negated),
            (Truth::No, false) | (Truth::Yes, true)
        );
        members.extend(binding_members(binding, call, units)?);
        if inactive {
            continue;
        }
        for clause in &block.clauses {
            match clause {
                qcore_schemas::loop_lifetime::LocalClause::Prerequisite(text) => {
                    if entry_discharges(text, binding, &block.guard, call, entry) {
                        continue;
                    }
                    return Err(format!(
                        "conditional prerequisite is not discharged on every feasible case: {}#{} => {text}",
                        if block.guard.negated { "not " } else { "" },
                        block.guard.condition
                    ));
                }
                qcore_schemas::loop_lifetime::LocalClause::Registered(_) => {
                    return Err(
                        "conditional protocol needs a derived record/host-instance correlation"
                            .into(),
                    );
                }
            }
        }
    }
    Ok(members)
}
/// Select the same analyzed function and call tokens from the run evidence.
/// Every matching compile reading must discharge; an ambiguous occurrence or
/// missing source/config reading refuses rather than borrowing another call's
/// condition fact. Canonicalization fidelity is a named extraction premise,
/// not proved by the abstract numeric-ID model.
pub(crate) fn ready_at_source_call(
    function: &str,
    callee: &str,
    line: usize,
    call_tokens: &[String],
    lifetime: &qcore_schemas::loop_lifetime::LoopLifetimeContract,
    contracts: &ResourceContracts,
) -> Result<BTreeSet<String>, String> {
    // Unconditional protocols may be explicit declared assumptions; this
    // never claims global closure. Conditional type protocols still need an
    // independently checked record-to-host correlation.
    if lifetime.blocks.iter().any(|block| {
        block.clauses.iter().any(|clause| {
            matches!(
                clause,
                qcore_schemas::loop_lifetime::LocalClause::Registered(_)
            )
        })
    }) {
        return Err(
            "global registered protocol lacks retained independently replayable run evidence"
                .into(),
        );
    }
    let local = crate::typed_access_path::current_lifetime_source(contracts)?;
    if !local.open.is_empty() {
        return Err("retained local source has unresolved type/source identity evidence".into());
    }
    let run = crate::member_run_facts::RunMemberFacts::default();
    let (digest, source_sha256) = crate::typed_access_path::with_indirect_call_unit(|unit| {
        let body = unit.function(function)?;
        let digest = tokens_digest(&unit.node_tokens(body)?);
        let mut matches = 0usize;
        walk(body, &mut |node| {
            if kind(node) == "CallExpr"
                && direct(node) == Some(callee)
                && unit.node_line(node) == Some(line)
                && unit.node_tokens(node).as_deref() == Some(call_tokens)
            {
                matches += 1;
            }
        });
        (matches == 1).then(|| (digest, unit.source_sha256()))
    })
    .flatten()
    .ok_or("analyzed source has no unique typed direct call with this token identity")?;
    let units = vec![&local];
    let mut matched = 0;
    let mut members = BTreeSet::new();
    for unit in &units {
        let Some(body) = unit.functions.get(function) else {
            continue;
        };
        if body.tokens_sha256 != digest || unit.source_sha256 != source_sha256 {
            continue;
        }
        let candidates = body
            .calls
            .iter()
            .filter(|c| c.callee == callee && c.line == line && c.tokens == call_tokens)
            .collect::<Vec<_>>();
        let [call] = candidates.as_slice() else {
            return Err("a matching compile reading has an absent or ambiguous source call".into());
        };
        matched += 1;
        let own = contracts
            .lifetime_contract(function)
            .and_then(|c| c.lifetime.as_ref());
        let entry = own.map(|lifetime| EntryContext { lifetime, body });
        members.extend(call_ready(lifetime, call, &units, &run, entry.as_ref())?);
        if !lifetime.registered.is_empty() {
            let digest = qcore_schemas::sha256_hex(
                &serde_json::to_vec(contracts.source_assumptions()).map_err(|e| e.to_string())?,
            );
            let assumptions = lifetime
                .registered
                .iter()
                .map(|p| {
                    format!(
                        "loop-protocol-declared:{callee}:{digest}:{}.{}|->{}.{}",
                        p.record,
                        p.interior.join("."),
                        p.record,
                        p.holder.join(".")
                    )
                })
                .collect();
            crate::contract_dependencies::lifetime_entry_evidence(function, assumptions);
            crate::contract_dependencies::relied(callee);
        }
        if let Some(entry) = entry.as_ref() {
            let mut prerequisites = BTreeSet::new();
            if entry.lifetime.entry_snapshot_stable {
                prerequisites.insert(format!(
                    "snapshot-entry:{}:{}",
                    unit.identity,
                    qcore_contract::local_condition::SNAPSHOT_PREMISE_NAME
                ));
            }
            for block in &entry.lifetime.blocks {
                let Some(binding) = entry
                    .lifetime
                    .bindings
                    .iter()
                    .find(|b| b.name == block.guard.condition)
                else {
                    continue;
                };
                let Some(root) = entry.body.parameters.get(binding.parameter) else {
                    continue;
                };
                for clause in &block.clauses {
                    if let qcore_schemas::loop_lifetime::LocalClause::Prerequisite(text) = clause {
                        prerequisites.insert(format!(
                            "conditional-entry:{}{}.{} => {}",
                            if block.guard.negated { "not " } else { "" },
                            root,
                            binding.members.join("."),
                            text
                        ));
                    }
                }
            }
            crate::contract_dependencies::lifetime_entry_evidence(function, prerequisites);
        }
    }
    if matched == 0 {
        return Err("no unique matching source call in the run's exact function readings".into());
    }
    Ok(members)
}

/// Bounded unconditional self-record forwarding, independently rebuilt from
/// retained source on certificate replay. The protocol is an attributed
/// DeclaredAssumption, never a claim that ambient run closure was proved.
pub(crate) fn forwarding_at_source_call(
    function_name: &str,
    callee: &str,
    line: usize,
    formal: &str,
    interior: &[String],
    holder_fields: &[String],
    contracts: &ResourceContracts,
) -> Option<crate::loop_lifetime_summary::ForwardedLifetime> {
    use crate::loop_lifetime_summary::{
        ForwardedLifetime, Path as SymbolicPath, Registration, Root, Standing,
    };
    let index = crate::typed_access_path::with_indirect_call_unit(|unit| {
        children(unit.function(function_name)?)
            .iter()
            .filter(|n| kind(n) == "ParmVarDecl")
            .position(|p| p["name"].as_str() == Some(formal))
    })
    .flatten()?;
    let source = crate::typed_access_path::current_lifetime_source(contracts).ok()?;
    if !source.open.is_empty() {
        return None;
    }
    let body = source.functions.get(function_name)?;
    if !body.open.is_empty() {
        return None;
    }
    let root = body.parameters.get(index)?;
    let calls = body
        .calls
        .iter()
        .filter(|c| c.callee == callee && c.line == line)
        .collect::<Vec<_>>();
    let [call] = calls.as_slice() else {
        return None;
    };
    if !call.pure_operands {
        return None;
    }
    let operation = body.operations.iter().find(|op| {
        op.offset == call.offset
            && op.kind == OperationKind::Register
            && op
                .object
                .as_ref()
                .is_some_and(|p| p.root == *root && p.fields == interior)
            && op
                .holder
                .as_ref()
                .is_some_and(|p| p.root == *root && p.fields == holder_fields)
    })?;
    let object = operation.object.as_ref()?;
    let holder = operation.holder.as_ref()?;
    let typed_pair = contracts
        .retaining_contract(callee)?
        .retentions
        .iter()
        .any(|retained| {
            let RetentionHolder::Parameter { index } = retained.holder else {
                return false;
            };
            call.typed_effect_actuals
                .get(retained.retained)
                .and_then(Option::as_ref)
                == Some(object)
                && call
                    .typed_effect_actuals
                    .get(index)
                    .and_then(Option::as_ref)
                    == Some(holder)
        });
    if !typed_pair {
        return None;
    }
    if object.record != holder.record {
        return None;
    }
    let digest =
        qcore_schemas::sha256_hex(&serde_json::to_vec(contracts.source_assumptions()).ok()?);
    for (host, declaration) in contracts.lifetime_declarations() {
        let Some(lifetime) = &declaration.lifetime else {
            continue;
        };
        for protocol in &lifetime.registered {
            if protocol.record != object.record
                || protocol.interior != interior
                || protocol.holder != holder_fields
            {
                continue;
            }
            if matches!(
                crate::loop_lifetime_summary::standing(protocol, &[&source], &BTreeMap::new()),
                Standing::Withdrawn { .. }
            ) {
                continue;
            }
            // This does not equate the holder to an engine/worker loop or
            // another record. Both operands are this one exact caller root.
            let scope = format!("{}:{function_name}", source.identity);
            let premise = format!(
                "loop-protocol-declared:{host}:{digest}:{}.{}|->{}.{}",
                protocol.record,
                interior.join("."),
                protocol.record,
                holder_fields.join(".")
            );
            crate::contract_dependencies::relied(host);
            crate::contract_dependencies::lifetime_evidence(
                function_name,
                BTreeSet::from([
                    format!("{}.{}", protocol.record, interior.join(".")),
                    format!("{}.{}", protocol.record, holder_fields.join(".")),
                ]),
            );
            crate::contract_dependencies::lifetime_entry_evidence(
                function_name,
                BTreeSet::from([premise.clone()]),
            );
            return Some(ForwardedLifetime {
                host: host.clone(),
                premise,
                registration: Registration {
                    object: SymbolicPath {
                        root: Root::Formal {
                            scope: scope.clone(),
                            index,
                        },
                        fields: interior.to_vec(),
                    },
                    holder: SymbolicPath {
                        root: Root::Formal { scope, index },
                        fields: holder_fields.to_vec(),
                    },
                    poisoned: false,
                },
            });
        }
    }
    None
}

/// Import only a source-rederived outgoing registration. Cross-unit effects
/// require a separately retained home and exact replayed import record. Bounded
/// substitution requires an exact current caller formal and unchanged members;
/// no lookup by callee spelling alone or closed-type lifting establishes it.
pub(crate) fn substitute_forwarded_call(
    caller: &str,
    callee: &str,
    line: usize,
    actual_name: &str,
    effect: &crate::loop_lifetime_summary::ForwardedLifetime,
    contracts: &ResourceContracts,
) -> Option<crate::loop_lifetime_summary::ForwardedLifetime> {
    use crate::loop_lifetime_summary::{Path as SymbolicPath, Root};
    let source = crate::typed_access_path::current_lifetime_source(contracts).ok()?;
    if !source.open.is_empty() {
        return None;
    }
    let body = source.functions.get(caller)?;
    if !body.open.is_empty() {
        return None;
    }
    let caller_index = crate::typed_access_path::with_indirect_call_unit(|unit| {
        children(unit.function(caller)?)
            .iter()
            .filter(|n| kind(n) == "ParmVarDecl")
            .position(|p| p["name"].as_str() == Some(actual_name))
    })
    .flatten()?;
    let Root::Formal { scope, index } = &effect.registration.object.root else {
        return None;
    };
    let local_home =
        source.functions.contains_key(callee) && *scope == format!("{}:{callee}", source.identity);
    let imported_home = !source.functions.contains_key(callee)
        && contracts
            .lifetime_imports()
            .get(callee)
            .is_some_and(|s| s.effects.contains(effect))
        && contracts
            .cross_unit_import_identities()
            .contains_key(callee)
        && contracts.link_resolution_sha256().is_some();
    if effect.registration.holder.root != effect.registration.object.root
        || !(local_home || imported_home)
    {
        return None;
    }
    let calls = body
        .calls
        .iter()
        .filter(|c| c.callee == callee && c.line == line)
        .collect::<Vec<_>>();
    let [call] = calls.as_slice() else {
        return None;
    };
    if !call.pure_operands {
        return None;
    }
    let actual = call.condition_actuals.get(*index)?.as_ref()?;
    if body.parameters.get(caller_index) != Some(&actual.root) || !actual.fields.is_empty() {
        return None;
    }
    let host = contracts.lifetime_contract(&effect.host)?;
    let protocol = host.lifetime.as_ref()?.registered.iter().find(|p| {
        p.record == actual.record
            && p.interior == effect.registration.object.fields
            && p.holder == effect.registration.holder.fields
    })?;
    let digest =
        qcore_schemas::sha256_hex(&serde_json::to_vec(contracts.source_assumptions()).ok()?);
    let expected = format!(
        "loop-protocol-declared:{}:{digest}:{}.{}|->{}.{}",
        effect.host,
        protocol.record,
        protocol.interior.join("."),
        protocol.record,
        protocol.holder.join(".")
    );
    if effect.premise != expected {
        return None;
    }
    let mut forwarded = effect.clone();
    let root = Root::Formal {
        scope: format!("{}:{caller}", source.identity),
        index: caller_index,
    };
    forwarded.registration.object = SymbolicPath {
        root: root.clone(),
        fields: effect.registration.object.fields.clone(),
    };
    forwarded.registration.holder = SymbolicPath {
        root,
        fields: effect.registration.holder.fields.clone(),
    };
    crate::contract_dependencies::relied(&effect.host);
    crate::contract_dependencies::lifetime_evidence(
        caller,
        BTreeSet::from([
            format!("{}.{}", protocol.record, protocol.interior.join(".")),
            format!("{}.{}", protocol.record, protocol.holder.join(".")),
        ]),
    );
    crate::contract_dependencies::lifetime_entry_evidence(
        caller,
        BTreeSet::from([forwarded.premise.clone()]),
    );
    Some(forwarded)
}

/// The frame is tied to a typed direct call and its exact collector root.
/// A matching field spelling on a different record or formal never suffices.
pub(crate) fn holder_frame_at_call(
    caller: &str,
    callee: &str,
    formal: &str,
    fields: &[String],
    tokens: &[&str],
    parameter: usize,
    contracts: &ResourceContracts,
) -> bool {
    let Some(summary) = contracts.lifetime_summary(callee) else {
        return false;
    };
    let Ok(source) = crate::typed_access_path::current_lifetime_source(contracts) else {
        return false;
    };
    let Some(body) = source.functions.get(caller) else {
        return false;
    };
    if !source.open.is_empty() || !body.open.is_empty() {
        return false;
    }
    let Some(index) = crate::typed_access_path::with_indirect_call_unit(|unit| {
        children(unit.function(caller)?)
            .iter()
            .filter(|n| kind(n) == "ParmVarDecl")
            .position(|p| p["name"].as_str() == Some(formal))
    })
    .flatten() else {
        return false;
    };
    let matches = body
        .calls
        .iter()
        .filter(|c| {
            c.callee == callee
                && c.tokens
                    .iter()
                    .map(String::as_str)
                    .eq(tokens.iter().copied())
        })
        .collect::<Vec<_>>();
    let [call] = matches.as_slice() else {
        return false;
    };
    if !call.pure_operands {
        return false;
    }
    let Some(actual) = call
        .condition_actuals
        .get(parameter)
        .and_then(Option::as_ref)
    else {
        return false;
    };
    body.parameters.get(index) == Some(&actual.root)
        && actual.fields.is_empty()
        && summary.holder_frames.iter().any(|frame| {
            frame.parameter == parameter && frame.record == actual.record && frame.fields == fields
        })
}

/// An immutable local loop snapshot names only its original exact formal path.
pub(crate) fn immutable_holder_alias(
    function: &str,
    local: &str,
    contracts: &ResourceContracts,
) -> Option<(String, Vec<String>)> {
    let source = crate::typed_access_path::current_lifetime_source(contracts).ok()?;
    let facts = source.functions.get(function)?;
    if !source.open.is_empty() || !facts.open.is_empty() {
        return None;
    }
    let alias = facts.immutable_aliases.get(local)?;
    if alias.fields.len() > 1 {
        return None;
    }
    let index = facts.parameters.iter().position(|id| id == &alias.root)?;
    let formal = crate::typed_access_path::with_indirect_call_unit(|unit| {
        let parameter = children(unit.function(function)?)
            .iter()
            .filter(|p| kind(p) == "ParmVarDecl")
            .nth(index)?;
        let spelling = ty(parameter);
        if spelling.matches('*').count() != 1
            || !spelling.trim_end().ends_with('*')
            || record(spelling).as_deref() != Some(alias.record.as_str())
        {
            return None;
        }
        parameter["name"].as_str().map(str::to_owned)
    })
    .flatten()?;
    crate::contract_dependencies::lifetime_evidence(
        function,
        BTreeSet::from([format!("{}.{}", alias.record, alias.fields.join("."))]),
    );
    Some((formal, alias.fields.clone()))
}

/// Read exact, uniquely named collector copies and their initializer spans.
/// Every later use is still checked by the holder path census, under the
/// canonical original formal. No alias is guessed from type or numeric id.
pub(crate) fn collector_alias_frame(
    function: &str,
    formal: &str,
    body: &[&str],
    contracts: &ResourceContracts,
) -> (BTreeSet<String>, BTreeSet<usize>) {
    let empty = || (BTreeSet::new(), BTreeSet::new());
    let Ok(source) = crate::typed_access_path::current_lifetime_source(contracts) else {
        return empty();
    };
    let Some(facts) = source.functions.get(function) else {
        return empty();
    };
    if !source.open.is_empty() || !facts.open.is_empty() || !facts.complete_straight_line {
        return empty();
    }
    crate::typed_access_path::with_indirect_call_unit(|unit| {
        let ast = unit.function(function)?;
        let index = children(ast)
            .iter()
            .filter(|p| kind(p) == "ParmVarDecl")
            .position(|p| p["name"].as_str() == Some(formal))?;
        let parameter = children(ast)
            .iter()
            .filter(|p| kind(p) == "ParmVarDecl")
            .nth(index)?;
        if ty(parameter).matches('*').count() != 1 || !ty(parameter).trim_end().ends_with('*') {
            return None;
        }
        let root = facts.parameters.get(index)?;
        let mut aliases = BTreeSet::new();
        let mut allowed = BTreeSet::new();
        walk(ast, &mut |node| {
            if kind(node) != "VarDecl" {
                return;
            }
            let Some(name) = node["name"].as_str() else {
                return;
            };
            if !facts
                .immutable_aliases
                .get(name)
                .is_some_and(|p| p.root == *root && p.fields.is_empty())
            {
                return;
            }
            let Some(tokens) = unit.node_tokens(node) else {
                return;
            };
            if tokens.is_empty() {
                return;
            }
            let spans = body
                .windows(tokens.len())
                .enumerate()
                .filter(|(_, window)| window.iter().copied().eq(tokens.iter().map(String::as_str)))
                .map(|(i, _)| i)
                .collect::<Vec<_>>();
            let [at] = spans.as_slice() else { return };
            aliases.insert(name.into());
            allowed.extend(*at..at + tokens.len());
        });
        Some((aliases, allowed))
    })
    .flatten()
    .unwrap_or_else(empty)
}

/// A value formal is this frame's object, not a caller-supplied pointee.
pub(crate) fn caller_pointer_formal(function: &str, formal: &str) -> bool {
    crate::typed_access_path::with_indirect_call_unit(|unit| {
        children(unit.function(function)?)
            .iter()
            .find(|p| kind(p) == "ParmVarDecl" && p["name"].as_str() == Some(formal))
            .map(|p| {
                ty(p).matches('*').count() == 1
                    && ty(p).trim_end().ends_with('*')
                    && !ty(p).contains("volatile")
                    && !ty(p).contains("_Atomic")
            })
    })
    .flatten()
    .unwrap_or(false)
}
