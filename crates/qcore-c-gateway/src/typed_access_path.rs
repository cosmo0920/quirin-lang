//! Clang-typed, statement-position access paths. This reader is deliberately
//! narrower than C: an absent AST fact, ambiguous statement or unlisted node
//! refuses. The old shape readers remain alongside it.

use std::cell::OnceCell;
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::sync::Arc;
use std::sync::Mutex;

use serde_json::Value;

use crate::indirect_chain_access::AccessChain;
use crate::legacy_c::{C11TranslationUnit, CCompilerDialect, LegacyCertificateRequest, tokenize};
use crate::typed_c_ast::{children, kind, translation_unit_ast};
use crate::typed_field_access::TypedScalarPlan;

pub(crate) const PARITY_PATH: &str = "QCORE_TYPED_PATH_PARITY";
static PARITY_SINK: Mutex<Option<BufWriter<File>>> = Mutex::new(None);

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
    let Some((source_sha256, dialect, frontend)) = ANALYSED_SOURCE.with(|cell| {
        let context = cell.borrow();
        let context = context.as_ref()?;
        Some((
            context.source_sha256.clone(),
            format!("{:?}", context.dialect),
            format!("{:?}", context.request),
        ))
    }) else {
        return;
    };
    let disagreement = match (old, new) {
        (Some((old_scalar, old_chains)), Some(new)) => {
            old_scalar != &new.scalar || old_chains != new.chains || new.borrowed_destination
        }
        (None, None) => false,
        _ => true,
    };
    let value = serde_json::json!({
        "function": function,
        "source_sha256": source_sha256,
        "dialect": dialect,
        "frontend": frontend,
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
    });
    let mut guard = PARITY_SINK.lock().expect("typed access parity sink lock");
    let writer = guard.get_or_insert_with(|| {
        let path = std::env::var_os(PARITY_PATH).expect("typed access parity path");
        BufWriter::new(File::create(path).expect("typed access parity sink"))
    });
    writeln!(writer, "{value}").expect("typed access parity write");
    writer.flush().expect("typed access parity flush");
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
    source: Arc<String>,
    function: Value,
    bitfields: BTreeSet<String>,
}

thread_local! {
    static ANALYSED_SOURCE: RefCell<Option<SourceContext>> = const { RefCell::new(None) };
}

struct SourceContext {
    source: Arc<String>,
    source_sha256: String,
    dialect: CCompilerDialect,
    request: LegacyCertificateRequest,
    ast: OnceCell<Option<Value>>,
}

/// Keep the exact analysed source in the worker that builds this unit's CFGs.
/// The guard restores an outer source even on an early error return.
pub(crate) struct SourceScope(Option<SourceContext>);

pub(crate) fn enter_source(
    source: &str,
    dialect: CCompilerDialect,
    request: Option<&LegacyCertificateRequest>,
) -> SourceScope {
    SourceScope(ANALYSED_SOURCE.with(|cell| {
        cell.replace(Some(SourceContext {
            source: Arc::new(source.to_owned()),
            source_sha256: qcore_schemas::sha256_hex(source.as_bytes()),
            dialect,
            request: request
                .cloned()
                .unwrap_or_else(|| LegacyCertificateRequest {
                    include_paths: Vec::new(),
                    definitions: Vec::new(),
                    preprocessor_options: Vec::new(),
                    input: Default::default(),
                }),
            ast: OnceCell::new(),
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
            let mut bitfields = BTreeSet::new();
            collect_bitfields(ast, &mut bitfields);
            Some(Self {
                source: Arc::clone(&context.source),
                function: functions.remove(0).clone(),
                bitfields,
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
        let mut reader = Reader {
            plan: StatementPlan {
                scalar: TypedScalarPlan::default(),
                chains: Vec::new(),
                accesses: 0,
                borrowed_destination: false,
            },
            bitfields: &self.bitfields,
            never_bound: &never_bound,
        };
        reader.statement(node)?;
        Some(reader.plan)
    }

    fn node_line(&self, node: &Value) -> Option<usize> {
        let offset = offset(&node["range"]["begin"])?;
        Some(
            self.source
                .as_bytes()
                .get(..offset)?
                .iter()
                .filter(|&&b| b == b'\n')
                .count()
                + 1,
        )
    }

    fn node_tokens(&self, node: &Value) -> Option<Vec<String>> {
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

fn offset(location: &Value) -> Option<usize> {
    location["offset"]
        .as_u64()
        .or_else(|| location["expansionLoc"]["offset"].as_u64())
        .map(|offset| offset as usize)
}

fn collect_bitfields(node: &Value, fields: &mut BTreeSet<String>) {
    if kind(node) == "FieldDecl"
        && node["isBitfield"].as_bool() == Some(true)
        && let Some(id) = node["id"].as_str()
    {
        fields.insert(id.to_owned());
    }
    for child in children(node) {
        collect_bitfields(child, fields);
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

struct Reader<'a> {
    plan: StatementPlan,
    bitfields: &'a BTreeSet<String>,
    never_bound: &'a dyn Fn(&str) -> bool,
}

impl Reader<'_> {
    fn statement(&mut self, node: &Value) -> Option<()> {
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
                if !pointer && !is_scalar(decl) {
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
        if kind(node) == "DeclRefExpr" && is_scalar(node) {
            self.plan.scalar.destination =
                Some(node["referencedDecl"]["name"].as_str()?.to_owned());
            return Some(());
        }
        self.path(node, true, false)
    }

    fn expression(&mut self, node: &Value) -> Option<()> {
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
                if is_scalar(node) && children(node).first().is_some_and(is_scalar) =>
            {
                self.expression(children(node).first()?)
            }
            "IntegerLiteral" | "FloatingLiteral" | "CharacterLiteral" | "GNUNullExpr" => Some(()),
            "DeclRefExpr" if is_scalar(node) => Some(()),
            "UnaryExprOrTypeTraitExpr"
                if children(node).is_empty()
                    || (children(node).len() == 1 && is_scalar(&children(node)[0])) =>
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
                if pair.len() != 2 || !is_scalar(&pair[0]) || !is_scalar(&pair[1]) {
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

    fn path(&mut self, node: &Value, store: bool, pointer_read: bool) -> Option<()> {
        if !(is_scalar(node) || (pointer_read && !store && is_pointer(node))) {
            return None;
        }
        let mut steps = Vec::new();
        let mut cursor = node;
        loop {
            cursor = strip(cursor)?;
            match kind(cursor) {
                "MemberExpr" => {
                    if self
                        .bitfields
                        .contains(cursor["referencedMemberDecl"].as_str()?)
                    {
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
                    if pair.len() != 2 || !is_scalar(&pair[1]) {
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
        if steps.iter().any(|step| step.0 == "[]" && steps.len() != 1)
            || steps.iter().any(|step| step.0 == "*" && steps.len() != 1)
            || steps.iter().skip(1).any(|step| step.0 != "->")
        {
            return None;
        }
        if steps.len() > 1 || first.0 != "." {
            let members = steps.iter().map(|step| step.1.clone()).collect::<Vec<_>>();
            let links = if members.len() > 1 {
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

fn strip(mut node: &Value) -> Option<&Value> {
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

fn qual(node: &Value) -> Option<&str> {
    node["type"]["desugaredQualType"]
        .as_str()
        .or_else(|| node["type"]["qualType"].as_str())
}

fn is_scalar(node: &Value) -> bool {
    qual(node).is_some_and(|text| {
        !text.contains(['*', '[', ']', '(', ')'])
            && !text.contains("struct ")
            && !text.contains("union ")
            && !text.contains("volatile")
            && !text.contains("_Atomic")
    })
}

fn is_pointer(node: &Value) -> bool {
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
}
