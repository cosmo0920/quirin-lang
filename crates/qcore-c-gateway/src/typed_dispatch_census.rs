//! Typed field dispatch for the thread-entry census (card
//! **W47-LOOP-OWNER-TYPED-DISPATCH**, gateway rule 10p's amendment).
//!
//! The lexical census ([`crate::thread_entry_census`]) attributes a function
//! stored in a field `F` to every `->F (` call, whatever struct the call goes
//! through. This module reads, from Clang's typed AST of each translation unit
//! of one build, the **record type** each such call and each such store goes
//! through, so the census can key registration and dispatch by
//! `(record, field)` instead of by field name alone.
//!
//! # What is read
//!
//! For every translation unit (a compile command), Clang's JSON AST is
//! streamed (never held whole) one top-level declaration at a time:
//!
//! * a **dispatch**: a call whose callee is, through parentheses, implicit
//!   conversions and a `*`, a member expression `base->F` / `base.F`; its
//!   record is the one declaring the field Clang resolved (`referencedMemberDecl`),
//!   so typedefs are resolved by construction;
//! * a **store**: `base->F = f` / `base.F = f`, and a record initializer
//!   (`{ .F = f }` or positional) whose slot for `F` holds a function `f`;
//! * a **parameter store**: `base->F = p` with `p` a parameter of the enclosing
//!   function (how a registration function stores its callback);
//! * the string literal a listed call passes at a listed argument, and the
//!   string literal a named object's initializer stores in a listed field (the
//!   plugin name an instance is created from and the one each descriptor
//!   declares, for Step B of the census).
//!
//! Each site is keyed by the spelling file and line of its field token (a
//! dispatch) or of the function's name (a store), which is where the lexical
//! census reads it.
//!
//! # Fail-closed
//!
//! A record is **unmatched** — and the census then refuses to attribute the
//! site to any record — when the field belongs to a union or to an anonymous
//! record no typedef names, when an explicit cast stands on the dispatch's
//! object path or around its callee, when the stored function is converted
//! (anything but decay), or when an initializer's slots cannot be matched to
//! the record's fields. A site the typed reading does not see at all keeps
//! the lexical, field-keyed reading. Nothing here is a verdict: these are
//! Clang transport facts, read by the census.
//!
//! Premises stated, not proved (trust boundary 28): the recorded build's
//! compile commands are the program's; C's effective-type rule (an object is
//! not called through a record type other than its own); and Clang's
//! `-ast-dump=json` location de-duplication (a `file`/`line` attribute is
//! printed only when it changes) read in document order.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt;
use std::io::{BufReader, Read};
use std::path::{Component, Path, PathBuf};
use std::process::Stdio;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use serde::de::{DeserializeSeed, IgnoredAny, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};

/// The record a typed site goes through, or why it cannot be matched.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum TypedRecord {
    /// `struct NAME`, or `typedef NAME` for an anonymous record a typedef names.
    Record(String),
    Unmatched(String),
}

/// A registration function's parameter stored in a record field.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ParameterStore {
    pub function: String,
    pub record: String,
    pub field: String,
    pub parameter: String,
}

/// What to record besides dispatches and stores.
#[derive(Clone, Debug, Default)]
pub struct TypedOptions {
    /// `(function, argument index)`: record a string literal passed there.
    pub literal_calls: Vec<(String, usize)>,
    /// `(record, field)`: record `(object, literal)` for each object whose
    /// initializer stores a string literal there.
    pub named_objects: Vec<(String, String)>,
    /// A source read under another name (the previous test, read from a copy
    /// beside the tree it belongs to): its facts are keyed by that name.
    pub aliases: BTreeMap<PathBuf, String>,
}

/// The typed facts of one build (the union over its translation units).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypedFacts {
    /// Translation units read, by path under the source root.
    pub units: BTreeSet<String>,
    /// Translation units Clang could not read, with why.
    pub failures: BTreeMap<String, String>,
    /// `FILE|LINE|FIELD` → the records dispatched through.
    pub dispatches: BTreeMap<String, BTreeSet<TypedRecord>>,
    /// `FILE|LINE|FIELD|FUNCTION` → the records `FUNCTION` is stored in.
    pub stores: BTreeMap<String, BTreeSet<TypedRecord>>,
    pub parameter_stores: BTreeSet<ParameterStore>,
    /// `FILE|LINE|CALLEE` → the literals passed at the listed argument.
    pub literal_calls: BTreeMap<String, BTreeSet<String>>,
    /// `RECORD|FIELD|LITERAL` → the objects initialized with it.
    pub named_objects: BTreeMap<String, BTreeSet<String>>,
}

/// The key of a dispatch site.
#[must_use]
pub fn dispatch_key(file: &str, line: usize, field: &str) -> String {
    format!("{file}|{line}|{field}")
}

/// The key of a store site.
#[must_use]
pub fn store_key(file: &str, line: usize, field: &str, function: &str) -> String {
    format!("{file}|{line}|{field}|{function}")
}

/// The key of a literal call site.
#[must_use]
pub fn literal_call_key(file: &str, line: usize, callee: &str) -> String {
    format!("{file}|{line}|{callee}")
}

/// The key of a named object.
#[must_use]
pub fn named_object_key(record: &str, field: &str, literal: &str) -> String {
    format!("{record}|{field}|{literal}")
}

impl TypedFacts {
    /// Add another unit's facts.
    pub fn merge(&mut self, other: Self) {
        self.units.extend(other.units);
        self.failures.extend(other.failures);
        for (key, records) in other.dispatches {
            self.dispatches.entry(key).or_default().extend(records);
        }
        for (key, records) in other.stores {
            self.stores.entry(key).or_default().extend(records);
        }
        self.parameter_stores.extend(other.parameter_stores);
        for (key, literals) in other.literal_calls {
            self.literal_calls.entry(key).or_default().extend(literals);
        }
        for (key, objects) in other.named_objects {
            self.named_objects.entry(key).or_default().extend(objects);
        }
    }
}

/// One translation unit: its source, the directory its command runs in and
/// the command's arguments after the compiler (the recorded compile command).
#[derive(Clone, Debug)]
pub struct TypedUnit {
    pub file: PathBuf,
    pub directory: PathBuf,
    pub arguments: Vec<String>,
}

/// The typed reader's stack: deep enough for Clang's deepest statement nests.
const PARSE_STACK: usize = 256 << 20;

/// Read every unit with up to `jobs` Clang processes at a time. A unit Clang
/// cannot read is recorded in [`TypedFacts::failures`], never skipped silently.
#[must_use]
pub fn read_units(
    units: &[TypedUnit],
    root: &Path,
    options: &TypedOptions,
    jobs: usize,
) -> TypedFacts {
    let next = AtomicUsize::new(0);
    let collected = Mutex::new(TypedFacts::default());
    std::thread::scope(|scope| {
        for _ in 0..jobs.max(1) {
            scope.spawn(|| {
                loop {
                    let index = next.fetch_add(1, Ordering::SeqCst);
                    let Some(unit) = units.get(index) else {
                        break;
                    };
                    let name = relative(&normalize(&unit.file), root)
                        .unwrap_or_else(|| unit.file.to_string_lossy().into_owned());
                    let facts = match read_unit(unit, root, options) {
                        Ok(mut facts) => {
                            facts.units.insert(name);
                            facts
                        }
                        Err(error) => TypedFacts {
                            failures: BTreeMap::from([(name, error)]),
                            ..TypedFacts::default()
                        },
                    };
                    collected
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .merge(facts);
                }
            });
        }
    });
    collected
        .into_inner()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Read one unit's typed facts.
///
/// # Errors
/// Clang is unavailable, rejects the unit, or prints JSON this reader does
/// not understand.
pub fn read_unit(
    unit: &TypedUnit,
    root: &Path,
    options: &TypedOptions,
) -> Result<TypedFacts, String> {
    let mut command = crate::clang::command()?;
    let mut arguments = unit.arguments.iter();
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "-o" => {
                arguments.next();
            }
            "-c" => {}
            _ => {
                command.arg(argument);
            }
        }
    }
    command
        .args(["-fsyntax-only", "-w", "-Xclang", "-ast-dump=json"])
        .current_dir(&unit.directory)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|error| format!("Clang AST unavailable: {error}"))?;
    let stdout = child.stdout.take().ok_or("Clang stdout unavailable")?;
    let mut stderr = child.stderr.take().ok_or("Clang stderr unavailable")?;
    let (parsed, error_text) = std::thread::scope(|scope| {
        let errors = scope.spawn(move || {
            let mut text = String::new();
            let _ = stderr.read_to_string(&mut text);
            text
        });
        // Clang nests a statement two JSON levels per AST level (a long
        // `else if` chain included), so the parse runs without serde_json's
        // recursion limit, on a stack sized for it.
        let parse = std::thread::Builder::new()
            .stack_size(PARSE_STACK)
            .spawn_scoped(scope, move || {
                let mut reader = Reader::new(root.to_owned(), options);
                let mut deserializer =
                    serde_json::Deserializer::from_reader(BufReader::new(stdout));
                deserializer.disable_recursion_limit();
                UnitSeed(&mut reader)
                    .deserialize(&mut deserializer)
                    .map_err(|error| format!("unsupported Clang AST JSON: {error}"))
                    .map(|()| reader.facts)
            })
            .map_err(|error| format!("cannot start the AST reader: {error}"));
        let parsed = parse.and_then(|handle| {
            handle
                .join()
                .unwrap_or_else(|_| Err("the AST reader panicked".to_owned()))
        });
        (parsed, errors.join().unwrap_or_default())
    });
    let status = child
        .wait()
        .map_err(|error| format!("Clang AST failed: {error}"))?;
    if !status.success() {
        return Err(format!(
            "Clang AST rejected the unit: {}",
            error_text.lines().take(3).collect::<Vec<_>>().join(" / ")
        ));
    }
    parsed
}

/// A path with `.` and `..` components removed lexically.
fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other),
        }
    }
    out
}

fn relative(path: &Path, root: &Path) -> Option<String> {
    path.strip_prefix(normalize(root))
        .ok()
        .map(|relative| relative.to_string_lossy().replace('\\', "/"))
}

// ---------------------------------------------------------------------------
// Streaming the JSON AST.

/// A location: an index into [`Cursor::files`] and a line.
type Place = (usize, usize);

/// Clang prints a location's `file` and `line` only when they differ from the
/// location it printed last, so they are tracked in document order.
#[derive(Default)]
struct Cursor {
    files: Vec<String>,
    index: HashMap<String, usize>,
    file: Option<usize>,
    line: usize,
}

impl Cursor {
    fn set_file(&mut self, name: String) {
        let next = self.files.len();
        let id = *self.index.entry(name.clone()).or_insert(next);
        if id == next {
            self.files.push(name);
        }
        self.file = Some(id);
    }
}

#[derive(Default)]
struct Node {
    kind: String,
    id: Option<String>,
    name: Option<String>,
    tag_used: Option<String>,
    qual_type: Option<String>,
    desugared: Option<String>,
    referenced_member: Option<String>,
    referenced: Option<BareDecl>,
    union_field: Option<BareDecl>,
    decls: Vec<BareDecl>,
    opcode: Option<String>,
    cast_kind: Option<String>,
    value: Option<String>,
    is_bitfield: bool,
    complete: bool,
    begin: Option<Place>,
    end: Option<Place>,
    inner: Vec<Node>,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct BareDecl {
    #[serde(default)]
    id: String,
    #[serde(default)]
    kind: String,
    #[serde(default)]
    name: String,
}

/// A bare declaration reference, when the value is one (Clang uses the same
/// keys elsewhere for other shapes).
fn bare_decl(value: serde_json::Value) -> Option<BareDecl> {
    value
        .is_object()
        .then(|| serde_json::from_value(value).ok())
        .flatten()
}

#[derive(Default, Deserialize)]
struct TypeText {
    #[serde(default, rename = "qualType")]
    qual_type: Option<String>,
    #[serde(default, rename = "desugaredQualType")]
    desugared: Option<String>,
}

struct LocSeed<'a>(&'a mut Cursor);

impl<'de> DeserializeSeed<'de> for LocSeed<'_> {
    type Value = Option<Place>;
    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_map(self)
    }
}

impl<'de> Visitor<'de> for LocSeed<'_> {
    type Value = Option<Place>;
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a Clang source location")
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut spelling = None;
        let mut bare = false;
        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "spellingLoc" => spelling = map.next_value_seed(LocSeed(&mut *self.0))?,
                "expansionLoc" => {
                    map.next_value_seed(LocSeed(&mut *self.0))?;
                }
                "file" => {
                    let file = map.next_value::<String>()?;
                    self.0.set_file(file);
                }
                "line" => self.0.line = map.next_value::<usize>()?,
                "offset" => {
                    bare = true;
                    map.next_value::<IgnoredAny>()?;
                }
                _ => {
                    map.next_value::<IgnoredAny>()?;
                }
            }
        }
        if spelling.is_some() {
            return Ok(spelling);
        }
        Ok(if bare {
            self.0.file.map(|file| (file, self.0.line))
        } else {
            None
        })
    }
}

struct RangeSeed<'a>(&'a mut Cursor);

impl<'de> DeserializeSeed<'de> for RangeSeed<'_> {
    type Value = (Option<Place>, Option<Place>);
    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_map(self)
    }
}

impl<'de> Visitor<'de> for RangeSeed<'_> {
    type Value = (Option<Place>, Option<Place>);
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a Clang source range")
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let (mut begin, mut end) = (None, None);
        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "begin" => begin = map.next_value_seed(LocSeed(&mut *self.0))?,
                "end" => end = map.next_value_seed(LocSeed(&mut *self.0))?,
                _ => {
                    map.next_value_seed(SkipSeed(&mut *self.0))?;
                }
            }
        }
        Ok((begin, end))
    }
}

/// Any value this reader does not keep: its locations still advance the cursor.
struct SkipSeed<'a>(&'a mut Cursor);

impl<'de> DeserializeSeed<'de> for SkipSeed<'_> {
    type Value = ();
    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for SkipSeed<'_> {
    type Value = ();
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("any JSON value")
    }
    fn visit_bool<E>(self, _: bool) -> Result<(), E> {
        Ok(())
    }
    fn visit_i64<E>(self, _: i64) -> Result<(), E> {
        Ok(())
    }
    fn visit_u64<E>(self, _: u64) -> Result<(), E> {
        Ok(())
    }
    fn visit_f64<E>(self, _: f64) -> Result<(), E> {
        Ok(())
    }
    fn visit_str<E>(self, _: &str) -> Result<(), E> {
        Ok(())
    }
    fn visit_unit<E>(self) -> Result<(), E> {
        Ok(())
    }
    fn visit_none<E>(self) -> Result<(), E> {
        Ok(())
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<(), A::Error> {
        while seq.next_element_seed(SkipSeed(&mut *self.0))?.is_some() {}
        Ok(())
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<(), A::Error> {
        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "loc" | "begin" | "end" | "spellingLoc" | "expansionLoc" => {
                    map.next_value_seed(LocSeed(&mut *self.0))?;
                }
                "range" => {
                    map.next_value_seed(RangeSeed(&mut *self.0))?;
                }
                "includedFrom" => {
                    map.next_value::<IgnoredAny>()?;
                }
                _ => map.next_value_seed(SkipSeed(&mut *self.0))?,
            }
        }
        Ok(())
    }
}

struct NodeSeed<'a>(&'a mut Cursor);

impl<'de> DeserializeSeed<'de> for NodeSeed<'_> {
    type Value = Node;
    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_map(self)
    }
}

impl<'de> Visitor<'de> for NodeSeed<'_> {
    type Value = Node;
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a Clang AST node")
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Node, A::Error> {
        let mut node = Node::default();
        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "kind" => node.kind = map.next_value()?,
                "id" => node.id = Some(map.next_value()?),
                "name" => node.name = Some(map.next_value()?),
                "tagUsed" => node.tag_used = Some(map.next_value()?),
                "opcode" => node.opcode = Some(map.next_value()?),
                "castKind" => node.cast_kind = Some(map.next_value()?),
                "referencedMemberDecl" => node.referenced_member = Some(map.next_value()?),
                "referencedDecl" => node.referenced = bare_decl(map.next_value()?),
                "field" => node.union_field = bare_decl(map.next_value()?),
                "decl" | "ownedTagDecl" => node.decls.extend(bare_decl(map.next_value()?)),
                "isBitfield" => node.is_bitfield = map.next_value()?,
                "completeDefinition" => node.complete = map.next_value()?,
                "value" => {
                    node.value = map
                        .next_value::<serde_json::Value>()?
                        .as_str()
                        .map(str::to_owned);
                }
                "type" => {
                    let text = map.next_value::<TypeText>()?;
                    node.qual_type = text.qual_type;
                    node.desugared = text.desugared;
                }
                "loc" => {
                    map.next_value_seed(LocSeed(&mut *self.0))?;
                }
                "range" => {
                    let (begin, end) = map.next_value_seed(RangeSeed(&mut *self.0))?;
                    node.begin = begin;
                    node.end = end;
                }
                "inner" | "array_filler" => {
                    let children = map.next_value_seed(NodesSeed(&mut *self.0))?;
                    node.inner.extend(children);
                }
                _ => map.next_value_seed(SkipSeed(&mut *self.0))?,
            }
        }
        Ok(node)
    }
}

struct NodesSeed<'a>(&'a mut Cursor);

impl<'de> DeserializeSeed<'de> for NodesSeed<'_> {
    type Value = Vec<Node>;
    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_seq(self)
    }
}

impl<'de> Visitor<'de> for NodesSeed<'_> {
    type Value = Vec<Node>;
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a list of Clang AST nodes")
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Vec<Node>, A::Error> {
        let mut nodes = Vec::new();
        while let Some(node) = seq.next_element_seed(NodeSeed(&mut *self.0))? {
            nodes.push(node);
        }
        Ok(nodes)
    }
}

/// The translation unit: each top-level declaration is read, analysed and
/// dropped in turn.
struct UnitSeed<'a, 'o>(&'a mut Reader<'o>);

impl<'de> DeserializeSeed<'de> for UnitSeed<'_, '_> {
    type Value = ();
    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
        deserializer.deserialize_map(self)
    }
}

impl<'de> Visitor<'de> for UnitSeed<'_, '_> {
    type Value = ();
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a Clang translation unit")
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<(), A::Error> {
        while let Some(key) = map.next_key::<String>()? {
            if key == "inner" {
                map.next_value_seed(TopSeed(&mut *self.0))?;
            } else {
                map.next_value_seed(SkipSeed(&mut self.0.cursor))?;
            }
        }
        Ok(())
    }
}

struct TopSeed<'a, 'o>(&'a mut Reader<'o>);

impl<'de> DeserializeSeed<'de> for TopSeed<'_, '_> {
    type Value = ();
    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
        deserializer.deserialize_seq(self)
    }
}

impl<'de> Visitor<'de> for TopSeed<'_, '_> {
    type Value = ();
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("top-level declarations")
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<(), A::Error> {
        while let Some(node) = seq.next_element_seed(NodeSeed(&mut self.0.cursor))? {
            self.0.declaration(&node);
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Reading the facts.

struct RecordInfo {
    key: Option<String>,
    union: bool,
    /// The fields an initializer's slots stand for, in order (unnamed
    /// bit-fields take no slot).
    slots: Vec<String>,
}

struct Reader<'o> {
    root: PathBuf,
    options: &'o TypedOptions,
    cursor: Cursor,
    records: HashMap<String, RecordInfo>,
    field_owner: HashMap<String, String>,
    by_key: HashMap<String, String>,
    typedefs: HashMap<String, String>,
    facts: TypedFacts,
}

/// A value stored in a field or an initializer slot.
enum Stored {
    Function(String, Option<Place>, bool),
    Parameter(String, bool),
    Other,
}

impl<'o> Reader<'o> {
    fn new(root: PathBuf, options: &'o TypedOptions) -> Self {
        Self {
            root,
            options,
            cursor: Cursor::default(),
            records: HashMap::new(),
            field_owner: HashMap::new(),
            by_key: HashMap::new(),
            typedefs: HashMap::new(),
            facts: TypedFacts::default(),
        }
    }

    fn place(&self, place: Option<Place>) -> Option<(String, usize)> {
        let (file, line) = place?;
        let name = normalize(Path::new(self.cursor.files.get(file)?));
        if let Some(alias) = self.options.aliases.get(&name) {
            return Some((alias.clone(), line));
        }
        Some((relative(&name, &self.root)?, line))
    }

    fn declaration(&mut self, node: &Node) {
        let function = (node.kind == "FunctionDecl")
            .then(|| node.name.clone())
            .flatten();
        self.walk(node, function.as_deref(), None);
    }

    fn walk(&mut self, node: &Node, function: Option<&str>, object: Option<&str>) {
        match node.kind.as_str() {
            "RecordDecl" => self.record(node),
            "TypedefDecl" => self.typedef(node),
            "CallExpr" => self.call(node),
            "BinaryOperator" if node.opcode.as_deref() == Some("=") => {
                self.assignment(node, function);
            }
            "InitListExpr" => self.initializer(node, object),
            _ => {}
        }
        let object = if node.kind == "VarDecl" {
            node.name.as_deref()
        } else {
            object
        };
        for child in &node.inner {
            self.walk(child, function, object);
        }
    }

    fn record(&mut self, node: &Node) {
        let Some(id) = node.id.clone() else {
            return;
        };
        if !node.complete && self.records.contains_key(&id) {
            return;
        }
        let tag = node.tag_used.clone().unwrap_or_default();
        let key = node
            .name
            .as_ref()
            .filter(|name| !name.is_empty())
            .map(|name| format!("{tag} {name}"));
        let mut slots = Vec::new();
        for field in node.inner.iter().filter(|child| child.kind == "FieldDecl") {
            let name = field.name.clone().unwrap_or_default();
            if let Some(field_id) = &field.id {
                self.field_owner.insert(field_id.clone(), id.clone());
            }
            if !(name.is_empty() && field.is_bitfield) {
                slots.push(name);
            }
        }
        if let Some(key) = &key
            && node.complete
        {
            self.by_key.insert(key.clone(), id.clone());
        }
        self.records.insert(
            id,
            RecordInfo {
                key,
                union: tag == "union",
                slots,
            },
        );
    }

    fn typedef(&mut self, node: &Node) {
        let Some(name) = node.name.clone() else {
            return;
        };
        let mut stack: Vec<&Node> = node.inner.iter().collect();
        while let Some(child) = stack.pop() {
            if let Some(decl) = child.decls.iter().find(|d| d.kind == "RecordDecl") {
                if let Some(record) = self.records.get_mut(&decl.id)
                    && record.key.is_none()
                {
                    record.key = Some(format!("typedef {name}"));
                }
                self.typedefs.insert(name, decl.id.clone());
                return;
            }
            stack.extend(child.inner.iter());
        }
    }

    /// The record key of a field Clang resolved, or why it is unmatched.
    fn field_record(&self, field: Option<&String>) -> TypedRecord {
        let Some(record) = field
            .and_then(|field| self.field_owner.get(field))
            .and_then(|record| self.records.get(record))
        else {
            return TypedRecord::Unmatched("the field's record is not read".to_owned());
        };
        if record.union {
            return TypedRecord::Unmatched("a union field".to_owned());
        }
        record.key.clone().map_or_else(
            || TypedRecord::Unmatched("an anonymous record no typedef names".to_owned()),
            TypedRecord::Record,
        )
    }

    fn call(&mut self, node: &Node) {
        let Some(callee) = node.inner.first() else {
            return;
        };
        let mut cast = false;
        let mut current = callee;
        loop {
            match current.kind.as_str() {
                "ParenExpr" | "ImplicitCastExpr" => {}
                "UnaryOperator" if current.opcode.as_deref() == Some("*") => {}
                "CStyleCastExpr" => cast = true,
                _ => break,
            }
            let Some(next) = current.inner.first() else {
                break;
            };
            current = next;
        }
        if current.kind == "MemberExpr" {
            let Some((file, line)) = self.place(current.end) else {
                return;
            };
            let field = current.name.clone().unwrap_or_default();
            let record = if cast {
                TypedRecord::Unmatched("an explicit cast of the called value".to_owned())
            } else if current.inner.first().is_some_and(cast_on_object_path) {
                TypedRecord::Unmatched("an explicit cast on the dispatch's object path".to_owned())
            } else {
                self.field_record(current.referenced_member.as_ref())
            };
            self.facts
                .dispatches
                .entry(dispatch_key(&file, line, &field))
                .or_default()
                .insert(record);
        } else if current.kind == "DeclRefExpr"
            && let Some(decl) = &current.referenced
            && decl.kind == "FunctionDecl"
        {
            for (function, argument) in &self.options.literal_calls {
                if *function != decl.name {
                    continue;
                }
                let Some(literal) = node.inner.get(argument + 1).and_then(string_literal) else {
                    continue;
                };
                if let Some((file, line)) = self.place(current.begin) {
                    self.facts
                        .literal_calls
                        .entry(literal_call_key(&file, line, function))
                        .or_default()
                        .insert(literal);
                }
            }
        }
    }

    fn assignment(&mut self, node: &Node, function: Option<&str>) {
        let [target, value] = node.inner.as_slice() else {
            return;
        };
        let target = strip_parens(target);
        if target.kind != "MemberExpr" {
            return;
        }
        let field = target.name.clone().unwrap_or_default();
        let record = self.field_record(target.referenced_member.as_ref());
        self.stored(&field, &record, value, function);
    }

    fn stored(&mut self, field: &str, record: &TypedRecord, value: &Node, function: Option<&str>) {
        match stored_value(value) {
            Stored::Function(name, place, converted) => {
                let Some((file, line)) = self.place(place) else {
                    return;
                };
                let record = if converted {
                    TypedRecord::Unmatched("the stored function is converted".to_owned())
                } else {
                    record.clone()
                };
                self.facts
                    .stores
                    .entry(store_key(&file, line, field, &name))
                    .or_default()
                    .insert(record);
            }
            Stored::Parameter(parameter, false) => {
                if let (Some(function), TypedRecord::Record(record)) = (function, record) {
                    self.facts.parameter_stores.insert(ParameterStore {
                        function: function.to_owned(),
                        record: record.clone(),
                        field: field.to_owned(),
                        parameter,
                    });
                }
            }
            Stored::Parameter(_, true) | Stored::Other => {}
        }
    }

    /// The record an initializer list stands for, when it is a record.
    fn initializer_record(&self, node: &Node) -> Option<Result<&RecordInfo, String>> {
        let qual = node.qual_type.as_deref()?;
        if qual.ends_with(']') {
            return None;
        }
        let text = node.desugared.as_deref().unwrap_or(qual);
        let text = text
            .trim_start_matches("const ")
            .trim_start_matches("volatile ");
        let named = |key: &str| {
            self.by_key
                .get(key)
                .and_then(|id| self.records.get(id))
                .ok_or_else(|| format!("the record `{key}` is not read"))
        };
        if (text.starts_with("struct ") || text.starts_with("union "))
            && !text.contains('(')
            && !text.contains('*')
        {
            return Some(named(text));
        }
        let alias = qual
            .trim_start_matches("const ")
            .trim_start_matches("volatile ");
        if let Some(record) = self.typedefs.get(alias).and_then(|id| self.records.get(id)) {
            return Some(Ok(record));
        }
        if text.starts_with("struct ") || text.starts_with("union ") {
            return Some(Err(format!("the record type `{text}` is not matched")));
        }
        None
    }

    fn initializer(&mut self, node: &Node, object: Option<&str>) {
        let Some(record) = self.initializer_record(node) else {
            return;
        };
        let (key, pairs): (TypedRecord, Vec<(String, &Node)>) = match record {
            Err(why) => (
                TypedRecord::Unmatched(why),
                node.inner
                    .iter()
                    .map(|init| (String::new(), init))
                    .collect(),
            ),
            Ok(record) if record.union => {
                let field = node
                    .union_field
                    .as_ref()
                    .map(|field| field.name.clone())
                    .unwrap_or_default();
                (
                    TypedRecord::Unmatched("a union field".to_owned()),
                    node.inner
                        .iter()
                        .map(|init| (field.clone(), init))
                        .collect(),
                )
            }
            Ok(record) if node.inner.len() > record.slots.len() => (
                TypedRecord::Unmatched(
                    "an initializer whose slots do not match the record's fields".to_owned(),
                ),
                node.inner
                    .iter()
                    .map(|init| (String::new(), init))
                    .collect(),
            ),
            Ok(record) => match &record.key {
                Some(key) => (
                    TypedRecord::Record(key.clone()),
                    record
                        .slots
                        .iter()
                        .cloned()
                        .zip(node.inner.iter())
                        .collect(),
                ),
                None => (
                    TypedRecord::Unmatched("an anonymous record no typedef names".to_owned()),
                    record
                        .slots
                        .iter()
                        .cloned()
                        .zip(node.inner.iter())
                        .collect(),
                ),
            },
        };
        for (field, init) in pairs {
            if let (Some(object), TypedRecord::Record(record)) = (object, &key)
                && self
                    .options
                    .named_objects
                    .iter()
                    .any(|(r, f)| r == record && *f == field)
                && let Some(literal) = string_literal(init)
            {
                self.facts
                    .named_objects
                    .entry(named_object_key(record, &field, &literal))
                    .or_default()
                    .insert(object.to_owned());
            }
            if let Stored::Function(..) = stored_value(init) {
                // An unmatched initializer's slot names no field the census
                // could key; the census reads the field from the source.
                if field.is_empty() {
                    if let Stored::Function(name, place, _) = stored_value(init)
                        && let Some((file, line)) = self.place(place)
                    {
                        self.facts
                            .stores
                            .entry(store_key(&file, line, "", &name))
                            .or_default()
                            .insert(key.clone());
                    }
                } else {
                    self.stored(&field, &key, init, None);
                }
            }
        }
    }
}

fn strip_parens(mut node: &Node) -> &Node {
    while node.kind == "ParenExpr"
        && let Some(inner) = node.inner.first()
    {
        node = inner;
    }
    node
}

/// Whether an explicit cast stands on the object path of a member access:
/// through member bases, parentheses, implicit conversions, `*`, `&` and array
/// bases (not into a call's arguments).
fn cast_on_object_path(node: &Node) -> bool {
    let mut current = node;
    loop {
        match current.kind.as_str() {
            "CStyleCastExpr" => return true,
            "ParenExpr" | "ImplicitCastExpr" | "MemberExpr" | "ArraySubscriptExpr" => {}
            "UnaryOperator" if matches!(current.opcode.as_deref(), Some("*" | "&")) => {}
            _ => return false,
        }
        let Some(next) = current.inner.first() else {
            return false;
        };
        current = next;
    }
}

fn stored_value(node: &Node) -> Stored {
    let mut converted = false;
    let mut current = node;
    loop {
        match current.kind.as_str() {
            "ParenExpr" => {}
            "ImplicitCastExpr" => {
                if !matches!(
                    current.cast_kind.as_deref(),
                    Some("FunctionToPointerDecay" | "LValueToRValue" | "NoOp")
                ) {
                    converted = true;
                }
            }
            "CStyleCastExpr" => converted = true,
            "DeclRefExpr" => {
                let Some(decl) = &current.referenced else {
                    return Stored::Other;
                };
                return match decl.kind.as_str() {
                    "FunctionDecl" => Stored::Function(decl.name.clone(), current.begin, converted),
                    "ParmVarDecl" => Stored::Parameter(decl.name.clone(), converted),
                    _ => Stored::Other,
                };
            }
            _ => return Stored::Other,
        }
        let Some(next) = current.inner.first() else {
            return Stored::Other;
        };
        current = next;
    }
}

fn string_literal(node: &Node) -> Option<String> {
    let mut current = node;
    loop {
        match current.kind.as_str() {
            "ParenExpr" | "ImplicitCastExpr" | "CStyleCastExpr" => {}
            "StringLiteral" => {
                let value = current.value.as_deref()?;
                return value
                    .strip_prefix('"')
                    .and_then(|v| v.strip_suffix('"'))
                    .map(str::to_owned);
            }
            _ => return None,
        }
        current = current.inner.first()?;
    }
}
