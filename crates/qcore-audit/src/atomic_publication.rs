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

//! Card CG19-ATOMIC-SOURCE-ROUTE (gateway rule 11i): the production caller
//! of the bounded atomic-publication judgment,
//! `quirin-audit atomic-publication CONFIG.json` (absorbs
//! CT41-ROUTE-HAS-NO-PRODUCTION-CALLER).
//!
//! The configuration selects one C translation unit and a publication
//! profile (publisher, consumer, flag, payload) with the caller-declared
//! invocation bounds. The route runs
//! `qcore_verification::atomic_publication_source::atomic_publication_verdict`
//! and writes a report carrying the verdict contract's fields. The verdict is
//! `Safe` exactly when the source extraction and the core weak-memory model
//! both accept; any refusal, or any input that cannot be read, is `Unknown`
//! and the command exits non-zero. Every report carries the single-graph
//! assumption: one checked candidate execution is not race freedom over every
//! execution (W07, CG29), and nothing about liveness or eventual observation
//! is claimed (W11).

use std::fs;
use std::path::{Path, PathBuf};

use qcore_c_gateway::legacy_c::C11TranslationUnit;
use qcore_verification::atomic_publication_source::{
    AtomicPublicationExecutionBounds, AtomicPublicationSourceProfile, AtomicPublicationVerdict,
    atomic_publication_verdict,
};
use serde::{Deserialize, Serialize};

pub const USAGE: &str = "quirin-audit atomic-publication CONFIG.json";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AtomicPublicationConfig {
    pub schema_version: u16,
    /// A label for the artifact; the report binds the source bytes by digest.
    pub artifact: String,
    /// The C source, relative to the configuration's directory.
    pub source: PathBuf,
    pub source_revision: String,
    #[serde(default)]
    pub include_paths: Vec<String>,
    #[serde(default)]
    pub definitions: Vec<String>,
    pub publisher: String,
    pub consumer: String,
    pub flag: String,
    pub payload: String,
    /// Caller-declared premises, recorded in the report as bounds.
    pub publisher_invocations: usize,
    pub successful_consumer_invocations: usize,
    /// The report, relative to the configuration's directory.
    pub report: PathBuf,
}

#[derive(Debug, Serialize)]
struct Stages {
    initialise: String,
    publish: String,
    observe: String,
    use_: String,
    reclaim: String,
}

#[derive(Debug, Serialize)]
struct AtomicPublicationReport {
    artifact: String,
    property: String,
    semantic_profile: String,
    scope: String,
    bounds: Vec<String>,
    assumptions: Vec<String>,
    evidence_version: String,
    source_revision: String,
    source_sha256: String,
    result: String,
    refusal: Option<String>,
    stages: Option<Stages>,
}

fn resolve(base: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_owned()
    } else {
        base.join(path)
    }
}

/// Run the configured judgment and write its report. `Ok(true)` is `Safe`,
/// `Ok(false)` a written `Unknown`; `Err` is a configuration or input that
/// could not be read (also `Unknown`, with no report).
pub fn run_atomic_publication_config(path: &Path) -> Result<bool, String> {
    let config_path = path.canonicalize().map_err(|error| error.to_string())?;
    let base = config_path.parent().ok_or("configuration has no parent")?;
    let config: AtomicPublicationConfig =
        serde_json::from_slice(&fs::read(&config_path).map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())?;
    if config.schema_version != 1 || config.artifact.is_empty() {
        return Err("unsupported atomic-publication configuration".into());
    }
    let source_path = resolve(base, &config.source);
    let source = fs::read_to_string(&source_path)
        .map_err(|error| format!("read {}: {error}", source_path.display()))?;
    let unit = C11TranslationUnit {
        source: &source,
        include_paths: config.include_paths.clone(),
        definitions: config.definitions.clone(),
        ..Default::default()
    };
    let verdict = atomic_publication_verdict(
        &unit,
        &config.source_revision,
        &AtomicPublicationSourceProfile {
            publisher: &config.publisher,
            consumer: &config.consumer,
            flag: &config.flag,
            payload: &config.payload,
        },
        AtomicPublicationExecutionBounds {
            publisher_invocations: config.publisher_invocations,
            successful_consumer_invocations: config.successful_consumer_invocations,
        },
    );
    let safe = matches!(verdict, AtomicPublicationVerdict::Safe(_));
    let (property, semantic_profile, scope, evidence_version, refusal, stages) = match &verdict {
        AtomicPublicationVerdict::Safe(certificate) => (
            certificate.property,
            certificate.semantic_profile,
            certificate.scope,
            certificate.evidence_version,
            None,
            Some(Stages {
                initialise: certificate.source.stages.initialise.clone(),
                publish: certificate.source.stages.publish.clone(),
                observe: certificate.source.stages.observe.clone(),
                use_: certificate.source.stages.use_.clone(),
                reclaim: certificate.source.stages.reclaim.to_owned(),
            }),
        ),
        AtomicPublicationVerdict::Unknown { refusal, .. } => (
            "successful release/acquire publication orders one payload write before one payload read",
            "c11-bounded-atomic-publication-v1",
            "the selected full preprocessed translation unit, exactly one declared publisher invocation, and exactly one successful selected consumer invocation",
            "ct41-sync-model-v1",
            Some(format!("{refusal:?}")),
            None,
        ),
    };
    let report = AtomicPublicationReport {
        artifact: config.artifact.clone(),
        property: property.to_owned(),
        semantic_profile: semantic_profile.to_owned(),
        scope: scope.to_owned(),
        bounds: vec![
            format!("publisher_invocations = {}", config.publisher_invocations),
            format!(
                "successful_consumer_invocations = {}",
                config.successful_consumer_invocations
            ),
        ],
        assumptions: verdict
            .assumptions()
            .iter()
            .map(|assumption| (*assumption).to_owned())
            .collect(),
        evidence_version: evidence_version.to_owned(),
        source_revision: config.source_revision.clone(),
        source_sha256: qcore_schemas::sha256_hex(source.as_bytes()),
        result: verdict.status().to_owned(),
        refusal,
        stages,
    };
    let report_path = resolve(base, &config.report);
    fs::write(
        &report_path,
        serde_json::to_vec_pretty(&report).map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("write {}: {error}", report_path.display()))?;
    Ok(safe)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOURCE: &str = "#include <stdatomic.h>
static _Atomic int done = 0;
static int payload;
void publish(void) { payload = 7; atomic_store_explicit(&done, 1, memory_order_release); }
int consume(void) { int seen = atomic_load_explicit(&done, memory_order_acquire); if (seen != 1) return 0; return payload; }
";

    fn run(source: &str, directory: &Path) -> (Result<bool, String>, serde_json::Value) {
        fs::create_dir_all(directory).unwrap();
        fs::write(directory.join("case.c"), source).unwrap();
        fs::write(
            directory.join("config.json"),
            serde_json::json!({
                "schema_version": 1,
                "artifact": "cg19-fixture",
                "source": "case.c",
                "source_revision": "fixture",
                "publisher": "publish",
                "consumer": "consume",
                "flag": "done",
                "payload": "payload",
                "publisher_invocations": 1,
                "successful_consumer_invocations": 1,
                "report": "report.json",
            })
            .to_string(),
        )
        .unwrap();
        let outcome = run_atomic_publication_config(&directory.join("config.json"));
        let report = fs::read(directory.join("report.json"))
            .map(|bytes| serde_json::from_slice(&bytes).unwrap())
            .unwrap_or(serde_json::Value::Null);
        (outcome, report)
    }

    fn clang_available() -> bool {
        match qcore_c_gateway::clang_command() {
            Ok(_) => true,
            Err(reason) => {
                assert!(
                    std::env::var_os("QCORE_CLANG").is_none(),
                    "invalid explicit Clang: {reason}"
                );
                false
            }
        }
    }

    fn clang_atomic_ast_available() -> bool {
        if !clang_available() {
            return false;
        }
        match qcore_c_gateway::clang_version() {
            Ok(version) if version.supports_cg19_atomic_ast() => true,
            Ok(version) => {
                eprintln!(
                    "skipping atomic-publication AST observation on Clang {}.{}.{}: CG19 requires Clang 18 or newer",
                    version.major, version.minor, version.patch
                );
                false
            }
            Err(reason) => {
                panic!("Clang was resolved but its release could not be detected: {reason}")
            }
        }
    }

    /// The production route: `Safe` with the derived stages and (card CG29,
    /// rule 11z) the all-executions assumption, and a refused control
    /// `Unknown` with the single-graph assumption.
    #[test]
    fn cg19_atomic_publication_route_reports_safe_and_unknown() {
        if !clang_atomic_ast_available() {
            return;
        }
        let root =
            std::env::temp_dir().join(format!("cg19-atomic-publication-{}", std::process::id()));
        let single_graph = qcore_c_gateway::atomic_source_route::SINGLE_GRAPH_ASSUMPTION;
        let all_executions = qcore_c_gateway::atomic_source_route::ALL_EXECUTIONS_ASSUMPTION;
        let (safe, report) = run(SOURCE, &root.join("safe"));
        assert_eq!(safe, Ok(true));
        assert_eq!(report["result"], "Safe");
        assert!(
            report["stages"]["publish"]
                .as_str()
                .unwrap()
                .contains("Release")
        );
        let assumptions = report["assumptions"].as_array().unwrap();
        assert!(assumptions.iter().any(|a| a == all_executions));
        assert!(!assumptions.iter().any(|a| a == single_graph));
        let relaxed = SOURCE.replace("memory_order_release", "memory_order_relaxed");
        let (unknown, report) = run(&relaxed, &root.join("relaxed"));
        assert_eq!(unknown, Ok(false));
        assert_eq!(report["result"], "Unknown");
        assert!(
            report["refusal"]
                .as_str()
                .unwrap()
                .contains("StoreIsNotRelease")
        );
        assert!(
            report["assumptions"]
                .as_array()
                .unwrap()
                .iter()
                .any(|a| a == single_graph)
        );
        let _ = fs::remove_dir_all(&root);
    }
}
