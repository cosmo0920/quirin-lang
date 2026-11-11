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

//! Outer contract v1: what code loaded at runtime, from outside the audited
//! program, may do (card **OUTER-CONTRACT**, contract specification C04.20,
//! grammar G05g, gateway rule 31).
//!
//! USER DECISIONS 2026-10-17 (recorded verbatim in the card's report): D1
//! "B: loader envelope", D2 "Yes, as proposed", D3 "Both from one run", D4
//! "Later".
//!
//! # Vocabulary
//!
//! An outer contract is hosted on the declaration of the **loader** that
//! hands objects to code the audit never reads (`flb_plugin_load`,
//! `flb_plugin_proxy_create`), so it is bound to that body and pinned to it:
//!
//! ```text
//! extern fn flb_plugin_load(path: ptr<mut, u8>, ctx: ptr<mut, u8>, config: ptr<mut, u8>) -> i32
//! contract {
//!     requires outer_contract_v1;
//!     quant #outer [[contract]] flb_external_plugin, "STATEMENT";   // defines NAME
//!     quant #outer where #config in #loaded [[runtime]] when #result == 0;
//!     quant #outer #loaded: flb_input_instance.p |-> #kept;         // envelope
//!     quant #outer #loaded: #premise NAME;                         // envelope
//! };
//! ```
//!
//! * `quant #outer [[contract]] NAME, "STATEMENT";` **defines** the contract
//!   (once in a selection); `quant #outer [[contract]] NAME;` on another
//!   loader **cites** it.
//! * `quant #outer where #P in #loaded [[runtime]] [when #result OP N];` is
//!   the runtime condition: after a call (meeting the guard), the object the
//!   pointer formal `P` names, and what is reachable from it, is held by
//!   loaded code. `#loaded` is a value keyword: that set of objects.
//! * An envelope line vouches for one closed-world fact (rule 31): `TAG.FIELD
//!   |-> #kept` (loaded code never stores into the member), `TAG.FIELD |->
//!   #registered` (it writes the member only through a declared registration,
//!   each callback meeting the member's candidate contract), `#premise NAME`
//!   (the named premise holds with loaded code present). Envelope lines stand
//!   only on the defining declaration.
//!
//! Loading is total-or-reject: any other clause, a malformed or repeated form,
//! a binder that is not a pointer formal, a guard on a result that is not an
//! integer, a citation of an undefined contract or a second definition
//! rejects the whole selection, with a named diagnostic. The resource and
//! bounds loaders reject every outer module (no selector of theirs, and the
//! `quant #outer` clause is theirs to refuse), so the three views are
//! disjoint.
//!
//! # Meaning
//!
//! The declarations decide no static verdict. The audit's runtime projection
//! (`qcore_audit::outer_selection`) reads them: a row resting on a
//! closed-world fact reachable from a `#loaded` binder is `Pending`
//! (`outer-contract: NAME@DIGEST`) where an envelope line covers the fact and
//! `Unknown` where none does. The Lean model is `QCore.OuterContract`.

use std::collections::{BTreeMap, BTreeSet};

use qcore_frontend::frontend::{
    DeclarationKind, SourceContractClauseKind, SourceDeclaration, SourceModuleMode,
    parse_source_program,
};
use qcore_frontend::lexer::{Symbol, TokenKind, lex};
use qcore_schemas::digest::sha256_hex;

/// The selector.
pub const OUTER_CONTRACT_MARKER: &str = "outer_contract_v1";
/// The vocabulary version, part of the projection's evidence identity.
pub const OUTER_CONTRACT_VERSION: &str = "qcore.outer.contract.v1";

/// Every identifier this vocabulary reserves.
pub const RESERVED_SPELLINGS: [&str; 1] = [OUTER_CONTRACT_MARKER];

/// What an envelope line says loaded code does to a member.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MemberEnvelope {
    /// `|-> #kept`: loaded code never stores into the member.
    Kept,
    /// `|-> #registered`: loaded code writes the member only through a
    /// declared registration whose callback meets the member's candidate
    /// contract.
    Registered,
}

impl MemberEnvelope {
    /// The value keyword, without `#`.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Kept => "kept",
            Self::Registered => "registered",
        }
    }
}

/// One envelope line.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EnvelopeLine {
    /// `TAG.PATH |-> #kept | #registered`.
    Member {
        tag: String,
        path: String,
        envelope: MemberEnvelope,
    },
    /// `#premise NAME`.
    Premise { name: String },
}

impl EnvelopeLine {
    /// The fact key the line covers: `TAG.PATH` or `premise:NAME`.
    #[must_use]
    pub fn key(&self) -> String {
        match self {
            Self::Member { tag, path, .. } => format!("{tag}.{path}"),
            Self::Premise { name } => format!("premise:{name}"),
        }
    }
}

/// A comparison in a runtime guard.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuardOp {
    Equal,
    NotEqual,
}

/// `when #result OP N`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeGuard {
    pub op: GuardOp,
    pub value: i64,
}

/// `quant #outer where #P in #loaded [[runtime]] [when …];`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeCondition {
    /// The pointer formal whose object loaded code holds.
    pub binder: String,
    /// Its position among the declaration's parameters.
    pub index: usize,
    pub guard: Option<RuntimeGuard>,
}

/// Why a loader declaration is not in force (C06.2's pin standing).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PinHold {
    pub reason: String,
    pub detail: String,
}

/// One loader declaration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OuterLoader {
    /// The loader function.
    pub name: String,
    /// The `crate:module` key it was loaded from.
    pub module: String,
    /// The contract it hosts.
    pub contract: String,
    /// The statement, on the defining declaration only.
    pub statement: Option<String>,
    pub runtime: Vec<RuntimeCondition>,
    /// Envelope lines, on the defining declaration only, in written order.
    pub envelope: Vec<EnvelopeLine>,
    /// The normalised text's digest (every clause), the pin's contract digest.
    pub text_sha256: String,
    /// Set when its pin is not in force over the audited source.
    pub pin_hold: Option<PinHold>,
}

/// One outer contract: its definition and every loader hosting it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OuterContract {
    pub name: String,
    pub statement: String,
    /// The loader whose declaration defines it.
    pub defined_by: String,
    /// Every loader hosting it (the definition and each citation).
    pub loaders: BTreeSet<String>,
    pub envelope: Vec<EnvelopeLine>,
}

/// The loaded environment.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OuterContracts {
    /// Per loader function, its declaration.
    pub loaders: BTreeMap<String, OuterLoader>,
    /// Per contract name, the contract.
    pub contracts: BTreeMap<String, OuterContract>,
    /// Per `crate:module` key, the SHA-256 of its text.
    pub sources: BTreeMap<String, String>,
}

impl OuterContracts {
    /// The contract's digest: SHA-256 over every hosting loader's name and
    /// text digest, in name order. Any change to a definition, a citation, a
    /// runtime condition or an envelope line changes it.
    #[must_use]
    pub fn digest(&self, contract: &str) -> Option<String> {
        let contract = self.contracts.get(contract)?;
        let text = contract
            .loaders
            .iter()
            .filter_map(|loader| self.loaders.get(loader))
            .map(|loader| format!("{}={}", loader.name, loader.text_sha256))
            .collect::<Vec<_>>()
            .join("\n");
        Some(sha256_hex(text.as_bytes()))
    }

    /// `NAME@DIGEST`, the spelling a projected `Pending` names.
    #[must_use]
    pub fn identity(&self, contract: &str) -> Option<String> {
        self.digest(contract)
            .map(|digest| format!("{contract}@{digest}"))
    }

    /// `true` when every loader hosting the contract is in force (its pin).
    #[must_use]
    pub fn in_force(&self, contract: &str) -> bool {
        self.contracts.get(contract).is_some_and(|c| {
            c.loaders.iter().all(|loader| {
                self.loaders
                    .get(loader)
                    .is_some_and(|l| l.pin_hold.is_none())
            })
        })
    }

    /// Hold a loader whose pin is not in force (C06.2): the contract it hosts
    /// then covers nothing (`OuterContracts::in_force`).
    ///
    /// # Errors
    ///
    /// An unknown loader.
    pub fn hold_for_pin_mismatch(
        &mut self,
        loader: &str,
        reason: &str,
        detail: &str,
    ) -> Result<(), String> {
        let entry = self
            .loaders
            .get_mut(loader)
            .ok_or_else(|| format!("no outer contract loader `{loader}` to hold"))?;
        entry.pin_hold = Some(PinHold {
            reason: reason.to_owned(),
            detail: detail.to_owned(),
        });
        Ok(())
    }

    /// SHA-256 of the whole environment (every loader's text digest and
    /// hold), for the projection's evidence identity.
    #[must_use]
    pub fn environment_digest(&self) -> String {
        let text = self
            .loaders
            .values()
            .map(|l| {
                format!(
                    "{}:{}:{}",
                    l.name,
                    l.text_sha256,
                    l.pin_hold.as_ref().map_or("", |h| h.reason.as_str())
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        sha256_hex(text.as_bytes())
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

fn render(tokens: &[TokenKind]) -> String {
    let mut out = String::new();
    for token in tokens {
        let piece = match token {
            TokenKind::Word(w) | TokenKind::Integer(w) | TokenKind::Float(w) => w.clone(),
            TokenKind::StringLiteral(s) => format!("\"{s}\""),
            TokenKind::CharLiteral(s) => format!("'{s}'"),
            TokenKind::Symbol(Symbol::Hash) => "#".into(),
            TokenKind::Symbol(Symbol::LeftBracket) => "[".into(),
            TokenKind::Symbol(Symbol::RightBracket) => "]".into(),
            TokenKind::Symbol(Symbol::Comma) => ",".into(),
            TokenKind::Symbol(Symbol::Colon) => ":".into(),
            TokenKind::Symbol(Symbol::Dot) => ".".into(),
            TokenKind::Symbol(Symbol::MapsTo) => "|->".into(),
            TokenKind::Symbol(Symbol::Minus) => "-".into(),
            TokenKind::Symbol(Symbol::EqualEqual) => "==".into(),
            TokenKind::Symbol(Symbol::NotEqual) => "!=".into(),
            other => format!("{other:?}"),
        };
        let glue = matches!(piece.as_str(), "," | ":" | "." | "]")
            || out.ends_with('#')
            || out.ends_with('[')
            || out.ends_with('.')
            || out.ends_with('-');
        if !out.is_empty() && !glue {
            out.push(' ');
        }
        out.push_str(&piece);
    }
    out
}

fn lowercase_identifier(name: &str) -> bool {
    name.chars()
        .next()
        .is_some_and(|first| first.is_ascii_lowercase())
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

fn c_identifier(name: &str) -> bool {
    name.chars()
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn plain_statement(statement: &str) -> bool {
    !statement.trim().is_empty()
        && !statement.contains('\\')
        && !statement.chars().any(char::is_control)
}

const INTEGER_TYPES: [&str; 10] = [
    "i8", "i16", "i32", "i64", "isize", "u8", "u16", "u32", "u64", "usize",
];

fn is_pointer_type(source: &str) -> bool {
    source.trim().starts_with("ptr<")
}

fn is_integer_type(source: &str) -> bool {
    INTEGER_TYPES.contains(&source.trim())
}

fn hash_word(tokens: &[TokenKind], at: usize, word: &str) -> bool {
    matches!(tokens.get(at), Some(TokenKind::Symbol(Symbol::Hash)))
        && matches!(tokens.get(at + 1), Some(TokenKind::Word(w)) if w == word)
}

fn qualifier(tokens: &[TokenKind], at: usize, word: &str) -> bool {
    matches!(
        tokens.get(at..at + 5),
        Some([
            TokenKind::Symbol(Symbol::LeftBracket),
            TokenKind::Symbol(Symbol::LeftBracket),
            TokenKind::Word(w),
            TokenKind::Symbol(Symbol::RightBracket),
            TokenKind::Symbol(Symbol::RightBracket),
        ]) if w == word
    )
}

/// One clause of a loader declaration.
enum Clause {
    Header {
        name: String,
        statement: Option<String>,
    },
    Runtime(RuntimeCondition),
    Envelope(EnvelopeLine),
}

struct Signature<'a> {
    parameters: &'a [(String, String)],
    result: Option<&'a str>,
}

fn read_clause(tokens: &[TokenKind], signature: &Signature<'_>) -> Result<Clause, String> {
    if !hash_word(tokens, 0, "outer") {
        return Err("outer-binder: an outer clause begins `quant #outer`".into());
    }
    let rest = &tokens[2..];
    // `[[contract]] NAME[, "STATEMENT"]`
    if qualifier(rest, 0, "contract") {
        return match &rest[5..] {
            [TokenKind::Word(name)] if lowercase_identifier(name) => Ok(Clause::Header {
                name: name.clone(),
                statement: None,
            }),
            [
                TokenKind::Word(name),
                TokenKind::Symbol(Symbol::Comma),
                TokenKind::StringLiteral(statement),
            ] if lowercase_identifier(name) => {
                if !plain_statement(statement) {
                    return Err(
                        "outer-statement: the statement is one plain, non-blank string with no escape and no control character"
                            .into(),
                    );
                }
                Ok(Clause::Header {
                    name: name.clone(),
                    statement: Some(statement.clone()),
                })
            }
            _ => Err(
                "outer-contract-shape: write `quant #outer [[contract]] NAME, \"STATEMENT\";` (a definition) or `quant #outer [[contract]] NAME;` (a citation), NAME a lowercase identifier"
                    .into(),
            ),
        };
    }
    // `where #P in #loaded [[runtime]] [when #result OP N]`
    if matches!(rest.first(), Some(TokenKind::Word(w)) if w == "where") {
        let shape = || {
            "outer-runtime-shape: write `quant #outer where #P in #loaded [[runtime]];` (optionally `when #result == N` or `!= N`), P a pointer formal".to_owned()
        };
        let (Some(TokenKind::Symbol(Symbol::Hash)), Some(TokenKind::Word(binder))) =
            (rest.get(1), rest.get(2))
        else {
            return Err(shape());
        };
        if !matches!(rest.get(3), Some(TokenKind::Word(w)) if w == "in") {
            return Err(shape());
        }
        if !hash_word(rest, 4, "loaded") {
            if matches!(rest.get(4), Some(TokenKind::Word(w)) if w == "loaded") {
                return Err("outer-loaded-keyword: `loaded` is the value keyword `#loaded`".into());
            }
            return Err(shape());
        }
        if !qualifier(rest, 6, "runtime") {
            return Err(shape());
        }
        let index = signature
            .parameters
            .iter()
            .position(|(name, _)| name == binder)
            .ok_or_else(|| {
                format!("outer-binder-parameter: `#{binder}` names no parameter of the loader")
            })?;
        if !is_pointer_type(&signature.parameters[index].1) {
            return Err(format!(
                "outer-binder-pointer: `#{binder}` is not a pointer formal"
            ));
        }
        let guard = match &rest[11..] {
            [] => None,
            [
                TokenKind::Word(when),
                TokenKind::Symbol(Symbol::Hash),
                TokenKind::Word(result),
                op @ ..,
            ] if when == "when" && result == "result" => {
                let (op, value) = match op {
                    [TokenKind::Symbol(Symbol::EqualEqual), value @ ..] => (GuardOp::Equal, value),
                    [TokenKind::Symbol(Symbol::NotEqual), value @ ..] => (GuardOp::NotEqual, value),
                    _ => return Err(shape()),
                };
                let value = match value {
                    [TokenKind::Integer(n)] => n.parse::<i64>().map_err(|_| shape())?,
                    [TokenKind::Symbol(Symbol::Minus), TokenKind::Integer(n)] => {
                        -n.parse::<i64>().map_err(|_| shape())?
                    }
                    _ => return Err(shape()),
                };
                if !signature.result.is_some_and(is_integer_type) {
                    return Err(
                        "outer-guard-result: a `when #result` guard needs an integer result".into(),
                    );
                }
                Some(RuntimeGuard { op, value })
            }
            _ => return Err(shape()),
        };
        return Ok(Clause::Runtime(RuntimeCondition {
            binder: binder.clone(),
            index,
            guard,
        }));
    }
    // `#loaded: TAG.PATH |-> #kept | #registered` or `#loaded: #premise NAME`
    if hash_word(rest, 0, "loaded") && matches!(rest.get(2), Some(TokenKind::Symbol(Symbol::Colon)))
    {
        let line = &rest[3..];
        if hash_word(line, 0, "premise") {
            return match &line[2..] {
                [TokenKind::Word(name)] if !name.is_empty() => {
                    Ok(Clause::Envelope(EnvelopeLine::Premise {
                        name: name.clone(),
                    }))
                }
                _ => Err(
                    "outer-envelope-premise: write `quant #outer #loaded: #premise NAME;`".into(),
                ),
            };
        }
        let shape = || {
            "outer-envelope-shape: write `quant #outer #loaded: TAG.FIELD |-> #kept;` or `|-> #registered;`, or `quant #outer #loaded: #premise NAME;`".to_owned()
        };
        let Some(arrow) = line
            .iter()
            .position(|token| *token == TokenKind::Symbol(Symbol::MapsTo))
        else {
            return Err(shape());
        };
        let (path, value) = (&line[..arrow], &line[arrow + 1..]);
        let mut segments = Vec::new();
        for (at, token) in path.iter().enumerate() {
            match (at % 2, token) {
                (0, TokenKind::Word(word)) if c_identifier(word) => segments.push(word.clone()),
                (1, TokenKind::Symbol(Symbol::Dot)) => {}
                _ => return Err(shape()),
            }
        }
        if segments.len() < 2 || path.len() % 2 == 0 {
            return Err(
                "outer-envelope-member: the left of `|->` is a dotted `TAG.FIELD` path".into(),
            );
        }
        let envelope = match value {
            [TokenKind::Symbol(Symbol::Hash), TokenKind::Word(w)] if w == "kept" => {
                MemberEnvelope::Kept
            }
            [TokenKind::Symbol(Symbol::Hash), TokenKind::Word(w)] if w == "registered" => {
                MemberEnvelope::Registered
            }
            [TokenKind::Word(w)] if w == "kept" || w == "registered" => {
                return Err(format!(
                    "outer-envelope-keyword: `{w}` is the value keyword `#{w}`"
                ));
            }
            _ => {
                return Err(
                    "outer-envelope-value: the right of `|->` is `#kept` or `#registered`".into(),
                );
            }
        };
        return Ok(Clause::Envelope(EnvelopeLine::Member {
            tag: segments[0].clone(),
            path: segments[1..].join("."),
            envelope,
        }));
    }
    if matches!(rest.first(), Some(TokenKind::Word(w)) if w == "loaded") {
        return Err("outer-loaded-keyword: `loaded` is the value keyword `#loaded`".into());
    }
    Err(
        "outer-clause-unknown: an outer clause is `[[contract]] NAME…`, `where #P in #loaded [[runtime]]…` or `#loaded: …`"
            .into(),
    )
}

fn load_loader(path: &str, decl: &SourceDeclaration) -> Result<OuterLoader, String> {
    if decl.kind != DeclarationKind::ExternFunction || decl.c_ffi.is_some() {
        return Err("outer contract modules accept only extern functions".into());
    }
    let mut parameters = Vec::new();
    for parameter in &decl.parameters {
        if parameter.usage.is_some() {
            return Err("an outer contract declaration takes no usage annotation".into());
        }
        if parameters
            .iter()
            .any(|(name, _): &(String, String)| *name == parameter.name.value)
        {
            return Err(format!("duplicate parameter `{}`", parameter.name.value));
        }
        parameters.push((
            parameter.name.value.clone(),
            parameter.type_source.trim().to_owned(),
        ));
    }
    let result = match decl.result_type.as_ref().map(|r| r.source.trim()) {
        Some("unit") => None,
        Some(source) => Some(source),
        None => return Err("an outer contract declaration states its result type".into()),
    };
    let signature = Signature {
        parameters: &parameters,
        result,
    };
    let contract = decl
        .function_contract
        .as_ref()
        .ok_or("missing outer contract")?;
    let mut normalised = vec![format!(
        "extern fn {}({}) -> {}",
        decl.name,
        parameters
            .iter()
            .map(|(name, ty)| format!("{name}: {ty}"))
            .collect::<Vec<_>>()
            .join(", "),
        decl.result_type.as_ref().map_or("", |r| r.source.trim())
    )];
    let mut selectors = 0;
    let mut header: Option<(String, Option<String>)> = None;
    let mut runtime: Vec<RuntimeCondition> = Vec::new();
    let mut envelope: Vec<EnvelopeLine> = Vec::new();
    for clause in &contract.clauses {
        let clause = &clause.value;
        let tokens = tokens_of(&clause.expression_source)?;
        match &clause.kind {
            SourceContractClauseKind::Requires => {
                normalised.push(format!("requires {}", render(&tokens)));
                if matches!(tokens.as_slice(), [TokenKind::Word(w)] if w == OUTER_CONTRACT_MARKER) {
                    selectors += 1;
                    continue;
                }
                return Err(format!(
                    "outer-clause-foreign: an outer contract declaration admits only `requires {OUTER_CONTRACT_MARKER};` and `quant #outer` clauses"
                ));
            }
            SourceContractClauseKind::QuantOuter => {
                normalised.push(format!("quant {}", render(&tokens)));
                match read_clause(&tokens, &signature)? {
                    Clause::Header { name, statement } => {
                        if header.is_some() {
                            return Err(
                                "outer-contract-repeated: a loader hosts exactly one `quant #outer [[contract]]`"
                                    .into(),
                            );
                        }
                        header = Some((name, statement));
                    }
                    Clause::Runtime(condition) => {
                        if runtime.iter().any(|c| c.binder == condition.binder) {
                            return Err(format!(
                                "outer-runtime-repeated: `#{}` is bound twice",
                                condition.binder
                            ));
                        }
                        runtime.push(condition);
                    }
                    Clause::Envelope(line) => {
                        if envelope.iter().any(|seen| seen.key() == line.key()) {
                            return Err(format!(
                                "outer-envelope-repeated: `{}` is stated twice",
                                line.key()
                            ));
                        }
                        envelope.push(line);
                    }
                }
            }
            _ => {
                return Err(format!(
                    "outer-clause-foreign: an outer contract declaration admits only `requires {OUTER_CONTRACT_MARKER};` and `quant #outer` clauses"
                ));
            }
        }
    }
    if selectors != 1 {
        return Err(format!(
            "an outer contract declaration carries exactly one `requires {OUTER_CONTRACT_MARKER};`"
        ));
    }
    let (name, statement) = header
        .ok_or("outer-contract-missing: a loader hosts one `quant #outer [[contract]] NAME…;`")?;
    if runtime.is_empty() {
        return Err(
            "outer-runtime-missing: a loader states at least one `quant #outer where #P in #loaded [[runtime]];`"
                .into(),
        );
    }
    if statement.is_none() && !envelope.is_empty() {
        return Err(
            "outer-envelope-citation: envelope lines stand only on the declaration that defines the contract"
                .into(),
        );
    }
    Ok(OuterLoader {
        name: decl.name.clone(),
        module: path.to_owned(),
        contract: name,
        statement,
        runtime,
        envelope,
        text_sha256: sha256_hex(normalised.join(";\n").as_bytes()),
        pin_hold: None,
    })
}

/// Load every selected module, total-or-reject.
///
/// # Errors
///
/// Any malformed module, declaration or clause, a duplicate loader, a
/// contract defined twice or cited without a definition.
pub fn load_outer_contract_sources(
    sources: &BTreeMap<String, String>,
) -> Result<OuterContracts, String> {
    if sources.is_empty() {
        return Err("no outer contract inputs selected".into());
    }
    let mut contracts = OuterContracts::default();
    for (path, source) in sources {
        let parsed = parse_source_program(source).map_err(|e| format!("{path}: {e}"))?;
        if parsed.mode != SourceModuleMode::Contract || parsed.declarations.is_empty() {
            return Err(format!("{path}: expected a nonempty module contract"));
        }
        for declaration in &parsed.declarations {
            let decl = &declaration.value;
            let context = format!("{path}: {} at byte {}", decl.name, declaration.span.start);
            if contracts.loaders.contains_key(&decl.name) {
                return Err(format!("{context}: duplicate outer contract loader"));
            }
            let loaded = load_loader(path, decl).map_err(|e| format!("{context}: {e}"))?;
            contracts.loaders.insert(decl.name.clone(), loaded);
        }
        contracts
            .sources
            .insert(path.clone(), sha256_hex(source.as_bytes()));
    }
    for loader in contracts.loaders.values() {
        if let Some(statement) = &loader.statement {
            if let Some(first) = contracts.contracts.get(&loader.contract) {
                return Err(format!(
                    "outer-contract-defined-twice: `{}` is defined by `{}` and `{}`",
                    loader.contract, first.defined_by, loader.name
                ));
            }
            contracts.contracts.insert(
                loader.contract.clone(),
                OuterContract {
                    name: loader.contract.clone(),
                    statement: statement.clone(),
                    defined_by: loader.name.clone(),
                    loaders: BTreeSet::new(),
                    envelope: loader.envelope.clone(),
                },
            );
        }
    }
    for loader in contracts.loaders.values() {
        let contract = contracts
            .contracts
            .get_mut(&loader.contract)
            .ok_or_else(|| {
                format!(
                    "outer-contract-undefined: `{}` cites `{}`, which no loader defines",
                    loader.name, loader.contract
                )
            })?;
        contract.loaders.insert(loader.name.clone());
    }
    Ok(contracts)
}
