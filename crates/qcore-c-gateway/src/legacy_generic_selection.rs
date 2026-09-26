//! C11 generic selections (card **W44-C11-GENERIC-SELECTION**).
//!
//! # What C says
//!
//! `_Generic (controlling-expression, T1: e1, …, default: ed)` is a primary
//! expression (C11 §6.5.1.1). Its controlling expression is **not evaluated**:
//! only its type, after lvalue conversion, is read, and it selects exactly one
//! association at translation time. The result is the selected association's
//! expression, and only that expression is evaluated.
//!
//! # Where the corpus meets it
//!
//! glibc 2.43 defines, under `__GLIBC_USE (ISOC23)` (which `_GNU_SOURCE`
//! enables) and C11 or later, a const-preserving macro for `memchr`, `strchr`,
//! `strrchr`, `strpbrk`, `strstr`, `bsearch` and five wide-character searches
//! (`sys/cdefs.h`, `__glibc_const_generic`):
//!
//! ```text
//! _Generic (0 ? (PTR) : (void *) 1, const void *: (CTYPE) (CALL), default: CALL)
//! ```
//!
//! `CALL` is the function call itself (the macro's own name is not re-expanded
//! inside its replacement list). Before this card every such selection reached
//! the classifier as a call to an uncontracted function `_Generic`, or as an
//! unmodeled call in the condition, return or argument it sat in.
//!
//! # The route: selection invariance
//!
//! The analyzer has no C type checker, so it cannot tell which association the
//! compiler selects. It does not need to when every association is **the same
//! expression up to value-preserving pointer casts**: whichever association is
//! selected, the effects evaluated are that expression's, and the value is the
//! same pointer. [`resolve_in_body`] therefore lowers such a selection to that
//! common expression — for glibc's shape, exactly the text a pre-C23 header
//! produced — and drops the controlling expression, which C does not evaluate.
//! The ordinary classifier then reads the call and applies its declared
//! contract (a derived `strchr` result stays a derived result, with no cast to
//! carry it through).
//!
//! A selection is admitted only when all of the following hold, and is left
//! **byte-for-byte as it was** otherwise, so it keeps whatever refusal it had:
//!
//! 1. it is `_Generic ( control , association {, association} )` with a
//!    nonempty controlling expression, every association `key : expression`
//!    with both halves nonempty, and at most one `default`;
//! 2. after removing enclosing parentheses and leading casts, every
//!    association's expression is token-for-token the same core. A cast is
//!    removed only when its type name is a **pointer** type name (identifiers
//!    followed by `*`s and pointer qualifiers, nothing else) and its operand is
//!    a whole parenthesised group, a direct call or a single name, so the cast
//!    applies to the whole rest of the association and not to a prefix of it
//!    (`(T *) a + b` is never read as a cast of `a + b`);
//! 3. if any cast was removed, the core is a direct call `NAME ( … )` and this
//!    unit's own C declaration of `NAME` reads, unambiguously, as returning an
//!    object pointer ([`declared_abi::declares_object_pointer_result`]), so
//!    every removed cast converted a pointer to a pointer.
//!
//! Selections nested in an association are resolved first; a controlling
//! expression is never rewritten, since it is dropped whole. A refused outer
//! selection is emitted unchanged, inner selections included.
//!
//! # Why the lowering is sound
//!
//! Evaluating the selected association evaluates its core exactly once (a cast
//! and a parenthesis evaluate nothing) and yields the core's value converted by
//! the removed casts. Each removed cast converts an object pointer to an object
//! pointer, which leaves the designated object unchanged (C11 §6.3.2.3p2, p7)
//! under the no-undefined-behaviour object model the gateway already assumes
//! (`AOM-POISON-NO-UB`). So every association has the core's effects and the
//! core's value, whichever the compiler selects. `QCore.GenericSelection`
//! proves this of its model: `an_admitted_association_evaluates_as_the_core`,
//! `the_lowered_effect_is_independent_of_the_selection` and, for the rewrite as
//! a whole, `resolve_preserves_outcomes`.
//!
//! The equality in 2 is equality of the analyzer's own token view. That view
//! collapses every string and character literal to one token, so two cores
//! that differ only in a literal compare equal; the analyzer lowers both
//! identically, because it reads nothing else, and its lowering of either
//! covers the concrete one selected. This is the tokenizer premise of trust
//! boundary 28, already stated for `QCore.LiteralTailActual`.
//!
//! # What it does not do
//!
//! It does not reproduce the compiler's selection (card route 2, reading
//! Clang's `GenericSelectionExpr`, was not built: the ABI and mode readers are
//! kept off the typed AST so that W4 derivation, W4 validation and certificate
//! rechecking reproduce every decision from the preprocessed tokens, and route 1
//! already admits every corpus occurrence). It does not widen what the
//! classifier admits: the lowered core meets exactly the rules it would meet if
//! the source had spelled it.

use std::collections::BTreeMap;

use super::{Token, declared_abi, declared_modes, find_function_body, is_identifier, matching};

/// The C11 keyword.
pub(crate) const GENERIC_KEYWORD: &str = "_Generic";

/// Qualifiers that may follow a `*` in a pointer type name.
const POINTER_QUALIFIERS: &[&str] = &[
    "const",
    "volatile",
    "restrict",
    "__restrict",
    "__restrict__",
];

/// Why a selection was left as it was.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Refusal {
    /// Not `_Generic ( control , key : expression … )` as this reader reads it.
    Malformed,
    /// More than one `default` association.
    TwoDefaults,
    /// Two associations differ after the removals.
    NotSelectionInvariant,
    /// A cast was removed but the core is not a direct call.
    CastOperandNotACall,
    /// A cast was removed but this unit does not declare the callee with an
    /// object-pointer result.
    CalleeResultNotADeclaredPointer,
}

/// Lower every admitted generic selection in `function`'s body.
///
/// `None` means the stream is unchanged: the body has no `_Generic`, or every
/// selection in it was refused. Tokens outside the body are never touched.
pub(crate) fn resolve_in_body(tokens: &[Token], function: &str) -> Option<Vec<Token>> {
    let (start, _) = find_function_body(tokens, function)?;
    let end = matching(tokens, start, "{", "}")?;
    if !tokens[start..end]
        .iter()
        .any(|token| token.text == GENERIC_KEYWORD)
    {
        return None;
    }
    let texts = tokens
        .iter()
        .map(|token| token.text.as_str())
        .collect::<Vec<_>>();
    let mut declarations = PointerResults {
        texts: &texts,
        read: BTreeMap::new(),
    };
    let (body, changed) = resolve_span(&tokens[start + 1..end], &mut declarations);
    if !changed {
        return None;
    }
    let mut resolved = Vec::with_capacity(tokens.len());
    resolved.extend_from_slice(&tokens[..=start]);
    resolved.extend(body);
    resolved.extend_from_slice(&tokens[end..]);
    Some(resolved)
}

/// Whether a callee is declared, in this unit, with an object-pointer result.
/// The answer is read once per name.
struct PointerResults<'a> {
    texts: &'a [&'a str],
    read: BTreeMap<String, bool>,
}

impl PointerResults<'_> {
    fn declared(&mut self, callee: &str) -> bool {
        // A withdrawn or premise-restricted callee arrives renamed; its C
        // declaration is the original name's. The call then refuses with the
        // withdrawal's own wording, exactly as a spelled call would.
        let name = declared_abi::withdrawn_callee(callee)
            .map(|(_, name)| name)
            .or_else(|| declared_modes::withdrawn_callee(callee).map(|(_, name)| name))
            .or_else(|| super::literal_premise::unmet_callee(callee))
            .or_else(|| super::review_hold::held_callee(callee))
            .or_else(|| super::pin_hold::held_callee(callee))
            .unwrap_or(callee);
        if let Some(answer) = self.read.get(name) {
            return *answer;
        }
        let answer = declared_abi::declares_object_pointer_result(self.texts, name);
        self.read.insert(name.to_owned(), answer);
        answer
    }
}

/// Resolve every selection in `span`, left to right; an admitted one is
/// replaced by its core, a refused one is copied unchanged.
fn resolve_span(span: &[Token], declarations: &mut PointerResults<'_>) -> (Vec<Token>, bool) {
    let mut out = Vec::with_capacity(span.len());
    let mut changed = false;
    let mut index = 0;
    while index < span.len() {
        if span[index].text == GENERIC_KEYWORD
            && let Some(close) = selection_close(span, index)
        {
            match read_selection(span, index, close, declarations) {
                Ok(core) => {
                    out.extend(core);
                    changed = true;
                }
                Err(_) => out.extend_from_slice(&span[index..=close]),
            }
            index = close + 1;
            continue;
        }
        out.push(span[index].clone());
        index += 1;
    }
    (out, changed)
}

/// The index of the `)` closing the selection whose keyword is at `at`.
fn selection_close(span: &[Token], at: usize) -> Option<usize> {
    if span.get(at + 1)?.text != "(" {
        return None;
    }
    matching(span, at + 1, "(", ")")
}

/// Read one selection spanning `at..=close` and return its lowered core.
fn read_selection(
    span: &[Token],
    at: usize,
    close: usize,
    declarations: &mut PointerResults<'_>,
) -> Result<Vec<Token>, Refusal> {
    let parts = split_top_level(&span[at + 2..close], ",");
    if parts.len() < 2 || parts[0].is_empty() {
        return Err(Refusal::Malformed);
    }
    let mut defaults = 0usize;
    let mut cores = Vec::with_capacity(parts.len() - 1);
    let mut chosen = 0usize;
    for association in &parts[1..] {
        let colon = first_top_level(association, ":").ok_or(Refusal::Malformed)?;
        let (key, expression) = (&association[..colon], &association[colon + 1..]);
        if key.is_empty() || expression.is_empty() {
            return Err(Refusal::Malformed);
        }
        if key.len() == 1 && key[0].text == "default" {
            defaults += 1;
            chosen = cores.len();
        }
        // Innermost first: a selection nested in this association is lowered
        // before the associations are compared.
        let (resolved, _) = resolve_span(expression, declarations);
        let (start, end, cast) = strip(&resolved);
        cores.push((resolved[start..end].to_vec(), cast));
    }
    if defaults > 1 {
        return Err(Refusal::TwoDefaults);
    }
    let same = |one: &[Token], other: &[Token]| {
        one.len() == other.len() && one.iter().zip(other).all(|(a, b)| a.text == b.text)
    };
    let first = &cores[0].0;
    if cores.iter().any(|(core, _)| !same(core, first)) {
        return Err(Refusal::NotSelectionInvariant);
    }
    if cores.iter().any(|(_, cast)| *cast) {
        let texts = first.iter().map(|t| t.text.as_str()).collect::<Vec<_>>();
        if !is_direct_call(&texts) {
            return Err(Refusal::CastOperandNotACall);
        }
        if !declarations.declared(texts[0]) {
            return Err(Refusal::CalleeResultNotADeclaredPointer);
        }
    }
    // The default association's spelling (glibc's is the bare call).
    let core = cores.swap_remove(chosen).0;
    let texts = core.iter().map(|t| t.text.as_str()).collect::<Vec<_>>();
    if is_direct_call(&texts) || (texts.len() == 1 && is_identifier(texts[0])) {
        return Ok(core);
    }
    // Anything else keeps the selection's primary-expression precedence.
    let line = span[at].line;
    let mut parenthesised = Vec::with_capacity(core.len() + 2);
    parenthesised.push(Token {
        text: "(".to_owned(),
        line,
    });
    parenthesised.extend(core);
    parenthesised.push(Token {
        text: ")".to_owned(),
        line,
    });
    Ok(parenthesised)
}

/// Remove enclosing parentheses and leading pointer casts. Returns the core's
/// half-open range and whether a cast was removed.
fn strip(tokens: &[Token]) -> (usize, usize, bool) {
    let texts = tokens.iter().map(|t| t.text.as_str()).collect::<Vec<_>>();
    let (mut start, mut end, mut cast) = (0usize, texts.len(), false);
    loop {
        let view = &texts[start..end];
        if wraps(view) {
            start += 1;
            end -= 1;
            continue;
        }
        if view.first() == Some(&"(")
            && let Some(close) = close_of(view, 0)
            && is_pointer_type_name(&view[1..close])
            && is_closed_operand(&view[close + 1..])
        {
            start += close + 1;
            cast = true;
            continue;
        }
        return (start, end, cast);
    }
}

/// `( … )` whose parentheses enclose the whole of `texts`.
fn wraps(texts: &[&str]) -> bool {
    texts.len() >= 2 && texts[0] == "(" && close_of(texts, 0) == Some(texts.len() - 1)
}

/// The `)` matching the `(` at `open`.
fn close_of(texts: &[&str], open: usize) -> Option<usize> {
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

/// `NAME ( … )` spanning the whole of `texts`.
fn is_direct_call(texts: &[&str]) -> bool {
    texts.len() >= 3
        && is_identifier(texts[0])
        && texts[0] != GENERIC_KEYWORD
        && texts[1] == "("
        && close_of(texts, 1) == Some(texts.len() - 1)
}

/// An operand a leading cast applies to as a whole.
fn is_closed_operand(texts: &[&str]) -> bool {
    wraps(texts) || is_direct_call(texts) || (texts.len() == 1 && is_identifier(texts[0]))
}

/// Identifiers (specifiers, qualifiers, a tag or a typedef name), then at
/// least one `*`, each optionally followed by pointer qualifiers. Nothing
/// else: no parenthesis (a function pointer), no bracket (an array), no
/// number. The trailing `*` also separates it from a parenthesised
/// expression, which cannot end in a binary operator.
fn is_pointer_type_name(texts: &[&str]) -> bool {
    let Some(first_star) = texts.iter().position(|text| *text == "*") else {
        return false;
    };
    first_star > 0
        && texts[..first_star].iter().all(|text| {
            is_identifier(text) && text.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
        })
        && texts[first_star..]
            .iter()
            .all(|text| *text == "*" || POINTER_QUALIFIERS.contains(text))
}

/// Split `tokens` at every top-level `separator`.
fn split_top_level<'a>(tokens: &'a [Token], separator: &str) -> Vec<&'a [Token]> {
    let mut parts = Vec::new();
    let mut depth = 0usize;
    let mut start = 0;
    for (index, token) in tokens.iter().enumerate() {
        match token.text.as_str() {
            "(" | "[" | "{" => depth += 1,
            ")" | "]" | "}" => depth = depth.saturating_sub(1),
            text if text == separator && depth == 0 => {
                parts.push(&tokens[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    parts.push(&tokens[start..]);
    parts
}

/// The first top-level `separator` in `tokens`.
fn first_top_level(tokens: &[Token], separator: &str) -> Option<usize> {
    let mut depth = 0usize;
    for (index, token) in tokens.iter().enumerate() {
        match token.text.as_str() {
            "(" | "[" | "{" => depth += 1,
            ")" | "]" | "}" => depth = depth.saturating_sub(1),
            text if text == separator && depth == 0 => return Some(index),
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::super::tokenize;
    use super::*;

    const DECLARATIONS: &str = "char *strchr (const char *s, int c); \
                                unsigned long strlen (const char *s); ";

    /// The body of `w` after resolution, as one spaced string.
    fn resolved(body: &str) -> String {
        let source = format!("{DECLARATIONS} void w (const char *s, char *m) {{ {body} }}");
        let tokens = tokenize(&source);
        let out = resolve_in_body(&tokens, "w").unwrap_or(tokens);
        let (start, _) = find_function_body(&out, "w").expect("body");
        let end = matching(&out, start, "{", "}").expect("close");
        out[start + 1..end]
            .iter()
            .map(|t| t.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }

    const GLIBC: &str = "_Generic (0 ? (s) : (void *) 1, const void *: (const char *) \
                         (strchr (s, 'x')), default: strchr (s, 'x'))";

    #[test]
    fn the_glibc_shape_lowers_to_the_call() {
        assert_eq!(
            resolved(&format!("char *p = {GLIBC};")),
            "char * p = strchr ( s , __qcore_literal__ ) ;"
        );
    }

    #[test]
    fn a_nested_selection_lowers_innermost_first() {
        let nested =
            format!("_Generic (m, const void *: (const char *) ({GLIBC}), default: {GLIBC})");
        assert_eq!(
            resolved(&format!("return {nested};")),
            "return strchr ( s , __qcore_literal__ ) ;"
        );
    }

    #[test]
    fn differing_associations_are_left_as_they_were() {
        let body = "p = _Generic (s, const char *: strchr (s, 1), default: strchr (m, 1));";
        assert_eq!(
            resolved(body),
            "p = _Generic ( s , const char * : strchr ( s , 1 ) , default : strchr ( m , 1 ) ) ;"
        );
    }

    #[test]
    fn a_cast_of_an_integer_result_is_refused() {
        let body = "_Generic (s, const void *: (const char *) (strlen (s)), default: strlen (s));";
        assert!(resolved(body).starts_with("_Generic"));
    }

    #[test]
    fn a_cast_of_a_prefix_is_not_a_cast_of_the_whole() {
        // `(char *) strchr (s, 1) + 1` casts the call, then adds.
        let body =
            "_Generic (s, const void *: (char *) strchr (s, 1) + 1, default: strchr (s, 1) + 1);";
        assert!(resolved(body).starts_with("_Generic"));
    }

    #[test]
    fn a_non_pointer_cast_is_not_removed() {
        let body = "_Generic (s, const void *: (long) (strchr (s, 1)), default: strchr (s, 1));";
        assert!(resolved(body).starts_with("_Generic"));
    }

    #[test]
    fn two_defaults_and_no_association_refuse() {
        assert!(resolved("_Generic (s, default: m, default: m);").starts_with("_Generic"));
        assert!(resolved("_Generic (s);").starts_with("_Generic"));
    }

    #[test]
    fn identical_associations_need_no_declaration() {
        assert_eq!(
            resolved("x = _Generic (s++, const char *: m [ 0 ], default: m [ 0 ]);"),
            "x = ( m [ 0 ] ) ;"
        );
    }

    #[test]
    fn the_controlling_expression_is_dropped() {
        let body = "p = _Generic (strchr (m, 2), const void *: (const char *) (strchr (s, 1)), \
                    default: strchr (s, 1));";
        assert_eq!(resolved(body), "p = strchr ( s , 1 ) ;");
    }
}
