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

//! Card **CG30-DIALECTS-FLAGS-TYPES** (gateway rule 11y): the frontend's
//! dialect, target and type identities, and the extensions it refuses.
//!
//! The Clang gap analysis's I01, I02, I03, I05, N12, N13 and N14. Four shared
//! items live here, each named so other cards key on it rather than on a copy:
//!
//! * **Dialects** ([`DialectDifference`], [`dialect_differences`]). C17, C23
//!   and C2y are explicit [`CCompilerDialect`] identities. Each dialect's AST
//!   differences from the C11/GNU baseline the readers were written against are
//!   *registered* here, together, and the registry joins the evidence identity
//!   ([`dialect_identity_bytes`]): a verdict reached under one dialect is never
//!   the verdict of another. No difference is adopted. A registered difference
//!   is refused by name wherever a reader meets it (route G:
//!   [`refuse_dialect_differences`]; route T: [`dialect_refusal_in_ast`]), and a
//!   C23 digit separator, which the token reader cannot lex, refuses the whole
//!   unit ([`digit_separator_refusal`]).
//! * **Targets** ([`CTargetIdentity`], [`CTargetTriple`], [`CAbiName`],
//!   [`CDataModel`]). A structured `--target`/`-target` triple and a `-mabi=`
//!   name are parsed into closed vocabularies, transported verbatim to every
//!   Clang invocation and serialised into both the gateway request digest and
//!   the compile-task identity, so a changed target is a different evidence
//!   identity. Every reader takes the LP64 data model as a premise (the ABI
//!   reader says so in its projection table), so a target is admitted only when
//!   its data model is LP64; ILP32, LLP64 and an unknown model are refused by
//!   name. Integer widths per target are card CG20's (rule 11q); CG20 keys them
//!   on [`CTargetIdentity`]. There is no unrestricted flag passthrough.
//! * **Types** ([`CTypeId`]). One canonical identity per C type, built from
//!   Clang's canonical (desugared) spelling for routes G and T and from the
//!   specifier multiset for the lexical ABI reader, so `ul`, `unsigned long`
//!   and `long unsigned int` are one identity in all three. Type compatibility
//!   stays Clang's; the identity is a key, not a type checker.
//! * **Extensions** ([`CExtensionFamily`]). `_Complex`/`_Imaginary`, vector and
//!   matrix types, `_BitInt` and fixed point are refused by name in every
//!   dialect ([`extension_in_ast`], [`unit_spells_extension`]). None is
//!   adopted: the census found no use in an analysed definition's own types.
//!
//! Lean: `QCore.DialectTargetIdentity`.

use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value;

use crate::legacy_c::CCompilerDialect;

// ---------------------------------------------------------------------------
// Dialects
// ---------------------------------------------------------------------------

/// How a registered difference is detected in the gateway's token stream.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenPattern {
    /// One exact token spelling.
    Word(&'static str),
    /// Two adjacent `[` tokens (a standard attribute specifier).
    AttributeOpen,
    /// `break` or `continue` followed by an identifier (a named loop jump).
    LabelledJump,
    /// `if (` or `switch (` whose parentheses hold a `;` at depth one (a
    /// declaration in the condition).
    DeclarationCondition,
    /// A pp-number spelled with the `0o`/`0O` octal prefix.
    OctalPrefix,
}

/// One registered AST difference of a dialect from the C11/GNU baseline.
///
/// `standing` is always [`DifferenceStanding::Refused`] today: adopting one
/// means giving every reader its semantics, which needs measured demand.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DialectDifference {
    /// Stable key, used in the refusal and the evidence identity.
    pub key: &'static str,
    /// What the source spells.
    pub construct: &'static str,
    /// The Clang AST node or type spelling route T meets (`None` when the
    /// construct has no node of its own, such as a keyword spelling of a type
    /// Clang already had).
    pub clang_ast: Option<&'static str>,
    pub token: TokenPattern,
    pub standing: DifferenceStanding,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DifferenceStanding {
    Refused,
}

const fn refused(
    key: &'static str,
    construct: &'static str,
    clang_ast: Option<&'static str>,
    token: TokenPattern,
) -> DialectDifference {
    DialectDifference {
        key,
        construct,
        clang_ast,
        token,
        standing: DifferenceStanding::Refused,
    }
}

/// C23's registered differences (ISO/IEC 9899:2024 against C17/GNU17).
pub const C23_DIFFERENCES: &[DialectDifference] = &[
    refused(
        "c23-nullptr",
        "`nullptr`",
        Some("CXXNullPtrLiteralExpr"),
        TokenPattern::Word("nullptr"),
    ),
    refused(
        "c23-nullptr-t",
        "`nullptr_t`",
        Some("nullptr_t"),
        TokenPattern::Word("nullptr_t"),
    ),
    refused(
        "c23-true",
        "`true` keyword",
        Some("CXXBoolLiteralExpr"),
        TokenPattern::Word("true"),
    ),
    refused(
        "c23-false",
        "`false` keyword",
        Some("CXXBoolLiteralExpr"),
        TokenPattern::Word("false"),
    ),
    refused(
        "c23-bool",
        "`bool` keyword",
        Some("bool"),
        TokenPattern::Word("bool"),
    ),
    refused(
        "c23-constexpr",
        "`constexpr` object",
        Some("constexpr"),
        TokenPattern::Word("constexpr"),
    ),
    refused(
        "c23-typeof",
        "`typeof` keyword",
        None,
        TokenPattern::Word("typeof"),
    ),
    refused(
        "c23-typeof-unqual",
        "`typeof_unqual`",
        None,
        TokenPattern::Word("typeof_unqual"),
    ),
    refused(
        "c23-auto",
        "`auto` type inference",
        None,
        TokenPattern::Word("auto"),
    ),
    refused(
        "c23-static-assert",
        "`static_assert` keyword",
        None,
        TokenPattern::Word("static_assert"),
    ),
    refused(
        "c23-alignas",
        "`alignas` keyword",
        None,
        TokenPattern::Word("alignas"),
    ),
    refused(
        "c23-alignof",
        "`alignof` keyword",
        None,
        TokenPattern::Word("alignof"),
    ),
    refused(
        "c23-thread-local",
        "`thread_local` keyword",
        None,
        TokenPattern::Word("thread_local"),
    ),
    refused(
        "c23-attribute",
        "`[[…]]` attribute",
        None,
        TokenPattern::AttributeOpen,
    ),
    refused(
        "c23-decimal32",
        "`_Decimal32`",
        Some("_Decimal32"),
        TokenPattern::Word("_Decimal32"),
    ),
    refused(
        "c23-decimal64",
        "`_Decimal64`",
        Some("_Decimal64"),
        TokenPattern::Word("_Decimal64"),
    ),
    refused(
        "c23-decimal128",
        "`_Decimal128`",
        Some("_Decimal128"),
        TokenPattern::Word("_Decimal128"),
    ),
];

/// C2y's registered differences beyond C23 (the working draft as Clang 21
/// accepts it; a construct a later Clang accepts is still refused by its
/// pattern, whichever frontend identity reads it).
pub const C2Y_DIFFERENCES: &[DialectDifference] = &[
    refused(
        "c2y-countof",
        "`_Countof`",
        Some("_Countof"),
        TokenPattern::Word("_Countof"),
    ),
    refused(
        "c2y-countof-macro",
        "`countof`",
        Some("countof"),
        TokenPattern::Word("countof"),
    ),
    refused(
        "c2y-octal-prefix",
        "`0o` octal literal",
        None,
        TokenPattern::OctalPrefix,
    ),
    refused(
        "c2y-named-jump",
        "`break`/`continue` naming a loop",
        None,
        TokenPattern::LabelledJump,
    ),
    refused(
        "c2y-condition-declaration",
        "declaration in an `if`/`switch` condition",
        None,
        TokenPattern::DeclarationCondition,
    ),
    refused(
        "c2y-generic-type-operand",
        "`_Generic` (type operand form)",
        None,
        TokenPattern::Word("_Generic"),
    ),
];

/// Every registered difference of `dialect` from the C11/GNU baseline. Empty
/// for C11, C17 (a defect-report revision of C11 with no AST difference) and
/// the three GNU modes, which are the baseline the readers were written for.
pub fn dialect_differences(dialect: CCompilerDialect) -> Vec<&'static DialectDifference> {
    match dialect {
        CCompilerDialect::C11
        | CCompilerDialect::Gnu99
        | CCompilerDialect::Gnu11
        | CCompilerDialect::Gnu17
        | CCompilerDialect::C17 => Vec::new(),
        CCompilerDialect::C23 => C23_DIFFERENCES.iter().collect(),
        CCompilerDialect::C2y => C23_DIFFERENCES.iter().chain(C2Y_DIFFERENCES).collect(),
    }
}

impl CCompilerDialect {
    /// The `-std=` spelling, without the prefix.
    pub fn std_name(self) -> &'static str {
        match self {
            Self::C11 => "c11",
            Self::Gnu99 => "gnu99",
            Self::Gnu11 => "gnu11",
            Self::Gnu17 => "gnu17",
            Self::C17 => "c17",
            Self::C23 => "c23",
            Self::C2y => "c2y",
        }
    }

    /// The dialect a `-std=` value names, exactly. `c18`, `gnu2x`, `c2x` and
    /// every other alias stay unadmitted rather than being folded onto a
    /// neighbouring identity.
    pub fn from_std_name(name: &str) -> Option<Self> {
        Some(match name {
            "c11" => Self::C11,
            "gnu99" => Self::Gnu99,
            "gnu11" => Self::Gnu11,
            "gnu17" => Self::Gnu17,
            "c17" => Self::C17,
            "c23" => Self::C23,
            "c2y" => Self::C2y,
            _ => return None,
        })
    }
}

/// The dialect's contribution to the gateway's analysis-request digest: its
/// byte, then, for a dialect with registered differences, the registry
/// (`qcore.c-dialect-differences.v1`, each key length-prefixed). The four
/// baseline dialects keep exactly their one historical byte, so no existing
/// digest moves; C17 has its own byte.
pub fn dialect_identity_bytes(dialect: CCompilerDialect) -> Vec<u8> {
    let mut bytes = vec![match dialect {
        CCompilerDialect::C11 => 0,
        CCompilerDialect::Gnu99 => 1,
        CCompilerDialect::Gnu11 => 2,
        CCompilerDialect::Gnu17 => 3,
        CCompilerDialect::C17 => 4,
        CCompilerDialect::C23 => 5,
        CCompilerDialect::C2y => 6,
    }];
    let differences = dialect_differences(dialect);
    if !differences.is_empty() {
        bytes.extend_from_slice(b"qcore.c-dialect-differences.v1\0");
        for difference in differences {
            bytes.extend_from_slice(&(difference.key.len() as u64).to_be_bytes());
            bytes.extend_from_slice(difference.key.as_bytes());
        }
    }
    bytes
}

/// The registry's digest, for the compile-task identity. `None` for a dialect
/// with no registered difference (its identity is its name alone, as before).
pub fn dialect_differences_sha256(dialect: CCompilerDialect) -> Option<String> {
    (!dialect_differences(dialect).is_empty())
        .then(|| qcore_schemas::sha256_hex(&dialect_identity_bytes(dialect)))
}

/// The refusal a registered difference publishes.
pub fn dialect_refusal(dialect: CCompilerDialect, difference: &DialectDifference) -> String {
    format!(
        "{} ({}) is a registered {} difference that is not adopted (CG30)",
        difference.construct,
        difference.key,
        dialect.std_name()
    )
}

fn token_matches(pattern: TokenPattern, texts: &[&str], index: usize) -> bool {
    let text = texts[index];
    match pattern {
        TokenPattern::Word(word) => text == word,
        TokenPattern::AttributeOpen => text == "[" && texts.get(index + 1) == Some(&"["),
        TokenPattern::LabelledJump => {
            matches!(text, "break" | "continue")
                && texts.get(index + 1).is_some_and(|next| is_identifier(next))
        }
        TokenPattern::DeclarationCondition => {
            if !matches!(text, "if" | "switch") || texts.get(index + 1) != Some(&"(") {
                return false;
            }
            let mut depth = 0usize;
            for next in &texts[index + 1..] {
                match *next {
                    "(" | "[" | "{" => depth += 1,
                    ")" | "]" | "}" => {
                        depth = depth.saturating_sub(1);
                        if depth == 0 {
                            return false;
                        }
                    }
                    ";" if depth == 1 => return true,
                    _ => {}
                }
            }
            false
        }
        TokenPattern::OctalPrefix => {
            let bytes = text.as_bytes();
            bytes.len() > 2 && bytes[0] == b'0' && matches!(bytes[1], b'o' | b'O')
        }
    }
}

fn is_identifier(text: &str) -> bool {
    let mut characters = text.chars();
    characters
        .next()
        .is_some_and(|first| first == '_' || first.is_ascii_alphabetic())
        && characters.all(|character| character == '_' || character.is_ascii_alphanumeric())
}

/// The first registered, unadopted difference of `dialect` spelled in `texts`.
pub fn dialect_difference_in_tokens(
    dialect: CCompilerDialect,
    texts: &[&str],
) -> Option<&'static DialectDifference> {
    let differences = dialect_differences(dialect);
    if differences.is_empty() {
        return None;
    }
    (0..texts.len()).find_map(|index| {
        differences
            .iter()
            .copied()
            .find(|difference| token_matches(difference.token, texts, index))
    })
}

/// Route T: the first registered difference of `dialect` in a function's
/// Clang AST, by node kind, `constexpr` flag, trait name or type spelling.
pub fn dialect_refusal_in_ast(dialect: CCompilerDialect, function: &Value) -> Option<String> {
    let differences = dialect_differences(dialect);
    if differences.is_empty() {
        return None;
    }
    let mut found = None;
    walk(function, &mut |node| {
        if found.is_some() {
            return;
        }
        let kind = node["kind"].as_str().unwrap_or("");
        let trait_name = node["name"].as_str().unwrap_or("");
        let types = type_spellings(node);
        found = differences.iter().copied().find(|difference| {
            let Some(ast) = difference.clang_ast else {
                return false;
            };
            kind == ast
                || (ast == "constexpr" && node["constexpr"] == true)
                || (kind == "UnaryExprOrTypeTraitExpr" && trait_name == ast)
                || types
                    .iter()
                    .any(|spelling| type_words(spelling).any(|word| word == ast))
        });
    });
    found.map(|difference| dialect_refusal(dialect, difference))
}

/// Route G's thin hook: every defined function whose declaration or body
/// spells a registered, unadopted difference of `dialect` has its body
/// replaced by one call to a marked callee no environment can contract
/// (`qcore_dialect_refused__KEY`), *before* any reader, the W4 derivation or
/// certificate replay reads the stream. The function is then `Unknown` with no
/// derived summary, whatever the token reader would have made of the
/// construct, and never `Unsafe` on a reading the construct could have
/// misled. Returns the rewritten stream and each refused function's refusal.
pub(crate) fn refuse_dialect_differences(
    dialect: CCompilerDialect,
    tokens: Vec<crate::legacy_c::Token>,
    definitions: &[String],
) -> (Vec<crate::legacy_c::Token>, BTreeMap<String, String>) {
    let mut refusals = BTreeMap::new();
    if dialect_differences(dialect).is_empty() {
        return (tokens, refusals);
    }
    let mut tokens = tokens;
    for name in definitions {
        let Some((open, _)) = crate::legacy_c::find_function_body(&tokens, name) else {
            continue;
        };
        let Some(close) = crate::legacy_c::matching(&tokens, open, "{", "}") else {
            continue;
        };
        let start = declaration_start(&tokens, open);
        let texts = tokens[start..=close]
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        let Some(difference) = dialect_difference_in_tokens(dialect, &texts) else {
            continue;
        };
        let line = tokens[open].line;
        let marker = format!(
            "{DIALECT_REFUSED_PREFIX}{}",
            difference.key.replace('-', "_")
        );
        let replacement = ["{", marker.as_str(), "(", ")", ";", "}"]
            .into_iter()
            .map(|text| crate::legacy_c::Token {
                text: text.to_owned(),
                line,
            })
            .collect::<Vec<_>>();
        tokens.splice(open..=close, replacement);
        refusals.insert(name.clone(), dialect_refusal(dialect, difference));
    }
    (tokens, refusals)
}

/// The reserved prefix of route G's dialect-refusal marker callee
/// (`legacy_reserved_spelling` escapes any source identifier spelled with it).
pub(crate) const DIALECT_REFUSED_PREFIX: &str = "qcore_dialect_refused__";

/// The first token of the file-scope declaration whose body opens at `open`.
fn declaration_start(tokens: &[crate::legacy_c::Token], open: usize) -> usize {
    let mut depth = 0usize;
    for index in (0..open).rev() {
        match tokens[index].text.as_str() {
            ")" | "]" => depth += 1,
            "(" | "[" => depth = depth.saturating_sub(1),
            ";" | "}" if depth == 0 => return index + 1,
            _ => {}
        }
    }
    0
}

/// A C23/C2y digit separator (`1'000`) in the preprocessed text. The token
/// reader would lex its `'` as a character literal and lose the tokens up to
/// the next `'`, so a unit spelling one is refused whole under a dialect that
/// admits separators. Strings, character literals and identifiers are skipped
/// exactly as C lexes them; only a `'` inside a pp-number counts.
pub fn digit_separator_refusal(dialect: CCompilerDialect, source: &str) -> Option<String> {
    if !matches!(dialect, CCompilerDialect::C23 | CCompilerDialect::C2y) {
        return None;
    }
    let bytes = source.as_bytes();
    let mut cursor = 0;
    while cursor < bytes.len() {
        let byte = bytes[cursor];
        if byte == b'"' || byte == b'\'' {
            cursor = skip_quoted(bytes, cursor);
        } else if byte == b'_' || byte.is_ascii_alphabetic() {
            while cursor < bytes.len()
                && (bytes[cursor] == b'_' || bytes[cursor].is_ascii_alphanumeric())
            {
                cursor += 1;
            }
        } else if byte.is_ascii_digit()
            || (byte == b'.' && bytes.get(cursor + 1).is_some_and(u8::is_ascii_digit))
        {
            cursor += 1;
            while cursor < bytes.len() {
                let current = bytes[cursor];
                if current == b'\''
                    && bytes
                        .get(cursor + 1)
                        .is_some_and(|next| *next == b'_' || next.is_ascii_alphanumeric())
                {
                    return Some(format!(
                        "C23 digit separator (c23-digit-separator) is a registered {} difference \
                         that is not adopted (CG30)",
                        dialect.std_name()
                    ));
                }
                let exponent_sign = matches!(current, b'+' | b'-')
                    && matches!(bytes[cursor - 1], b'e' | b'E' | b'p' | b'P');
                if exponent_sign
                    || current == b'_'
                    || current == b'.'
                    || current.is_ascii_alphanumeric()
                {
                    cursor += 1;
                } else {
                    break;
                }
            }
        } else {
            cursor += 1;
        }
    }
    None
}

fn skip_quoted(bytes: &[u8], start: usize) -> usize {
    let quote = bytes[start];
    let mut cursor = start + 1;
    while cursor < bytes.len() {
        match bytes[cursor] {
            b'\\' => cursor += 2,
            b'\n' => return cursor + 1,
            current if current == quote => return cursor + 1,
            _ => cursor += 1,
        }
    }
    cursor
}

// ---------------------------------------------------------------------------
// Targets
// ---------------------------------------------------------------------------

/// The data model a target's C integer and pointer widths follow.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub enum CDataModel {
    /// `int` 32, `long` and pointers 64 (Linux, macOS and BSD on 64-bit
    /// architectures): the premise every reader takes today.
    Lp64,
    /// `int` and `long` 32, pointers 64 (64-bit Windows).
    Llp64,
    /// `int`, `long` and pointers 32.
    Ilp32,
}

impl CDataModel {
    pub fn name(self) -> &'static str {
        match self {
            Self::Lp64 => "LP64",
            Self::Llp64 => "LLP64",
            Self::Ilp32 => "ILP32",
        }
    }
}

/// A structured `--target` triple: the spelling Clang receives, verbatim,
/// with its architecture, operating system and environment read from a
/// closed vocabulary. `vendor` is recorded and decides nothing.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct CTargetTriple {
    spelled: String,
    architecture: String,
    operating_system: Option<String>,
    environment: Option<String>,
}

/// Architectures with a recorded pointer width: `(name, 64-bit)`.
const ARCHITECTURES: &[(&str, bool)] = &[
    ("x86_64", true),
    ("amd64", true),
    ("aarch64", true),
    ("arm64", true),
    ("arm64e", true),
    ("riscv64", true),
    ("powerpc64", true),
    ("powerpc64le", true),
    ("ppc64", true),
    ("ppc64le", true),
    ("s390x", true),
    ("mips64", true),
    ("mips64el", true),
    ("loongarch64", true),
    ("sparcv9", true),
    ("i386", false),
    ("i486", false),
    ("i586", false),
    ("i686", false),
    ("arm", false),
    ("armv6", false),
    ("armv7", false),
    ("armv7a", false),
    ("armv7s", false),
    ("armv7k", false),
    ("armv8", false),
    ("thumbv7", false),
    ("arm64_32", false),
    ("riscv32", false),
    ("powerpc", false),
    ("ppc", false),
    ("mips", false),
    ("mipsel", false),
    ("wasm32", false),
];

/// Operating systems with a recorded data-model rule.
const OPERATING_SYSTEMS: &[&str] = &[
    "linux", "darwin", "macos", "macosx", "ios", "freebsd", "netbsd", "openbsd", "windows",
    "win32", "none", "elf",
];

/// Environments with a recorded data-model rule.
const ENVIRONMENTS: &[&str] = &[
    "gnu",
    "gnueabi",
    "gnueabihf",
    "gnux32",
    "gnu_ilp32",
    "gnuabi64",
    "gnuabin32",
    "musl",
    "musleabi",
    "musleabihf",
    "msvc",
    "android",
    "eabi",
    "eabihf",
    "macabi",
    "simulator",
];

fn plain_word(text: &str) -> bool {
    !text.is_empty()
        && text
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'.')
}

impl CTargetTriple {
    /// Parse a `--target` value. Two to four `-`-separated components of
    /// `[A-Za-z0-9_.]`, the first a recorded architecture; every later
    /// component is a vendor, a recorded operating system (an OS version
    /// suffix such as `macosx14.0` is read as its OS) or a recorded
    /// environment. Anything else is refused: an unrecorded component could
    /// change the data model.
    pub fn parse(text: &str) -> Result<Self, String> {
        let components = text.split('-').collect::<Vec<_>>();
        if !(2..=4).contains(&components.len()) || !components.iter().all(|part| plain_word(part)) {
            return Err(format!(
                "unsupported compilation target `{text}`: a target is two to four plain components"
            ));
        }
        let architecture = components[0].to_owned();
        if !ARCHITECTURES.iter().any(|(name, _)| *name == architecture) {
            return Err(format!(
                "unsupported compilation target `{text}`: architecture `{architecture}` has no recorded data model"
            ));
        }
        let mut operating_system = None;
        let mut environment = None;
        for (position, component) in components.iter().enumerate().skip(1) {
            let os = OPERATING_SYSTEMS.iter().find(|os| {
                component.strip_prefix(**os).is_some_and(|version| {
                    version.is_empty()
                        || version
                            .bytes()
                            .all(|byte| byte.is_ascii_digit() || byte == b'.')
                })
            });
            if let Some(os) = os
                && operating_system.is_none()
            {
                operating_system = Some((*os).to_owned());
            } else if ENVIRONMENTS.contains(component) && environment.is_none() && position > 1 {
                environment = Some((*component).to_owned());
            } else if position == 1 && components.len() > 2 {
                // The vendor position (`pc`, `unknown`, `apple`, …).
            } else {
                return Err(format!(
                    "unsupported compilation target `{text}`: component `{component}` is not recorded"
                ));
            }
        }
        Ok(Self {
            spelled: text.to_owned(),
            architecture,
            operating_system,
            environment,
        })
    }

    pub fn spelled(&self) -> &str {
        &self.spelled
    }

    /// The triple's own data model, before any `-mabi` selection.
    pub fn data_model(&self) -> Option<CDataModel> {
        let wide = architecture_is_64_bit(&self.architecture)?;
        let environment = self.environment.as_deref();
        if matches!(environment, Some("gnux32" | "gnu_ilp32" | "gnuabin32")) {
            return Some(CDataModel::Ilp32);
        }
        if !wide {
            return Some(CDataModel::Ilp32);
        }
        match self.operating_system.as_deref() {
            Some("windows" | "win32") => Some(CDataModel::Llp64),
            Some(_) => Some(CDataModel::Lp64),
            // A 64-bit triple naming no operating system has no recorded model.
            None => None,
        }
    }
}

fn architecture_is_64_bit(architecture: &str) -> Option<bool> {
    ARCHITECTURES
        .iter()
        .find(|(name, _)| *name == architecture)
        .map(|(_, wide)| *wide)
}

/// The data model of an Apple `-arch` value (the existing two-word flag).
pub fn architecture_data_model(architecture: &str) -> Option<CDataModel> {
    match architecture {
        "arm64" | "arm64e" | "x86_64" | "x86_64h" => Some(CDataModel::Lp64),
        "i386" | "armv7" | "armv7s" | "armv7k" | "arm64_32" => Some(CDataModel::Ilp32),
        _ => None,
    }
}

/// A `-mabi=` value from the closed table: its data-model consequence, if it
/// has one (`None`: the ABI keeps the target's integer and pointer widths and
/// changes only the calling convention or floating-point passing).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct CAbiName(String);

const ABI_NAMES: &[(&str, Option<CDataModel>)] = &[
    ("sysv", None),
    ("ms", None),
    ("aapcs", None),
    ("aapcs-soft", None),
    ("darwinpcs", None),
    ("lp64", Some(CDataModel::Lp64)),
    ("lp64f", Some(CDataModel::Lp64)),
    ("lp64d", Some(CDataModel::Lp64)),
    ("lp64e", Some(CDataModel::Lp64)),
    ("lp64s", Some(CDataModel::Lp64)),
    ("elfv1", None),
    ("elfv2", None),
    ("ilp32", Some(CDataModel::Ilp32)),
    ("ilp32f", Some(CDataModel::Ilp32)),
    ("ilp32d", Some(CDataModel::Ilp32)),
    ("ilp32e", Some(CDataModel::Ilp32)),
    ("n32", Some(CDataModel::Ilp32)),
    ("o32", Some(CDataModel::Ilp32)),
];

impl CAbiName {
    pub fn parse(text: &str) -> Result<Self, String> {
        if ABI_NAMES.iter().any(|(name, _)| *name == text) {
            Ok(Self(text.to_owned()))
        } else {
            Err(format!(
                "unsupported compilation ABI `{text}`: no recorded data-model consequence"
            ))
        }
    }

    pub fn name(&self) -> &str {
        &self.0
    }

    pub fn data_model(&self) -> Option<CDataModel> {
        ABI_NAMES
            .iter()
            .find(|(name, _)| *name == self.0)
            .and_then(|(_, model)| *model)
    }
}

/// **The shared target identity** (named for card CG20, rule 11q, which keys
/// integer widths on it): the explicit target selections of one compile task.
/// `None` everywhere is the configured Clang's default target, recorded in the
/// compile-task identity's `ClangIdentity`.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct CTargetIdentity {
    pub triple: Option<CTargetTriple>,
    pub abi: Option<CAbiName>,
    /// The Apple `-arch` selection, when the command carries one.
    pub architecture: Option<String>,
}

impl CTargetIdentity {
    /// The data model the explicit selections fix. `Ok(None)`: no explicit
    /// selection fixes one, so the default target's model — the readers'
    /// standing premise — applies, exactly as before this card.
    pub fn data_model(&self) -> Result<Option<CDataModel>, String> {
        if self.triple.is_some() && self.architecture.is_some() {
            return Err(
                "unsupported compilation target: `--target` and `-arch` select a target twice"
                    .into(),
            );
        }
        let mut model = match (&self.triple, &self.architecture) {
            (Some(triple), _) => Some(triple.data_model().ok_or_else(|| {
                format!(
                    "unsupported compilation target `{}`: no recorded data model",
                    triple.spelled
                )
            })?),
            (None, Some(architecture)) => Some(architecture_data_model(architecture).ok_or_else(
                || format!("unsupported compilation target: `-arch {architecture}` has no recorded data model"),
            )?),
            (None, None) => None,
        };
        if let Some(abi_model) = self.abi.as_ref().and_then(CAbiName::data_model) {
            model = Some(abi_model);
        }
        Ok(model)
    }

    /// Admit the selections only under the LP64 premise every reader takes.
    pub fn admit(&self) -> Result<(), String> {
        match self.data_model()? {
            None | Some(CDataModel::Lp64) => Ok(()),
            Some(model) => Err(format!(
                "unsupported compilation target: data model {} is not the readers' LP64 premise; \
                 integer widths per target are card CG20's (rule 11q)",
                model.name()
            )),
        }
    }

    /// Admit the explicit selections a unit's own frontend flags carry,
    /// whoever built the unit: every `-arch` word separately, then the triple
    /// and ABI together (and with the first `-arch`). The gateway asks this
    /// before any reader, so a caller that bypasses the compile-database
    /// reader cannot have LP64 readers read an ILP32 or LLP64 unit. With every
    /// admitted unit LP64 by its own selection, the lexical ABI reader's
    /// default-target `long` question (`c_numeric::plain_long_is_lp64`) is
    /// never answered for a different data model than the unit's
    /// (CG20-ABI-READER-ARCH-FLAG).
    pub fn admit_flags<'a>(
        flags: impl IntoIterator<Item = &'a crate::legacy_c::CFrontendFlag>,
    ) -> Result<(), String> {
        use crate::legacy_c::CFrontendFlag;
        let mut identity = Self::default();
        for flag in flags {
            match flag {
                CFrontendFlag::Target(triple) => {
                    if identity.triple.replace(triple.clone()).is_some() {
                        return Err(
                            "unsupported compilation target: the unit selects a target twice"
                                .into(),
                        );
                    }
                }
                CFrontendFlag::Abi(abi) => {
                    if identity.abi.replace(abi.clone()).is_some() {
                        return Err(
                            "unsupported compilation ABI: the unit selects an ABI twice".into()
                        );
                    }
                }
                CFrontendFlag::Architecture(architecture) => {
                    Self {
                        architecture: Some(architecture.clone()),
                        ..Self::default()
                    }
                    .admit()?;
                    identity
                        .architecture
                        .get_or_insert_with(|| architecture.clone());
                }
                _ => {}
            }
        }
        identity.admit()
    }
}

// ---------------------------------------------------------------------------
// Canonical type identities
// ---------------------------------------------------------------------------

/// **The canonical type identity** shared by routes G and T and the lexical
/// ABI reader. Its content is Clang's canonical spelling of the type with
/// whitespace collapsed and the C23 keyword spelling `bool` read as `_Bool`
/// (one type, two dialect spellings). The lexical reader reaches the same
/// spelling from a specifier multiset through [`Self::from_specifiers`].
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct CTypeId(String);

/// The canonical arithmetic spellings Clang prints, by specifier multiset.
fn canonical_arithmetic(words: &[&str]) -> Option<&'static str> {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for word in words {
        let word = if *word == "bool" { "_Bool" } else { word };
        if !matches!(
            word,
            "signed"
                | "unsigned"
                | "char"
                | "short"
                | "int"
                | "long"
                | "float"
                | "double"
                | "_Bool"
                | "void"
                | "__int128"
        ) {
            return None;
        }
        *counts.entry(word).or_default() += 1;
    }
    let count = |word: &str| counts.get(word).copied().unwrap_or(0);
    let (signed, unsigned) = (count("signed"), count("unsigned"));
    if signed + unsigned > 1 {
        return None;
    }
    let others = |allowed: &[&str]| {
        counts
            .keys()
            .all(|word| allowed.contains(word) || matches!(*word, "signed" | "unsigned"))
    };
    let integer = |signed_name: &'static str, unsigned_name: &'static str| {
        Some(if unsigned == 1 {
            unsigned_name
        } else {
            signed_name
        })
    };
    match (
        count("void"),
        count("_Bool"),
        count("float"),
        count("double"),
        count("char"),
        count("short"),
        count("long"),
        count("int"),
        count("__int128"),
    ) {
        (1, 0, 0, 0, 0, 0, 0, 0, 0) if signed + unsigned == 0 => Some("void"),
        (0, 1, 0, 0, 0, 0, 0, 0, 0) if signed + unsigned == 0 => Some("_Bool"),
        (0, 0, 1, 0, 0, 0, 0, 0, 0) if signed + unsigned == 0 => Some("float"),
        (0, 0, 0, 1, 0, 0, 0, 0, 0) if signed + unsigned == 0 => Some("double"),
        (0, 0, 0, 1, 0, 0, 1, 0, 0) if signed + unsigned == 0 => Some("long double"),
        (0, 0, 0, 0, 1, 0, 0, 0, 0) if others(&["char"]) => Some(match (signed, unsigned) {
            (1, 0) => "signed char",
            (0, 1) => "unsigned char",
            _ => "char",
        }),
        (0, 0, 0, 0, 0, 1, 0, 0 | 1, 0) => integer("short", "unsigned short"),
        (0, 0, 0, 0, 0, 0, 1, 0 | 1, 0) => integer("long", "unsigned long"),
        (0, 0, 0, 0, 0, 0, 2, 0 | 1, 0) => integer("long long", "unsigned long long"),
        (0, 0, 0, 0, 0, 0, 0, 1, 0) => integer("int", "unsigned int"),
        (0, 0, 0, 0, 0, 0, 0, 0, 0) if signed + unsigned == 1 => integer("int", "unsigned int"),
        (0, 0, 0, 0, 0, 0, 0, 0, 1) => integer("__int128", "unsigned __int128"),
        _ => None,
    }
}

impl CTypeId {
    /// Normalise a Clang canonical spelling. A spelling made only of
    /// arithmetic specifiers (and `const`/`volatile`) is reordered into
    /// Clang's own order, which is the identity Clang already prints; every
    /// other spelling is kept as Clang printed it.
    pub fn from_canonical_spelling(text: &str) -> Option<Self> {
        let collapsed = text.split_whitespace().collect::<Vec<_>>();
        if collapsed.is_empty() {
            return None;
        }
        let qualifiers = collapsed
            .iter()
            .filter(|word| matches!(**word, "const" | "volatile"))
            .copied()
            .collect::<Vec<_>>();
        let rest = collapsed
            .iter()
            .filter(|word| !matches!(**word, "const" | "volatile"))
            .copied()
            .collect::<Vec<_>>();
        if let Some(arithmetic) = canonical_arithmetic(&rest) {
            let mut spelling = String::new();
            for qualifier in ["const", "volatile"] {
                if qualifiers.contains(&qualifier) {
                    spelling.push_str(qualifier);
                    spelling.push(' ');
                }
            }
            spelling.push_str(arithmetic);
            return Some(Self(spelling));
        }
        Some(Self(
            collapsed
                .iter()
                .map(|word| if *word == "bool" { "_Bool" } else { word })
                .collect::<Vec<_>>()
                .join(" "),
        ))
    }

    /// A Clang AST node's type: the desugared spelling when Clang gives one
    /// (typedefs resolved by Clang), else the spelling itself.
    pub fn from_clang_type(node: &Value) -> Option<Self> {
        let ty = &node["type"];
        ty["desugaredQualType"]
            .as_str()
            .or_else(|| ty["qualType"].as_str())
            .and_then(Self::from_canonical_spelling)
    }

    /// The lexical reader's specifier multiset (already typedef-resolved),
    /// optionally `const`-qualified, under `stars` pointer levels.
    pub fn from_specifiers(words: &[&str], stars: usize) -> Option<Self> {
        let const_qualified = words.contains(&"const");
        let core = words
            .iter()
            .filter(|word| !matches!(**word, "const"))
            .copied()
            .collect::<Vec<_>>();
        let base =
            canonical_arithmetic(&core)
                .map(str::to_owned)
                .or_else(|| match core.as_slice() {
                    [tag @ ("struct" | "union" | "enum"), name] if is_identifier(name) => {
                        Some(format!("{tag} {name}"))
                    }
                    _ => None,
                })?;
        let mut spelling = if const_qualified {
            format!("const {base}")
        } else {
            base
        };
        if stars > 0 {
            spelling.push(' ');
            spelling.push_str(&"*".repeat(stars));
        }
        Self::from_canonical_spelling(&spelling)
    }

    pub fn spelling(&self) -> &str {
        &self.0
    }

    /// The unadopted extension family this type belongs to, if any.
    pub fn extension(&self) -> Option<CExtensionFamily> {
        extension_of_spelling(&self.0)
    }
}

// ---------------------------------------------------------------------------
// Extensions
// ---------------------------------------------------------------------------

/// An extension type family. None is adopted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub enum CExtensionFamily {
    /// `_Complex`, `_Imaginary`, `__real__`/`__imag__`, imaginary literals.
    Complex,
    /// GCC/Clang vector, OpenCL ext-vector, Arm/RISC-V sizeless and matrix
    /// types, and their shuffle/convert/element operations.
    Vector,
    /// `_BitInt(N)` (and the older `_ExtInt(N)`).
    BitPrecise,
    /// ISO/IEC TR 18037 fixed point: `_Fract`, `_Accum`, `_Sat`.
    FixedPoint,
}

impl CExtensionFamily {
    pub fn name(self) -> &'static str {
        match self {
            Self::Complex => "complex",
            Self::Vector => "vector",
            Self::BitPrecise => "bit-precise integer",
            Self::FixedPoint => "fixed point",
        }
    }
}

/// The refusal an extension use publishes.
pub fn extension_refusal(family: CExtensionFamily, spelling: &str) -> String {
    format!(
        "C extension type `{spelling}` ({}) is not adopted (CG30)",
        family.name()
    )
}

fn type_words(spelling: &str) -> impl Iterator<Item = &str> {
    spelling
        .split(|character: char| !(character == '_' || character.is_ascii_alphanumeric()))
        .filter(|word| !word.is_empty())
}

fn extension_of_spelling(spelling: &str) -> Option<CExtensionFamily> {
    type_words(spelling).find_map(extension_of_word)
}

fn extension_of_word(word: &str) -> Option<CExtensionFamily> {
    match word {
        "_Complex" | "__complex__" | "_Imaginary" | "__real__" | "__imag__" | "__real"
        | "__imag" | "__builtin_complex" => Some(CExtensionFamily::Complex),
        "_BitInt" | "_ExtInt" => Some(CExtensionFamily::BitPrecise),
        "_Fract" | "_Accum" | "_Sat" => Some(CExtensionFamily::FixedPoint),
        "vector_size"
        | "__vector_size__"
        | "ext_vector_type"
        | "__ext_vector_type__"
        | "neon_vector_type"
        | "neon_polyvector_type"
        | "matrix_type"
        | "__matrix_type__"
        | "__builtin_shufflevector"
        | "__builtin_convertvector" => Some(CExtensionFamily::Vector),
        _ if word.starts_with("__rvv_") || (word.starts_with("__SV") && word.ends_with("_t")) => {
            Some(CExtensionFamily::Vector)
        }
        _ => None,
    }
}

/// Whether a pp-number spelling is an imaginary constant (`2.0i`, `1if`,
/// `0x1p3i`): no decimal, hexadecimal or binary digit, exponent or ordinary
/// suffix is an `i` or `j`, so a pp-number holding one is imaginary (or an
/// MSVC `i64`-style suffix, refused alike).
fn imaginary_literal(text: &str) -> bool {
    let bytes = text.as_bytes();
    let starts_number = bytes.first().is_some_and(u8::is_ascii_digit)
        || (bytes.first() == Some(&b'.') && bytes.get(1).is_some_and(u8::is_ascii_digit));
    starts_number
        && bytes
            .iter()
            .any(|byte| matches!(byte, b'i' | b'I' | b'j' | b'J'))
}

/// Whether any token of the unit spells an extension. A unit without one
/// cannot use an extension type, so route G's per-definition typed check is
/// skipped there (a sound prefilter: every extension type and operation has a
/// spelling in the preprocessed text).
pub(crate) fn unit_spells_extension(texts: &[&str]) -> bool {
    texts
        .iter()
        .any(|text| extension_of_word(text).is_some() || imaginary_literal(text))
}

const EXTENSION_NODES: &[(&str, CExtensionFamily)] = &[
    ("ImaginaryLiteral", CExtensionFamily::Complex),
    ("ShuffleVectorExpr", CExtensionFamily::Vector),
    ("ConvertVectorExpr", CExtensionFamily::Vector),
    ("ExtVectorElementExpr", CExtensionFamily::Vector),
    ("MatrixSubscriptExpr", CExtensionFamily::Vector),
    ("FixedPointLiteral", CExtensionFamily::FixedPoint),
];

/// The first extension use in a definition's Clang AST: a type of any node
/// (declared or computed) in an extension family, an extension-only node, or
/// a `__real`/`__imag` operator. `type_id` is the route's own canonical type
/// reader ([`CTypeId`]); the sugared spelling is read too, so a typedef name
/// Clang leaves undesugared is still seen through its canonical half.
pub fn extension_in_ast(
    function: &Value,
    type_id: &dyn Fn(&Value) -> Option<CTypeId>,
) -> Option<(CExtensionFamily, String)> {
    let mut found = None;
    walk(function, &mut |node| {
        if found.is_some() {
            return;
        }
        let kind = node["kind"].as_str().unwrap_or("");
        if let Some((_, family)) = EXTENSION_NODES.iter().find(|(name, _)| *name == kind) {
            found = Some((*family, kind.to_owned()));
            return;
        }
        if kind == "UnaryOperator"
            && let Some(opcode @ ("__real" | "__imag")) = node["opcode"].as_str()
        {
            found = Some((CExtensionFamily::Complex, opcode.to_owned()));
            return;
        }
        if let Some(id) = type_id(node)
            && let Some(family) = id.extension()
        {
            found = Some((family, id.spelling().to_owned()));
            return;
        }
        for spelling in type_spellings(node) {
            if let Some(family) = extension_of_spelling(spelling) {
                found = Some((family, spelling.to_owned()));
                return;
            }
        }
    });
    found
}

/// Route T's thin hook: the named refusal of a definition that uses an
/// extension or a registered dialect difference.
pub fn typed_source_refusal(dialect: CCompilerDialect, function: &Value) -> Option<String> {
    extension_in_ast(function, &crate::typed_source::canonical_type_id)
        .map(|(family, spelling)| extension_refusal(family, &spelling))
        .or_else(|| dialect_refusal_in_ast(dialect, function))
}

fn type_spellings(node: &Value) -> Vec<&str> {
    let ty = &node["type"];
    [ty["qualType"].as_str(), ty["desugaredQualType"].as_str()]
        .into_iter()
        .flatten()
        .collect()
}

fn walk(node: &Value, visit: &mut impl FnMut(&Value)) {
    match node {
        Value::Object(map) => {
            visit(node);
            for value in map.values() {
                if value.is_object() || value.is_array() {
                    walk(value, visit);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                walk(item, visit);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
#[path = "../tests/unit/c_frontend_identity.rs"]
mod tests;
