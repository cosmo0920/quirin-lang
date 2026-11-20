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

//! Card **SP1-SYMBOLIC-OPENSSL-PREMISES** (contract specification C04.18.9):
//! the symbolic premise statement, its reader's fail-closed arms, and the
//! binding of its decision vectors, vocabulary and the four OpenSSL formulas
//! to the Lean model `QCore.PremiseFormula`. Card
//! **SP4-SYMBOLIC-LIBEVENT-PREMISES**: the SP4 vocabulary block and decision
//! vectors, and the seven libevent formulas bound to
//! `QCore.PremiseFormula.Libevent`.

use std::collections::BTreeMap;

use qcore_frontend::lexer::{TokenKind, lex};
use qcore_frontend::premise_formula::{
    CLASSES, DECISION_VECTORS, INERT_DEFINITION, MAX_DEPTH, MAX_TOKENS, PremiseStatement, Raw,
    Sort, VOCABULARY, canonical, check, lean, parse_tokens, premise_clause, read_source, sort,
};

const LEAN: &str = include_str!("../../../proof/QCore/PremiseFormula.lean");

/// Card SP4-SYMBOLIC-LIBEVENT-PREMISES: the Lean definitions of the libevent
/// formulas.
const LEAN_LIBEVENT: &str = include_str!("../../../proof/QCore/PremiseFormula/Libevent.lean");

/// The one module of `qcore-provider-libevent` that states a premise.
const LIBEVENT_MODULES: &[(&str, &str)] = &[(
    "monkey/intrusive_list.qv",
    include_str!("../../../providers/qcore-provider-libevent/contracts/monkey/intrusive_list.qv"),
)];

/// Every module of the OpenSSL user crate (`user-crates-load-path.json`'s
/// 17), as shipped.
const OPENSSL_MODULES: &[(&str, &str)] = &[
    (
        "bio.qv",
        include_str!("../../../providers/quirin-provider-openssl/contracts/openssl/bio.qv"),
    ),
    (
        "bio_slots.qv",
        include_str!("../../../providers/quirin-provider-openssl/contracts/openssl/bio_slots.qv"),
    ),
    (
        "engine.qv",
        include_str!("../../../providers/quirin-provider-openssl/contracts/openssl/engine.qv"),
    ),
    (
        "err.qv",
        include_str!("../../../providers/quirin-provider-openssl/contracts/openssl/err.qv"),
    ),
    (
        "evp.qv",
        include_str!("../../../providers/quirin-provider-openssl/contracts/openssl/evp.qv"),
    ),
    (
        "evp_digest.qv",
        include_str!("../../../providers/quirin-provider-openssl/contracts/openssl/evp_digest.qv"),
    ),
    (
        "init.qv",
        include_str!("../../../providers/quirin-provider-openssl/contracts/openssl/init.qv"),
    ),
    (
        "lookup.qv",
        include_str!("../../../providers/quirin-provider-openssl/contracts/openssl/lookup.qv"),
    ),
    (
        "pem.qv",
        include_str!("../../../providers/quirin-provider-openssl/contracts/openssl/pem.qv"),
    ),
    (
        "pkcs12.qv",
        include_str!("../../../providers/quirin-provider-openssl/contracts/openssl/pkcs12.qv"),
    ),
    (
        "ssl.qv",
        include_str!("../../../providers/quirin-provider-openssl/contracts/openssl/ssl.qv"),
    ),
    (
        "ssl_ctx.qv",
        include_str!("../../../providers/quirin-provider-openssl/contracts/openssl/ssl_ctx.qv"),
    ),
    (
        "ssl_handshake.qv",
        include_str!(
            "../../../providers/quirin-provider-openssl/contracts/openssl/ssl_handshake.qv"
        ),
    ),
    (
        "ssl_io.qv",
        include_str!("../../../providers/quirin-provider-openssl/contracts/openssl/ssl_io.qv"),
    ),
    (
        "stack.qv",
        include_str!("../../../providers/quirin-provider-openssl/contracts/openssl/stack.qv"),
    ),
    (
        "x509.qv",
        include_str!("../../../providers/quirin-provider-openssl/contracts/openssl/x509.qv"),
    ),
    (
        "x509_store.qv",
        include_str!("../../../providers/quirin-provider-openssl/contracts/openssl/x509_store.qv"),
    ),
];

fn tokens(source: &str) -> Vec<TokenKind> {
    lex(source)
        .unwrap()
        .into_iter()
        .map(|token| token.kind)
        .filter(|kind| *kind != TokenKind::Eof)
        .collect()
}

fn raw(source: &str) -> Raw {
    parse_tokens(&tokens(source)).unwrap_or_else(|e| panic!("{source}: {e}"))
}

/// The rows of a Lean list definition, one per line, between its opening
/// line and the closing `]`.
fn lean_rows(opening: &str) -> Vec<String> {
    let start = LEAN
        .find(opening)
        .unwrap_or_else(|| panic!("no `{opening}`"));
    LEAN[start + opening.len()..]
        .lines()
        .map(str::trim)
        .skip_while(|line| line.is_empty())
        .take_while(|line| *line != "]")
        // A card's block heading (`-- Card ...`) is a comment, not a row.
        .filter(|line| !line.starts_with("--"))
        .map(|line| line.trim_end_matches(',').to_owned())
        .collect()
}

/// The right-hand side of a one-line Lean definition.
fn lean_definition(opening: &str) -> String {
    lean_definition_in(LEAN, opening)
}

/// The right-hand side of a one-line Lean definition in `module`.
fn lean_definition_in(module: &str, opening: &str) -> String {
    let line = module
        .lines()
        .find(|line| line.starts_with(opening))
        .unwrap_or_else(|| panic!("no `{opening}`"));
    line[opening.len()..].trim().to_owned()
}

#[test]
fn the_decision_vectors_are_the_lean_table() {
    let rust = DECISION_VECTORS
        .iter()
        .map(|(source, decision)| {
            let tree = raw(source);
            assert_eq!(check(&tree).is_ok(), *decision, "{source}");
            // The canonical printing reads back as the same tree.
            assert_eq!(raw(&canonical(&tree)), tree, "{source}");
            format!("({}, {decision})", lean(&tree))
        })
        .collect::<Vec<_>>();
    assert_eq!(
        lean_rows("def decisionVectors : List (Raw × Bool) := ["),
        rust
    );
}

#[test]
fn the_vocabulary_is_the_lean_table() {
    let rust = VOCABULARY
        .iter()
        .map(|term| {
            format!(
                "(\"{}\", [{}], {})",
                term.name,
                term.arguments
                    .iter()
                    .map(|sort| sort.lean())
                    .collect::<Vec<_>>()
                    .join(", "),
                term.result.lean()
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        lean_rows("def vocab : List (String × List Srt × Srt) := ["),
        rust
    );
    let quoted = |opening: &str, closing: &str| {
        let start = LEAN.find(opening).unwrap() + opening.len();
        let end = start + LEAN[start..].find(closing).unwrap();
        LEAN[start..end]
            .split('"')
            .skip(1)
            .step_by(2)
            .map(str::to_owned)
            .collect::<Vec<_>>()
    };
    assert_eq!(quoted("def classes : List String :=", "\n\n"), CLASSES);
    // Every name is distinct, and no constant is a term.
    let mut names = VOCABULARY.iter().map(|t| t.name).collect::<Vec<_>>();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), VOCABULARY.len());
}

#[test]
fn inert_is_its_definition() {
    let tree = raw(INERT_DEFINITION);
    assert_eq!(sort(&tree), Ok(Sort::Eff));
    assert_eq!(canonical(&tree), INERT_DEFINITION);
    assert_eq!(lean(&tree), lean_definition("def inertDefinition : Raw :="));
}

/// Every `resource_premise` clause of the OpenSSL crate's modules, read by
/// the contract format's premise reader: `(module, name, statement)`.
fn openssl_premises() -> Vec<(&'static str, String, PremiseStatement)> {
    premises_of(OPENSSL_MODULES)
}

/// Every `resource_premise` clause of `modules`, read by the contract
/// format's premise reader: `(module, name, statement)`.
fn premises_of(
    modules: &[(&'static str, &'static str)],
) -> Vec<(&'static str, String, PremiseStatement)> {
    let mut out = Vec::new();
    for (module, text) in modules {
        let stripped = text
            .lines()
            .map(|line| line.split("//").next().unwrap_or_default())
            .collect::<Vec<_>>()
            .join("\n");
        let mut rest = stripped.as_str();
        while let Some(at) = rest.find("requires resource_premise(") {
            let clause = &rest[at + "requires ".len()..];
            let end = clause.find(';').unwrap();
            let (name, statement) = premise_clause(&tokens(&clause[..end]))
                .unwrap_or_else(|e| panic!("{module}: {e}"))
                .unwrap();
            out.push((*module, name, statement));
            rest = &clause[end..];
        }
    }
    out
}

#[test]
fn the_openssl_formulas_are_the_lean_definitions() {
    // Each premise name with the Lean definitions its statements may be, and
    // how many clauses state each.
    let lean_names: &[(&str, &[(&str, usize)])] = &[
        (
            "openssl_ex_data_callbacks",
            &[("def exDataFormula : Raw :=", 30)],
        ),
        (
            "openssl_unstated_callback_slots",
            &[
                ("def unstatedFormula : Raw :=", 5),
                ("def unstatedFreeFormula : Raw :=", 3),
            ],
        ),
        (
            "openssl_builtin_bio_methods",
            &[("def builtinFormula : Raw :=", 10)],
        ),
        (
            "openssl_builtin_lookup_methods",
            &[("def builtinLookupFormula : Raw :=", 3)],
        ),
        (
            "openssl_bio_free_callbacks",
            &[("def bioFreeFormula : Raw :=", 2)],
        ),
    ];
    let mut by_name: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (module, name, statement) in openssl_premises() {
        let PremiseStatement::Formula(formula) = statement else {
            panic!("{module}: `{name}` is stated in prose");
        };
        assert_eq!(lean(&formula.raw), lean(&raw(&formula.canonical)));
        by_name.entry(name).or_default().push(lean(&formula.raw));
    }
    assert_eq!(by_name.len(), lean_names.len(), "{:?}", by_name.keys());
    let mut total = 0;
    for (name, definitions) in lean_names {
        let trees = &by_name[*name];
        for (definition, count) in *definitions {
            let tree = lean_definition(definition);
            assert_eq!(
                trees.iter().filter(|t| **t == tree).count(),
                *count,
                "{name} {definition}"
            );
            total += count;
        }
        assert_eq!(
            trees.len(),
            definitions.iter().map(|(_, count)| count).sum::<usize>(),
            "{name}: a statement matches no Lean definition"
        );
    }
    assert_eq!(total, 53);
}

#[test]
fn a_premise_clause_is_prose_or_a_formula() {
    let read = |source: &str| premise_clause(&tokens(source));
    assert_eq!(read("resource_borrowed(p)"), Ok(None));
    assert_eq!(read("resource_premise_all(a, b)"), Ok(None));
    let Ok(Some((name, PremiseStatement::Prose(text)))) =
        read("resource_premise(served, \"a callback takes nothing\")")
    else {
        panic!("prose premise");
    };
    assert_eq!(
        (name.as_str(), text.as_str()),
        ("served", "a callback takes nothing")
    );
    let Ok(Some((name, PremiseStatement::Formula(formula)))) =
        read("resource_premise(slots, quant  all #f in #stated :\n #f |-> #inert)")
    else {
        panic!("formula premise");
    };
    assert_eq!(name, "slots");
    // The statement is the canonical printing, whatever the spacing.
    assert_eq!(formula.canonical, "quant all #f in #stated: #f |-> #inert");
    for malformed in [
        "resource_premise",
        "resource_premise()",
        "resource_premise(Name, #stated |-> #inert)",
        "resource_premise(name)",
        "resource_premise(name, )",
        "resource_premise(name, \"\")",
        "resource_premise(name, \"a\", \"b\")",
        // A quoted formula: its identity would collide with the formula's.
        "resource_premise(name, \"#stated |-> #inert\")",
        // Fail-closed: an unknown term, an ill-sorted or unbound formula.
        "resource_premise(name, #unknown |-> #inert)",
        "resource_premise(name, #freed |-> #inert)",
        "resource_premise(name, #x |-> #inert)",
    ] {
        assert!(read(malformed).is_err(), "{malformed}");
    }
}

#[test]
fn malformed_syntax_is_refused() {
    for malformed in [
        "",
        "#args()",
        "#of(SSL, #args, #freed) |-> #inert",
        "#args | #created \\ #freed",
        "#args \\ #created \\ #freed",
        "#args \\ #created | #freed",
        // Set difference has one spelling, `\\`: `-` is refused.
        "#slots(#args) - #stated |-> #inert",
        "#args - #created",
        "#args | #created - #freed",
        "#stated |-> #inert |-> #inert",
        "#a in #b in #c",
        "#a == #b == #c",
        "quant #x in #args: #x in #freed",
        "quant all x in #args: #x in #freed",
        "quant all #x #args: #x in #freed",
        "quant all #x in #args #x in #freed",
        "quant all #X in #args: #X in #freed",
        "{#args}",
        "42 |-> #inert",
        "\"text\"",
        "ssl in #args",
        "#stated ||-> #inert",
        "#stated ->| #inert",
        "#stated !-> #inert",
        "#stated |-> #inert)",
        "(#stated |-> #inert",
        "#stated |-> #inert #inert",
        "#Stated |-> #inert",
        "#stated & #stated",
        "#stated || #stated",
        "!(#stated |-> #inert)",
    ] {
        let lexed = lex(malformed);
        let refused = match lexed {
            Err(_) => true,
            Ok(tokens) => {
                let kinds = tokens
                    .into_iter()
                    .map(|t| t.kind)
                    .filter(|k| *k != TokenKind::Eof)
                    .collect::<Vec<_>>();
                parse_tokens(&kinds).is_err()
            }
        };
        assert!(refused, "{malformed}");
    }
    // Bounds: nesting and length.
    let deep = format!(
        "{}#stated |-> #inert{}",
        "(".repeat(MAX_DEPTH + 1),
        ")".repeat(MAX_DEPTH + 1)
    );
    assert!(read_source(&deep).is_err());
    let long = vec!["#stated |-> #inert"; MAX_TOKENS / 3 + 1].join(" && ");
    assert!(read_source(&long).is_err());
    let fits = ["#stated |-> #inert"; 8].join(" && ");
    assert!(read_source(&fits).is_ok());
}

#[test]
fn the_printing_parenthesizes_exactly_where_needed() {
    for (source, printed) in [
        (
            "(#a |-> #inert) && (#b |-> #inert)",
            "#a |-> #inert && #b |-> #inert",
        ),
        (
            "#a |-> #inert && (#b |-> #inert && #c |-> #inert)",
            "#a |-> #inert && (#b |-> #inert && #c |-> #inert)",
        ),
        (
            "(quant all #x in #args: #x in #freed) && #stated |-> #inert",
            "(quant all #x in #args: #x in #freed) && #stated |-> #inert",
        ),
        (
            "#stated |-> #inert && (quant all #x in #args: #x in #freed)",
            "#stated |-> #inert && quant all #x in #args: #x in #freed",
        ),
        (
            "(#stated |-> #inert -> #stated |-> #inert) -> #stated |-> #inert",
            "(#stated |-> #inert -> #stated |-> #inert) -> #stated |-> #inert",
        ),
        ("((#args | #freed) | #created)", "#args | #freed | #created"),
        ("#args | (#freed | #created)", "#args | (#freed | #created)"),
        (
            "(#args | #freed) \\ #created",
            "(#args | #freed) \\ #created",
        ),
        (
            "#args \\ (#freed \\ #created)",
            "#args \\ (#freed \\ #created)",
        ),
    ] {
        let tree = raw(source);
        assert_eq!(canonical(&tree), printed, "{source}");
        assert_eq!(raw(printed), tree, "{source}");
    }
}

/// Card SP4-SYMBOLIC-LIBEVENT-PREMISES: each of the seven premises of
/// `mk_http_thread_destroy` is stated as a formula, once, and it is exactly
/// the Lean definition `QCore.PremiseFormula.Libevent` proves well-formed.
#[test]
fn the_libevent_formulas_are_the_lean_definitions() {
    let lean_names = [
        (
            "scheduler_exact_parent_embedding",
            "def parentEmbeddingFormula : Raw :=",
        ),
        (
            "scheduler_tls_params_owned",
            "def tlsParamsFormula : Raw :=",
        ),
        ("scheduler_callee_live", "def calleeLiveFormula : Raw :="),
        (
            "scheduler_borrowed_chain_live",
            "def borrowedChainFormula : Raw :=",
        ),
        (
            "scheduler_valid_membership",
            "def validMembershipFormula : Raw :=",
        ),
        (
            "scheduler_no_later_alias_use",
            "def noLaterAliasFormula : Raw :=",
        ),
        (
            "scheduler_valgrind_request_neutral",
            "def valgrindNeutralFormula : Raw :=",
        ),
    ];
    let premises = premises_of(LIBEVENT_MODULES);
    assert_eq!(premises.len(), lean_names.len());
    for (name, definition) in lean_names {
        let stated = premises
            .iter()
            .filter(|(_, premise, _)| premise == name)
            .collect::<Vec<_>>();
        assert_eq!(stated.len(), 1, "{name}");
        let PremiseStatement::Formula(formula) = &stated[0].2 else {
            panic!("`{name}` is stated in prose");
        };
        assert_eq!(raw(&formula.canonical), formula.raw, "{name}");
        assert_eq!(
            lean(&formula.raw),
            lean_definition_in(LEAN_LIBEVENT, definition),
            "{name}"
        );
        assert!(tokens(&formula.canonical).len() <= MAX_TOKENS, "{name}");
    }
}
