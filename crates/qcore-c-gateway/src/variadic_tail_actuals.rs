//! Card **CG26-TAIL-RESIDUAL-ACTUALS** (gateway rule 12g): a trailing actual
//! of a declared variadic tail read from Clang's own node, for the actuals
//! rule 11w's token premises (T1-T8) leave refused.
//!
//! Rule 11w reads each trailing actual from the statement's tokens, against
//! the lexical declaration reader's facts (`FunctionTypes`). Where that reader
//! refuses a function (an anonymous `struct { … } a[] = …`, a function-pointer
//! parameter or array: any declaration it cannot read), it answers nothing, so
//! T4 (a declared scalar), T5 (a typed chain) and T8 (a plain name under a
//! borrowed tail) all refuse, and every tail actual but a literal (T6) keeps
//! O04-TRACE's refusal. This module reads the same actual from the call's
//! Clang argument node instead, which carries the type Clang gave it.
//!
//! The reading is one recursion ([`classify`]) over an abstract expression
//! ([`TailExpr`], built from the node by [`TailExpr::from_node`]) into three
//! classes:
//!
//! * **data** — a value that carries no tracked identity: a literal, a
//!   predefined identifier, an enumeration constant, a `sizeof`/`_Alignof` of
//!   a constant-size type (unevaluated, C11 6.5.3.4p2), a name whose answer is
//!   "carries nothing", and a unary arithmetic operator, a binary arithmetic,
//!   bitwise, shift, comparison or logical operator, or a conditional `?:`
//!   **every** operand of which is data;
//! * **identity** — exactly one name that may carry a tracked identity, read
//!   through parentheses and implicit conversions only;
//! * **refused**, by name ([`TailRefusal`]).
//!
//! What a name carries is an answer the adapter supplies, never derived here:
//! an arithmetic or enumeration object carries an identity only when the
//! projection may bind it to an integer-carried one (a descriptor name), and
//! a pointer or array object carries one unless rule 10l's served never-bound
//! premise holds for it (the lexical closure where the lexical reader read the
//! function, the Clang closure joined with the lexical one where it did not).
//!
//! [`read`] turns a class into a tail reading: data is data; an identity is a
//! `Borrowed` place of that name (rule 11w's T8, read from the node) only
//! under `resource_variadic_borrowed_tail()`, only for a pointer or array
//! object (an integer-carried identity refuses), and only when rule 10l
//! answers the name (declared exactly once in the function, as a formal or an
//! automatic local, no reference naming another object of that spelling), so
//! the flow's liveness and alias checks apply to the object the actual
//! denotes. Under the data tail an identity keeps O04-TRACE's refusal: this
//! never widens the data tail (`QCore.VariadicTailActuals`).
//!
//! Refused by name: an explicit cast anywhere in the actual (the card
//! CAST-FORMALIZATION reads casts; none is read here), pointer arithmetic
//! (its root is not a place the projection identifies), a conditional with an
//! identity-carrying arm, an identity-carrying operand of an operator or of a
//! condition, an integer-carried identity, an unanswered root, and every
//! other node (an address, a dereference, a member, a subscript, a call, an
//! assignment, a comma, a compound literal, a statement expression, a GNU
//! `?:`, a variably modified `sizeof`).

use serde_json::Value;

use crate::typed_c_ast::{children, kind};

/// Why a trailing actual stays refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TailRefusal {
    /// An explicit cast: left to the card CAST-FORMALIZATION.
    Cast,
    /// Pointer arithmetic: its root is not a place the projection identifies.
    PointerArithmetic,
    /// A conditional one of whose arms may carry a tracked identity.
    IdentityArm,
    /// An operator operand or a condition that may carry a tracked identity.
    IdentityOperand,
    /// An identity under the data tail, which admits data only.
    IdentityUnderDataTail,
    /// An arithmetic object the projection may bind to an integer-carried
    /// identity.
    IntegerIdentity,
    /// A pointer name rule 10l does not answer (shadowed, foreign, static).
    UnansweredRoot,
    /// Any other node.
    Unread,
}

impl TailRefusal {
    /// The parity row's name of the refusal.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Cast => "cast",
            Self::PointerArithmetic => "pointer-arithmetic",
            Self::IdentityArm => "identity-arm",
            Self::IdentityOperand => "identity-operand",
            Self::IdentityUnderDataTail => "identity-under-data-tail",
            Self::IntegerIdentity => "integer-identity",
            Self::UnansweredRoot => "unanswered-root",
            Self::Unread => "unread",
        }
    }
}

/// One trailing actual's reading.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum TailReading {
    /// Carries no tracked identity: no place, nothing demanded.
    Data,
    /// A `Borrowed` place of exactly this name.
    Borrowed(String),
    Refused(TailRefusal),
}

impl TailReading {
    /// The parity row's spelling.
    pub(crate) fn describe(&self) -> String {
        match self {
            Self::Data => "data".to_owned(),
            Self::Borrowed(name) => format!("borrowed:{name}"),
            Self::Refused(refusal) => format!("refused:{}", refusal.name()),
        }
    }
}

/// The abstract shape of a trailing actual (`QCore.VariadicTailActuals.Expr`).
/// Parentheses and implicit conversions are transparent: Clang inserts the
/// latter, and neither changes which object a value comes from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum TailExpr {
    /// A literal, a predefined identifier, an enumeration constant or a
    /// constant-size `sizeof`/`_Alignof`.
    Literal,
    /// A formal or variable; `scalar` when its type is arithmetic or an
    /// enumeration, otherwise a pointer or an array.
    Name { name: String, scalar: bool },
    /// `-`, `+`, `~` or `!`.
    Unary(Box<TailExpr>),
    /// An arithmetic, bitwise, shift, comparison or logical operator;
    /// `pointer` when its result is a pointer.
    Binary {
        pointer: bool,
        left: Box<TailExpr>,
        right: Box<TailExpr>,
    },
    /// `c ? a : b`.
    Conditional(Box<TailExpr>, Box<TailExpr>, Box<TailExpr>),
    /// An explicit cast (CAST-FORMALIZATION's).
    Cast(Box<TailExpr>),
    /// Every other node.
    Other,
}

const UNARY: &[&str] = &["-", "+", "~", "!"];
const BINARY: &[&str] = &[
    "*", "/", "%", "+", "-", "<<", ">>", "&", "^", "|", "<", ">", "<=", ">=", "==", "!=", "&&",
    "||",
];

/// The type text Clang gave a node.
fn type_text(node: &Value) -> &str {
    node["type"]["desugaredQualType"]
        .as_str()
        .or_else(|| node["type"]["qualType"].as_str())
        .unwrap_or("")
        .trim()
}

/// An arithmetic or enumeration type, from Clang's own spelling.
fn arithmetic(text: &str) -> bool {
    if text.starts_with("enum ") && !text.contains(['*', '[', '(']) {
        return true;
    }
    let mut base = false;
    for word in text.split_whitespace() {
        match word {
            "const" | "volatile" => {}
            "signed" | "unsigned" | "short" | "long" | "char" | "int" | "float" | "double"
            | "_Bool" | "bool" | "__int128" => base = true,
            _ => return false,
        }
    }
    base
}

/// An object pointer or an array of constant or unknown size (which decays
/// to one), never a function pointer.
fn pointer_or_array(text: &str) -> bool {
    !text.contains('(') && (text.ends_with('*') || text.ends_with(']') || text.ends_with("* const"))
}

/// Whether every `[N]` of a type text is a decimal constant: a `sizeof` of a
/// variably modified type evaluates its operand (C11 6.5.3.4p2).
fn constant_size(text: &str) -> bool {
    text.split('[').skip(1).all(|part| {
        part.split_once(']').is_some_and(|(size, _)| {
            !size.trim().is_empty() && size.trim().bytes().all(|b| b.is_ascii_digit())
        })
    })
}

impl TailExpr {
    /// The abstract shape of a Clang expression node.
    pub(crate) fn from_node(node: &Value) -> Self {
        match kind(node) {
            "ParenExpr" | "ImplicitCastExpr" => match children(node) {
                [inner] => Self::from_node(inner),
                _ => Self::Other,
            },
            "IntegerLiteral" | "CharacterLiteral" | "FloatingLiteral" | "StringLiteral"
            | "PredefinedExpr" => Self::Literal,
            "UnaryExprOrTypeTraitExpr" => {
                let operand = node["argType"]["desugaredQualType"]
                    .as_str()
                    .or_else(|| node["argType"]["qualType"].as_str())
                    .or_else(|| children(node).first().map(type_text))
                    .unwrap_or("[]");
                if matches!(
                    node["name"].as_str(),
                    Some("sizeof" | "alignof" | "_Alignof")
                ) && constant_size(operand)
                {
                    Self::Literal
                } else {
                    Self::Other
                }
            }
            "DeclRefExpr" => {
                let declaration = &node["referencedDecl"];
                match declaration["kind"].as_str() {
                    Some("EnumConstantDecl") => Self::Literal,
                    Some("VarDecl" | "ParmVarDecl") => {
                        let Some(name) = declaration["name"].as_str() else {
                            return Self::Other;
                        };
                        let text = type_text(node);
                        if arithmetic(text) {
                            Self::Name {
                                name: name.to_owned(),
                                scalar: true,
                            }
                        } else if pointer_or_array(text) {
                            Self::Name {
                                name: name.to_owned(),
                                scalar: false,
                            }
                        } else {
                            Self::Other
                        }
                    }
                    _ => Self::Other,
                }
            }
            "UnaryOperator"
                if node["opcode"]
                    .as_str()
                    .is_some_and(|op| UNARY.contains(&op))
                    && node["isPostfix"].as_bool() != Some(true) =>
            {
                match children(node) {
                    [inner] => Self::Unary(Box::new(Self::from_node(inner))),
                    _ => Self::Other,
                }
            }
            "BinaryOperator"
                if node["opcode"]
                    .as_str()
                    .is_some_and(|op| BINARY.contains(&op)) =>
            {
                match children(node) {
                    [left, right] => Self::Binary {
                        pointer: !arithmetic(type_text(node)),
                        left: Box::new(Self::from_node(left)),
                        right: Box::new(Self::from_node(right)),
                    },
                    _ => Self::Other,
                }
            }
            "ConditionalOperator" => match children(node) {
                [condition, then, otherwise] => Self::Conditional(
                    Box::new(Self::from_node(condition)),
                    Box::new(Self::from_node(then)),
                    Box::new(Self::from_node(otherwise)),
                ),
                _ => Self::Other,
            },
            "CStyleCastExpr" => match children(node) {
                [inner] => Self::Cast(Box::new(Self::from_node(inner))),
                _ => Self::Cast(Box::new(Self::Other)),
            },
            _ => Self::Other,
        }
    }
}

/// What one name carries, answered by the adapter.
pub(crate) struct NameFacts<'f> {
    /// Whether the projection may bind this name to a tracked identity: for
    /// an arithmetic object an integer-carried one, for a pointer or array
    /// object unless rule 10l's never-bound premise holds.
    pub(crate) carries: &'f dyn Fn(&str, bool) -> bool,
    /// Rule 10l: the name is declared exactly once, as a formal or an
    /// automatic local, and no reference names another object of it.
    pub(crate) answered: &'f dyn Fn(&str) -> bool,
}

/// The class of an actual (`QCore.VariadicTailActuals.classify`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum TailClass {
    Data,
    Identity { name: String, scalar: bool },
    Refused(TailRefusal),
}

/// The one recursion. Operands are classified left to right, and the first
/// refusal is the answer; an operator's own refusal comes after its operands'.
pub(crate) fn classify(expression: &TailExpr, facts: &NameFacts<'_>) -> TailClass {
    use TailClass::{Data, Identity, Refused};
    match expression {
        TailExpr::Literal => Data,
        TailExpr::Name { name, scalar } => {
            if (facts.carries)(name, *scalar) {
                Identity {
                    name: name.clone(),
                    scalar: *scalar,
                }
            } else {
                Data
            }
        }
        TailExpr::Unary(inner) => match classify(inner, facts) {
            Data => Data,
            Identity { .. } => Refused(TailRefusal::IdentityOperand),
            refused => refused,
        },
        TailExpr::Binary {
            pointer,
            left,
            right,
        } => {
            let left = classify(left, facts);
            let right = classify(right, facts);
            match (left, right) {
                (Refused(refusal), _) | (_, Refused(refusal)) => Refused(refusal),
                _ if *pointer => Refused(TailRefusal::PointerArithmetic),
                (Data, Data) => Data,
                _ => Refused(TailRefusal::IdentityOperand),
            }
        }
        TailExpr::Conditional(condition, then, otherwise) => {
            let condition = classify(condition, facts);
            let then = classify(then, facts);
            let otherwise = classify(otherwise, facts);
            match (condition, then, otherwise) {
                (Refused(refusal), _, _) | (_, Refused(refusal), _) | (_, _, Refused(refusal)) => {
                    Refused(refusal)
                }
                (Identity { .. }, _, _) => Refused(TailRefusal::IdentityOperand),
                (Data, Data, Data) => Data,
                _ => Refused(TailRefusal::IdentityArm),
            }
        }
        TailExpr::Cast(_) => Refused(TailRefusal::Cast),
        TailExpr::Other => Refused(TailRefusal::Unread),
    }
}

/// The tail reading (`QCore.VariadicTailActuals.read`).
pub(crate) fn read(
    expression: &TailExpr,
    facts: &NameFacts<'_>,
    borrowed_tail: bool,
) -> TailReading {
    match classify(expression, facts) {
        TailClass::Data => TailReading::Data,
        TailClass::Refused(refusal) => TailReading::Refused(refusal),
        TailClass::Identity { scalar: true, .. } => {
            TailReading::Refused(TailRefusal::IntegerIdentity)
        }
        TailClass::Identity { .. } if !borrowed_tail => {
            TailReading::Refused(TailRefusal::IdentityUnderDataTail)
        }
        TailClass::Identity { name, .. } if (facts.answered)(&name) => TailReading::Borrowed(name),
        TailClass::Identity { .. } => TailReading::Refused(TailRefusal::UnansweredRoot),
    }
}

/// The shared vectors (`QCore.VariadicTailActuals.readVectors`, row by row):
/// the expression and the expected reading in Lean's spelling, the names
/// that carry an identity and the names rule 10l answers (name `n` is `vn`),
/// and whether the tail is borrowed.
#[cfg(test)]
pub(crate) const READ_VECTORS: &[(&str, &[usize], &[usize], bool, &str)] = &[
    (".lit", &[], &[], false, ".data"),
    ("(.name 1 true)", &[], &[], false, ".data"),
    (
        "(.name 1 true)",
        &[1],
        &[1],
        true,
        "(.refused .integerIdentity)",
    ),
    ("(.name 1 false)", &[], &[], false, ".data"),
    ("(.name 1 false)", &[1], &[1], true, "(.borrowed 1)"),
    (
        "(.name 1 false)",
        &[1],
        &[1],
        false,
        "(.refused .identityUnderDataTail)",
    ),
    (
        "(.name 1 false)",
        &[1],
        &[],
        true,
        "(.refused .unansweredRoot)",
    ),
    (
        "(.binary false (.name 1 true) .lit)",
        &[],
        &[],
        true,
        ".data",
    ),
    (
        "(.binary false (.name 1 false) .lit)",
        &[1],
        &[1],
        true,
        "(.refused .identityOperand)",
    ),
    (
        "(.binary true (.name 1 false) .lit)",
        &[],
        &[1],
        true,
        "(.refused .pointerArithmetic)",
    ),
    (
        "(.binary true (.name 1 false) .lit)",
        &[1],
        &[1],
        true,
        "(.refused .pointerArithmetic)",
    ),
    (
        "(.binary true (.cast (.name 1 false)) .lit)",
        &[1],
        &[1],
        true,
        "(.refused .cast)",
    ),
    ("(.unary (.name 1 true))", &[], &[], false, ".data"),
    (
        "(.unary (.name 1 false))",
        &[1],
        &[1],
        true,
        "(.refused .identityOperand)",
    ),
    ("(.cond (.name 1 true) .lit .lit)", &[], &[], false, ".data"),
    (
        "(.cond (.name 1 false) (.name 1 false) .lit)",
        &[],
        &[1],
        true,
        ".data",
    ),
    (
        "(.cond (.name 1 false) (.name 1 false) .lit)",
        &[1],
        &[1],
        true,
        "(.refused .identityOperand)",
    ),
    (
        "(.cond .lit (.name 1 false) .lit)",
        &[1],
        &[1],
        true,
        "(.refused .identityArm)",
    ),
    (
        "(.cond .lit .lit (.name 2 false))",
        &[2],
        &[2],
        true,
        "(.refused .identityArm)",
    ),
    ("(.cast (.name 1 true))", &[], &[], true, "(.refused .cast)"),
    (
        "(.binary false (.cast (.name 1 true)) .lit)",
        &[],
        &[],
        true,
        "(.refused .cast)",
    ),
    (
        "(.cast (.name 1 false))",
        &[1],
        &[1],
        true,
        "(.refused .cast)",
    ),
    (".other", &[], &[], true, "(.refused .unread)"),
    (
        "(.binary false .other (.cast .lit))",
        &[],
        &[],
        true,
        "(.refused .unread)",
    ),
];

#[cfg(test)]
#[path = "../tests/unit/variadic_tail_actuals.rs"]
mod tests;
