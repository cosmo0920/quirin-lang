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

//! Unit tests of card THREAD-LOCAL-CUSTODY-FREE-READ (gateway rule 54): the
//! decision table and the type and declaration readers.

use super::*;

#[test]
fn the_decision_table_is_the_vectors() {
    assert_eq!(ALL.len(), DECISION_VECTORS.len());
    for ((subject, used), (s, u, d)) in ALL.iter().zip(DECISION_VECTORS) {
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
fn only_a_custody_free_use_is_admitted() {
    for used in [Use::CopyToLocal, Use::InitLocal, Use::Return, Use::NullTest] {
        assert_eq!(decide(Subject::Object, used), Decision::Admit);
        assert_eq!(decide(Subject::Copy, used), Decision::Admit);
    }
    // A write to the object itself (a take-from-thread-local would hide a
    // transfer) withholds; a copy may be rebound.
    assert_eq!(decide(Subject::Object, Use::Rebind), Decision::Withhold);
    assert_eq!(decide(Subject::Copy, Use::Rebind), Decision::Admit);
    for used in [
        Use::Write,
        Use::Address,
        Use::Dereference,
        Use::CallArgument,
        Use::NotObjectPointer,
        Use::Other,
    ] {
        assert_eq!(decide(Subject::Object, used), Decision::Withhold);
        assert_eq!(decide(Subject::Copy, used), Decision::Withhold);
    }
}

#[test]
fn object_pointer_reads_the_type_spelling() {
    assert!(object_pointer("struct mk_event_loop *"));
    assert!(object_pointer("flb_ctx_t *"));
    assert!(object_pointer("void *"));
    assert!(object_pointer("const char *"));
    assert!(!object_pointer("char[64]"));
    assert!(!object_pointer("struct r"));
    assert!(!object_pointer("int"));
    assert!(!object_pointer("void (*)(void)"));
    assert!(!object_pointer("void *volatile"));
    assert!(!object_pointer("_Atomic(void *)"));
}

#[test]
fn a_thread_local_declaration_is_read_from_clangs_tls_field() {
    let decl = |text: &str| serde_json::from_str::<Value>(text).unwrap();
    assert!(declares_thread_local(&decl(
        r#"{"kind":"VarDecl","name":"t","tls":"static"}"#
    )));
    assert!(!declares_thread_local(&decl(
        r#"{"kind":"VarDecl","name":"g","storageClass":"static"}"#
    )));
    assert!(!declares_thread_local(&decl(
        r#"{"kind":"FunctionDecl","name":"f","tls":"static"}"#
    )));
}

#[test]
fn an_empty_reading_admits_no_copy() {
    let reads = ThreadLocalReads::default();
    assert!(reads.is_empty());
    assert!(!reads.admits_copy("c", "t"));
    let reads = ThreadLocalReads {
        objects: BTreeSet::from(["t".to_owned()]),
        copies: BTreeSet::from(["c".to_owned(), "d".to_owned()]),
    };
    assert!(reads.admits_copy("c", "t"));
    assert!(reads.admits_copy("d", "c"));
    assert!(!reads.admits_copy("t", "c"));
    assert!(!reads.admits_copy("e", "t"));
    assert!(!reads.admits_copy("c", "u"));
}
