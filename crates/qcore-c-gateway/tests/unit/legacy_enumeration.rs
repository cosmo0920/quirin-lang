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

//! The enumeration lexer is a copy of the gateway tokenizer with one extra
//! branch. `docs/reports/remaining-e01-f1/README.md` §1.1 claimed a test pinned
//! that the two agree on marker-free input; no such test existed, and unit
//! **E01-F1-RAW** (`docs/reports/remaining-e01-f1-raw/README.md`) changed the
//! shared lexical rule, so it is supplied here.

use super::tokenize_marked;
use crate::legacy_c::tokenize;

/// Sources that exercise every branch the two lexers share, including the
/// E01-F1-RAW logical-line rule. A marker-free input must tokenize identically,
/// token text for token text, or the copy has drifted.
const SHARED: &[&str] = &[
    "int a(void) { return 0; }\n",
    "#define PLAIN 1\nint a(void) { return PLAIN; }\n",
    "#define WRAP(E)   \\\n        {E, &(#E)[0]}\nint a(void) { return 0; }\n",
    "#define JOIN(a, b) a##b\nint a(void) { return 0; }\n",
    "// comment\n/* block\n   comment */\nint a(void) { return 0; }\n",
    "// spliced comment \\\nint hidden(void) { return 0; }\nint a(void) { return 1; }\n",
    "const char *s = \"#not a directive\";\nchar c = '#';\nint a(void) { return 0; }\n",
    "    #   define INDENTED 1\nint a(void) { return 0; }\n",
    "int a(void) { int x = 0; # (void) x; }\n",
    "#if 0\nint gone(void) {}\n#endif\nint a(void) { return 0; }\n",
    "#pragma once\nint a(void) { return 0; }\n",
];

#[test]
fn the_two_tokenizers_agree_on_marker_free_input() {
    for source in SHARED {
        let plain = tokenize(source)
            .iter()
            .map(|token| token.text.clone())
            .collect::<Vec<_>>();
        let marked = tokenize_marked(source)
            .tokens
            .iter()
            .map(|token| token.text.clone())
            .collect::<Vec<_>>();
        assert_eq!(plain, marked, "lexers disagree on {source:?}");
    }
}

/// A `# <line> "<file>"` marker is read only when it is line-initial. A
/// marker-looking run after code on the same line is ordinary punctuation, so
/// no attribution can be forged by a source that spells one mid-line.
#[test]
fn a_marker_is_read_only_at_a_logical_line_start() {
    let real = tokenize_marked("# 1 \"main.c\"\nint a(void) { return 0; }\n");
    assert_eq!(real.primary.as_deref(), Some("main.c"));
    assert!(!real.unbalanced);

    let forged = tokenize_marked("int a(void) { return 0; } # 1 \"forged.c\" 1\n");
    assert_eq!(forged.primary, None);
    assert!(
        forged.tokens.iter().any(|token| token.text == "#"),
        "a mid-line `#` is an ordinary punctuator"
    );
    assert!(
        forged.depths.iter().all(|depth| *depth == 0),
        "a mid-line marker-looking run must not push the include stack"
    );
}

/// A directive continued with `\`-newline ends at its logical line, so the
/// marker that follows it is still line-initial and still read.
#[test]
fn a_marker_after_a_continued_directive_is_still_read() {
    let stream = tokenize_marked(
        "# 1 \"main.c\"\n#define WRAP(E)   \\\n        {E, &(#E)[0]}\n# 1 \"other.c\" 1\nint a(void) { return 0; }\n",
    );
    assert_eq!(stream.primary.as_deref(), Some("main.c"));
    // The second marker enters a file, so it is read: every emitted token is
    // at depth 1. The macro's replacement list contributes none of them — it
    // is directive text, not source.
    assert!(
        stream.depths.iter().all(|depth| *depth == 1),
        "the trailing enter marker must be read, and the macro body must \
         contribute no token"
    );
    assert!(
        stream
            .tokens
            .iter()
            .all(|token| token.text != "WRAP" && token.text != "E"),
        "the macro's replacement list must contribute no token"
    );
}

// ---------------------------------------------------------------------------
// Include-depth attribution (unit E01-TAIL, residual E01-F1-STDIN)
// ---------------------------------------------------------------------------

/// The reader-level matrix for the depth rule. Each row is a marker stream
/// Clang could emit (or, for the refusal rows, could not), with the depth every
/// token must land at.
///
/// The load-bearing rows are the two `#line` directions: a flagless marker
/// changes the *name* Clang reports and nothing else, so it must leave the
/// depth alone in both directions.
#[test]
fn include_depth_follows_the_enter_and_return_flags() {
    // Flags 3 and 4 say nothing about the stack.
    let system = tokenize_marked("# 1 \"m.c\"\n# 1 \"s.h\" 1 3 4\nint a;\n# 2 \"m.c\" 2\nint b;\n");
    assert_eq!(system.depths, vec![1, 1, 1, 0, 0, 0], "`1 3 4` enters once");
    assert!(!system.unbalanced);

    // A flagless marker inside a header (a `#line`) keeps the header's depth.
    let relabel_in =
        tokenize_marked("# 1 \"m.c\"\n# 1 \"h.h\" 1\n# 9 \"m.c\"\nint a;\n# 2 \"m.c\" 2\nint b;\n");
    assert_eq!(
        relabel_in.depths,
        vec![1, 1, 1, 0, 0, 0],
        "a `#line` writing the primary spelling must not leave the header"
    );

    // A flagless marker in the primary input (a `#line`) keeps depth 0.
    let relabel_out = tokenize_marked("# 1 \"m.c\"\nint a;\n# 9 \"far.h\"\nint b;\n");
    assert_eq!(
        relabel_out.depths,
        vec![0, 0, 0, 0, 0, 0],
        "a `#line` writing a foreign spelling must not leave the primary input"
    );

    // Nesting.
    let nested = tokenize_marked(
        "# 1 \"m.c\"\n# 1 \"a.h\" 1\nint a;\n# 1 \"b.h\" 1\nint b;\n# 3 \"a.h\" 2\nint c;\n# 2 \"m.c\" 2\nint d;\n",
    );
    assert_eq!(nested.depths, vec![1, 1, 1, 2, 2, 2, 1, 1, 1, 0, 0, 0]);
    assert!(!nested.unbalanced);
}

/// Two ways an include stack fails to balance, and the well-formed control.
/// Neither failure may be attributed: `inventory_from_marked` refuses the
/// completeness claim on `unbalanced-include-markers` instead.
#[test]
fn unbalanced_marker_streams_are_refused() {
    // A return with nothing to return to.
    let underflow = tokenize_marked("# 1 \"m.c\"\n# 1 \"m.c\" 2\nint a;\n");
    assert!(underflow.unbalanced, "a return at depth 0 is unreadable");

    // A file still open at end of output.
    let unclosed = tokenize_marked("# 1 \"m.c\"\n# 1 \"h.h\" 1\nint a;\n");
    assert!(unclosed.unbalanced, "an unclosed file leaves depth nonzero");

    // The control: balanced, so readable.
    let balanced = tokenize_marked("# 1 \"m.c\"\n# 1 \"h.h\" 1\nint a;\n# 2 \"m.c\" 2\nint b;\n");
    assert!(!balanced.unbalanced);
}

/// A trailing word that is not a flag makes the line not a marker at all, so
/// it neither names a primary input nor moves the stack. This is what keeps a
/// `#pragma` — and any future directive shape — from being read as one.
#[test]
fn an_unreadable_trailing_word_is_not_a_marker() {
    let stream = tokenize_marked("# 1 \"m.c\" 1 bogus\nint a;\n");
    assert_eq!(stream.primary, None, "not a marker, so no primary input");
    assert!(
        stream.depths.iter().all(|depth| *depth == 0),
        "an unread directive must not move the include stack"
    );
}

// ---------------------------------------------------------------------------
// Exhaustive agreement with the Lean model (unit E01-F1-RAW)
// ---------------------------------------------------------------------------

/// The seven single-byte symbols of `QCore.RawDirectiveLexing.digestAlphabet`,
/// in its order. Identifier runs and comments are excluded there because they
/// are not one byte each in this lexer.
const DIGEST_ALPHABET: [u8; 7] = *b"#\n\\ {};";

/// `QCore.RawDirectiveLexing.Tok.code`.
fn token_code(text: &str) -> u64 {
    match text {
        "#" => 1,
        "\\" => 2,
        "{" => 3,
        "}" => 4,
        ";" => 5,
        _ => 6,
    }
}

/// `QCore.RawDirectiveLexing.strings n`: every byte string of length exactly
/// `n`, generated by the same recurrence — each step prepends, so the *last*
/// symbol varies slowest and the two enumerations coincide element for element.
fn strings(n: usize) -> Vec<Vec<u8>> {
    if n == 0 {
        return vec![Vec::new()];
    }
    let mut out = Vec::new();
    for shorter in strings(n - 1) {
        for symbol in DIGEST_ALPHABET {
            let mut next = Vec::with_capacity(shorter.len() + 1);
            next.push(symbol);
            next.extend_from_slice(&shorter);
            out.push(next);
        }
    }
    out
}

/// The exhaustive-domain tie to `QCore.RawDirectiveLexing`. Both sides run
/// their own lexer over the same 400 inputs and fold the same checksum; the
/// number below is the one the Lean kernel decides in
/// `the_domain_checksum_is_pinned`, transcribed rather than recomputed here.
///
/// Any divergence between the model's directive rule and this tokenizer's
/// moves the number, so neither can be changed alone.
#[test]
fn the_lean_domain_checksum_reproduces() {
    let mut domain = Vec::new();
    for length in 0..=3 {
        domain.extend(strings(length));
    }
    assert_eq!(domain.len(), 400, "the domain must match `domain 3`");

    let mut accumulator: u64 = 11;
    for input in &domain {
        let text = String::from_utf8(input.clone()).expect("the alphabet is ASCII");
        let mut token_checksum: u64 = 7;
        for token in tokenize(&text) {
            token_checksum = (token_checksum * 31 + token_code(&token.text)) % 1_000_003;
        }
        accumulator = (accumulator * 1_000_033 + token_checksum) % 1_000_003;
    }
    assert_eq!(
        accumulator, 478_977,
        "the gateway tokenizer and QCore.RawDirectiveLexing.lex disagree over the length-3 domain"
    );
}
