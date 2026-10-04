//! Bounds contract v1: reviewed index and extent declarations (card
//! **CG16-INDEX-AND-OFFSET-EXTENTS**, contract specification C09.1, grammar
//! G05b).
//!
//! # Consumer selection
//!
//! `requires bounds_contract_v1;` is this consumer's selector, exactly as
//! `requires resource_contract_v1;` is the resource loader's (C02). A module
//! is loaded total-or-reject: every declaration must carry the selector once
//! and map completely into [`BoundsFunctionContract`], or the whole selection
//! is refused and no partial environment escapes. The two vocabularies are
//! disjoint, so the resource-v1 loader rejects a bounds module (no resource
//! selector, unknown predicates) and this loader rejects a resource module (no
//! bounds selector): neither consumer can silently drop the other's clauses
//! and publish a full-contract result.
//!
//! # Vocabulary
//!
//! ```text
//! requires bounds_contract_v1;
//! requires bounds_extent(P, COUNT, UNIT);      // P points to >= COUNT UNIT at entry
//! requires bounds_index(I, COUNT);             // 0 <= I < COUNT at entry
//! requires bounds_global_extent(G, N, UNIT);   // C object G has N UNIT (external storage)
//! ensures normal bounds_return_extent(COUNT, UNIT);  // the result points to COUNT UNIT
//! ensures normal bounds_preserves_extents();   // the call resizes, releases nothing
//! ensures normal bounds_frame_reads(P);        // CG24: reads only through P
//! ensures normal bounds_frame_writes(P);       // CG24: may write what P reaches
//! ensures normal bounds_frame_moves(P);        // CG24: may resize, move, release it
//! ensures normal bounds_return_derived(P, OFF, LEN, UNIT); // CG24: result is P + OFF, LEN UNIT
//! quant checks P, Idx #i: PREDICATE;         // QUANT-INDEX-CHECKS: every index through P
//! COUNT = a scalar parameter of the declaration | a canonical decimal literal
//! UNIT  = elements | bytes
//! ```
//!
//! # Quantified index checks (card QUANT-INDEX-CHECKS)
//!
//! `quant checks P, Idx #i: PREDICATE;` binds `#i` universally: every index
//! the function reads or writes through the pointer formal `P` satisfies
//! `PREDICATE` (comparisons `==`, `<`, `>`, `<=`, `>=` of `#i` against a bound
//! term, joined by `&&` and `||`). It is a check of the body and an obligation
//! of each caller (the actual's extent covers every admitted index), never a
//! postcondition, so it has its own introducer and no exit selector. The
//! reader is [`crate::bounds_quant`]; the decisions are
//! [`qcore_core::index_quant`]'s.
//!
//! # Frames and derived relations (card CG24-ALIAS-EXTENT-FRAME-VOCABULARY)
//!
//! A frame is **closed**: a declaration that states one states a mode for every
//! pointer parameter and every *slot* parameter (`ptr<mut, ptr<mut, u8>>`, a C
//! `T **`, admitted only in a frame), and the callee touches nothing else and
//! keeps nothing derived from a parameter beyond the call (its `Declared`
//! premise). Frames are not custody modes: `resource_borrowed` says nothing
//! about writes or reallocation, and these clauses say nothing about custody.
//! For a slot, `bounds_frame_moves` is the in-place reallocation of the object
//! the slot designates (the slot is rewritten; the base kept or moved).
//! `bounds_return_derived` states the result's relation to one base, or to
//! several (multi-base: the result is one of them); it excludes
//! `bounds_return_extent` and a base the call may move. The decisions are
//! [`qcore_core::index_extent_frames`].
//!
//! `elements` counts objects of the C pointee type of the parameter (or the
//! result, or the object's element type) as the audited C declaration states
//! it; `bytes` counts bytes. `Idx<N>` in C09.1 is conceptual notation for
//! `bounds_index`; it is not syntax, and this vocabulary does not touch the
//! closed index evaluator (`index.closed`) or `Idx + Ordered`.
//!
//! # Review
//!
//! A person states an external fact; until a person has reviewed it, it is
//! **held**. The marker reuses C06.1's semantics with this consumer's own
//! spelling, so the resource-v1 marker never reaches it implicitly:
//!
//! ```text
//! requires bounds_review_pending("REASON"[, "OWNER"][, rewrite_before_acceptance]);
//! requires bounds_review_pending(PRECONDITION, "REASON"[, "OWNER"][, …]);
//! ensures normal bounds_review_pending(POSTCONDITION, "REASON"[, "OWNER"][, …]);
//! ```
//!
//! Resolution is an explicit edit of the text after review: removing the
//! marker (or rewriting the clause when it carries
//! `rewrite_before_acceptance`). Nothing here, no loader, assistant or plugin,
//! can approve a declaration. A contract is applied as one conjunction, so one
//! held clause holds the whole declaration; the report names the clause. A
//! resolved declaration is a `Declared` assumption, never a proof of its C
//! body. Every declaration's normalised text is digested, so a changed clause
//! never reuses an earlier admission.

use std::collections::BTreeMap;

use qcore_core::index_extent_frames::FrameMode;
use qcore_frontend::frontend::{
    DeclarationKind, SourceContractClauseKind, SourceContractExitSelector, SourceDeclaration,
    SourceModuleMode, parse_source_program,
};
use qcore_frontend::lexer::{Symbol, TokenKind, lex};
use qcore_schemas::digest::sha256_hex;

/// The selector.
pub const BOUNDS_CONTRACT_MARKER: &str = "bounds_contract_v1";
/// The vocabulary version, part of every bounds row's evidence identity.
pub const BOUNDS_CONTRACT_VERSION: &str = "qcore.bounds.contract.v1";
/// The review marker of this consumer.
pub const BOUNDS_REVIEW_MARKER: &str = "bounds_review_pending";
/// The disposition word, as in C06.1.
pub const REWRITE_BEFORE_ACCEPTANCE: &str = "rewrite_before_acceptance";

/// Every identifier this vocabulary reserves (the source-spelling census
/// checks them against the contract roots).
pub const RESERVED_SPELLINGS: [&str; 12] = [
    BOUNDS_CONTRACT_MARKER,
    BOUNDS_REVIEW_MARKER,
    "bounds_extent",
    "bounds_index",
    "bounds_global_extent",
    "bounds_return_extent",
    "bounds_preserves_extents",
    REWRITE_BEFORE_ACCEPTANCE,
    "bounds_frame_reads",
    "bounds_frame_writes",
    "bounds_frame_moves",
    "bounds_return_derived",
];

const SCALAR_TYPES: [&str; 11] = [
    "bool", "i8", "i16", "i32", "i64", "isize", "u8", "u16", "u32", "u64", "usize",
];
const POINTER_TYPES: [&str; 2] = ["ptr<mut, u8>", "ptr<const, u8>"];
/// Card CG24: a pointer to a pointer (a C `T **`), admitted only in a frame.
const SLOT_TYPES: [&str; 1] = ["ptr<mut, ptr<mut, u8>>"];

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BoundsUnit {
    Elements,
    Bytes,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParameterKind {
    Scalar,
    Pointer,
    /// Card CG24: a pointer to a pointer, named only by a frame clause.
    Slot,
}

impl ParameterKind {
    /// A C pointer at the call (a pointer or a slot).
    pub fn is_pointer(self) -> bool {
        matches!(self, Self::Pointer | Self::Slot)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoundsParameter {
    pub name: String,
    pub kind: ParameterKind,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BoundsCount {
    Const(u128),
    /// A scalar parameter of the same declaration (its value at entry).
    Parameter(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BoundsClause {
    Extent {
        parameter: String,
        count: BoundsCount,
        unit: BoundsUnit,
    },
    Index {
        parameter: String,
        count: BoundsCount,
    },
    GlobalExtent {
        object: String,
        count: u128,
        unit: BoundsUnit,
    },
    ReturnExtent {
        count: BoundsCount,
        unit: BoundsUnit,
    },
    PreservesExtents,
    /// Card CG24: the callee's frame at one pointer or slot parameter.
    Frame {
        parameter: String,
        mode: FrameMode,
    },
    /// Card CG24: the result is `parameter + offset`, with `length` units.
    ReturnDerived {
        parameter: String,
        offset: BoundsCount,
        length: BoundsCount,
        unit: BoundsUnit,
    },
    /// Card QUANT-INDEX-CHECKS: every index through `parameter` satisfies
    /// `predicate` over the bound variable `#variable`.
    QuantCheck {
        parameter: String,
        variable: String,
        predicate: qcore_core::index_quant::QuantPredicate,
    },
}

impl BoundsClause {
    pub fn is_precondition(&self) -> bool {
        matches!(
            self,
            Self::Extent { .. }
                | Self::Index { .. }
                | Self::GlobalExtent { .. }
                | Self::QuantCheck { .. }
        )
    }
}

/// One review marker's note.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewNote {
    pub reason: String,
    pub owner: Option<String>,
    pub rewrite_before_acceptance: bool,
}

impl ReviewNote {
    /// C06.1's category, read from the structure, never from the prose.
    pub fn category(&self) -> &'static str {
        if self.rewrite_before_acceptance {
            "must-be-rewritten-before-acceptance"
        } else {
            "acceptable-as-written-after-review"
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoundsClauseEntry {
    pub clause: BoundsClause,
    /// The clause's own expression text, as written (without a marker).
    pub text: String,
    pub review: Option<ReviewNote>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoundsFunctionContract {
    pub name: String,
    pub module: String,
    pub parameters: Vec<BoundsParameter>,
    /// `None` for `unit`.
    pub result: Option<ParameterKind>,
    pub clauses: Vec<BoundsClauseEntry>,
    pub contract_review: Option<ReviewNote>,
    /// SHA-256 of the normalised declaration text (tokens joined by one space).
    pub text_sha256: String,
}

impl BoundsFunctionContract {
    /// Held: one marker anywhere holds the whole conjunction.
    pub fn held(&self) -> bool {
        self.contract_review.is_some() || self.clauses.iter().any(|c| c.review.is_some())
    }

    /// Every hold, named: `NAME: CLAUSE — REASON [owner OWNER] (CATEGORY)`.
    pub fn holds(&self) -> Vec<String> {
        let render = |scope: &str, note: &ReviewNote| {
            let owner = note
                .owner
                .as_ref()
                .map(|owner| format!(" owner {owner}"))
                .unwrap_or_default();
            format!(
                "{}: {scope} — {}{owner} ({}; sha256 {})",
                self.name,
                note.reason,
                note.category(),
                self.text_sha256
            )
        };
        let mut out = Vec::new();
        if let Some(note) = &self.contract_review {
            out.push(render("contract", note));
        }
        for clause in &self.clauses {
            if let Some(note) = &clause.review {
                out.push(render(&clause.text, note));
            }
        }
        out
    }

    pub fn parameter(&self, name: &str) -> Option<(usize, &BoundsParameter)> {
        self.parameters
            .iter()
            .enumerate()
            .find(|(_, parameter)| parameter.name == name)
    }

    /// Card CG24: the closed frame, one mode per parameter (`None` for a
    /// scalar), or `None` when the declaration states no frame.
    pub fn frame(&self) -> Option<Vec<Option<FrameMode>>> {
        let mut modes = vec![None; self.parameters.len()];
        let mut any = false;
        for entry in &self.clauses {
            if let BoundsClause::Frame { parameter, mode } = &entry.clause
                && let Some((index, _)) = self.parameter(parameter)
            {
                modes[index] = Some(*mode);
                any = true;
            }
        }
        any.then_some(modes)
    }

    /// Card CG24: the parameters a `bounds_return_derived` names, in order.
    pub fn derived_bases(&self) -> Vec<&str> {
        self.clauses
            .iter()
            .filter_map(|entry| match &entry.clause {
                BoundsClause::ReturnDerived { parameter, .. } => Some(parameter.as_str()),
                _ => None,
            })
            .collect()
    }
}

/// A loaded bounds environment.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BoundsContracts {
    pub functions: BTreeMap<String, BoundsFunctionContract>,
    /// SHA-256 of each selected module's text, by path.
    pub sources: BTreeMap<String, String>,
    /// Declarations an analysed body refutes: withdrawn from every dependent
    /// use ([`Self::contract`]); the defining body still reads its own.
    pub refuted: std::collections::BTreeSet<String>,
}

impl BoundsContracts {
    /// The environment's identity: the version and every module digest.
    pub fn digest(&self) -> String {
        let mut text = String::from(BOUNDS_CONTRACT_VERSION);
        for (path, sha) in &self.sources {
            text.push('\n');
            text.push_str(path);
            text.push(' ');
            text.push_str(sha);
        }
        for name in &self.refuted {
            text.push_str("\nrefuted ");
            text.push_str(name);
        }
        sha256_hex(text.as_bytes())
    }

    /// The declaration a caller may rely on: none for a refuted name.
    pub fn contract(&self, name: &str) -> Option<&BoundsFunctionContract> {
        if self.refuted.contains(name) {
            return None;
        }
        self.functions.get(name)
    }

    /// The declaration of a body the consumer analyses, refuted or not.
    pub fn own(&self, name: &str) -> Option<&BoundsFunctionContract> {
        self.functions.get(name)
    }
}

fn word(token: &TokenKind) -> Option<&str> {
    match token {
        TokenKind::Word(word) => Some(word),
        _ => None,
    }
}

fn tokens_of(source: &str) -> Result<Vec<TokenKind>, String> {
    Ok(lex(source)
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|token| token.kind)
        .filter(|kind| *kind != TokenKind::Eof)
        .collect())
}

fn type_kind(source: &str) -> Result<ParameterKind, String> {
    let tokens = tokens_of(source)?;
    let same = |spelling: &str| tokens_of(spelling).ok().as_ref() == Some(&tokens);
    if SCALAR_TYPES.iter().any(|ty| same(ty)) {
        Ok(ParameterKind::Scalar)
    } else if POINTER_TYPES.iter().any(|ty| same(ty)) {
        Ok(ParameterKind::Pointer)
    } else if SLOT_TYPES.iter().any(|ty| same(ty)) {
        Ok(ParameterKind::Slot)
    } else {
        Err(format!("unsupported bounds parameter type `{source}`"))
    }
}

/// Split `NAME ( ARGS )` into the name and its top-level comma-separated
/// argument token groups. Anything else is `None`.
fn call(tokens: &[TokenKind]) -> Option<(&str, Vec<&[TokenKind]>)> {
    let name = word(tokens.first()?)?;
    if tokens.get(1)? != &TokenKind::Symbol(Symbol::LeftParen)
        || tokens.last()? != &TokenKind::Symbol(Symbol::RightParen)
    {
        return None;
    }
    let inner = &tokens[2..tokens.len() - 1];
    let mut groups = Vec::new();
    let mut depth = 0usize;
    let mut start = 0;
    for (index, token) in inner.iter().enumerate() {
        match token {
            TokenKind::Symbol(Symbol::LeftParen) => depth += 1,
            TokenKind::Symbol(Symbol::RightParen) => depth = depth.checked_sub(1)?,
            TokenKind::Symbol(Symbol::Comma) if depth == 0 => {
                groups.push(&inner[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    if depth != 0 {
        return None;
    }
    if !inner.is_empty() {
        groups.push(&inner[start..]);
    }
    if groups.iter().any(|group| group.is_empty()) {
        return None;
    }
    Some((name, groups))
}

fn single_word(group: &[TokenKind]) -> Option<&str> {
    match group {
        [token] => word(token),
        _ => None,
    }
}

fn canonical_literal(group: &[TokenKind]) -> Option<u128> {
    match group {
        [TokenKind::Integer(digits)]
            if digits.bytes().all(|b| b.is_ascii_digit())
                && (digits == "0" || !digits.starts_with('0')) =>
        {
            digits.parse().ok()
        }
        _ => None,
    }
}

fn unit(group: &[TokenKind]) -> Result<BoundsUnit, String> {
    match single_word(group) {
        Some("elements") => Ok(BoundsUnit::Elements),
        Some("bytes") => Ok(BoundsUnit::Bytes),
        _ => Err("a bounds unit is `elements` or `bytes`".into()),
    }
}

fn plain_string(token: &TokenKind) -> Result<String, String> {
    match token {
        TokenKind::StringLiteral(text)
            if !text.trim().is_empty()
                && !text.contains('\\')
                && !text.chars().any(char::is_control) =>
        {
            Ok(text.clone())
        }
        _ => Err("a review reason or owner is one plain, non-blank string".into()),
    }
}

fn review_note(groups: &[&[TokenKind]]) -> Result<ReviewNote, String> {
    let mut strings = Vec::new();
    let mut rewrite = false;
    for (index, group) in groups.iter().enumerate() {
        match group {
            [token @ TokenKind::StringLiteral(_)] if !rewrite => strings.push(plain_string(token)?),
            [TokenKind::Word(word)]
                if word == REWRITE_BEFORE_ACCEPTANCE
                    && index + 1 == groups.len()
                    && !strings.is_empty() =>
            {
                rewrite = true;
            }
            _ => return Err("malformed bounds review note".into()),
        }
    }
    match strings.as_slice() {
        [reason] => Ok(ReviewNote {
            reason: reason.clone(),
            owner: None,
            rewrite_before_acceptance: rewrite,
        }),
        [reason, owner] => Ok(ReviewNote {
            reason: reason.clone(),
            owner: Some(owner.clone()),
            rewrite_before_acceptance: rewrite,
        }),
        [] => Err("a bounds review marker gives no reason; a reason is mandatory".into()),
        _ => Err("a bounds review marker takes a reason and an optional owner".into()),
    }
}

fn render(tokens: &[TokenKind]) -> String {
    let mut out = String::new();
    for token in tokens {
        let piece = match token {
            TokenKind::Word(w) | TokenKind::Integer(w) | TokenKind::Float(w) => w.clone(),
            TokenKind::StringLiteral(s) => format!("\"{s}\""),
            TokenKind::CharLiteral(s) => format!("'{s}'"),
            TokenKind::Symbol(Symbol::LeftParen) => "(".into(),
            TokenKind::Symbol(Symbol::RightParen) => ")".into(),
            TokenKind::Symbol(Symbol::Comma) => ",".into(),
            other => format!("{other:?}"),
        };
        if !out.is_empty() && !matches!(piece.as_str(), "(" | ")" | ",") && !out.ends_with('(') {
            out.push(' ');
        }
        out.push_str(&piece);
    }
    out
}

struct Reader<'a> {
    parameters: &'a [BoundsParameter],
    result: Option<ParameterKind>,
}

impl crate::bounds_quant::Formals for Reader<'_> {
    fn is_scalar(&self, name: &str) -> bool {
        self.parameters
            .iter()
            .any(|p| p.name == name && p.kind == ParameterKind::Scalar)
    }
    fn is_pointer(&self, name: &str) -> bool {
        self.parameters
            .iter()
            .any(|p| p.name == name && p.kind == ParameterKind::Pointer)
    }
}

impl Reader<'_> {
    fn count(&self, group: &[TokenKind]) -> Result<BoundsCount, String> {
        if let Some(value) = canonical_literal(group) {
            return Ok(BoundsCount::Const(value));
        }
        match single_word(group) {
            Some(name)
                if self
                    .parameters
                    .iter()
                    .any(|p| p.name == name && p.kind == ParameterKind::Scalar) =>
            {
                Ok(BoundsCount::Parameter(name.to_owned()))
            }
            _ => Err("a bounds count is a canonical decimal literal or a scalar parameter".into()),
        }
    }

    fn parameter(&self, group: &[TokenKind], kind: ParameterKind) -> Result<String, String> {
        match single_word(group) {
            Some(name)
                if self
                    .parameters
                    .iter()
                    .any(|p| p.name == name && p.kind == kind) =>
            {
                Ok(name.to_owned())
            }
            _ => Err(format!(
                "a bounds clause names a {} parameter of its declaration",
                match kind {
                    ParameterKind::Scalar => "scalar",
                    ParameterKind::Pointer => "pointer",
                    ParameterKind::Slot => "slot",
                }
            )),
        }
    }

    fn clause(&self, requires: bool, tokens: &[TokenKind]) -> Result<BoundsClause, String> {
        let (name, groups) = call(tokens).ok_or("malformed bounds clause")?;
        match (requires, name, groups.as_slice()) {
            (true, "bounds_extent", [p, n, u]) => Ok(BoundsClause::Extent {
                parameter: self.parameter(p, ParameterKind::Pointer)?,
                count: self.count(n)?,
                unit: unit(u)?,
            }),
            (true, "bounds_index", [i, n]) => Ok(BoundsClause::Index {
                parameter: self.parameter(i, ParameterKind::Scalar)?,
                count: self.count(n)?,
            }),
            (true, "bounds_global_extent", [g, n, u]) => Ok(BoundsClause::GlobalExtent {
                object: single_word(g)
                    .filter(|g| !self.parameters.iter().any(|p| p.name == *g))
                    .ok_or("a global extent names a C object, not a parameter")?
                    .to_owned(),
                count: canonical_literal(n).ok_or("a global extent's count is a literal")?,
                unit: unit(u)?,
            }),
            (false, "bounds_return_extent", [n, u]) => {
                if self.result != Some(ParameterKind::Pointer) {
                    return Err("a return extent needs a pointer result".into());
                }
                Ok(BoundsClause::ReturnExtent {
                    count: self.count(n)?,
                    unit: unit(u)?,
                })
            }
            (false, "bounds_preserves_extents", []) => Ok(BoundsClause::PreservesExtents),
            (false, "bounds_frame_reads" | "bounds_frame_writes" | "bounds_frame_moves", [p]) => {
                let parameter = single_word(p)
                    .filter(|name| {
                        self.parameters
                            .iter()
                            .any(|q| q.name == *name && q.kind.is_pointer())
                    })
                    .ok_or("a frame clause names a pointer or slot parameter")?;
                Ok(BoundsClause::Frame {
                    parameter: parameter.to_owned(),
                    mode: match name {
                        "bounds_frame_reads" => FrameMode::Reads,
                        "bounds_frame_writes" => FrameMode::Writes,
                        _ => FrameMode::Moves,
                    },
                })
            }
            (false, "bounds_return_derived", [p, off, len, u]) => {
                if self.result != Some(ParameterKind::Pointer) {
                    return Err("a derived result needs a pointer result".into());
                }
                Ok(BoundsClause::ReturnDerived {
                    parameter: self.parameter(p, ParameterKind::Pointer)?,
                    offset: self.count(off)?,
                    length: self.count(len)?,
                    unit: unit(u)?,
                })
            }
            _ => Err(format!(
                "unsupported bounds clause `{}` under `{}`",
                render(tokens),
                if requires {
                    "requires"
                } else {
                    "ensures normal"
                }
            )),
        }
    }
}

fn load_function(path: &str, decl: &SourceDeclaration) -> Result<BoundsFunctionContract, String> {
    if decl.kind != DeclarationKind::ExternFunction || decl.c_ffi.is_some() {
        return Err("bounds modules accept only extern functions".into());
    }
    let mut parameters = Vec::new();
    for parameter in &decl.parameters {
        if parameter.usage.is_some() {
            return Err("a bounds declaration takes no usage annotation".into());
        }
        if parameters
            .iter()
            .any(|p: &BoundsParameter| p.name == parameter.name.value)
        {
            return Err(format!("duplicate parameter `{}`", parameter.name.value));
        }
        parameters.push(BoundsParameter {
            name: parameter.name.value.clone(),
            kind: type_kind(&parameter.type_source)?,
        });
    }
    let result = match decl.result_type.as_ref().map(|r| r.source.trim()) {
        Some("unit") => None,
        Some(source) => Some(type_kind(source)?),
        None => return Err("a bounds declaration states its result type".into()),
    };
    let contract = decl
        .function_contract
        .as_ref()
        .ok_or("missing bounds contract")?;
    let reader = Reader {
        parameters: &parameters,
        result,
    };
    let mut selectors = 0;
    let mut contract_review = None;
    let mut clauses: Vec<BoundsClauseEntry> = Vec::new();
    let mut normalised = vec![format!(
        "extern fn {}({}) -> {}",
        decl.name,
        decl.parameters
            .iter()
            .map(|p| format!("{}: {}", p.name.value, p.type_source.trim()))
            .collect::<Vec<_>>()
            .join(", "),
        decl.result_type.as_ref().map_or("", |r| r.source.trim())
    )];
    for clause in &contract.clauses {
        let clause = &clause.value;
        // Card QUANT-INDEX-CHECKS: the collocated index check.
        if clause.kind == SourceContractClauseKind::QuantCheck {
            let tokens = tokens_of(&clause.expression_source)?;
            let text = crate::bounds_quant::clause_text(&tokens);
            normalised.push(text.clone());
            let read = crate::bounds_quant::read_quant_check(&tokens, &reader)?;
            clauses.push(BoundsClauseEntry {
                clause: BoundsClause::QuantCheck {
                    parameter: read.parameter,
                    variable: read.variable,
                    predicate: read.predicate,
                },
                text,
                review: None,
            });
            continue;
        }
        let requires = match &clause.kind {
            SourceContractClauseKind::Requires => true,
            SourceContractClauseKind::Ensures(SourceContractExitSelector::Normal) => false,
            _ => return Err("bounds contracts admit only `requires` and `ensures normal`".into()),
        };
        let tokens = tokens_of(&clause.expression_source)?;
        normalised.push(format!(
            "{} {}",
            if requires {
                "requires"
            } else {
                "ensures normal"
            },
            render(&tokens)
        ));
        if requires
            && matches!(tokens.as_slice(), [TokenKind::Word(w)] if w == BOUNDS_CONTRACT_MARKER)
        {
            selectors += 1;
            continue;
        }
        if let Some((BOUNDS_REVIEW_MARKER, groups)) = call(&tokens) {
            let wraps = groups
                .first()
                .is_some_and(|group| matches!(group.first(), Some(TokenKind::Word(_))));
            if !wraps {
                if !requires {
                    return Err("a contract-level bounds review marker is a `requires`".into());
                }
                if contract_review.is_some() || clauses.iter().any(|c| c.review.is_some()) {
                    return Err("duplicate bounds review marker".into());
                }
                contract_review = Some(review_note(&groups)?);
                continue;
            }
            if contract_review.is_some() {
                return Err("duplicate bounds review marker".into());
            }
            let inner = groups[0];
            if matches!(call(inner), Some((BOUNDS_REVIEW_MARKER, _))) {
                return Err("a bounds review marker cannot wrap another marker".into());
            }
            if matches!(inner, [TokenKind::Word(w)] if w == BOUNDS_CONTRACT_MARKER) {
                return Err("the bounds selector cannot be held on its own".into());
            }
            let note = review_note(&groups[1..])?;
            let parsed = reader.clause(requires, inner)?;
            clauses.push(BoundsClauseEntry {
                clause: parsed,
                text: render(inner),
                review: Some(note),
            });
            continue;
        }
        let parsed = reader.clause(requires, &tokens)?;
        clauses.push(BoundsClauseEntry {
            clause: parsed,
            text: render(&tokens),
            review: None,
        });
    }
    if selectors != 1 {
        return Err(format!(
            "a bounds declaration carries exactly one `requires {BOUNDS_CONTRACT_MARKER};`"
        ));
    }
    // Static conditions: at most one fact per position.
    let mut seen = std::collections::BTreeSet::new();
    for entry in &clauses {
        let key = match &entry.clause {
            BoundsClause::Extent { parameter, .. } => format!("extent {parameter}"),
            BoundsClause::Index { parameter, .. } => format!("index {parameter}"),
            BoundsClause::GlobalExtent { object, .. } => format!("global {object}"),
            BoundsClause::ReturnExtent { .. } => "return".into(),
            BoundsClause::PreservesExtents => "preserves".into(),
            BoundsClause::Frame { parameter, .. } => format!("frame {parameter}"),
            BoundsClause::ReturnDerived { parameter, .. } => format!("derived {parameter}"),
            BoundsClause::QuantCheck { parameter, .. } => format!("quant {parameter}"),
        };
        if !seen.insert(key.clone()) {
            return Err(format!("duplicate bounds clause for {key}"));
        }
    }
    // Card QUANT-INDEX-CHECKS: a pointer states its extent or its index
    // check, not both (the check's caller obligation is the extent it needs).
    for entry in &clauses {
        if let BoundsClause::QuantCheck { parameter, .. } = &entry.clause
            && seen.contains(&format!("extent {parameter}"))
        {
            return Err(format!(
                "quant-check-with-extent: `{parameter}` states both a bounds_extent and a quant check"
            ));
        }
    }
    frame_conditions(&parameters, &clauses)?;
    Ok(BoundsFunctionContract {
        name: decl.name.clone(),
        module: path.to_owned(),
        parameters,
        result,
        clauses,
        contract_review,
        text_sha256: sha256_hex(normalised.join(";\n").as_bytes()),
    })
}

/// Card CG24's static conditions: a closed frame names every pointer and slot
/// parameter; a slot appears only in a frame; preservation excludes a move;
/// a derived result excludes a return extent and a moved base.
fn frame_conditions(
    parameters: &[BoundsParameter],
    clauses: &[BoundsClauseEntry],
) -> Result<(), String> {
    let framed = clauses
        .iter()
        .filter_map(|entry| match &entry.clause {
            BoundsClause::Frame { parameter, mode } => Some((parameter.as_str(), *mode)),
            _ => None,
        })
        .collect::<BTreeMap<_, _>>();
    let has_slot = parameters.iter().any(|p| p.kind == ParameterKind::Slot);
    if !framed.is_empty() || has_slot {
        for parameter in parameters.iter().filter(|p| p.kind.is_pointer()) {
            if !framed.contains_key(parameter.name.as_str()) {
                return Err(format!(
                    "a frame is closed: pointer parameter `{}` states no frame",
                    parameter.name
                ));
            }
        }
    }
    let moves = framed.values().any(|mode| *mode == FrameMode::Moves);
    if moves
        && clauses
            .iter()
            .any(|entry| entry.clause == BoundsClause::PreservesExtents)
    {
        return Err("bounds_preserves_extents() contradicts a bounds_frame_moves".into());
    }
    let derived = clauses
        .iter()
        .filter_map(|entry| match &entry.clause {
            BoundsClause::ReturnDerived { parameter, .. } => Some(parameter.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>();
    if !derived.is_empty() {
        if clauses
            .iter()
            .any(|entry| matches!(entry.clause, BoundsClause::ReturnExtent { .. }))
        {
            return Err("a derived result excludes a bounds_return_extent".into());
        }
        if let Some(base) = derived
            .iter()
            .find(|base| framed.get(**base) == Some(&FrameMode::Moves))
        {
            return Err(format!(
                "a derived result cannot alias `{base}`, which the call may move"
            ));
        }
    }
    Ok(())
}

/// Load every selected module, total-or-reject.
pub fn load_bounds_contract_sources(
    sources: &BTreeMap<String, String>,
) -> Result<BoundsContracts, String> {
    if sources.is_empty() {
        return Err("no bounds contract inputs selected".into());
    }
    let mut contracts = BoundsContracts::default();
    for (path, source) in sources {
        let parsed = parse_source_program(source).map_err(|e| format!("{path}: {e}"))?;
        if parsed.mode != SourceModuleMode::Contract || parsed.declarations.is_empty() {
            return Err(format!("{path}: expected a nonempty module contract"));
        }
        for declaration in &parsed.declarations {
            let decl = &declaration.value;
            let context = format!("{path}: {} at byte {}", decl.name, declaration.span.start);
            if contracts.functions.contains_key(&decl.name) {
                return Err(format!("{context}: duplicate bounds contract"));
            }
            let loaded = load_function(path, decl).map_err(|e| format!("{context}: {e}"))?;
            contracts.functions.insert(decl.name.clone(), loaded);
        }
        contracts
            .sources
            .insert(path.clone(), sha256_hex(source.as_bytes()));
    }
    Ok(contracts)
}
