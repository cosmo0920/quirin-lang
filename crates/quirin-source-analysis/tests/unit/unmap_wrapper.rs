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
use crate::typed_source::{SourceImportProfile, SourceResult, import_source_functions};
const SOURCE: &str = include_str!("../fixtures/chunkio/unmap_source.c");
fn import(c: &str, function: &str) -> SourceSummary {
    let sources = import_source_functions(
        &C11TranslationUnit {
            source: c,
            ..Default::default()
        },
        PINNED_FLUENT_BIT_REVISION,
        &[function],
        &SourceImportProfile {
            records: &["cio_ctx", "cio_stream", "cio_chunk", "cio_file"],
            sizeof_types: &[],
            signature_constraints: &[],
            restricted_calls: &[],
        },
    )
    .unwrap();
    let SourceResult::Summary(s) = &sources[0] else {
        panic!("{sources:?}")
    };
    s.clone()
}
fn dependencies() -> (CleanupCertificate, CounterCertificate) {
    let native = check_cleanup_outcomes(
        &import(SOURCE, "cio_file_native_unmap"),
        &file_outcomes::profile("cio_file_native_unmap").unwrap(),
        file_outcomes::CONTRACT,
    )
    .unwrap();
    let counter = check_counter_contract(
        &import(SOURCE, "cio_chunk_counter_total_up_sub"),
        COUNTER_CONTRACT,
        "struct cio_ctx *",
        "ChunkContext",
    )
    .unwrap();
    (native, counter)
}
#[test]
fn unmap_composes_native_outcomes_with_exact_up_counter_bounds() {
    if !crate::test_support::posix_clang_test() {
        return;
    }
    let (native, counter) = dependencies();
    let cert = check_unmap_wrapper_source(
        &C11TranslationUnit {
            source: SOURCE,
            ..Default::default()
        },
        PINNED_FLUENT_BIT_REVISION,
        &native,
        &counter,
    )
    .unwrap();
    assert_eq!(cert.cases().len(), 8);
    assert_eq!(
        cert.cases()
            .iter()
            .filter(|c| c.counter_decrements == 1)
            .count(),
        3
    );
    for (i, c) in cert.cases().iter().enumerate() {
        if c.native_result == Some(0) {
            assert_eq!(cert.counter_after(i, 3, 10).unwrap(), 2);
            assert!(cert.counter_after(i, 0, 10).is_err());
            assert_eq!(c.wrapper_writes, ["data_size", "alloc_size"]);
        } else {
            assert_eq!(cert.counter_after(i, 3, 10).unwrap(), 3);
            assert_eq!(cert.counter_after(i, 0, 10).unwrap(), 0);
            assert!(c.wrapper_writes.is_empty());
        }
        assert!(cert.counter_after(i, 11, 10).is_err());
    }
    assert!(cert.counter_after(8, 3, 10).is_err());
    // Failed sync still attempts native unmap; only native success decrements.
    assert_eq!(
        cert.cases()
            .iter()
            .filter(|c| c.sync_result == Some(-1) && c.native_result.is_some())
            .count(),
        2
    );
}
#[test]
fn unmap_rejects_wrong_context_effect_order_and_outcome_mutations() {
    if !crate::test_support::posix_clang_test() {
        return;
    }
    let (native, counter) = dependencies();
    for (old, new) in [
        ("if(ret!=0)return -1;", "if(ret==0)return -1;"),
        (
            "if(ret!=0)return -1;",
            "if(ret!=0){cio_chunk_counter_total_up_sub(ctx);return -1;}",
        ),
        (
            "if(ret!=0)return -1;",
            "if(ret!=0){cf->data_size=0;return -1;}",
        ),
        ("cio_file_sync(ch)", "cio_file_sync((void*)0)"),
        (
            "ret=cio_file_native_unmap(cf)",
            "ret=cio_file_native_unmap((void*)0)",
        ),
        (
            "cio_chunk_counter_total_up_sub(ctx);return 0;",
            "cio_chunk_counter_total_sub(ctx);return 0;",
        ),
        (
            "cio_chunk_counter_total_up_sub(ctx);return 0;",
            "cio_chunk_counter_total_up_sub((void*)0);return 0;",
        ),
        ("cf->data_size=0;cf->alloc_size=0;", "cf->data_size=0;"),
        (
            "cf->data_size=0;cf->alloc_size=0;",
            "cf->data_size=0;cf->data_size=0;",
        ),
        (
            "cf->data_size=0;cf->alloc_size=0;",
            "cio_chunk_counter_total_up_sub(ctx);cf->data_size=0;cf->alloc_size=0;",
        ),
        ("if(!cf->map)return -1;", "if(!cf->map)return 0;"),
        ("if(cf->synced==0)", "if(cf->synced==1)"),
        ("ch->st->name,ch->name", "ch->name,ch->st->name"),
        (
            "cio_chunk_counter_total_up_sub(ctx);return 0;",
            "cio_chunk_counter_total_up_sub(ctx);free(cf);return 0;",
        ),
    ] {
        assert!(
            check(
                &import(&SOURCE.replace(old, new), "munmap_file"),
                &native,
                &counter,
                CONTRACT
            )
            .is_err(),
            "accepted {old} -> {new}"
        );
    }
    let wrong_counter = check_counter_contract(
        &import(SOURCE, "cio_chunk_counter_total_sub"),
        COUNTER_CONTRACT,
        "struct cio_ctx *",
        "ChunkContext",
    )
    .unwrap();
    assert!(
        check(
            &import(SOURCE, "munmap_file"),
            &native,
            &wrong_counter,
            CONTRACT
        )
        .is_err()
    );
}
#[test]
fn unmap_native_contract_clauses_are_executed() {
    if !crate::test_support::posix_clang_test() {
        return;
    }
    let (native, counter) = dependencies();
    let source = import(SOURCE, "munmap_file");
    for marker in ["requires ", "ensures normal "] {
        for (i, _) in CONTRACT.match_indices(marker) {
            let j = i + CONTRACT[i..].find(';').unwrap();
            for pred in ["false", "unbound_unmap_fact"] {
                let mutated = format!("{}{marker}{pred}{}", &CONTRACT[..i], &CONTRACT[j..]);
                assert!(check(&source, &native, &counter, &mutated).is_err());
            }
        }
    }
}
