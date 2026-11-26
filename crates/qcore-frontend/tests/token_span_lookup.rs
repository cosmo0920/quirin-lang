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

//! The span-keyed token lookups read the tokens a scan of the whole stream
//! reads: `span_candidates` is a superset of the tokens inside a span, exact
//! for a stream in source order and the whole stream otherwise.

use qcore_frontend::frontend::parse_source_program;
use qcore_frontend::lexer::{
    Span, Token, TokenKind, TokenStream, lex, span_candidates, tokens_in_source_order,
};

fn inside(span: Span, token: &Token) -> bool {
    span.start <= token.span.start && token.span.end <= span.end
}

fn scan(tokens: &[Token], span: Span) -> Vec<&Token> {
    tokens.iter().filter(|token| inside(span, token)).collect()
}

/// Every span over the stream's own boundaries, plus one byte either side.
fn spans(tokens: &[Token]) -> Vec<Span> {
    let mut bounds: Vec<usize> = tokens
        .iter()
        .flat_map(|token| [token.span.start, token.span.end])
        .flat_map(|at| [at.saturating_sub(1), at, at + 1])
        .collect();
    bounds.sort_unstable();
    bounds.dedup();
    let mut spans = Vec::new();
    for (index, start) in bounds.iter().enumerate() {
        for end in &bounds[index..] {
            spans.push(Span {
                start: *start,
                end: *end,
            });
        }
    }
    spans
}

const SOURCE: &str = "fn helper(x: i32) -> i32 {\n    let y = x + 1;\n    return y;\n}\n\
                      fn main() -> i32 {\n    let r = helper(41);\n    return r;\n}\n";

#[test]
fn a_lexed_stream_is_in_source_order_and_its_candidates_are_exact() {
    let mut tokens = lex(SOURCE).expect("lexes");
    tokens.pop();
    assert!(tokens_in_source_order(&tokens));
    for span in spans(&tokens) {
        let range = span_candidates(&tokens, true, span);
        let near: Vec<&Token> = tokens[range].iter().collect();
        assert_eq!(near, scan(&tokens, span), "{span:?}");
    }
}

#[test]
fn a_stream_out_of_source_order_is_returned_whole() {
    let mut tokens = lex(SOURCE).expect("lexes");
    tokens.pop();
    tokens.swap(1, 6);
    assert!(!tokens_in_source_order(&tokens));
    for span in spans(&tokens) {
        assert_eq!(span_candidates(&tokens, false, span), 0..tokens.len());
        let stream = TokenStream::new(&tokens, false);
        let near: Vec<&Token> = stream
            .near(span)
            .iter()
            .filter(|token| inside(span, token))
            .collect();
        assert_eq!(near, scan(&tokens, span), "{span:?}");
    }
}

#[test]
fn zero_width_tokens_keep_the_candidates_exact() {
    let mut tokens = lex("a b c").expect("lexes");
    tokens.pop();
    let at = tokens[1].span.start;
    tokens.insert(
        1,
        Token {
            kind: TokenKind::Word("synthesized".to_owned()),
            span: Span { start: at, end: at },
        },
    );
    assert!(tokens_in_source_order(&tokens));
    for span in spans(&tokens) {
        let range = span_candidates(&tokens, true, span);
        let near: Vec<&Token> = tokens[range].iter().collect();
        assert_eq!(near, scan(&tokens, span), "{span:?}");
    }
}

#[test]
fn a_parsed_program_reads_the_scanned_tokens_with_and_without_a_pipe() {
    let piped = "fn helper(x: i32) -> i32 {\n    return x + 1;\n}\n\
                 fn main() -> i32 {\n    let r = 41 |> helper;\n    return r;\n}\n";
    for source in [SOURCE, piped] {
        let program = parse_source_program(source).expect("parses");
        let tokens = program.lexical_tokens();
        for span in spans(tokens) {
            let near: Vec<&Token> = program
                .tokens_near(span)
                .iter()
                .filter(|token| inside(span, token))
                .collect();
            assert_eq!(near, scan(tokens, span), "{span:?}");
            let stream: Vec<&Token> = program
                .token_stream()
                .near(span)
                .iter()
                .filter(|token| inside(span, token))
                .collect();
            assert_eq!(stream, scan(tokens, span), "{span:?}");
        }
    }
}
