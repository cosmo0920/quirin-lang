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

use qcore_core::module_imports::ModuleImports;
use qcore_frontend::frontend::{elaborate_source, parse_source_program};
use qcore_frontend::module_imports::check_source_imports;

#[test]
fn no_module_is_imported_by_default_and_unknown_names_fail_closed() {
    assert!(!ModuleImports::default().concurrency());
    assert!(!ModuleImports::resolve([]).unwrap().concurrency());
    for path in [
        "std",
        "std.concurrency.extra",
        "std.concurrencyx",
        "local.worker",
    ] {
        assert!(ModuleImports::resolve(["std.concurrency", path]).is_err());
        let source =
            parse_source_program(&format!("#import {path}; fn main() -> i32 {{ return 0; }}"))
                .unwrap();
        let errors = elaborate_source(&source).unwrap_err();
        assert_eq!(errors[0].code, "Q014");
        assert_eq!(
            &source.source()[errors[0].span.start..errors[0].span.end],
            format!("#import {path};")
        );
    }
}

#[test]
fn imports_are_structural_exact_and_idempotent() {
    let source = "module library; #import std.concurrency; #import std.concurrency; fn worker() -> unit { return; }";
    let parsed = parse_source_program(source).unwrap();
    assert_eq!(parsed.imports().len(), 2);
    assert_eq!(parsed.declarations.len(), 1);
    assert!(check_source_imports(&parsed).unwrap().concurrency());
    elaborate_source(&parsed).unwrap();
    assert!(parsed.syntax_equivalent(&parse_source_program(&parsed.to_string()).unwrap()));
    for malformed in [
        "import;",
        "#import std.;",
        "#import std.concurrency",
        "import std::*;",
        "import \"std.concurrency\";",
    ] {
        assert!(parse_source_program(malformed).is_err(), "{malformed}");
    }
}

#[test]
fn concurrency_requires_a_real_import_even_in_uncalled_bodies() {
    for body in [
        "concurrency checked; fn main() -> i32 { return 0; }",
        "fn unused() -> unit { thread scope s {} } fn main() -> i32 { return 0; }",
        "fn unused() -> unit { let m = mutex guards x; }",
        "fn unused() -> unit { let b = qcore_buffer_new(1); }",
        "type Handle = task<i32>;",
    ] {
        for prefix in [
            "",
            "// #import std.concurrency;\n",
            "/* #import std.concurrency; */ ",
        ] {
            let parsed = parse_source_program(&format!("{prefix}{body}")).unwrap();
            assert_eq!(elaborate_source(&parsed).unwrap_err()[0].code, "Q015");
        }
        let parsed = parse_source_program(&format!("#import std.concurrency; {body}")).unwrap();
        assert!(check_source_imports(&parsed).unwrap().concurrency());
    }
    let parsed = parse_source_program("fn main() -> i32 { return 0; }").unwrap();
    elaborate_source(&parsed).unwrap();
    assert!(!check_source_imports(&parsed).unwrap().concurrency());
}
