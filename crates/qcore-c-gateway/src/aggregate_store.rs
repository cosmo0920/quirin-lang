//! Typed aggregate field stores (O00 blocker **B08**) for the resource
//! projection.
//!
//! Card C02 admitted direct *scalar* field accesses through bases the
//! projection never binds, and card C02-P/O04-ARMS admitted pointer-field
//! *reads* through such bases. Both deliberately left the **destination** side
//! of the lexical gate in [`crate::legacy_c`] untouched: every statement whose
//! assignment destination contains `->`, `.` or `[` keeps its
//! `indirect memory access is outside the resource projection` refusal.
//!
//! That refusal is the binding constraint on the routes-mask arm of
//! `flb_input_chunk_create` (`docs/reports/remaining-o04-arms/README.md` §3.2):
//! the arm is reached only through `ic->chunk = chunk` and eleven sibling
//! stores, and it branches on `ic->routes_mask == NULL`.
//!
//! This module supplies the missing typed premise for exactly one destination
//! shape — a **single** `->` selection on a local aggregate pointer this
//! function acquired — and for the null test on such a field.
//!
//! ## What an admitted store means
//!
//! An admitted store binds the *place* `BASE->FIELD` in the abstract state, in
//! exactly the way a local variable is bound, and records a **field link**
//! `(owner site, field) -> child sites` in [`crate::legacy_c`]'s abstract
//! state. A field link is deliberately **not** the `CONTAINED` relation a
//! `Stored` contract parameter creates:
//!
//! * escaping the aggregate escapes every live linked child, so publishing an
//!   aggregate does not report the resources reachable from it as leaks;
//! * **releasing** the aggregate does *not* cascade. A declared
//!   `resource_released` release names exactly one identity, and a plain C
//!   field store carries no declared destructor obligation, so a child still
//!   live when its aggregate is released stays live and is reported as the
//!   residual it is. This is premise **P-B08-SHALLOW**, recorded in the card
//!   report and *not* proved here;
//! * overwriting a field replaces its link, so a second store never silently
//!   discharges the first resource.
//!
//! ## Premises
//!
//! **On the base** ([`store_bases`], computed once per function body from the
//! same preprocessed token stream the importer projects):
//!
//! 1. **Readable declarations.** [`FunctionTypes::is_readable`] holds; else
//!    nothing at all is admitted.
//! 2. **Typed local aggregate.** `BASE`'s declared type is exactly
//!    `struct TAG *` ([`DeclaredType::RecordPointer`]) and `BASE` is **not** a
//!    formal parameter, so its identity is one this body acquired.
//! 3. **Single acquired binding.** `BASE` is the destination of exactly one
//!    top-level simple assignment in the body, and that assignment's
//!    right-hand side *calls* something — so the aggregate is one this body
//!    obtained, not one it read out of somebody else's memory. An aggregate
//!    bound by a field read, by a copy of another pointer or by any other
//!    expression is refused, which is what keeps card C02-P's pin
//!    `a_store_through_the_field_between_the_read_and_the_call_is_never_admitted`
//!    refusing with its own existing diagnostic.
//! 4. **No escape of the pointer itself.** `BASE` is never address-taken whole,
//!    never incremented or compound-assigned, never dereferenced (`* BASE`),
//!    never subscripted (`BASE [`) and never selected with `.`.
//! 5. **No aliasing of the destination.** `BASE` is never the entire
//!    right-hand side of an assignment, so no other declared name denotes the
//!    same aggregate; and it never appears as a whole argument at an `Out`,
//!    `Stored` or `Removed` position of a *contracted* call, which could
//!    rebind or attach through it invisibly. An **un**contracted call needs no
//!    premise here: its statement is already refused, which poisons every
//!    pointer on that path, so the flow-sensitive owner check below fails
//!    closed at every later store.
//!
//! **On the field** ([`admitted_store`]):
//!
//! 6. The destination tokens are exactly `BASE - > FIELD` — one selection,
//!    written `->`, with nothing before or after it.
//! 7. `TAG` is a `struct` defined exactly once in the unit whose every member
//!    declaration was read, and `FIELD` is either a scalar member
//!    ([`TypedAccessModel::scalar_field`]) or a member declared with exactly
//!    one `*` over a specifier list the reader resolved
//!    ([`TypedAccessModel::single_pointer_field`]). Unions, bitfields, arrays,
//!    `volatile`/`_Atomic` members, aggregate sub-objects, function pointers,
//!    `void *`, multi-star members and unresolved typedefs all refuse.
//!
//! **On the stored value** (decided by [`crate::legacy_c`] against
//! [`classify_value`]): a scalar expression into a scalar field, the null
//! pointer constant into a pointer field, a declared pointer variable of
//! *exactly the field's declared record-pointer type* into that field, or the
//! result of a contracted call.
//!
//! **Flow-sensitively**, at the store itself, [`crate::legacy_c`] additionally
//! requires the aggregate to resolve to exactly one **live** identity this
//! body acquired — never a caller-held one — so a store through a released,
//! escaped, null, unknown or caller-owned aggregate refuses. That is the C00
//! premise in its store direction and it is not weakened here.
//!
//! Both premises card C02 records — declaration completeness and C effective
//! types — are inherited unchanged and are not proved here.

use std::collections::BTreeSet;

use qcore_schemas::contract::{ResourceContracts, ResourceParameterMode};

use crate::typed_field_access::{DeclaredType, FunctionTypes, TypedAccessModel, is_name};

/// The local aggregate pointers of one function body a store may target.
#[derive(Clone, Debug, Default)]
pub(crate) struct AggregateStoreBases {
    admissible: BTreeSet<String>,
}

impl AggregateStoreBases {
    pub(crate) fn contains(&self, name: &str) -> bool {
        self.admissible.contains(name)
    }

    #[cfg(test)]
    pub(crate) fn names(&self) -> Vec<&str> {
        self.admissible.iter().map(String::as_str).collect()
    }
}

/// One admitted aggregate store destination.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AdmittedStore {
    /// The canonical place, `BASE->FIELD`.
    pub(crate) place: String,
    /// Whether the field is declared with exactly one `*`. A scalar field can
    /// never hold a tracked identity, so only a pointer field may be linked.
    pub(crate) pointer_field: bool,
    /// The record tag the field points at, when the field is declared exactly
    /// `struct TAG *`. Used to require an exact declared-type match on a
    /// variable stored into it.
    pub(crate) pointee_record: Option<String>,
}

/// What an admitted store writes into the field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum StoredValue {
    /// A value carrying no tracked identity: a scalar expression into a scalar
    /// field, or a declared pointer this function never binds.
    Untracked,
    /// The null pointer constant, into a pointer field.
    Null,
    /// A declared pointer variable of exactly the field's declared type, which
    /// this function may bind to a tracked identity.
    Tracked(String),
}

/// The owner name of an admitted field place.
pub(crate) fn place_owner(place: &str) -> Option<&str> {
    place.find("->").map(|at| &place[..at])
}

/// The field name of an admitted field place.
pub(crate) fn place_field(place: &str) -> Option<&str> {
    place.find("->").map(|at| &place[at + 2..])
}

/// Whether `place` is a single-selection field place this module could have
/// produced. `BASE->F->G`, `BASE.f` and `*out` are all excluded.
pub(crate) fn is_field_place(place: &str) -> bool {
    match (place_owner(place), place_field(place)) {
        (Some(owner), Some(field)) => {
            is_name(owner) && is_name(field) && !field.contains("->") && !place.contains(['.', '['])
        }
        _ => false,
    }
}

/// The local aggregate pointers of `body` that premises 1-5 admit.
pub(crate) fn store_bases(
    body: &[&str],
    formal_parameters: &BTreeSet<String>,
    contracts: &ResourceContracts,
    model: &TypedAccessModel,
    types: &FunctionTypes,
) -> AggregateStoreBases {
    if !types.is_readable() {
        return AggregateStoreBases::default();
    }
    let mut candidates = BTreeSet::new();
    let mut refused = BTreeSet::new();
    let mut assignments: Vec<&str> = Vec::new();
    for index in 0..body.len() {
        let text = body[index];
        if !is_name(text) {
            continue;
        }
        let next = body.get(index + 1).copied();
        let after = body.get(index + 2).copied();
        let previous = index.checked_sub(1).and_then(|at| body.get(at)).copied();
        let before = index.checked_sub(2).and_then(|at| body.get(at)).copied();
        let selects = matches!(next, Some("-")) && after == Some(">");
        // Premise 2: a local whose declared type is `struct TAG *`.
        if !formal_parameters.contains(text)
            && matches!(types.declared_type(text), Some(DeclaredType::RecordPointer(tag))
                if model.record_definition_is_complete(tag))
        {
            candidates.insert(text.to_owned());
        }
        // Premise 4. A `*` immediately before the name is a dereference only
        // when what precedes *it* cannot end a type specifier or an operand:
        // `struct box * b ;` and `n * b` are not dereferences, `= * b`,
        // `( * b` and `; * b` are.
        let dereferenced = previous == Some("*")
            && !before.is_some_and(|text| {
                is_name(text)
                    || text.as_bytes().first().is_some_and(u8::is_ascii_digit)
                    || matches!(text, ")" | "]")
            });
        if (previous == Some("&") && !selects)
            || dereferenced
            || matches!(next, Some(".") | Some("["))
            || matches!(
                (before, previous),
                (Some("+"), Some("+")) | (Some("-"), Some("-"))
            )
            || matches!(
                (next, after),
                (Some("+"), Some("+")) | (Some("-"), Some("-"))
            )
        {
            refused.insert(text.to_owned());
            continue;
        }
        if matches!(
            next,
            Some("+")
                | Some("-")
                | Some("*")
                | Some("/")
                | Some("%")
                | Some("&")
                | Some("|")
                | Some("^")
        ) && after == Some("=")
        {
            refused.insert(text.to_owned());
            continue;
        }
        if matches!(
            (next, after),
            (Some("<"), Some("<")) | (Some(">"), Some(">"))
        ) && body.get(index + 3).copied() == Some("=")
        {
            refused.insert(text.to_owned());
            continue;
        }
        // Premise 3: count top-level simple assignments whose destination is
        // exactly this name.
        if next == Some("=")
            && after != Some("=")
            && !matches!(previous, Some("=") | Some("!") | Some("<") | Some(">"))
        {
            assignments.push(text);
        }
    }
    // Premise 3: exactly one binding, and that binding calls something.
    let acquired = acquired_bindings(body);
    for name in &candidates {
        let bindings = assignments.iter().filter(|other| *other == name).count();
        if bindings != 1 || !acquired.contains(name) {
            refused.insert(name.clone());
        }
    }
    // Premise 5.
    refuse_aliased_sources(body, &mut refused);
    refuse_rebinding_arguments(body, contracts, &mut refused);
    AggregateStoreBases {
        admissible: candidates.difference(&refused).cloned().collect(),
    }
}

/// Premise 3, second half: the names bound by an assignment whose right-hand
/// side calls something. An aggregate read out of another object's memory —
/// `storage = in->storage;` — is deliberately absent, so no store through it
/// is ever admitted.
fn acquired_bindings(body: &[&str]) -> BTreeSet<String> {
    let mut acquired = BTreeSet::new();
    for statement in statements(body) {
        let Some(equal) = simple_assignment(statement) else {
            continue;
        };
        let Some(destination) = equal
            .checked_sub(1)
            .and_then(|at| statement.get(at))
            .copied()
            .filter(|text| is_name(text))
        else {
            continue;
        };
        if equal != statement.len().saturating_sub(1)
            && statement[equal + 1..]
                .windows(2)
                .any(|pair| is_name(pair[0]) && pair[1] == "(")
        {
            acquired.insert(destination.to_owned());
        }
    }
    acquired
}

/// Premise 5, first half: a name that is the entire right-hand side of an
/// assignment may be denoted by a second declared name afterwards.
fn refuse_aliased_sources(body: &[&str], refused: &mut BTreeSet<String>) {
    for statement in statements(body) {
        let Some(equal) = simple_assignment(statement) else {
            continue;
        };
        let right = strip_parentheses(&statement[equal + 1..]);
        if let [name] = right
            && is_name(name)
        {
            refused.insert((*name).to_owned());
        }
    }
}

/// Premise 5, second half: a whole argument at an `Out`, `Stored` or `Removed`
/// position of a contracted call could rebind the pointer or attach through it
/// in a relation this module does not maintain.
fn refuse_rebinding_arguments(
    body: &[&str],
    contracts: &ResourceContracts,
    refused: &mut BTreeSet<String>,
) {
    for index in 0..body.len().saturating_sub(1) {
        if !is_name(body[index]) || body[index + 1] != "(" {
            continue;
        }
        let Some(contract) = contracts.contract(body[index]) else {
            continue;
        };
        let Some(arguments) = argument_spans(body, index + 1) else {
            continue;
        };
        for (position, (start, end)) in arguments.iter().copied().enumerate() {
            let rebinding = matches!(
                contract.parameters.get(position),
                Some(
                    ResourceParameterMode::Out { .. }
                        | ResourceParameterMode::Stored { .. }
                        | ResourceParameterMode::Removed { .. }
                )
            );
            if !rebinding {
                continue;
            }
            for text in &body[start..end] {
                if is_name(text) {
                    refused.insert((*text).to_owned());
                }
            }
        }
    }
}

/// Premises 6 and 7 over the tokens *before* the assignment's `=`.
pub(crate) fn admitted_store(
    left: &[&str],
    bases: &AggregateStoreBases,
    model: &TypedAccessModel,
    types: &FunctionTypes,
) -> Option<AdmittedStore> {
    let [base, "-", ">", field] = left else {
        return None;
    };
    admitted_field(base, field, bases, model, types)
}

/// The same premises over a stand-alone `BASE - > FIELD` access.
pub(crate) fn admitted_field(
    base: &str,
    field: &str,
    bases: &AggregateStoreBases,
    model: &TypedAccessModel,
    types: &FunctionTypes,
) -> Option<AdmittedStore> {
    if !is_name(base) || !is_name(field) || !bases.contains(base) {
        return None;
    }
    let Some(DeclaredType::RecordPointer(tag)) = types.declared_type(base) else {
        return None;
    };
    let scalar = model.scalar_field(tag, field);
    let pointer = model.single_pointer_field(tag, field);
    // A member is classified as exactly one of the two, by construction: a
    // scalar member has no star and a single-pointer member has one.
    if scalar == pointer {
        return None;
    }
    Some(AdmittedStore {
        place: format!("{base}->{field}"),
        pointer_field: pointer,
        pointee_record: model
            .record_pointer_field(tag, field)
            .map(ToOwned::to_owned),
    })
}

/// Classify the right-hand side of an admitted, call-free store.
///
/// `Some` only for the four value shapes the module header names; every other
/// right-hand side keeps its existing refusal.
pub(crate) fn classify_value(
    store: &AdmittedStore,
    right: &[&str],
    model: &TypedAccessModel,
    types: &FunctionTypes,
    untracked: &crate::borrowed_actual::UntrackedBases,
    is_null_constant: impl Fn(&[&str]) -> bool,
) -> Option<StoredValue> {
    if right.is_empty() {
        return None;
    }
    if !store.pointer_field {
        // A scalar member can never hold a tracked identity; the value must be
        // an admitted scalar expression so that nothing else hides in it.
        return crate::typed_field_access::is_scalar_only_noop(right, model, types)
            .then_some(StoredValue::Untracked);
    }
    if is_null_constant(right) {
        return Some(StoredValue::Null);
    }
    let [name] = strip_parentheses(right) else {
        return None;
    };
    if !is_name(name) {
        return None;
    }
    // An exact declared-type match: the field is `struct TAG *` and so is the
    // stored variable. A pointer field whose pointee this reader resolved to a
    // non-record type has no variable form admitted here.
    let pointee = store.pointee_record.as_deref()?;
    if types.declared_type(name) != Some(&DeclaredType::RecordPointer(pointee.to_owned())) {
        return None;
    }
    if untracked.never_bound(name) {
        return Some(StoredValue::Untracked);
    }
    Some(StoredValue::Tracked((*name).to_owned()))
}

fn strip_parentheses<'a, 'b>(mut texts: &'b [&'a str]) -> &'b [&'a str] {
    while texts.first() == Some(&"(") && texts.last() == Some(&")") {
        let inner = &texts[1..texts.len() - 1];
        let mut depth = 0usize;
        let mut balanced = true;
        for text in inner {
            match *text {
                "(" => depth += 1,
                ")" => {
                    if depth == 0 {
                        balanced = false;
                        break;
                    }
                    depth -= 1;
                }
                _ => {}
            }
        }
        if !balanced || depth != 0 {
            break;
        }
        texts = inner;
    }
    texts
}

/// Split a body into `;`-delimited statements at brace level.
fn statements<'a, 'b>(body: &'b [&'a str]) -> Vec<&'b [&'a str]> {
    let mut out = Vec::new();
    let mut start = 0usize;
    for (index, text) in body.iter().enumerate() {
        if matches!(*text, ";" | "{" | "}") {
            if index > start {
                out.push(&body[start..index]);
            }
            start = index + 1;
        }
    }
    if start < body.len() {
        out.push(&body[start..]);
    }
    out
}

/// The index of a top-level simple `=`.
fn simple_assignment(statement: &[&str]) -> Option<usize> {
    let mut depth = 0usize;
    for (index, text) in statement.iter().enumerate() {
        match *text {
            "(" | "[" => depth += 1,
            ")" | "]" => depth = depth.saturating_sub(1),
            "=" if depth == 0 => {
                let previous = index
                    .checked_sub(1)
                    .and_then(|at| statement.get(at))
                    .copied();
                let compound = matches!(
                    previous,
                    Some("=" | "!" | "<" | ">" | "+" | "-" | "*" | "/" | "%" | "&" | "|" | "^")
                );
                if !compound && statement.get(index + 1) != Some(&"=") {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

/// The `[start, end)` token spans of each argument of the call whose `(` is at
/// `open`.
fn argument_spans(body: &[&str], open: usize) -> Option<Vec<(usize, usize)>> {
    let mut spans = Vec::new();
    let mut depth = 0usize;
    let mut start = open + 1;
    for (index, text) in body.iter().enumerate().skip(open) {
        match *text {
            "(" | "[" => depth += 1,
            ")" | "]" => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    if index > start {
                        spans.push((start, index));
                    }
                    return Some(spans);
                }
            }
            "," if depth == 1 => {
                spans.push((start, index));
                start = index + 1;
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
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

    const RECORDS: &str = "struct child { int n ; } ; \
         struct box { int flag ; struct child * kid ; struct child * other ; \
         unsigned long * raw ; void * any ; struct child ** deep ; \
         struct box * self ; } ;";

    struct Facts {
        model: TypedAccessModel,
        types: FunctionTypes,
        untracked: crate::borrowed_actual::UntrackedBases,
        bases: AggregateStoreBases,
    }

    fn facts(declarations: &str, body_source: &str, formals: &[&str]) -> Facts {
        let unit = format!("{RECORDS} {declarations}");
        let unit_tokens = texts(&unit);
        let model = TypedAccessModel::read(&unit_tokens);
        let body = texts(body_source);
        let formal_set = formals
            .iter()
            .map(|name| (*name).to_owned())
            .collect::<BTreeSet<_>>();
        let types = FunctionTypes::read(&model, &[], &body, &BTreeSet::new());
        let contracts = ResourceContracts::default();
        let untracked = crate::borrowed_actual::untracked_bases(
            &body,
            &BTreeSet::new(),
            &contracts,
            &model,
            &types,
        );
        let bases = store_bases(&body, &formal_set, &contracts, &model, &types);
        Facts {
            model,
            types,
            untracked,
            bases,
        }
    }

    #[test]
    fn a_singly_bound_local_aggregate_pointer_is_an_admissible_base() {
        let facts = facts("", "struct box * b ; b = make ( ) ; b -> flag = 1 ;", &[]);
        assert_eq!(facts.bases.names(), vec!["b"]);
    }

    #[test]
    fn a_formal_is_never_an_admissible_base() {
        let facts = facts("", "struct box * b ; b = make ( ) ;", &["b"]);
        assert!(facts.bases.names().is_empty());
    }

    #[test]
    fn a_twice_bound_base_is_refused() {
        let facts = facts("", "struct box * b ; b = make ( ) ; b = make ( ) ;", &[]);
        assert!(facts.bases.names().is_empty());
    }

    #[test]
    fn an_unbound_base_is_refused() {
        let facts = facts("", "struct box * b ; b -> flag = 1 ;", &[]);
        assert!(facts.bases.names().is_empty());
    }

    #[test]
    fn an_address_taken_base_is_refused() {
        let facts = facts("", "struct box * b ; b = make ( ) ; f ( & b ) ;", &[]);
        assert!(facts.bases.names().is_empty());
    }

    #[test]
    fn a_copied_base_is_refused() {
        let facts = facts(
            "",
            "struct box * b ; struct box * c ; b = make ( ) ; c = b ;",
            &[],
        );
        assert!(!facts.bases.contains("b"));
    }

    #[test]
    fn a_dereferenced_or_subscripted_base_is_refused() {
        let star = facts("", "struct box * b ; b = make ( ) ; g ( * b ) ;", &[]);
        assert!(!star.bases.contains("b"));
        let index = facts("", "struct box * b ; b = make ( ) ; g ( b [ 0 ] ) ;", &[]);
        assert!(!index.bases.contains("b"));
    }

    #[test]
    fn a_scalar_and_a_single_pointer_field_are_admitted_and_nothing_else() {
        let facts = facts("", "struct box * b ; b = make ( ) ;", &[]);
        for (field, pointer) in [("flag", false), ("kid", true), ("raw", true)] {
            let store = admitted_field("b", field, &facts.bases, &facts.model, &facts.types)
                .unwrap_or_else(|| panic!("{field} admitted"));
            assert_eq!(store.pointer_field, pointer, "{field}");
        }
        for field in ["any", "deep", "missing"] {
            assert!(
                admitted_field("b", field, &facts.bases, &facts.model, &facts.types).is_none(),
                "{field} refused"
            );
        }
    }

    #[test]
    fn a_record_pointer_field_takes_only_an_exactly_typed_variable() {
        let facts = facts(
            "",
            "struct box * b ; struct child * k ; struct box * q ; b = make ( ) ; k = make ( ) ; q = make ( ) ;",
            &[],
        );
        let store = admitted_field("b", "kid", &facts.bases, &facts.model, &facts.types).unwrap();
        let null = |texts: &[&str]| texts == ["NULL"];
        assert_eq!(
            classify_value(
                &store,
                &["k"],
                &facts.model,
                &facts.types,
                &facts.untracked,
                null
            ),
            Some(StoredValue::Tracked("k".to_owned()))
        );
        assert_eq!(
            classify_value(
                &store,
                &["q"],
                &facts.model,
                &facts.types,
                &facts.untracked,
                null
            ),
            None
        );
        assert_eq!(
            classify_value(
                &store,
                &["NULL"],
                &facts.model,
                &facts.types,
                &facts.untracked,
                null
            ),
            Some(StoredValue::Null)
        );
    }

    #[test]
    fn a_pointer_field_never_takes_a_scalar_expression() {
        let facts = facts("", "struct box * b ; int n ; b = make ( ) ; n = 0 ;", &[]);
        let store = admitted_field("b", "kid", &facts.bases, &facts.model, &facts.types).unwrap();
        assert_eq!(
            classify_value(
                &store,
                &["n"],
                &facts.model,
                &facts.types,
                &facts.untracked,
                |texts| texts == ["NULL"]
            ),
            None
        );
    }

    #[test]
    fn field_places_split_exactly_once() {
        assert!(is_field_place("b->kid"));
        assert_eq!(place_owner("b->kid"), Some("b"));
        assert_eq!(place_field("b->kid"), Some("kid"));
        assert!(!is_field_place("b"));
        assert!(!is_field_place("*out"));
        assert!(!is_field_place("b.kid"));
        assert!(!is_field_place("b->kid->n"));
    }
}
