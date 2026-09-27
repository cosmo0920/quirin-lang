//! Clang-typed, statement-position access paths. This reader is deliberately
//! narrower than C: an absent AST fact, ambiguous statement or unlisted node
//! refuses. The old shape readers remain alongside it.

use std::cell::OnceCell;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

use serde_json::Value;

use crate::indirect_chain_access::AccessChain;
use crate::legacy_c::{C11TranslationUnit, CCompilerDialect, LegacyCertificateRequest, tokenize};
use crate::typed_c_ast::{children, kind, translation_unit_ast};
use crate::typed_field_access::TypedScalarPlan;

pub(crate) const PARITY_PATH: &str = "QCORE_TYPED_PATH_PARITY";

/// Card **W47-ACCESS-PATH-POSITIONS**, step B: the owner-aware enum arm.
/// With it, a value whose type is `enum TAG` for a declared `EnumDecl`, or a
/// typedef whose own declaration is directly an `EnumType`, is a scalar.
/// Every other typedef spelling keeps the positive-builtin refusal.
pub(crate) const ENUM_ARM: bool = true;
static PARITY_SINK: Mutex<Option<ParitySink>> = Mutex::new(None);

struct ParitySink {
    rows: BufWriter<File>,
    contexts: BufWriter<File>,
    seen_frontends: BTreeSet<String>,
}

pub(crate) fn parity_enabled() -> bool {
    std::env::var_os(PARITY_PATH).is_some()
}

/// Optional side-by-side record. Verdict artifacts remain the arbiter; this
/// stream identifies every statement where the old and Clang readers differ.
pub(crate) fn record_parity(
    function: &str,
    line: usize,
    texts: &[&str],
    old_route: &str,
    old: Option<(&TypedScalarPlan, &[AccessChain])>,
    new: Option<&StatementPlan>,
) {
    if !parity_enabled() {
        return;
    }
    let disagreement = match (old, new) {
        (Some((old_scalar, old_chains)), Some(new)) => {
            old_scalar != &new.scalar || old_chains != new.chains || new.borrowed_destination
        }
        (None, None) => false,
        _ => true,
    };
    write_parity(serde_json::json!({
        "function": function,
        "line": line,
        "texts": texts,
        "old_route": old_route,
        "old_admitted": old.is_some(),
        "new_admitted": new.is_some(),
        "old_scalar": old.map(|(plan, _)| format!("{plan:?}")),
        "new_scalar": new.map(|plan| format!("{:?}", plan.scalar)),
        "old_chains": old.map(|(_, chains)| format!("{chains:?}")),
        "new_chains": new.map(|plan| format!("{:?}", plan.chains)),
        "new_borrowed_destination": new.is_some_and(|plan| plan.borrowed_destination),
        "disagreement": disagreement,
    }));
}

/// Older routes with different plan vocabularies stay authoritative. Record
/// their complete instruction beside the typed candidate; parity remains
/// unproved even if both admit, so every such row is a review item.
pub(crate) fn record_legacy_route(
    function: &str,
    line: usize,
    texts: &[&str],
    old_route: &str,
    old_instruction: &str,
    new: Option<&StatementPlan>,
) {
    if !parity_enabled() {
        return;
    }
    write_parity(serde_json::json!({
        "function": function,
        "line": line,
        "texts": texts,
        "old_route": old_route,
        "old_admitted": true,
        "new_admitted": new.is_some(),
        "old_instruction": old_instruction,
        "new_scalar": new.map(|plan| format!("{:?}", plan.scalar)),
        "new_chains": new.map(|plan| format!("{:?}", plan.chains)),
        "new_borrowed_destination": new.is_some_and(|plan| plan.borrowed_destination),
        "disagreement": true,
        "comparison": "distinct plan vocabularies; no parity claim",
    }));
}

/// Card **W47-ACCESS-PATH-POSITIONS**: a condition operand, return or call
/// actual every existing route refused, with the typed answer (admitted or
/// not). Rows without a `position` key are P2's statement rows.
pub(crate) fn record_position(
    function: &str,
    line: usize,
    texts: &[&str],
    position: &str,
    new: Option<&StatementPlan>,
) {
    if !parity_enabled() {
        return;
    }
    write_parity(serde_json::json!({
        "function": function,
        "line": line,
        "texts": texts,
        "position": position,
        "old_route": "old_refusal",
        "old_admitted": false,
        "new_admitted": new.is_some(),
        "new_scalar": new.map(|plan| format!("{:?}", plan.scalar)),
        "new_chains": new.map(|plan| format!("{:?}", plan.chains)),
        "new_borrowed_destination": new.is_some_and(|plan| plan.borrowed_destination),
        "disagreement": new.is_some(),
    }));
}

/// Card **W47-ACCESS-PATH-POSITIONS**: an existing route admitted a
/// condition operand, return or call actual; its instruction beside the typed
/// candidate. Distinct vocabularies: no parity claim.
pub(crate) fn record_position_legacy(
    function: &str,
    line: usize,
    texts: &[&str],
    position: &str,
    old_route: &str,
    old_instruction: &str,
    new: Option<&StatementPlan>,
) {
    if !parity_enabled() {
        return;
    }
    write_parity(serde_json::json!({
        "function": function,
        "line": line,
        "texts": texts,
        "position": position,
        "old_route": old_route,
        "old_admitted": true,
        "new_admitted": new.is_some(),
        "old_instruction": old_instruction,
        "new_scalar": new.map(|plan| format!("{:?}", plan.scalar)),
        "new_chains": new.map(|plan| format!("{:?}", plan.chains)),
        "new_borrowed_destination": new.is_some_and(|plan| plan.borrowed_destination),
        "disagreement": true,
        "comparison": "distinct plan vocabularies; no parity claim",
    }));
}

fn write_parity(mut value: Value) {
    let Some((source_sha256, dialect, frontend_sha256, frontend_debug)) =
        ANALYSED_SOURCE.with(|cell| {
            let context = cell.borrow();
            let context = context.as_ref()?;
            Some((
                context.source_sha256.clone(),
                format!("{:?}", context.dialect),
                context.frontend_sha256.clone(),
                Arc::clone(&context.frontend_debug),
            ))
        })
    else {
        return;
    };
    let object = value.as_object_mut().expect("typed parity object");
    object.insert("source_sha256".into(), source_sha256.into());
    object.insert("dialect".into(), dialect.into());
    object.insert("frontend_sha256".into(), frontend_sha256.into());
    let mut guard = PARITY_SINK.lock().expect("typed access parity sink lock");
    let sink = guard.get_or_insert_with(|| {
        let path = PathBuf::from(std::env::var_os(PARITY_PATH).expect("typed access parity path"));
        let mut context_path = path.clone().into_os_string();
        context_path.push(".contexts.jsonl");
        ParitySink {
            rows: BufWriter::new(File::create(path).expect("typed access parity sink")),
            contexts: BufWriter::new(
                File::create(PathBuf::from(context_path)).expect("typed access context sink"),
            ),
            seen_frontends: BTreeSet::new(),
        }
    });
    let frontend_sha256 = value["frontend_sha256"].as_str().expect("frontend digest");
    if sink.seen_frontends.insert(frontend_sha256.to_owned()) {
        let context = serde_json::json!({
            "frontend_sha256": frontend_sha256,
            "frontend_debug": frontend_debug.as_str(),
        });
        writeln!(sink.contexts, "{context}").expect("typed access context write");
        sink.contexts.flush().expect("typed access context flush");
    }
    writeln!(sink.rows, "{value}").expect("typed access parity write");
    sink.rows.flush().expect("typed access parity flush");
}

#[derive(Clone, Debug)]
pub(crate) struct StatementPlan {
    pub(crate) scalar: TypedScalarPlan,
    pub(crate) chains: Vec<AccessChain>,
    pub(crate) accesses: usize,
    /// A whole pointer-valued path read becomes a target-free borrowed alias.
    pub(crate) borrowed_destination: bool,
}

/// An AST and the exact token rendering handed to Clang. This is created
/// lazily, only after a statement has reached an access-related refusal.
pub(crate) struct StatementIndex {
    pub(crate) source: Arc<String>,
    /// Byte offsets of every `\n` of `source`, ascending: a node's line is
    /// found by binary search instead of rescanning the source per node.
    newlines: Arc<Vec<usize>>,
    pub(crate) function: Value,
    pub(crate) fields: Arc<BTreeMap<String, Option<FieldFacts>>>,
    pub(crate) enums: Arc<EnumFacts>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FieldFacts {
    union_owner: bool,
    bitfield: bool,
}

/// Card **W47-ACCESS-PATH-POSITIONS**, step B: positive enum evidence read
/// from the unit's own declarations. `typedefs` maps a typedef name to
/// whether its declaration is directly an `EnumType` (a name declared twice
/// with different answers maps to `false`); `tags` holds every named
/// `EnumDecl`.
#[derive(Clone, Debug, Default)]
pub(crate) struct EnumFacts {
    typedefs: BTreeMap<String, bool>,
    tags: BTreeSet<String>,
}

impl EnumFacts {
    fn collect(node: &Value, facts: &mut Self) {
        match kind(node) {
            "EnumDecl" => {
                if let Some(name) = node["name"].as_str() {
                    facts.tags.insert(name.to_owned());
                }
            }
            "TypedefDecl" => {
                if let Some(name) = node["name"].as_str() {
                    let own = node["type"]["qualType"].as_str().unwrap_or("*");
                    let direct = children(node).first().is_some_and(|inner| {
                        let inner = if kind(inner) == "ElaboratedType" {
                            children(inner).first().unwrap_or(inner)
                        } else {
                            inner
                        };
                        kind(inner) == "EnumType"
                            && inner["decl"]["kind"].as_str() == Some("EnumDecl")
                    });
                    let enum_typedef = direct
                        && children(node).len() == 1
                        && !own.contains('*')
                        && !own.contains('[')
                        && !own.contains('(');
                    facts
                        .typedefs
                        .entry(name.to_owned())
                        .and_modify(|old| *old = *old && enum_typedef)
                        .or_insert(enum_typedef);
                }
            }
            _ => {}
        }
        for child in children(node) {
            Self::collect(child, facts);
        }
    }

    /// Whether `node`'s type is positively an enumeration: `enum TAG` for a
    /// declared tag, or exactly one typedef name whose declaration is an
    /// `EnumType`. `const` is the only qualifier admitted.
    pub(crate) fn is_enum(&self, node: &Value) -> bool {
        let Some(text) = qual(node) else { return false };
        let words = text
            .split_whitespace()
            .filter(|word| *word != "const")
            .collect::<Vec<_>>();
        match words.as_slice() {
            ["enum", tag] => self.tags.contains(*tag),
            [name] => self.typedefs.get(*name) == Some(&true),
            _ => false,
        }
    }
}

thread_local! {
    static ANALYSED_SOURCE: RefCell<Option<SourceContext>> = const { RefCell::new(None) };
}

struct SourceContext {
    source: Arc<String>,
    source_sha256: String,
    frontend_sha256: String,
    frontend_debug: Arc<String>,
    dialect: CCompilerDialect,
    request: LegacyCertificateRequest,
    ast: OnceCell<Option<Value>>,
    fields: OnceCell<Arc<BTreeMap<String, Option<FieldFacts>>>>,
    enums: OnceCell<Arc<EnumFacts>>,
    newlines: OnceCell<Arc<Vec<usize>>>,
}

/// Keep the exact analysed source in the worker that builds this unit's CFGs.
/// The guard restores an outer source even on an early error return.
pub(crate) struct SourceScope(Option<SourceContext>);

pub(crate) fn enter_source(
    source: &str,
    dialect: CCompilerDialect,
    request: Option<&LegacyCertificateRequest>,
) -> SourceScope {
    let request = request
        .cloned()
        .unwrap_or_else(|| LegacyCertificateRequest {
            include_paths: Vec::new(),
            definitions: Vec::new(),
            preprocessor_options: Vec::new(),
            input: Default::default(),
        });
    let frontend_debug = Arc::new(format!("{request:?}"));
    SourceScope(ANALYSED_SOURCE.with(|cell| {
        cell.replace(Some(SourceContext {
            source: Arc::new(source.to_owned()),
            source_sha256: qcore_schemas::sha256_hex(source.as_bytes()),
            frontend_sha256: qcore_schemas::sha256_hex(frontend_debug.as_bytes()),
            frontend_debug,
            dialect,
            request,
            ast: OnceCell::new(),
            fields: OnceCell::new(),
            enums: OnceCell::new(),
            newlines: OnceCell::new(),
        }))
    }))
}

impl Drop for SourceScope {
    fn drop(&mut self) {
        ANALYSED_SOURCE.with(|cell| {
            cell.replace(self.0.take());
        });
    }
}

impl StatementIndex {
    pub(crate) fn read(function: &str) -> Option<Self> {
        ANALYSED_SOURCE.with(|cell| {
            let borrow = cell.borrow();
            let context = borrow.as_ref()?;
            let ast = context
                .ast
                .get_or_init(|| {
                    let unit = C11TranslationUnit {
                        source: &context.source,
                        include_paths: context.request.include_paths.clone(),
                        definitions: context.request.definitions.clone(),
                        preprocessor_options: context.request.preprocessor_options.clone(),
                        dialect: context.dialect,
                        ..C11TranslationUnit::default()
                    };
                    translation_unit_ast(&context.source, &unit).ok()
                })
                .as_ref()?;
            let mut functions = children(ast)
                .iter()
                .filter(|node| {
                    kind(node) == "FunctionDecl"
                        && node["name"].as_str() == Some(function)
                        && children(node)
                            .iter()
                            .any(|child| kind(child) == "CompoundStmt")
                })
                .collect::<Vec<_>>();
            if functions.len() != 1 {
                return None;
            }
            let fields = context.fields.get_or_init(|| {
                let mut fields = BTreeMap::new();
                collect_field_facts(ast, &mut fields);
                Arc::new(fields)
            });
            let enums = context.enums.get_or_init(|| {
                let mut enums = EnumFacts::default();
                EnumFacts::collect(ast, &mut enums);
                Arc::new(enums)
            });
            let newlines = context.newlines.get_or_init(|| {
                Arc::new(
                    context
                        .source
                        .bytes()
                        .enumerate()
                        .filter_map(|(at, byte)| (byte == b'\n').then_some(at))
                        .collect(),
                )
            });
            Some(Self {
                source: Arc::clone(&context.source),
                newlines: Arc::clone(newlines),
                function: functions.remove(0).clone(),
                fields: Arc::clone(fields),
                enums: Arc::clone(enums),
            })
        })
    }

    /// Require a unique complete Clang statement whose tokens and internal
    /// line are exactly the gateway statement's tokens and line.
    pub(crate) fn statement(
        &self,
        texts: &[&str],
        line: usize,
        never_bound: impl Fn(&str) -> bool,
    ) -> Option<StatementPlan> {
        let mut candidates = Vec::new();
        let body = children(&self.function)
            .iter()
            .find(|node| kind(node) == "CompoundStmt")?;
        statements(body, &mut candidates);
        let mut matched = candidates.into_iter().filter(|node| {
            self.node_line(node) == Some(line)
                && self.node_tokens(node).is_some_and(|found| found == texts)
        });
        let node = matched.next()?;
        if matched.next().is_some() {
            return None;
        }
        let mut reader = self.reader(&never_bound);
        reader.statement(node)?;
        Some(reader.plan)
    }

    /// A fresh reader over this function's field and enum facts.
    pub(crate) fn reader<'a>(&'a self, never_bound: &'a dyn Fn(&str) -> bool) -> Reader<'a> {
        Reader {
            plan: StatementPlan {
                scalar: TypedScalarPlan::default(),
                chains: Vec::new(),
                accesses: 0,
                borrowed_destination: false,
            },
            fields: &self.fields,
            enums: &self.enums,
            never_bound,
        }
    }

    pub(crate) fn node_line(&self, node: &Value) -> Option<usize> {
        let offset = offset(&node["range"]["begin"])?;
        if offset > self.source.len() {
            return None;
        }
        // The number of newlines strictly before `offset`, plus one.
        Some(self.newlines.partition_point(|&at| at < offset) + 1)
    }

    pub(crate) fn node_tokens(&self, node: &Value) -> Option<Vec<String>> {
        let start = offset(&node["range"]["begin"])?;
        let end = offset(&node["range"]["end"])?
            .checked_add(node["range"]["end"]["tokLen"].as_u64()? as usize)?;
        let source = self.source.get(start..end)?;
        let mut found = tokenize(source)
            .into_iter()
            .map(|token| token.text)
            .collect::<Vec<_>>();
        if found.last().is_some_and(|text| text == ";") {
            found.pop();
        }
        Some(found)
    }
}

pub(crate) fn offset(location: &Value) -> Option<usize> {
    location["offset"]
        .as_u64()
        .or_else(|| location["expansionLoc"]["offset"].as_u64())
        .map(|offset| offset as usize)
}

fn collect_field_facts(node: &Value, fields: &mut BTreeMap<String, Option<FieldFacts>>) {
    if kind(node) == "RecordDecl" {
        let owner = node["tagUsed"].as_str();
        let complete = node["completeDefinition"].as_bool() == Some(true);
        for child in children(node)
            .iter()
            .filter(|child| kind(child) == "FieldDecl")
        {
            let Some(id) = child["id"].as_str() else {
                continue;
            };
            let facts = match owner {
                Some("struct" | "union") if complete && qual(child).is_some() => Some(FieldFacts {
                    union_owner: owner == Some("union"),
                    bitfield: child["isBitfield"].as_bool() == Some(true),
                }),
                _ => None,
            };
            fields
                .entry(id.to_owned())
                .and_modify(|old| {
                    if *old != facts {
                        *old = None;
                    }
                })
                .or_insert(facts);
        }
    }
    for child in children(node) {
        collect_field_facts(child, fields);
    }
}

fn statements<'a>(node: &'a Value, out: &mut Vec<&'a Value>) {
    match kind(node) {
        "CompoundStmt" => {
            for child in children(node) {
                statement_body(child, out);
            }
        }
        "IfStmt" => {
            let inner = children(node);
            if node["hasElse"].as_bool() == Some(true) && inner.len() >= 3 {
                statement_body(&inner[inner.len() - 2], out);
            }
            if let Some(body) = inner.last() {
                statement_body(body, out);
            }
        }
        "WhileStmt" | "SwitchStmt" => {
            if let Some(body) = children(node).last() {
                statement_body(body, out);
            }
        }
        "DoStmt" => {
            if let Some(body) = children(node).first() {
                statement_body(body, out);
            }
        }
        "ForStmt" => {
            // Clang's five slots are init, condition-variable placeholder,
            // condition, increment, body. The condition is P3's, not P2's.
            if let Some(inner) = node["inner"].as_array()
                && inner.len() == 5
            {
                if !kind(&inner[0]).is_empty() {
                    out.push(&inner[0]);
                }
                if !kind(&inner[3]).is_empty() {
                    out.push(&inner[3]);
                }
                statement_body(&inner[4], out);
            }
        }
        "CaseStmt" | "DefaultStmt" | "LabelStmt" => {
            if let Some(body) = children(node).last() {
                statement_body(body, out);
            }
        }
        _ => {}
    }
}

fn statement_body<'a>(node: &'a Value, out: &mut Vec<&'a Value>) {
    if is_control(node) {
        statements(node, out);
    } else if kind(node) != "ReturnStmt" && !kind(node).is_empty() {
        out.push(node);
    }
}

fn is_control(node: &Value) -> bool {
    matches!(
        kind(node),
        "CompoundStmt"
            | "IfStmt"
            | "WhileStmt"
            | "DoStmt"
            | "SwitchStmt"
            | "ForStmt"
            | "CaseStmt"
            | "DefaultStmt"
            | "LabelStmt"
    )
}

pub(crate) struct Reader<'a> {
    pub(crate) plan: StatementPlan,
    fields: &'a BTreeMap<String, Option<FieldFacts>>,
    enums: &'a EnumFacts,
    pub(crate) never_bound: &'a dyn Fn(&str) -> bool,
}

impl Reader<'_> {
    /// A proven builtin scalar, or (step B) a positively proven enumeration.
    pub(crate) fn scalar(&self, node: &Value) -> bool {
        is_scalar(node) || (ENUM_ARM && self.enums.is_enum(node))
    }

    pub(crate) fn statement(&mut self, node: &Value) -> Option<()> {
        match kind(node) {
            "BinaryOperator" if node["opcode"].as_str() == Some("=") => {
                let pair = children(node);
                if pair.len() != 2 {
                    return None;
                }
                if let Some(destination) = pointer_name(&pair[0]) {
                    self.plan.scalar.destination = Some(destination.to_owned());
                    self.plan.borrowed_destination = true;
                    return self.path(strip(&pair[1])?, false, true);
                }
                self.store(&pair[0])?;
                self.expression(&pair[1])
            }
            "DeclStmt" => {
                let declarations = children(node);
                if declarations.len() > 1 {
                    // A multi-declarator pointer alias statement has no
                    // access only when every source and destination is
                    // statically outside tracked-root custody.
                    for decl in declarations {
                        if kind(decl) != "VarDecl" || !is_pointer(decl) {
                            return None;
                        }
                        let name = decl["name"].as_str()?;
                        let source = untracked_pointer_source(children(decl).first()?)?;
                        if children(decl).len() != 1
                            || !(self.never_bound)(name)
                            || !(self.never_bound)(source)
                        {
                            return None;
                        }
                    }
                    return Some(());
                }
                if declarations.len() != 1 || kind(&declarations[0]) != "VarDecl" {
                    return None;
                }
                let decl = &declarations[0];
                let pointer = is_pointer(decl);
                if !pointer && !self.scalar(decl) {
                    return None;
                }
                let name = decl["name"].as_str()?.to_owned();
                if children(decl).is_empty() {
                    // An uninitialized local introduces no pointer value;
                    // forget any same-named fact in the name-based model.
                    self.plan.scalar.destination = Some(name);
                    return Some(());
                }
                if children(decl).len() != 1 {
                    return None;
                }
                self.plan.scalar.destination = Some(name);
                if pointer {
                    self.plan.borrowed_destination = true;
                    self.path(strip(&children(decl)[0])?, false, true)
                } else {
                    self.expression(&children(decl)[0])
                }
            }
            _ => self.expression(node),
        }
    }

    fn store(&mut self, node: &Value) -> Option<()> {
        let node = strip(node)?;
        if kind(node) == "DeclRefExpr" && self.scalar(node) {
            self.plan.scalar.destination =
                Some(node["referencedDecl"]["name"].as_str()?.to_owned());
            return Some(());
        }
        self.path(node, true, false)
    }

    pub(crate) fn expression(&mut self, node: &Value) -> Option<()> {
        match kind(node) {
            "ParenExpr" | "ImplicitCastExpr" => {
                if kind(node) == "ImplicitCastExpr"
                    && !matches!(
                        node["castKind"].as_str()?,
                        "LValueToRValue" | "NoOp" | "IntegralCast"
                    )
                {
                    return None;
                }
                self.expression(children(node).first()?)
            }
            "CStyleCastExpr"
                if self.scalar(node)
                    && children(node)
                        .first()
                        .is_some_and(|inner| self.scalar(inner)) =>
            {
                self.expression(children(node).first()?)
            }
            "IntegerLiteral" | "FloatingLiteral" | "CharacterLiteral" | "GNUNullExpr" => Some(()),
            "DeclRefExpr" if self.scalar(node) => Some(()),
            "UnaryExprOrTypeTraitExpr"
                if node["name"].as_str() == Some("sizeof")
                    && children(node).is_empty()
                    && node["argType"]["qualType"]
                        .as_str()
                        .is_some_and(|ty| !ty.contains('[')) =>
            {
                Some(())
            }
            "UnaryOperator"
                if matches!(
                    node["opcode"].as_str(),
                    Some("+") | Some("-") | Some("!") | Some("~")
                ) =>
            {
                self.expression(children(node).first()?)
            }
            "BinaryOperator"
                if matches!(
                    node["opcode"].as_str(),
                    Some("+")
                        | Some("-")
                        | Some("*")
                        | Some("/")
                        | Some("%")
                        | Some("<<")
                        | Some(">>")
                        | Some("&")
                        | Some("|")
                        | Some("^")
                        | Some("==")
                        | Some("!=")
                        | Some("<")
                        | Some(">")
                        | Some("<=")
                        | Some(">=")
                ) =>
            {
                let pair = children(node);
                if pair.len() != 2 || !self.scalar(&pair[0]) || !self.scalar(&pair[1]) {
                    return None;
                }
                self.expression(&pair[0])?;
                self.expression(&pair[1])
            }
            "MemberExpr" | "ArraySubscriptExpr" => self.path(node, false, false),
            "UnaryOperator" if node["opcode"].as_str() == Some("*") => {
                self.path(node, false, false)
            }
            _ => None,
        }
    }

    pub(crate) fn path(&mut self, node: &Value, store: bool, pointer_read: bool) -> Option<()> {
        if !(self.scalar(node) || (pointer_read && !store && is_pointer(node))) {
            return None;
        }
        let mut steps = Vec::new();
        let mut cursor = node;
        loop {
            cursor = strip(cursor)?;
            match kind(cursor) {
                "MemberExpr" => {
                    let field = self
                        .fields
                        .get(cursor["referencedMemberDecl"].as_str()?)?
                        .as_ref()?;
                    if field.union_owner || field.bitfield {
                        return None;
                    }
                    let member = cursor["name"].as_str()?.to_owned();
                    let base = children(cursor).first()?;
                    let arrow = cursor["isArrow"].as_bool() == Some(true);
                    let base_type = qual(base)?;
                    if base_type.contains("union ")
                        || base_type.contains("volatile")
                        || base_type.contains("_Atomic")
                    {
                        return None;
                    }
                    if arrow && !base_type.trim_end().ends_with('*') {
                        return None;
                    }
                    if !arrow && base_type.trim_end().ends_with('*') {
                        return None;
                    }
                    steps.push((if arrow { "->" } else { "." }, member));
                    cursor = base;
                }
                "ArraySubscriptExpr" => {
                    let pair = children(cursor);
                    if pair.len() != 2 || !self.scalar(&pair[1]) {
                        return None;
                    }
                    self.expression(&pair[1])?;
                    steps.push(("[]", String::new()));
                    cursor = &pair[0];
                }
                "UnaryOperator" if cursor["opcode"].as_str() == Some("*") => {
                    steps.push(("*", String::new()));
                    cursor = children(cursor).first()?;
                }
                "DeclRefExpr" => break,
                _ => return None,
            }
        }
        let root = cursor["referencedDecl"]["name"].as_str()?;
        if !matches!(
            cursor["referencedDecl"]["kind"].as_str(),
            Some("VarDecl") | Some("ParmVarDecl")
        ) {
            return None;
        }
        if steps.is_empty() {
            return None;
        }
        steps.reverse();
        let first = steps.first()?;
        if matches!(first.0, "->" | "*" | "[]") {
            if !self.plan.scalar.bases.contains(&root.to_owned()) {
                self.plan.scalar.bases.push(root.to_owned());
            }
            if first.0 == "->" && !self.plan.scalar.record_bases.contains(&root.to_owned()) {
                self.plan.scalar.record_bases.push(root.to_owned());
            }
        } else if !(self.never_bound)(root) {
            return None;
        }
        let mixed_path = steps.iter().any(|step| step.0 == "[]" && steps.len() != 1)
            || steps.iter().any(|step| step.0 == "*" && steps.len() != 1)
            || steps.iter().skip(1).any(|step| step.0 != "->");
        // The existing tracked field-link model names only direct `->`
        // places. A path through embedded records or indexed objects has
        // no tracked place representation, so it is admitted only when the
        // custody closure proves the root can never bind a resource. Then
        // every loaded pointer is outside this projection's tracked graph.
        if mixed_path && !(self.never_bound)(root) {
            return None;
        }
        if steps.len() > 1 || first.0 != "." {
            let members = steps.iter().map(|step| step.1.clone()).collect::<Vec<_>>();
            let links = if mixed_path {
                Vec::new()
            } else if members.len() > 1 {
                members[..members.len() - 1].to_vec()
            } else {
                Vec::new()
            };
            self.plan.chains.push(AccessChain {
                root: root.to_owned(),
                dereferenced_root: first.0 != ".",
                root_never_bound: (self.never_bound)(root),
                links,
                member: members.last()?.clone(),
            });
        }
        if store && matches!(first.0, "*" | "[]") {
            self.plan.scalar.writes_through_pointer = true;
        }
        self.plan.accesses += 1;
        Some(())
    }
}

pub(crate) fn strip(mut node: &Value) -> Option<&Value> {
    while matches!(kind(node), "ParenExpr" | "ImplicitCastExpr") {
        if kind(node) == "ImplicitCastExpr"
            && !matches!(
                node["castKind"].as_str()?,
                "LValueToRValue" | "NoOp" | "ArrayToPointerDecay"
            )
        {
            return None;
        }
        node = children(node).first()?;
    }
    Some(node)
}

pub(crate) fn qual(node: &Value) -> Option<&str> {
    node["type"]["desugaredQualType"]
        .as_str()
        .or_else(|| node["type"]["qualType"].as_str())
}

pub(crate) fn is_scalar(node: &Value) -> bool {
    // A missing/opaque desugared spelling is not evidence of a scalar. In
    // particular Clang may leave a typedef to an anonymous struct/union as
    // `a_lock_t`; the former negative filter admitted it as an integer.
    let Some(text) = qual(node) else { return false };
    let words = text.split_whitespace().collect::<Vec<_>>();
    let mut base = false;
    for word in words {
        match word {
            "const" | "restrict" => {}
            "signed" | "unsigned" | "short" | "long" => base = true,
            "char" | "int" | "float" | "double" | "_Bool" | "bool" | "__int128" => {
                base = true;
            }
            _ => return false,
        }
    }
    base
}

pub(crate) fn is_pointer(node: &Value) -> bool {
    qual(node).is_some_and(|text| {
        text.trim_end().ends_with('*')
            && !text.contains("union ")
            && !text.contains("volatile")
            && !text.contains("_Atomic")
            && !text.contains("(")
    })
}

fn pointer_name(node: &Value) -> Option<&str> {
    let bare = strip(node)?;
    (kind(bare) == "DeclRefExpr" && is_pointer(bare))
        .then(|| bare["referencedDecl"]["name"].as_str())
        .flatten()
}

fn untracked_pointer_source(mut node: &Value) -> Option<&str> {
    loop {
        if kind(node) != "ImplicitCastExpr" {
            break;
        }
        if !matches!(
            node["castKind"].as_str()?,
            "LValueToRValue" | "BitCast" | "NoOp" | "ArrayToPointerDecay"
        ) {
            return None;
        }
        node = children(node).first()?;
    }
    pointer_name(node)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan(source: &str, statement: &str) -> Option<StatementPlan> {
        let _scope = enter_source(source, CCompilerDialect::C11, None);
        let index = StatementIndex::read("f")?;
        let wanted = tokenize(statement);
        let texts = wanted
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        index.statement(&texts, 1, |_| true)
    }

    #[test]
    fn typed_scalar_chain_names_each_obligation() {
        let source = "struct S { struct S *next; int value; }; int f(struct S *p) { int x = p->next->value; return 0; }";
        let plan = plan(source, "int x = p->next->value").expect("typed chain");
        assert_eq!(plan.scalar.bases, ["p"]);
        assert_eq!(plan.chains.len(), 1);
        assert_eq!(plan.chains[0].links, ["next"]);
        assert_eq!(plan.scalar.destination.as_deref(), Some("x"));
    }

    #[test]
    fn unsupported_pointer_store_union_cast_and_call_refuse() {
        let cases = [
            (
                "struct S { int *next; }; int f(struct S *p, int *q) { p->next = q; return 0; }",
                "p->next = q",
            ),
            (
                "union U { int value; }; int f(union U *p) { int x = p->value; return 0; }",
                "int x = p->value",
            ),
            (
                "int f(int *p) { int x = *(int *)p; return 0; }",
                "int x = *(int *)p",
            ),
            (
                "int g(void); int f(void) { int x = g(); return 0; }",
                "int x = g()",
            ),
        ];
        for (source, statement) in cases {
            assert!(plan(source, statement).is_none(), "{statement}");
        }
    }

    #[test]
    fn for_init_is_a_statement_but_the_condition_is_not() {
        let source = "struct S { int value; }; int f(struct S *p) { for (int x = p->value; x < 1; x = x + 1) { } return 0; }";
        assert!(plan(source, "int x = p->value").is_some());
        assert!(plan(source, "x < 1").is_none());
    }

    #[test]
    fn untracked_multi_pointer_declaration_has_no_access() {
        let source = "struct S { int x; }; int f(void *a, void *b) { const struct S *x = a, *y = b; return 0; }";
        let _scope = enter_source(source, CCompilerDialect::C11, None);
        let index = StatementIndex::read("f").expect("typed function");
        let tokens = tokenize("const struct S *x = a, *y = b");
        let texts = tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        assert_eq!(index.statement(&texts, 1, |_| true).unwrap().accesses, 0);
        assert!(index.statement(&texts, 1, |name| name != "a").is_none());
    }

    #[test]
    fn sizeof_vla_bound_may_execute_and_refuses() {
        let source = "int close(int); int f(int n, int fd) { int x = sizeof(int[(close(fd), n)]); return x; }";
        assert!(plan(source, "int x = sizeof(int[(close(fd), n)])").is_none());
        let static_source = "int f(void) { int x = sizeof(int *); return x; }";
        assert!(plan(static_source, "int x = sizeof(int *)").is_some());
    }

    #[test]
    fn mixed_embedded_and_indexed_path_needs_untracked_root() {
        let source = "struct I { int value; }; struct S { struct I inner; struct I *items; }; int f(struct S *p, int i) { int x = p->inner.value + p->items[i].value; return 0; }";
        let _scope = enter_source(source, CCompilerDialect::C11, None);
        let index = StatementIndex::read("f").expect("typed function");
        let tokens = tokenize("int x = p->inner.value + p->items[i].value");
        let texts = tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        let plan = index
            .statement(&texts, 1, |_| true)
            .expect("untracked path");
        assert_eq!(plan.accesses, 2);
        assert!(plan.chains.iter().all(|chain| chain.links.is_empty()));
        assert!(index.statement(&texts, 1, |name| name != "p").is_none());
    }

    #[test]
    fn field_owner_refuses_typedef_union_but_accepts_typedef_struct() {
        let union = "typedef union U { int x; int y; } U; int f(U *u) { int x = u->x; return 0; }";
        assert!(plan(union, "int x = u->x").is_none());
        let nested = "typedef union U { int x; int y; } U; typedef struct S { U *u; } S; int f(S *s) { int x = s->u->x; return 0; }";
        assert!(plan(nested, "int x = s->u->x").is_none());
        let accepted = "typedef struct S { int x; } S; int f(S *s) { int x = s->x; return 0; }";
        assert!(plan(accepted, "int x = s->x").is_some());
    }

    /// Card W47-ACCESS-PATH-POSITIONS, step B: an enumeration is a scalar
    /// only on positive `EnumType` evidence, anonymous or tagged; an anonymous
    /// aggregate or union typedef keeps its refusal, and so does a typedef of
    /// a pointer to an enumeration.
    #[test]
    fn the_enum_arm_needs_positive_enum_evidence() {
        const { assert!(ENUM_ARM) };
        let anonymous = "typedef enum { A, B } mode_t2; struct S { mode_t2 m; }; int f(struct S *s) { s->m = B; return 0; }";
        assert!(plan(anonymous, "s->m = B").is_some());
        let tagged = "enum color { RED }; typedef enum color color_t; struct S { color_t c; enum color d; }; int f(struct S *s) { int x = s->c == RED && s->d == RED; return x; }";
        assert!(plan(tagged, "int x = s->c == RED && s->d == RED").is_none());
        let compare = "enum color { RED }; typedef enum color color_t; struct S { color_t c; enum color d; }; int f(struct S *s) { int x = s->c + s->d; return x; }";
        assert!(plan(compare, "int x = s->c + s->d").is_some());
        let aggregate = "typedef struct { int a; } agg_t; struct S { agg_t v; }; int f(struct S *s, struct S *t) { s->v = t->v; return 0; }";
        assert!(plan(aggregate, "s->v = t->v").is_none());
        let union = "typedef union { int a; float b; } un_t; struct S { un_t v; }; int f(struct S *s, struct S *t) { s->v = t->v; return 0; }";
        assert!(plan(union, "s->v = t->v").is_none());
        let pointer = "typedef enum { A } e_t; typedef e_t *ep_t; struct S { ep_t p; }; int f(struct S *s, ep_t q) { s->p = q; return 0; }";
        assert!(plan(pointer, "s->p = q").is_none());
    }
}
