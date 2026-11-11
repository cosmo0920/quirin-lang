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

//! Shared lexical boundary for a modeled call in a C controlling expression.
//!
//! A custody effect may be projected out of a condition only when C evaluates
//! it on every visit to that condition.  This module decides that deliberately
//! narrow shape; the caller still has to validate the isolated call through
//! the ordinary [`crate::legacy_c`] statement classifier and validate the
//! residual expression independently.

/// The half-open token span of the only ordinary call in a controlling
/// expression, when that call is evaluated unconditionally.
///
/// Short-circuit, conditional and assignment expressions stay refused.  C
/// type operators stay refused too: an ordinary call nested in `sizeof` need
/// not execute.  The restriction is conservative; it can withhold a modeled
/// effect, but can never apply one on a path where C skipped it.
pub(crate) fn unconditional_call_span(tokens: &[&str]) -> Option<(usize, usize)> {
    if tokens.iter().any(|token| matches!(*token, "?" | ":"))
        || tokens
            .windows(2)
            .any(|pair| matches!(pair, ["&", "&"] | ["|", "|"] | ["+", "+"] | ["-", "-"]))
        || contains_assignment(tokens)
    {
        return None;
    }

    let mut found = None;
    for (index, pair) in tokens.windows(2).enumerate() {
        if !is_identifier(pair[0]) || pair[1] != "(" {
            continue;
        }
        if crate::legacy_c::TYPE_OPERATORS.contains(&pair[0]) {
            // Refuse the whole expression, rather than risk selecting a call
            // whose evaluation the type operator suppresses.
            return None;
        }
        let end = matching_close(tokens, index + 1)? + 1;
        if found.replace((index, end)).is_some() {
            return None;
        }
    }
    found
}

fn matching_close(tokens: &[&str], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (index, token) in tokens.iter().enumerate().skip(open) {
        match *token {
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

fn contains_assignment(tokens: &[&str]) -> bool {
    tokens.iter().enumerate().any(|(index, token)| {
        if *token != "=" {
            return false;
        }
        let previous = index.checked_sub(1).and_then(|i| tokens.get(i)).copied();
        let next = tokens.get(index + 1).copied();
        next != Some("=")
            && (!matches!(previous, Some("=" | "!" | "<" | ">"))
                || (matches!(previous, Some("<" | ">"))
                    && index.checked_sub(2).and_then(|i| tokens.get(i)).copied() == previous))
    })
}

fn is_identifier(text: &str) -> bool {
    let mut chars = text.chars();
    chars
        .next()
        .is_some_and(|first| first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

#[cfg(test)]
mod tests {
    use super::unconditional_call_span;

    #[test]
    fn finds_one_unconditional_call() {
        assert_eq!(
            unconditional_call_span(&["(", "release", "(", "p", ")", ",", "1", ")"]),
            Some((1, 5))
        );
        assert_eq!(
            unconditional_call_span(&["probe", "(", "p", ")", "=", "=", "0"]),
            Some((0, 4))
        );
    }

    #[test]
    fn refuses_conditional_or_out_of_scope_evaluation() {
        for tokens in [
            &["ready", "&", "&", "release", "(", "p", ")"][..],
            &["ready", "?", "release", "(", "p", ")", ":", "0"][..],
            &["release", "(", "p", ")", "+", "other", "(", ")"][..],
            &["sizeof", "(", "release", "(", "p", ")", ")"][..],
            &["_Alignof", "(", "probe", "(", ")", ")"][..],
            &["x", "=", "release", "(", "p", ")"][..],
        ] {
            assert_eq!(unconditional_call_span(tokens), None, "{tokens:?}");
        }
    }
}
