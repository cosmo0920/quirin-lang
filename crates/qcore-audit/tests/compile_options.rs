use qcore_audit::compile_db::{CompileEntry, compile_entries, compiler_options, shell_words};
use qcore_c_gateway::legacy_c::{C11TranslationUnit, validate_unrestricted_c11};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT: AtomicUsize = AtomicUsize::new(0);
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
        let mut arguments = vec!["clang", "-std=c11"];
        arguments.extend_from_slice(options);
        arguments.extend(["-c", "src/unit.c"]);
        compile_entries(
            &serde_json::json!([{"directory":self.0,"file":"src/unit.c","arguments":arguments}])
                .to_string(),
        )
        .unwrap()
        .remove(0)
    }
    fn preprocess(&self, source: &str, flags: &[&str]) -> String {
        let options = compiler_options(&self.entry(flags)).unwrap();
        validate_unrestricted_c11(&C11TranslationUnit {
            source,
            include_paths: options.include_paths,
            definitions: options.definitions,
            preprocessor_options: options.preprocessor_options,
            dialect: options.dialect,
            ..Default::default()
        })
        .unwrap()
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
        "clang -c src/unit.c -Wall",
        "clang -c src/unit.c -fPIC",
        "clang -c src/unit.c -include hidden.h",
    ] {
        let mut entry = f.entry(&[]);
        entry.command = command.into();
        assert!(compiler_options(&entry).is_err(), "{command}");
    }
}
