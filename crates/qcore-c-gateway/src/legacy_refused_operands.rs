//! Card **W44-LEAK-PIN-AUDIT**: the refused-escape rule.
//!
//! # The mechanism this module repairs
//!
//! A statement the projection refuses reports `Unsupported` and poisons the
//! pointer **bindings** it may write (card AOM-BINDING-POISON), but it never
//! touches a resource **status**. When the refused statement is a store or a
//! return whose *value* is a tracked identity — `map->parent = (void *) counter;`,
//! `*bufferp = buf;`, `arr[cnt++] = val;`, `otel_span->links = otel_links;`,
//! `conn = (struct mk_sched_conn *) data;`, `return (const char **) arr;` — the
//! identity has left the names the projection follows, by the statement's own
//! syntax, yet it stays `LIVE` and `report_leaks` accuses it at the next exit.
//! The card's audit of the 271 baseline leak leaves measured that shape as a
//! class (b) false-leak mechanism: an escape the analyzer mis-models
//! (`docs/reports/w44-leak-pin-audit/README.md`).
//!
//! The projection already answers the modelled half of the situation. A
//! contracted call whose actual it cannot resolve marks the actual's identities
//! through `mark_unresolved_effect`, whose documented reading is "custody
//! retained after a rejected effect; its residual is not a proven leak because
//! the unmodeled action may have discharged it", and card CTWIN-RESIDUALS marks
//! the value operands of a refused continued-access publication the same way.
//! This module applies that reading to every refused statement whose value side
//! hands on an identity.
//!
//! # What is deliberately not withdrawn
//!
//! An identity handed to an **uncontracted callee** — `destroy_metric(metric)`,
//! `mk_list_init(listeners)`, `cfl_list_add(&sample->_head, …)` — is not
//! withdrawn here. Whether that call releases, retains or merely reads its
//! argument is a fact about the callee, which the projection does not have; the
//! accusation there rests on a premise about that callee, named per callee in
//! the card's report (`REFUSED-CALL-ARGUMENT-PREMISE`), and it is discharged by
//! a contract or a validated summary for the callee, never by this rule. A
//! statement's syntax says nothing about what a callee does; it does say that
//! a stored or returned value has left the projection.
//!
//! # Direction
//!
//! The rule only ever **adds** sites to `unresolved_effect_sites`. That set is
//! joined by union and only grows, so the work list still terminates, and a
//! site in it is never reported leaked, never counted as a settled residual
//! (`site_disposition` answers `Mixed`, the exit `Unknown`) and never accused
//! by the settled violation passes that already skip it. It can therefore only
//! **withdraw** an accusation, never make one and never discharge an
//! obligation. It fires only at a node whose own transfer reported
//! `Unsupported`, so the function carrying a withdrawn leaf keeps that refusal
//! and cannot become `Safe` through this rule. Any imprecision of the reader
//! below therefore costs only precision: a hand-on it misses leaves the pre-card
//! accusation in place.
//!
//! # The reader
//!
//! A place is handed on by a statement's **value side** — the operand of a
//! `return`, or the right-hand side of a top-level simple assignment — outside
//! every call's argument list, where the statement reads it as a value or takes
//! the address of it or of storage reached through it:
//!
//! * `x = p`, `x->f = (void *) p`, `*out = p`, `return p` hand on `p`;
//!   `x->h = &p->head` hands on `p` and `p->head`;
//! * `x = p->buf` hands on the place `p->buf`, not `p`; `x = p->len + 1`
//!   hands on nothing the projection binds; `x = p->items[i]` and
//!   `x = p->fn(…)` read through `p` and hand on nothing of `p`'s;
//! * `x = f(p)` passes `p` to a callee (the case above), and `sizeof (p)` does
//!   not evaluate `p`.
//!
//! Places are spelled as `canonical_place` spells them (`p`, `p->f`, `p.f`,
//! `p[0]`, `*p`), and a bound place is matched exactly; the marking then
//! cascades through the containment and field links `mark_unresolved_effect`
//! already follows. The reader also records, for the card's census only, the
//! places a statement hands to a callee ([`HandOn::Call`]); the rule never reads
//! them.
//!
//! The CFG node carries its statement's preprocessed line, not its tokens, so
//! the table is keyed by line, as CTWIN-RESIDUALS' `publications.indirect` is.
//! A macro that expands several statements onto one line unions them, which
//! again can only withdraw more.

use std::collections::{BTreeMap, BTreeSet};

use super::{AbstractState, LIVE, LegacyDiagnostic, LegacyDiagnosticKind, is_identifier};

/// How a statement hands a place on. `Escape` dominates when two statements
/// of one line hand the same place on differently.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum HandOn {
    /// Passed, or its address passed, to a call: the callee decides.
    Call,
    /// Stored or returned as the statement's value: it leaves the projection.
    Escape,
}

/// The places each preprocessed line's statements hand on.
pub(crate) type LineReads = BTreeMap<usize, BTreeMap<String, HandOn>>;

/// Read one body's [`LineReads`] from its token texts and their lines.
///
/// `body` excludes the braces; `lines[i]` is the preprocessed line of
/// `body[i]`. A statement runs to the next `;`, `{` or `}`.
pub(crate) fn line_reads(body: &[&str], lines: &[usize]) -> LineReads {
    let mut reads = LineReads::new();
    let mut start = 0usize;
    for index in 0..=body.len() {
        let boundary = index == body.len() || matches!(body[index], ";" | "{" | "}");
        if !boundary {
            continue;
        }
        if index > start {
            let places = handed_on(&body[start..index]);
            if !places.is_empty() {
                for line in lines[start..index].iter().copied().collect::<BTreeSet<_>>() {
                    let entry = reads.entry(line).or_default();
                    for (place, how) in &places {
                        let current = entry.entry(place.clone()).or_insert(*how);
                        *current = (*current).max(*how);
                    }
                }
            }
        }
        start = index + 1;
    }
    reads
}

/// Whether the token before `index` ends an operand, so that a `*` or `&` at
/// `index` is binary rather than unary.
fn ends_operand(tokens: &[&str], index: usize) -> bool {
    index
        .checked_sub(1)
        .and_then(|previous| tokens.get(previous))
        .is_some_and(|text| {
            is_identifier(text)
                || matches!(*text, ")" | "]")
                || text.as_bytes().first().is_some_and(u8::is_ascii_digit)
        })
}

/// The keywords whose `(` opens a grouping, not a call.
const GROUPING_HEADS: &[&str] = &["if", "while", "for", "switch", "return"];

/// The type operators whose parenthesised operand is not evaluated as a value.
const UNEVALUATED_HEADS: &[&str] = &[
    "sizeof",
    "_Alignof",
    "alignof",
    "typeof",
    "__typeof",
    "__typeof__",
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Paren {
    Group,
    Call,
    Unevaluated,
}

/// The places one statement hands on, and how (see the module header).
pub(crate) fn handed_on(statement: &[&str]) -> BTreeMap<String, HandOn> {
    let mut places = BTreeMap::new();
    let returns = statement.first() == Some(&"return");
    // The index of the top-level simple assignment, if any.
    let assignment = {
        let mut depth = 0usize;
        let mut found = None;
        for (index, text) in statement.iter().enumerate() {
            match *text {
                "(" | "[" => depth += 1,
                ")" | "]" => depth = depth.saturating_sub(1),
                "=" if depth == 0
                    && index > 0
                    && statement.get(index + 1) != Some(&"=")
                    && !matches!(
                        statement[index - 1],
                        "=" | "!" | "<" | ">" | "+" | "-" | "*" | "/" | "%" | "&" | "|" | "^"
                    ) =>
                {
                    found = Some(index);
                    break;
                }
                _ => {}
            }
        }
        found
    };
    let mut parens: Vec<Paren> = Vec::new();
    let mut index = 0usize;
    while index < statement.len() {
        let text = statement[index];
        match text {
            "(" => {
                let head = index.checked_sub(1).map(|previous| statement[previous]);
                let kind = match head {
                    Some(head) if UNEVALUATED_HEADS.contains(&head) => Paren::Unevaluated,
                    Some(head) if GROUPING_HEADS.contains(&head) => Paren::Group,
                    Some(head) if is_identifier(head) => Paren::Call,
                    // `(*fp)(…)` and `a[i](…)`: a call through an expression.
                    Some(")" | "]") => Paren::Call,
                    _ => Paren::Group,
                };
                parens.push(kind);
                index += 1;
                continue;
            }
            ")" => {
                parens.pop();
                index += 1;
                continue;
            }
            _ => {}
        }
        let member = index >= 1
            && (statement[index - 1] == "."
                || (statement[index - 1] == ">" && index >= 2 && statement[index - 2] == "-"));
        if !is_identifier(text) || member || GROUPING_HEADS.contains(&text) {
            index += 1;
            continue;
        }
        // The access chain rooted here, spelled as `canonical_place` spells it.
        let mut chain = vec![text.to_owned()];
        let mut cursor = index;
        loop {
            match (
                statement.get(cursor + 1),
                statement.get(cursor + 2),
                statement.get(cursor + 3),
            ) {
                (Some(&"."), Some(field), _) if is_identifier(field) => {
                    let spelled = format!("{}.{field}", chain.last().expect("root"));
                    chain.push(spelled);
                    cursor += 2;
                }
                (Some(&"-"), Some(&">"), Some(field)) if is_identifier(field) => {
                    let spelled = format!("{}->{field}", chain.last().expect("root"));
                    chain.push(spelled);
                    cursor += 3;
                }
                (Some(&"["), Some(digits), Some(&"]"))
                    if digits.bytes().all(|byte| byte.is_ascii_digit()) =>
                {
                    let spelled = format!("{}[{digits}]", chain.last().expect("root"));
                    chain.push(spelled);
                    cursor += 3;
                }
                _ => break,
            }
        }
        let how = if parens.contains(&Paren::Unevaluated) {
            None
        } else if parens.contains(&Paren::Call) {
            Some(HandOn::Call)
        } else if returns || assignment.is_some_and(|equal| index > equal) {
            Some(HandOn::Escape)
        } else {
            None
        };
        // A unary `&` before the chain, possibly behind opening parentheses.
        let mut before = index;
        while before > 0 && statement[before - 1] == "(" {
            before -= 1;
        }
        let address =
            before > 0 && statement[before - 1] == "&" && !ends_operand(statement, before - 1);
        let dereference =
            index > 0 && statement[index - 1] == "*" && !ends_operand(statement, index - 1);
        let next = statement.get(cursor + 1).copied();
        let written = next == Some("=") && statement.get(cursor + 2) != Some(&"=");
        if let Some(how) = how {
            let full = chain.last().expect("root").clone();
            let handed = if address {
                chain
            } else if written || matches!(next, Some("[" | "(")) {
                // Overwritten, or the base of a subscript or of a call through
                // a member: the chain's own value is not handed on.
                Vec::new()
            } else if dereference && chain.len() == 1 {
                vec![format!("*{full}")]
            } else {
                vec![full]
            };
            for place in handed {
                let current = places.entry(place).or_insert(how);
                *current = (*current).max(how);
            }
        }
        index = cursor + 1;
    }
    places
}

/// The identities a statement at `line` hands on as an `Escape` in `state`:
/// the targets of every such bound place, restricted to those that may still
/// be live there. An identity already released or escaped on every path owes
/// nothing a refusal could discharge, and leaving it alone keeps every
/// accusation about it exactly as it was.
///
/// Computed **before** the statement's transfer, from the state it reads.
pub(super) fn read_sites(state: &AbstractState, reads: &LineReads, line: usize) -> BTreeSet<usize> {
    handed_sites(state, reads, line, HandOn::Escape)
}

/// The live identities a statement at `line` hands on at least as `how`.
pub(super) fn handed_sites(
    state: &AbstractState,
    reads: &LineReads,
    line: usize,
    how: HandOn,
) -> BTreeSet<usize> {
    let Some(places) = reads.get(&line) else {
        return BTreeSet::new();
    };
    state
        .pointers
        .iter()
        .filter(|(place, _)| places.get(*place).is_some_and(|handed| *handed >= how))
        .flat_map(|(_, pointer)| pointer.targets.iter().copied())
        .filter(|site| {
            state
                .resources
                .get(site)
                .is_some_and(|status| status & LIVE != 0)
        })
        .collect()
}

/// Whether one node's own transfer refused the statement.
pub(super) fn refused(diagnostics: &BTreeSet<LegacyDiagnostic>) -> bool {
    diagnostics
        .iter()
        .any(|diagnostic| matches!(diagnostic.kind, LegacyDiagnosticKind::Unsupported { .. }))
}

/// Mark `sites`, and everything contained in or field-linked from them, as
/// carrying an unresolved effect — `mark_unresolved_effect`'s cascade over an
/// explicit site set.
pub(super) fn withdraw(state: &mut AbstractState, sites: &BTreeSet<usize>) {
    let mut pending = sites.clone();
    let mut visited = BTreeSet::new();
    while let Some(site) = pending.pop_first() {
        if visited.insert(site) {
            state.unresolved_effect_sites.insert(site);
            if let Some(children) = state.contained.get(&site) {
                pending.extend(children);
            }
            if let Some(fields) = state.field_links.get(&site) {
                pending.extend(fields.values().flatten().copied());
            }
        }
    }
}

/// The rule: after a node's transfer, if the node refused its statement, the
/// identities the statement's value side handed on before it ran carry an
/// unresolved effect.
pub(super) fn apply(
    output: &mut AbstractState,
    read: &BTreeSet<usize>,
    diagnostics: &BTreeSet<LegacyDiagnostic>,
) {
    if !read.is_empty() && refused(diagnostics) {
        withdraw(output, read);
    }
}

#[cfg(test)]
#[path = "../tests/unit/legacy_refused_operands.rs"]
mod tests;
