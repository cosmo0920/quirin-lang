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

//! Card **W45-PARENTHESISED-CALLEE**: a call whose callee is not spelled
//! `NAME (` (gateway constraint 10c).
//!
//! # The hole
//!
//! Every lexical call reader of the gateway — `find_call`, `contains_call`,
//! `nested_call`, the condition, return and neutral-position readers, the
//! contract-withdrawal and review-hold rewrites — recognises a call only as
//! the token pair `NAME (`. In C, `(f)(x)` calls `f` (the parentheses only
//! suppress a function-like macro), and `(c ? f : g)(x)`, `((T) f)(x)`,
//! `(&f)(x)` and `g()(x)` are calls too. None of them contains the pair, so
//! each was invisible to every reader, and the statement classifier's last
//! arm lowered a statement with no visible call, assignment, increment or
//! access to `NoOp`: `q_free(p); (q_free)(p);` was a certified `Safe` double
//! release. The same arm is reached by a `for` header's init and increment
//! clauses, which reach the classifier with their grouping tokens stripped
//! (`W4-EXPR-POS-FOR-CLAUSES`), so `q_free(p)` there was the two names
//! `q_free p`.
//!
//! # The rule
//!
//! 1. **Normalise** ([`normalise_unit`]). In every function body, a callee
//!    `(`ᵏ `NAME` `)`ᵏ immediately followed by `(` is rewritten to `NAME`,
//!    before any reader of the token stream runs, so the withdrawal and hold
//!    rewrites, W4 derivation and validation, the certificate recheck and
//!    every position of the classifier see `NAME (` — exactly the spelling C
//!    gives the same meaning. Only a group that *is* a callee is rewritten:
//!    `NAME` is not a keyword and not a type name this function trusts
//!    ([`TypeNames`]; `(T)(x)` is a cast), and the group is not a call's
//!    argument list (`g (f) (x)` calls `g(f)`'s result), which is decided by
//!    the token before it.
//! 2. **Refuse** ([`opaque_call`]). After normalisation, a `(` that applies a
//!    call to anything but a name — it follows `]`, or a `)` that closes an
//!    argument list or a parenthesised expression rather than a cast or a
//!    control header — refuses its position with [`REFUSAL`], wherever the
//!    position would otherwise have been admitted. A position already refused
//!    keeps its own refusal byte for byte.
//! 3. **`for` clauses** ([`for_clauses`], [`stripping_is_faithful`]). A
//!    clause whose stripped spelling is not faithful to its original — a
//!    bracket, or a group that is an argument list, a callee, a cast or wraps
//!    more than one name — is refused with [`FOR_CLAUSE_REFUSAL`] unless its
//!    stripped classification was already a refusal. Classifying the clause
//!    from its original spelling, as the condition already is, is the
//!    follow-on `W45-FOR-CLAUSE-ORIGINAL-SPELLING`.
//!
//! Only rule 1 can admit, and it admits exactly what the plain spelling
//! admits. Rules 2 and 3 only replace an admission by a refusal.
//! `QCore.ParenthesisedCallee` proves both of the model.

use std::collections::BTreeSet;

use super::{ConditionalMark, LegacyInstruction, Token};

/// The refusal of a call whose callee expression is not a name.
pub(crate) const REFUSAL: &str =
    "call through a callee expression that is not a name is outside the resource projection";

/// The refusal of a `for` clause whose stripped spelling lost its groups.
pub(crate) const FOR_CLAUSE_REFUSAL: &str =
    "for-loop clause with a call or grouping is not classified from its stripped spelling";

/// Type specifier keywords a cast's type name may contain.
const TYPE_SPECIFIERS: &[&str] = &[
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
    "__int128",
    "__signed",
    "__signed__",
    "_Float16",
    "_Float32",
    "_Float64",
    "_Float128",
    "_Float32x",
    "_Float64x",
    "__float128",
];

/// Qualifiers a cast's type name may contain.
const QUALIFIERS: &[&str] = &[
    "const",
    "volatile",
    "restrict",
    "__restrict",
    "__restrict__",
    "__const",
    "__volatile",
    "__volatile__",
    "_Atomic",
];

/// Storage classes and function specifiers that may begin a declaration.
const DECLARATION_PREFIXES: &[&str] = &[
    "static",
    "extern",
    "register",
    "auto",
    "typedef",
    "inline",
    "__inline",
    "__inline__",
    "_Thread_local",
    "__thread",
    "_Noreturn",
];

/// Keywords whose parenthesised group is a control header.
const HEADERS: &[&str] = &["if", "while", "for", "switch"];

/// Keywords after which a parenthesised group begins an expression.
const EXPRESSION_KEYWORDS: &[&str] = &["return", "else", "do", "__extension__"];

/// Keywords whose parenthesised group is a type, attribute or assembler
/// operand, never a callee's value.
const OPERAND_KEYWORDS: &[&str] = &[
    "__typeof",
    "__typeof__",
    "typeof",
    "__attribute__",
    "__attribute",
    "__asm__",
    "__asm",
    "asm",
    "__declspec",
    "_Alignas",
    "_Static_assert",
];

/// Every other keyword: never a callee name.
const OTHER_KEYWORDS: &[&str] = &[
    "break",
    "case",
    "continue",
    "default",
    "goto",
    "sizeof",
    "_Alignof",
    "alignof",
    "__alignof__",
    "_Generic",
    "struct",
    "union",
    "enum",
    "__real__",
    "__imag__",
    "__label__",
    "__auto_type",
    "bool",
];

pub(super) fn is_name(text: &str) -> bool {
    let mut bytes = text.bytes();
    bytes
        .next()
        .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_')
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

pub(super) fn is_keyword(text: &str) -> bool {
    [
        TYPE_SPECIFIERS,
        QUALIFIERS,
        DECLARATION_PREFIXES,
        HEADERS,
        EXPRESSION_KEYWORDS,
        OPERAND_KEYWORDS,
        OTHER_KEYWORDS,
    ]
    .iter()
    .any(|list| list.contains(&text))
}

/// The typedef names one function trusts as types.
///
/// A name is a type here when a file-scope typedef declaration of the unit
/// declares it ([`typedef_declarators`]) **and** nothing in the function's
/// parameters or body could be an object of the same name ([`object_use`]).
/// A typedef name the function may shadow with an object is not trusted:
/// `(T)(x)` there may call through the object, and it is read as a call, the
/// refusing reading.
#[derive(Clone, Debug, Default)]
pub(crate) struct TypeNames {
    names: BTreeSet<String>,
}

impl TypeNames {
    /// Read the trusted type names over `texts[range]`, one function's
    /// parameter list and body, with the whole unit's typedef declarators
    /// read afresh.
    pub(crate) fn read(texts: &[&str], range: std::ops::Range<usize>) -> Self {
        Self::read_with(&typedef_declarators(texts), texts, range)
    }

    /// [`Self::read`] with the unit's typedef declarators already read.
    pub(crate) fn read_with(
        declared: &BTreeSet<String>,
        texts: &[&str],
        range: std::ops::Range<usize>,
    ) -> Self {
        let range = range.start.min(texts.len())..range.end.min(texts.len());
        let mut candidates = BTreeSet::new();
        let mut shadowed = BTreeSet::new();
        for index in range {
            let text = texts[index];
            if !is_name(text) || is_keyword(text) || !declared.contains(text) {
                continue;
            }
            candidates.insert(text);
            if object_use(texts, index) {
                shadowed.insert(text);
            }
        }
        Self {
            names: candidates
                .difference(&shadowed)
                .map(|name| (*name).to_owned())
                .collect(),
        }
    }

    fn is_type(&self, name: &str) -> bool {
        self.names.contains(name)
    }
}

/// The names file-scope typedef declarations of the unit declare, read by
/// declarator position. The declaration model's typedef reader
/// (`TypedAccessModel::declares_typedef`) reads only a name that is a
/// typedef's last token, at any depth, so it misses a function-pointer typedef
/// (`typedef void (*rel_t)(void *);`) and a typedef with a trailing attribute
/// (`typedef long long __v2di __attribute__ ((__vector_size__ (16)));`, the
/// vector types of the compiler's intrinsic headers, whose bodies cast with
/// `(__m128i)(x)`). A name counts only in a declarator position of a
/// `typedef` declaration, outside every brace and bracket:
///
/// * followed by `;`, `,`, `[`, `(` or an attribute, not in any group;
/// * or alone after `*`s in a declarator group, `( * NAME )`, where the group
///   is not a parameter list (its `(` follows no name, `)` or `]`).
///
/// A tag (`struct T`) and a parameter name never count. Reading a name that is
/// not a type as one would read `(f)(x)` as a cast, so every position here is
/// a declarator position C gives the typedef name to; a declarator this
/// reader does not recognise is simply absent, which reads its casts as calls,
/// the refusing direction.
pub(crate) fn typedef_declarators(texts: &[&str]) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut depth = 0usize;
    for start in 0..texts.len() {
        match texts[start] {
            "{" => depth += 1,
            "}" => depth = depth.saturating_sub(1),
            _ => {}
        }
        // Only a file-scope typedef: a block-scope one names a type in its
        // own block alone, and elsewhere the name may be a function.
        if texts[start] != "typedef" || depth != 0 {
            continue;
        }
        let Some(end) = crate::typed_field_access::statement_end(texts, start) else {
            continue;
        };
        let mut groups: Vec<usize> = Vec::new();
        let mut braces = 0usize;
        let mut brackets = 0usize;
        for index in start + 1..end {
            let text = texts[index];
            match text {
                "(" => groups.push(index),
                ")" => {
                    groups.pop();
                }
                "{" => braces += 1,
                "}" => braces = braces.saturating_sub(1),
                "[" => brackets += 1,
                "]" => brackets = brackets.saturating_sub(1),
                _ => {}
            }
            if braces > 0 || brackets > 0 || !is_name(text) || is_keyword(text) {
                continue;
            }
            let previous = texts[index - 1];
            if matches!(previous, "struct" | "union" | "enum") {
                continue;
            }
            let next = texts.get(index + 1).copied().unwrap_or(";");
            let declarator = match groups.last() {
                None => matches!(
                    next,
                    ";" | "," | "[" | "(" | "__attribute__" | "__attribute"
                ),
                Some(&open) => {
                    let before = texts[open - 1];
                    let parameter_list =
                        before == ")" || before == "]" || (is_name(before) && !is_keyword(before));
                    !parameter_list
                        && next == ")"
                        && texts[open + 1..index].iter().all(|text| *text == "*")
                        && open + 1 < index
                }
            };
            if declarator {
                names.insert(text.to_owned());
            }
        }
    }
    names
}

/// Whether the name at `index` may be an ordinary identifier denoting an
/// object: followed by a token only an object (or a declared object) can be
/// followed by. A tag (`struct T`) and a member (`s.T`, `p->T`) are other name
/// spaces and are skipped. Over-approximating is the refusing direction.
fn object_use(texts: &[&str], index: usize) -> bool {
    let previous = index.checked_sub(1).map(|at| texts[at]);
    if matches!(previous, Some("struct" | "union" | "enum" | "."))
        || (previous == Some(">") && index >= 2 && texts[index - 2] == "-")
    {
        return false;
    }
    match texts.get(index + 1).copied() {
        Some("=" | ";" | "," | "[" | ".") => true,
        // `T (*fp)(…)` declares `fp`; an object named `T` would also have a
        // declaration of its own, which one of the other arms reads.
        Some("(") => texts.get(index + 2) != Some(&"*"),
        Some("-") => matches!(texts.get(index + 2).copied(), Some(">" | "-")),
        Some("+") => texts.get(index + 2) == Some(&"+"),
        Some(")") => {
            !matches!(previous, Some("(")) && !previous.is_some_and(|p| QUALIFIERS.contains(&p))
        }
        _ => false,
    }
}

/// Whether `inner` is a type name: specifiers, qualifiers, tags, trusted
/// typedef names and `*`, with at least one specifier. Abstract declarators
/// with their own groups are not read, which reads such a cast as an
/// expression: the refusing direction.
fn is_type_name(inner: &[&str], types: &TypeNames) -> bool {
    let mut specifier = false;
    let mut index = 0;
    while index < inner.len() {
        let text = inner[index];
        if TYPE_SPECIFIERS.contains(&text) || types.is_type(text) {
            specifier = true;
        } else if matches!(text, "struct" | "union" | "enum") {
            if !inner.get(index + 1).is_some_and(|tag| is_name(tag)) {
                return false;
            }
            specifier = true;
            index += 1;
        } else if !QUALIFIERS.contains(&text) && text != "*" {
            return false;
        }
        index += 1;
    }
    specifier
}

/// What a parenthesised group `texts[open..=close]` is, from the token before
/// it and, for a primary group, its contents.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Group {
    /// `if (…)`, `while (…)`, `for (…)`, `switch (…)`.
    Header,
    /// `__typeof__ (…)`, `__attribute__ (…)` and the like.
    Operand,
    /// A call's argument list: after a name, `]`, or a `)` that closes an
    /// argument list or a parenthesised expression.
    Arguments,
    /// A cast's type name.
    Cast,
    /// A parenthesised expression (or declarator).
    Expression,
}

fn open_of(texts: &[&str], close: usize) -> Option<usize> {
    let mut depth = 0usize;
    for index in (0..=close).rev() {
        match texts[index] {
            ")" => depth += 1,
            "(" => {
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

fn group(texts: &[&str], open: usize, close: usize, types: &TypeNames) -> Group {
    match open.checked_sub(1).map(|at| texts[at]) {
        Some(before) if HEADERS.contains(&before) => Group::Header,
        Some(before) if OPERAND_KEYWORDS.contains(&before) => Group::Operand,
        Some(before) if is_name(before) && !is_keyword(before) => Group::Arguments,
        Some("]") => Group::Arguments,
        Some(")") => {
            match open_of(texts, open - 1).map(|previous| group(texts, previous, open - 1, types)) {
                Some(Group::Header | Group::Cast | Group::Operand) => {
                    primary(texts, open, close, types)
                }
                // An unmatched `)` is read as a call's: the refusing reading.
                _ => Group::Arguments,
            }
        }
        _ => primary(texts, open, close, types),
    }
}

fn primary(texts: &[&str], open: usize, close: usize, types: &TypeNames) -> Group {
    if is_type_name(&texts[open + 1..close], types) {
        Group::Cast
    } else {
        Group::Expression
    }
}

/// Whether the `(` at `index` applies a call to a callee that is not a name.
fn is_opaque_call_operator(texts: &[&str], index: usize, types: &TypeNames) -> bool {
    if texts[index] != "(" || index == 0 {
        return false;
    }
    match texts[index - 1] {
        "]" => true,
        ")" => match open_of(texts, index - 1) {
            Some(open) => matches!(
                group(texts, open, index - 1, types),
                Group::Arguments | Group::Expression
            ),
            None => true,
        },
        _ => false,
    }
}

/// Whether `texts` (one complete statement, condition or return expression)
/// applies a call to a callee expression that is not a name.
///
/// A declaration's own declarators are exempt — `void (*cb)(void *);` is a
/// declarator, not a call — outside its initialisers and array bounds, which
/// are evaluated (C11 6.7.6.2p5, 6.7.9).
pub(crate) fn opaque_call(texts: &[&str], types: &TypeNames) -> bool {
    let declaration = is_declaration(texts, types);
    let mut parentheses = 0usize;
    let mut brackets = 0usize;
    let mut initialiser = false;
    for index in 0..texts.len() {
        let exempt = declaration && !initialiser && brackets == 0;
        if !exempt && is_opaque_call_operator(texts, index, types) {
            return true;
        }
        match texts[index] {
            "(" => parentheses += 1,
            ")" => parentheses = parentheses.saturating_sub(1),
            "[" => brackets += 1,
            "]" => brackets = brackets.saturating_sub(1),
            "=" if parentheses == 0 && brackets == 0 => initialiser = true,
            "," if parentheses == 0 && brackets == 0 => initialiser = false,
            _ => {}
        }
    }
    false
}

fn is_declaration(texts: &[&str], types: &TypeNames) -> bool {
    texts
        .iter()
        .find(|text| **text != "__extension__")
        .is_some_and(|first| {
            TYPE_SPECIFIERS.contains(first)
                || QUALIFIERS.contains(first)
                || DECLARATION_PREFIXES.contains(first)
                || matches!(*first, "struct" | "union" | "enum")
                || types.is_type(first)
        })
}

impl super::Parser<'_> {
    /// Rule 2 over one complete statement, condition or return expression.
    pub(super) fn opaque_callee(&self, tokens: &[Token]) -> bool {
        let texts = tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        opaque_call(&texts, &self.callee_types)
    }

    /// Rule 2's statement hook: `instruction`, unless it admits a statement
    /// that applies a call to a callee expression that is not a name. A
    /// replaced admission hands back the sites and element lowerings it
    /// consumed, so every other statement keeps its numbering.
    pub(super) fn refuse_opaque_call(
        &mut self,
        tokens: &[Token],
        instruction: LegacyInstruction,
        saved: &super::probe_state::ParserProbeState,
    ) -> LegacyInstruction {
        if matches!(instruction, LegacyInstruction::Unknown { .. }) || !self.opaque_callee(tokens) {
            return instruction;
        }
        self.restore_probe_state(saved);
        let texts = tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        LegacyInstruction::Unknown {
            description: REFUSAL.to_owned(),
            writes: self.refused_writes(&texts),
        }
    }

    /// Rule 3: a `for` clause's stripped classification, unless its original
    /// spelling carried grouping tokens the stripped one lost and the
    /// stripped classification admitted it.
    pub(super) fn classify_for_clause(
        &mut self,
        stripped: &[Token],
        original: Option<&[Token]>,
    ) -> LegacyInstruction {
        let saved = self.probe_state();
        let instruction = self.classify_simple(stripped);
        // Card SCALAR-STATEMENT-FORMS (rule 41): a clause refused with one of
        // its five refusals, read from the original spelling (a clause has no
        // fallback table, so this is asked after every route of the clause).
        if let Some(admitted) = self.scalar_form_clause(original.unwrap_or(stripped), &instruction)
        {
            return admitted;
        }
        let grouped = original.is_none_or(|original| {
            let texts = original
                .iter()
                .map(|token| token.text.as_str())
                .collect::<Vec<_>>();
            !stripping_is_faithful(&texts, &self.callee_types)
        });
        if !grouped || matches!(instruction, LegacyInstruction::Unknown { .. }) {
            return instruction;
        }
        self.restore_probe_state(&saved);
        let texts = original
            .unwrap_or(stripped)
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        LegacyInstruction::Unknown {
            description: FOR_CLAUSE_REFUSAL.to_owned(),
            writes: self.refused_writes(&texts),
        }
    }
}

/// Whether stripping every grouping token from `texts` (one `for` clause)
/// leaves an expression C gives the same meaning: no bracket, and every
/// parenthesised group only wraps one name (`(elm) = (tmpelm)`, the
/// increment of `TAILQ_FOREACH_SAFE`), which designates what the name does
/// (C11 6.5.1p5). A group after a name, `)` or `]` is an argument list, a
/// group before `(` a callee, and a group around a type name a cast; each
/// makes the stripped spelling unfaithful.
pub(crate) fn stripping_is_faithful(texts: &[&str], types: &TypeNames) -> bool {
    for (index, text) in texts.iter().enumerate() {
        match *text {
            "[" | "]" => return false,
            "(" => {
                let Some(close) = close_of(texts, index) else {
                    return false;
                };
                let inner = texts[index + 1..close]
                    .iter()
                    .filter(|text| !matches!(**text, "(" | ")"))
                    .collect::<Vec<_>>();
                let single_name = matches!(inner.as_slice(),
                    [name] if is_name(name) && !is_keyword(name) && !types.is_type(name));
                let arguments = index.checked_sub(1).is_some_and(|before| {
                    matches!(texts[before], ")" | "]") || is_name(texts[before])
                });
                let callee = texts.get(close + 1) == Some(&"(");
                if !single_name || arguments || callee {
                    return false;
                }
            }
            _ => {}
        }
    }
    true
}

fn close_of(texts: &[&str], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (index, text) in texts.iter().enumerate().skip(open) {
        match *text {
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

/// A `for` header's three clauses **with their grouping tokens intact**, in
/// the same split as `split_for_clauses`.
pub(crate) fn for_clauses(tokens: &[Token]) -> Option<[Vec<Token>; 3]> {
    let mut clauses = vec![Vec::new()];
    let mut parentheses = 0usize;
    let mut brackets = 0usize;
    for token in tokens {
        match token.text.as_str() {
            "(" => parentheses += 1,
            ")" => parentheses = parentheses.checked_sub(1)?,
            "[" => brackets += 1,
            "]" => brackets = brackets.checked_sub(1)?,
            ";" if parentheses == 0 && brackets == 0 => {
                clauses.push(Vec::new());
                continue;
            }
            _ => {}
        }
        clauses.last_mut()?.push(token.clone());
    }
    clauses.try_into().ok()
}

/// One rewritten callee, for the census and the unit tests.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Rewrite {
    pub(crate) function: String,
    pub(crate) line: usize,
    pub(crate) callee: String,
    /// How many parentheses enclosed the name.
    pub(crate) depth: usize,
}

/// Rule 1 over one translation unit's token stream: every function body's
/// parenthesised name callees rewritten to the bare name, and the conditional
/// marks moved onto the tokens they preceded. `None` when nothing changes.
pub(crate) fn normalise_unit(
    tokens: &[Token],
    conditionals: &[ConditionalMark],
) -> Option<(Vec<Token>, Vec<ConditionalMark>, Vec<Rewrite>)> {
    let texts = tokens
        .iter()
        .map(|token| token.text.as_str())
        .collect::<Vec<_>>();
    // Cheap pre-filter: no `NAME ) (` or `) ) (`, no rewrite.
    if !texts.windows(3).any(|window| {
        window[1] == ")" && window[2] == "(" && (is_name(window[0]) || window[0] == ")")
    }) {
        return None;
    }
    let declared = typedef_declarators(&texts);
    let mut removed = vec![false; tokens.len()];
    let mut rewrites = Vec::new();
    let mut seen = BTreeSet::new();
    for function in super::function_names(tokens, conditionals).definitions {
        let Some((body_start, _)) = super::find_function_body(tokens, &function) else {
            continue;
        };
        let Some(body_end) = super::matching(tokens, body_start, "{", "}") else {
            continue;
        };
        if !seen.insert(body_start) {
            continue;
        }
        let start = super::function_parameter_range(tokens, &function, body_start)
            .map_or(body_start, |(open, _)| open);
        let types = TypeNames::read_with(&declared, &texts, start..body_end + 1);
        for (at, depth) in callee_sites(&texts, body_start + 1..body_end, &types) {
            for offset in 1..=depth {
                removed[at - offset] = true;
                removed[at + offset] = true;
            }
            rewrites.push(Rewrite {
                function: function.clone(),
                line: tokens[at].line,
                callee: texts[at].to_owned(),
                depth,
            });
        }
    }
    if rewrites.is_empty() {
        return None;
    }
    let mut before = Vec::with_capacity(tokens.len() + 1);
    let mut count = 0usize;
    for flag in &removed {
        before.push(count);
        count += usize::from(*flag);
    }
    before.push(count);
    let kept = tokens
        .iter()
        .zip(&removed)
        .filter(|(_, removed)| !**removed)
        .map(|(token, _)| token.clone())
        .collect();
    let marks = conditionals
        .iter()
        .map(|mark| ConditionalMark {
            token_index: mark.token_index - before[mark.token_index.min(tokens.len())],
            directive: mark.directive,
            span: mark.span,
        })
        .collect();
    Some((kept, marks, rewrites))
}

/// [`normalise_unit`] over an owned stream, handing it back unchanged when
/// nothing is rewritten.
pub(crate) fn normalise_owned(
    tokens: Vec<Token>,
    conditionals: Vec<ConditionalMark>,
) -> (Vec<Token>, Vec<ConditionalMark>) {
    match normalise_unit(&tokens, &conditionals) {
        Some((tokens, conditionals, _rewrites)) => {
            #[cfg(feature = "indirect-census")]
            census::record(&_rewrites);
            (tokens, conditionals)
        }
        None => (tokens, conditionals),
    }
}

/// The same rewrite for an entry point that holds a borrowed stream with no
/// conditional marks.
pub(crate) fn normalise_tokens(tokens: &[Token]) -> Option<Vec<Token>> {
    normalise_unit(tokens, &[]).map(|(tokens, _, _)| tokens)
}

/// Card CG05-ELABORATOR-CALL-FOOTPRINTS: rule 1's rewrite over one Clang
/// node's own tokens, so that the elaborator can match a node against a
/// statement the token stream already normalised. Only the node's own tokens
/// are read, so no type name is trusted and a `(T)(x)` cast may be stripped
/// too; the caller uses the result only to match against an already
/// normalised statement (where such a cast kept its parentheses), so a wrong
/// strip can only fail to match.
pub(crate) fn strip_callee_parentheses(texts: &[&str]) -> Vec<String> {
    // A leading `;` gives every group a token before it.
    let mut padded = Vec::with_capacity(texts.len() + 1);
    padded.push(";");
    padded.extend_from_slice(texts);
    let types = TypeNames::default();
    let mut removed = vec![false; padded.len()];
    for (at, depth) in callee_sites(&padded, 1..padded.len(), &types) {
        for offset in 1..=depth {
            removed[at - offset] = true;
            removed[at + offset] = true;
        }
    }
    padded
        .iter()
        .zip(&removed)
        .skip(1)
        .filter(|(_, removed)| !**removed)
        .map(|(text, _)| (*text).to_owned())
        .collect()
}

/// The parenthesised name callees in `texts[range]`: each name's index and
/// how many parentheses enclose it.
fn callee_sites(
    texts: &[&str],
    range: std::ops::Range<usize>,
    types: &TypeNames,
) -> Vec<(usize, usize)> {
    let mut sites = Vec::new();
    for at in range.clone() {
        let name = texts[at];
        if !is_name(name) || is_keyword(name) || types.is_type(name) {
            continue;
        }
        let closing = texts[at + 1..range.end]
            .iter()
            .take_while(|text| **text == ")")
            .count();
        let opening = texts[range.start..at]
            .iter()
            .rev()
            .take_while(|text| **text == "(")
            .count();
        if closing == 0 || closing > opening || texts.get(at + closing + 1) != Some(&"(") {
            continue;
        }
        let open = at - closing;
        if !callee_position(texts, open, types) {
            continue;
        }
        sites.push((at, closing));
    }
    sites
}

/// Whether a parenthesised group opening at `open` is in a position where a
/// following `(` makes it a callee: not a call's argument list, and not a
/// cast's operand after anything but a cast or a control header.
fn callee_position(texts: &[&str], open: usize, types: &TypeNames) -> bool {
    let before = texts[open - 1];
    match before {
        ")" => open_of(texts, open - 1).is_some_and(|previous| {
            matches!(
                group(texts, previous, open - 1, types),
                Group::Header | Group::Cast
            )
        }),
        "]" => false,
        _ if is_name(before) => EXPRESSION_KEYWORDS.contains(&before),
        _ => !before
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_digit() || byte == b'"' || byte == b'\''),
    }
}

/// The instrumented census of rule 1's rewrites: one JSON line per rewritten
/// callee, to the path [`census::PARENTHESISED_CALLEE_CENSUS_PATH`] names. A
/// measurement, not a rule: behind the off-by-default `indirect-census`
/// feature, written after the rewrite was decided, and read only by the
/// card's scripts. An entry point that tokenizes a unit more than once writes
/// its rewrites more than once; the scripts deduplicate on the record.
#[cfg(feature = "indirect-census")]
pub mod census {
    use std::fs::File;
    use std::io::{BufWriter, Write};
    use std::sync::Mutex;

    use super::Rewrite;

    /// The environment variable naming the census stream.
    pub const PARENTHESISED_CALLEE_CENSUS_PATH: &str = "QCORE_PARENTHESISED_CALLEE_CENSUS";

    static SINK: Mutex<Option<Option<BufWriter<File>>>> = Mutex::new(None);

    pub(crate) fn record(rewrites: &[Rewrite]) {
        let Ok(mut guard) = SINK.lock() else {
            return;
        };
        let opened = guard.get_or_insert_with(|| {
            let path = std::env::var_os(PARENTHESISED_CALLEE_CENSUS_PATH)?;
            File::create(path).ok().map(BufWriter::new)
        });
        let Some(writer) = opened.as_mut() else {
            return;
        };
        let unit_seq = crate::indirect_census::unit_seq();
        for rewrite in rewrites {
            let line = serde_json::json!({
                "unit_seq": unit_seq,
                "function": rewrite.function,
                "line": rewrite.line,
                "callee": rewrite.callee,
                "depth": rewrite.depth,
            });
            let _ = writeln!(writer, "{line}");
        }
        let _ = writer.flush();
    }
}

#[cfg(test)]
#[path = "../tests/unit/legacy_parenthesised_callee.rs"]
mod tests;
