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
const SOURCE: &str = r#"
struct cio_file{int fd;char *map;char *path;int unrelated;};
int unlink(const char *);int *__errno_location(void);int cio_errno_print(int,const char *,int);
int cio_file_native_delete(struct cio_file *cf){
 if(cf->fd != -1 || cf->map != (void*)0)return -1;
 int ret=unlink(cf->path);
 if(ret == -1){cio_errno_print(*__errno_location(),"test.c",1);return -1;}
 return 0;
}
"#;
fn check(c: &str) -> Result<GuardedDeleteCertificate, String> {
    check_native_delete_source(
        &C11TranslationUnit {
            source: c,
            ..Default::default()
        },
        PINNED_FLUENT_BIT_REVISION,
    )
}
#[test]
fn delete_checks_eligibility_outcomes_and_preserves_allocations() {
    if !crate::test_support::clang_available() {
        return;
    }
    let certificate = check(SOURCE).unwrap();
    assert_eq!(certificate.cases().len(), 5);
    assert_eq!(certificate.source().paths().len(), 4);
    for case in certificate.cases() {
        assert!(case.fields_preserved);
        assert_eq!(
            case.syscall_calls,
            usize::from(!case.descriptor_field_active && !case.mapping_field_active)
        );
        assert_eq!(
            case.returned,
            if case.syscall_result == Some(0) {
                0
            } else {
                -1
            }
        );
    }
    assert_eq!(recheck_native_delete(&certificate).unwrap(), certificate);
    assert!(
        check_native_delete_source(
            &C11TranslationUnit {
                source: SOURCE,
                ..Default::default()
            },
            "wrong"
        )
        .is_err()
    );
}
#[test]
fn delete_rejects_wrong_guards_path_status_and_record_mutations() {
    if !crate::test_support::clang_available() {
        return;
    }
    for (old, new) in [
        (" || ", " && "),
        ("cf->fd != -1", "cf->fd == -1"),
        ("cf->map != (void*)0", "cf->map == (void*)0"),
        ("unlink(cf->path)", "unlink(cf->map)"),
        ("unlink(cf->path)", "unlink((void*)0)"),
        ("ret == -1", "ret == 0"),
        ("return 0;", "cf->fd = -1;return 0;"),
        ("return 0;", "cf->unrelated = 0;return 0;"),
        ("return 0;", "return -1;"),
        ("char *path;", "int path;"),
        ("cf->fd != -1", "cf->fd != 0"),
    ] {
        assert!(
            check(&SOURCE.replace(old, new)).is_err(),
            "accepted {old} -> {new}"
        );
    }
}
#[test]
fn delete_contract_premises_and_outcomes_are_executed() {
    if !crate::test_support::clang_available() {
        return;
    }
    let certificate = check(SOURCE).unwrap();
    for marker in ["requires ", "ensures normal "] {
        for (i, _) in CONTRACT.match_indices(marker) {
            let j = i + CONTRACT[i..].find(';').unwrap();
            for predicate in ["false", "unknown_delete_fact"] {
                let changed = format!("{}{marker}{predicate}{}", &CONTRACT[..i], &CONTRACT[j..]);
                assert!(
                    check_guarded_delete_outcomes(
                        certificate.source(),
                        &profile(),
                        "map",
                        &changed
                    )
                    .is_err()
                );
            }
        }
    }
}
