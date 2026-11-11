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

use qcore_contract::contract_observation::{
    ContractFact, check_contract_facts, check_typed_contract_facts, contract_fact_literal,
};

const POLICY: &str = r#"
extern fn source_identity_policy() -> unit contract {
    requires enabled == 1;
    ensures normal source_path == "src/subject.c";
    ensures normal source_hash == "9f86d081" and source_path != "src/other.c";
    ensures normal revision == 7;
};
"#;

fn accepted_facts() -> Vec<(&'static str, ContractFact)> {
    vec![
        ("enabled", ContractFact::Integer(1)),
        ("source_path", ContractFact::Text("src/subject.c".into())),
        ("source_hash", ContractFact::Text("9f86d081".into())),
        ("revision", ContractFact::Integer(7)),
    ]
}

#[test]
fn string_literals_use_the_standard_qcore_escape_decoder() {
    let source = r#"extern fn p() -> unit contract {
        ensures normal json == "[\"wait\",\"destroy\"]";
        ensures normal line == "first\nsecond";
    };"#;
    assert!(
        check_typed_contract_facts(
            source,
            "p",
            &[
                ("json", ContractFact::Text("[\"wait\",\"destroy\"]".into())),
                ("line", ContractFact::Text("first\nsecond".into())),
            ],
        )
        .is_ok()
    );
    let unsupported = r#"extern fn p() -> unit contract { ensures normal text == "\q"; };"#;
    assert!(
        check_typed_contract_facts(
            unsupported,
            "p",
            &[("text", ContractFact::Text("q".into()))],
        )
        .is_err()
    );
}

#[test]
fn literal_accessor_requires_one_exact_unconditional_clause() {
    let source = r#"extern fn identity() -> unit contract {
        ensures normal revision == -7;
        ensures normal callbacks == "[\"wait\",\"destroy\"]";
        ensures normal enabled == 1;
    };"#;
    assert_eq!(
        contract_fact_literal(source, "identity", "revision"),
        Ok(ContractFact::Integer(-7))
    );
    assert_eq!(
        contract_fact_literal(source, "identity", "callbacks"),
        Ok(ContractFact::Text("[\"wait\",\"destroy\"]".into()))
    );
    assert!(contract_fact_literal(source, "identity", "missing").is_err());

    let ambiguous = source.replace(
        "ensures normal enabled == 1;",
        "ensures normal enabled == 1; ensures normal enabled == 1;",
    );
    assert!(contract_fact_literal(&ambiguous, "identity", "enabled").is_err());
    let conditional_shape = source.replace(
        "ensures normal enabled == 1;",
        "ensures normal enabled == 1 and revision == -7;",
    );
    assert!(contract_fact_literal(&conditional_shape, "identity", "enabled").is_err());
}

#[test]
fn text_and_integer_facts_check_without_coercion() {
    assert!(
        check_typed_contract_facts(POLICY, "source_identity_policy", &accepted_facts()).is_ok()
    );

    for (name, replacement) in [
        ("source_path", ContractFact::Text("src/mutated.c".into())),
        ("source_hash", ContractFact::Text("00000000".into())),
        ("revision", ContractFact::Integer(8)),
    ] {
        let mut facts = accepted_facts();
        facts.iter_mut().find(|(fact, _)| *fact == name).unwrap().1 = replacement;
        assert!(
            check_typed_contract_facts(POLICY, "source_identity_policy", &facts).is_err(),
            "mutation of {name} was accepted"
        );
    }

    let changed_literal = POLICY.replace("9f86d081", "00000000");
    assert!(
        check_typed_contract_facts(
            &changed_literal,
            "source_identity_policy",
            &accepted_facts()
        )
        .is_err()
    );
}

#[test]
fn duplicate_missing_and_wrongly_typed_facts_refuse() {
    let duplicate = [
        ("value", ContractFact::Integer(1)),
        ("value", ContractFact::Integer(1)),
    ];
    assert!(
        check_typed_contract_facts(
            "extern fn p() -> unit contract { ensures normal value == 1; };",
            "p",
            &duplicate,
        )
        .unwrap_err()
        .contains("duplicate")
    );

    for expression in [
        "false and missing == 1",
        "true or missing == 1",
        "text == 1",
        "text < \"z\"",
        "text + 1 == 2",
        "text",
    ] {
        let source = format!("extern fn p() -> unit contract {{ ensures normal {expression}; }};");
        assert!(
            check_typed_contract_facts(&source, "p", &[("text", ContractFact::Text("a".into()))],)
                .is_err(),
            "accepted {expression}"
        );
    }
}

#[test]
fn strict_policy_declaration_shape_refuses_every_relaxation() {
    assert!(
        check_typed_contract_facts(
            "extern fn p() -> unit /* trivia is not syntax */ contract { ensures normal true; };",
            "p",
            &[],
        )
        .is_ok()
    );
    for source in [
        "fn p() -> unit contract { ensures normal true; } {}",
        "unsafe extern fn p() -> unit contract { ensures normal true; };",
        "extern fn p() -> unit unsafe contract { ensures normal true; };",
        "extern fn p<T>() -> unit contract { ensures normal true; };",
        "extern fn p(value: i32) -> unit contract { ensures normal true; };",
        "extern fn p() -> i32 contract { ensures normal true; };",
        "extern fn p() -> unit effects { unsafe } contract { ensures normal true; };",
        "extern fn p() -> unit contract { requires true; };",
        "extern fn p() -> unit contract { ensures error Failure true; };",
        "extern fn p() -> unit contract { ensures any true; };",
    ] {
        assert!(
            check_typed_contract_facts(source, "p", &[]).is_err(),
            "accepted {source}"
        );
    }
}

#[test]
fn typed_integer_semantics_match_the_legacy_numeric_api() {
    let source = "extern fn p() -> unit contract { requires a > 0; ensures normal (a * 2 + b == 7) and not false; };";
    for facts in [
        vec![("a", 3), ("b", 1)],
        vec![("a", 3), ("b", 2)],
        vec![("a", 0), ("b", 7)],
    ] {
        let typed: Vec<_> = facts
            .iter()
            .map(|(name, value)| (*name, ContractFact::Integer(*value)))
            .collect();
        assert_eq!(
            check_typed_contract_facts(source, "p", &typed),
            check_contract_facts(source, "p", &facts)
        );
    }

    let overflow =
        "extern fn p() -> unit contract { ensures normal maximum + 1 == 0; };".to_owned();
    assert_eq!(
        check_typed_contract_facts(
            &overflow,
            "p",
            &[("maximum", ContractFact::Integer(i128::MAX))],
        ),
        check_contract_facts(&overflow, "p", &[("maximum", i128::MAX)])
    );
}

#[test]
fn typed_policy_inputs_are_bounded() {
    let source = "extern fn p() -> unit contract { ensures normal text == \"ok\"; };";
    let oversized = "x".repeat(4097);
    assert!(
        check_typed_contract_facts(source, "p", &[("text", ContractFact::Text(oversized))],)
            .is_err()
    );

    let facts: Vec<_> = (0..257)
        .map(|index| {
            (
                Box::leak(format!("f{index}").into_boxed_str()) as &str,
                ContractFact::Integer(0),
            )
        })
        .collect();
    assert!(check_typed_contract_facts(source, "p", &facts).is_err());

    let too_many_tokens = format!(
        "extern fn p() -> unit contract {{ ensures normal {}; }};",
        std::iter::repeat_n("true and", 257)
            .collect::<Vec<_>>()
            .join(" ")
            + " true"
    );
    assert!(check_typed_contract_facts(&too_many_tokens, "p", &[]).is_err());
}
