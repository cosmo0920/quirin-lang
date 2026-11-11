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
use crate::typed_source::{
    SourceImportProfile, SourceResult, SourceSummary, import_source_functions,
};
use crate::verifier_plugins::chunkio::PINNED_FLUENT_BIT_REVISION;
use crate::verifier_plugins::chunkio::teardown_source::{
    MEMORY_BACKEND_CONTRACT, check_memory_backend_source,
};

const FIXTURE: &str = "
struct cio_memfs { char *name; char *buf_data; char *meta_data; };
struct cio_chunk { void *backend; };
void free(void *);
void cio_memfs_close(struct cio_chunk *ch) {
    struct cio_memfs *mf = ch->backend;
    if (!mf) { return; }
    free(mf->name);
    free(mf->buf_data);
    free(mf->meta_data);
    free(mf);
}";
fn import(source: &str) -> SourceSummary {
    let results = import_source_functions(
        &C11TranslationUnit {
            source,
            ..Default::default()
        },
        PINNED_FLUENT_BIT_REVISION,
        &["cio_memfs_close"],
        &SourceImportProfile {
            records: &["cio_chunk", "cio_memfs"],
            sizeof_types: &[],
            signature_constraints: &[],
            restricted_calls: &[],
        },
    )
    .unwrap();
    let SourceResult::Summary(summary) = &results[0] else {
        panic!("{results:?}")
    };
    summary.clone()
}
fn check(source: &str, contract: &str) -> Result<FieldDestructorCertificate, String> {
    check_field_destructor(&import(source), contract, "struct cio_chunk *", "Chunk")
}

#[test]
fn nullable_field_custody_exhausts_realizable_states() {
    if !crate::test_support::clang_available() {
        return;
    }
    let certificate = check_memory_backend_source(
        &C11TranslationUnit {
            source: FIXTURE,
            ..Default::default()
        },
        PINNED_FLUENT_BIT_REVISION,
    )
    .unwrap();
    assert_eq!(certificate.cases().len(), 9);
    assert_eq!(
        certificate.cases()[0].released_in_order,
        Vec::<String>::new()
    );
    let all = certificate.cases().last().unwrap();
    assert_eq!(
        all.released_in_order,
        [
            "backend__name",
            "backend__buf_data",
            "backend__meta_data",
            "backend"
        ]
    );
    assert_eq!(
        serde_json::to_value(&certificate).unwrap()["postconditions_per_case"],
        13
    );
    assert_eq!(certificate.source().paths().len(), 2);
    assert!(
        check_memory_backend_source(
            &C11TranslationUnit {
                source: FIXTURE,
                ..Default::default()
            },
            "wrong"
        )
        .is_err()
    );
    let renamed = FIXTURE
        .replace("*mf =", "*alias =")
        .replace("!mf", "!alias")
        .replace("mf->", "alias->")
        .replace("free(mf)", "free(alias)");
    assert_eq!(
        check(&renamed, MEMORY_BACKEND_CONTRACT).unwrap().cases(),
        certificate.cases()
    );
    let spelled = FIXTURE
        .replace("if (!mf)", "if ((!mf) == 0)")
        .replace("{ return; }", "{} else { return; }");
    assert_eq!(
        check(&spelled, MEMORY_BACKEND_CONTRACT)
            .unwrap()
            .cases()
            .len(),
        9
    );
}

#[test]
fn teardown_rejects_missing_duplicate_wrong_order_and_unmodeled_effects() {
    if !crate::test_support::clang_available() {
        return;
    }
    for changed in [
        FIXTURE.replace("free(mf->name);", ""),
        FIXTURE.replace("free(mf->name);", "free(mf->name); free(mf->name);"),
        FIXTURE.replace("free(mf->name);", "free(mf); free(mf->name);"),
        FIXTURE.replace("free(mf->name);", "free(ch);"),
        FIXTURE.replace("free(mf->name);", "mf->name = 0;"),
        FIXTURE.replace(
            "free(mf->name);",
            "if (0) { mf->name = 0; } free(mf->name);",
        ),
        FIXTURE.replace("free(mf->name);", "free((char *) mf->name + 1);"),
        FIXTURE.replace("if (!mf) { return; }", ""),
        FIXTURE
            .replace(
                "void free(void *);",
                "void free(void *); void unknown(void *);",
            )
            .replace("free(mf->name);", "if (0) unknown(mf); free(mf->name);"),
    ] {
        assert!(
            check(&changed, MEMORY_BACKEND_CONTRACT).is_err(),
            "accepted {changed}"
        );
    }
}

#[test]
fn contract_signature_domains_and_every_postcondition_fail_closed() {
    if !crate::test_support::clang_available() {
        return;
    }
    let summary = import(FIXTURE);
    for (old, new) in [
        ("mut, Chunk", "const, Chunk"),
        ("use one", ""),
        ("-> unit", "-> i32"),
        ("requires root_is_live;", ""),
        ("requires owned_fields_are_disjoint_from_root;", ""),
        ("requires free_resolves_to_libc;", "requires unknown;"),
        ("requires owned_or_null__backend;", ""),
        (
            "requires exclusive_access;",
            "requires exclusive_access; requires exclusive_access;",
        ),
        (
            "ensures normal root_preserved;",
            "ensures error root_preserved;",
        ),
        ("requires owned_or_null__backend__name;", ""),
        (
            "ensures normal live__backend == 0;",
            "ensures normal live__backend == 1;",
        ),
    ] {
        assert!(
            check_field_destructor(
                &summary,
                &MEMORY_BACKEND_CONTRACT.replace(old, new),
                "struct cio_chunk *",
                "Chunk"
            )
            .is_err(),
            "accepted {new}"
        );
    }
    for (start, _) in MEMORY_BACKEND_CONTRACT.match_indices("ensures normal ") {
        let end = start + MEMORY_BACKEND_CONTRACT[start..].find(';').unwrap();
        for predicate in ["false", "unsupported_predicate"] {
            let changed = format!(
                "{}ensures normal {predicate}{}",
                &MEMORY_BACKEND_CONTRACT[..start],
                &MEMORY_BACKEND_CONTRACT[end..]
            );
            assert!(
                check_field_destructor(&summary, &changed, "struct cio_chunk *", "Chunk").is_err()
            );
        }
    }
    assert!(
        check_field_destructor(&summary, MEMORY_BACKEND_CONTRACT, "struct other *", "Chunk")
            .is_err()
    );
    assert!(
        check_field_destructor(
            &summary,
            MEMORY_BACKEND_CONTRACT,
            "struct cio_chunk *",
            "Other"
        )
        .is_err()
    );
}
