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

//! Card W45-USER-CONTRACT-PLUGINS — the gateway's function and unit statuses
//! are folds of the four-value verdict semigroup, and the fold agrees with the
//! three-way decision it replaced on every status that decision could produce
//! (`docs/reports/w45-user-contract-plugins/README.md` §3).
//!
//! **The family is held out.** A `k_*` API unrelated to every production
//! library, declared inline; no production contract moves.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::verdict::{classify, diagnostic_verdict, is_held_call};
use qcore_c_gateway::legacy_c::{
    AnalysisStatus, LegacyDiagnostic, LegacyDiagnosticKind, LegacyProgramSummary,
    ResourceContracts, analyze_legacy_c_translation_unit, hold_description, pin_hold,
};
use qcore_contract::contract_loader::load_resource_contract_sources;
use qcore_schemas::verdict::Verdict;

/// The decision `classify_diagnostics` made before this card, verbatim in
/// shape: a violation, else any unresolved construct (a held call included),
/// else `Safe`.
fn historical(diagnostics: &[LegacyDiagnostic]) -> AnalysisStatus {
    let unsafe_found = diagnostics.iter().any(|diagnostic| {
        matches!(
            diagnostic.kind,
            LegacyDiagnosticKind::Leak { .. }
                | LegacyDiagnosticKind::DoubleRelease { .. }
                | LegacyDiagnosticKind::ReleaseAfterEscape { .. }
                | LegacyDiagnosticKind::CustodyConflict { .. }
                | LegacyDiagnosticKind::InvalidRelease { .. }
                | LegacyDiagnosticKind::UseAfterRelease { .. }
                | LegacyDiagnosticKind::KindMismatchedRelease { .. }
                | LegacyDiagnosticKind::UseAfterClose { .. }
                | LegacyDiagnosticKind::CountedUser { .. }
        )
    });
    let unknown_found = diagnostics.iter().any(|diagnostic| {
        matches!(
            diagnostic.kind,
            LegacyDiagnosticKind::UnknownAlias { .. }
                | LegacyDiagnosticKind::Unsupported { .. }
                | LegacyDiagnosticKind::SummaryConflict { .. }
        )
    });
    if unsafe_found {
        AnalysisStatus::Unsafe
    } else if unknown_found {
        AnalysisStatus::Unknown
    } else {
        AnalysisStatus::Safe
    }
}

fn diagnostic(kind: LegacyDiagnosticKind) -> LegacyDiagnostic {
    LegacyDiagnostic {
        function: "w".into(),
        line: 1,
        kind,
    }
}

/// One diagnostic of every verdict class, the two held-call refusals among
/// them.
fn kinds() -> Vec<LegacyDiagnosticKind> {
    vec![
        LegacyDiagnosticKind::Leak {
            acquisition_line: 1,
        },
        LegacyDiagnosticKind::DoubleRelease {
            acquisition_line: 1,
        },
        LegacyDiagnosticKind::CountedUser {
            description: "restored".into(),
        },
        LegacyDiagnosticKind::UnknownAlias {
            variable: "p".into(),
        },
        LegacyDiagnosticKind::Unsupported {
            description: "a goto".into(),
        },
        LegacyDiagnosticKind::Unsupported {
            description: hold_description("k_close"),
        },
        LegacyDiagnosticKind::Unsupported {
            description: pin_hold::hold_description("k_open"),
        },
    ]
}

#[test]
fn the_fold_is_the_historical_decision_except_that_a_held_call_alone_is_pending() {
    let kinds = kinds();
    for mask in 0u32..(1 << kinds.len()) {
        let diagnostics: Vec<_> = kinds
            .iter()
            .enumerate()
            .filter(|(index, _)| mask & (1 << index) != 0)
            .map(|(_, kind)| diagnostic(kind.clone()))
            .collect();
        let before = historical(&diagnostics);
        let after = classify(&diagnostics);
        let held = diagnostics.iter().any(|diagnostic| {
            matches!(&diagnostic.kind, LegacyDiagnosticKind::Unsupported { description }
                if is_held_call(description))
        });
        let other_unresolved = diagnostics
            .iter()
            .any(|diagnostic| diagnostic_verdict(&diagnostic.kind) == Verdict::Unknown);
        let expected = match before {
            AnalysisStatus::Unknown if held && !other_unresolved => AnalysisStatus::Pending,
            other => other,
        };
        assert_eq!(after, expected, "{diagnostics:?}");
        // Never towards `Safe`, never across `Unsafe`, never above before.
        assert_eq!(
            after == AnalysisStatus::Safe,
            before == AnalysisStatus::Safe
        );
        assert_eq!(
            after == AnalysisStatus::Unsafe,
            before == AnalysisStatus::Unsafe
        );
        assert!(after.verdict() <= before.verdict());
    }
}

#[test]
fn every_diagnostic_is_monotone_in_the_fold() {
    let kinds = kinds();
    for kind in &kinds {
        let alone = classify(&[diagnostic(kind.clone())]);
        assert_eq!(alone.verdict(), diagnostic_verdict(kind));
        for other in &kinds {
            let both = classify(&[diagnostic(kind.clone()), diagnostic(other.clone())]);
            assert!(both.verdict() >= alone.verdict(), "{kind:?} + {other:?}");
        }
    }
}

const CONTRACTS: &str = "module contract;

extern fn k_open() -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_return_owned(nullable);
};

extern fn k_close(h: ptr<mut, u8>, n: i32) -> unit
contract {
    requires resource_contract_v1;
    requires resource_review_pending(\"is the handle freed on every path?\");
    ensures normal resource_released(h, nonnull);
    ensures normal resource_borrowed(n);
    ensures normal resource_return_none();
};

extern fn k_free(h: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_released(h, nonnull);
    ensures normal resource_return_none();
};
";

const PRELUDE: &str = "void *k_open(void);\nvoid k_close(void *h, int n);\nvoid k_free(void *h);\n";
/// Held: closes through the contract awaiting review.
const HELD: &str = "void w(void) { void *h; h = k_open(); if (!h) { return; } k_close(h, 0); }";
/// Safe: frees through the contract in force.
const SAFE: &str = "void v(void) { void *h; h = k_open(); if (!h) { return; } k_free(h); }";
/// Unknown: hands the handle to an undeclared callee.
const UNKNOWN: &str =
    "void u(void) { void *h; h = k_open(); if (!h) { return; } mystery(h); k_free(h); }";
/// Unsafe: frees twice through the contract in force.
const TWICE: &str =
    "void x(void) { void *h; h = k_open(); if (!h) { return; } k_free(h); k_free(h); }";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "k.qcore".to_owned(),
        CONTRACTS.to_owned(),
    )]))
    .expect("the fixture module loads")
}

fn analyse(bodies: &[&str]) -> LegacyProgramSummary {
    let source = format!("{PRELUDE}{}", bodies.join("\n"));
    let entry = bodies[0]
        .strip_prefix("void ")
        .and_then(|rest| rest.split_once('('))
        .map(|(name, _)| name)
        .expect("a definition");
    analyze_legacy_c_translation_unit(&source, entry, &contracts()).expect("the fixture tokenizes")
}

#[test]
fn a_unit_status_is_the_fold_of_its_function_statuses() {
    for (bodies, expected) in [
        (vec![SAFE], AnalysisStatus::Safe),
        (vec![HELD], AnalysisStatus::Pending),
        (vec![HELD, SAFE], AnalysisStatus::Pending),
        (vec![HELD, SAFE, UNKNOWN], AnalysisStatus::Unknown),
        (vec![SAFE, UNKNOWN], AnalysisStatus::Unknown),
        (vec![HELD, UNKNOWN, TWICE], AnalysisStatus::Unsafe),
        (vec![SAFE, TWICE], AnalysisStatus::Unsafe),
    ] {
        let summary = analyse(&bodies);
        let statuses: Vec<_> = summary
            .functions
            .values()
            .map(|function| function.status)
            .collect();
        assert_eq!(statuses.len(), bodies.len(), "{bodies:?}");
        assert_eq!(summary.status, expected, "{bodies:?}: {statuses:?}");
        assert_eq!(
            summary.status.verdict(),
            Verdict::fold(statuses.iter().map(|status| status.verdict())),
            "{bodies:?}"
        );
        // The historical unit decision on the same function statuses, with a
        // `Pending` function read as the `Unknown` it used to be.
        let historical = if statuses.contains(&AnalysisStatus::Unsafe) {
            AnalysisStatus::Unsafe
        } else if statuses
            .iter()
            .any(|status| matches!(status, AnalysisStatus::Unknown | AnalysisStatus::Pending))
        {
            AnalysisStatus::Unknown
        } else {
            AnalysisStatus::Safe
        };
        match summary.status {
            AnalysisStatus::Pending => assert_eq!(historical, AnalysisStatus::Unknown),
            status => assert_eq!(status, historical, "{bodies:?}"),
        }
    }
}
