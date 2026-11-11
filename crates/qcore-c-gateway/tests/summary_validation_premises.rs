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

//! Card W47-SUMMARY-VALIDATION-PROTECTED-FORMALS — a W4 summary is validated
//! on the typed-root premises its function's reported row reads.
//!
//! The negative controls were written and run at the base first
//! (`docs/reports/w47-summary-validation-protected-formals/`): each caller
//! below that now refuses was `Safe` there, on a summary whose own row
//! refused. The fixtures use a generic `c_*` surface: `Safe` here is `Safe`
//! over synthetic trusted test assumptions and promotes nothing.

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

/// Every published summary's function and contract, when the unit is Safe.
fn summaries(source: &str) -> Vec<String> {
    match analyze(source) {
        UnrestrictedLegacyResult::Safe(certificate) => certificate
            .summaries
            .iter()
            .map(|record| format!("{}: {:?}", record.function, record.contract.parameters))
            .collect(),
        _ => Vec::new(),
    }
}

fn row<'a>(
    rows: &'a BTreeMap<String, (String, Vec<String>)>,
    name: &str,
) -> (&'a str, Vec<&'a str>) {
    let (status, keys) = rows
        .get(name)
        .unwrap_or_else(|| panic!("missing row `{name}`"));
    (status.as_str(), keys.iter().map(String::as_str).collect())
}

/// librdkafka's `rd_kafka_topic_partition_cmp` / `…_cmp_opaque` shape. The
/// comparator's summary (`Owned`, `Owned`) seeds `_a` and `_b` as entry
/// resources; its own row, analysed with that summary in force, protects
/// them, binds the copies `a` and `b` and refuses the reads through them. At
/// the base the summary was validated with neither formal protected, so the
/// copies were never-bound roots there and the wrapper certified on it. Now
/// the summary is checked on the reported premises, refuses, and the wrapper's
/// call is uncontracted.
const COMPARATOR: &str = "\
    int cmp(const void *_a, const void *_b) { \
    const struct c_tp *a = _a; const struct c_tp *b = _b; \
    int r = c_same(a->topic, b->topic); if (r) return r; \
    else return (int)((a->partition) < (b->partition) ? -1 : ((a->partition) > (b->partition))); } \
    static int w(const void *_a, const void *_b, void *opaque) { return cmp(_a, _b); }";

#[test]
fn a_summary_validated_through_a_copy_of_a_protected_formal_serves_no_caller() {
    if !clang_available() {
        return;
    }
    let rows = rows(COMPARATOR);
    // FLIPPED by CG06-VALUE-CONNECTIVES-AND-CONDITIONAL (rule 10z): under the
    // summary the comparator's own row no longer refuses. The return family
    // refused `RD_CMP`'s reads of the bound copies `a`/`b` because they stood
    // in a `?:` arm; rule 10z branches on the deciding comparison (read
    // unconditionally, as C does) and each edge's return reads them
    // unconditionally. The summary is validated on exactly the premises its
    // reported row reads (rule 4b holds), and that row is `Safe`, so the
    // wrapper's call is contracted. The other controls of this file keep a
    // refusing own row.
    assert_eq!(row(&rows, "cmp"), ("Safe", vec![]));
    assert_eq!(row(&rows, "w"), ("Safe", vec![]));
}

/// The same shape with one formal, whose copy is read in a `Borrowed` actual.
#[test]
fn a_copy_read_in_a_call_actual_serves_no_caller() {
    if !clang_available() {
        return;
    }
    // FLIPPED by CG10-INDIRECT-ACCESS-SOLE-BLOCKERS (rule 12i), on the
    // ground the comparator row above was flipped: `get`'s own row refused
    // `return c_same(p->topic, p->topic)` only because the return position
    // never asked rule W44, which reads those actuals at the statement
    // position. Now it does, the row is `Safe` on exactly the premises the
    // summary is validated on (rule 4b holds), and the caller's call is
    // contracted. The statement form is the control: neither of the card's
    // hooks can move it, and it was and is `Safe` with the same summaries.
    let rows = rows(
        "int get(const void *_p) { const struct c_tp *p = _p; return c_same(p->topic, p->topic); } \
         int w(const void *q) { return get(q); }",
    );
    assert_eq!(row(&rows, "w"), ("Safe", vec![]));
    assert_eq!(row(&rows, "get"), ("Safe", vec![]));
    let statement = crate::rows(
        "int get(const void *_p) { const struct c_tp *p = _p; \
         int r = c_same(p->topic, p->topic); return r; } \
         int w(const void *q) { return get(q); }",
    );
    assert_eq!(row(&statement, "w"), ("Safe", vec![]));
    assert_eq!(row(&statement, "get"), ("Safe", vec![]));
}

/// A withdrawn summary cascades: the middle function's body now holds an
/// uncontracted call, so it derives no summary either, and the outer caller
/// refuses on the middle call.
#[test]
fn a_withdrawn_summary_withdraws_its_callers_summaries() {
    if !clang_available() {
        return;
    }
    let rows = rows(
        "int get(const void *_p) { const struct c_tp *p = _p; return c_same(p->topic, p->topic); } \
         int mid(const void *q) { return get(q); } \
         int w(const void *q) { return mid(q); }",
    );
    // FLIPPED by CG10-INDIRECT-ACCESS-SOLE-BLOCKERS (rule 12i): `get` now
    // validates (see the row above), so neither summary is withdrawn.
    assert_eq!(row(&rows, "w"), ("Safe", vec![]));
    assert_eq!(row(&rows, "mid"), ("Safe", vec![]));
    // The cascade itself still holds where `get`'s own row refuses: a `?:`
    // actual, which neither rule W44 nor rule 12i reads.
    let rows = crate::rows(
        "int get(const void *_p) { const struct c_tp *p = _p; \
         return c_same(p->topic ? p->topic : \"\", p->topic); } \
         int mid(const void *q) { return get(q); } \
         int w(const void *q) { return mid(q); }",
    );
    assert_eq!(row(&rows, "get").0, "Unknown");
    assert_eq!(
        row(&rows, "mid"),
        (
            "Unknown",
            vec!["unsupported:call to uncontracted function `get`"]
        )
    );
    assert_eq!(
        row(&rows, "w"),
        (
            "Unknown",
            vec!["unsupported:call to uncontracted function `mid`"]
        )
    );
}

/// Positive controls. A copy of a seeded formal whose reads the reported run
/// also admits (a scalar read through the tracked alias) still validates on
/// the protected premises, and the caller certifies on the `Owned` summary.
#[test]
fn a_copy_whose_reads_certify_under_protection_still_validates() {
    if !clang_available() {
        return;
    }
    for source in [
        "int get(const void *_p) { const struct c_leaf *p = _p; return p->weight; } \
         int w(const void *q) { return get(q); }",
        "int get(struct c_leaf *_p) { struct c_leaf *p = _p; return p->weight; } \
         int w(struct c_leaf *q) { return get(q); }",
    ] {
        let rows = rows(source);
        assert_eq!(row(&rows, "w"), ("Safe", vec![]), "{source}");
        assert_eq!(row(&rows, "get"), ("Safe", vec![]), "{source}");
        assert!(
            summaries(source).contains(&"get: [Owned { nullable: false }]".to_owned()),
            "{source}"
        );
    }
}

/// A genuinely read-only formal (`Borrowed`, nothing to protect), a formal
/// the body releases (already protected lexically) and a copy handed to a
/// `Borrowed` callee keep their summaries and their callers' Safe.
#[test]
fn read_only_released_and_forwarded_formals_still_validate() {
    if !clang_available() {
        return;
    }
    for (source, summary) in [
        (
            "int get(struct c_leaf *l) { return l->weight; } \
             int w(struct c_leaf *q) { return get(q); }",
            "get: [Borrowed]",
        ),
        (
            "void drop2(struct c_box *b) { c_drop(b); } \
             int w(void) { struct c_box *b = c_make(1); if (!b) return 0; drop2(b); return 1; }",
            "drop2: [Released { nullable: false }]",
        ),
        (
            "void kd(struct c_box *_b) { struct c_box *b = _b; int f = b->flag; c_drop(_b); } \
             int w(void) { struct c_box *b = c_make(1); if (!b) return 0; kd(b); return 1; }",
            "kd: [Released { nullable: false }]",
        ),
        (
            "void k(struct c_leaf *_l) { struct c_leaf *l = _l; c_peek(l); } \
             int w(struct c_leaf *q) { k(q); return 0; }",
            "k: [Owned { nullable: false }]",
        ),
    ] {
        let rows = rows(source);
        assert!(
            rows.values()
                .all(|(status, keys)| status == "Safe" && keys.is_empty()),
            "{source}: {rows:?}"
        );
        assert!(summaries(source).contains(&summary.to_owned()), "{source}");
    }
}
