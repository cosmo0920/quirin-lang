//! Card **REFCOUNT-REFERENCE-RETURNING-DECLARATIONS** (gateway rule 13d): in
//! a unit that defines the body of a declaration whose result is a returned
//! reference (`resource_return_owned(n)` with `resource_return_kind(reference)`,
//! contract specification §C04.17), the declaration stands only where its body
//! validates it structurally. One thin hook, [`settle`], called from rule 13c's
//! `reference_route::settle` at the three unit entry points; the identity on
//! every environment with no such declaration or no such body.
//!
//! The check (structural, like rule 13c's): the body has no `goto`, and every
//! `return` hands back
//!
//! * a null constant (`NULL` as Clang preprocesses it, `( ( void * ) 0 )`, or
//!   `0`), only when the declaration is `nullable`;
//! * a call ([`reference_source`]) of a declared keep that returns its
//!   argument, or of another declared returned reference (pointer casts
//!   around it erased); or
//! * a local `x` (not a formal) every assignment of which is a null constant,
//!   such a call, or a call of a declared heap allocation (a constructor that
//!   initialises the count it returns: `rd_kafka_q_new0` over `rd_malloc`),
//!   whose address is never taken and which is never an actual at a declared
//!   release, consumption or drop.
//!
//! It establishes that every reference the body hands back is one it obtained
//! (from a keep, a reference-returning callee, or a fresh object) and did not
//! give away before returning; the count arithmetic (that a fresh object's
//! count starts at the one reference handed back) and the declared nullability
//! of a non-null return stay declared assumptions, pinned to the body (C06.2).
//! A declaration the body does not validate is **withdrawn** in that unit: it
//! moves to the retaining table marked `references_withdrawn`, so every call
//! there refuses (`reference-withdrawn`) and, as with rule 13c, no inference
//! reads the body in its place.

use qcore_schemas::contract::{
    ResourceContracts, ResourceKind, ResourceParameterMode, ResourceReturnMode,
};
use qcore_schemas::reference_count::ReferenceOperation;
use qcore_schemas::retained_by::RetainingContract;

use super::Token;

fn is_identifier(text: &str) -> bool {
    let mut chars = text.chars();
    chars
        .next()
        .is_some_and(|first| first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

/// The index of the bracket closing the one at `open`.
fn close_of(words: &[&str], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (index, word) in words.iter().enumerate().skip(open) {
        match *word {
            "(" | "[" | "{" => depth += 1,
            ")" | "]" | "}" => {
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

/// `words` with every enclosing pair of parentheses removed.
fn strip<'a>(mut words: &'a [&'a str]) -> &'a [&'a str] {
    while words.len() >= 2
        && words[0] == "("
        && words[words.len() - 1] == ")"
        && close_of(words, 0) == Some(words.len() - 1)
    {
        words = &words[1..words.len() - 1];
    }
    words
}

/// `words` with a leading pointer cast (`( T * )`, the type words
/// identifiers, `const`, `struct` and `*`) removed, repeatedly.
fn uncast<'a>(mut words: &'a [&'a str]) -> &'a [&'a str] {
    loop {
        words = strip(words);
        let Some(close) = (words.first() == Some(&"("))
            .then(|| close_of(words, 0))
            .flatten()
        else {
            return words;
        };
        let cast = &words[1..close];
        if close + 1 < words.len()
            && cast.last() == Some(&"*")
            && cast.iter().all(|word| *word == "*" || is_identifier(word))
        {
            words = &words[close + 1..];
        } else {
            return words;
        }
    }
}

/// Whether `words` is a null constant: `0` under pointer casts and
/// parentheses (Clang's `NULL` is `( ( void * ) 0 )`).
fn is_null_constant(words: &[&str]) -> bool {
    matches!(uncast(words), ["0"])
}

/// The callee of `words` when it is exactly one call `f ( … )`.
fn whole_call<'a>(words: &'a [&'a str]) -> Option<&'a str> {
    let words = uncast(words);
    match words {
        [callee, "(", ..] if is_identifier(callee) => {
            (close_of(words, 1) == Some(words.len() - 1)).then_some(*callee)
        }
        _ => None,
    }
}

/// What a returned or assigned expression is, for the check.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Source {
    Null,
    /// A call of a declared keep (alias result) or returned reference.
    Reference,
    /// A call of a declared heap allocation.
    Allocation,
    Other,
}

/// Whether `callee` hands its caller a reference: a declared keep whose
/// result is its argument, or a declared returned reference.
fn reference_callee(callee: &str, contracts: &ResourceContracts) -> bool {
    let returned = |contract: &qcore_schemas::contract::ResourceFunctionContract| {
        matches!(contract.result, ResourceReturnMode::Owned { .. })
            && contract.result_kind == ResourceKind::Reference
    };
    if let Some(declared) = contracts.retaining_contract(callee) {
        return !declared.references_withdrawn
            && (returned(&declared.contract)
                || (declared
                    .references
                    .iter()
                    .any(|clause| clause.operation == ReferenceOperation::Kept)
                    && matches!(declared.contract.result, ResourceReturnMode::Derived { .. })));
    }
    contracts.contract(callee).is_some_and(returned)
}

/// Read one expression.
fn reference_source(words: &[&str], contracts: &ResourceContracts) -> Source {
    if is_null_constant(words) {
        return Source::Null;
    }
    match whole_call(words) {
        Some(callee) if reference_callee(callee, contracts) => Source::Reference,
        Some(callee)
            if contracts.contract(callee).is_some_and(|contract| {
                matches!(contract.result, ResourceReturnMode::Owned { .. })
                    && contract.result_kind.is_heap()
            }) =>
        {
            Source::Allocation
        }
        _ => Source::Other,
    }
}

/// Whether the token at `at` spells an assignment operator `=` (the lexer
/// emits `==`, `!=`, `<=`, `>=` byte by byte).
fn assignment_at(body: &[&str], at: usize) -> bool {
    body.get(at) == Some(&"=")
        && body.get(at + 1) != Some(&"=")
        && !at.checked_sub(1).is_some_and(|before| {
            matches!(
                body[before],
                "=" | "!" | "<" | ">" | "+" | "-" | "*" | "/" | "%" | "&" | "|" | "^"
            )
        })
}

/// The end (exclusive) of the expression starting at `start`: the first `;`,
/// or `,` or closing bracket at its own depth.
fn expression_end(words: &[&str], start: usize) -> usize {
    let mut depth = 0usize;
    for (index, word) in words.iter().enumerate().skip(start) {
        match *word {
            "(" | "[" | "{" => depth += 1,
            ")" | "]" | "}" => {
                if depth == 0 {
                    return index;
                }
                depth -= 1;
            }
            ";" => return index,
            "," if depth == 0 => return index,
            _ => {}
        }
    }
    words.len()
}

/// The callee and argument position of the call whose argument list holds
/// position `at` as a whole actual, if any.
fn whole_actual_of<'a>(body: &[&'a str], at: usize) -> Option<(&'a str, usize)> {
    let previous = at.checked_sub(1).map(|before| body[before])?;
    let next = body.get(at + 1).copied()?;
    if !matches!(previous, "(" | ",") || !matches!(next, ")" | ",") {
        return None;
    }
    let mut depth = 0usize;
    let mut commas = 0usize;
    for index in (0..at).rev() {
        match body[index] {
            ")" | "]" | "}" => depth += 1,
            "(" if depth == 0 => {
                let callee = index.checked_sub(1).map(|c| body[c])?;
                return is_identifier(callee).then_some((callee, commas));
            }
            "(" | "[" | "{" => depth = depth.saturating_sub(1),
            "," if depth == 0 => commas += 1,
            ";" if depth == 0 => return None,
            _ => {}
        }
    }
    None
}

/// Whether position `position` of `callee` gives its actual away: a declared
/// release, consumption, store or drop.
fn gives_away(callee: &str, position: usize, contracts: &ResourceContracts) -> bool {
    let away = |contract: &qcore_schemas::contract::ResourceFunctionContract| {
        matches!(
            contract.parameters.get(position),
            Some(
                ResourceParameterMode::Released { .. }
                    | ResourceParameterMode::Consumed { .. }
                    | ResourceParameterMode::Stored { .. }
            )
        )
    };
    if let Some(declared) = contracts.retaining_contract(callee) {
        return away(&declared.contract)
            || declared.references.iter().any(|clause| {
                clause.operation == ReferenceOperation::Dropped && clause.parameter == position
            });
    }
    contracts.contract(callee).is_some_and(away)
}

/// Whether the local `name` is a reference the body obtained and kept until
/// it returns it: every assignment a null constant, a reference call or an
/// allocation; never address-taken; never given away.
fn local_validates(
    body: &[&str],
    name: &str,
    nullable: bool,
    contracts: &ResourceContracts,
) -> bool {
    let mut assigned = false;
    for (at, word) in body.iter().enumerate() {
        if *word != name {
            continue;
        }
        let previous = at.checked_sub(1).map(|before| body[before]);
        if matches!(previous, Some("." | "->")) {
            continue;
        }
        if previous == Some("&") {
            return false;
        }
        if assignment_at(body, at + 1) {
            let end = expression_end(body, at + 2);
            match reference_source(&body[at + 2..end], contracts) {
                Source::Reference | Source::Allocation => assigned = true,
                Source::Null if nullable => {}
                Source::Null | Source::Other => return false,
            }
            continue;
        }
        if let Some((callee, position)) = whole_actual_of(body, at)
            && gives_away(callee, position, contracts)
        {
            return false;
        }
    }
    assigned
}

/// Rule 13d: whether `body` validates a returned reference of the stated
/// nullability.
pub(super) fn validated(
    body: &[&str],
    formals: &[String],
    nullable: bool,
    contracts: &ResourceContracts,
) -> bool {
    if body.contains(&"goto") {
        return false;
    }
    let mut returns = 0usize;
    for (at, word) in body.iter().enumerate() {
        if *word != "return" {
            continue;
        }
        returns += 1;
        let end = expression_end(body, at + 1);
        let returned = strip(&body[at + 1..end]);
        let admitted = match reference_source(returned, contracts) {
            Source::Reference => true,
            Source::Null => nullable,
            Source::Allocation => false,
            Source::Other => match uncast(returned) {
                [name] if is_identifier(name) && !formals.iter().any(|formal| formal == name) => {
                    local_validates(body, name, nullable, contracts)
                }
                _ => false,
            },
        };
        if !admitted {
            return false;
        }
    }
    returns > 0
}

/// Whether a declaration's custody contract returns a reference, and its
/// nullability.
fn returned_reference(
    contract: &qcore_schemas::contract::ResourceFunctionContract,
) -> Option<bool> {
    match contract.result {
        ResourceReturnMode::Owned { nullable }
            if contract.result_kind == ResourceKind::Reference =>
        {
            Some(nullable)
        }
        _ => None,
    }
}

/// The hook (rule 13d): `contracts` with every returned-reference declaration
/// whose body this unit defines and does not validate marked withdrawn, or
/// `None` when nothing is.
pub(super) fn settle(tokens: &[Token], contracts: &ResourceContracts) -> Option<ResourceContracts> {
    let mut withdrawn: Vec<(String, RetainingContract)> = Vec::new();
    let candidates = contracts
        .functions()
        .iter()
        .filter_map(|(name, contract)| {
            returned_reference(contract).map(|nullable| {
                (
                    name.clone(),
                    nullable,
                    RetainingContract {
                        contract: contract.clone(),
                        retentions: Vec::new(),
                        resets: Vec::new(),
                        slot_moves: Vec::new(),
                        derived_outs: Vec::new(),
                        references: Vec::new(),
                        references_withdrawn: false,
                        callbacks: Vec::new(),
                        callbacks_withdrawn: false,
                        discriminant: None,
                        arms: Vec::new(),
                        arms_withdrawn: false,
                        constructed: Default::default(),
                        constructed_withdrawn: false,
                        handoff: None,
                        member_paths: Vec::new(),
                        member_paths_withdrawn: false,
                        path_retentions: Vec::new(),
                        removals: Vec::new(),
                        descriptor_cells: Default::default(),
                        lifetime: None,
                    },
                )
            })
        })
        .chain(
            contracts
                .retaining()
                .iter()
                .filter(|(_, declared)| !declared.references_withdrawn)
                .filter_map(|(name, declared)| {
                    returned_reference(&declared.contract)
                        .map(|nullable| (name.clone(), nullable, declared.clone()))
                }),
        )
        .collect::<Vec<_>>();
    for (name, nullable, declaration) in candidates {
        let Some((parameters, body)) = super::slot_move_route::defined_body(tokens, &name) else {
            continue;
        };
        if !validated(&body, &parameters, nullable, contracts) {
            withdrawn.push((name, declaration));
        } else {
            // Card VERDICT-CONTRACT-DEPENDENCIES (rule 13g).
            crate::contract_dependencies::validated(&name, "13d");
        }
    }
    if withdrawn.is_empty() {
        return None;
    }
    let mut settled = contracts.clone();
    for (name, mut declaration) in withdrawn {
        crate::contract_dependencies::withdrawn(&name, "13d", true);
        declaration.references_withdrawn = true;
        settled.insert_retaining(name, declaration);
    }
    Some(settled)
}

#[cfg(test)]
mod tests {
    use super::*;
    use qcore_schemas::contract::ResourceFunctionContract;
    use qcore_schemas::reference_count::ReferenceClause;

    fn words(text: &str) -> Vec<&str> {
        text.split_whitespace().collect()
    }

    fn environment() -> ResourceContracts {
        let mut contracts = ResourceContracts::default();
        let mut lookup = ResourceFunctionContract {
            parameters: vec![ResourceParameterMode::Borrowed],
            result: ResourceReturnMode::Owned { nullable: true },
            ..ResourceFunctionContract::default()
        };
        lookup.result_kind = ResourceKind::Reference;
        contracts.insert_contract("lookup", lookup);
        contracts.insert_contract(
            "alloc",
            ResourceFunctionContract {
                parameters: vec![ResourceParameterMode::Borrowed],
                result: ResourceReturnMode::Owned { nullable: true },
                ..ResourceFunctionContract::default()
            },
        );
        contracts.insert_retaining(
            "keep0",
            RetainingContract {
                contract: ResourceFunctionContract {
                    parameters: vec![ResourceParameterMode::Borrowed],
                    result: ResourceReturnMode::Derived {
                        parameter: 0,
                        extent: qcore_schemas::contract::DerivedExtent::Exact,
                        nullable: false,
                        const_dropped: false,
                    },
                    ..ResourceFunctionContract::default()
                },
                retentions: Vec::new(),
                resets: Vec::new(),
                slot_moves: Vec::new(),
                derived_outs: Vec::new(),
                references: vec![ReferenceClause {
                    parameter: 0,
                    operation: ReferenceOperation::Kept,
                }],
                references_withdrawn: false,
                callbacks: Vec::new(),
                callbacks_withdrawn: false,
                discriminant: None,
                arms: Vec::new(),
                arms_withdrawn: false,
                constructed: Default::default(),
                constructed_withdrawn: false,
                handoff: None,
                member_paths: Vec::new(),
                member_paths_withdrawn: false,
                path_retentions: Vec::new(),
                removals: Vec::new(),
                descriptor_cells: Default::default(),
                lifetime: None,
            },
        );
        contracts.insert_retaining(
            "drop0",
            RetainingContract {
                contract: ResourceFunctionContract {
                    parameters: vec![ResourceParameterMode::Borrowed],
                    result: ResourceReturnMode::None,
                    ..ResourceFunctionContract::default()
                },
                retentions: Vec::new(),
                resets: Vec::new(),
                slot_moves: Vec::new(),
                derived_outs: Vec::new(),
                references: vec![ReferenceClause {
                    parameter: 0,
                    operation: ReferenceOperation::Dropped,
                }],
                references_withdrawn: false,
                callbacks: Vec::new(),
                callbacks_withdrawn: false,
                discriminant: None,
                arms: Vec::new(),
                arms_withdrawn: false,
                constructed: Default::default(),
                constructed_withdrawn: false,
                handoff: None,
                member_paths: Vec::new(),
                member_paths_withdrawn: false,
                path_retentions: Vec::new(),
                removals: Vec::new(),
                descriptor_cells: Default::default(),
                lifetime: None,
            },
        );
        contracts
    }

    fn formals(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| (*name).to_owned()).collect()
    }

    #[test]
    fn the_measured_lookup_shapes_validate() {
        let contracts = environment();
        // `rd_kafka_toppar_get0`: a keep returned, or NULL.
        let get0 = words(
            "{ t * p ; if ( n ) p = o -> a [ n ] ; else return ( ( void * ) 0 ) ; \
             if ( p ) return keep0 ( p ) ; return ( ( void * ) 0 ) ; }",
        );
        assert!(validated(&get0, &formals(&["o", "n"]), true, &contracts));
        assert!(!validated(&get0, &formals(&["o", "n"]), false, &contracts));
        // `rd_kafka_toppar_get2`: a local assigned from a returned reference.
        let get2 = words(
            "{ t * p ; if ( ! k ) return ( ( void * ) 0 ) ; p = lookup ( k ) ; \
             unlock ( & k -> lock ) ; return p ; }",
        );
        assert!(validated(&get2, &formals(&["k"]), true, &contracts));
        // `rd_kafka_q_new0`: a fresh object from a declared allocation.
        let q_new0 = words(
            "{ q_t * q = alloc ( sizeof ( * q ) ) ; init ( q , rk ) ; \
             q -> flags |= 1 ; q -> name = func ; return q ; }",
        );
        assert!(validated(
            &q_new0,
            &formals(&["rk", "func"]),
            false,
            &contracts
        ));
        // A cast around the returned call.
        let cast = words("{ return ( t * ) lookup ( k ) ; }");
        assert!(validated(&cast, &formals(&["k"]), true, &contracts));
    }

    #[test]
    fn a_body_that_returns_anything_else_is_refused() {
        let contracts = environment();
        // A formal: the caller's object, no reference taken.
        let formal = words("{ return p ; }");
        assert!(!validated(&formal, &formals(&["p"]), true, &contracts));
        // An undeclared call.
        let other = words("{ return find ( k ) ; }");
        assert!(!validated(&other, &formals(&["k"]), true, &contracts));
        // A member read.
        let member = words("{ return o -> p ; }");
        assert!(!validated(&member, &formals(&["o"]), true, &contracts));
        // A local assigned from a member read.
        let read = words("{ t * p ; p = o -> p ; return p ; }");
        assert!(!validated(&read, &formals(&["o"]), true, &contracts));
        // A local dropped before it is returned.
        let dropped = words("{ t * p = lookup ( k ) ; drop0 ( p ) ; return p ; }");
        assert!(!validated(&dropped, &formals(&["k"]), true, &contracts));
        // A local whose address is taken.
        let address = words("{ t * p = lookup ( k ) ; f ( & p ) ; return p ; }");
        assert!(!validated(&address, &formals(&["k"]), true, &contracts));
        // A bare allocation returned without a local is not a reference.
        let bare = words("{ return alloc ( 8 ) ; }");
        assert!(!validated(&bare, &formals(&[]), true, &contracts));
        // NULL into a non-null declaration.
        let null = words("{ if ( k ) return lookup ( k ) ; return ( ( void * ) 0 ) ; }");
        assert!(!validated(&null, &formals(&["k"]), false, &contracts));
        // A jump.
        let jumps = words("{ t * p = lookup ( k ) ; goto out ; out : return p ; }");
        assert!(!validated(&jumps, &formals(&["k"]), true, &contracts));
        // No return at all.
        let none = words("{ f ( k ) ; }");
        assert!(!validated(&none, &formals(&["k"]), true, &contracts));
    }

    #[test]
    fn null_constants() {
        assert!(is_null_constant(&words("( ( void * ) 0 )")));
        assert!(is_null_constant(&words("0")));
        assert!(!is_null_constant(&words("p")));
        assert!(!is_null_constant(&words("( void * ) p")));
    }
}
