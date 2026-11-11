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

//! Card CG16-INDEX-AND-OFFSET-EXTENTS: bounds contract v1 (grammar G05b,
//! contract specification C09.1) loads total-or-reject, keeps review state,
//! and is never read by the resource-v1 consumer.

use std::collections::BTreeMap;

#[path = "../../../tests/support/tracked_files.rs"]
mod tracked_files;

use qcore_contract::bounds_contract::{
    BoundsClause, BoundsCount, BoundsUnit, RESERVED_SPELLINGS, load_bounds_contract_sources,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

fn one(text: &str) -> BTreeMap<String, String> {
    BTreeMap::from([("bounds.qv".to_owned(), text.to_owned())])
}

const FILL: &str = "module contract;
extern fn fill(p: ptr<mut, u8>, n: usize, i: usize) -> ptr<mut, u8>
contract {
    requires bounds_contract_v1;
    requires bounds_extent(p, n, elements);
    requires bounds_index(i, n);
    ensures normal bounds_return_extent(n, elements);
};
";

#[test]
fn a_complete_declaration_loads_with_its_clauses() {
    let contracts = load_bounds_contract_sources(&one(FILL)).expect("loads");
    let fill = contracts.contract("fill").expect("fill");
    assert!(!fill.held());
    assert_eq!(fill.clauses.len(), 3);
    assert_eq!(
        fill.clauses[0].clause,
        BoundsClause::Extent {
            parameter: "p".into(),
            count: BoundsCount::Parameter("n".into()),
            unit: BoundsUnit::Elements
        }
    );
    assert_eq!(fill.text_sha256.len(), 64);
    assert_eq!(contracts.digest().len(), 64);
}

#[test]
fn a_review_marker_holds_the_declaration_and_names_the_clause() {
    let held = FILL.replace(
        "requires bounds_index(i, n);",
        "requires bounds_review_pending(bounds_index(i, n), \"is i always below n?\", \"owner\");",
    );
    let contracts = load_bounds_contract_sources(&one(&held)).expect("loads");
    let fill = contracts.contract("fill").unwrap();
    assert!(fill.held());
    let holds = fill.holds();
    assert_eq!(holds.len(), 1);
    assert!(holds[0].contains("bounds_index(i, n)"), "{holds:?}");
    assert!(holds[0].contains("is i always below n?"));
    assert!(holds[0].contains("acceptable-as-written-after-review"));
    // Contract-level, with the rewrite disposition.
    let whole = FILL.replace(
        "requires bounds_contract_v1;",
        "requires bounds_contract_v1;\n    requires bounds_review_pending(\"unchecked\", rewrite_before_acceptance);",
    );
    let fill = load_bounds_contract_sources(&one(&whole))
        .unwrap()
        .functions["fill"]
        .clone();
    assert!(fill.held());
    assert!(fill.holds()[0].contains("must-be-rewritten-before-acceptance"));
    // Resolution is the edit: the same text without the marker is in force,
    // and its digest differs from the held text's.
    let resolved = load_bounds_contract_sources(&one(FILL)).unwrap();
    assert_ne!(resolved.functions["fill"].text_sha256, fill.text_sha256);
}

#[test]
fn malformed_declarations_reject_the_whole_module() {
    let cases = [
        // No selector.
        FILL.replace("requires bounds_contract_v1;\n", ""),
        // Two selectors.
        FILL.replace("requires bounds_contract_v1;", "requires bounds_contract_v1; requires bounds_contract_v1;"),
        // A pointer extent on a scalar.
        FILL.replace("bounds_extent(p, n, elements)", "bounds_extent(n, 4, elements)"),
        // An unknown unit.
        FILL.replace("bounds_extent(p, n, elements)", "bounds_extent(p, n, words)"),
        // A non-canonical literal.
        FILL.replace("bounds_extent(p, n, elements)", "bounds_extent(p, 08, elements)"),
        // A postcondition under requires.
        FILL.replace("ensures normal bounds_return_extent", "requires bounds_return_extent"),
        // A blank reason.
        FILL.replace("requires bounds_index(i, n);", "requires bounds_review_pending(\" \");"),
        // A marker wrapping a marker.
        FILL.replace(
            "requires bounds_index(i, n);",
            "requires bounds_review_pending(bounds_review_pending(\"a\"), \"b\");",
        ),
        // A held selector.
        FILL.replace(
            "requires bounds_contract_v1;",
            "requires bounds_contract_v1; requires bounds_review_pending(bounds_contract_v1, \"x\");",
        ),
        // Two clauses for one position.
        FILL.replace("requires bounds_index(i, n);", "requires bounds_index(i, n); requires bounds_index(i, 4);"),
        // A resource-v1 clause.
        FILL.replace("requires bounds_index(i, n);", "ensures normal resource_borrowed(p);"),
        // An error-exit clause.
        FILL.replace("ensures normal", "ensures any"),
        // Inclusion and inheritance (G05a) are not part of this vocabulary.
        FILL.replace("module contract;", "module contract;\n#include \"other.qv\";"),
        // Card CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS (rule 14j): an
        // inheriting declaration is refused too, so neither form reaches a
        // bounds view un-elaborated.
        format!(
            "{FILL}\nextern fn child(p: ptr<mut, u8>, i: usize, n: usize) -> ptr<mut, u8>\ncontract inherit fill;\n"
        ),
    ];
    for case in cases {
        assert!(load_bounds_contract_sources(&one(&case)).is_err(), "{case}");
    }
}

/// Acceptance control: the old resource-v1 consumer presented with the new
/// clause rejects totally; it never publishes a full-contract result.
#[test]
fn the_resource_v1_consumer_rejects_a_bounds_clause() {
    let mixed = "module contract;
extern fn q_fill(p: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    requires bounds_extent(p, 4, elements);
    ensures normal resource_borrowed(p);
    ensures normal resource_return_none();
};
";
    assert!(load_resource_contract_sources(&one(mixed)).is_err());
    assert!(load_resource_contract_sources(&one(FILL)).is_err());
    assert!(load_bounds_contract_sources(&one(mixed)).is_err());
}

/// The source-spelling census for the new identifiers: none of them occurs
/// in any tracked contract module before this card (they are new words, so a
/// module that already used one would change meaning).
///
/// Card CHUNKIO-META-LEN-BOUNDS-CONTRACTS (deliberate change): a tracked
/// module the bounds loader accepts (a provider's bounds view: chunkio's
/// `meta_len_bounds.qv`) uses this vocabulary with its meaning, so it is
/// exempt; every other tracked module still uses none of the words, and the
/// number of tracked bounds modules is pinned.
#[test]
fn the_reserved_spellings_are_unused_by_tracked_contracts() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let files = tracked_files::tracked_files(&root)
        .into_iter()
        .filter(|path| {
            // Frozen report source snapshots are not production corpus modules.
            path.starts_with(root.join("providers"))
                && matches!(
                    path.extension().and_then(|e| e.to_str()),
                    Some("qv" | "qcore")
                )
        })
        .collect::<Vec<_>>();
    assert!(!files.is_empty(), "no tracked contract modules");
    let mut bounds_modules = Vec::new();
    for file in files {
        let text = std::fs::read_to_string(&file).unwrap_or_default();
        if load_bounds_contract_sources(&one(&text)).is_ok() {
            bounds_modules.push(file);
            continue;
        }
        for spelling in RESERVED_SPELLINGS {
            if spelling == "rewrite_before_acceptance" {
                continue; // C06.1's own word, reused with its meaning.
            }
            assert!(
                !text.contains(spelling),
                "{} uses `{spelling}`",
                file.display()
            );
        }
    }
    // Card CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS adds the second:
    // `chunkio/file_st_bounds.qv`, the neighbouring header accessors.
    // Card NULL-SAFETY-PROPERTY adds three: the `quant null-deref` modules of
    // cmetrics, librdkafka and fluent-bit (C09.3).
    // Card NULL-TRIAGE-UNRESOLVED-PATHS adds cfl's (`cfl_list_size`'s result
    // range, G05e). Card CELL-GUARD-RESULT-RANGE-CHECKED adds two: libc's
    // `libc/result_sets.qv` (G05j) and libevent's `monkey/result_range.qv`.
    // Card CELL-RESULT-SET-WRITTEN-SET-DERIVATION adds Fluent Bit's
    // `fluent_bit/result_range.qv` (`flb_pipe_create`'s derived written sets).
    assert_eq!(
        bounds_modules.len(),
        9,
        "tracked bounds modules: {bounds_modules:?}"
    );
    let mut names = bounds_modules
        .iter()
        .map(|path| {
            let mut parts = path.iter().rev().take(2).collect::<Vec<_>>();
            parts.reverse();
            parts
                .iter()
                .map(|part| part.to_string_lossy())
                .collect::<Vec<_>>()
                .join("/")
        })
        .collect::<Vec<_>>();
    names.sort();
    assert_eq!(
        names,
        [
            "cfl/null_deref.qv",
            "chunkio/file_st_bounds.qv",
            "chunkio/meta_len_bounds.qv",
            "cmetrics/null_deref.qv",
            "fluent_bit/null_deref.qv",
            "fluent_bit/result_range.qv",
            "libc/result_sets.qv",
            "librdkafka/null_deref.qv",
            "monkey/result_range.qv",
        ]
    );
}

/// Card CHUNKIO-META-LEN-BOUNDS-CONTRACTS: a pin not in force holds the whole
/// declaration exactly as a review marker does (`QCore.BoundsPinHold`'s
/// `standingVectors`: held unless no marker and the pin in force), names
/// itself in the holds, and is part of the environment digest.
#[test]
fn a_pin_hold_holds_the_declaration_and_changes_the_digest() {
    let held = FILL.replace(
        "    requires bounds_extent",
        "    requires bounds_review_pending(\"layout\");\n    requires bounds_extent",
    );
    // (review marker, pin not in force, held)
    for (review, mismatch, expected) in [
        (false, false, false),
        (true, false, true),
        (false, true, true),
        (true, true, true),
    ] {
        let mut contracts =
            load_bounds_contract_sources(&one(if review { &held } else { FILL })).unwrap();
        let before = contracts.digest();
        if mismatch {
            contracts
                .hold_for_pin_mismatch("fill", "source-changed", "body differs")
                .unwrap();
            assert_ne!(contracts.digest(), before);
        } else {
            assert_eq!(contracts.digest(), before);
        }
        let fill = &contracts.functions["fill"];
        assert_eq!(fill.held(), expected, "{review} {mismatch}");
        assert_eq!(
            fill.holds().iter().any(
                |h| h.starts_with("contract-pin-mismatch: fill: contract — pin source-changed")
            ),
            mismatch
        );
    }
    let mut contracts = load_bounds_contract_sources(&one(FILL)).unwrap();
    assert!(
        contracts
            .hold_for_pin_mismatch("absent", "no-lock", "")
            .is_err()
    );
}
