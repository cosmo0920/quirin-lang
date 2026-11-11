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

//! Rule 37: real provider specifications require their assertion-bearing bodies.
use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, CCompilerDialect, CSourceInput, LegacyFunctionSummary,
    UnrestrictedLegacyResult, analyze_unrestricted_c11, check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_manifest;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

struct SourceFixture(PathBuf);
impl SourceFixture {
    fn new(source: &str) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "qcore-rule37-{}-{}-{nonce}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&directory).unwrap();
        let fixture = Self(directory);
        std::fs::write(fixture.0.join("wrapper.c"), source).unwrap();
        fixture
    }
}
impl Drop for SourceFixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

fn run(disabled: bool, has_body: bool, dialect: CCompilerDialect) -> LegacyFunctionSummary {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let contracts =
        load_resource_contract_manifest(&root.join("cycle19-resource-contracts.json")).unwrap();
    let wrappers = if has_body {
        r#"
static inline void *rd_calloc(size_t count, size_t size) {
    void *p = calloc(count, size); assert(p); return p;
}
static inline char *rd_strdup(const char *str) {
    char *p = strdup(str); assert(p); return p;
}
"#
    } else {
        "void *rd_calloc(size_t, size_t); char *rd_strdup(const char *);"
    };
    let source = format!(
        r#"
#include <assert.h>
typedef __SIZE_TYPE__ size_t;
void *calloc(size_t, size_t); char *strdup(const char *);
struct envelope {{ char *name; int state; }};
{wrappers}
struct envelope *build(const char *name) {{
    struct envelope *e = rd_calloc(1, sizeof(*e));
    e->name = rd_strdup(name); e->state = 1; return e;
}}
"#
    );
    let fixture = SourceFixture::new(&source);
    let unit = C11TranslationUnit {
        source: &source,
        dialect,
        input: CSourceInput::DeclaredFile {
            spelled: "wrapper.c".into(),
            directory: fixture.0.clone(),
        },
        definitions: if disabled {
            vec!["NDEBUG".into()]
        } else {
            vec![]
        },
        ..Default::default()
    };
    match analyze_unrestricted_c11(&unit, "build", &contracts).unwrap() {
        UnrestrictedLegacyResult::Safe(c) => {
            check_legacy_safety_certificate(&c).unwrap();
            c.summary.functions["build"].clone()
        }
        UnrestrictedLegacyResult::Unsafe(s)
        | UnrestrictedLegacyResult::Unknown {
            partial: Some(s), ..
        } => s.functions["build"].clone(),
        other => panic!("no analyzed body: {other:?}"),
    }
}
// The real libc failure declaration used by this positive fixture is __assert_fail.
// Other libc targets retain the fail-closed controls; no Windows body proof is claimed.
// Pinned rdkafka_admin.c uses GNU17: strict C11 chooses a different assert expansion.
#[cfg(all(target_os = "linux", target_env = "gnu"))]
#[test]
fn asserted_wrappers_establish_the_real_provider_specifications() {
    let result = run(false, true, CCompilerDialect::Gnu17);
    assert_eq!(result.status, AnalysisStatus::Safe, "{result:#?}");
}
#[test]
fn disabled_assertions_withdraw_the_nonnull_specifications() {
    let result = run(true, true, CCompilerDialect::Gnu17);
    assert_ne!(result.status, AnalysisStatus::Safe, "{result:#?}");
}
#[test]
fn absent_wrapper_bodies_do_not_supply_nonnull_results() {
    let result = run(false, false, CCompilerDialect::Gnu17);
    assert_ne!(result.status, AnalysisStatus::Safe, "{result:#?}");
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
#[test]
fn an_unread_strict_c11_assertion_does_not_supply_nonnull_results() {
    let result = run(false, true, CCompilerDialect::C11);
    assert_ne!(result.status, AnalysisStatus::Safe, "{result:#?}");
}
