//! Card OUT-PARAMETER-BODY-SUMMARY — out-parameter stores read off the body.
//!
//! Part 1 (rule 10k(5)): a `*B = V` store the importer named for an out slot
//! is a scalar frame write when the AST statement reader reads it as one (the
//! owner-aware enum arm included). A pointer-valued store keeps the refusal.
//!
//! Part 2 (rule 11r(6)): a return-code class that leaves an out slot
//! unwritten on every exit is that class's `resource_borrowed` beside
//! `resource_out` in another class, validated with the slot sanctioned. A
//! slot written on some exit of the class, holding a released site, or shared
//! by two slots keeps the refusal.
//!
//! Every test asserts the **complete** diagnostic vector of each named
//! function with its status, so a rule that dropped a pre-existing leak or
//! refusal while adding its own answer fails. The fixtures use a generic
//! `x_*` surface: `Safe` here is `Safe` over synthetic trusted test
//! assumptions and promotes nothing.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11, check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const CONTRACTS: &str = "\
module contract;\n\
extern fn x_alloc(n: usize) -> ptr<mut, u8>\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(n);\n\
           ensures normal resource_return_owned(nullable); };\n\
extern fn x_free(p: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_released(p, nullable);\n\
           ensures normal resource_return_none(); };";

const DECLARATIONS: &str = "\
char *x_alloc(__SIZE_TYPE__ n); void x_free(void *p); \
typedef enum { X_ERR_NONE = 0, X_ERR_BAD = -1, X_ERR_GONE = -2 } x_err_t; \
enum x_kind { X_ONE = 1, X_TWO = 2 }; \
typedef unsigned long x_size_t; \
struct x_t { x_err_t err; int n; };";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "out_parameter_body_summary.qv".to_owned(),
        CONTRACTS.to_owned(),
    )]))
    .expect("the synthetic contracts load")
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent out-parameter test: {reason}");
            false
        }
    }
}

fn program(result: &UnrestrictedLegacyResult) -> &LegacyProgramSummary {
    match result {
        UnrestrictedLegacyResult::Safe(certificate) => &certificate.summary,
        UnrestrictedLegacyResult::Unsafe(summary) => summary,
        UnrestrictedLegacyResult::Unknown {
            partial: Some(summary),
            ..
        } => summary,
        other => panic!("no projection at all: {other:?}"),
    }
}

fn key(kind: &LegacyDiagnosticKind) -> String {
    match kind {
        LegacyDiagnosticKind::UnknownAlias { variable } => format!("unknown_alias:{variable}"),
        LegacyDiagnosticKind::Leak { .. } => "leak".to_owned(),
        LegacyDiagnosticKind::UseAfterRelease { .. } => "use_after_release".to_owned(),
        LegacyDiagnosticKind::Unsupported { description } => format!("unsupported:{description}"),
        other => format!("{other:?}"),
    }
}

/// Every function's status and complete, sorted diagnostic vector.
fn outcomes(body: &str) -> BTreeMap<String, (String, Vec<String>)> {
    let source = format!("{DECLARATIONS}\n{body}");
    let result = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        "w",
        &contracts(),
    )
    .unwrap_or_else(|error| panic!("fixture rejected by Clang: {error:?}\n{source}"));
    if let UnrestrictedLegacyResult::Safe(certificate) = &result {
        check_legacy_safety_certificate(certificate)
            .expect("a Safe result carries a certificate that rechecks");
    }
    program(&result)
        .functions
        .iter()
        .map(|(name, function)| {
            let mut keys = function
                .diagnostics
                .iter()
                .map(|diagnostic| key(&diagnostic.kind))
                .collect::<Vec<_>>();
            keys.sort();
            keys.dedup();
            (name.clone(), (format!("{:?}", function.status), keys))
        })
        .collect()
}

/// `expected` lists `(function, status, diagnostics)` for every function.
fn expect(label: &str, body: &str, expected: &[(&str, &str, &[&str])]) {
    if !clang_available() {
        return;
    }
    let measured = outcomes(body);
    let expected = expected
        .iter()
        .map(|(name, status, keys)| {
            (
                (*name).to_owned(),
                (
                    (*status).to_owned(),
                    keys.iter().map(|key| (*key).to_owned()).collect::<Vec<_>>(),
                ),
            )
        })
        .collect::<BTreeMap<_, _>>();
    assert_eq!(measured, expected, "{label}\n{body}");
}

const READ: &str = "static int r(char **buf) { char *d = x_alloc(4); \
                    if (!d) { return -1; } *buf = d; return 0; }";

// -- Part 1: scalar stores named for an out slot ------------------------------

#[test]
fn an_enumerator_store_through_a_typedef_enum_formal_is_a_frame_write() {
    expect(
        "`*errp = X_ERR_BAD` through a typedef-defined enumeration",
        "void w(x_err_t *errp) { *errp = X_ERR_BAD; }",
        &[("w", "Safe", &[])],
    );
    expect(
        "guarded by the formal's null test",
        "void w(x_err_t *errp) { if (errp) *errp = X_ERR_BAD; }",
        &[("w", "Safe", &[])],
    );
    expect(
        "a formal of the enumeration type as the value",
        "void w(x_err_t *errp, x_err_t e) { *errp = e; }",
        &[("w", "Safe", &[])],
    );
    expect(
        "a tagged enumeration",
        "void w(enum x_kind *k) { *k = X_TWO; }",
        &[("w", "Safe", &[])],
    );
}

#[test]
fn the_toppar_get_avail_shape_certifies() {
    expect(
        "an error code set on each failure exit, a member read beside it",
        "void *w(struct x_t *t, x_err_t *errp) { \
         if (t->n == 0) { *errp = X_ERR_BAD; return 0; } \
         if (t->n < 0) { *errp = t->err; return 0; } return t; }",
        &[("w", "Safe", &[])],
    );
}

#[test]
fn a_caller_of_an_enumerator_writer_reads_a_borrowed_summary() {
    expect(
        "the writer's summary is all `Borrowed`",
        "static void s(x_err_t *errp) { *errp = X_ERR_BAD; } \
         int w(void) { x_err_t e; s(&e); return e == X_ERR_BAD; }",
        &[("s", "Safe", &[]), ("w", "Safe", &[])],
    );
}

#[test]
fn a_scalar_store_keeps_its_dereference_obligation() {
    expect(
        "a store through a released base is a use-after-release candidate",
        "void w(void) { x_err_t *e = (x_err_t *) x_alloc(4); if (!e) return; \
         x_free(e); *e = X_ERR_BAD; }",
        &[("w", "Unsafe", &["unknown_alias:e", "use_after_release"])],
    );
    expect(
        "a store through a live base",
        "void w(void) { x_err_t *e = (x_err_t *) x_alloc(4); if (!e) return; \
         *e = X_ERR_BAD; x_free(e); }",
        &[("w", "Safe", &[])],
    );
}

#[test]
fn a_pointer_valued_store_through_a_formal_stays_refused() {
    expect(
        "`*pp = 0` writes a pointer object",
        "void w(char **pp) { *pp = 0; }",
        &[(
            "w",
            "Unknown",
            &["unsupported:store through `*pp` without an out-parameter contract"],
        )],
    );
    expect(
        "`*pp = q` copies a pointer",
        "void w(char **pp, char *q) { *pp = q; }",
        &[(
            "w",
            "Unknown",
            &["unsupported:store through `*pp` without an out-parameter contract"],
        )],
    );
}

// -- Part 2: unwritten return-code classes ------------------------------------

#[test]
fn a_slot_fresh_on_success_and_unwritten_on_failure_certifies_both_sides() {
    expect(
        "the callee's own row and a caller that tests the code",
        &format!(
            "{READ} int w(void) {{ char *b; if (r(&b) != 0) return -1; x_free(b); return 0; }}"
        ),
        &[("r", "Safe", &[]), ("w", "Safe", &[])],
    );
}

#[test]
fn a_caller_that_leaks_the_delivered_resource_is_unsafe() {
    expect(
        "the success class delivers a fresh resource the caller drops",
        &format!("{READ} int w(void) {{ char *b; if (r(&b) != 0) return -1; return 0; }}"),
        &[("r", "Safe", &[]), ("w", "Unsafe", &["leak"])],
    );
    expect(
        "the success class overwrites the caller's live resource",
        &format!(
            "{READ} int w(void) {{ char *b = x_alloc(4); if (!b) return -1; \
             if (r(&b) != 0) {{ x_free(b); return -1; }} x_free(b); return 0; }}"
        ),
        &[
            ("r", "Safe", &[]),
            ("w", "Unsafe", &["leak", "unknown_alias:b"]),
        ],
    );
}

#[test]
fn the_unwritten_class_hands_the_caller_an_untracked_value() {
    expect(
        "a release of the slot's variable on the failure class refuses",
        &format!(
            "{READ} int w(void) {{ char *b = 0; if (r(&b) != 0) {{ x_free(b); return -1; }} \
             x_free(b); return 0; }}"
        ),
        &[("r", "Safe", &[]), ("w", "Unknown", &["unknown_alias:b"])],
    );
    expect(
        "an untested call keeps the join",
        &format!("{READ} int w(void) {{ char *b; r(&b); return 0; }}"),
        &[
            ("r", "Safe", &[]),
            (
                "w",
                "Unknown",
                &[
                    "unsupported:call to `r` whose return-code disposition needs its returned \
                   value tested against a constant where it stands (relational-untested)",
                ],
            ),
        ],
    );
}

#[test]
fn a_slot_written_on_some_exit_of_the_failure_class_stays_refused() {
    expect(
        "the failure class nulls the slot on one exit and leaves it on another",
        "static int r(char **buf, int k) { char *d = x_alloc(4); if (!d) { return -1; } \
         if (k) { *buf = 0; x_free(d); return -1; } *buf = d; return 0; } \
         int w(void) { char *b; if (r(&b, 1) != 0) return -1; x_free(b); return 0; }",
        &[
            (
                "r",
                "Unknown",
                &[
                    "unknown_alias:d",
                    "unsupported:store through `*buf` without an out-parameter contract",
                ],
            ),
            (
                "w",
                "Unknown",
                &[
                    "unknown_alias:b",
                    "unsupported:unmodeled call, aggregate or indirect access in branch condition",
                ],
            ),
        ],
    );
}

#[test]
fn a_store_after_release_stays_refused() {
    expect(
        "the failure class stores the resource it then releases",
        "static int r(char **buf, int k) { char *d = x_alloc(4); if (!d) { return -1; } \
         if (k) { *buf = d; x_free(d); return -1; } *buf = d; return 0; } \
         int w(void) { char *b; if (r(&b, 1) != 0) return -1; x_free(b); return 0; }",
        &[
            (
                "r",
                "Unknown",
                &[
                    "unknown_alias:d",
                    "unsupported:store through `*buf` without an out-parameter contract",
                ],
            ),
            (
                "w",
                "Unknown",
                &[
                    "unknown_alias:b",
                    "unsupported:unmodeled call, aggregate or indirect access in branch condition",
                ],
            ),
        ],
    );
}

#[test]
fn two_slots_receiving_one_site_stay_refused() {
    expect(
        "aliasing between out formals",
        "static int r(char **a, char **b) { char *d = x_alloc(4); if (!d) { return -1; } \
         *a = d; *b = d; return 0; } \
         int w(void) { char *p; char *q; if (r(&p, &q) != 0) return -1; x_free(p); return 0; }",
        &[
            (
                "r",
                "Unknown",
                &[
                    "unsupported:store through `*a` without an out-parameter contract",
                    "unsupported:store through `*b` without an out-parameter contract",
                ],
            ),
            (
                "w",
                "Unknown",
                &[
                    "unknown_alias:p",
                    "unsupported:unmodeled call, aggregate or indirect access in branch condition",
                ],
            ),
        ],
    );
}

#[test]
fn a_scalar_store_through_a_second_formal_after_the_slot_stays_refused() {
    // The chunkio shape (`*buf = data; *size = n;`): the scalar store's frame
    // may write the first slot's object unless the formals are known not to
    // alias, so the success class does not validate (the card's stop).
    expect(
        "`*size = 4` after `*buf = d`",
        "static int r(char **buf, x_size_t *size) { char *d = x_alloc(4); \
         if (!d) { return -1; } *buf = d; *size = 4; return 0; } \
         int w(void) { char *b; x_size_t n; if (r(&b, &n) != 0) return -1; x_free(b); \
         return 0; }",
        &[
            (
                "r",
                "Unknown",
                &["unsupported:store through `*buf` without an out-parameter contract"],
            ),
            (
                "w",
                "Unknown",
                &[
                    "unknown_alias:b",
                    "unsupported:unmodeled call, aggregate or indirect access in branch condition",
                ],
            ),
        ],
    );
}

#[test]
fn a_slot_every_class_writes_keeps_the_ordinary_summary() {
    expect(
        "NULL on failure, fresh on success: one ordinary `Out(nullable)`",
        "static int r(char **buf) { char *d = x_alloc(4); if (!d) { *buf = 0; return -1; } \
         *buf = d; return 0; } void w(void) { char *b; r(&b); x_free(b); }",
        &[("r", "Safe", &[]), ("w", "Safe", &[])],
    );
}
