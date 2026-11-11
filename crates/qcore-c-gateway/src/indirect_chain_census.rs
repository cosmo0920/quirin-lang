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

//! Card **W44-INDIRECT-CHAIN-ACCESS**: the instrumented member-chain census.
//!
//! A measurement, not a rule, built exactly like card W43-INDEX-TRAVERSAL's
//! traversal stream: the whole module and both of its call sites are behind
//! the off-by-default `indirect-census` feature, every record is written
//! **after** the classifier has decided, and nothing reads the stream but the
//! card's scripts. With the feature on, the pinned corpus audit must still
//! reproduce its artifacts byte for byte.
//!
//! One JSON line per refused statement, `for` clause or controlling expression
//! whose tokens carry a member selection, to the path named by
//! [`CHAIN_CENSUS_PATH`], with:
//!
//! * `admitted` — the chain recogniser's answer
//!   ([`crate::indirect_chain_access::chain_statement`] or
//!   [`crate::indirect_chain_access::chain_condition`]), exactly the function
//!   the rule calls, so the census cannot disagree with the rule it forecasts;
//! * `occurrences` — every position where the chain reader
//!   ([`crate::indirect_chain_access::parse_chain`]) found a chain-shaped
//!   access, with its answer or refusal, depth, root and final member.

use std::collections::BTreeSet;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::sync::Mutex;

use crate::borrowed_actual::UntrackedBases;
use crate::indirect_chain_access::{AccessChain, ChainRefusal, parse_chain};
use crate::typed_field_access::{DeclaredType, FunctionTypes, TypedAccessModel};

/// The environment variable naming the chain census stream.
pub const CHAIN_CENSUS_PATH: &str = "QCORE_CHAIN_CENSUS";

static SINK: Mutex<Option<Option<BufWriter<File>>>> = Mutex::new(None);

/// One chain-shaped access the reader met.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Occurrence {
    pub index: usize,
    /// `ok` or a [`ChainRefusal`] label.
    pub result: &'static str,
    /// Member selections, when parsed.
    pub selections: Option<usize>,
    pub root: String,
    /// `formal`, `local` (declared by this function), or `undeclared`.
    pub root_kind: &'static str,
    /// `record-pointer`, `record`, `scalar`, `opaque` or `none`.
    pub root_type: &'static str,
    pub root_never_bound: bool,
    pub root_may_bind: bool,
    /// `scalar`, `pointer` or `other`, when parsed.
    pub member_kind: Option<&'static str>,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct ChainSummary {
    pub root: String,
    pub dereferenced_root: bool,
    pub root_never_bound: bool,
    pub links: Vec<String>,
    pub member: String,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct ChainRecord {
    pub kind: &'static str,
    pub unit_seq: u64,
    pub function: String,
    pub line: usize,
    /// The statement clause (`statement`, `for-init`, `for-step`) or the
    /// condition position (`branch`, `loop`, `for`, `do-while`, `switch`).
    pub position: &'static str,
    /// A statement's refusal description; `None` for a condition.
    pub outcome: Option<String>,
    /// Whether the classifier consults the typed route for this refusal.
    pub consulted: bool,
    pub contains_call: bool,
    pub texts: Vec<String>,
    pub admitted: Option<Vec<ChainSummary>>,
    pub occurrences: Vec<Occurrence>,
}

fn emit(record: &ChainRecord) {
    let Ok(mut guard) = SINK.lock() else {
        return;
    };
    let opened = guard.get_or_insert_with(|| {
        let path = std::env::var_os(CHAIN_CENSUS_PATH)?;
        File::create(path).ok().map(BufWriter::new)
    });
    let Some(writer) = opened.as_mut() else {
        return;
    };
    if let Ok(line) = serde_json::to_string(record) {
        let _ = writeln!(writer, "{line}");
        let _ = writer.flush();
    }
}

/// The declaration facts one record reads.
pub(crate) struct ChainContext<'a> {
    pub(crate) function: &'a str,
    pub(crate) formal_parameters: &'a BTreeSet<String>,
    pub(crate) model: &'a TypedAccessModel,
    pub(crate) types: &'a FunctionTypes,
    pub(crate) bases: &'a UntrackedBases,
}

fn summaries(chains: Vec<AccessChain>) -> Vec<ChainSummary> {
    chains
        .into_iter()
        .map(|chain| ChainSummary {
            root: chain.root,
            dereferenced_root: chain.dereferenced_root,
            root_never_bound: chain.root_never_bound,
            links: chain.links,
            member: chain.member,
        })
        .collect()
}

fn occurrences(context: &ChainContext<'_>, texts: &[&str]) -> Vec<Occurrence> {
    let mut found = Vec::new();
    let mut index = 0usize;
    while index < texts.len() {
        let result = parse_chain(texts, index, context.model, context.types);
        if result == Err(ChainRefusal::NotAChain) {
            index += 1;
            continue;
        }
        let root = if texts.get(index) == Some(&"(") {
            texts.get(index + 2).copied().unwrap_or_default()
        } else {
            texts[index]
        };
        let root_kind = if context.formal_parameters.contains(root) {
            "formal"
        } else if context.types.declared_type(root).is_some() {
            "local"
        } else {
            "undeclared"
        };
        let root_type = match context.types.declared_type(root) {
            Some(DeclaredType::RecordPointer(_)) => "record-pointer",
            Some(DeclaredType::Record(_)) => "record",
            Some(DeclaredType::Scalar) => "scalar",
            Some(DeclaredType::Opaque) => "opaque",
            None => "none",
        };
        let (label, selections, member_kind, next) = match &result {
            Ok(chain) => (
                "ok",
                Some(chain.selections()),
                Some(chain.member_kind.label()),
                chain.end,
            ),
            Err(refusal) => (refusal.label(), None, None, index + 1),
        };
        found.push(Occurrence {
            index,
            result: label,
            selections,
            root: root.to_owned(),
            root_kind,
            root_type,
            root_never_bound: context.bases.never_bound(root),
            root_may_bind: context.types.may_bind(root),
            member_kind,
        });
        index = next;
    }
    found
}

fn selects_a_member(texts: &[&str]) -> bool {
    texts
        .windows(2)
        .any(|pair| pair[0] == "." || (pair[0] == "-" && pair[1] == ">"))
}

/// Record one refused statement or `for` clause carrying a member selection.
#[allow(clippy::too_many_arguments)]
pub(crate) fn record_statement(
    context: &ChainContext<'_>,
    line: usize,
    position: &'static str,
    outcome: String,
    consulted: bool,
    contains_call: bool,
    texts: &[&str],
) {
    if std::env::var_os(CHAIN_CENSUS_PATH).is_none() || !selects_a_member(texts) {
        return;
    }
    let admitted = crate::indirect_chain_access::chain_statement(
        texts,
        context.model,
        context.types,
        context.bases,
    )
    .map(|plan| summaries(plan.chains));
    emit(&ChainRecord {
        kind: "statement",
        unit_seq: crate::indirect_census::unit_seq(),
        function: context.function.to_owned(),
        line,
        position,
        outcome: Some(outcome),
        consulted,
        contains_call,
        texts: texts.iter().map(|text| (*text).to_owned()).collect(),
        admitted,
        occurrences: occurrences(context, texts),
    });
}

/// Record one refused controlling expression carrying a member selection.
pub(crate) fn record_condition(
    context: &ChainContext<'_>,
    line: usize,
    position: &'static str,
    contains_call: bool,
    texts: &[&str],
) {
    if std::env::var_os(CHAIN_CENSUS_PATH).is_none() || !selects_a_member(texts) {
        return;
    }
    let admitted = crate::indirect_chain_access::chain_condition(
        texts,
        context.model,
        context.types,
        context.bases,
    )
    .map(|plan| summaries(plan.chains));
    emit(&ChainRecord {
        kind: "condition",
        unit_seq: crate::indirect_census::unit_seq(),
        function: context.function.to_owned(),
        line,
        position,
        outcome: None,
        consulted: true,
        contains_call,
        texts: texts.iter().map(|text| (*text).to_owned()).collect(),
        admitted,
        occurrences: occurrences(context, texts),
    });
}
