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

//! Bounded typed C access paths shared by library source importers.
//!
//! These are source projections, NOT safety or custody certificates.
//! Reads and calls yield fresh symbols; pointer aliases retain those symbols.
//! Calls remain opaque (in particular, a recorded close call is not discharge).
//! All branches are retained, including infeasible alternatives. Consumers must
//! establish guards, pointer provenance, live/bounded storage, arithmetic, call
//! contracts, and synchronization before interpreting a path as safe.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use serde_json::Value;

use crate::legacy_c::{C11TranslationUnit, validate_unrestricted_c11};
use crate::typed_c_ast::{ast_declarations, children, function_ast, kind};
use crate::typed_source_nodes::{self, NodeAdmission};

// CG09-TYPED-SOURCE-CONTROL (rule 11l): route T's control statements, read
// only where the importing consumer admits them. A child module, so it sees
// the importer's private statement and path types.
#[path = "typed_source_control.rs"]
pub mod control;
pub use control::{CONTROL_TABLE, ControlAdmission, ControlFacts};

/// Default declarative acceptance rules used by the legacy typed-path importer.
/// This is shared policy data, not source evidence or a safety certificate.
pub const DEFAULT_SOURCE_RULES: &str = include_str!(
    "../../../providers/quirin-provider-shared/contracts/library/source_projection.qv"
);

fn check_source_rule(name: &str, facts: &[(&str, i128)]) -> Result<(), String> {
    qcore_contract::contract_observation::check_contract_facts(DEFAULT_SOURCE_RULES, name, facts)
}

const MAX_PATHS: usize = 64;
const MAX_OPERATIONS: usize = 512;
/// The importer's statement/expression depth and node budgets. CG09 binds
/// all four budgets to `QCore.TypedSourceControl.budgets`.
const MAX_DEPTH: usize = 64;
const MAX_NODES: usize = 2048;

/// Typed symbolic values. A symbol refers to an earlier Read or Call in this
/// same path. Integer casts and sizeof remain explicit, never host-evaluated.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SourceValue {
    pub c_type: String,
    pub expression: SourceExpression,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SourceExpression {
    Function {
        name: String,
        signature: String,
    },
    Parameter {
        index: usize,
    },
    LocalStorage {
        local: usize,
    },
    /// Typed induction value, scoped to the enclosing ScanLoop iteration.
    LoopIndex {
        local: usize,
    },
    Integer {
        value: String,
    },
    String {
        value: String,
    },
    /// A floating literal: Clang's spelling with its Clang type. No floating
    /// arithmetic is interpreted (CG01-T-OPERATOR-CLOSURE; IEEE is N09).
    Floating {
        value: String,
    },
    /// A character literal: Clang's integer code with its Clang type.
    Character {
        value: String,
    },
    SizeOf {
        argument_type: String,
    },
    /// Ordered initializer elements; layouts and bounds remain consumer obligations.
    Aggregate {
        elements: Vec<SourceValue>,
    },
    Symbol {
        operation: usize,
    },
    Cast {
        cast: String,
        value: Box<SourceValue>,
    },
    Unary {
        operator: String,
        value: Box<SourceValue>,
    },
    Binary {
        operator: String,
        left: Box<SourceValue>,
        right: Box<SourceValue>,
    },
    Field {
        base: Box<SourceValue>,
        name: String,
    },
    Index {
        base: Box<SourceValue>,
        index: Box<SourceValue>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SourceOperation {
    /// Repeat header, then match test, then store increment into local. A true
    /// match executes early_return and exits the FUNCTION; a false header continues outside
    /// the loop. Bounds, arithmetic and termination are consumer obligations.
    ScanLoop {
        local: usize,
        initial: SourceValue,
        header: Box<SourceRegion>,
        matches: Box<SourceRegion>,
        early_return: Box<SourceRegion>,
        increment: SourceValue,
    },
    /// Storage type is carried by place; this operation yields that type.
    Read {
        place: SourceValue,
    },
    Write {
        place: SourceValue,
        value: SourceValue,
    },
    /// Stack values are retained for address-taking (e.g. send's &val).
    LocalWrite {
        local: usize,
        value: SourceValue,
    },
    LocalStart {
        local: usize,
        c_type: String,
    },
    /// Signature is the source declaration's type, not an assumed API contract.
    Call {
        function: String,
        signature: String,
        arguments: Vec<SourceValue>,
        result_type: String,
    },
    Guard {
        condition: SourceValue,
        taken: bool,
    },
    LocalEnd {
        local: usize,
    },
    ReturnValue {
        value: SourceValue,
    },
    Return,
    /// A source-declared nonreturning call ended this path. This is not teardown.
    Terminate,
}

/// Symbol indices continue from the enclosing path through the three regions
/// of one iteration. Iteration producers are never available after loop exit.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SourceRegion {
    pub operations: Vec<SourceOperation>,
    pub result: SourceValue,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SourcePath {
    pub operations: Vec<SourceOperation>,
}

/// Construction is private: every definition node must pass the importer.
/// A serialized path report cannot be deserialized into source authority.
///
/// ```compile_fail
/// use qcore_c_gateway::typed_source::SourceSummary;
/// let forged: SourceSummary = serde_json::from_str("{}").unwrap();
/// ```
/// Nor can a caller manufacture one from mutable path records.
///
/// ```compile_fail
/// use qcore_c_gateway::typed_source::SourceSummary;
/// let forged = SourceSummary {
///     function: "f".into(), source_revision: "unverified".into(),
///     signature: "void ()".into(), paths: vec![],
///     profile_assumptions: Default::default(),
/// };
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SourceSummary {
    function: String,
    source_revision: String,
    signature: String,
    paths: Vec<SourcePath>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    profile_assumptions: BTreeMap<String, String>,
}

impl SourceSummary {
    pub fn profile_assumptions(&self) -> &BTreeMap<String, String> {
        &self.profile_assumptions
    }

    pub fn signature(&self) -> &str {
        &self.signature
    }
    pub fn function(&self) -> &str {
        &self.function
    }
    pub fn source_revision(&self) -> &str {
        &self.source_revision
    }
    pub fn paths(&self) -> &[SourcePath] {
        &self.paths
    }

    pub fn has_scan_loop(&self) -> bool {
        self.paths.iter().any(|path| {
            path.operations
                .iter()
                .any(|operation| matches!(operation, SourceOperation::ScanLoop { .. }))
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum SourceResult {
    Summary(SourceSummary),
    Unknown { function: String, reason: String },
}

/// Syntax-selection inputs only; these confer no call effects or safety claims.
pub struct SourceImportProfile<'a> {
    pub records: &'a [&'a str],
    pub sizeof_types: &'a [&'a str],
    pub signature_constraints: &'a [(&'a str, &'a str, &'a [&'a str])],
    /// Functions with an exact signature use this bounded call vocabulary.
    pub restricted_calls: &'a [&'a str],
}

/// Import a role-selected syntax surface from a validated, explicit source profile.
/// Function names are returned in deterministic lexical order. Callback signatures
/// are mandatory; wrappers without signatures preserve opaque unrestricted calls.
/// Successful summaries retain the exact declared profile assumption record.
pub fn import_source_functions_from_profile(
    unit: &C11TranslationUnit<'_>,
    source_revision: &str,
    profile: &qcore_contract::profile_loader::LoadedProfile,
    library: &str,
    role: qcore_schemas::profile::ImportRole,
) -> Result<Vec<SourceResult>, String> {
    import_source_functions_from_profile_admitting(
        unit,
        source_revision,
        profile,
        library,
        role,
        &NodeAdmission::LEGACY,
    )
}

/// As [`import_source_functions_from_profile`], admitting exactly the CG01
/// nodes the calling consumer classifies (rule 10r); every other CG01 node
/// is refused by name.
pub fn import_source_functions_from_profile_admitting(
    unit: &C11TranslationUnit<'_>,
    source_revision: &str,
    profile: &qcore_contract::profile_loader::LoadedProfile,
    library: &str,
    role: qcore_schemas::profile::ImportRole,
    admission: &NodeAdmission,
) -> Result<Vec<SourceResult>, String> {
    import_source_functions_from_profile_admitting_control(
        unit,
        source_revision,
        profile,
        library,
        role,
        admission,
        &ControlAdmission::LEGACY,
    )
}

/// As [`import_source_functions_from_profile_admitting`], also admitting
/// exactly the control forms the calling consumer lists (rule 11l).
pub fn import_source_functions_from_profile_admitting_control(
    unit: &C11TranslationUnit<'_>,
    source_revision: &str,
    profile: &qcore_contract::profile_loader::LoadedProfile,
    library: &str,
    role: qcore_schemas::profile::ImportRole,
    admission: &NodeAdmission,
    control: &ControlAdmission,
) -> Result<Vec<SourceResult>, String> {
    let selected = profile
        .library(library)
        .ok_or_else(|| format!("no library `{library}` in source profile"))?;
    let pin = profile
        .manifest()
        .pin_for(library)
        .ok_or("library has no source pin")?;
    if source_revision != pin.git_commit {
        return Err("source revision does not match the declared profile pin".into());
    }
    let typed = selected
        .typed_import
        .as_ref()
        .ok_or("library has no typed import profile")?;
    let functions = typed
        .functions
        .iter()
        .filter_map(|(name, found)| (*found == role).then_some(name.as_str()))
        .collect::<Vec<_>>();
    if functions.is_empty() {
        return Err(format!("profile has no {role:?} functions"));
    }
    let records = typed.records.iter().map(String::as_str).collect::<Vec<_>>();
    let sizeof_types = typed
        .sizeof_types
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let restricted_calls = typed
        .restricted_calls
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let parameters = typed
        .signature_constraints
        .values()
        .map(|s| s.parameters.iter().map(String::as_str).collect::<Vec<_>>())
        .collect::<Vec<_>>();
    let signatures = typed
        .signature_constraints
        .iter()
        .zip(&parameters)
        .map(|((name, s), p)| (name.as_str(), s.result.as_str(), p.as_slice()))
        .collect::<Vec<_>>();
    let mut results = import_source_functions_admitting_control(
        unit,
        source_revision,
        &functions,
        &SourceImportProfile {
            records: &records,
            sizeof_types: &sizeof_types,
            signature_constraints: &signatures,
            restricted_calls: &restricted_calls,
        },
        admission,
        control,
    )?;
    let mut assumptions = profile.assumptions();
    assumptions.insert("library".into(), library.into());
    assumptions.insert("typed_import_role".into(), format!("{role:?}"));
    for result in &mut results {
        if let SourceResult::Summary(summary) = result {
            summary.profile_assumptions = assumptions.clone();
        }
    }
    Ok(results)
}

/// Import with the pre-CG01 grammar: every CG01 node is refused by name
/// ([`NodeAdmission::LEGACY`]).
pub fn import_source_functions(
    unit: &C11TranslationUnit<'_>,
    source_revision: &str,
    functions: &[&str],
    profile: &SourceImportProfile<'_>,
) -> Result<Vec<SourceResult>, String> {
    import_source_functions_admitting(
        unit,
        source_revision,
        functions,
        profile,
        &NodeAdmission::LEGACY,
    )
}

/// Import admitting exactly the CG01 nodes `admission` classifies (rule 10r).
/// Reading a node is not admission: an unlisted node is refused by name.
pub fn import_source_functions_admitting(
    unit: &C11TranslationUnit<'_>,
    source_revision: &str,
    functions: &[&str],
    profile: &SourceImportProfile<'_>,
    admission: &NodeAdmission,
) -> Result<Vec<SourceResult>, String> {
    import_source_functions_admitting_control(
        unit,
        source_revision,
        functions,
        profile,
        admission,
        &ControlAdmission::LEGACY,
    )
}

/// Import admitting exactly the CG01 nodes `admission` classifies (rule 10r)
/// and the control forms `control` lists (rule 11l). Every other node or form
/// is refused by name.
pub fn import_source_functions_admitting_control(
    unit: &C11TranslationUnit<'_>,
    source_revision: &str,
    functions: &[&str],
    profile: &SourceImportProfile<'_>,
    admission: &NodeAdmission,
    control: &ControlAdmission,
) -> Result<Vec<SourceResult>, String> {
    admission.validate()?;
    control.validate()?;
    // Card CG30-DIALECTS-FLAGS-TYPES (rule 11y): the unit's own target
    // selections, under the readers' LP64 premise.
    crate::c_frontend_identity::CTargetIdentity::admit_flags(
        unit.preprocessor_options
            .iter()
            .filter_map(|option| match option {
                crate::legacy_c::CPreprocessorOption::FrontendFlag(flag) => Some(flag),
                _ => None,
            }),
    )?;
    let source = validate_unrestricted_c11(unit).map_err(|error| error.to_string())?;
    // Field IDs from independent Clang invocations are not stable. Validate the
    // selected complete records by name/type, then bind each MemberExpr to its
    // typed base and direct field (anonymous/union/bitfield access is rejected).
    let mut fields = BTreeMap::new();
    for &record in profile.records {
        let declarations = ast_declarations(&source, unit, record)?;
        let definitions: Vec<_> = declarations
            .iter()
            .filter(|node| {
                kind(node) == "RecordDecl"
                    && node["name"] == record
                    && node["completeDefinition"] == true
                    && node["tagUsed"] == "struct"
            })
            .collect();
        if let [definition] = definitions.as_slice() {
            for field in children(definition)
                .iter()
                .filter(|node| kind(node) == "FieldDecl")
            {
                if field["isBitfield"] != true
                    && children(field).is_empty()
                    && let (Some(name), Ok(ty)) = (field["name"].as_str(), c_type(field))
                {
                    fields.insert((format!("struct {record} *"), name.to_owned()), ty);
                }
            }
        }
    }
    Ok(functions
        .iter()
        .map(|function| {
            let result = function_ast(&source, unit, function).and_then(|mut ast| {
                // Card CG30-DIALECTS-FLAGS-TYPES (rule 11y): an extension type or
                // a registered dialect difference is refused by name first.
                if let Some(refusal) =
                    crate::c_frontend_identity::typed_source_refusal(unit.dialect, &ast)
                {
                    return Err(refusal);
                }
                normalize_result_type(&mut ast, &source, unit)?;
                let constants = enum_constants(&ast, &source, unit)?;
                let arrays = array_types(&ast, &source, unit)?;
                derive(
                    &ast,
                    function,
                    source_revision,
                    &fields,
                    &constants,
                    &arrays,
                    (profile, admission, control),
                )
            });
            match result {
                Ok(summary) => SourceResult::Summary(summary),
                Err(reason) => SourceResult::Unknown {
                    function: (*function).into(),
                    reason,
                },
            }
        })
        .collect())
}

// Resolve a scalar return typedef through this same Clang input. The ABI
// width is not inferred here; it remains an explicit consumer obligation.
fn normalize_result_type(
    ast: &mut Value,
    source: &str,
    unit: &C11TranslationUnit<'_>,
) -> Result<(), String> {
    let signature = c_type(ast)?;
    let (result, rest) = signature
        .split_once('(')
        .ok_or("missing function signature")?;
    let result = result.trim();
    if matches!(
        result,
        "void"
            | "void *"
            | "int"
            | "char *"
            | "unsigned int"
            | "unsigned long"
            | "unsigned long long"
    ) {
        return Ok(());
    }
    let declarations = ast_declarations(source, unit, result)?;
    let exact: Vec<_> = declarations
        .iter()
        .filter(|node| kind(node) == "TypedefDecl" && node["name"] == result)
        .collect();
    let [decl] = exact.as_slice() else {
        return Err("unresolved scalar result typedef".into());
    };
    let resolved = c_type(decl)?;
    if !matches!(
        resolved.as_str(),
        "unsigned int" | "unsigned long" | "unsigned long long"
    ) {
        return Err("result typedef outside unsigned scalar profile".into());
    }
    ast["type"]["desugaredQualType"] = Value::String(format!("{resolved} ({rest}"));
    Ok(())
}

// Clang preserves typedef spelling inside arrays. Resolve their element type
// from declarations in the same source rather than assuming a socket ABI.
fn array_types(
    ast: &Value,
    source: &str,
    unit: &C11TranslationUnit<'_>,
) -> Result<BTreeSet<String>, String> {
    fn collect(node: &Value, types: &mut BTreeSet<String>) {
        if kind(node) == "VarDecl"
            && let Ok(ty) = c_type(node)
            && ty.ends_with("[2]")
        {
            types.insert(ty);
        }
        for child in children(node) {
            collect(child, types);
        }
    }
    let mut types = BTreeSet::from(["int[2]".into()]);
    collect(ast, &mut types);
    for ty in &types {
        let base = ty.strip_suffix("[2]").unwrap();
        if base == "int" {
            continue;
        }
        let declarations = ast_declarations(source, unit, base)?;
        let exact: Vec<_> = declarations
            .iter()
            .filter(|node| kind(node) == "TypedefDecl" && node["name"] == base)
            .collect();
        if !matches!(exact.as_slice(), [decl] if c_type(decl).as_deref() == Ok("int")) {
            return Err("stack array element is not an unambiguous int typedef".into());
        }
    }
    Ok(types)
}

// Resolve enum values from this same preprocessed input, never from host
// constants: an explicit initializer's evaluated `ConstantExpr`, or (card
// CG08, N11) Clang's evaluation of an implicit enumerator. Ambiguous names
// (including shadowing) remain outside this profile. Clang's constant
// evaluator is a trusted boundary.
fn enum_constants(
    ast: &Value,
    source: &str,
    unit: &C11TranslationUnit<'_>,
) -> Result<BTreeMap<String, SourceValue>, String> {
    fn references(node: &Value, names: &mut BTreeSet<String>) {
        if kind(node) == "DeclRefExpr"
            && node["referencedDecl"]["kind"] == "EnumConstantDecl"
            && let Some(name) = node["referencedDecl"]["name"].as_str()
        {
            names.insert(name.into());
        }
        for child in children(node) {
            references(child, names);
        }
    }
    let mut names = BTreeSet::new();
    references(ast, &mut names);
    let mut constants = BTreeMap::new();
    for name in names {
        let declarations = ast_declarations(source, unit, &name)?;
        let exact: Vec<_> = declarations
            .iter()
            .filter(|node| kind(node) == "EnumConstantDecl" && node["name"] == name)
            .collect();
        let [decl] = exact.as_slice() else {
            return Err("enum constant requires one unambiguous declaration".into());
        };
        let [initial] = children(decl) else {
            // Card CG08-CASTS-AND-TYPE-QUERIES (N11): an implicit value is
            // Clang's own evaluation of the enumerator, never the host's
            // count from its predecessor; an unevaluable one refuses.
            let integer = crate::cast_representation::clang_constant(source, unit, &name)
                .map_err(|_| "enum constant requires a Clang-evaluated value")?;
            constants.insert(
                name,
                value(
                    c_type(decl)?,
                    SourceExpression::Integer {
                        value: integer.to_string(),
                    },
                ),
            );
            continue;
        };
        check_source_rule(
            "source_enum_value",
            &[
                (
                    "evaluated_constant",
                    i128::from(kind(initial) == "ConstantExpr"),
                ),
                (
                    "type_matches",
                    i128::from(c_type(initial)? == c_type(decl)?),
                ),
            ],
        )?;
        let integer = initial["value"]
            .as_str()
            .ok_or("missing evaluated enum value")?;
        integer
            .parse::<i128>()
            .map_err(|_| "enum value outside integer profile")?;
        constants.insert(
            name,
            value(
                c_type(decl)?,
                SourceExpression::Integer {
                    value: integer.into(),
                },
            ),
        );
    }
    Ok(constants)
}

/// Card CG30-DIALECTS-FLAGS-TYPES (rule 11y): route T's canonical type
/// identity of a node, read from the spelling [`c_type`] reads (before its
/// qualifier refusals, which stay [`c_type`]'s).
pub fn canonical_type_id(node: &Value) -> Option<crate::c_frontend_identity::CTypeId> {
    crate::c_frontend_identity::CTypeId::from_clang_type(node)
}

fn c_type(node: &Value) -> Result<String, String> {
    qual_type(&node["type"])
}

fn qual_type(ty: &Value) -> Result<String, String> {
    let ty = ty["desugaredQualType"]
        .as_str()
        .or_else(|| ty["qualType"].as_str())
        .ok_or("missing C type")?;
    // Card CG19-ATOMIC-SOURCE-ROUTE (gateway rule 11i): `_Atomic` has its
    // own arm, asked first (a `volatile _Atomic` object is atomic). The
    // `volatile` and `restrict` arm below is unchanged (CG15, CG18).
    if ty.contains("_Atomic") {
        return Err(crate::atomic_source_route::typed_source_qualifier_refusal(
            ty,
        ));
    }
    if ty.contains("volatile") || ty.contains("_Atomic") || ty.contains("restrict") {
        return Err("qualified access requires additional synchronization/alias evidence".into());
    }
    Ok(ty.into())
}

fn value(ty: String, expression: SourceExpression) -> SourceValue {
    SourceValue {
        c_type: ty,
        expression,
    }
}

#[derive(Clone)]
enum Expr {
    Local(String, String),
    Literal(SourceValue),
    Cast(String, String, Box<Self>),
    Unary(String, String, Box<Self>),
    Binary(String, String, Box<Self>, Box<Self>),
    Field(String, Box<Self>, String),
    Index(String, Box<Self>, Box<Self>),
    Read(String, Box<Self>),
    Call(String, String, String, Vec<Self>),
    Aggregate(String, Vec<Self>),
}

#[derive(Clone)]
enum Stmt {
    Block(Vec<Self>),
    Declare(String, String, Option<Expr>),
    Assign(Expr, Expr),
    OrAssign(Expr, Expr),
    /// CG01 compound assignment: place, operator (without `=`), place type,
    /// Clang's computation LHS and result types, right operand.
    Compound(Expr, String, String, String, String, Expr),
    Increment(Expr, String, String),
    Call(Expr),
    NoReturnCall(Expr),
    DoOnce(Box<Self>, Expr),
    ScanLoop(Box<Self>, String, String, Expr, Expr, Expr),
    If(Expr, Box<Self>, Option<Box<Self>>),
    Return(Option<Expr>),
    /// CG09: the controlling value (evaluated once) and the switch body's
    /// top level, flattened into labels and statements.
    Switch(Expr, Vec<control::SwitchItem>),
    /// CG09: a label (Clang's declaration ID) on its statement.
    Label(String, Box<Self>),
    /// CG09: `break`, `continue` or a forward `goto`.
    Jump(control::Jump),
}

struct Importer<'a> {
    admission: &'a NodeAdmission,
    control: &'a ControlAdmission,
    /// Labels already read, in source order: a `goto` to one is backward.
    labels: BTreeSet<String>,
    fields: &'a BTreeMap<(String, String), String>,
    constants: &'a BTreeMap<String, SourceValue>,
    allowed_calls: Option<&'a [&'a str]>,
    sizeof_types: &'a [&'a str],
    arrays: &'a BTreeSet<String>,
    locals: BTreeSet<String>,
    nodes: usize,
}

impl Importer<'_> {
    fn budget(&mut self, depth: usize) -> Result<(), String> {
        self.nodes += 1;
        if depth > MAX_DEPTH || self.nodes > MAX_NODES {
            return Err("callback exceeds bounded source import size/depth".into());
        }
        Ok(())
    }

    fn expression(&mut self, node: &Value, depth: usize) -> Result<Expr, String> {
        self.budget(depth)?;
        let ty = c_type(node)?;
        let inner = children(node);
        match (kind(node), inner) {
            ("DeclRefExpr", []) if node["referencedDecl"]["kind"] == "EnumConstantDecl" => {
                let constant = node["referencedDecl"]["name"]
                    .as_str()
                    .and_then(|name| self.constants.get(name))
                    .filter(|constant| constant.c_type == ty)
                    .ok_or("unresolved or mismatched enum constant")?;
                Ok(Expr::Literal(constant.clone()))
            }
            ("InitListExpr", elements) if ty == "struct timeval" => {
                if elements.len() != 2
                    || !self
                        .fields
                        .contains_key(&("struct timeval *".into(), "tv_sec".into()))
                    || !self
                        .fields
                        .contains_key(&("struct timeval *".into(), "tv_usec".into()))
                {
                    return Err("record initializer outside fixed timeval profile".into());
                }
                Ok(Expr::Aggregate(
                    ty,
                    elements
                        .iter()
                        .map(|element| self.expression(element, depth + 1))
                        .collect::<Result<_, _>>()?,
                ))
            }
            ("DeclRefExpr", [])
                if matches!(
                    node["referencedDecl"]["kind"].as_str(),
                    Some("VarDecl" | "ParmVarDecl")
                ) =>
            {
                let id = node["referencedDecl"]["id"]
                    .as_str()
                    .ok_or("missing local ID")?;
                if !self.locals.contains(id) {
                    return Err("global or unresolved local reference".into());
                }
                Ok(Expr::Local(id.into(), ty))
            }
            ("DeclRefExpr", []) if node["referencedDecl"]["kind"] == "FunctionDecl" => {
                Ok(Expr::Literal(value(
                    ty.clone(),
                    SourceExpression::Function {
                        name: node["referencedDecl"]["name"]
                            .as_str()
                            .ok_or("missing function identity")?
                            .into(),
                        signature: ty,
                    },
                )))
            }
            ("IntegerLiteral", []) | ("StringLiteral", []) => {
                let literal = node["value"].as_str().ok_or("missing literal")?.to_owned();
                Ok(Expr::Literal(value(
                    ty,
                    if kind(node) == "IntegerLiteral" {
                        SourceExpression::Integer { value: literal }
                    } else {
                        SourceExpression::String { value: literal }
                    },
                )))
            }
            ("UnaryExprOrTypeTraitExpr", []) if node["name"] == "sizeof" => {
                let argument_type = node["argType"]["desugaredQualType"]
                    .as_str()
                    .or_else(|| node["argType"]["qualType"].as_str())
                    .ok_or("unsupported sizeof operand")?;
                // Reject variably modified sizeof, whose operand can execute.
                if !self.sizeof_types.contains(&argument_type) {
                    return Err("sizeof outside fixed scalar type profile".into());
                }
                Ok(Expr::Literal(value(
                    ty,
                    SourceExpression::SizeOf {
                        argument_type: argument_type.into(),
                    },
                )))
            }
            ("ParenExpr", [child]) => self.expression(child, depth + 1),
            ("PredefinedExpr", [child]) if kind(child) == "StringLiteral" => {
                self.expression(child, depth + 1)
            }
            ("ImplicitCastExpr", [child]) if node["castKind"] == "LValueToRValue" => {
                Ok(Expr::Read(ty, Box::new(self.expression(child, depth + 1)?)))
            }
            ("ImplicitCastExpr" | "CStyleCastExpr", [child])
                if matches!(
                    node["castKind"].as_str(),
                    Some(
                        "BitCast"
                            | "IntegralCast"
                            | "NoOp"
                            | "ArrayToPointerDecay"
                            | "NullToPointer"
                            | "FunctionToPointerDecay"
                    )
                ) =>
            {
                Ok(Expr::Cast(
                    ty,
                    node["castKind"].as_str().unwrap().into(),
                    Box::new(self.expression(child, depth + 1)?),
                ))
            }
            ("MemberExpr", [base]) if node["isArrow"] == true => {
                let name = node["name"].as_str().ok_or("missing field name")?;
                if self.fields.get(&(c_type(base)?, name.into())) != Some(&ty) {
                    return Err("field is not a direct ordinary field of a selected record".into());
                }
                Ok(Expr::Field(
                    ty,
                    Box::new(self.expression(base, depth + 1)?),
                    name.into(),
                ))
            }
            ("MemberExpr", [base]) if node["isArrow"] == false => {
                let name = node["name"].as_str().ok_or("missing field name")?;
                let pointer_type = format!("{} *", c_type(base)?);
                if self.fields.get(&(pointer_type.clone(), name.into())) != Some(&ty) {
                    return Err("dot field outside selected ordinary record layout".into());
                }
                Ok(Expr::Field(
                    ty,
                    Box::new(Expr::Unary(
                        pointer_type,
                        "&".into(),
                        Box::new(self.expression(base, depth + 1)?),
                    )),
                    name.into(),
                ))
            }
            ("ArraySubscriptExpr", [base, index]) => Ok(Expr::Index(
                ty,
                Box::new(self.expression(base, depth + 1)?),
                Box::new(self.expression(index, depth + 1)?),
            )),
            ("UnaryOperator", [child])
                if matches!(node["opcode"].as_str(), Some("&" | "*" | "-" | "!")) =>
            {
                Ok(Expr::Unary(
                    ty,
                    node["opcode"].as_str().unwrap().into(),
                    Box::new(self.expression(child, depth + 1)?),
                ))
            }
            ("BinaryOperator", [left, right])
                if matches!(
                    node["opcode"].as_str(),
                    Some("&" | "|" | "==" | "!=" | "<" | ">" | "+" | "*" | "/" | "%" | "&&" | "||")
                ) =>
            {
                Ok(Expr::Binary(
                    ty,
                    node["opcode"].as_str().unwrap().into(),
                    Box::new(self.expression(left, depth + 1)?),
                    Box::new(self.expression(right, depth + 1)?),
                ))
            }
            ("CallExpr", [callee, arguments @ ..]) => {
                let [reference] = children(callee) else {
                    return Err("indirect or unmodeled call".into());
                };
                if kind(callee) != "ImplicitCastExpr"
                    || !matches!(
                        callee["castKind"].as_str(),
                        Some("FunctionToPointerDecay" | "BuiltinFnToFnPtr")
                    )
                    || kind(reference) != "DeclRefExpr"
                    || reference["referencedDecl"]["kind"] != "FunctionDecl"
                {
                    return Err("indirect or unmodeled call".into());
                }
                let name = reference["referencedDecl"]["name"]
                    .as_str()
                    .ok_or("missing call name")?;
                // This list bounds syntax only. It does not assume these calls
                // succeed or have a particular custody/destruction effect.
                if self
                    .allowed_calls
                    .is_some_and(|calls| !calls.contains(&name))
                {
                    return Err(format!(
                        "call {name} is outside the callback source profile"
                    ));
                }
                Ok(Expr::Call(
                    ty,
                    name.into(),
                    c_type(reference)?,
                    arguments
                        .iter()
                        .map(|arg| self.expression(arg, depth + 1))
                        .collect::<Result<_, _>>()?,
                ))
            }
            _ => self.extended_expression(node, ty, depth),
        }
    }

    // CG01: a node outside the pre-CG01 grammar is read only if it is a CG01
    // node the consumer admits (rule 10r), with promoted integer operands.
    fn extended_expression(
        &mut self,
        node: &Value,
        ty: String,
        depth: usize,
    ) -> Result<Expr, String> {
        // CG02-ELABORATION-CORE (rule 10s): a value form is named. No route-T
        // consumer decides one (CG01's census), so none is read.
        if let Some(form) = crate::elaboration::value_form(node) {
            return Err(crate::elaboration::t_refusal(form));
        }
        // CG19-ATOMIC-SOURCE-ROUTE (rule 11i): an atomic builtin is named by
        // the atomic route's answer. No route-T consumer reads a routed one.
        if let Some(reason) = crate::atomic_source_route::typed_source_refusal(node) {
            return Err(reason);
        }
        let Some(spelling) = typed_source_nodes::expression_spelling(node) else {
            return Err(format!("unmodeled callback expression {}", kind(node)));
        };
        self.admission.admit(&spelling)?;
        let operands = children(node)
            .iter()
            .map(c_type)
            .collect::<Result<Vec<_>, _>>()?;
        typed_source_nodes::check_integer_types(&spelling, &ty, &operands)?;
        let opcode = || node["opcode"].as_str().unwrap_or_default().to_owned();
        match (kind(node), children(node)) {
            ("BinaryOperator", [left, right]) => Ok(Expr::Binary(
                ty,
                opcode(),
                Box::new(self.expression(left, depth + 1)?),
                Box::new(self.expression(right, depth + 1)?),
            )),
            ("UnaryOperator", [child]) => Ok(Expr::Unary(
                ty,
                opcode(),
                Box::new(self.expression(child, depth + 1)?),
            )),
            ("FloatingLiteral", []) => {
                let literal = node["value"].as_str().ok_or("missing floating literal")?;
                Ok(Expr::Literal(value(
                    ty,
                    SourceExpression::Floating {
                        value: literal.into(),
                    },
                )))
            }
            ("CharacterLiteral", []) => {
                let code = node["value"]
                    .as_i64()
                    .ok_or("missing character literal code")?;
                Ok(Expr::Literal(value(
                    ty,
                    SourceExpression::Character {
                        value: code.to_string(),
                    },
                )))
            }
            _ => Err(format!("unmodeled callback expression {}", kind(node))),
        }
    }

    // CG01: a compound assignment statement the consumer admits. The left
    // operand is read once; the computation types are Clang's.
    fn compound(
        &mut self,
        node: &Value,
        left: &Value,
        right: &Value,
        depth: usize,
    ) -> Result<Stmt, String> {
        let spelling =
            typed_source_nodes::compound_spelling(node).ok_or("unmodeled compound assignment")?;
        self.admission.admit(&spelling)?;
        let place = c_type(left)?;
        let compute_lhs = qual_type(&node["computeLHSType"])?;
        let compute_result = qual_type(&node["computeResultType"])?;
        typed_source_nodes::check_compound_types(
            &spelling,
            &place,
            &c_type(right)?,
            &compute_lhs,
            &compute_result,
        )?;
        if c_type(node)? != place {
            return Err("compound assignment type differs from its place".into());
        }
        let operator = node["opcode"]
            .as_str()
            .and_then(|op| op.strip_suffix('='))
            .ok_or("unmodeled compound assignment")?;
        Ok(Stmt::Compound(
            self.expression(left, depth + 1)?,
            operator.into(),
            place,
            compute_lhs,
            compute_result,
            self.expression(right, depth + 1)?,
        ))
    }

    fn admits_compound(&self, node: &Value) -> bool {
        typed_source_nodes::compound_spelling(node)
            .is_some_and(|spelling| self.admission.admit(&spelling).is_ok())
    }

    // A compound assignment this consumer does not admit: named when it is a
    // CG01 node, the generic statement refusal otherwise.
    fn compound_refusal(&self, node: &Value) -> String {
        typed_source_nodes::compound_spelling(node)
            .and_then(|spelling| self.admission.admit(&spelling).err())
            .unwrap_or_else(|| format!("unmodeled callback statement {}", kind(node)))
    }

    // A canonical scan loop, the only `for` shape route T projects.
    fn scan_loop(&mut self, node: &Value, depth: usize) -> Result<Stmt, String> {
        let [initial, condition_variable, condition, increment, body] = children(node) else {
            return Err("scan loop requires a five-part for statement".into());
        };
        if !kind(condition_variable).is_empty() {
            return Err("scan loop has a condition variable".into());
        }
        let [index] = children(increment) else {
            return Err("scan loop requires a local increment".into());
        };
        let Expr::Local(id, ty) = self.expression(index, depth + 1)? else {
            return Err("scan index is not local".into());
        };
        if kind(increment) != "UnaryOperator"
            || increment["opcode"] != "++"
            || !matches!(
                ty.as_str(),
                "int" | "unsigned int" | "unsigned long" | "unsigned long long"
            )
        {
            return Err("scan loop requires a supported integer increment".into());
        }
        let initial = self.statement(initial, depth + 1)?;
        if !matches!(&initial, Stmt::Assign(Expr::Local(target, _), _) if target == &id) {
            return Err("scan initializer must assign its induction local".into());
        }
        let condition = self.expression(condition, depth + 1)?;
        let body = self.statement(body, depth + 1)?;
        fn single(stmt: &Stmt) -> &Stmt {
            if let Stmt::Block(statements) = stmt
                && let [only] = statements.as_slice()
            {
                single(only)
            } else {
                stmt
            }
        }
        let Stmt::If(matches, yes, None) = single(&body) else {
            return Err("scan body requires one conditional early return".into());
        };
        let Stmt::Return(Some(result)) = single(yes) else {
            return Err("scan match must return from the function".into());
        };
        check_source_rule(
            "source_scan_effects",
            &[
                ("header_calls", i128::from(effects(&condition).1)),
                ("match_calls", i128::from(effects(matches).1)),
                ("return_calls", i128::from(effects(result).1)),
            ],
        )?;
        Ok(Stmt::ScanLoop(
            Box::new(initial),
            id,
            ty,
            condition,
            matches.clone(),
            result.clone(),
        ))
    }

    fn statement(&mut self, node: &Value, depth: usize) -> Result<Stmt, String> {
        self.budget(depth)?;
        match (kind(node), children(node)) {
            ("CompoundStmt", statements) => Ok(Stmt::Block(
                statements
                    .iter()
                    .map(|stmt| self.statement(stmt, depth + 1))
                    .collect::<Result<_, _>>()?,
            )),
            ("NullStmt", []) => Ok(Stmt::Block(vec![])),
            // CG09 (rule 11l): a `for` that is not a canonical scan is a
            // general loop. Its back-edge carries no invariant, so it is
            // refused by name, with the scan shape's own reason.
            ("ForStmt", [_, _, _, _, _]) => self
                .scan_loop(node, depth)
                .map_err(|scan| self.control.back_edge_refusal("loop for", Some(&scan))),
            ("DoStmt", [body, condition]) => {
                // Macro-style single execution only. Parse the entire body;
                // `break` and `continue` target this statement (CG09), and a
                // nonliteral condition is a loop, refused by name.
                let mut literal = condition;
                while kind(literal) == "ParenExpr" && children(literal).len() == 1 {
                    literal = &children(literal)[0];
                }
                if kind(literal) != "IntegerLiteral" || literal["value"] != "0" {
                    return Err(self.control.back_edge_refusal("loop do", None));
                }
                Ok(Stmt::DoOnce(
                    Box::new(self.statement(body, depth + 1)?),
                    self.expression(condition, depth + 1)?,
                ))
            }
            ("DeclStmt", [decl])
                if kind(decl) == "VarDecl" && decl.get("storageClass").is_none() =>
            {
                let id = decl["id"]
                    .as_str()
                    .ok_or("missing declaration ID")?
                    .to_owned();
                let ty = c_type(decl)?;
                // Fixed storage only: no VLA, implicit aggregate initialization,
                // or destructor attributes. Array cells are explicit memory reads.
                if !(ty.ends_with('*')
                    || self.arrays.contains(&ty)
                    || matches!(
                        ty.as_str(),
                        "int"
                            | "short"
                            | "unsigned int"
                            | "unsigned long"
                            | "unsigned long long"
                            | "long"
                            | "int[2]"
                            | "struct timeval"
                    ))
                {
                    return Err("local outside scalar/pointer callback profile".into());
                }
                let initial = match children(decl) {
                    [] => None,
                    [expr] => Some(self.expression(expr, depth + 1)?),
                    _ => return Err("unmodeled local initialization/attributes".into()),
                };
                self.locals.insert(id.clone());
                Ok(Stmt::Declare(id, ty, initial))
            }
            ("BinaryOperator", [left, right]) if node["opcode"] == "=" => Ok(Stmt::Assign(
                self.expression(left, depth + 1)?,
                self.expression(right, depth + 1)?,
            )),
            ("CompoundAssignOperator", [left, right]) if self.admits_compound(node) => {
                self.compound(node, left, right, depth)
            }
            ("CompoundAssignOperator", [left, right]) if node["opcode"] == "|=" => {
                if c_type(left)? != "int" || c_type(right)? != "int" {
                    return Err("compound assignment requires int operands".into());
                }
                Ok(Stmt::OrAssign(
                    self.expression(left, depth + 1)?,
                    self.expression(right, depth + 1)?,
                ))
            }
            ("UnaryOperator", [place])
                if matches!(node["opcode"].as_str(), Some("++" | "--"))
                    && matches!(
                        c_type(node)?.as_str(),
                        "int" | "unsigned int" | "unsigned long" | "unsigned long long"
                    ) =>
            {
                let operator = if node["opcode"] == "++" { "+" } else { "-" };
                Ok(Stmt::Increment(
                    self.expression(place, depth + 1)?,
                    operator.into(),
                    c_type(node)?,
                ))
            }
            ("IfStmt", [condition, yes]) if node["hasElse"] != true => Ok(Stmt::If(
                self.expression(condition, depth + 1)?,
                Box::new(self.statement(yes, depth + 1)?),
                None,
            )),
            ("IfStmt", [condition, yes, no]) if node["hasElse"] == true => Ok(Stmt::If(
                self.expression(condition, depth + 1)?,
                Box::new(self.statement(yes, depth + 1)?),
                Some(Box::new(self.statement(no, depth + 1)?)),
            )),
            ("CallExpr", _) => {
                let expr = self.expression(node, depth + 1)?;
                if matches!(&expr, Expr::Call(_, _, signature, _) if signature.ends_with(" __attribute__((noreturn))"))
                {
                    Ok(Stmt::NoReturnCall(expr))
                } else {
                    Ok(Stmt::Call(expr))
                }
            }
            ("ReturnStmt", []) => Ok(Stmt::Return(None)),
            ("ReturnStmt", [result]) => Ok(Stmt::Return(Some(self.expression(result, depth + 1)?))),
            ("CompoundAssignOperator", _) => Err(self.compound_refusal(node)),
            // Card CG15-STORAGE-DURATION-AND-VOLATILE (rule 11h): no
            // typed-source consumer decides a static- or thread-duration
            // object's lifetime, so T names the form and refuses it.
            ("DeclStmt", declarations)
                if declarations.iter().any(|declaration| {
                    crate::storage_duration::storage_of(declaration, true)
                        .is_some_and(crate::storage_duration::Storage::outlives_frame)
                }) =>
            {
                Err(crate::storage_duration::T_STATIC_REFUSAL.to_owned())
            }
            // CG09 (rule 11l): control statements, each admitted or refused
            // by name; any other statement keeps the generic refusal.
            _ => self.control_statement(node, depth),
        }
    }
}

#[derive(Clone, Default)]
struct Path {
    locals: BTreeMap<String, (usize, String, Option<SourceValue>)>,
    addressed: BTreeSet<String>,
    operations: Vec<SourceOperation>,
    returned: bool,
    terminated: bool,
    return_value: Option<SourceValue>,
    next_local: usize,
    /// CG09: a pending `break`, `continue` or forward `goto`. Statements are
    /// skipped until its target takes it; every scope it leaves ends its
    /// locals on the way.
    jump: Option<control::Jump>,
}

impl Path {
    fn push(&mut self, operation: SourceOperation) -> Result<usize, String> {
        if self.operations.len() >= MAX_OPERATIONS {
            return Err("callback operation limit exceeded".into());
        }
        let index = self.operations.len();
        self.operations.push(operation);
        Ok(index)
    }

    fn read(&mut self, expression: &Expr, ty: &str) -> Result<SourceValue, String> {
        if let Expr::Local(id, _) = expression {
            return self
                .locals
                .get(id)
                .and_then(|local| local.2.clone())
                .ok_or_else(|| "uninitialized, escaped, or out-of-scope local read".into());
        }
        let place = self.evaluate(expression)?;
        let operation = self.push(SourceOperation::Read { place })?;
        Ok(value(ty.into(), SourceExpression::Symbol { operation }))
    }

    fn evaluate(&mut self, expression: &Expr) -> Result<SourceValue, String> {
        let result = match expression {
            Expr::Local(id, ty) => {
                let local = self.locals.get(id).ok_or("local outside its scope")?;
                value(
                    ty.clone(),
                    SourceExpression::LocalStorage { local: local.0 },
                )
            }
            Expr::Literal(value) => value.clone(),
            Expr::Aggregate(ty, elements) => {
                reject_unsequenced(&elements.iter().collect::<Vec<_>>())?;
                value(
                    ty.clone(),
                    SourceExpression::Aggregate {
                        elements: elements
                            .iter()
                            .map(|element| self.evaluate(element))
                            .collect::<Result<_, _>>()?,
                    },
                )
            }
            Expr::Read(ty, place) => self.read(place, ty)?,
            Expr::Cast(ty, cast, child) => value(
                ty.clone(),
                SourceExpression::Cast {
                    cast: cast.clone(),
                    value: Box::new(self.evaluate(child)?),
                },
            ),
            Expr::Unary(ty, operator, child) => {
                if operator == "&"
                    && let Expr::Local(id, _) = child.as_ref()
                {
                    self.addressed.insert(id.clone());
                }
                value(
                    ty.clone(),
                    SourceExpression::Unary {
                        operator: operator.clone(),
                        value: Box::new(self.evaluate(child)?),
                    },
                )
            }
            Expr::Binary(_, operator, _, _) if matches!(operator.as_str(), "&&" | "||") => {
                return Err("short-circuit values require a supported branch condition".into());
            }
            Expr::Binary(ty, operator, left, right) => {
                reject_unsequenced(&[left, right])?;
                value(
                    ty.clone(),
                    SourceExpression::Binary {
                        operator: operator.clone(),
                        left: Box::new(self.evaluate(left)?),
                        right: Box::new(self.evaluate(right)?),
                    },
                )
            }
            Expr::Field(ty, base, name) => value(
                ty.clone(),
                SourceExpression::Field {
                    base: Box::new(self.evaluate(base)?),
                    name: name.clone(),
                },
            ),
            Expr::Index(ty, base, index) => {
                reject_unsequenced(&[base, index])?;
                value(
                    ty.clone(),
                    SourceExpression::Index {
                        base: Box::new(self.evaluate(base)?),
                        index: Box::new(self.evaluate(index)?),
                    },
                )
            }
            Expr::Call(ty, function, signature, arguments) => {
                if signature.contains("noreturn") {
                    return Err("nonreturning call requires a terminal statement".into());
                }
                self.call(ty, function, signature, arguments)?
            }
        };
        check_value_size(&result)?;
        Ok(result)
    }

    fn call(
        &mut self,
        ty: &str,
        function: &str,
        signature: &str,
        arguments: &[Expr],
    ) -> Result<SourceValue, String> {
        reject_unsequenced(&arguments.iter().collect::<Vec<_>>())?;
        let arguments = arguments
            .iter()
            .map(|arg| self.evaluate(arg))
            .collect::<Result<_, _>>()?;
        let operation = self.push(SourceOperation::Call {
            function: function.into(),
            signature: signature.into(),
            arguments,
            result_type: ty.into(),
        })?;
        // Calls may change any escaped local, including through const aliases.
        self.invalidate_addressed();
        Ok(value(ty.into(), SourceExpression::Symbol { operation }))
    }

    fn update(&mut self, place: &Expr, operator: &str, right: SourceValue) -> Result<(), String> {
        let ty = right.c_type.clone();
        self.update_with(place, &ty, |left| {
            Ok(value(
                ty.clone(),
                SourceExpression::Binary {
                    operator: operator.into(),
                    left: Box::new(left),
                    right: Box::new(right),
                },
            ))
        })
    }

    // CG01 compound assignment: one place evaluation, the old value read at
    // the place type, converted to Clang's computation LHS type, the operation
    // at the computation result type, the conversion back, and the writeback.
    // Each conversion is an explicit IntegralCast, never host-evaluated.
    fn compound(
        &mut self,
        place: &Expr,
        operator: &str,
        types: [&str; 3],
        right: SourceValue,
    ) -> Result<(), String> {
        let [place_type, compute_lhs, compute_result] = types;
        let convert = |converted: SourceValue, ty: &str| {
            if converted.c_type == ty {
                converted
            } else {
                value(
                    ty.into(),
                    SourceExpression::Cast {
                        cast: "IntegralCast".into(),
                        value: Box::new(converted),
                    },
                )
            }
        };
        self.update_with(place, place_type, |left| {
            if left.c_type != place_type {
                return Err("compound assignment old value differs from its place type".into());
            }
            let result = value(
                compute_result.into(),
                SourceExpression::Binary {
                    operator: operator.into(),
                    left: Box::new(convert(left, compute_lhs)),
                    right: Box::new(right),
                },
            );
            Ok(convert(result, place_type))
        })
    }

    fn update_with(
        &mut self,
        place: &Expr,
        ty: &str,
        operation: impl FnOnce(SourceValue) -> Result<SourceValue, String>,
    ) -> Result<(), String> {
        // C evaluates a compound-assignment lvalue only once. Preserve the
        // resolved address rather than introducing a second nested field read.
        let resolved = if matches!(place, Expr::Local(..)) {
            None
        } else {
            Some(self.evaluate(place)?)
        };
        let left = if let Some(place) = &resolved {
            let operation = self.push(SourceOperation::Read {
                place: place.clone(),
            })?;
            value(ty.into(), SourceExpression::Symbol { operation })
        } else {
            self.read(place, ty)?
        };
        let result = operation(left)?;
        check_value_size(&result)?;
        if let Some(place) = resolved {
            self.push(SourceOperation::Write {
                place,
                value: result,
            })?;
            self.invalidate_addressed();
            Ok(())
        } else {
            self.write(place, result)
        }
    }

    fn invalidate_addressed(&mut self) {
        for id in &self.addressed {
            if let Some(local) = self.locals.get_mut(id) {
                local.2 = None;
            }
        }
    }

    fn write(&mut self, place: &Expr, result: SourceValue) -> Result<(), String> {
        check_value_size(&result)?;
        if let Expr::Local(id, _) = place {
            let local = self
                .locals
                .get_mut(id)
                .ok_or("assignment outside local scope")?;
            local.2 = Some(result.clone());
            let index = local.0;
            self.push(SourceOperation::LocalWrite {
                local: index,
                value: result,
            })?;
        } else {
            let place = self.evaluate(place)?;
            self.push(SourceOperation::Write {
                place,
                value: result,
            })?;
            // An indirect write may alias any address-taken local.
            self.invalidate_addressed();
        }
        Ok(())
    }
}

// Bound alias substitution as well as the input AST: repeated x = x | x
// must not allocate an exponentially growing symbolic report.
fn visit_value(
    value: &SourceValue,
    mut visit: impl FnMut(&SourceValue) -> Result<(), String>,
) -> Result<(), String> {
    let mut pending = vec![(value, 0)];
    let mut count = 0;
    while let Some((value, depth)) = pending.pop() {
        count += 1;
        if count > 2048 || depth > 128 {
            return Err("symbolic expression limit exceeded".into());
        }
        visit(value)?;
        match &value.expression {
            SourceExpression::Cast { value, .. } | SourceExpression::Unary { value, .. } => {
                pending.push((value, depth + 1))
            }
            SourceExpression::Field { base, .. } => pending.push((base, depth + 1)),
            SourceExpression::Binary { left, right, .. } => {
                pending.push((left, depth + 1));
                pending.push((right, depth + 1));
            }
            SourceExpression::Index { base, index } => {
                pending.push((base, depth + 1));
                pending.push((index, depth + 1));
            }
            SourceExpression::Aggregate { elements } => {
                pending.extend(elements.iter().map(|element| (element, depth + 1)));
            }
            _ => {}
        }
    }
    Ok(())
}

fn check_value_size(value: &SourceValue) -> Result<(), String> {
    visit_value(value, |_| Ok(()))
}

// Validate the symbolic-reference projection independently of construction.
// This establishes only prior typed producers, not memory/alias/call safety.
fn check_path_symbols(path: &SourcePath) -> Result<(), String> {
    let mut producers: Vec<Option<&str>> = Vec::new();
    for operation in &path.operations {
        if let SourceOperation::ScanLoop {
            local,
            initial,
            header,
            matches,
            early_return,
            increment,
        } = operation
        {
            check_references(initial, &producers, None)?;
            let mut iteration_producers = producers.clone();
            let scope = Some((*local, initial.c_type.as_str()));
            for region in [header, matches, early_return] {
                for operation in &region.operations {
                    check_source_rule(
                        "source_scan_region",
                        &[(
                            "read_only",
                            i128::from(matches!(operation, SourceOperation::Read { .. })),
                        )],
                    )?;
                    let SourceOperation::Read { place } = operation else {
                        return Err("scan region has no read place".into());
                    };
                    check_references(place, &iteration_producers, scope)?;
                    iteration_producers.push(Some(&place.c_type));
                }
                check_references(&region.result, &iteration_producers, scope)?;
            }
            check_references(increment, &iteration_producers, scope)?;
            producers.push(None);
            continue;
        }
        let (values, produced): (Vec<&SourceValue>, Option<&str>) = match operation {
            SourceOperation::ScanLoop { .. } => unreachable!(),
            SourceOperation::Read { place } => (vec![place], Some(&place.c_type)),
            SourceOperation::Write { place, value } => (vec![place, value], None),
            SourceOperation::LocalWrite { value, .. } | SourceOperation::ReturnValue { value } => {
                (vec![value], None)
            }
            SourceOperation::Call {
                arguments,
                result_type,
                ..
            } => (arguments.iter().collect(), Some(result_type)),
            SourceOperation::Guard { condition, .. } => (vec![condition], None),
            SourceOperation::LocalStart { .. }
            | SourceOperation::LocalEnd { .. }
            | SourceOperation::Return
            | SourceOperation::Terminate => (vec![], None),
        };
        for value in values {
            check_references(value, &producers, None)?;
        }
        producers.push(produced);
    }
    Ok(())
}

fn check_references(
    value: &SourceValue,
    producers: &[Option<&str>],
    loop_index: Option<(usize, &str)>,
) -> Result<(), String> {
    visit_value(value, |value| {
        match value.expression {
            SourceExpression::Symbol { operation } => {
                check_source_rule(
                    "source_symbol",
                    &[(
                        "producer_type_matches",
                        i128::from(
                            producers.get(operation).copied().flatten()
                                == Some(value.c_type.as_str()),
                        ),
                    )],
                )?;
            }
            SourceExpression::LoopIndex { local } => {
                check_source_rule(
                    "source_loop_index",
                    &[(
                        "scope_matches",
                        i128::from(loop_index == Some((local, value.c_type.as_str()))),
                    )],
                )?;
            }
            _ => {}
        }
        Ok(())
    })
}

// C does not sequence operands/arguments. Pure ordinary reads are projected in
// AST order under the exclusive-access boundary; an opaque call plus another
// read/call cannot be assigned an invented order.
fn effects(expr: &Expr) -> (bool, bool) {
    let combine = |a: (bool, bool), b: (bool, bool)| (a.0 || b.0, a.1 || b.1);
    match expr {
        Expr::Local(..) | Expr::Literal(..) => (false, false),
        Expr::Read(_, child) => combine((true, false), effects(child)),
        Expr::Cast(_, _, child) | Expr::Unary(_, _, child) | Expr::Field(_, child, _) => {
            effects(child)
        }
        Expr::Binary(_, _, left, right) | Expr::Index(_, left, right) => {
            combine(effects(left), effects(right))
        }
        Expr::Call(..) => (true, true),
        Expr::Aggregate(_, elements) => elements.iter().fold((false, false), |prior, element| {
            combine(prior, effects(element))
        }),
    }
}

fn reject_unsequenced(expressions: &[&Expr]) -> Result<(), String> {
    let effects: Vec<_> = expressions.iter().map(|expr| effects(expr)).collect();
    if effects.iter().any(|effect| effect.1) && effects.iter().filter(|effect| effect.0).count() > 1
    {
        Err("unsequenced call/read operands require an explicit ordering model".into())
    } else {
        Ok(())
    }
}

// Preserve C sequencing without adding eager logical operators to SourceValue.
// Only branch conditions receive this expansion; logical values in assignments,
// arguments, comparisons or scan regions remain Unknown. Cap every intermediate
// frontier as well as completed function paths.
fn branch_condition(condition: &Expr, path: Path, taken: bool) -> Result<Vec<Path>, String> {
    fn contains_logical(expr: &Expr) -> bool {
        match expr {
            Expr::Binary(_, op, _, _) if matches!(op.as_str(), "&&" | "||") => true,
            Expr::Unary(_, op, child) if op == "!" => contains_logical(child),
            _ => false,
        }
    }
    match condition {
        Expr::Unary(_, op, child) if op == "!" && contains_logical(child) => {
            branch_condition(child, path, !taken)
        }
        Expr::Binary(_, op, left, right) if matches!(op.as_str(), "&&" | "||") => {
            let is_and = op == "&&";
            let mut output = Vec::new();
            // true-first ordering matches existing conditional path ordering.
            for left_taken in [true, false] {
                let evaluate_right = left_taken == is_and;
                if !evaluate_right && taken != left_taken {
                    continue;
                }
                for left_path in branch_condition(left, path.clone(), left_taken)? {
                    if evaluate_right {
                        output.extend(branch_condition(right, left_path, taken)?);
                    } else {
                        output.push(left_path);
                    }
                    if output.len() > MAX_PATHS {
                        return Err("short-circuit path limit exceeded".into());
                    }
                }
            }
            Ok(output)
        }
        _ => {
            let mut path = path;
            let condition = path.evaluate(condition)?;
            path.push(SourceOperation::Guard { condition, taken })?;
            Ok(vec![path])
        }
    }
}

fn execute(statement: &Stmt, paths: Vec<Path>) -> Result<Vec<Path>, String> {
    if let Stmt::Block(statements) = statement {
        let declared: Vec<_> = statements
            .iter()
            .filter_map(|statement| {
                if let Stmt::Declare(id, _, _) = statement {
                    Some(id.clone())
                } else {
                    None
                }
            })
            .collect();
        let mut paths = paths;
        for index in 0..statements.len() {
            // CG09: a forward `goto` resumes at its label here; one that
            // would skip a declaration into its own scope is refused.
            control::enter_statement(statements, index, &mut paths)?;
            paths = execute(&statements[index], paths)?;
        }
        for path in &mut paths {
            // Remove block storage even on an early return. The final Return
            // operation is emitted after these lexical lifetime exits.
            for id in declared.iter().rev() {
                if let Some((local, _, _)) = path.locals.remove(id) {
                    path.push(SourceOperation::LocalEnd { local })?;
                }
            }
        }
        return Ok(paths);
    }
    let mut output = Vec::new();
    for mut path in paths {
        if path.returned || path.terminated || path.jump.is_some() {
            output.push(path);
            continue;
        }
        match statement {
            Stmt::ScanLoop(initial, id, ty, header, matches, early_return) => {
                let mut initialized = execute(initial, vec![path])?;
                let mut path = initialized.pop().ok_or("missing scan initializer path")?;
                check_source_rule(
                    "source_scan_initial",
                    &[
                        ("remaining_paths", initialized.len() as i128),
                        ("index_escaped", i128::from(path.addressed.contains(id))),
                    ],
                )?;
                let (local, _, initial) = path
                    .locals
                    .get(id)
                    .ok_or("scan local outside scope")?
                    .clone();
                let initial = initial.ok_or("unknown scan initial value")?;
                check_source_rule(
                    "source_type_match",
                    &[("type_matches", i128::from(initial.c_type == *ty))],
                )?;
                let index = value(ty.clone(), SourceExpression::LoopIndex { local });
                let mut iteration = path.clone();
                iteration.locals.get_mut(id).unwrap().2 = Some(index.clone());
                fn region(path: &mut Path, expr: &Expr) -> Result<SourceRegion, String> {
                    let start = path.operations.len();
                    let result = path.evaluate(expr)?;
                    Ok(SourceRegion {
                        operations: path.operations[start..].to_vec(),
                        result,
                    })
                }
                let header = region(&mut iteration, header)?;
                let matches = region(&mut iteration, matches)?;
                let early_return = region(&mut iteration, early_return)?;
                let increment = value(
                    ty.clone(),
                    SourceExpression::Binary {
                        operator: "+".into(),
                        left: Box::new(index),
                        right: Box::new(value(
                            ty.clone(),
                            SourceExpression::Integer { value: "1".into() },
                        )),
                    },
                );
                // The finite path evaluator does not infer the exit index.
                // Its iteration symbols and induction value cannot escape.
                path.locals.get_mut(id).unwrap().2 = None;
                path.push(SourceOperation::ScanLoop {
                    local,
                    initial,
                    header: Box::new(header),
                    matches: Box::new(matches),
                    early_return: Box::new(early_return),
                    increment,
                })?;
                output.push(path);
                if output.len() > MAX_PATHS {
                    return Err("callback path limit exceeded".into());
                }
                continue;
            }
            Stmt::Declare(id, ty, initial) => {
                let index = path.next_local;
                path.next_local += 1;
                let result = initial
                    .as_ref()
                    .map(|expr| path.evaluate(expr))
                    .transpose()?;
                path.locals
                    .insert(id.clone(), (index, ty.clone(), result.clone()));
                if ty.ends_with("[2]") || ty == "struct timeval" {
                    path.push(SourceOperation::LocalStart {
                        local: index,
                        c_type: ty.clone(),
                    })?;
                }
                if let Some(value) = result {
                    path.push(SourceOperation::LocalWrite {
                        local: index,
                        value,
                    })?;
                }
            }
            Stmt::Assign(place, expr) => {
                // A direct field base read from an unescaped pointer local
                // cannot be changed by the RHS call. Other addresses retain
                // the conservative call/read ordering gate.
                let stable_base = matches!(place, Expr::Field(_, base, _) if matches!(base.as_ref(), Expr::Read(_, local) if matches!(local.as_ref(), Expr::Local(id, _) if !path.addressed.contains(id))));
                if !stable_base {
                    reject_unsequenced(&[place, expr])?;
                }
                let result = path.evaluate(expr)?;
                path.write(place, result)?;
            }
            Stmt::OrAssign(place, expr) => {
                reject_unsequenced(&[place, expr])?;
                // Include the old lvalue read in ordering checks: an RHS call
                // could change an address-taken local before it is read.
                if effects(expr).1 {
                    return Err("compound assignment with an opaque call".into());
                }
                let right = path.evaluate(expr)?;
                path.update(place, "|", right)?;
            }
            Stmt::Compound(place, operator, place_type, compute_lhs, compute_result, expr) => {
                // As `|=`: the left operand is read and written, so an RHS
                // call or an effectful address has no invented order.
                reject_unsequenced(&[place, expr])?;
                if effects(expr).1 {
                    return Err("compound assignment with an opaque call".into());
                }
                if effects(place).1 {
                    return Err("compound assignment with an effectful address".into());
                }
                let right = path.evaluate(expr)?;
                path.compound(
                    place,
                    operator,
                    [place_type, compute_lhs, compute_result],
                    right,
                )?;
            }
            Stmt::Increment(place, operator, ty) => {
                if effects(place).1 {
                    return Err("increment with an effectful address".into());
                }
                path.update(
                    place,
                    operator,
                    value(ty.clone(), SourceExpression::Integer { value: "1".into() }),
                )?;
            }
            Stmt::Call(expr) => {
                path.evaluate(expr)?;
            }
            Stmt::NoReturnCall(Expr::Call(ty, function, signature, arguments)) => {
                path.call(ty, function, signature, arguments)?;
                path.terminated = true;
            }
            Stmt::NoReturnCall(_) => unreachable!(),
            Stmt::DoOnce(body, condition) => {
                let mut once = execute(body, vec![path])?;
                for path in &mut once {
                    // CG09: `break` leaves before the condition, `continue`
                    // evaluates it; a `goto` passes through.
                    if control::do_once_reaches_condition(path) {
                        let condition = path.evaluate(condition)?;
                        path.push(SourceOperation::Guard {
                            condition,
                            taken: false,
                        })?;
                    }
                }
                output.extend(once);
                if output.len() > MAX_PATHS {
                    return Err("callback path limit exceeded".into());
                }
                continue;
            }
            Stmt::If(condition, yes, no) => {
                let yes_paths = branch_condition(condition, path.clone(), true)?;
                let no_paths = branch_condition(condition, path, false)?;
                output.extend(execute(yes, yes_paths)?);
                output.extend(if let Some(no) = no {
                    execute(no, no_paths)?
                } else {
                    no_paths
                });
                if output.len() > MAX_PATHS {
                    return Err("callback path limit exceeded".into());
                }
                continue;
            }
            Stmt::Return(result) => {
                path.return_value = result
                    .as_ref()
                    .map(|value| path.evaluate(value))
                    .transpose()?;
                path.returned = true;
            }
            Stmt::Switch(controlling, items) => {
                output.extend(control::switch(path, controlling, items)?);
                if output.len() > MAX_PATHS {
                    return Err("callback path limit exceeded".into());
                }
                continue;
            }
            Stmt::Label(_, inner) => {
                output.extend(execute(inner, vec![path])?);
                if output.len() > MAX_PATHS {
                    return Err("callback path limit exceeded".into());
                }
                continue;
            }
            Stmt::Jump(jump) => path.jump = Some(jump.clone()),
            Stmt::Block(_) => unreachable!(),
        }
        output.push(path);
        if output.len() > MAX_PATHS {
            return Err("callback path limit exceeded".into());
        }
    }
    Ok(output)
}

fn derive(
    ast: &Value,
    function: &str,
    revision: &str,
    fields: &BTreeMap<(String, String), String>,
    constants: &BTreeMap<String, SourceValue>,
    arrays: &BTreeSet<String>,
    (profile, admission, control): (&SourceImportProfile<'_>, &NodeAdmission, &ControlAdmission),
) -> Result<SourceSummary, String> {
    let expected_signature = profile
        .signature_constraints
        .iter()
        .find(|(name, _, _)| *name == function);
    let signature = c_type(ast)?;
    let return_type = signature
        .split('(')
        .next()
        .ok_or("missing return type")?
        .trim();
    if ast["variadic"] == true
        || expected_signature.is_some_and(|(_, expected, _)| return_type != *expected)
    {
        return Err("function signature is outside the selected Unix profile".into());
    }
    if !matches!(
        return_type,
        "void"
            | "void *"
            | "int"
            | "char *"
            | "unsigned int"
            | "unsigned long"
            | "unsigned long long"
    ) {
        return Err("wrapper return type outside bounded profile".into());
    }
    let Some((body, parameters)) = children(ast).split_last() else {
        return Err("missing function body".into());
    };
    if kind(body) != "CompoundStmt"
        || expected_signature.is_some_and(|(_, _, types)| parameters.len() != types.len())
    {
        return Err("unmodeled function parameters/attributes".into());
    }
    let mut importer = Importer {
        admission,
        control,
        labels: BTreeSet::new(),
        fields,
        constants,
        allowed_calls: expected_signature.map(|_| profile.restricted_calls),
        sizeof_types: profile.sizeof_types,
        arrays,
        locals: BTreeSet::new(),
        nodes: 0,
    };
    let mut path = Path {
        next_local: parameters.len(),
        ..Path::default()
    };
    for (index, parameter) in parameters.iter().enumerate() {
        if kind(parameter) != "ParmVarDecl" || !children(parameter).is_empty() {
            return Err("unmodeled callback parameter".into());
        }
        let id = parameter["id"]
            .as_str()
            .ok_or("missing parameter ID")?
            .to_owned();
        let ty = c_type(parameter)?;
        if expected_signature.map_or_else(
            || !(ty.ends_with('*') || matches!(ty.as_str(), "int" | "unsigned int" | "long")),
            |(_, _, types)| ty != types[index],
        ) {
            return Err("callback parameter type outside selected Unix profile".into());
        }
        importer.locals.insert(id.clone());
        path.locals.insert(
            id,
            (
                index,
                ty.clone(),
                Some(value(ty, SourceExpression::Parameter { index })),
            ),
        );
    }
    // Parse all statements before executing any path; unreachable unsupported
    // operations still make the entire callback Unknown.
    let statement = importer.statement(body, 0)?;
    let paths = execute(&statement, vec![path])?
        .into_iter()
        .map(|mut path| {
            // CG09: a jump no statement took (a `goto` into a scope it does
            // not enclose) is refused, never dropped.
            if let Some(jump) = &path.jump {
                return Err(control::untaken(jump));
            }
            if path.operations.iter().any(|operation| matches!(operation, SourceOperation::ScanLoop { early_return, .. } if early_return.result.c_type != return_type)) {
                return Err("scan early return type differs from function signature".into());
            }
            if path.terminated {
                path.push(SourceOperation::Terminate)?;
            } else if return_type == "void" {
                if path.return_value.is_some() {
                    return Err("value returned from void function".into());
                }
                path.push(SourceOperation::Return)?;
            } else {
                let value = path
                    .return_value
                    .take()
                    .ok_or("missing nonvoid return on a source path")?;
                check_source_rule("source_type_match", &[("type_matches", i128::from(value.c_type == return_type))])?;
                path.push(SourceOperation::ReturnValue { value })?;
            }
            let path = SourcePath {
                operations: path.operations,
            };
            check_path_symbols(&path)?;
            control.check_local_lifetimes(&path)?;
            Ok(path)
        })
        .collect::<Result<_, String>>()?;
    Ok(SourceSummary {
        function: function.into(),
        source_revision: revision.into(),
        signature,
        paths,
        profile_assumptions: BTreeMap::new(),
    })
}

#[cfg(test)]
#[path = "../tests/unit/short_circuit_source.rs"]
mod short_circuit_tests;

#[cfg(test)]
#[path = "../tests/unit/source_symbols.rs"]
mod symbol_tests;

#[cfg(test)]
#[path = "../tests/unit/source_operators.rs"]
mod operators_tests;

#[cfg(test)]
#[path = "../tests/unit/source_control.rs"]
mod control_tests;
