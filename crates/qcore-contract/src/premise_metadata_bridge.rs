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

//! Exact additive-symbolic-metadata provenance, never a truth or verdict rule.
use qcore_frontend::frontend::{SourceContractClauseKind, parse_source_program};
use qcore_frontend::lexer::{TokenKind, lex};
use qcore_schemas::{
    hypothesis::Graph,
    premise_flow::{Expr, View},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct Counts {
    files: usize,
    declarations: usize,
    atoms: usize,
    groups: usize,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Plan {
    version: u32,
    base: String,
    counts: Counts,
    files: Vec<Module>,
    inheritance: Vec<Inheritance>,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Inheritance {
    file: String,
    declaration: String,
    source: String,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Module {
    path: String,
    sha256: String,
    bytes: usize,
    declarations: Vec<Declaration>,
    binding_manifest_lines: Vec<String>,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Declaration {
    declaration: String,
    view: View,
    atoms: Vec<Atom>,
    groups: Vec<Group>,
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct Atom {
    id: String,
    statement_literal: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct Group {
    kind: String,
    name: String,
    children: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ModuleReceipt {
    module: String,
    view: View,
    before_sha256: String,
    after_sha256: String,
    erased_sha256: String,
    declarations: usize,
    introduced_atoms: usize,
    conjunction_flows: usize,
    explicit_graphs_sha256: String,
    added_lines: Vec<String>,
}
/// Serialization is archival only. No Deserialize or unchecked constructor.
/// ```compile_fail
/// let _: qcore_contract::premise_metadata_bridge::CheckedBridge = serde_json::from_str("{}").unwrap();
/// ```
#[derive(Debug, Serialize)]
pub struct CheckedBridge {
    version: u32,
    plan_sha256: String,
    modules: Vec<ModuleReceipt>,
    #[serde(skip)]
    plan: Plan,
}
fn digest<T: Serialize>(value: &T) -> Result<String, String> {
    Ok(qcore_schemas::sha256_hex(
        &serde_json::to_vec(value).map_err(|e| e.to_string())?,
    ))
}
fn sha(s: &str) -> String {
    qcore_schemas::sha256_hex(s.as_bytes())
}
fn words(source: &str) -> Result<Vec<qcore_frontend::lexer::Token>, String> {
    Ok(lex(source)
        .map_err(|e| e.to_string())?
        .into_iter()
        .filter(|t| t.kind != TokenKind::Eof)
        .collect())
}
fn head(source: &str) -> Result<Option<String>, String> {
    Ok(words(source)?.first().and_then(|t| {
        if let TokenKind::Word(s) = &t.kind {
            Some(s.clone())
        } else {
            None
        }
    }))
}
fn atom(source: &str) -> Result<Atom, String> {
    let t = words(source)?;
    if t.len() != 6 {
        return Err("bridge: malformed original atom".into());
    }
    let TokenKind::Word(id) = &t[2].kind else {
        return Err("bridge: original atom ID absent".into());
    };
    let TokenKind::StringLiteral(_) = &t[4].kind else {
        return Err("bridge: original atom statement absent".into());
    };
    Ok(Atom {
        id: id.clone(),
        statement_literal: source[t[4].span.start..t[4].span.end].into(),
    })
}
fn group(source: &str, kind: &str) -> Result<Group, String> {
    let t = words(source)?;
    let Some(TokenKind::Word(name)) = t.get(2).map(|t| &t.kind) else {
        return Err("bridge: group name absent".into());
    };
    let children = t
        .iter()
        .skip(4)
        .filter_map(|t| {
            if let TokenKind::Word(s) = &t.kind {
                Some(s.clone())
            } else {
                None
            }
        })
        .collect();
    Ok(Group {
        kind: if kind == "resource_premise_all" {
            "all"
        } else {
            "decomposition"
        }
        .into(),
        name: name.clone(),
        children,
    })
}
fn expression(group: &Group) -> Result<(String, Expr), String> {
    let mut children = group.children.iter();
    let first = children.next().ok_or("bridge: empty conjunction")?;
    let mut text = first.clone();
    let mut value = Expr::Atom {
        atom: first.clone(),
    };
    for child in children {
        text = format!("({text} |-> {child})");
        value = Expr::And {
            left: Box::new(value),
            right: Box::new(Expr::Atom {
                atom: child.clone(),
            }),
        };
    }
    Ok((text, value))
}
fn graph(
    decl: &qcore_frontend::frontend::SourceDeclaration,
    legacy: &BTreeMap<String, Vec<crate::contract_loader::DeclaredPremise>>,
    groups: &[Group],
) -> Result<Graph, String> {
    // module_premises includes premise_all's erasure; it is not a second legacy singleton.
    let all = groups
        .iter()
        .filter(|g| g.kind == "all")
        .map(|g| g.name.as_str())
        .collect::<BTreeSet<_>>();
    let premises = legacy
        .get(&decl.name)
        .into_iter()
        .flatten()
        .filter(|p| !all.contains(p.name.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    crate::hypothesis::declaration(decl, &premises)
}
// Identical offset-preserving inspection technique to the inclusion elaborator.
// It resolves no parent and cannot produce a loaded contract.
fn inspection_probe(source: &str) -> Result<String, String> {
    let syntax = qcore_frontend::contract_inclusion::scan_contract_inclusion(source)
        .map_err(|e| e.to_string())?;
    let mut bytes = source.as_bytes().to_vec();
    for directive in syntax.includes {
        for byte in &mut bytes[directive.span.start..directive.span.end] {
            if *byte != b'\n' && *byte != b'\r' {
                *byte = b' ';
            }
        }
    }
    for site in syntax.inherits {
        for byte in &mut bytes[site.span.start..site.span.end] {
            if *byte != b'\n' && *byte != b'\r' {
                *byte = b' ';
            }
        }
        bytes[site.span.start] = b'{';
        bytes[site.span.end - 1] = b'}';
    }
    String::from_utf8(bytes).map_err(|e| e.to_string())
}
fn validate_module(
    module: &Module,
    before: &str,
    after: &str,
    expanded: bool,
) -> Result<ModuleReceipt, String> {
    if before.is_empty()
        || after.is_empty()
        || (!expanded && (before.len() != module.bytes || sha(before) != module.sha256))
    {
        return Err("bridge: stale held raw source".into());
    }
    let probe = inspection_probe(before)?;
    let parsed = parse_source_program(&probe).map_err(|e| e.to_string())?;
    let old_legacy = crate::contract_loader::module_premises(&probe)?;
    let wanted = module
        .declarations
        .iter()
        .map(|d| (d.declaration.as_str(), d))
        .collect::<BTreeMap<_, _>>();
    if wanted.len() != module.declarations.len() || wanted.is_empty() {
        return Err("bridge: empty/duplicate declaration inventory".into());
    }
    let mut found = BTreeSet::new();
    let mut insertions = BTreeMap::<usize, String>::new();
    let mut graphs = BTreeMap::new();
    let mut added_lines = Vec::new();
    let mut total_atoms = 0;
    let mut total_groups = 0;
    for located in &parsed.declarations {
        let decl = &located.value;
        let Some(plan) = wanted.get(decl.name.as_str()) else {
            if !expanded
                && decl.function_contract.as_ref().is_some_and(|c| {
                    c.clauses.iter().any(|c| {
                        matches!(
                            head(&c.value.expression_source).ok().flatten().as_deref(),
                            Some(
                                "resource_hypothesis"
                                    | "resource_premise_all"
                                    | "resource_premise_decomposition"
                            )
                        )
                    })
                })
            {
                return Err("bridge: omitted explicit declaration".into());
            }
            continue;
        };
        if !found.insert(decl.name.clone()) {
            return Err("bridge: repeated declaration".into());
        }
        let mut atoms = Vec::new();
        let mut groups = Vec::new();
        for clause in decl.function_contract.iter().flat_map(|c| &c.clauses) {
            let source = &clause.value.expression_source;
            let h = head(source)?;
            if matches!(
                h.as_deref(),
                Some("resource_lifecycle_v1" | "resource_premise_flow_v1")
            ) {
                return Err("bridge: original inventory already contains proposed symbols".into());
            }
            let addition = match h.as_deref() {
                Some("resource_hypothesis") => {
                    let a = atom(source)?;
                    let text = format!(
                        "requires resource_lifecycle_v1(introduce_{}, |^+ hypothesis({}));",
                        a.id, a.id
                    );
                    atoms.push(a);
                    text
                }
                Some(kind @ ("resource_premise_all" | "resource_premise_decomposition")) => {
                    let g = group(source, kind)?;
                    let text = format!(
                        "requires resource_premise_flow_v1(decompose_{}, {});",
                        g.name,
                        expression(&g)?.0
                    );
                    groups.push(g);
                    text
                }
                _ => continue,
            };
            if clause.value.kind != SourceContractClauseKind::Requires {
                return Err("bridge: original anchor is not requires".into());
            }
            let start = before[..clause.span.start].rfind('\n').map_or(0, |i| i + 1);
            let end = before[clause.span.end..]
                .find('\n')
                .map(|i| clause.span.end + i + 1)
                .ok_or("bridge: standalone LF anchor required")?;
            let indent = &before[start..clause.span.start];
            if !indent.bytes().all(|b| b == b' ' || b == b'\t')
                || !before[clause.span.end..end - 1].trim().is_empty()
                || before[start..end].contains('\r')
            {
                return Err("bridge: nonstandalone/CRLF anchor refused".into());
            }
            let line = format!("{indent}{addition}\n");
            if insertions.insert(end, line.clone()).is_some() {
                return Err("bridge: overlapping anchor".into());
            }
            added_lines.push(line);
        }
        if atoms != plan.atoms || groups != plan.groups {
            return Err(format!(
                "bridge: actual atom/group/order inventory mismatch: {}",
                decl.name
            ));
        }
        let g = graph(decl, &old_legacy, &groups)?;
        crate::hypothesis::validate(&g)?;
        if !crate::contract_lifecycle::declaration(decl, &g)?.is_empty()
            || !crate::premise_flow::declaration(decl, &g)?.is_empty()
        {
            return Err("bridge: preexisting metadata collision".into());
        }
        graphs.insert(decl.name.clone(), g);
        total_atoms += atoms.len();
        total_groups += groups.len();
    }
    if found.len() != wanted.len() {
        return Err("bridge: missing explicit declaration".into());
    }
    let mut generated = String::new();
    let mut cursor = 0;
    for (offset, line) in &insertions {
        generated.push_str(&before[cursor..*offset]);
        generated.push_str(line);
        cursor = *offset;
    }
    generated.push_str(&before[cursor..]);
    if generated != after {
        return Err(
            "bridge: TIP differs from exact deterministic typed metadata insertions".into(),
        );
    }
    let new_probe = inspection_probe(after)?;
    let new = parse_source_program(&new_probe).map_err(|e| e.to_string())?;
    let new_legacy = crate::contract_loader::module_premises(&new_probe)?;
    for located in &new.declarations {
        let decl = &located.value;
        let Some(plan) = wanted.get(decl.name.as_str()) else {
            continue;
        };
        let g = graph(decl, &new_legacy, &plan.groups)?;
        if graphs.get(&decl.name) != Some(&g) {
            return Err("bridge: original graph changed".into());
        }
        let lifecycle = crate::contract_lifecycle::declaration(decl, &g)?;
        let expected = plan
            .atoms
            .iter()
            .map(|a| {
                (
                    format!("introduce_{}", a.id),
                    crate::contract_lifecycle::Entry {
                        operation: crate::contract_lifecycle::Operation::Create,
                        target: crate::contract_lifecycle::Target::Hypothesis(a.id.clone()),
                    },
                )
            })
            .collect::<BTreeMap<_, _>>();
        if lifecycle != expected {
            return Err("bridge: actual typed lifecycle differs".into());
        }
        let flows = crate::premise_flow::declaration(decl, &g)?;
        let expected = plan
            .groups
            .iter()
            .map(|g| Ok((format!("decompose_{}", g.name), expression(g)?.1)))
            .collect::<Result<BTreeMap<_, _>, String>>()?;
        if flows != expected {
            return Err("bridge: actual typed flow/reference differs".into());
        }
    }
    if !expanded {
        let bindings = before
            .lines()
            .filter(|l| {
                l.trim()
                    .starts_with("// qcore-premise-binding-manifest-v1 ")
            })
            .map(str::to_owned)
            .collect::<Vec<_>>();
        if bindings != module.binding_manifest_lines {
            return Err("bridge: binding inventory differs".into());
        }
    }
    let view = module.declarations[0].view;
    if module.declarations.iter().any(|d| d.view != view) {
        return Err("bridge: mixed property inventory".into());
    }
    Ok(ModuleReceipt {
        module: module.path.clone(),
        view,
        before_sha256: sha(before),
        after_sha256: sha(after),
        erased_sha256: sha(before),
        declarations: found.len(),
        introduced_atoms: total_atoms,
        conjunction_flows: total_groups,
        explicit_graphs_sha256: digest(&graphs)?,
        added_lines,
    })
}
/// Actual raw files and the independent inventory/hash are required anew.
pub fn check(
    plan_bytes: &[u8],
    expected_plan_sha256: &str,
    before: &BTreeMap<String, String>,
    after: &BTreeMap<String, String>,
) -> Result<CheckedBridge, String> {
    if qcore_schemas::sha256_hex(plan_bytes) != expected_plan_sha256 {
        return Err("bridge: independent plan digest mismatch".into());
    }
    let plan: Plan = serde_json::from_slice(plan_bytes).map_err(|e| e.to_string())?;
    if plan.version != 1 || plan.base.len() != 40 || plan.files.is_empty() {
        return Err("bridge: unsupported/empty inventory".into());
    }
    let paths = plan
        .files
        .iter()
        .map(|m| m.path.clone())
        .collect::<BTreeSet<_>>();
    if paths.len() != plan.files.len()
        || before.keys().cloned().collect::<BTreeSet<_>>() != paths
        || after.keys().cloned().collect::<BTreeSet<_>>() != paths
    {
        return Err("bridge: complete source file inventory mismatch".into());
    }
    let mut modules = Vec::new();
    for module in &plan.files {
        modules.push(validate_module(
            module,
            &before[&module.path],
            &after[&module.path],
            false,
        )?);
    }
    let counts = Counts {
        files: modules.len(),
        declarations: modules.iter().map(|m| m.declarations).sum(),
        atoms: modules.iter().map(|m| m.introduced_atoms).sum(),
        groups: modules.iter().map(|m| m.conjunction_flows).sum(),
    };
    if counts != plan.counts {
        return Err("bridge: complete count inventory mismatch".into());
    }
    for inherited in &plan.inheritance {
        let source = before
            .get(&inherited.file)
            .ok_or("bridge: inherited module absent")?;
        let probe = inspection_probe(source)?;
        let parsed = parse_source_program(&probe).map_err(|e| e.to_string())?;
        let matching = parsed
            .declarations
            .iter()
            .filter(|d| d.value.name == inherited.declaration)
            .collect::<Vec<_>>();
        if matching.len() != 1
            || source[matching[0].span.start..matching[0].span.end] != inherited.source
        {
            return Err("bridge: inherited declaration bytes differ".into());
        }
    }
    Ok(CheckedBridge {
        version: 1,
        plan_sha256: expected_plan_sha256.into(),
        modules,
        plan,
    })
}
impl CheckedBridge {
    /// Loaded/elaborated text is independently held by the caller. Only the
    /// already raw-checked explicit declarations may receive added lines;
    /// inherited aliases and every other byte remain untouched.
    pub fn check_expanded(
        &self,
        module: &str,
        view: View,
        before: &str,
        expected_before_sha256: &str,
        after: &str,
    ) -> Result<ModuleReceipt, String> {
        if sha(before) != expected_before_sha256 {
            return Err("bridge: held expanded source digest mismatch".into());
        }
        let plan = self
            .plan
            .files
            .iter()
            .find(|m| m.path == module)
            .ok_or("bridge: module outside raw checked scope")?;
        if plan.declarations.iter().any(|d| d.view != view) {
            return Err("bridge: foreign property view".into());
        }
        validate_module(plan, before, after, true)
    }
}
