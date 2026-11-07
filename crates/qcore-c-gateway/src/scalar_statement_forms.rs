//! Card **SCALAR-STATEMENT-FORMS** (round 26 E, gateway rule 41): statements
//! whose every value Clang types as a non-pointer scalar, read from the Clang
//! AST after every older route of the statement refused.
//!
//! ## What was refused
//!
//! Five lexical refusals end statements no older route reads: `unsupported
//! assignment expression`, `unrecognized assignment destination`, `braced
//! initializer, expression, or declaration is not modeled`, `unmodeled
//! increment or decrement expression` and ``C operator `sizeof` requires
//! type/operand analysis``. The card's census (the classifier's own records,
//! `docs/reports/scalar-statement-forms/`) found that most of them are pointer
//! steps, pointer stores and pointer arithmetic, which stay refused, and that
//! the scalar residue has four causes: the lexical declaration reader answers
//! nothing for a function it cannot read (so `len -= 8192*3` or `k++` on a
//! declared `size_t` is refused, and a `for` clause never reaches the
//! elaborator), a declaration with more than one declarator, a declaration
//! that declares only a type (`typedef struct { … } t_a;`, a static assertion's
//! `enum { X = 1/(int)(!!(C)) }`), and a `sizeof` of an expression operand.
//!
//! ## The forms ([`Form`])
//!
//! * **Steps and compound assignments** of a scalar object: `X++`, `--X`,
//!   `X op= E`. C11 6.5.3.1p2 / 6.5.2.4p2: `++X` is `X += 1` and `X++` yields
//!   the old value with the same side effect; 6.5.16.2p3: `X op= E` is `X = X
//!   op (E)` with `X` evaluated once. So each is the plain store `X = X op E`
//!   of the typed statement reader (card W46-ACCESS-PATH-STATEMENTS), whose
//!   lowering it gets: a named object's write forgets its scalar fact
//!   (`ForgetScalar`), a path's read and write carry the reader's per-hop
//!   obligations (`IndirectScalarAccess`).
//! * **Plain assignments and single declarations** whose value only needed
//!   rule 41's reading (an unevaluated `sizeof`, or a named object the lexical
//!   reader did not read).
//! * **Several declarators** `T a = E1, b;`: C11 6.7.6p3 places a sequence
//!   point after each full declarator, so the statement is its declarators in
//!   order, each a single declaration.
//! * **Type declarations**: a `DeclStmt` holding only `typedef`, `struct`,
//!   `union` and `enum` declarations declares no object (C11 6.7p5 creates
//!   storage only for an object) and evaluates nothing: an enumerator's value
//!   is an integer constant expression (6.7.2.2p2), a member's type is never
//!   variably modified (6.7.2.1p9). A `typedef` of a variably modified type
//!   evaluates its size expressions (6.7.8p3) and is refused.
//!
//! A destination is admitted only when Clang types it as a non-pointer
//! builtin scalar or enumeration (never `volatile` or `_Atomic`), and, for a
//! named object, when it has automatic storage in this function, every
//! declaration of its name in the function has that class (the projection
//! names objects by spelling), it is not a protected formal, and it names no
//! tracked descriptor. A value is admitted only when the typed reader admits
//! it: scalar operators over scalar operands, typed paths with their per-hop
//! obligations, and (rule 41's one addition to the reader) a `sizeof` whose
//! operand Clang does not evaluate (C11 6.5.3.4p2).
//!
//! ## What stays refused, by name ([`Refusal`])
//!
//! A pointer destination (a pointer step or a pointer compound assignment is
//! W43-INDEX-TRAVERSAL's), pointer arithmetic or any pointer-valued operand in
//! the value, a `volatile` object (rule 11h), an aggregate destination or an
//! object declared with an inline record (rule 11f's), a variably modified
//! type or `sizeof`, a static or thread-duration object, a name declared with
//! another class, a protected formal, and a value the reader refuses. A
//! declined statement keeps its older refusal byte for byte. The Lean model
//! is `QCore.ScalarStatementForms`; its decision vectors are [`VECTORS`].

use serde_json::Value;

use crate::typed_access_path::{is_pointer, is_scalar, qual};
use crate::typed_c_ast::{children, kind};

/// The statement forms the rule reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Form {
    /// `X++`, `X--`, `++X`, `--X`.
    Step,
    /// `X op= E`.
    Compound,
    /// `X = E` whose value only rule 41 reads.
    Assignment,
    /// `T X = E;` or `T X;`, one declarator.
    Declaration,
    /// `T X = E, Y;`: every declarator a [`Form::Declaration`].
    Declarators,
    /// A `DeclStmt` of type declarations only.
    TypeDeclaration,
}

/// The class Clang gives the statement's destination.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Destination {
    /// A named automatic scalar object of this function.
    ScalarObject,
    /// A scalar place the typed reader reads (`*B`, `B[I]`, `B->F`, `S.F`).
    ScalarPath,
    /// No object at all (a type declaration).
    NoObject,
    Pointer,
    Volatile,
    Aggregate,
    /// A named object whose name another declaration in the function gives
    /// another class.
    Shadowed,
    /// A static- or thread-duration object.
    Static,
    /// A formal the function's entry custody protects.
    ProtectedFormal,
    /// A variably modified type (a `typedef` whose size expressions run).
    VariablyModified,
    /// A declaration of an object whose type the statement defines inline.
    InlineRecordObject,
    /// Anything else (no unique node, a place the reader refuses).
    Unread,
}

/// The class of the statement's value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ValueClass {
    /// No value (a step reads its own destination; an uninitialized
    /// declarator; a type declaration).
    None,
    /// A value the typed reader admits.
    Scalar,
    /// A pointer-valued operand or pointer arithmetic.
    Pointer,
    /// A `sizeof` whose operand Clang evaluates.
    EvaluatedQuery,
    /// Refused by the reader for another cause (a call, an address, a
    /// conditional, an assignment inside the value).
    Unread,
}

/// A refusal of the rule, by name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Refusal {
    PointerDestination,
    PointerValue,
    Volatile,
    Aggregate,
    InlineRecordObject,
    VariablyModified,
    StaticStorage,
    ShadowedName,
    ProtectedFormal,
    UnreadDestination,
    UnreadValue,
    /// A form whose destination class it never has (a type declaration with
    /// an object, a declaration of a path).
    Mismatched,
}

impl Refusal {
    pub(crate) fn reason(self) -> &'static str {
        match self {
            Self::PointerDestination => "pointer-destination",
            Self::PointerValue => "pointer-value",
            Self::Volatile => "volatile-object",
            Self::Aggregate => "aggregate-destination",
            Self::InlineRecordObject => "inline-record-object",
            Self::VariablyModified => "variably-modified",
            Self::StaticStorage => "static-storage",
            Self::ShadowedName => "shadowed-name",
            Self::ProtectedFormal => "protected-formal",
            Self::UnreadDestination => "unread-destination",
            Self::UnreadValue => "unread-value",
            Self::Mismatched => "mismatched-form",
        }
    }
}

/// The rule's decision on one declarator or statement shape
/// (`QCore.ScalarStatementForms.decide`).
pub(crate) fn decide(
    form: Form,
    destination: Destination,
    value: ValueClass,
) -> Result<(), Refusal> {
    let destination = match destination {
        Destination::NoObject => {
            return match (form, value) {
                (Form::TypeDeclaration, ValueClass::None) => Ok(()),
                _ => Err(Refusal::Mismatched),
            };
        }
        Destination::Pointer => return Err(Refusal::PointerDestination),
        Destination::Volatile => return Err(Refusal::Volatile),
        Destination::Aggregate => return Err(Refusal::Aggregate),
        Destination::InlineRecordObject => return Err(Refusal::InlineRecordObject),
        Destination::VariablyModified => return Err(Refusal::VariablyModified),
        Destination::Static => return Err(Refusal::StaticStorage),
        Destination::Shadowed => return Err(Refusal::ShadowedName),
        Destination::ProtectedFormal => return Err(Refusal::ProtectedFormal),
        Destination::Unread => return Err(Refusal::UnreadDestination),
        scalar @ (Destination::ScalarObject | Destination::ScalarPath) => scalar,
    };
    match (form, destination) {
        (Form::TypeDeclaration, _) => return Err(Refusal::Mismatched),
        (Form::Declaration | Form::Declarators, Destination::ScalarPath) => {
            return Err(Refusal::Mismatched);
        }
        _ => {}
    }
    match value {
        ValueClass::None | ValueClass::Scalar => Ok(()),
        ValueClass::Pointer => Err(Refusal::PointerValue),
        ValueClass::EvaluatedQuery => Err(Refusal::VariablyModified),
        ValueClass::Unread => Err(Refusal::UnreadValue),
    }
}

/// The shared decision vectors (`QCore.ScalarStatementForms.vectors`):
/// form, destination, value, and the reason (`""` for an admission).
#[cfg(test)]
pub(crate) const VECTORS: &[(Form, Destination, ValueClass, &str)] = &[
    (Form::Step, Destination::ScalarObject, ValueClass::None, ""),
    (Form::Step, Destination::ScalarPath, ValueClass::None, ""),
    (
        Form::Step,
        Destination::Pointer,
        ValueClass::None,
        "pointer-destination",
    ),
    (
        Form::Step,
        Destination::Volatile,
        ValueClass::None,
        "volatile-object",
    ),
    (
        Form::Step,
        Destination::Shadowed,
        ValueClass::None,
        "shadowed-name",
    ),
    (
        Form::Step,
        Destination::Static,
        ValueClass::None,
        "static-storage",
    ),
    (
        Form::Step,
        Destination::ProtectedFormal,
        ValueClass::None,
        "protected-formal",
    ),
    (
        Form::Compound,
        Destination::ScalarObject,
        ValueClass::Scalar,
        "",
    ),
    (
        Form::Compound,
        Destination::ScalarPath,
        ValueClass::Scalar,
        "",
    ),
    (
        Form::Compound,
        Destination::Pointer,
        ValueClass::Scalar,
        "pointer-destination",
    ),
    (
        Form::Compound,
        Destination::ScalarObject,
        ValueClass::Pointer,
        "pointer-value",
    ),
    (
        Form::Compound,
        Destination::ScalarObject,
        ValueClass::Unread,
        "unread-value",
    ),
    (
        Form::Assignment,
        Destination::ScalarObject,
        ValueClass::Scalar,
        "",
    ),
    (
        Form::Assignment,
        Destination::ScalarObject,
        ValueClass::Pointer,
        "pointer-value",
    ),
    (
        Form::Assignment,
        Destination::ScalarObject,
        ValueClass::EvaluatedQuery,
        "variably-modified",
    ),
    (
        Form::Declaration,
        Destination::ScalarObject,
        ValueClass::Scalar,
        "",
    ),
    (
        Form::Declaration,
        Destination::ScalarObject,
        ValueClass::None,
        "",
    ),
    (
        Form::Declaration,
        Destination::Aggregate,
        ValueClass::Scalar,
        "aggregate-destination",
    ),
    (
        Form::Declaration,
        Destination::ScalarPath,
        ValueClass::Scalar,
        "mismatched-form",
    ),
    (
        Form::Declaration,
        Destination::Volatile,
        ValueClass::Scalar,
        "volatile-object",
    ),
    (
        Form::Declarators,
        Destination::ScalarObject,
        ValueClass::Scalar,
        "",
    ),
    (
        Form::Declarators,
        Destination::Pointer,
        ValueClass::None,
        "pointer-destination",
    ),
    (
        Form::Declarators,
        Destination::InlineRecordObject,
        ValueClass::None,
        "inline-record-object",
    ),
    (
        Form::TypeDeclaration,
        Destination::NoObject,
        ValueClass::None,
        "",
    ),
    (
        Form::TypeDeclaration,
        Destination::VariablyModified,
        ValueClass::None,
        "variably-modified",
    ),
    (
        Form::TypeDeclaration,
        Destination::InlineRecordObject,
        ValueClass::None,
        "inline-record-object",
    ),
    (
        Form::TypeDeclaration,
        Destination::ScalarObject,
        ValueClass::None,
        "mismatched-form",
    ),
    (
        Form::Step,
        Destination::NoObject,
        ValueClass::None,
        "mismatched-form",
    ),
    (
        Form::Step,
        Destination::Unread,
        ValueClass::None,
        "unread-destination",
    ),
];

// ---------------------------------------------------------------------------
// Reading Clang's node
// ---------------------------------------------------------------------------

/// The statement node's form and its pieces.
pub(crate) enum Shape<'n> {
    /// A step or compound assignment: the place, and the value (`None` for a
    /// step, whose value is its own place).
    Update {
        form: Form,
        place: &'n Value,
        value: Option<&'n Value>,
    },
    /// `X = E`.
    Assignment { place: &'n Value, value: &'n Value },
    /// The declarators of a `DeclStmt` (one or more `VarDecl`s).
    Declarators(Vec<&'n Value>),
    /// A `DeclStmt` holding only type declarations.
    TypeDeclaration(&'n Value),
}

/// The node kinds a statement of this rule is matched to.
pub(crate) fn candidate(node: &Value) -> bool {
    match kind(node) {
        "DeclStmt" | "CompoundAssignOperator" => true,
        "UnaryOperator" => matches!(node["opcode"].as_str(), Some("++" | "--")),
        "BinaryOperator" => node["opcode"].as_str() == Some("="),
        _ => false,
    }
}

/// The shape of one matched node, `None` for any other node.
pub(crate) fn shape(node: &Value) -> Option<Shape<'_>> {
    match kind(node) {
        "UnaryOperator" if matches!(node["opcode"].as_str(), Some("++" | "--")) => {
            let [place] = children(node) else { return None };
            Some(Shape::Update {
                form: Form::Step,
                place,
                value: None,
            })
        }
        "CompoundAssignOperator" => {
            let [place, value] = children(node) else {
                return None;
            };
            Some(Shape::Update {
                form: Form::Compound,
                place,
                value: Some(value),
            })
        }
        "BinaryOperator" if node["opcode"].as_str() == Some("=") => {
            let [place, value] = children(node) else {
                return None;
            };
            Some(Shape::Assignment { place, value })
        }
        "DeclStmt" => {
            let declarations = children(node);
            if declarations.is_empty() {
                return None;
            }
            if declarations.iter().all(|declaration| {
                matches!(kind(declaration), "TypedefDecl" | "RecordDecl" | "EnumDecl")
            }) {
                return Some(Shape::TypeDeclaration(node));
            }
            Some(Shape::Declarators(declarations.iter().collect()))
        }
        _ => None,
    }
}

/// Whether a type spelling is volatile or atomic anywhere.
fn qualified(node: &Value) -> bool {
    ["qualType", "desugaredQualType"].iter().any(|key| {
        node["type"][key]
            .as_str()
            .is_some_and(|text| text.contains("volatile") || text.contains("_Atomic"))
    })
}

/// Whether a type spelling (either of Clang's) is variably modified.
fn variably_modified(ty: &Value) -> bool {
    ["qualType", "desugaredQualType"].iter().any(|key| {
        ty[key]
            .as_str()
            .is_some_and(crate::cast_representation::variably_modified)
    })
}

/// The class of a type declaration statement: [`Destination::NoObject`];
/// [`Destination::VariablyModified`] for a `typedef` whose size expressions
/// run; [`Destination::Shadowed`] when a `typedef` or an enumerator it
/// declares has the spelling of an object the function declares or names
/// (the projection names objects by spelling, so an enumerator `p` would be
/// read as the pointer `p` in the statements after it).
pub(crate) fn type_declaration_class(node: &Value, function: &Value) -> Destination {
    if children(node).iter().any(|declaration| {
        kind(declaration) == "TypedefDecl" && variably_modified(&declaration["type"])
    }) {
        return Destination::VariablyModified;
    }
    fn declared(node: &Value, out: &mut Vec<String>) {
        if matches!(kind(node), "TypedefDecl" | "EnumConstantDecl")
            && let Some(name) = node["name"].as_str()
        {
            out.push(name.to_owned());
        }
        for child in children(node) {
            declared(child, out);
        }
    }
    fn objects(node: &Value, names: &[String], found: &mut bool) {
        let named = match kind(node) {
            "VarDecl" | "ParmVarDecl" | "FunctionDecl" => node["name"].as_str(),
            "DeclRefExpr"
                if node["referencedDecl"]["kind"].as_str() != Some("EnumConstantDecl") =>
            {
                node["referencedDecl"]["name"].as_str()
            }
            _ => None,
        };
        if named.is_some_and(|name| names.iter().any(|declared| declared == name)) {
            *found = true;
            return;
        }
        for child in children(node) {
            objects(child, names, found);
        }
    }
    let mut names = Vec::new();
    declared(node, &mut names);
    let mut found = false;
    objects(function, &names, &mut found);
    if found {
        Destination::Shadowed
    } else {
        Destination::NoObject
    }
}

/// The scalar class of a value node's own type.
pub(crate) fn scalar_class(
    node: &Value,
    enum_scalar: &dyn Fn(&Value) -> bool,
) -> Option<Destination> {
    if qualified(node) {
        return Some(Destination::Volatile);
    }
    if is_pointer(node) || qual(node).is_some_and(|text| text.trim_end().ends_with('*')) {
        return Some(Destination::Pointer);
    }
    if is_scalar(node) || enum_scalar(node) {
        return None;
    }
    Some(Destination::Aggregate)
}

/// Every `VarDecl`/`ParmVarDecl` named `name` in `function`, and whether a
/// `RecordDecl` or `EnumDecl` the function defines precedes one of them in
/// the same `DeclStmt` (an object of an inline type).
fn declarations_named<'f>(function: &'f Value, name: &str, out: &mut Vec<&'f Value>) {
    if matches!(kind(function), "VarDecl" | "ParmVarDecl")
        && function["name"].as_str() == Some(name)
    {
        out.push(function);
    }
    for child in children(function) {
        declarations_named(child, name, out);
    }
}

/// The class of a named object written by the statement: its declaration
/// `decl` (the node's `referencedDecl` or the declarator itself), read in the
/// function's AST.
pub(crate) fn object_class(
    function: &Value,
    decl_id: &str,
    name: &str,
    enum_scalar: &dyn Fn(&Value) -> bool,
) -> Destination {
    let mut named = Vec::new();
    declarations_named(function, name, &mut named);
    let Some(own) = named
        .iter()
        .find(|declaration| declaration["id"].as_str() == Some(decl_id))
    else {
        // Not declared in this function: a file-scope object.
        return Destination::Static;
    };
    if kind(own) == "VarDecl"
        && crate::storage_duration::storage_of(own, true)
            .is_none_or(crate::storage_duration::Storage::outlives_frame)
    {
        return Destination::Static;
    }
    if let Some(class) = scalar_class(own, enum_scalar) {
        return class;
    }
    if named.iter().any(|declaration| {
        scalar_class(declaration, enum_scalar).is_some()
            || (kind(declaration) == "VarDecl"
                && crate::storage_duration::storage_of(declaration, true)
                    .is_none_or(crate::storage_duration::Storage::outlives_frame))
    }) {
        return Destination::Shadowed;
    }
    Destination::ScalarObject
}

/// The class of a value the reader refused, named for the census: a
/// pointer operand or pointer arithmetic, an evaluated `sizeof`, or another
/// cause.
pub(crate) fn refused_value_class(node: &Value) -> ValueClass {
    fn walk(node: &Value, class: &mut ValueClass) {
        match kind(node) {
            "UnaryExprOrTypeTraitExpr" => {
                if crate::cast_representation::query_operand(node)
                    != crate::cast_representation::QueryOperand::Unevaluated
                {
                    *class = ValueClass::EvaluatedQuery;
                }
                return;
            }
            "DeclRefExpr" | "MemberExpr" | "ArraySubscriptExpr" | "BinaryOperator"
                if *class == ValueClass::Unread
                    && (is_pointer(node)
                        || qual(node).is_some_and(|text| text.trim_end().ends_with('*'))) =>
            {
                *class = ValueClass::Pointer;
            }
            _ => {}
        }
        for child in children(node) {
            walk(child, class);
        }
    }
    let mut class = ValueClass::Unread;
    walk(node, &mut class);
    class
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gateway_rules::test_override::with_rules_off;
    use crate::legacy_c::{
        C11TranslationUnit, LegacyDiagnosticKind, UnrestrictedLegacyResult,
        analyze_unrestricted_c11,
    };

    /// The status and unsupported descriptions of `w`.
    fn outcome(body: &str) -> (&'static str, Vec<String>) {
        let contracts = qcore_contract::contract_loader::load_resource_contract_sources(
            &std::collections::BTreeMap::from([(
                "rule41.qcore".to_owned(),
                "module contract;\nextern fn x_unused(p: ptr<mut, u8>) -> unit\n\
                 contract { requires resource_contract_v1; ensures normal resource_borrowed(p);\n\
                 ensures normal resource_return_none(); };"
                    .to_owned(),
            )]),
        )
        .expect("the contract loads");
        let result = analyze_unrestricted_c11(
            &C11TranslationUnit {
                source: body,
                ..Default::default()
            },
            "w",
            &contracts,
        )
        .expect("the fixture parses");
        let (status, summary) = match &result {
            UnrestrictedLegacyResult::Safe(certificate) => ("Safe", &certificate.summary),
            UnrestrictedLegacyResult::Unsafe(summary) => ("Unsafe", summary),
            UnrestrictedLegacyResult::Unknown {
                partial: Some(summary),
                ..
            } => ("Unknown", summary),
            other => panic!("no projection: {other:?}"),
        };
        let mut keys = summary.functions["w"]
            .diagnostics
            .iter()
            .filter_map(|diagnostic| match &diagnostic.kind {
                LegacyDiagnosticKind::Unsupported { description } => Some(description.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        keys.sort();
        keys.dedup();
        (status, keys)
    }

    /// Rule 41 on admits the forms; off, every refusal stands byte for byte.
    #[test]
    fn off_every_refusal_stands() {
        if crate::clang_command().is_err() {
            return;
        }
        let body = "typedef unsigned long x_size; \
            int w(unsigned long long pattern) { x_size of = 0, r; \
            typedef struct { char c; long t; } t_a; \
            x_size const low = sizeof(pattern) - 1; r = of + low; \
            return (int) (r + sizeof(t_a)); }";
        assert_eq!(outcome(body), ("Safe", Vec::new()));
        with_rules_off("41", || {
            assert_eq!(
                outcome(body),
                (
                    "Unknown",
                    vec![
                        "C operator `sizeof` requires type/operand analysis".to_owned(),
                        "braced initializer, expression, or declaration is not modeled".to_owned(),
                        "unsupported assignment expression".to_owned(),
                    ]
                )
            );
        });
    }

    #[test]
    fn the_decision_vectors_hold() {
        for (form, destination, value, reason) in VECTORS {
            let decided = decide(*form, *destination, *value).map_or_else(Refusal::reason, |()| "");
            assert_eq!(decided, *reason, "{form:?} {destination:?} {value:?}");
        }
    }
}
