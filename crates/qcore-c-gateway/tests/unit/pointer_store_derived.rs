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

//! Card K3-POINTER-VALUED-STORES (rule 10v amended): the derived-value reading
//! against Clang, and its vectors bound to `QCore.PointerStoreDerived`.

use super::*;
use crate::legacy_c::CCompilerDialect;
use crate::typed_access_path::{StatementIndex, enter_source};

const RECORDS: &str = "struct S { char *name; char *end; void *state; struct S *self; \
char *slots[4]; char *elems; };\n\
struct E { char *base; long len; };\nstruct V { struct E *io; int n; };\nint f(void);\n";

/// The reading vectors shared with `proof/QCore/PointerStoreDerived.lean`
/// (`readingVectors`): a statement in `h` (formals `s`, `t` of `struct S *`,
/// `v` of `struct V *`, `b` of `char *`, integers `n`, `i`, `j`), and the
/// reading: `same-place`, `same-object` or `refused`.
pub(crate) const READING_VECTORS: &[(&str, &str)] = &[
    ("s->name = s->name + n", "same-place"),
    ("s->state = (char *) s->state + n", "same-place"),
    ("s->name = n + s->name", "same-place"),
    ("s->name = s->name - 1", "same-place"),
    ("s->end = s->name + n", "same-object"),
    ("s->slots[i] = s->slots[i] + 1", "same-place"),
    ("v->io[i].base = (char *) v->io[i].base + n", "same-place"),
    ("s->self->name = s->self->name + 1", "same-place"),
    ("s->name = b + n", "refused"),
    ("s->name = t->name + n", "refused"),
    ("s->self->name = s->name + 1", "refused"),
    ("s->end = s->self->name + 1", "refused"),
    ("s->slots[i] = s->slots[j] + 1", "same-object"),
    ("s->slots[f()] = s->slots[f()] + 1", "same-object"),
    ("s->name = (char *) (s->end - s->name)", "refused"),
    ("s->name = (char *) ((long) s->name + n)", "refused"),
];

fn assignments(node: &Value, out: &mut Vec<Value>) {
    if kind(node) == "BinaryOperator" && node["opcode"].as_str() == Some("=") {
        out.push(node.clone());
    }
    for child in children(node) {
        assignments(child, out);
    }
}

#[test]
fn every_reading_vector_is_clangs_reading() {
    if crate::clang_command().is_err() {
        return;
    }
    let body = READING_VECTORS
        .iter()
        .map(|(spelling, _)| format!("\n {spelling};"))
        .collect::<String>();
    let source = format!(
        "{RECORDS} void h(struct S *s, struct S *t, struct V *v, char *b, int n, int i, int j) \
         {{{body}\n }}"
    );
    let _scope = enter_source(&source, CCompilerDialect::C11, None);
    let index = StatementIndex::read("h").expect("typed function");
    let mut found = Vec::new();
    assignments(&index.function, &mut found);
    assert_eq!(found.len(), READING_VECTORS.len());
    for (node, (spelling, expected)) in found.iter().zip(READING_VECTORS) {
        let pair = children(node);
        let destination = crate::typed_place::read_place(&pair[0], &index.fields)
            .unwrap_or_else(|refusal| panic!("{spelling}: {refusal:?}"));
        let got = read(&pair[0], &pair[1], &destination, &index.fields)
            .map_or("refused", |derived| derived.base.code());
        assert_eq!(got, *expected, "{spelling}");
    }
}

/// The mechanical binding of `proof/QCore/PointerStoreDerived.lean`: its
/// reading vectors are the Rust ones, row by row.
#[test]
fn k3_lean_vectors_match_the_rust_table() {
    let lean = include_str!("../../../../proof/QCore/PointerStoreDerived.lean");
    let table = lean
        .split("def readingVectors : List (String × Reading) := [")
        .nth(1)
        .expect("readingVectors");
    let table = &table[..table.find("\n]").expect("end of vectors")];
    let rows = table
        .lines()
        .filter(|line| line.trim_start().starts_with("(\""))
        .map(|line| {
            let line = line.trim().trim_end_matches(',');
            let line = &line[2..line.len() - 1];
            let (spelling, reading) = line.rsplit_once("\", ").expect("spelling");
            let reading = match reading.trim() {
                ".samePlace" => "same-place",
                ".sameObject" => "same-object",
                ".refused" => "refused",
                other => panic!("unknown reading {other}"),
            };
            (spelling.to_owned(), reading)
        })
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), READING_VECTORS.len());
    for (row, vector) in rows.iter().zip(READING_VECTORS) {
        assert_eq!(row.0, vector.0);
        assert_eq!(row.1, vector.1, "{}", vector.0);
    }
}

#[test]
fn the_same_spelling_compares_declarations_members_and_literals() {
    if crate::clang_command().is_err() {
        return;
    }
    let source = format!(
        "{RECORDS} void h(struct S *s, struct S *t, int i) {{ s->slots[i] = 0; s->slots[i] = 0; \
         t->slots[i] = 0; s->slots[1] = 0; s->slots[1] = 0; s->slots[2] = 0; }}"
    );
    let _scope = enter_source(&source, CCompilerDialect::C11, None);
    let index = StatementIndex::read("h").expect("typed function");
    let mut found = Vec::new();
    assignments(&index.function, &mut found);
    let places = found
        .iter()
        .map(|node| children(node)[0].clone())
        .collect::<Vec<_>>();
    assert!(same_spelling(&places[0], &places[1]));
    assert!(!same_spelling(&places[0], &places[2]));
    assert!(!same_spelling(&places[0], &places[3]));
    assert!(same_spelling(&places[3], &places[4]));
    assert!(!same_spelling(&places[4], &places[5]));
}
