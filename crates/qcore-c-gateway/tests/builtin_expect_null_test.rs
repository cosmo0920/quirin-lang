//! Card BUILTIN-EXPECT-NULL-TEST — a null test inside `__builtin_expect`
//! (`likely()` / `unlikely()`) refines the tested acquisition as the bare
//! test does.
//!
//! Every fixture runs the real Clang-backed gateway over the synthetic `k_*`
//! surface (`fixtures/contracts/resource_kinds_generic.qv`: `k_alloc` returns
//! a nullable owned pointer, `k_free` releases a nullable one). `Safe` here is
//! `Safe` over synthetic trusted test assumptions and promotes nothing.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary,
    ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const DECLARATIONS: &str = "\
typedef unsigned long size_t; typedef long ssize_t; \
void *k_alloc(size_t size); void k_free(void *p); int opaque(void); \
int snprintf(char *s, size_t n, const char *f, ...); char *strerror(int e); \
int *__errno_location(void);\n\
#define errno (*__errno_location ())\n\
#define unlikely(x) __builtin_expect((x), 0)\n\
#define likely(x) __builtin_expect((x), 1)\n";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "resource_kinds_generic.qcore".to_owned(),
        include_str!("fixtures/contracts/resource_kinds_generic.qv").to_owned(),
    )]))
    .unwrap()
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent builtin_expect test: {reason}");
            false
        }
    }
}

#[derive(Debug)]
struct Run {
    status: AnalysisStatus,
    diagnostics: Vec<LegacyDiagnosticKind>,
    lines: Vec<(usize, LegacyDiagnosticKind)>,
}

impl Run {
    fn leaks(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|kind| matches!(kind, LegacyDiagnosticKind::Leak { .. }))
            .count()
    }

    fn refusals(&self) -> Vec<&str> {
        self.diagnostics
            .iter()
            .filter_map(|kind| match kind {
                LegacyDiagnosticKind::Unsupported { description } => Some(description.as_str()),
                _ => None,
            })
            .collect()
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

fn analyze(body: &str) -> Run {
    let source = format!("{DECLARATIONS}\n{body}\n");
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
    let summary = summary_of(&outcome);
    let result = &summary.functions["subject"];
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
            .map(|diagnostic| (diagnostic.line, diagnostic.kind.clone()))
            .collect(),
    }
}

/// The wrapped form and its bare twin decide identically: the same status
/// and the same diagnostics on the same lines.
fn assert_transparent(wrapped: &str, bare: &str) -> Run {
    let left = analyze(wrapped);
    let right = analyze(bare);
    assert_eq!(
        left.status, right.status,
        "{wrapped}\n{left:?}\n{bare}\n{right:?}"
    );
    assert_eq!(left.lines, right.lines, "{wrapped}\n{bare}");
    left
}

/// `likely()` / `unlikely()` around a null test and a non-null test refine
/// the tested acquisition as the bare test does, in both directions: the
/// failure arm holds nothing, and a test whose non-null arm returns still
/// reports the leak there.
#[test]
fn likely_and_unlikely_around_null_tests_refine_as_the_bare_test() {
    if !clang_available() {
        return;
    }
    for (wrapped, bare) in [
        (
            "void subject(size_t n) { void *p = k_alloc(n); if (unlikely(!p)) return; k_free(p); }",
            "void subject(size_t n) { void *p = k_alloc(n); if (!p) return; k_free(p); }",
        ),
        (
            "void subject(size_t n) { void *p = k_alloc(n); if (unlikely(p == 0)) return; k_free(p); }",
            "void subject(size_t n) { void *p = k_alloc(n); if (p == 0) return; k_free(p); }",
        ),
        (
            "void subject(size_t n) { void *p = k_alloc(n); if (likely(p != 0)) k_free(p); }",
            "void subject(size_t n) { void *p = k_alloc(n); if (p != 0) k_free(p); }",
        ),
        (
            "void subject(size_t n) { void *p = k_alloc(n); if (likely(!!(p))) k_free(p); }",
            "void subject(size_t n) { void *p = k_alloc(n); if (!!(p)) k_free(p); }",
        ),
        (
            "void subject(size_t n) { void *p; p = k_alloc(n); if (unlikely(!p)) { return; } k_free(p); }",
            "void subject(size_t n) { void *p; p = k_alloc(n); if (!p) { return; } k_free(p); }",
        ),
        (
            "void subject(size_t n) { void *p = k_alloc(n); do { if (!p) break; k_free(p); p = 0; } while (unlikely(p != 0)); }",
            "void subject(size_t n) { void *p = k_alloc(n); do { if (!p) break; k_free(p); p = 0; } while (p != 0); }",
        ),
    ] {
        let run = assert_transparent(wrapped, bare);
        assert_eq!(run.status, AnalysisStatus::Safe, "{wrapped}\n{run:?}");
    }
    // Two positions where the wrapped test is read by rule 5h's desugaring
    // and the bare one by an older route that does not refine it (follow-up
    // BARE-CONNECTIVE-NULL-LEAF-REFINEMENT): the wrapped form is decided.
    for wrapped in [
        "void *subject(size_t n) { void *p = k_alloc(n); return likely(p != 0) ? p : 0; }",
        "void subject(size_t n) { void *p = k_alloc(n); if (unlikely(!p) && n) return; if (!p) return; k_free(p); }",
    ] {
        let run = analyze(wrapped);
        assert_eq!(run.status, AnalysisStatus::Safe, "{wrapped}\n{run:?}");
    }
    // The non-null arm returns with the object live: a leak, wrapped or bare.
    for (wrapped, bare) in [
        (
            "void subject(size_t n) { void *p = k_alloc(n); if (unlikely(p != 0)) return; k_free(p); }",
            "void subject(size_t n) { void *p = k_alloc(n); if (p != 0) return; k_free(p); }",
        ),
        (
            "void subject(size_t n) { void *p = k_alloc(n); if (likely(!!(p))) return; }",
            "void subject(size_t n) { void *p = k_alloc(n); if (!!(p)) return; }",
        ),
    ] {
        let run = assert_transparent(wrapped, bare);
        assert_eq!(run.status, AnalysisStatus::Unsafe, "{wrapped}\n{run:?}");
        assert_eq!(run.leaks(), 1, "{wrapped}\n{run:?}");
    }
}

/// Card BUILTIN-EXPECT-NULL-TEST's admission: a hint spelled as any integer
/// literal (C11 6.4.4.1) is read; before the card only a bare digit sequence
/// was, and `0L` or `0x0` kept the refusal.
#[test]
fn an_integer_literal_hint_is_read() {
    if !clang_available() {
        return;
    }
    for hint in ["0", "(0)", "0L", "0UL", "0x0", "00"] {
        let run = assert_transparent(
            &format!(
                "void subject(size_t n) {{ void *p = k_alloc(n); if (__builtin_expect(!p, {hint})) return; k_free(p); }}"
            ),
            "void subject(size_t n) { void *p = k_alloc(n); if (!p) return; k_free(p); }",
        );
        assert_eq!(run.status, AnalysisStatus::Safe, "{hint}\n{run:?}");
    }
}

/// A wrapped operand that is not a test the route reads bare keeps the
/// refusal the bare operand has; so does `__builtin_expect_with_probability`,
/// which this card does not admit.
#[test]
fn an_operand_that_is_not_a_bare_test_stays_refused() {
    if !clang_available() {
        return;
    }
    let refusal = "unmodeled call, aggregate or indirect access in branch condition";
    for (wrapped, bare) in [
        (
            "void subject(size_t n) { void *p = k_alloc(n); if (unlikely(!p | opaque())) return; k_free(p); }",
            "void subject(size_t n) { void *p = k_alloc(n); if (!p | opaque()) return; k_free(p); }",
        ),
        (
            "void subject(size_t n) { void *p = k_alloc(n); if (unlikely(opaque())) { k_free(p); return; } k_free(p); }",
            "void subject(size_t n) { void *p = k_alloc(n); if (opaque()) { k_free(p); return; } k_free(p); }",
        ),
    ] {
        let run = assert_transparent(wrapped, bare);
        assert_eq!(run.status, AnalysisStatus::Unknown, "{wrapped}\n{run:?}");
        assert!(
            run.refusals()
                .iter()
                .any(|r| r.starts_with("unmodeled call") || r.starts_with("call to uncontracted")),
            "{wrapped}\n{run:?}"
        );
        assert_eq!(run.leaks(), 0);
    }
    let run = analyze(
        "void subject(size_t n) { void *p = k_alloc(n); if (__builtin_expect_with_probability(!p, 0, 0.9)) return; k_free(p); }",
    );
    assert_eq!(run.status, AnalysisStatus::Unknown, "{run:?}");
    assert_eq!(run.refusals(), [refusal]);
}

/// A hint that may have an evaluation event, or whose value the reading
/// cannot see is a constant, keeps the whole condition refused: the call
/// would otherwise drop the hint's effect.
#[test]
fn a_hint_with_a_side_effect_stays_refused() {
    if !clang_available() {
        return;
    }
    let refusal = "unmodeled call, aggregate or indirect access in branch condition";
    for hint in ["opaque()", "k++", "k = 1", "k", "-1", "(long)0"] {
        let run = analyze(&format!(
            "void subject(size_t n, long k) {{ void *p = k_alloc(n); if (__builtin_expect(!p, {hint})) return; k_free(p); }}"
        ));
        assert_eq!(run.status, AnalysisStatus::Unknown, "{hint}\n{run:?}");
        assert!(run.refusals().contains(&refusal), "{hint}\n{run:?}");
        assert_eq!(run.leaks(), 0, "{hint}\n{run:?}");
    }
}

/// snappy.c `rd_kafka_snappy_java_uncompress` (librdkafka 2.15.0, lines
/// 1705-1712): `outbuf = rd_malloc(uof); if (unlikely(!outbuf)) { …; return
/// NULL; }`. The failure arm refines the allocation away exactly as the bare
/// `!outbuf` does. In the function's two-pass loop the arm's return still
/// reports a leaf of the same allocation site, identically with the bare
/// test: that leaf is the earlier loop generation (it also appears at the
/// `uof <= 0` return, before any allocation of the iteration), card
/// LOOP-REACQUISITION-LEAK-ACCUSATION's mechanism, not the wrapper.
#[test]
fn the_snappy_failure_arm_refines_the_allocation_away() {
    if !clang_available() {
        return;
    }
    let run = assert_transparent(
        SNAPPY_NO_LOOP,
        &SNAPPY_NO_LOOP.replace("unlikely(!outbuf)", "!outbuf"),
    );
    assert_eq!(run.status, AnalysisStatus::Safe, "{run:?}");
    let run = assert_transparent(
        SNAPPY_PASS,
        &SNAPPY_PASS.replace("unlikely(!outbuf)", "!outbuf"),
    );
    assert_eq!(run.status, AnalysisStatus::Safe, "{run:?}");
    let run = assert_transparent(
        LOOP_ONE_GENERATION,
        &LOOP_ONE_GENERATION.replace("unlikely(!outbuf)", "!outbuf"),
    );
    assert_eq!(run.status, AnalysisStatus::Safe, "{run:?}");
    // The two-pass loop: identical wrapped and bare, the leaves resting on the
    // loop generation join (`if (uof <= 0) return 0;` precedes the iteration's
    // allocation and is accused too).
    let run = assert_transparent(
        SNAPPY_LOOP,
        &SNAPPY_LOOP.replace("unlikely(!outbuf)", "!outbuf"),
    );
    // Lines are those of the `-E -P` text; the allocation is two lines below
    // the pre-allocation `return` and two above the failure arm's `return`.
    let mut acquisitions = run.lines.iter().filter_map(|(line, kind)| match kind {
        LegacyDiagnosticKind::Leak { acquisition_line } => Some((*line, *acquisition_line)),
        _ => None,
    });
    let (_, acquisition) = acquisitions.next().expect("the loop form keeps its leaves");
    let leak_lines = run
        .lines
        .iter()
        .filter_map(|(line, kind)| match kind {
            LegacyDiagnosticKind::Leak { acquisition_line } => {
                assert_eq!(*acquisition_line, acquisition, "{run:?}");
                Some(*line)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(
        leak_lines.contains(&(acquisition - 2)),
        "the pre-allocation return: {run:?}"
    );
    assert!(
        leak_lines.contains(&(acquisition + 2)),
        "the failure arm's return: {run:?}"
    );
}

const SNAPPY_NO_LOOP: &str = "char *subject(size_t uof) {
    char *outbuf = 0;
    outbuf = k_alloc(uof);
    if (unlikely(!outbuf)) {
        return 0;
    }
    return outbuf;
}";

const SNAPPY_PASS: &str = "char *subject(size_t inlen, size_t *outlenp, int pass) {
    char *outbuf = 0;
    ssize_t uof = (ssize_t)inlen;
    if (pass == 1) {
        if (uof <= 0) {
            return 0;
        }
        outbuf = k_alloc(uof);
        if (unlikely(!outbuf)) {
            return 0;
        }
    } else {
        *outlenp = uof;
    }
    return outbuf;
}";

const LOOP_ONE_GENERATION: &str = "char *subject(size_t n) {
    char *outbuf = 0;
    for (;;) {
        outbuf = k_alloc(n);
        if (unlikely(!outbuf)) {
            return 0;
        }
        break;
    }
    return outbuf;
}";

const SNAPPY_LOOP: &str = "char *subject(size_t inlen, size_t *outlenp) {
    int pass;
    char *outbuf = 0;
    for (pass = 1; pass <= 2; pass++) {
        ssize_t uof = (ssize_t)inlen;
        if (pass == 1) {
            if (uof <= 0) {
                return 0;
            }
            outbuf = k_alloc(uof);
            if (unlikely(!outbuf)) {
                return 0;
            }
        } else {
            *outlenp = uof;
        }
    }
    return outbuf;
}";

/// The census harness of `docs/reports/builtin-expect-null-test/`: each unit
/// of the list `QCORE_BUILTIN_EXPECT_CENSUS` names (written by
/// `scripts/census.py --harness-list`) is analysed twice under the resource
/// contracts of `QCORE_BUILTIN_EXPECT_CENSUS_CONTRACTS` (a contract
/// manifest), each time from its recorded compile configuration with its main
/// source definitions in scope, as the current route analyses it: once as
/// configured ("wrapped") and once with `-D__builtin_expect(e,c)=(e)` added
/// ("bare"), which replaces every wrapper by its operand on the same line of
/// the unit's own preprocessed text. The per-function statuses and diagnostics
/// of both are written to `QCORE_BUILTIN_EXPECT_CENSUS_OUT`. Evidence only;
/// ignored by the suite and inert without the variable.
#[test]
#[ignore = "census harness: run explicitly with QCORE_BUILTIN_EXPECT_CENSUS"]
fn wrapper_census_harness() {
    use qcore_c_gateway::legacy_c::{
        CCompilerDialect, CFrontendFlag, CPreprocessorOption, CSourceInput, LegacyFunctionScope,
    };
    let Some(list) = std::env::var_os("QCORE_BUILTIN_EXPECT_CENSUS") else {
        return;
    };
    let out = std::env::var_os("QCORE_BUILTIN_EXPECT_CENSUS_OUT").expect("an output path");
    let manifest = std::env::var_os("QCORE_BUILTIN_EXPECT_CENSUS_CONTRACTS")
        .expect("a resource-contract manifest");
    let contracts = qcore_contract::contract_loader::load_resource_contract_manifest(
        std::path::Path::new(&manifest),
    )
    .unwrap();
    let list: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(list).unwrap()).unwrap();
    let jobs = std::env::var("QCORE_BUILTIN_EXPECT_CENSUS_JOBS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(1)
        .max(1);
    let option = |kind: &str, value: &str| -> CPreprocessorOption {
        match kind {
            "Define" => CPreprocessorOption::Define(value.to_owned()),
            "Undefine" => CPreprocessorOption::Undefine(value.to_owned()),
            "Include" => CPreprocessorOption::Include(value.to_owned()),
            "SystemInclude" => CPreprocessorOption::SystemInclude(value.to_owned()),
            "QuoteInclude" => CPreprocessorOption::QuoteInclude(value.to_owned()),
            "FrontendFlag" => CPreprocessorOption::FrontendFlag(match value {
                "AllWarnings" => CFrontendFlag::AllWarnings,
                "ExtraWarnings" => CFrontendFlag::ExtraWarnings,
                "DebugInformation" => CFrontendFlag::DebugInformation,
                "PositionIndependentCode" => CFrontendFlag::PositionIndependentCode,
                "AssemblerNoExecStack" => CFrontendFlag::AssemblerNoExecStack,
                other => panic!("the harness renders no frontend flag {other}"),
            }),
            other => panic!("the harness renders no option {other}"),
        }
    };
    let entries = list.as_array().unwrap().clone();
    let next = std::sync::atomic::AtomicUsize::new(0);
    let results = std::sync::Mutex::new(Vec::new());
    let analyse = |entry: &serde_json::Value, bare: bool| -> serde_json::Value {
        let mut preprocessor_options = entry["options"]
            .as_array()
            .unwrap()
            .iter()
            .map(|pair| option(pair[0].as_str().unwrap(), pair[1].as_str().unwrap()))
            .collect::<Vec<_>>();
        if bare {
            preprocessor_options.push(CPreprocessorOption::Define(
                "__builtin_expect(e,c)=(e)".to_owned(),
            ));
        }
        let directory = std::path::PathBuf::from(entry["directory"].as_str().unwrap());
        let spelled = entry["spelled"].as_str().unwrap();
        let source = std::fs::read_to_string(directory.join(spelled)).unwrap();
        let unit = C11TranslationUnit {
            source: &source,
            dialect: match entry["dialect"].as_str().unwrap() {
                "gnu17" => CCompilerDialect::Gnu17,
                "gnu11" => CCompilerDialect::Gnu11,
                "gnu99" => CCompilerDialect::Gnu99,
                "c11" => CCompilerDialect::C11,
                other => panic!("the harness renders no dialect {other}"),
            },
            function_scope: LegacyFunctionScope::MainSourceDefinitions,
            input: CSourceInput::DeclaredFile {
                spelled: spelled.to_owned(),
                directory,
            },
            preprocessor_options,
            ..Default::default()
        };
        match analyze_unrestricted_c11(&unit, entry["entry"].as_str().unwrap(), &contracts) {
            Err(error) => serde_json::json!({ "error": format!("{error:?}") }),
            Ok(outcome) => {
                let Some(summary) = (match &outcome {
                    UnrestrictedLegacyResult::Safe(certificate) => Some(&certificate.summary),
                    UnrestrictedLegacyResult::Unsafe(summary) => Some(summary),
                    UnrestrictedLegacyResult::Unknown { partial, .. } => partial.as_ref(),
                }) else {
                    return serde_json::json!({ "error": "no partial summary" });
                };
                let functions = summary
                    .functions
                    .iter()
                    .map(|(name, result)| {
                        let mut diagnostics = result
                            .diagnostics
                            .iter()
                            .map(|d| format!("{}: {:?}", d.line, d.kind))
                            .collect::<Vec<_>>();
                        diagnostics.sort();
                        (
                            name.clone(),
                            serde_json::json!({
                                "status": format!("{:?}", result.status),
                                "diagnostics": diagnostics,
                            }),
                        )
                    })
                    .collect::<serde_json::Map<_, _>>();
                serde_json::Value::Object(functions)
            }
        }
    };
    std::thread::scope(|scope| {
        for _ in 0..jobs {
            scope.spawn(|| {
                loop {
                    let index = next.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    let Some(entry) = entries.get(index) else {
                        break;
                    };
                    let row = serde_json::json!({
                        "unit": entry["unit"],
                        "wrapped": analyse(entry, false),
                        "bare": analyse(entry, true),
                    });
                    eprintln!("census: {}", entry["unit"]);
                    results.lock().unwrap().push(row);
                }
            });
        }
    });
    let mut results = results.into_inner().unwrap();
    results.sort_by(|a, b| a["unit"].as_str().cmp(&b["unit"].as_str()));
    std::fs::write(out, serde_json::to_vec_pretty(&results).unwrap()).unwrap();
}
