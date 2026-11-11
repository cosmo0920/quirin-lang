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

//! Manifest-resident unchecked linkage: the waiver premise that names **which
//! build** was waived (card `UNSAFE-LINKAGE-ARTIFACT`,
//! `docs/design/NATIVE_LIBRARY_LINKAGE_V1.md` §4 and
//! `docs/design/UNSAFE_LINKAGE_V1.md` §4.2 — level L1 of the linkage spectrum).
//!
//! Four things are pinned here, and they are not the same thing.
//!
//! 1. **The unmanifested world does not move.** With no manifest selected, and
//!    with a manifest that names other symbols, every byte of a wave-18 waiver
//!    is what it was: the same bare premise, the same certificate, the same
//!    emitted C.
//! 2. **A manifest sharpens, and only sharpens.** A resident symbol's premise
//!    gains the artifact identity and the stated ABI premises. The admitted
//!    surface is untouched — a manifest cannot buy a pointer.
//! 3. **Disagreement refuses.** A manifest that contradicts the declaration
//!    refuses the module rather than withdrawing to the weaker premise, and a
//!    manifest that does not load refuses rather than being ignored.
//! 4. **The premise still cannot be dropped.** All three of card
//!    `UNSAFE-LINKAGE`'s omission guards — structural marking, independent
//!    rederivation, publication tripwire — cover the sharpened premise too.
use std::collections::BTreeMap;

use qcore_compiler::{
    CompileOptions, SourceExecutionProfile, SourceFile, compile_files, extern_call, native_library,
};

const ROUND_TRIP: &str = include_str!("fixtures/unsafe_linkage/round_trip.qr");
const MANIFEST: &str = include_str!("fixtures/native_library/unchecked_demo.qrlib");
const MANIFEST_PATH: &str = "fixtures/native_library/unchecked_demo.qrlib";
const CONTRACTS: &str = include_str!("fixtures/extern_calls/contracts/buffer_api.qv");
const CONTRACTED_ROUND_TRIP: &str = include_str!("fixtures/extern_calls/round_trip.qr");

fn selection(manifests: &[(&str, String)]) -> Option<BTreeMap<String, String>> {
    Some(
        manifests
            .iter()
            .map(|(path, text)| ((*path).to_owned(), text.clone()))
            .collect(),
    )
}

fn compile_with(
    source: &str,
    native_libraries: Option<BTreeMap<String, String>>,
) -> Result<qcore_compiler::CompiledModule, Vec<qcore_compiler::FileDiagnostic>> {
    compile_files(
        &[SourceFile::new("unsafe.qcore", source)],
        &CompileOptions {
            entry: None,
            extern_contracts: None,
            native_libraries,
            native_library_headers: None,
        },
    )
}

fn codes(source: &str, native_libraries: Option<BTreeMap<String, String>>) -> Vec<String> {
    compile_with(source, native_libraries)
        .map(|module| module.execution_profile)
        .expect_err("this fixture must be refused")
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect()
}

fn premises(module: &qcore_compiler::CompiledModule) -> Vec<extern_call::ExternAssumption> {
    module.declared_assumptions().to_vec()
}

fn manifest_premises(
    module: &qcore_compiler::CompiledModule,
) -> Vec<(String, String, String, String)> {
    premises(module)
        .into_iter()
        .filter_map(|assumption| match assumption {
            extern_call::ExternAssumption::ManifestUncheckedLinkage {
                callee,
                signature,
                residence,
            } => Some((
                callee,
                signature,
                residence.identity.artifact.clone(),
                residence.identity.hash.clone(),
            )),
            _ => None,
        })
        .collect()
}

fn bare_premises(module: &qcore_compiler::CompiledModule) -> Vec<String> {
    premises(module)
        .into_iter()
        .filter_map(|assumption| match assumption {
            extern_call::ExternAssumption::UncheckedLinkage { callee, .. } => Some(callee),
            _ => None,
        })
        .collect()
}

/// The fixture manifest with one entry rewritten, for the disagreement rows.
fn manifest_with(original: &str, replacement: &str) -> String {
    let edited = MANIFEST.replace(original, replacement);
    assert_ne!(edited, MANIFEST, "the manifest edit must have applied");
    edited
}

// --- The sharpened premise ---

#[test]
fn a_manifested_waiver_names_the_artifact_it_was_waived_from() {
    let module = compile_with(
        ROUND_TRIP,
        selection(&[(MANIFEST_PATH, MANIFEST.to_owned())]),
    )
    .expect("a manifest that agrees with the declarations admits them");
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::ExternCallsV1
    );
    let hash = "sha256:9f2c4a1e7b3d05860f14ca9e2d7b58311c06ae4f9d2b7c8e5a0341f6d9b8c72e";
    assert_eq!(
        manifest_premises(&module),
        vec![
            (
                "k_unchecked_add".to_owned(),
                "(int32_t, int32_t) -> int32_t".to_owned(),
                "libqcore_unchecked_demo.a".to_owned(),
                hash.to_owned(),
            ),
            (
                "k_unchecked_measure".to_owned(),
                "(uint64_t) -> uint64_t".to_owned(),
                "libqcore_unchecked_demo.a".to_owned(),
                hash.to_owned(),
            ),
            (
                "k_unchecked_side_effect".to_owned(),
                "(uint32_t) -> void".to_owned(),
                "libqcore_unchecked_demo.a".to_owned(),
                hash.to_owned(),
            ),
        ]
    );
    // A sharpened premise replaces the bare one; it never sits beside it, so a
    // reader cannot see two different statements about the same callee.
    assert!(bare_premises(&module).is_empty());
    // And the stated ABI premises travel with it, all four.
    let residences = premises(&module)
        .into_iter()
        .filter_map(|assumption| match assumption {
            extern_call::ExternAssumption::ManifestUncheckedLinkage { residence, .. } => {
                Some(residence)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    for residence in &residences {
        assert_eq!(
            residence.premises,
            native_library::AbiPremises {
                data_model: native_library::DataModel::Lp64,
                unwinding: native_library::UnwindingPosture::Abort,
                allocator_custody: native_library::AllocatorCustody::Closed,
                thread_posture: native_library::ThreadPosture::SingleCaller,
            }
        );
        assert_eq!(residence.identity.library, "qcore_unchecked_demo");
        assert_eq!(residence.identity.version, "0.1.0");
    }
}

#[test]
fn the_sharpened_premise_says_strictly_more_and_stays_distinguishable() {
    // Level L0 and level L1 are separate records rather than one record with an
    // optional field, so an L0 premise stays a valid *weaker* statement instead
    // of becoming a malformed L1 one, and a reader tells them apart by
    // inspection. Both are present in this pair of modules and neither can be
    // read as the other.
    let bare = compile_with(ROUND_TRIP, None).unwrap();
    let sharp = compile_with(
        ROUND_TRIP,
        selection(&[(MANIFEST_PATH, MANIFEST.to_owned())]),
    )
    .unwrap();
    assert_eq!(bare_premises(&bare).len(), 3);
    assert!(manifest_premises(&bare).is_empty());
    assert!(bare_premises(&sharp).is_empty());
    assert_eq!(manifest_premises(&sharp).len(), 3);
    assert_eq!(premises(&bare).len(), premises(&sharp).len());
}

// --- The unmanifested world does not move ---

#[test]
fn without_a_manifest_every_byte_of_the_wave_18_route_is_unchanged() {
    let bare = compile_with(ROUND_TRIP, None).unwrap();
    // `None` and an explicitly empty selection are the same thing, and both are
    // the wave-18 world.
    let empty = compile_with(ROUND_TRIP, Some(BTreeMap::new())).unwrap();
    assert_eq!(premises(&bare), premises(&empty));
    assert_eq!(
        bare_premises(&bare),
        vec![
            "k_unchecked_add".to_owned(),
            "k_unchecked_measure".to_owned(),
            "k_unchecked_side_effect".to_owned(),
        ]
    );
    assert_eq!(bare.c_source, empty.c_source);
}

#[test]
fn a_manifest_that_names_other_symbols_changes_nothing() {
    let unrelated = manifest_with(
        "unchecked fn k_unchecked_add(int32_t, int32_t) -> int32_t;",
        "unchecked fn k_somewhere_else(int32_t) -> int32_t;",
    );
    let unrelated = unrelated
        .lines()
        .filter(|line| {
            !line.contains("k_unchecked_side_effect") && !line.contains("k_unchecked_measure")
        })
        .collect::<Vec<_>>()
        .join("\n");
    let module = compile_with(ROUND_TRIP, selection(&[("other.qrlib", unrelated)])).unwrap();
    let bare = compile_with(ROUND_TRIP, None).unwrap();
    assert_eq!(premises(&module), premises(&bare));
    assert!(manifest_premises(&module).is_empty());
}

#[test]
fn a_manifest_naming_some_symbols_sharpens_exactly_those() {
    // Partial residence is admitted and is the interesting middle case: the two
    // premise kinds coexist in one certificate, each saying exactly what it
    // knows, and the module is not refused for the symbols the manifest omits.
    let partial = MANIFEST
        .lines()
        .filter(|line| !line.contains("k_unchecked_measure"))
        .collect::<Vec<_>>()
        .join("\n");
    let module = compile_with(ROUND_TRIP, selection(&[("partial.qrlib", partial)])).unwrap();
    assert_eq!(
        bare_premises(&module),
        vec!["k_unchecked_measure".to_owned()]
    );
    assert_eq!(
        manifest_premises(&module)
            .into_iter()
            .map(|(callee, ..)| callee)
            .collect::<Vec<_>>(),
        vec![
            "k_unchecked_add".to_owned(),
            "k_unchecked_side_effect".to_owned()
        ]
    );
}

#[test]
fn the_emitted_c_and_the_checked_evidence_are_unchanged_by_the_manifest() {
    // A manifest is a statement about provenance, not about execution. Nothing
    // it carries reaches the backend or the reference interpreter.
    let bare = compile_with(ROUND_TRIP, None).unwrap();
    let sharp = compile_with(
        ROUND_TRIP,
        selection(&[(MANIFEST_PATH, MANIFEST.to_owned())]),
    )
    .unwrap();
    assert_eq!(bare.c_source, sharp.c_source);
    let bare_checked = bare.checked.as_extern_calls().unwrap();
    let sharp_checked = sharp.checked.as_extern_calls().unwrap();
    assert_eq!(
        bare_checked.certificate.calls,
        sharp_checked.certificate.calls
    );
    assert_eq!(
        bare_checked.certificate.effects,
        sharp_checked.certificate.effects
    );
    assert_eq!(
        bare_checked.certificate.acquisitions,
        sharp_checked.certificate.acquisitions
    );
    // The one field that differs is the assumption list, which is the point.
    assert_ne!(
        bare_checked.certificate.assumptions,
        sharp_checked.certificate.assumptions
    );
}

// --- Q041: the manifest type gate ---

#[test]
fn a_manifest_signature_the_declaration_contradicts_refuses() {
    for replacement in [
        // A different result type.
        "unchecked fn k_unchecked_add(int32_t, int32_t) -> int64_t;",
        // A different parameter type.
        "unchecked fn k_unchecked_add(int32_t, uint32_t) -> int32_t;",
        // A different arity.
        "unchecked fn k_unchecked_add(int32_t) -> int32_t;",
    ] {
        let edited = manifest_with(
            "unchecked fn k_unchecked_add(int32_t, int32_t) -> int32_t;",
            replacement,
        );
        assert_eq!(
            codes(ROUND_TRIP, selection(&[(MANIFEST_PATH, edited)])),
            ["Q041"],
            "`{replacement}` contradicts the declaration and must refuse"
        );
    }
}

#[test]
fn a_disagreement_refuses_rather_than_withdrawing_to_the_weaker_premise() {
    // The contracted route withdraws a disagreeing declaration and lets the
    // call sites fall through to a refusal. Here the only weaker reading is the
    // *bare* waiver, so a withdrawal would reward a contradiction with a
    // premise that says less about the artifact. There is therefore no
    // accepted-with-bare-premise outcome to find.
    let edited = manifest_with(
        "unchecked fn k_unchecked_add(int32_t, int32_t) -> int32_t;",
        "unchecked fn k_unchecked_add(int32_t, int32_t) -> int64_t;",
    );
    let refused = compile_with(ROUND_TRIP, selection(&[(MANIFEST_PATH, edited)]));
    assert!(refused.is_err());
}

#[test]
fn a_manifest_cannot_buy_a_pointer() {
    // The admitted surface is decided before the manifest is consulted, so a
    // pointer declaration refuses at `Q033` whether or not a manifest names the
    // symbol. An artifact hash says which code, not what it does with your
    // pointer.
    let source = ROUND_TRIP.replace(
        "unsafe extern fn k_unchecked_add(a: i32, b: i32) -> i32;",
        "unsafe extern fn k_unchecked_add(a: ptr<const, u8>, b: i32) -> i32;",
    );
    assert_ne!(source, ROUND_TRIP);
    assert_eq!(
        codes(&source, selection(&[(MANIFEST_PATH, MANIFEST.to_owned())])),
        ["Q033"]
    );
    // And the manifest could not have declared one either: a contract-less
    // entry with a pointer rejects the whole manifest at load.
    let pointer_entry = manifest_with(
        "unchecked fn k_unchecked_add(int32_t, int32_t) -> int32_t;",
        "unchecked fn k_unchecked_add(const uint8_t *, int32_t) -> int32_t;",
    );
    assert_eq!(
        codes(ROUND_TRIP, selection(&[(MANIFEST_PATH, pointer_entry)])),
        ["Q039"]
    );
}

// --- Q039/Q040: the selection ---

#[test]
fn a_manifest_that_does_not_load_refuses_rather_than_being_ignored() {
    // Card `UNSAFE-LINKAGE` §7.1's ground, applied to this surface: a silently
    // dropped selection would downgrade every resident premise to a bare
    // waiver, so shipping a deliberately broken manifest would become a way to
    // stop a certificate saying which build it rests on.
    for broken in [
        MANIFEST.replace("data_model lp64;", "data_model ilp32;"),
        manifest_with("library qcore_unchecked_demo;", ""),
        // A `contract fn` with no `from MODULE` tail. Under card
        // `UNSAFE-LINKAGE-ARTIFACT` this refused because a contracted entry was
        // an entry kind the loader declined; under card
        // `NATIVE-LIBRARY-CONTRACT-SLICE` it refuses because the two entry kinds
        // are told apart by their **keyword** and a contract entry that names no
        // contract module is malformed rather than demoted to a waiver. The
        // reason moved, the refusal and its code did not.
        MANIFEST.replace(
            "unchecked fn k_unchecked_add",
            "contract fn k_unchecked_add",
        ),
        manifest_with("native_library manifest v1;", ""),
        MANIFEST.replace("hash sha256:9f2c", "hash sha256:zzzz"),
    ] {
        assert_eq!(
            codes(ROUND_TRIP, selection(&[(MANIFEST_PATH, broken)])),
            ["Q039"]
        );
    }
}

#[test]
fn a_broken_manifest_refuses_even_when_no_callee_would_have_resolved() {
    let unrelated_symbols = MANIFEST
        .replace("k_unchecked_add", "k_elsewhere_a")
        .replace("k_unchecked_side_effect", "k_elsewhere_b")
        .replace("k_unchecked_measure", "k_elsewhere_c")
        .replace("unwinding abort;", "unwinding propagate;");
    assert_eq!(
        codes(ROUND_TRIP, selection(&[(MANIFEST_PATH, unrelated_symbols)])),
        ["Q039"]
    );
}

#[test]
fn two_manifests_may_not_bind_the_same_symbol() {
    // The binding must be a function: a symbol resolving to two artifacts has
    // no defined premise, and resolving it by selection order would make the
    // published premise depend on the order the builder listed its manifests.
    let twin = MANIFEST.replace("library qcore_unchecked_demo;", "library qcore_twin;");
    assert_eq!(
        codes(
            ROUND_TRIP,
            selection(&[(MANIFEST_PATH, MANIFEST.to_owned()), ("twin.qrlib", twin)])
        ),
        ["Q040"]
    );
}

// --- Q042: the mirror of Q034 ---

#[test]
fn a_contract_may_not_be_asserted_about_a_manifest_declared_contract_less_symbol() {
    // `Q034` refuses a waiver for a name the contract environment declares: you
    // may not waive a contract you have. This is its mirror — the named build's
    // own manifest says this symbol has no contract, so asserting modes and
    // effects about it is refused too. Where two declarations are visible they
    // are compared, and the refusing direction wins.
    let contracted = CONTRACTED_ROUND_TRIP;
    let manifest = manifest_with(
        "unchecked fn k_unchecked_add(int32_t, int32_t) -> int32_t;",
        "unchecked fn qcore_extern_sum(uintptr_t) -> int32_t;",
    );
    let refused = compile_files(
        &[SourceFile::new("contracted.qcore", contracted)],
        &CompileOptions {
            entry: None,
            extern_contracts: Some(BTreeMap::from([(
                "fixtures/extern_calls/contracts/buffer_api.qcore".to_owned(),
                CONTRACTS.to_owned(),
            )])),
            native_libraries: selection(&[(MANIFEST_PATH, manifest)]),
            native_library_headers: None,
        },
    )
    .expect_err("a contract may not be asserted about a contract-less manifest symbol");
    assert!(
        refused.iter().any(|diagnostic| diagnostic.code == "Q042"),
        "{refused:?}"
    );
}

// --- The three omission guards, extended to the sharpened premise ---

#[test]
fn the_publication_tripwire_covers_the_sharpened_premise() {
    let module = compile_with(
        ROUND_TRIP,
        selection(&[(MANIFEST_PATH, MANIFEST.to_owned())]),
    )
    .unwrap();
    let checked = module.checked.as_extern_calls().unwrap();
    // The demanded inventory, recomputed from the program alone, is the
    // sharpened one — not the bare one it would have been without a manifest.
    let required = extern_call::required_unchecked_assumptions(&checked.lowered.program);
    assert_eq!(required.len(), 3);
    assert!(required.iter().all(|assumption| matches!(
        assumption,
        extern_call::ExternAssumption::ManifestUncheckedLinkage { .. }
    )));
    assert!(extern_call::certificate_carries_unchecked_linkage(
        &checked.lowered.program,
        &checked.certificate
    ));
    // Dropping a sharpened premise is caught, and so is *downgrading* one to
    // the bare premise, which is the interesting new laundering vector: a
    // certificate that quietly forgot which build it rests on.
    let mut laundered = checked.certificate.clone();
    laundered.assumptions = laundered
        .assumptions
        .into_iter()
        .map(|assumption| match assumption {
            extern_call::ExternAssumption::ManifestUncheckedLinkage {
                callee, signature, ..
            } => extern_call::ExternAssumption::UncheckedLinkage { callee, signature },
            other => other,
        })
        .collect();
    assert!(!extern_call::certificate_carries_unchecked_linkage(
        &checked.lowered.program,
        &laundered
    ));
    assert!(
        extern_call::recheck_extern_certificate(&checked.lowered.program, &laundered).is_err(),
        "a downgraded assumption list must not survive the independent recheck"
    );
}

#[test]
fn the_structural_marking_reaches_every_callee_lookup() {
    let module = compile_with(
        ROUND_TRIP,
        selection(&[(MANIFEST_PATH, MANIFEST.to_owned())]),
    )
    .unwrap();
    let checked = module.checked.as_extern_calls().unwrap();
    for (name, callee) in &checked.lowered.program.callees {
        assert_eq!(callee.trust, extern_call::ExternTrust::UncheckedLinkage);
        let residence = callee
            .residence
            .as_ref()
            .unwrap_or_else(|| panic!("`{name}` is manifest-resident"));
        assert_eq!(residence.identity.library, "qcore_unchecked_demo");
        // The premise is a function of the callee, so a rederivation reproduces
        // exactly what was published.
        assert!(
            checked
                .certificate
                .assumptions
                .contains(&extern_call::assumption_for_callee(callee))
        );
    }
}

#[test]
fn a_contracted_callee_never_names_an_artifact() {
    // The manifest slice this card landed binds contract-less entries only, so
    // there is no representation of a contracted callee carrying a residence —
    // and therefore no way for a declared contract to acquire an artifact
    // premise it was not measured against.
    let module = compile_files(
        &[SourceFile::new("contracted.qcore", CONTRACTED_ROUND_TRIP)],
        &CompileOptions {
            entry: None,
            extern_contracts: Some(BTreeMap::from([(
                "fixtures/extern_calls/contracts/buffer_api.qcore".to_owned(),
                CONTRACTS.to_owned(),
            )])),
            native_libraries: selection(&[(MANIFEST_PATH, MANIFEST.to_owned())]),
            native_library_headers: None,
        },
    )
    .unwrap();
    assert!(manifest_premises(&module).is_empty());
}
