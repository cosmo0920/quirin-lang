//! Typed borrowed pointer-field actuals (C02-P) for the resource projection.
//!
//! Card C02 admitted one conjunctive whitelist of direct *scalar* field
//! accesses as custody-neutral no-ops, and explicitly left pointer fields
//! refused (`docs/reports/remaining-c02/README.md` §7). Card O00 measured the
//! consequence on the constructor-to-caller path it selected: the caller's
//! constructor call reads two pointer members of a local aggregate, so the
//! lexical gate in [`crate::legacy_c`] rejects the whole call statement
//! *before* the declared contract is ever consulted, `chunk` never binds, and
//! every cleanup arm downstream reports an unresolved alias (O00 §7 blocker
//! B01, probe pair O-A/O-B).
//!
//! This module supplies the missing typed premise for exactly two shapes, both
//! **reads**:
//!
//! * a **borrowed actual** — a whole argument of a contracted call that is
//!   exactly `base -> field` or `base . field`, at an argument index the
//!   contract in force declares [`ResourceParameterMode::Borrowed`];
//! * an **opaque field read** — an assignment whose entire right-hand side is
//!   exactly such an access and whose destination is a plain local name.
//!
//! Card O04-ARMS widens both shapes from one member selection to a **bounded
//! chain** of them (`base -> f1 -> f2`), under premise (3) below, which O04-ARMS widened.
//!
//! Nothing here relaxes the *destination* side of the lexical gate. A store
//! through `->`, `.` or `[` keeps its existing rejection and its existing
//! diagnostic, which is what makes premise (4) below hold by construction.
//!
//! ## Premises
//!
//! 1. **Typed base.** `base` is a direct identifier whose declared type is a
//!    single-star pointer to a record (for `->`) or that record itself (for
//!    `.`), read from the same preprocessed token stream the importer projects.
//! 2. **Complete non-union record.** The record is a `struct`, defined exactly
//!    once in the unit, and every one of its member declarations was read. The
//!    member is declared with no array modifier and no `volatile`/`_Atomic`,
//!    and is either a scalar or a pointer — never an aggregate sub-object.
//! 3. **Bounded, fully typed chain** (widened by O04-ARMS). The access is
//!    `base` followed by at most [`ACCESS_CHAIN_LIMIT`] member selections. Every selection *before the
//!    last* is written `->` and its member must be declared *exactly*
//!    `struct POINTEE *` — [`TypedAccessModel::record_pointer_field`] — so the
//!    walk re-enters the same declaration model at `POINTEE`, where an
//!    incomplete, ambiguous or `union` record refuses. The last selection must
//!    satisfy premise (2) and must not continue into `->`, `.`, `[` or `(`. A
//!    `.` may appear only as the first selection, on a record-valued base:
//!    continuing with `.` would cross a nested aggregate sub-object, refused.
//!
//!    A chain reads through intermediate *places* this function never names.
//!    That is the same exposure premise (4) already accepts at depth one and
//!    that C02-P recorded as **C02-P-ALIAS**: two distinct declared names that
//!    may alias are unrelated by this name-based projection. Nothing new is
//!    assumed — an intermediate place can only have been released in this
//!    function through some local bound by an *opaque* field read, and
//!    releasing an opaque-bound name reports `UnknownAlias`, so no such body
//!    can reach `Safe`. That containment is pinned by
//!    `releasing_an_opaque_bound_name_is_never_silently_admitted`.
//! 4. **The base carries no tracked identity.** [`untracked_bases`] computes a
//!    syntactic over-approximation of the names this function may bind to a
//!    resource identity, and refuses every base in it. This is the premise C02
//!    §2 had to add after admitting a read through a released pointer turned an
//!    honest `Unknown` into `Safe`; it is kept here, and it is *stronger* than
//!    C02's, because a borrowed actual survives into the modeled call rather
//!    than disappearing into a no-op.
//! 5. **Borrowed position.** For the actual form, the contract in force must
//!    declare that argument index `Borrowed`. An argument beyond the declared
//!    parameter vector — a variadic tail — is never admitted, so a contract
//!    cannot silently cover actuals it does not describe.
//!
//! Under (1)–(5) the read is custody-neutral: it evaluates a member of a live
//! aggregate the projection does not track, yields a value that is not one of
//! this function's tracked identities, and is handed to a callee that the
//! contract in force says borrows it. Both premises C02 records — declaration
//! completeness and C effective types — are inherited unchanged and are not
//! proved here.
//!
//! ## What the over-approximation costs
//!
//! [`untracked_bases`] is deliberately blunt: a name is refused as a base as
//! soon as it is address-taken whole, incremented, compound-assigned, passed at
//! a non-`Borrowed` position of a contracted call, passed at *any* position of
//! an **uncontracted** call, or assigned from anything but an admitted field
//! read or a call-free, `&`-free expression over already-admitted names.
//! Refusing too many names only narrows the whitelist.

use std::collections::BTreeSet;

use qcore_schemas::contract::{ResourceContracts, ResourceParameterMode};

use crate::typed_field_access::{DeclaredType, FunctionTypes, TypedAccessModel, is_name};

/// The names of one function that may bind a tracked resource identity.
#[derive(Clone, Debug, Default)]
pub(crate) struct UntrackedBases {
    risky: BTreeSet<String>,
    /// False when the declaration reader refused this function, in which case
    /// nothing at all is admitted.
    readable: bool,
}

impl UntrackedBases {
    /// Whether `name` is a name this function can never bind to a tracked
    /// resource identity. False whenever the declaration reader refused the
    /// function, so nothing is admitted there.
    ///
    /// This is premise (4) read in the positive direction. Card O04-B08 uses it
    /// for the *value* of an aggregate store: a value that carries no tracked
    /// identity transfers no custody, so the store is custody-neutral for the
    /// field it overwrites.
    pub(crate) fn never_bound(&self, name: &str) -> bool {
        self.readable && !self.risky.contains(name)
    }
}

/// Bound on the monotone descent below; each round adds at least one name.
const CLOSURE_ROUNDS: usize = 64;

/// Over-approximate the names of one function body that may bind a tracked
/// resource identity, so that every other name is an admissible access base.
///
/// `body` is the token slice strictly inside the function's braces.
pub(crate) fn untracked_bases(
    body: &[&str],
    protected_formals: &BTreeSet<String>,
    contracts: &ResourceContracts,
    variadic_tails: &BTreeSet<String>,
    model: &TypedAccessModel,
    types: &FunctionTypes,
) -> UntrackedBases {
    if !types.is_readable() {
        return UntrackedBases::default();
    }
    let mut risky = protected_formals.clone();
    seed_addresses_and_writes(body, &mut risky);
    seed_call_arguments(body, contracts, variadic_tails, &mut risky);
    // Monotone descent: an assignment can only *add* to the refused set, and a
    // base that becomes refused can only refuse more assignments.
    for _ in 0..CLOSURE_ROUNDS {
        let before = risky.len();
        close_over_assignments(body, model, types, &mut risky);
        if risky.len() == before {
            break;
        }
    }
    UntrackedBases {
        risky,
        readable: true,
    }
}

/// `&name`, `name++`, `--name` and `name op= …` all bind or escape the name.
///
/// `&name->member` and `&name.member` are *not* whole-object address-of: they
/// take the address of a sub-object, which cannot reassign or release the base
/// pointer itself. That is a declared premise, recorded in the module header.
fn seed_addresses_and_writes(body: &[&str], risky: &mut BTreeSet<String>) {
    for index in 0..body.len() {
        let text = body[index];
        if !is_name(text) {
            continue;
        }
        let next = body.get(index + 1).copied();
        let after = body.get(index + 2).copied();
        let previous = index.checked_sub(1).and_then(|at| body.get(at)).copied();
        let before = index.checked_sub(2).and_then(|at| body.get(at)).copied();
        let continues = matches!(next, Some("-") if after == Some(">"))
            || matches!(next, Some(".") | Some("[") | Some("("));
        if previous == Some("&") && !continues {
            risky.insert(text.to_owned());
            continue;
        }
        if matches!(
            (before, previous),
            (Some("+"), Some("+")) | (Some("-"), Some("-"))
        ) || matches!(
            (next, after),
            (Some("+"), Some("+")) | (Some("-"), Some("-"))
        ) {
            risky.insert(text.to_owned());
            continue;
        }
        // `name op= …` for every compound operator. A plain `name = …` is not
        // seeded here; `close_over_assignments` decides it from its own
        // right-hand side.
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
            risky.insert(text.to_owned());
            continue;
        }
        if matches!(
            (next, after),
            (Some("<"), Some("<")) | (Some(">"), Some(">"))
        ) && body.get(index + 3).copied() == Some("=")
        {
            risky.insert(text.to_owned());
        }
    }
}

/// Every call argument that is not a declared `Borrowed` position may have its
/// custody changed by the callee, so its root names are refused as bases.
fn seed_call_arguments(
    body: &[&str],
    contracts: &ResourceContracts,
    variadic_tails: &BTreeSet<String>,
    risky: &mut BTreeSet<String>,
) {
    for (open, callee) in call_sites(body) {
        let contract = contracts.contract(callee);
        let Some(arguments) = argument_spans(body, open) else {
            // An unbalanced argument list: refuse every root name after it
            // rather than guess where it ended.
            for text in &body[open..] {
                if is_name(text) {
                    risky.insert((*text).to_owned());
                }
            }
            continue;
        };
        for (index, (start, end)) in arguments.iter().copied().enumerate() {
            // Card O04-TRACE: a position *beyond* the declared parameter
            // vector of a callee that declares a variadic data tail changes no
            // custody either, by exactly the declaration this rule already
            // trusts for a `Borrowed` position. The exemption is decided from
            // the declaration alone, never from the actual, so a name that is
            // risky for any other reason — an acquisition, an address, a
            // custody position elsewhere — stays risky here.
            let borrowed = contract.is_some_and(|contract| match contract.parameters.get(index) {
                Some(ResourceParameterMode::Borrowed) => true,
                Some(_) => false,
                None => variadic_tails.contains(callee),
            });
            if borrowed {
                continue;
            }
            for name in root_names(&body[start..end]) {
                risky.insert(name);
            }
        }
    }
}

/// Names an argument mentions as a *place root* — an identifier that is not a
/// callee and does not continue into a member access.
fn root_names(argument: &[&str]) -> Vec<String> {
    let mut names = Vec::new();
    for index in 0..argument.len() {
        let text = argument[index];
        if !is_name(text) {
            continue;
        }
        match argument.get(index + 1).copied() {
            Some("(") | Some(".") => continue,
            Some("-") if argument.get(index + 2) == Some(&">") => continue,
            _ => {}
        }
        names.push(text.to_owned());
    }
    names
}

/// Refuse the destination of every simple assignment whose right-hand side is
/// not provably free of tracked identities.
fn close_over_assignments(
    body: &[&str],
    model: &TypedAccessModel,
    types: &FunctionTypes,
    risky: &mut BTreeSet<String>,
) {
    let mut added = Vec::new();
    for statement in statements(body) {
        let Some(equal) = simple_assignment(statement) else {
            continue;
        };
        let Some(destination) = statement
            .get(equal.wrapping_sub(1))
            .copied()
            .filter(|text| is_name(text))
        else {
            continue;
        };
        if risky.contains(destination) {
            continue;
        }
        let right = &statement[equal + 1..];
        // An admitted field read denotes a place this function never binds.
        if admitted_access(right, 0, model, types, risky) == Some(right.len()) {
            continue;
        }
        let opaque = right.iter().enumerate().any(|(index, text)| {
            *text == "&" || (is_name(text) && right.get(index + 1) == Some(&"("))
        }) || right.iter().any(|text| risky.contains(*text));
        if opaque {
            added.push(destination.to_owned());
        }
    }
    risky.extend(added);
}

/// Split a body into `;`-delimited statements at brace level, the same way the
/// protected-formal closure in [`crate::legacy_c`] does.
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

/// The index of a top-level simple `=` — not `==`, `!=`, `<=`, `>=` or any
/// compound assignment.
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
                let next = statement.get(index + 1).copied();
                let compound = matches!(
                    previous,
                    Some("=")
                        | Some("!")
                        | Some("<")
                        | Some(">")
                        | Some("+")
                        | Some("-")
                        | Some("*")
                        | Some("/")
                        | Some("%")
                        | Some("&")
                        | Some("|")
                        | Some("^")
                );
                if !compound && next != Some("=") {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

/// Call sites of a token slice, as `(index of the callee name, callee)`.
fn call_sites<'a>(body: &[&'a str]) -> Vec<(usize, &'a str)> {
    (0..body.len())
        .filter(|index| is_name(body[*index]) && body.get(index + 1) == Some(&"("))
        .map(|index| (index, body[index]))
        .collect()
}

/// Absolute `[start, end)` spans of each argument of the call whose callee name
/// is at `call_index`, or `None` when the list is unbalanced.
fn argument_spans(body: &[&str], call_index: usize) -> Option<Vec<(usize, usize)>> {
    let mut spans = Vec::new();
    let mut start = call_index + 2;
    let mut parentheses = 0usize;
    let mut brackets = 0usize;
    let mut cursor = start;
    while cursor < body.len() {
        match body[cursor] {
            "(" => parentheses += 1,
            ")" if parentheses == 0 && brackets == 0 => {
                if cursor > start {
                    spans.push((start, cursor));
                }
                return Some(spans);
            }
            ")" => parentheses = parentheses.checked_sub(1)?,
            "[" => brackets += 1,
            "]" => brackets = brackets.checked_sub(1)?,
            "," if parentheses == 0 && brackets == 0 => {
                spans.push((start, cursor));
                start = cursor + 1;
            }
            _ => {}
        }
        cursor += 1;
    }
    None
}

/// Maximum number of member selections one admitted access may contain.
///
/// Depth 1 is C02-P's original direct access; O04-ARMS raises it so that
/// `storage -> stream -> name` and `in -> config -> router` — the two chains
/// that keep the caller's first early-return arm and its allocator-size read
/// unmodeled — can be read. A longer chain is refused, not truncated.
const ACCESS_CHAIN_LIMIT: usize = 4;

/// If an admitted member access starts at `start`, its end (exclusive).
///
/// The access is `base` followed by one to [`ACCESS_CHAIN_LIMIT`] member
/// selections. Every selection *before the last* must be an
/// [`TypedAccessModel::record_pointer_field`] continued with `->`; the last
/// selects a [`TypedAccessModel::borrowable_field`] and must not continue into
/// `->`, `.`, `[` or `(`. A `.` may only appear as the first selection, on a
/// record-valued base: continuing with `.` would cross a nested aggregate
/// sub-object, which this rule refuses by construction.
fn admitted_access(
    texts: &[&str],
    start: usize,
    model: &TypedAccessModel,
    types: &FunctionTypes,
    risky: &BTreeSet<String>,
) -> Option<usize> {
    let base = texts.get(start).copied().filter(|text| is_name(text))?;
    if risky.contains(base) {
        return None;
    }
    let (mut field_index, arrow) =
        if texts.get(start + 1) == Some(&"-") && texts.get(start + 2) == Some(&">") {
            (start + 3, true)
        } else if texts.get(start + 1) == Some(&".") {
            (start + 2, false)
        } else {
            return None;
        };
    let mut tag = match (types.declared_type(base), arrow) {
        (Some(DeclaredType::RecordPointer(tag)), true) => tag.clone(),
        (Some(DeclaredType::Record(tag)), false) => tag.clone(),
        _ => return None,
    };
    for _ in 0..ACCESS_CHAIN_LIMIT {
        let field = texts
            .get(field_index)
            .copied()
            .filter(|field| is_name(field))?;
        let continues_with_arrow =
            texts.get(field_index + 1) == Some(&"-") && texts.get(field_index + 2) == Some(&">");
        if continues_with_arrow {
            // An intermediate link: the member must itself be a fully typed
            // `struct POINTEE *`, and the walk re-enters this model at
            // `POINTEE`, where an incomplete, ambiguous or union record
            // refuses.
            tag = model.record_pointer_field(&tag, field)?.to_owned();
            field_index += 3;
            continue;
        }
        if !model.borrowable_field(&tag, field) {
            return None;
        }
        match texts.get(field_index + 1) {
            Some(&".") | Some(&"[") | Some(&"(") => return None,
            _ => {}
        }
        return Some(field_index + 1);
    }
    None
}

/// The token spans of one statement that are admitted borrowed pointer-field
/// actuals of the statement's first call.
///
/// Empty when nothing is admitted, in which case the lexical gate governs
/// unchanged.
pub(crate) fn borrowed_actual_spans(
    texts: &[&str],
    contracts: &ResourceContracts,
    model: &TypedAccessModel,
    types: &FunctionTypes,
    bases: &UntrackedBases,
) -> Vec<(usize, usize)> {
    if !bases.readable {
        return Vec::new();
    }
    let Some((call_index, callee)) = call_sites(texts).into_iter().next() else {
        return Vec::new();
    };
    let Some(contract) = contracts.contract(callee) else {
        return Vec::new();
    };
    let Some(arguments) = argument_spans(texts, call_index) else {
        return Vec::new();
    };
    let mut spans = Vec::new();
    for (index, (start, end)) in arguments.iter().copied().enumerate() {
        if !matches!(
            contract.parameters.get(index),
            Some(ResourceParameterMode::Borrowed)
        ) {
            continue;
        }
        if admitted_access(texts, start, model, types, &bases.risky) == Some(end) {
            spans.push((start, end));
        }
    }
    spans
}

/// Whether `right` is exactly one admitted direct member access, so that an
/// assignment from it binds a value carrying no tracked identity.
pub(crate) fn is_opaque_field_read(
    right: &[&str],
    model: &TypedAccessModel,
    types: &FunctionTypes,
    bases: &UntrackedBases,
) -> bool {
    bases.readable && admitted_access(right, 0, model, types, &bases.risky) == Some(right.len())
}

#[cfg(test)]
mod tests {
    use qcore_schemas::contract::{
        ResourceFunctionContract, ResourceParameterMode, ResourceReturnMode,
    };

    use super::*;

    /// Split like the importer's own tokenizer: identifiers, numbers and one
    /// character per punctuation token, so `->` is `-` followed by `>`.
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

    /// One record with a scalar and two pointer members, plus the locals every
    /// case below declares. Deliberately generic: no rule reads a name.
    const DECLARATIONS: &str = "struct rec { int state ; struct a * one ; struct b * two ; } ; \
         union view { struct a * one ; int tag ; } ;";

    const LOCALS: &str = "struct rec * base ; struct rec * other ; struct rec value ; \
         struct a * held ; int n ;";

    /// `borrow` borrows both pointer positions; `sink` consumes; `drop`
    /// releases; `variadic` declares only its first parameter.
    fn contracts() -> ResourceContracts {
        let mut contracts = ResourceContracts::default();
        contracts.insert_contract(
            "borrow",
            ResourceFunctionContract::new(
                [
                    ResourceParameterMode::Borrowed,
                    ResourceParameterMode::Borrowed,
                ],
                ResourceReturnMode::None,
            ),
        );
        contracts.insert_contract(
            "sink",
            ResourceFunctionContract::new(
                [ResourceParameterMode::Consumed { nullable: false }],
                ResourceReturnMode::None,
            ),
        );
        contracts.insert_contract(
            "drop",
            ResourceFunctionContract::new(
                [ResourceParameterMode::Released { nullable: false }],
                ResourceReturnMode::None,
            ),
        );
        contracts.insert_contract(
            "variadic",
            ResourceFunctionContract::new(
                [ResourceParameterMode::Borrowed],
                ResourceReturnMode::None,
            ),
        );
        contracts
    }

    struct Fixture {
        model: TypedAccessModel,
        types: FunctionTypes,
        bases: UntrackedBases,
        contracts: ResourceContracts,
    }

    /// Build the three declaration facts the classification reads, over one
    /// function body written in the importer's own token shape.
    fn fixture(body: &str) -> Fixture {
        fixture_with_declarations(DECLARATIONS, body)
    }

    fn fixture_with_declarations(declarations: &str, body: &str) -> Fixture {
        let unit = format!("{declarations} {LOCALS} {body}");
        let unit: &'static str = Box::leak(unit.into_boxed_str());
        let all = texts(unit);
        let model = TypedAccessModel::read(&all);
        let body_tokens = texts(Box::leak(format!("{LOCALS} {body}").into_boxed_str()));
        let protected = BTreeSet::new();
        let types = FunctionTypes::read(&model, &[], &body_tokens, &protected);
        let contracts = contracts();
        let bases = untracked_bases(
            &body_tokens,
            &protected,
            &contracts,
            &BTreeSet::new(),
            &model,
            &types,
        );
        Fixture {
            model,
            types,
            bases,
            contracts,
        }
    }

    /// The admitted spans of one statement, written in isolation.
    fn spans(fixture: &Fixture, statement: &str) -> Vec<(usize, usize)> {
        let statement: &'static str = Box::leak(statement.to_owned().into_boxed_str());
        let tokens = texts(statement);
        borrowed_actual_spans(
            &tokens,
            &fixture.contracts,
            &fixture.model,
            &fixture.types,
            &fixture.bases,
        )
    }

    fn opaque_read(fixture: &Fixture, right: &str) -> bool {
        let right: &'static str = Box::leak(right.to_owned().into_boxed_str());
        let tokens = texts(right);
        is_opaque_field_read(&tokens, &fixture.model, &fixture.types, &fixture.bases)
    }

    #[test]
    fn a_pointer_member_at_a_borrowed_position_is_admitted() {
        let fixture = fixture("borrow ( base -> one , base -> two ) ;");
        assert_eq!(
            spans(&fixture, "borrow ( base -> one , base -> two ) ;").len(),
            2
        );
    }

    #[test]
    fn a_scalar_member_at_a_borrowed_position_is_admitted_too() {
        let fixture = fixture("borrow ( base -> state , base -> two ) ;");
        assert_eq!(
            spans(&fixture, "borrow ( base -> state , base -> two ) ;").len(),
            2
        );
    }

    #[test]
    fn a_record_valued_base_is_admitted_through_the_dot_form() {
        let fixture = fixture("borrow ( value . one , value . two ) ;");
        assert_eq!(
            spans(&fixture, "borrow ( value . one , value . two ) ;").len(),
            2
        );
    }

    #[test]
    fn a_consuming_position_is_never_admitted() {
        let fixture = fixture("sink ( base -> one ) ;");
        assert!(spans(&fixture, "sink ( base -> one ) ;").is_empty());
    }

    #[test]
    fn a_releasing_position_is_never_admitted() {
        let fixture = fixture("drop ( base -> one ) ;");
        assert!(spans(&fixture, "drop ( base -> one ) ;").is_empty());
    }

    #[test]
    fn an_uncontracted_callee_never_admits_an_actual() {
        let fixture = fixture("unknown_callee ( base -> one ) ;");
        assert!(spans(&fixture, "unknown_callee ( base -> one ) ;").is_empty());
    }

    #[test]
    fn an_actual_beyond_the_declared_vector_is_never_admitted() {
        let fixture = fixture("variadic ( n , base -> one ) ;");
        assert!(spans(&fixture, "variadic ( n , base -> one ) ;").is_empty());
    }

    #[test]
    fn a_partial_argument_is_never_admitted() {
        let fixture = fixture("borrow ( base -> state + 1 , base -> two ) ;");
        // Only the whole second argument qualifies; the sum does not.
        assert_eq!(
            spans(&fixture, "borrow ( base -> state + 1 , base -> two ) ;").len(),
            1
        );
    }

    /// O04-ARMS deliberately widened C02-P's "a chain is never admitted" pin to
    /// "a chain through a record this reader did not fully type is never
    /// admitted". `struct a` has no definition in this unit, so the walk
    /// refuses at its second link — the load-bearing half of the original
    /// expectation, kept and now attributed to its real reason.
    #[test]
    fn a_chain_through_an_undefined_record_is_never_admitted() {
        let fixture = fixture("borrow ( base -> one -> next , base -> two ) ;");
        assert_eq!(
            spans(&fixture, "borrow ( base -> one -> next , base -> two ) ;").len(),
            1
        );
    }

    /// The positive direction: with every link fully typed the chain is
    /// admitted, so the refusals around it are refusals and not an empty rule.
    #[test]
    fn a_fully_typed_chain_is_admitted() {
        let fixture = fixture_with_declarations(
            "struct leaf { int state ; struct b * two ; } ; \
             struct mid { struct leaf * down ; } ; \
             struct rec { int state ; struct mid * one ; struct b * two ; } ;",
            "borrow ( base -> one -> down -> two , base -> two ) ;",
        );
        assert_eq!(
            spans(
                &fixture,
                "borrow ( base -> one -> down -> two , base -> two ) ;"
            )
            .len(),
            2
        );
    }

    /// A chain longer than the bound is refused rather than truncated.
    #[test]
    fn a_chain_beyond_the_bound_is_never_admitted() {
        let fixture = fixture_with_declarations(
            "struct leaf { struct leaf * down ; struct b * two ; } ; \
             struct rec { int state ; struct leaf * one ; struct b * two ; } ;",
            "borrow ( base -> one -> down -> down -> down -> two , base -> two ) ;",
        );
        assert_eq!(
            spans(
                &fixture,
                "borrow ( base -> one -> down -> down -> down -> two , base -> two ) ;"
            )
            .len(),
            1
        );
    }

    /// A `union` anywhere in the chain refuses the whole access.
    #[test]
    fn a_chain_through_a_union_is_never_admitted() {
        let fixture = fixture_with_declarations(
            "union mid { struct b * two ; int tag ; } ; \
             struct rec { int state ; union mid * one ; struct b * two ; } ;",
            "borrow ( base -> one -> two , base -> two ) ;",
        );
        assert_eq!(
            spans(&fixture, "borrow ( base -> one -> two , base -> two ) ;").len(),
            1
        );
    }

    /// A tag defined twice is ambiguous, and refuses as an intermediate link
    /// exactly as it refuses as a final one.
    #[test]
    fn a_chain_through_an_ambiguous_tag_is_never_admitted() {
        let fixture = fixture_with_declarations(
            "struct mid { struct b * two ; } ; struct mid { int state ; } ; \
             struct rec { int state ; struct mid * one ; struct b * two ; } ;",
            "borrow ( base -> one -> two , base -> two ) ;",
        );
        assert_eq!(
            spans(&fixture, "borrow ( base -> one -> two , base -> two ) ;").len(),
            1
        );
    }

    /// A double-star member is not `struct POINTEE *`, so it never continues.
    #[test]
    fn a_chain_through_a_double_pointer_member_is_never_admitted() {
        let fixture = fixture_with_declarations(
            "struct mid { struct b * two ; } ; \
             struct rec { int state ; struct mid * * one ; struct b * two ; } ;",
            "borrow ( base -> one -> two , base -> two ) ;",
        );
        assert_eq!(
            spans(&fixture, "borrow ( base -> one -> two , base -> two ) ;").len(),
            1
        );
    }

    /// Continuing with `.` would cross a nested aggregate sub-object.
    #[test]
    fn a_chain_continued_with_a_dot_is_never_admitted() {
        let fixture = fixture_with_declarations(
            "struct mid { struct b * two ; } ; \
             struct rec { int state ; struct mid * one ; struct b * two ; } ;",
            "borrow ( base -> one . two , base -> two ) ;",
        );
        assert_eq!(
            spans(&fixture, "borrow ( base -> one . two , base -> two ) ;").len(),
            1
        );
    }

    /// A chain's root is still subject to premise (4): a bound base refuses the
    /// whole chain, at any depth.
    #[test]
    fn a_bound_base_refuses_the_whole_chain() {
        let fixture = fixture_with_declarations(
            "struct mid { struct b * two ; } ; \
             struct rec { int state ; struct mid * one ; struct b * two ; } ;",
            "sink ( base ) ; borrow ( base -> one -> two , n ) ;",
        );
        assert!(spans(&fixture, "borrow ( base -> one -> two , n ) ;").is_empty());
    }

    /// A chain is an opaque field read too, so the same value may bind a local.
    #[test]
    fn a_fully_typed_chain_is_an_opaque_field_read() {
        let fixture = fixture_with_declarations(
            "struct mid { struct b * two ; } ; \
             struct rec { int state ; struct mid * one ; struct b * two ; } ;",
            "held = base -> one -> two ;",
        );
        assert!(opaque_read(&fixture, "base -> one -> two"));
        assert!(!opaque_read(&fixture, "base -> one -> two -> next"));
        assert!(!opaque_read(&fixture, "base -> one -> two + 1"));
    }

    #[test]
    fn a_union_base_is_never_admitted() {
        let fixture = fixture("union view * seen ; borrow ( seen -> one , base -> two ) ;");
        assert!(!fixture.model.borrowable_field("view", "one"));
        // Only the `struct rec` argument is admitted; the union base is not.
        assert_eq!(
            spans(&fixture, "borrow ( seen -> one , base -> two ) ;"),
            vec![(7, 11)]
        );
    }

    #[test]
    fn a_base_consumed_anywhere_in_the_body_is_never_admitted() {
        let fixture = fixture("sink ( base ) ; borrow ( base -> one , base -> two ) ;");
        assert!(spans(&fixture, "borrow ( base -> one , base -> two ) ;").is_empty());
    }

    #[test]
    fn a_base_released_anywhere_in_the_body_is_never_admitted() {
        let fixture = fixture("drop ( base ) ; borrow ( base -> one , base -> two ) ;");
        assert!(spans(&fixture, "borrow ( base -> one , base -> two ) ;").is_empty());
    }

    #[test]
    fn a_base_passed_to_an_uncontracted_callee_is_never_admitted() {
        let fixture = fixture("unknown_callee ( base ) ; borrow ( base -> one , base -> two ) ;");
        assert!(spans(&fixture, "borrow ( base -> one , base -> two ) ;").is_empty());
    }

    #[test]
    fn an_address_taken_base_is_never_admitted() {
        let fixture = fixture("held = & base ; borrow ( base -> one , base -> two ) ;");
        assert!(spans(&fixture, "borrow ( base -> one , base -> two ) ;").is_empty());
    }

    #[test]
    fn an_incremented_base_is_never_admitted() {
        let fixture = fixture("base ++ ; borrow ( base -> one , base -> two ) ;");
        assert!(spans(&fixture, "borrow ( base -> one , base -> two ) ;").is_empty());
    }

    #[test]
    fn a_compound_assigned_base_is_never_admitted() {
        let fixture = fixture("base += 1 ; borrow ( base -> one , base -> two ) ;");
        assert!(spans(&fixture, "borrow ( base -> one , base -> two ) ;").is_empty());
    }

    #[test]
    fn a_base_assigned_from_an_opaque_expression_is_never_admitted() {
        let fixture =
            fixture("base = unknown_callee ( n ) ; borrow ( base -> one , base -> two ) ;");
        assert!(spans(&fixture, "borrow ( base -> one , base -> two ) ;").is_empty());
    }

    /// The refusal is flow-insensitive on purpose: the read appears before the
    /// assignment, and is refused anyway.
    #[test]
    fn the_refusal_of_a_base_does_not_depend_on_statement_order() {
        let fixture =
            fixture("borrow ( base -> one , base -> two ) ; base = unknown_callee ( n ) ;");
        assert!(spans(&fixture, "borrow ( base -> one , base -> two ) ;").is_empty());
    }

    /// The transitive closure: a name assigned from a refused name is refused.
    #[test]
    fn the_refusal_closes_over_assignments() {
        let fixture = fixture(
            "base = unknown_callee ( n ) ; other = base ; borrow ( other -> one , other -> two ) ;",
        );
        assert!(spans(&fixture, "borrow ( other -> one , other -> two ) ;").is_empty());
    }

    /// A base assigned from an admitted field read stays admissible: that is
    /// the shape the real constructor-to-caller path needs.
    #[test]
    fn a_base_assigned_from_an_admitted_field_read_stays_admissible() {
        let fixture = fixture("other = base -> one ; borrow ( base -> one , base -> two ) ;");
        assert_eq!(
            spans(&fixture, "borrow ( base -> one , base -> two ) ;").len(),
            2
        );
    }

    #[test]
    fn a_whole_admitted_access_is_an_opaque_field_read() {
        let fixture = fixture("other = base -> one ;");
        assert!(opaque_read(&fixture, "base -> one"));
        assert!(!opaque_read(&fixture, "base -> one -> next"));
        assert!(!opaque_read(&fixture, "base -> one + 1"));
    }

    /// An unreadable declaration refuses every access in the function, which is
    /// what makes the declaration-completeness premise hold for the admitted set.
    #[test]
    fn an_unreadable_function_admits_nothing() {
        let fixture = fixture_with_declarations(
            "struct rec { int state ; struct a * one ; struct b * two ; } ;",
            "void ( * hook ) ( int ) ; borrow ( base -> one , base -> two ) ;",
        );
        assert!(!fixture.types.is_readable());
        assert!(spans(&fixture, "borrow ( base -> one , base -> two ) ;").is_empty());
        assert!(!opaque_read(&fixture, "base -> one"));
    }

    /// Non-vacuity: with every premise satisfied the whitelist does admit, so
    /// the refusals above are refusals and not an empty rule.
    #[test]
    fn the_whitelist_is_not_empty() {
        let fixture = fixture("borrow ( base -> one , base -> two ) ;");
        assert!(!spans(&fixture, "borrow ( base -> one , base -> two ) ;").is_empty());
        assert!(opaque_read(&fixture, "base -> one"));
    }
}
