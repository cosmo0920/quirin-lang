//! Card W4-EXPR-POS: the typed neutral-expression-position admission.
//!
//! # What this module decides
//!
//! `legacy_c`'s six expression positions — a branch condition, a `while`
//! condition, a `for`-loop condition, a `do`/`while` condition, a `switch`
//! expression and a `return` expression — are classified by one shared token
//! predicate, `legacy_c::supported_neutral_expression`. That predicate is
//! purely lexical: it refuses any expression carrying a call, a brace or an
//! access its `contains_unmodeled_access` scan cannot rule out, and it has no
//! access to the contract environment. Card C03-REACH measured the consequence
//! (`docs/reports/remaining-c03-reach/README.md` §3.2): the six families this
//! one predicate produces are 618 of the retained instructions that keep the
//! neutral-frame recursive route's corpus reach at zero.
//!
//! `docs/design/W4_NEUTRAL_EXPRESSION_POSITIONS.md` §5 specifies the widening
//! and names its architectural cost: the premise the rule needs is the contract
//! environment `build_function_cfg` is *already handed*, and the work is to
//! make it reachable from the predicate rather than to loosen the predicate
//! where it lives. That is what this module is. The predicate stays exactly as
//! it is; `Parser` gains a method that consults it **first** and only then asks
//! this module, so the admission is strictly additive: an expression this
//! project accepted before is accepted by the same code, for the same reason.
//!
//! # The three admitted families, each conjunctive and typed
//!
//! **F1, the neutral call.** A call `f ( … )` is resource-neutral when the
//! contract *in force at this point of the bottom-up walk* declares every
//! parameter [`ResourceParameterMode::Borrowed`] and the result
//! [`ResourceReturnMode::None`], the written argument count equals the declared
//! parameter count, and every argument is itself admitted by this same
//! predicate. Under those conditions the call has no custody effect the
//! projection could observe: the result is untracked, and — by F2 and F3 — no
//! argument can name a pointer at all.
//!
//! **F2, the typed scalar operand.** Every identifier outside an admitted call
//! span is an enumeration constant or a variable this function declares with a
//! non-pointer, non-array arithmetic or enumeration type; every other token is
//! a numeric literal or one of C02's admitted operators. `&`, `*`, `[`, `]`,
//! `{`, `}`, `,` and `;` are absent from that operator set, so an address, an
//! indirection, a subscript, an aggregate and the comma operator each refuse.
//!
//! **F3, the typed direct scalar-field read.** Exactly card C02's whitelist,
//! evaluated in an expression position instead of a statement position: a
//! direct `base -> field` or `base . field` where `base` is declared exactly
//! `struct TAG *` or `struct TAG`, `TAG` is defined once with every member read
//! and no `union`, bitfield or nested anonymous aggregate, `field` is a
//! non-pointer, non-array arithmetic or enumeration member without `volatile`
//! or `_Atomic`, the access does not continue, and `base` is **not** a name
//! this function may bind to a resource identity.
//!
//! F2 and F3 are not re-implemented here. Both are
//! [`crate::typed_field_access::scalar_statement_accesses`], the single walker
//! card C02 landed and `QCore.LegacyCAnalyzer`'s whitelist theorems already
//! cover; this module calls it through
//! [`crate::typed_field_access::is_scalar_operand_expression`] on the tokens
//! that lie outside every admitted call span.
//!
//! # Why the permissive direction is safe here
//!
//! The load-bearing consequence of the conjunction is one sentence:
//!
//! > **An admitted neutral expression contains no pointer-valued operand
//! > anywhere.**
//!
//! Every identifier is an enumeration constant, a declared *scalar* variable,
//! a callee name whose contract is neutral, or the base or field of an admitted
//! direct scalar-field access whose base the body never binds. A pointer-typed
//! local, a formal holding a resource, a `void *`, an array and every base the
//! body may bind are each refused by name.
//!
//! That is what makes turning the node into a `NoOp` sound rather than merely
//! convenient. A statement-position call is modeled — it checks the liveness of
//! its `Borrowed` actuals, and card C02-P-ALIAS attaches a may-alias obligation
//! to each of its read arguments. Skipping that modeling would be exactly the
//! C00 hazard the project already paid for once. Here nothing is skipped,
//! because there is nothing to model: no argument can carry a tracked identity,
//! and `crate::may_alias` relates only names declared `struct TAG *`, none of
//! which can appear.
//!
//! # What is deliberately not admitted
//!
//! * A call whose callee has **no** contract in force, or whose contract
//!   carries any non-`Borrowed` parameter or a non-`None` result. An
//!   `Acquire`, `Release`, `Escape`, `Out`, `Stored`, `Removed` or
//!   `Reallocated` callee refuses the whole expression.
//! * A call whose written arity differs from its declared parameter count.
//!   A variadic callee therefore refuses at every arity but its declared one,
//!   which is the same treatment `O04-ARMS-VARIADIC` records.
//! * A call through a function pointer, a member (`s.f(…)`, `p->f(…)`) or an
//!   expression (`(*fp)(…)`): none of those is a plain declared callee name,
//!   and each keeps the existing refusal.
//! * `sizeof`, `_Alignof` and every other reserved word in call position: they
//!   are call-shaped to [`legacy_c::is_identifier`] and are not admitted here,
//!   so the whole expression refuses exactly as it does today.
//! * Any expression in which *some* call-shaped occurrence is not admitted.
//!   The rule is all-or-nothing over the expression: one unadmitted call
//!   refuses the whole position, so no unmodelled call can hide beside an
//!   admitted one.
//!
//! Nothing here changes a diagnostic code, an admission of a *statement*, the
//! null-test refinement, the pointer-inequality refinement or any contract.

use qcore_schemas::contract::{ResourceContracts, ResourceParameterMode, ResourceReturnMode};

use crate::legacy_c::is_identifier;
use crate::typed_field_access::{
    FunctionTypes, TypedAccessModel, is_name, is_scalar_operand_expression,
};

/// Nested admitted-call depth bound. A deeper expression is refused rather
/// than walked, in the style of the bounded access chain of
/// [`crate::borrowed_actual`]: the bound is a refusal, never a truncation.
const CALL_NESTING_LIMIT: usize = 4;

/// Token-count bound for one classified expression. Beyond it the expression is
/// refused, so the walk is finite on any input the importer can hand it.
const EXPRESSION_TOKEN_LIMIT: usize = 4096;

/// Whether an expression position is resource-neutral under the contract
/// environment in force.
///
/// This is consulted **only after** `legacy_c::supported_neutral_expression`
/// has already refused, so a `true` here is always a new admission and a
/// `false` here always leaves the existing refusal in place.
pub(crate) fn is_neutral_expression(
    texts: &[&str],
    contracts: &ResourceContracts,
    model: &TypedAccessModel,
    types: &FunctionTypes,
) -> bool {
    if texts.is_empty() || texts.len() > EXPRESSION_TOKEN_LIMIT {
        return false;
    }
    neutral(texts, contracts, model, types, 0)
}

fn neutral(
    texts: &[&str],
    contracts: &ResourceContracts,
    model: &TypedAccessModel,
    types: &FunctionTypes,
    depth: usize,
) -> bool {
    if depth > CALL_NESTING_LIMIT {
        return false;
    }
    let mut spans: Vec<(usize, usize)> = Vec::new();
    let mut index = 0usize;
    while index + 1 < texts.len() {
        if !is_call_shaped(texts, index) {
            index += 1;
            continue;
        }
        // All-or-nothing: an occurrence this rule cannot admit refuses the
        // whole expression, exactly as `contains_call` refuses it today.
        let Some(end) = admitted_neutral_call(texts, index, contracts, model, types, depth) else {
            return false;
        };
        spans.push((index, end));
        index = end;
    }
    let outside = outside_spans(texts, &spans);
    is_scalar_operand_expression(&outside, model, types)
}

/// Whether `texts[index]` opens a call-shaped occurrence.
///
/// Deliberately the *same* test `legacy_c::find_call` makes, including its
/// treatment of reserved words as identifiers, so that the set of occurrences
/// this rule must account for is exactly the set the existing lexical predicate
/// refuses on.
fn is_call_shaped(texts: &[&str], index: usize) -> bool {
    is_identifier(texts[index]) && texts.get(index + 1) == Some(&"(")
}

/// The index just past the closing parenthesis of an admitted neutral call at
/// `index`, or `None`.
fn admitted_neutral_call(
    texts: &[&str],
    index: usize,
    contracts: &ResourceContracts,
    model: &TypedAccessModel,
    types: &FunctionTypes,
    depth: usize,
) -> Option<usize> {
    let name = texts[index];
    // A reserved word is never a declared callee.
    if !is_name(name) {
        return None;
    }
    // The callee must be a plain name, not a selected member and not a
    // declarator following a type specifier or a literal.
    if index > 0 {
        let previous = texts[index - 1];
        if previous == "."
            || (previous == ">" && index >= 2 && texts[index - 2] == "-")
            || is_identifier(previous)
            || previous.as_bytes().first().is_some_and(u8::is_ascii_digit)
        {
            return None;
        }
    }
    let close = matching_close(texts, index + 1)?;
    let contract = contracts.contract(name)?;
    if contract.result != ResourceReturnMode::None {
        return None;
    }
    if !contract
        .parameters
        .iter()
        .all(|mode| *mode == ResourceParameterMode::Borrowed)
    {
        return None;
    }
    let inner = &texts[index + 2..close];
    let arguments = if inner.is_empty() {
        Vec::new()
    } else {
        split_arguments(inner)
    };
    // An exact arity, so a variadic or mis-declared callee cannot pass an
    // argument this rule never examined.
    if arguments.len() != contract.parameters.len() {
        return None;
    }
    for argument in &arguments {
        if argument.is_empty() || !neutral(argument, contracts, model, types, depth + 1) {
            return None;
        }
    }
    Some(close + 1)
}

/// The index of the `)` matching the `(` at `open`, or `None` when the
/// expression is unbalanced.
fn matching_close(texts: &[&str], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (index, text) in texts.iter().enumerate().skip(open) {
        match *text {
            "(" => depth += 1,
            ")" => {
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

/// One call's arguments, split on the commas that are not inside a nested
/// grouping. `[` and `{` are counted so that a comma inside one is not a
/// separator; both refuse later anyway, in [`is_scalar_operand_expression`].
fn split_arguments<'a>(texts: &[&'a str]) -> Vec<Vec<&'a str>> {
    let mut groups: Vec<Vec<&'a str>> = vec![Vec::new()];
    let mut depth = 0usize;
    for text in texts {
        match *text {
            "(" | "[" | "{" => depth += 1,
            ")" | "]" | "}" => depth = depth.saturating_sub(1),
            "," if depth == 0 => {
                groups.push(Vec::new());
                continue;
            }
            _ => {}
        }
        if let Some(last) = groups.last_mut() {
            last.push(*text);
        }
    }
    groups
}

/// The tokens outside every admitted call span.
///
/// Deleting a whole `NAME ( … )` span can only bring operators or parentheses
/// into contact. It can never manufacture an admitted access out of neighbours
/// that did not have one, because every token an access needs — `*`, `&`, `[`,
/// `]`, `.` and the `-` `>` pair — is refused by
/// [`is_scalar_operand_expression`] wherever it survives, and a deleted span
/// contributes no identifier to the result.
fn outside_spans<'a>(texts: &[&'a str], spans: &[(usize, usize)]) -> Vec<&'a str> {
    texts
        .iter()
        .enumerate()
        .filter(|(index, _)| {
            !spans
                .iter()
                .any(|(start, end)| index >= start && index < end)
        })
        .map(|(_, text)| *text)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use qcore_schemas::contract::{ResourceApiKind, ResourceFunctionContract};
    use std::collections::BTreeSet;

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

    /// A neutral contract of the given arity: exactly what
    /// `legacy_neutral::neutral_contract` seeds and what ordinary W4 inference
    /// publishes for an effect-free body.
    fn neutral_contract(arity: usize) -> ResourceFunctionContract {
        ResourceFunctionContract::new(
            vec![ResourceParameterMode::Borrowed; arity],
            ResourceReturnMode::None,
        )
    }

    fn environment(entries: Vec<(&str, ResourceFunctionContract)>) -> ResourceContracts {
        let mut contracts = ResourceContracts::default();
        for (name, contract) in entries {
            contracts.insert_contract(name, contract);
        }
        contracts
    }

    /// Declaration facts for a fixture: `unit` is the translation-unit text and
    /// `body` the tokens between the function's braces. No formal is protected,
    /// which is the neutral route's own entry shape.
    fn facts(unit: &str, body: &str) -> (TypedAccessModel, FunctionTypes) {
        let unit_tokens = texts(unit);
        let model = TypedAccessModel::read(&unit_tokens);
        let body_tokens = texts(body);
        let types = FunctionTypes::read(&model, &[], &body_tokens, &BTreeSet::new());
        (model, types)
    }

    fn admits(unit: &str, body: &str, expression: &str, env: &ResourceContracts) -> bool {
        let (model, types) = facts(unit, body);
        let tokens = texts(expression);
        is_neutral_expression(&tokens, env, &model, &types)
    }

    const SCALAR_BODY: &str = "unsigned long v ; int n ;";
    const NO_UNIT: &str = "struct r { int a ; struct q * p ; } ;";

    #[test]
    fn the_digits10_return_expression_is_admitted() {
        let env = environment(vec![("digits10", neutral_contract(1))]);
        assert!(admits(
            NO_UNIT,
            SCALAR_BODY,
            "12 + digits10 ( v / 1000000000000UL )",
            &env
        ));
    }

    #[test]
    fn a_call_without_a_contract_refuses() {
        let env = environment(Vec::new());
        assert!(!admits(NO_UNIT, SCALAR_BODY, "12 + digits10 ( v )", &env));
    }

    #[test]
    fn a_releasing_callee_refuses() {
        let env = environment(vec![(
            "digits10",
            ResourceFunctionContract::from(ResourceApiKind::Release),
        )]);
        assert!(!admits(NO_UNIT, SCALAR_BODY, "digits10 ( v )", &env));
    }

    #[test]
    fn an_acquiring_callee_refuses() {
        let env = environment(vec![(
            "make",
            ResourceFunctionContract::from(ResourceApiKind::Acquire),
        )]);
        assert!(!admits(NO_UNIT, SCALAR_BODY, "make ( )", &env));
    }

    #[test]
    fn an_owned_result_refuses() {
        let env = environment(vec![(
            "peek",
            ResourceFunctionContract::new(
                [ResourceParameterMode::Borrowed],
                ResourceReturnMode::Owned { nullable: true },
            ),
        )]);
        assert!(!admits(NO_UNIT, SCALAR_BODY, "peek ( v )", &env));
    }

    #[test]
    fn a_wrong_arity_refuses() {
        let env = environment(vec![("f", neutral_contract(1))]);
        assert!(!admits(NO_UNIT, SCALAR_BODY, "f ( v , n )", &env));
        assert!(!admits(NO_UNIT, SCALAR_BODY, "f ( )", &env));
    }

    #[test]
    fn a_pointer_argument_refuses() {
        let env = environment(vec![("f", neutral_contract(1))]);
        let unit = "struct q { int a ; } ;";
        let body = "struct q * p ;";
        assert!(!admits(unit, body, "f ( p )", &env));
    }

    #[test]
    fn an_address_of_argument_refuses() {
        let env = environment(vec![("f", neutral_contract(1))]);
        assert!(!admits(NO_UNIT, SCALAR_BODY, "f ( & n )", &env));
    }

    #[test]
    fn a_dereferenced_argument_refuses() {
        let env = environment(vec![("f", neutral_contract(1))]);
        let unit = "struct q { int a ; } ;";
        let body = "int * q ;";
        assert!(!admits(unit, body, "f ( * q )", &env));
    }

    #[test]
    fn a_subscripted_argument_refuses() {
        let env = environment(vec![("f", neutral_contract(1))]);
        let unit = "struct q { int a ; } ;";
        let body = "int a [ 4 ] ;";
        assert!(!admits(unit, body, "f ( a [ 0 ] )", &env));
    }

    #[test]
    fn a_member_call_refuses() {
        let env = environment(vec![("f", neutral_contract(1))]);
        let unit = "struct r { int a ; } ;";
        let body = "struct r * s ; unsigned long v ;";
        assert!(!admits(unit, body, "s -> f ( v )", &env));
        assert!(!admits(unit, body, "s . f ( v )", &env));
    }

    #[test]
    fn a_function_pointer_call_refuses() {
        let env = environment(vec![("fp", neutral_contract(1))]);
        let body = "unsigned long v ;";
        assert!(!admits(NO_UNIT, body, "( * fp ) ( v )", &env));
    }

    #[test]
    fn sizeof_is_never_an_admitted_call() {
        let env = environment(vec![("sizeof", neutral_contract(1))]);
        assert!(!admits(NO_UNIT, SCALAR_BODY, "sizeof ( n )", &env));
    }

    #[test]
    fn one_unadmitted_call_refuses_the_whole_expression() {
        let env = environment(vec![("f", neutral_contract(1))]);
        assert!(!admits(NO_UNIT, SCALAR_BODY, "f ( n ) + g ( n )", &env));
    }

    #[test]
    fn a_comma_operator_refuses() {
        let env = environment(vec![("f", neutral_contract(1))]);
        assert!(!admits(NO_UNIT, SCALAR_BODY, "f ( n ) , n", &env));
    }

    #[test]
    fn a_nested_neutral_call_is_admitted() {
        let env = environment(vec![("f", neutral_contract(1)), ("g", neutral_contract(1))]);
        assert!(admits(NO_UNIT, SCALAR_BODY, "f ( g ( n ) )", &env));
    }

    #[test]
    fn a_nested_unadmitted_call_refuses() {
        let env = environment(vec![("f", neutral_contract(1))]);
        assert!(!admits(NO_UNIT, SCALAR_BODY, "f ( g ( n ) )", &env));
    }

    #[test]
    fn a_direct_scalar_field_read_is_admitted() {
        let env = environment(Vec::new());
        let unit = "struct r { int a ; struct q * p ; } ;";
        let body = "struct r * s ;";
        assert!(admits(unit, body, "s -> a", &env));
    }

    #[test]
    fn a_pointer_field_read_refuses() {
        let env = environment(Vec::new());
        let unit = "struct r { int a ; struct q * p ; } ;";
        let body = "struct r * s ;";
        assert!(!admits(unit, body, "s -> p", &env));
    }

    #[test]
    fn a_bound_base_refuses() {
        let env = environment(Vec::new());
        let unit = "struct r { int a ; } ;";
        // `s` appears in a call argument list, so `bound_names` records it.
        let body = "struct r * s ; use ( s ) ;";
        assert!(!admits(unit, body, "s -> a", &env));
    }

    #[test]
    fn an_unreadable_declaration_refuses_every_expression() {
        let env = environment(vec![("f", neutral_contract(1))]);
        let (model, types) = facts(NO_UNIT, "struct { int a ; } anon ;");
        assert!(!types.is_readable());
        let tokens = texts("f ( 1 )");
        assert!(!is_neutral_expression(&tokens, &env, &model, &types));
    }

    #[test]
    fn an_empty_expression_refuses() {
        let env = environment(Vec::new());
        assert!(!admits(NO_UNIT, SCALAR_BODY, "", &env));
    }

    #[test]
    fn a_deeply_nested_call_chain_refuses_at_the_bound() {
        let env = environment(vec![("f", neutral_contract(1))]);
        assert!(admits(
            NO_UNIT,
            SCALAR_BODY,
            "f ( f ( f ( f ( n ) ) ) )",
            &env
        ));
        assert!(!admits(
            NO_UNIT,
            SCALAR_BODY,
            "f ( f ( f ( f ( f ( f ( n ) ) ) ) ) )",
            &env
        ));
    }

    #[test]
    fn an_unbalanced_call_refuses() {
        let env = environment(vec![("f", neutral_contract(1))]);
        assert!(!admits(NO_UNIT, SCALAR_BODY, "f ( n", &env));
    }
}
