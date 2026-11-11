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

//! Narrow typing for ordinary local `let` annotations.
//!
//! This module separates an ordinary type annotation (`: i32`, `: unit`,
//! `: own<T>`) from a usage annotation (`use one`, and so on).  Executable
//! routes can therefore compare a supported ordinary annotation with the value
//! they actually lower without treating a parsed spelling as checked.  The
//! representation is deliberately small: a route that cannot establish one of
//! these types must refuse the explicit annotation rather than guessing.

use crate::lexer::{Symbol, Token, TokenKind};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LocalTypeAnnotation {
    I32,
    Unit,
    Owned(String),
    Unsupported,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExecutableLocalType {
    I32,
    Unit,
    /// An owned identity whose nominal payload is known, or `None` when the
    /// operation establishes custody without publishing a nominal type.
    Owned {
        nominal: Option<String>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalBindingHead {
    pub equal: usize,
    pub annotation: Option<LocalTypeAnnotation>,
    pub has_usage_annotation: bool,
}

fn word(token: Option<&Token>, expected: &str) -> bool {
    matches!(token.map(|token| &token.kind), Some(TokenKind::Word(found)) if found == expected)
}

fn symbol(token: Option<&Token>, expected: Symbol) -> bool {
    matches!(token.map(|token| &token.kind), Some(TokenKind::Symbol(found)) if *found == expected)
}

/// Parse only the `let NAME [: TYPE] [use ...] =` head.
///
/// Usage syntax is reported but never interpreted here.  This keeps usage
/// diagnostics with their existing pass while ordinary type compatibility
/// belongs to `OrdinaryTypes`.
pub fn local_binding_head(tokens: &[Token]) -> Result<LocalBindingHead, &'static str> {
    let equal = tokens
        .iter()
        .position(|token| token.kind == TokenKind::Symbol(Symbol::Equal))
        .ok_or("an initialized local binding requires `=`")?;
    if !word(tokens.first(), "let")
        || !matches!(
            tokens.get(1).map(|token| &token.kind),
            Some(TokenKind::Word(_))
        )
    {
        return Err("an executable local binding must begin `let NAME`");
    }

    let mut cursor = 2;
    let annotation = if symbol(tokens.get(cursor), Symbol::Colon) {
        cursor += 1;
        let type_start = cursor;
        while cursor < equal && !word(tokens.get(cursor), "use") {
            cursor += 1;
        }
        if type_start == cursor {
            return Err("a local `:` must be followed by a type annotation");
        }
        let annotation = match &tokens[type_start..cursor] {
            [
                Token {
                    kind: TokenKind::Word(name),
                    ..
                },
            ] if name == "i32" => LocalTypeAnnotation::I32,
            [
                Token {
                    kind: TokenKind::Word(name),
                    ..
                },
            ] if name == "unit" => LocalTypeAnnotation::Unit,
            [
                Token {
                    kind: TokenKind::Word(own),
                    ..
                },
                less,
                Token {
                    kind: TokenKind::Word(name),
                    ..
                },
                greater,
            ] if own == "own"
                && symbol(Some(less), Symbol::Less)
                && symbol(Some(greater), Symbol::Greater) =>
            {
                LocalTypeAnnotation::Owned(name.clone())
            }
            _ => LocalTypeAnnotation::Unsupported,
        };
        Some(annotation)
    } else {
        None
    };

    let has_usage_annotation = word(tokens.get(cursor), "use");
    if has_usage_annotation {
        cursor += 1;
        if cursor == equal {
            return Err("a local `use` must be followed by a usage annotation");
        }
        cursor = equal;
    }
    if cursor != equal {
        return Err("unsupported tokens precede this local binding's `=`");
    }
    Ok(LocalBindingHead {
        equal,
        annotation,
        has_usage_annotation,
    })
}

/// Check an explicit annotation against the type the lowering established.
/// Omitted annotations remain inferred.  An owned value without a published
/// nominal payload rejects every explicit `own<T>` annotation: custody alone
/// is not evidence that the identity denotes `T`.
pub fn check_local_annotation(
    annotation: Option<&LocalTypeAnnotation>,
    actual: &ExecutableLocalType,
) -> Result<(), &'static str> {
    match (annotation, actual) {
        (None, _) => Ok(()),
        (Some(LocalTypeAnnotation::I32), ExecutableLocalType::I32)
        | (Some(LocalTypeAnnotation::Unit), ExecutableLocalType::Unit) => Ok(()),
        (
            Some(LocalTypeAnnotation::Owned(annotation)),
            ExecutableLocalType::Owned {
                nominal: Some(actual),
            },
        ) if annotation == actual => Ok(()),
        (Some(LocalTypeAnnotation::Unsupported), _) => {
            Err("unsupported explicit local type annotation")
        }
        (Some(LocalTypeAnnotation::Owned(_)), ExecutableLocalType::Owned { nominal: None }) => Err(
            "this operation does not publish a nominal owned type, so its explicit `own<T>` annotation cannot be checked",
        ),
        _ => Err("local initializer does not match its explicit type annotation"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex;

    fn head(source: &str) -> LocalBindingHead {
        local_binding_head(&lex(source).unwrap()).unwrap()
    }

    #[test]
    fn separates_ordinary_and_usage_annotations() {
        let parsed = head("let value: own<Buffer> use one = source;");
        assert_eq!(
            parsed.annotation,
            Some(LocalTypeAnnotation::Owned("Buffer".to_owned()))
        );
        assert!(parsed.has_usage_annotation);
    }

    #[test]
    fn checks_only_types_the_route_establishes() {
        assert!(
            check_local_annotation(
                head("let n: i32 = 1;").annotation.as_ref(),
                &ExecutableLocalType::I32
            )
            .is_ok()
        );
        assert!(
            check_local_annotation(
                head("let n: unit = 1;").annotation.as_ref(),
                &ExecutableLocalType::I32
            )
            .is_err()
        );
        assert!(
            check_local_annotation(
                head("let r: own<File> = make();").annotation.as_ref(),
                &ExecutableLocalType::Owned { nominal: None }
            )
            .is_err()
        );
        assert!(
            check_local_annotation(
                head("let r: own<Buffer> = make();").annotation.as_ref(),
                &ExecutableLocalType::Owned {
                    nominal: Some("Buffer".to_owned())
                }
            )
            .is_ok()
        );
    }
}
