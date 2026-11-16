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

//! A lexical reader of C source: tokens with their lines, and the one
//! definition or declaration of a named function.
//!
//! It reads the text as written, before preprocessing. That is what the
//! checker's obligations are stated over, and it is why a function whose
//! body holds a preprocessor directive is refused (its tokens would not be
//! the tokens compiled), as is a name defined or declared more than once.

/// One token, the physical line it starts on (1-based), and its logical
/// line: the count of newlines before it that end a line (a continuation
/// and a newline inside a block comment do not), so a directive is every
/// token of its logical line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    pub text: String,
    pub line: usize,
    pub logical: usize,
}

/// Multi-character punctuators, longest first.
const PUNCTUATORS: [&str; 22] = [
    "<<=", ">>=", "...", "->", "++", "--", "<<", ">>", "<=", ">=", "==", "!=", "&&", "||", "+=",
    "-=", "*=", "/=", "%=", "&=", "|=", "^=",
];

/// Split `text` into tokens: identifiers and numbers, string and character
/// literals (kept whole), and punctuators. Comments, whitespace and line
/// continuations separate tokens and are dropped; `#` is a token.
///
/// # Errors
///
/// An unterminated comment or literal.
pub fn tokenize(text: &str) -> Result<Vec<Token>, String> {
    let bytes = text.as_bytes();
    let mut tokens = Vec::new();
    let mut index = 0;
    let mut line = 1;
    let mut logical = 0;
    let push = |tokens: &mut Vec<Token>, text: &str, line: usize, logical: usize| {
        tokens.push(Token {
            text: text.to_owned(),
            line,
            logical,
        });
    };
    while index < bytes.len() {
        let byte = bytes[index];
        if byte == b'\n' {
            line += 1;
            logical += 1;
            index += 1;
        } else if byte == b'\\' && bytes.get(index + 1) == Some(&b'\n') {
            line += 1;
            index += 2;
        } else if byte.is_ascii_whitespace() {
            index += 1;
        } else if text[index..].starts_with("//") {
            while index < bytes.len() && bytes[index] != b'\n' {
                index += 1;
            }
        } else if text[index..].starts_with("/*") {
            let end = text[index + 2..]
                .find("*/")
                .ok_or_else(|| format!("line {line}: unterminated comment"))?;
            line += text[index..index + 2 + end].matches('\n').count();
            index += 2 + end + 2;
        } else if byte == b'"' || byte == b'\'' {
            let start = index;
            index += 1;
            while index < bytes.len() && bytes[index] != byte {
                if bytes[index] == b'\\' {
                    index += 1;
                }
                if bytes.get(index) == Some(&b'\n') {
                    return Err(format!("line {line}: unterminated literal"));
                }
                index += 1;
            }
            if index >= bytes.len() {
                return Err(format!("line {line}: unterminated literal"));
            }
            index += 1;
            push(&mut tokens, &text[start..index], line, logical);
        } else if byte.is_ascii_alphanumeric() || byte == b'_' {
            let start = index;
            while index < bytes.len()
                && (bytes[index].is_ascii_alphanumeric() || bytes[index] == b'_')
            {
                index += 1;
            }
            push(&mut tokens, &text[start..index], line, logical);
        } else {
            let rest = &text[index..];
            let width = PUNCTUATORS
                .iter()
                .find(|punctuator| rest.starts_with(**punctuator))
                .map_or_else(
                    || rest.chars().next().map_or(1, char::len_utf8),
                    |p| p.len(),
                );
            push(&mut tokens, &rest[..width], line, logical);
            index += width;
        }
    }
    Ok(tokens)
}

/// The words of a pattern written with spaces between tokens.
#[must_use]
pub fn words(pattern: &str) -> Vec<&str> {
    pattern.split_whitespace().collect()
}

/// Every start index at which `pattern` occurs contiguously in `tokens`.
#[must_use]
pub fn occurrences(tokens: &[Token], pattern: &[&str]) -> Vec<usize> {
    if pattern.is_empty() || pattern.len() > tokens.len() {
        return Vec::new();
    }
    (0..=tokens.len() - pattern.len())
        .filter(|start| {
            pattern
                .iter()
                .enumerate()
                .all(|(offset, word)| tokens[start + offset].text == *word)
        })
        .collect()
}

/// The index of the token closing the bracket opened at `open`.
fn closing(tokens: &[Token], open: usize) -> Option<usize> {
    let (left, right) = match tokens[open].text.as_str() {
        "(" => ("(", ")"),
        "{" => ("{", "}"),
        "[" => ("[", "]"),
        _ => return None,
    };
    let mut depth = 0usize;
    for (index, token) in tokens.iter().enumerate().skip(open) {
        if token.text == left {
            depth += 1;
        } else if token.text == right {
            depth -= 1;
            if depth == 0 {
                return Some(index);
            }
        }
    }
    None
}

/// A function's definition: its header (from the end of the previous
/// top-level item to the parameter list's `)`) and its body (inside the
/// braces).
#[derive(Clone, Debug)]
pub struct Definition {
    pub header: Vec<Token>,
    pub body: Vec<Token>,
    pub first_line: usize,
    pub last_line: usize,
}

/// A function's declaration: from the end of the previous top-level item to
/// its `;`.
#[derive(Clone, Debug)]
pub struct Declaration {
    pub tokens: Vec<Token>,
    pub line: usize,
}

/// The top-level items of a token stream: each is a half-open token range
/// ending after a top-level `;` or a top-level `}`, or at a preprocessor
/// line's end (a `#` and the tokens on its line).
fn items(tokens: &[Token]) -> Vec<(usize, usize)> {
    let mut items = Vec::new();
    let mut start = 0;
    let mut index = 0;
    while index < tokens.len() {
        let token = &tokens[index];
        if token.text == "#" && index == start {
            // A directive: every token of its logical line.
            let mut end = index + 1;
            while end < tokens.len() && tokens[end].logical == token.logical {
                end += 1;
            }
            items.push((index, end));
            start = end;
            index = end;
            continue;
        }
        if index == start {
            // A C++ linkage specification (`extern "C" {`, inside an
            // `#ifdef __cplusplus`) and its closing brace are transparent:
            // the declarations inside stay top-level items.
            let opens_linkage = token.text == "extern"
                && tokens.get(index + 1).is_some_and(|t| t.text == "\"C\"")
                && tokens.get(index + 2).is_some_and(|t| t.text == "{");
            if opens_linkage || token.text == "}" {
                let skip = if opens_linkage { 3 } else { 1 };
                start = index + skip;
                index += skip;
                continue;
            }
        }
        match token.text.as_str() {
            "(" | "[" | "{" => {
                let Some(close) = closing(tokens, index) else {
                    items.push((start, tokens.len()));
                    return items;
                };
                if token.text == "{" {
                    // A brace at top level: a function body or an aggregate.
                    // The item ends at the brace unless a `;` follows
                    // (a declaration with an initializer or a type).
                    let mut end = close + 1;
                    let is_body = tokens[start..index].iter().any(|t| t.text == "(")
                        && tokens
                            .get(index.wrapping_sub(1))
                            .is_some_and(|t| t.text == ")");
                    if !is_body {
                        while end < tokens.len() && tokens[end].text != ";" {
                            end += 1;
                        }
                        end = (end + 1).min(tokens.len());
                    }
                    items.push((start, end));
                    start = end;
                    index = end;
                } else {
                    index = close + 1;
                }
            }
            ";" => {
                items.push((start, index + 1));
                start = index + 1;
                index += 1;
            }
            _ => index += 1,
        }
    }
    if start < tokens.len() {
        items.push((start, tokens.len()));
    }
    items
}

/// The name an item defines or declares as a function: the identifier
/// directly before the first top-level `(` of the item.
fn function_name(tokens: &[Token]) -> Option<&str> {
    let open = tokens.iter().position(|token| token.text == "(")?;
    let name = tokens.get(open.checked_sub(1)?)?;
    let first = name.text.as_bytes().first()?;
    (first.is_ascii_alphabetic() || *first == b'_').then_some(name.text.as_str())
}

/// The one definition of `name` in `tokens`.
///
/// # Errors
///
/// No definition, more than one, or a preprocessor directive in its body.
pub fn definition(tokens: &[Token], name: &str) -> Result<Definition, String> {
    let mut found = Vec::new();
    for (start, end) in items(tokens) {
        let item = &tokens[start..end];
        if item.first().is_some_and(|t| t.text == "#") {
            continue;
        }
        if function_name(item) != Some(name) || item.last().is_none_or(|t| t.text != "}") {
            continue;
        }
        let Some(open) = item.iter().position(|t| t.text == "{") else {
            continue;
        };
        found.push(Definition {
            header: item[..open].to_vec(),
            body: item[open + 1..item.len() - 1].to_vec(),
            first_line: item[0].line,
            last_line: item[item.len() - 1].line,
        });
    }
    let definition = match found.len() {
        1 => found.remove(0),
        0 => return Err(format!("no definition of `{name}`")),
        count => return Err(format!("{count} definitions of `{name}`")),
    };
    if let Some(directive) = definition.body.iter().find(|t| t.text == "#") {
        return Err(format!(
            "the body of `{name}` holds a preprocessor directive (line {}), so its tokens \
             are not the tokens compiled",
            directive.line
        ));
    }
    Ok(definition)
}

/// The one declaration (a prototype ending in `;`) of `name` in `tokens`.
///
/// # Errors
///
/// No declaration, or more than one.
pub fn declaration(tokens: &[Token], name: &str) -> Result<Declaration, String> {
    let mut found: Vec<Declaration> = items(tokens)
        .into_iter()
        .map(|(start, end)| &tokens[start..end])
        .filter(|item| {
            item.first().is_some_and(|t| t.text != "#")
                && item.last().is_some_and(|t| t.text == ";")
                && !item.iter().any(|t| t.text == "{")
                && function_name(item) == Some(name)
        })
        .map(|item| Declaration {
            tokens: item.to_vec(),
            line: item[0].line,
        })
        .collect();
    match found.len() {
        1 => Ok(found.remove(0)),
        0 => Err(format!("no declaration of `{name}`")),
        count => Err(format!("{count} declarations of `{name}`")),
    }
}

/// The token texts joined by single spaces.
#[must_use]
pub fn spelled(tokens: &[Token]) -> String {
    tokens
        .iter()
        .map(|token| token.text.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_drop_comments_and_keep_literals_and_lines() {
        let tokens = tokenize("a /* x\n y */ ->b // c\n\"s\\\"t\" '%' x++ \\\n y").unwrap();
        let texts: Vec<_> = tokens.iter().map(|t| t.text.as_str()).collect();
        assert_eq!(texts, ["a", "->", "b", "\"s\\\"t\"", "'%'", "x", "++", "y"]);
        assert_eq!(tokens[1].line, 2);
        assert_eq!(tokens[3].line, 3);
        assert_eq!(tokens[7].line, 4);
        assert!(tokenize("/* open").is_err());
        assert!(tokenize("\"open\n\"").is_err());
    }

    #[test]
    fn a_definition_and_a_declaration_are_found_once() {
        let text = "#include <x.h>\n#define F(a) \\\n  g(a)\nstruct s { int a; };\n\
                    int f(int a);\nstatic int g(void) { return 1; }\n\
                    int f(int a)\n{\n  if(a) { return g(); }\n  return 0;\n}\n";
        let tokens = tokenize(text).unwrap();
        let found = definition(&tokens, "f").unwrap();
        assert_eq!(spelled(&found.header), "int f ( int a )");
        assert_eq!(
            spelled(&found.body),
            "if ( a ) { return g ( ) ; } return 0 ;"
        );
        assert_eq!((found.first_line, found.last_line), (7, 11));
        assert_eq!(
            spelled(&declaration(&tokens, "f").unwrap().tokens),
            "int f ( int a ) ;"
        );
        assert!(definition(&tokens, "h").is_err());
        assert!(declaration(&tokens, "g").is_err());
    }

    #[test]
    fn a_directive_in_a_body_or_a_second_definition_is_refused() {
        let tokens =
            tokenize("int f(void)\n{\n#ifdef X\n  return 1;\n#endif\n  return 0;\n}\n").unwrap();
        assert!(definition(&tokens, "f").unwrap_err().contains("directive"));
        let tokens = tokenize("int f(void) { return 0; }\nint f(void) { return 1; }\n").unwrap();
        assert!(
            definition(&tokens, "f")
                .unwrap_err()
                .contains("2 definitions")
        );
    }

    #[test]
    fn occurrences_are_contiguous() {
        let tokens = tokenize("a b a b c").unwrap();
        assert_eq!(occurrences(&tokens, &["a", "b"]), [0, 2]);
        assert_eq!(occurrences(&tokens, &["b", "c"]), [3]);
        assert!(occurrences(&tokens, &[]).is_empty());
    }
}
