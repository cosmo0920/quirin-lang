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
fn import(c: &str) -> SourceResult {
    import_source_functions(
        &C11TranslationUnit {
            source: c,
            ..Default::default()
        },
        "short-circuit-fixture",
        &["f"],
        &SourceImportProfile {
            records: &["item"],
            sizeof_types: &[],
            signature_constraints: &[],
            restricted_calls: &[],
        },
    )
    .unwrap()
    .remove(0)
}
fn summary(c: &str) -> SourceSummary {
    let SourceResult::Summary(s) = import(c) else {
        panic!("short-circuit source rejected: {c}")
    };
    s
}
fn calls(path: &SourcePath) -> Vec<&str> {
    path.operations
        .iter()
        .filter_map(|o| match o {
            SourceOperation::Call { function, .. } => Some(function.as_str()),
            _ => None,
        })
        .collect()
}
#[test]
fn short_circuit_calls_keep_sequencing_and_skip_rhs() {
    if !crate::test_support::clang_available() {
        return;
    }
    for (op, expected) in [
        (
            "&&",
            vec![vec!["left", "right"], vec!["left", "right"], vec!["left"]],
        ),
        (
            "||",
            vec![vec!["left"], vec!["left", "right"], vec!["left", "right"]],
        ),
    ] {
        let c = format!(
            "int left(void);int right(void);int f(void){{if(left(){op}right())return 1;return 0;}}"
        );
        let s = summary(&c);
        assert_eq!(s.paths().iter().map(calls).collect::<Vec<_>>(), expected);
        for p in s.paths() {
            let left = p
                .operations
                .iter()
                .position(|o| matches!(o,SourceOperation::Call{function,..} if function=="left"))
                .unwrap();
            let guard = p
                .operations
                .iter()
                .position(|o| matches!(o, SourceOperation::Guard { .. }))
                .unwrap();
            assert!(left < guard);
            if let Some(right) = p
                .operations
                .iter()
                .position(|o| matches!(o,SourceOperation::Call{function,..} if function=="right"))
            {
                assert!(guard < right);
            }
            check_path_symbols(p).unwrap();
        }
    }
}
#[test]
fn short_circuit_null_guard_and_nested_negation_retain_truth_paths() {
    if !crate::test_support::clang_available() {
        return;
    }
    let s =
        summary("struct item{int live;};int f(struct item *p){if(p && p->live)return 1;return 0;}");
    assert_eq!(s.paths().len(), 3);
    assert!(
        !s.paths()[2]
            .operations
            .iter()
            .any(|o| matches!(o, SourceOperation::Read { .. }))
    );
    let s = summary(
        "int a(void);int b(void);int c(void);int f(void){if(!(a() || (b() && c())))return 1;return 0;}",
    );
    assert_eq!(s.paths().len(), 4);
    let traces = s.paths().iter().map(calls).collect::<Vec<_>>();
    assert!(traces.contains(&vec!["a"]));
    assert!(traces.contains(&vec!["a", "b"]));
    assert!(traces.contains(&vec!["a", "b", "c"]));
    for p in s.paths() {
        check_path_symbols(p).unwrap();
    }
    // Independently replay every Boolean valuation (including non-one true
    // values) to detect missing, duplicated or inverted short-circuit paths.
    for a in [0, -1, 7] {
        for b in [0, -1, 7] {
            for c in [0, -1, 7] {
                let mut selected = Vec::new();
                for path in s.paths() {
                    let mut symbols = BTreeMap::new();
                    for (i, op) in path.operations.iter().enumerate() {
                        match op {
                            SourceOperation::Call { function, .. } => {
                                symbols.insert(
                                    i,
                                    match function.as_str() {
                                        "a" => a,
                                        "b" => b,
                                        "c" => c,
                                        _ => panic!("unexpected call"),
                                    },
                                );
                            }
                            SourceOperation::Guard { condition, taken } => {
                                let SourceExpression::Symbol { operation } = &condition.expression
                                else {
                                    panic!("expected direct producer guard")
                                };
                                if (symbols[operation] != 0) != *taken {
                                    break;
                                }
                            }
                            SourceOperation::ReturnValue { value } => {
                                let SourceExpression::Integer { value } = &value.expression else {
                                    panic!("expected literal result")
                                };
                                selected.push(value.parse::<i32>().unwrap());
                            }
                            _ => panic!("unexpected truth-table operation"),
                        }
                    }
                }
                assert_eq!(selected, [i32::from(!(a != 0 || (b != 0 && c != 0)))]);
            }
        }
    }
}
#[test]
fn short_circuit_values_unsequenced_operands_and_path_explosion_stay_unknown() {
    if !crate::test_support::clang_available() {
        return;
    }
    for c in [
        "int a(void);int b(void);int f(void){return a() && b();}",
        "int a(void);int b(void);int f(void){int x=a()||b();return x;}",
        "int a(void);int b(void);int f(void){if((a()&&b())==1)return 1;return 0;}",
        "int a(void);int b(void);int c(void);int f(void){if((a()+b())&&c())return 1;return 0;}",
        "int a(void);int f(void){if(0 && a()){goto missing;}missing:return 0;}",
    ] {
        assert!(
            matches!(import(c), SourceResult::Unknown { .. }),
            "accepted {c}"
        );
    }
    let condition = std::iter::repeat_n("(a() || a())", 7)
        .collect::<Vec<_>>()
        .join(" && ");
    let c = format!("int a(void);int f(void){{if({condition})return 1;return 0;}}");
    assert!(matches!(import(&c),SourceResult::Unknown{reason,..} if reason.contains("path limit")));
}
