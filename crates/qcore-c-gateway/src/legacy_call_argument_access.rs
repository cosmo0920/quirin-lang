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
//!
//! Bounds, alignment and the liveness of objects the projection does not track
//! are outside the property, exactly as for the typed scalar route.

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
/// The operand is a type name only when every token is a type keyword, `*`, or
/// a name this function does not declare as a variable (a typedef or a tag):
/// no `[` (so never a variable-length array type, the one operand C evaluates,
/// C11 6.5.3.4p2), no `(`, no member access. Its size is an integer constant,
/// which is all the scalar walker needs to know. Every other `sizeof` is left
/// in place and refuses there; `sizeof (*v)` and `sizeof (x)` over a declared
/// variable are never folded.
fn fold_type_sizes<'a>(argument: &[&'a str], types: &FunctionTypes) -> Vec<&'a str> {
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
                && operand.iter().all(|text| {
                    *text == "*"
                        || TYPE_KEYWORDS.contains(text)
                        || (is_name(text) && types.declared_type(text).is_none())
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
    let folded = fold_type_sizes(argument, types);
    if let Some(plan) = crate::typed_field_access::typed_scalar_value(&folded, model, types) {
        return Ok(ArgumentRead {
            bases: plan.bases,
            record_bases: plan.record_bases,
            pointer_valued: false,
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
    callee: String,
    index: usize,
    texts: Vec<String>,
    read: ArgumentRead,
    /// Set by the classifier when the actual left the gate through this rule
    /// rather than through an existing route.
    used: bool,
}

impl AdmittedArgument {
    fn matches(&self, callee: &str, index: usize, argument: &[&str]) -> bool {
        self.callee == callee
            && self.index == index
            && self.texts.len() == argument.len()
            && self.texts.iter().zip(argument).all(|(a, b)| a == b)
    }
}

/// The probe state one statement's re-classification reads. Empty outside
/// [`Parser::call_argument_access`], which makes every hook the identity.
#[derive(Clone, Debug, Default)]
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
            if let Some(entry) = self
                .argument_probe
                .admitted
                .iter_mut()
                .find(|entry| entry.matches(callee, index, argument))
            {
                entry.used = true;
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
    /// classifier refused; `None` keeps the refusal.
    pub(super) fn call_argument_access(
        &mut self,
        tokens: &[Token],
        instruction: &LegacyInstruction,
    ) -> Option<Vec<LegacyInstruction>> {
        if !cfg!(feature = "indirect-census") {
            return None;
        }
        // Census only at this commit: the probe is classified, recorded and
        // discarded, so the analysis is exactly the base tree's.
        let probe = self.probe_call_arguments(tokens, instruction)?;
        let _ = (&probe.result, &probe.used);
        self.next_site = probe.saved_site;
        self.element_lowerings = probe.saved_elements;
        None
    }

    /// The candidate actuals of one refused statement. Also returns, for the
    /// census, every actual of the first call with its position and recogniser
    /// answer. `None` when the statement carries no call.
    fn candidate_arguments(
        &self,
        texts: &[&str],
    ) -> Option<(String, Vec<AdmittedArgument>, Vec<ArgumentFacts>)> {
        let (call_index, callee) = find_call(texts)?;
        let contract = self.contracts.contract(callee);
        let spans = call_argument_spans(texts, call_index)?;
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
                recognise(
                    argument,
                    &self.typed_model,
                    &self.typed_types,
                    &self.collection_types,
                )
            });
            if let Some(Ok(read)) = &answer
                && position.admits()
            {
                admitted.push(AdmittedArgument {
                    callee: callee.to_owned(),
                    index,
                    texts: argument.iter().map(|text| (*text).to_owned()).collect(),
                    read: read.clone(),
                    used: false,
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
        let (callee, admitted, facts) = self.candidate_arguments(&texts)?;
        if descriptor || admitted.is_empty() {
            #[cfg(feature = "indirect-census")]
            self.census_call_arguments(tokens, &callee, &facts, descriptor, None, &[]);
            let _ = (&callee, &facts);
            return None;
        }
        let saved_site = self.next_site;
        let saved_elements = self.element_lowerings;
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
                LegacyInstruction::ContractCall { .. } => true,
                LegacyInstruction::ElementStore { call, .. } => call.is_some(),
                _ => false,
            };
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
            self.next_site = saved_site;
            self.element_lowerings = saved_elements;
            return None;
        }
        Some(Probe {
            result,
            used,
            saved_site,
            saved_elements,
        })
    }
}

/// One accepted probe: the classification with the admitted actuals, the
/// actuals it used, and the counters before it.
pub(super) struct Probe {
    pub(super) result: LegacyInstruction,
    pub(super) used: Vec<AdmittedArgument>,
    saved_site: usize,
    saved_elements: usize,
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
mod census {
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
