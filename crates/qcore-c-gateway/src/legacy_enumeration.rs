//! Complete the main-source function *denominator* (unit **E01-F1**).
//!
//! `docs/reports/remaining-e01/README.md` boundary **E01-B1**: the audited
//! function inventory was a raw token scan of the unpreprocessed main source
//! (`legacy_c_source_functions`). Two real definition shapes escaped it
//! entirely — no obligation *and no residual `Unknown` row*:
//!
//! * a definition produced by macro expansion, whose text exists only after
//!   preprocessing (`DEFN(run)` where `DEFN` expands to a whole definition);
//! * a K&R definition, whose parameters are an identifier list followed by
//!   separate declarations before the body.
//!
//! A third cause was *found* by implementing this module: the raw tokenizer
//! treated every `#` as a directive to end of line, so a `#` stringification
//! operator on a macro's **continuation** line swallowed the rest of that
//! line — closing brace included. The raw scan's file-scope brace depth then
//! never returned to zero and every later definition in the file was invisible
//! to it. On the pinned corpus that hid 146 definitions of `librdkafka`'s
//! `src/rdkafka.c`, which reported 19 of its 165 definitions
//! (`docs/reports/remaining-e01-f1/README.md` §2.2). E01-F1 covered it here;
//! unit **E01-F1-RAW** then repaired the tokenizer itself
//! (`docs/reports/remaining-e01-f1-raw/README.md`), so that cause no longer
//! reaches [`SourceDefinitionInventory::absent_from_raw_scan`] and this module
//! covers the macro-expansion and K&R causes it was written for.
//!
//! This module enumerates all of these and says, per definition, whether any
//! implemented analysis reads it. It **completes the denominator; it does not
//! extend the analyzer**. A newly enumerated definition is analysed only when
//! the unmodified analyzer already finds it in the preprocessed translation
//! unit (a definition that is ordinary after expansion is one of those);
//! everything else is an explicit `Unknown` row with a classified
//! [`ReasonCategory`].
//!
//! # Mechanism
//!
//! Macro expansion is decided by the preprocessor, not by a token scan, so the
//! enumeration reads a *second, enumeration-only* Clang preprocessing of the
//! same unit run **with** line markers (`-E`, no `-P`). Each definition in that
//! output is attributed to the file the marker names, so a definition spelled
//! in a header stays out of scope (boundary **E01-B2**, unchanged) while a
//! definition produced by expanding a macro *at a main-source line* is in
//! scope. The unit's published `preprocessed_source` — the `-E -P` text every
//! certificate, identity and diagnostic line number is bound to — is not
//! touched by this run.
//!
//! # Fail-closed
//!
//! The enumeration never guesses. When the expansion is unavailable, the
//! output does not name its primary input, a file-scope delimiter has no
//! match, or a file-scope construct cannot be classified as a definition or a
//! declaration, the inventory's *completeness claim is refused* with an
//! explicit ground ([`EnumerationRefusal`]). A refused claim is a scoped
//! `Unknown`: callers must not publish a `Safe` unit verdict over a definition
//! set they could not establish.
//!
//! # Monotonicity
//!
//! [`SourceDefinitionInventory::definitions`] is a superset of the historical
//! raw scan by construction: the raw names are unioned in, never filtered.
//! A definition guarded out by `#if 0` therefore keeps its historical row.

use std::collections::{BTreeMap, BTreeSet};
use std::process::Stdio;

use qcore_schemas::reason::ReasonCategory;

use super::{
    C11TranslationUnit, ConditionalScanRefusal, LegacyCError, Token, is_identifier,
    legacy_c_source_scan, matching,
};

/// The shape of an enumerated definition the resource analyzer cannot read.
///
/// "Unanalyzable" is scoped to the resource projection — the route whose
/// `Safe`/`Unsafe`/`Unknown` unit verdicts this inventory feeds. A separately
/// configured capability that reads Clang's typed AST rather than this token
/// stream may well derive a contract for the same definition, and does; that is
/// a capability's own inventory, not this one's.
///
/// This is deliberately a closed enumeration: an "unanalyzable" row asserts a
/// *named* form, never an unexplained absence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum UnanalyzableForm {
    /// A K&R definition: `f(a, b) int a; int b; { … }`. The analyzer's body
    /// finder requires `)` immediately followed by `{`, so it never reads one.
    IdentifierListParameters,
}

impl UnanalyzableForm {
    /// Stable report spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::IdentifierListParameters => "identifier-list-parameters",
        }
    }

    /// The free-text reason carried by the row.
    #[must_use]
    pub const fn reason(self) -> &'static str {
        match self {
            Self::IdentifierListParameters => {
                "enumerated definition uses an identifier-list (K&R) parameter list, which the resource analyzer's body finder does not read"
            }
        }
    }

    /// The closed §L21.1 category. A K&R body is a construct of the analysed
    /// source outside the modeled grammar.
    #[must_use]
    pub const fn category(self) -> ReasonCategory {
        match self {
            Self::IdentifierListParameters => ReasonCategory::UnsupportedSyntax,
        }
    }
}

/// One enumerated definition that carries no analysis of its own.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct UnanalyzableDefinition {
    function: String,
    form: UnanalyzableForm,
}

impl UnanalyzableDefinition {
    /// The enumerated definition's name.
    #[must_use]
    pub fn function(&self) -> &str {
        &self.function
    }

    /// Which named form kept it out of every implemented analysis.
    #[must_use]
    pub const fn form(&self) -> UnanalyzableForm {
        self.form
    }
}

/// Why an inventory does not claim to be the complete main-source definition
/// set. Every ground is mechanical; none is a judgement call.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EnumerationRefusal {
    /// The enumeration-only line-marked preprocessing did not run, so a
    /// definition that exists only after expansion cannot be seen at all.
    ExpansionUnavailable,
    /// The line-marked output does not open with a line marker naming its
    /// primary input, so no definition can be attributed to the main source.
    PrimaryInputUnidentified,
    /// A file-scope `(` or `{` has no match, so definition boundaries cannot be
    /// established in this token stream.
    UnbalancedDelimiters,
    /// A file-scope `name(identifier-list)` is followed by neither a body, a
    /// `;`, nor a declaration list this scanner reads, so whether it is a
    /// definition cannot be decided.
    UnclassifiedFileScopeConstruct,
    /// The raw half of the union could not establish a conditional group's
    /// contribution to file-scope brace depth, so a definition guarded out of
    /// the expansion — the only definitions the raw half contributes — may be
    /// missing from the union after that group (unit **E01-RAW-COND**).
    ConditionalArmDepthDisagreement,
    /// The raw half met a conditional group it cannot read at all: an `#elif`,
    /// `#else` or `#endif` with no open group, an `#elif`/`#else` after an
    /// `#else`, or a group never closed (unit **E01-RAW-COND**).
    UnstructuredConditionalGroup,
}

impl EnumerationRefusal {
    /// Stable report spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ExpansionUnavailable => "expansion-unavailable",
            Self::PrimaryInputUnidentified => "primary-input-unidentified",
            Self::UnbalancedDelimiters => "unbalanced-delimiters",
            Self::UnclassifiedFileScopeConstruct => "unclassified-file-scope-construct",
            Self::ConditionalArmDepthDisagreement => "conditional-arm-depth-disagreement",
            Self::UnstructuredConditionalGroup => "unstructured-conditional-group",
        }
    }

    /// The ground a raw-scan conditional refusal contributes to the inventory.
    const fn from_conditional(refusal: ConditionalScanRefusal) -> Self {
        match refusal {
            ConditionalScanRefusal::ArmDepthDisagreement => Self::ConditionalArmDepthDisagreement,
            ConditionalScanRefusal::UnstructuredGroup => Self::UnstructuredConditionalGroup,
        }
    }

    /// The free-text reason carried by the refusal row.
    #[must_use]
    pub const fn reason(self) -> &'static str {
        match self {
            Self::ExpansionUnavailable => {
                "enumeration completeness refused: macro expansion for this unit is unavailable, so a definition produced only by preprocessing cannot be enumerated"
            }
            Self::PrimaryInputUnidentified => {
                "enumeration completeness refused: the line-marked preprocessing does not name its primary input, so main-source attribution is unavailable"
            }
            Self::UnbalancedDelimiters => {
                "enumeration completeness refused: a file-scope delimiter has no match, so definition boundaries cannot be established"
            }
            Self::UnclassifiedFileScopeConstruct => {
                "enumeration completeness refused: a file-scope construct is neither a definition nor a declaration this scanner reads, so definition boundaries cannot be established"
            }
            Self::ConditionalArmDepthDisagreement => {
                "enumeration completeness refused: a conditional group's arms leave different file-scope brace depths in the raw scan, and deciding which arm survives is conditional evaluation, so a definition guarded out of the expansion may be missing after that group"
            }
            Self::UnstructuredConditionalGroup => {
                "enumeration completeness refused: a conditional group cannot be read by the raw scan — an `#elif`, `#else` or `#endif` with no open group, or a group never closed"
            }
        }
    }

    /// The closed §L21.1 category.
    ///
    /// The first two grounds are failures of the *analysis environment* — a
    /// tool did not run, or produced output the transport cannot read — and
    /// say nothing about the analysed source. The last two name a construct in
    /// the analysed token stream that the scanner's grammar does not cover.
    #[must_use]
    pub const fn category(self) -> ReasonCategory {
        match self {
            Self::ExpansionUnavailable | Self::PrimaryInputUnidentified => {
                ReasonCategory::EnvironmentUnavailable
            }
            Self::UnbalancedDelimiters
            | Self::UnclassifiedFileScopeConstruct
            | Self::ConditionalArmDepthDisagreement
            | Self::UnstructuredConditionalGroup => ReasonCategory::UnsupportedSyntax,
        }
    }
}

/// The enumerated main-source definition inventory of one translation unit.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SourceDefinitionInventory {
    definitions: Vec<String>,
    unanalyzable: Vec<UnanalyzableDefinition>,
    absent_from_raw_scan: Vec<String>,
    refusals: Vec<EnumerationRefusal>,
}

impl SourceDefinitionInventory {
    /// Every enumerated main-source definition, sorted and deduplicated.
    ///
    /// This is both the reported denominator and the analyzer's selection: the
    /// analyzer intersects it with the definitions it can find in the
    /// preprocessed unit, so an [`unanalyzable`](Self::unanalyzable) name is
    /// inert there and stays in `not_projected`.
    #[must_use]
    pub fn definitions(&self) -> &[String] {
        &self.definitions
    }

    /// Enumerated definitions no implemented analysis reads, with their forms.
    #[must_use]
    pub fn unanalyzable(&self) -> &[UnanalyzableDefinition] {
        &self.unanalyzable
    }

    /// Prototype-shaped definitions the historical raw token scan did not see;
    /// a subset of [`definitions`](Self::definitions), disjoint from that scan.
    ///
    /// The name is deliberately literal rather than "macro-expanded", because
    /// this list does not distinguish *why* the raw scan missed a definition.
    /// E01-F1 measured two reasons:
    ///
    /// * the definition genuinely exists only after macro expansion — the
    ///   E01-B1 case, which is why this module exists;
    /// * the definition is written literally in the main source but the raw
    ///   scan lost it to a lexical defect. Unit **E01-F1-RAW** repaired the
    ///   measured instance of that second reason — a `#` on a macro
    ///   *continuation* line read as a directive, swallowing a closing brace
    ///   and desynchronising file-scope brace depth for the rest of the file
    ///   (`docs/reports/remaining-e01-f1-raw/README.md`) — so the second reason
    ///   no longer contributes on the pinned corpus. It is not asserted to be
    ///   impossible: the name stays literal precisely so a future lexical gap
    ///   lands here rather than being mislabeled.
    ///
    /// Either way these are main-source definitions that carried no row before,
    /// and they are ordinary definitions in the preprocessed unit, so the
    /// unmodified analyzer reads them.
    #[must_use]
    pub fn absent_from_raw_scan(&self) -> &[String] {
        &self.absent_from_raw_scan
    }

    /// The grounds on which the completeness claim was refused. Empty means
    /// the claim stands.
    #[must_use]
    pub fn refusals(&self) -> &[EnumerationRefusal] {
        &self.refusals
    }

    /// Whether this inventory claims to be the complete main-source definition
    /// set.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.refusals.is_empty()
    }

    /// Whether a `Safe` unit verdict may be published over this inventory.
    ///
    /// Fail-closed: a definition set that is not established, or that carries
    /// a definition no analysis read, cannot support a whole-unit `Safe` claim
    /// over the main-source scope (AGENTS.md invariants 1 and 2).
    #[must_use]
    pub fn supports_unit_safe(&self) -> bool {
        self.is_complete() && self.unanalyzable.is_empty()
    }

    /// The reason a `Safe` unit verdict is withheld, if it is.
    #[must_use]
    pub fn withheld_safe_reason(&self) -> Option<String> {
        if let Some(refusal) = self.refusals.first() {
            return Some(refusal.reason().to_owned());
        }
        self.unanalyzable.first().map(|definition| {
            format!(
                "enumerated definition `{}` is not analysed: {}",
                definition.function,
                definition.form.reason()
            )
        })
    }

    /// Build an inventory that carries only the historical raw scan, with an
    /// explicit refusal ground. Used when the expansion cannot be read.
    fn raw_only(raw: &RawHalf, refusal: EnumerationRefusal) -> Self {
        let mut refusals = raw.refusals.clone();
        refusals.insert(refusal);
        Self {
            definitions: raw.names.clone(),
            unanalyzable: Vec::new(),
            absent_from_raw_scan: Vec::new(),
            refusals: refusals.into_iter().collect(),
        }
    }
}

/// The raw token scan of the unpreprocessed main source: the half of the union
/// that keeps a `#if 0` definition in the denominator, together with the
/// grounds on which its conditional groups could not be decided exactly.
struct RawHalf {
    names: Vec<String>,
    refusals: BTreeSet<EnumerationRefusal>,
}

/// Enumerate one unit's main-source definitions.
///
/// Never fails: a unit whose expansion cannot be read still returns the
/// historical raw scan, with the completeness claim explicitly refused.
#[must_use]
pub fn enumerate_source_definitions(unit: &C11TranslationUnit<'_>) -> SourceDefinitionInventory {
    let scan = legacy_c_source_scan(unit.source);
    let raw = RawHalf {
        names: scan.definitions().to_vec(),
        refusals: scan
            .refusals()
            .iter()
            .map(|refusal| EnumerationRefusal::from_conditional(*refusal))
            .collect(),
    };
    match preprocess_with_line_markers(unit) {
        Ok(marked) => inventory_from_marked(&raw, &marked),
        Err(_) => {
            SourceDefinitionInventory::raw_only(&raw, EnumerationRefusal::ExpansionUnavailable)
        }
    }
}

/// Preprocess the unit a second time, keeping Clang's line markers.
///
/// This is the same command `validate_unrestricted_c11` builds, minus `-P` and
/// minus the syntax phase. Its output is used only to attribute definitions to
/// files; it is never published, never digested and never analysed, so the
/// unit's `-E -P` `preprocessed_source` — and every identity bound to it — is
/// unchanged by this module.
fn preprocess_with_line_markers(unit: &C11TranslationUnit<'_>) -> Result<String, LegacyCError> {
    let mut command = crate::clang::command().map_err(LegacyCError::ProductionParserUnavailable)?;
    command.args(["-x", "c", unit.dialect.clang_argument(), "-E"]);
    for include in &unit.include_paths {
        command.arg(format!("-I{include}"));
    }
    for definition in &unit.definitions {
        command.arg(format!("-D{definition}"));
    }
    for option in &unit.preprocessor_options {
        option.append_to(&mut command);
    }
    let piped = unit.append_primary_input(&mut command)?;
    let mut child = command
        .stdin(if piped { Stdio::piped() } else { Stdio::null() })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| LegacyCError::ProductionParserUnavailable(error.to_string()))?;
    if piped {
        use std::io::Write;
        child
            .stdin
            .take()
            .ok_or_else(|| LegacyCError::ProductionParserUnavailable("missing stdin".into()))?
            .write_all(unit.source.as_bytes())
            .map_err(|error| LegacyCError::ProductionParserUnavailable(error.to_string()))?;
    }
    let output = child
        .wait_with_output()
        .map_err(|error| LegacyCError::ProductionParserUnavailable(error.to_string()))?;
    if !output.status.success() {
        return Err(LegacyCError::ProductionParserRejected(
            String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// A token of line-marked preprocessed output together with the file the
/// markers attribute it to. `file` indexes [`MarkedStream::files`].
struct MarkedStream {
    tokens: Vec<Token>,
    files: Vec<usize>,
    /// The quoted spelling of the primary input, exactly as the first line
    /// marker writes it, or `None` when the output does not open with one.
    primary: Option<String>,
    /// Distinct file spellings in first-appearance order; `files` indexes it.
    names: Vec<String>,
}

/// Lex line-marked preprocessed output.
///
/// The lexical rules are the gateway tokenizer's, including its C11 5.1.1.2
/// phase-2 line splicing and its 6.10 rule that only a *line-initial* `#`
/// introduces a directive (unit **E01-F1-RAW**): `//` and `/* */` comments are
/// dropped, string and character literals collapse to `__qcore_literal__`,
/// identifiers and numbers are single tokens and every other byte — a
/// non-line-initial `#` included — is its own token. The one difference is the
/// directive branch, which *reads* a `# <line> "<file>"` marker instead of
/// discarding the line; a directive that is not a marker (`#pragma`) is
/// discarded exactly as before.
/// `the_two_tokenizers_agree_on_marker_free_input` pins that both tokenizers
/// produce identical token text on marker-free input, so the copy cannot drift
/// unnoticed.
fn tokenize_marked(source: &str) -> MarkedStream {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut files = Vec::new();
    let mut names: Vec<String> = Vec::new();
    let mut index: BTreeMap<String, usize> = BTreeMap::new();
    let mut current = usize::MAX;
    let mut primary = None;
    let mut cursor = 0;
    let mut line = 1usize;
    let mut at_line_start = true;
    while cursor < bytes.len() {
        let byte = bytes[cursor];
        if let Some(width) = super::line_splice_width(bytes, cursor) {
            line += 1;
            cursor += width;
        } else if byte == b'\n' {
            line += 1;
            cursor += 1;
            at_line_start = true;
        } else if byte == b' ' || byte == b'\t' || byte == b'\r' || byte == 0x0b || byte == 0x0c {
            cursor += 1;
        } else if byte == b'/' && bytes.get(cursor + 1) == Some(&b'/') {
            cursor += 2;
            while cursor < bytes.len() && bytes[cursor] != b'\n' {
                if let Some(width) = super::line_splice_width(bytes, cursor) {
                    line += 1;
                    cursor += width;
                } else {
                    cursor += 1;
                }
            }
        } else if byte == b'/' && bytes.get(cursor + 1) == Some(&b'*') {
            cursor += 2;
            while cursor + 1 < bytes.len() && !(bytes[cursor] == b'*' && bytes[cursor + 1] == b'/')
            {
                if bytes[cursor] == b'\n' {
                    line += 1;
                    at_line_start = true;
                }
                cursor += 1;
            }
            cursor = (cursor + 2).min(bytes.len());
        } else if byte == b'#' && at_line_start {
            let start = cursor;
            while cursor < bytes.len() && bytes[cursor] != b'\n' {
                if let Some(width) = super::line_splice_width(bytes, cursor) {
                    line += 1;
                    cursor += width;
                } else {
                    cursor += 1;
                }
            }
            if let Some(name) = line_marker_file(&source[start..cursor]) {
                let next = index.len();
                let slot = *index.entry(name.clone()).or_insert_with(|| {
                    names.push(name.clone());
                    next
                });
                current = slot;
                if primary.is_none() && tokens.is_empty() {
                    primary = Some(name);
                }
            }
        } else if byte == b'"' || byte == b'\'' {
            at_line_start = false;
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
            files.push(current);
        } else if byte.is_ascii_alphabetic() || byte == b'_' {
            at_line_start = false;
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
            files.push(current);
        } else if byte.is_ascii_digit() {
            at_line_start = false;
            let start = cursor;
            cursor += 1;
            while cursor < bytes.len() && bytes[cursor].is_ascii_alphanumeric() {
                cursor += 1;
            }
            tokens.push(Token {
                text: source[start..cursor].to_owned(),
                line,
            });
            files.push(current);
        } else {
            at_line_start = false;
            tokens.push(Token {
                text: char::from(byte).to_string(),
                line,
            });
            files.push(current);
            cursor += 1;
        }
    }
    MarkedStream {
        tokens,
        files,
        primary,
        names,
    }
}

/// Read the quoted file spelling of `# <digits> "<file>" …`.
///
/// The quoted body is returned exactly as written, escapes included: it is
/// used only for identity comparison against the primary marker, so nothing is
/// gained by unescaping and a wrong unescaping could merge two files.
fn line_marker_file(directive: &str) -> Option<String> {
    let rest = directive.strip_prefix('#')?.trim_start();
    let digits = rest
        .split(|c: char| c.is_whitespace())
        .next()
        .filter(|word| !word.is_empty() && word.bytes().all(|b| b.is_ascii_digit()))?;
    let after = rest[digits.len()..].trim_start();
    let body = after.strip_prefix('"')?;
    let mut name = String::new();
    let mut bytes = body.char_indices();
    while let Some((_, character)) = bytes.next() {
        match character {
            '\\' => {
                let (_, escaped) = bytes.next()?;
                name.push('\\');
                name.push(escaped);
            }
            '"' => return Some(name),
            _ => name.push(character),
        }
    }
    None
}

/// Which definition shape a file-scope `name(` construct turned out to be.
enum FileScopeConstruct {
    /// `name(…) { … }`: the shape the analyzer's body finder reads.
    PrototypeDefinition,
    /// `name(a, b) int a; … { … }`: enumerated, never analysed.
    IdentifierListDefinition,
    /// `name(…) ;` — a declaration, and every other well-formed file-scope use
    /// of a parenthesised name (a function-pointer declarator, an attribute).
    NotADefinition,
    /// Boundaries could not be established here.
    Unclassified(EnumerationRefusal),
}

fn inventory_from_marked(raw: &RawHalf, marked: &str) -> SourceDefinitionInventory {
    let stream = tokenize_marked(marked);
    let Some(primary) = stream.primary.as_ref() else {
        return SourceDefinitionInventory::raw_only(
            raw,
            EnumerationRefusal::PrimaryInputUnidentified,
        );
    };
    let Some(primary_index) = stream.names.iter().position(|name| name == primary) else {
        return SourceDefinitionInventory::raw_only(
            raw,
            EnumerationRefusal::PrimaryInputUnidentified,
        );
    };
    let mut prototype = BTreeSet::new();
    let mut identifier_list = BTreeSet::new();
    let mut refusals = raw.refusals.clone();
    let tokens = &stream.tokens;
    let mut depth = 0usize;
    let mut linkage_closes = BTreeSet::new();
    for index in 0..tokens.len() {
        let text = tokens[index].text.as_str();
        if text == "{" {
            // Same `extern "C" { … }` linkage allowance the raw scan makes.
            if index >= 2
                && tokens[index - 2].text == "extern"
                && tokens[index - 1].text == "__qcore_literal__"
                && let Some(close) = matching(tokens, index, "{", "}")
            {
                linkage_closes.insert(close);
                continue;
            }
            depth += 1;
            continue;
        }
        if text == "}" {
            if linkage_closes.remove(&index) {
                continue;
            }
            depth = depth.saturating_sub(1);
            continue;
        }
        if depth != 0 || stream.files[index] != primary_index {
            continue;
        }
        if !is_identifier(text)
            || matches!(text, "if" | "while" | "for" | "switch")
            || tokens.get(index + 1).map(|next| next.text.as_str()) != Some("(")
        {
            continue;
        }
        let previous = index
            .checked_sub(1)
            .map(|before| tokens[before].text.as_str());
        match classify_file_scope(tokens, index, previous) {
            FileScopeConstruct::PrototypeDefinition => {
                prototype.insert(text.to_owned());
            }
            FileScopeConstruct::IdentifierListDefinition => {
                identifier_list.insert(text.to_owned());
            }
            FileScopeConstruct::NotADefinition => {}
            FileScopeConstruct::Unclassified(refusal) => {
                refusals.insert(refusal);
            }
        }
    }
    let raw_names = raw.names.iter().cloned().collect::<BTreeSet<_>>();
    let absent_from_raw_scan = prototype
        .difference(&raw_names)
        .cloned()
        .collect::<Vec<_>>();
    // Monotone by construction: the historical raw names are unioned in, never
    // filtered, so a `#if 0` definition keeps its historical row.
    let mut definitions = raw_names
        .union(&prototype)
        .cloned()
        .collect::<BTreeSet<_>>();
    let unanalyzable = identifier_list
        .difference(&definitions)
        .map(|function| UnanalyzableDefinition {
            function: function.clone(),
            form: UnanalyzableForm::IdentifierListParameters,
        })
        .collect::<Vec<_>>();
    definitions.extend(identifier_list);
    SourceDefinitionInventory {
        definitions: definitions.into_iter().collect(),
        unanalyzable,
        absent_from_raw_scan,
        refusals: refusals.into_iter().collect(),
    }
}

/// Whether `previous` — the token immediately before a file-scope name — can
/// end the declaration specifiers of a definition.
///
/// This gates the K&R arm only, and it exists because an identifier list is
/// indistinguishable from an *expression* argument list: `sizeof(a), sizeof(b)`
/// inside a file-scope initialiser table has the same token shape as
/// `f(a, b) int a; { … }`. A definition's name is preceded by its return type
/// (an identifier or a `*`), by nothing, or — for a pre-C99 implicit `int`
/// return — by the `;` or `}` that ended the previous declaration. It is never
/// preceded by `,`, `{`, `=`, `(`, `[` or an operator. Refusing to enter the
/// K&R arm from an expression context is what keeps the completeness claim from
/// being refused across ordinary initialiser tables; a real declarator that the
/// declaration-list reader cannot read still refuses.
fn starts_a_declaration(previous: Option<&str>) -> bool {
    match previous {
        None => true,
        Some(";" | "}" | "*") => true,
        Some(text) => is_identifier(text),
    }
}

/// Decide what a file-scope `tokens[name] (` construct is.
fn classify_file_scope(
    tokens: &[Token],
    name: usize,
    previous: Option<&str>,
) -> FileScopeConstruct {
    let Some(close) = matching(tokens, name + 1, "(", ")") else {
        return FileScopeConstruct::Unclassified(EnumerationRefusal::UnbalancedDelimiters);
    };
    match tokens.get(close + 1).map(|token| token.text.as_str()) {
        Some("{") => return FileScopeConstruct::PrototypeDefinition,
        // An old-style declaration `int f(a, b);` and an ordinary prototype
        // both end here: neither is a definition.
        Some(";") | None => return FileScopeConstruct::NotADefinition,
        // A *declarator suffix*, not a K&R declaration list. `typedef T (name)
        // (args);` and a `T (name)(args)` function-pointer parameter both put a
        // parenthesised name in front of another declarator; a K&R body's
        // declaration list can never begin with `(` or `[`. Measured on the
        // pinned corpus: without this arm, `librdkafka`'s `rdkafka_admin.c`
        // (three function typedefs) and `rdkafka_partition.c` (function-pointer
        // parameters) refused their completeness claim on a well-formed idiom.
        Some("(" | "[") => return FileScopeConstruct::NotADefinition,
        _ => {}
    }
    // A definition's name is an identifier, never a declaration specifier
    // keyword. `unsigned int (hash)(const void *)` puts `int` in the name
    // position; reading it as a K&R name is a category error.
    if DECLARATION_SPECIFIERS.contains(&tokens[name].text.as_str()) {
        return FileScopeConstruct::NotADefinition;
    }
    if !starts_a_declaration(previous) {
        // An expression context: this parenthesised name is an operand, not a
        // declarator, so it is neither a definition nor an ambiguity.
        return FileScopeConstruct::NotADefinition;
    }
    if !is_identifier_list(&tokens[name + 2..close]) {
        // A function-pointer declarator, an attribute, an initialiser — a
        // parameter list carrying types is not a K&R declarator list, so this
        // construct is not an unread definition.
        return FileScopeConstruct::NotADefinition;
    }
    // K&R: one or more declarations, each a run of tokens free of `(`, `)` and
    // `{` terminated by `;`, and then the body.
    let mut cursor = close + 1;
    loop {
        let start = cursor;
        while let Some(token) = tokens.get(cursor) {
            match token.text.as_str() {
                ";" => break,
                "(" | ")" | "{" | "}" => {
                    return FileScopeConstruct::Unclassified(
                        EnumerationRefusal::UnclassifiedFileScopeConstruct,
                    );
                }
                _ => cursor += 1,
            }
        }
        if tokens.get(cursor).is_none() || cursor == start {
            // Ran out of input, or an empty declaration: not a K&R body.
            return FileScopeConstruct::Unclassified(
                EnumerationRefusal::UnclassifiedFileScopeConstruct,
            );
        }
        cursor += 1;
        match tokens.get(cursor).map(|token| token.text.as_str()) {
            Some("{") => return FileScopeConstruct::IdentifierListDefinition,
            Some(_) => {}
            None => {
                return FileScopeConstruct::Unclassified(
                    EnumerationRefusal::UnclassifiedFileScopeConstruct,
                );
            }
        }
    }
}

/// Whether a parameter list is a K&R identifier list: one or more plain
/// identifiers separated by commas, none of them a declaration specifier.
///
/// `f(size_t)` is deliberately admitted here as an identifier list, because it
/// is genuinely ambiguous in isolation; the caller only reaches the K&R arm
/// when a declaration list *and* a body follow, which `f(size_t) { … }` — an
/// ordinary prototype definition — never has.
fn is_identifier_list(parameters: &[Token]) -> bool {
    if parameters.is_empty() {
        return false;
    }
    let mut expect_identifier = true;
    for token in parameters {
        let text = token.text.as_str();
        if expect_identifier {
            if !is_identifier(text) || DECLARATION_SPECIFIERS.contains(&text) {
                return false;
            }
            expect_identifier = false;
        } else {
            if text != "," {
                return false;
            }
            expect_identifier = true;
        }
    }
    !expect_identifier
}

/// Keywords that make a parameter list a prototype rather than a K&R
/// identifier list. This is the gateway's existing specifier table.
const DECLARATION_SPECIFIERS: &[&str] = super::DECLARATION_SPECIFIERS;

#[cfg(test)]
#[path = "../tests/unit/legacy_enumeration.rs"]
mod tests;
