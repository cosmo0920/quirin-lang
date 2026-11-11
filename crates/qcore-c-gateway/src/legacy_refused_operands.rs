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

//! Card **W44-LEAK-PIN-AUDIT**: the refused hand-on rule.
//!
//! # The mechanism this module repairs
//!
//! A statement the projection refuses reports `Unsupported` and poisons the
//! pointer **bindings** it may write (card AOM-BINDING-POISON), but it never
//! touches a resource **status**. So an identity the refused statement hands
//! on stays `LIVE`, and `report_leaks` accuses it at the next exit although the
//! refused statement may have stored it (`map->parent = (void *) counter;`,
//! `*bufferp = buf;`, `return (const char **) arr;`), released it
//! (`destroy_metric(metric);`), published it (`cfl_list_add(&sample->_head, …)`)
//! or handed it to an owner (`rkbuf = rd_kafka_buf_new_shadow(buffer, …,
//! rd_free);`). A leak claimed after the identity reached a statement of
//! unknown effect is not a validated reachable violation. The card's audit of
//! the 271 baseline leak leaves measured the mechanism, and card
//! W44-COLLECTION-ALLOCATION measured it again as 107 of its 125 new leak
//! claims (`W44-REFUSED-HANDOFF-LEAK-CLAIMS`).
//!
//! The projection already answers the modelled half of the situation. A
//! contracted call whose actual it cannot resolve marks the actual's identities
//! through `mark_unresolved_effect`, whose documented reading is "custody
//! retained after a rejected effect; its residual is not a proven leak because
//! the unmodeled action may have discharged it", and card CTWIN-RESIDUALS marks
//! the value operands of a refused continued-access publication the same way.
//! This module applies that reading to every refused statement: the live
//! identities it hands on carry an unresolved effect from there on.
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
//! # The reader: what a statement hands on
//!
//! Only a mention the classifier's own tokens show counts; a name only inside a
//! macro body the tokens do not show is not a mention. A place is handed on
//! where the statement names it as a value or takes the address of it or of
//! storage reached through it, tagged by position ([`HandOn`]):
//!
//! * **`Escape`**, the statement's value side — a `return` operand or the
//!   right-hand side of a top-level simple assignment, outside every call:
//!   `x = p`, `x->f = (void *) p`, `*out = p`, `local.slot = p`, `return p`;
//! * **`Call`**, inside a call's argument list: `f(p)`, `f(&p->head)`,
//!   `f(p + 1)`;
//! * **`Operand`**, anywhere else it is evaluated: `if (!p || g(q))`, `p + n`.
//!
//! `&p->head` and `&p[0]` take an address inside `*p`, so they hand on `p` and
//! every prefix of the chain. What is **not** a hand-on: a member value read
//! through a base (`p->len`, `p->buf[i]`, `p->fn(…)` name the member, and
//! `p->buf` hands on the place `p->buf`, not `p`); the destination of a simple
//! assignment (`p = …`, `p->next = …`, `*out = …`), which is written; and the
//! operand of `sizeof` and its kin, which is not evaluated. A statement that
//! reads or writes only a member through the identity is thereby assumed not to
//! hand the identity on (`REFUSED-OPERAND-MEMBER-VALUE-READ` in the card's
//! report; a self-referencing member, such as an empty intrusive list head read
//! as `p->head.next`, is the one C shape that breaks it).
//!
//! Places are spelled as `canonical_place` spells them (`p`, `p->f`, `p.f`,
//! `p[0]`, `*p`), and a bound place is matched exactly; the marking then
//! cascades through the containment and field links `mark_unresolved_effect`
//! already follows.
//!
//! # Which statement a refused node is
//!
//! A CFG node carries its statement's preprocessed line, not its tokens, so the
//! table is keyed by line, as CTWIN-RESIDUALS' `publications.indirect` is. A
//! macro expands several statements onto one line (`RD_UT_ASSERT(cond, …)` is
//! `if (!(cond)) { fprintf(…); …; return 1; }`), so each line keeps two tables:
//! the **condition** of an `if`, `while` or `switch` (and every clause of a
//! `for` header), and the **statements**. A node refused as a condition reads
//! the condition table, a node refused as a statement reads the statement
//! table, and a node whose refusal came from a modelled instruction's transfer
//! reads both. The statements of one line still union, which only withdraws
//! more (`REFUSED-ESCAPE-LINE-GRANULARITY`).

use std::collections::{BTreeMap, BTreeSet};

use super::{
    AbstractState, LIVE, LegacyCfgNode, LegacyDiagnostic, LegacyDiagnosticKind, LegacyInstruction,
    is_identifier,
};

/// How a statement hands a place on, weakest first. The stronger tag wins when
/// two statements of one line hand the same place on differently.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum HandOn {
    /// Named as an operand outside the value side and outside every call.
    Operand,
    /// Passed, or its address passed, to a call: the callee decides.
    Call,
    /// Stored or returned as the statement's value: it leaves the projection.
    Escape,
}

/// The weakest hand-on the rule withdraws on: every one.
pub(crate) const WITHDRAWN_FROM: HandOn = HandOn::Operand;

/// The places one line's statements hand on, split by position.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LineHandOns {
    /// Handed on by the condition of an `if`, `while` or `switch`, or by a
    /// clause of a `for` header.
    pub condition: BTreeMap<String, HandOn>,
    /// Handed on by every other statement.
    pub statement: BTreeMap<String, HandOn>,
}

impl LineHandOns {
    /// Union another line table into this one, the stronger tag winning.
    pub(crate) fn absorb(&mut self, other: Self) {
        merge(&mut self.condition, &other.condition);
        merge(&mut self.statement, &other.statement);
    }
}

fn merge(into: &mut BTreeMap<String, HandOn>, from: &BTreeMap<String, HandOn>) {
    for (place, how) in from {
        let current = into.entry(place.clone()).or_insert(*how);
        *current = (*current).max(*how);
    }
}

/// The places each preprocessed line's statements hand on.
pub(crate) type LineReads = BTreeMap<usize, LineHandOns>;

/// The keywords whose parenthesised group is a controlling condition.
const CONDITION_HEADS: &[&str] = &["if", "while", "switch"];

/// Read one body's [`LineReads`] from its token texts and their lines.
///
/// `body` excludes the braces; `lines[i]` is the preprocessed line of
/// `body[i]`. A statement runs to the next `;`, `{` or `}`. A statement headed
/// by `if`, `while` or `switch` splits at its condition's closing parenthesis
/// (a brace-less body follows it on the same statement); a `for` header's
/// clauses go to both tables.
pub(crate) fn line_reads(body: &[&str], lines: &[usize]) -> LineReads {
    let mut reads = LineReads::new();
    let mut start = 0usize;
    let mut for_clauses = 0usize;
    for index in 0..=body.len() {
        let boundary = index == body.len() || matches!(body[index], ";" | "{" | "}");
        if !boundary {
            continue;
        }
        if index > start {
            let segment = &body[start..index];
            let segment_lines = &lines[start..index];
            let mut head = 0usize;
            if segment.first() == Some(&"else") {
                head = 1;
            }
            if segment.get(head) == Some(&"for") {
                for_clauses = 2;
                record(&mut reads, segment, segment_lines, Table::Both);
            } else if for_clauses > 0 {
                for_clauses -= 1;
                record(&mut reads, segment, segment_lines, Table::Both);
            } else if segment
                .get(head)
                .is_some_and(|word| CONDITION_HEADS.contains(word))
                && segment.get(head + 1) == Some(&"(")
                && let Some(close) = matching_close(segment, head + 1)
            {
                record(
                    &mut reads,
                    &segment[..=close],
                    &segment_lines[..=close],
                    Table::Condition,
                );
                record(
                    &mut reads,
                    &segment[close + 1..],
                    &segment_lines[close + 1..],
                    Table::Statement,
                );
            } else {
                record(&mut reads, segment, segment_lines, Table::Statement);
            }
        }
        start = index + 1;
    }
    reads
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Table {
    Condition,
    Statement,
    Both,
}

fn record(reads: &mut LineReads, segment: &[&str], lines: &[usize], table: Table) {
    if segment.is_empty() {
        return;
    }
    let places = handed_on(segment);
    if places.is_empty() {
        return;
    }
    for line in lines.iter().copied().collect::<BTreeSet<_>>() {
        let entry = reads.entry(line).or_default();
        if table != Table::Statement {
            merge(&mut entry.condition, &places);
        }
        if table != Table::Condition {
            merge(&mut entry.statement, &places);
        }
    }
}

/// The index of the `)` matching the `(` at `open`.
fn matching_close(tokens: &[&str], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (index, text) in tokens.iter().enumerate().skip(open) {
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
        if !is_identifier(text)
            || member
            || GROUPING_HEADS.contains(&text)
            || matches!(text, "else" | "do")
        {
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
            Some(HandOn::Operand)
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

/// Whether a refused `Unknown` instruction is a controlling expression's
/// refusal rather than a statement's.
fn refuses_a_condition(description: &str) -> bool {
    description.contains(" condition")
        || description.contains("switch expression")
        || description.contains("case expression")
}

/// The live identities the statement of `node` hands on at least as `how`:
/// the targets of every bound place of the node's table, restricted to those
/// that may still be live there. An identity already released or escaped on
/// every path owes nothing a refusal could discharge, and leaving it alone
/// keeps every accusation about it exactly as it was.
///
/// Computed **before** the statement's transfer, from the state it reads.
pub(super) fn handed_sites(
    state: &AbstractState,
    reads: &LineReads,
    node: &LegacyCfgNode,
    how: HandOn,
) -> BTreeSet<usize> {
    let Some(line) = reads.get(&node.line) else {
        return BTreeSet::new();
    };
    let (condition, statement) = match &node.instruction {
        LegacyInstruction::Unknown { description, .. } if refuses_a_condition(description) => {
            (true, false)
        }
        LegacyInstruction::Unknown { .. } => (false, true),
        _ => (true, true),
    };
    let handed = |place: &str| {
        (condition && line.condition.get(place).is_some_and(|tag| *tag >= how))
            || (statement && line.statement.get(place).is_some_and(|tag| *tag >= how))
    };
    state
        .pointers
        .iter()
        .filter(|(place, _)| handed(place))
        .flat_map(|(_, pointer)| pointer.targets.iter().copied())
        .filter(|site| {
            state
                .resources
                .get(site)
                .is_some_and(|status| status & LIVE != 0)
        })
        .collect()
}

/// The identities the rule withdraws if `node` refuses its statement.
pub(super) fn read_sites(
    state: &AbstractState,
    reads: &LineReads,
    node: &LegacyCfgNode,
) -> BTreeSet<usize> {
    handed_sites(state, reads, node, WITHDRAWN_FROM)
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

/// Card **W45-FB-BACKLOG-CUSTODY** (integration review): a contracted call the
/// projection refuses *before its effects* — a member-address or bound-read
/// actual whose aggregate does not resolve (cards O04-B09B10, O04-READ-BOUND)
/// — is a refused statement, although it reports the unresolved identity
/// (`UnknownAlias`) rather than `Unsupported`. The call may have published,
/// stored or released what its contract lets it move, so the live identities
/// the caller passes at those places — the actuals at custody positions, the
/// entries of its container effects and the bound reads it takes — carry an
/// unresolved effect, exactly as [`apply`] marks a refused statement's. An
/// actual at a `Borrowed` position with no container effect is not handed on:
/// the declared contract says the call moves nothing through it. Only ever adds to
/// `unresolved_effect_sites`: it withdraws accusations and discharges nothing,
/// and the node keeps its `UnknownAlias`, so the function is never `Safe`.
pub(super) fn withdraw_refused_call<'a>(
    state: &mut AbstractState,
    places: impl IntoIterator<Item = &'a str>,
) {
    let sites = places
        .into_iter()
        .filter_map(|place| state.pointers.get(place))
        .flat_map(|pointer| pointer.targets.iter().copied())
        .filter(|site| {
            state
                .resources
                .get(site)
                .is_some_and(|status| status & LIVE != 0)
        })
        .collect::<BTreeSet<_>>();
    withdraw(state, &sites);
}

/// The rule: after a node's transfer, if the node refused its statement, the
/// identities the statement handed on before it ran carry an unresolved
/// effect.
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
