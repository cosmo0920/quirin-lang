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

//! Card **CG14-AGGREGATE-VALUES-AND-INITIALIZERS** (gateway rule 11f):
//! whole-record copies, braced initializers, compound literals and string
//! initializers of character arrays, read from the Clang AST on route G.
//!
//! ## What was refused
//!
//! The lexical gate refuses every statement holding a brace (`braced
//! initializer, expression, or declaration is not modeled`), W46's typed
//! statement reader stores and reads only scalars and pointers (its `path`
//! requires a scalar place), and a character array initialised from a string
//! literal is an unrecognised destination. So `*a = *b`, `h->value =
//! h->value`, `struct P s = { 0 };`, `q_observe(&(struct P){ .x = 1 })` and
//! `char buf[] = "abc";` kept their refusals (CG00's M08–M11 controls).
//!
//! ## The four forms
//!
//! 1. **Copies (M08).** `P = Q` and `T l = Q;` of record type are a field-wise
//!    copy. A copied scalar is a scalar read and write; a copied pointer member
//!    would be an *alias* of the source's value, never a second owner
//!    (`QCore.AggregateValue.copy_adds_no_owner`). This rule admits a copy only
//!    of a **custody-free** record ([`AggregateFacts::class_of`]: every member,
//!    transitively, a builtin scalar, an enumeration, or an array or record of
//!    them), whose copy moves no identity at all: it is the source place's
//!    typed read and the destination place's typed store, each with the
//!    place's own per-hop obligations (W46/CG10's [`Reader::path`], asked with
//!    [`Reader::aggregate_target`]). A record with a pointer member is
//!    refused by name (`pointer-members`): the owned-field model forbids
//!    copying a record that holds an owned field
//!    (`QCore.AggregateOwnedField.an_aggregate_is_never_copied`), and no
//!    contract or owner premise here says which copy owns, or that the source's
//!    members own nothing. A union copy composes with CG13's reading: its path
//!    steps are decided by rule 10w as a scalar read or store, and a union
//!    that can hold a pointer is not custody-free.
//! 2. **Initializers (M09).** A declaration of an automatic record or array
//!    initialised by a braced list is read from Clang's *semantic*
//!    `InitListExpr`, where every member and element is explicit: a designator
//!    is resolved to its member, and a member or element the list does not name
//!    is an `ImplicitValueInitExpr` (or the array filler), the zero of its type.
//!    A zero pointer member is the null pointer, with no identity
//!    (`QCore.AggregateValue.zero_fill_has_no_identity`). Every leaf must carry
//!    no identity ([`Leaf::admitted`]): a zero, a scalar the typed reader reads
//!    (its places with their obligations), the null constant, a string literal,
//!    a function designator, or a custody-free record read as in (1). An
//!    initial alias — a formal's or local's pointer, a pointer read, an address,
//!    a call — refuses by name: the local would hold an identity no binding
//!    names. A local whose record or array can hold a pointer is guarded in
//!    the flow by rule 10v's pointer-store arm (owner `None`, value class
//!    `null`): it keeps its older refusal where a place rooted at the local is
//!    bound with a target or unknown (a stale binding its new value would
//!    contradict), and every such place is forgotten after it; as for every
//!    admitted pointer store, its body publishes no derived summary.
//! 3. **Compound literals (M10).** A compound literal in a function body has
//!    automatic storage whose lifetime is its enclosing block. Its value or
//!    address is admitted only as a whole actual of a contracted call at a
//!    `Borrowed` formal, with an initializer as in (2) that reads no memory: a
//!    borrow ends when the call returns, inside the block
//!    (`QCore.AggregateValue.a_literal_borrow_ends_in_its_block`). The actual
//!    carries no identity, so the call is instantiated (rule 10u) with the
//!    literal respelled as the null constant. A custody-taking formal refuses
//!    (`literal-custody-position`), and every other position — a `return`, a
//!    store, an initializer of a pointer — keeps the gate's refusal: an escape
//!    past its block is never admitted.
//! 4. **String initializers (M11).** `char buf[N] = "…";` (or `char buf[] =
//!    "…";`) of an automatic character array is a declaration with no custody
//!    effect. Its byte extent (`N`, or the literal's length with its
//!    terminator) and terminator fact (whether the literal's bytes plus the
//!    terminator fit) are recorded on the parity stream for CG16. **No bound
//!    is claimed** here: an access is read exactly as before, with no extent.
//!
//! Every other form refuses by name ([`Refusal`]), and a declined statement
//! keeps its older refusal byte for byte. The rule is asked after every
//! older route of the statement (hard invariant 3). Route T is unchanged: its
//! one initializer is the two-field `struct timeval`, and no T consumer
//! decides a wider one.
//!
//! The Lean model is `QCore.AggregateValue`; its decision vectors are this
//! module's [`COPY_VECTORS`], [`LEAF_VECTORS`] and [`LITERAL_VECTORS`].

use std::collections::BTreeMap;

use serde_json::Value;

use crate::typed_access_path::{Reader, qual};
use crate::typed_c_ast::{children, kind};

/// Recursion bound on type resolution: records cannot contain themselves by
/// value, so a chain this deep is refused as unread.
const DEPTH: usize = 32;

// ---------------------------------------------------------------------------
// Record facts
// ---------------------------------------------------------------------------

/// The unit's record and typedef declarations, read from its Clang AST once.
#[derive(Clone, Debug, Default)]
pub(crate) struct AggregateFacts {
    /// A complete `RecordDecl` id → its members, in declaration order.
    records: BTreeMap<String, Vec<Member>>,
    /// `struct NAME` / `union NAME` → its one complete definition (`None`
    /// when the unit defines the spelling more than once).
    named: BTreeMap<String, Option<String>>,
    /// A `TypedefDecl` id → what it names.
    typedefs: BTreeMap<String, Target>,
    /// A typedef's name → its one declaration (`None` when ambiguous).
    typedef_names: BTreeMap<String, Option<String>>,
}

#[derive(Clone, Debug)]
struct Member {
    ty: Value,
    /// The record defined inline right before the member, when the member's
    /// type is that unnamed record.
    inline: Option<String>,
}

#[derive(Clone, Debug)]
enum Target {
    Record(String),
    Enum,
    Alias(String),
    Type(Value),
}

/// The custody class of a type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TypeClass {
    /// A builtin scalar or an enumeration (or an array of them).
    Scalar,
    /// An object or function pointer (or an array of them).
    Pointer,
    /// A record none of whose members can hold a pointer.
    CustodyFree,
    /// A record some member of which (transitively) is a pointer.
    PointerBearing,
    /// A type this reader does not resolve (incomplete, ambiguous,
    /// `volatile`/`_Atomic`, or unlisted).
    Unread,
}

impl TypeClass {
    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::Scalar => "scalar",
            Self::Pointer => "pointer",
            Self::CustodyFree => "custody-free",
            Self::PointerBearing => "pointer-bearing",
            Self::Unread => "unread",
        }
    }

    /// The class of a record or array from its members' (or element's)
    /// classes: unread dominates, then any pointer.
    fn join(classes: impl IntoIterator<Item = TypeClass>) -> TypeClass {
        let mut out = TypeClass::CustodyFree;
        for class in classes {
            match class {
                TypeClass::Unread => return TypeClass::Unread,
                TypeClass::Pointer | TypeClass::PointerBearing => out = TypeClass::PointerBearing,
                TypeClass::Scalar | TypeClass::CustodyFree => {}
            }
        }
        out
    }

    /// Whether a value of this class can hold an identity.
    pub(crate) fn may_hold_identity(self) -> bool {
        !matches!(self, Self::Scalar | Self::CustodyFree)
    }
}

impl AggregateFacts {
    /// Collect the facts of every record and typedef under `node`.
    pub(crate) fn collect(node: &Value, facts: &mut Self) {
        match kind(node) {
            "RecordDecl" if node["completeDefinition"].as_bool() == Some(true) => {
                facts.record(node);
            }
            "TypedefDecl" => facts.typedef(node),
            _ => {}
        }
        for child in children(node) {
            Self::collect(child, facts);
        }
    }

    fn record(&mut self, record: &Value) {
        let Some(id) = record["id"].as_str() else {
            return;
        };
        let mut members = Vec::new();
        let mut previous: Option<&str> = None;
        for child in children(record) {
            match kind(child) {
                "RecordDecl" => {
                    previous = child["id"].as_str();
                }
                "FieldDecl" => {
                    let text = child["type"]["qualType"].as_str().unwrap_or("");
                    let inline = previous
                        .take()
                        .filter(|_| text.contains("(unnamed") || text.contains("(anonymous"))
                        .map(str::to_owned);
                    members.push(Member {
                        ty: child["type"].clone(),
                        inline,
                    });
                }
                _ => previous = None,
            }
        }
        self.records.insert(id.to_owned(), members);
        if let (Some(tag @ ("struct" | "union")), Some(name)) = (
            record["tagUsed"].as_str(),
            record["name"].as_str().filter(|name| !name.is_empty()),
        ) {
            let spelling = format!("{tag} {name}");
            match self.named.get(&spelling) {
                None => {
                    self.named.insert(spelling, Some(id.to_owned()));
                }
                Some(Some(existing)) if existing == id => {}
                Some(_) => {
                    self.named.insert(spelling, None);
                }
            }
        }
    }

    fn typedef(&mut self, node: &Value) {
        let (Some(id), Some(name)) = (node["id"].as_str(), node["name"].as_str()) else {
            return;
        };
        let mut cursor = children(node).first();
        let target = loop {
            let Some(inner) = cursor else {
                break Target::Type(node["type"].clone());
            };
            match kind(inner) {
                "ElaboratedType" | "ParenType" => cursor = children(inner).first(),
                "RecordType" => match inner["decl"]["id"].as_str() {
                    Some(record) => break Target::Record(record.to_owned()),
                    None => break Target::Type(node["type"].clone()),
                },
                "EnumType" => break Target::Enum,
                "TypedefType" => match inner["decl"]["id"].as_str() {
                    Some(alias) if alias != id => break Target::Alias(alias.to_owned()),
                    _ => break Target::Type(node["type"].clone()),
                },
                _ => break Target::Type(node["type"].clone()),
            }
        };
        self.typedefs.insert(id.to_owned(), target);
        match self.typedef_names.get(name) {
            None => {
                self.typedef_names
                    .insert(name.to_owned(), Some(id.to_owned()));
            }
            Some(Some(existing)) if existing == id => {}
            Some(_) => {
                self.typedef_names.insert(name.to_owned(), None);
            }
        }
    }

    /// The class of the complete record `id` (an unnamed record defined
    /// inline in a declaration).
    pub(crate) fn record_class_of(&self, id: &str) -> TypeClass {
        self.record_class(id, 0)
    }

    /// The class of the type of a Clang node (`node["type"]`).
    pub(crate) fn class_of(&self, node: &Value) -> TypeClass {
        self.class_of_type(&node["type"], 0)
    }

    fn class_of_type(&self, ty: &Value, depth: usize) -> TypeClass {
        let Some(text) = ty["desugaredQualType"]
            .as_str()
            .or_else(|| ty["qualType"].as_str())
        else {
            return TypeClass::Unread;
        };
        self.class_of_text(text, ty["typeAliasDeclId"].as_str(), depth)
    }

    fn class_of_text(&self, text: &str, alias: Option<&str>, depth: usize) -> TypeClass {
        if depth > DEPTH || text.contains("volatile") || text.contains("_Atomic") {
            return TypeClass::Unread;
        }
        let text = text.trim();
        if text.contains("(*") || text.contains("(^") {
            return TypeClass::Pointer;
        }
        if text.ends_with(']') {
            let Some(open) = text.find('[') else {
                return TypeClass::Unread;
            };
            let element = self.class_of_text(&text[..open], None, depth + 1);
            return match element {
                TypeClass::Scalar | TypeClass::Pointer | TypeClass::Unread => element,
                record => record,
            };
        }
        if text.ends_with('*') {
            return TypeClass::Pointer;
        }
        if scalar_words(text) {
            return TypeClass::Scalar;
        }
        let bare = strip_qualifiers(text);
        if bare.starts_with("enum ") && !bare.contains('(') {
            return TypeClass::Scalar;
        }
        if let Some(alias) = alias {
            return self.typedef_class(alias, depth + 1);
        }
        if bare.starts_with("struct ") || bare.starts_with("union ") {
            return match self.named.get(bare) {
                Some(Some(record)) => self.record_class(record, depth + 1),
                _ => TypeClass::Unread,
            };
        }
        if bare
            .bytes()
            .all(|byte| byte == b'_' || byte.is_ascii_alphanumeric())
        {
            return match self.typedef_names.get(bare) {
                Some(Some(typedef)) => self.typedef_class(typedef, depth + 1),
                _ => TypeClass::Unread,
            };
        }
        TypeClass::Unread
    }

    fn typedef_class(&self, id: &str, depth: usize) -> TypeClass {
        if depth > DEPTH {
            return TypeClass::Unread;
        }
        match self.typedefs.get(id) {
            Some(Target::Record(record)) => self.record_class(record, depth + 1),
            Some(Target::Enum) => TypeClass::Scalar,
            Some(Target::Alias(alias)) => self.typedef_class(alias, depth + 1),
            Some(Target::Type(ty)) => self.class_of_type(ty, depth + 1),
            None => TypeClass::Unread,
        }
    }

    fn record_class(&self, id: &str, depth: usize) -> TypeClass {
        if depth > DEPTH {
            return TypeClass::Unread;
        }
        let Some(members) = self.records.get(id) else {
            return TypeClass::Unread;
        };
        TypeClass::join(members.iter().map(|member| match &member.inline {
            Some(inline) => self.record_class(inline, depth + 1),
            None => self.class_of_type(&member.ty, depth + 1),
        }))
    }
}

fn strip_qualifiers(mut text: &str) -> &str {
    loop {
        let before = text;
        for qualifier in ["const ", "restrict "] {
            text = text.trim_start_matches(qualifier);
        }
        text = text.trim_end_matches(" const").trim();
        if text == before {
            return text;
        }
    }
}

/// A builtin arithmetic type spelled with its keywords only.
fn scalar_words(text: &str) -> bool {
    let mut any = false;
    for word in text.split_whitespace() {
        match word {
            "const" | "restrict" => {}
            "signed" | "unsigned" | "short" | "long" | "char" | "int" | "float" | "double"
            | "_Bool" | "bool" | "__int128" => any = true,
            _ => return false,
        }
    }
    any
}

// ---------------------------------------------------------------------------
// Refusals and decisions (shared with `QCore.AggregateValue`)
// ---------------------------------------------------------------------------

/// Why this rule does not read a form, each by name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Refusal {
    /// A copied record with a pointer member: which copy owns is unstated.
    PointerMembers,
    /// A record type this reader does not resolve.
    UnreadRecord,
    /// A copy operand that is neither a typed place nor an automatic object
    /// (a call, a conditional, a static object, …).
    CopyOperand(&'static str),
    /// The typed reader refused a place or a scalar value (W46/CG10).
    Reader,
    /// An initializer leaf that may carry an identity, by class.
    Leaf(Leaf),
    /// A form of initializer this rule does not list.
    InitializerForm(&'static str),
    /// A compound literal at a custody-taking formal.
    LiteralCustodyPosition,
    /// A compound literal whose initializer reads memory.
    LiteralReads,
    /// A compound literal at an actual of a call with no contract in force:
    /// no formal mode says the literal is only borrowed.
    UncontractedCallee,
    /// A compound literal anywhere but a whole actual of a contracted call.
    LiteralPosition,
    /// A declaration this rule does not read (static, several declarators,
    /// a non-aggregate type).
    Declaration(&'static str),
    /// No unique Clang node for the statement.
    Node,
    /// The respelled call was refused by the one application point (rule
    /// 10u), with its refusal code.
    Instantiation(String),
}

impl Refusal {
    pub(crate) fn code(&self) -> String {
        match self {
            Self::PointerMembers => "pointer-members".into(),
            Self::UnreadRecord => "unread-record".into(),
            Self::CopyOperand(what) => format!("copy-operand:{what}"),
            Self::Reader => "reader".into(),
            Self::Leaf(leaf) => format!("leaf:{}", leaf.code()),
            Self::InitializerForm(what) => format!("initializer-form:{what}"),
            Self::LiteralCustodyPosition => "literal-custody-position".into(),
            Self::LiteralReads => "literal-reads".into(),
            Self::UncontractedCallee => "literal-callee-uncontracted".into(),
            Self::LiteralPosition => "literal-position".into(),
            Self::Declaration(what) => format!("declaration:{what}"),
            Self::Node => "node".into(),
            Self::Instantiation(code) => format!("instantiation:{code}"),
        }
    }
}

/// One side of a copy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Side {
    /// A member, subscript or indirection the typed reader reads.
    Place,
    /// An automatic object or formal of this frame, named whole.
    Local,
    /// Anything else.
    Other,
}

impl Side {
    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::Place => "place",
            Self::Local => "frame",
            Self::Other => "other",
        }
    }
}

/// The copy decision (M08): a pure function of the record's class and the
/// two sides. `QCore.AggregateValue.decideCopy` is the same function.
pub(crate) fn decide_copy(
    class: TypeClass,
    destination: Side,
    source: Side,
) -> Result<(), Refusal> {
    match class {
        TypeClass::CustodyFree => {}
        TypeClass::PointerBearing => return Err(Refusal::PointerMembers),
        _ => return Err(Refusal::UnreadRecord),
    }
    if destination == Side::Other {
        return Err(Refusal::CopyOperand("destination"));
    }
    if source == Side::Other {
        return Err(Refusal::CopyOperand("source"));
    }
    Ok(())
}

/// The class of one initializer leaf (M09, M10).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Leaf {
    /// An implicit value initialization: the zero of its type.
    Zero,
    /// A scalar value the typed reader reads.
    Scalar,
    /// A character array initialised by a string literal.
    StringArray,
    /// The null pointer constant.
    Null,
    /// A string literal decaying to a pointer (static storage).
    StringLiteral,
    /// A function designator, or its address.
    Function,
    /// A custody-free record value read as a copy.
    CustodyFreeRecord,
    /// A record value that can hold a pointer.
    PointerBearingRecord,
    /// A formal's object pointer.
    FormalPointer,
    /// Another named object pointer.
    LocalPointer,
    /// A pointer read from memory.
    PointerRead,
    /// An address (of an object, a member or an array).
    Address,
    /// A call.
    Call,
    /// A nested compound literal.
    CompoundLiteral,
    /// Anything else.
    Other,
}

impl Leaf {
    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::Zero => "zero",
            Self::Scalar => "scalar",
            Self::StringArray => "string-array",
            Self::Null => "null",
            Self::StringLiteral => "string",
            Self::Function => "function",
            Self::CustodyFreeRecord => "custody-free-record",
            Self::PointerBearingRecord => "pointer-bearing-record",
            Self::FormalPointer => "formal-pointer",
            Self::LocalPointer => "local-pointer",
            Self::PointerRead => "pointer-read",
            Self::Address => "address",
            Self::Call => "call",
            Self::CompoundLiteral => "compound-literal",
            Self::Other => "other",
        }
    }

    /// The leaves that carry no identity. `QCore.AggregateValue.Leaf.admitted`
    /// is the same predicate.
    pub(crate) fn admitted(self) -> bool {
        matches!(
            self,
            Self::Zero
                | Self::Scalar
                | Self::StringArray
                | Self::Null
                | Self::StringLiteral
                | Self::Function
                | Self::CustodyFreeRecord
        )
    }
}

/// Where a compound literal's value or address is used.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LiteralUse {
    /// A whole actual at a `Borrowed` formal of a contracted call.
    Borrowed,
    /// A whole actual at a custody-taking formal.
    CustodyTaking,
    /// Any other position: returned, stored, bound, nested.
    Elsewhere,
}

impl LiteralUse {
    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::Borrowed => "borrowed",
            Self::CustodyTaking => "custody-taking",
            Self::Elsewhere => "elsewhere",
        }
    }
}

/// The compound-literal decision (M10). `QCore.AggregateValue.decideLiteral`
/// is the same function.
pub(crate) fn decide_literal(
    identity_free: bool,
    reads: bool,
    use_: LiteralUse,
) -> Result<(), Refusal> {
    match use_ {
        LiteralUse::Elsewhere => Err(Refusal::LiteralPosition),
        LiteralUse::CustodyTaking => Err(Refusal::LiteralCustodyPosition),
        LiteralUse::Borrowed if !identity_free => Err(Refusal::Leaf(Leaf::Other)),
        LiteralUse::Borrowed if reads => Err(Refusal::LiteralReads),
        LiteralUse::Borrowed => Ok(()),
    }
}

/// `(spelling, record class, destination, source, decision)`, shared with
/// `proof/QCore/AggregateValue.lean` (`copyVectors`).
#[cfg(test)]
pub(crate) const COPY_VECTORS: &[(&str, &str, &str, &str, &str)] = &[
    ("*a = *b", "custody-free", "place", "place", "admit"),
    (
        "h->value = h->value",
        "custody-free",
        "place",
        "place",
        "admit",
    ),
    ("l = *b", "custody-free", "frame", "place", "admit"),
    (
        "struct P l = b->q",
        "custody-free",
        "frame",
        "place",
        "admit",
    ),
    ("*a = l", "custody-free", "place", "frame", "admit"),
    (
        "*a = *b",
        "pointer-bearing",
        "place",
        "place",
        "pointer-members",
    ),
    (
        "struct S l = *b",
        "pointer-bearing",
        "frame",
        "place",
        "pointer-members",
    ),
    ("*a = *b", "unread", "place", "place", "unread-record"),
    (
        "*a = f()",
        "custody-free",
        "place",
        "other",
        "copy-operand:source",
    ),
    (
        "g_rec = *b",
        "custody-free",
        "other",
        "place",
        "copy-operand:destination",
    ),
];

/// `(leaf, admitted)`, shared with `copyVectors`' companion `leafVectors`.
#[cfg(test)]
pub(crate) const LEAF_VECTORS: &[(&str, bool)] = &[
    ("zero", true),
    ("scalar", true),
    ("string-array", true),
    ("null", true),
    ("string", true),
    ("function", true),
    ("custody-free-record", true),
    ("pointer-bearing-record", false),
    ("formal-pointer", false),
    ("local-pointer", false),
    ("pointer-read", false),
    ("address", false),
    ("call", false),
    ("compound-literal", false),
    ("other", false),
];

/// `(identity-free, reads, use, decision)`, shared with `literalVectors`.
#[cfg(test)]
pub(crate) const LITERAL_VECTORS: &[(bool, bool, &str, &str)] = &[
    (true, false, "borrowed", "admit"),
    (true, true, "borrowed", "literal-reads"),
    (false, false, "borrowed", "leaf:other"),
    (true, false, "custody-taking", "literal-custody-position"),
    (true, false, "elsewhere", "literal-position"),
    (false, true, "elsewhere", "literal-position"),
];

// ---------------------------------------------------------------------------
// Readers
// ---------------------------------------------------------------------------

/// Parentheses and the listed implicit or explicit casts; the casts seen.
fn peel<'a>(mut node: &'a Value, casts: &[&str]) -> (&'a Value, Vec<&'a str>) {
    let mut seen = Vec::new();
    loop {
        match kind(node) {
            "ParenExpr" => {}
            "ImplicitCastExpr" | "CStyleCastExpr" => match node["castKind"].as_str() {
                Some(cast) if casts.contains(&cast) => seen.push(cast),
                _ => return (node, seen),
            },
            _ => return (node, seen),
        }
        match children(node).first() {
            Some(inner) => node = inner,
            None => return (node, seen),
        }
    }
}

/// The class of a pointer-typed initializer leaf, from the AST only.
pub(crate) fn pointer_leaf(node: &Value) -> Leaf {
    let (inner, casts) = peel(
        node,
        &[
            "NullToPointer",
            "BitCast",
            "NoOp",
            "ArrayToPointerDecay",
            "FunctionToPointerDecay",
            "LValueToRValue",
        ],
    );
    match kind(inner) {
        "IntegerLiteral" | "GNUNullExpr"
            if casts.contains(&"NullToPointer")
                && (kind(inner) == "GNUNullExpr" || inner["value"].as_str() == Some("0")) =>
        {
            Leaf::Null
        }
        "StringLiteral" if casts.contains(&"ArrayToPointerDecay") => Leaf::StringLiteral,
        "DeclRefExpr" => match inner["referencedDecl"]["kind"].as_str() {
            Some("FunctionDecl") if casts.contains(&"FunctionToPointerDecay") => Leaf::Function,
            Some("VarDecl" | "ParmVarDecl") if casts.contains(&"ArrayToPointerDecay") => {
                Leaf::Address
            }
            Some("ParmVarDecl") => Leaf::FormalPointer,
            Some("VarDecl") => Leaf::LocalPointer,
            _ => Leaf::Other,
        },
        "UnaryOperator" if inner["opcode"].as_str() == Some("&") => {
            let operand = children(inner).first().map(|operand| peel(operand, &[]).0);
            if operand.is_some_and(|operand| {
                kind(operand) == "DeclRefExpr"
                    && operand["referencedDecl"]["kind"].as_str() == Some("FunctionDecl")
            }) {
                Leaf::Function
            } else if operand.is_some_and(|operand| kind(operand) == "CompoundLiteralExpr") {
                Leaf::CompoundLiteral
            } else {
                Leaf::Address
            }
        }
        "MemberExpr" | "ArraySubscriptExpr" => {
            if casts.contains(&"ArrayToPointerDecay") {
                Leaf::Address
            } else {
                Leaf::PointerRead
            }
        }
        "UnaryOperator" if inner["opcode"].as_str() == Some("*") => Leaf::PointerRead,
        "CallExpr" => Leaf::Call,
        "CompoundLiteralExpr" => Leaf::CompoundLiteral,
        _ => Leaf::Other,
    }
}

/// Whether `node` is a pointer-typed value (object or function pointer).
fn pointer_typed(node: &Value) -> bool {
    qual(node).is_some_and(|text| {
        let text = text.trim();
        text.ends_with('*') || text.contains("(*")
    })
}

/// The side of a copy operand; `function` is the body, for the storage of a
/// named object.
pub(crate) fn side(node: &Value, function: Option<&Value>) -> Side {
    let (inner, _) = peel(node, &["LValueToRValue", "NoOp"]);
    match kind(inner) {
        "MemberExpr" | "ArraySubscriptExpr" => Side::Place,
        "UnaryOperator" if inner["opcode"].as_str() == Some("*") => Side::Place,
        "DeclRefExpr" => {
            let Some(id) = inner["referencedDecl"]["id"].as_str() else {
                return Side::Other;
            };
            if function.is_some_and(|function| automatic_object(function, id)) {
                Side::Local
            } else {
                Side::Other
            }
        }
        _ => Side::Other,
    }
}

/// Whether `id` declares a formal or an automatic local of `function`.
pub(crate) fn automatic_object(function: &Value, id: &str) -> bool {
    fn walk(node: &Value, id: &str) -> bool {
        match kind(node) {
            "ParmVarDecl" if node["id"].as_str() == Some(id) => return true,
            "VarDecl" if node["id"].as_str() == Some(id) => {
                return node["storageClass"].is_null();
            }
            _ => {}
        }
        children(node).iter().any(|child| walk(child, id))
    }
    walk(function, id)
}

/// Read a copy `destination = source` (or a declaration whose initializer is
/// `source`, `destination` `None`) of the record type of `source` into
/// `reader`'s plan: the source's typed read and the destination's typed
/// store, each with its per-hop obligations.
pub(crate) fn read_copy(
    reader: &mut Reader<'_>,
    facts: &AggregateFacts,
    destination: Option<&Value>,
    source: &Value,
) -> Result<TypeClass, Refusal> {
    let class = facts.class_of(source);
    let function = reader.function();
    let destination_side = destination.map_or(Side::Local, |node| side(node, function));
    let source_side = side(source, function);
    decide_copy(class, destination_side, source_side)?;
    if source_side == Side::Place {
        let (place, _) = peel(source, &["LValueToRValue", "NoOp"]);
        reader.aggregate_target = true;
        reader.path(place, false, false).ok_or(Refusal::Reader)?;
    }
    if let Some(destination) = destination
        && destination_side == Side::Place
    {
        let (place, _) = peel(destination, &["NoOp"]);
        reader.aggregate_target = true;
        reader.path(place, true, false).ok_or(Refusal::Reader)?;
    }
    Ok(class)
}

/// Walk one initializer (the semantic `InitListExpr` tree, or a single value),
/// reading every scalar and custody-free record leaf into `reader`'s plan and
/// classifying every leaf.
pub(crate) fn read_initializer(
    reader: &mut Reader<'_>,
    facts: &AggregateFacts,
    node: &Value,
    leaves: &mut Vec<Leaf>,
) -> Result<(), Refusal> {
    match kind(node) {
        "InitListExpr" => {
            for child in children(node) {
                read_initializer(reader, facts, child, leaves)?;
            }
            if let Some(filler) = node["array_filler"].as_array() {
                for child in filler {
                    read_initializer(reader, facts, child, leaves)?;
                }
            }
            Ok(())
        }
        "ImplicitValueInitExpr" => {
            leaves.push(Leaf::Zero);
            Ok(())
        }
        "DesignatedInitUpdateExpr" | "NoInitExpr" | "DesignatedInitExpr" => {
            Err(Refusal::InitializerForm("designated update"))
        }
        "StringLiteral" if qual(node).is_some_and(|text| text.ends_with(']')) => {
            leaves.push(Leaf::StringArray);
            Ok(())
        }
        _ if pointer_typed(node) => {
            let leaf = pointer_leaf(node);
            leaves.push(leaf);
            if leaf.admitted() {
                Ok(())
            } else {
                Err(Refusal::Leaf(leaf))
            }
        }
        _ if reader.scalar(node) => {
            reader.expression(node).ok_or(Refusal::Reader)?;
            leaves.push(Leaf::Scalar);
            Ok(())
        }
        _ if kind(peel(node, &["LValueToRValue", "NoOp"]).0) == "CompoundLiteralExpr" => {
            leaves.push(Leaf::CompoundLiteral);
            Err(Refusal::Leaf(Leaf::CompoundLiteral))
        }
        _ => match facts.class_of(node) {
            TypeClass::CustodyFree => {
                read_copy(reader, facts, None, node)?;
                leaves.push(Leaf::CustodyFreeRecord);
                Ok(())
            }
            TypeClass::PointerBearing => {
                leaves.push(Leaf::PointerBearingRecord);
                Err(Refusal::Leaf(Leaf::PointerBearingRecord))
            }
            _ => {
                leaves.push(Leaf::Other);
                Err(Refusal::Leaf(Leaf::Other))
            }
        },
    }
}

/// The compound literal an actual denotes, and whether the actual is its
/// address (`&(T){…}`, or an array literal decaying) rather than its value.
pub(crate) fn literal_actual(actual: &Value) -> Option<&Value> {
    let (inner, casts) = peel(
        actual,
        &["BitCast", "NoOp", "LValueToRValue", "ArrayToPointerDecay"],
    );
    match kind(inner) {
        "CompoundLiteralExpr" => {
            // A value (a record read whole) or an array decaying to its first
            // element's address; either is storage the literal owns.
            let _ = casts;
            Some(inner)
        }
        "UnaryOperator" if inner["opcode"].as_str() == Some("&") => {
            let operand = peel(children(inner).first()?, &[]).0;
            (kind(operand) == "CompoundLiteralExpr").then_some(operand)
        }
        _ => None,
    }
}

/// Whether `node`'s subtree holds a compound literal.
pub(crate) fn holds_literal(node: &Value) -> bool {
    kind(node) == "CompoundLiteralExpr" || children(node).iter().any(holds_literal)
}

// ---------------------------------------------------------------------------
// String initializers (M11): recorded facts, no bound claimed
// ---------------------------------------------------------------------------

/// The byte extent and terminator fact of `char buf[N] = "…";`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct StringExtent {
    /// The array's bytes (`N`, or Clang's completed bound).
    pub(crate) array_bytes: u64,
    /// The literal's bytes before its terminator, when every escape decodes.
    pub(crate) literal_bytes: Option<u64>,
}

impl StringExtent {
    /// Whether the array holds the literal's terminator: C11 6.7.9p14 stores
    /// it only when room is left.
    pub(crate) fn terminated(self) -> Option<bool> {
        self.literal_bytes.map(|bytes| bytes < self.array_bytes)
    }
}

/// The extent of a character array initialised by a string literal, from
/// the declaration's type and the literal's spelling.
pub(crate) fn string_extent(declaration: &Value, literal: &Value) -> Option<StringExtent> {
    let text = qual(declaration)?.trim();
    let element = &text[..text.find('[')?];
    if !matches!(
        strip_qualifiers(element),
        "char" | "signed char" | "unsigned char"
    ) {
        return None;
    }
    let bound = text[text.find('[')? + 1..text.rfind(']')?].trim();
    let array_bytes = bound.parse::<u64>().ok()?;
    Some(StringExtent {
        array_bytes,
        literal_bytes: literal["value"].as_str().and_then(decoded_length),
    })
}

/// The number of bytes a plain C string literal's spelling (`"…"`) denotes,
/// its terminator excluded; `None` for a prefix, a concatenation Clang did not
/// fold, or an escape this decoder does not read.
fn decoded_length(spelling: &str) -> Option<u64> {
    let body = spelling.strip_prefix('"')?.strip_suffix('"')?;
    let mut bytes = 0u64;
    let mut chars = body.chars();
    while let Some(c) = chars.next() {
        if c == '"' {
            return None;
        }
        if c != '\\' {
            bytes += c.len_utf8() as u64;
            continue;
        }
        match chars.next()? {
            // `\0` alone; a longer octal escape is not decoded.
            '0' if chars.as_str().starts_with(|next: char| next.is_digit(8)) => {
                return None;
            }
            'n' | 't' | 'r' | '0' | '\\' | '\'' | '"' | 'a' | 'b' | 'f' | 'v' | '?' => bytes += 1,
            'x' => {
                let rest = chars.as_str();
                let digits = rest.bytes().take_while(u8::is_ascii_hexdigit).count();
                if digits == 0 || digits > 2 {
                    return None;
                }
                chars = rest[digits..].chars();
                bytes += 1;
            }
            _ => return None,
        }
    }
    Some(bytes)
}

// ---------------------------------------------------------------------------
// Parity
// ---------------------------------------------------------------------------

/// Parity only: one `aggregate-value` row per consultation.
pub(crate) fn record(
    function: &str,
    line: usize,
    texts: &[&str],
    form: &str,
    outcome: &Result<(), Refusal>,
    detail: serde_json::Value,
) {
    if !crate::typed_access_path::parity_enabled() {
        return;
    }
    crate::typed_access_path::record_row(serde_json::json!({
        "function": function,
        "line": line,
        "texts": texts,
        "position": "aggregate-value",
        "form": form,
        "admitted": outcome.is_ok(),
        "refusal": outcome.as_ref().err().map(Refusal::code),
        "detail": detail,
    }));
}

#[cfg(test)]
#[path = "../tests/unit/aggregate_value.rs"]
mod tests;
