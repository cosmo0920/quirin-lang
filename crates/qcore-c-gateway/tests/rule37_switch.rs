//! Card RULE37-PILOT: rule 37's run-time switch through the public gateway
//! entry point, read from the process environment as an analyzer run reads
//! it. The switch is read once per process, so each switched case runs this
//! binary again as a child with `QCORE_GATEWAY_RULES_OFF` set (`child_*` tests
//! do nothing unless `RULE37_SWITCH_CHILD` names them).
//!
//! Fixtures: rule 37's constructor of `result_constructors.rs` (the typed
//! `sizeof` exemption alone, under a plain non-null allocator) and the real
//! provider specifications of `checked_allocator_constructors.rs` (the
//! exemption and the `asserted_allocation` premise). With nothing off they are
//! `Safe`; with 37 off they keep their refusals, so off never widens.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use qcore_c_gateway::gateway_rules;
use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, CCompilerDialect, CSourceInput, LegacyDiagnosticKind,
    LegacyFunctionSummary, ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::{
    load_resource_contract_manifest, load_resource_contract_sources,
};

const CHILD: &str = "RULE37_SWITCH_CHILD";

/// The refusal of the constructor below with the exemption off. Off makes the
/// store scan's span list empty, which is the scan's code before rule 37
/// (`1f1615cfa^`), so `sizeof(*e)` reads as a dereference of the base
/// (premise 4), the base is refused, and the store is not admitted.
const PRE_RULE_37_REFUSAL: [&str; 2] = [
    "UnknownAlias { variable: \"e\" }",
    "indirect memory access is outside the resource projection",
];

const CONSTRUCTOR: &str = "struct envelope *build(const char *name) { struct envelope *e; \
                           e=allocate(sizeof(*e)); e->name=duplicate(name); e->state=3; return e; }";

fn plain_contracts() -> ResourceContracts {
    let text = r#"module contract;
extern fn allocate(size: usize) -> ptr<mut, u8> contract {
 requires resource_contract_v1;
 ensures normal resource_borrowed(size);
 ensures normal resource_return_owned(nonnull);
};
extern fn duplicate(s: ptr<const, u8>) -> ptr<mut, u8> contract {
 requires resource_contract_v1;
 ensures normal resource_borrowed(s);
 ensures normal resource_return_owned(nullable);
};
"#;
    load_resource_contract_sources(&BTreeMap::from([(
        "constructor.qv".to_owned(),
        text.to_owned(),
    )]))
    .unwrap()
}

fn summary(result: UnrestrictedLegacyResult, function: &str) -> LegacyFunctionSummary {
    match result {
        UnrestrictedLegacyResult::Safe(c) => {
            check_legacy_safety_certificate(&c).unwrap();
            c.summary.functions[function].clone()
        }
        UnrestrictedLegacyResult::Unsafe(s)
        | UnrestrictedLegacyResult::Unknown {
            partial: Some(s), ..
        } => s.functions[function].clone(),
        other => panic!("no analyzed body: {other:?}"),
    }
}

fn leaves(function: &LegacyFunctionSummary) -> Vec<String> {
    function
        .diagnostics
        .iter()
        .map(|diagnostic| match &diagnostic.kind {
            LegacyDiagnosticKind::Unsupported { description } => description.clone(),
            other => format!("{other:?}"),
        })
        .collect()
}

fn plain_constructor() -> LegacyFunctionSummary {
    let source = format!(
        "typedef __SIZE_TYPE__ size_t;\n struct envelope {{ char *name; int state; }};\n \
         void *allocate(size_t size); char *duplicate(const char *s);\n {CONSTRUCTOR}"
    );
    summary(
        analyze_unrestricted_c11(
            &C11TranslationUnit {
                source: &source,
                ..Default::default()
            },
            "build",
            &plain_contracts(),
        )
        .unwrap(),
        "build",
    )
}

struct SourceFixture(PathBuf);
impl SourceFixture {
    fn new(source: &str) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "qcore-rule37-switch-{}-{}-{nonce}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&directory).unwrap();
        std::fs::write(directory.join("wrapper.c"), source).unwrap();
        Self(directory)
    }
}
impl Drop for SourceFixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

/// The real provider specifications of `rd_calloc` and `rd_strdup`, with
/// their asserting bodies in the unit; `size` is the allocation's size
/// argument (`sizeof(*e)` reaches the exemption, a literal does not).
fn provider_constructor(size: &str) -> LegacyFunctionSummary {
    provider_row(size, "build")
}

fn provider_row(size: &str, function: &str) -> LegacyFunctionSummary {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let contracts =
        load_resource_contract_manifest(&root.join("cycle19-resource-contracts.json")).unwrap();
    let source = format!(
        r#"
#include <assert.h>
typedef __SIZE_TYPE__ size_t;
void *calloc(size_t, size_t); char *strdup(const char *);
struct envelope {{ char *name; int state; }};
static inline void *rd_calloc(size_t count, size_t size) {{
    void *p = calloc(count, size); assert(p); return p;
}}
static inline char *rd_strdup(const char *str) {{
    char *p = strdup(str); assert(p); return p;
}}
struct envelope *build(const char *name) {{
    struct envelope *e = rd_calloc(1, {size});
    e->name = rd_strdup(name); e->state = 1; return e;
}}
struct envelope *make(void) {{ return build("x"); }}
int answer(int x) {{ return x + 1; }}
"#
    );
    let fixture = SourceFixture::new(&source);
    summary(
        analyze_unrestricted_c11(
            &C11TranslationUnit {
                source: &source,
                dialect: CCompilerDialect::Gnu17,
                input: CSourceInput::DeclaredFile {
                    spelled: "wrapper.c".into(),
                    directory: fixture.0.clone(),
                },
                ..Default::default()
            },
            function,
            &contracts,
        )
        .unwrap(),
        function,
    )
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent RULE37-PILOT test: {reason}");
            false
        }
    }
}

fn run_child(test: &str, list: &str) {
    let status = Command::new(std::env::current_exe().expect("the test binary"))
        .args([test, "--exact", "--nocapture", "--test-threads=1"])
        .env(CHILD, test)
        .env(gateway_rules::RULES_OFF_VARIABLE, list)
        .status()
        .expect("the child test runs");
    assert!(status.success(), "{test} with {list:?}: {status}");
}

fn is_child(test: &str) -> bool {
    std::env::var(CHILD).is_ok_and(|named| named == test)
}

#[test]
fn with_nothing_off_rule_37_admits_the_constructor() {
    if std::env::var_os(CHILD).is_some() || !clang_available() {
        return;
    }
    assert!(gateway_rules::effective_off().is_empty());
    let constructor = plain_constructor();
    assert_eq!(constructor.status, AnalysisStatus::Safe, "{constructor:#?}");
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
#[test]
fn with_nothing_off_rule_37_admits_the_provider_constructors() {
    if std::env::var_os(CHILD).is_some() || !clang_available() {
        return;
    }
    for size in ["sizeof(*e)", "16"] {
        let constructor = provider_constructor(size);
        assert_eq!(
            constructor.status,
            AnalysisStatus::Safe,
            "{size}: {constructor:#?}"
        );
        // Card ARM-OBLIGATIONS: the admitted specifications stand in this
        // unit, so every row resting on them carries their admission's
        // premise among its assumptions, and the verdict is unchanged.
        // Card ARM-OBLIGATIONS-UNIT-GRAIN: the rows whose derivation reaches
        // `rd_calloc` or `rd_strdup` are the specifications' own rows, their
        // caller `build` and, through `build`'s in-unit summary, `make`;
        // `answer` reaches neither and carries nothing. The unit is `Safe`,
        // so the certificate recheck (outside any recording) reproduced it.
        for function in ["build", "make", "rd_calloc", "rd_strdup"] {
            let row = provider_row(size, function);
            assert_eq!(
                row.rule_obligations,
                asserted_allocation(),
                "{size} {function}"
            );
        }
        let answer = provider_row(size, "answer");
        assert_eq!(answer.status, AnalysisStatus::Safe, "{answer:#?}");
        assert!(answer.rule_obligations.is_empty(), "{size}: {answer:#?}");
    }
}

/// Rule 37's admission obligation, as a row carries it.
fn asserted_allocation() -> qcore_c_gateway::arm_obligations::Carried {
    qcore_c_gateway::arm_obligations::Carried {
        premises: [qcore_c_gateway::arm_obligations::Carriage {
            rule: "37".into(),
            name: "asserted_allocation".into(),
        }]
        .into(),
        open: Default::default(),
    }
}

#[test]
fn switching_rule_37_off_keeps_the_refusals() {
    if std::env::var_os(CHILD).is_some() || !clang_available() {
        return;
    }
    run_child("child_rule_37_off", "37");
}

#[test]
fn child_rule_37_off() {
    if !is_child("child_rule_37_off") {
        return;
    }
    assert_eq!(
        gateway_rules::effective_off()
            .into_iter()
            .collect::<Vec<_>>(),
        ["37"]
    );
    // The exemption off: `sizeof(*e)` is a mention again, so the store scan
    // refuses the base exactly as before rule 37.
    let constructor = plain_constructor();
    assert_ne!(constructor.status, AnalysisStatus::Safe, "{constructor:#?}");
    assert_eq!(leaves(&constructor), PRE_RULE_37_REFUSAL);
    // The premise off: both provider specifications are withdrawn (their
    // defining rows are Unknown with the withdrawal), and the constructor
    // keeps the store refusal.
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    {
        let constructor = provider_constructor("sizeof(*e)");
        assert_ne!(constructor.status, AnalysisStatus::Safe, "{constructor:#?}");
        assert_eq!(leaves(&constructor), PRE_RULE_37_REFUSAL);
        for wrapper in ["rd_calloc", "rd_strdup"] {
            let row = provider_row("16", wrapper);
            assert_eq!(row.status, AnalysisStatus::Unknown, "{row:#?}");
            assert_eq!(
                leaves(&row),
                [format!(
                    "declared specification of `{wrapper}` is not established: the gateway \
                     rule owning its named premise does not admit it (switched off)"
                )]
            );
        }
        // A withdrawn checked specification leaves its callers whatever W4
        // validates from the body in the unit (card W45-FB-BACKLOG-CUSTODY's
        // existing withdrawal): here the asserting body itself, which is why
        // a constructor with no `sizeof` stays `Safe` with rule 37 off. This
        // is the pre-existing inference, not an admission of rule 37, and its
        // row carries no `asserted_allocation` premise (follow-up
        // RULE37-W4-ASSERT-PREMISE-VISIBILITY).
        let literal = provider_constructor("16");
        assert_eq!(literal.status, AnalysisStatus::Safe, "{literal:#?}");
        // Card ARM-OBLIGATIONS: no specification stands, so no row carries
        // rule 37's premise: nothing rests on a withdrawn admission.
        assert!(literal.rule_obligations.is_empty(), "{literal:#?}");
    }
}

#[test]
fn switching_another_rule_off_leaves_rule_37_admitting() {
    if std::env::var_os(CHILD).is_some() || !clang_available() {
        return;
    }
    run_child("child_other_rule_off", "35");
}

#[test]
fn child_other_rule_off() {
    if !is_child("child_other_rule_off") {
        return;
    }
    let constructor = plain_constructor();
    assert_eq!(constructor.status, AnalysisStatus::Safe, "{constructor:#?}");
}
