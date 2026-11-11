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

//! A checked read-only call actual for the pinned librdkafka list count body.
//!
//! This is deliberately narrower than `Borrowed`: only the exact header body
//! `return rl->rl_cnt` and one direct `rd_list_cnt(&ROOT->member[.member]*)`
//! actual remove that occurrence of ROOT from the pointer-return `may_bind`
//! test. Every other occurrence and the ordinary call obligation remain.

use std::collections::BTreeSet;

use crate::legacy_c::{Token, find_function_body, matching};
use crate::typed_field_access::bound_names;

const BODY: &[&str] = &["return", "rl", "-", ">", "rl_cnt", ";"];
const PARAMETER: &[&str] = &["const", "rd_list_t", "*", "rl"];

pub(crate) fn certified(tokens: &[Token]) -> bool {
    let Some((start, _)) = find_function_body(tokens, "rd_list_cnt") else {
        return false;
    };
    let Some(end) = matching(tokens, start, "{", "}") else {
        return false;
    };
    let body = tokens[start + 1..end]
        .iter()
        .map(|token| token.text.as_str())
        .collect::<Vec<_>>();
    if body != BODY {
        return false;
    }
    // Require the prototype adjacent to this definition, not merely a body
    // elsewhere in the translation unit. Qualifier and parameter type are
    // load-bearing parts of the read-only claim.
    let Some(open) = (0..start).rev().find(|&i| {
        tokens[i].text == "rd_list_cnt" && tokens.get(i + 1).is_some_and(|t| t.text == "(")
    }) else {
        return false;
    };
    let Some(close) = matching(tokens, open + 1, "(", ")") else {
        return false;
    };
    if close + 1 != start || open == 0 || tokens[open - 1].text != "int" {
        return false;
    }
    tokens[open + 2..close]
        .iter()
        .map(|token| token.text.as_str())
        .collect::<Vec<_>>()
        == PARAMETER
}

fn name(text: &str) -> bool {
    text.as_bytes()
        .first()
        .is_some_and(|c| c.is_ascii_alphabetic() || *c == b'_')
        && text.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
}

/// The one identifier index inside a precisely framed lexical actual.
fn framed_root(words: &[&str], at: usize) -> Option<(usize, usize)> {
    if words.get(at..at + 3)? != ["rd_list_cnt", "(", "&"] {
        return None;
    }
    let root = at + 3;
    if !name(words[root]) || words.get(root + 1..root + 3)? != ["-", ">"] {
        return None;
    }
    let mut cursor = root + 3;
    if !name(words.get(cursor)?) {
        return None;
    }
    cursor += 1;
    while words.get(cursor) == Some(&".") {
        cursor += 1;
        if !name(words.get(cursor)?) {
            return None;
        }
        cursor += 1;
    }
    (words.get(cursor) == Some(&")")).then_some((root, cursor))
}

/// The old lexical bound-name reader with only certified actual roots masked.
/// A second write, address or call occurrence still names the root.
pub(crate) fn lexical_root_clear(
    tokens: &[Token],
    function: &str,
    root: &str,
    protected_formals: &BTreeSet<String>,
    body_certified: bool,
) -> bool {
    if !body_certified || protected_formals.contains(root) {
        return false;
    }
    let Some((start, _)) = find_function_body(tokens, function) else {
        return false;
    };
    let Some(end) = matching(tokens, start, "{", "}") else {
        return false;
    };
    let mut words = tokens[start + 1..end]
        .iter()
        .map(|token| token.text.as_str())
        .collect::<Vec<_>>();
    let mut found = false;
    for at in 0..words.len() {
        if words[at] != "rd_list_cnt" {
            continue;
        }
        if let Some((index, _close)) = framed_root(&words, at)
            && words[index] == root
        {
            words[index] = "__qcore_framed_actual_root__";
            found = true;
        }
    }
    found && !bound_names(&words, protected_formals).contains(root)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stream(body: &str) -> Vec<Token> {
        format!(
            "static inline int rd_list_cnt ( const rd_list_t * rl ) \
             {{ return rl - > rl_cnt ; }} {}",
            body
        )
        .split_whitespace()
        .map(|text| Token {
            text: text.to_owned(),
            line: 1,
        })
        .collect()
    }

    #[test]
    fn the_exact_header_body_and_actual_are_framed() {
        let tokens = stream(
            "const void * f ( const box * rko ) { \
            int n = rd_list_cnt ( & rko - > list . results ) ; \
            return rko - > list . results . rl_elems ; }",
        );
        assert!(certified(&tokens));
        assert!(lexical_root_clear(
            &tokens,
            "f",
            "rko",
            &BTreeSet::new(),
            certified(&tokens)
        ));
    }

    #[test]
    fn a_changed_body_or_any_other_binding_refuses() {
        let mut tokens = stream(
            "const void * f ( const box * rko ) { \
            int n = rd_list_cnt ( & rko - > list ) ; \
            return rko - > list . rl_elems ; }",
        );
        let body_word = tokens.iter_mut().find(|t| t.text == "rl_cnt").unwrap();
        body_word.text = "rl_elems".to_owned();
        assert!(!certified(&tokens));

        for extra in ["release ( rko ) ;", "rko = other ;", "& rko ;"] {
            let tokens = stream(&format!(
                "const void * f ( const box * rko ) {{ \
                int n = rd_list_cnt ( & rko - > list ) ; {} \
                return rko - > list . rl_elems ; }}",
                extra
            ));
            assert!(
                !lexical_root_clear(&tokens, "f", "rko", &BTreeSet::new(), certified(&tokens)),
                "other binding: {extra}"
            );
        }
    }

    #[test]
    fn a_different_actual_is_not_framed() {
        for actual in ["rko", "& rko", "& rko - > list [ i ]", "& rko - > list + 1"] {
            let tokens = stream(&format!(
                "const void * f ( const box * rko ) {{ \
                int n = rd_list_cnt ( {} ) ; return rko - > list . rl_elems ; }}",
                actual
            ));
            assert!(
                !lexical_root_clear(&tokens, "f", "rko", &BTreeSet::new(), certified(&tokens)),
                "different actual: {actual}"
            );
        }
    }
}
