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

//! Card GATEWAY-ANALYSIS-PATH-OPTIMIZATION: the token stream: the
//! tokenizer, conditional-directive scan, raw definition scan, body and
//! parameter readers and bracket matching.
//!
//! Moved verbatim from `legacy_c.rs` (only visibility widened to
//! `pub(super)` where the parent or a sibling module reads an item).

use super::*;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct Token {
    pub(crate) text: String,
    pub(crate) line: usize,
}

/// Width of a C11 5.1.1.2 phase-2 line splice at `cursor`, if there is one.
///
/// Phase 2 deletes each backslash immediately followed by a new-line, joining
/// the two physical lines into one *logical* line, before any preprocessing
/// token is formed. Only the two spellings a conforming source can use are
/// admitted: `\` `\n` and `\` `\r` `\n`. A backslash followed by anything else
/// — trailing blanks included — is not a splice here, so a source that relies
/// on a compiler extension keeps its existing tokenization rather than
/// silently acquiring a different one.
pub(crate) fn line_splice_width(bytes: &[u8], cursor: usize) -> Option<usize> {
    if bytes.get(cursor) != Some(&b'\\') {
        return None;
    }
    match bytes.get(cursor + 1) {
        Some(&b'\n') => Some(2),
        Some(&b'\r') if bytes.get(cursor + 2) == Some(&b'\n') => Some(3),
        _ => None,
    }
}

/// Lex C source into the gateway's coarse token stream.
///
/// # Directives (unit **E01-F1-RAW**)
///
/// C11 6.10 reads a preprocessing directive only when `#` is the **first
/// preprocessing token of a logical line** — that is, when nothing but white
/// space and comments precedes it since the last new-line that phase 2 did not
/// delete — and the directive then runs to the end of that *logical* line. Two
/// consequences are load-bearing here and were both wrong before:
///
/// * a `#` that is *not* line-initial is an ordinary punctuator: the `#`
///   stringification and `##` pasting operators of a macro replacement list,
///   which may sit on a backslash-continuation line. Reading one as a directive
///   discarded the rest of that line — a closing brace included — and
///   desynchronised the file-scope brace depth for the whole rest of the file
///   (`docs/reports/remaining-e01-f1/README.md` §2.2, measured on
///   `librdkafka/src/rdkafka.c`);
/// * a directive continued with `\`-newline carries its whole replacement list,
///   so a multi-line macro body is skipped in full, exactly as a single-line
///   one already was. Macro *bodies* therefore contribute no token and no brace
///   to this scan, uniformly; a macro's braces perturb no depth.
///
/// A `#` reached in any other position is emitted as its own punctuator token
/// rather than discarded, so unmodeled text stays visible to the callers that
/// refuse what they cannot read.
pub(crate) fn tokenize(source: &str) -> Vec<Token> {
    tokenize_with_conditionals(source).0
}

/// Which conditional-inclusion directive a line-initial `#` introduces
/// (unit **E01-RAW-COND**).
///
/// Only the four *structural* roles matter to the raw scan: a group opens, an
/// arm ends and another begins, the last arm begins, the group closes. Which
/// controlling expression a directive carries is deliberately never read: this
/// scan does not evaluate conditionals and never selects an arm.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ConditionalDirective {
    /// `#if`, `#ifdef`, `#ifndef`.
    Open,
    /// `#elif`, `#elifdef`, `#elifndef`.
    Alternative,
    /// `#else`. The group's arms then cover every outcome, so there is no
    /// implicit empty arm.
    Otherwise,
    /// `#endif`.
    Close,
}

/// One conditional directive, positioned in the token stream it contributes no
/// token to: `token_index` is the number of tokens emitted before it, so the
/// directive stands immediately before `tokens[token_index]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ConditionalMark {
    pub(super) token_index: usize,
    pub(super) directive: ConditionalDirective,
    /// The directive's byte range in the source, `#` to the end of its logical
    /// line (card W47-LOOP-OWNER-COROUTINE-HOP reads the condition).
    pub(super) span: (usize, usize),
}

impl ConditionalMark {
    /// The number of tokens emitted before the directive (card
    /// W47-LOOP-OWNER-CONTRACTS reads the group structure too).
    pub(crate) fn token_index(self) -> usize {
        self.token_index
    }

    /// The directive's structural role.
    pub(crate) fn directive(self) -> ConditionalDirective {
        self.directive
    }

    /// The directive's byte range in the source it was read from.
    pub(crate) fn span(self) -> (usize, usize) {
        self.span
    }
}

/// Read the directive word of one complete directive line, which begins at its
/// `#`, and classify it if it is a conditional-inclusion directive.
///
/// The word is separated from the `#` by nothing, by horizontal white space, by
/// phase-2 splices, or by block comments; a `//` comment ends the line before a
/// word can follow. Recognition is total and exact — a directive introducer is
/// never macro-expanded (C11 6.10.3p10), so `#if` has exactly these spellings —
/// and it is deliberately *not* extended to unknown words: `#define`,
/// `#include`, `#pragma` and the null directive stay what they were, opaque
/// text that contributes nothing.
pub(super) fn conditional_directive(line: &str) -> Option<ConditionalDirective> {
    let bytes = line.as_bytes();
    let mut cursor = 1; // the `#` itself
    loop {
        match bytes.get(cursor) {
            Some(&b'\\') if line_splice_width(bytes, cursor).is_some() => {
                cursor += line_splice_width(bytes, cursor)?;
            }
            Some(byte) if byte.is_ascii_whitespace() => cursor += 1,
            Some(&b'/') if bytes.get(cursor + 1) == Some(&b'*') => {
                cursor += 2;
                while cursor + 1 < bytes.len()
                    && !(bytes[cursor] == b'*' && bytes[cursor + 1] == b'/')
                {
                    cursor += 1;
                }
                cursor = (cursor + 2).min(bytes.len());
            }
            _ => break,
        }
    }
    let start = cursor;
    while cursor < bytes.len() && (bytes[cursor].is_ascii_alphanumeric() || bytes[cursor] == b'_') {
        cursor += 1;
    }
    match &line[start..cursor] {
        "if" | "ifdef" | "ifndef" => Some(ConditionalDirective::Open),
        "elif" | "elifdef" | "elifndef" => Some(ConditionalDirective::Alternative),
        "else" => Some(ConditionalDirective::Otherwise),
        "endif" => Some(ConditionalDirective::Close),
        _ => None,
    }
}

/// Lex as [`tokenize`] does, additionally recording where each
/// conditional-inclusion directive stood.
///
/// The token stream is **byte-identical** to [`tokenize`]'s: a directive still
/// contributes no token, so every consumer that reads only tokens is unchanged.
/// The marks are a side channel for [`function_names`], which needs the group
/// structure to keep the arms of one `#if … #endif` from each contributing a
/// brace to the same file-scope depth (unit **E01-RAW-COND**).
pub(crate) fn tokenize_with_conditionals(source: &str) -> (Vec<Token>, Vec<ConditionalMark>) {
    let (tokens, conditionals, _) = tokenize_with_directives(source);
    (tokens, conditionals)
}

/// Lex as [`tokenize_with_conditionals`] does, additionally recording where
/// every *other* directive stood, as `(token_index, byte_start)` (card
/// **TAIL-FILE-DEFINITION-CENSUS-INCOMPLETE**, rule 49): a `#define`,
/// `#undef`, `#include` or `#pragma` may change which macros are defined, so
/// the guard-correlation scan forgets what it assumed about every guard there.
/// The token stream and the conditional marks are exactly
/// [`tokenize_with_conditionals`]'s.
pub(crate) fn tokenize_with_directives(
    source: &str,
) -> (Vec<Token>, Vec<ConditionalMark>, Vec<(usize, usize)>) {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut conditionals = Vec::new();
    let mut others = Vec::new();
    let mut cursor = 0;
    let mut line = 1;
    // True while nothing but white space and comments has been seen since the
    // last undeleted new-line. A splice does not restore it: the logical line
    // continues.
    let mut at_logical_line_start = true;
    while cursor < bytes.len() {
        let byte = bytes[cursor];
        if let Some(width) = line_splice_width(bytes, cursor) {
            line += 1;
            cursor += width;
        } else if byte == b'\n' {
            line += 1;
            cursor += 1;
            at_logical_line_start = true;
        } else if byte.is_ascii_whitespace() {
            cursor += 1;
        } else if byte == b'/' && bytes.get(cursor + 1) == Some(&b'/') {
            // A `//` comment is white space that ends at the next *logical*
            // line boundary, so a spliced continuation line stays commented.
            cursor += 2;
            while cursor < bytes.len() && bytes[cursor] != b'\n' {
                if let Some(width) = line_splice_width(bytes, cursor) {
                    line += 1;
                    cursor += width;
                } else {
                    cursor += 1;
                }
            }
        } else if byte == b'/' && bytes.get(cursor + 1) == Some(&b'*') {
            // A block comment is white space too. A new-line inside it is a
            // real new-line, so a `#` after it is still line-initial.
            cursor += 2;
            while cursor + 1 < bytes.len() && !(bytes[cursor] == b'*' && bytes[cursor + 1] == b'/')
            {
                if bytes[cursor] == b'\n' {
                    line += 1;
                    at_logical_line_start = true;
                }
                cursor += 1;
            }
            cursor = (cursor + 2).min(bytes.len());
        } else if byte == b'#' && at_logical_line_start {
            // A directive, consumed to the end of its logical line.
            let start = cursor;
            while cursor < bytes.len() && bytes[cursor] != b'\n' {
                if let Some(width) = line_splice_width(bytes, cursor) {
                    line += 1;
                    cursor += width;
                } else {
                    cursor += 1;
                }
            }
            if let Some(directive) = conditional_directive(&source[start..cursor]) {
                conditionals.push(ConditionalMark {
                    token_index: tokens.len(),
                    directive,
                    span: (start, cursor),
                });
            } else {
                others.push((tokens.len(), start));
            }
        } else if byte == b'"' || byte == b'\'' {
            at_logical_line_start = false;
            let quote = byte;
            let token_line = line;
            cursor += 1;
            while cursor < bytes.len() {
                if bytes[cursor] == b'\\' {
                    cursor = (cursor + 2).min(bytes.len());
                } else if bytes[cursor] == quote {
                    cursor += 1;
                    break;
                } else {
                    if bytes[cursor] == b'\n' {
                        line += 1;
                    }
                    cursor += 1;
                }
            }
            tokens.push(Token {
                text: "__qcore_literal__".to_owned(),
                line: token_line,
            });
        } else if byte.is_ascii_alphabetic() || byte == b'_' {
            at_logical_line_start = false;
            let start = cursor;
            cursor += 1;
            while cursor < bytes.len()
                && (bytes[cursor].is_ascii_alphanumeric() || bytes[cursor] == b'_')
            {
                cursor += 1;
            }
            tokens.push(Token {
                text: source[start..cursor].to_owned(),
                line,
            });
        } else if byte.is_ascii_digit() {
            at_logical_line_start = false;
            let start = cursor;
            cursor += 1;
            while cursor < bytes.len() && bytes[cursor].is_ascii_alphanumeric() {
                cursor += 1;
            }
            tokens.push(Token {
                text: source[start..cursor].to_owned(),
                line,
            });
        } else {
            // Every other byte is its own punctuator, `#` and `##` included:
            // a non-line-initial `#` is a stringification or pasting operator,
            // never a directive introducer.
            at_logical_line_start = false;
            tokens.push(Token {
                text: char::from(byte).to_string(),
                line,
            });
            cursor += 1;
        }
    }
    (tokens, conditionals, others)
}

/// Card **W43-INDIRECT-ACCESS-GENERAL**, census only: every `NAME = CALLEE (`
/// in the body starting at `body_start` whose callee's in-force contract
/// declares a derived or an owned result. A label for the census's base axis,
/// never consulted by any rule.
#[cfg(feature = "indirect-census")]
pub(super) fn census_body_bindings(
    tokens: &[Token],
    body_start: usize,
    contracts: &ResourceContracts,
) -> crate::indirect_census::BodyBindings {
    let mut bindings = crate::indirect_census::BodyBindings::default();
    let Some(body_end) = matching(tokens, body_start, "{", "}") else {
        return bindings;
    };
    let texts = tokens[body_start..body_end]
        .iter()
        .map(|token| token.text.as_str())
        .collect::<Vec<_>>();
    for (index, window) in texts.windows(4).enumerate() {
        let [name, "=", callee, "("] = window else {
            continue;
        };
        // A chained assignment `a = NAME = f (` is left unlabelled.
        if index > 0 && texts[index - 1] == "=" {
            continue;
        }
        if !is_identifier(name) || !is_identifier(callee) {
            continue;
        }
        match contracts.contract(callee).map(|contract| contract.result) {
            Some(ResourceReturnMode::Derived { .. }) => {
                bindings.derived.insert((*name).to_owned());
            }
            Some(ResourceReturnMode::Owned { .. } | ResourceReturnMode::Reallocated { .. }) => {
                bindings.owned.insert((*name).to_owned());
            }
            _ => {}
        }
    }
    bindings
}

pub(crate) fn find_function_body(tokens: &[Token], function: &str) -> Option<(usize, usize)> {
    for (index, token) in tokens.iter().enumerate() {
        if token.text != function || tokens.get(index + 1).map(|t| t.text.as_str()) != Some("(") {
            continue;
        }
        let close = matching(tokens, index + 1, "(", ")")?;
        if tokens.get(close + 1).map(|t| t.text.as_str()) == Some("{") {
            return Some((close + 1, token.line));
        }
    }
    None
}

/// Why the raw scan could not establish a conditional group's contribution to
/// file-scope brace depth (unit **E01-RAW-COND**).
///
/// Both grounds are mechanical, and neither is a defect claim about the
/// analysed source: they say the *scan* cannot decide, not that the source is
/// wrong.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConditionalScanRefusal {
    /// Two arms of one `#if … #endif` group leave different file-scope brace
    /// depths, so which depth survives is decided by evaluating the controlling
    /// expressions — preprocessing, not lexing. The scan takes the greatest
    /// arm depth, which can only hide definitions, never invent one.
    ArmDepthDisagreement,
    /// A conditional group is not well formed in this token stream: an
    /// `#elif`, `#else` or `#endif` with no open group, an `#elif`/`#else`
    /// after an `#else`, or a group never closed.
    UnstructuredGroup,
}

impl ConditionalScanRefusal {
    /// Stable report spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ArmDepthDisagreement => "conditional-arm-depth-disagreement",
            Self::UnstructuredGroup => "unstructured-conditional-group",
        }
    }

    /// The free-text reason carried by the refusal row.
    #[must_use]
    pub const fn reason(self) -> &'static str {
        match self {
            Self::ArmDepthDisagreement => {
                "raw scan refused a conditional group: its arms leave different file-scope brace depths, and deciding which arm survives is conditional evaluation, not lexing"
            }
            Self::UnstructuredGroup => {
                "raw scan refused a conditional group: an `#elif`, `#else` or `#endif` has no open group, or a group is never closed"
            }
        }
    }
}

/// The raw token scan of one main source: the definitions it found, and the
/// grounds on which it could not establish a conditional group's contribution.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LegacyRawScan {
    pub(super) definitions: Vec<String>,
    pub(super) refusals: Vec<ConditionalScanRefusal>,
}

impl LegacyRawScan {
    /// The definitions this scan found, sorted and deduplicated.
    #[must_use]
    pub fn definitions(&self) -> &[String] {
        &self.definitions
    }

    /// The grounds on which a conditional group's contribution was refused.
    /// Empty means every conditional group in this source was decided exactly.
    #[must_use]
    pub fn refusals(&self) -> &[ConditionalScanRefusal] {
        &self.refusals
    }

    /// Whether every conditional group was decided exactly. A scan that is not
    /// exact is still a *subset* of the definitions of every arm selection —
    /// the depth it carries dominates every selection's — but it may be a
    /// proper subset, so it cannot support a completeness claim.
    #[must_use]
    pub fn is_exact(&self) -> bool {
        self.refusals.is_empty()
    }
}

/// One open `#if … #endif` group, while its arms are being scanned.
pub(super) struct ConditionalFrame {
    /// File-scope brace depth where the group began: every arm is scanned from
    /// here, so a definition written in *any* arm is seen at the depth it would
    /// have if that arm survived (E01-F1 §1.4 monotonicity).
    pub(super) entry_depth: usize,
    /// The depth each already-scanned arm ended at.
    pub(super) arm_depths: Vec<usize>,
    /// Whether an `#else` arm has been opened.
    pub(super) saw_else: bool,
}

/// Apply one conditional directive to the scan state (unit **E01-RAW-COND**).
///
/// The rule, stated once: **a group's resulting depth is the greatest depth its
/// arms reach**, where a group without `#else` has an implicit empty arm that
/// reaches the entry depth. When every arm reaches the same depth — the
/// overwhelmingly common case, and the `rd_kafka_dir_is_empty` case this unit
/// closes — that greatest depth *is* the depth of every arm, so the group is
/// decided exactly without any arm being selected. When the arms disagree, the
/// scan does not guess: it takes the greatest depth, which dominates every arm
/// selection and can therefore only hide definitions, and records
/// [`ConditionalScanRefusal::ArmDepthDisagreement`] so the enumeration's
/// completeness claim is refused rather than silently over-claimed.
pub(super) fn apply_conditional(
    mark: ConditionalMark,
    frames: &mut Vec<ConditionalFrame>,
    brace_depth: &mut usize,
    refusals: &mut BTreeSet<ConditionalScanRefusal>,
) {
    match mark.directive {
        ConditionalDirective::Open => frames.push(ConditionalFrame {
            entry_depth: *brace_depth,
            arm_depths: Vec::new(),
            saw_else: false,
        }),
        ConditionalDirective::Alternative | ConditionalDirective::Otherwise => {
            let Some(frame) = frames.last_mut() else {
                refusals.insert(ConditionalScanRefusal::UnstructuredGroup);
                return;
            };
            if frame.saw_else {
                // An `#elif` or a second `#else` after `#else` is not a group
                // this scan can read.
                refusals.insert(ConditionalScanRefusal::UnstructuredGroup);
            }
            frame.saw_else |= mark.directive == ConditionalDirective::Otherwise;
            frame.arm_depths.push(*brace_depth);
            *brace_depth = frame.entry_depth;
        }
        ConditionalDirective::Close => {
            let Some(mut frame) = frames.pop() else {
                refusals.insert(ConditionalScanRefusal::UnstructuredGroup);
                return;
            };
            frame.arm_depths.push(*brace_depth);
            if !frame.saw_else {
                // Every controlling expression may be false: an implicit empty
                // arm that reaches the entry depth is one of the outcomes.
                frame.arm_depths.push(frame.entry_depth);
            }
            let first = frame.arm_depths[0];
            if frame.arm_depths.iter().any(|depth| *depth != first) {
                refusals.insert(ConditionalScanRefusal::ArmDepthDisagreement);
            }
            *brace_depth = frame.arm_depths.into_iter().max().unwrap_or(first);
        }
    }
}

pub(super) fn function_names(tokens: &[Token], conditionals: &[ConditionalMark]) -> LegacyRawScan {
    let mut names = BTreeSet::new();
    let mut refusals = BTreeSet::new();
    let mut frames: Vec<ConditionalFrame> = Vec::new();
    let mut next_mark = 0usize;
    let mut brace_depth = 0usize;
    let mut linkage_closes = BTreeSet::new();
    for (index, token) in tokens.iter().enumerate() {
        while next_mark < conditionals.len() && conditionals[next_mark].token_index <= index {
            apply_conditional(
                conditionals[next_mark],
                &mut frames,
                &mut brace_depth,
                &mut refusals,
            );
            next_mark += 1;
        }
        if token.text == "{" {
            // Raw C sources commonly retain an `extern "C"` wrapper behind
            // `#ifdef __cplusplus`; it is a linkage boundary, not a function.
            if index >= 2
                && tokens[index - 2].text == "extern"
                && tokens[index - 1].text == "__qcore_literal__"
                && let Some(close) = matching(tokens, index, "{", "}")
            {
                linkage_closes.insert(close);
                continue;
            }
            brace_depth += 1;
            continue;
        }
        if token.text == "}" {
            if linkage_closes.remove(&index) {
                continue;
            }
            brace_depth = brace_depth.saturating_sub(1);
            continue;
        }
        if brace_depth != 0 {
            continue;
        }
        if !is_identifier(&token.text)
            || matches!(token.text.as_str(), "if" | "while" | "for" | "switch")
            || tokens.get(index + 1).map(|next| next.text.as_str()) != Some("(")
        {
            continue;
        }
        if let Some(close) = matching(tokens, index + 1, "(", ")")
            && tokens.get(close + 1).map(|next| next.text.as_str()) == Some("{")
        {
            names.insert(token.text.clone());
        }
    }
    while next_mark < conditionals.len() {
        apply_conditional(
            conditionals[next_mark],
            &mut frames,
            &mut brace_depth,
            &mut refusals,
        );
        next_mark += 1;
    }
    if !frames.is_empty() {
        refusals.insert(ConditionalScanRefusal::UnstructuredGroup);
    }
    LegacyRawScan {
        definitions: names.into_iter().collect(),
        refusals: refusals.into_iter().collect(),
    }
}

/// Return the function definitions written in a source file before headers
/// and macros are expanded. This is the auditable scope used by
/// [`LegacyFunctionScope::MainSourceDefinitions`].
#[must_use]
pub fn legacy_c_source_functions(source: &str) -> Vec<String> {
    legacy_c_source_scan(source).definitions
}

/// Scan a source file for the function definitions written in it, before
/// headers and macros are expanded, reporting whether every conditional group
/// was decided exactly (unit **E01-RAW-COND**).
///
/// This is [`legacy_c_source_functions`] with its conditional-group grounds
/// kept, so a caller that publishes a completeness claim over the result can
/// refuse it. The definition list is identical.
#[must_use]
pub fn legacy_c_source_scan(source: &str) -> LegacyRawScan {
    let (tokens, conditionals) = tokenize_with_conditionals(source);
    function_names(&tokens, &conditionals)
}

/// Declaration specifiers that can end a parameter declarator without naming
/// it. If one of these is the last token, the parameter is unnamed and the body
/// cannot refer to it.
pub(super) const DECLARATION_SPECIFIERS: &[&str] = &[
    "void",
    "char",
    "short",
    "int",
    "long",
    "float",
    "double",
    "signed",
    "unsigned",
    "_Bool",
    "_Complex",
    "_Imaginary",
    "const",
    "volatile",
    "restrict",
    "__restrict",
    "__restrict__",
    "struct",
    "union",
    "enum",
    "static",
    "register",
    "inline",
    "_Atomic",
];

/// Formal parameter names of a defined function, in declaration order.
///
/// `Some(vec![Some(name), None, ..])` names each formal the body can refer to;
/// an inner `None` is an unnamed formal. The whole result is `None` whenever
/// the declarator list cannot be read *unambiguously* — variadic lists,
/// function-pointer parameters, K&R-style single-identifier declarators,
/// unnamed tagged types, and duplicate names all reject the function outright.
/// Misreading which declarator carries a name would attach custody to the wrong
/// argument, so this parser refuses everything it cannot pin down.
pub(crate) fn function_parameters(tokens: &[Token], function: &str) -> Option<Vec<Option<String>>> {
    for (index, token) in tokens.iter().enumerate() {
        if token.text != function || tokens.get(index + 1).map(|t| t.text.as_str()) != Some("(") {
            continue;
        }
        let close = matching(tokens, index + 1, "(", ")")?;
        if tokens.get(close + 1).map(|t| t.text.as_str()) != Some("{") {
            continue;
        }
        return parse_parameter_declarators(&tokens[index + 2..close]);
    }
    None
}

pub(super) fn parse_parameter_declarators(tokens: &[Token]) -> Option<Vec<Option<String>>> {
    let texts = tokens
        .iter()
        .map(|token| token.text.as_str())
        .collect::<Vec<_>>();
    // `(`/`)` mark a function-pointer or nested declarator; `.` is the first
    // token of a variadic `...`. Neither is read here.
    if texts
        .iter()
        .any(|text| matches!(*text, "(" | ")" | "{" | "}" | "."))
    {
        return None;
    }
    if texts.is_empty() {
        return Some(Vec::new());
    }
    let groups = texts.split(|text| *text == ",").collect::<Vec<_>>();
    if groups.len() == 1 && groups[0] == ["void"] {
        return Some(Vec::new());
    }
    let mut names = Vec::new();
    let mut seen = BTreeSet::new();
    for group in groups {
        let mut group = group;
        // Drop array declarator suffixes: `char buf[16]` names `buf`.
        while group.last() == Some(&"]") {
            let open = group.iter().rposition(|text| *text == "[")?;
            group = &group[..open];
        }
        if group.is_empty() {
            return None;
        }
        let last = *group.last()?;
        if !is_identifier(last) {
            // An abstract declarator such as `const char *`: unnamed.
            names.push(None);
            continue;
        }
        if DECLARATION_SPECIFIERS.contains(&last) {
            names.push(None);
            continue;
        }
        if group.len() < 2 {
            // `f(n)` (K&R) and `f(size_t)` are indistinguishable here.
            return None;
        }
        if matches!(group[group.len() - 2], "struct" | "union" | "enum") {
            // `struct foo` without a declarator: the identifier is the tag.
            names.push(None);
            continue;
        }
        if !seen.insert(last.to_owned()) {
            return None;
        }
        names.push(Some(last.to_owned()));
    }
    Some(names)
}

/// Names in `candidates` that the body of `function` calls directly.
pub(crate) fn direct_calls(
    tokens: &[Token],
    function: &str,
    candidates: &BTreeSet<String>,
) -> BTreeSet<String> {
    let mut called = BTreeSet::new();
    let Some((body_start, _)) = find_function_body(tokens, function) else {
        return called;
    };
    let Some(body_end) = matching(tokens, body_start, "{", "}") else {
        return called;
    };
    for index in body_start..body_end {
        if tokens.get(index + 1).map(|token| token.text.as_str()) != Some("(") {
            continue;
        }
        let name = &tokens[index].text;
        if candidates.contains(name) {
            called.insert(name.clone());
        }
    }
    called
}

pub(crate) fn matching(tokens: &[Token], start: usize, open: &str, close: &str) -> Option<usize> {
    let mut depth = 0usize;
    for (index, token) in tokens.iter().enumerate().skip(start) {
        if token.text == open {
            depth += 1;
        } else if token.text == close {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some(index);
            }
        }
    }
    None
}
