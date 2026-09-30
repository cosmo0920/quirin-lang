//! Card CG04-LOOP-SITE-GENERATIONS: a re-acquired site on a CFG cycle keeps
//! its earlier generations apart (`loop_generations.rs`,
//! `legacy_loop_generations.rs`), on route G.
//!
//! Every fixture runs the real Clang-backed gateway over the synthetic `q_*`,
//! `k_*` and `cg4_*` surfaces. `Safe` here is `Safe` over synthetic trusted
//! test assumptions and promotes nothing.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary,
    ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const DECLARATIONS: &str = "\
typedef unsigned long size_t;
void *q_alloc(void); void q_free(void *p); void q_observe(void *p);
void *q_realloc(void *p, size_t n); void q_make(void **d);
void *q_maybe_alloc(void); void q_nullable_free(void *p);
int k_open(const char *path); int k_close(int fd);
";

const SHIFT_REFUSED: &str =
    "loop re-acquisition: an earlier generation that may be live is still held";

fn contracts() -> ResourceContracts {
    let file = |name: &str, text: &str| (name.to_owned(), text.to_owned());
    load_resource_contract_sources(&BTreeMap::from([
        file("q_base.qv", include_str!("fixtures/contracts/q_base.qv")),
        file(
            "q_rich_modes.qv",
            include_str!("fixtures/contracts/q_rich_modes.qv"),
        ),
        file(
            "q_realloc.qv",
            include_str!("fixtures/contracts/q_realloc.qv"),
        ),
        file(
            "q_nullable.qv",
            include_str!("fixtures/contracts/q_nullable.qv"),
        ),
        file(
            "descriptor_generic.qv",
            include_str!("fixtures/contracts/descriptor_generic.qv"),
        ),
    ]))
    .expect("the fixture contracts load")
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent CG04-LOOP-SITE-GENERATIONS test: {reason}");
            false
        }
    }
}

#[derive(Debug)]
struct Run {
    status: AnalysisStatus,
    diagnostics: Vec<LegacyDiagnosticKind>,
    lines: Vec<usize>,
}

impl Run {
    fn leaks(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|kind| matches!(kind, LegacyDiagnosticKind::Leak { .. }))
            .count()
    }

    fn refused(&self, description: &str) -> bool {
        self.diagnostics.iter().any(|kind| {
            matches!(kind, LegacyDiagnosticKind::Unsupported { description: d } if d == description)
        })
    }
}

fn summary_of(outcome: &UnrestrictedLegacyResult) -> &LegacyProgramSummary {
    match outcome {
        UnrestrictedLegacyResult::Safe(certificate) => &certificate.summary,
        UnrestrictedLegacyResult::Unsafe(summary) => summary,
        UnrestrictedLegacyResult::Unknown { partial, .. } => {
            partial.as_ref().expect("these fixtures project partially")
        }
    }
}

/// `body` is the function `subject(int flag, int n)`'s body.
fn analyze(body: &str) -> Run {
    let source = format!("{DECLARATIONS}void subject(int flag, int n) {{\n{body}\n}}\n");
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        "subject",
        &contracts(),
    )
    .unwrap_or_else(|error| panic!("valid fixture rejected by Clang: {error:?}\n{source}"));
    if let UnrestrictedLegacyResult::Safe(certificate) = &outcome {
        check_legacy_safety_certificate(certificate).expect("a Safe result rechecks");
    }
    let result = &summary_of(&outcome).functions["subject"];
    Run {
        status: result.status,
        diagnostics: result
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.kind.clone())
            .collect(),
        lines: result
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.line)
            .collect(),
    }
}

#[test]
fn probe() {
    if !clang_available() {
        return;
    }
    for body in [
        "void *p;\nfor (;;) { p = q_maybe_alloc(); if (p == 0) break; q_observe(p); }",
        "void *p;\ndo { p = q_maybe_alloc(); if (p) q_observe(p); } while (p != 0);",
        "void *p;\nwhile (flag) { p = q_maybe_alloc(); if (p == 0) return; q_observe(p); }",
        "void *p;\nwhile (flag) { p = q_maybe_alloc(); if (p == 0) return; q_free(p); }",
        "void *p = q_alloc();\nwhile (flag) { p = q_alloc(); }\nq_free(p);",
        "void *p;\nwhile (flag) { p = q_alloc(); q_observe(p); q_free(p); p = q_alloc(); q_free(p); }",
        "void *prev = 0; void *cur;\nwhile (flag) { cur = q_alloc(); if (prev) q_free(prev); prev = cur; }\nif (prev) q_free(prev);",
        "void *p = q_alloc(); void *q;\nwhile (flag) { q = q_realloc(p, 8); if (q == 0) { q_free(p); return; } p = q; }\nq_free(p);",
        "void *p;\nwhile (flag) { q_make(&p); q_free(p); }",
        "void *p;\nq_make(&p);\nwhile (flag) { q_make(&p); }\nq_free(p);",
        "int fd;\nwhile (flag) { fd = k_open(\"x\"); if (fd < 0) break; k_close(fd); }",
        "int fd;\nwhile (flag) { fd = k_open(\"x\"); if (fd < 0) break; }",
        "void *p;\nwhile ((p = q_maybe_alloc()) != 0) { q_free(p); }",
        "void *p;\nwhile ((p = q_maybe_alloc()) != 0) { q_observe(p); }",
    ] {
        let run = analyze(body);
        eprintln!(
            "=== {body}\n{:?} leaks={} {:?} {:?}",
            run.status,
            run.leaks(),
            run.diagnostics,
            run.lines
        );
    }
    let _ = SHIFT_REFUSED;
    let _ = Run::refused;
}
