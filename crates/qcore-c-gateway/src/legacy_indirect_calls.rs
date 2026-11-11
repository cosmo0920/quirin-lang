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

//! Card **CG22-INDIRECT-CALLS-AND-CALLBACKS** (gateway rule 11p), route G:
//! a call through a function pointer whose candidate set the unit closes is
//! rewritten, in the token stream, to the calls C may make.
//!
//! # Where
//!
//! Once per translation unit, right after card W45-PARENTHESISED-CALLEE's
//! normalisation (rule 10c) and before every other rewrite and reader: the
//! reserved-spelling escape, the review, pin and refutation holds, both
//! declared-contract withdrawals, the literal premises, W4's derivation scope
//! and every classifier position. So a rewritten candidate is held,
//! withdrawn, restricted, summarised and classified exactly as a direct call
//! of it written in the source would be; this pass decides no contract.
//!
//! # The rewrite
//!
//! For each call in a body whose callee is not a designator, Clang's AST
//! gives the callee's candidate set (`indirect_calls::resolve`):
//!
//! * **a singleton** `{f}`: the callee's tokens are replaced by `f` (the call
//!   C makes, in every position);
//! * **a larger set** `{f₁, …, fₙ}`, for a call that is a whole expression
//!   statement, the value of `NAME = …;` or a `return` operand: the
//!   statement `S` becomes `{ int c₁; …; if (c₁) S[f₁]; else if (c₂) S[f₂];
//!   … else S[fₙ]; }`, each `cₖ` a fresh, never-assigned scalar, so the flow
//!   explores every candidate's arm and joins them (rule 5's join; the
//!   verdict is the join over the candidates, `QCore.IndirectCalls`);
//! * **an open set**, or a site this pass cannot place in the stream
//!   uniquely: nothing changes, and the call keeps its older refusal byte for
//!   byte (`call to uncontracted function …`, the indirect-access gate, or
//!   rule 10c's opaque-callee refusal).
//!
//! A site is placed only when the callee's tokens occur on its line exactly
//! as often as Clang has sites with those tokens there, every such site has
//! the same resolution, and no conditional directive falls inside the span.
//! A unit that spells the choice prefix itself gets no branch rewrite.
//!
//! Parity runs record every site (`position: "indirect-call"`) and every call
//! passing a function designator (`position: "callback-registration"`).

use std::collections::{BTreeMap, BTreeSet};

use qcore_schemas::contract::{ResourceContracts, ResourceParameterMode};
use serde_json::json;

use super::{ConditionalMark, Token};
use crate::indirect_calls::{
    Form, Resolution, Verdict, callback_trace, context_trace, indirect_sites, join_over,
    local_definitions, registrations, resolve,
};
use crate::typed_access_path::{IndirectCallUnit, record_row, with_indirect_call_unit};

/// Whether this pass rewrites (`false`: census rows only).
pub(crate) const ADMIT: bool = true;

/// The prefix of the fresh choice scalars a branch rewrite declares.
pub(crate) const CHOICE_PREFIX: &str = "qcore_indirect_choice__";

/// The sites of one (line, callee tokens) group: index and resolution.
type Group = Vec<(usize, Resolution)>;

/// One edit of the stream: `[start, end)` replaced by `tokens`.
struct Edit {
    start: usize,
    end: usize,
    tokens: Vec<Token>,
}

/// A run of `(`ᵏ NAME `)`ᵏ is NAME, as rule 10c's normalisation spells it.
fn normalised(texts: Vec<String>) -> Vec<String> {
    let mut call = texts.iter().map(String::as_str).collect::<Vec<_>>();
    call.push("(");
    let mut out = super::parenthesised_callee::strip_callee_parentheses(&call);
    out.pop();
    out
}

/// Positions in `range` on `line` where `needle` occurs followed by `next`,
/// not preceded by a member operator.
fn occurrences(
    tokens: &[Token],
    range: std::ops::Range<usize>,
    line: usize,
    needle: &[String],
    next: &str,
) -> Vec<usize> {
    if needle.is_empty() {
        return Vec::new();
    }
    range
        .filter(|&at| {
            tokens[at].line == line
                && at + needle.len() < tokens.len()
                && tokens[at..at + needle.len()]
                    .iter()
                    .zip(needle)
                    .all(|(token, text)| &token.text == text)
                && tokens[at + needle.len()].text == next
                && !(at > 0 && matches!(tokens[at - 1].text.as_str(), "." | "->"))
        })
        .collect()
}

fn token(text: &str, line: usize) -> Token {
    Token {
        text: text.to_owned(),
        line,
    }
}

/// The census row of one site.
#[allow(clippy::too_many_arguments)]
fn site_row(
    function: &str,
    line: usize,
    callee: &[String],
    form: Form,
    resolution: &Resolution,
    contracts: &ResourceContracts,
    definitions: &BTreeSet<String>,
    rewrite: &str,
) {
    let (candidates, open) = match &resolution.candidates {
        Ok(set) => (
            set.iter()
                .map(|name| {
                    json!({
                        "name": name,
                        "declared": contracts.contract(name).is_some(),
                        "defined": definitions.contains(name),
                    })
                })
                .collect::<Vec<_>>(),
            None,
        ),
        Err(open) => (Vec::new(), Some(open.describe())),
    };
    record_row(json!({
        "function": function,
        "line": line,
        "texts": callee,
        "position": "indirect-call",
        "shape": resolution.shape.as_str(),
        "form": form.as_str(),
        "candidates": candidates,
        "open": open,
        "rewrite": rewrite,
    }));
}

/// The rewritten stream and marks, or `None` when nothing changes.
pub(super) fn rewrite_unit(
    tokens: &[Token],
    conditionals: &[ConditionalMark],
    contracts: &ResourceContracts,
) -> Option<(Vec<Token>, Vec<ConditionalMark>)> {
    let definitions = super::function_names(tokens, conditionals).definitions;
    if !may_call_indirectly(tokens, &definitions) {
        return None;
    }
    let defined = definitions.iter().cloned().collect::<BTreeSet<_>>();
    let choice_spelled = tokens
        .iter()
        .any(|token| token.text.starts_with(CHOICE_PREFIX));
    let edits = with_indirect_call_unit(|unit| {
        let mut edits = Vec::new();
        let mut seen = BTreeSet::new();
        let mut choices = 0usize;
        for function in &definitions {
            let Some((body_start, _)) = super::find_function_body(tokens, function) else {
                continue;
            };
            let Some(body_end) = super::matching(tokens, body_start, "{", "}") else {
                continue;
            };
            if !seen.insert(body_start) {
                continue;
            }
            let Some(ast) = unit.function(function) else {
                continue;
            };
            function_edits(
                unit,
                tokens,
                body_start + 1..body_end,
                function,
                ast,
                contracts,
                &defined,
                !choice_spelled,
                &mut choices,
                &mut edits,
            );
        }
        edits
    })?;
    if !ADMIT || edits.is_empty() {
        return None;
    }
    apply(tokens, conditionals, edits)
}

/// A cost pre-filter, never an admission: whether some body may call
/// through something other than a name the unit declares at file scope (a
/// `NAME (` there), so Clang's AST is worth reading. A body whose every call
/// is `NAME (` of such a name, a keyword or a builtin has no site this pass
/// could rewrite; skipping it leaves the stream unchanged, every older
/// refusal standing.
fn may_call_indirectly(tokens: &[Token], definitions: &[String]) -> bool {
    let mut declared = BTreeSet::new();
    let mut depth = 0usize;
    for (at, token) in tokens.iter().enumerate() {
        match token.text.as_str() {
            "{" => depth += 1,
            "}" => depth = depth.saturating_sub(1),
            text if depth == 0
                && tokens.get(at + 1).is_some_and(|next| next.text == "(")
                && super::parenthesised_callee::is_name(text) =>
            {
                declared.insert(text);
            }
            _ => {}
        }
    }
    definitions.iter().any(|function| {
        let Some((body_start, _)) = super::find_function_body(tokens, function) else {
            return false;
        };
        let Some(body_end) = super::matching(tokens, body_start, "{", "}") else {
            return false;
        };
        (body_start + 1..body_end).any(|at| {
            if tokens[at + 1].text != "(" {
                return false;
            }
            let text = tokens[at].text.as_str();
            match text {
                ")" | "]" => true,
                _ if !super::parenthesised_callee::is_name(text) => false,
                _ if at > 0 && matches!(tokens[at - 1].text.as_str(), "." | "->") => true,
                _ => {
                    !declared.contains(text)
                        && !super::parenthesised_callee::is_keyword(text)
                        && !super::TYPE_OPERATORS.contains(&text)
                        && !text.starts_with("__builtin")
                }
            }
        })
    })
}

#[allow(clippy::too_many_arguments)]
fn function_edits(
    unit: &IndirectCallUnit<'_>,
    tokens: &[Token],
    body: std::ops::Range<usize>,
    function: &str,
    ast: &serde_json::Value,
    contracts: &ResourceContracts,
    defined: &BTreeSet<String>,
    branches: bool,
    choices: &mut usize,
    edits: &mut Vec<Edit>,
) {
    // Census (K02): every call passing a function designator, with the
    // callback relation's decision over the trace this body shows for each
    // named context (the registration, then each release of it).
    let released = |callee: &str, at: usize| {
        contracts.contract(callee).is_some_and(|contract| {
            matches!(
                contract.parameters.get(at),
                Some(ResourceParameterMode::Released { .. })
            )
        })
    };
    for registration in registrations(ast) {
        let traces = registration
            .contexts
            .iter()
            .flatten()
            .map(|context| {
                let trace = context_trace(ast, &registration, context, &released);
                format!("{:?}", callback_trace(&trace))
            })
            .collect::<Vec<_>>();
        let callee = registration.callee.as_deref();
        record_row(json!({
            "function": function,
            "line": unit.node_line(registration.call),
            "position": "callback-registration",
            "callee": callee,
            "callee_declared": callee.is_some_and(|name| contracts.contract(name).is_some()),
            "designators": registration.designators,
            "contexts": registration.contexts.len(),
            "named_contexts": traces.len(),
            "context_relation": traces,
        }));
    }
    let sites = indirect_sites(ast);
    // Each resolved local: its definitions' values, its reference count, its
    // assignment count and the rewritten callee reads of it.
    #[allow(clippy::type_complexity)]
    let mut locals: BTreeMap<
        String,
        (
            Vec<&serde_json::Value>,
            usize,
            usize,
            Vec<&serde_json::Value>,
        ),
    > = BTreeMap::new();
    // Group the sites by (line, callee tokens): each group is placed whole.
    let mut groups: BTreeMap<(usize, Vec<String>), Group> = BTreeMap::new();
    for (index, site) in sites.iter().enumerate() {
        let Some(callee) = crate::typed_c_ast::children(site.call).first() else {
            continue;
        };
        let (Some(line), Some(texts)) = (unit.node_line(callee), unit.node_tokens(callee)) else {
            continue;
        };
        let resolution = resolve(callee, ast, unit.tables());
        groups
            .entry((line, normalised(texts)))
            .or_default()
            .push((index, resolution));
    }
    for ((line, callee), members) in groups {
        let found = occurrences(tokens, body.clone(), line, &callee, "(");
        let same = members.windows(2).all(|pair| {
            pair[0].1.candidates == pair[1].1.candidates
                && sites[pair[0].0].form == sites[pair[1].0].form
        });
        let placed = found.len() == members.len() && same;
        for (at_member, (index, resolution)) in members.iter().enumerate() {
            let site = &sites[*index];
            // `QCore.IndirectCalls`: an open set's join is `Unknown` whatever
            // its members, so it is never rewritten; the older refusal stays.
            let covers = resolution.candidates.is_ok();
            let decision = match (&resolution.candidates, placed) {
                _ if join_over(covers, &[Verdict::Safe]) == Verdict::Unknown => {
                    "none: open set".to_owned()
                }
                (Err(_), _) => "none: open set".to_owned(),
                (Ok(_), false) => "none: not placed uniquely".to_owned(),
                (Ok(set), true) if set.len() == 1 => {
                    let at = found[at_member];
                    let name = set.iter().next().expect("one candidate");
                    edits.push(Edit {
                        start: at,
                        end: at + callee.len(),
                        tokens: vec![token(name, line)],
                    });
                    "singleton".to_owned()
                }
                (Ok(set), true) => match branch_edit(
                    unit,
                    tokens,
                    &body,
                    site,
                    &callee,
                    found[at_member],
                    set,
                    branches,
                    choices,
                ) {
                    Ok(edit) => {
                        edits.push(edit);
                        "branch".to_owned()
                    }
                    Err(reason) => format!("none: {reason}"),
                },
            };
            if decision == "singleton" || decision == "branch" {
                let callee_node = crate::typed_c_ast::children(site.call)
                    .first()
                    .expect("a call has a callee");
                if let Some(definitions) = local_definitions(callee_node, ast) {
                    let entry = locals.entry(definitions.id.clone()).or_insert_with(|| {
                        (
                            definitions.values.clone(),
                            definitions.references,
                            definitions.assigned,
                            Vec::new(),
                        )
                    });
                    if !entry
                        .3
                        .iter()
                        .any(|seen: &&serde_json::Value| std::ptr::eq(*seen, definitions.read))
                    {
                        entry.3.push(definitions.read);
                    }
                }
            }
            site_row(
                function, line, &callee, site.form, resolution, contracts, defined, &decision,
            );
        }
    }
    // A resolved local every other reference of which is a rewritten callee
    // is never read again: each of its definitions' values becomes `0`.
    for (name, (values, references, assigned, reads)) in locals {
        if references != assigned + reads.len() {
            continue;
        }
        let Some(local) = reads
            .first()
            .and_then(|read| read["referencedDecl"]["name"].as_str())
        else {
            continue;
        };
        let _ = name;
        let mut by_place: BTreeMap<(usize, Vec<String>), usize> = BTreeMap::new();
        let mut complete = true;
        for value in &values {
            match (unit.node_line(value), unit.node_tokens(value)) {
                (Some(line), Some(texts)) => {
                    *by_place.entry((line, normalised_value(texts))).or_default() += 1;
                }
                _ => complete = false,
            }
        }
        if !complete {
            continue;
        }
        let mut placed = Vec::new();
        for ((line, texts), count) in &by_place {
            let found = body
                .clone()
                .filter(|&at| {
                    at >= 2
                        && tokens[at].line == *line
                        && tokens[at - 1].text == "="
                        && tokens[at - 2].text == local
                        && at + texts.len() < tokens.len()
                        && tokens[at..at + texts.len()]
                            .iter()
                            .zip(texts)
                            .all(|(token, text)| &token.text == text)
                        && matches!(tokens[at + texts.len()].text.as_str(), ";" | ",")
                })
                .collect::<Vec<_>>();
            if found.len() != *count {
                complete = false;
                break;
            }
            placed.extend(found.into_iter().map(|at| (at, texts.len(), *line)));
        }
        if !complete {
            continue;
        }
        for (at, length, line) in placed {
            edits.push(Edit {
                start: at,
                end: at + length,
                tokens: vec![token("0", line)],
            });
        }
    }
}

/// A value's tokens as the normalised stream spells them.
fn normalised_value(texts: Vec<String>) -> Vec<String> {
    super::parenthesised_callee::strip_callee_parentheses(
        &texts.iter().map(String::as_str).collect::<Vec<_>>(),
    )
}

/// The branch rewrite of one statement whose callee has several candidates.
#[allow(clippy::too_many_arguments)]
fn branch_edit(
    unit: &IndirectCallUnit<'_>,
    tokens: &[Token],
    body: &std::ops::Range<usize>,
    site: &crate::indirect_calls::Site<'_>,
    callee: &[String],
    callee_at: usize,
    candidates: &BTreeSet<String>,
    branches: bool,
    choices: &mut usize,
) -> Result<Edit, &'static str> {
    if !branches {
        return Err("choice prefix spelled by the unit");
    }
    if !matches!(site.form, Form::Statement | Form::Assignment | Form::Return) {
        return Err("several candidates outside a statement, assignment or return");
    }
    let statement = site.statement.ok_or("no statement node")?;
    let line = unit.node_line(statement).ok_or("no statement line")?;
    let texts = unit.node_tokens(statement).ok_or("no statement tokens")?;
    let texts = super::parenthesised_callee::strip_callee_parentheses(
        &texts.iter().map(String::as_str).collect::<Vec<_>>(),
    );
    let found = occurrences(tokens, body.clone(), line, &texts, ";");
    let [start] = found.as_slice() else {
        return Err("statement not placed uniquely");
    };
    let start = *start;
    let end = start + texts.len() + 1;
    if !(start..end - 1).contains(&callee_at) || callee_at + callee.len() > end - 1 {
        return Err("callee outside its statement");
    }
    let mut out = vec![token("{", line)];
    let names = (0..candidates.len() - 1)
        .map(|_| {
            *choices += 1;
            format!("{CHOICE_PREFIX}{choices}")
        })
        .collect::<Vec<_>>();
    for name in &names {
        out.extend([token("int", line), token(name, line), token(";", line)]);
    }
    for (k, candidate) in candidates.iter().enumerate() {
        if k > 0 {
            out.push(token("else", line));
        }
        if let Some(name) = names.get(k) {
            out.extend([
                token("if", line),
                token("(", line),
                token(name, line),
                token(")", line),
            ]);
        }
        out.extend(tokens[start..callee_at].iter().cloned());
        out.push(token(candidate, line));
        out.extend(tokens[callee_at + callee.len()..end].iter().cloned());
    }
    out.push(token("}", line));
    Ok(Edit {
        start,
        end,
        tokens: out,
    })
}

/// Apply non-overlapping edits that hold no conditional directive inside,
/// remapping the marks.
fn apply(
    tokens: &[Token],
    conditionals: &[ConditionalMark],
    mut edits: Vec<Edit>,
) -> Option<(Vec<Token>, Vec<ConditionalMark>)> {
    edits.sort_by_key(|edit| (edit.start, edit.end));
    let mut kept: Vec<Edit> = Vec::new();
    for edit in edits {
        let overlaps = kept.last().is_some_and(|last| edit.start < last.end);
        let marked = conditionals
            .iter()
            .any(|mark| edit.start < mark.token_index && mark.token_index < edit.end);
        if overlaps || marked {
            // A nested site inside a branch rewrite keeps its older form.
            continue;
        }
        kept.push(edit);
    }
    if kept.is_empty() {
        return None;
    }
    let mut out = Vec::with_capacity(tokens.len());
    let mut cursor = 0;
    for edit in &kept {
        out.extend(tokens[cursor..edit.start].iter().cloned());
        out.extend(edit.tokens.iter().cloned());
        cursor = edit.end;
    }
    out.extend(tokens[cursor..].iter().cloned());
    let marks = conditionals
        .iter()
        .map(|mark| {
            let shift = kept
                .iter()
                .filter(|edit| edit.end <= mark.token_index)
                .map(|edit| edit.tokens.len() as isize - (edit.end - edit.start) as isize)
                .sum::<isize>();
            ConditionalMark {
                token_index: (mark.token_index as isize + shift) as usize,
                directive: mark.directive,
                span: mark.span,
            }
        })
        .collect();
    Some((out, marks))
}

/// [`rewrite_unit`] over an owned stream.
pub(super) fn rewrite_owned(
    tokens: Vec<Token>,
    conditionals: Vec<ConditionalMark>,
    contracts: &ResourceContracts,
) -> (Vec<Token>, Vec<ConditionalMark>) {
    match rewrite_unit(&tokens, &conditionals, contracts) {
        Some(rewritten) => rewritten,
        None => (tokens, conditionals),
    }
}

/// The same rewrite for an entry that holds a borrowed stream with no marks.
pub(super) fn rewrite_tokens(
    tokens: &[Token],
    contracts: &ResourceContracts,
) -> Option<Vec<Token>> {
    rewrite_unit(tokens, &[], contracts).map(|(tokens, _)| tokens)
}
