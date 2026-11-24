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

//! Card MEMBER-EFFECT-STORES (rule 38k): the formulas, the decision table and
//! the lexical readings of `member_handoff.rs`.

use super::*;

const LEAN: &str = include_str!("../../../../proof/QCore/MemberHandoff.lean");

fn fused(text: &str) -> Vec<String> {
    let raw = text.split(' ').collect::<Vec<_>>();
    crate::legacy_c::container_publication::fuse(&raw)
        .into_iter()
        .map(|(text, _)| text)
        .collect()
}

#[test]
fn every_formula_is_a_validated_typed_rendering() {
    for (id, formula) in formulas() {
        qcore_frontend::premise_formula::guarded_proposition(id, formula)
            .unwrap_or_else(|error| panic!("{id}: {error}"));
    }
    assert_eq!(validated(), Ok(()));
    // Every id is distinct and prefixed; the premise is first.
    let ids = formulas().iter().map(|(id, _)| *id).collect::<Vec<_>>();
    assert_eq!(ids[0], PREMISE_ID);
    let mut sorted = ids.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), ids.len());
    assert!(ids.iter().all(|id| id.starts_with("mes_")));
}

#[test]
fn prose_is_refused_by_the_validator() {
    for prose in [
        "the old value of the slot is not this body's obligation",
        "\"#mes_premise_carried()\"",
        "#mes_unregistered_term()",
        // A registered term in the wrong sort.
        "#mes_in(#mes_store,#mes_parent)",
    ] {
        assert!(
            qcore_frontend::premise_formula::guarded_proposition("mes_probe", prose).is_err(),
            "{prose}"
        );
    }
}

#[test]
fn every_refusal_is_the_negation_of_an_admission_term() {
    // Each refusal names its own formula, and the statement prints it first.
    for reason in Reason::ALL {
        assert!(reason.statement().starts_with(reason.id()));
        assert!(reason.statement().ends_with(reason.formula()));
    }
}

#[test]
fn the_decision_vectors_hold_and_are_lean_s() {
    for (spelling, differs, expected) in DECISION_VECTORS {
        let decided = match decide(&vector_facts(differs)) {
            Ok(()) => "admit",
            Err(reason) => reason.code(),
        };
        assert_eq!(decided, *expected, "{spelling}");
    }
    let rows = DECISION_VECTORS
        .iter()
        .map(|(spelling, differs, expected)| {
            format!("  (\"{spelling}\", \"{differs}\", \"{expected}\")")
        })
        .collect::<Vec<_>>()
        .join(",\n");
    let table = format!("def decisionVectors : List (String × String × String) := [\n{rows}\n]");
    assert!(LEAN.contains(&table), "no Lean table\n{table}");
}

#[test]
fn the_formula_table_is_lean_s() {
    for (id, formula) in formulas() {
        let line = format!("  (\"{id}\", \"{formula}\"),");
        let last = format!("  (\"{id}\", \"{formula}\")");
        assert!(
            LEAN.lines().any(|l| l == line || l == last),
            "no Lean row for {id}"
        );
    }
}

#[test]
fn the_mes_block_is_lean_s() {
    let names = qcore_frontend::premise_formula::RENDERED_TERMS
        .iter()
        .filter(|term| term.block == "MES")
        .map(|term| format!("\"{}\"", term.name))
        .collect::<Vec<_>>();
    assert!(names.iter().all(|name| name.starts_with("\"mes_")));
    let line = format!("def mesTerms : List String := [{}]", names.join(", "));
    assert!(LEAN.lines().any(|l| l == line), "no `{line}`");
}

#[test]
fn every_admitting_fact_is_needed() {
    for value in [ValueKind::OwnedResult, ValueKind::LocalAcquisition] {
        assert_eq!(decide(&admitting(value)), Ok(()));
    }
    assert_eq!(
        decide(&admitting(ValueKind::Reallocated)),
        Err(Reason::ReallocSelf)
    );
    assert_eq!(
        decide(&admitting(ValueKind::Other)),
        Err(Reason::ValueShape)
    );
}

#[test]
fn loops_and_jumps() {
    let body =
        fused("int i ; for ( i = 0 ; i < n ; i ++ ) { p -> a = q_alloc ( ) ; } p -> b = 0 ;");
    let body = body.iter().map(String::as_str).collect::<Vec<_>>();
    let ranges = loop_ranges(&body).expect("no jump");
    let store = body.iter().position(|t| *t == "a").unwrap();
    assert!(ranges.iter().any(|&(o, c)| o <= store && store <= c));
    let after = body.iter().position(|t| *t == "b").unwrap();
    assert!(!ranges.iter().any(|&(o, c)| o <= after && after <= c));
    let jump = fused("goto out ; out : return ;");
    let jump = jump.iter().map(String::as_str).collect::<Vec<_>>();
    assert_eq!(loop_ranges(&jump), None);
    let unbraced = fused("while ( c ) p -> a = q_alloc ( ) ; return ;");
    let unbraced = unbraced.iter().map(String::as_str).collect::<Vec<_>>();
    let ranges = loop_ranges(&unbraced).expect("no jump");
    let store = unbraced.iter().position(|t| *t == "a").unwrap();
    assert!(ranges.iter().any(|&(o, c)| o <= store && store <= c));
}

#[test]
fn null_tests_are_truth_values_only() {
    for (text, start, end) in [
        ("if ( ! p -> a ) return ;", 3, 6),
        ("if ( p -> a == NULL ) return ;", 2, 5),
        ("if ( p -> a == ( ( void * ) 0 ) ) return ;", 2, 5),
        ("if ( NULL == p -> a ) return ;", 4, 7),
        ("if ( n > 0 && p -> a == NULL ) return ;", 6, 9),
        ("if ( p -> a ) return ;", 2, 5),
    ] {
        let body = fused(text);
        let body = body.iter().map(String::as_str).collect::<Vec<_>>();
        assert!(null_test(&body, start, end), "{text}");
    }
    for (text, start, end) in [
        ("use ( p -> a ) ;", 2, 5),
        ("x = p -> a ;", 2, 5),
        ("if ( p -> a == q ) return ;", 2, 5),
        ("return p -> a ;", 1, 4),
    ] {
        let body = fused(text);
        let body = body.iter().map(String::as_str).collect::<Vec<_>>();
        assert!(!null_test(&body, start, end), "{text}");
    }
}

#[test]
fn the_place_is_used_only_at_the_store() {
    let check = |text: &str| {
        let body = fused(text);
        let body = body.iter().map(String::as_str).collect::<Vec<_>>();
        let store_base = body.iter().position(|t| *t == "p").unwrap();
        let store_end = (store_base..body.len())
            .find(|&at| body[at] == ";")
            .unwrap();
        place_and_formal(
            &body,
            "p",
            &["a".to_owned(), "b".to_owned()],
            store_base,
            store_end,
        )
    };
    assert_eq!(check("p -> a . b = q_alloc ( ) ;"), Ok(()));
    assert_eq!(
        check("p -> a . b = q_alloc ( ) ; p -> a . c = 0 ; p -> n = 1 ;"),
        Ok(())
    );
    assert_eq!(
        check("p -> a . b = q_alloc ( ) ; if ( ! p -> a . b ) return ;"),
        Ok(())
    );
    assert_eq!(
        check("p -> a . b = q_alloc ( ) ; n = sizeof ( p -> a . b ) ;"),
        Ok(())
    );
    assert_eq!(
        check("p -> a . b = q_alloc ( ) ; use ( p -> a . b ) ;"),
        Err(Reason::PlaceUsed)
    );
    assert_eq!(
        check("p -> a . b = q_alloc ( ) ; p -> a . b = 0 ;"),
        Err(Reason::PlaceUsed)
    );
    assert_eq!(
        check("p -> a . b = q_alloc ( ) ; x = p -> a ;"),
        Err(Reason::PlaceUsed)
    );
    assert_eq!(
        check("p -> a . b = q_alloc ( ) ; p -> a . b -> n = 0 ;"),
        Err(Reason::PlaceUsed)
    );
    assert_eq!(
        check("p -> a . b = q_alloc ( ) ; q_take ( p ) ;"),
        Err(Reason::BorrowedUseAfter)
    );
    assert_eq!(
        check("p -> a . b = q_alloc ( ) ; if ( p == NULL ) return ;"),
        Ok(())
    );
}

#[test]
fn the_local_is_kept() {
    let check = |text: &str| {
        let body = fused(text);
        let body = body.iter().map(String::as_str).collect::<Vec<_>>();
        let store = body
            .windows(2)
            .position(|w| w == ["=", "l"])
            .map(|at| at + 1)
            .unwrap();
        let store_end = (store..body.len()).find(|&at| body[at] == ";").unwrap();
        local_kept(&body, "l", store, store_end)
    };
    assert_eq!(
        check(
            "struct S * l ; l = q_alloc ( ) ; if ( NULL == l ) return ; l -> m = 0 ; p -> a = l ; return ;"
        ),
        Ok(())
    );
    assert_eq!(
        check("struct S * l = q_alloc ( ) ; p -> a = l ; q_free ( l ) ;"),
        Err(Reason::LocalUsed)
    );
    assert_eq!(
        check("struct S * l = q_alloc ( ) ; x = l ; p -> a = l ;"),
        Err(Reason::LocalUsed)
    );
    assert_eq!(
        check("struct S * l = q_alloc ( ) ; l = q_alloc ( ) ; p -> a = l ;"),
        Err(Reason::LocalUsed)
    );
}

#[test]
fn indirect_accesses_after_the_store() {
    let check = |text: &str| {
        let body = fused(text);
        let body = body.iter().map(String::as_str).collect::<Vec<_>>();
        let store_end = body.iter().position(|t| *t == ";").unwrap();
        indirect_after(&body, "p", store_end)
    };
    assert!(!check("p -> a = q_alloc ( ) ; p -> n = 1 ; return ;"));
    assert!(check("p -> a = q_alloc ( ) ; p -> self -> a = 0 ;"));
    assert!(check("p -> a = q_alloc ( ) ; r -> a = 0 ;"));
    assert!(check("p -> a = q_alloc ( ) ; v [ 0 ] = 0 ;"));
    assert!(check("p -> a = q_alloc ( ) ; * v = 0 ;"));
    assert!(!check("p -> a = q_alloc ( ) ; n = sizeof ( r -> a ) ;"));
}
