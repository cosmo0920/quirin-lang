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

use serde::de::{MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::Value;

use crate::indirect_chain_access::AccessChain;
// Card W47-TYPED-CONST-POINTER-VALUE (rule 10m): the value check and the
// pointer-destination source.
use crate::legacy_c::{C11TranslationUnit, CCompilerDialect, LegacyCertificateRequest, tokenize};
use crate::typed_c_ast::{children, kind, translation_unit_ast_with};
use crate::typed_const_pointer_value::{pointer_source, pointer_value};
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

/// Card **W47-TYPED-ROOT-DECLARATION-READER**: one side-by-side row of
/// another reader's answers (rule 10l's `root-declarations`).
pub(crate) fn record_row(value: Value) {
    if parity_enabled() {
        write_parity(value);
    }
}

thread_local! {
    static PARITY_MUTED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Card **W47-SUMMARY-VALIDATION-PROTECTED-FORMALS**: run `f` without
/// writing parity rows. The W4 re-check rebuilds a body's CFG a second time;
/// its readers' rows would repeat the first build's under other premises.
/// Nothing but the stream is affected.
pub(crate) fn without_parity_rows<T>(f: impl FnOnce() -> T) -> T {
    struct Restore(bool);
    impl Drop for Restore {
        fn drop(&mut self) {
            PARITY_MUTED.with(|muted| muted.set(self.0));
        }
    }
    let _restore = Restore(PARITY_MUTED.with(|muted| muted.replace(true)));
    f()
}

fn write_parity(mut value: Value) {
    if PARITY_MUTED.with(std::cell::Cell::get) {
        return;
    }
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
    pub(crate) function: Arc<Value>,
    pub(crate) fields: Arc<BTreeMap<String, Option<FieldFacts>>>,
    pub(crate) enums: Arc<EnumFacts>,
    by_line: Arc<BTreeMap<usize, Vec<Vec<usize>>>>,
    /// The unit's line starts: a node's line is found by binary search.
    line_starts: Arc<Vec<usize>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FieldFacts {
    pub(crate) union_owner: bool,
    pub(crate) bitfield: bool,
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
    /// Card W47-ACCESS-PATH-POSITION-RESIDUALS (rule 10n): `FieldDecl` ids of
    /// members declared with an inline anonymous `enum { … }` type.
    pub(crate) anonymous_members: BTreeSet<String>,
}

impl EnumFacts {
    fn collect(node: &Value, facts: &mut Self) {
        match kind(node) {
            "RecordDecl" => crate::typed_position_shapes::collect_anonymous_enum_members(
                node,
                &mut facts.anonymous_members,
            ),
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
    ast: OnceCell<Option<AstIndex>>,
}

/// Keep only facts later typed-path lookups can use. Decode declarations one
/// at a time, retaining function bodies and collected field facts for the CFGs
/// built from this translation unit.
struct AstIndex {
    functions: BTreeMap<String, Vec<FunctionEntry>>,
    fields: Arc<BTreeMap<String, Option<FieldFacts>>>,
    enums: Arc<EnumFacts>,
    line_starts: Arc<Vec<usize>>,
}

struct FunctionEntry {
    ast: Arc<Value>,
    by_line: OnceCell<Arc<BTreeMap<usize, Vec<Vec<usize>>>>>,
}

#[derive(Default)]
struct RetainedDeclarations {
    functions: BTreeMap<String, Vec<FunctionEntry>>,
    fields: BTreeMap<String, Option<FieldFacts>>,
    enums: EnumFacts,
}

impl RetainedDeclarations {
    fn accept(&mut self, node: Value) {
        collect_field_facts(&node, &mut self.fields);
        EnumFacts::collect(&node, &mut self.enums);
        if kind(&node) == "FunctionDecl"
            && children(&node)
                .iter()
                .any(|child| kind(child) == "CompoundStmt")
            && let Some(name) = node["name"].as_str()
        {
            self.functions
                .entry(name.to_owned())
                .or_default()
                .push(FunctionEntry {
                    ast: Arc::new(node),
                    by_line: OnceCell::new(),
                });
        }
    }
}

impl<'de> Deserialize<'de> for RetainedDeclarations {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct DeclarationsVisitor;
        impl<'de> Visitor<'de> for DeclarationsVisitor {
            type Value = RetainedDeclarations;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a Clang translation-unit declaration array")
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut retained = RetainedDeclarations::default();
                while let Some(node) = seq.next_element::<Value>()? {
                    retained.accept(node);
                }
                Ok(retained)
            }

            // serde_json::Value accepts a non-array `inner` but `children` then
            // returns no declarations. Preserve that refusal and consume the
            // complete value instead of silently leaving malformed JSON unread.
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                while map.next_entry::<String, Value>()?.is_some() {}
                Ok(RetainedDeclarations::default())
            }

            fn visit_bool<E: serde::de::Error>(self, _: bool) -> Result<Self::Value, E> {
                Ok(RetainedDeclarations::default())
            }

            fn visit_i64<E: serde::de::Error>(self, _: i64) -> Result<Self::Value, E> {
                Ok(RetainedDeclarations::default())
            }

            fn visit_u64<E: serde::de::Error>(self, _: u64) -> Result<Self::Value, E> {
                Ok(RetainedDeclarations::default())
            }

            fn visit_f64<E: serde::de::Error>(self, _: f64) -> Result<Self::Value, E> {
                Ok(RetainedDeclarations::default())
            }

            fn visit_str<E: serde::de::Error>(self, _: &str) -> Result<Self::Value, E> {
                Ok(RetainedDeclarations::default())
            }

            fn visit_string<E: serde::de::Error>(self, _: String) -> Result<Self::Value, E> {
                Ok(RetainedDeclarations::default())
            }

            fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> {
                Ok(RetainedDeclarations::default())
            }
        }
        deserializer.deserialize_any(DeclarationsVisitor)
    }
}

struct RetainedRoot(RetainedDeclarations);

impl<'de> Deserialize<'de> for RetainedRoot {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct RootVisitor;
        impl<'de> Visitor<'de> for RootVisitor {
            type Value = RetainedRoot;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a Clang translation-unit AST object")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut retained = RetainedDeclarations::default();
                let mut root_kind = Value::Null;
                while let Some(key) = map.next_key::<String>()? {
                    if key == "inner" {
                        // `Value` keeps the last duplicate object key. Replace
                        // every earlier declaration set, including its facts.
                        retained = map.next_value::<RetainedDeclarations>()?;
                    } else if key == "kind" {
                        root_kind = map.next_value::<Value>()?;
                    } else {
                        // Keep Value's numeric/depth validation for metadata too.
                        // These root fields are consumed and immediately dropped.
                        map.next_value::<Value>()?;
                    }
                }
                if root_kind.as_str() != Some("TranslationUnitDecl") {
                    return Err(serde::de::Error::custom(
                        "expected Clang TranslationUnitDecl",
                    ));
                }
                Ok(RetainedRoot(retained))
            }
        }
        deserializer.deserialize_map(RootVisitor)
    }
}

impl AstIndex {
    fn from_json(bytes: &[u8], source: &str) -> Result<Self, serde_json::Error> {
        // `from_slice` checks the entire document, including trailing bytes.
        let RetainedRoot(retained) = serde_json::from_slice(bytes)?;
        Ok(Self {
            functions: retained.functions,
            fields: Arc::new(retained.fields),
            enums: Arc::new(retained.enums),
            line_starts: Arc::new(line_starts(source)),
        })
    }

    #[cfg(test)]
    fn from_ast(mut ast: Value, source: &str) -> Self {
        let mut retained = RetainedDeclarations::default();
        if let Some(Value::Array(top_level)) =
            ast.as_object_mut().and_then(|obj| obj.remove("inner"))
        {
            for node in top_level {
                retained.accept(node);
            }
        }
        Self {
            functions: retained.functions,
            fields: Arc::new(retained.fields),
            enums: Arc::new(retained.enums),
            line_starts: Arc::new(line_starts(source)),
        }
    }
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
                    translation_unit_ast_with(&context.source, &unit, |bytes| {
                        AstIndex::from_json(bytes, &context.source)
                    })
                    .ok()
                })
                .as_ref()?;
            let [function] = ast.functions.get(function)?.as_slice() else {
                return None;
            };
            let body_index = children(&function.ast)
                .iter()
                .position(|node| kind(node) == "CompoundStmt")?;
            let by_line = function.by_line.get_or_init(|| {
                Arc::new(candidate_index(
                    &function.ast,
                    body_index,
                    context.source.len(),
                    &ast.line_starts,
                ))
            });
            Some(Self {
                source: Arc::clone(&context.source),
                function: Arc::clone(&function.ast),
                fields: Arc::clone(&ast.fields),
                enums: Arc::clone(&ast.enums),
                by_line: Arc::clone(by_line),
                line_starts: Arc::clone(&ast.line_starts),
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
        let mut matched = self.by_line.get(&line)?.iter().filter_map(|path| {
            let node = node_at_path(&self.function, path)?;
            self.node_tokens(node)
                .is_some_and(|found| found == texts)
                .then_some(node)
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
            address_target: false,
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
        node_line(node, self.source.len(), &self.line_starts)
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

fn line_starts(source: &str) -> Vec<usize> {
    let mut starts = vec![0];
    starts.extend(
        source
            .as_bytes()
            .iter()
            .enumerate()
            .filter_map(|(index, &byte)| (byte == b'\n').then_some(index + 1)),
    );
    starts
}

fn node_line(node: &Value, source_len: usize, line_starts: &[usize]) -> Option<usize> {
    let offset = offset(&node["range"]["begin"])?;
    (offset <= source_len).then(|| line_starts.partition_point(|&start| start <= offset))
}

fn node_at_path<'a>(root: &'a Value, path: &[usize]) -> Option<&'a Value> {
    path.iter()
        .try_fold(root, |node, &index| children(node).get(index))
}

fn candidate_index(
    function: &Value,
    body_index: usize,
    source_len: usize,
    line_starts: &[usize],
) -> BTreeMap<usize, Vec<Vec<usize>>> {
    let mut candidates = Vec::new();
    let mut path = vec![body_index];
    statements(&children(function)[body_index], &mut path, &mut candidates);
    let mut by_line: BTreeMap<usize, Vec<Vec<usize>>> = BTreeMap::new();
    for path in candidates {
        // These paths come from the unchanged tree's own child traversal.
        // The line filter retains the old out-of-range/missing-offset refusal.
        if let Some(node) = node_at_path(function, &path)
            && let Some(line) = node_line(node, source_len, line_starts)
        {
            by_line.entry(line).or_default().push(path);
        }
    }
    by_line
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

fn statements(node: &Value, path: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
    match kind(node) {
        "CompoundStmt" => {
            for (index, child) in children(node).iter().enumerate() {
                path.push(index);
                statement_body(child, path, out);
                path.pop();
            }
        }
        "IfStmt" => {
            let inner = children(node);
            if node["hasElse"].as_bool() == Some(true) && inner.len() >= 3 {
                let index = inner.len() - 2;
                path.push(index);
                statement_body(&inner[index], path, out);
                path.pop();
            }
            if let Some(body) = inner.last() {
                path.push(inner.len() - 1);
                statement_body(body, path, out);
                path.pop();
            }
        }
        "WhileStmt" | "SwitchStmt" => {
            if let Some(body) = children(node).last() {
                path.push(children(node).len() - 1);
                statement_body(body, path, out);
                path.pop();
            }
        }
        "DoStmt" => {
            if let Some(body) = children(node).first() {
                path.push(0);
                statement_body(body, path, out);
                path.pop();
            }
        }
        "ForStmt" => {
            // Clang's five slots are init, condition-variable placeholder,
            // condition, increment, body. The condition is P3's, not P2's.
            if let Some(inner) = node["inner"].as_array()
                && inner.len() == 5
            {
                if !kind(&inner[0]).is_empty() {
                    path.push(0);
                    out.push(path.clone());
                    path.pop();
                }
                if !kind(&inner[3]).is_empty() {
                    path.push(3);
                    out.push(path.clone());
                    path.pop();
                }
                path.push(4);
                statement_body(&inner[4], path, out);
                path.pop();
            }
        }
        "CaseStmt" | "DefaultStmt" | "LabelStmt" => {
            if let Some(body) = children(node).last() {
                path.push(children(node).len() - 1);
                statement_body(body, path, out);
                path.pop();
            }
        }
        _ => {}
    }
}

fn statement_body(node: &Value, path: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
    if is_control(node) {
        statements(node, path, out);
    } else if kind(node) != "ReturnStmt" && !kind(node).is_empty() {
        out.push(path.clone());
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
    /// Rule 10n: the next [`Reader::path`] computes the address of its place
    /// (`&PATH`) instead of reading it, so the place's own type is not read.
    pub(crate) address_target: bool,
    fields: &'a BTreeMap<String, Option<FieldFacts>>,
    enums: &'a EnumFacts,
    pub(crate) never_bound: &'a dyn Fn(&str) -> bool,
}

impl Reader<'_> {
    /// A proven builtin scalar, or (step B) a positively proven enumeration.
    pub(crate) fn scalar(&self, node: &Value) -> bool {
        is_scalar(node)
            || (ENUM_ARM && self.enums.is_enum(node))
            || crate::typed_position_shapes::anonymous_enum_member(self.enums, node)
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
                    return self.path(pointer_source(&pair[1], &pair[0])?, false, true);
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
                    self.path(pointer_source(&children(decl)[0], decl)?, false, true)
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
        let address = std::mem::take(&mut self.address_target) && !store;
        if !(address || self.scalar(node) || (pointer_read && !store && pointer_value(node))) {
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
                    // Rule 10n: `(&X)->f` is `X.f`.
                    let (base, arrow) =
                        crate::typed_position_shapes::arrow_through_address(base, arrow);
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
    qual(node).is_some_and(is_pointer_type)
}

/// The positive pointer test on a type's text (`is_pointer`'s own).
pub(crate) fn is_pointer_type(text: &str) -> bool {
    text.trim_end().ends_with('*')
        && !text.contains("union ")
        && !text.contains("volatile")
        && !text.contains("_Atomic")
        && !text.contains("(")
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

    #[test]
    fn duplicate_same_line_refuses_but_distinct_lines_keep_their_candidates() {
        let repeated = "struct S { int x; }; int f(struct S *p) { p->x = 1; p->x = 1; return 0; }";
        let _scope = enter_source(repeated, CCompilerDialect::C11, None);
        let index = StatementIndex::read("f").expect("typed function");
        let tokens = tokenize("p->x = 1");
        let texts = tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        assert!(index.statement(&texts, 1, |_| true).is_none());

        let separated =
            "struct S { int x; }; int f(struct S *p) {\n p->x = 1;\n p->x = 1;\n return 0; }";
        let _scope = enter_source(separated, CCompilerDialect::C11, None);
        let index = StatementIndex::read("f").expect("typed function");
        assert!(index.statement(&texts, 2, |_| true).is_some());
        assert!(index.statement(&texts, 3, |_| true).is_some());
        assert!(index.statement(&texts, 1, |_| true).is_none());
    }

    #[test]
    fn nested_control_candidates_and_function_ast_are_reused() {
        let source = "struct S { int x; }; int f(struct S *p) {\n if (1) {\n p->x = 1;\n } else {\n p->x = 2;\n }\n return 0; }";
        let _scope = enter_source(source, CCompilerDialect::C11, None);
        let first = StatementIndex::read("f").expect("typed function");
        let second = StatementIndex::read("f").expect("typed function");
        assert!(Arc::ptr_eq(&first.function, &second.function));
        assert!(Arc::ptr_eq(&first.by_line, &second.by_line));
        for (statement, line) in [("p->x = 1", 3), ("p->x = 2", 5)] {
            let tokens = tokenize(statement);
            let texts = tokens
                .iter()
                .map(|token| token.text.as_str())
                .collect::<Vec<_>>();
            assert!(first.statement(&texts, line, |_| true).is_some());
            assert!(second.statement(&texts, line, |_| true).is_some());
        }
    }

    #[test]
    fn compact_ast_keeps_duplicate_definitions_and_conflicting_field_facts() {
        let ast = serde_json::json!({
            "inner": [
                {"kind": "RecordDecl", "tagUsed": "struct", "completeDefinition": true,
                 "inner": [{"kind": "FieldDecl", "id": "field", "type": {"qualType": "int"}}]},
                {"kind": "RecordDecl", "tagUsed": "union", "completeDefinition": true,
                 "inner": [{"kind": "FieldDecl", "id": "field", "type": {"qualType": "int"}}]},
                {"kind": "FunctionDecl", "name": "f", "inner": [{"kind": "CompoundStmt"}]},
                {"kind": "FunctionDecl", "name": "f", "inner": [{"kind": "CompoundStmt"}]},
                {"kind": "FunctionDecl", "name": "g", "inner": []},
                {"kind": "TypedefDecl", "name": "unused"}
            ]
        });
        let compact = AstIndex::from_ast(ast, "a\nb");
        assert_eq!(compact.functions["f"].len(), 2);
        assert!(!compact.functions.contains_key("g"));
        assert_eq!(compact.fields["field"], None);
        assert_eq!(*compact.line_starts, [0, 2]);
        let node = serde_json::json!({"range": {"begin": {"offset": 3}}});
        assert_eq!(node_line(&node, 3, &compact.line_starts), Some(2));
        assert_eq!(node_line(&node, 2, &compact.line_starts), None);
    }

    #[test]
    fn streamed_decoder_matches_full_value_on_real_clang_ast() {
        let source = "struct S { int x; }; int f(struct S *p) { p->x = 1; return 0; }";
        let unit = C11TranslationUnit {
            source,
            ..C11TranslationUnit::default()
        };
        let (full, streamed) = translation_unit_ast_with(source, &unit, |bytes| {
            let full: Value = serde_json::from_slice(bytes)?;
            let streamed = AstIndex::from_json(bytes, source)?;
            Ok((full, streamed))
        })
        .expect("Clang AST");
        let original = AstIndex::from_ast(full, source);
        assert_eq!(streamed.fields, original.fields);
        assert_eq!(streamed.line_starts, original.line_starts);
        assert_eq!(
            streamed.functions.keys().collect::<Vec<_>>(),
            original.functions.keys().collect::<Vec<_>>()
        );
        for (name, functions) in &original.functions {
            assert_eq!(
                streamed.functions[name]
                    .iter()
                    .map(|entry| entry.ast.as_ref())
                    .collect::<Vec<_>>(),
                functions
                    .iter()
                    .map(|entry| entry.ast.as_ref())
                    .collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn streamed_decoder_uses_last_root_inner_and_rejects_incomplete_json() {
        let duplicate = br#"{"kind":"TranslationUnitDecl","inner":[{"kind":"FunctionDecl","name":"old","inner":[{"kind":"CompoundStmt"}]}],"inner":{"unused":true},"inner":[{"kind":"FunctionDecl","name":"new","inner":[{"kind":"CompoundStmt"}]}]}"#;
        let full: Value = serde_json::from_slice(duplicate).unwrap();
        let original = AstIndex::from_ast(full, "");
        let streamed = AstIndex::from_json(duplicate, "").unwrap();
        assert_eq!(
            streamed.functions.keys().collect::<Vec<_>>(),
            original.functions.keys().collect::<Vec<_>>()
        );
        assert_eq!(streamed.functions["new"].len(), 1);
        for invalid in [
            br#"{"inner":["#.as_slice(),
            br#"{"kind":"TranslationUnitDecl","inner":[]} trailing"#.as_slice(),
            br#"{"kind":"RecordDecl","inner":[]}"#.as_slice(),
            br#"{"inner":[]}"#.as_slice(),
            br#"{"kind":"TranslationUnitDecl","ignored":1e400,"inner":[]}"#.as_slice(),
            br#"{"kind":"TranslationUnitDecl","inner":{"ignored":1e400},"inner":[]}"#.as_slice(),
        ] {
            assert!(AstIndex::from_json(invalid, "").is_err());
        }
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
