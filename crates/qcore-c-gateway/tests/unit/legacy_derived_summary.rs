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
        "char *f(char *d) { return d; }",
        "void f(char *d) { d_copy(d, \"x\", 1); }",
    ] {
        assert_eq!(refusal(body, "f"), DerivedRefusal::NotCandidate, "{body}");
    }
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
