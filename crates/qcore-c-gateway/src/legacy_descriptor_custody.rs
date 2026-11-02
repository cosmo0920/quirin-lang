//! Card **W43-DESCRIPTOR-RESOURCE-TRACKING**: kind-indexed custody and
//! integer-carried identities in the C gateway.
//!
//! Before this card the projection tracked one kind of resource, and every
//! route that read a declared scalar treated it as custody-neutral, because a
//! scalar could never be bound to an identity. A file descriptor breaks both
//! premises at once: it is an obligation of a *different kind* from a heap
//! allocation (`free(fd)` discharges nothing, `close(p)` neither), and it is
//! carried by an `int`. This module holds the pieces the rest of the gateway
//! consults:
//!
//! * [`DescriptorNames`] — the names of one body that may hold a tracked
//!   integer-carried identity, read from the same preprocessed tokens and the
//!   same contract environment as every other typed premise, so W4 derivation,
//!   W4 validation and certificate rederivation reproduce it.
//! * [`value_carries_descriptor`] — **the single "is this value
//!   custody-carrying?" predicate** every custody-neutral scalar route asks.
//!   A route that stores, publishes or otherwise hands an integer value
//!   somewhere the projection does not follow must refuse when this answers
//!   `Some`. W42's `ScalarMemberStore`, the typed scalar-field no-op, the B08
//!   aggregate store and the scalar-source publication exemption ask it here;
//!   card W43-INDIRECT-ACCESS-GENERAL's indirect scalar stores are meant to
//!   ask the same function.
//! * [`validity_test`] — the guards that narrow an integer-carried identity's
//!   validity (`fd < 0`, `fd == -1`, …), the counterpart of a null test.
//! * [`site_kinds`] — the static kind of every site a CFG can create.
//!
//! Everything here refuses rather than approximates. It adds no admission a
//! heap-only environment could observe: with no kinded declaration in force,
//! [`DescriptorNames`] is empty, every site is heap, and every rule below is
//! the identity.

use std::collections::{BTreeMap, BTreeSet};

use super::{
    LegacyInstruction, ResourceContracts, ResourceKind, ResourceParameterMode, ResourceReturnMode,
    direct_assignment_destination, is_identifier, strip_parentheses,
};

/// The names of one function body that may hold a tracked integer-carried
/// identity.
///
/// Flow-insensitive and over-approximating on purpose: a name is in the set if
/// **any** statement of the body binds it to an integer-kinded result, or
/// copies one into it, or if it is a formal whose own declared contract gives
/// it an integer kind. Every rule that reads the set only ever *refuses* on
/// membership, so a larger set can only refuse more.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct DescriptorNames {
    names: BTreeSet<String>,
}

impl DescriptorNames {
    /// Read the set from one body's tokens (`body` excludes the braces).
    ///
    /// A name enters on either of two spellings, both at statement or
    /// expression level anywhere in the body:
    ///
    /// * `NAME = CALLEE (` — optionally with one parenthesised cast between
    ///   `=` and the callee — where `CALLEE`'s contract in force has an owned
    ///   result of an integer-carried kind;
    /// * `NAME = OTHER ;` or `NAME = OTHER )` where `OTHER` is already in the
    ///   set (a copy), iterated to a fixed point.
    ///
    /// `formals` adds the parameters whose kind the function's *own* contract
    /// declares integer-carried, for a body checked against a declaration.
    pub(crate) fn read(
        body: &[&str],
        contracts: &ResourceContracts,
        own_contract: Option<&qcore_schemas::contract::ResourceFunctionContract>,
        parameters: &[Option<String>],
    ) -> Self {
        let mut names = BTreeSet::new();
        if let Some(contract) = own_contract {
            for (index, name) in parameters.iter().enumerate() {
                if let Some(name) = name
                    && contract.parameter_kind(index).is_integer()
                    && !matches!(
                        contract.parameters.get(index),
                        None | Some(ResourceParameterMode::Borrowed)
                    )
                {
                    names.insert(name.clone());
                }
            }
        }
        // A contract with no integer-kinded result anywhere makes the scan a
        // no-op; skip it so a heap-only environment pays nothing.
        let integer_callees = contracts
            .functions()
            .iter()
            .filter(|(_, contract)| {
                contract.result_kind.is_integer()
                    && matches!(contract.result, ResourceReturnMode::Owned { .. })
            })
            .map(|(name, _)| name.as_str())
            .collect::<BTreeSet<_>>();
        if integer_callees.is_empty() && names.is_empty() {
            return Self { names };
        }
        let assignments = plain_assignments(body);
        for (destination, value) in &assignments {
            if let Some(callee) = called_after_cast(value)
                && integer_callees.contains(callee)
            {
                names.insert((*destination).to_owned());
            }
        }
        loop {
            let before = names.len();
            for (destination, value) in &assignments {
                if let [source] = strip_parentheses(value)
                    && names.contains(*source)
                {
                    names.insert((*destination).to_owned());
                }
            }
            if names.len() == before {
                break;
            }
        }
        Self { names }
    }

    pub(crate) fn contains(&self, name: &str) -> bool {
        self.names.contains(name)
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.names.is_empty()
    }
}

/// Every `NAME = VALUE` in `body` whose destination is a plain name, with
/// `VALUE` running to the next top-level `;`, `,` or unmatched `)`.
fn plain_assignments<'a>(body: &[&'a str]) -> Vec<(&'a str, Vec<&'a str>)> {
    let mut found = Vec::new();
    for index in 0..body.len() {
        if body[index] != "="
            || body.get(index + 1) == Some(&"=")
            || index == 0
            || matches!(
                body[index - 1],
                "=" | "!" | "<" | ">" | "+" | "-" | "*" | "/" | "%" | "&" | "|" | "^"
            )
        {
            continue;
        }
        let destination = body[index - 1];
        if !is_identifier(destination) {
            continue;
        }
        // A member name (`conn->fd = …`, `s.fd = …`) is not a variable.
        if index >= 2 && (body[index - 2] == "." || body[index - 2] == ">") {
            continue;
        }
        let mut depth = 0usize;
        let mut value = Vec::new();
        for text in &body[index + 1..] {
            match *text {
                "(" | "[" => depth += 1,
                ")" | "]" if depth == 0 => break,
                ")" | "]" => depth -= 1,
                ";" | "," if depth == 0 => break,
                _ => {}
            }
            value.push(*text);
        }
        found.push((destination, value));
    }
    found
}

/// Card **MK-EVENT-DESCRIPTOR-QUANT** (rule 38): the names of one body
/// assigned the result of a declaration whose descriptor cells are guarded by
/// a descriptor-valued witness (`#result != -1`). A validity test of such a
/// name decides the creation's class; it is never a tracked identity (the
/// result owns nothing), so the set feeds only the condition reader, never
/// the custody predicates of [`DescriptorNames`].
pub(crate) fn cell_witness_names(body: &[&str], contracts: &ResourceContracts) -> DescriptorNames {
    let witnessed = contracts
        .retaining()
        .iter()
        .filter(|(_, declaration)| {
            declaration.descriptor_cells.states.iter().any(|state| {
                state.value == qcore_schemas::descriptor_cell::CellValue::Fresh
                    && state.guard.is_some_and(|guard| guard.is_validity())
            })
        })
        .map(|(name, _)| name.as_str())
        .collect::<BTreeSet<_>>();
    if witnessed.is_empty() {
        return DescriptorNames::default();
    }
    let names = plain_assignments(body)
        .into_iter()
        .filter(|(_, value)| {
            called_after_cast(value).is_some_and(|callee| witnessed.contains(callee))
        })
        .map(|(destination, _)| destination.to_owned())
        .collect();
    DescriptorNames { names }
}

/// The callee of `VALUE` when it is `CALLEE ( … )`, optionally behind one
/// parenthesised cast.
fn called_after_cast<'a>(value: &[&'a str]) -> Option<&'a str> {
    let mut value = value;
    if value.first() == Some(&"(")
        && let Some(close) = value.iter().position(|text| *text == ")")
        && value.get(close + 2) == Some(&"(")
    {
        value = &value[close + 1..];
    }
    match value {
        [callee, "(", ..] if is_identifier(callee) => Some(callee),
        _ => None,
    }
}

/// **The** custody-carrying-value predicate: the first name in `value` that
/// may hold a tracked integer-carried identity, if any.
///
/// A member name is not the variable it spells (`conn->fd` names a field even
/// when the body also has a local `fd`), so a name after `.` or `->` never
/// counts. Every scalar route that would otherwise treat `value` as
/// custody-neutral must refuse when this returns `Some`: a tracked descriptor
/// stored into a field, an array element, a location reached through a
/// pointer or an object that outlives the call is an escape or a custody
/// transfer, never a no-op.
pub(crate) fn value_carries_descriptor<'a>(
    value: &[&'a str],
    names: &DescriptorNames,
) -> Option<&'a str> {
    if names.is_empty() {
        return None;
    }
    value.iter().enumerate().find_map(|(index, text)| {
        let member = index >= 1
            && (value[index - 1] == "."
                || (value[index - 1] == ">" && index >= 2 && value[index - 2] == "-"));
        (!member && names.contains(text)).then_some(*text)
    })
}

/// The descriptor name one complete statement **reads**, if any.
///
/// A plain-name destination of the statement's top-level assignment is a
/// write, not a read (`fd = -1;`), and so is a declaration of it
/// (`int fd = -1;`). Everything else — the value, an index, a member base, a
/// compound assignment's left operand — is a read.
pub(crate) fn statement_reads_descriptor<'a>(
    texts: &[&'a str],
    names: &DescriptorNames,
) -> Option<&'a str> {
    if names.is_empty() {
        return None;
    }
    let assignment = texts.iter().enumerate().position(|(index, text)| {
        *text == "="
            && texts.get(index + 1) != Some(&"=")
            && (index == 0
                || !matches!(
                    texts[index - 1],
                    "=" | "!" | "<" | ">" | "+" | "-" | "*" | "/" | "%" | "&" | "|" | "^"
                ))
    });
    let Some(equal) = assignment else {
        // A declaration with no initializer (`int g;`, `ssize_t n, m;`) names
        // its declarators; it reads nothing unless an array bound or a
        // parenthesised declarator could evaluate one.
        let declaration = texts.first().is_some_and(|first| {
            super::DECLARATION_SPECIFIERS.contains(first)
                || (is_identifier(first)
                    && texts.get(1).is_some_and(|second| is_identifier(second)))
        });
        if declaration && !texts.iter().any(|text| matches!(*text, "[" | "(")) {
            return None;
        }
        return value_carries_descriptor(texts, names);
    };
    if let Some(read) = value_carries_descriptor(&texts[equal + 1..], names) {
        return Some(read);
    }
    let destination = &texts[..equal];
    let written = direct_assignment_destination(destination);
    let plain = written
        .as_deref()
        .is_some_and(|place| is_identifier(place) && destination.last() == Some(&place));
    if plain {
        return None;
    }
    value_carries_descriptor(destination, names)
}

/// Whether `instruction`, the classification of one statement that reads a
/// tracked descriptor, states that read's effect exactly.
///
/// Admitted: a contracted call (whose own positions decide), a copy into a
/// plain name, a return, a release or escape, a null store, a validity test,
/// a scalar computation into a plain name that is not itself a descriptor
/// name (`n = fd + 1` creates and aliases no identity), and a refusal.
/// Everything else — the member, aggregate and indirect scalar stores, the
/// unbound scalar-field no-op, an opaque read — would treat the descriptor as
/// custody-neutral data, and is refused. The match is written so that a
/// **new** instruction variant is refused until someone decides otherwise.
pub(crate) fn admits_descriptor_read(
    instruction: &LegacyInstruction,
    names: &DescriptorNames,
) -> bool {
    let plain = |place: &str| is_identifier(place);
    match instruction {
        LegacyInstruction::ContractCall { .. }
        | LegacyInstruction::Release { .. }
        | LegacyInstruction::Escape { .. }
        | LegacyInstruction::EscapeIfOwned { .. }
        | LegacyInstruction::Clear { .. }
        | LegacyInstruction::BranchOnValidity { .. }
        | LegacyInstruction::Unknown { .. } => true,
        LegacyInstruction::Alias { destination, .. } => plain(destination),
        LegacyInstruction::ForgetScalar { destination } => {
            plain(destination) && !names.contains(destination)
        }
        LegacyInstruction::ExactFormalReturn { .. }
        | LegacyInstruction::SelfInitCall { .. }
        | LegacyInstruction::Acquire { .. }
        | LegacyInstruction::BranchOnNull { .. }
        | LegacyInstruction::BranchOnDistinct { .. }
        // Card MK-EVENT-CELL-CROSS-FUNCTION (rule 38c): refused as a read.
        | LegacyInstruction::CellProtocolStep { .. }
        | LegacyInstruction::OpaqueValue { .. }
        // Card VA-LOOP-AND-POINTER-ARG (rule 12c): a caller's pointer.
        | LegacyInstruction::VaArgPointer { .. }
        | LegacyInstruction::ScalarMemberStore { .. }
        // Card W43-INDIRECT-ACCESS-GENERAL's typed scalar store or read
        // through `*B`, `B[I]`, `B->F` or `S.F`: its value is custody-neutral
        // data by construction, which a tracked descriptor is not. Card
        // W44-INDIRECT-CHAIN-ACCESS adds no variant: its member chains ride
        // on this one (`chains`), and are refused here by the same decision.
        | LegacyInstruction::IndirectScalarAccess { .. }
        // Card W43-INDEX-TRAVERSAL's pointer step (`P++`, `P += E`,
        // `P = Q ± E`): it derives a pointer and never states what a tracked
        // descriptor read on the same line does, so it is refused
        // (integration of W43-DESCRIPTOR-RESOURCE-TRACKING).
        | LegacyInstruction::PointerStep { .. }
        // Card W44-INDIRECT-CALL-ARGUMENT-ACCESS's actual reads: the rule
        // declines every statement that reads a descriptor name, so this node
        // never meets one; refused anyway, since it reads its names as data.
        | LegacyInstruction::ArgumentReads { .. }
        // Card W43-INDEXED-RESOURCE-COLLECTIONS's element operations and
        // counted-loop nodes (integration of W43-DESCRIPTOR-RESOURCE-TRACKING).
        // An element store or release is lowered only for a statement that
        // reads no descriptor name (`Parser::element_statement` declines one,
        // keeping the classifier's refusal), and the loop nodes and element
        // null tests are never produced by this classifier. Each is refused
        // here anyway: an element operation reads its index as custody-neutral
        // data and moves custody into a heap collection, and states nothing
        // about a descriptor read on the same line.
        | LegacyInstruction::ElementStore { .. }
        | LegacyInstruction::ElementRelease { .. }
        | LegacyInstruction::BranchOnElementNull { .. }
        | LegacyInstruction::CountedLoopHeader { .. }
        | LegacyInstruction::CountedLoopStep { .. }
        | LegacyInstruction::AggregateStore { .. }
        // Card W45-SUB-OBJECT-STORES: a store into a member. A descriptor is
        // custody, never data, so a store whose statement reads one is never
        // this rule's (`Parser::sub_object_store_statement` declines it and
        // the gate's refusal stands); refused here anyway, since a scalar or
        // untracked store states nothing about a descriptor on its line.
        | LegacyInstruction::SubObjectStore { .. }
        // Card CG11-POINTER-VALUED-STORES: a pointer store is read only for a
        // value that carries no identity, so it states nothing about a
        // descriptor on its line.
        | LegacyInstruction::PointerStore { .. }
        | LegacyInstruction::TrackedSelfInit { .. }
        | LegacyInstruction::NoOp => false,
    }
}

/// What a validity test establishes on one edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ValidityFact {
    /// The identity is the failure sentinel on this edge: no resource exists.
    Invalid,
    /// The identity is a valid descriptor on this edge.
    Valid,
    /// Nothing is established (`fd > 0` false: the value is `0` or `-1`).
    Unrefined,
}

/// A condition that tests one descriptor name's validity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ValidityTest {
    pub(crate) source: String,
    pub(crate) when_true: ValidityFact,
    pub(crate) when_false: ValidityFact,
}

/// The comparison operators the reader recognises, as importer tokens.
const OPERATORS: [(&[&str], Comparison); 6] = [
    (&["<", "="], Comparison::LessEqual),
    (&[">", "="], Comparison::GreaterEqual),
    (&["=", "="], Comparison::Equal),
    (&["!", "="], Comparison::NotEqual),
    (&["<"], Comparison::Less),
    (&[">"], Comparison::Greater),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Comparison {
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Equal,
    NotEqual,
}

impl Comparison {
    /// `a OP b` is `b OP' a`.
    const fn mirrored(self) -> Self {
        match self {
            Self::Less => Self::Greater,
            Self::LessEqual => Self::GreaterEqual,
            Self::Greater => Self::Less,
            Self::GreaterEqual => Self::LessEqual,
            Self::Equal => Self::Equal,
            Self::NotEqual => Self::NotEqual,
        }
    }
}

/// The literal a validity test compares against: `0` or `-1`, possibly in
/// matched parentheses. Every other spelling is not a validity test.
fn sentinel_literal(tokens: &[&str]) -> Option<i8> {
    match strip_parentheses(tokens) {
        ["0"] => Some(0),
        ["-", "1"] => Some(-1),
        _ => None,
    }
}

/// What `NAME OP LITERAL` establishes on its true and false edges, for a
/// descriptor whose values are `-1` (the sentinel) or a valid number `>= 0`.
fn facts(comparison: Comparison, literal: i8) -> (ValidityFact, ValidityFact) {
    use ValidityFact::{Invalid, Unrefined, Valid};
    match (comparison, literal) {
        (Comparison::Less, 0) | (Comparison::LessEqual, -1) | (Comparison::Equal, -1) => {
            (Invalid, Valid)
        }
        (Comparison::GreaterEqual, 0) | (Comparison::Greater, -1) | (Comparison::NotEqual, -1) => {
            (Valid, Invalid)
        }
        // `fd > 0`: true is valid; false is `0` (valid) or `-1` (invalid).
        (Comparison::Greater, 0) => (Valid, Unrefined),
        (Comparison::LessEqual, 0) => (Unrefined, Valid),
        // `fd == 0` / `fd != 0` say nothing about validity: `0` is a valid
        // descriptor. `fd < -1` and friends are unreachable for the declared
        // value set, and are left unrefined rather than pruned.
        _ => (Unrefined, Unrefined),
    }
}

/// Read a controlling expression as a validity test of a descriptor name.
///
/// Recognised: `N OP L` and `L OP N` for `OP` one of `< <= > >= == !=` and `L`
/// one of `0`, `-1`; a bare `N` or `!N` (a null-shaped test, which refines
/// nothing for an integer); and `!( … )` of any of these. `None` for every
/// other expression, including a test of a name that is not a descriptor name.
pub(crate) fn validity_test(condition: &[&str], names: &DescriptorNames) -> Option<ValidityTest> {
    if names.is_empty() {
        return None;
    }
    let condition = strip_parentheses(condition);
    if let Some(rest) = condition.strip_prefix(&["!"]) {
        let inner = validity_test(rest, names)?;
        return Some(ValidityTest {
            source: inner.source,
            when_true: inner.when_false,
            when_false: inner.when_true,
        });
    }
    if let [name] = condition
        && names.contains(name)
    {
        return Some(ValidityTest {
            source: (*name).to_owned(),
            when_true: ValidityFact::Unrefined,
            when_false: ValidityFact::Unrefined,
        });
    }
    for index in 1..condition.len() {
        for (spelling, comparison) in OPERATORS {
            if !condition[index..].starts_with(spelling) {
                continue;
            }
            // `<` inside `<=` and `<<`, `>` inside `->` and `>=`: only the
            // longest operator at this position counts.
            let single = spelling.len() == 1;
            if single
                && (matches!(condition.get(index + 1), Some(&"=" | &"<" | &">"))
                    || matches!(condition[index - 1], "-" | "<" | ">"))
            {
                continue;
            }
            let left = strip_parentheses(&condition[..index]);
            let right = strip_parentheses(&condition[index + spelling.len()..]);
            let (name, comparison, other) = match (left, right) {
                ([name], other) if names.contains(name) => (*name, comparison, other),
                (other, [name]) if names.contains(name) => (*name, comparison.mirrored(), other),
                _ => return None,
            };
            // The other operand is one literal or one name. A compound
            // condition (`fd < 0 && x`) is not a validity test at all.
            let literal = sentinel_literal(other);
            if literal.is_none()
                && !matches!(other, [single] if is_identifier(single) || single.bytes().all(|b| b.is_ascii_digit()))
            {
                return None;
            }
            let (when_true, when_false) = match literal {
                Some(literal) => facts(comparison, literal),
                // A comparison with anything else (`fd > max`) is a scalar
                // decision that refines nothing.
                None => (ValidityFact::Unrefined, ValidityFact::Unrefined),
            };
            return Some(ValidityTest {
                source: name.to_owned(),
                when_true,
                when_false,
            });
        }
    }
    None
}

/// The fact edge `edge` (0 = true, 1 = false) of a validity branch carries.
pub(crate) const fn edge_fact(
    when_true: ValidityFact,
    when_false: ValidityFact,
    edge: usize,
) -> ValidityFact {
    if edge == 0 { when_true } else { when_false }
}

/// The callees whose contract in force is **declared**, as opposed to an
/// inferred W4 summary: the names a retained trusted source text declares.
///
/// A tracked descriptor handed to a `Borrowed` position is a *use* only where
/// a declaration states that the callee retains nothing. A validated W4
/// summary was derived with its integer formals untracked, so a body that
/// stored the formal into a member or a global still earned `Borrowed`; for a
/// descriptor that is exactly the launder constraint 10 closed for pointers,
/// and the call is refused instead.
pub(crate) fn declared_callees(contracts: &ResourceContracts) -> BTreeSet<String> {
    qcore_contract::contract_loader::declared_abi_signatures_of(contracts)
        .map(|signatures| signatures.into_keys().collect())
        .unwrap_or_default()
}

/// The static kind of every non-heap site `nodes` can create.
///
/// A call's produced sites are its result's (when it has an owned or
/// reallocated result) followed by one per `Out` parameter in index order —
/// the order `apply_contract_call` consumes them in.
pub(crate) fn site_kinds<'a>(
    instructions: impl IntoIterator<Item = &'a LegacyInstruction>,
) -> BTreeMap<usize, ResourceKind> {
    let mut kinds = BTreeMap::new();
    for instruction in instructions {
        // Card REFCOUNT-RESOURCE-KIND (rule 13b): the family a keep mints.
        if let Some((site, kind)) = super::reference_route::site_kind(instruction) {
            kinds.insert(site, kind);
        }
        let LegacyInstruction::ContractCall {
            contract,
            produced_sites,
            ..
        } = instruction
        else {
            continue;
        };
        let mut sites = produced_sites.iter().copied();
        if matches!(
            contract.result,
            ResourceReturnMode::Owned { .. } | ResourceReturnMode::Reallocated { .. }
        ) && let Some(site) = sites.next()
            && !contract.result_kind.is_heap()
        {
            kinds.insert(site, contract.result_kind);
        }
        for (index, mode) in contract.parameters.iter().enumerate() {
            if matches!(mode, ResourceParameterMode::Out { .. })
                && let Some(site) = sites.next()
            {
                let kind = contract.parameter_kind(index);
                if !kind.is_heap() {
                    kinds.insert(site, kind);
                }
            }
        }
    }
    kinds
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(list: &[&str]) -> DescriptorNames {
        DescriptorNames {
            names: list.iter().map(|name| (*name).to_owned()).collect(),
        }
    }

    fn test(source: &str) -> Option<(String, ValidityFact, ValidityFact)> {
        let texts = source.split_whitespace().collect::<Vec<_>>();
        validity_test(&texts, &names(&["fd"])).map(|t| (t.source, t.when_true, t.when_false))
    }

    #[test]
    fn exact_tests_decide_both_edges() {
        use ValidityFact::{Invalid, Valid};
        for (source, expected) in [
            ("fd < 0", (Invalid, Valid)),
            ("fd < = - 1", (Invalid, Valid)),
            ("fd = = - 1", (Invalid, Valid)),
            ("- 1 = = fd", (Invalid, Valid)),
            ("0 > fd", (Invalid, Valid)),
            ("fd ! = - 1", (Valid, Invalid)),
            ("fd > = 0", (Valid, Invalid)),
            ("fd > - 1", (Valid, Invalid)),
            ("0 < = fd", (Valid, Invalid)),
            ("( fd ) < ( 0 )", (Invalid, Valid)),
            ("! ( fd < 0 )", (Valid, Invalid)),
        ] {
            assert_eq!(
                test(source),
                Some(("fd".to_owned(), expected.0, expected.1)),
                "{source}"
            );
        }
    }

    #[test]
    fn zero_is_a_valid_descriptor_so_null_shaped_tests_refine_nothing() {
        use ValidityFact::{Unrefined, Valid};
        assert_eq!(test("fd > 0"), Some(("fd".to_owned(), Valid, Unrefined)));
        assert_eq!(test("fd < = 0"), Some(("fd".to_owned(), Unrefined, Valid)));
        for source in ["fd", "! fd", "fd = = 0", "fd ! = 0", "fd > max", "fd < 2"] {
            assert_eq!(
                test(source),
                Some(("fd".to_owned(), Unrefined, Unrefined)),
                "{source}"
            );
        }
    }

    #[test]
    fn other_names_and_shapes_are_not_validity_tests() {
        assert_eq!(test("other < 0"), None);
        assert_eq!(test("fd + 1 < 0"), None);
        assert_eq!(test("fd < 0 & & x"), None);
        // A negative operand other than the sentinel is not one literal.
        assert_eq!(test("fd < - 2"), None);
        let empty = DescriptorNames::default();
        assert_eq!(validity_test(&["fd", "<", "0"], &empty), None);
    }

    #[test]
    fn the_predicate_ignores_member_names_and_sees_every_other_read() {
        let set = names(&["fd"]);
        assert_eq!(
            value_carries_descriptor(&["conn", "-", ">", "fd"], &set),
            None
        );
        assert_eq!(value_carries_descriptor(&["s", ".", "fd"], &set), None);
        assert_eq!(value_carries_descriptor(&["fd"], &set), Some("fd"));
        assert_eq!(
            value_carries_descriptor(&["fd", "+", "1"], &set),
            Some("fd")
        );
        assert_eq!(
            value_carries_descriptor(&["(", "int", ")", "fd"], &set),
            Some("fd")
        );
        assert_eq!(
            value_carries_descriptor(&["fd"], &DescriptorNames::default()),
            None
        );
    }

    #[test]
    fn a_plain_destination_is_a_write_and_everything_else_a_read() {
        let set = names(&["fd"]);
        let reads = |source: &str| {
            let texts = source.split_whitespace().collect::<Vec<_>>();
            statement_reads_descriptor(&texts, &set).map(str::to_owned)
        };
        assert_eq!(reads("fd = - 1"), None);
        assert_eq!(reads("int fd = - 1"), None);
        assert_eq!(reads("conn - > fd = fd"), Some("fd".to_owned()));
        assert_eq!(reads("* p = fd"), Some("fd".to_owned()));
        assert_eq!(reads("a [ i ] = fd"), Some("fd".to_owned()));
        assert_eq!(reads("a [ fd ] = 0"), Some("fd".to_owned()));
        assert_eq!(reads("fd + = 1"), Some("fd".to_owned()));
        assert_eq!(reads("n = fd + 1"), Some("fd".to_owned()));
        assert_eq!(reads("( void ) fd"), Some("fd".to_owned()));
        assert_eq!(reads("int fd"), None);
        assert_eq!(reads("int g , fd"), None);
        assert_eq!(reads("ssize_t fd"), None);
        assert_eq!(reads("int a [ fd ]"), Some("fd".to_owned()));
    }

    /// Card W44-ASSIGNMENT-DESTINATION-SUFFIX-PLACE: the destination reader
    /// used to name `conns [ fd ] . flags` as `flags`, which this guard read as
    /// a plain-name write, so the index read of `fd` went unseen. A member
    /// store's last name is not a written variable.
    #[test]
    fn an_index_read_under_a_member_store_is_a_read() {
        let set = names(&["fd"]);
        let reads = |source: &str| {
            let texts = source.split_whitespace().collect::<Vec<_>>();
            statement_reads_descriptor(&texts, &set).map(str::to_owned)
        };
        assert_eq!(reads("conns [ fd ] . flags = 0"), Some("fd".to_owned()));
        assert_eq!(reads("( * fds ) . fd = 0"), None);
        assert_eq!(reads("struct k_conn * fd = 0"), None);
    }
}
