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
//!    declaration was read, and `FIELD` is exactly one of three classes: a
//!    scalar member ([`TypedAccessModel::scalar_field`]), a member declared
//!    with exactly one `*` over a specifier list the reader resolved
//!    ([`TypedAccessModel::single_pointer_field`]), or — card
//!    **ALIAS-AGGREGATE** — a member declared exactly `void *`
//!    ([`TypedAccessModel::void_pointer_field`]). Unions, bitfields, arrays,
//!    `volatile`/`_Atomic` members, aggregate sub-objects, function pointers,
//!    `const void *`, `void **`, multi-star members and unresolved typedefs
//!    all refuse.
//!
//! **On the stored value** (decided by [`crate::legacy_c`] against
//! [`classify_value`]): a scalar expression into a scalar field, the null
//! pointer constant into a pointer field, a declared pointer variable of
//! *exactly the field's declared record-pointer type* into that field, a
//! declared record-pointer variable of **any** tag into a `void *` field, or
//! the result of a contracted call.
//!
//! ## The opaque member (card ALIAS-AGGREGATE)
//!
//! A `void *` member is admitted as a **store destination only**. Three facts
//! make the asymmetry the sound one, and each is pinned by a test:
//!
//! * a store replaces exactly one pointer value, so the link `(owner, field) ->
//!   child` remains single-valued — the property every use of a field link
//!   depends on. Nothing in that machinery reads the member's declared type:
//!   escape cascades by identity, overwrite replaces by place, and release
//!   deliberately does not cascade (**P-B08-SHALLOW**, unchanged);
//! * a *read* is refused ([`bound_read_actuals`]), because `void *` names no
//!   pointee and there is therefore nothing to match against the callee's
//!   declared parameter type. Admitting one would hand a callee an identity on
//!   a type premise this reader cannot state;
//! * the stored value must still be a **declared record pointer**, so the
//!   identity written into the member is one this projection can name. A
//!   scalar, a cast, a call result that is not contracted, an address-of and
//!   every opaque local keep the existing refusal.
//!
//! The measured motivation is recorded in `docs/reports/remaining-alias-aggregate/`:
//! on the production corpus `struct flb_input_chunk` declares `void *chunk`,
//! not `struct cio_chunk *chunk` as the reduced O04 fixture prelude does, so
//! card O04-B08's flagship store never reached production at all.
//!
//! **Flow-sensitively**, at the store itself, [`crate::legacy_c`] additionally
//! requires the aggregate to resolve to exactly one **live** identity this
//! body acquired — never a caller-held one — so a store through a released,
//! escaped, null, unknown or caller-owned aggregate refuses. That is the C00
//! premise in its store direction and it is not weakened here.
//!
//! ## What an admitted bound *read* means (card O04-READ-BOUND)
//!
//! [`bound_read_actuals`] is the read half of the same premise set. Card C02-P
//! admits a member read only through a base the projection **never** binds; a
//! read through a base this body *did* bind stayed refused, which was the named
//! blocker `O04-B08-READ-BOUND-BASE`. It is now admitted for exactly one shape —
//! a whole `Borrowed` actual written `BASE - > FIELD` on a [`store_bases`] base
//! whose member is a single-star pointer — and only when [`crate::legacy_c`]'s
//! transfer function additionally finds, *here*, that the aggregate resolves to
//! one live identity this body acquired and that the member carries the exact
//! field link an admitted store recorded. Everything else keeps its refusal.
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
    /// The strict subset of [`Self::admissible`] that satisfies premise **F**
    /// of card O04-B09B10: no callee of this body can reach the aggregate at
    /// all, except through an argument that is exactly one admitted
    /// `BASE - > FIELD` member read. See [`insulated_bases`].
    insulated: BTreeSet<String>,
}

impl AggregateStoreBases {
    pub(crate) fn contains(&self, name: &str) -> bool {
        self.admissible.contains(name)
    }

    /// Whether premise **F** holds for `name`: every occurrence of it inside
    /// any call's argument list is exactly one admitted member read, so no
    /// callee ever receives the aggregate, its address, or the address of one
    /// of its members. Only then may a member read of `name` occupy a position
    /// that *takes* custody (card O04-B09B10).
    pub(crate) fn is_insulated(&self, name: &str) -> bool {
        self.insulated.contains(name)
    }

    #[cfg(test)]
    pub(crate) fn names(&self) -> Vec<&str> {
        self.admissible.iter().map(String::as_str).collect()
    }

    #[cfg(test)]
    pub(crate) fn insulated_names(&self) -> Vec<&str> {
        self.insulated.iter().map(String::as_str).collect()
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
    /// Card **ALIAS-AGGREGATE**: whether the field is declared exactly
    /// `void *`. Such a member is a legitimate *store* destination — it holds
    /// one pointer value, so a store replaces exactly one identity — but it is
    /// never a legitimate bound *read*, because `void *` names no pointee and
    /// nothing can match it against a callee's declared parameter type.
    /// [`bound_read_actuals`] refuses it explicitly.
    pub(crate) opaque_pointer: bool,
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
    parameters: &[&str],
    formal_parameters: &BTreeSet<String>,
    contracts: &ResourceContracts,
    model: &TypedAccessModel,
    types: &FunctionTypes,
) -> AggregateStoreBases {
    if !types.is_readable() {
        return AggregateStoreBases::default();
    }
    // Rule 37 (its package, card RULE37-PILOT): only spans the existing
    // checked non-VLA reader admits, and only while rule 37 is on, make a
    // mention unevaluated. They supply neither an owner identity nor
    // non-nullness; off, the scan counts every mention as before rule 37.
    let unevaluated = crate::result_constructor_rule::exempt_operand_spans(model, parameters, body);
    let mut candidates = BTreeSet::new();
    let mut refused = BTreeSet::new();
    let mut assignments: Vec<&str> = Vec::new();
    for index in 0..body.len() {
        let text = body[index];
        if !is_name(text)
            || unevaluated
                .iter()
                .any(|(start, end)| (*start..*end).contains(&index))
        {
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
    // Card W47-LEXICAL-CLOSURE-NESTED-ASSIGNMENT (rule 10o): every simple
    // assignment wherever it stands, a parenthesised destination included, and
    // every parenthesised write.
    let every = crate::lexical_nested_assignment::assignments(body);
    let parenthesised = crate::lexical_nested_assignment::parenthesised_writes(body);
    for name in &candidates {
        let bindings = assignments.iter().filter(|other| *other == name).count();
        let every_binding = every
            .iter()
            .filter(|assignment| assignment.destination == name)
            .count();
        if bindings != 1
            || every_binding != 1
            || parenthesised.contains(name)
            || !acquired.contains(name)
        {
            refused.insert(name.clone());
        }
    }
    // Premise 5.
    refuse_aliased_sources(body, &mut refused);
    // Rule 10o: a nested, later-declarator or parenthesised alias `x = BASE`.
    for assignment in &every {
        if let [name] = strip_parentheses(&body[assignment.value.clone()])
            && is_name(name)
        {
            refused.insert((*name).to_owned());
        }
    }
    refuse_rebinding_arguments(body, contracts, &mut refused);
    let admissible: BTreeSet<String> = candidates.difference(&refused).cloned().collect();
    let insulated = insulated_bases(body, &admissible, contracts, model, types, &unevaluated);
    AggregateStoreBases {
        admissible,
        insulated,
    }
}

/// Premise **F** of card O04-B09B10, the frame condition a *custody-taking*
/// member read needs and a borrowed one does not.
///
/// An admitted store records a field link, and a later read of that member
/// claims the link still describes what the member holds. A **borrowed** read
/// may claim that on a stale link without harm: borrowing is the identity
/// effect on every disposition, so at worst the callee borrows a pointer the
/// projection has misnamed. A read at a position that *releases* or *consumes*
/// cannot: if some callee rewrote the member since the store, the released
/// pointer is not the linked identity, and recording the release against the
/// link would both discharge an obligation nothing discharged and hide a real
/// one. Card O04-READ-BOUND refused every such position outright and recorded
/// the gap as `O04-READ-BOUND-CUSTODY`.
///
/// This is the missing premise, decided syntactically over the whole body. A
/// base is *insulated* when no **contracted** call of the body can leave the
/// aggregate both live and rewritable. Only three argument shapes mentioning
/// the base are permitted, and each has its own reason:
///
/// * exactly `BASE - > FIELD` — a *read* of a member cannot write one;
/// * exactly `& BASE - > FIELD` where `FIELD` is declared with no `*`
///   ([`TypedAccessModel::addressable_field`], the member-address rule's own
///   premise) — the callee receives an interior pointer to a sub-object whose
///   storage C's object model keeps disjoint from every linkable member;
/// * exactly `BASE` at a declared `Released` or `Consumed` position — after
///   such a call the identity is no longer live, so no later read through the
///   aggregate can resolve at all. This is what lets a body hand its aggregate
///   to its own destructor without losing the premise.
///
/// Every other shape — the aggregate whole at a `Borrowed` position above all —
/// refuses the base.
///
/// **Uncontracted** calls need no premise here, exactly as [`store_bases`]'s
/// premise 5 already argues for its own case: such a statement is refused, and
/// a refused statement poisons every pointer on its path. If it precedes the
/// read, the read fails its flow-sensitive owner check; if it follows, it
/// cannot have changed what the read saw. Statements that are refused for any
/// other reason — a `&` on a *pointer* member, a member buried in a larger
/// expression — are covered by the same argument.
///
/// Together with premises 4 and 5 — the pointer is never address-taken whole
/// and never denoted by a second declared name — the only writes to
/// `BASE->FIELD` that can occur before an admitted read are the admitted stores
/// this module already models, under the C effective types premise the whole
/// module inherits.
fn insulated_bases(
    body: &[&str],
    admissible: &BTreeSet<String>,
    contracts: &ResourceContracts,
    model: &TypedAccessModel,
    types: &FunctionTypes,
    unevaluated: &[(usize, usize)],
) -> BTreeSet<String> {
    let mut insulated = admissible.clone();
    for index in 0..body.len().saturating_sub(1) {
        if !is_name(body[index]) || body[index + 1] != "(" {
            continue;
        }
        let Some(contract) = contracts.contract(body[index]) else {
            continue;
        };
        let Some(arguments) = argument_spans(body, index + 1) else {
            // An argument list this reader cannot split may hide any shape;
            // refuse every candidate that occurs in the rest of the body.
            insulated.retain(|name| !body[index..].contains(&name.as_str()));
            continue;
        };
        for (position, (start, end)) in arguments.iter().copied().enumerate() {
            let terminal = matches!(
                contract.parameters.get(position),
                Some(
                    ResourceParameterMode::Released { .. } | ResourceParameterMode::Consumed { .. }
                )
            );
            insulated.retain(|name| {
                if !crate::sizeof_operand::evaluated(&body[start..end], start, unevaluated).any(|text| text == name.as_str()) {
                    return true;
                }
                match &body[start..end] {
                    [base, "-", ">", field] => base == name && is_name(field),
                    ["&", base, "-", ">", field] => {
                        base == name
                            && is_name(field)
                            && matches!(types.declared_type(base), Some(DeclaredType::RecordPointer(tag))
                                if model.addressable_field(tag, field))
                    }
                    [base] => terminal && base == name,
                    _ => false,
                }
            });
        }
    }
    insulated
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
        // Card CG10-COMMON-TYPED-PLACE (W44-AGGREGATE-PREMISE3-MEMBER-NAME):
        // a name after `.` or `->` is a member, never a binding of the
        // same-named local. Refusing direction only: fewer acquisitions.
        let before = |back: usize| {
            equal
                .checked_sub(back)
                .and_then(|at| statement.get(at))
                .copied()
        };
        if before(2) == Some(".") || (before(2) == Some(">") && before(3) == Some("-")) {
            continue;
        }
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
    // Card ALIAS-AGGREGATE: exactly `void *`, disjoint from both of the above
    // by construction — `void` classifies `Opaque`, and a scalar member has no
    // star at all.
    let opaque = model.void_pointer_field(tag, field);
    // A member is classified as exactly one of the three, by construction. The
    // sum is checked rather than assumed: a member the reader placed in two
    // classes would be a defect in the model, and refusing it is fail-closed.
    if usize::from(scalar) + usize::from(pointer) + usize::from(opaque) != 1 {
        return None;
    }
    Some(AdmittedStore {
        place: format!("{base}->{field}"),
        pointer_field: pointer || opaque,
        pointee_record: model
            .record_pointer_field(tag, field)
            .map(ToOwned::to_owned),
        opaque_pointer: opaque,
    })
}

/// One admitted **bound read** of an aggregate member (card O04-READ-BOUND).
///
/// The token span is the whole call argument the read occupies, and `place` is
/// the canonical place `BASE->FIELD` an admitted store bound.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BoundFieldRead {
    /// Inclusive start index of the argument in the statement's tokens.
    pub(crate) start: usize,
    /// Exclusive end index.
    pub(crate) end: usize,
    /// The canonical place, `BASE->FIELD`.
    pub(crate) place: String,
    /// Whether the declared position *takes* custody of what it reads, so the
    /// call transfers the linked child out of the aggregate (card O04-B09B10).
    /// `false` is card O04-READ-BOUND's borrowed read, which discharges
    /// nothing.
    pub(crate) custody: bool,
}

/// One admitted **member-address actual** (card O04-B09B10, O00 blockers
/// **B09**/**B10**): a whole `Borrowed` argument written `& BASE - > FIELD`
/// where `FIELD` is declared with no `*`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MemberAddressActual {
    /// Inclusive start index of the argument in the statement's tokens.
    pub(crate) start: usize,
    /// Exclusive end index.
    pub(crate) end: usize,
    /// The base name when the base is one *this* body bound through
    /// [`store_bases`], so [`crate::legacy_c`]'s transfer function must find it
    /// live here. `None` for a C02-P untracked base, which by premise carries
    /// no tracked identity at all.
    pub(crate) bound_owner: Option<String>,
}

/// The admitted member-address actuals of the call whose callee name is at
/// `call_index`.
///
/// The blocked statements card O04-READ-BOUND left on the constructor's success
/// exit are `msgpack_packer_init(&ic->mp_pck, ic, …)`,
/// `mk_list_add(&ic->_head, &in->chunks)` and
/// `pthread_mutex_lock(&in->metrics_chunk_lock)`: each hands a callee the
/// address of a *member*, which the lexical gate refuses because the argument
/// contains `->`. This function supplies the missing typed premise for exactly
/// that shape.
///
/// Conjunctive, and every conjunct is checked against declarations read from
/// this same token stream:
///
/// 1. the callee has a contract in force and the argument index it covers is
///    declared **`Borrowed`** — a borrowed actual discharges nothing, so an
///    admitted address can only withdraw a coverage refusal, never manufacture
///    a release. A variadic tail beyond the declared vector is never admitted;
/// 2. the argument is exactly `& BASE - > FIELD` and is the **whole** argument
///    — no cast, no parentheses, no arithmetic, no chain, no subscript;
/// 3. `BASE` is declared exactly `struct TAG *` and `TAG` is a `struct` defined
///    once in this unit whose every member declaration was read;
/// 4. `FIELD` is declared with **no** `*`
///    ([`TypedAccessModel::addressable_field`]). A pointer member is refused
///    outright: its address is a handle with which the callee could overwrite
///    the very link this projection maintains. A non-pointer member is a
///    sub-object whose storage C's object model keeps disjoint from every
///    linkable member of the same `struct`, so no field link can be reached
///    through it;
/// 5. `BASE` is either a [`store_bases`] base this body bound — in which case
///    [`crate::legacy_c`] additionally requires it to resolve *here* to one
///    live identity this body acquired, so the address of a member of a
///    released aggregate is never admitted — or a C02-P
///    [`crate::borrowed_actual::UntrackedBases`] name this body can never bind
///    to a tracked identity.
pub(crate) fn member_address_actuals(
    texts: &[&str],
    call_index: usize,
    contracts: &ResourceContracts,
    model: &TypedAccessModel,
    types: &FunctionTypes,
    bases: &AggregateStoreBases,
    untracked: &crate::borrowed_actual::UntrackedBases,
) -> Vec<MemberAddressActual> {
    if !types.is_readable() {
        return Vec::new();
    }
    let Some(callee) = texts.get(call_index).copied().filter(|text| is_name(text)) else {
        return Vec::new();
    };
    if texts.get(call_index + 1) != Some(&"(") {
        return Vec::new();
    }
    let Some(contract) = contracts.contract(callee) else {
        return Vec::new();
    };
    let Some(arguments) = argument_spans(texts, call_index + 1) else {
        return Vec::new();
    };
    let mut addresses = Vec::new();
    for (position, (start, end)) in arguments.iter().copied().enumerate() {
        if !matches!(
            contract.parameters.get(position),
            Some(ResourceParameterMode::Borrowed)
        ) {
            continue;
        }
        let ["&", base, "-", ">", field] = &texts[start..end] else {
            continue;
        };
        if !is_name(base) || !is_name(field) {
            continue;
        }
        let Some(DeclaredType::RecordPointer(tag)) = types.declared_type(base) else {
            continue;
        };
        if !model.record_definition_is_complete(tag) || !model.addressable_field(tag, field) {
            continue;
        }
        let bound_owner = if bases.contains(base) {
            Some((*base).to_owned())
        } else if untracked.never_bound(base) {
            None
        } else {
            continue;
        };
        addresses.push(MemberAddressActual {
            start,
            end,
            bound_owner,
        });
    }
    addresses
}

/// The admitted bound reads of the call whose callee name is at `call_index`.
///
/// The index is supplied by [`crate::legacy_c`] from its **own** call scan, so
/// the arguments this function reads are exactly the arguments the statement's
/// modeled `ContractCall` will read. Deriving it independently here would risk
/// excluding a span belonging to a different call from the lexical gate.
///
/// Card C02-P admits a member read only through a base the projection **never**
/// binds (its premise 4). This function is the complementary half: a member read
/// through a base this body *did* bind, through [`store_bases`]'s premises 1-5,
/// so the read denotes the identity an admitted store linked into that member.
///
/// Deliberately restricted to **`Borrowed`** argument positions and to
/// **single-star pointer** members:
///
/// * a borrowed actual discharges nothing, so admitting one can only withdraw a
///   coverage refusal — it can never manufacture a release. Every
///   custody-bearing position (`Owned`, `Consumed`, `Released`, `Stored`,
///   `Removed`) stays refused, because a contracted callee that merely
///   *borrows* the aggregate could rewrite the member invisibly and this
///   projection maintains no frame condition for that. Recorded as the named
///   boundary `O04-READ-BOUND-CUSTODY`;
/// * a scalar member can never hold an identity, so reading one at a borrowed
///   position denotes nothing this projection tracks and keeps its refusal.
///
/// The flow-sensitive half — the aggregate resolving *here* to exactly one live
/// identity this body acquired, and the member actually carrying the field link
/// an admitted store recorded — is required by [`crate::legacy_c`]'s transfer
/// function, exactly as the store direction requires it.
pub(crate) fn bound_read_actuals(
    texts: &[&str],
    call_index: usize,
    contracts: &ResourceContracts,
    model: &TypedAccessModel,
    types: &FunctionTypes,
    bases: &AggregateStoreBases,
) -> Vec<BoundFieldRead> {
    if !types.is_readable() {
        return Vec::new();
    }
    let Some(callee) = texts.get(call_index).copied().filter(|text| is_name(text)) else {
        return Vec::new();
    };
    if texts.get(call_index + 1) != Some(&"(") {
        return Vec::new();
    }
    let Some(contract) = contracts.contract(callee) else {
        return Vec::new();
    };
    let Some(arguments) = argument_spans(texts, call_index + 1) else {
        return Vec::new();
    };
    let mut reads = Vec::new();
    for (position, (start, end)) in arguments.iter().copied().enumerate() {
        // O04-B09B10: `Released` and `Consumed` join `Borrowed`, and only for a
        // base premise **F** insulates ([`insulated_bases`]). `Owned`,
        // `Stored`, `Removed` and `Out` keep their refusals: each either
        // rebinds the member or attaches it into a relation this module does
        // not maintain, and `store_bases` premise 5 already refuses a base that
        // occupies one.
        let custody = match contract.parameters.get(position) {
            Some(ResourceParameterMode::Borrowed) => false,
            Some(
                ResourceParameterMode::Released { .. } | ResourceParameterMode::Consumed { .. },
            ) => true,
            _ => continue,
        };
        let [base, "-", ">", field] = &texts[start..end] else {
            continue;
        };
        if custody && !bases.is_insulated(base) {
            continue;
        }
        let Some(store) = admitted_field(base, field, bases, model, types) else {
            continue;
        };
        if !store.pointer_field {
            continue;
        }
        // Card ALIAS-AGGREGATE: a `void *` member is a store destination only.
        // Its declared type names no pointee, so this rule has nothing to match
        // against the callee's declared parameter type and the read keeps its
        // existing refusal.
        if store.opaque_pointer {
            continue;
        }
        reads.push(BoundFieldRead {
            start,
            end,
            place: store.place,
            custody,
        });
    }
    reads
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
    if store.opaque_pointer {
        // Card ALIAS-AGGREGATE. A `void *` member names no pointee, so there is
        // no declared type to match exactly. What the store needs is that the
        // value is one *pointer identity* this projection can name, and the
        // only spellings admitted are declared record pointers — the same class
        // of variable the typed branch below admits. Scalars, opaque locals,
        // casts, calls, `&x`, arithmetic and every other right-hand side keep
        // their refusal, because `strip_parentheses` already required a single
        // name and `declared_type` commits to `RecordPointer` for nothing else.
        //
        // The member is never read back (`bound_read_actuals` refuses it), so
        // no declared type is ever matched against this link: the link is by
        // identity, and identity carries no type.
        if !matches!(
            types.declared_type(name),
            Some(&DeclaredType::RecordPointer(_))
        ) {
            return None;
        }
        return if untracked.never_bound(name) {
            Some(StoredValue::Untracked)
        } else {
            Some(StoredValue::Tracked((*name).to_owned()))
        };
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
         struct box * self ; struct child pad ; } ;";

    struct Facts {
        model: TypedAccessModel,
        types: FunctionTypes,
        untracked: crate::borrowed_actual::UntrackedBases,
        bases: AggregateStoreBases,
    }

    fn facts(declarations: &str, body_source: &str, formals: &[&str]) -> Facts {
        facts_with(
            declarations,
            body_source,
            formals,
            ResourceContracts::default(),
        )
    }

    fn facts_with(
        declarations: &str,
        body_source: &str,
        formals: &[&str],
        contracts: ResourceContracts,
    ) -> Facts {
        let unit = format!("{RECORDS} {declarations}");
        let unit_tokens = texts(&unit);
        let model = TypedAccessModel::read(&unit_tokens);
        let body = texts(body_source);
        let formal_set = formals
            .iter()
            .map(|name| (*name).to_owned())
            .collect::<BTreeSet<_>>();
        let types = FunctionTypes::read(&model, &[], &body, &BTreeSet::new());
        let untracked = crate::borrowed_actual::untracked_bases(
            &body,
            &BTreeSet::new(),
            &contracts,
            &BTreeSet::new(),
            &model,
            &types,
        );
        let bases = store_bases(&body, &[], &formal_set, &contracts, &model, &types);
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

    /// Card CG10-COMMON-TYPED-PLACE (W44-AGGREGATE-PREMISE3-MEMBER-NAME): a
    /// member name before `=` is not an acquisition of the same-named local.
    #[test]
    fn a_member_name_is_not_an_acquired_binding() {
        for body in [
            "struct box * kid ; struct box h ; h . kid = make ( ) ;",
            "struct box * kid ; struct box * h ; h -> kid = make ( ) ;",
        ] {
            assert!(!acquired_bindings(&texts(body)).contains("kid"), "{body}");
        }
        assert!(acquired_bindings(&texts("struct box * kid ; kid = make ( ) ;")).contains("kid"));
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

    /// **Deliberate pin update, card ALIAS-AGGREGATE.** `void * any` moves from
    /// refused to admitted as the third member class — an *opaque* pointer
    /// field, a store destination that is never a bound read. The load-bearing
    /// negative half is unchanged and is asserted here in full: `struct child
    /// ** deep` and an undeclared member stay refused, and every field's class
    /// is asserted rather than only its admission.
    #[test]
    fn a_scalar_a_single_pointer_and_an_opaque_pointer_field_are_admitted_and_nothing_else() {
        let facts = facts("", "struct box * b ; b = make ( ) ;", &[]);
        for (field, pointer, opaque) in [
            ("flag", false, false),
            ("kid", true, false),
            ("raw", true, false),
            ("any", true, true),
        ] {
            let store = admitted_field("b", field, &facts.bases, &facts.model, &facts.types)
                .unwrap_or_else(|| panic!("{field} admitted"));
            assert_eq!(store.pointer_field, pointer, "{field} pointer");
            assert_eq!(store.opaque_pointer, opaque, "{field} opaque");
        }
        for field in ["deep", "missing"] {
            assert!(
                admitted_field("b", field, &facts.bases, &facts.model, &facts.types).is_none(),
                "{field} refused"
            );
        }
    }

    /// The opaque member takes a declared record pointer of **any** tag and the
    /// null constant, and nothing else. `void *` names no pointee, so the exact
    /// declared-type match of the typed branch has nothing to match; what
    /// replaces it is the requirement that the value be one pointer identity
    /// this projection can name.
    #[test]
    fn an_opaque_pointer_field_takes_a_declared_record_pointer_and_the_null_constant() {
        let facts = facts(
            "",
            "struct box * b ; struct child * k ; unsigned long * u ; int n ; \
             b = make ( ) ; k = make ( ) ; u = make ( ) ; n = 0 ;",
            &[],
        );
        let store = admitted_field("b", "any", &facts.bases, &facts.model, &facts.types).unwrap();
        assert!(store.opaque_pointer);
        let null = |texts: &[&str]| texts == ["NULL"];
        let classify = |right: &[&str]| {
            classify_value(
                &store,
                right,
                &facts.model,
                &facts.types,
                &facts.untracked,
                null,
            )
        };
        assert_eq!(classify(&["k"]), Some(StoredValue::Tracked("k".to_owned())));
        assert_eq!(classify(&["NULL"]), Some(StoredValue::Null));
        // Not a declared record pointer: a scalar variable, a non-record
        // pointer, an address-of, a cast and a call all keep their refusal.
        for right in [
            vec!["n"],
            vec!["u"],
            vec!["&", "n"],
            vec!["(", "void", "*", ")", "k"],
            vec!["make", "(", ")"],
            vec!["0"],
        ] {
            assert_eq!(classify(&right), None, "{right:?}");
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

    fn borrowing_contract(arity: usize) -> ResourceContracts {
        use qcore_schemas::contract::{ResourceFunctionContract, ResourceReturnMode};
        let mut contracts = ResourceContracts::default();
        contracts.insert_contract(
            "sink".to_owned(),
            ResourceFunctionContract {
                parameters: vec![ResourceParameterMode::Borrowed; arity],
                result: ResourceReturnMode::None,
                ..ResourceFunctionContract::default()
            },
        );
        contracts.insert_contract(
            "eat".to_owned(),
            ResourceFunctionContract {
                parameters: vec![ResourceParameterMode::Released { nullable: false }],
                result: ResourceReturnMode::None,
                ..ResourceFunctionContract::default()
            },
        );
        contracts.insert_contract(
            "hold".to_owned(),
            ResourceFunctionContract {
                parameters: vec![ResourceParameterMode::Stored {
                    owner_parameter: 0,
                    nullable: false,
                }],
                result: ResourceReturnMode::None,
                ..ResourceFunctionContract::default()
            },
        );
        contracts.insert_contract(
            "own".to_owned(),
            ResourceFunctionContract {
                parameters: vec![ResourceParameterMode::Owned { nullable: false }],
                result: ResourceReturnMode::None,
                ..ResourceFunctionContract::default()
            },
        );
        contracts.insert_contract(
            "pull".to_owned(),
            ResourceFunctionContract {
                parameters: vec![ResourceParameterMode::Removed {
                    owner_parameter: 0,
                    nullable: false,
                }],
                result: ResourceReturnMode::None,
                ..ResourceFunctionContract::default()
            },
        );
        contracts
    }

    fn reads(body_source: &str, statement: &str) -> Vec<String> {
        let facts = facts("", body_source, &[]);
        let contracts = borrowing_contract(2);
        let tokens = texts(statement);
        bound_read_actuals(
            &tokens,
            0,
            &contracts,
            &facts.model,
            &facts.types,
            &facts.bases,
        )
        .into_iter()
        .map(|read| read.place)
        .collect()
    }

    const BOUND: &str = "struct box * b ; b = make ( ) ; b -> kid = 0 ;";

    #[test]
    fn a_whole_borrowed_pointer_member_actual_is_an_admitted_bound_read() {
        assert_eq!(
            reads(BOUND, "sink ( b -> kid , b -> other )"),
            ["b->kid", "b->other"]
        );
    }

    #[test]
    fn a_scalar_member_is_never_an_admitted_bound_read() {
        assert!(reads(BOUND, "sink ( b -> flag , b -> flag )").is_empty());
    }

    /// Card **ALIAS-AGGREGATE**: the store/read asymmetry, in one test. A
    /// `void *` member is an admitted store destination and is **never** an
    /// admitted bound read — in a borrowed position or a custody-taking one —
    /// while the typed member beside it in the same record and the same
    /// statement still reads. Nothing about the opaque member reaches a callee.
    #[test]
    fn an_opaque_pointer_member_stores_but_never_reads() {
        let facts = facts_with("", BOUND, &[], borrowing_contract(2));
        assert!(
            admitted_field("b", "any", &facts.bases, &facts.model, &facts.types)
                .is_some_and(|store| store.opaque_pointer)
        );
        let borrowed = reads(BOUND, "sink ( b -> any , b -> kid )");
        assert_eq!(borrowed, ["b->kid"], "{borrowed:?}");
        assert!(reads(BOUND, "eat ( b -> any )").is_empty());
    }

    /// Card O04-B09B10 supersedes card O04-READ-BOUND's blanket refusal of
    /// custody-bearing positions (`O04-READ-BOUND-CUSTODY`) for exactly two
    /// modes and exactly the bases premise **F** insulates. The two halves that
    /// were load-bearing are asserted separately below and are unchanged: a
    /// base any callee can reach still refuses
    /// (`a_custody_bearing_read_through_an_exposed_base_is_never_admitted`), and
    /// every *other* custody-bearing mode still refuses
    /// (`only_released_and_consumed_positions_take_custody_of_a_bound_read`).
    #[test]
    fn a_released_position_through_an_insulated_base_takes_custody() {
        let facts = facts("", BOUND, &[]);
        let contracts = borrowing_contract(2);
        let tokens = texts("eat ( b -> kid )");
        let reads = bound_read_actuals(
            &tokens,
            0,
            &contracts,
            &facts.model,
            &facts.types,
            &facts.bases,
        );
        assert_eq!(reads.len(), 1, "{reads:?}");
        assert_eq!(reads[0].place, "b->kid");
        assert!(reads[0].custody, "a released position takes custody");
        assert_eq!(facts.bases.insulated_names(), ["b"]);
    }

    /// Premise **F**: a base some callee can reach — passed whole here — can
    /// have been rewritten between the store and the read, so a custody-taking
    /// read of its member keeps card O04-READ-BOUND's refusal. The *borrowed*
    /// read of the same member stays admitted, because borrowing discharges
    /// nothing.
    #[test]
    fn a_custody_bearing_read_through_an_exposed_base_is_never_admitted() {
        let exposed = "struct box * b ; b = make ( ) ; b -> kid = 0 ; sink ( b , b ) ;";
        let contracts = borrowing_contract(2);
        let facts = facts_with("", exposed, &[], contracts.clone());
        assert_eq!(facts.bases.names(), ["b"]);
        assert!(facts.bases.insulated_names().is_empty());
        assert!(
            bound_read_actuals(
                &texts("eat ( b -> kid )"),
                0,
                &contracts,
                &facts.model,
                &facts.types,
                &facts.bases
            )
            .is_empty()
        );
        assert_eq!(
            bound_read_actuals(
                &texts("sink ( b -> kid , b -> other )"),
                0,
                &contracts,
                &facts.model,
                &facts.types,
                &facts.bases
            )
            .len(),
            2
        );
    }

    /// A `&` argument preserves premise **F** exactly when the member-address
    /// rule's own premise holds for it; a pointer member's address is a handle
    /// with which the callee could rewrite the very link the read believes.
    #[test]
    fn only_a_non_pointer_member_address_preserves_the_insulation_premise() {
        // `& b -> pad` on a *non-pointer* member preserves the premise: the
        // callee receives storage disjoint from every linkable member. The
        // pointer member `& b -> kid` does not.
        let permitted =
            "struct box * b ; b = make ( ) ; b -> kid = 0 ; sink ( & b -> pad , b -> kid ) ;";
        let contracts = borrowing_contract(2);
        let facts = facts_with("", permitted, &[], contracts.clone());
        assert_eq!(facts.bases.insulated_names(), ["b"]);
        let exposed =
            "struct box * b ; b = make ( ) ; b -> kid = 0 ; sink ( & b -> kid , b -> kid ) ;";
        let facts = facts_with("", exposed, &[], contracts);
        assert_eq!(facts.bases.names(), ["b"]);
        assert!(facts.bases.insulated_names().is_empty());
    }

    /// The member-address rule, with no Clang in the loop. `addresses` reports
    /// `(place tokens, bound owner)` for each admitted argument.
    fn addresses(
        declarations: &str,
        body_source: &str,
        formals: &[&str],
        statement: &str,
    ) -> Vec<(String, Option<String>)> {
        let facts = facts(declarations, body_source, formals);
        let contracts = borrowing_contract(2);
        let tokens = texts(statement);
        member_address_actuals(
            &tokens,
            0,
            &contracts,
            &facts.model,
            &facts.types,
            &facts.bases,
            &facts.untracked,
        )
        .into_iter()
        .map(|address| {
            (
                tokens[address.start..address.end].join(""),
                address.bound_owner,
            )
        })
        .collect()
    }

    #[test]
    fn a_whole_borrowed_member_address_on_a_bound_base_is_admitted() {
        assert_eq!(
            addresses("", BOUND, &[], "sink ( & b -> pad , & b -> flag )"),
            [
                ("&b->pad".to_owned(), Some("b".to_owned())),
                ("&b->flag".to_owned(), Some("b".to_owned())),
            ]
        );
    }

    #[test]
    fn a_member_address_on_an_untracked_base_carries_no_owner() {
        assert_eq!(
            addresses(
                "",
                "struct box * p ;",
                &["p"],
                "sink ( & p -> pad , & p -> flag )"
            ),
            [("&p->pad".to_owned(), None), ("&p->flag".to_owned(), None)]
        );
    }

    #[test]
    fn a_pointer_member_address_is_never_admitted() {
        for member in ["kid", "other", "raw", "any", "deep", "self", "missing"] {
            assert!(
                addresses(
                    "",
                    BOUND,
                    &[],
                    &format!("sink ( & b -> {member} , & b -> flag )")
                )
                .iter()
                .all(|(place, _)| place != &format!("&b->{member}")),
                "`&b->{member}` must keep its refusal"
            );
        }
    }

    #[test]
    fn a_custody_bearing_position_never_admits_a_member_address() {
        for callee in ["eat", "hold", "own", "pull"] {
            assert!(
                addresses("", BOUND, &[], &format!("{callee} ( & b -> pad )")).is_empty(),
                "`{callee}` must keep its refusal"
            );
        }
    }

    #[test]
    fn a_partial_or_continued_member_address_is_never_admitted() {
        for argument in [
            "& b -> pad . n",
            "( & b -> pad )",
            "& b -> pad + 1",
            "b -> pad",
            "& b",
            "& b -> kid -> n",
        ] {
            assert!(
                addresses("", BOUND, &[], &format!("sink ( {argument} , b )")).is_empty(),
                "`{argument}` must keep its refusal"
            );
        }
    }

    #[test]
    fn an_uncontracted_callee_admits_no_member_address() {
        let facts = facts("", BOUND, &[]);
        let tokens = texts("other ( & b -> pad )");
        assert!(
            member_address_actuals(
                &tokens,
                0,
                &ResourceContracts::default(),
                &facts.model,
                &facts.types,
                &facts.bases,
                &facts.untracked,
            )
            .is_empty()
        );
    }

    /// The other four custody-bearing modes keep their refusal in full: each
    /// either rebinds the member or attaches it into a relation this module
    /// does not maintain.
    #[test]
    fn only_released_and_consumed_positions_take_custody_of_a_bound_read() {
        let facts = facts("", BOUND, &[]);
        let contracts = borrowing_contract(2);
        for callee in ["hold", "own", "pull"] {
            let statement = format!("{callee} ( b -> kid )");
            let tokens = texts(&statement);
            assert!(
                bound_read_actuals(
                    &tokens,
                    0,
                    &contracts,
                    &facts.model,
                    &facts.types,
                    &facts.bases
                )
                .is_empty(),
                "`{callee}` must keep its refusal"
            );
        }
    }

    #[test]
    fn an_uncontracted_callee_admits_no_bound_read() {
        let facts = facts("", BOUND, &[]);
        let tokens = texts("other ( b -> kid )");
        assert!(
            bound_read_actuals(
                &tokens,
                0,
                &ResourceContracts::default(),
                &facts.model,
                &facts.types,
                &facts.bases
            )
            .is_empty()
        );
    }

    #[test]
    fn a_refused_base_admits_no_bound_read() {
        // Bound twice, so premise 3 refuses the base outright.
        let twice = "struct box * b ; b = make ( ) ; b = make ( ) ;";
        assert!(reads(twice, "sink ( b -> kid , b -> other )").is_empty());
    }

    #[test]
    fn a_refused_member_admits_no_bound_read() {
        for member in ["any", "deep", "missing"] {
            let statement = format!("sink ( b -> {member} , b -> {member} )");
            assert!(
                reads(BOUND, &statement).is_empty(),
                "`{member}` must admit no bound read"
            );
        }
    }

    #[test]
    fn a_partial_or_chained_actual_admits_no_bound_read() {
        for statement in [
            "sink ( b -> kid -> n , b -> other )",
            "sink ( & b -> kid , b -> other )",
            "sink ( b -> kid + 1 , b -> other )",
            "sink ( ( b -> kid ) , b -> other )",
        ] {
            assert_eq!(
                reads(BOUND, statement),
                ["b->other"],
                "only the whole-argument form is admitted in `{statement}`"
            );
        }
    }

    #[test]
    fn an_unreadable_function_admits_no_bound_read() {
        let facts = facts("", BOUND, &[]);
        let contracts = borrowing_contract(2);
        let tokens = texts("sink ( b -> kid , b -> other )");
        let unreadable = FunctionTypes::default();
        assert!(
            bound_read_actuals(
                &tokens,
                0,
                &contracts,
                &facts.model,
                &unreadable,
                &facts.bases
            )
            .is_empty()
        );
    }

    /// The caller supplies the call index, so a *second* call in the same
    /// statement admits nothing through this route: the reads always belong to
    /// the call the statement's modeled `ContractCall` describes.
    #[test]
    fn only_the_supplied_call_admits_bound_reads() {
        let facts = facts("", BOUND, &[]);
        let contracts = borrowing_contract(2);
        let tokens = texts("eat ( sink ( b -> kid , b -> other ) )");
        let outer = bound_read_actuals(
            &tokens,
            0,
            &contracts,
            &facts.model,
            &facts.types,
            &facts.bases,
        );
        assert!(outer.is_empty(), "the outer releasing call admits nothing");
        let inner = bound_read_actuals(
            &tokens,
            2,
            &contracts,
            &facts.model,
            &facts.types,
            &facts.bases,
        )
        .into_iter()
        .map(|read| read.place)
        .collect::<Vec<_>>();
        assert_eq!(inner, ["b->kid", "b->other"]);
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
