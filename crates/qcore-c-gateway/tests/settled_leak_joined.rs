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

//! Card SETTLED-LEAK-JOINED-GENUINE-WITHDRAWN (gateway rule 84): a leak rule
//! 74c (A) withdraws is published when a path avoiding every mark of its
//! identity, and every refusal, carries it.
//!
//! Every fixture runs the real Clang-backed gateway over the generic `k_*`
//! descriptor and stream surface (`fixtures/contracts/descriptor_generic.qv`:
//! `k_open` returns a sentinel descriptor, `k_close` releases a valid one,
//! `k_fopen` returns a nullable stream, `k_fclose` releases a non-null one)
//! and uncontracted `u_*` calls. Each `Unsafe` is provisional, over synthetic
//! trusted test assumptions.

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
    let source =
        format!("{DECLARATIONS}\nint w(const char *path, int flag, int n) {{\n{body}\n}}\n");
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

const NULLABLE_FD: &str = "nullable resource `fd` used by a non-null contract";
const NULLABLE_S: &str = "nullable resource `s` used by a non-null contract";

impl Run {
    fn refused(&self, description: &str) -> bool {
        self.diagnostics.iter().any(|kind| {
            matches!(kind, LegacyDiagnosticKind::Unsupported { description: d } if d == description)
        })
    }
}

/// Rule 84's re-published leaks: a path from the acquisition to an exit
/// avoids every refused release (and every other mark) of the identity and
/// keeps it live, so the leak on it is published beside the refused close's
/// own refusal. Each body was `Unknown` with no leak at `43410b230`; the
/// pinned instance is `condition_call_desugar`'s.
#[test]
fn a_path_avoiding_every_refused_release_publishes_the_leak() {
    if !clang_available() {
        return;
    }
    for (name, body, refusal) in [
        // `flag` false: the close never runs, a valid descriptor stays open.
        (
            "pinned",
            "int fd = k_open(path); if (flag && k_close(fd) == 0) { } return 0;",
            NULLABLE_FD,
        ),
        (
            "one arm",
            "int fd = k_open(path); if (flag) { k_close(fd); } return 0;",
            NULLABLE_FD,
        ),
        // The loop's exit edge before the first iteration.
        (
            "in a loop",
            "int fd = k_open(path); while (n > 0) { k_close(fd); n = u_read(); } return 0;",
            NULLABLE_FD,
        ),
        // Neither arm of two runs.
        (
            "two refused closes",
            "int fd = k_open(path); if (flag) { k_close(fd); } else if (n) { k_close(fd); } return 0;",
            NULLABLE_FD,
        ),
        (
            "an early return",
            "int fd = k_open(path); if (flag) { k_close(fd); } if (n) { return 1; } return 0;",
            NULLABLE_FD,
        ),
        // A nullable stream closed `nonnull` on one arm.
        (
            "a stream on one arm",
            "char *s = k_fopen(path); if (flag) { k_fclose(s); } return 0;",
            NULLABLE_S,
        ),
    ] {
        let run = analyze(body);
        assert_eq!(run.status, AnalysisStatus::Unsafe, "{name}: {run:?}");
        assert_eq!(run.leaks(), 1, "{name}: {run:?}");
        assert!(run.refused(refusal), "{name}: {run:?}");
    }
}

/// No path avoiding every mark keeps the identity live, so rule 74c (A)'s
/// withdrawal stands: `Unknown`, no leak, the refusal kept.
#[test]
fn without_an_avoiding_live_path_the_withdrawal_stands() {
    if !clang_available() {
        return;
    }
    for (name, body) in [
        // The only path passes the refused close.
        ("only path", "int fd = k_open(path); k_close(fd); return 0;"),
        (
            "both arms",
            "int fd = k_open(path); if (flag) { k_close(fd); } else { k_close(fd); } return 0;",
        ),
        // The body runs at least once.
        (
            "do while",
            "int fd = k_open(path); do { k_close(fd); n = u_read(); } while (n > 0); return 0;",
        ),
        // The other arm hands the stream to an uncontracted call: a refused
        // hand-on marks it too.
        (
            "another refusal",
            "char *s = k_fopen(path); if (flag) { k_fclose(s); } else { u_keep(s); } return 0;",
        ),
        // The avoiding arm closes a valid descriptor and owes nothing on the
        // invalid one.
        (
            "the avoiding arm discharges",
            "int fd = k_open(path); if (flag) { k_close(fd); } else if (fd >= 0) { k_close(fd); } return 0;",
        ),
        // The avoiding arm hands the descriptor to the caller.
        (
            "the avoiding arm returns it",
            "int fd = k_open(path); if (flag) { k_close(fd); return -1; } return fd;",
        ),
    ] {
        let run = analyze(body);
        assert_eq!(run.status, AnalysisStatus::Unknown, "{name}: {run:?}");
        assert_eq!(run.leaks(), 0, "{name}: {run:?}");
        assert!(
            run.refused(NULLABLE_FD) || run.refused(NULLABLE_S),
            "{name}: {run:?}"
        );
    }
}
