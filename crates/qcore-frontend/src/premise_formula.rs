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

//! Card **SP1-SYMBOLIC-OPENSSL-PREMISES** (contract specification C04.18.9,
//! grammar G05; USER DECISION 2026-11-20, verbatim: "we must write down as
//! condition symbols like |-> or other types of logical symbols to represent
//! the prerequisites which compose for premise conditions"): the symbolic
//! statement of a named premise, `requires resource_premise(NAME, FORMULA);`.
//!
//! A formula is read in two steps, both total-or-reject:
//!
//! 1. **Syntax** ([`parse_tokens`]): a bounded precedence reader over the
//!    language's existing tokens into a [`Raw`] tree. Loosest to tightest:
//!    `->` (implication, right-associative), `&&` (conjunction), `|->`
//!    (maps-to, non-associative), `in` and `==` (non-associative), `|` and
//!    `\` (set union and difference: a union chain or one difference, never
//!    mixed without parentheses; `-` is refused, so difference has one
//!    spelling), then a primary: `( F )`,
//!    `quant all #x in SET: F` (its body extends as far as it can), a term
//!    `#name`, `#name(A)` or `#name(A, B)`, an upper-case constant, `{}`.
//! 2. **Sorts** ([`check`]): every term is in the closed [`VOCABULARY`] with
//!    its signature, every constant in [`CLASSES`], every
//!    variable bound by an enclosing `quant` (never shadowing a term or
//!    another variable), and the whole formula is a proposition.
//!
//! The canonical printing ([`canonical`]) is the premise's statement: its
//! identity is NAME plus that text, and every rendering of the premise prints
//! it. A premise stays an undischarged assumption: no consumer that decides a
//! verdict reads it. The Lean model is `QCore.PremiseFormula`; its decision
//! vectors, vocabulary and the four OpenSSL formulas are bound to this module
//! by `crates/qcore-frontend/tests/premise_formula.rs`.
//!
//! **`|->` in this context.** The lexer's [`Symbol::MapsTo`] is one token
//! everywhere (three bytes, matched before every shorter spelling, so `|->`
//! never lexes as a union followed by an implication). Its meaning is decided
//! by the delimited context that reads it: a contract state clause, a
//! premise-flow expression (C04.18.3), or this formula, which is exactly the
//! tokens between the comma after `NAME` and the clause's closing parenthesis.
//! Here `F |-> E` says every run of the callback `F` (or of each member of the
//! callback set `F`) during the call has the effect `E`.

use crate::lexer::{Symbol, TokenKind, lex};

/// At most this many tokens in one formula.
pub const MAX_TOKENS: usize = 1024;
/// At most this many nested primaries and operators.
pub const MAX_DEPTH: usize = 48;
/// At most this many tree nodes.
pub const MAX_NODES: usize = 512;

/// A sort of the formula language.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Sort {
    /// An object the call touches.
    Obj,
    /// An object class.
    Cls,
    /// A callback: a function at a position the library calls it from.
    Fn,
    /// A BIO method table.
    Meth,
    /// A resource the caller holds.
    Res,
    SetObj,
    SetFn,
    SetMeth,
    SetRes,
    /// A proposition.
    Prop,
    /// A property of one run of a callback (the right of `|->`).
    Eff,
}

impl Sort {
    /// The Lean constructor (`QCore.PremiseFormula.Srt`).
    #[must_use]
    pub fn lean(self) -> &'static str {
        match self {
            Self::Obj => ".obj",
            Self::Cls => ".cls",
            Self::Fn => ".fn",
            Self::Meth => ".meth",
            Self::Res => ".res",
            Self::SetObj => ".sobj",
            Self::SetFn => ".sfn",
            Self::SetMeth => ".smeth",
            Self::SetRes => ".sres",
            Self::Prop => ".prop",
            Self::Eff => ".eff",
        }
    }

    /// The set of this element sort.
    fn set(self) -> Option<Self> {
        match self {
            Self::Obj => Some(Self::SetObj),
            Self::Fn => Some(Self::SetFn),
            Self::Meth => Some(Self::SetMeth),
            _ => None,
        }
    }
}

/// A binary operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    /// `->`
    Imp,
    /// `&&`
    Conj,
    /// `|->`
    MapsTo,
    /// `in`
    Mem,
    /// `==`
    Eqv,
    /// `|`
    Union,
    /// `\`
    Diff,
}

impl Op {
    fn spelling(self) -> &'static str {
        match self {
            Self::Imp => "->",
            Self::Conj => "&&",
            Self::MapsTo => "|->",
            Self::Mem => "in",
            Self::Eqv => "==",
            Self::Union => "|",
            Self::Diff => "\\",
        }
    }

    fn lean(self) -> &'static str {
        match self {
            Self::Imp => ".imp",
            Self::Conj => ".conj",
            Self::MapsTo => ".mapsTo",
            Self::Mem => ".mem",
            Self::Eqv => ".eqv",
            Self::Union => ".union",
            Self::Diff => ".diff",
        }
    }

    /// Binding strength, loosest first.
    fn level(self) -> u8 {
        match self {
            Self::Imp => 1,
            Self::Conj => 2,
            Self::MapsTo => 3,
            Self::Mem | Self::Eqv => 4,
            Self::Union | Self::Diff => 5,
        }
    }
}

/// A formula as read, before its sorts are checked. Term and variable names
/// are stored without their `#`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Raw {
    /// `quant all #VAR in DOMAIN: BODY`
    Quant {
        var: String,
        domain: Box<Raw>,
        body: Box<Raw>,
    },
    /// `LEFT OP RIGHT`
    Bin(Op, Box<Raw>, Box<Raw>),
    /// `#name`: a nullary term or a bound variable.
    App0(String),
    /// `#name(A)`
    App1(String, Box<Raw>),
    /// `#name(A, B)`
    App2(String, Box<Raw>, Box<Raw>),
    /// An upper-case constant (a class: `SSL`, `BIO`).
    Const(String),
    /// `{}`
    Empty,
}

/// One registered term: `#NAME(ARGS…)` of sort `RESULT`, with its meaning
/// (contract specification C04.18.9, where each is defined).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Term {
    pub name: &'static str,
    pub arguments: &'static [Sort],
    pub result: Sort,
    pub meaning: &'static str,
}

/// The closed vocabulary. Every term a formula names is here; an unknown
/// term rejects the module.
pub const VOCABULARY: &[Term] = &[
    Term {
        name: "args",
        arguments: &[],
        result: Sort::SetObj,
        meaning: "the objects the call's pointer arguments designate",
    },
    Term {
        name: "created",
        arguments: &[],
        result: Sort::SetObj,
        meaning: "the objects the call allocates, directly or as a temporary object inside the call",
    },
    Term {
        name: "copied",
        arguments: &[],
        result: Sort::SetObj,
        meaning: "the objects the call copies (the source of a dup)",
    },
    Term {
        name: "freed",
        arguments: &[],
        result: Sort::SetObj,
        meaning: "the objects whose last reference the call drops: directly, in turn (an object a freed object owns), or as a replaced or temporary object it frees inside the call",
    },
    Term {
        name: "class",
        arguments: &[Sort::Obj],
        result: Sort::Cls,
        meaning: "the object's class",
    },
    Term {
        name: "of",
        arguments: &[Sort::Cls, Sort::SetObj],
        result: Sort::SetObj,
        meaning: "the members of the set whose class is the class",
    },
    Term {
        name: "ctx",
        arguments: &[Sort::SetObj],
        result: Sort::SetObj,
        meaning: "the SSL_CTX of each SSL object of the set",
    },
    Term {
        name: "bios",
        arguments: &[Sort::SetObj],
        result: Sort::SetObj,
        meaning: "the BIOs each SSL object of the set holds (its read and write BIO)",
    },
    Term {
        name: "chain",
        arguments: &[Sort::SetObj],
        result: Sort::SetObj,
        meaning: "the BIOs of the set and every BIO chained after one of them, transitively",
    },
    Term {
        name: "ssl_of",
        arguments: &[Sort::Obj],
        result: Sort::SetObj,
        meaning: "the SSL object a BIO_f_ssl BIO holds (no object for any other object)",
    },
    Term {
        name: "slots",
        arguments: &[Sort::SetObj],
        result: Sort::SetFn,
        meaning: "every callback slot of an object of the set: each function pointer member the library calls, including the arms SSL_CTX_callback_ctrl writes (the servername, status and ticket key callbacks among them)",
    },
    Term {
        name: "stated",
        arguments: &[],
        result: Sort::SetFn,
        meaning: "the callbacks a clause of the declaration states the call runs",
    },
    Term {
        name: "ex_data",
        arguments: &[Sort::Cls],
        result: Sort::SetFn,
        meaning: "the new, dup and free callbacks CRYPTO_get_ex_new_index registers for the class",
    },
    Term {
        name: "ex_data_free",
        arguments: &[Sort::Cls],
        result: Sort::SetFn,
        meaning: "the free callbacks CRYPTO_get_ex_new_index registers for the class",
    },
    Term {
        name: "method",
        arguments: &[Sort::Obj],
        result: Sort::Meth,
        meaning: "the method table of a BIO (its BIO_METHOD) or of an X509_LOOKUP (its X509_LOOKUP_METHOD)",
    },
    Term {
        name: "builtin",
        arguments: &[],
        result: Sort::SetMeth,
        meaning: "the method tables the library defines itself, which no setter is handed: the BIO_METHOD BIO_s_socket, BIO_s_mem, BIO_f_buffer or another built-in BIO_s_ or BIO_f_ function returns, and the X509_LOOKUP_METHOD X509_LOOKUP_file, X509_LOOKUP_hash_dir or X509_LOOKUP_store returns",
    },
    Term {
        name: "fns",
        arguments: &[Sort::Obj],
        result: Sort::SetFn,
        meaning: "the functions of the object's method table (a BIO's BIO_METHOD, an X509_LOOKUP's X509_LOOKUP_METHOD), each as run on that object",
    },
    Term {
        name: "store",
        arguments: &[Sort::SetObj],
        result: Sort::SetObj,
        meaning: "the certificate store (X509_STORE) each SSL_CTX of the set holds",
    },
    Term {
        name: "lookups",
        arguments: &[Sort::SetObj],
        result: Sort::SetObj,
        meaning: "the lookups (X509_LOOKUP) each X509_STORE of the set holds",
    },
    Term {
        name: "teardown",
        arguments: &[Sort::Obj],
        result: Sort::SetFn,
        meaning: "the callbacks the free of the object runs from its method tables, its provider and its engine, and the callbacks the object holds that its free runs",
    },
    Term {
        name: "settable",
        arguments: &[],
        result: Sort::SetFn,
        meaning: "the callbacks a setter some declaration of the declaration's crate states can write",
    },
    // Card OPENSSL-FREE-REMAINING-INDIRECT-CALLBACKS (C04.18.9 amended).
    Term {
        name: "library",
        arguments: &[],
        result: Sort::SetFn,
        meaning: "the functions the library defines itself and installs without a setter call of the run: those of the method tables it defines (its default RSA, DSA, DH and EC_KEY methods, its EVP_PKEY_ASN1_METHOD, EVP_PKEY_METHOD, EVP_MD and EVP_CIPHER tables, the EC_METHOD of a group), of the engines it builds in, and the dispatch functions of the default, base, legacy and null providers",
    },
    Term {
        name: "loaded",
        arguments: &[],
        result: Sort::SetFn,
        meaning: "the functions of a module the library loads from outside the run: a provider other than the default, base, legacy and null providers (the FIPS provider and any provider a configuration or OSSL_PROVIDER_load loads among them), and an engine the dynamic engine loads from a shared object",
    },
    Term {
        name: "in_turn",
        arguments: &[Sort::Fn, Sort::SetObj],
        result: Sort::SetFn,
        meaning: "the callbacks a run of the function runs in turn, directly or through the calls it makes, on an object of the set",
    },
    Term {
        name: "caller_held",
        arguments: &[],
        result: Sort::SetRes,
        meaning: "the resources the caller holds when the run starts",
    },
    Term {
        name: "takes",
        arguments: &[Sort::SetRes],
        result: Sort::SetRes,
        meaning: "the members of the set the run takes over",
    },
    Term {
        name: "releases",
        arguments: &[Sort::SetRes],
        result: Sort::SetRes,
        meaning: "the members of the set the run releases",
    },
    Term {
        name: "retains",
        arguments: &[Sort::SetRes],
        result: Sort::SetRes,
        meaning: "the members of the set the run keeps beyond its return",
    },
    Term {
        name: "inert",
        arguments: &[],
        result: Sort::Eff,
        meaning: INERT_DEFINITION,
    },
];

/// The one defined term: `#inert` abbreviates this effect (C04.18.9).
pub const INERT_DEFINITION: &str = "#takes(#caller_held) == {} && #releases(#caller_held) == {} \
&& #retains(#caller_held) == {}";

/// The class constants: `CRYPTO_get_ex_new_index`'s (3.5) ex_data classes.
pub const CLASSES: &[&str] = &[
    "BIO",
    "DH",
    "DSA",
    "EC_KEY",
    "ENGINE",
    "EVP_PKEY",
    "RSA",
    "SSL",
    "SSL_CTX",
    "SSL_SESSION",
    "UI",
    "UI_METHOD",
    "X509",
    "X509_STORE",
    "X509_STORE_CTX",
];

fn error(message: &str) -> String {
    format!("premise-formula: {message}")
}

fn lowercase_identifier(word: &str) -> bool {
    word.len() <= 64
        && word.bytes().next().is_some_and(|b| b.is_ascii_lowercase())
        && word
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

fn constant_word(word: &str) -> bool {
    word.len() <= 64
        && word.bytes().next().is_some_and(|b| b.is_ascii_uppercase())
        && word
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
}

struct Reader<'a> {
    tokens: &'a [TokenKind],
    at: usize,
    nodes: usize,
    depth: usize,
}

impl Reader<'_> {
    fn peek(&self) -> Option<&TokenKind> {
        self.tokens.get(self.at)
    }

    fn symbol(&self, symbol: Symbol) -> bool {
        self.peek() == Some(&TokenKind::Symbol(symbol))
    }

    fn keyword(&self, word: &str) -> bool {
        matches!(self.peek(), Some(TokenKind::Word(w)) if w == word)
    }

    fn take(&mut self, symbol: Symbol) -> bool {
        let found = self.symbol(symbol);
        if found {
            self.at += 1;
        }
        found
    }

    fn need(&mut self, symbol: Symbol, what: &str) -> Result<(), String> {
        if self.take(symbol) {
            Ok(())
        } else {
            Err(error(&format!("expected {what}")))
        }
    }

    fn need_keyword(&mut self, word: &str) -> Result<(), String> {
        if self.keyword(word) {
            self.at += 1;
            Ok(())
        } else {
            Err(error(&format!("expected `{word}`")))
        }
    }

    fn name(&mut self) -> Result<String, String> {
        match self.peek() {
            Some(TokenKind::Word(word)) if lowercase_identifier(word) => {
                let word = word.clone();
                self.at += 1;
                Ok(word)
            }
            _ => Err(error("`#` is followed by a lower-case name")),
        }
    }

    fn node(&mut self, raw: Raw) -> Result<Raw, String> {
        self.nodes += 1;
        if self.nodes > MAX_NODES {
            return Err(error("too many nodes"));
        }
        Ok(raw)
    }

    fn enter(&mut self) -> Result<(), String> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return Err(error("nested too deeply"));
        }
        Ok(())
    }

    fn bin(&mut self, op: Op, left: Raw, right: Raw) -> Result<Raw, String> {
        self.node(Raw::Bin(op, Box::new(left), Box::new(right)))
    }

    fn implication(&mut self) -> Result<Raw, String> {
        self.enter()?;
        let left = self.conjunction()?;
        let raw = if self.take(Symbol::Arrow) {
            let right = self.implication()?;
            self.bin(Op::Imp, left, right)?
        } else {
            left
        };
        self.depth -= 1;
        Ok(raw)
    }

    fn conjunction(&mut self) -> Result<Raw, String> {
        let mut left = self.maps_to()?;
        while self.take(Symbol::AndAnd) {
            let right = self.maps_to()?;
            left = self.bin(Op::Conj, left, right)?;
        }
        Ok(left)
    }

    fn maps_to(&mut self) -> Result<Raw, String> {
        let left = self.relation()?;
        if !self.take(Symbol::MapsTo) {
            return Ok(left);
        }
        let right = self.relation()?;
        if self.symbol(Symbol::MapsTo) {
            return Err(error("`|->` does not chain: parenthesize"));
        }
        self.bin(Op::MapsTo, left, right)
    }

    fn relation(&mut self) -> Result<Raw, String> {
        let left = self.set()?;
        let op = if self.keyword("in") {
            Op::Mem
        } else if self.symbol(Symbol::EqualEqual) {
            Op::Eqv
        } else {
            return Ok(left);
        };
        self.at += 1;
        let right = self.set()?;
        if self.keyword("in") || self.symbol(Symbol::EqualEqual) {
            return Err(error("`in` and `==` do not chain: parenthesize"));
        }
        self.bin(op, left, right)
    }

    fn set(&mut self) -> Result<Raw, String> {
        let mut left = self.primary()?;
        if self.symbol(Symbol::Pipe) {
            while self.take(Symbol::Pipe) {
                let right = self.primary()?;
                left = self.bin(Op::Union, left, right)?;
            }
            if self.symbol(Symbol::Backslash) {
                return Err(error("`|` and `\\` do not mix: parenthesize"));
            }
        } else if self.take(Symbol::Backslash) {
            let right = self.primary()?;
            left = self.bin(Op::Diff, left, right)?;
            if self.symbol(Symbol::Pipe) || self.symbol(Symbol::Backslash) {
                return Err(error("`\\` does not chain or mix with `|`: parenthesize"));
            }
        }
        if self.symbol(Symbol::Minus) {
            return Err(error("set difference is spelled `\\`, not `-`"));
        }
        Ok(left)
    }

    fn primary(&mut self) -> Result<Raw, String> {
        self.enter()?;
        let raw = match self.peek().cloned() {
            Some(TokenKind::Symbol(Symbol::LeftParen)) => {
                self.at += 1;
                let inner = self.implication()?;
                self.need(Symbol::RightParen, "`)`")?;
                inner
            }
            Some(TokenKind::Symbol(Symbol::LeftBrace)) => {
                self.at += 1;
                self.need(Symbol::RightBrace, "`}` (the only set literal is `{}`)")?;
                self.node(Raw::Empty)?
            }
            Some(TokenKind::Word(word)) if word == "quant" => {
                self.at += 1;
                self.need_keyword("all")?;
                self.need(Symbol::Hash, "`#` before the bound variable")?;
                let var = self.name()?;
                self.need_keyword("in")?;
                let domain = self.set()?;
                self.need(Symbol::Colon, "`:` after the domain")?;
                let body = self.implication()?;
                self.node(Raw::Quant {
                    var,
                    domain: Box::new(domain),
                    body: Box::new(body),
                })?
            }
            Some(TokenKind::Word(word)) if constant_word(&word) => {
                self.at += 1;
                self.node(Raw::Const(word))?
            }
            Some(TokenKind::Symbol(Symbol::Hash)) => {
                self.at += 1;
                let name = self.name()?;
                if self.take(Symbol::LeftParen) {
                    let first = self.implication()?;
                    let raw = if self.take(Symbol::Comma) {
                        let second = self.implication()?;
                        if self.symbol(Symbol::Comma) {
                            return Err(error("no term takes more than two arguments"));
                        }
                        Raw::App2(name, Box::new(first), Box::new(second))
                    } else {
                        Raw::App1(name, Box::new(first))
                    };
                    self.need(Symbol::RightParen, "`)` after the arguments")?;
                    self.node(raw)?
                } else {
                    self.node(Raw::App0(name))?
                }
            }
            _ => return Err(error("expected a term, `quant`, `(` or `{}`")),
        };
        self.depth -= 1;
        Ok(raw)
    }
}

/// The syntax step: the whole of `tokens` (no end-of-file token) as one
/// formula, or a rejection.
pub fn parse_tokens(tokens: &[TokenKind]) -> Result<Raw, String> {
    if tokens.is_empty() {
        return Err(error("empty formula"));
    }
    if tokens.len() > MAX_TOKENS {
        return Err(error("too many tokens"));
    }
    let mut reader = Reader {
        tokens,
        at: 0,
        nodes: 0,
        depth: 0,
    };
    let raw = reader.implication()?;
    if reader.at != tokens.len() {
        return Err(error("unexpected token after the formula"));
    }
    Ok(raw)
}

/// The registered term `name`.
#[must_use]
pub fn term(name: &str) -> Option<&'static Term> {
    VOCABULARY.iter().find(|term| term.name == name)
}

fn sort_of(context: &[(String, Sort)], raw: &Raw) -> Result<Sort, String> {
    let ill = |what: &str| Err(error(&format!("ill-sorted {what}")));
    match raw {
        Raw::Quant { var, domain, body } => {
            if term(var).is_some() || context.iter().any(|(bound, _)| bound == var) {
                return Err(error(&format!(
                    "`#{var}` shadows a term or a bound variable"
                )));
            }
            let element = match sort_of(context, domain)? {
                Sort::SetObj => Sort::Obj,
                Sort::SetFn => Sort::Fn,
                Sort::SetMeth => Sort::Meth,
                _ => return ill("quantifier domain"),
            };
            let mut inner = context.to_vec();
            inner.push((var.clone(), element));
            match sort_of(&inner, body)? {
                Sort::Prop => Ok(Sort::Prop),
                _ => ill("quantifier body"),
            }
        }
        Raw::Bin(op, left, right) => {
            let (l, r) = (sort_of(context, left)?, sort_of(context, right)?);
            match (op, l, r) {
                (Op::Imp, Sort::Prop, Sort::Prop) => Ok(Sort::Prop),
                (Op::Conj, Sort::Prop, Sort::Prop) => Ok(Sort::Prop),
                (Op::Conj, Sort::Eff, Sort::Eff) => Ok(Sort::Eff),
                (Op::MapsTo, Sort::Fn | Sort::SetFn, Sort::Eff) => Ok(Sort::Prop),
                (Op::Mem, element, set) if element.set() == Some(set) => Ok(Sort::Prop),
                (Op::Eqv, Sort::SetRes, Sort::SetRes) => Ok(Sort::Eff),
                (Op::Union | Op::Diff, Sort::SetObj, Sort::SetObj) => Ok(Sort::SetObj),
                (Op::Union | Op::Diff, Sort::SetFn, Sort::SetFn) => Ok(Sort::SetFn),
                (Op::Union | Op::Diff, Sort::SetMeth, Sort::SetMeth) => Ok(Sort::SetMeth),
                _ => ill(&format!("`{}`", op.spelling())),
            }
        }
        Raw::App0(name) => {
            if let Some((_, sort)) = context.iter().rev().find(|(bound, _)| bound == name) {
                return Ok(*sort);
            }
            match term(name) {
                Some(term) if term.arguments.is_empty() => Ok(term.result),
                Some(term) => Err(error(&format!(
                    "`#{name}` takes {} argument(s)",
                    term.arguments.len()
                ))),
                None => Err(error(&format!(
                    "unknown term or unbound variable `#{name}`"
                ))),
            }
        }
        Raw::App1(name, argument) => {
            let Some(term) = term(name) else {
                return Err(error(&format!("unknown term `#{name}`")));
            };
            let argument = sort_of(context, argument)?;
            if term.arguments == [argument] {
                Ok(term.result)
            } else {
                ill(&format!("application of `#{name}`"))
            }
        }
        Raw::App2(name, first, second) => {
            let Some(term) = term(name) else {
                return Err(error(&format!("unknown term `#{name}`")));
            };
            let (first, second) = (sort_of(context, first)?, sort_of(context, second)?);
            if term.arguments == [first, second] {
                Ok(term.result)
            } else {
                ill(&format!("application of `#{name}`"))
            }
        }
        Raw::Const(word) => {
            if CLASSES.contains(&word.as_str()) {
                Ok(Sort::Cls)
            } else {
                Err(error(&format!("unknown constant `{word}`")))
            }
        }
        Raw::Empty => Ok(Sort::SetRes),
    }
}

/// The sort of a closed formula, or the rejection (`QCore.PremiseFormula.check`).
pub fn sort(raw: &Raw) -> Result<Sort, String> {
    sort_of(&[], raw)
}

/// The well-formedness judgment: a closed, well-sorted proposition
/// (`QCore.PremiseFormula.wf`).
pub fn check(raw: &Raw) -> Result<(), String> {
    match sort(raw)? {
        Sort::Prop => Ok(()),
        _ => Err(error("a premise states a proposition")),
    }
}

/// A well-formed premise formula and its canonical printing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PremiseFormula {
    pub raw: Raw,
    pub canonical: String,
}

/// Syntax, then sorts: the formula `tokens` spell, or the rejection.
pub fn read(tokens: &[TokenKind]) -> Result<PremiseFormula, String> {
    let raw = parse_tokens(tokens)?;
    check(&raw)?;
    let canonical = canonical(&raw);
    Ok(PremiseFormula { raw, canonical })
}

/// [`read`] over source text.
pub fn read_source(source: &str) -> Result<PremiseFormula, String> {
    let tokens = lex(source)
        .map_err(|e| error(&e.to_string()))?
        .into_iter()
        .map(|token| token.kind)
        .filter(|kind| *kind != TokenKind::Eof)
        .collect::<Vec<_>>();
    read(&tokens)
}

fn level(raw: &Raw) -> u8 {
    match raw {
        Raw::Quant { .. } => 0,
        Raw::Bin(op, ..) => op.level(),
        _ => 6,
    }
}

/// `open`: nothing follows this node inside its parentheses, so a
/// quantifier may stand bare there (its body extends to the end).
fn print(raw: &Raw, open: bool, out: &mut String) {
    match raw {
        Raw::Quant { var, domain, body } => {
            if !open {
                out.push('(');
            }
            out.push_str("quant all #");
            out.push_str(var);
            out.push_str(" in ");
            operand(domain, level(domain) < 5, false, out);
            out.push_str(": ");
            print(body, true, out);
            if !open {
                out.push(')');
            }
        }
        Raw::Bin(op, left, right) => {
            let (l, r) = (level(left), level(right));
            let (left_parens, right_parens) = match op {
                // Right-associative.
                Op::Imp => (l <= 1, r < 1 && !open),
                // Left-associative.
                Op::Conj => (l < 2, r <= 2 && !(r == 0 && open)),
                Op::MapsTo => (l <= 3, r <= 3),
                Op::Mem | Op::Eqv => (l <= 4, r <= 4),
                Op::Union => (l < 5 || matches!(**left, Raw::Bin(Op::Diff, ..)), r <= 5),
                Op::Diff => (l <= 5, r <= 5),
            };
            operand(left, left_parens, false, out);
            out.push(' ');
            out.push_str(op.spelling());
            out.push(' ');
            operand(right, right_parens, open, out);
        }
        Raw::App0(name) => {
            out.push('#');
            out.push_str(name);
        }
        Raw::App1(name, argument) => {
            out.push('#');
            out.push_str(name);
            out.push('(');
            print(argument, true, out);
            out.push(')');
        }
        Raw::App2(name, first, second) => {
            out.push('#');
            out.push_str(name);
            out.push('(');
            print(first, true, out);
            out.push_str(", ");
            print(second, true, out);
            out.push(')');
        }
        Raw::Const(word) => out.push_str(word),
        Raw::Empty => out.push_str("{}"),
    }
}

fn operand(raw: &Raw, parens: bool, open: bool, out: &mut String) {
    if parens {
        out.push('(');
        print(raw, true, out);
        out.push(')');
    } else {
        print(raw, open, out);
    }
}

/// The canonical printing: single spaces around every binary operator, `: `
/// after a quantifier's domain, `, ` between arguments, and parentheses only
/// where the precedence needs them. [`parse_tokens`] of the printing is the
/// same tree.
#[must_use]
pub fn canonical(raw: &Raw) -> String {
    let mut out = String::new();
    print(raw, true, &mut out);
    out
}

/// The tree in the Lean model's syntax (`QCore.PremiseFormula.Raw`).
#[must_use]
pub fn lean(raw: &Raw) -> String {
    match raw {
        Raw::Quant { var, domain, body } => {
            format!("(.quant \"{var}\" {} {})", lean(domain), lean(body))
        }
        Raw::Bin(op, left, right) => format!("(.bin {} {} {})", op.lean(), lean(left), lean(right)),
        Raw::App0(name) => format!("(.app0 \"{name}\")"),
        Raw::App1(name, argument) => format!("(.app1 \"{name}\" {})", lean(argument)),
        Raw::App2(name, first, second) => {
            format!("(.app2 \"{name}\" {} {})", lean(first), lean(second))
        }
        Raw::Const(word) => format!("(.const \"{word}\")"),
        Raw::Empty => ".empty".to_owned(),
    }
}

/// What a named premise states.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PremiseStatement {
    /// `resource_premise(NAME, "STATEMENT")`.
    Prose(String),
    /// `resource_premise(NAME, FORMULA)`.
    Formula(PremiseFormula),
}

impl PremiseStatement {
    /// The statement as every rendering prints it: the prose, or the
    /// formula's canonical printing.
    #[must_use]
    pub fn text(&self) -> &str {
        match self {
            Self::Prose(text) => text,
            Self::Formula(formula) => &formula.canonical,
        }
    }
}

/// The introducer of a named premise.
pub const PREMISE_HEAD: &str = "resource_premise";

/// A named premise clause, read from its expression's tokens (no `requires`,
/// no `;`, no end-of-file token). `Ok(None)` when the clause is not headed
/// `resource_premise(`; a clause so headed is `resource_premise(NAME,
/// "STATEMENT")` with a plain, non-blank string that is not itself a formula,
/// or `resource_premise(NAME, FORMULA)`, `NAME` a lower-case identifier, and
/// anything else rejects.
pub fn premise_clause(tokens: &[TokenKind]) -> Result<Option<(String, PremiseStatement)>, String> {
    if !matches!(tokens.first(), Some(TokenKind::Word(head)) if head == PREMISE_HEAD) {
        return Ok(None);
    }
    let malformed = || {
        "malformed premise: write `requires resource_premise(NAME, \"STATEMENT\");` or \
         `requires resource_premise(NAME, FORMULA);` with a lowercase identifier"
            .to_owned()
    };
    let [
        _,
        TokenKind::Symbol(Symbol::LeftParen),
        TokenKind::Word(name),
        TokenKind::Symbol(Symbol::Comma),
        statement @ ..,
        TokenKind::Symbol(Symbol::RightParen),
    ] = tokens
    else {
        return Err(malformed());
    };
    let identifier = name.bytes().next().is_some_and(|b| b.is_ascii_lowercase())
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_');
    if !identifier {
        return Err(malformed());
    }
    if let [TokenKind::StringLiteral(text)] = statement {
        if text.trim().is_empty() || text.contains('\\') || text.chars().any(char::is_control) {
            return Err(malformed());
        }
        // A quoted formula would collide with the formula form's identity.
        if read_source(text).is_ok() {
            return Err(format!(
                "premise `{name}`: its statement is a formula, so write it unquoted"
            ));
        }
        return Ok(Some((name.clone(), PremiseStatement::Prose(text.clone()))));
    }
    let formula = read(statement).map_err(|e| format!("premise `{name}`: {e}"))?;
    Ok(Some((name.clone(), PremiseStatement::Formula(formula))))
}

/// The decision vectors shared with `proof/QCore/PremiseFormula.lean`
/// (`decisionVectors`): a formula [`parse_tokens`] reads, and whether the
/// judgment [`check`] accepts it. The test binds each row's tree to the Lean
/// table and its decision to this reader.
pub const DECISION_VECTORS: &[(&str, bool)] = &[
    (
        "quant all #o in #created | #copied | #freed: #ex_data(#class(#o)) |-> #inert",
        true,
    ),
    (
        "#stated |-> (#takes(#caller_held) == {} && #releases(#caller_held) == {} && #retains(#caller_held) == {})",
        true,
    ),
    ("quant all #f in #stated: #f |-> #inert", true),
    (
        "quant all #b in #of(BIO, #args): #b in #freed -> #ex_data_free(#class(#b)) |-> #inert",
        true,
    ),
    (
        "quant all #b in #args: #method(#b) in #builtin -> #fns(#b) |-> #inert",
        true,
    ),
    (
        "#slots(#args) \\ #stated |-> #inert && #slots(#ctx(#args)) |-> #inert",
        true,
    ),
    // An unknown term.
    ("#unknown |-> #inert", false),
    // An ill-sorted application: `#class` takes an object, not a set.
    ("#ex_data(#class(#args)) |-> #inert", false),
    // An unbound variable.
    (
        "quant all #o in #created: #ex_data(#class(#x)) |-> #inert",
        false,
    ),
    // A variable shadowing a term.
    ("quant all #args in #created: #stated |-> #inert", false),
    // A variable shadowing a variable.
    (
        "quant all #o in #created: quant all #o in #freed: #stated |-> #inert",
        false,
    ),
    // Not a proposition: a callback set, an effect.
    ("#stated", false),
    ("#inert", false),
    // A union of two sorts.
    ("#created | #stated |-> #inert", false),
    // Membership of a class in a set of objects.
    ("SSL in #args", false),
    // A set of objects is not a callback set.
    ("#freed |-> #inert", false),
    // An unknown constant.
    (
        "quant all #b in #of(NOT_A_CLASS, #args): #fns(#b) |-> #inert",
        false,
    ),
    // Arguments in the wrong order.
    (
        "quant all #b in #of(#args, BIO): #fns(#b) |-> #inert",
        false,
    ),
    // A term used without its arguments.
    ("#slots |-> #inert", false),
    // A quantifier body that is not a proposition.
    ("quant all #o in #created: #o", false),
    // The empty set is a resource set only.
    ("{} |-> #inert", false),
    // An effect where a proposition stands.
    ("#stated |-> #inert && #inert", false),
];
