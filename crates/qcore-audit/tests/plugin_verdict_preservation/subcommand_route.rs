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

//! Card W48-PLUGIN-VERDICT-PRESERVATION, the external-subcommand route end to
//! end (the route the OpenSSL and libcurl contract plugins take): a test
//! plugin's claims beside an audit report's four host values, to the
//! published report rows, each of which is projected back to its
//! contributors (`qcore_audit::plugin_verdicts::project_row`).
//!
//! The command line has no trusted checker for a subcommand plugin, so a
//! plugin's `Safe` or `Unsafe` is `Pending` here (W45-SUBCOMMAND-CERTIFICATE-
//! ROUTE): the four values reach a row from the host, and every plugin value
//! the firewall can give (`Pending`, `Unknown`) is traced beside each of them.
//!
//! Included as a module of `subcommand_plugins`, so it runs in that binary's
//! exclusive slot (`scripts/run_test_binaries.py`).

use std::path::{Path, PathBuf};

use qcore_audit::plugin_verdicts::{Recovered, Source, SourceKind, of_source, project_row};
use qcore_audit::subcommand_plugins::HOST_SOURCE;
use qcore_schemas::verdict::Verdict;
use serde_json::{Value, json};

use super::{emit, plugin, report, response, run, scratch, serial, stderr};

/// An audit report with one unit row per `(file, status, reason_code)`.
fn audit_report(directory: &Path, rows: &[(&str, &str, Option<&str>)]) -> PathBuf {
    let path = directory.join("audit.json");
    let units = rows
        .iter()
        .map(|(file, status, code)| {
            json!({"library": "l", "file": file, "status": status,
                   "reason": format!("host says {status}"), "reason_code": code})
        })
        .collect::<Vec<_>>();
    std::fs::write(&path, json!({ "units": units }).to_string()).unwrap();
    path
}

fn claim(artifact: &str, verdict: &str, reason: &str) -> Value {
    json!({
        "artifact": artifact, "property": "resource-custody",
        "semantic_profile": "test.v1", "scope": "whole unit", "bounds": "single path",
        "assumptions": {}, "evidence_version": 1, "verdict": verdict,
        "reason": reason, "certificates": [],
    })
}

fn row<'a>(report: &'a Value, artifact: &str) -> &'a Value {
    report["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["artifact"] == artifact)
        .unwrap_or_else(|| panic!("no row {artifact}"))
}

/// The row's records, checked to compose to it.
fn records(report: &Value, artifact: &str) -> Vec<Recovered> {
    project_row(row(report, artifact)).unwrap_or_else(|error| panic!("{artifact}: {error}"))
}

/// The plugin's own records in a row: its proposal and reason, and the
/// firewall's value for it.
fn plugin_records<'a>(records: &'a [Recovered], label: &str) -> Vec<&'a Recovered> {
    of_source(records, &Source::new(SourceKind::Plugin, label))
}

/// Every value of the host beside each plugin value the firewall can give on
/// the command line, and two claims disagreeing about one artifact.
#[test]
fn every_row_keeps_and_recovers_its_contributors() {
    let _serial = serial();
    let directory = scratch("verdict-preservation");
    let host = audit_report(
        &directory,
        &[
            ("safe.c", "Safe", None),
            ("pending.c", "Pending", Some("contract-pin-mismatch")),
            ("unknown.c", "Unknown", None),
            ("unsafe.c", "Unsafe", Some("residual-custody")),
            ("unclaimed.c", "Safe", None),
        ],
    );
    let claims = json!([
        // A plugin Safe beside a host Safe: Pending, never Safe.
        claim("safe.c", "Safe", "every path returns the item"),
        // Two claims disagreeing about one artifact.
        claim("pending.c", "Safe", "the pool is balanced"),
        claim("pending.c", "Unknown", "the second checker gave up"),
        // A plugin Unsafe beside a host Unknown: the join is Unknown.
        claim("unknown.c", "Unsafe", "a leak on the error path"),
        // Nothing a plugin says lowers a host Unsafe.
        claim("unsafe.c", "Safe", "the plugin disagrees with the host"),
        // An artifact the host has no row for.
        claim("new.c", "Unsafe", "an artifact the host never audited"),
    ]);
    plugin(
        &directory,
        "preserve",
        &emit(&response(&json!([]), &claims)),
    );
    let output = run(
        &directory,
        &["--audit-report", host.to_str().unwrap()],
        "preserve",
        &[],
    );
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let report = report(&directory, "preserve");
    let label = report["plugin"]["label"].as_str().unwrap().to_owned();
    let expected = [
        ("safe.c", Verdict::Pending),
        ("pending.c", Verdict::Unknown),
        ("unknown.c", Verdict::Unknown),
        ("unsafe.c", Verdict::Unsafe),
        ("unclaimed.c", Verdict::Safe),
        ("new.c", Verdict::Pending),
    ];
    for (artifact, verdict) in expected {
        assert_eq!(
            row(&report, artifact)["verdict"],
            verdict.as_str(),
            "{artifact}"
        );
        let records = records(&report, artifact);
        // The host's own value and reasons come back whole.
        let host = of_source(&records, &Source::new(SourceKind::Host, HOST_SOURCE));
        if artifact == "new.c" {
            assert!(host.is_empty());
        } else {
            assert_eq!(host.len(), 1, "{artifact}");
            assert_eq!(
                host[0].verdict.as_str(),
                row(&report, artifact)["host"]["verdict"]
            );
        }
        // The completed run contributes Safe, the identity.
        let run = of_source(&records, &Source::new(SourceKind::Run, label.clone()));
        assert_eq!(run.len(), 1);
        assert_eq!(run[0].verdict, Verdict::Safe);
    }
    // Every claim comes back under the plugin, with what it proposed, its
    // own reason and the firewall's value and reasons.
    for (index, claim) in claims.as_array().unwrap().iter().enumerate() {
        let artifact = claim["artifact"].as_str().unwrap();
        let records = records(&report, artifact);
        let own = plugin_records(&records, &label);
        let matching = own
            .iter()
            .find(|record| record.answer.as_ref().unwrap().reasons[0].detail == claim["reason"])
            .unwrap_or_else(|| panic!("claim {index} is not recovered: {own:#?}"));
        let answer = matching.answer.as_ref().unwrap();
        assert_eq!(answer.proposed.as_deref(), claim["verdict"].as_str());
        let (class, code, verdict) = match claim["verdict"].as_str().unwrap() {
            "Safe" => ("uncertified-safe", Some("QP008"), Verdict::Pending),
            "Unsafe" => ("unvalidated-unsafe", Some("QP010"), Verdict::Pending),
            _ => ("undecided", None, Verdict::Unknown),
        };
        assert_eq!(answer.claim, class);
        assert_eq!(answer.code.as_deref(), code);
        assert_eq!(matching.verdict, verdict);
        assert_eq!(
            matching.reasons,
            serde_json::from_value::<Vec<_>>(
                report["claims"][index]["firewall"]["reasons"].clone()
            )
            .unwrap()
        );
    }
    // Two claims disagreeing: both are recovered, and the row is their join
    // with the host's.
    assert_eq!(
        plugin_records(&records(&report, "pending.c"), &label).len(),
        2
    );
    // The report's verdict is the join of every row and the run.
    assert_eq!(report["verdict"], "Unsafe");
}

/// A failing plugin: the run is Unknown in every row beside every host value,
/// its claims are recorded with their answers but joined nowhere, and every
/// row still projects to its records.
#[test]
fn a_failing_plugin_is_unknown_and_its_claims_are_kept_but_not_joined() {
    let _serial = serial();
    let directory = scratch("verdict-preservation-failed");
    let host = audit_report(
        &directory,
        &[
            ("safe.c", "Safe", None),
            ("unsafe.c", "Unsafe", Some("residual-custody")),
        ],
    );
    let claims = json!([claim("safe.c", "Safe", "said before it failed")]);
    plugin(
        &directory,
        "fails-late",
        &format!("{}\nexit 1", emit(&response(&json!([]), &claims))),
    );
    let output = run(
        &directory,
        &["--audit-report", host.to_str().unwrap()],
        "fails-late",
        &[],
    );
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    let report = report(&directory, "fails-late");
    let label = report["plugin"]["label"].as_str().unwrap().to_owned();
    assert_eq!(report["run"]["code"], "plugin-exit");
    for (artifact, verdict) in [("safe.c", Verdict::Unknown), ("unsafe.c", Verdict::Unsafe)] {
        assert_eq!(row(&report, artifact)["verdict"], verdict.as_str());
        let records = records(&report, artifact);
        let run = of_source(&records, &Source::new(SourceKind::Run, label.clone()));
        assert_eq!(run[0].verdict, Verdict::Unknown);
        assert_eq!(run[0].reasons[0].code, "provider-failure");
        assert!(plugin_records(&records, &label).is_empty());
    }
    assert_eq!(report["claims"][0]["joined"], false);
    assert_eq!(
        report["claims"][0]["claim"]["reason"],
        "said before it failed"
    );
    assert_eq!(report["verdict"], "Unsafe");
}
