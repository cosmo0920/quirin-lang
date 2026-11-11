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

//! Card W43-RESULT-ALIAS-RETURNED: which refused shape the derived-result route
//! names for each body the card lists, read directly from the route's decision
//! (the census label is this same value). The end-to-end behaviour of each
//! shape, including its caller, is pinned in `tests/derived_result_returned.rs`.

use super::*;

use crate::legacy_c::tokenize;

const DERIVED: &str = include_str!("../fixtures/contracts/derived_result_generic.qv");

const DECLARATIONS: &str = "\
char *d_alloc(unsigned long n); \
void d_free(void *p); \
void d_consume(void *p); \
unsigned long d_len(const char *s); \
char *d_chr(const char *s, int c); \
void *d_copy(void *dst, const void *src, unsigned long n); \
char *g; ";

fn decide(body: &str, function: &str) -> Result<ResourceFunctionContract, DerivedRefusal> {
    let contracts = crate::contract_loader::load_resource_contract_sources(
        &std::collections::BTreeMap::from([(
            "derived_result_generic.qcore".to_owned(),
            DERIVED.to_owned(),
        )]),
    )
    .expect("the fixture loads");
    let tokens = tokenize(&format!("{DECLARATIONS}{body}"));
    infer_derived_result_summary(&tokens, function, &contracts)
}

fn refusal(body: &str, function: &str) -> DerivedRefusal {
    decide(body, function).expect_err(body)
}

#[test]
fn the_two_ledger_shapes_are_admitted() {
    assert_eq!(
        decide(
            "void *f(void *dest, const void *src, unsigned long size) \
             { return d_copy(dest, src, size); }",
            "f"
        )
        .map(|contract| contract.result),
        Ok(ResourceReturnMode::Derived {
            parameter: 0,
            extent: DerivedExtent::Exact,
            nullable: false,
            const_dropped: false,
        })
    );
    assert_eq!(
        decide(
            "char *f(char *heystack, char *needle) \
             { if (!heystack || !needle) { return ((void *)0); } return d_chr(heystack, 58); }",
            "f"
        )
        .map(|contract| contract.result),
        Ok(ResourceReturnMode::Derived {
            parameter: 0,
            extent: DerivedExtent::Interior,
            nullable: true,
            const_dropped: true,
        })
    );
}

#[test]
fn a_body_with_no_derived_return_is_not_a_candidate() {
    for body in [
        "unsigned long f(char *d) { return d_len(d); }",
        "void f(char *d) { d_copy(d, \"x\", 1); }",
    ] {
        assert_eq!(refusal(body, "f"), DerivedRefusal::NotCandidate, "{body}");
    }
    // Rule 36's cheap pointer-formal candidate still refuses without the
    // prepared typed AST. No source fact is guessed from these tokens.
    assert_eq!(
        refusal("char *f(char *d) { return d; }", "f"),
        DerivedRefusal::NoDerivedExit
    );
    // A derived call whose result no exit returns.
    for body in [
        "char *f(char *d) { d_copy(d, \"x\", 1); return 0; }",
        "char *f(char *d) { char *p; p = d_chr(d, 58); if (p) { return 0; } return 0; }",
    ] {
        assert_eq!(refusal(body, "f"), DerivedRefusal::NoDerivedExit, "{body}");
    }
}

#[test]
fn mixed_exits_are_named() {
    for body in [
        "char *f(char *d) { if (!d) { return d_alloc(8); } return d_copy(d, \"x\", 1); }",
        "char *f(char *d) { if (!d) { return g; } return d_copy(d, \"x\", 1); }",
        "char *f(char *d, int n) { if (n) { return d + 1; } return d_copy(d, \"x\", 1); }",
    ] {
        assert_eq!(refusal(body, "f"), DerivedRefusal::MixedExits, "{body}");
    }
}

#[test]
fn several_formals_are_named() {
    assert_eq!(
        refusal(
            "char *f(char *d, char *e) { if (!e) { return d_copy(d, \"x\", 1); } \
             return d_copy(e, \"x\", 1); }",
            "f"
        ),
        DerivedRefusal::SeveralFormals
    );
}

#[test]
fn a_released_or_consumed_base_is_named() {
    for body in [
        "char *f(char *d) { char *p; p = d_chr(d, 58); d_free(d); return p; }",
        "char *f(char *d) { char *p; p = d_chr(d, 58); d_consume(d); return p; }",
        // Returning the formal itself escapes it.
        "char *f(char *d, int n) { if (n) { return d; } return d_copy(d, \"x\", 1); }",
        // A nullable release on a path that then returns NULL.
        "char *f(char *d, int n) { if (n) { d_free(d); return 0; } return d_copy(d, \"x\", 1); }",
    ] {
        assert_eq!(refusal(body, "f"), DerivedRefusal::BaseCustody, "{body}");
    }
}

#[test]
fn a_non_formal_base_is_named() {
    for body in [
        "char *f(char *d) { char *b; b = d_alloc(8); d_len(d); return d_copy(b, \"x\", 1); }",
        "char *f(char *d) { d_len(d); return d_chr(\"abc\", 58); }",
        "char *f(char *d) { d_len(d); return d_copy(g, \"x\", 1); }",
    ] {
        assert_eq!(refusal(body, "f"), DerivedRefusal::NoFormalBase, "{body}");
    }
}

#[test]
fn a_mutated_local_copy_is_named() {
    // Integration of W43-INDEX-TRAVERSAL: a copy stepped forward by a literal
    // amount is an interior pointer strictly after the formal's start, so the
    // body's result is derived from that formal. Before traversal the
    // classifier dropped the copy's binding and the body was refused.
    for body in [
        "char *f(char *d) { char *q; q = d; q = q + 1; return d_copy(q, \"x\", 1); }",
        "char *f(char *d) { char *q; q = d; q++; return d_copy(q, \"x\", 1); }",
    ] {
        assert_eq!(
            decide(body, "f").expect(body).result,
            ResourceReturnMode::Derived {
                parameter: 0,
                extent: DerivedExtent::Interior,
                nullable: false,
                const_dropped: false,
            },
            "{body}"
        );
    }
    // A copy that may be stepped back to the formal's start (`Stepped::Any`)
    // is never published as derived: a caller's release of it could be the
    // formal's own allocation, so it must not be accused.
    for body in [
        "char *f(char *d) { char *q; q = d; q = q - 1; return d_copy(q, \"x\", 1); }",
        "char *f(char *d) { char *q; q = d; q--; return d_copy(q, \"x\", 1); }",
    ] {
        assert_eq!(refusal(body, "f"), DerivedRefusal::NoFormalBase, "{body}");
    }
    // An unmutated copy is the formal's own binding and is admitted.
    assert!(
        decide(
            "char *f(char *d) { char *q; q = d; return d_copy(q, \"x\", 1); }",
            "f"
        )
        .is_ok()
    );
}

#[test]
fn a_body_the_ordinary_analysis_refuses_is_named() {
    assert_eq!(
        refusal(
            "char *f(char *d) { char *b; b = d_alloc(8); return d_copy(d, \"x\", 1); }",
            "f"
        ),
        DerivedRefusal::BodyNotModeled
    );
}

#[test]
fn checked_indexed_child_lookup_keeps_borrowed_result_summary() {
    let fixture = include_str!("../fixtures/detach-lookup-membership/cjson_lookup_frame.h")
        .replace("\r\n", "\n");
    let fixture = if cfg!(windows) {
        fixture.replace(
            "typedef long unsigned int size_t;",
            "typedef unsigned long long size_t;",
        )
    } else {
        fixture
    };
    let mut sources = vec![fixture];
    if let Some(path) = std::env::var_os("QCORE_LOOKUP_PRODUCTION_INPUT") {
        sources.push(
            std::fs::read_to_string(path)
                .expect("explicit actual source must be readable")
                .replace("\r\n", "\n"),
        );
    }
    for source in &sources {
        let _source = crate::typed_access_path::enter_source(
            source,
            crate::legacy_c::CCompilerDialect::C11,
            None,
        );
        let result = infer_derived_result_summary(
            &tokenize(source),
            "get_array_item",
            &ResourceContracts::default(),
        );
        assert_eq!(
            result.map(|contract| contract.result),
            Ok(ResourceReturnMode::Derived {
                parameter: 0,
                extent: DerivedExtent::Borrowed,
                nullable: true,
                const_dropped: true,
            })
        );
    }
}

#[test]
fn lookup_without_retained_source_context_keeps_the_lexical_refusal() {
    let source = include_str!("../fixtures/detach-lookup-membership/cjson_lookup_frame.h");
    assert_eq!(
        infer_derived_result_summary(
            &tokenize(source),
            "get_array_item",
            &ResourceContracts::default()
        ),
        Err(DerivedRefusal::NoDerivedExit)
    );
}
