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

//! Main-source attribution by include depth (unit **E01-TAIL**, residual
//! **E01-F1-STDIN**, `docs/reports/remaining-e01-tail/README.md`).
//!
//! E01-F1 recorded the residual: the enumeration read the *primary input* from
//! the first line marker and attributed a token to the main source by comparing
//! the current marker's file spelling against it. For the pinned Cycle 19
//! compatibility route that spelling is the fixed, guessable `<stdin>`
//! (`CSourceInput::HistoricalStdin`), so the header/main split rested on the
//! transport rather than on where Clang actually read the text.
//!
//! These tests are that residual's acceptance. They drive the **same** source
//! through both transports and pin both directions a file name can be moved:
//!
//! * a `#line` in a *header* that writes the primary input's spelling must not
//!   pull the header's definitions into the audited main-source scope
//!   (boundary E01-B2 must hold under a relabel);
//! * a `#line` in the *main source* that writes some other spelling must not
//!   push the rest of the main source out of the denominator — the fail-open
//!   direction, where a macro-produced definition after the directive has no
//!   raw-scan row to fall back on;
//! * the two transports must agree, definition for definition;
//! * a marker stream whose include stack does not balance refuses the
//!   completeness claim on a named environment ground rather than attributing
//!   anything.
//!
//! The transport itself is untouched: `CSourceInput` keeps its variants and
//! `HistoricalStdin` keeps its identity bytes (`remaining-g02`). What changes
//! is that attribution no longer consults the file *name* at all.

use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, CSourceInput, EnumerationRefusal, LegacyFunctionScope,
    enumerate_source_definitions, legacy_c_source_functions,
};
use qcore_schemas::reason::{ReasonCategory, ReasonSubject};

fn clang_available() -> bool {
    match std::env::var("QCORE_CLANG") {
        Ok(path) => !path.trim().is_empty(),
        Err(_) => std::process::Command::new("clang")
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success()),
    }
}

struct Fixture {
    directory: std::path::PathBuf,
}

impl Fixture {
    fn new(tag: &str) -> Self {
        let directory = std::env::temp_dir().join(format!(
            "qcore-e01tail-{tag}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).unwrap();
        Self { directory }
    }

    fn write(&self, name: &str, contents: &str) -> std::path::PathBuf {
        let path = self.directory.join(name);
        std::fs::write(&path, contents).unwrap();
        path
    }

    fn include(&self) -> String {
        self.directory.to_string_lossy().into_owned()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

/// The same unit under both transports: piped standard input (the pinned
/// Cycle 19 projection) and a declared file argument.
fn both_transports<'a>(
    fixture: &Fixture,
    source: &'a str,
) -> (C11TranslationUnit<'a>, C11TranslationUnit<'a>) {
    fixture.write("subject.c", source);
    let stdin = C11TranslationUnit {
        source,
        include_paths: vec![fixture.include()],
        function_scope: LegacyFunctionScope::MainSourceDefinitions,
        input: CSourceInput::HistoricalStdin,
        ..C11TranslationUnit::default()
    };
    let declared = C11TranslationUnit {
        source,
        include_paths: vec![fixture.include()],
        function_scope: LegacyFunctionScope::MainSourceDefinitions,
        input: CSourceInput::DeclaredFile {
            spelled: "subject.c".to_owned(),
            directory: fixture.directory.clone(),
        },
        ..C11TranslationUnit::default()
    };
    (stdin, declared)
}

// ---------------------------------------------------------------------------
// 1. A header cannot relabel itself into the audited scope
// ---------------------------------------------------------------------------

/// A header whose `#line` writes `"<stdin>"` — the exact spelling the pinned
/// compatibility transport gives its primary input, and one an author needs no
/// knowledge of the build to write.
///
/// Under name attribution the stdin transport counted `relabelled_header_fn`
/// as a main-source definition while the declared-file transport did not: the
/// audited scope differed by transport alone. Under depth attribution the
/// header sits at include depth 1 whatever it calls itself, so E01-B2 holds
/// in both.
#[test]
fn a_header_relabelled_to_the_primary_spelling_stays_out_of_scope() {
    if !clang_available() {
        return;
    }
    let fixture = Fixture::new("hdr-relabel");
    fixture.write(
        "relabel.h",
        "#line 100 \"<stdin>\"\nint relabelled_header_fn(void) { return 1; }\n",
    );
    let source = "#include \"relabel.h\"\nint subject(void) { return 0; }\n";
    let (stdin, declared) = both_transports(&fixture, source);

    let piped = enumerate_source_definitions(&stdin);
    assert_eq!(
        piped.definitions(),
        ["subject"],
        "a header's own definition must not enter the main-source denominator \
         by writing the primary input's spelling"
    );
    assert!(piped.is_complete(), "{:?}", piped.refusals());

    let file = enumerate_source_definitions(&declared);
    assert_eq!(file.definitions(), ["subject"]);
    assert_eq!(
        piped.definitions(),
        file.definitions(),
        "the audited scope must not depend on the transport"
    );
}

/// The same attack aimed at a *declared-file* unit. Clang's primary marker
/// there carries the compile command's own input word, so the header writes
/// exactly that word — `subject.c` — rather than an absolute path, which would
/// miss by string comparison and pass vacuously.
///
/// Measured against the pre-repair rule, this is the asymmetry E01-F1-STDIN
/// named: a declared-file unit can only be attacked by an author who knows how
/// the compilation database spells the input, while every `HistoricalStdin`
/// unit in the pinned corpus answers to the one fixed word `<stdin>`. Depth
/// attribution removes both, and nothing about it is special to `<stdin>`.
#[test]
fn a_header_relabelled_to_the_declared_path_stays_out_of_scope() {
    if !clang_available() {
        return;
    }
    let fixture = Fixture::new("hdr-path-relabel");
    fixture.write(
        "path_relabel.h",
        "#line 100 \"subject.c\"\nint relabelled_header_fn(void) { return 1; }\n",
    );
    let source = "#include \"path_relabel.h\"\nint subject(void) { return 0; }\n";
    let (_, declared) = both_transports(&fixture, source);
    let inventory = enumerate_source_definitions(&declared);
    assert_eq!(inventory.definitions(), ["subject"]);
    assert!(inventory.is_complete(), "{:?}", inventory.refusals());
}

// ---------------------------------------------------------------------------
// 2. The main source cannot relabel itself out of the audited scope
// ---------------------------------------------------------------------------

/// The fail-open direction. After `#line 500 "elsewhere.h"` the rest of the
/// main source carries a foreign spelling, so name attribution dropped it from
/// the enumerated prototype set. A definition spelled literally is rescued by
/// the raw-scan half of the union; a **macro-produced** one is not, and that
/// is what this pins: `LATE(hidden_after_relabel)` has no raw-scan row at all.
#[test]
fn a_relabelled_main_source_tail_stays_in_scope() {
    if !clang_available() {
        return;
    }
    let fixture = Fixture::new("main-relabel");
    let source = concat!(
        "#define LATE(n) int n(void) { return 3; }\n",
        "int before(void) { return 0; }\n",
        "#line 500 \"elsewhere.h\"\n",
        "int literal_after_relabel(void) { return 1; }\n",
        "LATE(hidden_after_relabel)\n",
    );
    // The raw scan sees only the two literal definitions: the macro-produced
    // one exists in no unpreprocessed text, so nothing else can rescue it.
    assert_eq!(
        legacy_c_source_functions(source),
        ["before", "literal_after_relabel"]
    );
    let (stdin, declared) = both_transports(&fixture, source);

    for (label, unit) in [("stdin", &stdin), ("declared", &declared)] {
        let inventory = enumerate_source_definitions(unit);
        assert_eq!(
            inventory.definitions(),
            ["before", "hidden_after_relabel", "literal_after_relabel"],
            "{label}: a `#line` in the main source must not shrink the denominator"
        );
        assert_eq!(
            inventory.absent_from_raw_scan(),
            ["hidden_after_relabel"],
            "{label}: the macro-produced definition is the one only the \
             expansion can contribute"
        );
        assert!(
            inventory.is_complete(),
            "{label}: {:?}",
            inventory.refusals()
        );
    }
}

// ---------------------------------------------------------------------------
// 3. The two transports agree on an ordinary unit
// ---------------------------------------------------------------------------

/// A control with no `#line` anywhere: header definitions out, main-source
/// definitions in, macro expansions at main-source lines in, and the two
/// transports identical. Without this the tests above could pass vacuously.
#[test]
fn both_transports_enumerate_an_ordinary_unit_identically() {
    if !clang_available() {
        return;
    }
    let fixture = Fixture::new("control");
    fixture.write(
        "control.h",
        "static int header_only(void) { return 1; }\n#define MK(n) int n(void) { return 2; }\n",
    );
    let source =
        "#include \"control.h\"\nMK(from_macro)\nint plain(void) { return 0; }\nint other(void);\n";
    let (stdin, declared) = both_transports(&fixture, source);
    let piped = enumerate_source_definitions(&stdin);
    let file = enumerate_source_definitions(&declared);
    assert_eq!(piped.definitions(), ["from_macro", "plain"]);
    assert_eq!(piped.definitions(), file.definitions());
    assert_eq!(piped.absent_from_raw_scan(), ["from_macro"]);
    assert_eq!(piped.absent_from_raw_scan(), file.absent_from_raw_scan());
    assert!(piped.is_complete() && file.is_complete());
    assert!(piped.supports_unit_safe() && file.supports_unit_safe());
}

// ---------------------------------------------------------------------------
// 4. An unreadable include stack refuses rather than attributing
// ---------------------------------------------------------------------------

/// The ground a broken include stack refuses on.
///
/// A stream Clang actually emits always balances, so the refusal cannot be
/// provoked through a real preprocessing from here; the reader-level matrix
/// that drives both imbalances (a return with nothing to return to, and a file
/// still open at end of output) is
/// `unbalanced_marker_streams_are_refused` in
/// `src/../tests/unit/legacy_enumeration.rs`. What this pins is the ground's
/// classification and that a real, balanced stream is *not* refused, so the new
/// check cannot silently start refusing ordinary units.
#[test]
fn a_balanced_stream_is_established_and_the_unbalanced_ground_is_environmental() {
    if !clang_available() {
        return;
    }
    let fixture = Fixture::new("balanced");
    fixture.write("balanced.h", "int in_header(void) { return 1; }\n");
    let source = "#include \"balanced.h\"\nint subject(void) { return 0; }\n";
    let (stdin, declared) = both_transports(&fixture, source);
    for (label, unit) in [("stdin", &stdin), ("declared", &declared)] {
        let inventory = enumerate_source_definitions(unit);
        assert!(
            inventory.is_complete(),
            "{label}: a real preprocessing balances: {:?}",
            inventory.refusals()
        );
        assert_eq!(inventory.definitions(), ["subject"], "{label}");
    }

    // The ground is a failure of the *tool's* output, not of the analysed
    // source, so under §L21.1 rule 1 it can never contribute to a provisional
    // `Unsafe`.
    let refusal = EnumerationRefusal::UnbalancedIncludeMarkers;
    assert_eq!(refusal.as_str(), "unbalanced-include-markers");
    assert_eq!(refusal.category(), ReasonCategory::EnvironmentUnavailable);
    assert_eq!(refusal.category().subject(), ReasonSubject::Environment);
}
