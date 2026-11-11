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

//! Card **W45-SUB-OBJECT-STORES**: the instrumented census of stores into a
//! sub-object — the assignments whose left-hand side gateway constraint 5i
//! (card W44-ASSIGNMENT-DESTINATION-SUFFIX-PLACE) and the older lexical
//! destination arm refuse: `p->f = v`, `a[i].f = v`, `(*p)->f = v`,
//! `((T *) p)->f = v`, `(&s)->f = v`.
//!
//! A measurement, not a rule, built exactly like the destination-suffix and
//! member-chain streams: the whole module and its one call site are behind the
//! off-by-default `indirect-census` feature, every record is written **after**
//! the classifier has decided, and nothing reads the stream but the card's
//! scripts. With the feature on, the pinned corpus audit must still reproduce
//! its artifacts byte for byte.
//!
//! One JSON line, to the path named by [`SUB_OBJECT_STORE_CENSUS_PATH`], per
//! ordinary statement whose left-hand side (the tokens before its first `=`,
//! the classifier's own slice) the lexical gate reads as a sub-object store:
//! a named destination containing `.`, `[` or `->`, or one
//! `names_a_sub_object` recognises. Each record carries the classifier's
//! outcome and the declaration, binding and contract facts the card's scripts
//! classify the store by: the left-hand side's shape and resolved base, and
//! whether the stored value is a scalar, a pointer with no custody, an owned
//! or derived resource, a call result or a tracked descriptor.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::sync::Mutex;

use crate::indirect_census::Chain;

/// The environment variable naming the sub-object store census stream.
pub const SUB_OBJECT_STORE_CENSUS_PATH: &str = "QCORE_SUB_OBJECT_STORE_CENSUS";

static SINK: Mutex<Option<Option<BufWriter<File>>>> = Mutex::new(None);

/// One call the statement's value side makes, with the contract in force.
#[derive(Clone, Debug, serde::Serialize)]
pub struct CalleeFact {
    pub callee: String,
    /// `None` when no contract is in force for the callee.
    pub result: Option<String>,
    /// The contract's declared result kind (`Heap` unless a descriptor or
    /// other non-heap kind is declared).
    pub result_kind: Option<String>,
    /// Whether every declared parameter is `Borrowed`.
    pub parameters_borrowed: Option<bool>,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct StoreRecord {
    pub unit_seq: u64,
    pub function: String,
    pub line: usize,
    pub texts: Vec<String>,
    /// Index of the first `=`, the classifier's assignment.
    pub equal: usize,
    /// Whether that `=` is the first half of `==`.
    pub comparison: bool,
    /// The instruction's variant name.
    pub outcome: String,
    /// A refusal's description.
    pub description: Option<String>,
    pub admitted: bool,
    /// The formal parameters of the function the statement mentions.
    pub formals: Vec<String>,
    /// The declaration reader's label for each identifier.
    pub declared: BTreeMap<String, String>,
    /// [`crate::typed_field_access::FunctionTypes::may_bind`] per identifier.
    pub may_bind: Vec<String>,
    /// [`crate::borrowed_actual::UntrackedBases::never_bound`] per identifier.
    pub never_bound: Vec<String>,
    /// Names the body assigns an owned contracted result.
    pub owned_names: Vec<String>,
    /// Names the body assigns a derived contracted result.
    pub derived_names: Vec<String>,
    /// Names that may hold a tracked integer-carried identity.
    pub descriptors: Vec<String>,
    /// The access chains of the left-hand side, as the indirect census reads
    /// them.
    pub destination_chains: Vec<Chain>,
    /// Every call on the value side, in order.
    pub callees: Vec<CalleeFact>,
    /// The rule's own reader over this statement: `destination:<refusal>`,
    /// `value:<refusal>`, or `read:<member>:<value>:<caller-visible>` when
    /// both sides resolve (a call value is decided by the classifier, not
    /// here).
    pub reader: String,
}

/// Whether the stream is being collected.
pub(crate) fn collecting() -> bool {
    std::env::var_os(SUB_OBJECT_STORE_CENSUS_PATH).is_some()
}

/// Record one classified statement.
pub(crate) fn record(record: &StoreRecord) {
    let Ok(mut guard) = SINK.lock() else {
        return;
    };
    let opened = guard.get_or_insert_with(|| {
        let path = std::env::var_os(SUB_OBJECT_STORE_CENSUS_PATH)?;
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

/// The identifiers of `texts` that `keep` accepts, sorted and without repeats.
pub(crate) fn names_where(texts: &[&str], keep: impl Fn(&str) -> bool) -> Vec<String> {
    texts
        .iter()
        .filter(|text| crate::typed_field_access::is_name(text) && keep(text))
        .map(|text| (*text).to_owned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_filtered_sorted_and_unique() {
        let texts = ["b", "-", ">", "f", "=", "a", "+", "b", "+", "1"];
        assert_eq!(names_where(&texts, |_| true), vec!["a", "b", "f"]);
        assert_eq!(names_where(&texts, |name| name == "a"), vec!["a"]);
    }
}
