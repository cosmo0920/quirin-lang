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

//! Card **SP2-SYMBOLIC-LIBRDKAFKA-PREMISES** (contract specification
//! C04.18.9): every `resource_premise` clause of `qcore-provider-librdkafka`
//! states a formula, each formula is a definition of the Lean module
//! `QCore.PremiseFormula.Librdkafka`, and every term of the SP2 vocabulary
//! block is used by one of them.

use std::collections::{BTreeMap, BTreeSet};

use qcore_frontend::lexer::{TokenKind, lex};
use qcore_frontend::premise_formula::{PremiseFormula, Raw, VOCABULARY, lean, premise_clause};

const LEAN: &str = include_str!("../../../proof/QCore/PremiseFormula/Librdkafka.lean");

/// Every module of the crate (`providers/contract-load-path.json`'s 14), as
/// shipped.
const MODULES: &[(&str, &str)] = &[
    (
        "cjson.qv",
        include_str!("../../../providers/qcore-provider-librdkafka/contracts/librdkafka/cjson.qv"),
    ),
    (
        "client.qv",
        include_str!("../../../providers/qcore-provider-librdkafka/contracts/librdkafka/client.qv"),
    ),
    (
        "header_inlines.qv",
        include_str!(
            "../../../providers/qcore-provider-librdkafka/contracts/librdkafka/header_inlines.qv"
        ),
    ),
    (
        "kafka_mem.qv",
        include_str!(
            "../../../providers/qcore-provider-librdkafka/contracts/librdkafka/kafka_mem.qv"
        ),
    ),
    (
        "log_caller_frame.qv",
        include_str!(
            "../../../providers/qcore-provider-librdkafka/contracts/librdkafka/log_caller_frame.qv"
        ),
    ),
    (
        "logging.qv",
        include_str!(
            "../../../providers/qcore-provider-librdkafka/contracts/librdkafka/logging.qv"
        ),
    ),
    (
        "lz4_stream.qv",
        include_str!(
            "../../../providers/qcore-provider-librdkafka/contracts/librdkafka/lz4_stream.qv"
        ),
    ),
    (
        "memory.qv",
        include_str!("../../../providers/qcore-provider-librdkafka/contracts/librdkafka/memory.qv"),
    ),
    (
        "null_deref.qv",
        include_str!(
            "../../../providers/qcore-provider-librdkafka/contracts/librdkafka/null_deref.qv"
        ),
    ),
    (
        "outer.qv",
        include_str!("../../../providers/qcore-provider-librdkafka/contracts/librdkafka/outer.qv"),
    ),
    (
        "queue_destructor.qv",
        include_str!(
            "../../../providers/qcore-provider-librdkafka/contracts/librdkafka/queue_destructor.qv"
        ),
    ),
    (
        "refcount.qv",
        include_str!(
            "../../../providers/qcore-provider-librdkafka/contracts/librdkafka/refcount.qv"
        ),
    ),
    (
        "topic_partition.qv",
        include_str!(
            "../../../providers/qcore-provider-librdkafka/contracts/librdkafka/topic_partition.qv"
        ),
    ),
    (
        "typed_queue.qv",
        include_str!(
            "../../../providers/qcore-provider-librdkafka/contracts/librdkafka/typed_queue.qv"
        ),
    ),
];

/// Each premise name with the Lean definitions its statements are, and how
/// many clauses state each (the census at base: 220 clauses, 12 names).
const LEAN_NAMES: &[(&str, &[(&str, usize)])] = &[
    (
        "rk_plugin_callback_closure",
        &[("def closureFormula : Raw :=", 180)],
    ),
    (
        "conf_property_writer",
        &[("def confWriterFormula : Raw :=", 18)],
    ),
    (
        "asserted_allocation",
        &[
            ("def assertedCallocFormula : Raw :=", 5),
            ("def assertedTopparFormula : Raw :=", 1),
            ("def assertedMallocFormula : Raw :=", 1),
            ("def assertedDeployedFormula : Raw :=", 2),
        ],
    ),
    ("served_queue", &[("def servedQueueFormula : Raw :=", 2)]),
    (
        "broker_monitor_callback",
        &[("def brokerMonitorFormula : Raw :=", 1)],
    ),
    ("assignor_callback", &[("def assignorFormula : Raw :=", 1)]),
    (
        "share_ack_batches_nonempty",
        &[("def shareAckFormula : Raw :=", 1)],
    ),
    (
        "rk_log_enqueue_no_event_callback",
        &[("def logNoEventFormula : Raw :=", 2)],
    ),
    (
        "rk_log_enqueue_forward_not_last",
        &[("def logForwardFormula : Raw :=", 2)],
    ),
    (
        "cjson_allocator_hooks",
        &[
            ("def hooksNewItemFormula : Raw :=", 1),
            ("def hooksPrintFormula : Raw :=", 1),
        ],
    ),
    (
        "cjson_list_invariant",
        &[("def listInvariantFormula : Raw :=", 1)],
    ),
    (
        "detach_list_representation",
        &[("def detachFormula : Raw :=", 1)],
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

/// The right-hand side of a one-line Lean definition.
fn lean_definition(opening: &str) -> String {
    let line = LEAN
        .lines()
        .find(|line| line.starts_with(opening))
        .unwrap_or_else(|| panic!("no `{opening}`"));
    line[opening.len()..].trim().to_owned()
}

/// Every `resource_premise` clause of the crate, read by the contract
/// format's premise reader: `(module, name, statement)`. Line comments are
/// dropped first, so a clause quoted in a comment is not a clause.
fn premises() -> Vec<(&'static str, String, PremiseFormula)> {
    let mut out = Vec::new();
    for (module, text) in MODULES {
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
fn the_librdkafka_formulas_are_the_lean_definitions() {
    let mut by_name: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (_, name, formula) in premises() {
        by_name.entry(name).or_default().push(lean(&formula.raw));
    }
    assert_eq!(by_name.len(), LEAN_NAMES.len(), "{:?}", by_name.keys());
    let mut total = 0;
    for (name, definitions) in LEAN_NAMES {
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
    assert_eq!(total, 220);
}

fn terms_of(raw: &Raw, out: &mut BTreeSet<String>) {
    match raw {
        Raw::Quant { domain, body, .. } => {
            terms_of(domain, out);
            terms_of(body, out);
        }
        Raw::Bin(_, left, right) => {
            terms_of(left, out);
            terms_of(right, out);
        }
        Raw::App0(name) => {
            out.insert(name.clone());
        }
        Raw::App1(name, argument) => {
            out.insert(name.clone());
            terms_of(argument, out);
        }
        Raw::App2(name, first, second) => {
            out.insert(name.clone());
            terms_of(first, out);
            terms_of(second, out);
        }
        Raw::Const(_) | Raw::Empty => {}
    }
}

/// The SP2 block holds no unused term: each is named by a librdkafka
/// formula (the vocabulary is closed and every entry is load-bearing).
#[test]
fn every_sp2_term_is_used_by_a_librdkafka_formula() {
    let first = VOCABULARY
        .iter()
        .position(|term| term.name == "false")
        .expect("the SP2 block starts at `#false`");
    let mut used = BTreeSet::new();
    for (_, _, formula) in premises() {
        terms_of(&formula.raw, &mut used);
    }
    let block = VOCABULARY[first..first + 75]
        .iter()
        .map(|term| term.name.to_owned())
        .collect::<BTreeSet<_>>();
    assert_eq!(block.len(), 75);
    let unused = block.difference(&used).collect::<Vec<_>>();
    assert!(unused.is_empty(), "unused SP2 terms: {unused:?}");
}
