use super::*;
use crate::frontend::parse_source_program;

const CHILD: &str = "module contract;\n\
#include \"fixture-libc:libc/string.qcore\";\n\
#include \"fixture-libc:libc/heap.qcore\";\n\
extern fn copy(d: ptr<mut, u8>, s: ptr<const, u8>, n: usize) -> ptr<mut, u8>\n\
contract inherit libc::memcpy;\n\
extern fn drop_it(p: ptr<mut, u8>) -> unit contract inherit libc::free;\n";

#[test]
fn reads_directives_and_inherit_sites_with_their_spans() {
    let syntax = scan_contract_inclusion(CHILD).unwrap();
    assert_eq!(syntax.includes.len(), 2);
    assert_eq!(syntax.includes[0].crate_name, "fixture-libc");
    assert_eq!(syntax.includes[0].module, "libc/string.qcore");
    assert_eq!(
        syntax.includes[1].target(),
        "fixture-libc:libc/heap.qcore".to_owned()
    );
    let first = &CHILD[syntax.includes[0].span.start..syntax.includes[0].span.end];
    assert_eq!(first, "#include \"fixture-libc:libc/string.qcore\";");
    let parents: Vec<_> = syntax.inherits.iter().map(|s| s.parent.as_str()).collect();
    assert_eq!(parents, ["memcpy", "free"]);
    let site = &syntax.inherits[0];
    assert_eq!(
        &CHILD[site.span.start..site.span.end],
        "inherit libc::memcpy"
    );
    assert_eq!(
        &CHILD[site.contract_span.start..site.contract_span.end],
        "contract"
    );
    assert!(contains_contract_inclusion_syntax(CHILD));
}

#[test]
fn a_module_without_either_form_is_empty_and_unaffected() {
    let plain = "module contract;\nextern fn f(p: ptr<mut, u8>) -> unit contract {\n\
                 requires resource_contract_v1;\n\
                 ensures normal resource_released(p, nullable);\n\
                 ensures normal resource_return_none();\n};\n";
    assert!(!contains_contract_inclusion_syntax(plain));
    assert!(scan_contract_inclusion(plain).unwrap().is_empty());
    // Contextual, not reserved: both words are ordinary identifiers elsewhere.
    let words = "module contract;\nextern fn f(inherit: usize, include: usize) -> unit contract {\n\
                 requires resource_contract_v1;\n\
                 ensures normal resource_borrowed(inherit);\n\
                 ensures normal resource_borrowed(include);\n\
                 ensures normal resource_return_none();\n};\n";
    assert!(!contains_contract_inclusion_syntax(words));
    assert!(parse_source_program(words).is_ok());
}

#[test]
fn the_structural_parser_refuses_both_forms_by_name() {
    let error = parse_source_program(CHILD).unwrap_err();
    assert!(error.message.contains("`#include`"), "{error}");
    let inherit_only =
        "module contract;\nextern fn f(p: ptr<mut, u8>) -> unit contract inherit libc::free;\n";
    let error = parse_source_program(inherit_only).unwrap_err();
    assert!(error.message.contains("`contract inherit"), "{error}");
}

fn refusal(source: &str) -> ContractInclusionSyntaxError {
    scan_contract_inclusion(source).unwrap_err()
}

#[test]
fn misplaced_and_malformed_forms_are_refused() {
    use ContractInclusionSyntaxErrorKind::*;
    let cases: &[(&str, ContractInclusionSyntaxErrorKind)] = &[
        // Not a contract module.
        ("#include \"a:b.qcore\";\n", Malformed),
        ("module library;\n#include \"a:b.qcore\";\n", Malformed),
        // Operand shape.
        ("module contract;\n#include a;\n", Malformed),
        ("module contract;\n#include \"a:b.qcore\"\n", Malformed),
        // After a declaration.
        (
            "module contract;\nextern fn f() -> unit contract inherit g;\n#include \"a:b.qcore\";\n",
            Malformed,
        ),
        // Inherit tail shape.
        (
            "module contract;\nextern fn f() -> unit contract inherit;\n",
            Malformed,
        ),
        (
            "module contract;\nextern fn f() -> unit contract inherit g h;\n",
            Malformed,
        ),
        (
            "module contract;\nextern fn f() -> unit contract { requires contract inherit g; };\n",
            Malformed,
        ),
        // Extension clauses.
        (
            "module contract;\nextern fn f() -> unit contract inherit g { requires resource_contract_v1; };\n",
            ExtensionClauses,
        ),
    ];
    for (source, kind) in cases {
        assert_eq!(refusal(source).kind, *kind, "{source}");
    }
}

#[test]
fn include_targets_admit_only_crate_qualified_relative_module_paths() {
    let target = |text: &str| {
        scan_contract_inclusion(&format!("module contract;\n#include \"{text}\";\n"))
            .map(|syntax| syntax.includes[0].target())
            .map_err(|error| error.kind)
    };
    assert_eq!(
        target("quirin-provider-libc:libc/memory.qcore"),
        Ok("quirin-provider-libc:libc/memory.qcore".to_owned())
    );
    assert_eq!(
        target("a_b:x/y_z-1.qcore"),
        Ok("a_b:x/y_z-1.qcore".to_owned())
    );
    for bad in [
        "libc/memory.qcore",
        "quirin-provider-libc:/libc/memory.qcore",
        "quirin-provider-libc:../libc/memory.qcore",
        "quirin-provider-libc:libc/../memory.qcore",
        "quirin-provider-libc:./libc/memory.qcore",
        "quirin-provider-libc:libc//memory.qcore",
        "quirin-provider-libc:libc\\\\memory.qcore",
        "quirin-provider-libc:libc/memory",
        "quirin-provider-libc:libc/Memory.qcore",
        "Quirin:libc/memory.qcore",
        "-libc:libc/memory.qcore",
        ":libc/memory.qcore",
        "a:b:c.qcore",
        "a:.qcore",
    ] {
        assert_eq!(
            target(bad),
            Err(ContractInclusionSyntaxErrorKind::Target),
            "{bad}"
        );
    }
}
