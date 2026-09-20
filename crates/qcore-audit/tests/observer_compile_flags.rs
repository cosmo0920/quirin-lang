use qcore_audit::compile_db::{compile_entries, compiler_options};
use qcore_c_gateway::legacy_c::{C11TranslationUnit, prepare_translation_unit};

#[test]
fn observer_flags_preserve_sanitizer_preprocessor_features() {
    let directory = std::env::temp_dir().join(format!("qcore-observer-flags-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let source = "#if __has_feature(address_sanitizer) && __has_feature(undefined_behavior_sanitizer)\nint actual_sanitizers;\n#else\nint wrong_configuration;\n#endif\n";
    let file = directory.join("flags.c");
    std::fs::write(&file, source).unwrap();
    let entries = compile_entries(&serde_json::json!([{
        "directory": directory, "file": file,
        "arguments": ["clang", "-std=gnu17", "-ffunction-sections", "-fdata-sections", "-fsanitize=address,undefined", "-c", file]
    }]).to_string()).unwrap();
    let options = compiler_options(&entries[0]).unwrap();
    let unit = C11TranslationUnit { source, input: options.input, dialect: options.dialect,
        preprocessor_options: options.preprocessor_options, ..Default::default() };
    let prepared = prepare_translation_unit(&unit);
    let effective = prepared.preprocessed_source().unwrap();
    assert!(effective.contains("actual_sanitizers"));
    assert!(!effective.contains("wrong_configuration"));
    let mut unsupported = entries[0].clone();
    unsupported.command = unsupported.command.replace("-fsanitize=address,undefined", "-fsanitize=thread");
    assert!(compiler_options(&unsupported).err().unwrap().contains("-fsanitize=thread"));
    std::fs::remove_dir_all(directory).unwrap();
}
