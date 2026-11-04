//! Checked relations between declarations of one function in several contract
//! modules (card CONTRACT-VIEW-PROJECTION-CHECK).
//!
//! The canonical corpus is read by several consumers with disjoint
//! vocabularies (`contract/README.md`, "Which interpreter owns which clause
//! family"). A function such as `flb_malloc` may therefore be declared once per
//! consuming checker: a detailed allocator-observation contract evaluated on
//! native executions, and a resource-v1 view the resource loader reads. Those
//! views are hand-written, so nothing but review kept them in agreement.
//!
//! This module makes every multi-module declaration a **checked relation**:
//!
//! * [`census`] measures every function declared in more than one module and
//!   classifies each declaration's vocabulary from its own text;
//! * [`REGISTERED_RELATIONS`] names, for every such pair, the relation it
//!   stands in, and [`check_registry`] refuses an unregistered pair, a stale
//!   registration, and any relation whose check fails;
//! * a **projection** is checked by deciding that the detailed contract, under
//!   a declared bridge from its vocabulary to the resource-v1 predicates,
//!   entails each clause of the view ([`entails`]). The premises the view does
//!   not itself discharge are computed, not declared, and must equal the pinned
//!   list, so a view that silently outgrows its detailed contract fails;
//! * a **renamed** pair must be token-equal under a declared renaming and give
//!   identical results under both real frame evaluators;
//! * declarations of **different kinds** (source-shape metadata beside a typed
//!   behavioural declaration, or two lifecycle profiles) must agree on
//!   everything both state: arity, ABI category of every position, scalar
//!   types, the result, and every recorded declaration fact.
//!
//! Nothing here loads a contract for analysis or changes what any consumer
//! reads. Every refusal is fail-closed: an unrecognised clause, a predicate
//! outside the decidable fragment, or a domain too large to enumerate is an
//! error, never an acceptance.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use qcore_frontend::frontend::{
    DeclarationKind, SourceContractClauseKind, SourceContractExitSelector, SourceDeclaration,
    parse_source_program,
};
use qcore_frontend::lexer::{Symbol, TokenKind, lex};
use qcore_frontend::source_extern::normalize_type;
use serde::Serialize;

use crate::chunk_contract::{evaluate_boolean_frame_contract, evaluate_chunk_frame_contract};
use crate::contract_observation::{ContractFact, check_typed_contract_facts};
use qcore_schemas::chunk_summary::ChunkFrameSummaryRecord;

/// Upper bound on the assignments one entailment check may enumerate.
pub const MAX_ENUMERATED_ASSIGNMENTS: u64 = 1 << 24;

// ---------------------------------------------------------------------------
// Declarations and the census
// ---------------------------------------------------------------------------

/// One contract-bearing `extern fn` declaration, as its module states it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ViewDeclaration {
    pub module: String,
    pub name: String,
    pub parameters: Vec<ViewParameter>,
    pub result: String,
    pub clauses: Vec<ViewClause>,
    pub has_body: bool,
    pub has_effects: bool,
    pub is_unsafe: bool,
    #[serde(skip)]
    tokens: Vec<TokenKind>,
    /// The declaration's own source text.
    #[serde(skip)]
    text: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ViewParameter {
    pub name: String,
    /// Whitespace-free type text (`qcore_frontend::source_extern::normalize_type`).
    pub type_source: String,
    /// `None`, `one`, or `other`.
    pub usage: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ViewClause {
    /// `requires`, `normal`, `error`, `any`, `quant normal`, `quant error`,
    /// `quant any`.
    pub kind: String,
    pub expression: String,
}

impl ViewDeclaration {
    fn from_source(module: &str, declaration: &SourceDeclaration, text: &str) -> Option<Self> {
        if declaration.kind != DeclarationKind::ExternFunction {
            return None;
        }
        let contract = declaration.function_contract.as_ref()?;
        let clauses = contract
            .clauses
            .iter()
            .map(|clause| ViewClause {
                kind: match &clause.value.kind {
                    SourceContractClauseKind::Requires => "requires".into(),
                    SourceContractClauseKind::Ensures(selector) => selector_name(selector).into(),
                    SourceContractClauseKind::Quant(selector) => {
                        format!("quant {}", selector_name(selector))
                    }
                    SourceContractClauseKind::QuantEffect(selector) => {
                        format!("quant effect {}", selector_name(selector))
                    }
                    SourceContractClauseKind::QuantProposition => "quant proposition".into(),
                    SourceContractClauseKind::QuantCondition(selector) => {
                        format!("quant condition {selector}")
                    }
                    SourceContractClauseKind::QuantTheorem => "quant theorem".into(),
                    SourceContractClauseKind::QuantCheck => "quant checks".into(),
                    SourceContractClauseKind::QuantNullDeref => "quant null-deref".into(),
                    SourceContractClauseKind::QuantDeref => "quant deref".into(),
                    SourceContractClauseKind::QuantLive => "quant live".into(),
                    SourceContractClauseKind::QuantOuter => "quant".into(),
                    SourceContractClauseKind::QuantConditionBinding => "quant".into(),
                    SourceContractClauseKind::QuantConditionBlock { negated, .. } => {
                        if *negated {
                            "quant not".into()
                        } else {
                            "quant".into()
                        }
                    }
                    SourceContractClauseKind::QuantRegistered => "quant registered".into(),
                    SourceContractClauseKind::QuantCell => "quant cell".into(),
                },
                expression: clause.value.expression_source.trim().to_owned(),
            })
            .collect();
        let word = |wanted: &str| {
            declaration
                .tokens
                .iter()
                .any(|token| matches!(token, TokenKind::Word(word) if word == wanted))
        };
        Some(Self {
            module: module.to_owned(),
            name: declaration.name.clone(),
            parameters: declaration
                .parameters
                .iter()
                .map(|parameter| ViewParameter {
                    name: parameter.name.value.clone(),
                    type_source: normalize_type(&parameter.type_source),
                    usage: match parameter.usage.as_ref().map(|usage| usage.value) {
                        None => "none",
                        Some(qcore_core::usage::Usage::ONE) => "one",
                        Some(_) => "other",
                    },
                })
                .collect(),
            result: declaration
                .result_type
                .as_ref()
                .map(|result| normalize_type(&result.source))
                .unwrap_or_default(),
            clauses,
            has_body: declaration.body.is_some(),
            has_effects: word("effects"),
            is_unsafe: word("unsafe"),
            tokens: declaration.tokens.clone(),
            text: text.to_owned(),
        })
    }

    fn requires(&self) -> impl Iterator<Item = &str> {
        self.clauses
            .iter()
            .filter(|clause| clause.kind == "requires")
            .map(|clause| clause.expression.as_str())
    }

    fn ensures_normal(&self) -> impl Iterator<Item = &str> {
        self.clauses
            .iter()
            .filter(|clause| clause.kind == "normal")
            .map(|clause| clause.expression.as_str())
    }
}

fn selector_name(selector: &SourceContractExitSelector) -> &'static str {
    match selector {
        SourceContractExitSelector::Normal => "normal",
        SourceContractExitSelector::Error(_) => "error",
        SourceContractExitSelector::Any => "any",
    }
}

/// The consumer-facing vocabulary a declaration is written in, read off its
/// own text (`contract/README.md`'s clause-family table).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Vocabulary {
    /// `requires resource_contract_v1`: the resource loader.
    ResourceV1,
    /// Integer/Boolean ghost comparisons: the observation interpreter.
    IntegerObservation,
    /// Bare named propositions (and `not A or B` over them): the
    /// mechanism/lifecycle interpreters.
    AtomicLifecycle,
    /// `own`/`borrow` parameters with `effects`, `quant`, `ensures any` or
    /// `ensures error`: the QCore language front end.
    Language,
    /// A parameterless `-> unit` typed policy that records a C `signature`:
    /// source-shape metadata.
    SourceShapeMetadata,
    /// Any other parameterless `-> unit` typed policy with text facts.
    TypedPolicy,
}

/// Classify one declaration's vocabulary from its clauses and signature.
pub fn vocabulary(declaration: &ViewDeclaration) -> Vocabulary {
    if declaration
        .clauses
        .iter()
        .any(|clause| clause.expression == "resource_contract_v1")
    {
        return Vocabulary::ResourceV1;
    }
    if declaration.parameters.is_empty() && declaration.result == "unit" {
        let mentions_signature = declaration.clauses.iter().any(|clause| {
            words(&clause.expression)
                .iter()
                .any(|word| word == "signature")
        });
        if mentions_signature {
            return Vocabulary::SourceShapeMetadata;
        }
        if declaration
            .clauses
            .iter()
            .any(|clause| clause.expression.contains('"'))
        {
            return Vocabulary::TypedPolicy;
        }
    }
    if declaration.has_effects
        || declaration
            .clauses
            .iter()
            .any(|clause| clause.kind.starts_with("quant") || clause.kind == "any")
    {
        return Vocabulary::Language;
    }
    let atomic = declaration.clauses.iter().all(|clause| {
        lex_tokens(&clause.expression).is_ok_and(|tokens| {
            tokens.iter().all(|token| {
                matches!(
                    token,
                    TokenKind::Word(_) | TokenKind::Symbol(Symbol::LeftParen | Symbol::RightParen)
                )
            })
        })
    });
    if atomic {
        Vocabulary::AtomicLifecycle
    } else {
        Vocabulary::IntegerObservation
    }
}

/// Every contract-bearing `extern fn` in `modules` (logical path → text),
/// keyed by function name, in logical-path order. A function declared twice
/// in one module is refused.
pub fn collect_declarations(
    modules: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, Vec<ViewDeclaration>>, String> {
    let mut declarations: BTreeMap<String, Vec<ViewDeclaration>> = BTreeMap::new();
    for (module, text) in modules {
        let parsed = parse_source_program(text).map_err(|error| format!("{module}: {error}"))?;
        for declaration in &parsed.declarations {
            let slice = text
                .get(declaration.span.start..declaration.span.end)
                .ok_or_else(|| format!("{module}: declaration span out of range"))?;
            let Some(view) = ViewDeclaration::from_source(module, &declaration.value, slice) else {
                continue;
            };
            let entry = declarations.entry(view.name.clone()).or_default();
            if entry.iter().any(|existing| existing.module == *module) {
                return Err(format!("{module}: `{}` is declared twice", view.name));
            }
            entry.push(view);
        }
    }
    Ok(declarations)
}

/// One function declared in more than one module.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CensusEntry {
    pub function: String,
    /// `(module, vocabulary)` in logical-path order.
    pub declarations: Vec<(String, Vocabulary)>,
    /// Every declaration has the same token sequence (comments aside): a
    /// genuine duplicate, to be removed rather than registered.
    pub token_identical: bool,
}

/// Every function declared in more than one module.
pub fn census(modules: &BTreeMap<String, String>) -> Result<Vec<CensusEntry>, String> {
    Ok(collect_declarations(modules)?
        .into_iter()
        .filter(|(_, declarations)| declarations.len() > 1)
        .map(|(function, declarations)| CensusEntry {
            function,
            declarations: declarations
                .iter()
                .map(|declaration| (declaration.module.clone(), vocabulary(declaration)))
                .collect(),
            token_identical: declarations
                .windows(2)
                .all(|pair| pair[0].tokens == pair[1].tokens),
        })
        .collect())
}

/// Every unordered pair of modules declaring the same function, as
/// `(function, first module, second module)` with the modules sorted.
pub fn census_pairs(entries: &[CensusEntry]) -> BTreeSet<(String, String, String)> {
    let mut pairs = BTreeSet::new();
    for entry in entries {
        for (index, (left, _)) in entry.declarations.iter().enumerate() {
            for (right, _) in &entry.declarations[index + 1..] {
                let (a, b) = if left <= right {
                    (left, right)
                } else {
                    (right, left)
                };
                pairs.insert((entry.function.clone(), a.clone(), b.clone()));
            }
        }
    }
    pairs
}

/// Read the corpus the index at `index` selects: every `.qcore` or `.qv` under its
/// roots, keyed by logical path, with a module that uses contract inclusion
/// read as its elaboration on the index's load path (the text every loader
/// parses). Mirrors the corpus test's own discovery.
pub fn read_corpus(index: &Path) -> Result<BTreeMap<String, String>, String> {
    let manifest: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(index).map_err(|error| format!("{}: {error}", index.display()))?,
    )
    .map_err(|error| format!("{}: {error}", index.display()))?;
    let base = index.parent().ok_or("corpus index has no directory")?;
    let load_path = match manifest.get("load_path").and_then(|value| value.as_str()) {
        Some(relative) => Some(
            crate::load_path_manifest::load_contract_load_path_manifest(&base.join(relative))
                .map_err(|error| error.to_string())?,
        ),
        None => None,
    };
    let mut modules: BTreeMap<String, String> = BTreeMap::new();
    for root in manifest["roots"]
        .as_array()
        .ok_or("corpus index has no roots")?
    {
        let root = base.join(root.as_str().ok_or("corpus root is not text")?);
        let mut pending = vec![root.clone()];
        while let Some(directory) = pending.pop() {
            for entry in std::fs::read_dir(&directory)
                .map_err(|error| format!("{}: {error}", directory.display()))?
            {
                let path = entry.map_err(|error| error.to_string())?.path();
                if path.is_symlink() {
                    return Err(format!("symbolic link in corpus: {}", path.display()));
                }
                if path.is_dir() {
                    pending.push(path);
                    continue;
                }
                if path
                    .extension()
                    .is_none_or(|extension| extension != "qcore" && extension != "qv")
                {
                    continue;
                }
                let logical = path
                    .strip_prefix(&root)
                    .map_err(|error| error.to_string())?
                    .to_string_lossy()
                    .replace('\\', "/");
                let written = std::fs::read_to_string(&path)
                    .map_err(|error| format!("{}: {error}", path.display()))?;
                let alias = qcore_frontend::source_extension::contract_module_stem(&logical);
                if let Some(stem) = alias
                    && let Some(other) = modules.keys().find(|key| {
                        qcore_frontend::source_extension::contract_module_stem(key) == Some(stem)
                    })
                {
                    return Err(format!(
                        "ambiguous contract module aliases `{other}` and `{logical}`"
                    ));
                }
                let text = if qcore_frontend::contract_inclusion::contains_contract_inclusion_syntax(
                    &written,
                ) {
                    let load_path = load_path
                        .as_ref()
                        .ok_or("a module uses inclusion but the corpus names no load path")?;
                    let krate = load_path
                        .crate_for_root(&root)
                        .ok_or_else(|| format!("{logical}: inclusion outside a load-path crate"))?;
                    crate::contract_inclusion::elaborate_contract_module(
                        &load_path.load_path,
                        krate,
                        &logical,
                    )
                    .map_err(|error| format!("{logical}: {error}"))?
                    .text
                } else {
                    written
                };
                crate::extension_compatibility::validate_source_name(&logical, &text)
                    .map_err(|error| format!("{logical}: {error}"))?;
                if modules.insert(logical.clone(), text).is_some() {
                    return Err(format!("duplicate logical contract: {logical}"));
                }
            }
        }
    }
    Ok(modules)
}

// ---------------------------------------------------------------------------
// The decidable predicate fragment and entailment
// ---------------------------------------------------------------------------

/// A predicate of the equality fragment the observation and lifecycle
/// interpreters share: integer literals, `true`/`false`, ghost names, `==`,
/// `!=`, `and`, `or`, `not`/`!` and parentheses. Semantics are exactly the
/// shared interpreter's (`contract_observation`): values are integers, a
/// comparison yields 0/1, a connective reads nonzero as true, and `not`
/// binds to the following primary only.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Term {
    Literal(i128),
    Name(String),
    Not(Box<Term>),
    Equal(Box<Term>, Box<Term>),
    NotEqual(Box<Term>, Box<Term>),
    And(Box<Term>, Box<Term>),
    Or(Box<Term>, Box<Term>),
}

fn lex_tokens(source: &str) -> Result<Vec<TokenKind>, String> {
    Ok(lex(source)
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|token| token.kind)
        .filter(|token| *token != TokenKind::Eof)
        .collect())
}

fn words(source: &str) -> Vec<String> {
    lex_tokens(source)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|token| match token {
            TokenKind::Word(word) => Some(word),
            _ => None,
        })
        .collect()
}

struct FragmentParser<'a> {
    tokens: &'a [TokenKind],
    cursor: usize,
}

impl FragmentParser<'_> {
    fn parse(source: &str) -> Result<Term, String> {
        let tokens = lex_tokens(source)?;
        let mut parser = FragmentParser {
            tokens: &tokens,
            cursor: 0,
        };
        let term = parser.expression(0, 0)?;
        if parser.cursor != tokens.len() {
            return Err(format!("outside the equality fragment: {source}"));
        }
        Ok(term)
    }

    fn expression(&mut self, minimum: u8, depth: usize) -> Result<Term, String> {
        if depth > 64 {
            return Err("predicate nesting limit".into());
        }
        let token = self
            .tokens
            .get(self.cursor)
            .ok_or("expected predicate term")?
            .clone();
        self.cursor += 1;
        let mut left = match token {
            TokenKind::Integer(number) => Term::Literal(
                number
                    .parse()
                    .map_err(|_| format!("invalid predicate integer {number}"))?,
            ),
            TokenKind::Word(word) if word == "true" => Term::Literal(1),
            TokenKind::Word(word) if word == "false" => Term::Literal(0),
            TokenKind::Word(word) if word == "not" => {
                Term::Not(Box::new(self.expression(6, depth + 1)?))
            }
            TokenKind::Word(word) if matches!(word.as_str(), "and" | "or") => {
                return Err(format!("misplaced connective {word}"));
            }
            TokenKind::Symbol(Symbol::Bang) => Term::Not(Box::new(self.expression(6, depth + 1)?)),
            TokenKind::Word(word) => {
                // A call (`old(x)`, `resource_released(p, …)`) is outside the
                // fragment; so is any other token after a name below.
                if self.tokens.get(self.cursor) == Some(&TokenKind::Symbol(Symbol::LeftParen)) {
                    return Err(format!("call `{word}(…)` is outside the equality fragment"));
                }
                Term::Name(word)
            }
            TokenKind::Symbol(Symbol::Minus) => match self.tokens.get(self.cursor) {
                Some(TokenKind::Integer(number)) => {
                    self.cursor += 1;
                    Term::Literal(
                        format!("-{number}")
                            .parse()
                            .map_err(|_| format!("invalid predicate integer -{number}"))?,
                    )
                }
                _ => return Err("arithmetic negation is outside the equality fragment".into()),
            },
            TokenKind::Symbol(Symbol::LeftParen) => {
                let inner = self.expression(0, depth + 1)?;
                if self.tokens.get(self.cursor) != Some(&TokenKind::Symbol(Symbol::RightParen)) {
                    return Err("missing predicate closing parenthesis".into());
                }
                self.cursor += 1;
                inner
            }
            other => return Err(format!("unsupported predicate term {other:?}")),
        };
        while let Some(operator) = self.tokens.get(self.cursor).cloned() {
            let priority = match &operator {
                TokenKind::Word(word) if word == "or" => 1,
                TokenKind::Word(word) if word == "and" => 2,
                TokenKind::Symbol(Symbol::EqualEqual | Symbol::NotEqual) => 3,
                TokenKind::Symbol(
                    Symbol::Less
                    | Symbol::LessEqual
                    | Symbol::Greater
                    | Symbol::GreaterEqual
                    | Symbol::Plus
                    | Symbol::Minus
                    | Symbol::Star,
                ) => {
                    return Err(format!(
                        "ordering or arithmetic ({operator:?}) is outside the equality fragment"
                    ));
                }
                _ => break,
            };
            if priority < minimum {
                break;
            }
            self.cursor += 1;
            let right = Box::new(self.expression(priority + 1, depth + 1)?);
            let boxed = Box::new(left);
            left = match operator {
                TokenKind::Word(word) if word == "or" => Term::Or(boxed, right),
                TokenKind::Word(word) if word == "and" => Term::And(boxed, right),
                TokenKind::Symbol(Symbol::EqualEqual) => Term::Equal(boxed, right),
                TokenKind::Symbol(Symbol::NotEqual) => Term::NotEqual(boxed, right),
                _ => unreachable!("priority table covers every operator"),
            };
        }
        Ok(left)
    }
}

impl Term {
    fn evaluate(&self, values: &BTreeMap<&str, i128>) -> Result<i128, String> {
        Ok(match self {
            Self::Literal(value) => *value,
            Self::Name(name) => *values
                .get(name.as_str())
                .ok_or_else(|| format!("unassigned ghost name {name}"))?,
            Self::Not(inner) => i128::from(inner.evaluate(values)? == 0),
            Self::Equal(left, right) => {
                i128::from(left.evaluate(values)? == right.evaluate(values)?)
            }
            Self::NotEqual(left, right) => {
                i128::from(left.evaluate(values)? != right.evaluate(values)?)
            }
            Self::And(left, right) => {
                let left = left.evaluate(values)?;
                let right = right.evaluate(values)?;
                i128::from(left != 0 && right != 0)
            }
            Self::Or(left, right) => {
                let left = left.evaluate(values)?;
                let right = right.evaluate(values)?;
                i128::from(left != 0 || right != 0)
            }
        })
    }

    fn names(&self, into: &mut BTreeSet<String>) {
        match self {
            Self::Literal(_) => {}
            Self::Name(name) => {
                into.insert(name.clone());
            }
            Self::Not(inner) => inner.names(into),
            Self::Equal(left, right)
            | Self::NotEqual(left, right)
            | Self::And(left, right)
            | Self::Or(left, right) => {
                left.names(into);
                right.names(into);
            }
        }
    }

    fn literals(&self, into: &mut BTreeSet<i128>) {
        match self {
            Self::Literal(value) => {
                into.insert(*value);
            }
            Self::Name(_) => {}
            Self::Not(inner) => inner.literals(into),
            Self::Equal(left, right)
            | Self::NotEqual(left, right)
            | Self::And(left, right)
            | Self::Or(left, right) => {
                left.literals(into);
                right.literals(into);
            }
        }
    }

    /// Pairs of names compared directly with each other (`x == y`, `x != y`).
    fn name_comparisons(&self, into: &mut Vec<(String, String)>) {
        match self {
            Self::Literal(_) | Self::Name(_) => {}
            Self::Not(inner) => inner.name_comparisons(into),
            Self::Equal(left, right) | Self::NotEqual(left, right) => {
                if let (Self::Name(a), Self::Name(b)) = (left.as_ref(), right.as_ref()) {
                    into.push((a.clone(), b.clone()));
                }
                left.name_comparisons(into);
                right.name_comparisons(into);
            }
            Self::And(left, right) | Self::Or(left, right) => {
                left.name_comparisons(into);
                right.name_comparisons(into);
            }
        }
    }
}

/// The outcome of one entailment question.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Entailment {
    pub holds: bool,
    /// Premises outside the fragment. Dropping a premise can only make an
    /// entailment harder, so this is sound; they are reported, not hidden.
    pub dropped_premises: Vec<String>,
    /// Assignments enumerated.
    pub assignments: u64,
    /// A falsifying assignment when the entailment fails.
    pub counterexample: Option<BTreeMap<String, i128>>,
}

/// Decide whether the conjunction of `premises` entails `goal` over the
/// integers, for predicates of the equality fragment.
///
/// Decision procedure: restrict the premises to the cone of influence of the
/// goal's names (dropping premises is sound), then enumerate every assignment
/// of each name over the literals of the involved predicates together with 0
/// and 1, plus fresh values above them — one per name in the largest group of
/// names compared directly with each other. In this fragment every atom
/// compares a name with a literal, a 0/1-valued subterm, or another name, and
/// connectives read only zero versus nonzero; so any integer assignment can be
/// mapped to one in that finite domain with every atom's truth value kept
/// (literals and 0/1 are fixed, other values map injectively per comparison
/// group to fresh nonzero values). The enumeration therefore decides the
/// question exactly; beyond [`MAX_ENUMERATED_ASSIGNMENTS`] it refuses.
pub fn entails(premises: &[&str], goal: &str) -> Result<Entailment, String> {
    let goal_term = FragmentParser::parse(goal).map_err(|error| format!("goal: {error}"))?;
    let mut parsed = Vec::new();
    let mut dropped = Vec::new();
    for premise in premises {
        match FragmentParser::parse(premise) {
            Ok(term) => parsed.push(term),
            Err(_) => dropped.push((*premise).to_owned()),
        }
    }
    // Cone of influence.
    let mut names = BTreeSet::new();
    goal_term.names(&mut names);
    let mut selected = vec![false; parsed.len()];
    loop {
        let mut grew = false;
        for (index, term) in parsed.iter().enumerate() {
            if selected[index] {
                continue;
            }
            let mut own = BTreeSet::new();
            term.names(&mut own);
            if own.is_empty() || own.iter().any(|name| names.contains(name)) {
                selected[index] = true;
                names.extend(own);
                grew = true;
            }
        }
        if !grew {
            break;
        }
    }
    let premises: Vec<&Term> = parsed
        .iter()
        .zip(&selected)
        .filter_map(|(term, keep)| keep.then_some(term))
        .collect();
    // Domain.
    let mut literals = BTreeSet::from([0, 1]);
    goal_term.literals(&mut literals);
    let mut comparisons = Vec::new();
    goal_term.name_comparisons(&mut comparisons);
    for term in &premises {
        term.literals(&mut literals);
        term.name_comparisons(&mut comparisons);
    }
    let largest_group = largest_comparison_group(&names, &comparisons);
    let top = *literals.iter().next_back().expect("0 and 1 are present");
    let mut domain: Vec<i128> = literals.iter().copied().collect();
    for offset in 1..=i128::try_from(largest_group.max(1)).map_err(|e| e.to_string())? {
        domain.push(top.checked_add(offset).ok_or("domain overflow")?);
    }
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    let mut total: u64 = 1;
    for _ in &names {
        total = total
            .checked_mul(domain.len() as u64)
            .filter(|total| *total <= MAX_ENUMERATED_ASSIGNMENTS)
            .ok_or_else(|| {
                format!(
                    "entailment domain too large to enumerate: {} names over {} values",
                    names.len(),
                    domain.len()
                )
            })?;
    }
    let mut indices = vec![0usize; names.len()];
    let mut assignments = 0u64;
    loop {
        assignments += 1;
        let values: BTreeMap<&str, i128> = names
            .iter()
            .zip(&indices)
            .map(|(name, index)| (*name, domain[*index]))
            .collect();
        let mut premises_hold = true;
        for term in &premises {
            if term.evaluate(&values)? == 0 {
                premises_hold = false;
                break;
            }
        }
        if premises_hold && goal_term.evaluate(&values)? == 0 {
            return Ok(Entailment {
                holds: false,
                dropped_premises: dropped,
                assignments,
                counterexample: Some(
                    values
                        .into_iter()
                        .map(|(name, value)| (name.to_owned(), value))
                        .collect(),
                ),
            });
        }
        // Next assignment (odometer).
        let mut position = 0;
        loop {
            if position == indices.len() {
                return Ok(Entailment {
                    holds: true,
                    dropped_premises: dropped,
                    assignments,
                    counterexample: None,
                });
            }
            indices[position] += 1;
            if indices[position] < domain.len() {
                break;
            }
            indices[position] = 0;
            position += 1;
        }
    }
}

fn largest_comparison_group(names: &BTreeSet<String>, comparisons: &[(String, String)]) -> usize {
    let index: BTreeMap<&str, usize> = names
        .iter()
        .enumerate()
        .map(|(position, name)| (name.as_str(), position))
        .collect();
    let mut parent: Vec<usize> = (0..names.len()).collect();
    fn find(parent: &mut [usize], mut node: usize) -> usize {
        while parent[node] != node {
            parent[node] = parent[parent[node]];
            node = parent[node];
        }
        node
    }
    for (left, right) in comparisons {
        if let (Some(&a), Some(&b)) = (index.get(left.as_str()), index.get(right.as_str())) {
            let (a, b) = (find(&mut parent, a), find(&mut parent, b));
            parent[a] = b;
        }
    }
    let mut sizes = BTreeMap::new();
    for node in 0..names.len() {
        *sizes.entry(find(&mut parent, node)).or_insert(0usize) += 1;
    }
    sizes.into_values().max().unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Signatures
// ---------------------------------------------------------------------------

/// The ABI category of one QCore type position: a scalar spelling, `unit`, or
/// a pointer with its mutability (the pointee's nominal is erased, because the
/// resource-v1 vocabulary projects every custody pointer onto `u8`).
fn abi_category(type_source: &str) -> Result<String, String> {
    const SCALARS: &[&str] = &[
        "bool", "i8", "i16", "i32", "i64", "isize", "u8", "u16", "u32", "u64", "usize", "f32",
        "f64", "unit",
    ];
    if SCALARS.contains(&type_source) {
        return Ok(type_source.to_owned());
    }
    if let Some(inner) = type_source
        .strip_prefix("ptr<")
        .and_then(|rest| rest.strip_suffix('>'))
    {
        let (mutability, pointee) = inner
            .split_once(',')
            .ok_or_else(|| format!("unsupported pointer type {type_source}"))?;
        if !matches!(mutability, "mut" | "const") {
            return Err(format!("unsupported pointer mutability {type_source}"));
        }
        let pointee = if pointee.starts_with("ptr<") {
            abi_category(pointee)?
        } else {
            "_".to_owned()
        };
        return Ok(format!("ptr<{mutability},{pointee}>"));
    }
    if type_source.starts_with("own<") {
        return Ok("ptr<mut,_>".into());
    }
    if let Some(inner) = type_source
        .strip_prefix("borrow<")
        .and_then(|rest| rest.strip_suffix('>'))
    {
        let parts: Vec<&str> = inner.split(',').collect();
        if let [_, mutability, _] = parts.as_slice()
            && matches!(*mutability, "mut" | "const")
        {
            return Ok(format!("ptr<{mutability},_>"));
        }
    }
    Err(format!("type {type_source} has no ABI category"))
}

/// The nominal a typed pointer position names (`own<EventContext>` →
/// `EventContext`), if any.
fn pointer_nominal(type_source: &str) -> Option<&str> {
    let inner = type_source
        .strip_prefix("own<")
        .or_else(|| type_source.strip_prefix("borrow<"))
        .or_else(|| type_source.strip_prefix("ptr<"))?
        .strip_suffix('>')?;
    inner.rsplit(',').next()
}

// These heads identify a validation obligation, not permission to skip a clause.
fn symbolic_metadata_head(expression: &str) -> bool {
    crate::premise_flow::is_clause(expression)
        || crate::contract_lifecycle::is_clause(expression)
        || crate::guarded_decomposition::is_source_clause(expression)
}

#[derive(PartialEq, Eq)]
struct ValidatedSymbolicMetadata {
    guarded: Option<crate::guarded_decomposition::Inline>,
    flows: BTreeMap<String, qcore_schemas::premise_flow::Expr>,
    lifecycle: BTreeMap<String, crate::contract_lifecycle::Entry>,
}

fn validated_symbolic_metadata(
    view: &ViewDeclaration,
    graph: &qcore_schemas::hypothesis::Graph,
) -> Result<ValidatedSymbolicMetadata, String> {
    // The caller first ran the complete property loader and original graph /
    // erasure checks. Still validate every entry, reference and placement here;
    // neither an unused entry nor a familiar head is a neutral fallback.
    let program = parse_source_program(&format!("module contract;\n{}\n", view.text))
        .map_err(|error| error.to_string())?;
    let declaration = program
        .declarations
        .iter()
        .find(|declaration| declaration.value.name == view.name)
        .ok_or("null view: missing actual declaration for metadata replay")?;
    Ok(ValidatedSymbolicMetadata {
        guarded: crate::guarded_decomposition::declaration(&declaration.value, graph)?,
        flows: crate::premise_flow::declaration(&declaration.value, graph)?,
        lifecycle: crate::contract_lifecycle::declaration(&declaration.value, graph)?,
    })
}

/// Positional agreement of two declarations' signatures: equal arity, equal
/// ABI category at every position and for the result.
/// Card NULL-SAFETY-PROPERTY: the bounds declaration of a
/// [`RelationCheck::NullDerefBeside`] pair states the bounds selector and
/// `quant null-deref` checks only, so it says nothing custody could read.
fn check_null_deref_only(
    declaration: &ViewDeclaration,
    resource: &ViewDeclaration,
) -> Result<(), String> {
    // Graph metadata is neutral only after this exact bounds/property source
    // passes the full loader and same-name legacy-erasure replay. A familiar
    // head alone never licenses a new semantic clause or premise.
    let has_metadata = declaration
        .clauses
        .iter()
        .any(|clause| crate::hypothesis::is_clause(&clause.expression))
        || [declaration, resource].into_iter().any(|view| {
            view.clauses
                .iter()
                .any(|clause| symbolic_metadata_head(&clause.expression))
        });
    if has_metadata {
        let sources = BTreeMap::from([(
            declaration.module.clone(),
            format!("module contract;\n{}\n", declaration.text),
        )]);
        let bounds_graphs = crate::bounds_hypotheses::replay(&sources)?;
        let resource_sources = BTreeMap::from([(
            resource.module.clone(),
            format!("module contract;\n{}\n", resource.text),
        )]);
        let resource_graphs = crate::hypothesis::replay(&resource_sources)?;
        let graph = bounds_graphs
            .get(&declaration.name)
            .ok_or("null view: missing replayed bounds graph")?;
        let peer = resource_graphs
            .get(&resource.name)
            .ok_or("null view: missing replayed resource graph")?;
        // Compare proposed metadata only. Original premise vectors, graph
        // identities and property Contexts are independently retained.
        for (name, attachment) in &graph.refinements {
            let expected = peer
                .refinements
                .get(name)
                .ok_or("null view: metadata has no independently declared resource peer")?;
            if attachment.children != expected.children
                || graph.premises[&attachment.premise].erasure
                    != peer.premises[&expected.premise].erasure
            {
                return Err("null view: neutral attachment differs from resource peer".into());
            }
        }
        for (id, atom) in &graph.atoms {
            if !id.starts_with("legacy_atom_") && peer.atoms.get(id) != Some(atom) {
                return Err("null view: proposed atom differs from resource peer".into());
            }
        }
        // Match complete maps, including IDs and every AST / typed target.
        // Contexts, legacy premise vectors and graph identities stay separate;
        // this comparison produces no interpretation or evidence disposition.
        let null_metadata = validated_symbolic_metadata(declaration, graph)?;
        let resource_metadata = validated_symbolic_metadata(resource, peer)?;
        // The existing pure-null relation has no proposed graph to share.
        // Its resource peer remains fully validated but nothing crosses into
        // null scope. Any introduced null graph or metadata leaves this case.
        let independent_pure_null = graph.atoms.is_empty()
            && graph.premises.is_empty()
            && graph.refinements.is_empty()
            && null_metadata.flows.is_empty()
            && null_metadata.lifecycle.is_empty()
            && null_metadata.guarded.is_none()
            && declaration.clauses.iter().all(|clause| {
                (clause.kind == "requires" && clause.expression == "bounds_contract_v1")
                    || (has_metadata && clause.kind == "quant proposition")
                    || clause.kind == "quant null-deref"
            });
        if !independent_pure_null && null_metadata != resource_metadata {
            return Err("null view: neutral symbolic metadata differs from resource peer".into());
        }
    }
    let only = !declaration.clauses.is_empty()
        && declaration.clauses.iter().all(|clause| {
            (clause.kind == "requires" && clause.expression == "bounds_contract_v1")
                || (has_metadata
                    && clause.kind == "requires"
                    && (crate::hypothesis::is_clause(&clause.expression)
                        || symbolic_metadata_head(&clause.expression)))
                || (has_metadata && clause.kind == "quant proposition")
                || clause.kind == "quant null-deref"
                // Card NULL-CANDIDATE-TRIAGE: the null property's member and
                // result declarations (G05e), which custody never reads; card
                // NONNULL-MEMBER-UNIFIED-QUALIFIER: every kind of the member
                // clause, the stable member and the function-scope entry
                // (G05f) included (their retired words are the bounds
                // loader's rejection).
                || (clause.kind == "requires"
                    && clause.expression.starts_with("nonnull_member("))
                || (clause.kind == "normal"
                    && clause.expression.replace(' ', "") == "nonnull_result()")
                // Card NULL-TRIAGE-UNRESOLVED-PATHS: the result range (G05e).
                || (clause.kind == "normal"
                    && clause.expression.starts_with("nonnull_result_range("))
                // Card LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER: a member fact's
                // named premise (its placement beside a member clause is the
                // bounds loader's check).
                || (clause.kind == "requires"
                    && clause.expression.starts_with("resource_premise("))
                // Card CELL-GUARD-RESULT-RANGE-CHECKED (C09.4, G05j): a C
                // library result set and its flexible marker, a value set
                // that states no custody.
                || (clause.kind == "requires"
                    && clause.expression.replace(' ', "") == "bounds_revision(flexible)")
                || (clause.kind == "normal" && clause.expression.starts_with("result_set("))
                // Card CELL-RESULT-SET-WRITTEN-SET-DERIVATION (G05j as
                // amended): a slot's written set, a value set that states no
                // custody (a pinned creator's, derived at pin time).
                || (clause.kind == "normal" && clause.expression.starts_with("written_set("))
        });
    if only {
        Ok(())
    } else {
        Err(format!(
            "`{}` in {} is registered as a null-deref check beside its resource view but states other clauses",
            declaration.name, declaration.module
        ))
    }
}

fn check_signature_agreement(
    left: &ViewDeclaration,
    right: &ViewDeclaration,
) -> Result<Vec<String>, String> {
    if left.parameters.len() != right.parameters.len() {
        return Err(format!(
            "arity disagrees: {} has {}, {} has {}",
            left.module,
            left.parameters.len(),
            right.module,
            right.parameters.len()
        ));
    }
    let mut agreed = Vec::new();
    for (index, (a, b)) in left.parameters.iter().zip(&right.parameters).enumerate() {
        let (ca, cb) = (abi_category(&a.type_source)?, abi_category(&b.type_source)?);
        if ca != cb {
            return Err(format!(
                "parameter {index} disagrees: `{}: {}` in {} is {ca}, `{}: {}` in {} is {cb}",
                a.name, a.type_source, left.module, b.name, b.type_source, right.module
            ));
        }
        agreed.push(format!("parameter {index}: {ca}"));
    }
    let (ra, rb) = (abi_category(&left.result)?, abi_category(&right.result)?);
    if ra != rb {
        return Err(format!(
            "result disagrees: {} returns {ra}, {} returns {rb}",
            left.module, right.module
        ));
    }
    agreed.push(format!("result: {ra}"));
    Ok(agreed)
}

// ---------------------------------------------------------------------------
// Bridges: what a resource-v1 clause means in a detailed vocabulary
// ---------------------------------------------------------------------------

/// How the parameter a custody rule names must be declared in the detailed
/// contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum ParameterShape {
    /// Any pointer position; custody is carried by the rule's premise.
    Pointer,
    /// `own<T> use one`: the language front end's consumed ownership.
    OwnedConsumed,
}

/// The meaning of one resource-v1 clause in a detailed vocabulary.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum RuleMeaning {
    /// `requires resource_contract_v1`: the selector, no obligation.
    Marker,
    /// `resource_borrowed($*)` on a scalar: scalars carry no custody, so the
    /// obligation is that the position is a scalar in both declarations.
    ScalarBorrowed,
    /// `resource_return_none()`: the detailed result is `unit` too.
    UnitResult,
    /// A result or parameter custody mode. The v1 checker's call-site custody
    /// discipline establishes `premise`; the detailed contract must entail
    /// `obligation` from it together with its own clauses.
    Custody {
        premise: &'static str,
        obligation: &'static str,
        shape: Option<ParameterShape>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct BridgeRule {
    /// The v1 clause, whitespace-free, parameter names replaced by `$*`.
    pub view_clause: &'static str,
    pub meaning: RuleMeaning,
}

/// A declared translation from one detailed vocabulary to resource-v1.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Bridge {
    pub name: &'static str,
    /// Value domains the vocabulary's producer guarantees (bridge premises).
    pub domains: &'static [&'static str],
    pub rules: &'static [BridgeRule],
}

const MARKER: BridgeRule = BridgeRule {
    view_clause: "resource_contract_v1",
    meaning: RuleMeaning::Marker,
};
const SCALAR_BORROWED: BridgeRule = BridgeRule {
    view_clause: "resource_borrowed($*)",
    meaning: RuleMeaning::ScalarBorrowed,
};
const RETURN_NONE: BridgeRule = BridgeRule {
    view_clause: "resource_return_none()",
    meaning: RuleMeaning::UnitResult,
};

/// `fluent_bit/memory.qv`'s allocator-observation ghosts (the native
/// evaluation's adapter, `docs/reports/cycle19-native-contracts/`):
/// `result_null` is the 0/1 null test of the returned pointer, `live_after`
/// counts tracked live allocations after the call, `result_is_backend` says
/// the returned pointer is the backend allocator's, `valid_input_extent` says
/// the input is null or a live allocation of the same allocator, and
/// `forwarded_input` says the backend received that input.
pub const ALLOCATOR_OBSERVATION_BRIDGE: Bridge = Bridge {
    name: "allocator-observation",
    domains: &["result_null == 0 or result_null == 1"],
    rules: &[
        MARKER,
        SCALAR_BORROWED,
        RETURN_NONE,
        BridgeRule {
            // Null, with nothing live; or a fresh backend allocation, live,
            // handed to the caller.
            view_clause: "resource_return_owned(nullable)",
            meaning: RuleMeaning::Custody {
                premise: "true",
                obligation: "(result_null == 1 and live_after == 0) or \
                             (result_null == 0 and live_after == 1 and result_is_backend == 1)",
                shape: None,
            },
        },
        BridgeRule {
            // v1 nullable custody of the argument is the observation's valid
            // input (null, or a live allocation of this allocator); release
            // forwards it to the backend and leaves nothing live.
            view_clause: "resource_released($*,nullable)",
            meaning: RuleMeaning::Custody {
                premise: "valid_input_extent == 1",
                obligation: "live_after == 0 and forwarded_input == 1",
                shape: Some(ParameterShape::Pointer),
            },
        },
    ],
};

/// `chunkio/memory_close.qv`'s lifecycle-mechanism propositions
/// (`qcore_verification::memory_close_composition`): `nullable_owned_chunk`
/// is the caller's raw nullable ownership, `chunk_was_nonnull` the entry null
/// test, `chunk_released` the release of the chunk object itself.
pub const CHUNK_CLOSE_MECHANISM_BRIDGE: Bridge = Bridge {
    name: "chunk-close-mechanism",
    domains: &[],
    rules: &[
        MARKER,
        SCALAR_BORROWED,
        RETURN_NONE,
        BridgeRule {
            view_clause: "resource_released($*,nonnull)",
            meaning: RuleMeaning::Custody {
                premise: "nullable_owned_chunk and chunk_was_nonnull",
                obligation: "chunk_released",
                shape: Some(ParameterShape::Pointer),
            },
        },
    ],
};

/// `chunkio/chunk_destructor.qv`'s language lifecycle: the chunk is
/// `own<Chunk> use one` (consumed), `chunk_is_live` its liveness, and
/// `chunk_name_and_object_are_released` the release of the chunk object.
pub const CHUNK_CLOSE_LANGUAGE_BRIDGE: Bridge = Bridge {
    name: "chunk-close-language",
    domains: &[],
    rules: &[
        MARKER,
        SCALAR_BORROWED,
        RETURN_NONE,
        BridgeRule {
            view_clause: "resource_released($*,nonnull)",
            meaning: RuleMeaning::Custody {
                premise: "chunk_is_live",
                obligation: "chunk_name_and_object_are_released",
                shape: Some(ParameterShape::OwnedConsumed),
            },
        },
    ],
};

// ---------------------------------------------------------------------------
// The registry
// ---------------------------------------------------------------------------

/// Why a detailed premise the view does not discharge is acceptable, or not.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PremiseClass {
    /// A build/configuration profile fixed for the audited build.
    Profile,
    /// A caller or environment obligation the v1 checker does not track.
    Environment,
    /// A restriction of which paths the detailed contract covers at all: the
    /// view claims more than the detailed contract supports outside it.
    Coverage,
}

/// The relation a registered pair stands in, and how it is checked.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum RelationCheck {
    /// Token-identical declarations (a duplicate kept only because a consumer
    /// cannot include; none is registered today).
    Identical,
    /// The detailed contract entails the resource-v1 view under a bridge.
    Projection { bridge: &'static Bridge },
    /// Token-equal under `renaming` (weaker-module word → stronger-module
    /// word), and equal results under both frame evaluators whenever the
    /// protected set excludes the declared write footprint.
    RenamedFrame {
        renaming: &'static [(&'static str, &'static str)],
        record: &'static str,
        /// Protected sets the frame helper imports really declare.
        protected_sets: &'static [&'static [&'static str]],
    },
    /// Two lifecycle profiles in different vocabularies; neither entails the
    /// other. Checked: every position's ABI category and the result.
    LifecycleProfiles,
    /// Card NULL-SAFETY-PROPERTY (gateway rule 14o): a bounds contract v1
    /// declaration holding only `quant null-deref` checks beside the
    /// resource-v1 declaration of the same function. The two vocabularies
    /// state disjoint properties (null dereference, custody) and no consumer
    /// reads both; checked: every position's ABI category and the result, and
    /// that the bounds declaration states nothing but null-deref checks.
    NullDerefBeside,
    /// `contract_NAME`/`NAME` source-shape metadata in the detailed module
    /// against the typed declaration in the view module. Checked: every
    /// recorded C signature alternative against the typed signature, and every
    /// recorded declaration fact against the typed declaration.
    SourceShape {
        /// Typed parameter `i` corresponds to C parameter `projection[i]`;
        /// `None` means positionally, with equal arity.
        projection: Option<&'static [usize]>,
        /// C struct tag → QCore nominal.
        nominals: &'static [(&'static str, &'static str)],
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct RegisteredRelation {
    pub function: &'static str,
    /// The stronger/detailed (or metadata) module.
    pub detailed: &'static str,
    /// The weaker view (or typed) module.
    pub view: &'static str,
    pub check: RelationCheck,
    /// The detailed premises the view does not discharge, computed by the
    /// check and pinned here with their classes.
    pub assumed_premises: &'static [(&'static str, PremiseClass)],
    /// A registered follow-up recording a finding about this pair.
    pub finding: Option<&'static str>,
}

const MONKEY_NOMINALS: &[(&str, &str)] = &[
    ("mk_event_loop", "EventLoop"),
    ("mk_event_ctx", "EventContext"),
];

const MEMORY_CLOSE_ASSUMED: &[(&str, PremiseClass)] = &[
    ("nonnull_chunk_has_memory_backend", PremiseClass::Coverage),
    (
        "backend_and_name_have_disjoint_owned_provenance",
        PremiseClass::Environment,
    ),
    (
        "both_links_are_live_reciprocal_memberships",
        PremiseClass::Environment,
    ),
    (
        "embedded_links_have_distinct_stream_memberships",
        PremiseClass::Environment,
    ),
    (
        "context_and_stream_outlive_chunk",
        PremiseClass::Environment,
    ),
    (
        "context_counter_covers_live_chunk",
        PremiseClass::Environment,
    ),
    (
        "no_lock_busy_task_route_or_transaction_obligations",
        PremiseClass::Environment,
    ),
    ("exclusive_access", PremiseClass::Environment),
    ("cross_unit_source_and_abi_bindings", PremiseClass::Profile),
    ("free_resolves_to_libc", PremiseClass::Profile),
];

/// Every pair of modules declaring the same function, and its checked
/// relation. The census guard refuses a pair missing here and an entry whose
/// pair no longer exists.
pub const REGISTERED_RELATIONS: &[RegisteredRelation] = &[
    // Fluent Bit allocator wrappers: the native-evaluated allocator contract
    // against the resource-v1 view (which inherits libc's clauses).
    RegisteredRelation {
        function: "flb_calloc",
        detailed: "fluent_bit/memory.qv",
        view: "fluent_bit/resource_memory.qv",
        check: RelationCheck::Projection {
            bridge: &ALLOCATOR_OBSERVATION_BRIDGE,
        },
        assumed_premises: &[("ordinary_allocator_profile == 1", PremiseClass::Profile)],
        finding: None,
    },
    RegisteredRelation {
        function: "flb_free",
        detailed: "fluent_bit/memory.qv",
        view: "fluent_bit/resource_memory.qv",
        check: RelationCheck::Projection {
            bridge: &ALLOCATOR_OBSERVATION_BRIDGE,
        },
        assumed_premises: &[("ordinary_allocator_profile == 1", PremiseClass::Profile)],
        finding: None,
    },
    RegisteredRelation {
        function: "flb_malloc",
        detailed: "fluent_bit/memory.qv",
        view: "fluent_bit/resource_memory.qv",
        check: RelationCheck::Projection {
            bridge: &ALLOCATOR_OBSERVATION_BRIDGE,
        },
        assumed_premises: &[("ordinary_allocator_profile == 1", PremiseClass::Profile)],
        finding: None,
    },
    // ChunkIO close: two lifecycle profiles and the resource-v1 view.
    RegisteredRelation {
        function: "cio_chunk_close",
        detailed: "chunkio/chunk_destructor.qv",
        view: "chunkio/resource_open_close.qv",
        check: RelationCheck::Projection {
            bridge: &CHUNK_CLOSE_LANGUAGE_BRIDGE,
        },
        assumed_premises: &[
            (
                "quant-condition-entry:r024:97600c83feb29b145764ac6d21cdd3c027eedb582c3a888b1e078fdb9a6a238a",
                PremiseClass::Environment,
            ),
            (
                "quant-condition-entry:r025:43d691a1c163627f640177bf3f2078be4b2cb6d4f0ccf5dec13f0a0d2f2b39b4",
                PremiseClass::Environment,
            ),
            (
                "quant-condition-entry:r023:d0f98b4fa015afc57a59a876970cadec48da5f33275141ef5cefd2a7024815ec",
                PremiseClass::Environment,
            ),
        ],
        finding: Some("CONTRACT-VIEW-UNENFORCED-PREMISES"),
    },
    RegisteredRelation {
        function: "cio_chunk_close",
        detailed: "chunkio/memory_close.qv",
        view: "chunkio/resource_open_close.qv",
        check: RelationCheck::Projection {
            bridge: &CHUNK_CLOSE_MECHANISM_BRIDGE,
        },
        assumed_premises: MEMORY_CLOSE_ASSUMED,
        finding: Some("CONTRACT-VIEW-CHUNK-CLOSE-BACKEND-COVERAGE"),
    },
    RegisteredRelation {
        function: "cio_chunk_close",
        detailed: "chunkio/chunk_destructor.qv",
        view: "chunkio/memory_close.qv",
        check: RelationCheck::LifecycleProfiles,
        assumed_premises: &[],
        finding: None,
    },
    // Card W45-FB-RETRY-LIFETIME: Monkey's list unlink, as the ChunkIO frame
    // contract states its link-cell writes and the resource-v1 view states the
    // retired publication (§C04.8). Different vocabularies; neither entails
    // the other (the frame contract's `no_object_release` is compatible with
    // an unlink that releases nothing).
    RegisteredRelation {
        function: "mk_list_del",
        detailed: "chunkio/list_unlink.qv",
        view: "monkey/intrusive_list.qv",
        check: RelationCheck::LifecycleProfiles,
        assumed_premises: &[],
        finding: None,
    },
    // Card NULL-SAFETY-PROPERTY (gateway rule 14o): `quant null-deref`
    // checks (bounds contract v1) beside the resource-v1 declarations of the
    // same functions: disjoint properties, checked for signature agreement.
    RegisteredRelation {
        function: "cmt_destroy",
        detailed: "cmetrics/null_deref.qv",
        view: "cmetrics/context_lifetime.qv",
        check: RelationCheck::NullDerefBeside,
        assumed_premises: &[],
        finding: None,
    },
    RegisteredRelation {
        function: "flb_input_chunk_get_name",
        detailed: "fluent_bit/null_deref.qv",
        view: "fluent_bit/input_chunk_name.qv",
        check: RelationCheck::NullDerefBeside,
        assumed_premises: &[],
        finding: None,
    },
    // Card FB-PLUGIN-DESCRIPTOR-NONNULL-SCOPE (rule 14x): the input
    // constructor hosts the stable plugin descriptor beside its resource-v1
    // declaration.
    RegisteredRelation {
        function: "flb_input_new",
        detailed: "fluent_bit/null_deref.qv",
        view: "fluent_bit/input_instance.qv",
        check: RelationCheck::NullDerefBeside,
        assumed_premises: &[],
        finding: None,
    },
    // ... and the input worker's exit signal carries a function-scope entry
    // (rule 14z) beside its held resource-v1 declaration.
    RegisteredRelation {
        function: "flb_input_thread_instance_exit",
        detailed: "fluent_bit/null_deref.qv",
        view: "fluent_bit/lib_engine_thread.qv",
        check: RelationCheck::NullDerefBeside,
        assumed_premises: &[],
        finding: None,
    },
    // Card NULL-CANDIDATE-TRIAGE (rule 14t): the toppar constructor hosts a
    // declared non-null member beside its resource-v1 declaration.
    RegisteredRelation {
        function: "rd_kafka_toppar_new0",
        detailed: "librdkafka/null_deref.qv",
        view: "librdkafka/client.qv",
        check: RelationCheck::NullDerefBeside,
        assumed_premises: &[],
        finding: None,
    },
    // Card NULL-CANDIDATE-TRIAGE (rule 14u): a declared non-null result
    // beside the resource-v1 declaration of the same function.
    RegisteredRelation {
        function: "rd_kafka_topic_keep",
        detailed: "librdkafka/null_deref.qv",
        view: "librdkafka/client.qv",
        check: RelationCheck::NullDerefBeside,
        assumed_premises: &[],
        finding: None,
    },
    // Card NULL-MEMBER-INVARIANT-SCOPE (rule 14z): a function-scope entry
    // beside the resource-v1 declaration of the same function.
    RegisteredRelation {
        function: "rd_kafka_toppar_fetch_backoff",
        detailed: "librdkafka/null_deref.qv",
        view: "librdkafka/client.qv",
        check: RelationCheck::NullDerefBeside,
        assumed_premises: &[],
        finding: None,
    },
    RegisteredRelation {
        function: "rd_kafka_topic_partition_list_destroy",
        detailed: "librdkafka/null_deref.qv",
        view: "librdkafka/client.qv",
        check: RelationCheck::NullDerefBeside,
        assumed_premises: &[],
        finding: None,
    },
    RegisteredRelation {
        function: "rd_list_add",
        detailed: "librdkafka/null_deref.qv",
        view: "librdkafka/client.qv",
        check: RelationCheck::NullDerefBeside,
        assumed_premises: &[],
        finding: None,
    },
    // Card CG22-OP-CALLBACKS-AND-SERVED-QUEUES (rules 12n, 12o): the op
    // destroyer's resource-v1 declaration (the release, the op's member
    // callback, the arms) beside its `quant null-deref #rko`.
    RegisteredRelation {
        function: "rd_kafka_op_destroy",
        detailed: "librdkafka/null_deref.qv",
        view: "librdkafka/client.qv",
        check: RelationCheck::NullDerefBeside,
        assumed_premises: &[],
        finding: None,
    },
    RegisteredRelation {
        function: "rd_list_destroy",
        detailed: "librdkafka/null_deref.qv",
        view: "librdkafka/client.qv",
        check: RelationCheck::NullDerefBeside,
        assumed_premises: &[],
        finding: None,
    },
    // Card W45-CALLEE-CASCADE-RESIDUAL round 3: `cfl_list_size`'s resource-v1
    // declaration beside the null property's result range (rule 15q).
    RegisteredRelation {
        function: "cfl_list_size",
        detailed: "cfl/null_deref.qv",
        view: "cfl/sds_lifetime.qv",
        check: RelationCheck::NullDerefBeside,
        assumed_premises: &[],
        finding: None,
    },
    // Card CELL-GUARD-RESULT-RANGE-CHECKED (rule 38e): the channel creator's
    // pin-derived result range beside its resource-v1 declaration.
    RegisteredRelation {
        function: "mk_event_channel_create",
        detailed: "monkey/result_range.qv",
        view: "monkey/event_loop.qv",
        check: RelationCheck::NullDerefBeside,
        assumed_premises: &[],
        finding: None,
    },
    // Card CELL-GUARD-RESULT-RANGE-CHECKED (C09.4): `open`'s C library
    // result set (a value set, no custody) beside its resource-v1
    // declaration, which it leaves exactly as it is.
    RegisteredRelation {
        function: "open",
        detailed: "libc/result_sets.qv",
        view: "libc/unistd.qv",
        check: RelationCheck::NullDerefBeside,
        assumed_premises: &[],
        finding: None,
    },
    // ChunkIO error reset: the generic configured frame policy and the
    // retained Chunk-flavour policy.
    RegisteredRelation {
        function: "cio_error_reset",
        detailed: "chunkio/error_frame.qv",
        view: "chunkio/error_state.qv",
        check: RelationCheck::RenamedFrame {
            renaming: &[
                ("Chunk", "cio_chunk"),
                ("chunk_is_live", "object_is_live"),
                ("exclusive_chunk_access", "exclusive_object_access"),
                ("lock_is_preserved", "modeled_fields_are_preserved"),
            ],
            record: "cio_chunk",
            protected_sets: &[&["lock"], &["busy", "lock"]],
        },
        assumed_premises: &[],
        finding: Some("CONTRACT-VIEW-ERROR-RESET-RENAMED-EQUIVALENT"),
    },
    // Monkey: typed-policy source-shape metadata against the typed
    // declarations its consumer checks.
    RegisteredRelation {
        function: "_mk_event_loop_create",
        detailed: "monkey/wait_source_policy.qv",
        view: "monkey/wrapper_paths.qv",
        check: RelationCheck::SourceShape {
            projection: None,
            nominals: MONKEY_NOMINALS,
        },
        assumed_premises: &[],
        finding: None,
    },
    RegisteredRelation {
        function: "_mk_event_loop_destroy",
        detailed: "monkey/wait_source_policy.qv",
        view: "monkey/wrapper_paths.qv",
        check: RelationCheck::SourceShape {
            projection: None,
            nominals: MONKEY_NOMINALS,
        },
        assumed_premises: &[],
        finding: None,
    },
    RegisteredRelation {
        function: "_mk_event_wait_2",
        detailed: "monkey/wait_source_policy.qv",
        view: "monkey/wrapper_paths.qv",
        check: RelationCheck::SourceShape {
            projection: None,
            nominals: MONKEY_NOMINALS,
        },
        assumed_premises: &[],
        finding: None,
    },
    RegisteredRelation {
        function: "_mk_event_wait_with_flags",
        detailed: "monkey/wait_source_policy.qv",
        view: "monkey/wrapper_paths.qv",
        check: RelationCheck::SourceShape {
            projection: None,
            nominals: MONKEY_NOMINALS,
        },
        assumed_premises: &[],
        finding: None,
    },
    // The Libevent callback's summary declares only its user argument, the
    // C callback's third parameter.
    RegisteredRelation {
        function: "cb_wait_2_timeout",
        detailed: "monkey/wait_source_policy.qv",
        view: "monkey/composition.qv",
        check: RelationCheck::SourceShape {
            projection: Some(&[2]),
            nominals: MONKEY_NOMINALS,
        },
        assumed_premises: &[],
        finding: None,
    },
];

// ---------------------------------------------------------------------------
// Checking
// ---------------------------------------------------------------------------

/// What one relation check established.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RelationEvidence {
    pub function: String,
    pub detailed: String,
    pub view: String,
    pub relation: &'static str,
    /// One line per discharged obligation.
    pub discharged: Vec<String>,
    /// Detailed premises the view does not discharge (computed).
    pub assumed_premises: Vec<String>,
    /// Premises outside the fragment, dropped (soundly) from entailments.
    pub dropped_premises: Vec<String>,
}

fn find_declaration<'a>(
    declarations: &'a BTreeMap<String, Vec<ViewDeclaration>>,
    function: &str,
    module: &str,
) -> Result<&'a ViewDeclaration, String> {
    declarations
        .get(function)
        .and_then(|all| all.iter().find(|declaration| declaration.module == module))
        .ok_or_else(|| format!("{module} declares no `{function}`"))
}

/// Check one registered relation against the modules.
pub fn check_relation(
    relation: &RegisteredRelation,
    modules: &BTreeMap<String, String>,
) -> Result<RelationEvidence, String> {
    let declarations = collect_declarations(modules)?;
    let detailed = find_declaration(&declarations, relation.function, relation.detailed)?;
    let view = find_declaration(&declarations, relation.function, relation.view)?;
    let context = |error: String| {
        format!(
            "{} ({} / {}): {error}",
            relation.function, relation.detailed, relation.view
        )
    };
    let evidence = match relation.check {
        RelationCheck::Identical => check_identical(detailed, view),
        RelationCheck::Projection { bridge } => check_projection(detailed, view, bridge),
        RelationCheck::RenamedFrame {
            renaming,
            record,
            protected_sets,
        } => check_renamed_frame(detailed, view, renaming, record, protected_sets),
        RelationCheck::LifecycleProfiles => {
            check_signature_agreement(detailed, view).map(|discharged| RelationEvidence {
                function: detailed.name.clone(),
                detailed: detailed.module.clone(),
                view: view.module.clone(),
                relation: "lifecycle_profiles",
                discharged,
                assumed_premises: Vec::new(),
                dropped_premises: Vec::new(),
            })
        }
        RelationCheck::NullDerefBeside => check_null_deref_only(detailed, view).and_then(|()| {
            check_signature_agreement(detailed, view).map(|discharged| RelationEvidence {
                function: detailed.name.clone(),
                detailed: detailed.module.clone(),
                view: view.module.clone(),
                relation: "null_deref_beside",
                discharged,
                assumed_premises: Vec::new(),
                dropped_premises: Vec::new(),
            })
        }),
        RelationCheck::SourceShape {
            projection,
            nominals,
        } => {
            let metadata = find_declaration(
                &declarations,
                &format!("contract_{}", relation.function),
                relation.detailed,
            )?;
            check_source_shape(
                detailed,
                metadata,
                view,
                &modules[relation.detailed],
                projection,
                nominals,
            )
        }
    }
    .map_err(context)?;
    let pinned: Vec<String> = relation
        .assumed_premises
        .iter()
        .map(|(premise, _)| (*premise).to_owned())
        .collect();
    if evidence.assumed_premises != pinned {
        return Err(context(format!(
            "the view assumes {:?}, but {:?} is pinned",
            evidence.assumed_premises, pinned
        )));
    }
    if relation
        .assumed_premises
        .iter()
        .any(|(_, class)| *class == PremiseClass::Coverage)
        && relation.finding.is_none()
    {
        return Err(context(
            "a coverage-restricting premise needs a registered finding".into(),
        ));
    }
    Ok(evidence)
}

fn check_identical(
    detailed: &ViewDeclaration,
    view: &ViewDeclaration,
) -> Result<RelationEvidence, String> {
    if detailed.tokens != view.tokens {
        return Err("registered as identical, but the declarations differ".into());
    }
    Ok(RelationEvidence {
        function: detailed.name.clone(),
        detailed: detailed.module.clone(),
        view: view.module.clone(),
        relation: "identical",
        discharged: vec!["token-identical declarations".into()],
        assumed_premises: Vec::new(),
        dropped_premises: Vec::new(),
    })
}

/// A v1 clause with its parameter arguments replaced by `$*`, and the
/// positions they named.
fn normalize_view_clause(view: &ViewDeclaration, expression: &str) -> (String, Vec<usize>) {
    let compact: String = expression.split_whitespace().collect();
    let Some((head, rest)) = compact.split_once('(') else {
        return (compact, Vec::new());
    };
    let Some(arguments) = rest.strip_suffix(')') else {
        return (compact, Vec::new());
    };
    let mut positions = Vec::new();
    let arguments: Vec<String> = if arguments.is_empty() {
        Vec::new()
    } else {
        arguments
            .split(',')
            .map(|argument| {
                match view
                    .parameters
                    .iter()
                    .position(|parameter| parameter.name == argument)
                {
                    Some(position) => {
                        positions.push(position);
                        "$*".to_owned()
                    }
                    None => argument.to_owned(),
                }
            })
            .collect()
    };
    (format!("{head}({})", arguments.join(",")), positions)
}

fn check_projection(
    detailed: &ViewDeclaration,
    view: &ViewDeclaration,
    bridge: &Bridge,
) -> Result<RelationEvidence, String> {
    if vocabulary(view) != Vocabulary::ResourceV1 {
        return Err("a projection's view must be a resource-v1 declaration".into());
    }
    let mut discharged = check_signature_agreement(detailed, view)?;
    let mut view_premises: Vec<&'static str> = Vec::new();
    let mut obligations: Vec<(String, &'static str, &'static str)> = Vec::new();
    for clause in &view.clauses {
        if clause.kind == "requires" {
            if clause.expression == "resource_contract_v1"
                || clause.expression.starts_with("resource_literal_argument(")
            {
                // The marker selects; a literal premise only narrows where the
                // view applies, which cannot make it harder to entail.
                continue;
            }
            return Err(format!(
                "unbridged view precondition `{}`",
                clause.expression
            ));
        }
        if clause.kind != "normal" {
            return Err(format!("unbridged view clause kind `{}`", clause.kind));
        }
        let (normalized, positions) = normalize_view_clause(view, &clause.expression);
        let rule = bridge
            .rules
            .iter()
            .find(|rule| rule.view_clause == normalized)
            .ok_or_else(|| {
                format!(
                    "view clause `{}` has no meaning in the {} bridge",
                    clause.expression, bridge.name
                )
            })?;
        match rule.meaning {
            RuleMeaning::Marker => {}
            RuleMeaning::ScalarBorrowed => {
                let [position] = positions.as_slice() else {
                    return Err(format!("`{}` names no single parameter", clause.expression));
                };
                for declaration in [detailed, view] {
                    let category = abi_category(&declaration.parameters[*position].type_source)?;
                    if category.starts_with("ptr<") || category == "unit" {
                        return Err(format!(
                            "`{}` borrows a non-scalar position in {}",
                            clause.expression, declaration.module
                        ));
                    }
                }
                discharged.push(format!("{}: scalar, no custody", clause.expression));
            }
            RuleMeaning::UnitResult => {
                if detailed.result != "unit" {
                    return Err(format!(
                        "`{}` but the detailed result is {}",
                        clause.expression, detailed.result
                    ));
                }
                discharged.push(format!("{}: unit result", clause.expression));
            }
            RuleMeaning::Custody {
                premise,
                obligation,
                shape,
            } => {
                if let Some(shape) = shape {
                    let [position] = positions.as_slice() else {
                        return Err(format!("`{}` names no single parameter", clause.expression));
                    };
                    let parameter = &detailed.parameters[*position];
                    let admitted = match shape {
                        ParameterShape::Pointer => {
                            abi_category(&parameter.type_source)?.starts_with("ptr<")
                        }
                        ParameterShape::OwnedConsumed => {
                            parameter.type_source.starts_with("own<") && parameter.usage == "one"
                        }
                    };
                    if !admitted {
                        return Err(format!(
                            "`{}` needs a {shape:?} parameter, found `{}: {}`",
                            clause.expression, parameter.name, parameter.type_source
                        ));
                    }
                }
                view_premises.push(premise);
                obligations.push((clause.expression.clone(), premise, obligation));
            }
        }
    }
    let requires: Vec<&str> = detailed.requires().collect();
    let ensures: Vec<&str> = detailed.ensures_normal().collect();
    let mut dropped = BTreeSet::new();
    // Which detailed premises does the view's own custody discipline
    // discharge? The rest are assumptions the relation holds under.
    let mut discharging: Vec<&str> = bridge.domains.to_vec();
    discharging.extend(view_premises.iter().copied());
    let mut assumed = Vec::new();
    for requirement in &requires {
        match entails(&discharging, requirement) {
            Ok(result) if result.holds => {
                discharged.push(format!("requires {requirement}: discharged by the view"));
            }
            _ => assumed.push((*requirement).to_owned()),
        }
    }
    // The arithmetic/legacy entailer has no valuation for typed relations.
    // Keep every formal entry root explicitly assumed and pin its exact source
    // identity. An old English role cannot discharge a new relation parameter.
    for clause in detailed
        .clauses
        .iter()
        .filter(|c| c.kind == "quant condition entry")
    {
        let definition = qcore_frontend::quant_condition::parse(
            qcore_schemas::quant_condition::Selector::Entry,
            &clause.expression,
        )?;
        assumed.push(format!(
            "quant-condition-entry:{}:{}",
            definition.proposition.id,
            qcore_schemas::sha256_hex(clause.expression.as_bytes())
        ));
    }
    for (clause, premise, obligation) in obligations {
        let mut premises: Vec<&str> = bridge.domains.to_vec();
        premises.push(premise);
        premises.extend(requires.iter().copied());
        premises.extend(ensures.iter().copied());
        let result = entails(&premises, obligation)?;
        dropped.extend(result.dropped_premises.iter().cloned());
        if !result.holds {
            return Err(format!(
                "`{clause}` is not implied: under `{premise}` the detailed contract does not \
                 entail `{obligation}` (counterexample {:?})",
                result.counterexample.unwrap_or_default()
            ));
        }
        discharged.push(format!(
            "{clause}: entailed ({} assignments)",
            result.assignments
        ));
    }
    Ok(RelationEvidence {
        function: detailed.name.clone(),
        detailed: detailed.module.clone(),
        view: view.module.clone(),
        relation: "projection",
        discharged,
        assumed_premises: assumed,
        dropped_premises: dropped.into_iter().collect(),
    })
}

fn check_renamed_frame(
    stronger: &ViewDeclaration,
    weaker: &ViewDeclaration,
    renaming: &[(&str, &str)],
    record: &str,
    protected_sets: &[&[&str]],
) -> Result<RelationEvidence, String> {
    let map: BTreeMap<&str, &str> = renaming.iter().copied().collect();
    let renamed: Vec<TokenKind> = weaker
        .tokens
        .iter()
        .map(|token| match token {
            TokenKind::Word(word) => TokenKind::Word(
                map.get(word.as_str())
                    .map_or_else(|| word.clone(), |target| (*target).to_owned()),
            ),
            other => other.clone(),
        })
        .collect();
    if renamed != stronger.tokens {
        return Err("the declarations are not token-equal under the registered renaming".into());
    }
    let mut discharged = vec![format!(
        "token-equal under {} renamed words",
        renaming.len()
    )];
    let weaker_source = format!("module contract;\n{}\n", weaker.text);
    let stronger_source = format!("module contract;\n{}\n", stronger.text);
    // The declared write footprint: every `field_NAME` the contract constrains.
    let footprint: BTreeSet<String> = stronger
        .ensures_normal()
        .flat_map(words)
        .filter_map(|word| word.strip_prefix("field_").map(str::to_owned))
        .collect();
    let family = frame_family(&stronger.name, &footprint);
    for protected in protected_sets {
        let protected: BTreeSet<String> =
            protected.iter().map(|field| (*field).to_owned()).collect();
        if !protected.is_disjoint(&footprint) || !protected.contains("lock") {
            return Err(format!(
                "protected set {protected:?} must contain `lock` and exclude the footprint {footprint:?}"
            ));
        }
        let mut accepted = 0;
        for summary in &family {
            let chunk = evaluate_chunk_frame_contract(summary, &weaker_source).is_ok();
            let generic =
                evaluate_boolean_frame_contract(summary, &stronger_source, record, &protected)
                    .is_ok();
            if chunk != generic {
                return Err(format!(
                    "the frame evaluators disagree on writes {:?} with protection {protected:?}",
                    summary.writes
                ));
            }
            accepted += usize::from(chunk);
        }
        if accepted == 0 {
            return Err("no frame summary is accepted: the parity check is vacuous".into());
        }
        discharged.push(format!(
            "both evaluators agree on {} summaries ({accepted} accepted) with protection {protected:?}",
            family.len()
        ));
    }
    Ok(RelationEvidence {
        function: stronger.name.clone(),
        detailed: stronger.module.clone(),
        view: weaker.module.clone(),
        relation: "renamed_equivalent",
        discharged,
        assumed_premises: Vec::new(),
        dropped_premises: Vec::new(),
    })
}

/// Every write vector of length ≤ 2 over the footprint fields and the frame
/// fields `lock`/`busy`, each with values 0 and 1.
pub(crate) fn frame_family(
    function: &str,
    footprint: &BTreeSet<String>,
) -> Vec<ChunkFrameSummaryRecord> {
    let mut fields: Vec<String> = footprint.iter().cloned().collect();
    fields.extend(["lock".to_owned(), "busy".to_owned()]);
    let mut writes: Vec<Vec<(String, i32)>> = vec![Vec::new()];
    for field in &fields {
        for value in [0, 1] {
            writes.push(vec![(field.clone(), value)]);
            for other in &fields {
                if other != field {
                    for other_value in [0, 1] {
                        writes.push(vec![(field.clone(), value), (other.clone(), other_value)]);
                    }
                }
            }
        }
    }
    writes
        .into_iter()
        .map(|writes| ChunkFrameSummaryRecord {
            function: function.to_owned(),
            source: String::new(),
            layout: Vec::new(),
            writes,
        })
        .collect()
}

/// A C type from a recorded signature, reduced to what a typed declaration
/// can be compared with.
#[derive(Clone, Debug, PartialEq, Eq)]
enum CType {
    Void,
    Scalar(&'static str),
    Pointer { is_const: bool, tag: Option<String> },
}

fn c_type(text: &str) -> Result<CType, String> {
    let text = text.trim();
    if let Some(pointee) = text.strip_suffix('*') {
        let pointee = pointee.trim();
        let is_const = pointee.starts_with("const ");
        let pointee = pointee.trim_start_matches("const ").trim();
        let tag = pointee
            .strip_prefix("struct ")
            .map(|tag| tag.trim().to_owned());
        if tag.is_none() && pointee != "void" {
            return Err(format!("unsupported C pointee `{pointee}`"));
        }
        return Ok(CType::Pointer { is_const, tag });
    }
    Ok(match text {
        "void" => CType::Void,
        "int" => CType::Scalar("i32"),
        "short" => CType::Scalar("i16"),
        "long" => CType::Scalar("i64"),
        "unsigned int" => CType::Scalar("u32"),
        "size_t" => CType::Scalar("usize"),
        // Libevent's Unix spelling: `#define evutil_socket_t int`.
        "evutil_socket_t" => CType::Scalar("i32"),
        other => return Err(format!("unsupported C type `{other}`")),
    })
}

fn c_signature(text: &str) -> Result<(CType, Vec<CType>), String> {
    let open = text
        .find('(')
        .ok_or_else(|| format!("C signature `{text}` has no parameter list"))?;
    let inner = text[open + 1..]
        .strip_suffix(')')
        .ok_or_else(|| format!("C signature `{text}` does not end its parameter list"))?;
    let result = c_type(&text[..open])?;
    let parameters = if inner.trim().is_empty() || inner.trim() == "void" {
        Vec::new()
    } else {
        inner.split(',').map(c_type).collect::<Result<_, _>>()?
    };
    Ok((result, parameters))
}

fn c_agrees(
    c: &CType,
    typed: &str,
    nominals: &[(&str, &str)],
    position: &str,
) -> Result<String, String> {
    let category = abi_category(typed)?;
    match c {
        CType::Void if category == "unit" => Ok(format!("{position}: void = unit")),
        CType::Scalar(scalar) if category == *scalar => Ok(format!("{position}: {scalar}")),
        CType::Pointer { is_const, tag } if category.starts_with("ptr<") => {
            // A C non-const pointer may be borrowed const by a callee that does
            // not write through it; a C const pointer must stay const.
            if *is_const && !category.starts_with("ptr<const") {
                return Err(format!("{position}: C const pointer declared {typed}"));
            }
            if let Some(tag) = tag {
                let expected = nominals
                    .iter()
                    .find(|(c_tag, _)| c_tag == tag)
                    .map(|(_, nominal)| *nominal)
                    .ok_or_else(|| {
                        format!("{position}: no registered nominal for `struct {tag}`")
                    })?;
                if pointer_nominal(typed) != Some(expected) {
                    return Err(format!(
                        "{position}: `struct {tag} *` is {expected}, declared {typed}"
                    ));
                }
            }
            Ok(format!("{position}: pointer {typed}"))
        }
        _ => Err(format!("{position}: C {c:?} disagrees with {typed}")),
    }
}

/// The recorded C signature alternatives of a source-shape declaration:
/// every `signature == "…"` literal in its clauses. A clause naming
/// `signature` in any other shape is refused.
fn signature_alternatives(metadata: &ViewDeclaration) -> Result<Vec<String>, String> {
    let mut alternatives = Vec::new();
    for clause in &metadata.clauses {
        let tokens = lex_tokens(&clause.expression)?;
        if !tokens
            .iter()
            .any(|token| matches!(token, TokenKind::Word(word) if word == "signature"))
        {
            continue;
        }
        let mut index = 0;
        while index < tokens.len() {
            match &tokens[index] {
                TokenKind::Symbol(Symbol::LeftParen | Symbol::RightParen) => index += 1,
                TokenKind::Word(word) if word == "or" => index += 1,
                TokenKind::Word(word) if word == "signature" => {
                    match (tokens.get(index + 1), tokens.get(index + 2)) {
                        (
                            Some(TokenKind::Symbol(Symbol::EqualEqual)),
                            Some(TokenKind::StringLiteral(raw)),
                        ) => {
                            alternatives
                                .push(crate::contract_observation::decode_text_literal(raw)?);
                            index += 3;
                        }
                        _ => return Err("a signature fact must be `signature == \"…\"`".into()),
                    }
                }
                other => {
                    return Err(format!("unsupported token {other:?} in a signature clause"));
                }
            }
        }
    }
    if alternatives.is_empty() {
        return Err(format!("{} records no C signature", metadata.name));
    }
    Ok(alternatives)
}

fn check_source_shape(
    shape: &ViewDeclaration,
    metadata: &ViewDeclaration,
    typed: &ViewDeclaration,
    policy_source: &str,
    projection: Option<&[usize]>,
    nominals: &[(&str, &str)],
) -> Result<RelationEvidence, String> {
    if vocabulary(shape) != Vocabulary::SourceShapeMetadata {
        return Err("the detailed declaration records no source shape".into());
    }
    let mut discharged = Vec::new();
    for alternative in signature_alternatives(shape)? {
        let (result, parameters) = c_signature(&alternative)?;
        let positions: Vec<usize> = match projection {
            None => {
                if parameters.len() != typed.parameters.len() {
                    return Err(format!(
                        "`{alternative}` has {} parameters, the typed declaration {}",
                        parameters.len(),
                        typed.parameters.len()
                    ));
                }
                (0..parameters.len()).collect()
            }
            Some(projection) => {
                if projection.len() != typed.parameters.len()
                    || projection.iter().any(|index| *index >= parameters.len())
                {
                    return Err(format!(
                        "projection {projection:?} does not fit `{alternative}` and {} typed parameters",
                        typed.parameters.len()
                    ));
                }
                projection.to_vec()
            }
        };
        for (typed_index, c_index) in positions.iter().enumerate() {
            c_agrees(
                &parameters[*c_index],
                &typed.parameters[typed_index].type_source,
                nominals,
                &format!("`{alternative}` parameter {c_index}"),
            )?;
        }
        c_agrees(
            &result,
            &typed.result,
            nominals,
            &format!("`{alternative}` result"),
        )?;
        discharged.push(format!("C signature `{alternative}` agrees"));
    }
    // Every recorded declaration fact equals the typed declaration's.
    let join = |values: Vec<String>| values.join(",");
    let clause_sequence = typed
        .clauses
        .iter()
        .map(|clause| match clause.kind.as_str() {
            "requires" | "normal" => Ok(format!("{}:{}", clause.kind, clause.expression)),
            other => Err(format!("clause kind `{other}` has no recorded spelling")),
        })
        .collect::<Result<Vec<_>, _>>()?
        .join("|");
    let facts = [
        ("declaration_is_extern", ContractFact::Integer(1)),
        (
            "declaration_has_body",
            ContractFact::Integer(i128::from(typed.has_body)),
        ),
        (
            "declaration_is_unsafe",
            ContractFact::Integer(i128::from(typed.is_unsafe)),
        ),
        (
            "declaration_has_effects",
            ContractFact::Integer(i128::from(typed.has_effects)),
        ),
        (
            "parameter_names",
            ContractFact::Text(join(
                typed.parameters.iter().map(|p| p.name.clone()).collect(),
            )),
        ),
        (
            "parameter_types",
            ContractFact::Text(join(
                typed
                    .parameters
                    .iter()
                    .map(|p| p.type_source.clone())
                    .collect(),
            )),
        ),
        (
            "parameter_usages",
            ContractFact::Text(join(
                typed
                    .parameters
                    .iter()
                    .map(|p| p.usage.to_owned())
                    .collect(),
            )),
        ),
        ("result_type", ContractFact::Text(typed.result.clone())),
        ("clause_sequence", ContractFact::Text(clause_sequence)),
    ];
    check_typed_contract_facts(policy_source, &metadata.name, &facts)?;
    discharged.push(format!(
        "{}: all {} recorded declaration facts agree",
        metadata.name,
        facts.len()
    ));
    Ok(RelationEvidence {
        function: typed.name.clone(),
        detailed: shape.module.clone(),
        view: typed.module.clone(),
        relation: "source_shape",
        discharged,
        assumed_premises: Vec::new(),
        dropped_premises: Vec::new(),
    })
}

/// The census guard's verdict over a corpus.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RegistryReport {
    pub census: Vec<CensusEntry>,
    pub relations: Vec<RelationEvidence>,
}

/// Check that every multi-module pair is registered, every registration names
/// an existing pair, and every registered relation holds.
pub fn check_registry(
    modules: &BTreeMap<String, String>,
    registry: &[RegisteredRelation],
) -> Result<RegistryReport, Vec<String>> {
    let census = census(modules).map_err(|error| vec![error])?;
    let pairs = census_pairs(&census);
    let mut registered = BTreeSet::new();
    let mut errors = Vec::new();
    for relation in registry {
        let (a, b) = if relation.detailed <= relation.view {
            (relation.detailed, relation.view)
        } else {
            (relation.view, relation.detailed)
        };
        let key = (relation.function.to_owned(), a.to_owned(), b.to_owned());
        if !registered.insert(key.clone()) {
            errors.push(format!("pair registered twice: {key:?}"));
        }
        // A genuine duplicate is removed, not related. It may be kept only as
        // an `Identical` registration with a finding naming why a consumer
        // cannot yet read the single owner.
        let identical = census
            .iter()
            .any(|entry| entry.function == relation.function && entry.token_identical);
        let kept_as_identical = relation.check == RelationCheck::Identical;
        if identical && !kept_as_identical {
            errors.push(format!(
                "`{}` is a textual duplicate in {a} and {b}: remove one declaration",
                relation.function
            ));
        }
        if kept_as_identical && relation.finding.is_none() {
            errors.push(format!(
                "`{}` is kept as an identical duplicate without a registered finding",
                relation.function
            ));
        }
        if !pairs.contains(&key) {
            errors.push(format!(
                "stale registration: {} is not declared in both {a} and {b}",
                relation.function
            ));
        }
    }
    for pair in pairs.difference(&registered) {
        errors.push(format!(
            "unregistered multi-module declaration: `{}` in {} and {}; register a checked \
             relation in qcore_contract::contract_views::REGISTERED_RELATIONS or remove the \
             duplicate",
            pair.0, pair.1, pair.2
        ));
    }
    let mut relations = Vec::new();
    for relation in registry {
        match check_relation(relation, modules) {
            Ok(evidence) => relations.push(evidence),
            Err(error) => errors.push(error),
        }
    }
    if errors.is_empty() {
        Ok(RegistryReport { census, relations })
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fragment_evaluator_is_the_shared_interpreter() {
        // Every predicate in the fragment evaluates exactly as the shared
        // observation interpreter does, over a small grid.
        let predicates = [
            "live_after == (result_null == 0)",
            "size != 0 or result_null == 1",
            "not chunk_was_nonnull or chunk_released",
            "! a == b",
            "a == b == c",
            "a and b or c",
            "-1 != a",
            "true and not false",
        ];
        for predicate in predicates {
            let term = FragmentParser::parse(predicate).unwrap();
            let mut names = BTreeSet::new();
            term.names(&mut names);
            let names: Vec<_> = names.into_iter().collect();
            let domain = [-1i128, 0, 1, 2];
            let mut indices = vec![0usize; names.len()];
            loop {
                let values: BTreeMap<&str, i128> = names
                    .iter()
                    .zip(&indices)
                    .map(|(name, index)| (name.as_str(), domain[*index]))
                    .collect();
                let owned: BTreeMap<String, i128> = values
                    .iter()
                    .map(|(name, value)| ((*name).to_owned(), *value))
                    .collect();
                assert_eq!(
                    term.evaluate(&values).unwrap(),
                    crate::contract_observation::evaluate_predicate(predicate, &owned).unwrap(),
                    "{predicate} at {owned:?}"
                );
                let mut position = 0;
                while position < indices.len() {
                    indices[position] += 1;
                    if indices[position] < domain.len() {
                        break;
                    }
                    indices[position] = 0;
                    position += 1;
                }
                if position == indices.len() {
                    break;
                }
            }
        }
    }

    #[test]
    fn entailment_is_decided_in_both_directions() {
        assert!(entails(&["x == 1"], "x != 0").unwrap().holds);
        assert!(!entails(&["x != 0"], "x == 1").unwrap().holds);
        // A name-name equality needs fresh values: x == y and y != 0 do not
        // force x == 1.
        assert!(!entails(&["x == y", "y != 0"], "x == 1").unwrap().holds);
        assert!(entails(&["x == y", "y == 3"], "x == 3").unwrap().holds);
        // Premises outside the fragment are dropped, soundly and visibly.
        let result = entails(&["x > 0", "x == 2"], "x != 0").unwrap();
        assert!(result.holds);
        assert_eq!(result.dropped_premises, ["x > 0"]);
        assert!(!entails(&["x > 0"], "x != 0").unwrap().holds);
        // A goal outside the fragment is refused, never accepted.
        assert!(entails(&[], "x > 0").is_err());
        assert!(entails(&[], "old(x) == 1").is_err());
    }
}
