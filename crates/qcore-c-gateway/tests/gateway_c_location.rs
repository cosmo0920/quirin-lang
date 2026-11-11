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

//! Original (pre-preprocessing) coordinates for gateway diagnostics
//! (card **GATEWAY-C-LOCATION**, `docs/reports/gateway-c-location/README.md`).
//!
//! Every line number the analyzer carries is a line of the `-E -P` preprocessed
//! translation unit. That number stays what it was — it is the leaf identity
//! every pinned comparison is keyed on — but it names a text nobody reads, so a
//! *presented* diagnostic now names the C a person wrote.
//!
//! These tests drive real Clang preprocessings and pin four things:
//!
//! * a coordinate is the original file and line, and a definition produced by a
//!   macro expansion presents the **expansion site's** line;
//! * a `#line` directive in analysed source cannot **forge** one — this is
//!   standing constraint 12's presentation half. Both measured directions are
//!   covered: a header writing the primary input's own spelling, and a main
//!   source renaming its own tail. In both the coordinate is *withdrawn*,
//!   never renamed, and in both the **audited scope** is unchanged, which is
//!   the constraint-12 property itself;
//! * the two transports agree, because the primary input's presented name is
//!   the caller's and no marker's;
//! * the map is built against the analysed text, and disagreement refuses
//!   rather than approximating.

use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, CSourceInput, LegacyFunctionScope, OriginFile,
    enumerate_source_definitions, map_preprocessed_lines, validate_unrestricted_c11,
};

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
            "qcore-location-{tag}-{}-{:?}",
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

fn piped<'a>(fixture: &Fixture, source: &'a str) -> C11TranslationUnit<'a> {
    C11TranslationUnit {
        source,
        include_paths: vec![fixture.include()],
        function_scope: LegacyFunctionScope::MainSourceDefinitions,
        input: CSourceInput::HistoricalStdin,
        ..C11TranslationUnit::default()
    }
}

fn declared<'a>(fixture: &Fixture, source: &'a str) -> C11TranslationUnit<'a> {
    fixture.write("subject.c", source);
    C11TranslationUnit {
        source,
        include_paths: vec![fixture.include()],
        function_scope: LegacyFunctionScope::MainSourceDefinitions,
        input: CSourceInput::DeclaredFile {
            spelled: "subject.c".to_owned(),
            directory: fixture.directory.clone(),
        },
        ..C11TranslationUnit::default()
    }
}

/// The preprocessed line whose text, trimmed, is exactly `needle`.
fn line_of(preprocessed: &str, needle: &str) -> usize {
    let mut found = None;
    for (index, text) in preprocessed.split('\n').enumerate() {
        if text.trim() == needle {
            assert!(found.is_none(), "`{needle}` is not unique in the unit");
            found = Some(index + 1);
        }
    }
    found.unwrap_or_else(|| panic!("`{needle}` is not in the preprocessed unit"))
}

// ---------------------------------------------------------------------------
// 1. A coordinate is the original file and line
// ---------------------------------------------------------------------------

#[test]
fn a_main_source_line_presents_the_declared_file_and_its_own_line() {
    if !clang_available() {
        return;
    }
    let fixture = Fixture::new("main-line");
    // Line 1 is the include; the body's `return 7;` is on line 5.
    let source = "#include \"h.h\"\n\nint subject(void)\n{\n    return 7;\n}\n";
    fixture.write("h.h", "int helper(void);\n");
    let unit = piped(&fixture, source);
    let preprocessed = validate_unrestricted_c11(&unit).unwrap();
    let map = map_preprocessed_lines(&unit, &preprocessed);
    assert!(map.is_established(), "{:?}", map.refusals());

    let line = line_of(&preprocessed, "return 7;");
    let origin = map.original(line).expect("the body line carries an origin");
    assert_eq!(origin.depth, 0);
    assert_eq!(origin.file, OriginFile::Primary);
    assert_eq!(origin.line, 5);
    // The presented file is the caller's, never the transport's `<stdin>`.
    assert_eq!(
        map.present(line, "src/subject.c").unwrap(),
        "src/subject.c:5"
    );
    assert!(
        !map.present(line, "src/subject.c")
            .unwrap()
            .contains("stdin")
    );
}

#[test]
fn a_header_line_presents_the_header_its_entering_marker_named() {
    if !clang_available() {
        return;
    }
    let fixture = Fixture::new("header-line");
    fixture.write(
        "h.h",
        "int helper(void);\n\nstatic int inline_helper(void)\n{\n    return 3;\n}\n",
    );
    let source = "#include \"h.h\"\nint subject(void) { return inline_helper(); }\n";
    let unit = piped(&fixture, source);
    let preprocessed = validate_unrestricted_c11(&unit).unwrap();
    let map = map_preprocessed_lines(&unit, &preprocessed);
    assert!(map.is_established(), "{:?}", map.refusals());

    let line = line_of(&preprocessed, "return 3;");
    let origin = map
        .original(line)
        .expect("the header line carries an origin");
    assert_eq!(origin.depth, 1, "a header sits at include depth 1");
    assert_eq!(origin.line, 5);
    let presented = map.present(line, "src/subject.c").unwrap();
    assert!(
        presented.ends_with("h.h:5"),
        "a header line names the header, not the primary input: {presented}"
    );
}

#[test]
fn a_macro_expansion_presents_its_expansion_site() {
    if !clang_available() {
        return;
    }
    let fixture = Fixture::new("expansion");
    // `DEFN` expands to a whole definition. The expanded text exists only after
    // preprocessing; the line a reader can see is the invocation's, line 6.
    let source = concat!(
        "#define DEFN(name) \\\n",
        "    int name(void)   \\\n",
        "    {                \\\n",
        "        return 11;   \\\n",
        "    }\n",
        "DEFN(expanded)\n",
        "int subject(void) { return expanded(); }\n",
    );
    let unit = piped(&fixture, source);
    let preprocessed = validate_unrestricted_c11(&unit).unwrap();
    let map = map_preprocessed_lines(&unit, &preprocessed);
    assert!(map.is_established(), "{:?}", map.refusals());

    let line = preprocessed
        .split('\n')
        .position(|text| text.contains("int expanded(void)"))
        .expect("the expansion is in the preprocessed unit")
        + 1;
    let origin = map.original(line).expect("the expansion carries an origin");
    assert_eq!(origin.file, OriginFile::Primary);
    assert_eq!(
        origin.line, 6,
        "an expansion presents the line the invocation is written on"
    );
}

// ---------------------------------------------------------------------------
// 2. A `#line` cannot forge a coordinate (standing constraint 12)
// ---------------------------------------------------------------------------

/// A header whose `#line` writes `"<stdin>"` — the exact spelling the pinned
/// compatibility transport gives its primary input, and one an author needs no
/// knowledge of the build to write.
///
/// Two properties, both required: the audited **scope** is unmoved (this is
/// constraint 12, and `e01_stdin_attribution.rs` owns it), and the
/// **presentation** does not hand the header's line the main file either. The
/// coordinate is withdrawn, not renamed.
#[test]
fn a_header_relabelled_to_the_primary_spelling_presents_nothing() {
    if !clang_available() {
        return;
    }
    let fixture = Fixture::new("hdr-forge");
    fixture.write(
        "forge.h",
        "#line 100 \"<stdin>\"\nstatic int relabelled(void) { return 13; }\n",
    );
    let source = "#include \"forge.h\"\nint subject(void) { return 0; }\n";
    let unit = piped(&fixture, source);
    let preprocessed = validate_unrestricted_c11(&unit).unwrap();
    let map = map_preprocessed_lines(&unit, &preprocessed);
    assert!(map.is_established(), "{:?}", map.refusals());

    // The scope is unmoved: the header's definition is not main-source.
    let inventory = enumerate_source_definitions(&unit);
    assert_eq!(inventory.definitions(), ["subject"]);

    let forged = preprocessed
        .split('\n')
        .position(|text| text.contains("relabelled"))
        .expect("the relabelled definition is in the preprocessed unit")
        + 1;
    assert_eq!(
        map.original(forged),
        None,
        "a header that writes the primary spelling gets no coordinate at all"
    );
    assert_eq!(
        map.site(forged, "src/subject.c"),
        format!("preprocessed line {forged}")
    );
    assert!(map.relabelled_lines() >= 1);

    // …and the main source's own line is unaffected.
    let subject = line_of(&preprocessed, "int subject(void) { return 0; }");
    assert_eq!(
        map.present(subject, "src/subject.c").unwrap(),
        "src/subject.c:2"
    );
}

/// A `#line` in the **main source** naming another file. The name is never
/// adopted; the lines it governs carry no coordinate, and the audited scope
/// still contains the definitions after it.
#[test]
fn a_main_source_line_directive_naming_another_file_presents_nothing() {
    if !clang_available() {
        return;
    }
    let fixture = Fixture::new("tail-forge");
    let source = concat!(
        "int before(void) { return 1; }\n",
        "#line 5000 \"forged.c\"\n",
        "int after(void) { return 2; }\n",
    );
    let unit = piped(&fixture, source);
    let preprocessed = validate_unrestricted_c11(&unit).unwrap();
    let map = map_preprocessed_lines(&unit, &preprocessed);
    assert!(map.is_established(), "{:?}", map.refusals());

    // Constraint 12: the tail stays in the audited denominator.
    let inventory = enumerate_source_definitions(&unit);
    assert_eq!(inventory.definitions(), ["after", "before"]);

    let before = line_of(&preprocessed, "int before(void) { return 1; }");
    let after = line_of(&preprocessed, "int after(void) { return 2; }");
    assert_eq!(
        map.present(before, "src/subject.c").unwrap(),
        "src/subject.c:1"
    );
    assert_eq!(
        map.original(after),
        None,
        "a renamed tail is withdrawn, never presented under the forged name"
    );
    let presented = map.site(after, "src/subject.c");
    assert!(!presented.contains("forged.c"), "{presented}");
    assert_eq!(presented, format!("preprocessed line {after}"));
}

/// A `#line` with **no file spelling** renumbers the file it is written in,
/// which is its C11 §6.10.4 meaning and the one thing a directive may still do.
/// It moves no file identity, no include depth and no audited scope. Recorded
/// as `GATEWAY-LOCATION-LINE-ONLY-DIRECTIVE`.
#[test]
fn a_line_directive_without_a_file_renumbers_its_own_file_only() {
    if !clang_available() {
        return;
    }
    let fixture = Fixture::new("renumber");
    let source = concat!(
        "int before(void) { return 1; }\n",
        "#line 5000\n",
        "int after(void) { return 2; }\n",
    );
    let unit = piped(&fixture, source);
    let preprocessed = validate_unrestricted_c11(&unit).unwrap();
    let map = map_preprocessed_lines(&unit, &preprocessed);
    assert!(map.is_established(), "{:?}", map.refusals());

    let after = line_of(&preprocessed, "int after(void) { return 2; }");
    let origin = map
        .original(after)
        .expect("the renumbered line is attributed");
    assert_eq!(origin.depth, 0, "the directive moves no include depth");
    assert_eq!(
        origin.file,
        OriginFile::Primary,
        "the directive names no file, so the caller's still stands"
    );
    assert_eq!(origin.line, 5000, "the file's own author renumbered it");
}

// ---------------------------------------------------------------------------
// 3. The transports agree, and the analysed text is the one mapped
// ---------------------------------------------------------------------------

#[test]
fn both_transports_present_the_same_coordinates() {
    if !clang_available() {
        return;
    }
    let fixture = Fixture::new("transports");
    fixture.write("h.h", "int helper(void);\n");
    let source = "#include \"h.h\"\n\nint subject(void)\n{\n    return 7;\n}\n";
    let stdin_unit = piped(&fixture, source);
    let declared_unit = declared(&fixture, source);

    let stdin_text = validate_unrestricted_c11(&stdin_unit).unwrap();
    let declared_text = validate_unrestricted_c11(&declared_unit).unwrap();
    let stdin_map = map_preprocessed_lines(&stdin_unit, &stdin_text);
    let declared_map = map_preprocessed_lines(&declared_unit, &declared_text);
    assert!(stdin_map.is_established() && declared_map.is_established());

    let stdin_line = line_of(&stdin_text, "return 7;");
    let declared_line = line_of(&declared_text, "return 7;");
    assert_eq!(
        stdin_map.present(stdin_line, "src/subject.c"),
        declared_map.present(declared_line, "src/subject.c"),
        "the presented coordinate does not depend on how the input is transported"
    );
}

#[test]
fn a_map_built_against_a_different_text_refuses_rather_than_approximates() {
    if !clang_available() {
        return;
    }
    let fixture = Fixture::new("mismatch");
    let source = "int subject(void) { return 0; }\n";
    let unit = piped(&fixture, source);
    let map = map_preprocessed_lines(&unit, "int something_else(void) { return 0; }\n");
    assert!(
        !map.is_established(),
        "a text this unit did not produce must not be mapped"
    );
    assert_eq!(map.original(1), None);
}

// ---------------------------------------------------------------------------
// 4. The presentation is inert for the analysis
// ---------------------------------------------------------------------------

/// The map is read-only: deriving it changes neither the preprocessed text the
/// analyzer reads nor the enumerated denominator the scope rule produces.
#[test]
fn deriving_the_map_changes_neither_the_text_nor_the_scope() {
    if !clang_available() {
        return;
    }
    let fixture = Fixture::new("inert");
    fixture.write("h.h", "static int hidden(void) { return 1; }\n");
    let source = "#include \"h.h\"\nint subject(void) { return hidden(); }\n";
    let unit = piped(&fixture, source);

    let before_text = validate_unrestricted_c11(&unit).unwrap();
    let before_inventory = enumerate_source_definitions(&unit);
    let _map = map_preprocessed_lines(&unit, &before_text);
    let after_text = validate_unrestricted_c11(&unit).unwrap();
    let after_inventory = enumerate_source_definitions(&unit);

    assert_eq!(before_text, after_text);
    assert_eq!(before_inventory, after_inventory);
}
