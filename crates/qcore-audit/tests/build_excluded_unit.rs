//! Card **CURRENT-RUN-DEFECTS**, defect 2: librdkafka's `tinycthread.c` under
//! the configured build (`WITH_C11THREADS 1` in the generated `config.h`)
//! keeps none of its 30 raw-source definitions after preprocessing. The unit
//! stays `Unknown` with no analysed row and every raw name in
//! `not_projected` (Cycle 19: fail-closed, never data-only or `Safe`); its
//! published reason states that cause instead of reporting the first raw name,
//! the Windows-only `_cnd_timedwait_win32`, as a missing function. Both
//! projection routes (a prepared unit and a plain one) publish the same row.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use qcore_audit::compile_db::{
    CompileEntry, EffectiveInput, compile_task_identity, compiler_options,
};
use qcore_audit::projection::{
    AuditCounts, PreparedAuditCertificateContext, project_prepared_unit, project_unit,
};
use qcore_audit::report::AuditReport;
use qcore_c_gateway::ClangIdentity;
use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, LegacyFunctionScope, enumerate_source_definitions, prepare_translation_unit,
};
use qcore_contract::contract_loader::load_resource_contract_sources;
use qcore_schemas::contract::ResourceContracts;
use serde_json::{Value, json};

static NEXT: AtomicUsize = AtomicUsize::new(0);

/// Every definition compiled out by the unit's own configuration.
const EXCLUDED: &str = concat!(
    "#define WITH_C11THREADS 1\n",
    "#if !WITH_C11THREADS\n",
    "#if defined(_TTHREAD_WIN32_)\n",
    "int _cnd_timedwait_win32(int *c, int t) { return t; }\n",
    "#endif\n",
    "int mtx_lock(int *m) { return *m; }\n",
    "#endif\n",
);

/// One live definition beside a compiled-out one: the old reading, unchanged.
const MIXED: &str = concat!(
    "#if 0\nint hidden(void) { return 0; }\n#endif\n",
    "int live(void) { return 0; }\n",
);

const CONTRACT: &str = r#"module contract;
extern fn q_alloc() -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_return_owned(nonnull);
};
"#;

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "fixture.qcore".to_owned(),
        CONTRACT.to_owned(),
    )]))
    .unwrap()
}

struct Fixture {
    root: PathBuf,
    compiler: ClangIdentity,
}

impl Fixture {
    fn new(source: &str) -> Option<Self> {
        let compiler = match ClangIdentity::capture() {
            Ok(identity) => identity,
            Err(reason) => {
                assert!(
                    std::env::var_os("QCORE_CLANG").is_none(),
                    "explicit QCORE_CLANG is invalid: {reason}"
                );
                return None;
            }
        };
        let root = std::env::temp_dir().join(format!(
            "qcore-build-excluded-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/unit.c"), source).unwrap();
        Some(Self { root, compiler })
    }

    fn entry(&self) -> CompileEntry {
        CompileEntry {
            directory: self.root.clone(),
            command: format!("\"{}\" -std=c11 -c src/unit.c", self.compiler.program()),
            file: self.root.join("src/unit.c"),
        }
    }

    fn unit<'a>(&self, source: &'a str) -> C11TranslationUnit<'a> {
        let options = compiler_options(&self.entry()).unwrap();
        C11TranslationUnit {
            source,
            include_paths: options.include_paths,
            definitions: options.definitions,
            preprocessor_options: options.preprocessor_options,
            dialect: options.dialect,
            function_scope: LegacyFunctionScope::MainSourceDefinitions,
            input: options.input,
        }
    }

    fn report_row(&self, report: &AuditReport, name: &str) -> Value {
        let path = self.root.join(name);
        report.write(&path).unwrap();
        let report: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        report["units"][0].clone()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// The row each route publishes, and the counts it leaves.
fn rows(source: &str) -> Option<[(Value, AuditCounts); 2]> {
    let fixture = Fixture::new(source)?;
    let contracts = contracts();
    let unit = fixture.unit(source);
    // A plain unit.
    let inventory = enumerate_source_definitions(&unit);
    let mut plain = AuditReport::new("fixture-revision");
    let mut plain_counts = AuditCounts::default();
    project_unit(
        "fixture",
        "src/unit.c",
        &unit,
        &inventory,
        &contracts,
        &mut plain,
        &mut plain_counts,
    )
    .unwrap();
    // A prepared unit.
    let prepared = prepare_translation_unit(&unit);
    let identity = compile_task_identity(
        &fixture.entry(),
        &unit,
        EffectiveInput::Accepted(prepared.preprocessed_source().unwrap()),
        &fixture.compiler,
        BTreeMap::new(),
    )
    .unwrap();
    let profile = "0".repeat(64);
    let mut configured = AuditReport::new("fixture-revision");
    let mut configured_counts = AuditCounts::default();
    project_prepared_unit(
        "fixture",
        "src/unit.c",
        &prepared,
        &contracts,
        PreparedAuditCertificateContext {
            compile_task_identity: &identity,
            source_profile_sha256: &profile,
        },
        &mut configured,
        &mut configured_counts,
    )
    .unwrap();
    Some([
        (fixture.report_row(&plain, "plain.json"), plain_counts),
        (
            fixture.report_row(&configured, "configured.json"),
            configured_counts,
        ),
    ])
}

#[test]
fn a_compiled_out_unit_stays_unknown_and_states_why() {
    let Some(rows) = rows(EXCLUDED) else {
        return;
    };
    for (row, counts) in rows {
        assert_eq!(row["status"], "Unknown", "{row}");
        assert_eq!(counts.units, 1);
        assert_eq!(counts.unknown_units, 1);
        assert_eq!(counts.data_only_units, 0, "never data-only");
        assert_eq!(counts.safe_units, 0);
        assert_eq!(counts.source_functions, 2, "the raw rows stay counted");
        assert_eq!(
            row["source_functions"],
            json!(["_cnd_timedwait_win32", "mtx_lock"])
        );
        assert_eq!(row["not_projected"], row["source_functions"]);
        assert_eq!(row["functions"], json!([]));
        let reason = row["reason"].as_str().unwrap();
        assert!(
            reason.starts_with("no main-source definition remains after preprocessing"),
            "{reason}"
        );
        assert!(
            reason.contains("its 2 raw-source definition(s)"),
            "{reason}"
        );
        assert!(!reason.starts_with("C function"), "{reason}");
        assert_eq!(row["reason_code"], "incomplete-obligation-coverage");
    }
}

#[test]
fn a_unit_with_a_live_definition_keeps_its_reading() {
    let Some(rows) = rows(MIXED) else {
        return;
    };
    for (row, counts) in rows {
        assert_eq!(counts.data_only_units, 0);
        assert_eq!(row["not_projected"], json!(["hidden"]), "{row}");
        assert_eq!(row["functions"][0]["function"], "live", "{row}");
        let reason = row["reason"].as_str().unwrap_or_default();
        assert!(!reason.contains("build-excluded"), "{reason}");
    }
}
