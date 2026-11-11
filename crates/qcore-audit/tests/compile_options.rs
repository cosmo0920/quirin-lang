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

use qcore_audit::compile_db::{CompileEntry, compile_entries, compiler_options, shell_words};
use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, CSourceInput, LegacyCError, validate_unrestricted_c11,
};
use std::{
    fs,
    path::PathBuf,
    sync::{
        Once, OnceLock,
        atomic::{AtomicUsize, Ordering},
    },
};

static NEXT: AtomicUsize = AtomicUsize::new(0);
static CLANG_AVAILABILITY: OnceLock<Result<(), String>> = OnceLock::new();
static CLANG_WARNING: Once = Once::new();

fn clang_available() -> bool {
    let availability =
        CLANG_AVAILABILITY.get_or_init(|| qcore_c_gateway::clang_command().map(|_| ()));
    match availability {
        Ok(()) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG is invalid: {reason}"
            );
            CLANG_WARNING.call_once(|| {
                eprintln!("skipping Clang-dependent compile-option tests: {reason}");
            });
            false
        }
    }
}

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "qcore-option-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        for name in ["src", "regular", "system", "quoted"] {
            fs::create_dir_all(path.join(name)).unwrap();
        }
        Self(path)
    }
    fn entry(&self, options: &[&str]) -> CompileEntry {
        self.entry_with_compiler("clang", options)
    }
    fn clang_entry(&self, options: &[&str]) -> CompileEntry {
        let command = qcore_c_gateway::clang_command().unwrap();
        let compiler = command.get_program().to_str().unwrap();
        self.entry_with_compiler(compiler, options)
    }
    fn entry_with_compiler(&self, compiler: &str, options: &[&str]) -> CompileEntry {
        let mut arguments = vec![compiler, "-std=c11"];
        arguments.extend_from_slice(options);
        arguments.extend(["-c", "src/unit.c"]);
        compile_entries(
            &serde_json::json!([{"directory":self.0,"file":"src/unit.c","arguments":arguments}])
                .to_string(),
        )
        .unwrap()
        .remove(0)
    }
    /// Publish the unit's text where the recorded compile command says it is.
    /// A declared input is analyzed as the file Clang reads, not as loose text.
    fn write_source(&self, source: &str) {
        fs::write(self.0.join("src/unit.c"), source).unwrap();
    }
    fn unit<'a>(&self, source: &'a str, flags: &[&str]) -> C11TranslationUnit<'a> {
        self.write_source(source);
        let options = compiler_options(&self.clang_entry(flags)).unwrap();
        C11TranslationUnit {
            source,
            include_paths: options.include_paths,
            definitions: options.definitions,
            preprocessor_options: options.preprocessor_options,
            dialect: options.dialect,
            input: options.input,
            ..Default::default()
        }
    }
    fn preprocess(&self, source: &str, flags: &[&str]) -> String {
        validate_unrestricted_c11(&self.unit(source, flags)).unwrap()
    }
    /// Preprocess exactly as the recorded compile command does: the same input
    /// word, from the same directory, with the same options.
    fn direct(&self, flags: &[&str]) -> String {
        let output = qcore_c_gateway::clang_command()
            .unwrap()
            .current_dir(&self.0)
            .args(["-x", "c", "-std=c11", "-E", "-P"])
            .args(flags)
            .arg("src/unit.c")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap()
    }
    fn header(&self, directory: &str, value: i32) {
        fs::write(
            self.0.join(directory).join("choice.h"),
            format!("#define CHOICE {value}\n"),
        )
        .unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn system_headers_remain_after_regular_headers_even_when_listed_first() {
    if !clang_available() {
        return;
    }
    let f = Fixture::new();
    f.header("regular", 17);
    f.header("system", 29);
    let source = "#include <choice.h>\nint observed = CHOICE;";
    for flags in [
        vec!["-isystem", "system", "-Iregular"],
        vec!["-isystemsystem", "-I", "regular"],
    ] {
        assert!(f.preprocess(source, &flags).contains("int observed = 17;"));
    }
}

#[test]
fn source_directory_is_quote_only_and_does_not_shadow_angle_headers() {
    if !clang_available() {
        return;
    }
    let f = Fixture::new();
    f.header("src", 5);
    f.header("regular", 17);
    assert!(
        f.preprocess(
            "#include <choice.h>\nint observed = CHOICE;",
            &["-Iregular"]
        )
        .contains("int observed = 17;")
    );
    assert!(
        f.preprocess(
            "#include \"choice.h\"\nint observed = CHOICE;",
            &["-Iregular"]
        )
        .contains("int observed = 5;")
    );
}

#[test]
fn quote_include_class_and_order_are_preserved() {
    if !clang_available() {
        return;
    }
    let f = Fixture::new();
    f.header("regular", 17);
    f.header("quoted", 31);
    assert!(
        f.preprocess(
            "#include \"choice.h\"\nint observed = CHOICE;",
            &["-Iregular", "-iquote", "quoted"]
        )
        .contains("int observed = 31;")
    );
    assert!(
        f.preprocess(
            "#include <choice.h>\nint observed = CHOICE;",
            &["-iquotequoted", "-Iregular"]
        )
        .contains("int observed = 17;")
    );
}

#[test]
fn macro_definition_and_undefinition_order_is_preserved() {
    if !clang_available() {
        return;
    }
    let f = Fixture::new();
    let source = "#ifndef VALUE\n#define VALUE 3\n#endif\nint observed = VALUE;";
    assert!(
        f.preprocess(source, &["-DVALUE=8", "-UVALUE"])
            .contains("int observed = 3;")
    );
    assert!(
        f.preprocess(source, &["-U", "VALUE", "-D", "VALUE=8"])
            .contains("int observed = 8;")
    );
}

#[test]
fn empty_arguments_survive_decoding_and_reject_instead_of_consuming_a_flag() {
    assert_eq!(
        shell_words("clang '' a\"\"b \"\"").unwrap(),
        ["clang", "", "ab", ""]
    );
    assert_eq!(
        shell_words(r#"clang "a\zb" "a\\b""#).unwrap(),
        ["clang", r"a\zb", r"a\b"]
    );
    assert_eq!(
        shell_words("clang \\\n-c src/unit.c").unwrap(),
        ["clang", "-c", "src/unit.c"]
    );
    let f = Fixture::new();
    for flags in [
        vec!["-I", ""],
        vec!["-D", ""],
        vec!["-U", ""],
        vec!["-o", ""],
    ] {
        assert!(compiler_options(&f.entry(&flags)).is_err());
    }
    let mut entry = f.entry(&[]);
    entry.command = "'' -c src/unit.c".into();
    assert!(compiler_options(&entry).is_err());
}

#[test]
fn command_structure_and_unmodeled_options_fail_closed() {
    for expression in [
        "cc; true",
        "cc $(echo flags)",
        "cc `echo flags`",
        "cc *.c",
        "cc \"$FLAGS\"",
    ] {
        assert!(shell_words(expression).is_err(), "{expression}");
    }
    let f = Fixture::new();
    for command in [
        "clang",
        "clang src/unit.c",
        "clang -c",
        "clang -c src/unit.c src/unit.c",
        "clang -c -c src/unit.c",
        "clang -c src/unit.c -o one -o two",
        "clang -c src/unit.c -I=hidden",
        "clang -c src/unit.c -isystem=hidden",
        "clang -c src/unit.c -iquote=hidden",
        "clang -c src/unit.c -I-",
        "clang -c src/unit.c -Werror",
        "clang -c src/unit.c -fshort-enums",
        "clang -c src/unit.c -include hidden.h",
        "clang -std=c11 -c src/unit.c -Wundef-prefix=",
        "clang -std=c11 -c src/unit.c -arch",
        "clang -std=c11 -c src/unit.c -arch -Wall",
    ] {
        let mut entry = f.entry(&[]);
        entry.command = command.into();
        assert!(compiler_options(&entry).is_err(), "{command}");
    }
}

#[test]
fn build_flags_match_direct_clang_preprocessing_and_typed_ast() {
    if !clang_available() {
        return;
    }
    use qcore_c_gateway::{clang_command, typed_c_ast::function_ast};
    use std::io::Write;
    use std::process::Stdio;
    let f = Fixture::new();
    let source = "#if defined(__arm64__)\n#define ARCH 64\n#elif defined(__x86_64__)\n#define ARCH 86\n#else\n#define ARCH 0\n#endif\n#if defined(__PIE__)\n#define MODE (ARCH + 4)\n#elif defined(__PIC__)\n#define MODE (ARCH + 2)\n#else\n#define MODE ARCH\n#endif\nint observed(void) { return MODE; }";
    for flags in [
        vec!["-Wall"],
        vec!["-Wextra"],
        vec!["-g"],
        vec!["-fPIC"],
        vec!["-Wa,--noexecstack"],
        vec!["-Wundef-prefix=TARGET_OS_"],
        vec!["-arch", "arm64"],
        vec!["-arch", "x86_64"],
        vec!["-Wundef-prefix=TARGET_OS_", "-arch", "arm64"],
        vec!["-Wall", "-g", "-fPIC", "-Wa,--noexecstack", "-Wall"],
    ] {
        let mut child = clang_command()
            .unwrap()
            .args(["-x", "c", "-std=c11", "-E", "-P"])
            .args(&flags)
            .arg("-")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(source.as_bytes())
            .unwrap();
        let reference = child.wait_with_output().unwrap();
        let options = compiler_options(&f.clang_entry(&flags)).unwrap();
        let unit = C11TranslationUnit {
            source,
            dialect: options.dialect,
            preprocessor_options: options.preprocessor_options,
            ..Default::default()
        };
        let imported = validate_unrestricted_c11(&unit);
        if !reference.status.success() {
            // For example, PIC is unsupported for some native Windows targets.
            assert!(
                imported.is_err(),
                "unsupported target flags must fail closed"
            );
            continue;
        }
        let expected = String::from_utf8(reference.stdout).unwrap();
        assert_eq!(imported.unwrap(), expected, "{flags:?}");
        // Exercise the AST transport's own preprocessing, so forgetting flags
        // in that phase is visible even though normal inputs are preprocessed.
        let actual_ast = function_ast(source, &unit, "observed").unwrap();
        let expected_ast =
            function_ast(&expected, &C11TranslationUnit::default(), "observed").unwrap();
        fn literals(value: &serde_json::Value, result: &mut Vec<String>) {
            if value["kind"] == "IntegerLiteral" {
                result.push(value["value"].as_str().unwrap().into());
            }
            if let Some(children) = value["inner"].as_array() {
                for child in children {
                    literals(child, result);
                }
            }
        }
        let (mut actual, mut expected) = (vec![], vec![]);
        literals(&actual_ast, &mut actual);
        literals(&expected_ast, &mut expected);
        assert_eq!(actual, expected, "AST flags: {flags:?}");
    }
}

/// Preprocessing is not a function of the text alone. Everything below answers
/// to the input Clang was given, so the configured transport must give Clang
/// the recorded input word from the recorded directory.
const FILENAME_SENSITIVE_UNIT: &str = "#include \"choice.h\"\n\
     const char *file = __FILE__;\n\
     const char *name = __FILE_NAME__;\n\
     const char *base = __BASE_FILE__;\n\
     int observed = CHOICE;\n";

#[test]
fn filename_sensitive_preprocessing_matches_the_recorded_compile_command() {
    if !clang_available() {
        return;
    }
    let f = Fixture::new();
    // `choice.h` exists both beside the source and on the regular search path.
    // A direct compile resolves the quoted include against the includer's own
    // directory; only the recorded input can reproduce that.
    f.header("src", 5);
    f.header("regular", 17);
    let flags = ["-Iregular"];
    let imported = f.preprocess(FILENAME_SENSITIVE_UNIT, &flags);
    assert_eq!(imported, f.direct(&flags));
    assert!(imported.contains("int observed = 5;"));
    assert!(imported.contains("const char *name = \"unit.c\";"));
    assert!(!imported.contains("<stdin>"));

    // The historical standard-input projection is the divergence this transport
    // exists to remove: it answers `<stdin>` for all three file-name macros.
    let mut historical = f.unit(FILENAME_SENSITIVE_UNIT, &flags);
    historical.input = CSourceInput::HistoricalStdin;
    let historical = validate_unrestricted_c11(&historical).unwrap();
    assert_ne!(historical, imported);
    assert_eq!(historical.matches("\"<stdin>\"").count(), 3);
}

#[test]
fn units_without_a_declared_file_reject_constructs_they_cannot_answer() {
    if !clang_available() {
        return;
    }
    let f = Fixture::new();
    f.header("src", 5);
    // An anonymous text has no includer directory either, so its quoted include
    // must be reachable through a declared search path before anything else can
    // be observed about it.
    let quoted = ["-iquote", "src"];
    let mut anonymous = f.unit(FILENAME_SENSITIVE_UNIT, &quoted);
    anonymous.input = CSourceInput::AnonymousText;
    let error = validate_unrestricted_c11(&anonymous).unwrap_err();
    assert!(matches!(error, LegacyCError::UnsupportedInputIdentity(_)));
    let error = error.to_string();
    assert!(error.contains("__FILE__"), "{error}");
    assert!(error.contains("<stdin>"), "{error}");

    // `__builtin_FILE` survives preprocessing and is answered by the frontend,
    // which reads preprocessed text from standard input in every mode. It is
    // rejected even when the unit does declare its file.
    let builtin = "const char *where(void) { return __builtin_FILE(); }\n";
    for input in [
        CSourceInput::AnonymousText,
        compiler_options(&f.clang_entry(&[])).unwrap().input,
    ] {
        let mut unit = f.unit(builtin, &[]);
        unit.input = input;
        let error = validate_unrestricted_c11(&unit).unwrap_err().to_string();
        assert!(error.contains("__builtin_FILE"), "{error}");
    }

    // The pinned Cycle 19 compatibility route keeps its archived projection.
    for (source, flags) in [
        (FILENAME_SENSITIVE_UNIT, quoted.as_slice()),
        (builtin, [].as_slice()),
    ] {
        let mut historical = f.unit(source, flags);
        historical.input = CSourceInput::HistoricalStdin;
        assert!(validate_unrestricted_c11(&historical).is_ok());
    }
}

#[test]
fn a_declared_input_must_be_the_file_that_holds_the_declared_source() {
    if !clang_available() {
        return;
    }
    let f = Fixture::new();
    f.header("src", 5);
    let unit = f.unit(FILENAME_SENSITIVE_UNIT, &[]);
    // Analyzing text that differs from the file Clang reads would record one
    // unit's identity for another unit's bytes.
    f.write_source("int replaced(void) { return 0; }\n");
    let error = validate_unrestricted_c11(&unit).unwrap_err().to_string();
    assert!(
        error.contains("does not hold the declared source bytes"),
        "{error}"
    );

    fs::remove_file(f.0.join("src/unit.c")).unwrap();
    let error = validate_unrestricted_c11(&unit).unwrap_err().to_string();
    assert!(error.contains("unreadable"), "{error}");

    for input in [
        CSourceInput::DeclaredFile {
            spelled: "src/unit.c".into(),
            directory: PathBuf::from("relative"),
        },
        CSourceInput::DeclaredFile {
            spelled: "-nostdinc".into(),
            directory: f.0.clone(),
        },
    ] {
        let unit = C11TranslationUnit {
            source: "int value(void) { return 0; }",
            input,
            ..Default::default()
        };
        assert!(matches!(
            validate_unrestricted_c11(&unit),
            Err(LegacyCError::UnsupportedInputIdentity(_))
        ));
    }
}

#[test]
fn quoted_dollars_are_literal_c_macro_bytes_without_shell_or_make_expansion() {
    if !clang_available() {
        return;
    }
    let f = Fixture::new();
    let definition = r#"-D__FILENAME__="$$(subst /source/,,$$(abspath $$<))""#;
    let source = "const char *name = __FILENAME__;";
    let imported = f.preprocess(source, &[definition, "-Wextra"]);
    assert!(imported.contains(r#""$$(subst /source/,,$$(abspath $$<))""#));
    for rejected in [
        "-DVALUE=$(unresolved)",
        r#"-DVALUE="literal" + $(unresolved)"#,
        r#"-I$unresolved/include"#,
    ] {
        assert!(
            compiler_options(&f.clang_entry(&[rejected])).is_err(),
            "{rejected}"
        );
    }
}

#[test]
fn frontend_flags_require_the_actual_clang_executable_and_an_explicit_dialect() {
    if !clang_available() {
        return;
    }
    let f = Fixture::new();
    let entry = f.clang_entry(&["-Wall"]);
    assert!(compiler_options(&entry).is_ok());
    let mut absent_dialect = entry.clone();
    absent_dialect.command = absent_dialect.command.replace("'-std=c11'", "");
    assert!(
        compiler_options(&absent_dialect)
            .err()
            .unwrap()
            .contains("explicit supported -std=")
    );
    let other = f.0.join("untrusted-compiler");
    fs::write(&other, "this is never executed").unwrap();
    let mut cxx = entry.clone();
    cxx.command = "clang++ -std=c11 -Wall -c src/unit.c".into();
    assert!(
        compiler_options(&cxx)
            .err()
            .unwrap()
            .contains("Clang C driver")
    );
    let mut different = entry;
    different.command = format!("'{}' -std=c11 -Wall -c src/unit.c", other.display());
    assert!(
        compiler_options(&different)
            .err()
            .unwrap()
            .contains("compiler equivalence is unproven")
    );
}
