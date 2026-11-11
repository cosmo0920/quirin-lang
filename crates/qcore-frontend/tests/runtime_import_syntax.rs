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

use qcore_frontend::frontend::{elaborate_source, inspect_source_imports, parse_source_program};

#[test]
fn canonical_standard_directive_and_legacy_form_share_capabilities() {
    for directive in ["#import std.io;", "import std.io;"] {
        let text = format!("{directive} fn main() -> i32 {{ return 0; }}");
        let parsed = parse_source_program(&text).unwrap();
        assert_eq!(parsed.imports()[0].value, "std.io");
        assert_eq!(
            &text[parsed.imports()[0].span.start..parsed.imports()[0].span.end],
            directive
        );
        elaborate_source(&parsed).unwrap();
    }
    let parsed =
        parse_source_program("#import std.unknown; fn main() -> i32 { return 0; }").unwrap();
    assert_eq!(elaborate_source(&parsed).unwrap_err()[0].code, "Q014");
}

#[test]
fn quoted_directives_are_inventory_only_until_resolved() {
    let source = "module library;\n#import \"./helper\";\n#import \"../shared/other.qr\";\nfn f() -> i32 { return 1; }";
    let inventory = inspect_source_imports(source).unwrap();
    assert_eq!(inventory.runtime_imports.len(), 2);
    assert_eq!(inventory.runtime_imports[0].value, "./helper");
    let error = parse_source_program(source).unwrap_err();
    assert!(error.message.contains("unresolved runtime"));
    assert_eq!(
        &source[error.span.start..error.span.end],
        "#import \"./helper\";"
    );
}

#[test]
fn malformed_directives_and_bare_relative_imports_refuse() {
    for source in [
        "import \"./helper\";",
        "#import;",
        "#import std.*;",
        "#import \"a\"",
        "#import \"\";",
        "#import \"a\\nb\";",
        "#import \"a\nb\";",
        "#import \"a\" extra;",
        "fn main() -> i32 { #import \"a\"; return 0; }",
    ] {
        assert!(inspect_source_imports(source).is_err(), "{source:?}");
    }
}

#[test]
fn comments_and_quoted_lookalikes_do_not_add_graph_edges() {
    let text = "// #import \"missing\";\n/* #import \"other\"; */ fn main() -> i32 { return 0; }";
    assert!(
        inspect_source_imports(text)
            .unwrap()
            .runtime_imports
            .is_empty()
    );
    elaborate_source(&parse_source_program(text).unwrap()).unwrap();
}

#[test]
fn directives_between_functions_terminate_the_preceding_body() {
    let standard = "fn first() -> i32 { return 1; } #import std.io; fn main() -> i32 { return 0; }";
    assert_eq!(
        parse_source_program(standard).unwrap().declarations.len(),
        2
    );
    let runtime =
        "fn first() -> i32 { return 1; } #import \"helper\"; fn main() -> i32 { return 0; }";
    assert_eq!(
        inspect_source_imports(runtime)
            .unwrap()
            .runtime_imports
            .len(),
        1
    );
    assert!(
        parse_source_program(runtime)
            .unwrap_err()
            .message
            .contains("unresolved runtime")
    );
}
