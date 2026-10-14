//! Card **CG22-OP-CALLBACKS-AND-SERVED-QUEUES** (gateway rule 12n; contract
//! spec §C04.19.3): the closure of a function-pointer member's stores, the
//! candidate set of `resource_callback_member_invoked(OBJ, TAG.FIELD, A…)`.
//!
//! USER DECISION 2026-10-14 (clause 3): the dotted spelling `TAG.FIELD`, the
//! closure run-wide over Clang's stores, premise `P-CG22-MEMBER-CLOSED-WORLD`.
//!
//! # Facts (one unit)
//!
//! [`MemberFacts::collect`] reads every top-level declaration of a unit's
//! Clang AST once ([`crate::typed_access_path`]'s retained declarations):
//!
//! * every store into a function-pointer member (`X->F = V`, `X.a.b = V`),
//!   keyed `TAG.PATH` (the record the path is read from), with what `V` is
//!   ([`Flow`]): a function designator, a null pointer constant, a read of
//!   another such member, a formal of the enclosing function, or anything
//!   else (open, with why); a member whose address is taken is open;
//! * every function-pointer actual of a direct call, keyed by callee and
//!   position, the same way (so a formal's values are its callers' actuals);
//! * every function named other than as a direct callee (its address may
//!   escape, so its formals may be reached by a call no fact records);
//! * the functions the unit defines with internal linkage;
//! * every initializer list of a record the unit defines once at file scope,
//!   mapped onto its fields (card CALLBACK-MEMBER-DATA-ONLY-UNITS): each
//!   function-pointer field's entry is a store into `TAG.FIELD`, an implicit
//!   entry a null pointer;
//! * every record whose function-pointer members an initializer list that
//!   cannot be mapped so, a whole-object assignment from anything but an
//!   object of the same record, or a `memcpy`-family call may write (the
//!   record is open).
//!
//! A unit the gateway does not analyse (a data-only unit) is read the same
//! way by [`read_unit`] (card CALLBACK-MEMBER-DATA-ONLY-UNITS), so every unit
//! of the run contributes facts read from its AST; a unit whose AST is not
//! read contributes none, which opens every member.
//!
//! # Closure (any set of units)
//!
//! [`closure`] is the least set of designators the facts let flow into a
//! member: a designator joins it, a null pointer adds nothing, a member read
//! adds that member's set, a formal of a function whose address is never
//! taken adds its actuals at every call the facts record (an internal
//! function's only in its own unit), and anything else opens it. Over one
//! unit it is that unit's view (rule 12n's in-unit decision); over every
//! analysed unit it is the run's (the fold in `qcore-audit`, which demotes a
//! row whose in-unit set the run's does not contain).

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::typed_c_ast::{children, kind};

/// What a stored or handed value is.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "flow")]
pub enum Flow {
    /// A function designator.
    Function { name: String },
    /// A null pointer constant: no callback.
    Null,
    /// A read of a function-pointer member, `TAG.PATH`.
    Member { member: String },
    /// A formal of the enclosing function.
    Formal { function: String, index: usize },
    /// Anything else.
    Open { why: String },
}

/// One unit's facts.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemberFacts {
    /// `TAG.PATH` (or `?.NAME` when the record is not read) -> stored values.
    pub stores: BTreeMap<String, BTreeSet<Flow>>,
    /// Callee -> position -> function-pointer actuals.
    pub calls: BTreeMap<String, BTreeMap<usize, BTreeSet<Flow>>>,
    /// Functions named other than as a direct callee.
    pub address_taken: BTreeSet<String>,
    /// Functions this unit defines with internal linkage.
    pub internal: BTreeSet<String>,
    /// Records whose function-pointer members something opaque may write,
    /// with why.
    pub opened: BTreeMap<String, String>,
    /// Typedef names of function types and of pointers to them (build state).
    #[serde(skip)]
    function_typedefs: BTreeSet<String>,
    #[serde(skip)]
    pointer_typedefs: BTreeSet<String>,
    /// Typedef name -> the record it names (build state).
    #[serde(skip)]
    record_typedefs: BTreeMap<String, String>,
    /// Card CG22-OP-CALLBACKS-AND-SERVED-QUEUES (rule 12o): every enumerator's
    /// value by declaration id, for the arm check's case labels (not
    /// evidence).
    #[serde(skip)]
    enumerators: BTreeMap<String, i64>,
    /// Card CALLBACK-MEMBER-DATA-ONLY-UNITS (rule 12n): each record the unit
    /// defines once at file scope, with its fields in initializer order;
    /// `None` when its fields cannot be mapped (an anonymous member, a second
    /// different definition, a definition inside a function) (build state).
    #[serde(skip)]
    record_fields: BTreeMap<String, Option<RecordShape>>,
    /// Card OP-APP-RK-CALLBACKS-VIA-POLL-CB (rule 12n, Q3): each record the
    /// unit defines at file scope, as member keys read it (`None`: defined
    /// differently twice in the unit).
    #[serde(default)]
    pub records: BTreeMap<String, Option<RecordKeys>>,
    /// The union tags the unit defines (build state).
    #[serde(skip)]
    unions: BTreeSet<String>,
}

/// Card **OP-APP-RK-CALLBACKS-VIA-POLL-CB** (rule 12n, Q3): a record's
/// fields as member keys read them.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordKeys {
    pub union: bool,
    pub fields: BTreeMap<String, FieldKey>,
}

/// What one field holds, for member keys.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "field")]
pub enum FieldKey {
    /// A function pointer.
    Function,
    /// A named record by value (or an array of them).
    Named { record: String },
    /// An anonymous record by value.
    Inline { keys: Box<RecordKeys> },
    /// Anything else.
    Plain,
}

/// Card CALLBACK-MEMBER-DATA-ONLY-UNITS: a record's fields as an initializer
/// list names them (unnamed bit-fields take no initializer).
#[derive(Clone, Debug, PartialEq, Eq)]
struct RecordShape {
    union: bool,
    fields: Vec<FieldShape>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FieldShape {
    name: String,
    function_pointer: bool,
}

const COPIERS: [&str; 6] = [
    "memcpy",
    "memmove",
    "bcopy",
    "__builtin_memcpy",
    "__builtin_memmove",
    "__builtin___memcpy_chk",
];

fn type_text(node: &Value) -> &str {
    node["type"]["desugaredQualType"]
        .as_str()
        .or_else(|| node["type"]["qualType"].as_str())
        .unwrap_or("")
}

fn qual_text(node: &Value) -> &str {
    node["type"]["qualType"].as_str().unwrap_or("")
}

/// `struct NAME` / `union NAME` in a type spelling.
fn spelled_record(text: &str) -> Option<String> {
    for keyword in ["struct ", "union "] {
        if let Some(at) = text.find(keyword) {
            let name: String = text[at + keyword.len()..]
                .chars()
                .take_while(|ch| *ch == '_' || ch.is_ascii_alphanumeric())
                .collect();
            if !name.is_empty() {
                return Some(name);
            }
        }
    }
    None
}

fn strip_qualifiers(text: &str) -> &str {
    let mut text = text.trim();
    loop {
        let before = text;
        for prefix in ["const ", "volatile ", "restrict "] {
            text = text.strip_prefix(prefix).unwrap_or(text).trim();
        }
        for suffix in [" const", " volatile", " restrict"] {
            text = text.strip_suffix(suffix).unwrap_or(text).trim();
        }
        if before == text {
            return text;
        }
    }
}

pub(crate) fn strip(mut node: &Value) -> &Value {
    loop {
        match kind(node) {
            "ParenExpr" | "ImplicitCastExpr" => match children(node).first() {
                Some(inner) => node = inner,
                None => return node,
            },
            _ => return node,
        }
    }
}

pub(crate) fn null_constant(node: &Value) -> bool {
    let node = strip(node);
    match kind(node) {
        "ImplicitValueInitExpr" | "GNUNullExpr" => true,
        "IntegerLiteral" => node["value"].as_str() == Some("0"),
        "CStyleCastExpr" => children(node).first().is_some_and(null_constant),
        _ => false,
    }
}

pub(crate) fn designator(node: &Value) -> Option<&str> {
    let node = strip(node);
    (kind(node) == "DeclRefExpr" && node["referencedDecl"]["kind"].as_str() == Some("FunctionDecl"))
        .then(|| node["referencedDecl"]["name"].as_str())
        .flatten()
}

/// The direct callee of a call node, when it is a function designator.
pub(crate) fn direct_callee(call: &Value) -> Option<&str> {
    children(call).first().and_then(designator)
}

/// The enclosing function while walking a body.
struct Enclosing<'a> {
    name: &'a str,
    parameters: Vec<&'a str>,
}

impl MemberFacts {
    /// Whether a type spelling is a function pointer (directly, or a pointer
    /// to a function typedef, or a typedef of a function pointer).
    pub(crate) fn function_pointer(&self, node: &Value) -> bool {
        for text in [type_text(node), qual_text(node)] {
            if text.contains("(*)") {
                return true;
            }
            let bare = strip_qualifiers(text);
            if let Some(pointee) = bare.strip_suffix('*') {
                let pointee = strip_qualifiers(pointee);
                if self.function_typedefs.contains(pointee) && !pointee.contains('*') {
                    return true;
                }
            } else if self.pointer_typedefs.contains(bare) {
                return true;
            }
        }
        false
    }

    /// The record a type spelling names, through typedefs.
    fn record_of(&self, text: &str, pointee: bool) -> Option<String> {
        let mut bare = strip_qualifiers(text);
        if pointee {
            bare = strip_qualifiers(bare.strip_suffix('*')?);
            if bare.contains('*') {
                return None;
            }
        } else if bare.contains('*') {
            return None;
        }
        spelled_record(bare).or_else(|| self.record_typedefs.get(bare).cloned())
    }

    pub(crate) fn record_of_node(&self, node: &Value, pointee: bool) -> Option<String> {
        self.record_of(type_text(node), pointee)
            .or_else(|| self.record_of(qual_text(node), pointee))
    }

    /// `TAG.PATH` of a member expression, `?.NAME` when its record is not
    /// read.
    pub(crate) fn member_key(&self, member: &Value) -> String {
        let mut segments = Vec::new();
        let mut node = member;
        let tag = loop {
            let Some(name) = node["name"].as_str() else {
                break None;
            };
            segments.push(name.to_owned());
            let Some(base) = children(node).first() else {
                break None;
            };
            if node["isArrow"].as_bool() == Some(true) {
                break self.record_of_node(strip(base), true);
            }
            let base = strip(base);
            if kind(base) == "MemberExpr" {
                node = base;
                continue;
            }
            break self.record_of_node(base, false);
        };
        segments.reverse();
        match tag {
            Some(tag) => format!("{tag}.{}", segments.join(".")),
            None => format!("?.{}", segments.last().cloned().unwrap_or_default()),
        }
    }

    fn flows(&self, node: &Value, enclosing: Option<&Enclosing<'_>>) -> BTreeSet<Flow> {
        let node = strip(node);
        if let Some(name) = designator(node) {
            return BTreeSet::from([Flow::Function {
                name: name.to_owned(),
            }]);
        }
        if null_constant(node) {
            return BTreeSet::from([Flow::Null]);
        }
        match kind(node) {
            // Card OP-APP-RK-CALLBACKS-VIA-POLL-CB (Q3): keyed by its
            // innermost named record.
            "MemberExpr" => BTreeSet::from([Flow::Member {
                member: self.callback_key(node),
            }]),
            // A formal of another type (a `void *` a caller may hand any
            // function pointer through) is open: its calls record no actual.
            "DeclRefExpr"
                if node["referencedDecl"]["kind"].as_str() == Some("ParmVarDecl")
                    && !self.function_pointer(node) =>
            {
                BTreeSet::from([Flow::Open {
                    why: "a formal that is not a function pointer".to_owned(),
                }])
            }
            "DeclRefExpr" if node["referencedDecl"]["kind"].as_str() == Some("ParmVarDecl") => {
                let id = node["referencedDecl"]["id"].as_str().unwrap_or("");
                match enclosing.and_then(|enclosing| {
                    enclosing
                        .parameters
                        .iter()
                        .position(|parameter| *parameter == id)
                        .map(|index| (enclosing.name, index))
                }) {
                    Some((function, index)) => BTreeSet::from([Flow::Formal {
                        function: function.to_owned(),
                        index,
                    }]),
                    None => BTreeSet::from([Flow::Open {
                        why: "an unread formal".to_owned(),
                    }]),
                }
            }
            "ConditionalOperator" => {
                let arms = children(node);
                let mut out = BTreeSet::new();
                for arm in arms.iter().skip(1) {
                    out.extend(self.flows(arm, enclosing));
                }
                if arms.len() != 3 {
                    out.insert(Flow::Open {
                        why: "a conditional".to_owned(),
                    });
                }
                out
            }
            "DeclRefExpr" => BTreeSet::from([Flow::Open {
                why: "a variable".to_owned(),
            }]),
            other => BTreeSet::from([Flow::Open {
                why: format!("a `{other}`"),
            }]),
        }
    }

    /// Whether an initializer list holds a function-pointer value that is not
    /// null.
    fn initializes_function_pointer(&self, node: &Value) -> bool {
        children(node).iter().any(|child| {
            let inner = strip(child);
            if kind(inner) == "InitListExpr" {
                return self.initializes_function_pointer(inner);
            }
            designator(inner).is_some() || (self.function_pointer(child) && !null_constant(child))
        })
    }

    /// Card CALLBACK-MEMBER-DATA-ONLY-UNITS: whether an initializer entry for
    /// a field that is not a function pointer carries no callback: an implicit
    /// value, a string literal, a nested list whose entries carry none, or a
    /// scalar that is neither a function nor a function pointer. A record or
    /// array value (a copy that may carry function pointers) carries one.
    fn carries_no_callback(&self, node: &Value) -> bool {
        let inner = strip(node);
        match kind(inner) {
            "ImplicitValueInitExpr" | "StringLiteral" => true,
            "InitListExpr" => children(inner)
                .iter()
                .all(|child| self.carries_no_callback(child)),
            _ => {
                designator(inner).is_none()
                    && (null_constant(node) || !self.function_pointer(node))
                    && !self.function_pointer(inner)
                    && self.record_of_node(inner, false).is_none()
                    && !type_text(inner).contains('[')
            }
        }
    }

    /// Card CALLBACK-MEMBER-DATA-ONLY-UNITS (rule 12n): map an initializer
    /// list of `record` onto its fields: each function-pointer field's entry
    /// is a store of that value into `TAG.FIELD` (an implicit entry is a null
    /// pointer). `false`, recording nothing, when the list cannot be mapped
    /// exactly (the record's fields are unknown or ambiguous, the entries do
    /// not match them one for one, a union names no field, or another
    /// field's entry may carry a callback); the caller then opens the record.
    fn map_initializer(
        &mut self,
        record: &str,
        node: &Value,
        enclosing: Option<&Enclosing<'_>>,
    ) -> bool {
        let Some(Some(shape)) = self.record_fields.get(record) else {
            return false;
        };
        let entries = children(node);
        let pairs: Vec<(&FieldShape, &Value)> = if shape.union {
            let Some(name) = node["field"]["name"].as_str() else {
                return false;
            };
            let ([entry], Some(field)) = (
                entries,
                shape.fields.iter().find(|field| field.name == name),
            ) else {
                return false;
            };
            vec![(field, entry)]
        } else {
            if entries.len() != shape.fields.len() {
                return false;
            }
            shape.fields.iter().zip(entries).collect()
        };
        let mut stores = Vec::new();
        for (field, entry) in pairs {
            if field.function_pointer {
                stores.push((
                    format!("{record}.{}", field.name),
                    self.flows(entry, enclosing),
                ));
            } else if !self.carries_no_callback(entry) {
                return false;
            }
        }
        for (key, flows) in stores {
            self.stores.entry(key).or_default().extend(flows);
        }
        true
    }

    /// Card CALLBACK-MEMBER-DATA-ONLY-UNITS: a record definition's fields in
    /// initializer order, `None` when they cannot be mapped.
    fn record_shape(&self, node: &Value) -> Option<RecordShape> {
        let mut fields = Vec::new();
        for child in children(node) {
            if kind(child) != "FieldDecl" {
                continue;
            }
            match child["name"].as_str().filter(|name| !name.is_empty()) {
                // An unnamed bit-field takes no initializer.
                None if child["isBitfield"].as_bool() == Some(true) => {}
                // An anonymous structure or union member.
                None => return None,
                Some(name) => fields.push(FieldShape {
                    name: name.to_owned(),
                    function_pointer: self.function_pointer(child),
                }),
            }
        }
        Some(RecordShape {
            union: node["tagUsed"].as_str() == Some("union"),
            fields,
        })
    }

    /// Card CALLBACK-MEMBER-DATA-ONLY-UNITS: every file-scope record
    /// definition (nested ones included), each tag once.
    fn collect_records(&mut self, node: &Value) {
        if kind(node) != "RecordDecl" {
            return;
        }
        for child in children(node) {
            self.collect_records(child);
        }
        if node["completeDefinition"].as_bool() != Some(true) {
            return;
        }
        let Some(name) = node["name"].as_str().filter(|name| !name.is_empty()) else {
            return;
        };
        let shape = self.record_shape(node);
        let entry = self
            .record_fields
            .entry(name.to_owned())
            .or_insert(shape.clone());
        if *entry != shape {
            *entry = None;
        }
        // Card OP-APP-RK-CALLBACKS-VIA-POLL-CB (rule 12n, Q3): the record as
        // member keys read it.
        let keys = self.record_keys(node);
        if keys.union {
            self.unions.insert(name.to_owned());
        }
        let entry = self
            .records
            .entry(name.to_owned())
            .or_insert(Some(keys.clone()));
        if entry.as_ref() != Some(&keys) {
            *entry = None;
        }
    }

    /// Card OP-APP-RK-CALLBACKS-VIA-POLL-CB (rule 12n, Q3): a record
    /// definition's fields as member keys read them. An anonymous record a
    /// field names is read inline from the record declaration Clang puts just
    /// before it.
    fn record_keys(&self, node: &Value) -> RecordKeys {
        let mut keys = RecordKeys {
            union: node["tagUsed"].as_str() == Some("union"),
            fields: BTreeMap::new(),
        };
        let mut anonymous: Option<&Value> = None;
        for child in children(node) {
            match kind(child) {
                "RecordDecl" => {
                    anonymous = (child["name"].as_str().is_none_or(str::is_empty)
                        && child["completeDefinition"].as_bool() == Some(true))
                    .then_some(child);
                }
                "FieldDecl" => {
                    let name = child["name"].as_str().unwrap_or("").to_owned();
                    let text = type_text(child);
                    let field = if self.function_pointer(child) {
                        FieldKey::Function
                    } else if text.contains("(unnamed") || text.contains("(anonymous") {
                        match anonymous {
                            Some(record) if !text.contains('*') && !text.contains('[') => {
                                FieldKey::Inline {
                                    keys: Box::new(self.record_keys(record)),
                                }
                            }
                            _ => FieldKey::Plain,
                        }
                    } else if text.contains('*') {
                        FieldKey::Plain
                    } else {
                        // A record by value, or an array of them: a store
                        // through it is keyed by that record (`a[i].cb` reads
                        // its base's record).
                        let element = text.split('[').next().unwrap_or(text);
                        match spelled_record(strip_qualifiers(element)).or_else(|| {
                            self.record_typedefs.get(strip_qualifiers(element)).cloned()
                        }) {
                            Some(record) => FieldKey::Named { record },
                            None => FieldKey::Plain,
                        }
                    };
                    keys.fields.insert(name, field);
                    anonymous = None;
                }
                _ => {}
            }
        }
        keys
    }

    /// Card OP-APP-RK-CALLBACKS-VIA-POLL-CB: whether a node's type (or its
    /// pointee's) is a union.
    fn union_node(&self, node: &Value, pointee: bool) -> bool {
        [type_text(node), qual_text(node)].into_iter().any(|text| {
            let mut bare = strip_qualifiers(text);
            if pointee {
                match bare.strip_suffix('*') {
                    Some(inner) => bare = strip_qualifiers(inner),
                    None => return false,
                }
            }
            bare.starts_with("union ")
                || self
                    .record_typedefs
                    .get(bare)
                    .is_some_and(|record| self.unions.contains(record))
        })
    }

    /// Card **OP-APP-RK-CALLBACKS-VIA-POLL-CB** (gateway rule 12n, Q3): the
    /// key of a function-pointer member expression for the callback closure,
    /// by its **innermost named record**: `rk->rk_conf.dr_msg_cb` and
    /// `conf->dr_msg_cb` are both `rd_kafka_conf_s.dr_msg_cb`. A member read
    /// from a union object is keyed by the union with `*` (`T.u.*`, or
    /// `U.*` for a union record), so the union's members share their stores.
    /// `?.NAME` when the record is not read.
    pub(crate) fn callback_key(&self, member: &Value) -> String {
        let mut chain: Vec<(&str, &Value, bool)> = Vec::new();
        let mut node = member;
        loop {
            let Some(name) = node["name"].as_str() else {
                return "?.".to_owned();
            };
            let Some(base) = children(node).first() else {
                return format!("?.{name}");
            };
            let base = strip(base);
            let arrow = node["isArrow"].as_bool() == Some(true);
            chain.push((name, base, arrow));
            if !arrow && kind(base) == "MemberExpr" {
                node = base;
                continue;
            }
            break;
        }
        chain.reverse();
        let (first, root, arrow) = chain[0];
        let Some(mut tag) = self.record_of_node(root, arrow) else {
            let last = chain.last().map_or(first, |(name, _, _)| name);
            return format!("?.{last}");
        };
        if self.union_node(root, arrow) {
            return format!("{tag}.*");
        }
        let mut path: Vec<&str> = Vec::new();
        for (index, (name, base, _)) in chain.iter().enumerate() {
            if index > 0 {
                // `base` is the previous member's value: a record object.
                if self.union_node(base, false) {
                    // A named union is keyed by itself (`U.*`), as the
                    // tables normalise it; an anonymous one under its path.
                    if let Some(record) = self.record_of_node(base, false) {
                        return format!("{record}.*");
                    }
                    path.push("*");
                    return format!("{tag}.{}", path.join("."));
                }
                if let Some(record) = self.record_of_node(base, false) {
                    tag = record;
                    path.clear();
                }
            }
            path.push(name);
        }
        format!("{tag}.{}", path.join("."))
    }

    /// The value of the enumerator declared with `id`, as Clang read the
    /// unit (implicit values counted on from the previous enumerator).
    #[must_use]
    pub fn enumerator(&self, id: &str) -> Option<i64> {
        self.enumerators.get(id).copied()
    }

    /// Every enumerator's value by declaration id.
    #[must_use]
    pub fn enumerators(&self) -> &BTreeMap<String, i64> {
        &self.enumerators
    }

    fn collect_enumerators(&mut self, node: &Value) {
        if kind(node) == "EnumDecl" {
            let mut next = Some(0i64);
            for constant in children(node) {
                if kind(constant) != "EnumConstantDecl" {
                    continue;
                }
                let value = match children(constant).first() {
                    Some(init) => crate::arm_release_check::fold_with(init, &self.enumerators),
                    None => next,
                };
                if let (Some(value), Some(id)) = (value, constant["id"].as_str()) {
                    self.enumerators.insert(id.to_owned(), value);
                }
                next = value.and_then(|value| value.checked_add(1));
            }
        }
        for child in children(node) {
            if matches!(kind(child), "EnumDecl" | "TypedefDecl" | "RecordDecl") {
                self.collect_enumerators(child);
            }
        }
    }

    /// Read one top-level declaration.
    pub fn collect(&mut self, node: &Value) {
        self.collect_enumerators(node);
        self.collect_records(node);
        match kind(node) {
            "TypedefDecl" => {
                let Some(name) = node["name"].as_str() else {
                    return;
                };
                let text = qual_text(node);
                let desugared = type_text(node);
                if text.contains("(*)") || desugared.contains("(*)") {
                    self.pointer_typedefs.insert(name.to_owned());
                } else if text.contains('(') || desugared.contains('(') {
                    self.function_typedefs.insert(name.to_owned());
                } else if let Some(record) = spelled_record(strip_qualifiers(desugared))
                    .filter(|_| !desugared.contains('*'))
                    .or_else(|| {
                        let bare = strip_qualifiers(text);
                        (!bare.contains('*'))
                            .then(|| self.record_typedefs.get(bare).cloned())
                            .flatten()
                    })
                {
                    self.record_typedefs.insert(name.to_owned(), record);
                }
            }
            "FunctionDecl" => {
                let Some(name) = node["name"].as_str() else {
                    return;
                };
                let Some(body) = children(node)
                    .iter()
                    .find(|child| kind(child) == "CompoundStmt")
                else {
                    return;
                };
                if node["storageClass"].as_str() == Some("static") {
                    self.internal.insert(name.to_owned());
                }
                let parameters = children(node)
                    .iter()
                    .filter(|child| kind(child) == "ParmVarDecl")
                    .map(|child| child["id"].as_str().unwrap_or(""))
                    .collect();
                let enclosing = Enclosing { name, parameters };
                self.walk(body, Some(&enclosing));
            }
            _ => self.walk(node, None),
        }
    }

    fn walk(&mut self, node: &Value, enclosing: Option<&Enclosing<'_>>) {
        match kind(node) {
            "BinaryOperator" if node["opcode"].as_str() == Some("=") => {
                let [left, right] = children(node) else {
                    return;
                };
                let place = strip(left);
                if kind(place) == "MemberExpr" && self.function_pointer(place) {
                    let key = self.callback_key(place);
                    let flows = self.flows(right, enclosing);
                    self.stores.entry(key).or_default().extend(flows);
                } else if let Some(record) = self.record_of_node(place, false) {
                    // A whole-object assignment copies members of the same
                    // record (values already counted); anything else writes
                    // them opaquely.
                    let source = strip(right);
                    if self.record_of_node(source, false).as_deref() != Some(record.as_str())
                        || kind(source) == "CallExpr"
                    {
                        self.opened
                            .entry(record)
                            .or_insert_with(|| "a whole-object assignment".to_owned());
                    }
                }
                self.walk(left, enclosing);
                self.walk(right, enclosing);
            }
            "UnaryOperator" if node["opcode"].as_str() == Some("&") => {
                if let Some(operand) = children(node).first() {
                    let operand = strip(operand);
                    if kind(operand) == "MemberExpr" && self.function_pointer(operand) {
                        let key = self.callback_key(operand);
                        self.stores.entry(key).or_default().insert(Flow::Open {
                            why: "its address is taken".to_owned(),
                        });
                    }
                }
                for child in children(node) {
                    self.walk(child, enclosing);
                }
            }
            "CallExpr" => {
                let arguments = children(node);
                if let Some(callee) = direct_callee(node) {
                    for (index, argument) in arguments.iter().skip(1).enumerate() {
                        if self.function_pointer(argument) || designator(argument).is_some() {
                            let flows = self.flows(argument, enclosing);
                            self.calls
                                .entry(callee.to_owned())
                                .or_default()
                                .entry(index)
                                .or_default()
                                .extend(flows);
                        }
                    }
                    if COPIERS.contains(&callee)
                        && let Some(destination) = arguments.get(1)
                        && let Some(record) = self.record_of_node(strip(destination), true)
                    {
                        self.opened
                            .entry(record)
                            .or_insert_with(|| format!("a `{callee}` into it"));
                    }
                    // The callee designator is not an address taken.
                    for argument in arguments.iter().skip(1) {
                        self.walk(argument, enclosing);
                    }
                } else {
                    for child in arguments {
                        self.walk(child, enclosing);
                    }
                }
            }
            "DeclRefExpr" => {
                if node["referencedDecl"]["kind"].as_str() == Some("FunctionDecl")
                    && let Some(name) = node["referencedDecl"]["name"].as_str()
                {
                    self.address_taken.insert(name.to_owned());
                }
            }
            // Card CALLBACK-MEMBER-DATA-ONLY-UNITS: a record defined inside a
            // function may reuse a file-scope tag, so no list of that tag is
            // mapped onto fields.
            "RecordDecl" if enclosing.is_some() => {
                if let Some(name) = node["name"].as_str().filter(|name| !name.is_empty()) {
                    self.record_fields.insert(name.to_owned(), None);
                }
                for child in children(node) {
                    self.walk(child, enclosing);
                }
            }
            "InitListExpr" => {
                // Card CALLBACK-MEMBER-DATA-ONLY-UNITS: a list mapped onto its
                // record's fields stores each function-pointer entry;
                // otherwise the record is open, as before.
                if let Some(record) = self.record_of_node(node, false)
                    && self.initializes_function_pointer(node)
                    && !self.map_initializer(&record, node, enclosing)
                {
                    self.opened
                        .entry(record)
                        .or_insert_with(|| "an initializer list".to_owned());
                }
                for child in children(node) {
                    self.walk(child, enclosing);
                }
            }
            _ => {
                for child in children(node) {
                    self.walk(child, enclosing);
                }
            }
        }
    }
}

/// Card **CALLBACK-MEMBER-DATA-ONLY-UNITS** (rule 12n): one unit's facts
/// read from its whole Clang AST, for a unit the gateway does not otherwise
/// analyse (a data-only unit). `source` is the unit's preprocessed text and
/// `unit` gives its dialect and frontend flags: the input an analysed unit's
/// index is read from. Every top-level declaration is read with
/// [`MemberFacts::collect`], so a unit with no store into a member contributes
/// an empty set read from its AST. `Err` when Clang or the JSON fails, or the
/// document is not a translation unit with a declaration array; the caller
/// then records no facts, which opens every member.
pub fn read_unit(
    source: &str,
    unit: &crate::legacy_c::C11TranslationUnit<'_>,
) -> Result<MemberFacts, String> {
    crate::typed_c_ast::translation_unit_ast_with(source, unit, |bytes| {
        crate::deep_unit_ast::from_slice::<FactsRoot>(bytes)
    })
    .map(|FactsRoot(facts)| facts)
}

/// The translation unit, read for its member facts alone.
struct FactsRoot(MemberFacts);

/// The translation unit's declaration array.
struct FactsDeclarations(MemberFacts);

impl<'de> Deserialize<'de> for FactsDeclarations {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Declarations;
        impl<'de> serde::de::Visitor<'de> for Declarations {
            type Value = FactsDeclarations;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a Clang translation-unit declaration array")
            }

            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut seq: A,
            ) -> Result<Self::Value, A::Error> {
                let mut facts = MemberFacts::default();
                while let Some(node) = seq.next_element::<Value>()? {
                    facts.collect(&node);
                }
                Ok(FactsDeclarations(facts))
            }
        }
        // Anything but an array is refused (serde's default for the other
        // shapes): a unit whose declarations are not read records no facts.
        deserializer.deserialize_seq(Declarations)
    }
}

impl<'de> Deserialize<'de> for FactsRoot {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Root;
        impl<'de> serde::de::Visitor<'de> for Root {
            type Value = FactsRoot;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a Clang translation-unit AST object")
            }

            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut map: A,
            ) -> Result<Self::Value, A::Error> {
                let mut facts = None;
                let mut root_kind = Value::Null;
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        // A duplicate key replaces every earlier set, as the
                        // analysed unit's index reads it.
                        "inner" => facts = Some(map.next_value::<FactsDeclarations>()?.0),
                        "kind" => root_kind = map.next_value::<Value>()?,
                        _ => {
                            map.next_value::<serde::de::IgnoredAny>()?;
                        }
                    }
                }
                if root_kind.as_str() != Some("TranslationUnitDecl") {
                    return Err(serde::de::Error::custom(
                        "expected Clang TranslationUnitDecl",
                    ));
                }
                facts
                    .map(FactsRoot)
                    .ok_or_else(|| serde::de::Error::custom("no declaration array"))
            }
        }
        deserializer.deserialize_map(Root)
    }
}

/// The candidates of a member over a set of units: `Ok` the closed set of
/// designators, `Err` every reason it is open.
pub type Closure = Result<BTreeSet<String>, BTreeSet<String>>;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Node {
    Member(String),
    /// A formal: the unit when the function has internal linkage there.
    Formal(Option<usize>, String, usize),
}

fn record_of_key(member: &str) -> &str {
    member.split_once('.').map_or(member, |(tag, _)| tag)
}

fn last_segment(member: &str) -> &str {
    member.rsplit('.').next().unwrap_or(member)
}

/// Card **OP-APP-RK-CALLBACKS-VIA-POLL-CB** (gateway rule 12n, Q3): the
/// record tables of a set of units, every reading of each record tag (two
/// units may define one tag differently; a unit that defined it differently
/// twice gives no reading, so nothing is keyed through it).
#[derive(Clone, Debug, Default)]
pub struct Tables {
    records: BTreeMap<String, Vec<RecordKeys>>,
    ambiguous: BTreeSet<String>,
}

impl Tables {
    /// The tables of `units`.
    #[must_use]
    pub fn of(units: &[&MemberFacts]) -> Self {
        let mut tables = Self::default();
        for facts in units {
            for (tag, keys) in &facts.records {
                match keys {
                    Some(keys) => {
                        let readings = tables.records.entry(tag.clone()).or_default();
                        if !readings.contains(keys) {
                            readings.push(keys.clone());
                        }
                    }
                    None => {
                        tables.ambiguous.insert(tag.clone());
                    }
                }
            }
        }
        tables
    }

    fn walk(
        &self,
        tag: &str,
        keys: &RecordKeys,
        prefix: &mut Vec<String>,
        rest: &[&str],
        depth: usize,
        out: &mut BTreeSet<String>,
    ) -> Result<(), String> {
        if depth > 64 {
            return Err(format!("records nest too deeply below `{tag}`"));
        }
        let spell = |prefix: &[String], last: &str| {
            let mut path = prefix.to_vec();
            path.push(last.to_owned());
            format!("{tag}.{}", path.join("."))
        };
        if keys.union {
            out.insert(spell(prefix, "*"));
            return Ok(());
        }
        let Some((segment, more)) = rest.split_first() else {
            return Err(format!("no member path below `{tag}`"));
        };
        let Some(field) = keys.fields.get(*segment) else {
            return Err(format!("`{segment}` is not a member of `{tag}`"));
        };
        if more.is_empty() {
            return match field {
                FieldKey::Function => {
                    out.insert(spell(prefix, segment));
                    Ok(())
                }
                _ => Err(format!("`{tag}.{segment}` is not a function pointer")),
            };
        }
        match field {
            FieldKey::Named { record } => {
                self.walk_record(record, &mut Vec::new(), more, depth + 1, out)
            }
            FieldKey::Inline { keys } => {
                prefix.push((*segment).to_owned());
                let result = self.walk(tag, keys, prefix, more, depth + 1, out);
                prefix.pop();
                result
            }
            FieldKey::Function | FieldKey::Plain => {
                Err(format!("`{tag}.{segment}` is not a record by value"))
            }
        }
    }

    fn walk_record(
        &self,
        tag: &str,
        prefix: &mut Vec<String>,
        rest: &[&str],
        depth: usize,
        out: &mut BTreeSet<String>,
    ) -> Result<(), String> {
        if self.ambiguous.contains(tag) {
            return Err(format!("`{tag}` is defined differently within one unit"));
        }
        let readings = self
            .records
            .get(tag)
            .ok_or_else(|| format!("no unit defines `{tag}`"))?;
        for keys in readings {
            self.walk(tag, keys, prefix, rest, depth, out)?;
        }
        Ok(())
    }

    /// The normal forms of a member key `TAG.PATH`: keyed by its innermost
    /// named record, a union's members as `….*`; several where the units
    /// define a record differently. `Err` when the path is not a
    /// function-pointer member of records the units define.
    ///
    /// # Errors
    /// Why the path is not such a member.
    pub fn normalise(&self, key: &str) -> Result<BTreeSet<String>, String> {
        let (tag, path) = key
            .split_once('.')
            .ok_or_else(|| format!("`{key}` names no member"))?;
        let segments = path.split('.').collect::<Vec<_>>();
        if segments.last() == Some(&"*") {
            // Already a union's key.
            return Ok(BTreeSet::from([key.to_owned()]));
        }
        let mut out = BTreeSet::new();
        self.walk_record(tag, &mut Vec::new(), &segments, 0, &mut out)?;
        Ok(out)
    }

    /// The record keys at a union's normal key (`T.p.*`), every reading.
    fn union_at(&self, key: &str) -> Vec<RecordKeys> {
        let Some((tag, path)) = key.split_once('.') else {
            return Vec::new();
        };
        let segments = path.split('.').collect::<Vec<_>>();
        let Some((&"*", inner)) = segments.split_last() else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for keys in self.records.get(tag).into_iter().flatten() {
            let mut at = Some(keys);
            for segment in inner {
                at = match at.and_then(|keys| keys.fields.get(*segment)) {
                    Some(FieldKey::Inline { keys }) => Some(keys),
                    _ => None,
                };
            }
            if let Some(keys) = at {
                out.push(keys.clone());
            }
        }
        out
    }

    /// Every named record a record's fields hold by value, transitively.
    fn embedded(&self, keys: &RecordKeys) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        let mut pending = vec![keys.clone()];
        while let Some(keys) = pending.pop() {
            for field in keys.fields.values() {
                match field {
                    FieldKey::Named { record } => {
                        if out.insert(record.clone()) {
                            pending.extend(self.records.get(record).into_iter().flatten().cloned());
                        }
                    }
                    FieldKey::Inline { keys } => pending.push((**keys).clone()),
                    FieldKey::Function | FieldKey::Plain => {}
                }
            }
        }
        out
    }

    /// Whether a record holds a function pointer, directly or through
    /// records it holds by value (only such a record can carry a store).
    fn carries_function(&self, record: &str) -> bool {
        self.records
            .get(record)
            .into_iter()
            .flatten()
            .any(|keys| !self.names(keys).is_empty())
    }

    /// Every record that holds `record` by value, transitively.
    fn containers(&self, record: &str) -> BTreeSet<String> {
        self.records
            .iter()
            .filter(|(_, readings)| {
                readings
                    .iter()
                    .any(|keys| self.embedded(keys).contains(record))
            })
            .map(|(tag, _)| tag.clone())
            .collect()
    }

    /// Every function-pointer field name below a record's keys, through
    /// embedded records: the names a store into a record that is not read
    /// (`?.NAME`) may use, since only a function-pointer member is a store.
    fn names(&self, keys: &RecordKeys) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        let mut pending = vec![keys.clone()];
        let mut seen = BTreeSet::new();
        while let Some(keys) = pending.pop() {
            for (name, field) in &keys.fields {
                if *field == FieldKey::Function {
                    out.insert(name.clone());
                }
                match field {
                    FieldKey::Named { record } => {
                        if seen.insert(record.clone()) {
                            pending.extend(self.records.get(record).into_iter().flatten().cloned());
                        }
                    }
                    FieldKey::Inline { keys } => pending.push((**keys).clone()),
                    FieldKey::Function | FieldKey::Plain => {}
                }
            }
        }
        out
    }

    /// Whether a store keyed `store` may write the member keyed `member`
    /// (both normal): the same key; a union's key whose members embed the
    /// member's record; or the member is a union's and the store's record is
    /// embedded in it.
    fn aliases(&self, member: &str, store: &str) -> bool {
        if member == store {
            return true;
        }
        let embeds = |union: &str, record: &str| {
            self.union_at(union)
                .iter()
                .any(|keys| self.embedded(keys).contains(record))
        };
        (store.ends_with(".*") && embeds(store, record_of_key(member)))
            || (member.ends_with(".*") && embeds(member, record_of_key(store)))
    }
}

/// A node's direct contributions: designators, dependencies, why it is open.
type Contribution = (BTreeSet<String>, BTreeSet<Node>, BTreeSet<String>);

/// The units' stores by normal key, with the tables, built once for many
/// closures (the pre-pass's run).
#[derive(Clone, Debug, Default)]
pub struct Prepared {
    pub tables: Tables,
    /// `(unit, normal key, flows)`.
    stores: Vec<(usize, String, BTreeSet<Flow>)>,
}

impl Prepared {
    /// Index `units`.
    #[must_use]
    pub fn of(units: &[&MemberFacts]) -> Self {
        let tables = Tables::of(units);
        let mut stores = Vec::new();
        for (unit, facts) in units.iter().enumerate() {
            for (key, flows) in &facts.stores {
                let normal = if key.starts_with("?.") {
                    BTreeSet::from([key.clone()])
                } else {
                    tables
                        .normalise(key)
                        .unwrap_or_else(|_| BTreeSet::from([key.clone()]))
                };
                for normal in normal {
                    stores.push((unit, normal, flows.clone()));
                }
            }
        }
        Self { tables, stores }
    }
}

/// The closure of `member` (`TAG.PATH`) over `units`.
pub fn closure(units: &[&MemberFacts], member: &str) -> Closure {
    closure_prepared(units, &Prepared::of(units), member).0
}

/// Card **OP-APP-RK-CALLBACKS-VIA-POLL-CB**: the tokens the closure of
/// `member` over `units` reads (the record tags and member names of every
/// member node and of the records that hold or alias it, and every function
/// a formal node belongs to); the pre-pass reads every unit naming one.
#[must_use]
pub fn closure_tokens(units: &[&MemberFacts], member: &str) -> BTreeSet<String> {
    closure_prepared(units, &Prepared::of(units), member).1
}

/// [`closure`] over an index built once, with the tokens it read.
pub fn closure_prepared(
    units: &[&MemberFacts],
    prepared: &Prepared,
    member: &str,
) -> (Closure, BTreeSet<String>) {
    let tables = &prepared.tables;
    let mut tokens = BTreeSet::new();
    // The direct contributions of one node: designators, dependencies and
    // reasons it is open.
    let direct = |node: &Node, tokens: &mut BTreeSet<String>| -> Contribution {
        let mut names = BTreeSet::new();
        let mut dependencies = BTreeSet::new();
        let mut open = BTreeSet::new();
        let mut notes = BTreeSet::new();
        let mut take = |flows: &BTreeSet<Flow>, unit: usize| {
            for flow in flows {
                match flow {
                    Flow::Function { name } => {
                        names.insert(name.clone());
                    }
                    Flow::Null => {}
                    Flow::Member { member } => {
                        dependencies.insert(Node::Member(member.clone()));
                    }
                    Flow::Formal { function, index } => {
                        let scope = units[unit].internal.contains(function).then_some(unit);
                        dependencies.insert(Node::Formal(scope, function.clone(), *index));
                    }
                    Flow::Open { why } => {
                        open.insert(why.clone());
                    }
                }
            }
        };
        match node {
            Node::Member(member) => {
                if let Some(tag) = member.split('.').next() {
                    tokens.insert(tag.to_owned());
                }
                // Card OP-APP-RK-CALLBACKS-VIA-POLL-CB (Q3): every normal
                // form, the records that hold it (a whole-object write of one
                // writes it), and the names an unread record's store may use.
                let normalised = tables.normalise(member).ok();
                let targets = normalised
                    .clone()
                    .unwrap_or_else(|| BTreeSet::from([member.clone()]));
                let mut records = BTreeSet::new();
                let mut unread_names = BTreeSet::new();
                for target in &targets {
                    tokens.insert(record_of_key(target).to_owned());
                    let record = record_of_key(target).to_owned();
                    records.extend(tables.containers(&record));
                    records.insert(record);
                    if target.ends_with(".*") {
                        for keys in tables.union_at(target) {
                            records.extend(tables.embedded(&keys));
                            unread_names.extend(tables.names(&keys));
                        }
                    } else {
                        unread_names.insert(last_segment(target).to_owned());
                    }
                }
                let aliased =
                    |store: &str| targets.iter().any(|target| tables.aliases(target, store));
                for (_, store, _) in &prepared.stores {
                    if store.ends_with(".*") && aliased(store) {
                        for keys in tables.union_at(store) {
                            unread_names.extend(tables.names(&keys));
                        }
                    }
                }
                // The tokens a unit must name to hold a store the closure
                // reads: a record that can carry one (a whole-object write of
                // a holder of the member writes it), and a function-pointer
                // name an unread record's store may use.
                tokens.extend(
                    records
                        .iter()
                        .filter(|record| tables.carries_function(record))
                        .cloned(),
                );
                // A member keyed by named records is written only by a unit
                // that holds their definitions, which name the tags; the
                // names an unread record's store may use are tokens only
                // where the member is not keyed so.
                if normalised.is_none() {
                    tokens.extend(unread_names.iter().cloned());
                }
                for facts in units {
                    for record in &records {
                        if let Some(why) = facts.opened.get(record) {
                            notes.insert(format!("{why} writes a `struct {record}`"));
                        }
                    }
                }
                for (unit, store, flows) in &prepared.stores {
                    if let Some(name) = store.strip_prefix("?.") {
                        if unread_names.contains(name) {
                            notes.insert(format!("a store into `{name}` whose record is not read"));
                            take(flows, *unit);
                        }
                    } else if aliased(store) {
                        take(flows, *unit);
                    }
                }
            }
            Node::Formal(scope, function, index) => {
                tokens.insert(function.clone());
                let reached = |unit: usize| match scope {
                    Some(own) => unit == *own,
                    None => !units[unit].internal.contains(function),
                };
                for (unit, facts) in units.iter().enumerate() {
                    if !reached(unit) {
                        continue;
                    }
                    if facts.address_taken.contains(function) {
                        notes.insert(format!("`{function}`'s address is taken"));
                    }
                    if let Some(flows) = facts.calls.get(function).and_then(|at| at.get(index)) {
                        take(flows, unit);
                    }
                }
            }
        }
        open.extend(notes);
        (names, dependencies, open)
    };
    // Discover the reachable graph, then iterate to the least fixed point.
    let mut graph: BTreeMap<Node, Contribution> = BTreeMap::new();
    let mut pending = vec![Node::Member(member.to_owned())];
    while let Some(node) = pending.pop() {
        if graph.contains_key(&node) {
            continue;
        }
        let entry = direct(&node, &mut tokens);
        pending.extend(entry.1.iter().cloned());
        graph.insert(node, entry);
    }
    let mut values: BTreeMap<Node, (BTreeSet<String>, BTreeSet<String>)> = graph
        .iter()
        .map(|(node, (names, _, open))| (node.clone(), (names.clone(), open.clone())))
        .collect();
    loop {
        let mut changed = false;
        for (node, (_, dependencies, _)) in &graph {
            let mut names = values[node].0.clone();
            let mut open = values[node].1.clone();
            for dependency in dependencies {
                names.extend(values[dependency].0.iter().cloned());
                open.extend(values[dependency].1.iter().cloned());
            }
            if (names.len(), open.len()) != (values[node].0.len(), values[node].1.len()) {
                values.insert(node.clone(), (names, open));
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    let (names, open) = values
        .remove(&Node::Member(member.to_owned()))
        .unwrap_or_default();
    let result = if open.is_empty() {
        Ok(names)
    } else {
        Err(open)
    };
    (result, tokens)
}

/// `CLOSURE_VECTORS` (`QCore.CallbackContracts.closureVectors`): two units'
/// facts in a tiny spelling and the closure of `T.f`: `Some(names)` closed,
/// `None` open. Each fact is `(unit, key, flow)`: a store into `key`, or, for
/// `call:F:i`, an actual of `F` at `i`; `addr:F` marks `F` address-taken and
/// `static:F` internal.
#[cfg(test)]
#[allow(clippy::type_complexity)]
pub(crate) const CLOSURE_VECTORS: &[(&[(usize, &str, &str)], Option<&[&str]>)] = &[
    (&[(0, "T.f", "fn:a"), (1, "T.f", "fn:b")], Some(&["a", "b"])),
    (&[(0, "T.f", "null")], Some(&[])),
    (&[(0, "T.f", "open")], None),
    (
        &[(0, "T.f", "formal:reg:0"), (1, "call:reg:0", "fn:c")],
        Some(&["c"]),
    ),
    (
        &[
            (0, "T.f", "formal:reg:0"),
            (1, "call:reg:0", "fn:c"),
            (1, "addr:reg", ""),
        ],
        None,
    ),
    (
        &[
            (0, "T.f", "formal:reg:0"),
            (0, "static:reg", ""),
            (0, "call:reg:0", "fn:d"),
            (1, "call:reg:0", "fn:e"),
        ],
        Some(&["d"]),
    ),
    (
        &[
            (0, "T.f", "member:U.g"),
            (1, "U.g", "fn:h"),
            (0, "U.g", "member:T.f"),
        ],
        Some(&["h"]),
    ),
    (&[(0, "T.f", "fn:a"), (1, "opened:T", "")], None),
];

#[cfg(test)]
pub(crate) fn vector_units(facts: &[(usize, &str, &str)]) -> Vec<MemberFacts> {
    let mut units = vec![MemberFacts::default(), MemberFacts::default()];
    let flow = |text: &str| -> Flow {
        let parts = text.split(':').collect::<Vec<_>>();
        match parts.as_slice() {
            ["fn", name] => Flow::Function {
                name: (*name).to_owned(),
            },
            ["null"] => Flow::Null,
            ["member", member] => Flow::Member {
                member: (*member).to_owned(),
            },
            ["formal", function, index] => Flow::Formal {
                function: (*function).to_owned(),
                index: index.parse().unwrap(),
            },
            _ => Flow::Open {
                why: "open".to_owned(),
            },
        }
    };
    for (unit, key, value) in facts {
        let facts = &mut units[*unit];
        let parts = key.split(':').collect::<Vec<_>>();
        match parts.as_slice() {
            ["call", function, index] => {
                facts
                    .calls
                    .entry((*function).to_owned())
                    .or_default()
                    .entry(index.parse().unwrap())
                    .or_default()
                    .insert(flow(value));
            }
            ["addr", function] => {
                facts.address_taken.insert((*function).to_owned());
            }
            ["static", function] => {
                facts.internal.insert((*function).to_owned());
            }
            ["opened", record] => {
                facts
                    .opened
                    .insert((*record).to_owned(), "opened".to_owned());
            }
            _ => {
                facts
                    .stores
                    .entry((*key).to_owned())
                    .or_default()
                    .insert(flow(value));
            }
        }
    }
    units
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `KEY_VECTORS` (`QCore.AppRkCallbacks.keyVectors`): a record table
    /// (`(tag, field, record)`: the field holds that record by value), a key
    /// and its normal form. Every other field is a function pointer; an
    /// anonymous record (`sasl`, `oauthbearer`) is inline.
    #[allow(clippy::type_complexity)]
    const KEY_VECTORS: &[(&[(&str, &str, &str)], &str, &str)] = &[
        (
            &[("rd_kafka_s", "rk_conf", "rd_kafka_conf_s")],
            "rd_kafka_s.rk_conf.dr_msg_cb",
            "rd_kafka_conf_s.dr_msg_cb",
        ),
        (
            &[("rd_kafka_s", "rk_conf", "rd_kafka_conf_s")],
            "rd_kafka_conf_s.dr_msg_cb",
            "rd_kafka_conf_s.dr_msg_cb",
        ),
        (
            &[],
            "rd_kafka_conf_s.sasl.oauthbearer.token_refresh_cb",
            "rd_kafka_conf_s.sasl.oauthbearer.token_refresh_cb",
        ),
        (&[("a", "b", "c"), ("c", "d", "e")], "a.b.d.f", "e.f"),
    ];

    fn vector_tables(rows: &[(&str, &str, &str)], key: &str) -> MemberFacts {
        // Every record the key's path names, each field a function pointer
        // unless a row says it holds a record.
        let mut facts = MemberFacts::default();
        let mut tags = rows
            .iter()
            .flat_map(|(t, _, r)| [*t, *r])
            .collect::<Vec<_>>();
        tags.push(key.split('.').next().unwrap());
        for tag in tags {
            let mut keys = RecordKeys::default();
            for segment in key.split('.').skip(1) {
                keys.fields
                    .entry(segment.to_owned())
                    .or_insert(FieldKey::Function);
            }
            for (t, field, record) in rows {
                if *t == tag {
                    keys.fields.insert(
                        (*field).to_owned(),
                        FieldKey::Named {
                            record: (*record).to_owned(),
                        },
                    );
                }
            }
            facts.records.insert(tag.to_owned(), Some(keys));
        }
        // The anonymous records of the third vector.
        if key.contains(".sasl.") {
            let inner = |name: &str, field: FieldKey| RecordKeys {
                union: false,
                fields: BTreeMap::from([(name.to_owned(), field)]),
            };
            let oauth = inner("token_refresh_cb", FieldKey::Function);
            let sasl = inner(
                "oauthbearer",
                FieldKey::Inline {
                    keys: Box::new(oauth),
                },
            );
            facts.records.insert(
                "rd_kafka_conf_s".into(),
                Some(inner(
                    "sasl",
                    FieldKey::Inline {
                        keys: Box::new(sasl),
                    },
                )),
            );
        }
        facts
    }

    /// A development aid: the tables of one preprocessed unit
    /// (`QCORE_DEBUG_UNIT`), the containers and embedded records of
    /// `QCORE_DEBUG_RECORD`.
    #[test]
    #[ignore = "a development aid over a local preprocessed unit"]
    fn debug_tables() {
        let path = std::env::var("QCORE_DEBUG_UNIT").expect("QCORE_DEBUG_UNIT");
        let record = std::env::var("QCORE_DEBUG_RECORD").expect("QCORE_DEBUG_RECORD");
        let text = std::fs::read_to_string(path).unwrap();
        let facts = read_unit(
            &text,
            &crate::legacy_c::C11TranslationUnit {
                source: &text,
                dialect: crate::legacy_c::CCompilerDialect::Gnu11,
                ..Default::default()
            },
        )
        .unwrap();
        let tables = Tables::of(&[&facts]);
        println!("containers {:?}", tables.containers(&record));
        for keys in tables.records.get(&record).into_iter().flatten() {
            println!("embedded {:?}", tables.embedded(keys));
        }
        for (tag, readings) in &tables.records {
            for keys in readings {
                if tables.embedded(keys).contains(&record) {
                    let direct = keys
                        .fields
                        .iter()
                        .filter(|(_, field)| {
                            matches!(field, FieldKey::Named { .. } | FieldKey::Inline { .. })
                        })
                        .collect::<Vec<_>>();
                    println!("{tag} embeds via {direct:?}");
                }
            }
        }
    }

    #[test]
    fn key_vectors_hold() {
        for (rows, key, normal) in KEY_VECTORS {
            let facts = vector_tables(rows, key);
            assert_eq!(
                Tables::of(&[&facts]).normalise(key),
                Ok(BTreeSet::from([(*normal).to_owned()])),
                "{key}"
            );
        }
        assert_eq!(KEY_VECTORS.len(), 4);
    }

    #[test]
    fn closure_vectors_hold() {
        for (facts, expected) in CLOSURE_VECTORS {
            let units = vector_units(facts);
            let views = units.iter().collect::<Vec<_>>();
            let got = closure(&views, "T.f").ok();
            let expected = expected.map(|names| {
                names
                    .iter()
                    .map(|name| (*name).to_owned())
                    .collect::<BTreeSet<_>>()
            });
            assert_eq!(got, expected, "{facts:?}");
        }
        assert_eq!(CLOSURE_VECTORS.len(), 8);
    }

    #[test]
    fn a_unit_sees_a_subset_and_the_run_may_open_it() {
        // Unit 0 alone closes over `a`; the run adds unit 1's `b`.
        let units = vector_units(&[(0, "T.f", "fn:a"), (1, "T.f", "fn:b")]);
        assert_eq!(
            closure(&[&units[0]], "T.f"),
            Ok(BTreeSet::from(["a".to_owned()]))
        );
        let run = closure(&[&units[0], &units[1]], "T.f").unwrap();
        assert!(!run.is_subset(&BTreeSet::from(["a".to_owned()])));
    }

    fn ast(text: &str) -> Value {
        serde_json::from_str(text).unwrap()
    }

    #[test]
    fn facts_read_member_stores_formals_and_calls() {
        let mut facts = MemberFacts::default();
        facts.collect(&ast(
            r#"{"kind":"TypedefDecl","name":"cb_t","type":{"qualType":"int (void *)"}}"#,
        ));
        facts.collect(&ast(
            r#"{"kind":"TypedefDecl","name":"op_t","type":{"desugaredQualType":"struct op_s","qualType":"struct op_s"}}"#,
        ));
        // op_t *new_cb(cb_t *cb) { rko->op_cb = cb; }
        facts.collect(&ast(
            r#"{"kind":"FunctionDecl","name":"new_cb","inner":[
                {"kind":"ParmVarDecl","id":"0x1","name":"cb","type":{"qualType":"cb_t *"}},
                {"kind":"CompoundStmt","inner":[
                  {"kind":"BinaryOperator","opcode":"=","type":{"qualType":"cb_t *"},"inner":[
                    {"kind":"MemberExpr","name":"op_cb","isArrow":true,"type":{"qualType":"cb_t *"},"inner":[
                      {"kind":"ImplicitCastExpr","castKind":"LValueToRValue","type":{"qualType":"op_t *"},"inner":[
                        {"kind":"DeclRefExpr","type":{"qualType":"op_t *"},"referencedDecl":{"kind":"VarDecl","id":"0x9","name":"rko"}}]}]},
                    {"kind":"ImplicitCastExpr","castKind":"LValueToRValue","type":{"qualType":"cb_t *"},"inner":[
                      {"kind":"DeclRefExpr","type":{"qualType":"cb_t *"},"referencedDecl":{"kind":"ParmVarDecl","id":"0x1","name":"cb"}}]}]},
                  {"kind":"CallExpr","type":{"qualType":"op_t *"},"inner":[
                    {"kind":"ImplicitCastExpr","castKind":"FunctionToPointerDecay","inner":[
                      {"kind":"DeclRefExpr","referencedDecl":{"kind":"FunctionDecl","name":"new_cb"}}]},
                    {"kind":"ImplicitCastExpr","castKind":"FunctionToPointerDecay","type":{"qualType":"int (*)(void *)"},"inner":[
                      {"kind":"DeclRefExpr","referencedDecl":{"kind":"FunctionDecl","name":"handler"}}]}]}]}]}"#,
        ));
        assert_eq!(
            facts.stores["op_s.op_cb"],
            BTreeSet::from([Flow::Formal {
                function: "new_cb".into(),
                index: 0
            }])
        );
        assert_eq!(
            facts.calls["new_cb"][&0],
            BTreeSet::from([Flow::Function {
                name: "handler".into()
            }])
        );
        assert!(facts.address_taken.contains("handler"));
        assert!(!facts.address_taken.contains("new_cb"));
        assert_eq!(
            closure(&[&facts], "op_s.op_cb"),
            Ok(BTreeSet::from(["handler".to_owned()]))
        );
    }
}
