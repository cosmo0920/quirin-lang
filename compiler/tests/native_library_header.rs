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

//! The **exported-header level** of `source.native-library.v1` (card
//! `NATIVE-LIBRARY-HEADER`; design
//! `docs/design/NATIVE_LIBRARY_HEADER_V1.md`).
//!
//! Waves 19 and 20 built a manifest whose entries carry the exported header
//! declaration **as the author transcribed it**, and nothing compared that
//! transcription against the header the library ships. Since wave 20 it is a
//! transcription that *pointers* rest on. This suite pins the four things
//! reading the real `.h` must and must not do.
//!
//! 1. **A manifest bound to its real header is accepted, and the certificate
//!    says so.** The residence premise gains one **verified** identity — a
//!    reference that was read and a digest measured over exactly the bytes read
//!    — beside three declared ones.
//! 2. **A header that contradicts the manifest refuses.** Not a withdrawal: the
//!    weaker admissible reading here is the manifest *without* a header, and
//!    that is accepted, so withdrawing would reward a contradiction with a
//!    premise that says less. The weaker reading is exhibited beside the
//!    refusal.
//! 3. **A declaration outside the v1 subset refuses by name**, naming the symbol
//!    and the token that put it outside.
//! 4. **Nothing about the headerless world moves.** A manifest that names no
//!    header is byte-identical to what wave 20 published, whatever header texts
//!    are supplied.
//!
//! §5 is the **subset census** over the pinned Libevent submodule, measured by
//! the shipped reader.
use std::collections::{BTreeMap, BTreeSet};

use qcore_compiler::native_library_header::{HeaderReading, census_candidates, read_header};
use qcore_compiler::{
    CompileOptions, SourceExecutionProfile, SourceFile, compile_files, extern_call,
};

const ROUND_TRIP: &str = include_str!("fixtures/extern_calls/round_trip.qr");
const SLICE_MODULE: &str = "fixtures/native_library/contracts/buffer_lib.qv";
const SLICE: &str = include_str!("fixtures/native_library/contracts/buffer_lib.qv");
const MANIFEST_PATH: &str = "fixtures/native_library/buffer_lib_header.qrlib";
const MANIFEST: &str = include_str!("fixtures/native_library/buffer_lib_header.qrlib");
const HEADERLESS_PATH: &str = "fixtures/native_library/buffer_lib.qrlib";
const HEADERLESS: &str = include_str!("fixtures/native_library/buffer_lib.qrlib");
const HEADER_REFERENCE: &str = "include/qcore_buffer_demo.h";
const HEADER: &str = include_str!("fixtures/native_library/include/qcore_buffer_demo.h");
const MISMATCHED: &str =
    include_str!("fixtures/native_library/include/qcore_buffer_demo_mismatched.h");
const DECORATED: &str =
    include_str!("fixtures/native_library/include/qcore_buffer_demo_decorated.h");

/// The fixture header's text with line endings normalised.
///
/// **`HEADER-FIXTURE-EOL-NORMALISED`.** The manifest fixture pins this file's
/// SHA-256, and a checkout that translated line endings would hand the test
/// different bytes for the same file. Normalising here keeps the pin a statement
/// about the header's *content* — which is what the level is about — instead of
/// about a checkout's newline policy, which it is not. Nothing in the shipped
/// reader normalises anything: the driver digests exactly the bytes it was
/// given.
fn header_text(text: &str) -> String {
    text.replace("\r\n", "\n")
}

fn selection(entries: &[(&str, &str)]) -> Option<BTreeMap<String, String>> {
    Some(
        entries
            .iter()
            .map(|(key, text)| ((*key).to_owned(), (*text).to_owned()))
            .collect(),
    )
}

fn compile(
    manifest: (&str, &str),
    headers: Option<BTreeMap<String, String>>,
) -> Result<qcore_compiler::CompiledModule, Vec<qcore_compiler::FileDiagnostic>> {
    compile_files(
        &[SourceFile::new("extern.qcore", ROUND_TRIP)],
        &CompileOptions {
            entry: None,
            extern_contracts: selection(&[(SLICE_MODULE, SLICE)]),
            native_libraries: selection(&[manifest]),
            native_library_headers: headers,
        },
    )
}

fn bound_header(text: &str) -> Option<BTreeMap<String, String>> {
    Some(BTreeMap::from([(
        HEADER_REFERENCE.to_owned(),
        header_text(text),
    )]))
}

fn codes(
    manifest: (&str, &str),
    headers: Option<BTreeMap<String, String>>,
) -> Vec<(String, String)> {
    compile(manifest, headers)
        .map(|module| module.execution_profile)
        .expect_err("this fixture must be refused")
        .into_iter()
        .map(|diagnostic| (diagnostic.code, diagnostic.message))
        .collect()
}

/// The manifest with one entry or field rewritten.
fn manifest_with(original: &str, replacement: &str) -> String {
    let edited = MANIFEST.replace(original, replacement);
    assert_ne!(edited, MANIFEST, "the manifest edit must have applied");
    edited
}

// --- 1. The payoff: a manifest bound to its real header ---

#[test]
fn a_manifest_bound_to_its_real_header_is_accepted() {
    let module = compile((MANIFEST_PATH, MANIFEST), bound_header(HEADER))
        .expect("the header declares exactly what the manifest transcribes");
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::ExternCallsV1
    );
    let mut bound = module
        .declared_assumptions()
        .iter()
        .filter_map(|assumption| match assumption {
            extern_call::ExternAssumption::ManifestResidentContract {
                callee, residence, ..
            } => residence.header.as_ref().map(|header| {
                (
                    callee.clone(),
                    header.reference.clone(),
                    header.digest.clone(),
                )
            }),
            _ => None,
        })
        .collect::<Vec<_>>();
    bound.sort();
    let digest = "sha256:aa92b5d783202524bad45bcc39750944ab19af0547f555429623fc8a2a49a657";
    assert_eq!(
        bound,
        [
            (
                "qcore_extern_alloc".to_owned(),
                HEADER_REFERENCE.to_owned(),
                digest.to_owned()
            ),
            (
                "qcore_extern_fill".to_owned(),
                HEADER_REFERENCE.to_owned(),
                digest.to_owned()
            ),
            (
                "qcore_extern_release".to_owned(),
                HEADER_REFERENCE.to_owned(),
                digest.to_owned()
            ),
            (
                "qcore_extern_sum".to_owned(),
                HEADER_REFERENCE.to_owned(),
                digest.to_owned()
            ),
        ],
        "every manifest-resident premise names the header it was checked against"
    );
}

#[test]
fn the_header_binding_changes_the_premises_and_nothing_else() {
    // The manifest bound to its header, and the same manifest without the two
    // header fields, compiled over the same source. Everything a certificate
    // carries is equal but the residence's header identity — which is the whole
    // observable delta of this level.
    let with = compile((MANIFEST_PATH, MANIFEST), bound_header(HEADER)).expect("accepted");
    let without = compile((HEADERLESS_PATH, HEADERLESS), None).expect("accepted");
    assert_eq!(with.execution_profile, without.execution_profile);
    assert_eq!(with.c_source, without.c_source);
    assert_eq!(with.entry, without.entry);
    assert_eq!(with.declaration_count, without.declaration_count);
    let strip = |module: &qcore_compiler::CompiledModule| {
        module
            .declared_assumptions()
            .iter()
            .map(|assumption| match assumption {
                extern_call::ExternAssumption::ManifestResidentContract {
                    callee,
                    contract_module,
                    residence,
                } => {
                    let mut residence = residence.clone();
                    residence.header = None;
                    extern_call::ExternAssumption::ManifestResidentContract {
                        callee: callee.clone(),
                        contract_module: contract_module.clone(),
                        residence,
                    }
                }
                other => other.clone(),
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(
        strip(&with),
        strip(&without),
        "with the header identity removed the two premise inventories are equal"
    );
    assert!(
        with.declared_assumptions() != without.declared_assumptions(),
        "and with it present they are not — the level is not a no-op"
    );
}

// --- 2. A contradicting header refuses ---

#[test]
fn a_header_that_contradicts_the_manifest_refuses() {
    let digest = digest_of(MISMATCHED);
    let manifest = manifest_with(
        "header_hash sha256:aa92b5d783202524bad45bcc39750944ab19af0547f555429623fc8a2a49a657;",
        &format!("header_hash {digest};"),
    );
    let refused = codes((MANIFEST_PATH, &manifest), bound_header(MISMATCHED));
    assert_eq!(
        refused
            .iter()
            .map(|(code, _)| code.clone())
            .collect::<Vec<_>>(),
        ["Q064"],
        "one refusal, and it is the disagreement's own code"
    );
    let message = &refused[0].1;
    assert!(
        message.contains("qcore_extern_sum") && message.contains("parameter 0"),
        "the refusal names the symbol and the position: {message}"
    );
}

#[test]
fn a_disagreement_refuses_rather_than_withdrawing_to_the_headerless_reading() {
    // The rule both this card and card `NATIVE-LIBRARY-CONTRACT-SLICE` apply:
    // withdraw only where the weaker admissible reading refuses. Here it is
    // *accepted*, and the acceptance is exhibited rather than asserted about an
    // empty set — which is what makes the refusal a decision and not a default.
    let digest = digest_of(MISMATCHED);
    let manifest = manifest_with(
        "header_hash sha256:aa92b5d783202524bad45bcc39750944ab19af0547f555429623fc8a2a49a657;",
        &format!("header_hash {digest};"),
    );
    assert_eq!(
        codes((MANIFEST_PATH, &manifest), bound_header(MISMATCHED))
            .iter()
            .map(|(code, _)| code.clone())
            .collect::<Vec<_>>(),
        ["Q064"]
    );
    compile((HEADERLESS_PATH, HEADERLESS), None)
        .expect("the weaker reading — this manifest without a header — is admitted");
}

#[test]
fn a_manifest_entry_edited_away_from_its_header_refuses() {
    // The other direction of the same disagreement: the header is the fixture's
    // real one and the manifest's transcription is what moved.
    for (original, replacement) in [
        (
            "contract fn qcore_extern_sum(const uint8_t *, uint32_t) -> int32_t",
            "contract fn qcore_extern_sum(const uint8_t *, uint32_t) -> int64_t",
        ),
        (
            "contract fn qcore_extern_fill(uint8_t *, uint32_t) -> void",
            "contract fn qcore_extern_fill(uint8_t *) -> void",
        ),
        (
            "contract fn qcore_extern_alloc(uintptr_t) -> uint8_t *",
            "contract fn qcore_extern_alloc(uint32_t) -> uint8_t *",
        ),
    ] {
        let manifest = manifest_with(original, replacement);
        let refused = codes((MANIFEST_PATH, &manifest), bound_header(HEADER));
        assert_eq!(
            refused
                .iter()
                .map(|(code, _)| code.clone())
                .collect::<Vec<_>>(),
            ["Q064"],
            "editing `{original}` must refuse at the disagreement's code"
        );
    }
}

// --- 3. Outside the subset: refuse by name ---

#[test]
fn a_macro_decorated_declaration_refuses_naming_the_token() {
    let digest = digest_of(DECORATED);
    let manifest = manifest_with(
        "header_hash sha256:aa92b5d783202524bad45bcc39750944ab19af0547f555429623fc8a2a49a657;",
        &format!("header_hash {digest};"),
    );
    let refused = codes((MANIFEST_PATH, &manifest), bound_header(DECORATED));
    assert_eq!(
        refused
            .iter()
            .map(|(code, _)| code.clone())
            .collect::<Vec<_>>(),
        ["Q063"]
    );
    let message = &refused[0].1;
    assert!(
        message.contains("QCORE_BUFFER_DEMO_EXPORT"),
        "the refusal names the token that puts the declaration outside the subset: {message}"
    );
}

#[test]
fn a_header_the_manifest_names_and_nobody_supplied_refuses() {
    let refused = codes((MANIFEST_PATH, MANIFEST), None);
    assert_eq!(
        refused
            .iter()
            .map(|(code, _)| code.clone())
            .collect::<Vec<_>>(),
        ["Q063"]
    );
    assert!(
        refused[0].1.contains("no text was supplied"),
        "a header binding this driver cannot apply is not silently dropped: {}",
        refused[0].1
    );
}

#[test]
fn a_header_whose_digest_is_not_the_declared_one_refuses() {
    // The digest gate, at the boundary that matters: the *substituted* header
    // agrees with the manifest in every declaration. Only its identity differs,
    // and only the digest can see that.
    let substituted = header_text(HEADER).replace("/* Consume. */", "/* Release. */");
    let refused = codes(
        (MANIFEST_PATH, MANIFEST),
        Some(BTreeMap::from([(HEADER_REFERENCE.to_owned(), substituted)])),
    );
    assert_eq!(
        refused
            .iter()
            .map(|(code, _)| code.clone())
            .collect::<Vec<_>>(),
        ["Q063"]
    );
    assert!(
        refused[0].1.contains("measures"),
        "the refusal names both digests: {}",
        refused[0].1
    );
}

#[test]
fn a_bound_symbol_the_header_does_not_declare_refuses() {
    let stripped = header_text(HEADER).replace(
        "void qcore_extern_release(uint8_t *buffer);",
        "/* removed */",
    );
    let digest = digest_of(&stripped);
    let manifest = manifest_with(
        "header_hash sha256:aa92b5d783202524bad45bcc39750944ab19af0547f555429623fc8a2a49a657;",
        &format!("header_hash {digest};"),
    );
    let refused = codes(
        (MANIFEST_PATH, &manifest),
        Some(BTreeMap::from([(HEADER_REFERENCE.to_owned(), stripped)])),
    );
    assert_eq!(
        refused
            .iter()
            .map(|(code, _)| code.clone())
            .collect::<Vec<_>>(),
        ["Q063"]
    );
    assert!(
        refused[0].1.contains("qcore_extern_release"),
        "the refusal names the symbol: {}",
        refused[0].1
    );
}

// --- 4. The headerless world does not move ---

#[test]
fn a_manifest_naming_no_header_is_unaffected_by_supplied_texts() {
    // Not "no source matched": the supplied text contradicts every entry of the
    // manifest. A manifest that never opted in is not checked against it, so
    // this level cannot refuse a program that predates it.
    let module = compile(
        (HEADERLESS_PATH, HEADERLESS),
        Some(BTreeMap::from([(
            HEADER_REFERENCE.to_owned(),
            "void qcore_extern_alloc(void);".to_owned(),
        )])),
    )
    .expect("a manifest that names no header is untouched");
    assert!(
        module
            .declared_assumptions()
            .iter()
            .all(|assumption| match assumption {
                extern_call::ExternAssumption::ManifestResidentContract { residence, .. }
                | extern_call::ExternAssumption::ManifestUncheckedLinkage { residence, .. } =>
                    residence.header.is_none(),
                _ => true,
            }),
        "and it publishes no header identity"
    );
}

#[test]
fn the_two_header_fields_are_mandatory_together() {
    let without_hash = manifest_with(
        "header_hash sha256:aa92b5d783202524bad45bcc39750944ab19af0547f555429623fc8a2a49a657;",
        "",
    );
    let refused = codes((MANIFEST_PATH, &without_hash), bound_header(HEADER));
    assert_eq!(
        refused
            .iter()
            .map(|(code, _)| code.clone())
            .collect::<Vec<_>>(),
        ["Q039"],
        "a half-stated header binding is a manifest that does not load, not a header refusal"
    );
}

// --- 5. The subset census, over the pinned Libevent submodule ---

/// One header's census row.
#[derive(Default, Debug, PartialEq, Eq)]
struct Census {
    candidates: usize,
    read: usize,
    unreadable: usize,
    unspellable: usize,
    internal: usize,
    unspecified: usize,
    variadic: usize,
    ambiguous: usize,
    macro_defined: usize,
    conditional_only: usize,
    absent: usize,
}

fn census(text: &str) -> Census {
    let candidates = census_candidates(text);
    let borrowed = candidates
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let readings = read_header(text, &borrowed).expect("the census headers are in the subset");
    let mut row = Census {
        candidates: candidates.len(),
        ..Census::default()
    };
    for symbol in &borrowed {
        match &readings[*symbol] {
            HeaderReading::Absent => row.absent += 1,
            HeaderReading::Macro => row.macro_defined += 1,
            HeaderReading::ConditionalOnly => row.conditional_only += 1,
            HeaderReading::Ambiguous => row.ambiguous += 1,
            HeaderReading::Declared(declaration) => {
                let positions = std::iter::once(&declaration.result)
                    .chain(declaration.parameters.iter())
                    .collect::<Vec<_>>();
                if declaration.internal {
                    row.internal += 1;
                } else if declaration.variadic {
                    row.variadic += 1;
                } else if declaration.unspecified {
                    row.unspecified += 1;
                } else if positions.iter().any(|position| position.is_err()) {
                    row.unreadable += 1;
                } else if positions.iter().any(|position| {
                    matches!(
                        position,
                        Ok(qcore_compiler::native_library_header::HeaderProjection::Unspellable(_))
                    )
                }) {
                    row.unspellable += 1;
                } else {
                    row.read += 1;
                }
            }
        }
    }
    row
}

fn libevent(header: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../third_party/libevent/include")
        .join(header);
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

#[test]
fn the_subset_census_over_libevents_core_header_is_pinned() {
    // `event2/event.h` at the pinned submodule revision. The number that matters
    // is `read`: **nil**, for one named reason — every exported declaration is
    // decorated with `EVENT2_EXPORT_SYMBOL`, whose `#define` is behind an
    // `#include` this reader does not follow. That is the census's headline and
    // it is pinned here rather than only asserted in a report.
    let row = census(&libevent("event2/event.h"));
    assert_eq!(row.read, 0, "no declaration of this header is inside v1");
    assert!(
        row.unreadable >= 50,
        "and the reason is a reader boundary, not absence: {row:?}"
    );
    assert_eq!(row.internal, 0);
    assert_eq!(row.unspellable, 0);
}

#[test]
fn the_dominant_census_refusal_names_one_token() {
    // Not "many declarations are hard": *one* token makes them so, and naming it
    // is what turns the census into a follow-on
    // (`NATIVE-LIBRARY-HEADER-DECORATION`) rather than a shrug.
    let text = libevent("event2/event.h");
    let candidates = census_candidates(&text);
    let borrowed = candidates
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let readings = read_header(&text, &borrowed).expect("the header is in the subset");
    let mut decorated = 0usize;
    let mut other = 0usize;
    for reading in readings.values() {
        let HeaderReading::Declared(declaration) = reading else {
            continue;
        };
        let mut spellings = Vec::new();
        if let Err(spelling) = &declaration.result {
            spellings.push(spelling.clone());
        }
        for parameter in &declaration.parameters {
            if let Err(spelling) = parameter {
                spellings.push(spelling.clone());
            }
        }
        if spellings.is_empty() {
            continue;
        }
        if spellings
            .iter()
            .any(|spelling| spelling.contains("EVENT2_EXPORT_SYMBOL"))
        {
            decorated += 1;
        } else {
            other += 1;
        }
    }
    assert!(
        decorated > other * 4,
        "the export decoration dominates every other reason: {decorated} against {other}"
    );
}

#[test]
fn every_libevent_core_header_stays_inside_the_directive_subset() {
    // A reader that refused the *file* would say nothing about the
    // declarations. These four are read as files and refuse per declaration,
    // which is the boundary the design puts the line at.
    for header in ["event2/event.h", "event2/buffer.h", "event2/thread.h"] {
        let text = libevent(header);
        read_header(&text, &BTreeSet::new())
            .unwrap_or_else(|refusal| panic!("{header}: {refusal}"));
    }
}

#[test]
fn a_header_using_error_directives_refuses_as_a_file() {
    // `event2/util.h` carries `#error`, which v1 does not interpret. The whole
    // header refuses, and it refuses *as a file* rather than silently reading
    // the part before the directive.
    let text = libevent("event2/util.h");
    let refusal = read_header(&text, &BTreeSet::new())
        .expect_err("`#error` is outside the v1 directive subset");
    assert!(
        format!("{refusal}").contains("error"),
        "the refusal names the directive: {refusal}"
    );
}

// --- 6. Two readers of one vocabulary: agreement, and the named divergences ---
//
// The crate layering forbids sharing the code. `qcore-core` has **no**
// dependencies at all — that is what makes a manifest rederivable in the kernel
// — and the gateway's declared-ABI reader sits above the contract loader, so
// this card's reader is necessarily a second implementation of one projection.
// The wave-20 lesson about second readers is that the divergences must be
// *measured and named*, not assumed away; these rows are that measurement,
// pinned so a later change to either reader that moves one of them is a finding
// rather than a surprise. Retiring either reader for the other needs invariant 3
// parity first (`NATIVE-LIBRARY-HEADER-ONE-READER`).

/// Read one C text through **both** readers over one symbol and report what each
/// decided: `(gateway outcome, header-level outcome)`.
fn both_readers(c_text: &str, declared: &str, manifest_entry: &str) -> (String, String) {
    use qcore_compiler::legacy_c::{AbiOutcome, declared_abi_census};

    let contracts =
        qcore_compiler::contract_loader::load_resource_contract_sources(&BTreeMap::from([(
            "cross.qcore".to_owned(),
            format!("module contract;\n\n{declared}"),
        )]))
        .expect("the contract module loads");
    let gateway = declared_abi_census(c_text, &contracts)
        .into_iter()
        .map(|row| match row.outcome {
            AbiOutcome::Agreed => "agreed".to_owned(),
            AbiOutcome::Undeclared => "undeclared".to_owned(),
            AbiOutcome::Withdrawn(refusal) => refusal.as_str().to_owned(),
        })
        .collect::<Vec<_>>()
        .join(",");
    let manifest = format!(
        "native_library manifest v1;\nlibrary demo;\nversion 0.1.0;\nartifact libdemo.a;\n\
         hash sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef;\n\
         data_model lp64;\nunwinding abort;\nallocator_custody closed;\n\
         thread_posture single_caller;\nheader h;\nheader_hash {};\n{manifest_entry}\n",
        digest_of(c_text)
    );
    let libraries = qcore_compiler::native_library::NativeLibraries::load(&BTreeMap::from([(
        "demo.qrlib".to_owned(),
        manifest,
    )]))
    .expect("the manifest loads");
    let text = header_text(c_text);
    let digest = digest_of(c_text);
    let sources = BTreeMap::from([(
        "h".to_owned(),
        qcore_compiler::native_library_header::HeaderSource {
            reference: "h",
            text: &text,
            measured_digest: &digest,
        },
    )]);
    let header =
        match qcore_compiler::native_library_header::check_header_bindings(&libraries, &sources) {
            Ok(()) => "agreed".to_owned(),
            Err(failure) => format!("{failure:?}")
                .split_whitespace()
                .next()
                .unwrap_or("?")
                .trim_start_matches("Unreadable(")
                .trim_end_matches('{')
                .trim()
                .to_owned(),
        };
    (gateway, header)
}

#[test]
fn the_two_readers_agree_on_a_declaration_both_can_read() {
    // Post-preprocessing spellings, which is what the gateway's reader is
    // handed by Clang and what this reader also admits. Both agree, and both
    // refuse the same edit.
    let c_text = "int demo_sum(const unsigned char *buffer, unsigned int count);";
    assert_eq!(
        both_readers(
            c_text,
            "extern fn demo_sum(buffer: ptr<const, u8>, count: u32) -> i32\ncontract {\n  requires resource_contract_v1;\n  ensures normal resource_borrowed(buffer);\n  ensures normal resource_borrowed(count);\n  ensures normal resource_return_none();\n};",
            "contract fn demo_sum(const uint8_t *, uint32_t) -> int32_t from cross.qcore;",
        ),
        ("agreed".to_owned(), "agreed".to_owned())
    );
    assert_eq!(
        both_readers(
            c_text,
            "extern fn demo_sum(buffer: ptr<mut, u8>, count: u32) -> i32\ncontract {\n  requires resource_contract_v1;\n  ensures normal resource_borrowed(buffer);\n  ensures normal resource_borrowed(count);\n  ensures normal resource_return_none();\n};",
            "contract fn demo_sum(uint8_t *, uint32_t) -> int32_t from cross.qcore;",
        )
        .0,
        "abi-parameter-type-disagreement",
        "the gateway refuses the dropped `const`"
    );
    assert_eq!(
        both_readers(
            c_text,
            "extern fn demo_sum(buffer: ptr<const, u8>, count: u32) -> i32\ncontract {\n  requires resource_contract_v1;\n  ensures normal resource_borrowed(buffer);\n  ensures normal resource_borrowed(count);\n  ensures normal resource_return_none();\n};",
            "contract fn demo_sum(uint8_t *, uint32_t) -> int32_t from cross.qcore;",
        )
        .1,
        "Disagreement",
        "and so does this one"
    );
}

#[test]
fn the_reader_divergences_are_named_and_pinned() {
    // Four, each deliberate and each argued in the card report §7.
    //
    // 1. The `HEADER-STDINT-PRELUDE`. The gateway is handed a *preprocessed*
    //    unit in which `<stdint.h>` has already been expanded, so a surviving
    //    `uint32_t` is an unknown name to it. A header is unpreprocessed and
    //    writes exactly these names, so this reader commits to the ones C11
    //    itself fixes.
    let stdint = "int demo_take(uint32_t value);";
    let declared = "extern fn demo_take(value: u32) -> i32\ncontract {\n  requires resource_contract_v1;\n  ensures normal resource_borrowed(value);\n  ensures normal resource_return_none();\n};";
    assert_eq!(
        both_readers(
            stdint,
            declared,
            "unchecked fn demo_take(uint32_t) -> int32_t;"
        ),
        ("abi-unreadable-declaration".to_owned(), "agreed".to_owned())
    );
    // 2. `static`. The gateway reads a translation unit, where a `static`
    //    declaration is exactly what it must compare against; a manifest binds
    //    an *artifact's exported symbols*, and a symbol with internal linkage
    //    is not one.
    let internal = "static int demo_take(unsigned int value);";
    assert_eq!(
        both_readers(
            internal,
            declared,
            "unchecked fn demo_take(uint32_t) -> int32_t;"
        ),
        ("agreed".to_owned(), "InternalDeclaration".to_owned())
    );
    // 3. The unspecified parameter list. A declared contract naming no
    //    parameters asserts nothing about parameters, so the gateway admits it;
    //    a manifest entry `fn f() -> T` asserts the C declaration is `f(void)`,
    //    so there is something to contradict and nothing to check it against.
    let unspecified = "int demo_now();";
    let nullary = "extern fn demo_now() -> i32\ncontract {\n  requires resource_contract_v1;\n  ensures normal resource_return_none();\n};";
    assert_eq!(
        both_readers(unspecified, nullary, "unchecked fn demo_now() -> int32_t;"),
        ("agreed".to_owned(), "UnspecifiedParameters".to_owned())
    );
    // 4. The vocabulary. §C04.1 has `fnptr`, `f32` and `f64`; the manifest's
    //    ABI table had none of them, so this reader reported a read `double`
    //    as `UnspellableResult`. Card `W48-QRLIB-LINKAGE` gave the manifest
    //    `float` and `double` (`W48-QR-FLOAT-QRLIB-SPELLING`), so the row is
    //    corrected: a read `double` is now compared, and an `int64_t`
    //    transcription of it is a disagreement like any other. `fnptr` stays
    //    the vocabulary gap.
    let floating = "double demo_now(void);";
    let floating_contract = "extern fn demo_now() -> f64\ncontract {\n  requires resource_contract_v1;\n  ensures normal resource_return_none();\n};";
    assert_eq!(
        both_readers(
            floating,
            floating_contract,
            "unchecked fn demo_now() -> int64_t;"
        ),
        ("agreed".to_owned(), "Disagreement".to_owned())
    );
}

/// The digest of one text, in the manifest's spelling.
fn digest_of(text: &str) -> String {
    format!(
        "sha256:{}",
        qcore_compiler::digest::sha256_hex(header_text(text).as_bytes())
    )
}
