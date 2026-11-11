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
use crate::verifier_plugins::chunkio::{
    PINNED_FLUENT_BIT_REVISION, teardown_source::COUNTER_CONTRACT,
};
// Model an unsigned-long counter without redefining the host's size_t.
const FIXTURE: &str = "typedef unsigned long qcore_test_size_t; struct cio_ctx { qcore_test_size_t total_chunks; qcore_test_size_t other; }; qcore_test_size_t cio_chunk_counter_total_sub(struct cio_ctx *ctx) { ctx->total_chunks--; return ctx->total_chunks; }";
fn check(c: &str, contract: &str) -> Result<CounterCertificate, String> {
    let results = import_source_functions(
        &C11TranslationUnit {
            source: c,
            ..Default::default()
        },
        PINNED_FLUENT_BIT_REVISION,
        &["cio_chunk_counter_total_sub"],
        &SourceImportProfile {
            records: &["cio_ctx"],
            sizeof_types: &[],
            signature_constraints: &[],
            restricted_calls: &[],
        },
    )?;
    match &results[0] {
        SourceResult::Summary(summary) => {
            check_counter_contract(summary, contract, "struct cio_ctx *", "ChunkContext")
        }
        SourceResult::Unknown { reason, .. } => Err(reason.clone()),
    }
}
#[test]
fn source_counter_binds_direction_field_return_and_word_bounds() {
    if !crate::test_support::clang_available() {
        return;
    }
    let cert = check(FIXTURE, COUNTER_CONTRACT).unwrap();
    assert_eq!(cert.field(), "total_chunks");
    assert_eq!(cert.delta(), -1);
    for maximum in [1, 255, u32::MAX as u128, u64::MAX as u128, u128::MAX] {
        assert!(cert.instantiate(0, maximum).is_err());
        assert_eq!(cert.instantiate(1, maximum).unwrap(), 0);
        assert_eq!(cert.instantiate(maximum, maximum).unwrap(), maximum - 1);
    }
    assert!(cert.instantiate(256, 255).is_err());
    assert!(cert.instantiate(0, 0).is_err());
    let increment = COUNTER_CONTRACT
        .replace("ensures normal delta == -1;", "ensures normal delta == 1;")
        .replace(
            "requires counter_positive;",
            "requires counter_below_maximum;",
        );
    let cert = check(&FIXTURE.replace("--", "++"), &increment).unwrap();
    assert_eq!(cert.instantiate(0, 255).unwrap(), 1);
    assert_eq!(cert.instantiate(254, 255).unwrap(), 255);
    assert!(cert.instantiate(255, 255).is_err());
    assert!(cert.instantiate(u128::MAX, u128::MAX).is_err());
    assert_eq!(
        check(
            &FIXTURE.replace("total_chunks--;", "total_chunks++;"),
            COUNTER_CONTRACT
        )
        .unwrap_err(),
        "counter contract lacks exact field/domain/nonwrapping premises"
    );
}
#[test]
fn counter_source_changes_and_unreachable_effects_fail_closed() {
    if !crate::test_support::clang_available() {
        return;
    }
    for (old, new) in [
        ("ctx->total_chunks--;", "ctx->other--;"),
        ("return ctx->total_chunks;", "return ctx->other;"),
        ("return ctx->total_chunks;", "return 0;"),
        (
            "ctx->total_chunks--;",
            "ctx->total_chunks--; ctx->total_chunks--;",
        ),
        (
            "ctx->total_chunks--;",
            "if (0) { ctx->other--; } ctx->total_chunks--;",
        ),
        ("ctx->total_chunks--;", "while (0) {} ctx->total_chunks--;"),
        (
            "qcore_test_size_t total_chunks;",
            "volatile qcore_test_size_t total_chunks;",
        ),
        (
            "qcore_test_size_t total_chunks;",
            "qcore_test_size_t total_chunks : 5;",
        ),
        ("qcore_test_size_t total_chunks;", "long total_chunks;"),
    ] {
        assert!(
            check(&FIXTURE.replace(old, new), COUNTER_CONTRACT).is_err(),
            "accepted {new}"
        );
    }
}
#[test]
fn counter_contract_domains_and_all_predicates_are_checked() {
    if !crate::test_support::clang_available() {
        return;
    }
    for (old, new) in [
        ("requires counter_positive;", ""),
        ("requires counter_in_range;", ""),
        ("requires c_unsigned_matches_usize;", "requires arbitrary;"),
        (
            "requires counter_field__total_chunks;",
            "requires counter_field__other;",
        ),
        ("mut, ChunkContext", "const, ChunkContext"),
        ("-> usize", "-> i32"),
        ("use one", ""),
        (
            "ensures normal result_is_updated_counter;",
            "ensures error result_is_updated_counter;",
        ),
    ] {
        assert!(
            check(FIXTURE, &COUNTER_CONTRACT.replace(old, new)).is_err(),
            "accepted {new}"
        );
    }
    let start = COUNTER_CONTRACT
        .find("extern fn cio_chunk_counter_total_sub(")
        .unwrap();
    let end = start + COUNTER_CONTRACT[start..].find("};").unwrap();
    for (offset, _) in COUNTER_CONTRACT[start..end].match_indices("ensures normal ") {
        let i = start + offset;
        let j = i + COUNTER_CONTRACT[i..].find(';').unwrap();
        for predicate in ["false", "unsupported_predicate"] {
            let changed = format!(
                "{}ensures normal {predicate}{}",
                &COUNTER_CONTRACT[..i],
                &COUNTER_CONTRACT[j..]
            );
            assert!(check(FIXTURE, &changed).is_err());
        }
    }
}
