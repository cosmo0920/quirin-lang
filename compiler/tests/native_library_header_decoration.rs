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

//! The **export decoration** of an exported header (card
//! `NATIVE-LIBRARY-HEADER-DECORATION`), extending card `NATIVE-LIBRARY-HEADER`.
//!
//! That card's census measured **0 of 423** Libevent declarations inside its
//! subset, for one reason: every exported declaration is written
//! `EVENT2_EXPORT_SYMBOL struct event_base *event_base_new(void);`, and that
//! macro's `#define` lives in a header the reader did not follow. It named the
//! obvious repair and declined it:
//!
//! > a manifest-declared erasure list converts a *reader boundary* into an
//! > **unchecked premise about the header's own text**, which is precisely what
//! > this level exists to remove.
//!
//! This suite pins the derived route instead, and the whole content of the card
//! is the split it rests on: a manifest declares **which hash-identified texts
//! may be read**, and the reader **derives from those texts' own `#define`s**
//! which names are erasable decorations. Nothing a manifest says about a
//! header's contents is believed, because a manifest says nothing about them.
//!
//! 1. the derived route reads a decorated header, and the certificate names
//!    every text the reading rested on;
//! 2. erasing the name **is** expanding it — the two spellings read alike;
//! 3. every step of the derivation refuses when it does not hold: a text not
//!    supplied, a digest that does not match, a text nothing reaches, a text
//!    nobody declared, a body outside the vocabulary, a conditional arm that
//!    leaves the name undefined, a function-like macro, an `#undef`;
//! 4. the wave-22 world does not move.
//!
//! §5 is the **census delta** over the pinned Libevent submodule.
use std::collections::{BTreeMap, BTreeSet};

use qcore_compiler::native_library_header::{
    HeaderReading, census_candidates, read_header, read_header_with_includes,
};
use qcore_compiler::{
    CompileOptions, SourceExecutionProfile, SourceFile, compile_files, extern_call,
};

const ROUND_TRIP: &str = include_str!("fixtures/extern_calls/round_trip.qr");
const SLICE_MODULE: &str = "fixtures/native_library/contracts/buffer_lib.qv";
const SLICE: &str = include_str!("fixtures/native_library/contracts/buffer_lib.qv");
const MANIFEST_PATH: &str = "fixtures/native_library/buffer_lib_decorated.qrlib";
const MANIFEST: &str = include_str!("fixtures/native_library/buffer_lib_decorated.qrlib");
const HEADER_REFERENCE: &str = "include/qcore_buffer_demo_decorated.h";
const HEADER: &str = include_str!("fixtures/native_library/include/qcore_buffer_demo_decorated.h");
const VISIBILITY_REFERENCE: &str = "include/qcore_buffer_demo_visibility.h";
const VISIBILITY_SPELLING: &str = "\"qcore_buffer_demo_visibility.h\"";
const VISIBILITY: &str =
    include_str!("fixtures/native_library/include/qcore_buffer_demo_visibility.h");
const PLAIN: &str = include_str!("fixtures/native_library/include/qcore_buffer_demo.h");

/// `HEADER-FIXTURE-EOL-NORMALISED`, card `NATIVE-LIBRARY-HEADER`: the manifest
/// pins each fixture text's SHA-256, and a checkout that translated line endings
/// would hand the test different bytes for the same file. Nothing in the shipped
/// reader normalises anything.
fn text(source: &str) -> String {
    source.replace("\r\n", "\n")
}

fn selection(entries: &[(&str, &str)]) -> Option<BTreeMap<String, String>> {
    Some(
        entries
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect(),
    )
}

fn supplied(entries: &[(&str, &str)]) -> Option<BTreeMap<String, String>> {
    Some(
        entries
            .iter()
            .map(|(key, value)| ((*key).to_owned(), text(value)))
            .collect(),
    )
}

/// The two texts this manifest declares, which is the accepted configuration.
fn both_texts() -> Option<BTreeMap<String, String>> {
    supplied(&[
        (HEADER_REFERENCE, HEADER),
        (VISIBILITY_REFERENCE, VISIBILITY),
    ])
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

fn manifest_with(original: &str, replacement: &str) -> String {
    let manifest = text(MANIFEST);
    let edited = manifest.replace(original, replacement);
    assert_ne!(edited, manifest, "the manifest edit must have applied");
    edited
}

const SYMBOLS: [&str; 4] = [
    "qcore_extern_alloc",
    "qcore_extern_fill",
    "qcore_extern_release",
    "qcore_extern_sum",
];

fn named(symbols: &[&str]) -> BTreeSet<&'static str> {
    SYMBOLS
        .iter()
        .copied()
        .filter(|symbol| symbols.contains(symbol))
        .collect()
}

// --- 1. The payoff: a decorated header is read, and every text is named ---

#[test]
fn a_decorated_header_is_read_through_its_declared_visibility_text() {
    // Card `NATIVE-LIBRARY-HEADER` refuses this exact header by name, at
    // `Q063`, because `QCORE_BUFFER_DEMO_EXPORT` is a token it does not commit
    // to. One `include` statement — which says only *which text*, never what
    // the text means — is the whole delta.
    let module = compile((MANIFEST_PATH, MANIFEST), both_texts())
        .expect("the decorated header declares exactly what the manifest transcribes");
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::ExternCallsV1
    );
    let headers = module
        .declared_assumptions()
        .iter()
        .filter_map(|assumption| match assumption {
            extern_call::ExternAssumption::ManifestResidentContract { residence, .. } => {
                residence.header.clone()
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(!headers.is_empty(), "the premise carries a header identity");
    for header in &headers {
        assert_eq!(header.reference, HEADER_REFERENCE);
        // **The certificate names every text the reading rested on.** The
        // decoration was derived from the visibility text's own `#define`s, so
        // a premise naming only the header would publish a decision a recheck
        // could not reproduce.
        assert_eq!(
            header
                .includes
                .iter()
                .map(|include| (include.spelling.as_str(), include.reference.as_str()))
                .collect::<Vec<_>>(),
            [(VISIBILITY_SPELLING, VISIBILITY_REFERENCE)],
            "the spliced text is a published identity, not an implementation detail"
        );
    }
}

#[test]
fn the_derivation_names_exactly_the_decoration_and_reads_it_from_the_declared_text() {
    // What is derived, stated directly rather than only through its effect: one
    // name, and it is a name the *manifest never mentions*.
    let included = BTreeMap::from([(VISIBILITY_SPELLING.to_owned(), VISIBILITY)]);
    let readout = read_header_with_includes(HEADER, &included, &named(&SYMBOLS))
        .expect("the decorated header is inside the subset once its text is declared");
    assert_eq!(
        readout.erasures.iter().cloned().collect::<Vec<_>>(),
        ["QCORE_BUFFER_DEMO_EXPORT"],
        "exactly one name is derived erasable, and it is derived rather than declared"
    );
    // No *statement* of the manifest names the erased token. The commentary
    // above them explains the card; the grammar has no place to say it.
    assert!(
        !MANIFEST
            .lines()
            .filter(|line| !line.trim_start().starts_with('#'))
            .any(|line| line.contains("QCORE_BUFFER_DEMO_EXPORT")),
        "the manifest selects texts, not meanings"
    );
    assert!(
        readout.followed.contains(VISIBILITY_SPELLING),
        "the declared text was actually reached"
    );
    for symbol in SYMBOLS {
        assert!(
            matches!(readout.readings[symbol], HeaderReading::Declared(_)),
            "{symbol} is read"
        );
    }
}

#[test]
fn the_decorated_and_the_undecorated_headers_read_alike() {
    // The two fixture headers declare the same four symbols; one writes the
    // decoration and one does not. Equal readings is the statement that the
    // erasure took nothing away.
    let included = BTreeMap::from([(VISIBILITY_SPELLING.to_owned(), VISIBILITY)]);
    let decorated = read_header_with_includes(HEADER, &included, &named(&SYMBOLS))
        .expect("the decorated header reads")
        .readings;
    let plain = read_header(PLAIN, &named(&SYMBOLS)).expect("the plain header reads");
    assert_eq!(
        decorated, plain,
        "erasing the decoration leaves exactly the undecorated declaration"
    );
}

// --- 2. Erasure *is* expansion ---

#[test]
fn erasing_the_name_is_expanding_it() {
    // The derivation's whole justification: under the four conditions, deleting
    // the name equals expanding it and deleting the decoration the expansion
    // produced. So the four expansions the fixture's own conditional arms
    // contain, written out in the declaration's place, must read identically to
    // the macro form — and they do, because one vocabulary erases both.
    let included = BTreeMap::from([(VISIBILITY_SPELLING.to_owned(), VISIBILITY)]);
    let through_the_macro = read_header_with_includes(HEADER, &included, &named(&SYMBOLS))
        .expect("the macro form reads")
        .readings;
    for expansion in [
        "",
        "__global",
        "__attribute__((visibility(\"default\")))",
        "__declspec(dllexport)",
    ] {
        let expanded = HEADER.replace("QCORE_BUFFER_DEMO_EXPORT", expansion);
        let readings = read_header(&expanded, &named(&SYMBOLS))
            .unwrap_or_else(|refusal| panic!("`{expansion}`: {refusal}"));
        assert_eq!(
            readings, through_the_macro,
            "the expansion `{expansion}` must read as the macro does"
        );
    }
}

#[test]
fn a_decoration_that_changes_the_type_is_not_erased() {
    // The vocabulary is not "attributes are noise". `mode` changes the declared
    // type, so it is **absent** from the whitelist and the declaration keeps the
    // refusal card `NATIVE-LIBRARY-HEADER` already gave it. The contrast with
    // the row above is the point: same position, same shape, different name.
    let decorated = PLAIN.replace(
        "uint8_t *qcore_extern_alloc",
        "__attribute__((mode(__DI__))) uint8_t *qcore_extern_alloc",
    );
    let readings = read_header(&decorated, &named(&["qcore_extern_alloc"]))
        .expect("the file is in the subset");
    let HeaderReading::Declared(declaration) = &readings["qcore_extern_alloc"] else {
        panic!("the declaration is found");
    };
    let Err(spelling) = &declaration.result else {
        panic!("a type-changing attribute must not be erased: {declaration:?}");
    };
    assert!(
        spelling.contains("mode"),
        "and the refusal names the token that caused it: {spelling}"
    );
    // And it reaches the *route* as `Q063`, not as `Q064`. The distinction is
    // the whole difference between "this reader could not read the declaration"
    // and "the header contradicts the manifest", and the second is a claim about
    // the library that this case does not license. The first implementation of
    // this refusal reported an empty parameter list, which `compare_reading`
    // compares for arity before it reaches the result — so it produced exactly
    // that false disagreement.
    let wave22 = include_str!("fixtures/native_library/buffer_lib_header.qrlib");
    let rehashed = wave22.replace(
        "sha256:aa92b5d783202524bad45bcc39750944ab19af0547f555429623fc8a2a49a657",
        &format!(
            "sha256:{}",
            qcore_compiler::digest::sha256_hex(text(&decorated).as_bytes())
        ),
    );
    let refusals = codes(
        ("fixtures/native_library/buffer_lib_header.qrlib", &rehashed),
        supplied(&[("include/qcore_buffer_demo.h", &decorated)]),
    );
    assert_eq!(
        refusals
            .iter()
            .map(|(code, _)| code.clone())
            .collect::<Vec<_>>(),
        ["Q063"],
        "an unreadable declaration is not a disagreement: {refusals:?}"
    );
    assert!(
        refusals[0].1.contains("mode"),
        "and it names the token rather than a digest or an arity: {}",
        refusals[0].1
    );
}

// --- 3. Every step of the derivation refuses when it does not hold ---

#[test]
fn a_declared_include_text_that_is_not_supplied_refuses() {
    let refusals = codes(
        (MANIFEST_PATH, MANIFEST),
        supplied(&[(HEADER_REFERENCE, HEADER)]),
    );
    assert_eq!(
        refusals
            .iter()
            .map(|(code, _)| code.clone())
            .collect::<Vec<_>>(),
        ["Q063"]
    );
    assert!(
        refusals[0].1.contains(VISIBILITY_SPELLING)
            && refusals[0].1.contains("no text was supplied"),
        "the refusal names the text: {}",
        refusals[0].1
    );
}

#[test]
fn a_declared_include_text_whose_digest_is_not_the_declared_one_refuses() {
    // The digest decides **before** the text is read, exactly as the header's
    // own does: this substitution edits only a comment, so the derivation would
    // have reached the same erasure. It is still refused, because a decision
    // that rests on a text must name that text by content.
    let substituted = VISIBILITY.replace("The export-decoration header", "An edited comment");
    assert_ne!(
        substituted, VISIBILITY,
        "the substitution must have applied"
    );
    let refusals = codes(
        (MANIFEST_PATH, MANIFEST),
        supplied(&[
            (HEADER_REFERENCE, HEADER),
            (VISIBILITY_REFERENCE, &substituted),
        ]),
    );
    assert_eq!(
        refusals
            .iter()
            .map(|(code, _)| code.clone())
            .collect::<Vec<_>>(),
        ["Q063"]
    );
    assert!(
        refusals[0].1.contains("measures"),
        "the refusal reports both digests: {}",
        refusals[0].1
    );
}

#[test]
fn a_declared_include_text_the_header_never_reaches_refuses() {
    // A declared text nothing reads is a statement with no effect, and this
    // level's content is that its statements have effects. The spelling below is
    // well-formed and its digest matches; no `#include` names it.
    let edited = manifest_with(VISIBILITY_SPELLING, "\"qcore_buffer_demo_unreachable.h\"");
    let refusals = codes((MANIFEST_PATH, &edited), both_texts());
    assert_eq!(
        refusals
            .iter()
            .map(|(code, _)| code.clone())
            .collect::<Vec<_>>(),
        ["Q063"]
    );
    assert!(
        refusals[0].1.contains("which no `#include`"),
        "the refusal says the text is unreached rather than unreadable: {}",
        refusals[0].1
    );
}

#[test]
fn a_text_supplied_for_a_reference_nobody_declares_refuses() {
    // Transport totality. The texts this level reads are exactly the declared
    // ones once a header is bound, so an extra text is refused rather than
    // dropped on the floor — a reader who believed the transport carried only
    // declared texts would otherwise be wrong without being told.
    let refusals = codes(
        (MANIFEST_PATH, MANIFEST),
        supplied(&[
            (HEADER_REFERENCE, HEADER),
            (VISIBILITY_REFERENCE, VISIBILITY),
            ("include/nobody_declared_this.h", PLAIN),
        ]),
    );
    assert_eq!(
        refusals
            .iter()
            .map(|(code, _)| code.clone())
            .collect::<Vec<_>>(),
        ["Q063"]
    );
    assert!(
        refusals[0].1.contains("nobody_declared_this.h"),
        "the refusal names the undeclared text: {}",
        refusals[0].1
    );
}

#[test]
fn an_include_the_manifest_does_not_declare_is_not_followed() {
    // The set is closed in the other direction too, and *quietly*: the plain
    // fixture header's `#include <stdint.h>` is not declared, is not followed,
    // and nothing about that changed. A closed set that refused on every
    // undeclared `#include` would make the level unusable on any real header.
    let readings = read_header(PLAIN, &named(&SYMBOLS)).expect("the plain header reads");
    assert!(PLAIN.contains("#include <stdint.h>"));
    for symbol in SYMBOLS {
        assert!(matches!(readings[symbol], HeaderReading::Declared(_)));
    }
}

/// A visibility text with one line rewritten, so each derivation condition can
/// be broken in isolation while everything else stays as the accepted fixture.
fn visibility_with(original: &str, replacement: &str) -> String {
    let visibility = text(VISIBILITY);
    let edited = visibility.replace(original, replacement);
    assert_ne!(edited, visibility, "the visibility edit must have applied");
    edited
}

/// The manifest with its declared visibility digest re-measured over `edited`,
/// so a row that means to break one derivation condition is not refused by the
/// digest gate first. The digest gate has its own row above.
fn manifest_for(edited: &str) -> String {
    let digest = format!(
        "sha256:{}",
        qcore_compiler::digest::sha256_hex(text(edited).as_bytes())
    );
    manifest_with(
        "sha256:8fce29e5cac9277ea8ebbb8ed70ad4b7a58c8ba7d469a76af03e9da9bf611c0b",
        &digest,
    )
}

/// Read the decorated header against an edited visibility text, and report the
/// derived erasures.
fn erasures_with(visibility: &str) -> BTreeSet<String> {
    let included = BTreeMap::from([(VISIBILITY_SPELLING.to_owned(), visibility)]);
    read_header_with_includes(HEADER, &included, &named(&SYMBOLS))
        .expect("the texts stay inside the directive subset")
        .erasures
}

#[test]
fn a_decoration_body_outside_the_vocabulary_is_not_derived() {
    // One arm of four is enough. The reader evaluates no condition, so it cannot
    // know which arm a consumer takes — which is exactly why *every* body must
    // be erasable for the name to be.
    let edited = visibility_with(
        "#  define QCORE_BUFFER_DEMO_EXPORT __declspec(dllexport)",
        "#  define QCORE_BUFFER_DEMO_EXPORT __attribute__((aligned(16)))",
    );
    assert!(
        erasures_with(&edited).is_empty(),
        "one unerasable arm withdraws the whole derivation"
    );
}

#[test]
fn a_conditional_arm_that_leaves_the_name_undefined_is_not_derived() {
    // The all-paths condition, broken by deleting one `#else`. The remaining
    // arms still define the name and their bodies are still erasable; what is
    // gone is the guarantee that *some* arm defines it, and the reader will not
    // supply that guarantee from a configuration it cannot see.
    let edited = visibility_with(
        "#else /* QCORE_BUFFER_DEMO_SHARED */\n\n# define QCORE_BUFFER_DEMO_EXPORT\n\n",
        "",
    );
    assert!(
        erasures_with(&edited).is_empty(),
        "a group with no `#else` has an implicit arm that defines nothing"
    );
}

#[test]
fn a_function_like_decoration_macro_is_not_derived() {
    // A function-like macro's expansion depends on its arguments at each use
    // site, and this reader evaluates no expansion.
    let edited = visibility_with(
        "# define QCORE_BUFFER_DEMO_EXPORT\n",
        "# define QCORE_BUFFER_DEMO_EXPORT(x) x\n",
    );
    assert!(erasures_with(&edited).is_empty());
}

#[test]
fn an_undefined_decoration_name_is_not_derived() {
    // `#undef` anywhere withdraws the derivation: this reader tracks no scope
    // for a macro, and the conservative reading is the refusing one.
    let edited = visibility_with(
        "#endif /* QCORE_BUFFER_DEMO_VISIBILITY_H */",
        "#undef QCORE_BUFFER_DEMO_EXPORT\n#endif /* QCORE_BUFFER_DEMO_VISIBILITY_H */",
    );
    assert!(erasures_with(&edited).is_empty());
}

#[test]
fn a_decoration_the_reader_cannot_derive_keeps_the_wave_22_refusal() {
    // Every row above asserts what is *not* derived; this one asserts what that
    // costs. The manifest is refused at `Q063`, naming the symbol and the token
    // — card `NATIVE-LIBRARY-HEADER`'s refusal, unweakened, reached through the
    // new route.
    let edited = visibility_with(
        "#  define QCORE_BUFFER_DEMO_EXPORT __declspec(dllexport)",
        "#  define QCORE_BUFFER_DEMO_EXPORT __attribute__((aligned(16)))",
    );
    let refusals = codes(
        (MANIFEST_PATH, &manifest_for(&edited)),
        supplied(&[(HEADER_REFERENCE, HEADER), (VISIBILITY_REFERENCE, &edited)]),
    );
    assert_eq!(
        refusals
            .iter()
            .map(|(code, _)| code.clone())
            .collect::<Vec<_>>(),
        ["Q063"]
    );
    assert!(
        refusals[0].1.contains("QCORE_BUFFER_DEMO_EXPORT"),
        "the refusal names the token, as it did before this card: {}",
        refusals[0].1
    );
}

#[test]
fn an_include_text_carrying_an_uninterpreted_directive_refuses_the_whole_reading() {
    // A spliced text is part of the one text the reader is reading, so its
    // directives are read under the same subset. This is the shape that keeps
    // Libevent's `event2/util.h` out (§5): a header that declares it refuses as
    // a file rather than being read in part.
    let edited = visibility_with(
        "# define QCORE_BUFFER_DEMO_EXPORT\n",
        "# error this configuration is unsupported\n",
    );
    let refusals = codes(
        (MANIFEST_PATH, &manifest_for(&edited)),
        supplied(&[(HEADER_REFERENCE, HEADER), (VISIBILITY_REFERENCE, &edited)]),
    );
    assert_eq!(
        refusals
            .iter()
            .map(|(code, _)| code.clone())
            .collect::<Vec<_>>(),
        ["Q063"]
    );
    assert!(
        refusals[0].1.contains("error"),
        "the refusal names the directive: {}",
        refusals[0].1
    );
}

// --- 4. What did not move ---

#[test]
fn a_manifest_declaring_no_include_is_the_wave_22_binding() {
    // The wave-22 fixture manifest and its header, compiled with the wave-22
    // transport. Declaring no include text is the default and reads exactly as
    // it did.
    let headerless = include_str!("fixtures/native_library/buffer_lib_header.qrlib");
    let module = compile(
        (
            "fixtures/native_library/buffer_lib_header.qrlib",
            headerless,
        ),
        supplied(&[("include/qcore_buffer_demo.h", PLAIN)]),
    )
    .expect("the wave-22 binding is unchanged");
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::ExternCallsV1
    );
    let empty = module
        .declared_assumptions()
        .iter()
        .filter_map(|assumption| match assumption {
            extern_call::ExternAssumption::ManifestResidentContract { residence, .. } => residence
                .header
                .as_ref()
                .map(|header| header.includes.len()),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(empty, BTreeSet::from([0]));
}

#[test]
fn an_include_statement_without_a_header_is_refused_at_load() {
    // The declared texts exist only to be read *into* the exported header, so
    // one without a header names a text nothing opens.
    let edited = text(MANIFEST)
        .replace("header include/qcore_buffer_demo_decorated.h;\n", "")
        .replace(
            "header_hash sha256:ddbdc03f8e7ffda837b01f97575ad1b35dedc1134f70b9410fa4e635dc48e2bc;\n",
            "",
        );
    assert!(
        !edited.contains("header include/") && !edited.contains("header_hash sha256:"),
        "both halves of the header binding must have been removed"
    );
    let refusals = codes((MANIFEST_PATH, &edited), both_texts());
    assert_eq!(
        refusals
            .iter()
            .map(|(code, _)| code.clone())
            .collect::<Vec<_>>(),
        ["Q039"],
        "a half-stated binding is a manifest that does not load, not a header refusal"
    );
}

#[test]
fn a_malformed_include_statement_is_refused_at_load() {
    for (edit, expected) in [
        (
            manifest_with(" from include/qcore_buffer_demo_visibility.h", ""),
            "is not `include SPELLING from REFERENCE HASH`",
        ),
        (
            manifest_with(VISIBILITY_SPELLING, "qcore_buffer_demo_visibility.h"),
            "is not `<…>` or `\"…\"`",
        ),
    ] {
        let refusals = codes((MANIFEST_PATH, &edit), both_texts());
        assert_eq!(
            refusals
                .iter()
                .map(|(code, _)| code.clone())
                .collect::<Vec<_>>(),
            ["Q039"]
        );
        assert!(
            refusals[0].1.contains(expected),
            "expected `{expected}` in: {}",
            refusals[0].1
        );
    }
}

// --- 5. The census delta over the pinned Libevent submodule ---

fn libevent(header: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../third_party/libevent/include")
        .join(header);
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// How many candidates of one Libevent header are fully read, with and without
/// `event2/visibility.h` declared.
fn libevent_read(header: &str, included: &BTreeMap<String, &str>) -> usize {
    let text = libevent(header);
    let candidates = census_candidates(&text);
    let borrowed = candidates
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let readout = read_header_with_includes(&text, included, &borrowed)
        .expect("the census headers are inside the subset");
    readout
        .readings
        .values()
        .filter(|reading| match reading {
            HeaderReading::Declared(declaration) => {
                !declaration.internal
                    && !declaration.variadic
                    && !declaration.unspecified
                    && std::iter::once(&declaration.result)
                        .chain(declaration.parameters.iter())
                        .all(|position| {
                            matches!(
                            position,
                            Ok(
                                qcore_compiler::native_library_header::HeaderProjection::Spellings(
                                    _
                                )
                            )
                        )
                        })
            }
            _ => false,
        })
        .count()
}

#[test]
fn the_libevent_census_moves_from_nil_to_a_majority_on_one_declared_text() {
    // Card `NATIVE-LIBRARY-HEADER` measured `read == 0` over `event2/event.h`
    // and named `EVENT2_EXPORT_SYMBOL` as the single reason. Declaring the one
    // text that macro is defined in — `event2/visibility.h`, whose own four
    // conditional arms the reader evaluates none of — moves the row to a
    // majority of the header's candidates. Nothing else about the reader
    // changed, and the manifest still says nothing about what the macro means.
    let visibility = libevent("event2/visibility.h");
    let included = BTreeMap::from([("<event2/visibility.h>".to_owned(), visibility.as_str())]);
    assert_eq!(
        libevent_read("event2/event.h", &BTreeMap::new()),
        0,
        "the wave-22 row, unchanged"
    );
    assert!(
        libevent_read("event2/event.h", &included) >= 50,
        "and the derived route reads a majority of the same 70 candidates"
    );
    // The other headers of the pinned surface move the same way, for the same
    // one reason.
    for header in ["event2/http.h", "event2/bufferevent.h", "event2/dns.h"] {
        assert_eq!(libevent_read(header, &BTreeMap::new()), 0);
        assert!(
            libevent_read(header, &included) > 0,
            "{header} must move too"
        );
    }
}

#[test]
fn the_libevent_derivation_names_exactly_the_one_export_macro() {
    // The derivation over Libevent's real `visibility.h` — four `#define`s of
    // one name across a nested conditional structure the reader decides nothing
    // about — yields that one name and no other. `EVENT2_CORE_EXPORT_SYMBOL`,
    // defined in the same text, is **not** derived: its group carries no
    // `#else`, so a configuration exists in which it is undefined.
    let visibility = libevent("event2/visibility.h");
    let included = BTreeMap::from([("<event2/visibility.h>".to_owned(), visibility.as_str())]);
    let text = libevent("event2/event.h");
    let readout = read_header_with_includes(&text, &included, &BTreeSet::new())
        .expect("the header is inside the subset");
    assert_eq!(
        readout.erasures.iter().cloned().collect::<Vec<_>>(),
        ["EVENT2_EXPORT_SYMBOL"]
    );
    assert!(
        visibility.contains("EVENT2_CORE_EXPORT_SYMBOL"),
        "the text defines a second name, which the all-paths condition excludes"
    );
}

#[test]
fn declaring_a_text_outside_the_directive_subset_refuses_the_headers_that_read_it() {
    // Fail-closed, measured. `event2/util.h` is where `evutil_socket_t`,
    // `ev_ssize_t` and `ev_off_t` live — the dominant *remaining* reason of §5's
    // census — and it carries `#error`. Declaring it does not read those
    // typedefs; it refuses every header that includes it, as a file. That is the
    // boundary between this card and `NATIVE-LIBRARY-HEADER-CONDITIONAL`, and it
    // is a refusal rather than a partial reading.
    let util = libevent("event2/util.h");
    let included = BTreeMap::from([("<event2/util.h>".to_owned(), util.as_str())]);
    let text = libevent("event2/event.h");
    let refusal = read_header_with_includes(&text, &included, &BTreeSet::new())
        .expect_err("a spliced `#error` is outside the subset");
    assert!(
        format!("{refusal}").contains("error"),
        "the refusal names the directive: {refusal}"
    );
}
