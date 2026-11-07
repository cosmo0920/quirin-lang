//! Card **CHAINED-ASSIGNMENT-INNER-DESTINATION** (gateway rule 5i, amended):
//! an assignment whose assigned value writes another object is never admitted
//! as the one write of its outer destination.
//!
//! `a = b = E` assigns `E` to `b` and then that value, converted, to `a`
//! (C11 6.5.16p3): both names hold the same value afterwards. The statement
//! classifier models exactly one write per assignment statement — the place
//! [`direct_assignment_destination`] names on the left of the statement's
//! first `=` — and read `d = safe = rd_malloc(n)` as `d = rd_malloc(n)`,
//! dropping the write of `safe` without modelling or refusing it. In
//! `rd_kafka_sasl_safe_string` (`rdkafka_sasl_scram.c:243`, `return safe;` at
//! `:249`) that is a false leak; a release through the inner name was left
//! unmatched the same way.
//!
//! The guard below runs on [`Parser::classify_simple`]'s answer, so every
//! caller of the classifier (statements, the return and condition probes, the
//! elaborator's own pieces) sees it. It replaces an **admission** of a
//! statement holding a write inside an assignment's value by a named refusal
//! (restoring the probe state, so no site number moves) and leaves every
//! refusal byte for byte. It never admits anything. Rule 10s's elaborator, asked
//! after every older route refused, then lowers a chain as its two sequential
//! assignments — the inner assignment `b = E` and then `a = b`, each decided by
//! the older statement lowering exactly as a single assignment is — and keeps
//! this refusal when either piece is a destination the route refuses alone.

use super::*;

/// The refusal of a chain: the outer destination's admission would drop the
/// inner write.
pub(crate) const CHAINED_ASSIGNMENT_REFUSAL: &str = "chained assignment writes an inner destination \
     too; it is read only as its sequential assignments (C11 6.5.16p3)";

/// The refusal of an assignment nested in another operand of an assigned
/// value (`a = f(b = E)`, `a = (b = E) + 1`), which no route sequences.
pub(crate) const NESTED_ASSIGNMENT_REFUSAL: &str =
    "assignment nested in an operand of an assigned value is not modeled";

/// Whether `texts[index]` is a simple or compound assignment operator in the
/// importer's one-character punctuation spelling: an `=` that is neither half
/// of `==` nor the second character of `!=`, `<=` or `>=` (a `<<=` or `>>=`
/// is an assignment).
pub(crate) fn is_assignment_operator(texts: &[&str], index: usize) -> bool {
    if texts.get(index) != Some(&"=") {
        return false;
    }
    let previous = index.checked_sub(1).and_then(|i| texts.get(i)).copied();
    let next = texts.get(index + 1).copied();
    next != Some("=")
        && (!matches!(previous, Some("=" | "!" | "<" | ">"))
            || (matches!(previous, Some("<" | ">"))
                && index.checked_sub(2).and_then(|i| texts.get(i)).copied() == previous))
}

/// The shape of a write found inside an assignment's value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InnerWrite {
    /// `a = b = E` or `a = (b = E)`: the value is (parenthesised) exactly an
    /// assignment.
    Chain,
    /// Any other assignment inside the value (`a = f(b = E)`).
    Nested,
}

/// The first write inside an assignment's value in `texts`: per top-level
/// comma segment (a declaration's declarators, a comma expression's
/// operands), the segment's first assignment at parenthesis depth 0 is its
/// outer assignment, and an assignment operator anywhere after it in the same
/// segment is a write inside its value. Returns `(outer, inner, shape)`.
pub(crate) fn inner_write(texts: &[&str]) -> Option<(usize, usize, InnerWrite)> {
    let mut depth = 0usize;
    let mut outer: Option<usize> = None;
    for index in 0..texts.len() {
        match texts[index] {
            "(" | "[" => depth += 1,
            ")" | "]" => depth = depth.saturating_sub(1),
            "," if depth == 0 => outer = None,
            _ if is_assignment_operator(texts, index) => match outer {
                None if depth == 0 => outer = Some(index),
                None => {}
                Some(at) => {
                    let shape = if is_chain(texts, at, index) {
                        InnerWrite::Chain
                    } else {
                        InnerWrite::Nested
                    };
                    return Some((at, index, shape));
                }
            },
            _ => {}
        }
    }
    None
}

/// Whether the value of the assignment at `outer` (to the end of its comma
/// segment), with whole wrapping parentheses removed, starts with the left
/// operand of the assignment at `inner`: `a = b = E` and `a = (b = E)`, but
/// not `a = (b = E) + 1`.
fn is_chain(texts: &[&str], outer: usize, inner: usize) -> bool {
    let mut depth = 0usize;
    let mut end = texts.len();
    for (index, text) in texts.iter().enumerate().skip(outer + 1) {
        match *text {
            "(" | "[" => depth += 1,
            ")" | "]" => depth = depth.saturating_sub(1),
            "," if depth == 0 => {
                end = index;
                break;
            }
            _ => {}
        }
    }
    let value = &texts[outer + 1..end];
    let stripped = strip_parentheses(value);
    if stripped.is_empty() {
        return false;
    }
    // `strip_parentheses` removes the same number of tokens from both ends.
    let start = outer + 1 + (value.len() - stripped.len()) / 2;
    inner > start && inner < start + stripped.len() && left_operand_of_chain(&texts[start..inner])
}

/// Whether `rest`, the tokens from the start of the stripped value to the
/// inner assignment operator, is exactly a left operand: a place spelled by
/// names, `*`, `.`, `->`, bracketed indices and grouping parentheses, with no
/// call and no operator that would make the inner assignment an operand of
/// something else.
fn left_operand_of_chain(rest: &[&str]) -> bool {
    // A compound operator's first character (`+` of `+=`) belongs to the
    // operator.
    let rest = match rest.split_last() {
        Some((last, head))
            if matches!(*last, "+" | "-" | "*" | "/" | "%" | "&" | "|" | "^")
                || (matches!(*last, "<" | ">") && head.last() == Some(last)) =>
        {
            if matches!(*last, "<" | ">") {
                &head[..head.len() - 1]
            } else {
                head
            }
        }
        _ => rest,
    };
    if rest.is_empty() {
        return false;
    }
    let mut depth = 0usize;
    let mut groups = 0usize;
    for (index, text) in rest.iter().enumerate() {
        match *text {
            "[" => depth += 1,
            "]" => {
                if depth == 0 {
                    return false;
                }
                depth -= 1;
            }
            _ if depth > 0 => {}
            "*" | "." => {}
            "-" if rest.get(index + 1) == Some(&">") => {}
            ">" if index > 0 && rest[index - 1] == "-" => {}
            // A group after a name or a closing group applies a call.
            "(" if index > 0 && (is_identifier(rest[index - 1]) || rest[index - 1] == ")") => {
                return false;
            }
            "(" => groups += 1,
            // A group the left operand does not close (`(b = E) + 1`) makes
            // the inner assignment an operand.
            ")" => {
                if groups == 0 {
                    return false;
                }
                groups -= 1;
            }
            name if is_identifier(name) => {}
            _ => return false,
        }
    }
    depth == 0 && groups == 0
}

impl Parser<'_> {
    /// Replace an admission of a statement holding a write inside an
    /// assignment's value by the named refusal; a refusal stays itself.
    pub(super) fn chained_assignment_guard(
        &mut self,
        tokens: &[Token],
        instruction: LegacyInstruction,
        saved: &probe_state::ParserProbeState,
    ) -> LegacyInstruction {
        if matches!(instruction, LegacyInstruction::Unknown { .. }) {
            return instruction;
        }
        let texts = tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        let Some((_, _, shape)) = inner_write(&texts) else {
            return instruction;
        };
        self.restore_probe_state(saved);
        LegacyInstruction::Unknown {
            description: match shape {
                InnerWrite::Chain => CHAINED_ASSIGNMENT_REFUSAL,
                InnerWrite::Nested => NESTED_ASSIGNMENT_REFUSAL,
            }
            .to_owned(),
            writes: self.refused_writes(&texts),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(source: &str) -> Option<(usize, usize, InnerWrite)> {
        inner_write(&source.split_whitespace().collect::<Vec<_>>())
    }

    #[test]
    fn chains_are_found_in_every_spelling() {
        for source in [
            "d = safe = rd_malloc ( len + 1 )",
            "d = ( safe = rd_malloc ( len + 1 ) )",
            "a = b = c = q_alloc ( )",
            "x = y + = k",
            "x = y < < = 2",
            "o - > p = s = q_alloc ( )",
            "s = o - > p = q_alloc ( )",
            "* out = s = q_alloc ( )",
            "a [ i ] = b = 0",
            "char * d = s = q_alloc ( )",
            "int a = 1 , b = c = 2",
        ] {
            assert_eq!(
                read(source).map(|r| r.2),
                Some(InnerWrite::Chain),
                "{source}"
            );
        }
    }

    #[test]
    fn a_write_in_another_operand_is_nested() {
        for source in [
            "d = q_wrap ( s = q_alloc ( ) )",
            "x = ( y = a ) + 1",
            "x = c ? ( y = 1 ) : 2",
        ] {
            assert_eq!(
                read(source).map(|r| r.2),
                Some(InnerWrite::Nested),
                "{source}"
            );
        }
    }

    #[test]
    fn comparisons_declarators_and_single_writes_are_not_inner_writes() {
        for source in [
            "d = q_alloc ( )",
            "x = a = = b",
            "x = a ! = b",
            "x = a < = b",
            "x = a > = b",
            "int a = 1 , b = 2",
            "x = 1 , y = 2",
            "q_free ( p )",
            "f ( a = b )",
            "x + = 1",
        ] {
            assert_eq!(read(source), None, "{source}");
        }
    }
}
