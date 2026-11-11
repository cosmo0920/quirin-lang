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

//! Card CG10-COMMON-TYPED-PLACE (rule 10t): the place reader against Clang,
//! the decision table, and its mechanical binding to `QCore.TypedPlace`.

use super::*;
use crate::legacy_c::{CCompilerDialect, tokenize};
use crate::typed_access_path::{StatementIndex, enter_source};

fn step(code: &str) -> StepKind {
    match code {
        "arrow" => StepKind::Arrow,
        "dot" => StepKind::Dot,
        "index-pointer" => StepKind::Index {
            through_pointer: true,
        },
        "index-array" => StepKind::Index {
            through_pointer: false,
        },
        "deref-pointer" => StepKind::Deref {
            through_pointer: true,
        },
        "deref-array" => StepKind::Deref {
            through_pointer: false,
        },
        other => panic!("unknown step {other}"),
    }
}

fn root(code: &str) -> RootKind {
    match code {
        "pointer" => RootKind::Pointer,
        "record" => RootKind::Record,
        "array" => RootKind::Array,
        other => panic!("unknown root {other}"),
    }
}

#[test]
fn the_decision_vectors_hold() {
    for (spelling, root_code, steps, decision) in DECISION_VECTORS {
        let steps = steps.iter().map(|code| step(code)).collect::<Vec<_>>();
        assert_eq!(
            decide(root(root_code), &steps).code(),
            *decision,
            "{spelling}"
        );
    }
}

const RECORDS: &str = "struct I { int x; struct S *next; int arr[4]; }; \
struct S { int a; struct S *next; struct I in; int arr[4]; int *ints; int *ip; struct S *links[4]; };";

/// Every vector's spelling, read by Clang in a function where `s` is a
/// `struct S *`, `v` a `struct S **`, `o` a `struct S` and `t` a
/// `struct S[4]`, is exactly the vector's root and steps.
#[test]
fn every_vector_spelling_reads_as_its_steps() {
    if crate::clang_command().is_err() {
        return;
    }
    let mut body = String::new();
    for (index, (spelling, ..)) in DECISION_VECTORS.iter().enumerate() {
        body.push_str(&format!("\n y{index} = {spelling};"));
    }
    let declarations = (0..DECISION_VECTORS.len())
        .map(|index| format!("int y{index};"))
        .collect::<String>();
    let source = format!(
        "{RECORDS} int f(struct S *s, struct S **v, int i) {{ struct S o; struct S t[4]; {declarations}{body}\n return 0; }}"
    );
    let _scope = enter_source(&source, CCompilerDialect::C11, None);
    let index = StatementIndex::read("f").expect("typed function");
    for (line, (spelling, root_code, steps, _)) in DECISION_VECTORS.iter().enumerate() {
        let tokens = tokenize(spelling);
        let texts = tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        let node = index
            .expression_node(&texts, line + 2)
            .unwrap_or_else(|| panic!("{spelling}: no node"));
        let place = read_place(node, &index.fields).unwrap_or_else(|r| panic!("{spelling}: {r:?}"));
        assert_eq!(place.root.code(), *root_code, "{spelling}");
        assert_eq!(
            place
                .steps
                .iter()
                .map(|step| step.kind.code())
                .collect::<Vec<_>>(),
            *steps,
            "{spelling}"
        );
        assert!(
            place
                .steps
                .iter()
                .all(|step| !step.ty.is_empty() && !step.view.is_empty())
        );
        assert!(root_identity(&index.function, &place), "{spelling}");
    }
}

/// The mechanical binding of `proof/QCore/TypedPlace.lean`: its decision
/// vectors are the Rust ones, spelling by spelling.
#[test]
fn cg10_lean_vectors_match_the_rust_tables() {
    let lean = include_str!("../../../../proof/QCore/TypedPlace.lean");
    let table = lean
        .split("def decisionVectors : List (String × RootKind × List Step × Decision) := [")
        .nth(1)
        .expect("decisionVectors");
    let table = &table[..table.find("\n]").expect("end of vectors")];
    let lean_code = |name: &str| -> String {
        let name = name.trim().trim_start_matches('.');
        let mut code = String::new();
        for character in name.chars() {
            if character.is_ascii_uppercase() {
                code.push('-');
                code.push(character.to_ascii_lowercase());
            } else {
                code.push(character);
            }
        }
        code
    };
    let rows = table
        .lines()
        .filter(|line| line.trim_start().starts_with("(\""))
        .map(|line| {
            let line = line.trim().trim_end_matches(',');
            let (spelling, rest) = line[2..].split_once('"').expect("spelling");
            let rest = rest.trim_start_matches(',').trim();
            let (root_name, rest) = rest.split_once(',').expect("root");
            let (steps, decision) = rest
                .split_once('[')
                .expect("steps")
                .1
                .split_once(']')
                .expect("steps end");
            let steps = steps
                .split(',')
                .filter(|step| !step.trim().is_empty())
                .map(lean_code)
                .collect::<Vec<_>>();
            let decision = lean_code(
                decision
                    .trim()
                    .trim_start_matches(',')
                    .trim_end_matches(')'),
            );
            (spelling.to_owned(), lean_code(root_name), steps, decision)
        })
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), DECISION_VECTORS.len());
    for ((spelling, root_code, steps, decision), vector) in rows.iter().zip(DECISION_VECTORS) {
        assert_eq!(spelling, vector.0);
        assert_eq!(root_code, vector.1, "{spelling}");
        assert_eq!(steps, vector.2, "{spelling}");
        assert_eq!(decision, vector.3, "{spelling}");
    }
}

fn place_of(source: &str, spelling: &str, line: usize) -> Result<TrackedPlace, Refusal> {
    let _scope = enter_source(source, CCompilerDialect::C11, None);
    let index = StatementIndex::read("f").expect("typed function");
    let tokens = tokenize(spelling);
    let texts = tokens
        .iter()
        .map(|token| token.text.as_str())
        .collect::<Vec<_>>();
    let node = index.expression_node(&texts, line).expect("node");
    tracked_chain(node, &index.fields, Some(&index.function), None)
}

#[test]
fn a_tracked_place_lowers_to_the_chain_obligation() {
    if crate::clang_command().is_err() {
        return;
    }
    let source = format!(
        "{RECORDS} int f(struct S *s, int i) {{\n int y = s->in.x + s->ints[i] + s->next->in.x;\n return y; }}"
    );
    let root_only = place_of(&source, "s->in.x", 2).expect("root only");
    assert!(root_only.chain.links.is_empty() && !root_only.chain.root_never_bound);
    assert!(!root_only.through_element);
    let member = place_of(&source, "s->ints[i]", 2).expect("direct member");
    assert_eq!(member.chain.links, ["ints"]);
    assert!(member.through_element);
    let deeper = place_of(&source, "s->next->in.x", 2).expect("direct member");
    assert_eq!(deeper.chain.links, ["next"]);
    assert_eq!(deeper.chain.member, "in.x");
}

#[test]
fn deeper_element_record_array_and_shadowed_roots_refuse() {
    if crate::clang_command().is_err() {
        return;
    }
    let source = format!(
        "{RECORDS} int f(struct S *s, struct S **v, int i) {{ struct S o; struct S t[4];\n int y = s->in.next->a + v[i]->a + o.in.x + t[i].a;\n return y; }}"
    );
    for (spelling, decision) in [
        ("s->in.next->a", Decision::DeeperPlace),
        ("v[i]->a", Decision::ElementPlace),
        ("o.in.x", Decision::RecordRoot),
        ("t[i].a", Decision::ArrayRoot),
    ] {
        assert_eq!(
            place_of(&source, spelling, 2).unwrap_err(),
            Refusal::Decision(decision),
            "{spelling}"
        );
    }
    let shadowed = format!(
        "{RECORDS} int f(struct S *s) {{ {{ struct S *s = 0; (void)s; }}\n int y = s->in.x;\n return y; }}"
    );
    assert_eq!(
        place_of(&shadowed, "s->in.x", 2).unwrap_err(),
        Refusal::Identity
    );
    let global = format!("{RECORDS} struct S *g; int f(void) {{\n int y = g->in.x;\n return y; }}");
    assert_eq!(
        place_of(&global, "g->in.x", 2).unwrap_err(),
        Refusal::Identity
    );
    let static_local =
        format!("{RECORDS} int f(void) {{ static struct S *p;\n int y = p->in.x;\n return y; }}");
    assert_eq!(
        place_of(&static_local, "p->in.x", 2).unwrap_err(),
        Refusal::Identity
    );
}

#[test]
fn casts_calls_unions_bitfields_and_volatile_bases_refuse() {
    if crate::clang_command().is_err() {
        return;
    }
    let source = "struct I { int x; }; struct S { struct I in; unsigned bf : 3; union { int u; } un; }; \
struct S *get(void); int f(struct S *s, void *p, volatile struct S *w) {\n \
int y = ((struct S *)p)->in.x + get()->in.x + s->un.u + s->bf + w->in.x;\n return y; }";
    for spelling in ["((struct S *)p)->in.x", "get()->in.x", "w->in.x"] {
        assert!(place_of(source, spelling, 2).is_err(), "{spelling}");
    }
    // Card CG13 flip (rule 10w): a union member and a bit-field are steps of
    // a root-only place; `union_bitfield::decide` decides their access in
    // the reader that asks. BEFORE: refused (`Refusal::Field`).
    for spelling in ["s->un.u", "s->bf"] {
        let place = place_of(source, spelling, 2).unwrap_or_else(|r| panic!("{spelling}: {r:?}"));
        assert!(place.chain.links.is_empty(), "{spelling}");
    }
}

/// Card CG13 (rule 10w): a pointer loaded from a union member's storage is
/// never a link, so a hop after a union step refuses on a tracked root.
#[test]
fn a_hop_after_a_union_step_refuses() {
    if crate::clang_command().is_err() {
        return;
    }
    let source = "struct I { int x; }; union U { struct I *ip; long n; }; \
struct S { union U u; union U *pu; }; int f(struct S *s, union U *v) {\n \
int y = s->u.ip->x + v->ip->x + s->pu->n;\n return y; }";
    assert_eq!(
        place_of(source, "s->u.ip->x", 2).unwrap_err(),
        Refusal::Decision(Decision::DeeperPlace)
    );
    assert_eq!(
        place_of(source, "v->ip->x", 2).unwrap_err(),
        Refusal::UnionHop
    );
    // A union reached through an ordinary member pointer is a direct member.
    let place = place_of(source, "s->pu->n", 2).expect("direct member");
    assert_eq!(place.chain.links, ["pu"]);
}

#[test]
fn occurrences_count_only_on_the_run_line_and_need_the_stream() {
    let tokens = tokenize("p->x = 1; p->x = 1;\np->x = 1;");
    let width = tokenize("p->x = 1").len();
    let second = tokens[width + 1..2 * width + 1].to_vec();
    let found = occurrence(&tokens, Some((width + 1, width)), &second).expect("the statement");
    assert_eq!((found.ordinal, found.count), (1, 2));
    let third = tokens[2 * width + 2..3 * width + 2].to_vec();
    let found = occurrence(&tokens, Some((2 * width + 2, width)), &third).expect("the statement");
    assert_eq!((found.ordinal, found.count), (0, 1));
    // No origin, another origin, or a sub-run: no occurrence.
    assert!(occurrence(&tokens, None, &second).is_none());
    assert!(occurrence(&tokens, Some((2 * width + 2, width)), &second).is_none());
    assert!(occurrence(&tokens, Some((width + 1, width)), &second[..width - 1]).is_none());
}

/// I04: two identical statements on one line are told apart by the
/// gateway's own occurrence; a copied (synthetic) run keeps the refusal.
#[test]
fn identical_statements_on_one_line_are_told_apart_by_occurrence() {
    if crate::clang_command().is_err() {
        return;
    }
    let source = "struct S { int x; }; int f(struct S *p) { p->x = 1; p->x = 1; return 0; }";
    let _scope = enter_source(source, CCompilerDialect::C11, None);
    let index = StatementIndex::read("f").expect("typed function");
    let tokens = tokenize(source);
    let wanted = tokenize("p->x = 1");
    let width = wanted.len();
    let starts = (0..tokens.len())
        .filter(|&at| {
            tokens[at..]
                .iter()
                .take(width)
                .map(|token| token.text.as_str())
                .eq(wanted.iter().map(|token| token.text.as_str()))
        })
        .collect::<Vec<_>>();
    assert_eq!(starts.len(), 2);
    for &start in &starts {
        let run = &tokens[start..start + width];
        let texts = run
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        assert!(index.statement(&texts, 1, |_| true).is_none());
        let origin = Some((start, width));
        assert!(
            index
                .statement_with(
                    &texts,
                    1,
                    &|| occurrence(&tokens, origin, run),
                    |_| true,
                    None
                )
                .is_some()
        );
        assert!(
            index
                .statement_with(
                    &texts,
                    1,
                    &|| occurrence(&tokens, None, run),
                    |_| true,
                    None
                )
                .is_none()
        );
    }
    // A rewritten line (its tokens differ from the analysed source's) keeps
    // the refusal.
    let mut rewritten = tokens.clone();
    let at = rewritten
        .iter()
        .position(|token| token.text == "return")
        .unwrap();
    rewritten[at].text = "held".into();
    let run = &rewritten[starts[0]..starts[0] + width];
    let texts = run
        .iter()
        .map(|token| token.text.as_str())
        .collect::<Vec<_>>();
    assert!(
        index
            .statement_with(
                &texts,
                1,
                &|| occurrence(&rewritten, Some((starts[0], width)), run),
                |_| true,
                None
            )
            .is_none()
    );
}
