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

//! Unit tests of card AG2-UNRESOLVED-ALIAS (gateway rule 62): the decision
//! table, the type reader and the copy admission.

use super::*;

#[test]
fn the_decision_table_is_the_vectors() {
    let all: Vec<(Subject, Use)> = [Subject::Formal, Subject::Copy]
        .into_iter()
        .flat_map(|subject| ALL_USES.into_iter().map(move |used| (subject, used)))
        .collect();
    assert_eq!(all.len(), DECISION_VECTORS.len());
    for ((subject, used), (s, u, d)) in all.iter().zip(DECISION_VECTORS) {
        assert_eq!(spelled(*subject, *used), (*s, *u));
        let expected = match *d {
            "admit" => Decision::Admit,
            "withhold" => Decision::Withhold,
            other => panic!("unknown decision {other}"),
        };
        assert_eq!(decide(*subject, *used), expected, "{s} {u}");
    }
}

#[test]
fn a_copy_handed_back_or_escaping_withholds() {
    for used in [Use::Return, Use::Write, Use::Address, Use::Other] {
        assert_eq!(decide(Subject::Copy, used), Decision::Withhold);
    }
    for used in [Use::Rebind, Use::Write, Use::Address, Use::Other] {
        assert_eq!(decide(Subject::Formal, used), Decision::Withhold);
    }
}

#[test]
fn object_pointer_reads_the_type_spelling() {
    assert!(object_pointer("struct x_rec *"));
    assert!(object_pointer("const void *"));
    assert!(object_pointer("const rd_kafka_topic_partition_t *"));
    assert!(!object_pointer("int"));
    assert!(!object_pointer("int (*)(void *)"));
    assert!(!object_pointer("char [8]"));
    assert!(!object_pointer("struct x_rec *volatile"));
    assert!(!object_pointer("_Atomic(struct x_rec *)"));
}

#[test]
fn a_pointer_to_pointer_formal_is_not_single_level() {
    assert!(single_level_pointer("const void *"));
    assert!(single_level_pointer("struct x_rec *"));
    // A top-level-qualified pointer spelling is not read (narrow, as rule 54).
    assert!(!single_level_pointer("const char *const"));
    assert!(!single_level_pointer("void **"));
    assert!(!single_level_pointer("struct x_rec *const *"));
    assert!(!single_level_pointer("char * *"));
    assert!(!single_level_pointer("int"));
}

fn sample() -> CustodyFreeCopies {
    CustodyFreeCopies {
        literals: true,
        constants: ["X_OK".to_owned()].into(),
        formals: ["ctx".to_owned()].into(),
        copies: ["r".to_owned(), "s".to_owned()].into(),
        locals: ["r".to_owned(), "s".to_owned(), "err".to_owned()].into(),
    }
}

#[test]
fn admits_reads_each_reading_and_nothing_else() {
    let copies = sample();
    assert_eq!(copies.admits("r", "ctx"), Some(Reading::FormalCopy));
    assert_eq!(copies.admits("s", "r"), Some(Reading::FormalCopy));
    assert_eq!(copies.admits("err", LITERAL), Some(Reading::Literal));
    assert_eq!(copies.admits("err", "X_OK"), Some(Reading::Constant));
    // A formal destination, an unknown source, a non-copy destination of a
    // formal: no reading.
    assert_eq!(copies.admits("ctx", LITERAL), None);
    assert_eq!(copies.admits("err", "ctx"), None);
    assert_eq!(copies.admits("err", "other"), None);
    let ambiguous = CustodyFreeCopies {
        literals: false,
        ..sample()
    };
    assert_eq!(ambiguous.admits("err", LITERAL), None);
    assert!(CustodyFreeCopies::default().is_empty());
    assert!(!sample().is_empty());
}
