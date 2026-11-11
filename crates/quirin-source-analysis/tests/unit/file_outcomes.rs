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
typedef __SIZE_TYPE__ size_t;
struct cio_file { int fd; char *map; size_t alloc_size; int map_truncated_warned; int unrelated; };
int close(int); int munmap(void *,size_t); int *__errno_location(void);
int cio_errno_print(int,const char *,int);
#define REPORT cio_errno_print(*__errno_location(),"test.c",1)
int cio_file_native_close(struct cio_file *cf) {
    if (!cf) return -1;
    if (cf->fd != -1) {
        int result = close(cf->fd);
        if (result == -1) { REPORT; return -1; }
        cf->fd = -1;
    }
    return 0;
}
int cio_file_native_unmap(struct cio_file *cf) {
    if (!cf) return -1;
    if (!cf->map) return 0;
    int result = munmap(cf->map,cf->alloc_size);
    if (result != 0) { REPORT; return -1; }
    cf->alloc_size = 0; cf->map = (void*)0; cf->map_truncated_warned = 0;
    return 0;
}
"#;
fn check(c: &str) -> Result<Vec<CleanupCertificate>, String> {
    check_file_outcome_sources(
        &C11TranslationUnit {
            source: c,
            ..Default::default()
        },
        PINNED_FLUENT_BIT_REVISION,
    )
}
#[test]
fn native_cleanup_outcomes_cover_null_absent_failure_and_success() {
    if !crate::test_support::posix_clang_test() {
        return;
    }
    let certificates = check(SOURCE).unwrap();
    for c in &certificates {
        assert_eq!(c.cases().len(), 4);
        assert!(c.cases()[0].fields_preserved);
        assert_eq!(c.cases()[0].returned, -1);
        assert_eq!(c.cases()[1].returned, 0);
        assert_eq!(c.cases()[1].cleanup_calls, 0);
        assert!(c.cases()[2].fields_preserved);
        assert!(c.cases()[2].field_writes.is_empty());
        assert_eq!(c.cases()[2].returned, -1);
        assert!(c.cases()[3].fields_reset);
        assert_eq!(c.cases()[3].returned, 0);
    }
    assert_eq!(certificates[0].cases()[3].field_writes, ["fd"]);
    assert_eq!(
        certificates[1].cases()[3].field_writes,
        ["alloc_size", "map", "map_truncated_warned"]
    );
    assert!(
        check_file_outcome_sources(
            &C11TranslationUnit {
                source: SOURCE,
                ..Default::default()
            },
            "other"
        )
        .is_err()
    );
}
#[test]
fn native_cleanup_rejects_wrong_outcomes_fields_and_call_identities() {
    if !crate::test_support::posix_clang_test() {
        return;
    }
    for (old, new) in [
        ("result == -1", "result == 0"),
        ("result != 0", "result == 0"),
        ("REPORT; return -1;", "REPORT; return 0;"),
        ("REPORT; return -1;", "REPORT; cf->fd = -1; return -1;"),
        ("cf->fd = -1;", "cf->fd = 0;"),
        ("cf->fd = -1;", ""),
        ("cf->map = (void*)0;", ""),
        ("cf->alloc_size = 0;", "cf->alloc_size = 1;"),
        ("cf->map_truncated_warned = 0;", "cf->unrelated = 0;"),
        ("close(cf->fd)", "close(0)"),
        ("munmap(cf->map,cf->alloc_size)", "munmap(cf->map,0)"),
        ("close(cf->fd)", "close(cf->unrelated)"),
        ("if (!cf) return -1;", "if (!cf) return 0;"),
        ("if (!cf->map) return 0;", "if (!cf->map) return -1;"),
        ("cf->fd != -1", "cf->fd != 0"),
        ("int fd;", "unsigned int fd;"),
        ("cf->fd = -1;", "cf->fd = -1; close(cf->fd);"),
        ("cf->fd = -1;", "cf->fd = -1; cf->fd = -1;"),
    ] {
        assert!(
            check(&SOURCE.replace(old, new)).is_err(),
            "accepted mutation: {old} -> {new}"
        );
    }
}
#[test]
fn cleanup_profile_uses_symbolic_identity_and_retains_all_native_predicates() {
    if !crate::test_support::posix_clang_test() {
        return;
    }
    // Aliases, inverse guards and alternative equivalent error tests preserve
    // source semantics; data-dependent branches on arbitrary fd values do not.
    check(
        &SOURCE
            .replace(
                "int result = close(cf->fd);",
                "int fd = cf->fd; int result = close(fd);",
            )
            .replace("result == -1", "result != 0"),
    )
    .unwrap();
    assert!(
        check(&SOURCE.replace("cf->fd = -1;", "if(cf->fd == 17) return 0; cf->fd = -1;")).is_err()
    );
    for c in check(SOURCE).unwrap() {
        let profile = profile(c.source().function()).unwrap();
        let start = CONTRACT
            .find(&format!("extern fn {}", profile.policy))
            .unwrap();
        let end = start + CONTRACT[start..].find("};").unwrap();
        for (offset, _) in CONTRACT[start..end].match_indices("ensures normal ") {
            let i = start + offset;
            let j = i + CONTRACT[i..].find(';').unwrap();
            for predicate in ["false", "unknown_outcome"] {
                let changed = format!(
                    "{}ensures normal {predicate}{}",
                    &CONTRACT[..i],
                    &CONTRACT[j..]
                );
                assert!(check_cleanup_outcomes(c.source(), &profile, &changed).is_err());
            }
        }
    }
}
