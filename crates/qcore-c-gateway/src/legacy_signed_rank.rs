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

//! Card **CG27-LOOPS-RECURSION-BUDGETS** (gateway rule 11x): ranked recursion
//! over a *signed* integer control with a threshold guard and several
//! decreasing ranks.
//!
//! The zero/nonzero route ([`super::prepare`]) partitions an explicit integer
//! formal into `0` and `!= 0`, and admits a recursive `control - 1` actual only
//! for unsigned components. For a signed control that partition is not a rank:
//! `n - 1` from a negative `n` never reaches zero. This route partitions the
//! control at a literal threshold `c` instead:
//!
//! * the **base** context is `n <= c` (rank 0), checked with every summary of
//!   the component absent, so a recursive call reachable there refuses;
//! * the **step** context is `n > c`, rank `n - c`, checked with the
//!   component's base contracts in force. Every recursive actual for the
//!   control is `n - k` (a decimal literal `1 <= k <= MAX_STEP`) or the literal
//!   `0`, each of rank strictly below `n - c`: `max(n - k - c, 0) < n - c` and
//!   `max(0 - c, 0) = 0`.
//!
//! Several actuals with different `k` (multi-rank recursion, `f(n - 1)` beside
//! `f(n - 2)`) are admitted together. The hypothesis a step call reads is the
//! summary at a strictly smaller rank, never at its own: strong induction over
//! the rank, exactly as `QCore.UnsignedCountdown.publish_by_mutual_rank_induction`
//! states it and `QCore.LoopInvariants.SignedThreshold` instantiates it.
//!
//! **Representation.** Every member's control has the *same* type, one of
//! `int`, `long` or `long long` (in their `signed`/`int` spellings), so a
//! recursive actual is never converted: `n - k` is computed in that type and
//! cannot overflow because `n > c >= 0` and `k <= MAX_STEP`. `char`, `short`
//! and `_Bool` controls (promoted, then converted back with an
//! implementation-defined result) and mixed widths refuse.
//!
//! **Guards.** Every `if` condition naming the control must be exactly one sign
//! guard against the component's threshold: `n <= c`, `n < c + 1`, `n > c`,
//! `n >= c + 1`, or one of those mirrored (`c >= n`, …), with `0 <= c <=
//! MAX_THRESHOLD`. A null-shaped test (`!n`, `n == 0`), a different threshold,
//! a compound condition, or the control anywhere else refuses the component.
//! The guard is respelled as the null test whose truth agrees with it in each
//! context, so the existing context selection decides it; the certificate keeps
//! the original source and re-runs this extraction.
//!
//! This route is consulted only when [`super::prepare`] refuses, which it does
//! for every component with a sign guard on its control, so it never replaces
//! or weakens a contract the zero/nonzero route publishes.

use super::*;

/// The largest literal step `k` in an admitted `n - k` actual.
pub(crate) const MAX_STEP: u32 = 1024;
/// The largest literal threshold `c` of an admitted sign guard.
pub(crate) const MAX_THRESHOLD: u32 = 1024;

/// The signed control spellings this route admits: `int`, `long` and
/// `long long`, never narrower than `int`.
fn signed_wide(words: &[String]) -> bool {
    matches!(
        words
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .as_slice(),
        ["int"]
            | ["signed"]
            | ["signed", "int"]
            | ["long"]
            | ["long", "int"]
            | ["signed", "long"]
            | ["signed", "long", "int"]
            | ["long", "long"]
            | ["long", "long", "int"]
            | ["signed", "long", "long"]
            | ["signed", "long", "long", "int"]
    )
}

fn decimal(text: &str) -> Option<u32> {
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    if text.len() > 1 && text.starts_with('0') {
        // An octal spelling is not read here.
        return None;
    }
    text.parse().ok()
}

/// One sign guard: its threshold and whether it is true in the base context.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SignGuard {
    pub(crate) threshold: u32,
    pub(crate) true_in_base: bool,
}

/// Read `condition` (the tokens between an `if`'s parentheses) as one sign
/// guard on `control`, or `None`.
pub(crate) fn sign_guard(condition: &[&str], control: &str) -> Option<SignGuard> {
    let mut texts = condition;
    while texts.first() == Some(&"(") && texts.last() == Some(&")") {
        // Strip only parentheses that enclose the whole condition.
        let mut depth = 0usize;
        let mut whole = true;
        for (index, text) in texts.iter().enumerate() {
            match *text {
                "(" => depth += 1,
                ")" => {
                    depth = depth.checked_sub(1)?;
                    if depth == 0 && index + 1 != texts.len() {
                        whole = false;
                    }
                }
                _ => {}
            }
        }
        if !whole {
            break;
        }
        texts = &texts[1..texts.len() - 1];
    }
    // Fuse a split two-character comparison (`<` `=`), as the token stream
    // may spell it either way.
    let mut fused: Vec<String> = Vec::new();
    for text in texts {
        if *text == "="
            && fused
                .last()
                .is_some_and(|last| matches!(last.as_str(), "<" | ">"))
        {
            fused.last_mut()?.push('=');
        } else {
            fused.push((*text).to_owned());
        }
    }
    let [left, operator, right] = fused.as_slice() else {
        return None;
    };
    // Normalise to `control OP literal`.
    let (operator, literal) = if left == control {
        (operator.as_str(), right.as_str())
    } else if right == control {
        let mirrored = match operator.as_str() {
            "<" => ">",
            "<=" => ">=",
            ">" => "<",
            ">=" => "<=",
            _ => return None,
        };
        (mirrored, left.as_str())
    } else {
        return None;
    };
    let value = decimal(literal)?;
    let guard = match operator {
        "<=" => SignGuard {
            threshold: value,
            true_in_base: true,
        },
        "<" => SignGuard {
            threshold: value.checked_sub(1)?,
            true_in_base: true,
        },
        ">" => SignGuard {
            threshold: value,
            true_in_base: false,
        },
        ">=" => SignGuard {
            threshold: value.checked_sub(1)?,
            true_in_base: false,
        },
        _ => return None,
    };
    (guard.threshold <= MAX_THRESHOLD).then_some(guard)
}

/// Read a recursive call's control actual: `Some(Some(k))` for `CONTROL - k`,
/// `Some(None)` for the literal `0`, `None` for anything else.
pub(crate) fn step_actual(actual: &[&str], control: &str) -> Option<Option<u32>> {
    match actual {
        [name, "-", step] if *name == control => {
            let step = decimal(step)?;
            (1..=MAX_STEP).contains(&step).then_some(Some(step))
        }
        ["0"] => Some(None),
        _ => None,
    }
}

/// The signed threshold descriptors of `component`, or `None` when any member
/// is outside this route's grammar.
pub(crate) fn prepare(
    tokens: &[Token],
    component: &[String],
) -> Option<BTreeMap<String, ContextFunction>> {
    let mut headers = BTreeMap::new();
    for function in component {
        headers.insert(function.clone(), member_header(tokens, function)?);
    }
    // One control type for the whole component, signed and at least `int`
    // wide, so a recursive actual is never converted.
    let first = headers.values().next()?;
    let control_type = first.types[first.control_index].clone();
    if !signed_wide(&control_type)
        || headers
            .values()
            .any(|header| header.types[header.control_index] != control_type)
    {
        return None;
    }
    let mut threshold = None;
    let mut result = BTreeMap::new();
    for function in component {
        let header = &headers[function];
        let mut allowed = BTreeSet::new();
        let mut guards = BTreeMap::new();
        for i in header.body + 1..header.end {
            if tokens[i].text != "if" {
                continue;
            }
            if tokens.get(i + 1)?.text != "(" {
                return None;
            }
            let last = matching(tokens, i + 1, "(", ")")?;
            let condition = tokens[i + 2..last]
                .iter()
                .map(|token| token.text.as_str())
                .collect::<Vec<_>>();
            if !condition.contains(&header.control.as_str()) {
                continue;
            }
            let guard = sign_guard(&condition, &header.control)?;
            if *threshold.get_or_insert(guard.threshold) != guard.threshold {
                return None;
            }
            guards.insert(i + 2, (last - (i + 2), guard.true_in_base));
            allowed.extend(i + 2..last);
        }
        let mut countdowns = BTreeSet::new();
        let mut dependencies = BTreeSet::new();
        for i in header.body + 1..header.end {
            let Some(callee) = headers.get(&tokens[i].text) else {
                continue;
            };
            if tokens.get(i + 1)?.text != "(" {
                return None;
            }
            let last = matching(tokens, i + 1, "(", ")")?;
            if tokens.get(last + 1)?.text != ";" {
                return None;
            }
            let actuals = tokens[i + 2..last]
                .split(|t| t.text == ",")
                .collect::<Vec<_>>();
            if actuals.len() != callee.parameters.len() {
                return None;
            }
            let counter = actuals[callee.control_index]
                .iter()
                .map(|token| token.text.as_str())
                .collect::<Vec<_>>();
            if step_actual(&counter, &header.control)?.is_some() {
                let start = i
                    + 2
                    + actuals[..callee.control_index]
                        .iter()
                        .map(|a| a.len() + 1)
                        .sum::<usize>();
                countdowns.insert(start);
                allowed.insert(start);
            }
            if actuals.iter().enumerate().any(|(index, a)| {
                index != callee.control_index
                    && (a.len() != 1 || !(is_identifier(&a[0].text) || a[0].text == "0"))
            }) {
                return None;
            }
            // Only standalone calls or the exact RHS of an assignment, as on
            // the zero/nonzero route.
            if !matches!(
                tokens[i - 1].text.as_str(),
                "{" | "}" | ";" | "=" | ")" | "else"
            ) {
                return None;
            }
            dependencies.insert(tokens[i].text.clone());
        }
        // The control is read only by its guards and its step actuals: it is
        // never assigned, addressed, shadowed or passed anywhere else.
        if (header.body + 1..header.end)
            .any(|i| tokens[i].text == header.control && !allowed.contains(&i))
        {
            return None;
        }
        result.insert(
            function.clone(),
            ContextFunction {
                parameters: header.parameters.clone(),
                control_index: header.control_index,
                control: header.control.clone(),
                unsigned: false,
                allowed_control: allowed,
                countdowns,
                guards,
                dependencies,
            },
        );
    }
    // A component with no sign guard at all has no base context to read.
    threshold?;
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_guards_read_every_spelling_and_refuse_the_rest() {
        let guard = |text: &str| {
            let texts = text.split_whitespace().collect::<Vec<_>>();
            sign_guard(&texts, "n")
        };
        let base = |threshold| {
            Some(SignGuard {
                threshold,
                true_in_base: true,
            })
        };
        let step = |threshold| {
            Some(SignGuard {
                threshold,
                true_in_base: false,
            })
        };
        assert_eq!(guard("n < = 0"), base(0));
        assert_eq!(guard("n <= 1"), base(1));
        assert_eq!(guard("n < 1"), base(0));
        assert_eq!(guard("( n > 0 )"), step(0));
        assert_eq!(guard("n > = 2"), step(1));
        assert_eq!(guard("0 > = n"), base(0));
        assert_eq!(guard("1 > n"), base(0));
        assert_eq!(guard("0 < n"), step(0));
        assert_eq!(guard("1 < = n"), step(0));
        for refused in [
            "n == 0",
            "! n",
            "n",
            "n < 0",
            "n >= 0",
            "n <= -1",
            "n <= 01",
            "n <= 1025",
            "n <= m",
            "n + 1 <= 0",
            "( n ) <= 0",
            "n <= 0 && p",
            "( n <= 0 ) || ( n > 7 )",
        ] {
            assert_eq!(guard(refused), None, "{refused}");
        }
    }

    #[test]
    fn step_actuals_are_literal_decrements_or_zero() {
        let actual = |text: &str| {
            let texts = text.split_whitespace().collect::<Vec<_>>();
            step_actual(&texts, "n")
        };
        assert_eq!(actual("n - 1"), Some(Some(1)));
        assert_eq!(actual("n - 2"), Some(Some(2)));
        assert_eq!(actual("n - 1024"), Some(Some(1024)));
        assert_eq!(actual("0"), Some(None));
        for refused in [
            "n", "n - 0", "n + 1", "n - 1025", "1 - n", "n - m", "n -- ", "7", "-1",
        ] {
            assert_eq!(actual(refused), None, "{refused}");
        }
    }

    /// The bracketed tuples `(…)` of the Lean definition `name`, read up to the
    /// theorem that follows it.
    fn lean_tuples(lean: &str, name: &str) -> Vec<String> {
        let body = lean
            .split(&format!("def {name}"))
            .nth(1)
            .and_then(|rest| rest.split("theorem").next())
            .unwrap_or_else(|| panic!("`{name}` is in the Lean model"));
        let body = &body[body.find('[').expect("a list")..];
        let mut tuples = Vec::new();
        let mut depth = 0usize;
        let mut start = 0;
        for (index, character) in body.char_indices() {
            match character {
                '(' => {
                    if depth == 0 {
                        start = index + 1;
                    }
                    depth += 1;
                }
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        tuples.push(body[start..index].to_owned());
                    }
                }
                ']' if depth == 0 => break,
                _ => {}
            }
        }
        tuples
    }

    /// Card CG27's binding: `QCore.LoopInvariants.SignedThreshold.guardVectors`
    /// and `stepVectors` (proved by `guard_vectors_hold` and
    /// `step_vectors_hold` against `readGuard` and `Actual.admitted`) are the
    /// readings this module makes.
    #[test]
    fn lean_vectors_match_the_rust_reader() {
        let lean = include_str!("../../../proof/QCore/LoopInvariants.lean");
        let guards = lean_tuples(lean, "guardVectors");
        assert_eq!(guards.len(), 12);
        for tuple in guards {
            let (comparison, rest) = tuple.split_once(',').expect("a comparison");
            let (literal, reading) = rest.split_once(',').expect("a literal");
            let operator = match comparison.trim() {
                ".le" => "<=",
                ".lt" => "<",
                ".gt" => ">",
                ".ge" => ">=",
                other => panic!("unknown comparison {other}"),
            };
            let literal = literal.trim();
            let expected = match reading.trim() {
                "none" => None,
                some => {
                    let inner = some
                        .trim_start_matches("some")
                        .trim()
                        .trim_start_matches('(')
                        .trim_end_matches(')');
                    let (threshold, base) = inner.split_once(',').expect("a pair");
                    Some(SignGuard {
                        threshold: threshold.trim().parse().expect("a threshold"),
                        true_in_base: base.trim() == "true",
                    })
                }
            };
            assert_eq!(
                sign_guard(&["n", operator, literal], "n"),
                expected,
                "{tuple}"
            );
            // The split spelling the token stream may carry reads the same.
            if operator.len() == 2 {
                assert_eq!(
                    sign_guard(&["n", &operator[..1], "=", literal], "n"),
                    expected,
                    "{tuple}"
                );
            }
        }
        let steps = lean_tuples(lean, "stepVectors");
        assert_eq!(steps.len(), 5);
        for tuple in steps {
            let (step, admitted) = tuple.split_once(',').expect("a pair");
            assert_eq!(
                step_actual(&["n", "-", step.trim()], "n").is_some(),
                admitted.trim() == "true",
                "{tuple}"
            );
        }
    }

    #[test]
    fn only_int_or_wider_signed_controls_are_read() {
        let words = |text: &str| {
            text.split_whitespace()
                .map(str::to_owned)
                .collect::<Vec<_>>()
        };
        for admitted in ["int", "signed", "long", "long long int", "signed long"] {
            assert!(signed_wide(&words(admitted)), "{admitted}");
        }
        for refused in [
            "unsigned",
            "char",
            "signed char",
            "short",
            "short int",
            "_Bool",
        ] {
            assert!(!signed_wide(&words(refused)), "{refused}");
        }
    }
}
