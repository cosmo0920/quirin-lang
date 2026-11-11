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

//! Parity between the retained in-process ChunkIO `cio_file_close` adapters and
//! the moved two-level process route (card `P04-COMPOSE-A`).
//!
//! Both routes now call one relocated decision procedure, so this file measures
//! what remains genuinely separate: the in-process adapters' hardcoded record
//! profiles against the contract declarations the moved route derives for
//! itself, and the certificates the two routes publish — for **both** entry
//! points. Nothing here asserts anything about the OS: releasing the backend's
//! C allocation never discharges an OS cleanup obligation.
#![cfg(not(target_os = "windows"))]

#[path = "../../crates/qcore-verification/tests/fixtures/file_close_composition.rs"]
mod fixture;
use fixture::*;

use qcore_compiler::cleanup_outcome::check_cleanup_outcomes;
use qcore_compiler::counter_contract::check_counter_contract;
use qcore_compiler::legacy_c::C11TranslationUnit;
use qcore_compiler::typed_source::{
    SourceImportProfile, SourceResult, SourceSummary, import_source_functions,
};
use qcore_compiler::verifier_plugins::chunkio::{
    file_close, file_delete, file_outcomes, teardown_source::COUNTER_CONTRACT, unmap_wrapper,
};
use qcore_verification::cleanup_replay_source::declared_profile;

fn tu(source: &str) -> C11TranslationUnit<'_> {
    C11TranslationUnit {
        source,
        ..Default::default()
    }
}

fn summary(source: &str, function: &str) -> Result<SourceSummary, String> {
    let imported = import_source_functions(
        &tu(source),
        &revision(),
        &[function],
        &SourceImportProfile {
            records: &RECORDS,
            sizeof_types: &[],
            signature_constraints: &[],
            restricted_calls: &[],
        },
    )?;
    match imported.into_iter().next() {
        Some(SourceResult::Summary(summary)) => Ok(summary),
        other => Err(format!("missing summary for {function}: {other:?}")),
    }
}

/// The in-process route, called exactly as the Cycle 19 compatibility adapter
/// calls it. Returns the serialized certificates of both entry points.
fn in_process(source: &str) -> Result<(String, String), String> {
    let revision = revision();
    let unmap_native = check_cleanup_outcomes(
        &summary(source, UNMAP_NATIVE)?,
        &file_outcomes::profile(UNMAP_NATIVE)?,
        file_outcomes::CONTRACT,
    )?;
    let counter = check_counter_contract(
        &summary(source, COUNTER)?,
        COUNTER_CONTRACT,
        "struct cio_ctx *",
        "ChunkContext",
    )?;
    let unmap =
        unmap_wrapper::check_unmap_wrapper_source(&tu(source), &revision, &unmap_native, &counter)?;
    let close = check_cleanup_outcomes(
        &summary(source, CLOSE)?,
        &file_outcomes::profile(CLOSE)?,
        file_outcomes::CONTRACT,
    )?;
    let plain = file_close::check_file_close_source(&tu(source), &revision, &unmap, &close)?;
    let delete = file_delete::check_native_delete_source(&tu(source), &revision)?;
    let deleting = file_close::check_file_close_deletion_source(
        &tu(source),
        &revision,
        &unmap,
        &close,
        &delete,
    )?;
    Ok((
        serde_json::to_string(&plain).map_err(|e| e.to_string())?,
        serde_json::to_string(&deleting).map_err(|e| e.to_string())?,
    ))
}

/// The moved route: the same relocated procedure, reached through the sealed
/// composition import, the re-established prerequisites at **both** levels and
/// the core's own contract-derived record profiles.
fn moved(source: &str) -> Result<(String, String), String> {
    let rc = pinned();
    let plain = build_route(source, UNIX_TU, &rc, false)?.evaluate()?;
    let deleting = build_route(source, UNIX_TU, &rc, true)?.evaluate()?;
    let plain = match plain {
        qcore_verification::composition_source::FileCloseCertificate::NoDelete(evidence) => {
            serde_json::to_string(&*evidence).map_err(|e| e.to_string())?
        }
        _ => return Err("the non-deleting route published a deleting reading".into()),
    };
    let deleting = match deleting {
        qcore_verification::composition_source::FileCloseCertificate::Deleting(evidence) => {
            serde_json::to_string(&*evidence).map_err(|e| e.to_string())?
        }
        _ => return Err("the deleting route published a non-deleting reading".into()),
    };
    Ok((plain, deleting))
}

/// The retained adapters' hardcoded record profiles **are** the vocabularies the
/// pinned contracts declare, which is what the moved route derives for itself.
#[test]
fn the_adapters_profiles_are_the_contracts_own_declarations() {
    let close = declared_profile(OUTCOMES_CONTRACT, CLOSE_POLICY).unwrap();
    assert_eq!(close.1, None, "the close policy declares no eligibility");
    assert_eq!(close.0, file_outcomes::profile(CLOSE).unwrap());

    let delete = declared_profile(DELETE_OUTCOMES_CONTRACT, DELETE_POLICY).unwrap();
    assert_eq!(
        delete.1.as_deref(),
        Some("map"),
        "the delete policy must declare its eligibility field"
    );
    assert_eq!(delete.0, file_delete::profile());
}

/// The two routes publish the same certificates, byte for byte, on **both**
/// entry points.
#[test]
fn both_routes_publish_byte_identical_certificates() {
    if !clang_available() {
        return;
    }
    let (a_plain, a_deleting) = in_process(SOURCE).expect("in-process route");
    let (b_plain, b_deleting) = moved(SOURCE).expect("moved route");
    assert_eq!(a_plain, b_plain, "the non-deleting certificates differ");
    assert_eq!(a_deleting, b_deleting, "the deleting certificates differ");
    // The certificates are real two-level compositions, not empty ones: the
    // deleting reading embeds the non-deleting one, which embeds the nested
    // composition's certificate, which embeds its own two prerequisites'.
    assert!(a_deleting.contains(&a_plain[..64]));
    assert_eq!(a_plain.matches("\"unmap_case\"").count(), 43);
    assert_eq!(a_deleting.matches("\"delete_case\"").count(), 30);
    // The nested composition's own published figure survives into this level.
    assert!(a_plain.contains("\"native_source_path\""));
}

/// Refusal agreement: every body the retained route refuses is refused by the
/// moved route too, at both entry points, and the row count is pinned.
///
/// The rows deliberately span all three levels — the wrapper itself, the nested
/// composition's wrapper, the nested composition's own prerequisite body, the
/// native close body and the guarded-delete body — so a refusal that only one
/// level notices is still measured on both routes.
#[test]
fn refusal_agreement_on_mutated_bodies() {
    if !clang_available() {
        return;
    }
    let rows = [
        // The wrapper releases the backend before its path.
        ("free(cf->path);free(cf);", "free(cf);free(cf->path);"),
        // It frees the wrong allocation.
        ("free(cf->path);free(cf);", "free(ch->name);free(cf);"),
        // It stops releasing the backend at all.
        ("free(cf->path);free(cf);", "free(cf->path);"),
        // It reorders the two cleanup calls.
        (
            "munmap_file(ch->ctx,ch);cio_file_native_close(cf);",
            "cio_file_native_close(cf);munmap_file(ch->ctx,ch);",
        ),
        // It observes a cleanup result it must ignore.
        (
            "munmap_file(ch->ctx,ch);cio_file_native_close(cf);",
            "if(munmap_file(ch->ctx,ch))return;cio_file_native_close(cf);",
        ),
        // It passes the wrong identity to the nested composition.
        (
            "munmap_file(ch->ctx,ch);cio_file_native_close(cf);",
            "munmap_file((void*)0,ch);cio_file_native_close(cf);",
        ),
        // The deleting arm guards on the wrong value.
        ("if(delete==1)", "if(delete==0)"),
        // Its diagnostic reports the wrong arm.
        ("if(r!=0)cio_log_print", "if(r==0)cio_log_print"),
        // Its diagnostic transposes the two names it reports.
        (
            "at close %s:%s\",ch->st->name,ch->name",
            "at close %s:%s\",ch->name,ch->st->name",
        ),
        // Level two: the nested composition's wrapper stops decrementing.
        ("cio_chunk_counter_total_up_sub(ctx);return 0;", "return 0;"),
        // Level two: it decrements the wrong counter.
        (
            "cio_chunk_counter_total_up_sub(ctx)",
            "cio_chunk_counter_total_sub(ctx)",
        ),
        // Level three: the nested composition's own prerequisite body stops
        // resetting a declared field.
        ("cf->map_truncated_warned=0;return 0;", "return 0;"),
        // The native close body stops resetting its declared field.
        ("cf->fd=-1;}return 0;}", "}return 0;}"),
        // The guarded-delete body drops one of its two eligibility guards.
        (
            "if(cf->fd!=-1 || cf->map!=(void*)0)return -1;",
            "if(cf->fd!=-1)return -1;",
        ),
    ];
    assert_eq!(rows.len(), 14, "row count moved");
    for (old, new) in rows {
        let mutated = SOURCE.replace(old, new);
        assert_ne!(mutated, SOURCE, "`{old}` did not apply");
        assert!(
            in_process(&mutated).is_err(),
            "the retained route accepted `{old}` -> `{new}`"
        );
        assert!(
            moved(&mutated).is_err(),
            "the moved route accepted `{old}` -> `{new}`"
        );
    }
}
