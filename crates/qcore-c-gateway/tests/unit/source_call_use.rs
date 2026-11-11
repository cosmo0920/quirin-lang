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

use super::*;
use crate::legacy_c::C11TranslationUnit;
use crate::typed_source::{SourceImportProfile, SourceResult, import_source_functions};
#[test]
fn call_use_tracks_alias_return_guard_argument_and_discard() {
    if !crate::test_support::clang_available() {
        return;
    }
    let source = "int query(void); void consume(int); int f(int x){int a=query();int b=a;if(b==0){consume(b);return b;}query();return x;}";
    let summaries = import_source_functions(
        &C11TranslationUnit {
            source,
            ..Default::default()
        },
        "test",
        &["f"],
        &SourceImportProfile {
            records: &[],
            sizeof_types: &[],
            signature_constraints: &[],
            restricted_calls: &[],
        },
    )
    .unwrap();
    let SourceResult::Summary(summary) = &summaries[0] else {
        panic!("{summaries:?}")
    };
    let paths = analyze_source_call_uses(summary).unwrap();
    assert_eq!(paths.len(), 2);
    for path in &paths {
        let first = &path.calls[0];
        assert_eq!(first.function, "query");
        assert!(!first.uses.is_empty());
        assert_eq!(first.guard_uses.len(), 1);
    }
    let discarded = paths
        .iter()
        .flat_map(|p| &p.calls)
        .find(|c| c.function == "query" && c.uses.is_empty())
        .unwrap();
    assert!(discarded.guard_uses.is_empty());
}

#[test]
fn structured_loop_dependencies_remain_unknown_to_flat_use_analysis() {
    if !crate::test_support::clang_available() {
        return;
    }
    let source = "int f(int n){int i;for(i=0;i<n;i++){if(i==2){return 0;}}return 0;}";
    let summaries = import_source_functions(
        &C11TranslationUnit {
            source,
            ..Default::default()
        },
        "test",
        &["f"],
        &SourceImportProfile {
            records: &[],
            sizeof_types: &[],
            signature_constraints: &[],
            restricted_calls: &[],
        },
    )
    .unwrap();
    let SourceResult::Summary(summary) = &summaries[0] else {
        panic!("{summaries:?}")
    };
    assert!(summary.has_scan_loop());
    assert!(analyze_source_call_uses(summary).is_err());
}
