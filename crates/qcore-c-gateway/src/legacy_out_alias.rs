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

//! Private normalization of immutable entry-prefix Out-pointer aliases.
//! This proves only slot-name identity. Original-slot sanction, freshness,
//! nullability and every-exit discharge still require ordinary CFG validation.
use super::*;

fn matching(tokens: &[Token], start: usize, open: &str, close: &str) -> Option<usize> {
    if tokens.get(start)?.text != open {
        return None;
    }
    let mut depth = 0usize;
    for (index, token) in tokens.iter().enumerate().skip(start) {
        if token.text == open {
            depth += 1;
        }
        if token.text == close {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some(index);
            }
        }
    }
    None
}
fn pointer_type(tokens: &[Token]) -> Option<Vec<String>> {
    let words = tokens.iter().map(|t| t.text.clone()).collect::<Vec<_>>();
    if words.iter().filter(|w| *w == "*").count() < 2
        || words.iter().any(|w| {
            !matches!(
                w.as_str(),
                "void" | "char" | "int" | "long" | "short" | "signed" | "unsigned" | "const" | "*"
            )
        })
    {
        return None;
    }
    Some(words)
}

/// Return an identity-preserving token projection only after every alias use
/// is checked. Prefix declarations copy the exact same flat pointer type from
/// an original formal or earlier alias. No alias assignment, shadow, escape,
/// arithmetic, read-through, branch-local declaration or hidden call is erased.
pub(super) fn normalize(
    tokens: &[Token],
    function: &str,
    contracts: &ResourceContracts,
) -> Option<Vec<Token>> {
    let (body, _) = find_function_body(tokens, function)?;
    let end = matching(tokens, body, "{", "}")?;
    let name = (0..body).rev().find(|&i| {
        tokens[i].text == function && tokens.get(i + 1).is_some_and(|t| t.text == "(")
    })?;
    let close = matching(tokens, name + 1, "(", ")")?;
    if close + 1 != body {
        return None;
    }
    let parameters = function_parameters(tokens, function)?;
    let groups = tokens[name + 2..close]
        .split(|t| t.text == ",")
        .collect::<Vec<_>>();
    if groups.len() != parameters.len() {
        return None;
    }
    let mut known = BTreeMap::<String, (String, Vec<String>)>::new();
    let formals = parameters
        .iter()
        .flatten()
        .cloned()
        .collect::<BTreeSet<_>>();
    for (group, param) in groups.into_iter().zip(parameters) {
        if let Some(param) = param
            && group.last()?.text == param
            && let Some(ty) = pointer_type(&group[..group.len() - 1])
        {
            known.insert(param.clone(), (param, ty));
        }
    }
    let mut aliases = BTreeMap::<String, String>::new();
    let mut cursor = body + 1;
    // A single declaration and a bare initializer; no VLA, attribute, type
    // conversion or side effect can hide inside the discarded initializer.
    while let Some(semi) =
        (cursor..end).find(|&i| matches!(tokens[i].text.as_str(), ";" | "{" | "}"))
    {
        if tokens[semi].text != ";" {
            break;
        }
        let statement = &tokens[cursor..semi];
        let Some(equal) = statement.iter().position(|t| t.text == "=") else {
            break;
        };
        if equal < 2 || equal + 2 != statement.len() {
            break;
        }
        let alias = &statement[equal - 1].text;
        let source = &statement[equal + 1].text;
        let Some((original, expected)) = known.get(source).cloned() else {
            break;
        };
        let Some(ty) = pointer_type(&statement[..equal - 1]) else {
            break;
        };
        if ty != expected
            || !is_identifier(alias)
            || formals.contains(alias)
            || known.contains_key(alias)
        {
            return None;
        }
        aliases.insert(alias.clone(), original.clone());
        known.insert(alias.clone(), (original, ty));
        cursor = semi + 1;
    }
    if aliases.is_empty() {
        return None;
    }
    let mut allowed = BTreeSet::new();
    for index in cursor..end {
        // Only a complete simple indirect store may name an alias directly.
        if tokens[index].text == "*"
            && index > body
            && matches!(tokens[index - 1].text.as_str(), ";" | "{" | "}")
            && tokens.get(index + 2).is_some_and(|t| t.text == "=")
            && aliases.contains_key(&tokens[index + 1].text)
        {
            allowed.insert(index + 1);
        }
        let Some(contract) = contracts.contract(&tokens[index].text) else {
            continue;
        };
        if tokens.get(index + 1).is_none_or(|t| t.text != "(") {
            continue;
        }
        let last = matching(tokens, index + 1, "(", ")")?;
        let mut start = index + 2;
        let mut nesting = 0usize;
        let mut ranges = Vec::new();
        for (i, token) in tokens.iter().enumerate().take(last).skip(index + 2) {
            match token.text.as_str() {
                "(" | "[" => nesting += 1,
                ")" | "]" => nesting = nesting.checked_sub(1)?,
                "," if nesting == 0 => {
                    ranges.push((start, i));
                    start = i + 1;
                }
                _ => {}
            }
        }
        if start < last {
            ranges.push((start, last));
        }
        if ranges.len() != contract.parameters.len() {
            continue;
        }
        for ((mut first, mut limit), mode) in ranges.into_iter().zip(&contract.parameters) {
            if !matches!(mode, ResourceParameterMode::Out { .. }) {
                continue;
            }
            while first < limit
                && tokens[first].text == "("
                && matching(tokens, first, "(", ")") == Some(limit - 1)
            {
                first += 1;
                limit -= 1;
            }
            if first + 1 == limit && aliases.contains_key(&tokens[first].text) {
                allowed.insert(first);
            }
        }
    }
    if !(cursor..end).all(|i| !aliases.contains_key(&tokens[i].text) || allowed.contains(&i))
        || allowed.is_empty()
    {
        return None;
    }
    let mut result = tokens.to_vec();
    // Empty statements keep original line locations; declarations have no
    // effects besides the checked immutable identity copies.
    for token in &mut result[body + 1..cursor] {
        token.text = ";".into();
    }
    for index in allowed {
        result[index].text = aliases[&tokens[index].text].clone();
    }
    Some(result)
}
