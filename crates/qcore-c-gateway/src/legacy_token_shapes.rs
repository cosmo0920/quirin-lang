//! Card GATEWAY-ANALYSIS-PATH-OPTIMIZATION: lexical shape readers over
//! token texts: calls, arguments, conditions, places and declarators.
//!
//! Moved verbatim from `legacy_c.rs` (only visibility widened to
//! `pub(super)` where the parent or a sibling module reads an item).

use super::*;

/// Whether `tokens` is exactly one balanced parenthesized group carrying no
/// call, no assignment and no comma — the shape of a cast applied to the value
/// that follows it. An empty slice is not a cast; nor is anything with a
/// second top-level group.
pub(super) fn is_leading_cast(tokens: &[&str]) -> bool {
    if tokens.first() != Some(&"(") || tokens.last() != Some(&")") || tokens.len() < 3 {
        return false;
    }
    let mut depth = 0usize;
    for (index, text) in tokens.iter().enumerate() {
        match *text {
            "(" => depth += 1,
            ")" => {
                depth = match depth.checked_sub(1) {
                    Some(depth) => depth,
                    None => return false,
                };
                if depth == 0 && index + 1 != tokens.len() {
                    return false;
                }
            }
            "=" | "," | "[" | "]" | "{" | "}" | ";" => return false,
            _ => {}
        }
    }
    depth == 0 && find_call(tokens).is_none()
}

pub(super) fn find_call<'a>(tokens: &'a [&'a str]) -> Option<(usize, &'a str)> {
    tokens.windows(2).enumerate().find_map(|(index, pair)| {
        (is_identifier(pair[0]) && pair[1] == "(").then_some((index, pair[0]))
    })
}

/// C type-operator tokens, distinguished lexically from ordinary callees.
/// This list alone proves no operand unevaluated: sizeof can evaluate VLA
/// bounds. The fixed scalar query rule establishes its narrower typed premise.
pub(crate) const TYPE_OPERATORS: [&str; 6] = [
    "sizeof",
    "_Alignof",
    "alignof",
    "typeof",
    "__typeof",
    "__typeof__",
];

/// The name of a call in `texts` other than the one whose callee is at
/// `call_index`, ignoring the C type operators (card O04-B09B10).
///
/// [`find_call`] returns the *first* call of a statement and the modeling below
/// covers exactly that one, so any further call would have its acquisitions,
/// releases and transfers silently dropped. Reporting it is strictly more
/// refusing: this can only withdraw a projection, never create one.
pub(super) fn nested_call<'a>(texts: &[&'a str], call_index: usize) -> Option<&'a str> {
    texts.windows(2).enumerate().find_map(|(index, pair)| {
        (index != call_index
            && is_identifier(pair[0])
            && pair[1] == "("
            && !TYPE_OPERATORS.contains(&pair[0]))
        .then_some(pair[0])
    })
}

pub(super) fn contains_call(tokens: &[Token]) -> bool {
    let texts = tokens
        .iter()
        .map(|token| token.text.as_str())
        .collect::<Vec<_>>();
    find_call(&texts).is_some()
}

/// The tokens of `texts[from..]` that no admitted borrowed pointer-field
/// actual covers.
///
/// Every admitted span is a *whole* call argument, so deleting it leaves the
/// surrounding punctuation adjacent (`( ,` or `( )`) and can never manufacture
/// a unary `*` or a member `.` out of neighbours that did not have one.
/// Card **W43-INDIRECT-ACCESS-GENERAL**: the refusals after which the typed
/// scalar-access route is consulted. Each names the access, the destination
/// or the increment as the reason, never a call or a formal.
/// The formal-mutation gate's refusal (see `Parser::classify_simple_base`).
/// Card W43-INDEX-TRAVERSAL consults a recognised step of a non-formal, or a
/// stepped access none of whose stepped names is a formal, after it.
pub(super) const FORMAL_MUTATION_REFUSAL: &str =
    "unmodeled expression may mutate a formal resource identity";

/// Card CG02-ELABORATION-CORE: whether a lowered `return` is the position's
/// generic refusal (not a cast-return refusal, which stays).
pub(super) fn return_refused(statement: &Statement) -> bool {
    matches!(
        statement,
        Statement::Return {
            expression_supported: false,
            returned_evaluation: None,
            ..
        }
    )
}
/// Card **W43-INDEXED-RESOURCE-COLLECTIONS**: the plain name an element place
/// is renamed to while the ordinary classifier reads the call around it. A
/// reserved-looking identifier no C program of the corpus declares.
pub(super) const ELEMENT_TEMPORARY: &str = "__qcore_element__";

pub(super) const TYPED_SCALAR_CONSULTED: &[&str] = &[
    "indirect memory access is outside the resource projection",
    "unsupported assignment expression",
    "unrecognized assignment destination",
    "unmodeled increment or decrement expression",
];

pub(super) fn outside_spans<'a>(
    texts: &[&'a str],
    spans: &[(usize, usize)],
    from: usize,
) -> Vec<&'a str> {
    texts
        .iter()
        .enumerate()
        .skip(from)
        .filter(|(index, _)| {
            !spans
                .iter()
                .any(|(start, end)| index >= start && index < end)
        })
        .map(|(_, text)| *text)
        .collect()
}

/// Whether a declaration statement's every bracket group is an integer constant
/// bound (or an empty `[]`), so that the declaration allocates automatic
/// storage and evaluates nothing. A variable-length bound, a designator or a
/// function declarator is refused.
pub(super) fn constant_array_declaration(texts: &[&str]) -> bool {
    if texts
        .iter()
        .any(|text| matches!(*text, "(" | ")" | "{" | "}" | "="))
    {
        return false;
    }
    let mut cursor = 0usize;
    while cursor < texts.len() {
        if texts[cursor] != "[" {
            cursor += 1;
            continue;
        }
        let Some(close) = texts.iter().skip(cursor).position(|text| *text == "]") else {
            return false;
        };
        let bound = &texts[cursor + 1..cursor + close];
        if !bound
            .iter()
            .all(|text| text.bytes().all(|byte| byte.is_ascii_alphanumeric()) && is_numeric(text))
        {
            return false;
        }
        cursor += close + 1;
    }
    true
}

pub(super) fn is_numeric(text: &str) -> bool {
    text.as_bytes().first().is_some_and(u8::is_ascii_digit)
}

/// This is a lexical exclusion boundary, not a proof of C memory safety.
/// Only multiplication with a plain left operand is identifiable here; a star
/// after a closing parenthesis may follow a typedef cast and remains unknown.
pub(super) fn contains_unmodeled_access(texts: &[&str]) -> bool {
    texts.windows(2).any(|pair| pair == ["-", ">"])
        || texts.iter().enumerate().any(|(index, text)| match *text {
            "[" | "]" => true,
            "." => {
                !texts
                    .get(index.wrapping_sub(1))
                    .is_some_and(|left| left.bytes().all(|byte| byte.is_ascii_digit()))
                    || !texts
                        .get(index + 1)
                        .is_some_and(|right| right.bytes().all(|byte| byte.is_ascii_digit()))
            }
            // Card W48-BU-SINGLE-LEAF-BODIES (rule 35, arm 1; its own package
            // since card RULE35-PILOT): a `*` applied to a literal that is
            // only read reads a constant of a static object, never a pointer
            // the projection names.
            "*" if crate::rule_hosts::literal_read(texts, index) => false,
            "*" => !texts.get(index.wrapping_sub(1)).is_some_and(|left| {
                (is_identifier(left) || left.bytes().all(|byte| byte.is_ascii_digit()))
                    && !matches!(
                        *left,
                        "sizeof"
                            | "_Alignof"
                            | "alignof"
                            | "__extension__"
                            | "__real__"
                            | "__real"
                            | "__imag__"
                            | "__imag"
                    )
            }),
            _ => false,
        })
}

pub(super) fn supported_neutral_expression(tokens: &[Token]) -> bool {
    !contains_call(tokens)
        && !contains_braces(tokens)
        && !contains_unmodeled_access(
            &tokens
                .iter()
                .map(|token| token.text.as_str())
                .collect::<Vec<_>>(),
        )
}

pub(super) fn contains_braces(tokens: &[Token]) -> bool {
    tokens
        .iter()
        .any(|token| matches!(token.text.as_str(), "{" | "}"))
}

/// Is a controlling expression the single literal token `0`?
///
/// This is the whole key of the constant `do … while (0)` rule (card
/// `C-GATEWAY-CONSTANT-DO-WHILE`). It is deliberately a **token** test and
/// deliberately the narrowest one:
///
/// * Standing constraint 5a forbids deciding control flow from an *evaluated*
///   expression, so nothing here folds constants, reads a binding's value or
///   consults a scalar fact. `while (x)` with `x` known zero, `while (1 - 1)`
///   and `while ((0))` all answer `false` and keep the back edge.
/// * Answering `false` is the conservative answer. Keeping the back edge
///   over-approximates the paths, which can only *add* refusals and
///   accusations; answering `true` removes paths, which is the admitting
///   direction invariant 1 governs. Every widening of this predicate
///   therefore needs its own measured re-pin.
/// * Only the spelling `0` is claimed. `0u`, `0L`, `0x0` and `'\0'` are
///   literal zeros too and are **not** claimed here
///   (`CONSTANT-DO-WHILE-ZERO-SPELLINGS`).
pub(super) fn condition_is_literal_zero(tokens: &[Token]) -> bool {
    matches!(tokens, [token] if token.text == "0")
}

/// Split a `for` header into its three clauses.
///
/// The grouping tokens are consumed by the depth tracking and do **not** reach
/// the clause vectors. That is load-bearing for the `init` and `increment`
/// clauses, whose classification by [`Parser::classify_simple`] this card does
/// not touch, and it is why [`for_condition_tokens`] exists: the condition is
/// classified from the original spelling instead, so that a call, a subscript
/// or a parenthesized group in a `for` condition is visible to the predicates
/// that must refuse it. See `W4-EXPR-POS-FOR-CLAUSES` in
/// `docs/reports/remaining-w4-expr-pos/README.md`.
pub(super) fn split_for_clauses(tokens: &[Token]) -> Option<[Vec<Token>; 3]> {
    let mut clauses = vec![Vec::new()];
    let mut parentheses = 0usize;
    let mut brackets = 0usize;
    for token in tokens {
        match token.text.as_str() {
            "(" => parentheses += 1,
            ")" => parentheses = parentheses.checked_sub(1)?,
            "[" => brackets += 1,
            "]" => brackets = brackets.checked_sub(1)?,
            ";" if parentheses == 0 && brackets == 0 => clauses.push(Vec::new()),
            _ => clauses.last_mut()?.push(token.clone()),
        }
    }
    clauses.try_into().ok()
}

/// The `for` header's condition clause, **with its grouping tokens intact**.
///
/// A measured fail-open repair (card W4-EXPR-POS). `split_for_clauses` drops
/// `(`, `)`, `[` and `]`, so `for (; external(n); )` reached
/// `supported_neutral_expression` as the two tokens `external n`, in which
/// `contains_call` finds no call and `contains_unmodeled_access` finds no
/// subscript: the position was classified `NoOp` and the unmodelled call
/// disappeared. Every other position was already classified from its original
/// spelling; this makes the `for` condition agree with them.
///
/// This is a **refusal** repair — it can only turn an accepted condition into a
/// refused one — and it is confined to the condition. The `init` and
/// `increment` clauses keep the stripped spelling they have today, byte for
/// byte.
pub(super) fn for_condition_tokens(tokens: &[Token]) -> Option<Vec<Token>> {
    let mut clauses = vec![Vec::new()];
    let mut parentheses = 0usize;
    let mut brackets = 0usize;
    for token in tokens {
        match token.text.as_str() {
            "(" => parentheses += 1,
            ")" => parentheses = parentheses.checked_sub(1)?,
            "[" => brackets += 1,
            "]" => brackets = brackets.checked_sub(1)?,
            ";" if parentheses == 0 && brackets == 0 => {
                clauses.push(Vec::new());
                continue;
            }
            _ => {}
        }
        clauses.last_mut()?.push(token.clone());
    }
    let [_, condition, _]: [Vec<Token>; 3] = clauses.try_into().ok()?;
    Some(condition)
}

pub(super) fn pointer_null_condition(tokens: &[Token]) -> Option<PointerNullCondition> {
    let texts = tokens
        .iter()
        .map(|token| token.text.as_str())
        .collect::<Vec<_>>();
    if !supported_neutral_expression(tokens) {
        return None;
    }
    null_condition_text(&texts)
}

pub(super) fn null_condition_text(texts: &[&str]) -> Option<PointerNullCondition> {
    let texts = strip_parentheses(texts);
    if let Some(rest) = texts.strip_prefix(&["!"]) {
        let mut condition = null_condition_text(rest)?;
        condition.true_is_null = !condition.true_is_null;
        return Some(condition);
    }
    if let Some(source) = canonical_place(texts) {
        return Some(PointerNullCondition {
            source,
            true_is_null: false,
            validity: None,
            result_test: None,
        });
    }
    let mut depth = 0usize;
    for index in 0..texts.len().saturating_sub(1) {
        match texts[index] {
            "(" | "[" => depth += 1,
            ")" | "]" => depth = depth.checked_sub(1)?,
            "=" | "!" if depth == 0 && texts[index + 1] == "=" => {
                let left = &texts[..index];
                let right = &texts[index + 2..];
                let source = if is_null_constant(left) {
                    canonical_place(right)?
                } else if is_null_constant(right) {
                    canonical_place(left)?
                } else {
                    return None;
                };
                return Some(PointerNullCondition {
                    source,
                    true_is_null: texts[index] == "=",
                    validity: None,
                    result_test: None,
                });
            }
            _ => {}
        }
    }
    None
}

// Only erase parentheses that enclose the complete expression. In particular,
// `(p) + offset` must not become the resource place `p`.
pub(super) fn strip_parentheses<'a>(mut tokens: &'a [&'a str]) -> &'a [&'a str] {
    while tokens.first() == Some(&"(") && tokens.last() == Some(&")") {
        let mut depth = 0usize;
        let mut closes_at_end = false;
        for (index, token) in tokens.iter().enumerate() {
            match *token {
                "(" => depth += 1,
                ")" => {
                    let Some(next) = depth.checked_sub(1) else {
                        return tokens;
                    };
                    depth = next;
                    if depth == 0 {
                        closes_at_end = index == tokens.len() - 1;
                        break;
                    }
                }
                _ => {}
            }
        }
        if !closes_at_end {
            break;
        }
        tokens = &tokens[1..tokens.len() - 1];
    }
    tokens
}

pub(super) fn is_null_constant(tokens: &[&str]) -> bool {
    let tokens = strip_parentheses(tokens);
    matches!(tokens, ["NULL" | "nullptr" | "0"])
        || tokens
            .strip_prefix(&["(", "void", "*", ")"])
            .is_some_and(|rest| strip_parentheses(rest) == ["0"])
}

pub(crate) fn call_arguments<'a>(
    tokens: &'a [&'a str],
    call_index: usize,
) -> Option<Vec<&'a [&'a str]>> {
    Some(
        call_argument_spans(tokens, call_index)?
            .into_iter()
            .map(|(start, end)| &tokens[start..end])
            .collect(),
    )
}

/// The half-open token spans of one call's arguments, in order.
///
/// [`call_arguments`] is exactly this, resolved against `tokens`; card
/// O04-TRACE needs the indices themselves so an argument can be recognised as
/// *being* a nested call at a known position.
pub(crate) fn call_argument_spans(
    tokens: &[&str],
    call_index: usize,
) -> Option<Vec<(usize, usize)>> {
    let mut arguments = Vec::new();
    let mut start = call_index + 2;
    let mut parentheses = 0usize;
    let mut brackets = 0usize;
    let mut cursor = start;
    while cursor < tokens.len() {
        match tokens[cursor] {
            "(" => parentheses += 1,
            ")" if parentheses == 0 && brackets == 0 => {
                if cursor > start {
                    arguments.push((start, cursor));
                }
                return Some(arguments);
            }
            ")" => parentheses = parentheses.checked_sub(1)?,
            "[" => brackets += 1,
            "]" => brackets = brackets.checked_sub(1)?,
            "," if parentheses == 0 && brackets == 0 => {
                arguments.push((start, cursor));
                start = cursor + 1;
            }
            _ => {}
        }
        cursor += 1;
    }
    None
}

/// The modeled place of one actual at a declared `Borrowed` position.
pub(crate) fn borrowed_argument_place(argument: &[&str]) -> Option<String> {
    contract_argument_place(argument, ResourceParameterMode::Borrowed)
}

pub(super) fn contract_argument_place(
    argument: &[&str],
    mode: ResourceParameterMode,
) -> Option<String> {
    match mode {
        // Retain direct borrowed names for the enclosing Out-slot identity
        // guard; the callee's borrowed resource effect still consumes nothing.
        ResourceParameterMode::Borrowed => canonical_place(argument),
        ResourceParameterMode::Out { .. } => {
            let argument = strip_parentheses(argument);
            if argument.first() == Some(&"&") {
                canonical_place(&argument[1..])
            } else {
                // A bare pointer may forward the current function's out slot.
                // Naming *formal is not permission to write it: transfer must
                // find that exact slot sanctioned and its pointer unchanged.
                match argument {
                    [name] if is_identifier(name) && !matches!(*name, "NULL" | "nullptr") => {
                        Some(format!("*{name}"))
                    }
                    _ => None,
                }
            }
        }
        ResourceParameterMode::Owned { .. }
        | ResourceParameterMode::Consumed { .. }
        | ResourceParameterMode::Released { .. }
        | ResourceParameterMode::Stored { .. }
        | ResourceParameterMode::Removed { .. } => canonical_place(argument),
    }
}

/// What the base actual of a declared derived result denotes (card
/// W43-LIBC-RESULT-ALIAS).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum DerivedBase {
    /// A place, or a place plus or minus one offset token. C pointer arithmetic
    /// that is defined stays inside the object the place points to (C11
    /// 6.5.6p8), so the result is derived from the place's targets. `offset`
    /// records the second shape, whose value is not the place's own (card
    /// W43-RESULT-ALIAS-RETURNED reads it as interior).
    Place { place: String, offset: bool },
    /// A string literal or the address of a plain name: an object with static
    /// or automatic storage that no tracked identity can stand for.
    Untracked,
    /// Anything else.
    Unmodeled,
}

/// Read the base actual of a declared derived result. Structural only: no
/// value is computed, and every shape not listed on [`DerivedBase`] is
/// `Unmodeled`.
pub(super) fn derived_base(argument: &[&str]) -> DerivedBase {
    let argument = strip_parentheses(argument);
    if let Some(place) = canonical_place(argument) {
        return DerivedBase::Place {
            place,
            offset: false,
        };
    }
    match argument {
        [literal] if literal.starts_with('"') => return DerivedBase::Untracked,
        ["&", name] if is_identifier(name) && !matches!(*name, "NULL" | "nullptr") => {
            return DerivedBase::Untracked;
        }
        _ => {}
    }
    if let [place @ .., "+" | "-", offset] = argument
        && (is_identifier(offset) || offset.bytes().all(|byte| byte.is_ascii_digit()))
        && !matches!(place.last(), Some(&("+" | "-")))
        && let Some(place) = canonical_place(place)
    {
        return DerivedBase::Place {
            place,
            offset: true,
        };
    }
    DerivedBase::Unmodeled
}

/// Card **SLOT-MOVE-AND-INTERIOR-BORROW** (gateway rule 12y): the place a
/// borrowed result's base actual spelled `( T * ) place` designates. A
/// conversion of a pointer to another object pointer type keeps its address
/// (C11 6.3.2.3p7), so the parent the result is borrowed from is the place's
/// object. The type group is read structurally: identifiers and keywords, at
/// least one `*`, the last token a `*`; anything else is not read. Used only
/// for a `Borrowed` extent (`legacy_classify.rs`); every other derived result
/// keeps [`derived_base`]'s reading (`DERIVED-BASE-CAST-READING`).
pub(super) fn cast_place(argument: &[&str]) -> Option<String> {
    let argument = strip_parentheses(argument);
    if argument.first() != Some(&"(") {
        return None;
    }
    let close = argument.iter().position(|token| *token == ")")?;
    let group = &argument[1..close];
    if group.last() != Some(&"*")
        || !group
            .iter()
            .all(|token| *token == "*" || is_identifier(token))
        || !group.first().is_some_and(|token| is_identifier(token))
    {
        return None;
    }
    canonical_place(strip_parentheses(&argument[close + 1..]))
}

/// Card **W43-RESULT-ALIAS-RETURNED**: `contract` with a derived result read at
/// the interior extent. Used for a base spelled `place ± offset`, whose value
/// is inside the place's object but is not the place's own value.
pub(super) fn interior_extent(mut contract: ResourceFunctionContract) -> ResourceFunctionContract {
    // Card SLOT-MOVE-AND-INTERIOR-BORROW (rule 12y): a borrowed result stays
    // borrowed; an offset changes only an exact alias.
    if let ResourceReturnMode::Derived { extent, .. } = &mut contract.result
        && *extent == DerivedExtent::Exact
    {
        *extent = DerivedExtent::Interior;
    }
    contract
}

/// The place an assignment's left-hand side `tokens` writes, when this
/// importer can name it.
///
/// Card **W44-ASSIGNMENT-DESTINATION-SUFFIX-PLACE**: a left-hand side not
/// headed by `*` is named only when its first place-spelling suffix
/// ([`first_suffix_place`]) **is** the whole left-hand side, or when the tokens
/// before that suffix are a declaration's specifiers and declarator stars
/// ([`is_declarator_prefix`]). Before the card the suffix was named wherever it
/// started, so `recs [ i ] . name` was named `name`: the analyzer bound the
/// stored identity to the local `name`, a different object, and certified a
/// body that leaks one object and frees another. Such a left-hand side is a
/// store into a sub-object; [`names_a_sub_object`] recognises it and the
/// lexical indirect-access gate refuses it. Every left-hand side named before
/// and still named is named identically
/// (`QCore.AssignmentDestinationSuffix.the_repair_names_nothing_new`).
pub(super) fn direct_assignment_destination(tokens: &[&str]) -> Option<String> {
    if tokens.first() == Some(&"*") {
        // A single dereference of a plain identifier is the only indirect
        // destination this importer names: `*out = …` becomes the place
        // `*out`. It is *not* modeled here — [`transfer`] rejects it unless the
        // analysis in force sanctions `*out` as an out-parameter slot — but
        // naming it is what lets an out-parameter contract describe the store.
        // Anything deeper (`*p->field`, `**p`, `*(p + 1)`) stays unnamed.
        return match tokens {
            ["*", name] if is_identifier(name) && !matches!(*name, "NULL" | "nullptr") => {
                Some(format!("*{name}"))
            }
            _ => None,
        };
    }
    let (start, place) = first_suffix_place(tokens)?;
    (start == 0 || is_declarator_prefix(&tokens[..start])).then_some(place)
}

/// Card **W44-ASSIGNMENT-DESTINATION-SUFFIX-PLACE**: whether a left-hand side
/// ends with a place-spelling suffix that [`direct_assignment_destination`]
/// does not name — a store into a sub-object (`recs [ i ] . name`,
/// `( * p ) - > name`, `f ( ) - > name`) whose last member name the reader
/// used to name. Such a store is refused by the lexical indirect-access gate
/// exactly as a named `a.b`, `a[0]` or `p->f` destination is.
pub(super) fn names_a_sub_object(tokens: &[&str]) -> bool {
    tokens.first() != Some(&"*")
        && first_suffix_place(tokens).is_some()
        && direct_assignment_destination(tokens).is_none()
}

/// Card **W44-ASSIGNMENT-DESTINATION-SUFFIX-PLACE**: the first suffix of a
/// left-hand side that spells a place, with the index it starts at.
///
/// This is the reader [`direct_assignment_destination`] has always used for a
/// left-hand side not headed by `*`. A suffix that starts after index 0 is the
/// declared object of a declaration (`struct T * p`) **or** the last member
/// name of a store into a sub-object (`recs [ i ] . name`), and only
/// [`is_declarator_prefix`] tells the two apart.
pub(crate) fn first_suffix_place(tokens: &[&str]) -> Option<(usize, String)> {
    (0..tokens.len())
        .find_map(|start| canonical_place(&tokens[start..]).map(|place| (start, place)))
}

/// Card **W44-ASSIGNMENT-DESTINATION-SUFFIX-PLACE**: whether `prefix`, the
/// tokens of a left-hand side before the place it ends with, is a
/// declaration's specifiers and declarator stars — so that the place is the
/// object the statement declares and initializes.
///
/// Positive and fail-closed: a nonempty run of identifiers (type names,
/// keywords, qualifiers, storage classes, a comma operand), `*` and `,` only.
/// Every token that could select, subscript, call or dereference into another
/// object (`.`, `-` `>`, `[` `]`, `(` `)`) is outside the set, so a prefix
/// carrying one names a sub-object and never the suffix it ends with.
pub(crate) fn is_declarator_prefix(prefix: &[&str]) -> bool {
    !prefix.is_empty()
        && prefix
            .iter()
            .all(|text| is_identifier(text) || matches!(*text, "*" | ","))
}

/// Whether `place` denotes a store through a pointer rather than a variable.
/// Such a place is modeled only when the contract in force declares it an
/// out-parameter slot; see [`EntryCustody::out_slots`].
pub(super) fn is_indirect_place(place: &str) -> bool {
    place.starts_with('*')
}

pub(super) fn canonical_place(tokens: &[&str]) -> Option<String> {
    let tokens = strip_parentheses(tokens);
    let first = tokens.first().copied()?;
    if !is_identifier(first) || matches!(first, "NULL" | "nullptr") {
        return None;
    }
    let mut place = first.to_owned();
    let mut cursor = 1;
    while cursor < tokens.len() {
        match tokens[cursor] {
            "." if tokens
                .get(cursor + 1)
                .is_some_and(|field| is_identifier(field)) =>
            {
                place.push('.');
                place.push_str(tokens[cursor + 1]);
                cursor += 2;
            }
            "-" if tokens.get(cursor + 1) == Some(&">")
                && tokens
                    .get(cursor + 2)
                    .is_some_and(|field| is_identifier(field)) =>
            {
                place.push_str("->");
                place.push_str(tokens[cursor + 2]);
                cursor += 3;
            }
            "[" if tokens
                .get(cursor + 1)
                .is_some_and(|index| index.bytes().all(|byte| byte.is_ascii_digit()))
                && tokens.get(cursor + 2) == Some(&"]") =>
            {
                place.push('[');
                place.push_str(tokens[cursor + 1]);
                place.push(']');
                cursor += 3;
            }
            _ => return None,
        }
    }
    Some(place)
}

/// Whether a token could be spelled as an identifier. Reserved words are
/// *included*: this is a lexical shape test, and `find_call` relies on it to
/// treat `sizeof (` as call-shaped. [`crate::neutral_position`] uses the same
/// test so that the occurrences it must account for are exactly the ones
/// [`contains_call`] refuses on.
pub(crate) fn is_identifier(text: &str) -> bool {
    text.as_bytes()
        .first()
        .is_some_and(|byte| byte.is_ascii_alphabetic() || *byte == b'_')
}
