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

//! Card **W43-INDEX-TRAVERSAL**: the instrumented traversal census.
//!
//! A measurement, not a rule, built exactly like card
//! W43-INDIRECT-ACCESS-GENERAL's statement census: the whole module and every
//! call site of it are behind the off-by-default `indirect-census` feature,
//! every record is written **after** the classifier has decided, and nothing
//! reads the stream but the card's scripts. With the feature on, the pinned
//! corpus audit must still reproduce its artifacts byte for byte.
//!
//! One JSON line per record, to the path named by [`TRAVERSAL_CENSUS_PATH`]:
//!
//! * `condition` — every controlling expression (`if`, `while`, `for`,
//!   `do`-`while`, `switch`) the classifier refused, with the typed-condition
//!   recogniser's answer ([`crate::typed_field_access::typed_scalar_condition`]);
//! * `statement` — every statement or `for` clause that was refused, lowered
//!   to an indirect destination, copied between two typed pointer names or two
//!   declared scalars, or admitted by the typed scalar-access route with an
//!   increment inside a subscript, with the pointer-step and stepped-access
//!   recognisers' answers ([`crate::index_traversal`]);
//! * `function` — once per analysed function, its declared scalar and
//!   single-level pointer names.
//!
//! The recognisers are the ones the admission uses, so the census cannot
//! disagree with the rule it forecasts.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::sync::Mutex;

use crate::typed_field_access::{FunctionTypes, TypedAccessModel};

/// The environment variable naming the traversal census stream.
pub const TRAVERSAL_CENSUS_PATH: &str = "QCORE_TRAVERSAL_CENSUS";

static SINK: Mutex<Option<Option<BufWriter<File>>>> = Mutex::new(None);

#[derive(Clone, Debug, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TraversalRecord {
    Condition {
        unit_seq: u64,
        function: String,
        line: usize,
        position: &'static str,
        texts: Vec<String>,
        contains_call: bool,
        /// The typed-condition recogniser's bases, when it admits the
        /// condition.
        typed_bases: Option<Vec<String>>,
    },
    Statement {
        unit_seq: u64,
        function: String,
        line: usize,
        clause: &'static str,
        /// The instruction's outcome: a refusal's description, or the shape
        /// it was lowered to.
        outcome: String,
        texts: Vec<String>,
        /// `pointer-step`, `stepped-access`, `pointer-copy`, `scalar-copy`,
        /// `subscript-write` or `none`.
        recognised: &'static str,
        /// For a pointer step or copy: destination, source, offset.
        step: Option<(String, String, String)>,
        /// For a stepped access: its bases.
        bases: Vec<String>,
    },
    Function {
        unit_seq: u64,
        function: String,
        scalars: Vec<String>,
        pointers: Vec<String>,
    },
}

fn emit(record: &TraversalRecord) {
    let Ok(mut guard) = SINK.lock() else {
        return;
    };
    let opened = guard.get_or_insert_with(|| {
        let path = std::env::var_os(TRAVERSAL_CENSUS_PATH)?;
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

fn collecting() -> bool {
    std::env::var_os(TRAVERSAL_CENSUS_PATH).is_some()
}

fn owned(texts: &[&str]) -> Vec<String> {
    texts.iter().map(|text| (*text).to_owned()).collect()
}

/// Record one refused controlling expression.
pub(crate) fn record_condition(
    function: &str,
    line: usize,
    position: &'static str,
    texts: &[&str],
    contains_call: bool,
    model: &TypedAccessModel,
    types: &FunctionTypes,
) {
    if !collecting() {
        return;
    }
    let typed_bases = (!contains_call)
        .then(|| crate::typed_field_access::typed_scalar_condition(texts, model, types))
        .flatten()
        .map(|plan| plan.bases);
    emit(&TraversalRecord::Condition {
        unit_seq: crate::indirect_census::unit_seq(),
        function: function.to_owned(),
        line,
        position,
        texts: owned(texts),
        contains_call,
        typed_bases,
    });
}

/// What the traversal recognisers make of one already-classified statement,
/// or `None` when the statement is outside every recorded population.
pub(crate) struct StatementView<'a> {
    pub(crate) function: &'a str,
    pub(crate) line: usize,
    pub(crate) clause: &'static str,
    pub(crate) outcome: Option<String>,
    pub(crate) copy: Option<(&'static str, String, String)>,
    pub(crate) typed_with_subscript_write: bool,
    pub(crate) contains_call: bool,
}

/// Record one statement or `for` clause.
pub(crate) fn record_statement(
    view: StatementView<'_>,
    texts: &[&str],
    model: &TypedAccessModel,
    types: &FunctionTypes,
) {
    if !collecting() {
        return;
    }
    let (recognised, step, bases, outcome) = if let Some((kind, destination, source)) = view.copy {
        (
            kind,
            Some((destination, source, "copy".to_owned())),
            Vec::new(),
            view.outcome.unwrap_or_else(|| "copy".to_owned()),
        )
    } else if view.typed_with_subscript_write {
        (
            "subscript-write",
            None,
            Vec::new(),
            view.outcome
                .unwrap_or_else(|| "typed scalar access".to_owned()),
        )
    } else {
        let Some(outcome) = view.outcome else {
            return;
        };
        if view.contains_call {
            ("none", None, Vec::new(), outcome)
        } else if let Some(plan) = crate::index_traversal::pointer_step(texts, model, types) {
            (
                "pointer-step",
                Some((plan.destination, plan.source, format!("{:?}", plan.offset))),
                Vec::new(),
                outcome,
            )
        } else if let Some((plan, _)) = crate::index_traversal::stepped_access(texts, model, types)
        {
            ("stepped-access", None, plan.bases, outcome)
        } else {
            ("none", None, Vec::new(), outcome)
        }
    };
    emit(&TraversalRecord::Statement {
        unit_seq: crate::indirect_census::unit_seq(),
        function: view.function.to_owned(),
        line: view.line,
        clause: view.clause,
        outcome,
        texts: owned(texts),
        recognised,
        step,
        bases,
    });
}

/// Record one analysed function's declared traversal names.
pub(crate) fn record_function(function: &str, types: &FunctionTypes) {
    if !collecting() {
        return;
    }
    let (scalars, pointers) = types.traversal_names();
    emit(&TraversalRecord::Function {
        unit_seq: crate::indirect_census::unit_seq(),
        function: function.to_owned(),
        scalars: scalars.into_iter().collect(),
        pointers: pointers.into_iter().collect(),
    });
}
