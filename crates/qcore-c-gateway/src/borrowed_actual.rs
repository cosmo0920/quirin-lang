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
//! 3. **The access does not continue** into `->`, `.`, `[` or `(`. Chains such
//!    as `a->b->c` are refused, so a read never crosses an aggregate this
//!    projection cannot name.
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
    model: &TypedAccessModel,
    types: &FunctionTypes,
) -> UntrackedBases {
    if !types.is_readable() {
        return UntrackedBases::default();
    }
    let mut risky = protected_formals.clone();
    seed_addresses_and_writes(body, &mut risky);
    seed_call_arguments(body, contracts, &mut risky);
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
fn seed_call_arguments(body: &[&str], contracts: &ResourceContracts, risky: &mut BTreeSet<String>) {
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
            let borrowed = contract.is_some_and(|contract| {
                matches!(
                    contract.parameters.get(index),
                    Some(ResourceParameterMode::Borrowed)
                )
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

/// If an admitted direct member access starts at `start`, its end (exclusive).
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
    let (field_index, arrow) =
        if texts.get(start + 1) == Some(&"-") && texts.get(start + 2) == Some(&">") {
            (start + 3, true)
        } else if texts.get(start + 1) == Some(&".") {
            (start + 2, false)
        } else {
            return None;
        };
    let field = texts
        .get(field_index)
        .copied()
        .filter(|field| is_name(field))?;
    let tag = match (types.declared_type(base), arrow) {
        (Some(DeclaredType::RecordPointer(tag)), true) => tag,
        (Some(DeclaredType::Record(tag)), false) => tag,
        _ => return None,
    };
    if !model.borrowable_field(tag, field) {
        return None;
    }
    match texts.get(field_index + 1) {
        Some(&".") | Some(&"[") | Some(&"(") => return None,
        Some(&"-") if texts.get(field_index + 2) == Some(&">") => return None,
        _ => {}
    }
    Some(field_index + 1)
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
