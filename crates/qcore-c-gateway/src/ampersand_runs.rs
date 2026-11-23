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

//! Card **AMPERSAND-RUN-PARITY-OTHER-READERS** (gateway rule 74e): the one
//! reading of a run of `&` tokens every address decision of the unfused
//! token stream uses.
//!
//! The importer's lexer emits one token per punctuation character, so `p &&
//! q` reaches a reader as `p & & q` and `p && &q` as `p & & & q`. C lexes a
//! run of adjacent `&` characters by maximal munch (C11 6.4p4): `k` of them
//! are `k / 2` `&&` operators followed, when `k` is odd, by one `&`. A
//! whitespace-separated `a & &b` would be a binary `&` with a pointer operand,
//! which C11 6.5.10p2 forbids, so no conforming body spells it. The last `&`
//! of a run before an operand is therefore an `&` operator exactly when the
//! run is odd ([`ends_in_address`]); an even run ends in a logical AND
//! ([`ends_in_logical_and`]), or GNU's `&&label`, which names no object.
//!
//! Whether a single `&` is the unary address operator or a binary AND is a
//! separate question (the token before it), which each reader keeps deciding
//! as it did: the parity only replaces the reading of `&&`. Rule 74d (C)
//! introduced the parity for rule 10a's frame; `QCore.AmpersandRunParity`
//! states it over `QCore.NullableLazyConstructor`'s maximal-munch lemma and
//! lists every reader that uses it ([`READERS`]).

/// The number of consecutive `&` tokens ending just before `at`.
pub(crate) fn run_before<S: AsRef<str>>(texts: &[S], at: usize) -> usize {
    let end = at.min(texts.len());
    texts[..end]
        .iter()
        .rev()
        .take_while(|text| text.as_ref() == "&")
        .count()
}

/// Whether the run of `&` tokens ending just before `at` ends in a single
/// `&` operator (the run is odd): `&q` and `p && &q` take an address (when
/// that `&` is unary), `p && q` does not.
pub(crate) fn ends_in_address<S: AsRef<str>>(texts: &[S], at: usize) -> bool {
    run_before(texts, at) % 2 == 1
}

/// Whether the run of `&` tokens ending just before `at` is a non-empty even
/// run, so the operand at `at` is the right operand of a logical AND.
pub(crate) fn ends_in_logical_and<S: AsRef<str>>(texts: &[S], at: usize) -> bool {
    let run = run_before(texts, at);
    run >= 2 && run.is_multiple_of(2)
}

/// The index after each maximal run of `&` tokens that ends in a single `&`
/// operator (an odd run). Rule 74d's reading for rule 10a's frame, moved here
/// unchanged from `legacy_c.rs`.
pub(crate) fn unary_address_runs<S: AsRef<str>>(body: &[S]) -> Vec<usize> {
    let mut after = Vec::new();
    let mut index = 0;
    while index < body.len() {
        if body[index].as_ref() != "&" {
            index += 1;
            continue;
        }
        let start = index;
        while body.get(index).is_some_and(|text| text.as_ref() == "&") {
            index += 1;
        }
        if (index - start) % 2 == 1 {
            after.push(index);
        }
    }
    after
}

/// The files whose readers decide address-taking from the unfused token
/// stream through this module, with the number of call sites in each and the
/// class of the reading each replaced (`QCore.AmpersandRunParity.readers`
/// restates the list). A test counts the call sites, so a reader added or
/// removed updates this census.
#[cfg(test)]
pub(crate) const READERS: &[(&str, usize, &str)] = &[
    ("legacy_c.rs", 1, "frame"),
    ("loop_owner_instance.rs", 2, "unsound"),
    ("loop_owner_holder_opaque.rs", 2, "unsound"),
    ("thread_entry_census.rs", 3, "unsound"),
    ("legacy_retained_by.rs", 2, "admitting+fail-closed"),
    ("legacy_callback_route.rs", 2, "admitting+fail-closed"),
    ("legacy_list_iteration.rs", 2, "fail-closed"),
    ("legacy_reference_result.rs", 1, "fail-closed"),
    ("indexed_collection.rs", 1, "fail-closed"),
    ("aggregate_store.rs", 1, "fail-closed"),
    ("registration_by_id.rs", 2, "fail-closed"),
    ("typed_field_access.rs", 1, "fail-closed"),
    ("borrowed_actual.rs", 2, "fail-closed"),
    ("legacy_retention_guard.rs", 1, "fail-closed"),
    ("legacy_reference_count.rs", 1, "fail-closed"),
    ("legacy_slot_move_borrow.rs", 1, "fail-closed"),
    ("legacy_refused_operands.rs", 1, "fail-closed"),
    ("legacy_in_place_custody.rs", 1, "fail-closed"),
];

/// `(tokens, index of the operand, run ends in an & operator?, run ends in
/// a logical AND?)`: the vectors `QCore.AmpersandRunParity.decisionVectors`
/// restates.
#[cfg(test)]
pub(crate) const DECISION_VECTORS: &[(&str, usize, bool, bool)] = &[
    ("p & & & q", 4, true, false),
    ("p & & q", 3, false, true),
    ("p & q", 2, true, false),
    ("& & & q", 3, true, false),
    ("& ( q )", 1, true, false),
    ("a & & q", 3, false, true),
    ("= q", 1, false, false),
    ("& & label", 2, false, true),
    ("p & & & & q", 5, false, true),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rule_74e_decision_vectors_hold() {
        for (source, at, address, logical) in DECISION_VECTORS {
            let body = source.split(' ').collect::<Vec<_>>();
            assert_eq!(ends_in_address(&body, *at), *address, "{source}");
            assert_eq!(ends_in_logical_and(&body, *at), *logical, "{source}");
        }
    }

    #[test]
    fn the_runs_agree_with_the_per_index_reading() {
        let body = "p & & & q ; r & & s ; & t ; a & ( b )"
            .split(' ')
            .collect::<Vec<_>>();
        let runs = unary_address_runs(&body);
        let per_index = (1..body.len())
            .filter(|&at| body[at] != "&" && body[at - 1] == "&" && ends_in_address(&body, at))
            .collect::<Vec<_>>();
        assert_eq!(runs, per_index);
        assert_eq!(runs, vec![4, 12, 16]);
    }

    #[test]
    fn every_reader_of_the_census_reads_the_parity() {
        let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let needle = ["crate", "ampersand_runs", ""].join("::");
        let mut counted = std::collections::BTreeMap::new();
        for entry in walk(&source) {
            let text = std::fs::read_to_string(&entry).expect("source");
            let count = text.matches(&needle).count();
            if count > 0 {
                let name = entry.strip_prefix(&source).expect("prefix");
                counted.insert(name.to_string_lossy().into_owned(), count);
            }
        }
        let listed = READERS
            .iter()
            .map(|(file, count, _)| ((*file).to_owned(), *count))
            .collect::<std::collections::BTreeMap<_, _>>();
        assert_eq!(counted, listed);
    }

    fn walk(directory: &std::path::Path) -> Vec<std::path::PathBuf> {
        let mut out = Vec::new();
        for entry in std::fs::read_dir(directory).expect("directory") {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                out.extend(walk(&path));
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                out.push(path);
            }
        }
        out
    }

    #[test]
    fn a_run_at_the_start_and_beyond_the_end_is_read() {
        let body = ["&", "&", "&"];
        assert_eq!(run_before(&body, 3), 3);
        assert_eq!(run_before(&body, 9), 3);
        assert_eq!(run_before(&body, 0), 0);
        assert!(!ends_in_address(&body, 0));
    }
}
