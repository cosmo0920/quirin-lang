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

//! Card **W43-ABI-ENUM-AND-CONST-PP** (gateway rule 12w): guarded retention
//! clauses, `resource_retained_by(P, H) when X OP K` (contract specification
//! §C04.14.1).
//!
//! USER DECISION 2026-10-05 (recorded verbatim in the specification): a
//! clause conditioned on an integer argument's value applies only where the
//! caller's actual is decided to satisfy the guard; an undecided actual gets
//! the cautious reading, the union of every guarded clause, never the
//! favourable one; where the callee's body is in the unit, each guarded
//! clause is validated under its guard.
//!
//! Three hooks, each the identity for an environment with no guard:
//!
//! 1. [`decide`] (rule 12t's classifier, per retention): the call site's
//!    actual at the guard parameter is decided from its spelling — a decimal
//!    integer constant (C04.5.1's erasures: matched parentheses and an integer
//!    suffix; a leading `-`), or a file-scope enumeration constant whose value
//!    Clang states for the analysed unit and which the calling function does
//!    not shadow — and must lie in the parameter's declared range. A decided
//!    failure drops the clause at that call; a decided success and an
//!    undecided actual keep it.
//! 2. [`settle`] (the unit's environment, beside the ABI gate): in a unit
//!    that defines the callee's body, a guarded clause stands only when the
//!    body is **validated** under it ([`validated`]); otherwise its guard is
//!    removed in that unit, which is the cautious reading at every call.
//! 3. The census of what stands ([`settle`]'s answer is the environment the
//!    analysis reads, so W4 derivation, validation and the certificate
//!    recheck reproduce it from the same stream).
//!
//! # The body check
//!
//! The guard is validated by a structural over-approximation of every way the
//! body could keep `P`: every mention of `P` outside a **guard region** must
//! be one that cannot store `P` or a pointer derived from it — a null or
//! equality test (`!P`, `P == …`, `… != P`, `if (P)`, `while (P)`, `P && …`,
//! `P ? … : …`), or a dereference (`*P`, `*(T *)P`), never under `&`. A guard
//! region is the braced then-block of `if (X OP K)` with exactly the guard's
//! comparison (either operand order for `==`), or a `case L:` arm of
//! `switch (X)` that no other label falls into and whose constant satisfies
//! the guard. `X` must never be written in the body, and a body with `goto`
//! is not validated. Anything else — an alias assignment, a call taking `P`,
//! `&P`, a `return P`, a member or element access through `P` (an array
//! member decays to a pointer into `P`'s object) — outside a region makes the
//! clause unvalidated. The check claims nothing about what the guarded arm
//! does: the retention itself stays a declared assumption
//! (`RETAINED-BY-BODY-CHECK`); it establishes only that, when the guard does
//! not hold, no path of the body can keep `P`.
//!
//! `QCore.RetentionGuard` is the Lean model.

use qcore_schemas::contract::ResourceContracts;
use qcore_schemas::retained_by::{GuardDecision, RetainingContract, RetentionGuard};
use serde_json::Value;

use super::Token;
use crate::typed_c_ast::{children, kind};

/// The prefix of the enumeration-constant probe's declarations.
const PROBE_PREFIX: &str = "__qcore_guard_probe_";

/// The integer suffixes C11 6.4.4.1 admits: an optional `u`/`U` and an
/// optional `l`, `L`, `ll` or `LL` (the two letters of a long long suffix in
/// one case), in either order.
fn is_integer_suffix(suffix: &str) -> bool {
    let long = |text: &str| matches!(text, "" | "l" | "L" | "ll" | "LL");
    let unsigned = |text: &str| matches!(text, "u" | "U");
    long(suffix)
        || suffix.char_indices().any(|(at, _)| {
            let (head, tail) = suffix.split_at(at);
            (unsigned(head) && long(tail)) || (long(head) && unsigned(tail))
        })
        || unsigned(suffix)
}

/// A decimal integer constant, with an optional suffix: `0`, `12`, `12UL`.
/// A leading zero is an octal constant and is not read.
fn decimal(text: &str) -> Option<i128> {
    let digits_end = text
        .find(|ch: char| !ch.is_ascii_digit())
        .unwrap_or(text.len());
    let (digits, suffix) = text.split_at(digits_end);
    if digits.is_empty() || (digits.len() > 1 && digits.starts_with('0')) {
        return None;
    }
    if !is_integer_suffix(suffix) {
        return None;
    }
    digits.parse().ok()
}

/// Strip matched enclosing parentheses.
fn strip<'a>(mut words: &'a [&'a str]) -> &'a [&'a str] {
    while words.len() >= 2 && words[0] == "(" && words[words.len() - 1] == ")" {
        let mut depth = 0usize;
        let mut closes_at_end = true;
        for (index, word) in words.iter().enumerate() {
            match *word {
                "(" => depth += 1,
                ")" => {
                    depth -= 1;
                    if depth == 0 && index + 1 != words.len() {
                        closes_at_end = false;
                        break;
                    }
                }
                _ => {}
            }
        }
        if !closes_at_end {
            break;
        }
        words = &words[1..words.len() - 1];
    }
    words
}

/// The value of a constant spelled as `words`, when it is a decimal integer
/// constant or its negation.
#[must_use]
pub(crate) fn literal_value(words: &[&str]) -> Option<i128> {
    match strip(words) {
        [digits] => decimal(digits),
        ["-", digits] => decimal(digits).map(|value| -value),
        _ => None,
    }
}

fn is_identifier(text: &str) -> bool {
    let mut chars = text.chars();
    chars
        .next()
        .is_some_and(|first| first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

/// Whether `name` is lexically an enumeration constant declared in a
/// file-scope `enum … { … }` of the stream.
fn is_file_scope_enumerator(tokens: &[Token], name: &str) -> bool {
    let mut depth = 0usize;
    let mut index = 0usize;
    while index < tokens.len() {
        match tokens[index].text.as_str() {
            "{" => depth += 1,
            "}" => depth = depth.saturating_sub(1),
            "enum" if depth == 0 => {
                let open = match tokens.get(index + 1).map(|token| token.text.as_str()) {
                    Some("{") => index + 1,
                    Some(tag)
                        if is_identifier(tag)
                            && tokens.get(index + 2).map(|token| token.text.as_str())
                                == Some("{") =>
                    {
                        index + 2
                    }
                    _ => {
                        index += 1;
                        continue;
                    }
                };
                let Some(close) = super::matching(tokens, open, "{", "}") else {
                    return false;
                };
                let mut parentheses = 0usize;
                for at in open + 1..close {
                    match tokens[at].text.as_str() {
                        "(" => parentheses += 1,
                        ")" => parentheses = parentheses.saturating_sub(1),
                        text if text == name && parentheses == 0 => {
                            let previous = tokens[at - 1].text.as_str();
                            if previous == "{" || previous == "," {
                                return true;
                            }
                        }
                        _ => {}
                    }
                }
                index = close + 1;
                continue;
            }
            _ => {}
        }
        index += 1;
    }
    false
}

/// Whether `function`'s own Clang definition declares an object or
/// parameter named `name` (which would shadow a file-scope constant). `true`
/// when the definition cannot be read: the refusing answer.
pub(crate) fn shadowed_in(function: &str, name: &str) -> bool {
    fn declares(node: &Value, name: &str) -> bool {
        (matches!(kind(node), "VarDecl" | "ParmVarDecl") && node["name"].as_str() == Some(name))
            || children(node).iter().any(|child| declares(child, name))
    }
    crate::typed_access_path::definition_ast(function).is_none_or(|ast| declares(&ast, name))
}

/// The value Clang states for the file-scope enumeration constant `name` in
/// the analysed unit, through one appended probe.
fn clang_enumerator_value(name: &str) -> Option<i128> {
    let probe = format!("{PROBE_PREFIX}{name}");
    let declarations = crate::typed_access_path::appended_declarations(
        &format!("enum {probe} {{ {probe}_v = ({name}) }};"),
        PROBE_PREFIX,
    )?;
    let declaration = declarations
        .iter()
        .find(|node| kind(node) == "EnumDecl" && node["name"].as_str() == Some(probe.as_str()))?;
    let [constant] = children(declaration) else {
        return None;
    };
    let mut values = Vec::new();
    collect_values(constant, &mut values);
    match values.as_slice() {
        [value] => value.parse().ok(),
        _ => None,
    }
}

fn collect_values<'a>(node: &'a Value, values: &mut Vec<&'a str>) {
    if kind(node) == "ConstantExpr" {
        if let Some(value) = node["value"].as_str() {
            values.push(value);
        }
        return;
    }
    for child in children(node) {
        collect_values(child, values);
    }
}

/// The value of a constant `words` in the body of `function`: a decimal
/// constant, or an unshadowed file-scope enumeration constant Clang states.
pub(crate) fn constant_value(tokens: &[Token], function: &str, words: &[&str]) -> Option<i128> {
    if let Some(value) = literal_value(words) {
        return Some(value);
    }
    match strip(words) {
        [name]
            if is_identifier(name)
                && is_file_scope_enumerator(tokens, name)
                && !shadowed_in(function, name) =>
        {
            clang_enumerator_value(name)
        }
        _ => None,
    }
}

/// Hook 1: what the actual `words` at the guard parameter of a call in
/// `function` decides.
pub(crate) fn decide(
    tokens: &[Token],
    function: &str,
    words: Option<&[&str]>,
    guard: RetentionGuard,
) -> GuardDecision {
    guard.decide(words.and_then(|words| constant_value(tokens, function, words)))
}

/// The parameter names of the definition of `function` in `tokens`, in
/// order, and the index of its body's `{`.
pub(crate) fn definition(tokens: &[Token], function: &str) -> Option<(Vec<String>, usize)> {
    let (open, _) = super::find_function_body(tokens, function)?;
    let close_parameters = open.checked_sub(1)?;
    let mut depth = 0usize;
    let mut start = None;
    for at in (0..=close_parameters).rev() {
        match tokens[at].text.as_str() {
            ")" => depth += 1,
            "(" => {
                depth -= 1;
                if depth == 0 {
                    start = Some(at);
                    break;
                }
            }
            _ => {}
        }
    }
    let start = start?;
    // A group's name is its last identifier outside brackets; a group with a
    // parenthesised declarator (a function pointer) reads no name, which the
    // check refuses rather than misreads.
    let mut names = Vec::new();
    let mut group: Vec<&str> = Vec::new();
    let mut nested = 0usize;
    let mut parenthesised = false;
    let mut finish = |group: &mut Vec<&str>, parenthesised: &mut bool| {
        let name = if *parenthesised {
            String::new()
        } else {
            group.last().copied().unwrap_or_default().to_owned()
        };
        names.push(name);
        group.clear();
        *parenthesised = false;
    };
    for token in &tokens[start + 1..close_parameters] {
        match token.text.as_str() {
            "(" => {
                parenthesised = true;
                nested += 1;
            }
            "[" => nested += 1,
            ")" | "]" => nested = nested.saturating_sub(1),
            "," if nested == 0 => {
                finish(&mut group, &mut parenthesised);
                continue;
            }
            _ => {}
        }
        if nested == 0 && is_identifier(&token.text) {
            group.push(token.text.as_str());
        }
    }
    finish(&mut group, &mut parenthesised);
    Some((names, open))
}

/// Hook 2: the unit's environment with every guard its body does not
/// validate removed. `None` when nothing changes.
pub(crate) fn settle(tokens: &[Token], contracts: &ResourceContracts) -> Option<ResourceContracts> {
    let mut changed: Vec<(String, RetainingContract)> = Vec::new();
    for (name, declaration) in contracts.retaining() {
        if declaration
            .retentions
            .iter()
            .all(|retention| retention.guard.is_none())
        {
            continue;
        }
        let Some((parameters, open)) = definition(tokens, name) else {
            continue;
        };
        let Some(close) = super::matching(tokens, open, "{", "}") else {
            continue;
        };
        let body = &tokens[open..=close];
        let mut settled = declaration.clone();
        let mut any = false;
        for retention in &mut settled.retentions {
            let Some(guard) = retention.guard else {
                continue;
            };
            if !validated(tokens, name, body, &parameters, retention.retained, guard) {
                retention.guard = None;
                any = true;
            }
        }
        // Card VERDICT-CONTRACT-DEPENDENCIES (rule 13g): a guard the body
        // does not validate is dropped (the cautious union), not withdrawn,
        // and the declaration is then not validated here.
        if any {
            crate::contract_dependencies::failed(name, "12w");
            changed.push((name.clone(), settled));
        } else {
            crate::contract_dependencies::validated(name, "12w");
        }
    }
    if changed.is_empty() {
        return None;
    }
    let mut settled = contracts.clone();
    for (name, declaration) in changed {
        settled.insert_retaining(name, declaration);
    }
    Some(settled)
}

/// The words of `body` (its braces included).
fn words(body: &[Token]) -> Vec<&str> {
    body.iter().map(|token| token.text.as_str()).collect()
}

/// The guard's comparison read at `at` in `words`, as `X OP K` or, for
/// `==`, `K == X`; the constant is decided by `constant`.
fn comparison_at(
    words: &[&str],
    open: usize,
    close: usize,
    x: &str,
    guard: RetentionGuard,
    constant: &dyn Fn(&[&str]) -> Option<i128>,
) -> bool {
    let inside = &words[open + 1..close];
    let operator: &[&str] = match guard.relation.spelling() {
        "==" => &["=", "="],
        "<" => &["<"],
        ">" => &[">"],
        "<=" => &["<", "="],
        _ => &[">", "="],
    };
    let matches_constant = |words: &[&str]| constant(words) == Some(guard.constant);
    if inside.first() == Some(&x) && inside[1..].starts_with(operator) {
        let rest = &inside[1 + operator.len()..];
        // `X < K` must not be `X <= K` or `X << K`.
        let clean = !matches!(rest.first(), Some(&"=" | &"<" | &">"));
        return clean && matches_constant(rest);
    }
    if operator == ["=", "="] && inside.last() == Some(&x) {
        let head = &inside[..inside.len() - 1];
        if head.ends_with(&["=", "="]) {
            return matches_constant(&head[..head.len() - 2]);
        }
    }
    false
}

/// The index of the token closing the group opened at `open` in `words`.
fn close_of(words: &[&str], open: usize, opener: &str, closer: &str) -> Option<usize> {
    let mut depth = 0usize;
    for (index, word) in words.iter().enumerate().skip(open) {
        if *word == opener {
            depth += 1;
        } else if *word == closer {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some(index);
            }
        }
    }
    None
}

/// Whether the statement that ends just before `at` (at a `;`) is a jump:
/// `break;`, `continue;`, `return …;`.
fn ends_in_jump(words: &[&str], at: usize) -> bool {
    if at == 0 || words[at - 1] != ";" {
        return false;
    }
    let mut start = at - 1;
    while start > 0 && !matches!(words[start - 1], ";" | "{" | "}" | ":") {
        start -= 1;
    }
    matches!(words[start], "break" | "continue" | "return")
}

/// The guard regions of `words` (index ranges, exclusive of the braces and
/// labels that open them).
fn regions(
    words: &[&str],
    x: &str,
    guard: RetentionGuard,
    constant: &dyn Fn(&[&str]) -> Option<i128>,
) -> Vec<(usize, usize)> {
    let mut regions = Vec::new();
    for at in 0..words.len() {
        if words[at] == "if" && words.get(at + 1) == Some(&"(") {
            let Some(close) = close_of(words, at + 1, "(", ")") else {
                continue;
            };
            if words.get(close + 1) == Some(&"{")
                && comparison_at(words, at + 1, close, x, guard, constant)
                && let Some(end) = close_of(words, close + 1, "{", "}")
            {
                regions.push((close + 2, end));
            }
        }
        if words[at] == "switch"
            && words.get(at + 1) == Some(&"(")
            && words.get(at + 2) == Some(&x)
            && words.get(at + 3) == Some(&")")
            && words.get(at + 4) == Some(&"{")
        {
            let open = at + 4;
            let Some(end) = close_of(words, open, "{", "}") else {
                continue;
            };
            // The labels of this switch, at its own nesting level.
            // A nested switch's labels sit inside its own braces, so only
            // the labels at this switch's own level are collected.
            let mut labels = Vec::new();
            let mut depth = 0usize;
            for (index, word) in words.iter().enumerate().take(end).skip(open + 1) {
                match *word {
                    "{" => depth += 1,
                    "}" => depth = depth.saturating_sub(1),
                    "case" | "default" if depth == 0 => labels.push(index),
                    _ => {}
                }
            }
            for (position, &label) in labels.iter().enumerate() {
                if words[label] != "case" {
                    continue;
                }
                let Some(colon) = (label + 1..end).find(|&index| words[index] == ":") else {
                    continue;
                };
                let entered_alone = label - 1 == open || ends_in_jump(words, label);
                let next = labels.get(position + 1).copied().unwrap_or(end);
                if entered_alone
                    && constant(&words[label + 1..colon]).is_some_and(|value| guard.holds(value))
                {
                    regions.push((colon + 1, next));
                }
            }
        }
    }
    regions
}

/// Whether the occurrence of `p` at `at` cannot store it (see the module
/// documentation).
fn harmless_use(words: &[&str], at: usize) -> bool {
    let previous = |back: usize| at.checked_sub(back).map(|index| words[index]);
    let next = |forward: usize| words.get(at + forward).copied();
    let pair_after = (next(1), next(2));
    let pair_before = (previous(2), previous(1));
    if matches!(pair_after, (Some("=" | "!"), Some("="))) {
        return true;
    }
    if matches!(pair_before, (Some("=" | "!"), Some("="))) {
        return true;
    }
    if previous(1) == Some("!") {
        return true;
    }
    if previous(1) == Some("(")
        && next(1) == Some(")")
        && matches!(previous(2), Some("if" | "while"))
    {
        return true;
    }
    if matches!(pair_after, (Some("&"), Some("&")) | (Some("|"), Some("|")))
        || matches!(pair_before, (Some("&"), Some("&")) | (Some("|"), Some("|")))
        || next(1) == Some("?")
    {
        return true;
    }
    // A dereference: `*P`, or `*(T …)P`, the `*` unary (what stands before
    // it cannot end an operand) and not under `&`.
    let unary_star = |star: usize| {
        words[star] == "*"
            && star.checked_sub(1).is_some_and(|before| {
                matches!(
                    words[before],
                    "(" | ","
                        | "="
                        | ";"
                        | "{"
                        | "}"
                        | "return"
                        | "?"
                        | ":"
                        | "+"
                        | "-"
                        | "<"
                        | ">"
                        | "|"
                        | "^"
                        | "~"
                        | "!"
                )
            })
    };
    if at >= 1 && unary_star(at - 1) {
        return true;
    }
    if previous(1) == Some(")") {
        let mut depth = 0usize;
        for open in (0..at).rev() {
            match words[open] {
                ")" => depth += 1,
                "(" => {
                    depth -= 1;
                    if depth == 0 {
                        return open >= 1 && unary_star(open - 1);
                    }
                }
                _ => {}
            }
        }
    }
    false
}

/// Hook 2's check: is the guarded retention of the parameter at `retained`
/// validated by the body `body` (braces included) of `function`?
pub(crate) fn validated(
    tokens: &[Token],
    function: &str,
    body: &[Token],
    parameters: &[String],
    retained: usize,
    guard: RetentionGuard,
) -> bool {
    let (Some(p), Some(x)) = (parameters.get(retained), parameters.get(guard.parameter)) else {
        return false;
    };
    let (p, x) = (p.as_str(), x.as_str());
    if p.is_empty() || x.is_empty() || p == x {
        return false;
    }
    let words = words(body);
    if words.contains(&"goto") {
        return false;
    }
    // `X` is never written.
    for (at, word) in words.iter().enumerate() {
        if *word != x {
            continue;
        }
        let next = (words.get(at + 1).copied(), words.get(at + 2).copied());
        let previous = at.checked_sub(1).map(|index| words[index]);
        let assigned = match next {
            (Some("="), second) => second != Some("="),
            (Some("+" | "-"), Some(second)) if second == words[at + 1] => true,
            (Some("+" | "-" | "*" | "/" | "%" | "&" | "|" | "^"), Some("=")) => true,
            (Some("<"), Some("<")) | (Some(">"), Some(">")) => true,
            _ => false,
        };
        let incremented =
            at >= 2 && matches!((words[at - 2], words[at - 1]), ("+", "+") | ("-", "-"));
        if assigned || incremented || previous == Some("&") {
            return false;
        }
    }
    let constant = |spelled: &[&str]| constant_value(tokens, function, spelled);
    let regions = regions(&words, x, guard, &constant);
    for (at, word) in words.iter().enumerate() {
        if *word != p {
            continue;
        }
        let in_region = regions.iter().any(|(start, end)| *start <= at && at < *end);
        // A member or a scope that redeclares `p` is not the parameter; the
        // check refuses rather than distinguishes.
        if !in_region && !harmless_use(&words, at) {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use qcore_schemas::retained_by::GuardRelation;

    fn guard(relation: GuardRelation, constant: i128) -> RetentionGuard {
        RetentionGuard {
            parameter: 1,
            relation,
            constant,
            low: 0,
            high: u32::MAX.into(),
        }
    }

    fn check(body: &str, relation: GuardRelation, constant: i128) -> bool {
        let source = format!("int f(void *h, unsigned t, void *p) {body}");
        let tokens = super::super::tokenize(&source);
        let (parameters, open) = definition(&tokens, "f").expect("defined");
        let close = super::super::matching(&tokens, open, "{", "}").expect("closed");
        let mut guard = guard(relation, constant);
        guard.parameter = 1;
        validated(&tokens, "f", &tokens[open..=close], &parameters, 2, guard)
    }

    #[test]
    fn literal_spellings_are_decided_and_others_are_not() {
        assert_eq!(literal_value(&["2"]), Some(2));
        assert_eq!(literal_value(&["(", "2UL", ")"]), Some(2));
        assert_eq!(literal_value(&["-", "1"]), Some(-1));
        assert_eq!(literal_value(&["02"]), None);
        assert_eq!(literal_value(&["0x2"]), None);
        assert_eq!(literal_value(&["(", "int", ")", "2"]), None);
        assert_eq!(literal_value(&["1", "+", "1"]), None);
    }

    #[test]
    fn a_retention_inside_the_guarded_arm_only_validates() {
        let switch = "{ switch (t) { case 2: *(void **)h = p; break; default: break; } return 0; }";
        assert!(check(switch, GuardRelation::Eq, 2));
        let branch = "{ if (!p) return 1; if (t == 2) { *(void **)h = p; } return *(int *)p; }";
        assert!(check(branch, GuardRelation::Eq, 2));
    }

    #[test]
    fn a_body_that_can_keep_p_outside_the_guard_is_not_validated() {
        // Retained in the arm of another value.
        let other = "{ switch (t) { case 1: *(void **)h = p; break; } return 0; }";
        assert!(!check(other, GuardRelation::Eq, 2));
        // Fallthrough into the guarded arm.
        let fall = "{ switch (t) { case 1: case 2: *(void **)h = p; break; } return 0; }";
        assert!(!check(fall, GuardRelation::Eq, 2));
        // An alias taken before the guard.
        let alias = "{ void *q = p; if (t == 2) { *(void **)h = q; } return 0; }";
        assert!(!check(alias, GuardRelation::Eq, 2));
        // A call taking `p`.
        let call = "{ keep(h, p); return 0; }";
        assert!(!check(call, GuardRelation::Eq, 2));
        // The guard parameter rewritten.
        let written = "{ t = 2; if (t == 2) { *(void **)h = p; } return 0; }";
        assert!(!check(written, GuardRelation::Eq, 2));
        // `goto` anywhere.
        let jump = "{ if (t == 2) { *(void **)h = p; } goto out; out: return 0; }";
        assert!(!check(jump, GuardRelation::Eq, 2));
        // The address of an element of `*p`.
        let interior = "{ *(void **)h = &((char *)p)[1]; return 0; }";
        assert!(!check(interior, GuardRelation::Eq, 2));
        // The comparison must be the guard's.
        let looser = "{ if (t >= 2) { *(void **)h = p; } return 0; }";
        assert!(!check(looser, GuardRelation::Eq, 2));
    }

    #[test]
    fn the_cautious_reading_keeps_the_clause_unless_a_failure_is_decided() {
        let guard = guard(GuardRelation::Eq, 2);
        assert_eq!(guard.decide(Some(2)), GuardDecision::Holds);
        assert_eq!(guard.decide(Some(1)), GuardDecision::Fails);
        assert_eq!(guard.decide(None), GuardDecision::Undecided);
        // Out of the parameter's range: C converts it, so not decided.
        assert_eq!(guard.decide(Some(-1)), GuardDecision::Undecided);
        assert!(RetentionGuard::in_force(GuardDecision::Holds));
        assert!(RetentionGuard::in_force(GuardDecision::Undecided));
        assert!(!RetentionGuard::in_force(GuardDecision::Fails));
    }
}
