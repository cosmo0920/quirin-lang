//! Card **W44-INDIRECT-CALL-ARGUMENT-ACCESS**: typed indirect reads in the
//! actuals of a contracted call.
//!
//! The lexical indirect-access gate of [`super::Parser::classify_simple_base`]
//! refuses a statement as soon as one of its tokens outside an admitted span
//! reads memory (`->`, `.`, `[`, a unary `*`). Card W43-INDIRECT-ACCESS-GENERAL
//! admitted typed scalar accesses in statements **without** a call; every
//! statement carrying one kept the refusal (`W43-INDIRECT-CALL-ARGUMENT-ACCESS`).
//! The existing call routes admit only narrow actuals: C02-P's access chains
//! through a base the body never binds, O04-READ-BOUND's bound reads,
//! O04-B09B10's member addresses, NEIGHBOURS-CHAINED-TAIL's tail chains and
//! CONSTRUCTOR-RESIDUALS' dereferenced nested results.
//!
//! This module admits one more family of **whole actuals**, each a *read*:
//!
//! * a **scalar value** carrying at least one typed scalar access — the value
//!   walker of the typed scalar route, unchanged
//!   ([`crate::typed_field_access::typed_scalar_value`]): `f(p->len)`,
//!   `f(a[i] + 1)`, `f(*n)`, `f(s.count)`;
//! * a **typed pointer read**: `B[I]` or `*B` with `B` declared `T ** B` at
//!   every declaration and `I` a scalar value with no access
//!   (`strdup(src[i])`); `B->F` with `B` declared `struct T *` and `F` a
//!   pointer member of a completely read, single-definition, non-union
//!   `struct T`; `S.F` with `S` declared `struct T` and `F` such a member;
//! * a **typed sub-object address**: `&B->F` or `&S.F` with `F` declared with
//!   no `*`, or `&B[I]` over a scalar array or pointer (`mtx_lock(&rk->lock)`,
//!   `snprintf(&buf[n], …)`), under O04-B09B10's premise that no field link is
//!   reachable through such an address.
//!
//! A `sizeof` whose operand is a type name is an integer constant to the scalar
//! walker ([`fold_type_sizes`]); `sizeof (*v)` is never folded.
//!
//! ## Premises
//!
//! 1. **Position.** The actual sits at an argument index the contract in force
//!    declares [`ResourceParameterMode::Borrowed`], or beyond the declared
//!    parameter vector of a callee whose declaration carries a variadic data
//!    tail. Every custody-taking position (`Owned`, `Consumed`, `Released`,
//!    `Out`, `Stored`, `Removed`) keeps the refusal: the callee would take
//!    custody of an object the projection cannot name. An uncontracted callee
//!    keeps the refusal too.
//! 2. **Only what the gate refused.** The rule is consulted only for a simple
//!    statement the gate refused (`indirect memory access is outside the
//!    resource projection`) and that reads no tracked descriptor. The
//!    statement is classified again with the admitted actuals leaving the gate
//!    exactly as a C02-P borrowed actual does; the rule applies only when that
//!    classification is a contracted call (or an element store around one).
//!    Anything else keeps the original refusal byte for byte, and the site
//!    counter the probe used is restored. An actual an existing route already
//!    exempts keeps that route's semantics and carries no new obligation.
//! 3. **The obligation.** The call is preceded, at the same line, by the typed
//!    read of W43-INDIRECT-ACCESS-GENERAL over every base the admitted actuals
//!    dereference (`IndirectScalarAccess` with no destination and no frame):
//!    each base must be unbound on every reaching path, a non-null binding
//!    with no tracked target, or exactly one known, non-null, live identity,
//!    else `UnknownAlias`; a base settled on a released identity is a
//!    `use_after_release` candidate. C evaluates every actual before the call
//!    (C11 6.5.2.2p10), so the reads are checked before the call's effects.
//! 4. **The second obligation** (`LegacyInstruction::ArgumentReads`,
//!    [`refused_reads`]), after the typed read and before the call: a name the
//!    actuals read that is bound to an identity carried by an integer refuses
//!    (a tracked descriptor is custody, never data); a pointer-valued actual
//!    read through a base whose target owns sub-custody (a field link, a
//!    contained child, an indexed collection) refuses, because the pointer read
//!    may be a tracked object the read does not name; and a pointer-valued
//!    actual whose own place (`p->f`) is bound refuses. Neither read node
//!    changes any map, so the call sees the state it saw before this card.
//!
//! Bounds, alignment and the liveness of objects the projection does not track
//! are outside the property, exactly as for the typed scalar route.
//!
//! ## Cell-bearing calls (card MK-EVENT-CELL-ARRAY-SLOTS, gateway rule 38a)
//!
//! At a withheld declaration that carries a descriptor-cell state
//! (§C04.6.2), two more things hold.
//!
//! * **The element actual.** At a `Borrowed` position, a constant-index
//!   element of an integer array member is recognised as the member read of
//!   its base ([`element_read`]): `X->f[K]`, `S.f[K]`, `&X->f[K]` or
//!   `&S.f[K]`, with `K` a canonical decimal literal below the declared
//!   length. It is asked only after every older route refused.
//! * **The minted name.** The probe's admitted actuals are also found under
//!   rule 12t's minted name for the callee (`AdmittedArgument::matches`).
//!   Rule 12t classifies the call again under that name. Without the match,
//!   an actual admitted here left the gate only in the outer classification,
//!   and the inner one kept the refusal.
//!
//! Rule 38 then decides the cell. With rule 38 switched off, the call refuses
//! as every cell call does.
//!
//! ## Every withheld declaration (card RETAINING-CALL-MINTED-NAME-ACTUALS, gateway rule 38b)
//!
//! The minted-name match is not limited to cell-bearing declarations. At
//! every withheld declaration (rule 12t), an actual the probe admitted at a
//! declared `Borrowed` position is also found under the minted name, for its
//! own callee only ([`minted_admission`]). A rule-1 release actual is not:
//! its position takes custody. The element actual stays limited to
//! cell-bearing declarations. After classification, every actual that left
//! the gate under the minted name must sit at a `Borrowed` position of the
//! call in force ([`minted_positions_borrowed`]); rule 12t's route reads a
//! callback's requirement as `resource_owned` after the inner pass, and such
//! a call keeps the refusal.
//!
//! ## A declared cell's close (card CELL-PROTOCOL-CLOSE-ACTUAL, gateway rule 38d)
//!
//! Premise 1 has one exception, asked only after the statement's whole
//! lowering refused at the gate ([`Parser::cell_close_actual`]): a statement
//! that is exactly rule 38c's `close(r->CELL);` of a declared protocol
//! record's one-segment cell, whose callee's function-map contract releases
//! its one actual with kind `descriptor`, and whose actual is exactly the
//! integer member read `r->F` or `r->F[K]` of that cell
//! ([`integer_member`], [`element_read`]). It lowers to the actual's typed
//! read and second obligation, the call with the released position read as
//! `Borrowed`, and rule 38c's close marked admitted, which is the
//! statement's whole custody effect: where rule 38c reads nothing the gate's
//! refusal stands, and where it does not decide it refuses. The decision is
//! `rule_38_descriptor_cells::close_actual::admits`; everything else keeps the
//! gate's refusal byte for byte.

use qcore_schemas::contract::ResourceParameterMode;

use super::{LegacyInstruction, Parser, Token, call_argument_spans, find_call, strip_parentheses};
use crate::indexed_collection::CollectionTypes;
use crate::typed_field_access::{DeclaredType, FunctionTypes, TypedAccessModel, is_name};

/// The refusal this rule is consulted for: the lexical gate's.
pub(super) const GATE_REFUSAL: &str = "indirect memory access is outside the resource projection";

/// Where one actual sits, as far as the callee's declaration says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Position {
    /// A declared `Borrowed` parameter.
    Borrowed,
    /// Beyond the declared parameter vector of a declared variadic data tail.
    Tail,
    /// A declared custody-taking parameter.
    Custody(&'static str),
    /// Beyond the declared parameter vector, with no declared tail.
    Beyond,
    /// The callee has no contract in force.
    Uncontracted,
}

impl Position {
    pub(super) fn of(
        contract: &qcore_schemas::contract::ResourceFunctionContract,
        variadic_tail: bool,
        index: usize,
    ) -> Self {
        match contract.parameters.get(index) {
            Some(ResourceParameterMode::Borrowed) => Self::Borrowed,
            Some(ResourceParameterMode::Owned { .. }) => Self::Custody("owned"),
            Some(ResourceParameterMode::Consumed { .. }) => Self::Custody("consumed"),
            Some(ResourceParameterMode::Released { .. }) => Self::Custody("released"),
            Some(ResourceParameterMode::Out { .. }) => Self::Custody("out"),
            Some(ResourceParameterMode::Stored { .. }) => Self::Custody("stored"),
            Some(ResourceParameterMode::Removed { .. }) => Self::Custody("removed"),
            None if variadic_tail => Self::Tail,
            None => Self::Beyond,
        }
    }

    /// Premise 1: only a declared `Borrowed` position or a declared data tail
    /// admits a read.
    pub(super) const fn admits(self) -> bool {
        matches!(self, Self::Borrowed | Self::Tail)
    }

    #[cfg(feature = "indirect-census")]
    const fn label(self) -> &'static str {
        match self {
            Self::Borrowed => "borrowed",
            Self::Tail => "tail",
            Self::Custody(mode) => mode,
            Self::Beyond => "beyond",
            Self::Uncontracted => "uncontracted",
        }
    }
}

/// One recognised actual.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct ArgumentRead {
    /// Every name the actual dereferences, first occurrence first.
    pub(super) bases: Vec<String>,
    /// The subset of `bases` read through `->` (their C02-P-ALIAS obligation).
    pub(super) record_bases: Vec<String>,
    /// Whether the value handed to the callee is a pointer read from memory
    /// (`B[I]`, `*B`, `B->F`, `S.F` over a pointer member), rather than a
    /// scalar.
    pub(super) pointer_valued: bool,
    /// Card W47-ACCESS-PATH-POSITIONS: the per-link obligations of a typed
    /// actual's paths (rule 10k). Empty for every lexically recognised actual.
    pub(super) chains: Vec<crate::indirect_chain_access::AccessChain>,
}

impl ArgumentRead {
    fn add_base(&mut self, base: &str, record: bool) {
        if !self.bases.iter().any(|known| known == base) {
            self.bases.push(base.to_owned());
        }
        if record && !self.record_bases.iter().any(|known| known == base) {
            self.record_bases.push(base.to_owned());
        }
    }
}

/// Why an actual was not recognised (census labels; the rule reads only
/// `Ok`/`Err`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Refusal {
    /// The function's declarations were not all read.
    Unreadable,
    /// A call inside the actual.
    Call,
    /// A unary `&` anywhere in the actual.
    Address,
    /// A member chain of depth two or more, or a subscript continued by a
    /// member.
    Chain,
    /// A leading parenthesised cast before a pointer read.
    Cast,
    /// `*`, `[` over a name that is not declared `T **` (for a pointer read) or
    /// `SCALAR *`/`SCALAR [N]` (for a scalar read).
    UntypedBase,
    /// `->`/`.` over a member the reader did not classify.
    UntypedMember,
    /// An increment, decrement or assignment in the actual.
    Write,
    /// Anything else.
    Other,
}

impl Refusal {
    #[cfg(feature = "indirect-census")]
    const fn label(self) -> &'static str {
        match self {
            Self::Unreadable => "unreadable",
            Self::Call => "call",
            Self::Address => "address",
            Self::Chain => "chain",
            Self::Cast => "cast",
            Self::UntypedBase => "untyped-base",
            Self::UntypedMember => "untyped-member",
            Self::Write => "write",
            Self::Other => "other",
        }
    }
}

/// Whether the token at `at` continues an access into another member, a
/// subscript or a call.
fn continues(texts: &[&str], at: usize) -> bool {
    match texts.get(at).copied() {
        Some(".") | Some("[") | Some("(") => true,
        Some("-") => texts.get(at + 1) == Some(&">"),
        _ => false,
    }
}

/// The index of the `]` matching the `[` at `open`.
fn closing_bracket(texts: &[&str], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (index, text) in texts.iter().enumerate().skip(open) {
        match *text {
            "[" => depth += 1,
            "]" => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

/// Whether `field` is a pointer member of `tag`: a member the reader classes
/// as a value (a scalar or a pointer of any depth, no array, no rejecting
/// qualifier) that is not a scalar.
fn pointer_member(model: &TypedAccessModel, tag: &str, field: &str) -> bool {
    model.borrowable_field(tag, field) && !model.scalar_field(tag, field)
}

/// A typed **pointer** read spanning the whole actual (after enclosing
/// parentheses): `B[I]`, `*B`, `B->F`, `S.F`.
fn pointer_read(
    argument: &[&str],
    model: &TypedAccessModel,
    types: &FunctionTypes,
    collections: &CollectionTypes,
) -> Result<ArgumentRead, Refusal> {
    let texts = strip_parentheses(argument);
    let mut read = ArgumentRead {
        pointer_valued: true,
        ..ArgumentRead::default()
    };
    match texts {
        ["*", base] if is_name(base) => {
            if !collections.pointer_element(base) {
                return Err(Refusal::UntypedBase);
            }
            read.add_base(base, false);
            Ok(read)
        }
        [base, "[", .., "]"] if is_name(base) => {
            let close = closing_bracket(texts, 1).ok_or(Refusal::Other)?;
            if close + 1 != texts.len() {
                return Err(Refusal::Chain);
            }
            if !collections.pointer_element(base) {
                return Err(Refusal::UntypedBase);
            }
            let index = &texts[2..close];
            if index
                .windows(2)
                .any(|pair| matches!(pair, ["+", "+"] | ["-", "-"]))
                || index.contains(&"=")
            {
                return Err(Refusal::Write);
            }
            if !crate::typed_field_access::is_scalar_value_expression(index, model, types) {
                return Err(Refusal::Other);
            }
            read.add_base(base, false);
            Ok(read)
        }
        [base, "-", ">", field, rest @ ..] if is_name(base) && is_name(field) => {
            if !rest.is_empty() {
                return Err(if continues(texts, 4) {
                    Refusal::Chain
                } else {
                    Refusal::Other
                });
            }
            let Some(DeclaredType::RecordPointer(tag)) = types.declared_type(base) else {
                return Err(Refusal::UntypedBase);
            };
            if !pointer_member(model, tag, field) {
                return Err(Refusal::UntypedMember);
            }
            read.add_base(base, true);
            Ok(read)
        }
        [object, ".", field, rest @ ..] if is_name(object) && is_name(field) => {
            if !rest.is_empty() {
                return Err(if continues(texts, 3) {
                    Refusal::Chain
                } else {
                    Refusal::Other
                });
            }
            let Some(DeclaredType::Record(tag)) = types.declared_type(object) else {
                return Err(Refusal::UntypedBase);
            };
            if !pointer_member(model, tag, field) {
                return Err(Refusal::UntypedMember);
            }
            Ok(read)
        }
        _ => Err(Refusal::Other),
    }
}

/// Type keywords a `sizeof` operand may spell besides typedef names.
const TYPE_KEYWORDS: &[&str] = &[
    "struct", "union", "enum", "const", "signed", "unsigned", "char", "short", "int", "long",
    "float", "double", "void", "_Bool",
];

/// Replace every `sizeof ( TYPE-NAME )` of `argument` by the literal `1`.
///
/// The operand is a type name only when every token is a type keyword, `*`, a
/// tag after `struct`/`union`/`enum`, or a typedef name the unit's declaration
/// reader read and this function does not redeclare as a variable. A name
/// neither (a file-scope object, or anything the reader did not read) keeps
/// the refusal: the W44-COLLECTION-ALLOCATION integration found
/// `sizeof(*g)` over a file-scope `g` folded as a type name. Also:
/// no `[` (so never a variable-length array type, the one operand C evaluates,
/// C11 6.5.3.4p2), no `(`, no member access. Its size is an integer constant,
/// which is all the scalar walker needs to know. Every other `sizeof` is left
/// in place and refuses there; `sizeof (*v)` and `sizeof (x)` over a declared
/// variable are never folded.
fn fold_type_sizes<'a>(
    argument: &[&'a str],
    model: &TypedAccessModel,
    types: &FunctionTypes,
) -> Vec<&'a str> {
    let mut folded = Vec::with_capacity(argument.len());
    let mut index = 0usize;
    while index < argument.len() {
        if argument[index] == "sizeof"
            && argument.get(index + 1) == Some(&"(")
            && let Some(close) = argument[index + 1..]
                .iter()
                .position(|text| *text == ")")
                .map(|offset| index + 1 + offset)
        {
            let operand = &argument[index + 2..close];
            let type_name = !operand.is_empty()
                && operand.iter().enumerate().all(|(position, text)| {
                    *text == "*"
                        || TYPE_KEYWORDS.contains(text)
                        || (is_name(text)
                            && types.declared_type(text).is_none()
                            && (model.declares_typedef(text)
                                || position.checked_sub(1).is_some_and(|before| {
                                    matches!(operand[before], "struct" | "union" | "enum")
                                })))
                });
            if type_name {
                folded.push("1");
                index = close + 1;
                continue;
            }
        }
        folded.push(argument[index]);
        index += 1;
    }
    folded
}

/// The address of a typed **non-pointer** sub-object: `& B -> F` (`B` declared
/// `struct T *`), `& S . F` (`S` declared `struct T`) with `F` declared with no
/// `*` ([`TypedAccessModel::addressable_field`]), or `& B [ I ]` with `B`
/// declared `SCALAR *` or `SCALAR [N]` and `I` a scalar value with no access.
///
/// The value is a pointer into `B`'s object that no field link can be reached
/// through: C's object model keeps a non-pointer member's storage disjoint from
/// every linkable member (card O04-B09B10's premise 4), and a scalar element
/// holds no pointer. What it keeps is `B`'s dereference obligation, as a read.
fn member_address(
    argument: &[&str],
    model: &TypedAccessModel,
    types: &FunctionTypes,
) -> Option<ArgumentRead> {
    let texts = strip_parentheses(argument);
    let mut read = ArgumentRead::default();
    match texts {
        ["&", base, "-", ">", field] if is_name(base) && is_name(field) => {
            let Some(DeclaredType::RecordPointer(tag)) = types.declared_type(base) else {
                return None;
            };
            model.addressable_field(tag, field).then_some(())?;
            read.add_base(base, true);
            Some(read)
        }
        ["&", object, ".", field] if is_name(object) && is_name(field) => {
            let Some(DeclaredType::Record(tag)) = types.declared_type(object) else {
                return None;
            };
            model.addressable_field(tag, field).then_some(read)
        }
        ["&", base, "[", .., "]"] if is_name(base) => {
            types.scalar_element(base)?;
            let close = closing_bracket(texts, 2)?;
            if close + 1 != texts.len() {
                return None;
            }
            let index = &texts[3..close];
            if index.contains(&"=")
                || index
                    .windows(2)
                    .any(|pair| matches!(pair, ["+", "+"] | ["-", "-"]))
                || !crate::typed_field_access::is_scalar_value_expression(index, model, types)
            {
                return None;
            }
            read.add_base(base, false);
            Some(read)
        }
        _ => None,
    }
}

/// Card **MK-EVENT-CELL-ARRAY-SLOTS** (gateway rule 38a): a constant-index
/// element of an integer array member, read or addressed — `B -> F [ K ]`,
/// `S . F [ K ]`, `& B -> F [ K ]`, `& S . F [ K ]` — spanning the whole actual
/// (after enclosing parentheses).
///
/// `B` is declared `struct T *` and `S` `struct T`, exactly as for the typed
/// member read `B -> F` this module already admits; `F` is declared
/// `INTEGER-TYPE F [ N ]` in `T`'s single complete non-union definition
/// ([`integer_array_length`]); `K` is a canonical decimal literal
/// ([`canonical_decimal`]) with `K < N`, so the element is an object inside
/// `F` (C11 6.5.2.1, 6.5.6p8) and the read is in bounds. The value read is an
/// integer, and the address is a pointer to an integer element that holds no
/// pointer, so the actual carries exactly the member read's obligation: `B`'s
/// typed read (none for `S`, a record of this frame). A variable, negative,
/// out-of-bounds or non-canonical index, a pointer member, a non-integer
/// element and every other shape answer `None`.
///
/// `unit` is the translation unit's token texts, where the record definition
/// is read. The caller asks this only at a `Borrowed` position of a
/// cell-bearing withheld declaration, after every older route refused.
pub(super) fn element_read(
    argument: &[&str],
    unit: &[&str],
    model: &TypedAccessModel,
    types: &FunctionTypes,
) -> Option<ArgumentRead> {
    let texts = strip_parentheses(argument);
    let element = match texts {
        ["&", rest @ ..] => rest,
        _ => texts,
    };
    let in_bounds = |tag: &str, field: &str, index: &str| {
        let length = integer_array_length(unit, model, tag, field)?;
        let index = canonical_decimal(index)?;
        (index < length).then_some(())
    };
    let mut read = ArgumentRead::default();
    match element {
        [base, "-", ">", field, "[", index, "]"] if is_name(base) && is_name(field) => {
            let Some(DeclaredType::RecordPointer(tag)) = types.declared_type(base) else {
                return None;
            };
            in_bounds(tag, field, index)?;
            read.add_base(base, true);
            Some(read)
        }
        [object, ".", field, "[", index, "]"] if is_name(object) && is_name(field) => {
            let Some(DeclaredType::Record(tag)) = types.declared_type(object) else {
                return None;
            };
            in_bounds(tag, field, index)?;
            Some(read)
        }
        _ => None,
    }
}

/// Card **MK-EVENT-CELL-ARRAY-SLOTS** (rule 38a): the value of a canonical
/// decimal literal, `0` or a nonzero digit followed by digits, with no suffix.
/// A leading `0` would be octal (C11 6.4.4.1), so `010` is refused rather
/// than read as ten; a suffix, a sign or any expression is refused too.
pub(super) fn canonical_decimal(text: &str) -> Option<u64> {
    let canonical = text == "0"
        || (text.bytes().all(|byte| byte.is_ascii_digit())
            && text.bytes().next().is_some_and(|first| first != b'0'));
    if canonical { text.parse().ok() } else { None }
}

/// Type qualifiers that keep a member out of the element class.
const ELEMENT_REJECTING_QUALIFIERS: &[&str] = &["volatile", "_Atomic"];

/// Card **MK-EVENT-CELL-ARRAY-SLOTS** (rule 38a): the declared length `N`
/// when `tag`'s definition in `unit` declares `field` as exactly one member
/// declaration `SPECIFIERS field [ N ] ;`, where:
///
/// * the declaration reader read `tag`'s single complete non-union definition
///   ([`TypedAccessModel::record_definition_is_complete`]);
/// * the declaration has one declarator, one dimension, no `*`, and `N` is a
///   canonical decimal literal;
/// * `SPECIFIERS` is an integer type ([`TypedAccessModel::integer_class`]: no
///   floating, `_Bool`, enumeration or plain `char`) with no `volatile` or
///   `_Atomic`.
///
/// `None` for every other member. The definition is the one occurrence of
/// `struct tag {` in `unit` (single by the first premise). Read here rather
/// than as a class of `typed_field_access.rs`, whose bytes are pinned whole by
/// `docs/reports/libc-reading-site-evidence/bindings.json`.
pub(super) fn integer_array_length(
    unit: &[&str],
    model: &TypedAccessModel,
    tag: &str,
    field: &str,
) -> Option<u64> {
    if !model.record_definition_is_complete(tag) {
        return None;
    }
    let open = unit
        .windows(3)
        .position(|window| window == ["struct", tag, "{"])?
        + 2;
    let mut depth = 0usize;
    let mut close = None;
    for (index, text) in unit.iter().enumerate().skip(open) {
        match *text {
            "{" => depth += 1,
            "}" => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    close = Some(index);
                    break;
                }
            }
            _ => {}
        }
    }
    let body = &unit[open + 1..close?];
    let mut found = None;
    for member in body.split(|text| *text == ";") {
        let [specifiers @ .., name, "[", length, "]"] = member else {
            continue;
        };
        if *name != field {
            continue;
        }
        if found.is_some()
            || specifiers.is_empty()
            || specifiers
                .iter()
                .any(|text| matches!(*text, "*" | "," | "[" | "]" | "(" | ")"))
            || specifiers
                .iter()
                .any(|text| ELEMENT_REJECTING_QUALIFIERS.contains(text))
            || model.integer_class(specifiers).is_none()
        {
            return None;
        }
        found = Some(canonical_decimal(length)?);
    }
    found
}

/// Card **CELL-PROTOCOL-CLOSE-ACTUAL** (rule 38d): whether `tag`'s definition
/// in `unit` declares `field` as exactly one member declaration
/// `SPECIFIERS field ;` with one declarator, no `*`, no dimension, no
/// bit-field, `SPECIFIERS` an integer type
/// ([`TypedAccessModel::integer_class`]) with no `volatile` or `_Atomic`, in
/// the single complete non-union definition the declaration reader read.
/// The scalar counterpart of [`integer_array_length`], read the same way.
pub(super) fn integer_member(
    unit: &[&str],
    model: &TypedAccessModel,
    tag: &str,
    field: &str,
) -> bool {
    if !model.record_definition_is_complete(tag) {
        return false;
    }
    let Some(open) = unit
        .windows(3)
        .position(|window| window == ["struct", tag, "{"])
        .map(|at| at + 2)
    else {
        return false;
    };
    let mut depth = 0usize;
    let mut close = None;
    for (index, text) in unit.iter().enumerate().skip(open) {
        match *text {
            "{" => depth += 1,
            "}" => {
                let Some(next) = depth.checked_sub(1) else {
                    return false;
                };
                depth = next;
                if depth == 0 {
                    close = Some(index);
                    break;
                }
            }
            _ => {}
        }
    }
    let Some(close) = close else {
        return false;
    };
    // A nested definition's members are not `tag`'s: refuse the record.
    if unit[open + 1..close].contains(&"{") {
        return false;
    }
    let mut found = false;
    for member in unit[open + 1..close].split(|text| *text == ";") {
        if !member.contains(&field) {
            continue;
        }
        // `field` spelled anywhere but last (an array, a bit-field, a second
        // declarator, a type name): not this shape, and the record refuses.
        let [specifiers @ .., name] = member else {
            return false;
        };
        if *name != field
            || found
            || specifiers.is_empty()
            || specifiers
                .iter()
                .any(|text| matches!(*text, "*" | "," | "[" | "]" | "(" | ")" | ":" | "{" | "}"))
            || specifiers
                .iter()
                .any(|text| ELEMENT_REJECTING_QUALIFIERS.contains(text))
            || model.integer_class(specifiers).is_none()
        {
            return false;
        }
        found = true;
    }
    found
}

/// Replace every `( NAME )` whose `NAME` this function declares as a variable
/// by `NAME` alone.
///
/// Macro expansion writes `mtx_lock(&(rkb)->rkb_lock)`. A parenthesised
/// primary expression designates exactly what the name does (C11 6.5.1p5);
/// requiring a declared variable keeps a cast to a typedef name, `( T ) x`,
/// out of the rewrite.
fn unparenthesise_names<'a>(argument: &[&'a str], types: &FunctionTypes) -> Vec<&'a str> {
    let mut rewritten = Vec::with_capacity(argument.len());
    let mut index = 0usize;
    while index < argument.len() {
        if argument[index] == "("
            && argument.get(index + 2) == Some(&")")
            && let Some(name) = argument.get(index + 1).copied()
            && is_name(name)
            && types.declared_type(name).is_some()
            && !index
                .checked_sub(1)
                .is_some_and(|before| is_name(argument[before]) || argument[before] == "sizeof")
        {
            rewritten.push(name);
            index += 3;
            continue;
        }
        rewritten.push(argument[index]);
        index += 1;
    }
    rewritten
}

/// Recognise one whole actual, or say why not.
pub(super) fn recognise(
    argument: &[&str],
    model: &TypedAccessModel,
    types: &FunctionTypes,
    collections: &CollectionTypes,
) -> Result<ArgumentRead, Refusal> {
    if !types.is_readable() {
        return Err(Refusal::Unreadable);
    }
    let unparenthesised = unparenthesise_names(argument, types);
    let argument = unparenthesised.as_slice();
    if argument
        .iter()
        .enumerate()
        .any(|(index, text)| is_name(text) && argument.get(index + 1) == Some(&"("))
    {
        return Err(Refusal::Call);
    }
    let folded = fold_type_sizes(argument, model, types);
    if let Some(plan) = crate::typed_field_access::typed_scalar_value(&folded, model, types) {
        return Ok(ArgumentRead {
            bases: plan.bases,
            record_bases: plan.record_bases,
            pointer_valued: false,
            chains: Vec::new(),
        });
    }
    // `sizeof (T) * n`: the lexical gate reads a `*` after `)` as a possible
    // dereference; once the type size is folded the actual is a scalar value
    // with no memory access at all, and carries no obligation.
    if folded.len() != argument.len()
        && crate::typed_field_access::is_scalar_value_expression(&folded, model, types)
    {
        return Ok(ArgumentRead::default());
    }
    if let Some(read) = member_address(argument, model, types) {
        return Ok(read);
    }
    match pointer_read(argument, model, types, collections) {
        Ok(read) => Ok(read),
        Err(refusal) => {
            // Classify the most common refusals for the census; every one of
            // them refuses.
            let stripped = strip_parentheses(argument);
            if stripped.first() == Some(&"&") {
                return Err(Refusal::Address);
            }
            if stripped.first() == Some(&"(") {
                return Err(Refusal::Cast);
            }
            if stripped
                .windows(2)
                .any(|pair| matches!(pair, ["+", "+"] | ["-", "-"]))
            {
                return Err(Refusal::Write);
            }
            let selections = stripped
                .windows(2)
                .filter(|pair| *pair == ["-", ">"])
                .count()
                + stripped.iter().filter(|text| **text == ".").count();
            if selections >= 2 || (selections >= 1 && stripped.contains(&"[")) {
                return Err(Refusal::Chain);
            }
            Err(refusal)
        }
    }
}

/// One actual this rule admitted for the statement being classified: the
/// callee, the argument index and the actual's exact tokens. Keyed by content,
/// so the same actual is found again when the element-store probe renames the
/// statement's destination.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct AdmittedArgument {
    pub(super) callee: String,
    pub(super) index: usize,
    texts: Vec<String>,
    read: ArgumentRead,
    /// Set by the classifier when the actual left the gate through this rule
    /// rather than through an existing route.
    used: bool,
    /// Card W45-CALLEE-SUMMARY-CASCADE: admitted at a `Released { nullable:
    /// true }` position by `member_release::admits_release`, not at a
    /// `Borrowed` position or a data tail.
    pub(super) release: bool,
    /// Whether the actual is also found under the minted name rule 12t
    /// classifies the call under ([`crate::retained_by::retained_name`]):
    /// the same call, the same actuals and the same declared positions.
    /// Without it the probe's admission left the gate only in the outer
    /// classification, and the inner one refused every actual no older route
    /// exempts. Card MK-EVENT-CELL-ARRAY-SLOTS (rule 38a) set it at a
    /// cell-bearing withheld declaration; card
    /// RETAINING-CALL-MINTED-NAME-ACTUALS (rule 38b) sets it at every
    /// withheld declaration's `Borrowed` position ([`minted_admission`]).
    minted: bool,
    /// Card RETAINING-CALL-MINTED-NAME-ACTUALS (rule 38b): set by hook (a)
    /// when the actual left the gate under the minted name, not the plain
    /// one. [`minted_positions_borrowed`] then requires the position in
    /// force at the classified call to be `Borrowed` still.
    minted_used: bool,
}

/// Card RETAINING-CALL-MINTED-NAME-ACTUALS (rule 38b): whether an admitted
/// actual is also found under rule 12t's minted name for its callee. At a
/// cell-bearing withheld declaration every admitted actual is (rule 38a, as
/// before); at any other withheld declaration, an actual the probe admitted
/// at a declared `Borrowed` position, never a rule-1 release. A function-map
/// callee (`withheld` false) never is: rule 12t never classifies it under a
/// minted name.
pub(super) const fn minted_admission(
    withheld: bool,
    cell_bearing: bool,
    position: Position,
    release: bool,
) -> bool {
    cell_bearing || (withheld && matches!(position, Position::Borrowed) && !release)
}

/// Card RETAINING-CALL-MINTED-NAME-ACTUALS: [`minted_admission`]'s decision
/// table, `(withheld, cell_bearing, position, release, minted)`, shared with
/// `QCore.RetainingCallMintedName.mintedAdmissionVectors` entry for entry.
/// The position's custody kind is spelled as `Position::Custody`'s word.
#[cfg(test)]
pub(super) const MINTED_ADMISSION_VECTORS: &[(bool, bool, Position, bool, bool)] = &[
    // Every withheld declaration's `Borrowed` read (rule 38b).
    (true, false, Position::Borrowed, false, true),
    // A rule-1 release, a custody position, a data tail, beyond the vector.
    (true, false, Position::Borrowed, true, false),
    (true, false, Position::Custody("released"), true, false),
    (true, false, Position::Custody("consumed"), false, false),
    (true, false, Position::Custody("owned"), false, false),
    (true, false, Position::Tail, false, false),
    (true, false, Position::Beyond, false, false),
    // Rule 38a, unchanged: every admitted actual of a cell-bearing call.
    (true, true, Position::Borrowed, false, true),
    (true, true, Position::Borrowed, true, true),
    (true, true, Position::Custody("released"), true, true),
    // A function-map callee and an uncontracted one.
    (false, false, Position::Borrowed, false, false),
    (false, false, Position::Tail, false, false),
    (false, false, Position::Custody("released"), true, false),
    (false, false, Position::Uncontracted, false, false),
];

impl AdmittedArgument {
    /// `Some(true)` when the actual matches under the minted name,
    /// `Some(false)` under the plain one, `None` when it does not match.
    fn matched(&self, callee: &str, index: usize, argument: &[&str]) -> Option<bool> {
        let minted = if self.callee == callee {
            false
        } else if self.minted
            && crate::retained_by::retained_callee(callee) == Some(self.callee.as_str())
        {
            true
        } else {
            return None;
        };
        (self.index == index
            && self.texts.len() == argument.len()
            && self.texts.iter().zip(argument).all(|(a, b)| a == b))
        .then_some(minted)
    }

    fn matches(&self, callee: &str, index: usize, argument: &[&str]) -> bool {
        self.matched(callee, index, argument).is_some()
    }
}

/// Card RETAINING-CALL-MINTED-NAME-ACTUALS (rule 38b): every actual that left
/// the gate under the minted name sits, in the call the statement was
/// classified as, at a `Borrowed` position of its own callee. Rule 12t's
/// route reads some positions as `resource_owned` after the inner
/// classification (a callback's requirement, rule 13u), which is not a read;
/// such a call keeps the gate's refusal.
pub(super) fn minted_positions_borrowed(
    result: &LegacyInstruction,
    used: &[AdmittedArgument],
) -> bool {
    let minted = used
        .iter()
        .filter(|entry| entry.minted_used)
        .collect::<Vec<_>>();
    if minted.is_empty() {
        return true;
    }
    let call = match result {
        LegacyInstruction::ElementStore {
            call: Some(call), ..
        } => call.as_ref(),
        other => other,
    };
    let LegacyInstruction::ContractCall {
        function, contract, ..
    } = call
    else {
        return false;
    };
    minted.iter().all(|entry| {
        entry.callee == *function
            && matches!(
                contract.parameters.get(entry.index),
                Some(ResourceParameterMode::Borrowed)
            )
    })
}

/// The probe state one statement's re-classification reads. Empty outside
/// [`Parser::call_argument_access`], which makes every hook the identity.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct ArgumentProbe {
    admitted: Vec<AdmittedArgument>,
    /// True while the statement is being classified again; the census of the
    /// refusal it may meet again is suppressed (it was recorded the first time).
    #[cfg_attr(not(feature = "indirect-census"), allow(dead_code))]
    pub(super) rerun: bool,
}

impl Parser<'_> {
    /// Hook (a), in the gate's span collection: the spans of this statement's
    /// first call whose actuals the probe admitted and no existing route
    /// already exempted. Marks each such actual used.
    pub(super) fn admitted_argument_spans(
        &mut self,
        texts: &[&str],
        existing: &[(usize, usize)],
    ) -> Vec<(usize, usize)> {
        if self.argument_probe.admitted.is_empty() {
            return Vec::new();
        }
        let Some((call_index, callee)) = find_call(texts) else {
            return Vec::new();
        };
        let Some(spans) = call_argument_spans(texts, call_index) else {
            return Vec::new();
        };
        let mut admitted = Vec::new();
        for (index, (start, end)) in spans.into_iter().enumerate() {
            if existing.contains(&(start, end)) {
                continue;
            }
            let argument = &texts[start..end];
            if let Some((entry, minted)) =
                self.argument_probe.admitted.iter_mut().find_map(|entry| {
                    let minted = entry.matched(callee, index, argument)?;
                    Some((entry, minted))
                })
            {
                entry.used = true;
                entry.minted_used |= minted;
                admitted.push((start, end));
            }
        }
        admitted
    }

    /// Hook (b): whether the actual at `index` of `callee` is one this rule
    /// admitted. Its base keeps the flow-sensitive dereference obligation, so
    /// it is not "an unmodeled borrowed expression" over a protected formal.
    pub(super) fn argument_admitted(&self, callee: &str, index: usize, argument: &[&str]) -> bool {
        self.argument_probe
            .admitted
            .iter()
            .any(|entry| entry.used && entry.matches(callee, index, argument))
    }

    /// Hook (c): the actuals the variadic tail rule reads, with every actual
    /// this rule admitted read as data. An admitted tail actual is a read whose
    /// base keeps its obligation and whose value the declared tail says the
    /// callee neither keeps nor takes custody of; the tail rule's own premises
    /// T1-T5 still decide every other tail actual.
    pub(super) fn tail_view<'a>(
        &self,
        callee: &str,
        arguments: &[&'a [&'a str]],
    ) -> Vec<&'a [&'a str]> {
        const DATA: &[&str] = &["0"];
        arguments
            .iter()
            .enumerate()
            .map(|(index, argument)| {
                if self.argument_admitted(callee, index, argument) {
                    DATA
                } else {
                    *argument
                }
            })
            .collect()
    }

    /// The rule. Consulted by the statement walker for a simple statement the
    /// classifier refused; `None` keeps the refusal. `Some` is the block the
    /// statement lowers to: the typed read of every base the used actuals
    /// dereference, their second obligation, then the call.
    pub(super) fn call_argument_access(
        &mut self,
        tokens: &[Token],
        instruction: &LegacyInstruction,
    ) -> Option<Vec<LegacyInstruction>> {
        // Card W45-SR5-SOUNDNESS: a member release a later callee repeats
        // keeps its refusal, under a description naming the double release.
        if let Some(refusal) = self.member_double_release(tokens, instruction) {
            return Some(vec![refusal]);
        }
        let mut probe = self.probe_call_arguments(tokens, instruction)?;
        // Card W45-CALLEE-SUMMARY-CASCADE: the object a rule-1 actual releases
        // is outside the projection, so its position is read as `Borrowed`.
        super::member_release::rewrite_released_positions(&mut probe.result, &probe.used);
        Some(self.admitted_read_sequence(&probe.used, probe.result))
    }

    /// The block an admitted call lowers to: the typed read of every base the
    /// used actuals dereference (premise 3), their second obligation (premise
    /// 4), then the call `result`.
    fn admitted_read_sequence(
        &self,
        used: &[AdmittedArgument],
        result: LegacyInstruction,
    ) -> Vec<LegacyInstruction> {
        let mut bases = Vec::<String>::new();
        let mut record_bases = Vec::<String>::new();
        let mut chains = Vec::<crate::indirect_chain_access::AccessChain>::new();
        let mut names = Vec::<String>::new();
        let mut pointer_bases = Vec::<String>::new();
        let mut pointer_places = Vec::<String>::new();
        let push = |list: &mut Vec<String>, name: &str| {
            if !list.iter().any(|known| known == name) {
                list.push(name.to_owned());
            }
        };
        for entry in used {
            for base in &entry.read.bases {
                push(&mut bases, base);
            }
            for base in &entry.read.record_bases {
                push(&mut record_bases, base);
            }
            for chain in &entry.read.chains {
                if !chains.contains(chain) {
                    chains.push(chain.clone());
                }
            }
            let texts = entry.texts.iter().map(String::as_str).collect::<Vec<_>>();
            for (index, text) in texts.iter().enumerate() {
                let member = index >= 1
                    && (texts[index - 1] == "."
                        || (texts[index - 1] == ">" && index >= 2 && texts[index - 2] == "-"));
                if is_name(text)
                    && !member
                    && texts.get(index + 1) != Some(&"(")
                    && !self.typed_model.is_enum_constant(text)
                {
                    push(&mut names, text);
                }
            }
            if entry.read.pointer_valued {
                for base in &entry.read.bases {
                    push(&mut pointer_bases, base);
                }
                // The place as the recogniser read it: `(p)->f` is `p->f`.
                let place = unparenthesise_names(&texts, &self.typed_types);
                if let Some(place) = super::canonical_place(&place) {
                    push(&mut pointer_places, &place);
                }
            }
        }
        let mut sequence = Vec::new();
        if !bases.is_empty() || !chains.is_empty() {
            sequence.push(LegacyInstruction::IndirectScalarAccess {
                typed_alternative: None,
                alias_reads: record_bases
                    .iter()
                    .filter_map(|base| self.read_alias(Some(base)))
                    .collect(),
                bases,
                destination: None,
                frame: None,
                // W44-INDIRECT-CHAIN-ACCESS composition: the lexical
                // recogniser refuses every member chain (and
                // `typed_scalar_value` uses the unextended walker); only a
                // W47 typed actual carries chains, each kept here with its
                // per-link obligation.
                chains,
                borrowed_destination: false,
                readback: None,
            });
        }
        if !names.is_empty() || !pointer_bases.is_empty() || !pointer_places.is_empty() {
            sequence.push(LegacyInstruction::ArgumentReads {
                names,
                pointer_bases,
                pointer_places,
            });
        }
        sequence.push(result);
        sequence
    }

    /// Card **CELL-PROTOCOL-CLOSE-ACTUAL** (gateway rule 38d): the lowering of
    /// a statement the gate refused that is exactly rule 38c's close `step`,
    /// with its integer member actual admitted at `close`'s release position.
    /// `None` keeps the refusal (`lowered`, the statement's own lowering).
    ///
    /// Admitted exactly where [`rule_38_descriptor_cells::close_actual::admits`]
    /// holds: rule 38 in force; the statement `close(r->CELL)` with `r`
    /// declared a pointer to a record whose declared protocol names the
    /// one-segment `CELL` (the parser's step); a function-map callee whose
    /// only actual sits at a `resource_released` position of kind
    /// `descriptor`; the actual exactly the integer member read `r->CELL` (or
    /// a constant in-bounds element `r->F[K]` of an integer array member,
    /// [`element_read`]). The statement is classified again with that actual
    /// admitted; the classification must be the plain contracted call. It
    /// lowers to the actual's typed read and second obligation (premises 3
    /// and 4, as every admitted actual), the call with its released position
    /// read as `Borrowed` (the number read is no identity the projection
    /// tracks), and rule 38c's close marked admitted: that close is the
    /// statement's whole custody effect, so where rule 38c reads nothing the
    /// gate's refusal stands and where it does not decide it refuses.
    pub(super) fn cell_close_actual(
        &mut self,
        tokens: &[Token],
        line: usize,
        lowered: &super::Statement,
        step: &super::descriptor_cells_route::ProtocolStep,
    ) -> Option<super::Statement> {
        use rule_38_descriptor_cells::close_actual as rule;
        let super::Statement::Instruction(LegacyInstruction::Unknown { description, .. }, _) =
            lowered
        else {
            return None;
        };
        let super::descriptor_cells_route::ProtocolStep::Close { root, path, .. } = step else {
            return None;
        };
        if description != GATE_REFUSAL || super::contains_braces(tokens) {
            return None;
        }
        let texts = tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        if !self.descriptor_names.is_empty()
            && super::descriptor_custody::statement_reads_descriptor(&texts, &self.descriptor_names)
                .is_some()
        {
            return None;
        }
        let (call_index, callee) = find_call(&texts)?;
        let spans = call_argument_spans(&texts, call_index)?;
        let [(start, end)] = spans.as_slice() else {
            return None;
        };
        let argument = unparenthesise_names(&texts[*start..*end], &self.typed_types);
        let direct = self.contracts.contract(callee);
        let callee_class = if direct.is_some() {
            rule::Callee::FunctionMap
        } else if self.contracts.retaining_contract(callee).is_some()
            || self.contracts.lifetime_summary(callee).is_some()
        {
            rule::Callee::Withheld
        } else {
            rule::Callee::Uncontracted
        };
        let position = match direct.and_then(|contract| contract.parameters.first()) {
            Some(ResourceParameterMode::Released { .. }) => rule::Position::Release,
            Some(ResourceParameterMode::Borrowed) => rule::Position::Borrowed,
            Some(_) => rule::Position::OtherCustody,
            None => rule::Position::Beyond,
        };
        let kind = if direct.is_some_and(|contract| {
            contract.parameter_kind(0) == qcore_schemas::contract::ResourceKind::Descriptor
        }) {
            rule::Kind::Descriptor
        } else {
            rule::Kind::Other
        };
        let read = self.cell_read(&argument, root, path);
        let site = rule::Site {
            rule_on: crate::gateway_rules::enabled(rule_38_descriptor_cells::ID),
            statement: rule::Statement::ProtocolClose,
            callee: callee_class,
            position,
            kind,
            actual: if read.is_some() {
                rule::Actual::CellRead
            } else {
                rule::Actual::Other
            },
        };
        if !rule::admits(site) {
            return None;
        }
        let admitted = AdmittedArgument {
            callee: callee.to_owned(),
            index: 0,
            texts: texts[*start..*end]
                .iter()
                .map(|text| (*text).to_owned())
                .collect(),
            read: read?,
            used: false,
            release: false,
            minted: false,
            minted_used: false,
        };
        let saved = self.probe_state();
        self.argument_probe = ArgumentProbe {
            admitted: vec![admitted],
            rerun: true,
        };
        let mut result = self.classify_simple(tokens);
        let probe = std::mem::take(&mut self.argument_probe);
        let used = probe
            .admitted
            .into_iter()
            .filter(|entry| entry.used)
            .collect::<Vec<_>>();
        let plain_call = used.len() == 1
            && matches!(&result, LegacyInstruction::ContractCall {
                function,
                destination: None,
                contract,
                nested,
                ..
            } if function == callee
                && nested.is_empty()
                && matches!(contract.parameters.as_slice(), [ResourceParameterMode::Released { .. }])
                && contract.parameter_kind(0) == qcore_schemas::contract::ResourceKind::Descriptor);
        if !plain_call {
            self.restore_probe_state(&saved);
            return None;
        }
        if let LegacyInstruction::ContractCall { contract, .. } = &mut result
            && let Some(mode) = contract.parameters.get_mut(0)
        {
            *mode = ResourceParameterMode::Borrowed;
        }
        let mut close = step.clone();
        if let super::descriptor_cells_route::ProtocolStep::Close { admitted, .. } = &mut close {
            *admitted = true;
        }
        let mut sequence = self.admitted_read_sequence(&used, result);
        sequence.push(LegacyInstruction::CellProtocolStep {
            step: Box::new(close),
        });
        Some(super::Statement::Block(
            sequence
                .into_iter()
                .map(|instruction| super::Statement::Instruction(instruction, line))
                .collect(),
        ))
    }

    /// Rule 38d: the actual as the integer member read of the close step's
    /// own cell, `root -> path` with `path` one segment `F` or `F[K]`: `F` an
    /// integer member (or `F[K]` a constant in-bounds element of an integer
    /// array member) of the single complete definition of the record `root`
    /// is declared a pointer to. Its only base, and record base, is `root`.
    fn cell_read(&self, argument: &[&str], root: &str, path: &str) -> Option<ArgumentRead> {
        let Some(DeclaredType::RecordPointer(tag)) = self.typed_types.declared_type(root) else {
            return None;
        };
        let unit = self
            .tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        let read = match argument {
            [base, "-", ">", field] if *base == root && *field == path => {
                integer_member(&unit, &self.typed_model, tag, field).then(|| {
                    let mut read = ArgumentRead::default();
                    read.add_base(root, true);
                    read
                })?
            }
            [base, "-", ">", field, "[", index, "]"]
                if *base == root && format!("{field}[{index}]") == path =>
            {
                element_read(argument, &unit, &self.typed_model, &self.typed_types)?
            }
            _ => return None,
        };
        (read.bases == [root] && read.record_bases == [root] && !read.pointer_valued)
            .then_some(read)
    }

    /// Card W45-SR5-SOUNDNESS: the gate's refusal of a statement whose call
    /// hands a member place [`super::member_release::AdmissibleReleases`]
    /// withheld as a double release, restated under that reason with the same
    /// writes. `None` for every other statement.
    fn member_double_release(
        &self,
        tokens: &[Token],
        instruction: &LegacyInstruction,
    ) -> Option<LegacyInstruction> {
        let LegacyInstruction::Unknown {
            description,
            writes,
        } = instruction
        else {
            return None;
        };
        if description != GATE_REFUSAL {
            return None;
        }
        let texts = tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        let (call_index, _) = find_call(&texts)?;
        let spans = call_argument_spans(&texts, call_index)?;
        let description = spans
            .into_iter()
            .find_map(|(start, end)| self.member_releases.double_release(&texts[start..end]))?;
        Some(LegacyInstruction::Unknown {
            description,
            writes: writes.clone(),
        })
    }

    /// The candidate actuals of one refused statement. Also returns, for the
    /// census, every actual of the first call with its position and recogniser
    /// answer. `None` when the statement carries no call.
    fn candidate_arguments(
        &self,
        texts: &[&str],
        lines: &[usize],
    ) -> Option<(String, Vec<AdmittedArgument>, Vec<ArgumentFacts>)> {
        let (call_index, callee) = find_call(texts)?;
        // Card FB-EVENT-LOOP-RETENTION-AND-REGISTRATION (discharging
        // RETAINING-CALL-MEMBER-ACTUAL): a withheld declaration's borrowed
        // positions are read as the function map's are; what the call then
        // retains, removes or registers is its route-G hook's (rule 12t), and
        // a re-classification that is not a contracted call keeps the refusal.
        let direct = self.contracts.contract(callee);
        // Card MK-EVENT-CELL-ARRAY-SLOTS (rule 38a): the withheld declaration
        // in force, when it is one (asked exactly as before), and whether it
        // carries a descriptor-cell state (§C04.6.2), which rule 38 decides.
        let withheld = if direct.is_none() {
            self.contracts.retaining_contract(callee)
        } else {
            None
        };
        let cell_bearing = withheld.is_some_and(|declared| !declared.descriptor_cells.is_empty());
        let contract = direct
            .or_else(|| withheld.map(|declared| &declared.contract))
            .or_else(|| {
                self.contracts
                    .lifetime_summary(callee)
                    .map(|summary| &summary.contract)
            });
        let spans = call_argument_spans(texts, call_index)?;
        let arity = spans.len();
        let variadic_tail = self.variadic_tails.contains(callee);
        let mut admitted = Vec::new();
        let mut facts = Vec::new();
        for (index, (start, end)) in spans.into_iter().enumerate() {
            let argument = &texts[start..end];
            let access = super::contains_unmodeled_access(argument);
            let position = contract.map_or(Position::Uncontracted, |contract| {
                Position::of(contract, variadic_tail, index)
            });
            let answer = access.then(|| {
                let lexical = recognise(
                    argument,
                    &self.typed_model,
                    &self.typed_types,
                    &self.collection_types,
                );
                if let Ok(read) = &lexical
                    && position.admits()
                {
                    // Card W47-ACCESS-PATH-POSITIONS: parity only.
                    self.record_typed_argument_legacy(
                        callee,
                        index,
                        arity,
                        argument,
                        lines[start],
                        read,
                    );
                }
                // Card W47-ACCESS-PATH-POSITIONS (rule 10k): the typed actual,
                // asked only where the lexical recogniser refused and the
                // position admits a read.
                match lexical {
                    Err(refusal) if position.admits() => self
                        .typed_call_argument(callee, index, arity, argument, lines[start])
                        .map(|typed| ArgumentRead {
                            bases: typed.plan.scalar.bases,
                            record_bases: typed.plan.scalar.record_bases,
                            pointer_valued: typed.pointer_valued,
                            chains: typed.plan.chains,
                        })
                        // Card CG15-STORAGE-DURATION-AND-VOLATILE (rule 11h):
                        // the address of a static-duration object.
                        .or_else(|| {
                            self.static_address_argument(callee, index, argument, lines[start])
                        })
                        // Card MK-EVENT-CELL-ARRAY-SLOTS (rule 38a): a
                        // constant-index integer element, asked last and
                        // only at a cell-bearing withheld declaration's
                        // `Borrowed` position (never a data tail), so rule 38
                        // decides the cell it names.
                        .or_else(|| {
                            if !cell_bearing || position != Position::Borrowed {
                                return None;
                            }
                            let argument = unparenthesise_names(argument, &self.typed_types);
                            let unit = self
                                .tokens
                                .iter()
                                .map(|token| token.text.as_str())
                                .collect::<Vec<_>>();
                            element_read(&argument, &unit, &self.typed_model, &self.typed_types)
                        })
                        .ok_or(refusal),
                    // Card MEMBER-EFFECT-COMPONENT-SCOPE (rule 38j amended):
                    // a member spelled through a pointer typedef, at a
                    // nullable release of a place rule 5k or rule 38j
                    // admitted, reads as the lexical `B->F` would when Clang
                    // reads it as a pointer member's value.
                    Err(Refusal::UntypedMember)
                        if matches!(
                            contract.and_then(|contract| contract.parameters.get(index)),
                            Some(ResourceParameterMode::Released { nullable: true })
                        ) && self.member_releases.admits(argument)
                            && self.typed_member_release(
                                callee,
                                index,
                                arity,
                                argument,
                                lines[start],
                            ) =>
                    {
                        let mut read = ArgumentRead {
                            pointer_valued: true,
                            ..ArgumentRead::default()
                        };
                        if let [base, ..] = strip_parentheses(argument) {
                            read.add_base(base, true);
                        }
                        Ok(read)
                    }
                    other => other,
                }
            });
            // Card W45-CALLEE-SUMMARY-CASCADE: a pointer member handed to a
            // nullable release is admitted too (`member_release`, rule 1).
            let release = matches!(&answer, Some(Ok(read))
                if !position.admits()
                    && super::member_release::admits_release(contract, index, read)
                    && self.member_releases.admits(argument));
            if let Some(Ok(read)) = &answer
                && (position.admits() || release)
            {
                admitted.push(AdmittedArgument {
                    callee: callee.to_owned(),
                    index,
                    texts: argument.iter().map(|text| (*text).to_owned()).collect(),
                    read: read.clone(),
                    used: false,
                    release,
                    minted: minted_admission(withheld.is_some(), cell_bearing, position, release),
                    minted_used: false,
                });
            }
            facts.push(ArgumentFacts {
                index,
                position,
                access,
                answer,
                texts: argument.iter().map(|text| (*text).to_owned()).collect(),
            });
        }
        Some((callee.to_owned(), admitted, facts))
    }

    /// Classify the statement again with the candidate actuals admitted.
    /// `Some` only when that classification is a contracted call (or an
    /// element store around one) that used at least one candidate; every
    /// counter the classification advanced is restored otherwise, and the
    /// saved values travel with the probe so the caller can restore them too.
    fn probe_call_arguments(
        &mut self,
        tokens: &[Token],
        instruction: &LegacyInstruction,
    ) -> Option<Probe> {
        let LegacyInstruction::Unknown { description, .. } = instruction else {
            return None;
        };
        if description != GATE_REFUSAL || super::contains_braces(tokens) {
            return None;
        }
        let texts = tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        let descriptor = !self.descriptor_names.is_empty()
            && super::descriptor_custody::statement_reads_descriptor(
                &texts,
                &self.descriptor_names,
            )
            .is_some();
        let lines = tokens.iter().map(|token| token.line).collect::<Vec<_>>();
        let (callee, admitted, facts) = self.candidate_arguments(&texts, &lines)?;
        if descriptor || admitted.is_empty() {
            #[cfg(feature = "indirect-census")]
            self.census_call_arguments(tokens, &callee, &facts, descriptor, None, &[]);
            let _ = (&callee, &facts);
            return None;
        }
        // Card GAPO-RETURN-CORRECTNESS (rule 15x): the whole probe state,
        // taken before the probe is installed.
        let saved = self.probe_state();
        self.argument_probe = ArgumentProbe {
            admitted,
            rerun: true,
        };
        let result = self.classify_simple(tokens);
        let probe = std::mem::take(&mut self.argument_probe);
        let used = probe
            .admitted
            .into_iter()
            .filter(|entry| entry.used)
            .collect::<Vec<_>>();
        let admitted_call = !used.is_empty()
            && match &result {
                LegacyInstruction::ContractCall { destination, .. } => {
                    destination_is_exact(&texts, destination.as_deref())
                }
                LegacyInstruction::ElementStore { call, .. } => call.is_some(),
                _ => false,
            }
            // Card W45-CALLEE-SUMMARY-CASCADE: a rule-1 actual only in a plain
            // contracted call at its nullable release position.
            && super::member_release::released_positions_rewritable(&result, &used)
            // Card RETAINING-CALL-MINTED-NAME-ACTUALS (rule 38b): an actual
            // admitted under the minted name only where the call in force
            // still reads it at a `Borrowed` position.
            && minted_positions_borrowed(&result, &used);
        #[cfg(feature = "indirect-census")]
        self.census_call_arguments(
            tokens,
            &callee,
            &facts,
            descriptor,
            Some(&result),
            if admitted_call { &used } else { &[] },
        );
        if !admitted_call {
            self.restore_probe_state(&saved);
            return None;
        }
        Some(Probe { result, used })
    }

    /// Card CG10-INDIRECT-ACCESS-SOLE-RESIDUALS (gateway rule 12k; shared
    /// item added to this module): the classification of `tokens` with every
    /// memory-reading actual of its first call read as this rule reads an
    /// actual at a `Borrowed` position, whatever the callee's contract says.
    /// `None` when the statement has no call, no actual reads memory, or one
    /// that does is recognised neither lexically ([`recognise`]) nor by the
    /// typed actual (rule 10k). This is a **reading**, never a lowering: the
    /// probe is cleared and every counter the classification advanced is
    /// restored whatever it answers, and the typed reading records no parity.
    pub(super) fn reclassified_with_borrowed_actuals(
        &mut self,
        tokens: &[Token],
    ) -> Option<LegacyInstruction> {
        let texts = tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        let lines = tokens.iter().map(|token| token.line).collect::<Vec<_>>();
        let (call_index, callee) = find_call(&texts)?;
        let spans = call_argument_spans(&texts, call_index)?;
        let arity = spans.len();
        let mut admitted = Vec::new();
        for (index, (start, end)) in spans.into_iter().enumerate() {
            let argument = &texts[start..end];
            if !super::contains_unmodeled_access(argument) {
                continue;
            }
            let read = match recognise(
                argument,
                &self.typed_model,
                &self.typed_types,
                &self.collection_types,
            ) {
                Ok(read) => read,
                Err(_) => {
                    // Rule 12q: a consultation that records nothing keeps no
                    // parity decisions.
                    let typed = self.typed_positions_index().and_then(|positions| {
                        positions.argument(
                            callee,
                            index,
                            arity,
                            argument,
                            lines[start],
                            |name| self.position_never_bound(name),
                            None,
                        )
                    })?;
                    ArgumentRead {
                        bases: typed.plan.scalar.bases,
                        record_bases: typed.plan.scalar.record_bases,
                        pointer_valued: typed.pointer_valued,
                        chains: typed.plan.chains,
                    }
                }
            };
            admitted.push(AdmittedArgument {
                callee: callee.to_owned(),
                index,
                texts: argument.iter().map(|text| (*text).to_owned()).collect(),
                read,
                used: false,
                release: false,
                minted: false,
                minted_used: false,
            });
        }
        if admitted.is_empty() {
            return None;
        }
        let saved = self.probe_state();
        let previous = std::mem::replace(
            &mut self.argument_probe,
            ArgumentProbe {
                admitted,
                rerun: true,
            },
        );
        let result = self.classify_simple(tokens);
        self.argument_probe = previous;
        self.restore_probe_state(&saved);
        Some(result)
    }
}

/// Whether the destination the classifier named for this statement is exactly
/// its left-hand side.
///
/// On this card's base, `direct_assignment_destination` named the *first
/// suffix* of the left-hand side that spells a place, so
/// `result [ index ] . label = CALL ( … )` was named `label`; admitting the
/// actual must not also admit a destination the classifier misnamed (measured:
/// `flb_input_chunk_get_direct_routes`). Card
/// W44-ASSIGNMENT-DESTINATION-SUFFIX-PLACE (gateway constraint 5i) repaired the
/// naming itself: a sub-object store is now refused at the lexical gate before
/// a call is ever classified, so on the combined tree this check is
/// **redundant** — measured: the corpus run with it disabled is byte-identical.
/// It is kept as this route's own statement of the premise it relies on, so the
/// admission does not rest silently on another module's invariant; it can only
/// refuse.
///
/// The accepted left-hand sides are those with no memory access at all (a
/// plain name or a declaration), `* name` (the out-slot place `*name`, which
/// transfer sanctions or refuses), and one whose whole spelling is the named
/// place (the admitted aggregate store `p->f`). Everything else keeps the
/// refusal.
fn destination_is_exact(texts: &[&str], destination: Option<&str>) -> bool {
    let Some(destination) = destination else {
        return true;
    };
    let Some(equal) = texts.iter().position(|text| *text == "=") else {
        return false;
    };
    let left = &texts[..equal];
    !super::contains_unmodeled_access(left)
        || matches!(left, ["*", name] if destination.strip_prefix('*') == Some(*name))
        || super::canonical_place(left).as_deref() == Some(destination)
}

/// One accepted probe: the classification with the admitted actuals and the
/// actuals it used. The counters it advanced stay advanced: the call is kept.
pub(super) struct Probe {
    pub(super) result: LegacyInstruction,
    pub(super) used: Vec<AdmittedArgument>,
}

/// The names [`LegacyInstruction::ArgumentReads`] refuses in `state`: a name
/// bound to an identity carried by an integer, a pointer base bound to a target
/// that owns sub-custody (a field link, a contained child or an indexed
/// collection), and a bound pointer place. Reads nothing else and writes
/// nothing.
pub(super) fn refused_reads(
    state: &super::AbstractState,
    kinds: &std::collections::BTreeMap<usize, qcore_schemas::contract::ResourceKind>,
    names: &[String],
    pointer_bases: &[String],
    pointer_places: &[String],
) -> Vec<String> {
    let mut refused = Vec::new();
    for name in names {
        if super::carries_integer_identity(state, name, kinds) {
            refused.push(name.clone());
        }
    }
    for base in pointer_bases {
        let owns_sub_custody = state.pointers.get(base).is_some_and(|pointer| {
            pointer.targets.iter().any(|site| {
                state
                    .field_links
                    .get(site)
                    .is_some_and(|links| links.values().any(|children| !children.is_empty()))
                    || state
                        .contained
                        .get(site)
                        .is_some_and(|children| !children.is_empty())
                    || state.collections.contains_key(site)
            })
        });
        if owns_sub_custody && !refused.contains(base) {
            refused.push(base.clone());
        }
    }
    for place in pointer_places {
        if state.pointers.contains_key(place) && !refused.contains(place) {
            refused.push(place.clone());
        }
    }
    refused
}

/// One actual of a probed call, for the census.
#[derive(Clone, Debug)]
#[cfg_attr(not(feature = "indirect-census"), allow(dead_code))]
pub(super) struct ArgumentFacts {
    index: usize,
    position: Position,
    access: bool,
    answer: Option<Result<ArgumentRead, Refusal>>,
    texts: Vec<String>,
}

#[cfg(feature = "indirect-census")]
pub(crate) mod census {
    //! The call-argument census stream: one JSON line per simple statement the
    //! lexical gate refused whose first call is contracted, written after the
    //! classifier decided. Behind the off-by-default `indirect-census`
    //! feature; nothing reads it but the card's scripts.

    use std::fs::File;
    use std::io::{BufWriter, Write};
    use std::sync::Mutex;

    /// The environment variable naming the stream.
    pub const CALL_ARGUMENT_CENSUS_PATH: &str = "QCORE_CALL_ARGUMENT_CENSUS";

    static SINK: Mutex<Option<Option<BufWriter<File>>>> = Mutex::new(None);

    pub(super) fn collecting() -> bool {
        std::env::var_os(CALL_ARGUMENT_CENSUS_PATH).is_some()
    }

    pub(super) fn emit(record: &serde_json::Value) {
        let Ok(mut guard) = SINK.lock() else {
            return;
        };
        let opened = guard.get_or_insert_with(|| {
            let path = std::env::var_os(CALL_ARGUMENT_CENSUS_PATH)?;
            File::create(path).ok().map(BufWriter::new)
        });
        let Some(writer) = opened.as_mut() else {
            return;
        };
        if let Ok(line) = serde_json::to_string(record) {
            let _ = writeln!(writer, "{line}");
            let _ = writer.flush();
        }
    }
}

#[cfg(feature = "indirect-census")]
impl Parser<'_> {
    fn census_call_arguments(
        &self,
        tokens: &[Token],
        callee: &str,
        facts: &[ArgumentFacts],
        descriptor: bool,
        outcome: Option<&LegacyInstruction>,
        used: &[AdmittedArgument],
    ) {
        if self.admit_derived_returns || !census::collecting() {
            return;
        }
        let texts = tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        let base_facts = |base: &str| {
            serde_json::json!({
                "base": base,
                "declared": format!("{:?}", self.typed_types.declared_type(base)),
                "pointer_element": self.collection_types.pointer_element(base),
                "formal": self.formal_parameters.contains(base),
                "protected": self.protected_formals.contains(base),
                "may_bind": self.typed_types.may_bind(base),
                "never_bound": self.typed_bases.never_bound(base),
                "store_base": self.store_bases.contains(base),
            })
        };
        let arguments = facts
            .iter()
            .map(|fact| {
                let (recognised, reason, bases, pointer) = match &fact.answer {
                    None => (
                        serde_json::Value::Null,
                        serde_json::Value::Null,
                        Vec::new(),
                        false,
                    ),
                    Some(Ok(read)) => (
                        serde_json::json!(if read.pointer_valued {
                            "pointer"
                        } else {
                            "scalar"
                        }),
                        serde_json::Value::Null,
                        read.bases.iter().map(|base| base_facts(base)).collect(),
                        read.pointer_valued,
                    ),
                    Some(Err(refusal)) => (
                        serde_json::Value::Null,
                        serde_json::json!(refusal.label()),
                        Vec::new(),
                        false,
                    ),
                };
                serde_json::json!({
                    "index": fact.index,
                    "position": fact.position.label(),
                    "access": fact.access,
                    "recognised": recognised,
                    "reason": reason,
                    "pointer": pointer,
                    "bases": bases,
                    "texts": fact.texts,
                })
            })
            .collect::<Vec<_>>();
        let outcome = outcome.map(|instruction| match instruction {
            LegacyInstruction::ContractCall { .. } => "contract-call".to_owned(),
            LegacyInstruction::ElementStore { .. } => "element-store".to_owned(),
            LegacyInstruction::Unknown { description, .. } => description.clone(),
            other => format!("{other:?}").chars().take(40).collect(),
        });
        census::emit(&serde_json::json!({
            "kind": "call_argument",
            "unit_seq": crate::indirect_census::unit_seq(),
            "function": self.census_function,
            "line": tokens.first().map_or(0, |token| token.line),
            "callee": callee,
            "variadic_tail": self.variadic_tails.contains(callee),
            "descriptor": descriptor,
            "outcome": outcome,
            "used": used.iter().map(|entry| entry.index).collect::<Vec<_>>(),
            "arguments": arguments,
            "texts": texts,
        }));
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    /// Card RETAINING-CALL-MINTED-NAME-ACTUALS (rule 38b): the decision table
    /// `QCore.RetainingCallMintedName` checks, entry for entry.
    #[test]
    fn minted_admission_vectors_hold() {
        for &(withheld, cell_bearing, position, release, minted) in MINTED_ADMISSION_VECTORS {
            assert_eq!(
                minted_admission(withheld, cell_bearing, position, release),
                minted,
                "{withheld} {cell_bearing} {position:?} {release}"
            );
        }
    }

    /// Rule 38b: the minted name matches an entry only when it is minted and
    /// only as its own callee; a match under it is reported as minted, and
    /// the plain name is unchanged.
    #[test]
    fn the_minted_name_matches_exactly_the_plain_admission() {
        let entry = |minted| AdmittedArgument {
            callee: "mk_event_add".to_owned(),
            index: 1,
            texts: vec!["c".into(), "->".into(), "fd".into()],
            read: ArgumentRead::default(),
            used: false,
            release: false,
            minted,
            minted_used: false,
        };
        let actual = ["c", "->", "fd"];
        let minted_name = crate::retained_by::retained_name("mk_event_add");
        for minted in [false, true] {
            let e = entry(minted);
            assert_eq!(e.matched("mk_event_add", 1, &actual), Some(false));
            assert_eq!(
                e.matched(&minted_name, 1, &actual),
                minted.then_some(true),
                "minted {minted}"
            );
            assert_eq!(e.matched(&minted_name, 2, &actual), None);
            assert_eq!(e.matched(&minted_name, 1, &["c", "->", "ch"]), None);
            assert_eq!(
                e.matched(
                    &crate::retained_by::retained_name("mk_event_del"),
                    1,
                    &actual
                ),
                None
            );
            assert_eq!(e.matched("mk_event_del", 1, &actual), None);
        }
    }

    fn texts(source: &str) -> Vec<&str> {
        let bytes = source.as_bytes();
        let mut tokens = Vec::new();
        let mut cursor = 0;
        while cursor < bytes.len() {
            let byte = bytes[cursor];
            if byte.is_ascii_whitespace() {
                cursor += 1;
            } else if byte.is_ascii_alphabetic() || byte == b'_' {
                let start = cursor;
                while cursor < bytes.len()
                    && (bytes[cursor].is_ascii_alphanumeric() || bytes[cursor] == b'_')
                {
                    cursor += 1;
                }
                tokens.push(&source[start..cursor]);
            } else if byte.is_ascii_digit() {
                let start = cursor;
                while cursor < bytes.len() && bytes[cursor].is_ascii_alphanumeric() {
                    cursor += 1;
                }
                tokens.push(&source[start..cursor]);
            } else {
                tokens.push(&source[cursor..cursor + 1]);
                cursor += 1;
            }
        }
        tokens
    }

    const RECORDS: &str = "struct rec { int len ; char * name ; struct rec * next ; } ;";

    struct Facts {
        model: TypedAccessModel,
        types: FunctionTypes,
        collections: CollectionTypes,
    }

    fn facts(parameters: &str, body: &str) -> Facts {
        let unit: &'static str =
            Box::leak(format!("{RECORDS} {parameters} {body}").into_boxed_str());
        let model = TypedAccessModel::read(&texts(unit));
        let parameters: &'static str = Box::leak(parameters.to_owned().into_boxed_str());
        let body: &'static str = Box::leak(body.to_owned().into_boxed_str());
        let types = FunctionTypes::read(&model, &texts(parameters), &texts(body), &BTreeSet::new());
        let collections = CollectionTypes::read(&model, &texts(parameters), &texts(body));
        Facts {
            model,
            types,
            collections,
        }
    }

    fn answer(facts: &Facts, argument: &str) -> Result<ArgumentRead, Refusal> {
        let argument: &'static str = Box::leak(argument.to_owned().into_boxed_str());
        recognise(
            &texts(argument),
            &facts.model,
            &facts.types,
            &facts.collections,
        )
    }

    const PARAMETERS: &str = "char * * src , struct rec * p , int * n , int i";
    const BODY: &str = "struct rec s ; char buf [ 8 ] ;";

    #[test]
    fn scalar_values_carrying_accesses_are_recognised() {
        let facts = facts(PARAMETERS, BODY);
        let read = answer(&facts, "p -> len").expect("member");
        assert_eq!(read.bases, vec!["p"]);
        assert_eq!(read.record_bases, vec!["p"]);
        assert!(!read.pointer_valued);
        let read = answer(&facts, "* n + buf [ i ]").expect("sum");
        assert_eq!(read.bases, vec!["n", "buf"]);
        assert!(answer(&facts, "s . len").expect("object").bases.is_empty());
    }

    #[test]
    fn pointer_reads_are_recognised() {
        let facts = facts(PARAMETERS, BODY);
        let read = answer(&facts, "src [ i ]").expect("element");
        assert_eq!(read.bases, vec!["src"]);
        assert!(read.pointer_valued);
        assert!(answer(&facts, "* src").expect("deref").pointer_valued);
        let read = answer(&facts, "p -> name").expect("pointer member");
        assert_eq!(read.record_bases, vec!["p"]);
        assert!(read.pointer_valued);
        assert!(
            answer(&facts, "( s . name )")
                .expect("object member")
                .bases
                .is_empty()
        );
    }

    /// Card MK-EVENT-CELL-ARRAY-SLOTS (rule 38a): the element actual's shapes.
    #[test]
    fn element_reads_are_recognised_in_bounds_only() {
        let unit = "struct ch { int fd [ 2 ] ; unsigned long n [ 3 ] ; int * p ; double d [ 2 ] ; \
                    char k [ 2 ] ; int m [ 2 ] [ 2 ] ; volatile int v [ 2 ] ; int e [ 1 + 1 ] ; } ;";
        let unit_texts = texts(unit);
        let model = TypedAccessModel::read(&unit_texts);
        let types = FunctionTypes::read(
            &model,
            &texts("struct ch * c , int i"),
            &texts("struct ch s ;"),
            &BTreeSet::new(),
        );
        let read =
            |argument: &'static str| element_read(&texts(argument), &unit_texts, &model, &types);
        for admitted in ["c -> fd [ 0 ]", "& c -> fd [ 1 ]", "( c -> n [ 2 ] )"] {
            let read = read(admitted).unwrap_or_else(|| panic!("{admitted}"));
            assert_eq!(read.bases, vec!["c"], "{admitted}");
            assert_eq!(read.record_bases, vec!["c"], "{admitted}");
            assert!(!read.pointer_valued, "{admitted}");
        }
        for admitted in ["s . fd [ 1 ]", "& s . n [ 0 ]"] {
            let read = read(admitted).unwrap_or_else(|| panic!("{admitted}"));
            assert!(read.bases.is_empty(), "{admitted}");
        }
        for refused in [
            "c -> fd [ i ]",
            "c -> fd [ 2 ]",
            "c -> fd [ - 1 ]",
            "c -> fd [ 00 ]",
            "c -> fd [ 0x0 ]",
            "c -> fd [ 0 + 0 ]",
            "c -> p [ 0 ]",
            "c -> d [ 0 ]",
            "c -> k [ 0 ]",
            "c -> m [ 0 ] [ 0 ]",
            "c -> v [ 0 ]",
            "c -> e [ 0 ]",
            "c -> fd",
            "& c -> fd [ 0 ] + 1",
            "i -> fd [ 0 ]",
        ] {
            assert_eq!(read(refused), None, "{refused}");
        }
        assert_eq!(
            integer_array_length(&unit_texts, &model, "ch", "fd"),
            Some(2)
        );
        assert_eq!(integer_array_length(&unit_texts, &model, "ch", "e"), None);
        assert_eq!(canonical_decimal("0"), Some(0));
        assert_eq!(canonical_decimal("17"), Some(17));
        for refused in ["", "00", "010", "1u", "0x1", "-1"] {
            assert_eq!(canonical_decimal(refused), None);
        }
    }

    #[test]
    fn everything_else_refuses() {
        let facts = facts(PARAMETERS, BODY);
        assert_eq!(answer(&facts, "p -> next -> len"), Err(Refusal::Chain));
        assert_eq!(answer(&facts, "& p -> name"), Err(Refusal::Address));
        assert_eq!(answer(&facts, "& src [ i ]"), Err(Refusal::Address));
        assert_eq!(answer(&facts, "& p -> len + 1"), Err(Refusal::Address));
        assert_eq!(answer(&facts, "g ( p -> len )"), Err(Refusal::Call));
        assert_eq!(answer(&facts, "src [ i ++ ]"), Err(Refusal::Write));
        assert_eq!(answer(&facts, "( char * ) p -> name"), Err(Refusal::Cast));
        assert_eq!(answer(&facts, "* p"), Err(Refusal::UntypedBase));
        assert_eq!(answer(&facts, "src [ n [ 0 ] ]"), Err(Refusal::Other));
        assert!(answer(&facts, "p -> missing").is_err());
    }

    #[test]
    fn sub_object_addresses_are_recognised() {
        let facts = facts(PARAMETERS, BODY);
        let read = answer(&facts, "& p -> len").expect("member address");
        assert_eq!(read.record_bases, vec!["p"]);
        assert!(!read.pointer_valued);
        assert!(
            answer(&facts, "& s . len")
                .expect("object member")
                .bases
                .is_empty()
        );
        assert_eq!(
            answer(&facts, "& buf [ i ]").expect("element").bases,
            vec!["buf"]
        );
    }

    /// Integration of W44-COLLECTION-ALLOCATION: a bare name in a `sizeof`
    /// operand is a type only as a typedef the reader read or a tag. A
    /// file-scope object `g` is neither, so `sizeof ( * g )` keeps its refusal.
    #[test]
    fn a_type_size_needs_a_typedef_or_a_tag_not_a_file_scope_object() {
        let unit: &'static str =
            "struct rec { int len ; } ; typedef struct rec rec_t ; struct rec * g ; int i ;";
        let model = TypedAccessModel::read(&texts(unit));
        let types = FunctionTypes::read(&model, &texts("int i"), &texts("{ }"), &BTreeSet::new());
        let collections = CollectionTypes::read(&model, &texts("int i"), &texts("{ }"));
        let facts = Facts {
            model,
            types,
            collections,
        };
        assert_eq!(
            answer(&facts, "sizeof ( rec_t ) * i"),
            Ok(ArgumentRead::default())
        );
        assert_eq!(
            answer(&facts, "sizeof ( struct rec ) * i"),
            Ok(ArgumentRead::default())
        );
        assert!(answer(&facts, "sizeof ( * g ) * i").is_err());
        assert!(answer(&facts, "sizeof ( g ) * i").is_err());
    }

    /// Card CG08-CASTS-AND-TYPE-QUERIES (`SIZEOF-TYPE-FOLD-NO-LEAN`): the
    /// mechanical binding of `QCore.CastRepresentation.foldVectors` to
    /// `fold_type_sizes`, over the fixed reader of the test above.
    #[test]
    fn cg08_lean_fold_vectors_match_fold_type_sizes() {
        let lean = include_str!("../../../proof/QCore/CastRepresentation.lean");
        let body = lean
            .split("def foldVectors : List (String × Bool) := [")
            .nth(1)
            .expect("foldVectors");
        let body = &body[..body.find("\n]").expect("list end")];
        let unit: &'static str =
            "struct rec { int len ; } ; typedef struct rec rec_t ; struct rec * g ; int i ;";
        let model = TypedAccessModel::read(&texts(unit));
        let types = FunctionTypes::read(&model, &texts("int i"), &texts("{ }"), &BTreeSet::new());
        let mut count = 0;
        for line in body
            .lines()
            .filter(|line| line.trim_start().starts_with('('))
        {
            let argument = line.split('"').nth(1).expect("an argument");
            let expected = line.contains("true");
            let folded = fold_type_sizes(&texts(argument), &model, &types);
            assert_eq!(folded.contains(&"1"), expected, "{argument}: {folded:?}");
            assert_eq!(
                !folded.contains(&"sizeof"),
                expected,
                "{argument}: {folded:?}"
            );
            count += 1;
        }
        assert_eq!(count, 8);
    }

    #[test]
    fn a_type_size_is_a_constant_and_an_expression_size_is_not() {
        let facts = facts(PARAMETERS, BODY);
        let read = answer(&facts, "sizeof ( struct rec * ) * p -> len").expect("type size");
        assert_eq!(read.bases, vec!["p"]);
        assert_eq!(
            answer(&facts, "sizeof ( char * ) * i"),
            Ok(ArgumentRead::default())
        );
        assert!(answer(&facts, "sizeof ( * p ) * i").is_err());
        assert!(answer(&facts, "sizeof ( i ) * p -> len").is_err());
        assert!(answer(&facts, "sizeof ( int [ 4 ] ) * p -> len").is_err());
    }

    #[test]
    fn a_parenthesised_variable_is_the_variable() {
        let facts = facts(PARAMETERS, BODY);
        let read = answer(&facts, "& ( p ) - > len").expect("macro-expanded member address");
        assert_eq!(read.record_bases, vec!["p"]);
        assert_eq!(
            answer(&facts, "( p ) - > len + ( i )").expect("sum").bases,
            vec!["p"]
        );
        // A parenthesised chain is still a chain.
        assert!(answer(&facts, "& ( p - > next ) - > len").is_err());
    }

    #[test]
    fn an_unreadable_function_recognises_nothing() {
        let facts = facts(PARAMETERS, "void ( * hook ) ( int ) ;");
        assert_eq!(answer(&facts, "p -> len"), Err(Refusal::Unreadable));
    }

    /// A state binding `p` to site 1 and `n` to site 2, both live.
    fn bound_state() -> super::super::AbstractState {
        let mut state = super::super::AbstractState::default();
        for (name, site) in [("p", 1usize), ("n", 2usize)] {
            state.pointers.insert(
                name.to_owned(),
                super::super::AbstractPointer {
                    targets: std::collections::BTreeSet::from([site]),
                    unknown: false,
                    may_be_null: false,
                    derivation: super::super::Derivation::Identity,
                },
            );
            state.resources.insert(site, super::super::LIVE);
        }
        state
    }

    fn strings(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| (*name).to_owned()).collect()
    }

    #[test]
    fn the_second_obligation_refuses_only_what_it_names() {
        use qcore_schemas::contract::ResourceKind;
        let kinds = std::collections::BTreeMap::new();
        let state = bound_state();
        // Heap identities, no sub-custody, no bound place: nothing refuses.
        assert!(
            refused_reads(
                &state,
                &kinds,
                &strings(&["p", "n", "i"]),
                &strings(&["p"]),
                &strings(&["p->f"])
            )
            .is_empty()
        );
        // A name bound to an integer-carried identity is custody, never data.
        let kinds = std::collections::BTreeMap::from([(2usize, ResourceKind::Descriptor)]);
        assert_eq!(
            refused_reads(&state, &kinds, &strings(&["p", "n"]), &[], &[]),
            strings(&["n"])
        );
        let kinds = std::collections::BTreeMap::new();
        // A pointer read through an owner of a field link refuses its base.
        let mut linked = bound_state();
        linked
            .field_links
            .entry(1)
            .or_default()
            .insert("f".to_owned(), std::collections::BTreeSet::from([3]));
        assert_eq!(
            refused_reads(&linked, &kinds, &strings(&["p"]), &strings(&["p"]), &[]),
            strings(&["p"])
        );
        // ... and so does one through an owner of a contained child or of an
        // indexed collection.
        let mut contained = bound_state();
        contained
            .contained
            .insert(1, std::collections::BTreeSet::from([3]));
        assert_eq!(
            refused_reads(&contained, &kinds, &[], &strings(&["p"]), &[]),
            strings(&["p"])
        );
        let mut collection = bound_state();
        collection
            .collections
            .insert(1, crate::indexed_collection::Collection::Unknown);
        assert_eq!(
            refused_reads(&collection, &kinds, &[], &strings(&["p"]), &[]),
            strings(&["p"])
        );
        // A scalar read through the same owner is not a pointer read.
        assert!(refused_reads(&linked, &kinds, &strings(&["p"]), &[], &[]).is_empty());
        // A bound place refuses.
        let mut place = bound_state();
        place.pointers.insert(
            "p->f".to_owned(),
            super::super::AbstractPointer {
                targets: std::collections::BTreeSet::from([3]),
                unknown: false,
                may_be_null: false,
                derivation: super::super::Derivation::Identity,
            },
        );
        assert_eq!(
            refused_reads(&place, &kinds, &[], &[], &strings(&["p->f"])),
            strings(&["p->f"])
        );
        // An unbound name is never refused here: its obligation is the typed
        // read's.
        let empty = super::super::AbstractState::default();
        assert!(
            refused_reads(
                &empty,
                &kinds,
                &strings(&["q"]),
                &strings(&["q"]),
                &strings(&["q->f"])
            )
            .is_empty()
        );
    }

    #[test]
    fn only_borrowed_and_tail_positions_admit() {
        use qcore_schemas::contract::{ResourceFunctionContract, ResourceReturnMode};
        let contract = ResourceFunctionContract::new(
            [
                ResourceParameterMode::Borrowed,
                ResourceParameterMode::Released { nullable: false },
                ResourceParameterMode::Consumed { nullable: false },
                ResourceParameterMode::Owned { nullable: false },
                ResourceParameterMode::Out { nullable: false },
            ],
            ResourceReturnMode::None,
        );
        let admits = (0..6)
            .map(|index| Position::of(&contract, true, index).admits())
            .collect::<Vec<_>>();
        assert_eq!(admits, vec![true, false, false, false, false, true]);
        assert!(!Position::of(&contract, false, 5).admits());
    }
}
