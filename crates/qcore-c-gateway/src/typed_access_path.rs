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
use std::rc::Rc;
use std::sync::Arc;
use std::sync::Mutex;

use serde::de::{MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::Value;

use crate::clang_reuse::{UnitCache, UnitKey};
use crate::indirect_chain_access::AccessChain;
// Card W47-TYPED-CONST-POINTER-VALUE (rule 10m): the value check and the
// pointer-destination source.
use crate::legacy_c::{C11TranslationUnit, CCompilerDialect, LegacyCertificateRequest, tokenize};
#[cfg(test)]
use crate::typed_c_ast::translation_unit_ast_with;
use crate::typed_c_ast::{children, kind};
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
    /// Card CG11-POINTER-VALUED-STORES (rule 10v): the statement is a
    /// pointer-valued store this rule read; the flow decides it.
    pub(crate) pointer_store: Option<crate::pointer_store::PointerStore>,
    /// Card CG12-VOID-VIEW-READBACK (rule 11j): the pointer destination's
    /// value is a read of a `void *` member; the gateway decides whether it
    /// is a readback of the stored identity.
    pub(crate) readback: Option<crate::void_view::Readback>,
}

/// An AST and the exact token rendering handed to Clang. This is created
/// lazily, only after a statement has reached an access-related refusal.
/// Cloning shares every table (card CG02-ELABORATION-CORE).
#[derive(Clone)]
pub(crate) struct StatementIndex {
    pub(crate) source: Arc<String>,
    pub(crate) function: Arc<Value>,
    pub(crate) fields: Arc<BTreeMap<String, Option<FieldFacts>>>,
    pub(crate) enums: Arc<EnumFacts>,
    /// Card CG13-UNION-AND-BITFIELD-PLACES (rule 10w): union and bit-field
    /// record facts.
    pub(crate) records: Arc<crate::union_bitfield::RecordFacts>,
    /// Card CG14-AGGREGATE-VALUES-AND-INITIALIZERS (rule 11f): record and
    /// typedef facts.
    pub(crate) aggregates: Arc<crate::aggregate_value::AggregateFacts>,
    /// Card CG21-CALL-EXITS-AND-BOUNDARIES (rule 11o): the ids of the unit's
    /// file-scope function declarations carrying a `noreturn` attribute.
    pub(crate) no_return_declarations: Arc<BTreeSet<String>>,
    by_line: Arc<BTreeMap<usize, Vec<Vec<usize>>>>,
    /// The unit's line starts: a node's line is found by binary search.
    line_starts: Arc<Vec<usize>>,
    /// Card CG10 (I04): the analysed source's own tokens per line, read once.
    line_tokens: Arc<OnceCell<BTreeMap<usize, Vec<String>>>>,
    /// Card DEEP-AST-TRANSPORTS-AND-WALL-TIME (rules 13s, 13t): this
    /// function's readings computed once per index.
    reuse: Arc<crate::function_reading_reuse::FunctionReuse>,
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
        // Clang may desugar an anonymous enum typedef to `enum ALIAS`, even
        // though the EnumDecl itself has no tag. Read the spelled alias against
        // its positive EnumType declaration before consulting desugared tags.
        [qual(node), node["type"]["qualType"].as_str()]
            .into_iter()
            .flatten()
            .any(|text| {
                let words = text
                    .split_whitespace()
                    .filter(|word| *word != "const")
                    .collect::<Vec<_>>();
                match words.as_slice() {
                    ["enum", tag] => self.tags.contains(*tag),
                    [name] => self.typedefs.get(*name) == Some(&true),
                    _ => false,
                }
            })
    }
}

thread_local! {
    static ANALYSED_SOURCE: RefCell<Option<SourceContext>> = const { RefCell::new(None) };
    /// Card GATEWAY-ANALYSIS-PATH-OPTIMIZATION: the unit AST indexes and
    /// record-layout dumps this thread's scopes read, reused by a later
    /// scope over the same launch identity ([`crate::clang_reuse`]).
    static UNIT_ASTS: UnitCache<AstIndex> = const { UnitCache::new() };
    static UNIT_LAYOUTS: UnitCache<String> = const { UnitCache::new() };
}

pub(crate) fn release_reuse_caches() {
    UNIT_ASTS.with(UnitCache::clear);
    UNIT_LAYOUTS.with(UnitCache::clear);
}

struct SourceContext {
    source: Arc<String>,
    source_sha256: String,
    frontend_sha256: String,
    frontend_debug: Arc<String>,
    dialect: CCompilerDialect,
    request: LegacyCertificateRequest,
    ast: OnceCell<Option<Rc<AstIndex>>>,
    lifetime_source: LifetimeSourceCache,
}

impl SourceContext {
    /// Card GATEWAY-ANALYSIS-PATH-OPTIMIZATION: this scope's Clang launch
    /// identity, `None` when the launch may not be reused.
    fn launch_key(&self) -> Option<UnitKey> {
        UnitKey::new(self.dialect, &self.frontend_debug, &self.source)
    }
}

/// Keep only facts later typed-path lookups can use. Decode declarations one
/// at a time, retaining function bodies and collected field facts for the CFGs
/// built from this translation unit.
struct AstIndex {
    functions: BTreeMap<String, Vec<FunctionEntry>>,
    fields: Arc<BTreeMap<String, Option<FieldFacts>>>,
    enums: Arc<EnumFacts>,
    records: Arc<crate::union_bitfield::RecordFacts>,
    aggregates: Arc<crate::aggregate_value::AggregateFacts>,
    /// Card CG21-CALL-EXITS-AND-BOUNDARIES (rule 11o).
    no_return_declarations: Arc<BTreeSet<String>>,
    /// Card CG22-INDIRECT-CALLS-AND-CALLBACKS (rule 11p): constant tables.
    indirect_tables: Arc<crate::indirect_calls::TableFacts>,
    /// Card CG22-OP-CALLBACKS-AND-SERVED-QUEUES (rule 12n): the stores into
    /// function-pointer members and their sources.
    member_facts: Arc<crate::callback_member_closure::MemberFacts>,
    /// Card OP-CONSTRUCTOR-DECLARATIONS (rules 12o, 13w, 14e): the unit's
    /// classified stores into the members a result state names, by the set
    /// of members asked for (computed once per set).
    state_facts: std::cell::RefCell<
        BTreeMap<BTreeSet<String>, std::rc::Rc<crate::constructed_state::StateFacts>>,
    >,
    line_starts: Arc<Vec<usize>>,
    line_tokens: Arc<OnceCell<BTreeMap<usize, Vec<String>>>>,
}

struct FunctionEntry {
    ast: Arc<Value>,
    by_line: OnceCell<Arc<BTreeMap<usize, Vec<Vec<usize>>>>>,
    /// Card DEEP-AST-TRANSPORTS-AND-WALL-TIME (rules 13s, 13t).
    reuse: Arc<crate::function_reading_reuse::FunctionReuse>,
}

#[derive(Default)]
struct RetainedDeclarations {
    functions: BTreeMap<String, Vec<FunctionEntry>>,
    fields: BTreeMap<String, Option<FieldFacts>>,
    enums: EnumFacts,
    records: crate::union_bitfield::RecordFacts,
    aggregates: crate::aggregate_value::AggregateFacts,
    /// Card CG21-CALL-EXITS-AND-BOUNDARIES (rule 11o).
    no_return_declarations: BTreeSet<String>,
    /// Card CG22-INDIRECT-CALLS-AND-CALLBACKS (rule 11p).
    indirect_tables: crate::indirect_calls::TableFacts,
    /// Card CG22-OP-CALLBACKS-AND-SERVED-QUEUES (rule 12n).
    member_facts: crate::callback_member_closure::MemberFacts,
}

impl RetainedDeclarations {
    fn accept(&mut self, node: Value) {
        // Card CG21-CALL-EXITS-AND-BOUNDARIES (rule 11o).
        if crate::call_exits::declares_no_return(&node)
            && let Some(id) = node["id"].as_str()
        {
            self.no_return_declarations.insert(id.to_owned());
        }
        collect_field_facts(&node, &mut self.fields);
        EnumFacts::collect(&node, &mut self.enums);
        crate::union_bitfield::RecordFacts::collect(&node, &mut self.records);
        // Card CG14-AGGREGATE-VALUES-AND-INITIALIZERS (rule 11f).
        crate::aggregate_value::AggregateFacts::collect(&node, &mut self.aggregates);
        // Card CG22-INDIRECT-CALLS-AND-CALLBACKS (rule 11p).
        crate::indirect_calls::TableFacts::collect(&node, &mut self.indirect_tables);
        // Card CG22-OP-CALLBACKS-AND-SERVED-QUEUES (rule 12n).
        self.member_facts.collect(&node);
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
                    reuse: Arc::default(),
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
    #[cfg(test)]
    fn from_json(bytes: &[u8], source: &str) -> Result<Self, serde_json::Error> {
        Self::from_reader(&mut std::io::Cursor::new(bytes), source)
    }

    fn from_reader(
        reader: &mut (impl std::io::Read + std::io::Seek + Send),
        source: &str,
    ) -> Result<Self, serde_json::Error> {
        // `from_slice` checks the entire document, including trailing bytes.
        // Card RDHTTP-HEADER-SUMMARY-GAP (rules 13o, 13p): a document the
        // default recursion limit refuses is read again without it when it
        // nests within the admitted bound.
        let RetainedRoot(retained) = crate::deep_unit_ast::from_reader(reader)?;
        Ok(Self {
            functions: retained.functions,
            fields: Arc::new(retained.fields),
            enums: Arc::new(retained.enums),
            records: Arc::new(retained.records),
            aggregates: Arc::new(retained.aggregates),
            no_return_declarations: Arc::new(retained.no_return_declarations),
            indirect_tables: Arc::new(retained.indirect_tables),
            member_facts: Arc::new(retained.member_facts),
            state_facts: std::cell::RefCell::default(),
            line_starts: Arc::new(line_starts(source)),
            line_tokens: Arc::default(),
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
            records: Arc::new(retained.records),
            aggregates: Arc::new(retained.aggregates),
            no_return_declarations: Arc::new(retained.no_return_declarations),
            indirect_tables: Arc::new(retained.indirect_tables),
            member_facts: Arc::new(retained.member_facts),
            state_facts: std::cell::RefCell::default(),
            line_starts: Arc::new(line_starts(source)),
            line_tokens: Arc::default(),
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
            lifetime_source: LifetimeSourceCache::default(),
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

/// Card CG30-DIALECTS-FLAGS-TYPES (rule 11y): one definition's Clang AST by
/// Clang's own name filter, for a unit whose whole-unit index is unavailable
/// (the measured instance was librdkafka's `rdhttp.c`, whose translation-unit
/// JSON nests over 200 deep, beyond `serde_json`'s recursion limit of 128; card
/// RDHTTP-HEADER-SUMMARY-GAP's rule 13o now reads that index; card
/// DEEP-AST-TRANSPORTS-AND-WALL-TIME's rule 13q reads this filtered stream
/// within the same bound, `typed_c_ast::unit_declarations`). `None`
/// outside an analysed source, on any Clang or JSON failure, and unless
/// exactly one definition with a body has this exact name.
pub(crate) fn definition_ast(function: &str) -> Option<Value> {
    ANALYSED_SOURCE.with(|cell| {
        let borrow = cell.borrow();
        let context = borrow.as_ref()?;
        let unit = C11TranslationUnit {
            source: &context.source,
            include_paths: context.request.include_paths.clone(),
            definitions: context.request.definitions.clone(),
            preprocessor_options: context.request.preprocessor_options.clone(),
            dialect: context.dialect,
            ..C11TranslationUnit::default()
        };
        let declarations =
            crate::typed_c_ast::unit_declarations(&context.source, &unit, function).ok()?;
        let mut definitions = declarations.into_iter().filter(|node| {
            kind(node) == "FunctionDecl"
                && node["name"].as_str() == Some(function)
                && children(node)
                    .iter()
                    .any(|child| kind(child) == "CompoundStmt")
        });
        let definition = definitions.next()?;
        definitions.next().is_none().then_some(definition)
    })
}

type LifetimeSourceResult = Result<crate::loop_lifetime_source::SourceFacts, String>;
#[derive(Default)]
struct LifetimeSourceCache {
    // One entry bounds retained memory. This object belongs to one exact
    // source/frontend scope and is recreated for independent replay.
    entry: RefCell<Option<(String, LifetimeSourceResult)>>,
}
impl LifetimeSourceCache {
    fn get_or_read(
        &self,
        key: String,
        read: impl FnOnce() -> LifetimeSourceResult,
    ) -> LifetimeSourceResult {
        if let Some((old, value)) = self.entry.borrow().as_ref()
            && old == &key
        {
            return value.clone();
        }
        let value = read();
        self.entry.replace(Some((key, value.clone())));
        value
    }
}
fn lifetime_source_dependencies(
    contracts: &qcore_schemas::contract::ResourceContracts,
) -> Result<String, String> {
    use qcore_schemas::contract::ResourceParameterMode;
    // SourceFacts reads retaining holder/retention/removal declarations and
    // effective Released parameter modes (including imported/inferred modes).
    // Ordinary declarations override imports, exactly as contract() does.
    let mut effective = contracts.inferred_imports().clone();
    effective.extend(contracts.functions().clone());
    effective.retain(|_, contract| {
        contract
            .parameters
            .iter()
            .any(|mode| matches!(mode, ResourceParameterMode::Released { .. }))
    });
    serde_json::to_vec(&(
        contracts.source_assumptions(),
        contracts.retaining(),
        contracts.guarded_retaining(),
        contracts.declared_guarded(),
        effective,
    ))
    .map(|bytes| qcore_schemas::sha256_hex(&bytes))
    .map_err(|error| error.to_string())
}

/// S3 local prerequisites rederive their typed call/branch evidence from the
/// retained analyzed source and exact frontend request, including on certificate
/// replay. Ambient run facts are not a proof input for this bounded route.
pub(crate) fn current_lifetime_source(
    contracts: &qcore_schemas::contract::ResourceContracts,
) -> Result<crate::loop_lifetime_source::SourceFacts, String> {
    ANALYSED_SOURCE.with(|cell| {
        let borrow = cell.borrow();
        let context = borrow
            .as_ref()
            .ok_or("no retained analyzed source context")?;
        let unit = C11TranslationUnit {
            source: &context.source,
            include_paths: context.request.include_paths.clone(),
            definitions: context.request.definitions.clone(),
            preprocessor_options: context.request.preprocessor_options.clone(),
            dialect: context.dialect,
            input: context.request.input.clone(),
            ..C11TranslationUnit::default()
        };
        let key = lifetime_source_dependencies(contracts)?;
        context.lifetime_source.get_or_read(key, || {
            crate::loop_lifetime_source::read_unit(&context.source, &unit, contracts)
        })
    })
}

/// Card **W43-ABI-ENUM-AND-CONST-PP** (rule 12u): Clang's declarations named
/// by `filter` (its substring name filter) after `suffix` is appended to the
/// analysed translation unit, read with the unit's own frontend request.
/// The probe is raw transport: the caller owns the exact shape it reads.
/// `None` outside an analysed source, when the source already spells the
/// filter (a probe name the source could forge), and on any Clang or JSON
/// failure.
pub(crate) fn appended_declarations(suffix: &str, filter: &str) -> Option<Vec<Value>> {
    ANALYSED_SOURCE.with(|cell| {
        let borrow = cell.borrow();
        let context = borrow.as_ref()?;
        if context.source.contains(filter) {
            return None;
        }
        let probed = format!("{}\n{suffix}\n", context.source);
        let unit = C11TranslationUnit {
            source: &probed,
            include_paths: context.request.include_paths.clone(),
            definitions: context.request.definitions.clone(),
            preprocessor_options: context.request.preprocessor_options.clone(),
            dialect: context.dialect,
            ..C11TranslationUnit::default()
        };
        crate::typed_c_ast::unit_declarations(&probed, &unit, filter).ok()
    })
}

/// Card CG13-UNION-AND-BITFIELD-PLACES (rule 10w): Clang's record layout
/// dump of the analysed translation unit, with the frontend request the AST
/// was read with. `None` outside an analysed source or on any Clang failure.
pub(crate) fn record_layout_dump() -> Option<String> {
    ANALYSED_SOURCE.with(|cell| {
        let borrow = cell.borrow();
        let context = borrow.as_ref()?;
        let unit = C11TranslationUnit {
            source: &context.source,
            include_paths: context.request.include_paths.clone(),
            definitions: context.request.definitions.clone(),
            preprocessor_options: context.request.preprocessor_options.clone(),
            dialect: context.dialect,
            ..C11TranslationUnit::default()
        };
        // Card GATEWAY-ANALYSIS-PATH-OPTIMIZATION: one launch per unit,
        // not one per consultation; a failure is not kept.
        UNIT_LAYOUTS
            .with(|layouts| {
                layouts.get_or_launch(context.launch_key(), || {
                    crate::typed_c_ast::record_layout_dump(&context.source, &unit).ok()
                })
            })
            .map(|layout| layout.as_ref().clone())
    })
}

/// Card CG17-LAYOUT-DERIVED-EXTENTS (rule 11u): Clang's evaluation of each
/// integer constant expression over the analysed translation unit, in one
/// probe (`layout_extents::clang_constants`). Empty outside an analysed
/// source or on any Clang failure.
pub(crate) fn analysed_unit_constants(
    expressions: &[String],
) -> std::collections::BTreeMap<String, i128> {
    ANALYSED_SOURCE.with(|cell| {
        let borrow = cell.borrow();
        let Some(context) = borrow.as_ref() else {
            return Default::default();
        };
        let unit = C11TranslationUnit {
            source: &context.source,
            include_paths: context.request.include_paths.clone(),
            definitions: context.request.definitions.clone(),
            preprocessor_options: context.request.preprocessor_options.clone(),
            dialect: context.dialect,
            ..C11TranslationUnit::default()
        };
        crate::layout_extents::clang_constants(&context.source, &unit, expressions)
    })
}

/// The analysed unit's AST index, read from Clang once per source context,
/// and (card GATEWAY-ANALYSIS-PATH-OPTIMIZATION) shared with a later context
/// of this thread over the same launch identity.
fn unit_ast(context: &SourceContext) -> Option<&AstIndex> {
    context
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
            UNIT_ASTS.with(|asts| {
                asts.get_or_launch(context.launch_key(), || {
                    crate::typed_c_ast::translation_unit_ast_read(
                        &context.source,
                        &unit,
                        |reader| AstIndex::from_reader(reader, &context.source),
                    )
                    .ok()
                })
            })
        })
        .as_deref()
}

/// Card **CG22-INDIRECT-CALLS-AND-CALLBACKS** (rule 11p): the analysed
/// unit's function bodies and constant tables, for the indirect-call pass.
pub(crate) struct IndirectCallUnit<'a> {
    ast: &'a AstIndex,
    source: &'a str,
}

impl IndirectCallUnit<'_> {
    /// S3 call evidence is tied to this exact preprocessed source, beyond
    /// its per-function token identity. The run records configuration too.
    pub(crate) fn source_sha256(&self) -> String {
        qcore_schemas::sha256_hex(self.source.as_bytes())
    }

    /// Card OUT-PARAMETER-SCALAR-FRAME-PREMISE: whether `node`'s type is
    /// positively an enumeration (the owner-aware enum arm's reading).
    pub(crate) fn is_enum(&self, node: &Value) -> bool {
        self.ast.enums.is_enum(node)
    }

    /// The unique body Clang read for `name`.
    pub(crate) fn function(&self, name: &str) -> Option<&Value> {
        match self.ast.functions.get(name)?.as_slice() {
            [entry] => Some(&entry.ast),
            _ => None,
        }
    }

    pub(crate) fn tables(&self) -> &crate::indirect_calls::TableFacts {
        &self.ast.indirect_tables
    }

    /// Card MK-EVENT-FIRST-MEMBER-CELL (rule 38 (1)): the unit's record facts.
    pub(crate) fn records(&self) -> &crate::union_bitfield::RecordFacts {
        &self.ast.records
    }

    /// Card CG22-OP-CALLBACKS-AND-SERVED-QUEUES (rule 12n): the unit's
    /// function-pointer member facts.
    pub(crate) fn member_facts(&self) -> &crate::callback_member_closure::MemberFacts {
        &self.ast.member_facts
    }

    /// Card OP-CONSTRUCTOR-DECLARATIONS (rules 12o, 13w, 14e): the unit's
    /// classified stores into `keys`, over every function body it defines.
    pub(crate) fn state_facts(
        &self,
        keys: &BTreeSet<String>,
    ) -> std::rc::Rc<crate::constructed_state::StateFacts> {
        if let Some(found) = self.ast.state_facts.borrow().get(keys) {
            return found.clone();
        }
        let functions = self.ast.functions.iter().flat_map(|(name, entries)| {
            entries
                .iter()
                .map(move |entry| (name.as_str(), entry.ast.as_ref()))
        });
        let facts = std::rc::Rc::new(crate::constructed_state::collect(
            &self.ast.member_facts,
            functions,
            keys,
        ));
        self.ast
            .state_facts
            .borrow_mut()
            .insert(keys.clone(), facts.clone());
        facts
    }

    /// A node's tokens, exactly as the gateway tokenizes the analysed source.
    pub(crate) fn node_tokens(&self, node: &Value) -> Option<Vec<String>> {
        let start = offset(&node["range"]["begin"])?;
        let end = offset(&node["range"]["end"])?
            .checked_add(node["range"]["end"]["tokLen"].as_u64()? as usize)?;
        let text = self.source.get(start..end)?;
        Some(tokenize(text).into_iter().map(|token| token.text).collect())
    }

    /// The line a node begins on.
    pub(crate) fn node_line(&self, node: &Value) -> Option<usize> {
        node_line(node, self.source.len(), &self.ast.line_starts)
    }
}

/// Card OUT-PARAMETER-SCALAR-FRAME-PREMISE (rule 11r(7)): whether the
/// analysed unit's build leaves C11 6.5p7's effective-type rule in force: a
/// source context is entered and its frontend options do not carry
/// `-fno-strict-aliasing`. No context is no evidence, so `false`.
pub(crate) fn strict_aliasing_in_force() -> bool {
    ANALYSED_SOURCE.with(|cell| {
        cell.borrow().as_ref().is_some_and(|context| {
            !context.request.preprocessor_options.iter().any(|option| {
                matches!(
                    option,
                    crate::legacy_c::CPreprocessorOption::FrontendFlag(
                        crate::legacy_c::CFrontendFlag::NoStrictAliasing
                    )
                )
            })
        })
    })
}

/// Run `f` over the analysed unit's AST, when a source context is entered
/// and Clang read it.
pub(crate) fn with_indirect_call_unit<T>(f: impl FnOnce(&IndirectCallUnit<'_>) -> T) -> Option<T> {
    ANALYSED_SOURCE.with(|cell| {
        let borrow = cell.borrow();
        let context = borrow.as_ref()?;
        let ast = unit_ast(context)?;
        Some(f(&IndirectCallUnit {
            ast,
            source: &context.source,
        }))
    })
}

impl StatementIndex {
    pub(crate) fn read(function: &str) -> Option<Self> {
        ANALYSED_SOURCE.with(|cell| {
            let borrow = cell.borrow();
            let context = borrow.as_ref()?;
            let ast = unit_ast(context)?;
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
                records: Arc::clone(&ast.records),
                aggregates: Arc::clone(&ast.aggregates),
                no_return_declarations: Arc::clone(&ast.no_return_declarations),
                by_line: Arc::clone(by_line),
                line_starts: Arc::clone(&ast.line_starts),
                line_tokens: Arc::clone(&ast.line_tokens),
                reuse: Arc::clone(&function.reuse),
            })
        })
    }

    /// Card DEEP-AST-TRANSPORTS-AND-WALL-TIME (rule 13s):
    /// `call_exits::resolve(&self.function, callee,
    /// &self.no_return_declarations)`, from the function's table.
    pub(crate) fn call_resolution(&self, callee: &str) -> crate::call_exits::Resolution {
        self.reuse
            .resolution(&self.function, callee, &self.no_return_declarations)
    }

    /// Card DEEP-AST-TRANSPORTS-AND-WALL-TIME (rule 13t): every node's
    /// [`Self::node_line`] and [`Self::node_end_line`], in child order.
    pub(crate) fn line_tree(&self) -> &crate::function_reading_reuse::LineTree {
        self.reuse
            .line_tree(&self.function, &|node| self.node_line(node), &|node| {
                self.node_end_line(node)
            })
    }

    /// Card DEEP-AST-TRANSPORTS-AND-WALL-TIME (rule 13t):
    /// [`Self::node_tokens`], each source span tokenized once.
    pub(crate) fn node_tokens_once(&self, node: &Value) -> Option<Arc<Vec<String>>> {
        let start = offset(&node["range"]["begin"])?;
        let end = offset(&node["range"]["end"])?
            .checked_add(node["range"]["end"]["tokLen"].as_u64()? as usize)?;
        self.reuse
            .span_tokens((start, end), || self.span_tokens(start, end))
    }

    /// Require a unique complete Clang statement whose tokens and internal
    /// line are exactly the gateway statement's tokens and line.
    #[cfg(test)]
    pub(crate) fn statement(
        &self,
        texts: &[&str],
        line: usize,
        never_bound: impl Fn(&str) -> bool,
    ) -> Option<StatementPlan> {
        self.statement_with(texts, line, &|| None, never_bound, None)
    }

    /// [`Self::statement`], where several candidates are told apart by the
    /// gateway's own `occurrence` of the run (card CG10, I04). The reader's
    /// parity decisions go into `decisions` (rule 12q).
    pub(crate) fn statement_with(
        &self,
        texts: &[&str],
        line: usize,
        occurrence: &dyn Fn() -> Option<crate::typed_place::Occurrence>,
        never_bound: impl Fn(&str) -> bool,
        decisions: Option<&crate::consultation_decisions::Decisions>,
    ) -> Option<StatementPlan> {
        let node = self.statement_node(texts, line, occurrence)?;
        let mut reader = self.reader(&never_bound).recording(decisions);
        // Card CG11 (rule 10v): only the statement route reads a pointer store.
        reader.pointer_stores = true;
        reader.statement(node)?;
        Some(reader.plan)
    }

    /// The unique complete Clang statement whose tokens and internal line are
    /// exactly `texts` and `line`, several told apart by `occurrence`. Card
    /// CG10-INDIRECT-ACCESS-SOLE-RESIDUALS round 2 (shared item, read out of
    /// [`Self::statement_with`] unchanged).
    pub(crate) fn statement_node(
        &self,
        texts: &[&str],
        line: usize,
        occurrence: &dyn Fn() -> Option<crate::typed_place::Occurrence>,
    ) -> Option<&Value> {
        let matched = self
            .by_line
            .get(&line)?
            .iter()
            .filter_map(|path| {
                let node = node_at_path(&self.function, path)?;
                self.node_tokens(node)
                    .is_some_and(|found| found == texts)
                    .then_some(node)
            })
            .collect::<Vec<_>>();
        match matched.as_slice() {
            [node] => Some(*node),
            [_, _, ..] => self.by_occurrence(&matched, line, &occurrence()?),
            [] => None,
        }
    }

    /// A fresh reader over this function's field and enum facts.
    pub(crate) fn reader<'a>(&'a self, never_bound: &'a dyn Fn(&str) -> bool) -> Reader<'a> {
        Reader {
            address_target: false,
            pointer_store_target: false,
            pointer_stores: false,
            aggregate_target: false,
            opaque_call: None,
            plan: StatementPlan {
                scalar: TypedScalarPlan::default(),
                chains: Vec::new(),
                accesses: 0,
                borrowed_destination: false,
                pointer_store: None,
                readback: None,
            },
            fields: &self.fields,
            enums: &self.enums,
            records: &self.records,
            never_bound,
            function: Some(&self.function),
            decisions: None,
        }
    }

    pub(crate) fn node_line(&self, node: &Value) -> Option<usize> {
        node_line(node, self.source.len(), &self.line_starts)
    }

    /// Card CG02-ELABORATION-CORE: the line a node's range ends on.
    pub(crate) fn node_end_line(&self, node: &Value) -> Option<usize> {
        let offset = offset(&node["range"]["end"])?;
        (offset <= self.source.len())
            .then(|| self.line_starts.partition_point(|&start| start <= offset))
    }

    /// Card CG10 (I04): the analysed source's own tokens per line, read once.
    fn original_lines(&self) -> &BTreeMap<usize, Vec<String>> {
        self.line_tokens.get_or_init(|| {
            let mut lines: BTreeMap<usize, Vec<String>> = BTreeMap::new();
            for token in tokenize(&self.source) {
                lines.entry(token.line).or_default().push(token.text);
            }
            lines
        })
    }

    /// Card CG07-GENERIC-AND-STATEMENT-EXPRESSIONS (rule 11g): the analysed
    /// source's own tokens on `line` (empty for a line with none).
    pub(crate) fn original_line_texts(&self, line: usize) -> &[String] {
        self.original_lines()
            .get(&line)
            .map_or(&[][..], Vec::as_slice)
    }

    /// Card CG10 (I04): the candidate the gateway's occurrence names.
    pub(crate) fn by_occurrence<'a>(
        &self,
        candidates: &[&'a Value],
        line: usize,
        occurrence: &crate::typed_place::Occurrence,
    ) -> Option<&'a Value> {
        let original = self.original_lines();
        crate::typed_place::by_occurrence(
            candidates,
            occurrence,
            original.get(&line).map_or(&[][..], Vec::as_slice),
            line,
            |node| {
                let begin = offset(&node["range"]["begin"])?;
                let end = offset(&node["range"]["end"])?;
                let last = (end <= self.source.len())
                    .then(|| self.line_starts.partition_point(|&start| start <= end))?;
                Some((begin, self.node_line(node)?, last))
            },
        )
    }

    pub(crate) fn node_tokens(&self, node: &Value) -> Option<Vec<String>> {
        let start = offset(&node["range"]["begin"])?;
        let end = offset(&node["range"]["end"])?
            .checked_add(node["range"]["end"]["tokLen"].as_u64()? as usize)?;
        self.span_tokens(start, end)
    }

    /// The tokens of the source span `start..end`, a final `;` dropped (the
    /// text [`Self::node_tokens`] had, moved).
    fn span_tokens(&self, start: usize, end: usize) -> Option<Vec<String>> {
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
    /// Card CG11 (rule 10v): the next [`Reader::path`] stores a pointer into
    /// its place, so the place's type is not required to be a scalar.
    pub(crate) pointer_store_target: bool,
    /// Card CG11 (rule 10v): this reader reads a pointer-valued store (the
    /// statement route only).
    pub(crate) pointer_stores: bool,
    /// Card CG14-AGGREGATE-VALUES-AND-INITIALIZERS (rule 11f): the next
    /// [`Reader::path`] reads or stores a whole custody-free record, so the
    /// place's type is not required to be a scalar.
    pub(crate) aggregate_target: bool,
    /// Card CG10-INDIRECT-ACCESS-SOLE-RESIDUALS round 2 (rule 14a; shared
    /// item added to this reader): the one call of the statement whose value
    /// the caller lowers on its own, read here as an opaque scalar operand.
    /// `None` everywhere else, which keeps every call refused.
    pub(crate) opaque_call: Option<&'a Value>,
    fields: &'a BTreeMap<String, Option<FieldFacts>>,
    enums: &'a EnumFacts,
    /// Card CG13 (rule 10w): union and bit-field record facts.
    records: &'a crate::union_bitfield::RecordFacts,
    pub(crate) never_bound: &'a dyn Fn(&str) -> bool,
    /// Card CG10-COMMON-TYPED-PLACE (rule 10t): the function whose body the
    /// reader reads, for the tracked-root place's DeclID identity.
    function: Option<&'a Value>,
    /// Card GAPO-PARITY-DECISIONS-NONDETERMINISTIC (rule 12q): the record
    /// this reader's parity decisions go into, owned by the consumer that
    /// records them; `None` keeps nothing.
    decisions: Option<&'a crate::consultation_decisions::Decisions>,
}

impl<'a> Reader<'a> {
    /// Card CG11 (rule 10v): the unit's field facts.
    pub(crate) fn fields(&self) -> &'a BTreeMap<String, Option<FieldFacts>> {
        self.fields
    }

    /// Card GAPO-PARITY-DECISIONS-NONDETERMINISTIC (rule 12q): this reader
    /// writes its parity decisions into `decisions`, which its consumer owns
    /// and records.
    pub(crate) fn recording(
        mut self,
        decisions: Option<&'a crate::consultation_decisions::Decisions>,
    ) -> Self {
        self.decisions = decisions;
        self
    }

    /// Rule 12q: the record this reader writes its parity decisions into.
    pub(crate) fn decisions(&self) -> Option<&'a crate::consultation_decisions::Decisions> {
        self.decisions
    }

    /// Card CG11 (rule 10v): the function whose body the reader reads.
    pub(crate) fn function(&self) -> Option<&'a Value> {
        self.function
    }

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
                    // Card CG12-VOID-VIEW-READBACK (rule 11j): or an explicit
                    // view to the destination's type (asked first: `strip`
                    // stops at a cast, which `path` then always refused).
                    let source = crate::void_view::explicit_source(&pair[1], &pair[0])
                        .or_else(|| pointer_source(&pair[1], &pair[0]))?;
                    self.plan.readback = crate::void_view::readback_of(source)
                        .filter(|_| crate::void_view::local_destination(&pair[0], self.function));
                    return self.path(source, false, true);
                }
                // Card CG11-POINTER-VALUED-STORES (rule 10v): a store into a
                // place of pointer type, which `store` never admits.
                if self.pointer_stores && crate::pointer_store::is_pointer_place(&pair[0]) {
                    return crate::pointer_store::read_store(self, &pair[0], &pair[1]);
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
                    // Card CG12-VOID-VIEW-READBACK (rule 11j): or an explicit
                    // view to the declared type (asked first, as above).
                    let value = &children(decl)[0];
                    let source = crate::void_view::explicit_source(value, decl)
                        .or_else(|| pointer_source(value, decl))?;
                    self.plan.readback = crate::void_view::readback_of(source)
                        .filter(|_| crate::void_view::local_declaration(decl));
                    self.path(source, false, true)
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
            // Rule 14a: exactly the call the caller lowers itself, whose
            // value is a scalar; it reads nothing here.
            "CallExpr"
                if self
                    .opaque_call
                    .is_some_and(|call| std::ptr::eq(call, node))
                    && self.scalar(node) =>
            {
                Some(())
            }
            _ => None,
        }
    }

    pub(crate) fn path(&mut self, node: &Value, store: bool, pointer_read: bool) -> Option<()> {
        let address = std::mem::take(&mut self.address_target) && !store;
        let pointer_store = std::mem::take(&mut self.pointer_store_target) && store;
        // Card CG14-AGGREGATE-VALUES-AND-INITIALIZERS (rule 11f): a whole
        // custody-free record read or stored, whose class the caller checked.
        let aggregate = std::mem::take(&mut self.aggregate_target);
        if !(address
            || pointer_store
            || aggregate
            || self.scalar(node)
            || (pointer_read && !store && pointer_value(node)))
        {
            return None;
        }
        let mut steps = Vec::new();
        // Card CG13-UNION-AND-BITFIELD-PLACES (rule 10w): the union-member
        // and bit-field steps, each with whether a pointer is read from
        // memory after it (`seen_hop`, walking from the outermost step in).
        let mut member_steps = Vec::new();
        let mut seen_hop = false;
        // Card CG12-VOID-VIEW-READBACK (rule 11j): a `void *` view may stand
        // only where a pointer is read through (`pointer_operand`), and one
        // over a loaded pointer needs a never-bound root (`load_view`).
        let mut pointer_operand = false;
        let mut after_view = false;
        let mut load_view = false;
        let mut saw_view = false;
        let mut pending_key = None;
        let mut root_view_key: Option<String> = None;
        let mut cursor = node;
        loop {
            cursor = strip(cursor)?;
            if std::mem::take(&mut after_view) {
                if kind(cursor) == "DeclRefExpr" {
                    root_view_key = pending_key.take();
                } else {
                    load_view = true;
                }
            }
            // Card CAST-FORMALIZATION (rule 12h): a qualifier conversion of the
            // pointer read through is the same object at the same type.
            if pointer_operand && let Some(inner) = crate::cast_semantics::transparent_cast(cursor)
            {
                cursor = inner;
                continue;
            }
            if std::mem::take(&mut pointer_operand)
                && let Some(view) = crate::void_view::explicit_view(cursor)
            {
                pending_key = qual(cursor).and_then(crate::void_view::pointee_key);
                cursor = view.ok()?;
                after_view = true;
                saw_view = true;
                continue;
            }
            match kind(cursor) {
                "MemberExpr" => {
                    let field_id = cursor["referencedMemberDecl"].as_str()?;
                    let field = self.fields.get(field_id)?.as_ref()?;
                    let storage = self.records.storage(field, field_id);
                    let member = cursor["name"].as_str()?.to_owned();
                    let base = children(cursor).first()?;
                    let arrow = cursor["isArrow"].as_bool() == Some(true);
                    // Rule 10n: `(&X)->f` is `X.f`.
                    let (base, arrow) =
                        crate::typed_position_shapes::arrow_through_address(base, arrow);
                    let base_type = qual(base)?;
                    if (base_type.contains("union ")
                        && storage != crate::union_bitfield::MemberStorage::Union)
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
                    if storage != crate::union_bitfield::MemberStorage::Ordinary {
                        member_steps.push(crate::union_bitfield::MemberStep {
                            storage,
                            field: field_id.to_owned(),
                            record: self.records.union_owner.get(field_id).cloned(),
                            union_type: if arrow {
                                base_type.trim_end().trim_end_matches('*').trim_end()
                            } else {
                                base_type
                            }
                            .to_owned(),
                            hop_after: seen_hop,
                        });
                    }
                    seen_hop |= arrow;
                    steps.push((if arrow { "->" } else { "." }, member));
                    cursor = base;
                    pointer_operand = arrow;
                }
                "ArraySubscriptExpr" => {
                    let pair = children(cursor);
                    if pair.len() != 2 || !self.scalar(&pair[1]) {
                        return None;
                    }
                    self.expression(&pair[1])?;
                    seen_hop |= crate::union_bitfield::subscript_hop(&pair[0]);
                    steps.push(("[]", String::new()));
                    cursor = &pair[0];
                    pointer_operand = true;
                }
                "UnaryOperator" if cursor["opcode"].as_str() == Some("*") => {
                    seen_hop = true;
                    steps.push(("*", String::new()));
                    cursor = children(cursor).first()?;
                    pointer_operand = true;
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
        // Card CG12-VOID-VIEW-READBACK (rule 11j): a view over a loaded
        // pointer is read only on a root the closure proves never bound.
        if saw_view
            && !crate::void_view::view_position(
                if load_view {
                    crate::void_view::ViewPosition::Load
                } else {
                    crate::void_view::ViewPosition::Root
                },
                (self.never_bound)(root),
                false,
            )
        {
            return None;
        }
        // A view of the root as `struct TAG *` must not escape C02-P-ALIAS:
        // where the body declares another name of that record-pointer class,
        // a peer's release would relate it, and the `void *` root has no
        // class to carry the obligation. Refused there.
        if root_view_key.as_deref().is_some_and(|key| {
            crate::void_view::view_has_alias_peers(
                self.function,
                cursor["referencedDecl"]["id"].as_str(),
                key,
            )
        }) {
            return None;
        }
        // Card CG13 (rule 10w): decide every union-member and bit-field step;
        // a store through a union that can hold a pointer carries W43's
        // refused-write frame.
        let union_frame = if member_steps.is_empty() {
            false
        } else {
            // Card CG11's pointer-valued store (rule 10v) reaches this reader
            // with `pointer_store`: through a union member it is refused by
            // rule 10w's table, whatever value CG11 classified.
            let access = if pointer_store {
                crate::union_bitfield::Access::PointerStore
            } else if store {
                crate::union_bitfield::Access::ScalarStore
            } else if !address && pointer_read && pointer_value(node) {
                crate::union_bitfield::Access::PointerRead
            } else {
                crate::union_bitfield::Access::ScalarRead
            };
            crate::union_bitfield::decide_path(
                &member_steps,
                access,
                self.records,
                self.function,
                node,
            )
            .ok()?
        };
        steps.reverse();
        let first = steps.first()?;
        // Card CG10-INDIRECT-ACCESS-SOLE-BLOCKERS (rule 12j): a place rule 12j
        // admitted reads no pointer, so it carries no chain.
        let mut record_object = false;
        if matches!(first.0, "->" | "*" | "[]") {
            if !self.plan.scalar.bases.contains(&root.to_owned()) {
                self.plan.scalar.bases.push(root.to_owned());
            }
            if first.0 == "->" && !self.plan.scalar.record_bases.contains(&root.to_owned()) {
                self.plan.scalar.record_bases.push(root.to_owned());
            }
        } else if !(self.never_bound)(root) {
            // Card CG10-INDIRECT-ACCESS-SOLE-BLOCKERS (rule 12j): asked only
            // where the never-bound premise refused, a `.`-only scalar place
            // on a record object of this function's automatic storage.
            let access = if address || pointer_store || aggregate || !self.scalar(node) {
                crate::record_object_place::Access::Other
            } else if store {
                crate::record_object_place::Access::ScalarStore
            } else {
                crate::record_object_place::Access::ScalarRead
            };
            let storage = match (self.function, cursor["referencedDecl"]["id"].as_str()) {
                (Some(function), Some(decl)) => {
                    crate::record_object_place::root_storage(function, decl)
                }
                _ => crate::record_object_place::RootStorage::Outside,
            };
            let decision = crate::record_object_place::decide(
                steps.iter().all(|step| step.0 == "."),
                storage,
                access,
            );
            crate::record_object_place::record(self.decisions, root, storage, access, decision);
            if decision != crate::record_object_place::Decision::Admit {
                return None;
            }
            record_object = true;
        }
        let mixed_path = steps.iter().any(|step| step.0 == "[]" && steps.len() != 1)
            || steps.iter().any(|step| step.0 == "*" && steps.len() != 1)
            || steps.iter().skip(1).any(|step| step.0 != "->");
        // The existing tracked field-link model names only direct `->`
        // places. A path through embedded records or indexed objects under a
        // never-bound root is admitted as before: every loaded pointer is
        // outside this projection's tracked graph. Card CG10 (rule 10t): on
        // any other root, the common typed place carries the per-hop
        // obligations the chain model checks, or refuses.
        let tracked = if mixed_path && !record_object && !(self.never_bound)(root) {
            Some(
                crate::typed_place::tracked_chain(node, self.fields, self.function, self.decisions)
                    .ok()?,
            )
        } else {
            None
        };
        if let Some(tracked) = &tracked {
            self.plan.chains.push(tracked.chain.clone());
        } else if !record_object && (steps.len() > 1 || first.0 != ".") {
            let members = steps.iter().map(|step| step.1.clone()).collect::<Vec<_>>();
            let links = if mixed_path {
                Vec::new()
            } else if members.len() > 1 {
                members[..members.len() - 1].to_vec()
            } else {
                Vec::new()
            };
            let root_never_bound = (self.never_bound)(root);
            self.plan.chains.push(AccessChain {
                root: root.to_owned(),
                dereferenced_root: first.0 != ".",
                root_never_bound,
                // Card CG10-BOUND-UNTRACKED-ROOTS (rule 11d): a dereferenced
                // root the closure does not prove never bound may still be
                // an untracked borrowed object on every path; the flow decides.
                root_bound_untracked: !root_never_bound
                    && first.0 != "."
                    && self.function.is_some_and(|function| {
                        cursor["referencedDecl"]["id"].as_str().is_some_and(|decl| {
                            crate::bound_untracked::eligible_root(function, root, decl)
                        })
                    }),
                // Card CG12-VOID-VIEW-READBACK (rule 11j): a view is read
                // only over an untracked object; the flow refuses a tracked
                // root (its object's type is the contract's, not the view's).
                root_untracked_only: saw_view,
                links,
                member: members.last()?.clone(),
            });
        } else if saw_view {
            // A view with no chain to carry the untracked-only reading.
            return None;
        }
        if store
            && (matches!(first.0, "*" | "[]")
                || union_frame
                || tracked
                    .as_ref()
                    .is_some_and(|tracked| tracked.through_element))
        {
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

/// Card CG30-DIALECTS-FLAGS-TYPES (rule 11y): route G's canonical type
/// identity of a node, read from the same spelling [`qual`] reads.
pub(crate) fn type_id(node: &Value) -> Option<crate::c_frontend_identity::CTypeId> {
    qual(node).and_then(crate::c_frontend_identity::CTypeId::from_canonical_spelling)
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
    fn unsupported_pointer_store_cast_and_call_refuse() {
        // Card CG13 flip (rule 10w): the union row of this table
        // (`int x = p->value` through `union U *p`) is now a custody-neutral
        // scalar read; see `union_member_reads_are_decided_by_rule_10w`.
        let cases = [
            (
                "struct S { int *next; }; int f(struct S *p, int *q) { p->next = q; return 0; }",
                "p->next = q",
            ),
            // Card CAST-FORMALIZATION (rule 12h), a deliberate flip: `*(int *)p`
            // of an `int *p` is a qualifier conversion, read through; a pun
            // keeps the refusal.
            (
                "int f(long *p) { int x = *(int *)p; return 0; }",
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
    fn mixed_embedded_and_indexed_path_carries_per_hop_obligations() {
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
        // Card CG10 (rule 10t): on a root the closure does not prove never
        // bound, the common typed place carries the per-hop obligations the
        // flow checks: the root alone for the embedded member, and the root
        // plus the direct member place `p->items` for the indexed element.
        let tracked = index
            .statement(&texts, 1, |name| name != "p")
            .expect("tracked-root place");
        assert_eq!(tracked.accesses, 2);
        assert!(tracked.chains.iter().all(|chain| !chain.root_never_bound));
        assert_eq!(
            tracked
                .chains
                .iter()
                .map(|chain| chain.links.clone())
                .collect::<Vec<_>>(),
            [Vec::<String>::new(), vec!["items".to_owned()]]
        );
    }

    #[test]
    fn union_member_reads_are_decided_by_rule_10w() {
        // Card CG13 flip (rule 10w): a typedef-hidden union is still found by
        // its field's owner, and its scalar member is read as a union step.
        // BEFORE: both refused.
        let direct = "union U { int value; }; int f(union U *p) { int x = p->value; return 0; }";
        assert!(plan(direct, "int x = p->value").is_some());
        let union = "typedef union U { int x; int y; } U; int f(U *u) { int x = u->x; return 0; }";
        assert!(plan(union, "int x = u->x").is_some());
        let nested = "typedef union U { int x; int y; } U; typedef struct S { U *u; } S; int f(S *s) { int x = s->u->x; return 0; }";
        assert!(plan(nested, "int x = s->u->x").is_some());
        let accepted = "typedef struct S { int x; } S; int f(S *s) { int x = s->x; return 0; }";
        assert!(plan(accepted, "int x = s->x").is_some());
        // A pointer read through a union the function stores through another
        // member of is refused (punning); unpunned, it is a borrowed alias.
        let pointer = "union P { long n; int *p; }; int f(union P *u) { int *q = u->p; return 0; }";
        assert!(
            plan(pointer, "int *q = u->p")
                .expect("borrow")
                .borrowed_destination
        );
        let punned =
            "union P { long n; int *p; }; int f(union P *u) { u->n = 1; int *q = u->p; return 0; }";
        assert!(plan(punned, "int *q = u->p").is_none());
        // A store through a union that can hold a pointer writes with the
        // refused-write frame; through a custody-free union it has none.
        let framed = plan(
            "union P { long n; int *p; }; int f(union P *u) { u->n = 1; return 0; }",
            "u->n = 1",
        )
        .expect("framed store");
        assert!(framed.scalar.writes_through_pointer);
        let free = plan(
            "union N { long n; double d; }; int f(union N *u) { u->n = 1; return 0; }",
            "u->n = 1",
        )
        .expect("neutral store");
        assert!(!free.scalar.writes_through_pointer);
        // A bit-field store is a scalar store with no frame.
        let bits = plan(
            "struct B { unsigned a : 3; unsigned b : 5; }; int f(struct B *s) { s->a = 1; return 0; }",
            "s->a = 1",
        )
        .expect("unit write");
        assert!(!bits.scalar.writes_through_pointer);
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

#[cfg(test)]
mod lifetime_cache_tests {
    use super::*;
    use qcore_schemas::contract::{
        ResourceContracts, ResourceFunctionContract, ResourceParameterMode, ResourceReturnMode,
    };
    use qcore_schemas::retained_by::RetainingContract;
    #[test]
    fn failures_are_cached_and_dependency_changes_retry() {
        let cache = LifetimeSourceCache::default();
        assert!(
            cache
                .get_or_read("a".into(), || Err("typed source failed".into()))
                .is_err()
        );
        assert!(
            cache
                .get_or_read("a".into(), || panic!(
                    "a failure must not be replaced by fallback"
                ))
                .is_err()
        );
        assert!(
            cache
                .get_or_read("b".into(), || Ok(Default::default()))
                .is_ok()
        );
    }
    #[test]
    fn source_classification_dependencies_include_effective_release_and_retaining_maps() {
        let mut env = ResourceContracts::default();
        let original = lifetime_source_dependencies(&env).unwrap();
        let release = ResourceFunctionContract::new(
            [ResourceParameterMode::Released { nullable: true }],
            ResourceReturnMode::None,
        );
        let borrowed = ResourceFunctionContract::new(
            [ResourceParameterMode::Borrowed],
            ResourceReturnMode::None,
        );
        env.insert_contract("release", release.clone());
        let declared_release = lifetime_source_dependencies(&env).unwrap();
        assert_ne!(original, declared_release);
        env.insert_contract("release", borrowed.clone());
        assert_eq!(original, lifetime_source_dependencies(&env).unwrap());
        env.set_inferred_imports(BTreeMap::from([("imported_release".into(), release)]));
        assert_ne!(original, lifetime_source_dependencies(&env).unwrap());
        env.insert_contract("imported_release", borrowed.clone());
        assert_eq!(
            original,
            lifetime_source_dependencies(&env).unwrap(),
            "declared borrowing masks the imported release"
        );
        env.insert_retaining("retainer", RetainingContract::custody_only(borrowed));
        assert_ne!(original, lifetime_source_dependencies(&env).unwrap());
        let before_text = lifetime_source_dependencies(&env).unwrap();
        env.set_source_assumptions(BTreeMap::from([(
            "new.qv".into(),
            "changed evidence".into(),
        )]));
        assert_ne!(before_text, lifetime_source_dependencies(&env).unwrap());
    }
    #[test]
    fn cached_source_reclassifies_after_declared_imported_and_retaining_changes() {
        use crate::loop_lifetime_source::OperationKind;
        use qcore_schemas::retained_by::{Retention, RetentionHolder};
        let _scope = enter_source(
            "struct obj { int value; }; void primitive(void *, void *); void f(struct obj *p, struct obj *h) { primitive(p, h); }",
            CCompilerDialect::C11,
            None,
        );
        let borrowed = ResourceFunctionContract::new(
            [ResourceParameterMode::Borrowed; 2],
            ResourceReturnMode::None,
        );
        let release = ResourceFunctionContract::new(
            [
                ResourceParameterMode::Released { nullable: true },
                ResourceParameterMode::Borrowed,
            ],
            ResourceReturnMode::None,
        );
        let mut env = ResourceContracts::default();
        assert!(
            current_lifetime_source(&env).unwrap().functions["f"]
                .operations
                .is_empty()
        );
        env.insert_contract("primitive", release.clone());
        assert_eq!(
            current_lifetime_source(&env).unwrap().functions["f"].operations[0].kind,
            OperationKind::Release
        );
        env.insert_contract("primitive", borrowed.clone());
        assert!(
            current_lifetime_source(&env).unwrap().functions["f"]
                .operations
                .is_empty()
        );
        env.set_inferred_imports(BTreeMap::from([("primitive".into(), release)]));
        assert!(
            current_lifetime_source(&env).unwrap().functions["f"]
                .operations
                .is_empty(),
            "ordinary declaration masks an import"
        );
        let mut retained = RetainingContract::custody_only(borrowed.clone());
        retained.retentions.push(Retention {
            retained: 0,
            holder: RetentionHolder::Parameter { index: 1 },
            guard: None,
        });
        env.insert_retaining("primitive", retained);
        let facts = current_lifetime_source(&env).unwrap();
        assert!(
            facts.functions["f"]
                .operations
                .iter()
                .any(|op| op.kind == OperationKind::Register)
        );
        env.insert_retaining("primitive", RetainingContract::custody_only(borrowed));
        assert!(
            !current_lifetime_source(&env).unwrap().functions["f"]
                .operations
                .iter()
                .any(|op| op.kind == OperationKind::Register)
        );
    }
    #[test]
    fn guarded_map_masking_and_withdrawal_invalidate_imported_release_classification() {
        use crate::loop_lifetime_source::OperationKind;
        use qcore_schemas::loop_lifetime::{ConditionBinding, LoopLifetimeContract};
        let _scope = enter_source(
            "struct obj { int value; }; void primitive(void *); void f(struct obj *p) { primitive(p); }",
            CCompilerDialect::C11,
            None,
        );
        let mut env = ResourceContracts::default();
        let released = ResourceFunctionContract::new(
            [ResourceParameterMode::Released { nullable: true }],
            ResourceReturnMode::None,
        );
        env.set_inferred_imports(BTreeMap::from([("primitive".into(), released)]));
        assert_eq!(
            current_lifetime_source(&env).unwrap().functions["f"].operations[0].kind,
            OperationKind::Release
        );
        let mut guarded = RetainingContract::custody_only(ResourceFunctionContract::new(
            [ResourceParameterMode::Borrowed],
            ResourceReturnMode::None,
        ));
        guarded.lifetime = Some(LoopLifetimeContract {
            bindings: vec![ConditionBinding {
                name: "c".into(),
                parameter: 0,
                members: vec!["value".into()],
            }],
            ..Default::default()
        });
        env.insert_retaining("primitive", guarded);
        assert!(env.contract("primitive").is_none());
        assert!(
            current_lifetime_source(&env).unwrap().functions["f"]
                .operations
                .is_empty()
        );
        env.withdraw_declaration("primitive");
        assert!(env.lifetime_contract("primitive").is_none());
        assert!(env.has_guarded_interface("primitive"));
        assert!(
            current_lifetime_source(&env).unwrap().functions["f"]
                .operations
                .is_empty()
        );
        let mut fresh = ResourceContracts::default();
        fresh.set_inferred_imports(env.inferred_imports().clone());
        assert_eq!(
            current_lifetime_source(&fresh).unwrap().functions["f"].operations[0].kind,
            OperationKind::Release
        );
    }
    #[test]
    fn source_configurations_and_independent_replay_get_fresh_cache_scopes() {
        let _outer = enter_source("void f(void) {}", CCompilerDialect::C11, None);
        let original = ANALYSED_SOURCE.with(|cell| {
            let scope = cell.borrow();
            let scope = scope.as_ref().unwrap();
            scope
                .lifetime_source
                .get_or_read("k".into(), || Ok(Default::default()))
                .unwrap();
            (scope.source_sha256.clone(), scope.frontend_sha256.clone())
        });
        for (source, definitions) in [
            ("void g(void) {}", vec![]),
            ("void f(void) {}", vec!["S3_CONFIG=1".into()]),
            ("void f(void) {}", vec![]),
        ] {
            let request = LegacyCertificateRequest {
                include_paths: vec![],
                definitions,
                preprocessor_options: vec![],
                input: Default::default(),
            };
            let _fresh = enter_source(source, CCompilerDialect::C11, Some(&request));
            ANALYSED_SOURCE.with(|cell| {
                let scope = cell.borrow();
                let scope = scope.as_ref().unwrap();
                assert!(scope.lifetime_source.entry.borrow().is_none());
                if source.contains("g(") {
                    assert_ne!(scope.source_sha256, original.0);
                }
                if !request.definitions.is_empty() {
                    assert_ne!(scope.frontend_sha256, original.1);
                }
            });
        }
    }
}

#[cfg(test)]
mod audit_lifetime_tests {
    use super::*;

    #[test]
    fn unit_boundaries_release_ast_ownership_but_preserve_active_readers() {
        release_reuse_caches();
        let source = Arc::new("void subject(void) {}".to_owned());
        let frontend = Arc::new("request".to_owned());
        let key = || UnitKey::new(CCompilerDialect::C11, &frontend, &source);
        let lease = UNIT_ASTS.with(|cache| {
            cache
                .get_or_launch(key(), || {
                    Some(AstIndex::from_ast(
                        serde_json::json!({"kind":"TranslationUnitDecl","inner":[]}),
                        &source,
                    ))
                })
                .unwrap()
        });
        let weak = Rc::downgrade(&lease);
        crate::with_audit_unit_lifetimes(|| {
            assert_eq!(Rc::strong_count(&lease), 1);
            assert!(lease.functions.is_empty());
        });
        assert!(weak.upgrade().is_some());
        drop(lease);
        assert!(weak.upgrade().is_none());

        let weak = crate::with_audit_unit_lifetimes(|| {
            let value = UNIT_LAYOUTS.with(|cache| {
                cache
                    .get_or_launch(key(), || Some("layout".to_owned()))
                    .unwrap()
            });
            Rc::downgrade(&value)
        });
        assert!(weak.upgrade().is_none());
        let mut weak = None;
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            crate::with_audit_unit_lifetimes(|| {
                let value = UNIT_LAYOUTS.with(|cache| {
                    cache
                        .get_or_launch(key(), || Some("layout".to_owned()))
                        .unwrap()
                });
                weak = Some(Rc::downgrade(&value));
                panic!("cancel audit unit");
            });
        }));
        assert!(result.is_err());
        assert!(weak.unwrap().upgrade().is_none());
    }
}
