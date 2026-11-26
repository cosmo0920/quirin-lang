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

//! Card RETURN-STRICT-MEMBER-NULL-UNTRACKED-ROOT (rule 74v): what B01's
//! admitted opaque member read dereferences. End to end:
//! `tests/return_strict_member_null.rs`.

use std::collections::BTreeSet;

use super::super::return_family::{NameFacts, scalar_read};
use super::*;

fn texts(expression: &str) -> Vec<&str> {
    expression.split_whitespace().collect()
}

fn read(expression: &str) -> MemberReadBase<'_> {
    // The reading borrows its root from the token vector; leak it so the
    // test can compare against a `'static` answer.
    member_read_base(Box::leak(Box::new(texts(expression))))
}

/// `p` and `s` are untracked; nothing is a declared scalar or a type.
struct Untracked {
    objects: BTreeSet<String>,
}

impl NameFacts for Untracked {
    fn untracked(&self, _name: &str) -> bool {
        true
    }

    fn declared_scalar(&self, _name: &str) -> bool {
        false
    }

    fn type_name(&self, _words: &[&str]) -> bool {
        false
    }

    fn objects(&self) -> &BTreeSet<String> {
        &self.objects
    }
}

/// The rows of `QCore.ReturnStrictMemberNull.readVectors`: a spelling and
/// its class (`value`, `root NAME` or `declined`).
fn lean_rows() -> Vec<(String, String)> {
    const LEAN: &str = include_str!("../../../../proof/QCore/ReturnStrictMemberNull.lean");
    let body = LEAN
        .split("def readVectors : ")
        .nth(1)
        .expect("readVectors");
    let body = &body[..body.find("\n]").expect("end")];
    body.split("\n  (\"")
        .skip(1)
        .map(|row| {
            let mut fields = row.split('"');
            let spelling = fields.next().expect("spelling").to_owned();
            let rest = fields.collect::<Vec<_>>().join("\"");
            let class = if rest.contains(".value") {
                "value".to_owned()
            } else if rest.contains(".declined") {
                "declined".to_owned()
            } else {
                let root = rest
                    .split(".root \"")
                    .nth(1)
                    .and_then(|tail| tail.split('"').next())
                    .expect("root name");
                format!("root {root}")
            };
            (spelling, class)
        })
        .collect()
}

fn class(reading: MemberReadBase<'_>) -> String {
    match reading {
        MemberReadBase::Value => "value".to_owned(),
        MemberReadBase::Root(root) => format!("root {root}"),
        MemberReadBase::Declined => "declined".to_owned(),
    }
}

#[test]
fn rule_74v_read_vectors_hold() {
    let rows = lean_rows();
    assert!(rows.len() >= 12, "{rows:?}");
    for (spelling, expected) in rows {
        assert_eq!(class(read(&spelling)), expected, "{spelling}");
    }
}

/// The spelling B01 admits through a root: the root is the only base.
#[test]
fn a_member_read_through_a_root_dereferences_the_root() {
    assert_eq!(read("p - > n"), MemberReadBase::Root("p"));
    assert_eq!(read("ctx - > next"), MemberReadBase::Root("ctx"));
}

/// A record value's member reads no pointer and keeps the `OpaqueValue`.
#[test]
fn a_record_value_member_dereferences_nothing() {
    assert_eq!(read("s . n"), MemberReadBase::Value);
}

/// Every other spelling declines, so the later routes decide it: none is
/// admitted by B01 today, and none may keep the obligation-free node.
#[test]
fn every_other_spelling_declines() {
    for spelling in [
        "p - > q - > n",
        "( p ) - > n",
        "p [ 0 ]",
        "* p",
        "& p - > n",
        "p - > n + 1",
        "p - > a . b",
        "s . p - > n",
        "f ( p ) - > n",
        "1 - > n",
        "p - >",
    ] {
        assert_eq!(read(spelling), MemberReadBase::Declined, "{spelling}");
    }
}

/// The node rule 74v lowers a `->` read to is family S's: for every spelling
/// it reads as a root's dereference, `scalar_read` makes that root the only
/// base and the only `->` root.
#[test]
fn the_root_is_family_s_base() {
    let names = Untracked {
        objects: BTreeSet::new(),
    };
    for spelling in ["p - > n", "ctx - > next", "value - > field"] {
        let MemberReadBase::Root(root) = read(spelling) else {
            panic!("{spelling}");
        };
        let family = scalar_read(&texts(spelling), &names).expect(spelling);
        assert_eq!(family.bases, vec![root.to_owned()], "{spelling}");
        assert_eq!(family.arrow_roots, vec![root.to_owned()], "{spelling}");
    }
}
