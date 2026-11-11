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

//! Card W45-CONTRACT-DEFERRED-REVIEW: `quirin-audit contracts` publishes the
//! `pending_human_review` section and its console tally, and records the
//! `awaiting-human-review` unit reason code, end to end on the command line.
//!
//! A self-contained fixture tree (a held-out `alloc`/`dispose` API on one
//! temporary load path); no provider contract and no corpus pin moves.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

use serde_json::{Value, json};

static NEXT: AtomicUsize = AtomicUsize::new(0);

// Card W45-CONTRACT-REVISION-MODE: a synthetic API with no source to pin
// against, so its contracts bind by name (`revision flexible`).
const PENDING: &str = "module contract;\n\
extern fn alloc() -> ptr<mut, u8>\n\
contract {\n\
    requires resource_contract_v1;\n\
    requires resource_revision(flexible);\n\
    ensures normal resource_return_owned(nonnull);\n\
};\n\
extern fn dispose(p: ptr<mut, u8>) -> unit\n\
contract {\n\
    requires resource_contract_v1;\n\
    requires resource_revision(flexible);\n\
    ensures normal resource_review_pending(resource_released(p, nonnull), \"frees p?\", \"owner\");\n\
    ensures normal resource_return_none();\n\
};\n";

const PROTOTYPES: &str = "unsigned char *alloc(void);\nvoid dispose(unsigned char *p);\n";

struct Fixture(PathBuf);

impl Fixture {
    fn new(contracts: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "qcore-pending-review-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("tree")).unwrap();
        fs::write(
            root.join("tree/held.c"),
            format!(
                "{PROTOTYPES}void held(void) {{ unsigned char *p = alloc(); dispose(p); }}\n\
                 void quiet(void) {{ return; }}\n"
            ),
        )
        .unwrap();
        let module = root.join("providers/quirin-provider-alpha/contracts/alpha/heap.qcore");
        fs::create_dir_all(module.parent().unwrap()).unwrap();
        fs::write(module, contracts).unwrap();
        fs::write(
            root.join("providers/contract-load-path.json"),
            serde_json::to_string_pretty(&json!({"schema_version": 1, "crates": [
                {"name": "quirin-provider-alpha", "namespace": "alpha", "version": "0.1.0",
                 "contracts": "quirin-provider-alpha/contracts", "dependencies": [],
                 "modules": ["alpha/heap.qcore"], "published": {},
                 "resource_view": ["alpha/heap.qcore"]},
            ]}))
            .unwrap(),
        )
        .unwrap();
        Self(root)
    }

    fn run(&self) -> Output {
        Command::new(env!("CARGO_BIN_EXE_quirin-audit"))
            .current_dir(&self.0)
            .args([
                "contracts",
                "--source-root",
                "tree",
                "held.c",
                "--load-path",
                "providers/contract-load-path.json",
                "--report",
                "out/report.json",
            ])
            .output()
            .unwrap()
    }

    fn read(&self, path: &str) -> Value {
        serde_json::from_slice(&fs::read(self.0.join(path)).unwrap()).unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn stdout(output: &Output) -> String {
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn clang_available() -> bool {
    if qcore_c_gateway::clang_command().is_ok() {
        return true;
    }
    eprintln!("skipping Clang-dependent pending-review report test");
    false
}

#[test]
fn the_contracts_route_lists_the_pending_review_and_what_it_holds() {
    if !clang_available() {
        return;
    }
    let fixture = Fixture::new(PENDING);
    let console = stdout(&fixture.run());
    assert!(
        console.contains(
            "pending human review: 1 declaration(s), 1 marker(s); 1 function(s) and 1 call \
             site(s) held Pending (awaiting-human-review)"
        ),
        "{console}"
    );
    let report = fixture.read("out/report.json");
    let unit = &report["units"][0];
    // Card W45-USER-CONTRACT-PLUGINS: the held unit is Pending (it was Unknown).
    assert_eq!(unit["status"], "Pending");
    assert_eq!(unit["reason_code"], "awaiting-human-review");
    let section = &fixture.read("out/report.contracts.json")["pending_human_review"];
    assert_eq!(section["reason_code"], "awaiting-human-review");
    assert_eq!(section["declarations"], 1);
    assert_eq!(section["held_functions"], 1);
    assert_eq!(section["held_call_sites"], 1);
    let entry = &section["entries"][0];
    assert_eq!(entry["function"], "dispose");
    assert_eq!(entry["scope"], "clause");
    assert_eq!(
        entry["clause"],
        "ensures normal resource_released(p, nonnull)"
    );
    assert_eq!(entry["reason"], "frees p?");
    assert_eq!(entry["owner"], "owner");
    // The retained source-assumption key: the selected module's path.
    assert!(
        entry["module"]
            .as_str()
            .unwrap()
            .ends_with("providers/quirin-provider-alpha/contracts/alpha/heap.qcore"),
        "{entry}"
    );
    assert_eq!(
        entry["module_sha256"],
        qcore_schemas::sha256_hex(PENDING.as_bytes())
    );
    assert_eq!(entry["category"], "acceptable-as-written-after-review");
    // Card W47-REVIEW-HOLD-CONCRETE-REASON: the Pending row itself names the
    // held declaration, its clause, the reason verbatim and the category; the
    // row without a held call carries no such member.
    let functions = unit["functions"].as_array().unwrap();
    let held = functions
        .iter()
        .find(|function| function["function"] == "held")
        .unwrap();
    assert_eq!(held["status"], "Pending");
    assert_eq!(
        held["review_holds"],
        json!([{
            "declaration": "dispose",
            "module": entry["module"],
            "scope": "clause",
            "clause": "ensures normal resource_released(p, nonnull)",
            "reason": "frees p?",
            "owner": "owner",
            "category": "acceptable-as-written-after-review",
            "reason_code": "awaiting-human-review",
        }])
    );
    let quiet = functions
        .iter()
        .find(|function| function["function"] == "quiet")
        .unwrap();
    assert!(quiet.get("review_holds").is_none(), "{quiet}");
}

#[test]
fn a_marker_stating_the_text_must_be_rewritten_stays_pending_and_says_so() {
    if !clang_available() {
        return;
    }
    let rewrite = PENDING.replace(
        "\"frees p?\", \"owner\")",
        "\"retained, not released\", \"owner\", rewrite_before_acceptance)",
    );
    let fixture = Fixture::new(&rewrite);
    let console = stdout(&fixture.run());
    assert!(
        console.contains("held Pending (awaiting-human-review); 1 marker(s) must be rewritten"),
        "{console}"
    );
    let report = fixture.read("out/report.json");
    // The verdict is unchanged by the category (user decision 2026-09-29).
    assert_eq!(report["units"][0]["status"], "Pending");
    let held = &report["units"][0]["functions"][0];
    assert_eq!(held["function"], "held");
    assert_eq!(held["status"], "Pending");
    assert_eq!(held["review_holds"][0]["reason"], "retained, not released");
    assert_eq!(
        held["review_holds"][0]["category"],
        "must-be-rewritten-before-acceptance"
    );
    let section = &fixture.read("out/report.contracts.json")["pending_human_review"];
    assert_eq!(section["rewrite_markers"], 1);
    assert_eq!(
        section["entries"][0]["category"],
        "must-be-rewritten-before-acceptance"
    );
}

#[test]
fn a_run_with_nothing_pending_publishes_explicit_zeros_and_decides() {
    if !clang_available() {
        return;
    }
    let bare = PENDING.replace(
        "resource_review_pending(resource_released(p, nonnull), \"frees p?\", \"owner\")",
        "resource_released(p, nonnull)",
    );
    let fixture = Fixture::new(&bare);
    let console = stdout(&fixture.run());
    assert!(
        console.contains("pending human review: 0 declaration(s), 0 marker(s)"),
        "{console}"
    );
    assert_eq!(
        fixture.read("out/report.json")["units"][0]["status"],
        "Safe"
    );
    let section = &fixture.read("out/report.contracts.json")["pending_human_review"];
    assert_eq!(section["declarations"], 0);
    assert_eq!(section["entries"], json!([]));
}
