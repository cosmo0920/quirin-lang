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
    /// `(record, field)` (card W47-LOOP-OWNER-INSTANCE-FLOW): record every
    /// member access of that field with how it is used (a read, a write, an
    /// address taken, passed to a call, or read through), and every access of
    /// a listed field name whose record is not matched.
    pub member_fields: Vec<(String, String)>,
    /// Card W47-SERVICE-PROPERTY-KEYS: calls whose arguments, from a listed
    /// index on, are recorded as [`argument_descriptor`]s (a variadic
    /// setter's key/value tail, a keyed branch's comparison).
    pub argument_calls: Vec<ArgumentCall>,
    /// Card W47-SERVICE-PROPERTY-KEYS: file-scope arrays of records whose
    /// initializer is recorded element by element (a property key table).
    pub key_tables: Vec<String>,
    /// Card W47-PREMISES-INSTANCE: records (`struct NAME`) whose non-member
    /// uses are recorded ([`TypedFacts::record_views`]).
    pub record_views: Vec<String>,
}

/// A call whose arguments the typed reading records (card
/// W47-SERVICE-PROPERTY-KEYS).
#[derive(Clone, Debug)]
pub struct ArgumentCall {
    pub callee: String,
    /// The first argument recorded.
    pub first: usize,
    /// Only calls made inside this function, when given.
    pub caller: Option<String>,
}

/// How a member access is used, as [`TypedFacts::member_accesses`] records it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Use<'a> {
    /// Its value is read.
    Value,
    /// It is the destination of an assignment, a compound assignment or an
    /// increment.
    Write,
    /// Its address is taken and the pointer used other than below.
    Address,
    /// Its address is taken only to read through it (`(&x->f)->next`) or to
    /// compare it.
    AddressRead,
    /// An argument of the named call (`?` for an indirect call): its value
    /// is read, and an address taken there is passed to the call.
    Argument(&'a str),
    /// Its address is passed to the named call.
    AddressArgument(&'a str),
    /// The object of an arrow access: its pointee is read through.
    ArrowBase,
    /// An operand of a comparison.
    Compare,
    /// The called value of a call (a dispatch, recorded as such).
    Callee,
}

impl Use<'_> {
    fn label(self) -> String {
        match self {
            Use::Value | Use::ArrowBase | Use::Compare | Use::Callee | Use::Argument(_) => {
                "read".to_owned()
            }
            Use::Write => "write".to_owned(),
            Use::Address => "address".to_owned(),
            Use::AddressRead => "address-read".to_owned(),
            Use::AddressArgument(callee) => format!("arg:{callee}"),
        }
    }
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
    /// Card W47-LOOP-OWNER-INSTANCE-FLOW: `RECORD|FIELD` (or `?|FIELD` when
    /// the record is not matched) → `FILE|LINE|FUNCTION|USE` for each access
    /// of a listed member ([`TypedOptions::member_fields`]).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub member_accesses: BTreeMap<String, BTreeSet<String>>,
    /// The `RECORD|FIELD` pairs the reading recorded accesses of: an absent
    /// access means no access only for a listed pair.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub member_fields_read: BTreeSet<String>,
    /// `FILE|NAME` of every function definition read.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub functions: BTreeSet<String>,
    /// `FILE|LINE|FIELD` → the records of a function-pointer member read as
    /// a value (neither called nor tested), where the census reads a field
    /// read as a value.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub field_values: BTreeMap<String, BTreeSet<TypedRecord>>,
    /// `FILE|FUNCTION|LINE` of each call whose callee is neither a function
    /// named directly nor a member (a dispatch): a call through a function
    /// pointer the census does not model.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub indirect_calls: BTreeSet<String>,
    /// Card W47-SERVICE-PROPERTY-KEYS: `FILE|LINE|CALLEE` → each argument
    /// list a listed call passes there, from its listed index on, as
    /// [`argument_descriptor`]s.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub argument_lists: BTreeMap<String, BTreeSet<Vec<String>>>,
    /// Card W47-SERVICE-PROPERTY-KEYS: `FILE|NAME` of a listed key table →
    /// each reading of its initializer: one descriptor list per element.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub key_tables: BTreeMap<String, BTreeSet<Vec<Vec<String>>>>,
    /// Card W47-PREMISES-INSTANCE: `RECORD` → `FILE|LINE|FUNCTION|KIND` for
    /// each use of a listed record that is not a member access: `assign` (a
    /// whole-object assignment), `init` (a copy-initialised object),
    /// `param` (a by-value parameter), `init-list` (an initializer list of
    /// the record), and `conv:DEST|CONTEXT` (a pointer to the record
    /// converted to `DEST`, as the argument of the named call, `arg:CALLEE`,
    /// or `value`).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub record_views: BTreeMap<String, BTreeSet<String>>,
    /// The records whose non-member uses were recorded.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub record_views_read: BTreeSet<String>,
}

/// The key of a member's accesses.
#[must_use]
pub fn member_key(record: &str, field: &str) -> String {
    format!("{record}|{field}")
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
        for (key, sites) in other.member_accesses {
            self.member_accesses.entry(key).or_default().extend(sites);
        }
        self.member_fields_read.extend(other.member_fields_read);
        self.functions.extend(other.functions);
        for (key, records) in other.field_values {
            self.field_values.entry(key).or_default().extend(records);
        }
        self.indirect_calls.extend(other.indirect_calls);
        for (key, lists) in other.argument_lists {
            self.argument_lists.entry(key).or_default().extend(lists);
        }
        for (key, readings) in other.key_tables {
            self.key_tables.entry(key).or_default().extend(readings);
        }
        for (key, sites) in other.record_views {
            self.record_views.entry(key).or_default().extend(sites);
        }
        self.record_views_read.extend(other.record_views_read);
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
    is_arrow: bool,
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
                "isArrow" => node.is_arrow = map.next_value()?,
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
        let facts = TypedFacts {
            member_fields_read: options
                .member_fields
                .iter()
                .map(|(record, field)| member_key(record, field))
                .collect(),
            record_views_read: options.record_views.iter().cloned().collect(),
            ..TypedFacts::default()
        };
        Self {
            root,
            options,
            cursor: Cursor::default(),
            records: HashMap::new(),
            field_owner: HashMap::new(),
            by_key: HashMap::new(),
            typedefs: HashMap::new(),
            facts,
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
        if let Some(name) = &function
            && node.inner.iter().any(|child| child.kind == "CompoundStmt")
            && let Some((file, _)) = self.place(node.begin)
        {
            self.facts.functions.insert(format!("{file}|{name}"));
        }
        if function.is_some() {
            self.accesses(node, function.as_deref(), Use::Value);
        }
    }

    /// Card W47-LOOP-OWNER-INSTANCE-FLOW: record each listed member access
    /// with its use, and each function-pointer member read as a value.
    fn accesses<'n>(&mut self, node: &'n Node, function: Option<&str>, role: Use<'n>) {
        if !self.options.record_views.is_empty() {
            self.record_view(node, function, role);
        }
        let count = node.inner.len();
        let children = |role: Use<'n>| vec![role; count];
        let roles: Vec<Use<'n>> = match node.kind.as_str() {
            "MemberExpr" => {
                self.member_access(node, function, role);
                if node.is_arrow {
                    children(Use::ArrowBase)
                } else {
                    children(match role {
                        Use::Write => Use::Write,
                        Use::Address | Use::AddressArgument(_) => Use::Address,
                        Use::AddressRead | Use::ArrowBase | Use::Compare => Use::AddressRead,
                        Use::Value | Use::Callee | Use::Argument(_) => Use::Value,
                    })
                }
            }
            "ParenExpr" | "ImplicitCastExpr" | "CStyleCastExpr" => children(role),
            "UnaryOperator" => match node.opcode.as_deref() {
                Some("&") => children(match role {
                    Use::Argument(callee) => Use::AddressArgument(callee),
                    Use::ArrowBase | Use::Compare | Use::AddressRead => Use::AddressRead,
                    _ => Use::Address,
                }),
                Some("++" | "--") => children(Use::Write),
                _ => children(Use::Value),
            },
            "BinaryOperator" => match node.opcode.as_deref() {
                Some("=") => {
                    let mut roles = children(Use::Value);
                    if let Some(first) = roles.first_mut() {
                        *first = Use::Write;
                    }
                    roles
                }
                Some("==" | "!=" | "<" | ">" | "<=" | ">=") => children(Use::Compare),
                _ => children(Use::Value),
            },
            "CompoundAssignOperator" => {
                let mut roles = children(Use::Value);
                if let Some(first) = roles.first_mut() {
                    *first = Use::Write;
                }
                roles
            }
            "CallExpr" => {
                let callee = node.inner.first().and_then(called_function);
                if let Some(first) = node.inner.first()
                    && callee.is_none()
                    && !calls_a_member(first)
                    && let Some((file, line)) = self.place(node.begin)
                {
                    self.facts
                        .indirect_calls
                        .insert(format!("{file}|{}|{line}", function.unwrap_or("")));
                }
                let mut roles = children(Use::Argument(callee.unwrap_or("?")));
                if let Some(first) = roles.first_mut() {
                    *first = Use::Callee;
                }
                roles
            }
            _ => children(Use::Value),
        };
        for (child, role) in node.inner.iter().zip(roles) {
            self.accesses(child, function, role);
        }
    }

    fn member_access(&mut self, node: &Node, function: Option<&str>, role: Use<'_>) {
        let field = node.name.clone().unwrap_or_default();
        let Some((file, line)) = self.place(node.end) else {
            return;
        };
        let record = self.field_record(node.referenced_member.as_ref());
        if role == Use::Value
            && node
                .qual_type
                .as_deref()
                .is_some_and(|qual| qual.contains("(*)"))
        {
            self.facts
                .field_values
                .entry(format!("{file}|{line}|{field}"))
                .or_default()
                .insert(record.clone());
        }
        if !self.options.member_fields.iter().any(|(_, f)| *f == field) {
            return;
        }
        let key = match &record {
            TypedRecord::Record(record)
                if self
                    .options
                    .member_fields
                    .iter()
                    .any(|(r, f)| r == record && *f == field) =>
            {
                member_key(record, &field)
            }
            TypedRecord::Record(_) => return,
            TypedRecord::Unmatched(_) => member_key("?", &field),
        };
        let use_label = role.label();
        self.facts
            .member_accesses
            .entry(key)
            .or_default()
            .insert(format!(
                "{file}|{line}|{}|{use_label}",
                function.unwrap_or("")
            ));
    }

    fn walk(&mut self, node: &Node, function: Option<&str>, object: Option<&str>) {
        match node.kind.as_str() {
            "RecordDecl" => self.record(node),
            "TypedefDecl" => self.typedef(node),
            "CallExpr" => self.call(node, function),
            "BinaryOperator" if node.opcode.as_deref() == Some("=") => {
                self.assignment(node, function);
            }
            "InitListExpr" => {
                // Card W47-PREMISES-INSTANCE: an initializer list of a listed
                // record, at any scope.
                if let Some(record) = self.viewed_record(node, false) {
                    self.view(&record, node.begin, function, "init-list".to_owned());
                }
                self.initializer(node, object);
            }
            "VarDecl" if function.is_none() => self.key_table(node),
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

    /// Card W47-SERVICE-PROPERTY-KEYS: a listed file-scope array's
    /// initializer, one descriptor list per element (an element that is not
    /// itself an initializer list is recorded as `["?"]`, and an array filler
    /// as `["filler"]`, so the census refuses the table).
    fn key_table(&mut self, node: &Node) {
        let Some(name) = node.name.as_deref() else {
            return;
        };
        if !self.options.key_tables.iter().any(|table| table == name) {
            return;
        }
        let Some(list) = node.inner.iter().find(|child| child.kind == "InitListExpr") else {
            return;
        };
        let Some((file, _)) = self.place(node.begin) else {
            return;
        };
        let elements = list
            .inner
            .iter()
            .map(|element| {
                if element.kind == "InitListExpr" {
                    element.inner.iter().map(argument_descriptor).collect()
                } else if element.kind == "ImplicitValueInitExpr" {
                    vec!["filler".to_owned()]
                } else {
                    vec!["?".to_owned()]
                }
            })
            .collect::<Vec<Vec<String>>>();
        self.facts
            .key_tables
            .entry(format!("{file}|{name}"))
            .or_default()
            .insert(elements);
    }

    fn call(&mut self, node: &Node, function: Option<&str>) {
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
            // Card W47-SERVICE-PROPERTY-KEYS.
            for listed in &self.options.argument_calls {
                if listed.callee != decl.name
                    || listed
                        .caller
                        .as_deref()
                        .is_some_and(|caller| Some(caller) != function)
                {
                    continue;
                }
                let arguments = node
                    .inner
                    .iter()
                    .skip(1 + listed.first)
                    .map(argument_descriptor)
                    .collect::<Vec<_>>();
                if let Some((file, line)) = self.place(current.begin) {
                    self.facts
                        .argument_lists
                        .entry(literal_call_key(&file, line, &listed.callee))
                        .or_default()
                        .insert(arguments);
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
        // An anonymous record typedef may desugar to `struct ALIAS` on
        // Windows without declaring that tag. Its declaration still identifies
        // the record; resolve that positive alias before looking up a tag.
        let alias = qual
            .trim_start_matches("const ")
            .trim_start_matches("volatile ");
        if let Some(record) = self.typedefs.get(alias).and_then(|id| self.records.get(id)) {
            return Some(Ok(record));
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

/// Whether a call's callee is, through parentheses, conversions and `*`, a
/// member (a dispatch the census reads, typed or not).
fn calls_a_member(node: &Node) -> bool {
    let mut current = node;
    loop {
        match current.kind.as_str() {
            "ParenExpr" | "ImplicitCastExpr" | "CStyleCastExpr" => {}
            "UnaryOperator" if current.opcode.as_deref() == Some("*") => {}
            "MemberExpr" => return true,
            _ => return false,
        }
        let Some(next) = current.inner.first() else {
            return false;
        };
        current = next;
    }
}

/// The function a call names directly (through parentheses and implicit
/// conversions), if any.
fn called_function(node: &Node) -> Option<&str> {
    let mut current = node;
    loop {
        match current.kind.as_str() {
            "ParenExpr" | "ImplicitCastExpr" => current = current.inner.first()?,
            "UnaryOperator" if current.opcode.as_deref() == Some("*") => {
                current = current.inner.first()?;
            }
            "DeclRefExpr" => {
                let decl = current.referenced.as_ref()?;
                return (decl.kind == "FunctionDecl").then_some(decl.name.as_str());
            }
            _ => return None,
        }
    }
}

/// A type's spelling without qualifiers or white space (`const struct T *` →
/// `structT*`), to compare with a listed record.
fn bare_type(text: &str) -> String {
    text.split(|c: char| c.is_whitespace() || c == '*')
        .filter(|word| {
            !word.is_empty() && !matches!(*word, "const" | "volatile" | "restrict" | "__restrict")
        })
        .collect::<Vec<_>>()
        .join(" ")
        + &"*".repeat(text.matches('*').count())
}

impl Reader<'_> {
    /// Card W47-PREMISES-INSTANCE: the listed record a node's type is (or,
    /// with `pointer`, points to once).
    fn viewed_record(&self, node: &Node, pointer: bool) -> Option<String> {
        let spelled = [node.desugared.as_deref(), node.qual_type.as_deref()];
        self.options
            .record_views
            .iter()
            .find(|record| {
                let want = if pointer {
                    format!("{record}*")
                } else {
                    (*record).clone()
                };
                spelled.iter().flatten().any(|text| bare_type(text) == want)
            })
            .cloned()
    }

    fn view(&mut self, record: &str, place: Option<Place>, function: Option<&str>, kind: String) {
        let Some((file, line)) = self.place(place) else {
            return;
        };
        self.facts
            .record_views
            .entry(record.to_owned())
            .or_default()
            .insert(format!("{file}|{line}|{}|{kind}", function.unwrap_or("")));
    }

    /// Card W47-PREMISES-INSTANCE: record a non-member use of a listed record.
    fn record_view(&mut self, node: &Node, function: Option<&str>, role: Use<'_>) {
        match node.kind.as_str() {
            "ImplicitCastExpr" | "CStyleCastExpr" => {
                let Some(operand) = node.inner.first() else {
                    return;
                };
                let Some(record) = self.viewed_record(operand, true) else {
                    return;
                };
                if self.viewed_record(node, true).as_deref() == Some(record.as_str()) {
                    return;
                }
                let destination = node
                    .desugared
                    .as_deref()
                    .or(node.qual_type.as_deref())
                    .map_or_else(|| "?".to_owned(), bare_type);
                let context = match role {
                    Use::Argument(callee) => format!("arg:{callee}"),
                    Use::Compare => "compare".to_owned(),
                    _ => "value".to_owned(),
                };
                self.view(
                    &record,
                    node.begin,
                    function,
                    format!("conv:{destination}|{context}"),
                );
            }
            "BinaryOperator" if node.opcode.as_deref() == Some("=") => {
                if let Some(record) = self.viewed_record(node, false) {
                    self.view(&record, node.begin, function, "assign".to_owned());
                }
            }
            "VarDecl" if !node.inner.is_empty() => {
                if let Some(record) = self.viewed_record(node, false) {
                    self.view(&record, node.begin, function, "init".to_owned());
                }
            }
            "ParmVarDecl" => {
                if let Some(record) = self.viewed_record(node, false) {
                    self.view(&record, node.begin, function, "param".to_owned());
                }
            }
            _ => {}
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

/// Card W47-SERVICE-PROPERTY-KEYS: what one argument (or initializer slot)
/// is, through parentheses and casts: `s:TEXT` a string literal (`TEXT` as
/// Clang prints it, between its quotes, escapes kept), `null` the integer
/// constant `0` converted to a pointer by a `NullToPointer` cast (`NULL`),
/// `i:VALUE` any other integer literal, `v:NAME` a reference to a declaration
/// (a variable, a parameter, an enumerator), `offsetof` an `offsetof`
/// expression, `?` anything else.
fn argument_descriptor(node: &Node) -> String {
    let mut current = node;
    let mut null_cast = false;
    loop {
        match current.kind.as_str() {
            "ParenExpr" => {}
            "ImplicitCastExpr" | "CStyleCastExpr" => {
                null_cast |= current.cast_kind.as_deref() == Some("NullToPointer");
            }
            "StringLiteral" => {
                return current
                    .value
                    .as_deref()
                    .and_then(|value| value.strip_prefix('"'))
                    .and_then(|value| value.strip_suffix('"'))
                    .map_or_else(|| "?".to_owned(), |value| format!("s:{value}"));
            }
            "IntegerLiteral" => {
                return match current.value.as_deref() {
                    Some("0") if null_cast => "null".to_owned(),
                    Some(value) if !null_cast => format!("i:{value}"),
                    _ => "?".to_owned(),
                };
            }
            "DeclRefExpr" => {
                return current
                    .referenced
                    .as_ref()
                    .map_or_else(|| "?".to_owned(), |decl| format!("v:{}", decl.name));
            }
            "OffsetOfExpr" => return "offsetof".to_owned(),
            _ => return "?".to_owned(),
        }
        let Some(next) = current.inner.first() else {
            return "?".to_owned();
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
