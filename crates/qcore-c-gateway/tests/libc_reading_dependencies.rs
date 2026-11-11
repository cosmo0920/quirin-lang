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

//! Evidence-only refutation impact follows actual consultation closure.
use qcore_c_gateway::contract_dependencies::{Recording, hypothesis_closure, row_dependencies};
use qcore_c_gateway::legacy_c::{AnalysisStatus, analyze_legacy_c_translation_unit};
use qcore_contract::{contract_loader::load_resource_contract_sources, hypothesis::replay};
use std::collections::{BTreeMap, BTreeSet};

#[test]
fn an_errno_prerequisite_error_reaches_direct_and_summary_users_only() {
    let sources = BTreeMap::from([(
        "errno.qv".into(),
        include_str!("../../../providers/quirin-provider-libc/contracts/libc/errno.qv").into(),
    )]);
    let contracts = load_resource_contract_sources(&sources).unwrap();
    let graphs = replay(&sources).unwrap();
    let source = "int *__errno_location(void);\nint read_errno(void) { if (*__errno_location() == 1) return 1; return 0; }\nint caller(void) { return read_errno(); }\nint unrelated(void) { return 7; }\n";
    let recording = Recording::start();
    let summary = analyze_legacy_c_translation_unit(source, "caller", &contracts).unwrap();
    let record = recording.finish();
    assert_eq!(summary.functions["read_errno"].status, AnalysisStatus::Safe);
    assert_eq!(summary.functions["caller"].status, AnalysisStatus::Safe);
    let rows = row_dependencies(&record, &summary, &contracts);
    let impacted = rows
        .iter()
        .filter_map(|(name, row)| {
            let closure = hypothesis_closure(row, &graphs);
            closure
                .get("__errno_location")
                .is_some_and(|a| a.contains("scalar_declaration"))
                .then_some(name.as_str())
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(impacted, BTreeSet::from(["caller", "read_errno"]));
    assert!(rows["caller"].summaries.contains("read_errno"));
    assert!(hypothesis_closure(&rows["unrelated"], &graphs).is_empty());
    assert_eq!(summary.functions["unrelated"].status, AnalysisStatus::Safe);
    // This set is an evidence-impact report; no AnalysisStatus is mutated.
}
