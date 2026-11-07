//! Card OUT-PARAMETER-SCALAR-FRAME-PREMISE (rule 11r(7)), user decision
//! 2026-11-07 "A: effective-type".
//!
//! Rule 11r(6)'s per-class summary keeps a pointer out slot across a store
//! through a second out formal that Clang types as a non-character integer or
//! enumeration, under the listed premise `effective-type:` (C11 6.5p7), which
//! every row resting on it carries among its assumptions. A character or
//! floating store, a `memcpy` into the second formal, and a unit built with
//! `-fno-strict-aliasing` keep the refusal.
//!
//! Every test asserts each function's status, complete diagnostic vector and
//! carried premises. The fixtures use a generic `x_*` surface: `Safe` here is
//! `Safe` over synthetic trusted test assumptions and promotes nothing.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, CFrontendFlag, CPreprocessorOption, LegacyDiagnosticKind,
    LegacyProgramSummary, ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const PREMISE: &str = "effective-type: a non-character integer or enumeration store through \
                       another out formal does not modify a pointer out slot (C11 6.5p7)";

const CONTRACTS: &str = "\
module contract;\n\
extern fn x_alloc(n: usize) -> ptr<mut, u8>\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(n);\n\
           ensures normal resource_return_owned(nullable); };\n\
extern fn x_free(p: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_released(p, nullable);\n\
           ensures normal resource_return_none(); };\n\
extern fn x_copy(d: ptr<mut, u8>, s: ptr<const, u8>, n: usize) -> ptr<mut, u8>\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(d);\n\
           ensures normal resource_borrowed(s); ensures normal resource_borrowed(n);\n\
           ensures normal resource_return_alias(d, nonnull); };";

const DECLARATIONS: &str = "\
char *x_alloc(__SIZE_TYPE__ n); void x_free(void *p); \
void *x_copy(void *d, const void *s, __SIZE_TYPE__ n); \
typedef unsigned long x_size_t; \
typedef enum { X_OK = 0, X_BAD = 1 } x_err_t; \
struct x_chunk { char *data; x_size_t len; };";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "out_parameter_scalar_frame_premise.qv".to_owned(),
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
            eprintln!("skipping Clang-dependent effective-type test: {reason}");
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
        LegacyDiagnosticKind::Unsupported { description } => format!("unsupported:{description}"),
        other => format!("{other:?}"),
    }
}

type Outcome = (String, Vec<String>, Vec<String>);

/// Every function's status, sorted diagnostics and carried premise names.
fn outcomes(body: &str, no_strict_aliasing: bool) -> BTreeMap<String, Outcome> {
    let source = format!("{DECLARATIONS}\n{body}");
    let preprocessor_options = if no_strict_aliasing {
        vec![CPreprocessorOption::FrontendFlag(
            CFrontendFlag::NoStrictAliasing,
        )]
    } else {
        Vec::new()
    };
    let result = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            preprocessor_options,
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
            let premises = function
                .rule_obligations
                .premises
                .iter()
                .map(|carriage| format!("{}:{}", carriage.rule, carriage.name))
                .collect::<Vec<_>>();
            (
                name.clone(),
                (format!("{:?}", function.status), keys, premises),
            )
        })
        .collect()
}

fn premised() -> Vec<String> {
    vec![format!("11r:{PREMISE}")]
}

fn expect(label: &str, body: &str, no_strict_aliasing: bool, expected: &[(&str, Outcome)]) {
    if !clang_available() {
        return;
    }
    let measured = outcomes(body, no_strict_aliasing);
    let expected = expected
        .iter()
        .map(|(name, outcome)| ((*name).to_owned(), outcome.clone()))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(measured, expected, "{label}\n{body}");
}

fn safe(premises: Vec<String>) -> Outcome {
    ("Safe".to_owned(), Vec::new(), premises)
}

fn refused(slot: &str, extra: &[&str]) -> Outcome {
    let mut keys = extra
        .iter()
        .map(|key| (*key).to_owned())
        .collect::<Vec<_>>();
    keys.push(format!(
        "unsupported:store through `*{slot}` without an out-parameter contract"
    ));
    keys.sort();
    ("Unknown".to_owned(), keys, Vec::new())
}

fn caller_refused(variable: &str) -> Outcome {
    (
        "Unknown".to_owned(),
        vec![
            format!("unknown_alias:{variable}"),
            "unsupported:unmodeled call, aggregate or indirect access in branch condition"
                .to_owned(),
        ],
        Vec::new(),
    )
}

const CALLER: &str = "int w(void) {\n char *b;\n x_size_t n;\n if (r(&b, &n) != 0) return -1;\n \
                      x_free(b);\n return 0;\n }";

// -- The three chunkio shapes ---------------------------------------------------

#[test]
fn the_read_file_shape_certifies_under_the_premise() {
    // cio_utils_read_file: fresh on 0, unwritten on -1, the size after the slot.
    expect(
        "`*buf = data; *size = n;`",
        &format!(
            "static int r(char **buf, x_size_t *size) {{\n char *data;\n data = x_alloc(8);\n \
             if (!data) {{\n return -1;\n }} *buf = data;\n *size = 8;\n return 0;\n }} {CALLER}"
        ),
        false,
        &[("r", safe(premised())), ("w", safe(premised()))],
    );
}

#[test]
fn the_file_content_copy_shape_certifies_under_the_premise() {
    // cio_file_content_copy: a `void **` slot, a member-read size.
    expect(
        "`*out_buf = buf; *out_size = size;` after a member read",
        "static int r(struct x_chunk *ch, void **out_buf, x_size_t *out_size) {\n \
         char *buf;\n x_size_t size;\n size = ch->len;\n buf = x_alloc(size + 1);\n \
         if (!buf) {\n return -1;\n } x_copy(buf, ch->data, size);\n buf[size] = 0;\n \
         *out_buf = buf;\n *out_size = size;\n return 0;\n } \
         int w(struct x_chunk *ch) {\n void *b;\n x_size_t n;\n if (r(ch, &b, &n) != 0) return -1;\n \
         x_free(b);\n return 0;\n }",
        false,
        &[("r", safe(premised())), ("w", safe(premised()))],
    );
}

#[test]
fn the_memfs_content_copy_shape_certifies_under_the_premise() {
    // cio_memfs_content_copy: the size read through the chunk's member.
    expect(
        "`*out_buf = buf; *out_size = ch->len;`",
        "static int r(struct x_chunk *ch, void **out_buf, x_size_t *out_size) {\n \
         char *buf;\n buf = x_alloc(ch->len + 1);\n if (!buf) {\n return -1;\n } \
         *out_buf = buf;\n *out_size = ch->len;\n return 0;\n } \
         int w(struct x_chunk *ch) {\n void *b;\n x_size_t n;\n if (r(ch, &b, &n) != 0) return -1;\n \
         x_free(b);\n return 0;\n }",
        false,
        &[("r", safe(premised())), ("w", safe(premised()))],
    );
}

#[test]
fn an_enumeration_store_through_the_second_formal_is_covered() {
    expect(
        "`*buf = data; *err = X_OK;`",
        "static int r(char **buf, x_err_t *err) {\n char *data = x_alloc(8);\n \
         if (!data) {\n return -1;\n } *buf = data;\n *err = X_OK;\n return 0;\n } \
         int w(void) {\n char *b;\n x_err_t e;\n if (r(&b, &e) != 0) return -1;\n x_free(b);\n \
         return 0;\n }",
        false,
        &[("r", safe(premised())), ("w", safe(premised()))],
    );
}

// -- What keeps the refusal -----------------------------------------------------

#[test]
fn a_character_store_through_the_second_formal_stays_refused() {
    expect(
        "`*tag = 'x'` may alias anything (6.5p7's character exception)",
        "static int r(char **buf, char *tag) {\n char *data = x_alloc(8);\n \
         if (!data) {\n return -1;\n } *buf = data;\n *tag = 'x';\n return 0;\n } \
         int w(void) {\n char *b;\n char t;\n if (r(&b, &t) != 0) return -1;\n x_free(b);\n \
         return 0;\n }",
        false,
        &[("r", refused("buf", &[])), ("w", caller_refused("b"))],
    );
}

#[test]
fn a_floating_store_through_the_second_formal_stays_refused() {
    expect(
        "`*ratio = 0.5` is not an integer or enumeration store",
        "static int r(char **buf, double *ratio) {\n char *data = x_alloc(8);\n \
         if (!data) {\n return -1;\n } *buf = data;\n *ratio = 0.5;\n return 0;\n } \
         int w(void) {\n char *b;\n double q;\n if (r(&b, &q) != 0) return -1;\n x_free(b);\n \
         return 0;\n }",
        false,
        &[
            ("r", refused("buf", &["unknown_alias:ratio"])),
            ("w", caller_refused("b")),
        ],
    );
}

#[test]
fn a_copy_into_the_second_formal_is_not_read_under_the_premise() {
    // A call's write is untyped, so the premise never spares a slot for it.
    // This body is certified without the premise: a contracted call with
    // `Borrowed` positions writes no binding, so nothing poisons the slot.
    // That pre-existing reading is follow-up OUT-SLOT-CALL-WRITE-ALIASING.
    expect(
        "`x_copy(size, &n, sizeof n)` after the slot",
        "static int r(char **buf, x_size_t *size) {\n char *data = x_alloc(8);\n x_size_t n = 8;\n \
         if (!data) {\n return -1;\n } *buf = data;\n x_copy(size, &n, sizeof n);\n return 0;\n } \
         int w(void) {\n char *b;\n x_size_t m;\n if (r(&b, &m) != 0) return -1;\n x_free(b);\n \
         return 0;\n }",
        false,
        &[("r", safe(Vec::new())), ("w", safe(Vec::new()))],
    );
}

#[test]
fn a_line_with_another_write_is_not_keyed() {
    expect(
        "`*buf = data; *size = 8;` on one line: the store is not told apart",
        "static int r(char **buf, x_size_t *size) {\n char *data = x_alloc(8);\n \
         if (!data) {\n return -1;\n }\n *buf = data; *size = 8;\n return 0;\n }\n \
         int w(void) {\n char *b;\n x_size_t n;\n if (r(&b, &n) != 0) return -1;\n x_free(b);\n \
         return 0;\n }",
        false,
        &[("r", refused("buf", &[])), ("w", caller_refused("b"))],
    );
}

#[test]
fn a_unit_built_without_strict_aliasing_stays_refused() {
    expect(
        "`-fno-strict-aliasing` withdraws the premise",
        &format!(
            "static int r(char **buf, x_size_t *size) {{\n char *data;\n data = x_alloc(8);\n \
             if (!data) {{\n return -1;\n }} *buf = data;\n *size = 8;\n return 0;\n }} {CALLER}"
        ),
        true,
        &[("r", refused("buf", &[])), ("w", caller_refused("b"))],
    );
}

#[test]
fn the_other_refusals_of_rule_11r6_stand() {
    expect(
        "a store after release on the failure class, the scalar store beside it",
        "static int r(char **buf, x_size_t *size, int k) {\n char *d = x_alloc(4);\n \
         if (!d) {\n return -1;\n } if (k) {\n *buf = d;\n *size = 4;\n x_free(d);\n return -1;\n } \
         *buf = d;\n *size = 4;\n return 0;\n } \
         int w(void) {\n char *b;\n x_size_t n;\n if (r(&b, &n, 1) != 0) return -1;\n x_free(b);\n \
         return 0;\n }",
        false,
        &[
            ("r", refused("buf", &["unknown_alias:d"])),
            ("w", caller_refused("b")),
        ],
    );
}

#[test]
fn a_body_that_needs_no_premise_carries_none() {
    expect(
        "the scalar store before the slot: the slot is written last",
        "static int r(char **buf, x_size_t *size) {\n char *data = x_alloc(8);\n \
         if (!data) {\n return -1;\n } *size = 8;\n *buf = data;\n return 0;\n } \
         int w(void) {\n char *b;\n x_size_t n;\n if (r(&b, &n) != 0) return -1;\n x_free(b);\n \
         return 0;\n }",
        false,
        &[("r", safe(Vec::new())), ("w", safe(Vec::new()))],
    );
}
