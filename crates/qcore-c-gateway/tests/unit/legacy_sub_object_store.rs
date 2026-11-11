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

use std::collections::BTreeSet;

use super::*;

/// Split like the importer's own tokenizer: identifiers, numbers and one
/// character per punctuation token, so `->` is `-` followed by `>`.
fn texts(source: &str) -> Vec<&str> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut cursor = 0;
    while cursor < bytes.len() {
        let byte = bytes[cursor];
        if byte.is_ascii_whitespace() {
            cursor += 1;
        } else if byte.is_ascii_alphanumeric() || byte == b'_' {
            let start = cursor;
            while cursor < bytes.len()
                && (bytes[cursor].is_ascii_alphanumeric() || bytes[cursor] == b'_')
            {
                cursor += 1;
            }
            tokens.push(&source[start..cursor]);
        } else {
            tokens.push(&source[cursor..cursor + 1]);
            cursor += 1;
        }
    }
    tokens
}

const RECORDS: &str = "struct q { int x; }; struct r { int count; char *name; \
                       struct r *next; void *data; int a[4]; struct q inner; }; \
                       typedef struct t_s { int n; char *s; } t_t; typedef struct { int n; } anon; \
                       typedef t_t t_alias;";

fn fixture(parameters: &str, body: &str) -> (TypedAccessModel, FunctionTypes) {
    let model = TypedAccessModel::read(&texts(RECORDS));
    let types = FunctionTypes::read(&model, &texts(parameters), &texts(body), &BTreeSet::new());
    (model, types)
}

fn read(parameters: &str, body: &str, left: &str) -> Result<Destination, Refusal> {
    let (model, types) = fixture(parameters, body);
    let formals = ["p", "recs", "s", "h"]
        .into_iter()
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    let roots = RecordRoots::read(&model, &texts(parameters), &texts(body), &formals);
    read_destination(&texts(left), &model, &types, &roots)
}

const PARAMETERS: &str = "struct r *p, struct r *recs, int i, char *s";

#[test]
fn each_admitted_destination_shape_names_its_member_place() {
    let body = "struct r l; struct r arr [ 2 ]; t_t * tp; t_alias ta;";
    let cases = [
        ("arr [ i ] . count", None, "arr[].count", MemberKind::Scalar),
        ("tp -> s", Some("tp"), "tp->s", MemberKind::Pointer),
        ("ta . n", None, "ta.n", MemberKind::Scalar),
        ("p -> count", Some("p"), "p->count", MemberKind::Scalar),
        ("( p ) -> name", Some("p"), "p->name", MemberKind::Pointer),
        (
            "recs [ i ] . next",
            Some("recs"),
            "recs[].next",
            MemberKind::Pointer,
        ),
        (
            "( recs ) [ i ] . data",
            Some("recs"),
            "recs[].data",
            MemberKind::Pointer,
        ),
        ("l . count", None, "l.count", MemberKind::Scalar),
        ("( & l ) -> name", None, "l.name", MemberKind::Pointer),
        ("( l ) . count", None, "l.count", MemberKind::Scalar),
    ];
    for (left, base, place, member) in cases {
        let destination = read(PARAMETERS, body, left).unwrap_or_else(|refusal| {
            panic!("{left} refused: {refusal}");
        });
        assert_eq!(
            destination,
            Destination {
                base: base.map(str::to_owned),
                place: place.to_owned(),
                member,
            },
            "{left}"
        );
    }
}

#[test]
fn every_other_destination_keeps_its_refusal() {
    let body = "struct r l; void *v; struct r **pp; struct r arr[2]; anon *td;";
    let refused = [
        ("( * pp ) -> count", "prefix"),
        ("( ( struct r * ) v ) -> count", "prefix"),
        ("v -> count", "root-untyped"),
        ("undeclared -> count", "root-untyped"),
        ("p -> next -> count", "chain"),
        ("p -> inner . x", "chain"),
        ("p -> a", "member"),
        ("p -> inner", "member"),
        ("p -> missing", "member"),
        ("p . count", "selector-mismatch"),
        ("l -> count", "selector-mismatch"),
        ("( & p ) -> count", "address-of-pointer"),
        ("l [ i ] . count", "subscript-of-object"),
        ("( & arr ) -> count", "address-of-pointer"),
        ("arr . count", "selector-mismatch"),
        ("td -> count", "root-untyped"),
        ("recs [ i ++ ] . count", "subscript-index"),
        ("recs [ i ] -> count", "selector-mismatch"),
        ("p -> count +", "chain"),
    ];
    for (left, reason) in refused {
        assert_eq!(read(PARAMETERS, body, left), Err(reason), "{left}");
    }
}

#[test]
fn values_are_read_by_the_members_kind() {
    let (model, types) = fixture(PARAMETERS, "struct r l; int n; char *t;");
    let scalar = |right: &str| read_value(&texts(right), MemberKind::Scalar, &model, &types);
    let pointer = |right: &str| read_value(&texts(right), MemberKind::Pointer, &model, &types);
    assert_eq!(scalar("n + 1"), Ok(ValueSyntax::Scalar(Vec::new())));
    assert_eq!(
        scalar("p -> count"),
        Ok(ValueSyntax::Scalar(vec!["p".to_owned()]))
    );
    assert_eq!(scalar("f ( n )"), Err("value-not-scalar"));
    assert_eq!(scalar("s"), Err("value-not-scalar"));
    assert_eq!(pointer("( ( void * ) 0 )"), Ok(ValueSyntax::Untracked));
    assert_eq!(pointer("0"), Ok(ValueSyntax::Untracked));
    assert_eq!(pointer("__qcore_literal__"), Ok(ValueSyntax::Untracked));
    assert_eq!(pointer("t"), Ok(ValueSyntax::Name("t".to_owned())));
    assert_eq!(
        pointer("( char * ) s"),
        Ok(ValueSyntax::Name("s".to_owned()))
    );
    assert_eq!(pointer("n"), Err("value-scalar-into-pointer"));
    assert_eq!(pointer("undeclared"), Err("value-undeclared"));
    assert_eq!(pointer("g ( s )"), Ok(ValueSyntax::Call(4)));
    assert_eq!(pointer("( char * ) g ( s )"), Ok(ValueSyntax::Call(4)));
    assert_eq!(pointer("g ( s ) + 1"), Err("value-shape"));
    assert_eq!(pointer("p -> next"), Err("value-shape"));
    assert_eq!(pointer("& l"), Err("value-shape"));
}
