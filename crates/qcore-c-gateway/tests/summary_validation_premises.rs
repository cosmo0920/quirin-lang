//! Card W47-SUMMARY-VALIDATION-PROTECTED-FORMALS — a W4 summary is validated
//! on the typed-root premises its function's reported row reads.
//!
//! PROBE: exploratory outcomes, replaced by the pinned controls.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11, check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const CONTRACTS: &str = "\
module contract;\n\
extern fn c_make(n: i32) -> ptr<mut, u8>\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(n);\n\
           ensures normal resource_return_owned(nullable); };\n\
extern fn c_drop(b: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_released(b, nonnull);\n\
           ensures normal resource_return_none(); };\n\
extern fn c_peek(l: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(l);\n\
           ensures normal resource_return_none(); };\n\
extern fn c_same(a: ptr<const, u8>, b: ptr<const, u8>) -> i32\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(a);\n\
           ensures normal resource_borrowed(b); ensures normal resource_return_none(); };";

const DECLARATIONS: &str = "\
struct c_leaf { int weight; char *name; }; \
struct c_box { int flag; struct c_leaf *leaf; }; \
struct c_tp { char *topic; int partition; }; \
struct c_box *c_make(int n); \
void c_drop(struct c_box *b); \
void c_peek(struct c_leaf *l); \
int c_same(const char *a, const char *b);";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "summary_validation_premises.qv".to_owned(),
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
            eprintln!("skipping Clang-dependent summary-validation test: {reason}");
            false
        }
    }
}

fn analyze(source: &str) -> UnrestrictedLegacyResult {
    let source = format!("{DECLARATIONS}\n{source}");
    analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        "w",
        &contracts(),
    )
    .unwrap_or_else(|error| panic!("fixture rejected by Clang: {error:?}\n{source}"))
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
        LegacyDiagnosticKind::DoubleRelease { .. } => "double_release".to_owned(),
        LegacyDiagnosticKind::UseAfterRelease { .. } => "use_after_release".to_owned(),
        LegacyDiagnosticKind::Unsupported { description } => format!("unsupported:{description}"),
        other => format!("{other:?}"),
    }
}

/// Every row's status and complete, sorted, de-duplicated diagnostic keys.
fn rows(source: &str) -> BTreeMap<String, (String, Vec<String>)> {
    let result = analyze(source);
    if let UnrestrictedLegacyResult::Safe(certificate) = &result {
        check_legacy_safety_certificate(certificate)
            .expect("a Safe result carries a certificate that rechecks");
        for record in &certificate.summaries {
            eprintln!("  SUMMARY {record:?}");
        }
    }
    program(&result)
        .functions
        .iter()
        .map(|(name, row)| {
            let mut keys = row
                .diagnostics
                .iter()
                .map(|diagnostic| key(&diagnostic.kind))
                .collect::<Vec<_>>();
            keys.sort();
            keys.dedup();
            (name.clone(), (format!("{:?}", row.status), keys))
        })
        .collect()
}

#[test]
fn probe() {
    if !clang_available() {
        return;
    }
    for source in [
        // cmp / cmp_opaque
        "int cmp(const void *_a, const void *_b) { \
         const struct c_tp *a = _a; const struct c_tp *b = _b; \
         int r = c_same(a->topic, b->topic); if (r) return r; \
         else return (int)((a->partition) < (b->partition) ? -1 : ((a->partition) > (b->partition))); } \
         static int w(const void *_a, const void *_b, void *opaque) { return cmp(_a, _b); }",
        // one-formal void* getter
        "int get(const void *_p) { const struct c_leaf *p = _p; return p->weight; } \
         int w(const void *q) { return get(q); }",
        // one-formal typed copy getter
        "int get(struct c_leaf *_p) { struct c_leaf *p = _p; return p->weight; } \
         int w(struct c_leaf *q) { return get(q); }",
        // genuinely read-only formal
        "int get(struct c_leaf *l) { return l->weight; } \
         int w(struct c_leaf *q) { return get(q); }",
        // released through a lexically protected formal
        "void drop2(struct c_box *b) { c_drop(b); } \
         int w(void) { struct c_box *b = c_make(1); if (!b) return 0; drop2(b); return 1; }",
        // copy passed to a borrowed callee
        "void k(struct c_leaf *_l) { struct c_leaf *l = _l; c_peek(l); } \
         int w(struct c_leaf *q) { k(q); return 0; }",
        // copy of a formal, then read through the copy and release the formal
        "void kd(struct c_box *_b) { struct c_box *b = _b; int f = b->flag; c_drop(_b); } \
         int w(void) { struct c_box *b = c_make(1); if (!b) return 0; kd(b); return 1; }",
        // cmp with one formal
        "int cmp1(const void *_a, int k) { \
         const struct c_tp *a = _a; \
         int r = c_same(a->topic, a->topic); if (r) return r; \
         else return a->partition; } \
         int w(const void *_a) { return cmp1(_a, 1); }",
        // copy read in a call actual
        "int get(const void *_p) { const struct c_tp *p = _p; return c_same(p->topic, p->topic); } \
         int w(const void *q) { return get(q); }",
        // two-level cascade
        "int get(const void *_p) { const struct c_leaf *p = _p; return p->weight; } \
         int mid(const void *q) { return get(q); } \
         int w(const void *q) { return mid(q); }",
    ] {
        eprintln!("SOURCE {source}");
        for (name, row) in rows(source) {
            eprintln!("  {name}: {row:?}");
        }
    }
}
