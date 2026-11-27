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
//! Card **FB-CHUNK-LIFECYCLE** (gateway rule 90): an integer formal of a
//! `static` function that every call in the unit fixes to a constant is read
//! at that constant where the body tests it.
//!
//! # The row this decides
//!
//! Fluent Bit's `sb_remove_chunk_from_segregated_backlog(target_chunk,
//! backlog, destroy)` (`plugins/in_storage_backlog/sb.c:214-246`) unlinks the
//! matching wrapper (`mk_list_del`, declared `resource_unpublished`) and frees
//! it only `if (destroy)`. Read over an unconstrained `destroy`, the false
//! arm exits with the unlinked wrapper live, and the row was a provisional
//! `Unsafe` (P0-SB-REMOVE-CHUNK-DESTROY-FLAG). The function is `static`, its
//! address is never taken, and its one call passes `FLB_TRUE` (`1` after
//! preprocessing): no invocation in the program supplies the false arm.
//!
//! # The reading
//!
//! For a definition `function` in the unit's token stream, a formal `NAME`
//! at parameter index `i` is **fixed** to the truth value `t` when all of:
//!
//! 1. the definition has internal linkage (`static`, no `extern`, read off its
//!    own specifiers as [`super::callee_scope::has_internal_linkage`]), so by
//!    C11 6.2.2p3 every use of the identifier in this unit denotes this body
//!    and no use outside it can;
//! 2. every occurrence of the identifier `function` in the stream is the
//!    definition's own declarator, a file-scope prototype
//!    (`… function ( … ) ;` outside every function body), or a
//!    **direct call** (`function (` in a call position): any other occurrence
//!    (its address taken, a designator, a member or label of the same
//!    spelling, a parenthesised name) refuses, so the direct calls are every
//!    invocation of the body;
//! 3. there is at least one call, each has exactly the parameter count of
//!    actuals, and each actual at `i` is a decimal integer constant of at most
//!    four digits (no sign, suffix, octal or cast; matched enclosing
//!    parentheses allowed), all zero (`t` false) or all nonzero (`t` true);
//! 4. the formal is declared with only `const`, `int`, `signed`, `unsigned`
//!    or `_Bool` before its name, so the constant converts to a value of the
//!    same truth (C11 6.3.1.2, 6.3.1.3p1: at most 9999 is representable in
//!    every such type);
//! 5. every occurrence of `NAME` in the body is exactly `if ( NAME )` or
//!    `if ( ! NAME )`: the formal is never written, never has its address
//!    taken and is never shadowed, so its value is the actual's throughout;
//! 6. the built CFG has exactly as many two-way `BranchOnNull` nodes testing
//!    `NAME` as the body spells such tests, none a loop header and none a
//!    cell-protocol test.
//!
//! Then each such branch is replaced by a `NoOp` whose one successor is the
//! arm `t` selects (the selection `contextual_cfg` makes for a recursion
//! control, rule 11x). No node is removed: a node the selection leaves
//! unreachable stays in the graph, receives no state and so reports nothing,
//! and the certificate firewall still sees its instruction. Every other
//! instruction, edge, fact and diagnostic is unchanged, and anything short of
//! all six conditions leaves the CFG exactly as built.
//!
//! Applied inside `build_function_cfg_with`, over the stream the build was
//! handed, so W4 derivation, the checked-specification settlement, the row's
//! own analysis and certificate rederivation all read the same selection.
//!
//! # What the verdict then says
//!
//! The row is decided over the invocations the unit contains, which (1)-(2)
//! make every invocation the program can perform. The reading carries the
//! named premise [`PREMISE`] into every row whose derivation reaches the
//! function, so the scope is in the verdict's assumptions. The false arm is
//! not made `Safe`: under a zero actual it is the selected arm and its leak
//! stands; with actuals of both truths, a non-constant actual, an escaped
//! address or a written formal, nothing is selected and the old reading
//! (and its provisional `Unsafe`) stands.
//!
//! Premise stated, not proved: the token stream is the whole preprocessed
//! translation unit (trust boundary 28). `QCore.StaticConstantFormal`.

use super::{LegacyCfg, LegacyInstruction, Token, find_function_body, matching};

/// The rule id carried with the premise.
pub(crate) const RULE: &str = "90";

/// The named premise a row decided under a fixed formal lists.
pub(crate) const PREMISE: &str = "static-call-sites-closed";

/// One formal every call in the unit fixes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FixedFormal {
    pub(crate) name: String,
    /// Whether every actual at this formal is nonzero.
    pub(crate) truth: bool,
    /// How many `if ( NAME )` / `if ( ! NAME )` tests the body spells.
    tests: usize,
}

/// The type words a fixed formal may carry before its name.
const TYPE_WORDS: &[&str] = &["const", "int", "signed", "unsigned", "_Bool"];

/// Keywords after which `function (` is a call, not a declarator.
const CALL_KEYWORDS: &[&str] = &["return", "else", "do", "case"];

fn is_identifier(text: &str) -> bool {
    let mut chars = text.chars();
    chars
        .next()
        .is_some_and(|c| c == '_' || c.is_ascii_alphabetic())
        && chars.all(|c| c == '_' || c.is_ascii_alphanumeric())
}

/// The truth of a decimal constant actual, or `None` when it is not one.
fn constant_truth(actual: &[&str]) -> Option<bool> {
    let mut tokens = actual;
    while tokens.len() > 2 && tokens[0] == "(" && closes_at_end(tokens) {
        tokens = &tokens[1..tokens.len() - 1];
    }
    let [token] = tokens else {
        return None;
    };
    let digits_only =
        !token.is_empty() && token.len() <= 4 && token.bytes().all(|b| b.is_ascii_digit());
    // `0` alone is zero; any other leading zero is octal, which is not read.
    if !digits_only || (token.len() > 1 && token.starts_with('0')) {
        return None;
    }
    Some(*token != "0")
}

/// Whether the `(` at index 0 is matched by the last token.
fn closes_at_end(tokens: &[&str]) -> bool {
    let mut depth = 0usize;
    for (index, token) in tokens.iter().enumerate() {
        match *token {
            "(" => depth += 1,
            ")" => {
                depth = match depth.checked_sub(1) {
                    Some(depth) => depth,
                    None => return false,
                };
                if depth == 0 {
                    return index + 1 == tokens.len();
                }
            }
            _ => {}
        }
    }
    false
}

/// Split a parenthesised run (without its parentheses) at top-level commas.
fn split_commas<'a>(run: &[&'a str]) -> Option<Vec<Vec<&'a str>>> {
    let mut parts = vec![Vec::new()];
    let mut depth = 0usize;
    for token in run {
        match *token {
            "(" | "[" | "{" => depth += 1,
            ")" | "]" | "}" => depth = depth.checked_sub(1)?,
            "," if depth == 0 => {
                parts.push(Vec::new());
                continue;
            }
            _ => {}
        }
        parts.last_mut()?.push(*token);
    }
    if depth != 0 {
        return None;
    }
    if parts.len() == 1 && parts[0].is_empty() {
        parts.clear();
    }
    Some(parts)
}

/// The index of the `(` matching the `)` at `close`, searching backwards.
fn opening(texts: &[&str], close: usize) -> Option<usize> {
    let mut depth = 0usize;
    for index in (0..=close).rev() {
        match texts[index] {
            ")" => depth += 1,
            "(" => {
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

/// The formal name a parameter declares when (4) holds.
fn integer_formal(parameter: &[&str]) -> Option<String> {
    let (name, words) = parameter.split_last()?;
    let admitted = !words.is_empty()
        && words.iter().all(|word| TYPE_WORDS.contains(word))
        && words.iter().any(|word| *word != "const");
    (admitted && is_identifier(name) && !TYPE_WORDS.contains(name)).then(|| (*name).to_owned())
}

/// How many times the body spells `NAME` only in (5)'s two tests, or `None`
/// when any occurrence is another use.
fn branch_tests(body: &[&str], name: &str) -> Option<usize> {
    let mut tests = 0;
    for (index, token) in body.iter().enumerate() {
        if *token != name {
            continue;
        }
        let at = |offset: usize| index.checked_sub(offset).map(|i| body[i]);
        let closed = body.get(index + 1) == Some(&")");
        let plain = at(1) == Some("(") && at(2) == Some("if");
        let negated = at(1) == Some("!") && at(2) == Some("(") && at(3) == Some("if");
        if !(closed && (plain || negated)) {
            return None;
        }
        tests += 1;
    }
    Some(tests)
}

/// For each token, whether it lies inside a function body: a `{` at file
/// scope that follows `)` (a parameter list or an attribute) or `;` (an
/// old-style parameter declaration) opens one, through its matching `}`.
/// Any other file-scope brace (a record body, an initializer) opens none.
fn function_bodies(texts: &[&str]) -> Option<Vec<bool>> {
    let mut inside = vec![false; texts.len()];
    let mut index = 0;
    let mut depth = 0usize;
    while index < texts.len() {
        match texts[index] {
            "{" if depth == 0
                && index
                    .checked_sub(1)
                    .is_some_and(|previous| matches!(texts[previous], ")" | ";")) =>
            {
                let mut level = 0usize;
                let mut end = None;
                for (offset, text) in texts[index..].iter().enumerate() {
                    match *text {
                        "{" => level += 1,
                        "}" => {
                            level = level.checked_sub(1)?;
                            if level == 0 {
                                end = Some(index + offset);
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                let end = end?;
                inside[index..=end].iter_mut().for_each(|flag| *flag = true);
                index = end + 1;
                continue;
            }
            "{" => depth += 1,
            "}" => depth = depth.checked_sub(1)?,
            _ => {}
        }
        index += 1;
    }
    Some(inside)
}

/// The formals of `function` that (1)-(5) fix, read off `tokens`.
pub(crate) fn fixed_formals(tokens: &[Token], function: &str) -> Vec<FixedFormal> {
    fixed_formals_inner(tokens, function).unwrap_or_default()
}

fn fixed_formals_inner(tokens: &[Token], function: &str) -> Option<Vec<FixedFormal>> {
    // (1)
    if !super::callee_scope::has_internal_linkage(tokens, function) {
        return None;
    }
    let texts = tokens
        .iter()
        .map(|token| token.text.as_str())
        .collect::<Vec<_>>();
    let (body_start, _) = find_function_body(tokens, function)?;
    let body_end = matching(tokens, body_start, "{", "}")?;
    let close = body_start.checked_sub(1)?;
    if texts[close] != ")" {
        return None;
    }
    let open = opening(&texts, close)?;
    let definition = open.checked_sub(1)?;
    if texts[definition] != function {
        return None;
    }
    let parameters = split_commas(&texts[open + 1..close])?;
    // (4) and (5): the candidate formals.
    let body = &texts[body_start + 1..body_end];
    let mut candidates = parameters
        .iter()
        .enumerate()
        .filter_map(|(index, parameter)| {
            let name = integer_formal(parameter)?;
            let tests = branch_tests(body, &name)?;
            (tests > 0).then_some((index, name, tests, None::<bool>))
        })
        .collect::<Vec<_>>();
    if candidates.is_empty() {
        return None;
    }
    // (2) and (3): every occurrence of the function's identifier.
    let in_body = function_bodies(&texts)?;
    let mut calls = 0usize;
    for (index, text) in texts.iter().enumerate() {
        if *text != function || index == definition {
            continue;
        }
        if texts.get(index + 1) != Some(&"(") {
            return None;
        }
        let end = matching(tokens, index + 1, "(", ")")?;
        let previous = index.checked_sub(1).map(|i| texts[i]);
        let declarator = match previous {
            None => true,
            Some("." | "->" | "&") => return None,
            Some(word) if CALL_KEYWORDS.contains(&word) => false,
            Some(word) => word == "*" || is_identifier(word),
        };
        if declarator {
            // A file-scope prototype only; any other declarator-shaped use,
            // a block-scope one included, refuses.
            if in_body[index] || texts.get(end + 1) != Some(&";") {
                return None;
            }
            continue;
        }
        let actuals = split_commas(&texts[index + 2..end])?;
        if actuals.len() != parameters.len() {
            return None;
        }
        for (position, _, _, truth) in &mut candidates {
            let value = constant_truth(&actuals[*position])?;
            match truth {
                Some(seen) if *seen != value => return None,
                _ => *truth = Some(value),
            }
        }
        calls += 1;
    }
    if calls == 0 {
        return None;
    }
    Some(
        candidates
            .into_iter()
            .filter_map(|(_, name, tests, truth)| {
                Some(FixedFormal {
                    name,
                    truth: truth?,
                    tests,
                })
            })
            .collect(),
    )
}

/// Apply (6) and the selection to `cfg`, built for `function` from `tokens`.
/// The identity unless every condition holds for some formal.
pub(crate) fn select(tokens: &[Token], function: &str, mut cfg: LegacyCfg) -> LegacyCfg {
    let fixed = fixed_formals(tokens, function);
    let mut selected = false;
    for formal in &fixed {
        let branches = cfg
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| {
                matches!(&node.instruction, LegacyInstruction::BranchOnNull { source, .. }
                    if *source == formal.name)
            })
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        let admissible = branches.len() == formal.tests
            && branches.iter().all(|&index| {
                let node = &cfg.nodes[index];
                node.successors.len() == 2
                    && node.loop_exit_edge.is_none()
                    && !cfg.cell_tests.contains_key(&index)
            });
        if !admissible {
            continue;
        }
        for index in branches {
            let node = &mut cfg.nodes[index];
            let LegacyInstruction::BranchOnNull { true_is_null, .. } = node.instruction else {
                continue;
            };
            // Edge 0 assumes the tested value null exactly when
            // `true_is_null`, so the zero arm is edge `!true_is_null`.
            let zero = !formal.truth;
            let chosen = node.successors[usize::from(true_is_null != zero)];
            node.instruction = LegacyInstruction::NoOp;
            node.successors = vec![chosen];
            selected = true;
        }
    }
    if selected {
        crate::arm_obligations::carry_declaration_premise(function, RULE, PREMISE);
    }
    cfg
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_are_read_structurally() {
        assert_eq!(constant_truth(&["1"]), Some(true));
        assert_eq!(constant_truth(&["(", "1", ")"]), Some(true));
        assert_eq!(constant_truth(&["0"]), Some(false));
        assert_eq!(constant_truth(&["9999"]), Some(true));
        for refused in [
            &["10000"][..],
            &["01"],
            &["-", "1"],
            &["1u"],
            &["0x1"],
            &["x"],
            &["(", "int", ")", "1"],
            &["(", "1", ")", "+", "(", "0", ")"],
            &[],
        ] {
            assert_eq!(constant_truth(refused), None, "{refused:?}");
        }
    }

    #[test]
    fn formals_and_tests_are_read_exactly() {
        assert_eq!(
            integer_formal(&["int", "destroy"]).as_deref(),
            Some("destroy")
        );
        assert_eq!(
            integer_formal(&["const", "unsigned", "int", "d"]).as_deref(),
            Some("d")
        );
        assert_eq!(integer_formal(&["_Bool", "d"]).as_deref(), Some("d"));
        for refused in [
            &["const", "d"][..],
            &["volatile", "int", "d"],
            &["char", "d"],
            &["long", "d"],
            &["flag_t", "d"],
            &["int", "*", "d"],
            &["d"],
        ] {
            assert_eq!(integer_formal(refused), None, "{refused:?}");
        }
        assert_eq!(branch_tests(&["if", "(", "d", ")", "x", ";"], "d"), Some(1));
        assert_eq!(
            branch_tests(&["if", "(", "!", "d", ")", "x", ";"], "d"),
            Some(1)
        );
        assert_eq!(branch_tests(&["x", ";"], "d"), Some(0));
        for refused in [
            &["while", "(", "d", ")", ";"][..],
            &["d", "=", "0", ";"],
            &["if", "(", "d", "&&", "x", ")", ";"],
            &["if", "(", "p", "->", "d", ")", ";"],
            &["g", "(", "&", "d", ")", ";"],
        ] {
            assert_eq!(branch_tests(refused, "d"), None, "{refused:?}");
        }
    }
}
