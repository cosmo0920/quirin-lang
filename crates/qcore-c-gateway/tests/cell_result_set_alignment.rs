//! Card CELL-RESULT-SET-ALIGNMENT (gateway rule 38f as generalised; grammar
//! G05i as amended: `live_set(SET)` / `absent_set(SET)` on a per-end
//! sentinel). USER DECISIONS 2026-11-04 (verbatim): "Basically, Q1(a) and
//! Q2(i) But we must not handle descriptor 0 as a valid handle. So, we need
//! to exclude the 0 descriptor like: written_set(pipedes, [1, MAX]); a live
//! descriptor is in [1, MAX]"; "Write [1, MAX]"; Q3 "(α) live_set suffix";
//! "Accept the leak".
//!
//! Each fixture reads one destroyer shape under one declaration of its own
//! record (fixture records, so no production declaration is restated):
//!
//! * the declared bound `live_set([1, MAX])` with the pinned trees' absent
//!   set `0` (the production `teardown_destroy.qv` shape);
//! * POSIX's `[0, MAX]`, and no declared set at all: the contrast, where a
//!   `> 0` dead edge still holds descriptor 0 (the as-written refusal of
//!   the pseudo contracts' section (a));
//! * the user's third patch (`FLB_UNINITIALIZED_SOCKET` stored after the
//!   allocation and after each close, `> 0` tests): `absent_set(-1)`, the
//!   planned-fix shape of section (d), read as decided;
//! * the loader's refusals.
//!
//! A `Safe` here is `Safe` over the declared assumptions (the protocol and
//! the declared bound included) and promotes nothing; the bound is a
//! declared policy, not POSIX's set (descriptor 0 is observed natively).

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11, check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const LIBRARY: &str = "\
module contract;\n\
extern fn flb_free(ptr: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_released(ptr, nullable);\n\
           ensures normal resource_return_none(); };\n\
extern fn close(fd: i32) -> i32\n\
contract { requires resource_contract_v1; ensures normal resource_released(fd, valid);\n\
           ensures normal resource_kind(fd, descriptor); ensures normal resource_return_none(); };";

/// One host per fixture record: `declared_out` (the production shape),
/// `posix_out` ([0, MAX]), `bare_out` (no set), `patched_out` (the third
/// patch's format), `ingress_out` (`0 | -1`).
const ENDS: &str = "\
module contract;\n\
extern fn fixture_ends(c: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(c);\n\
           ensures normal resource_return_none();\n\
           quant cell #d: declared_out.ch[0] |-> #fresh(descriptor) when #d.ch[0] > 0 live_set([1, MAX]) absent_set(0);\n\
           quant cell #d: declared_out.ch[1] |-> #fresh(descriptor) when #d.ch[1] > 0 live_set([1, MAX]) absent_set(0);\n\
           quant cell #p: posix_out.ch[0] |-> #fresh(descriptor) when #p.ch[0] > 0 live_set([0, MAX]);\n\
           quant cell #p: posix_out.ch[1] |-> #fresh(descriptor) when #p.ch[1] > 0 live_set([0, MAX]);\n\
           quant cell #b: bare_out.ch[0] |-> #fresh(descriptor) when #b.ch[0] > 0;\n\
           quant cell #b: bare_out.ch[1] |-> #fresh(descriptor) when #b.ch[1] > 0;\n\
           quant cell #f: patched_out.ch[0] |-> #fresh(descriptor) when #f.ch[0] > 0 live_set([1, MAX]) absent_set(-1);\n\
           quant cell #f: patched_out.ch[1] |-> #fresh(descriptor) when #f.ch[1] > 0 live_set([1, MAX]) absent_set(-1);\n\
           quant cell #g: ingress_out.q[0] |-> #fresh(descriptor) when #g.q[0] > 0 live_set([1, MAX]) absent_set(0 | -1);\n\
           quant cell #g: ingress_out.q[1] |-> #fresh(descriptor) when #g.q[1] > 0 live_set([1, MAX]) absent_set(0 | -1); };";

const PRELUDE: &str = r"
#define FLB_UNINITIALIZED_SOCKET (-1)
#define mk_event_closesocket(s) close(s)
struct declared_out { int ch[2]; };
struct posix_out { int ch[2]; };
struct bare_out { int ch[2]; };
struct patched_out { int ch[2]; };
struct ingress_out { int q[2]; };
void flb_free(void *p);
int close(int fd);
";

fn environment() -> ResourceContracts {
    let sources = BTreeMap::from([
        ("library.qv".to_owned(), LIBRARY.to_owned()),
        ("ends.qv".to_owned(), ENDS.to_owned()),
    ]);
    load_resource_contract_sources(&sources).expect("the contracts load")
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent CELL-RESULT-SET-ALIGNMENT test: {reason}");
            false
        }
    }
}

fn summary_of(outcome: &UnrestrictedLegacyResult) -> Option<&LegacyProgramSummary> {
    match outcome {
        UnrestrictedLegacyResult::Safe(certificate) => Some(&certificate.summary),
        UnrestrictedLegacyResult::Unsafe(summary) => Some(summary),
        UnrestrictedLegacyResult::Unknown { partial, .. } => partial.as_ref(),
    }
}

/// The entry row's status and its sorted diagnostic texts.
fn outcome(body: &str) -> (String, Vec<String>) {
    let text = format!("{PRELUDE}\n{body}\n");
    let entry = body
        .split('(')
        .next()
        .and_then(|head| head.split_whitespace().last())
        .expect("a definition")
        .to_owned();
    let result = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
        &entry,
        &environment(),
    )
    .unwrap_or_else(|error| panic!("fixture rejected by Clang: {error:?}\n{text}"));
    if let UnrestrictedLegacyResult::Safe(certificate) = &result {
        check_legacy_safety_certificate(certificate).expect("a Safe result rechecks");
    }
    let summary = summary_of(&result).unwrap_or_else(|| panic!("no summary: {result:?}"));
    let row = &summary.functions[&entry];
    let mut diagnostics = row
        .diagnostics
        .iter()
        .map(|d| match &d.kind {
            LegacyDiagnosticKind::Unsupported { description } => description.clone(),
            LegacyDiagnosticKind::UnknownAlias { variable } => format!("unknown_alias:{variable}"),
            other => format!("{other:?}"),
        })
        .collect::<Vec<_>>();
    diagnostics.sort();
    diagnostics.dedup();
    (format!("{:?}", row.status), diagnostics)
}

fn expect(label: &str, body: &str, status: &str, diagnostics: &[&str]) {
    if !clang_available() {
        return;
    }
    let (actual_status, actual) = outcome(body);
    let mut wanted = diagnostics
        .iter()
        .map(|text| (*text).to_owned())
        .collect::<Vec<_>>();
    wanted.sort();
    assert_eq!(
        (actual_status.as_str(), &actual),
        (status, &wanted),
        "{label}\n{body}"
    );
}

/// A destroyer as Fluent Bit writes it at the pinned trees: both ends tested
/// `> 0`, closed, the holder released.
fn as_written(record: &str) -> String {
    format!(
        "int d(struct {record} *ins) {{\n\
         if (ins->ch[0] > 0) {{\n\
         mk_event_closesocket(ins->ch[0]);\n\
         }}\n\
         if (ins->ch[1] > 0) {{\n\
         mk_event_closesocket(ins->ch[1]);\n\
         }}\n\
         flb_free(ins);\n\
         return 0;\n\
         }}"
    )
}

const RELEASED: &str = "`ins` is released while a cell of it may hold descriptor 0, which a `> 0` test read as absent (descriptor-cell-zero-undecided)";

/// Under the declared `[1, MAX]` the `> 0` dead edges decide "absent": the
/// release refuses for nothing but route G's caller-record release.
#[test]
fn the_declared_bound_decides_the_positive_dead_edge() {
    expect(
        "declared [1, MAX], absent 0",
        &as_written("declared_out"),
        "Unknown",
        &["unknown_alias:ins"],
    );
}

/// POSIX's `[0, MAX]`: the dead edge holds descriptor 0, the release names it.
#[test]
fn posix_leaves_descriptor_zero_on_the_dead_edge() {
    expect(
        "POSIX [0, MAX]",
        &as_written("posix_out"),
        "Unknown",
        &[RELEASED, "unknown_alias:ins"],
    );
}

/// No declared set: rule 38f's reading, unchanged.
#[test]
fn without_sets_rule_38f_reads_as_before() {
    expect(
        "no set",
        &as_written("bare_out"),
        "Unknown",
        &[RELEASED, "unknown_alias:ins"],
    );
}

/// The third patch's shape: `> 0`, close, then the sentinel `-1`; the record
/// kept under its protocol (no release here, the caller holds it).
#[test]
fn the_patched_shape_keeps_its_protocol() {
    expect(
        "patched: close and store -1",
        "int d(struct patched_out *ins) {\n\
         if (ins->ch[0] > 0) {\n\
         mk_event_closesocket(ins->ch[0]);\n\
         ins->ch[0] = FLB_UNINITIALIZED_SOCKET;\n\
         }\n\
         if (ins->ch[1] > 0) {\n\
         mk_event_closesocket(ins->ch[1]);\n\
         ins->ch[1] = FLB_UNINITIALIZED_SOCKET;\n\
         }\n\
         return 0;\n\
         }",
        "Safe",
        &[],
    );
}

/// The patched shape with an unconditional reset after the tests
/// (flb_config_exit in the third patch).
#[test]
fn the_patched_unconditional_reset_keeps_its_protocol() {
    expect(
        "patched: reset after the tests",
        "int d(struct patched_out *ins) {\n\
         if (ins->ch[0] > 0) {\n\
         mk_event_closesocket(ins->ch[0]);\n\
         }\n\
         ins->ch[0] = FLB_UNINITIALIZED_SOCKET;\n\
         return 0;\n\
         }",
        "Safe",
        &[],
    );
}

/// A store outside the declared absent set breaks the sentinel: the patched
/// declaration's absent set is `-1`, so storing calloc's 0 refuses.
#[test]
fn a_store_outside_the_absent_set_refuses() {
    if !clang_available() {
        return;
    }
    let (status, diagnostics) = outcome(
        "int d(struct patched_out *ins) {\n\
         if (ins->ch[0] > 0) {\n\
         mk_event_closesocket(ins->ch[0]);\n\
         ins->ch[0] = 0;\n\
         }\n\
         return 0;\n\
         }",
    );
    assert_eq!(status, "Unknown");
    assert!(
        diagnostics
            .iter()
            .any(|d| d.contains("descriptor-cell-protocol-broken")),
        "{diagnostics:?}"
    );
}

/// `ingress_queue_channels`' pinned stores: `-1` lies in `absent_set(0 | -1)`.
#[test]
fn the_ingress_minus_one_stores_keep_the_protocol() {
    expect(
        "ingress -1 stores",
        "int d(struct ingress_out *ins) {\n\
         if (ins->q[0] <= 0) {\n\
         ins->q[0] = -1;\n\
         }\n\
         return 0;\n\
         }",
        "Safe",
        &[],
    );
}

fn refused(text: &str) -> String {
    let sources = BTreeMap::from([("bad.qv".to_owned(), text.to_owned())]);
    load_resource_contract_sources(&sources)
        .err()
        .map(|e| format!("{e:?}"))
        .unwrap_or_default()
}

fn host(clauses: &str) -> String {
    format!(
        "module contract;\n\
         extern fn h(c: ptr<mut, u8>) -> unit\n\
         contract {{ requires resource_contract_v1; ensures normal resource_borrowed(c);\n\
         ensures normal resource_return_none();\n{clauses} }};"
    )
}

/// The loader's refusals (USER DECISION 2026-11-04, Q2 (i)).
#[test]
fn the_loader_refuses_inconsistent_sets() {
    // An absent set on the guard's live edge (a never-created value read live).
    assert!(
        refused(&host(
            "quant cell #s: t.a[0] |-> #fresh(descriptor) when #s.a[0] > 0 absent_set(0 | 1);"
        ))
        .contains("quant-cell-absent-set")
    );
    // A live set that meets the absent set (POSIX's [0, MAX] over calloc's 0).
    assert!(
        refused(&host(
            "quant cell #s: t.a[0] |-> #fresh(descriptor) when #s.a[0] > 0 live_set([0, MAX]) absent_set(0);"
        ))
        .contains("quant-cell-live-set")
    );
    // Sets only on a per-end sentinel's own `#fresh` clause.
    assert!(
        refused(&host(
            "quant cell #s: t.a[0] |-> #fresh(descriptor) when #s.a[0] > 0;\n\
             quant cell #s: t.e.fd |-> #same(a[0]) when #s.a[0] > 0 absent_set(0);"
        ))
        .contains("quant-cell-set")
    );
    assert!(
        refused(&host(
            "quant cell #s: t.a[0] |-> #fresh(descriptor) when #s.on == 1 live_set([1, MAX]);"
        ))
        .contains("quant-cell-set")
    );
    // The two suffixes in their order, each once.
    assert!(
        refused(&host(
            "quant cell #s: t.a[0] |-> #fresh(descriptor) when #s.a[0] > 0 absent_set(0) live_set([1, MAX]);"
        ))
        .contains("quant-cell-set")
    );
    // `>= 0` stays refused (step 10: no form the patch does not use).
    assert!(
        refused(&host(
            "quant cell #s: t.a[0] |-> #fresh(descriptor) when #s.a[0] >= 0 live_set([1, MAX]);"
        ))
        .contains("quant-cell-guard")
    );
}
