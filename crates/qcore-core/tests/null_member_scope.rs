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

//! Card NULL-MEMBER-INVARIANT-SCOPE: the Rust decisions of
//! `qcore_core::null_member_scope` against the shared vectors of
//! `proof/QCore/NullMemberScope.lean` (`stableVectors`, `persistVectors`),
//! read from the Lean text so neither side can change alone.

use std::collections::BTreeSet;

use qcore_core::null_deref::{Nullness, Proof, Source};
use qcore_core::null_member_scope::{persist, stable_name, stable_standing};

const LEAN: &str = include_str!("../../../proof/QCore/NullMemberScope.lean");

fn lean_rows(header: &str) -> Vec<String> {
    let start = LEAN.find(header).expect("the Lean vector list") + header.len();
    let body = &LEAN[start..];
    let end = body.find("\n]").expect("the list's end");
    body[..end]
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| line.trim_end_matches(',').to_owned())
        .collect()
}

const STABLE_VECTORS: [(usize, usize, &str); 4] = [
    (0, 0, "declared-assumption"),
    (4, 0, "partly-validated"),
    (4, 1, "withdrawn"),
    (0, 2, "withdrawn"),
];

const PERSIST_VECTORS: [(&str, bool, &str); 5] = [
    ("nonnull", true, "kept"),
    ("nonnull", false, "forgotten"),
    ("null", true, "forgotten"),
    ("maybe", true, "forgotten"),
    ("maybe", false, "forgotten"),
];

fn fact(name: &str) -> Nullness {
    match name {
        "nonnull" => Nullness::proved(Proof::Tested(1)),
        "null" => Nullness::Null,
        _ => Nullness::Maybe(BTreeSet::from([Source::Formal("p".into())])),
    }
}

#[test]
fn the_vectors_are_the_leans() {
    let stable_rows = STABLE_VECTORS
        .iter()
        .map(|(e, f, s)| format!("({e}, {f}, \"{s}\")"))
        .collect::<Vec<_>>();
    assert_eq!(
        lean_rows("def stableVectors : List (Nat × Nat × String) := ["),
        stable_rows
    );
    let persist_rows = PERSIST_VECTORS
        .iter()
        .map(|(f, s, o)| format!("(\"{f}\", {s}, \"{o}\")"))
        .collect::<Vec<_>>();
    assert_eq!(
        lean_rows("def persistVectors : List (String × Bool × String) := ["),
        persist_rows
    );
}

#[test]
fn the_rust_decides_every_vector() {
    for (e, f, s) in STABLE_VECTORS {
        assert_eq!(stable_standing(e, f), s, "{e} {f}");
    }
    for (f, s, o) in PERSIST_VECTORS {
        let kept = persist(&fact(f), "S.next", s);
        assert_eq!(
            if kept.is_some() { "kept" } else { "forgotten" },
            o,
            "{f} {s}"
        );
    }
}

#[test]
fn a_kept_fact_carries_the_stable_proof() {
    let kept = persist(&fact("nonnull"), "S.next", true).expect("kept");
    let Nullness::NonNull(proofs) = kept else {
        panic!("non-null")
    };
    assert!(proofs.contains(&Proof::NonNullResult(stable_name("S.next"))));
    assert!(proofs.contains(&Proof::Tested(1)));
}
