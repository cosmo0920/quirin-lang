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

//! W48-BOTTOM-UP-BATCH3: a scalar observation of an external zero-argument
//! accessor in a condition. No value, pointer or branch refinement is inferred.
//! The adapter retains the ordinary call instruction before the observation.
//! Scalar typing comes from the unit, custody effects from the contract in force.
//! Faithful extraction and Rust/Lean correspondence remain boundary28 premises.

/// The pure admission mirrored by QCore.ScalarAccessorCondition. The source
/// shape and the ordinary instantiator are independent checks in the adapter.
pub(crate) fn eligible(
    external: bool,
    scalar: bool,
    empty: bool,
    none: bool,
    identity: bool,
) -> bool {
    external && scalar && empty && none && identity
}

/// The exact leaf `*F() == K`, `(*F()) != K` (and redundant whole-leaf
/// parentheses). K is an unsuffixed decimal token. It is never evaluated;
/// the comparison retains both successors. No actual, write or other read
/// can be hidden inside the scalar observation.
pub(crate) fn callee(texts: &[&str]) -> Option<usize> {
    let mut start = 0;
    let mut end = texts.len();
    while texts.get(start) == Some(&"(") {
        let mut depth = 0usize;
        let mut close = None;
        for (i, text) in texts.iter().enumerate().take(end).skip(start) {
            match *text {
                "(" => depth += 1,
                ")" => {
                    depth = depth.checked_sub(1)?;
                    if depth == 0 {
                        close = Some(i);
                        break;
                    }
                }
                _ => {}
            }
        }
        if close != Some(end.checked_sub(1)?) {
            break;
        }
        start += 1;
        end -= 1;
    }
    let (name, first, second, literal, offset) = match &texts[start..end] {
        ["*", name, "(", ")", first, second, literal] => (*name, *first, *second, *literal, 1),
        ["(", "*", name, "(", ")", ")", first, second, literal] => {
            (*name, *first, *second, *literal, 2)
        }
        _ => return None,
    };
    let mut chars = name.chars();
    if !chars.next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        || !chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        || !matches!((first, second), ("=", "=") | ("!", "="))
        || literal.is_empty()
        || !literal.bytes().all(|c| c.is_ascii_digit())
        // Leading zero would be octal in C. No value inference depends on it,
        // but keep the stated decimal source boundary exact.
        || (literal.len() > 1 && literal.starts_with('0'))
    {
        return None;
    }
    Some(start + offset)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eligibility_matches_every_lean_vector() {
        let proof = include_str!("../../../proof/QCore/ScalarAccessorCondition.lean");
        assert!(proof.contains("def vectors : List Bool := [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, true]"));
        for bits in 0..32 {
            assert_eq!(
                eligible(
                    bits & 16 != 0,
                    bits & 8 != 0,
                    bits & 4 != 0,
                    bits & 2 != 0,
                    bits & 1 != 0
                ),
                bits == 31
            );
        }
    }

    #[test]
    fn only_the_complete_read_comparison_is_extracted() {
        for (text, accepted) in [
            ("( * slot ( ) ) = = 11", true),
            ("* slot ( ) ! = 0", true),
            ("( ( * slot ( ) ) = = 11 )", true),
            ("( * slot ( p ) ) = = 11", false),
            ("( * slot ( ) ) = 11", false),
            ("( * slot ( ) ) + 11", false),
            ("( * slot ( ) ) = = p", false),
            ("( * slot ( ) ) = = 01", false),
            ("( * slot ( ) ) = = 11u", false),
            ("( * * slot ( ) ) = = 11", false),
            ("& ( * slot ( ) ) = = 11", false),
            ("( * slot ( ) ) = = 11 , p", false),
        ] {
            let tokens = text.split_whitespace().collect::<Vec<_>>();
            assert_eq!(callee(&tokens).is_some(), accepted, "{text}");
        }
    }
}
