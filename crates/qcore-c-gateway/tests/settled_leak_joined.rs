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

//! Card SETTLED-LEAK-JOINED-GENUINE-WITHDRAWN: probes of rule 74c (A)'s
//! withdrawal when a genuine leak's path joins a refused release's path.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary,
    ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const DECLARATIONS: &str = "\
int k_open(const char *path); int k_close(int fd); \
char *k_fopen(const char *path); int k_fclose(char *stream); \
void u_keep(char *stream); int u_read(void);";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "descriptor_generic.qcore".to_owned(),
        include_str!("fixtures/contracts/descriptor_generic.qv").to_owned(),
    )]))
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
            eprintln!("skipping Clang-dependent test: {reason}");
            false
        }
    }
}

#[derive(Debug)]
struct Run {
    status: AnalysisStatus,
    diagnostics: Vec<LegacyDiagnosticKind>,
}

impl Run {
    fn leaks(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|kind| matches!(kind, LegacyDiagnosticKind::Leak { .. }))
            .count()
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

/// `body` is `int w(const char *path, int flag, int n)`'s body.
fn analyze(body: &str) -> Run {
    let source = format!("{DECLARATIONS}\nint w(const char *path, int flag, int n) {{\n{body}\n}}\n");
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        "w",
        &contracts(),
    )
    .unwrap_or_else(|error| panic!("valid fixture rejected by Clang: {error:?}\n{source}"));
    if let UnrestrictedLegacyResult::Safe(certificate) = &outcome {
        check_legacy_safety_certificate(certificate).expect("a Safe result rechecks");
    }
    let result = &summary_of(&outcome).functions["w"];
    Run {
        status: result.status,
        diagnostics: result
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.kind.clone())
            .collect(),
    }
}

const PROBES: &[(&str, &str)] = &[
    (
        "pinned",
        "int fd = k_open(path); if (flag && k_close(fd) == 0) { } return 0;",
    ),
    (
        "one_arm",
        "int fd = k_open(path); if (flag) { k_close(fd); } return 0;",
    ),
    (
        "in_loop",
        "int fd = k_open(path); while (n > 0) { k_close(fd); n = u_read(); } return 0;",
    ),
    (
        "two_refused",
        "int fd = k_open(path); if (flag) { k_close(fd); } else if (n) { k_close(fd); } return 0;",
    ),
    (
        "early_return",
        "int fd = k_open(path); if (flag) { k_close(fd); } if (n) { return 1; } return 0;",
    ),
    (
        "stream_one_arm",
        "char *s = k_fopen(path); if (flag) { k_fclose(s); } return 0;",
    ),
    ("control_only_path", "int fd = k_open(path); k_close(fd); return 0;"),
    (
        "control_both_arms",
        "int fd = k_open(path); if (flag) { k_close(fd); } else { k_close(fd); } return 0;",
    ),
    (
        "control_do_while",
        "int fd = k_open(path); do { k_close(fd); n = u_read(); } while (n > 0); return 0;",
    ),
    (
        "control_other_refusal",
        "char *s = k_fopen(path); if (flag) { k_fclose(s); } else { u_keep(s); } return 0;",
    ),
];

#[test]
fn probe() {
    if !clang_available() {
        return;
    }
    for (name, body) in PROBES {
        let run = analyze(body);
        eprintln!(
            "PROBE {name}: {:?} leaks={} {:?}",
            run.status,
            run.leaks(),
            run.diagnostics
        );
    }
}
