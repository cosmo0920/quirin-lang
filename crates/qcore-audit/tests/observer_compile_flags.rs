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

use qcore_audit::compile_db::{compile_entries, compiler_options};
use qcore_c_gateway::ClangIdentity;
use qcore_c_gateway::legacy_c::{C11TranslationUnit, prepare_translation_unit};

#[test]
fn observer_flags_preserve_sanitizer_preprocessor_features() {
    let directory =
        std::env::temp_dir().join(format!("qcore-observer-flags-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let source = "#if __has_feature(address_sanitizer) && __has_feature(undefined_behavior_sanitizer)\nint actual_sanitizers;\n#else\nint wrong_configuration;\n#endif\n";
    let file = directory.join("flags.c");
    std::fs::write(&file, source).unwrap();
    let dependency = directory.join("flags.d");
    std::fs::write(&dependency, "retained build dependency output\n").unwrap();
    let compiler = ClangIdentity::capture().expect("Clang is required for this native check");
    let entries = compile_entries(&serde_json::json!([{
        "directory": directory, "file": file,
        "arguments": [compiler.program(), "-std=gnu17", "-ffunction-sections", "-fdata-sections", "-fsanitize=address,undefined", "-MMD", "-MF", dependency, "-c", file]
    }]).to_string()).unwrap();
    let options = compiler_options(&entries[0]).unwrap();
    let unit = C11TranslationUnit {
        source,
        input: options.input,
        dialect: options.dialect,
        preprocessor_options: options.preprocessor_options,
        ..Default::default()
    };
    let prepared = prepare_translation_unit(&unit);
    let effective = prepared.preprocessed_source().unwrap();
    assert!(effective.contains("actual_sanitizers"));
    assert!(!effective.contains("wrong_configuration"));
    assert_eq!(
        std::fs::read_to_string(dependency).unwrap(),
        "retained build dependency output\n"
    );
    let mut unpaired = entries[0].clone();
    unpaired.command = unpaired.command.replace("'-MMD'", "");
    assert!(
        compiler_options(&unpaired)
            .err()
            .unwrap()
            .contains("paired")
    );
    let mut unsupported = entries[0].clone();
    unsupported.command = unsupported
        .command
        .replace("-fsanitize=address,undefined", "-fsanitize=thread");
    assert!(
        compiler_options(&unsupported)
            .err()
            .unwrap()
            .contains("-fsanitize=thread")
    );
    std::fs::remove_dir_all(directory).unwrap();
}
