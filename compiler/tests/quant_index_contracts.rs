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

use qcore_compiler::{
    CompileOptions, IndexExpression, SemanticPass, SourceFile, compile_files, elaborate_source,
    parse_source_program,
};

fn source(quant: &str) -> String {
    format!(
        "dimension Bytes : Idx + Ordered; quant axis memory : credit<Bytes>; extern fn f(q: {quant} use one, tail: i32) -> unit; fn main() -> i32 {{ return 0; }}"
    )
}

#[test]
fn shorthand_and_vector_where_bindings_elaborate_to_dimensioned_values() {
    for quant in [
        "quant<n where n := 5>",
        "quant<credit, { memory = n } where n := m + 2, m := 3, n < 6, n > 4>",
    ] {
        let source = source(quant);
        let parsed = parse_source_program(&source).unwrap();
        let typed = elaborate_source(&parsed).unwrap();
        let function = typed
            .declarations
            .iter()
            .find(|decl| decl.name == "f")
            .unwrap();
        assert_eq!(function.parameters.len(), 2);
        let quant = &function.quant_index_types[0];
        assert_eq!(
            quant.fields["memory"].expression(),
            &IndexExpression::Literal(5)
        );
        assert_eq!(
            quant.bindings["n"].expression(),
            &IndexExpression::Literal(5)
        );
        assert_eq!(quant.fields["memory"].dimension().to_string(), "Bytes");
        assert!(parsed.syntax_equivalent(&parse_source_program(&parsed.to_string()).unwrap()));
        // Card `W48-QR-SCALAR-TYPES-R3` (`W48-INERT-EXTERN-DECLARATION-TYPES`):
        // the uncalled declaration is read, and a `quant<…>` parameter is no
        // type an extern declaration can state, so the module refuses at
        // `Q020` after every earlier pass — the elaboration above included —
        // admitted it. `remaining-v04`'s P08 and P10 are this shape.
        assert_inert_declaration_refusal(&source, "where.qcore");
    }
}

/// The one refusal an executable module declaring an uncalled `extern fn` with
/// a quantity parameter now receives (card `W48-QR-SCALAR-TYPES-R3`).
fn assert_inert_declaration_refusal(source: &str, path: &str) {
    let errors = compile_files(
        &[SourceFile::new(path, source.to_owned())],
        &CompileOptions::default(),
    )
    .expect_err("an unread quant parameter is refused");
    assert_eq!(
        errors.iter().map(|d| d.code.as_str()).collect::<Vec<_>>(),
        ["Q020"],
        "{errors:?}"
    );
    assert!(errors[0].message.contains("parameter `q` of `extern fn f`"));
}

#[test]
fn rejects_false_cyclic_runtime_and_overflowing_index_bindings() {
    for quant in [
        "quant<n where n := 5, n <= 4>",
        "quant<n where n := n + 1>",
        "quant<n where n := m, m := n>",
        "quant<n where n := tail>",
        "quant<n where n := 5, n := 6>",
        "quant<q where q := 5>",
        "quant<n where n := 340282366920938463463374607431768211455 + 1>",
        "quant<n where n := monus(2, 3)>",
        "quant<n where n := 5, unused := 7>",
    ] {
        let errors = compile_files(
            &[SourceFile::new("bad.qcore", source(quant))],
            &CompileOptions::default(),
        )
        .unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.pass == SemanticPass::Index && error.code == "Q016"),
            "{quant}: {errors:?}"
        );
    }
}

#[test]
fn rejects_effectful_and_nonlinear_where_syntax() {
    for quant in [
        "quant<n where n := allocate()>",
        "quant<n where n := object.field>",
        "quant<n where n := *pointer>",
        "quant<n where n := -5>",
        "quant<n where n := 1.5>",
        "quant<n where n := m * k, m := 2, k := 3>",
    ] {
        assert!(
            compile_files(
                &[SourceFile::new("bad.qcore", source(quant))],
                &CompileOptions::default()
            )
            .is_err(),
            "{quant}"
        );
    }
}

#[test]
fn shorthand_is_ambiguous_with_two_axes_and_binders_cannot_cross_dimensions() {
    let preamble = "dimension Bytes : Idx + Ordered; dimension Seconds : Idx + Ordered; quant axis memory : credit<Bytes>; quant axis time : credit<Seconds>;";
    for quant in [
        "quant<n where n := 5>",
        "quant<credit, { memory = n, time = n } where n := 5>",
        "quant<credit, { memory = n, time = m } where n := 5, m := 5, n == m>",
    ] {
        let parsed =
            parse_source_program(&format!("{preamble} extern fn f(q: {quant}) -> unit;")).unwrap();
        assert!(elaborate_source(&parsed).is_err(), "{quant}");
    }
    let parsed = parse_source_program(&format!("{preamble} extern fn f(q: quant<credit, {{ memory = n, time = m }} where n := 5, m := 5>) -> unit;")).unwrap();
    assert!(elaborate_source(&parsed).is_ok());
}

#[test]
fn bindings_are_type_local_and_natural_operators_are_checked() {
    let text =
        source("quant<credit, { memory = n } where n := max(min(8, 6), monus(8, 2)) + 2 * 3>");
    let typed = elaborate_source(&parse_source_program(&text).unwrap()).unwrap();
    assert_eq!(
        typed.declarations[2].quant_index_types[0].bindings["n"].expression(),
        &IndexExpression::Literal(12)
    );
    let parsed = parse_source_program("quant axis slots : credit<Count<Slot>>; extern fn f(a: quant<n where n := 5>, b: quant<n where n := 7>) -> unit;").unwrap();
    let typed = elaborate_source(&parsed).unwrap();
    assert_eq!(
        typed.declarations[1].quant_index_types[0].bindings["n"].expression(),
        &IndexExpression::Literal(5)
    );
    assert_eq!(
        typed.declarations[1].quant_index_types[1].bindings["n"].expression(),
        &IndexExpression::Literal(7)
    );
    let leak = source("quant<n where n := 5>").replace("return 0;", "return n;");
    assert!(elaborate_source(&parse_source_program(&leak).unwrap()).is_err());
}

#[test]
fn shared_definition_dependencies_are_evaluated_without_exponential_expansion() {
    let mut bindings = vec!["n0 := 1".to_owned()];
    for n in 1..50 {
        bindings.push(format!("n{n} := n{} + n{}", n - 1, n - 1));
    }
    let text = source(&format!("quant<n49 where {}>", bindings.join(", ")));
    let typed = elaborate_source(&parse_source_program(&text).unwrap()).unwrap();
    assert_eq!(
        typed.declarations[2].quant_index_types[0].fields["memory"].expression(),
        &IndexExpression::Literal(1u128 << 49)
    );
}

#[test]
fn where_binders_cannot_shadow_visible_runtime_locals() {
    let text = "quant axis slots : credit<Count<Slot>>; fn main() -> i32 { let n: i32 = 3; let q: quant<n where n := 5>; return 0; }";
    let errors = compile_files(
        &[SourceFile::new("local.qcore", text)],
        &CompileOptions::default(),
    )
    .unwrap_err();
    assert!(
        errors.iter().any(|error| error.code == "Q016"),
        "{errors:?}"
    );
}

#[test]
fn bracket_alias_elaborates_identically_to_the_angle_form() {
    for (angle, bracket) in [
        ("quant<n where n := 5>", "quant[|n where n := 5|]"),
        (
            "quant<credit, { memory = n } where n := m + 2, m := 3, n < 6, n > 4>",
            "quant[|credit, { memory = n } where n := m + 2, m := 3, n < 6, n > 4|]",
        ),
    ] {
        let angle_typed = elaborate_source(&parse_source_program(&source(angle)).unwrap()).unwrap();
        let parsed = parse_source_program(&source(bracket)).unwrap();
        let bracket_typed = elaborate_source(&parsed).unwrap();
        let angle_quant = &angle_typed
            .declarations
            .iter()
            .find(|decl| decl.name == "f")
            .unwrap()
            .quant_index_types[0];
        let bracket_quant = &bracket_typed
            .declarations
            .iter()
            .find(|decl| decl.name == "f")
            .unwrap()
            .quant_index_types[0];
        assert_eq!(angle_quant.mode, bracket_quant.mode, "{bracket}");
        assert_eq!(angle_quant.fields, bracket_quant.fields, "{bracket}");
        assert_eq!(angle_quant.bindings, bracket_quant.bindings, "{bracket}");
        assert!(parsed.syntax_equivalent(&parse_source_program(&parsed.to_string()).unwrap()));
        assert_inert_declaration_refusal(&source(bracket), "alias.qcore");
    }
}

#[test]
fn bracket_alias_parses_in_parameter_and_return_position_with_contract_clauses() {
    // The bracket twin of providers/quirin-provider-shared/contracts/language/quantified_indices.qv,
    // exercised inline so the pinned corpus fixture stays untouched.
    let text = "module contract;\n\
        quant axis chunks : credit<Count<Chunk>>;\n\
        extern fn preserve_five_chunk_credits(\n\
            credit: quant[|n where n := 5|] use one\n\
        ) -> quant[|credit, { chunks = total } where total := base + 2, base := 3, total <= 5|]\n\
        contract {\n\
            requires caller_owns_chunk_credit;\n\
            quant normal returned_credit.chunks == old(credit.chunks);\n\
        };\n";
    let parsed = parse_source_program(text).unwrap();
    assert!(parsed.syntax_equivalent(&parse_source_program(&parsed.to_string()).unwrap()));
    let function = parsed
        .declarations
        .iter()
        .find(|decl| decl.value.name == "preserve_five_chunk_credits")
        .unwrap();
    assert_eq!(function.value.quant_index_types.len(), 2);
    let contract = function.value.function_contract.as_ref().unwrap();
    assert_eq!(contract.clauses.len(), 2);
}

#[test]
fn mixed_or_unterminated_quant_delimiters_are_refused_not_repaired() {
    for quant in [
        "quant[|n where n := 5>",
        "quant<n where n := 5|]",
        "quant[|n where n := 5",
        "quant[|credit, { memory = n } where n := 5>",
    ] {
        assert!(parse_source_program(&source(quant)).is_err(), "{quant}");
    }
    // An unterminated `[|` closed by a `|>` is a mixed pair like the four
    // above, and still fails at parse.
    assert!(parse_source_program(&source("quant[|n where n := 5|>")).is_err());

    // A bare vertical bar is still not a quant delimiter; only `[|` / `|]` are
    // admitted. **Deliberate pin update, card `PIPE-OPERATOR`:** `|` was a lex
    // error, so this used to fail inside `parse_source_program`. The card
    // claims `|` as the logical-or token, so it now lexes and parses, and the
    // refusal is raised by the frontend's operator gate at `Q038` — measured,
    // not assumed: the layer moved one step further than "the quant grammar".
    // The load-bearing half is unchanged and re-pinned here at its new layer:
    // no program containing a bare `|` checks, and the message is actionable
    // rather than `unexpected character '|'`.
    let bar = source("quant<n where n := 5> | i32");
    assert!(parse_source_program(&bar).is_ok());
    let diagnostics = compile_files(
        &[SourceFile::new("quant.qcore", &bar)],
        &CompileOptions {
            entry: None,
            extern_contracts: None,
            native_libraries: None,
            native_library_headers: None,
        },
    )
    .map(|module| module.execution_profile)
    .expect_err("a bare vertical bar must still be refused");
    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect::<Vec<_>>(),
        ["Q038"]
    );
}
