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

//! Cards SP2-SYMBOLIC-LIBRDKAFKA-PREMISES and
//! SP3-SYMBOLIC-FLUENT-BIT-LIBC-PREMISES: the set union `|` and the
//! conjunction `&&` of a `requires resource_premise(NAME, FORMULA);` clause
//! (G05-formula, contract specification C04.18.9) are the formula's own, read
//! by the premise reader, so the frontend's operator gate (`Q038`) admits them
//! there, in a formula the reader accepts, and nowhere else in a contract
//! module. Before these cards a shipped module stating a formula failed the
//! contract corpus's compiler regression; SP1's OpenSSL modules belong to a
//! user crate the corpus does not compile.

use qcore_compiler::{CompileOptions, SourceFile, compile_files};

/// SP2's declaration shape (librdkafka).
fn kafka_module(clause: &str) -> String {
    format!(
        "module contract;
extern fn rd_kafka_q_purge0(rkq: ptr<mut, u8>, do_lock: i32) -> i32
contract {{
    requires resource_contract_v1;
{clause}
    ensures normal resource_borrowed(rkq);
    ensures normal resource_borrowed(do_lock);
    ensures normal resource_return_none();
}};
"
    )
}

/// SP3's declaration shape (Fluent Bit).
fn fluent_bit_module(clause: &str) -> String {
    format!(
        "module contract;
extern fn get(ic: ptr<mut, u8>) -> ptr<mut, u8>
contract {{
    requires resource_contract_v1;
{clause}
    ensures normal resource_borrowed(ic);
    ensures normal resource_return_none();
}};
"
    )
}

fn codes(text: &str) -> Vec<String> {
    match compile_files(
        &[SourceFile::new("premise.qv", text)],
        &CompileOptions::default(),
    ) {
        Ok(_) => Vec::new(),
        Err(diagnostics) => diagnostics.into_iter().map(|d| d.code).collect(),
    }
}

#[test]
fn a_premise_formula_keeps_its_union_and_conjunction() {
    for clause in [
        "    requires resource_premise(served_queue, #op_callbacks |-> #takes(#held_on(#served)) == {});",
        "    requires resource_premise(closure, (quant all #f in #ran(#slots(#rk_conf)) \\ #loaded_registered: #f in #registered | #defaults && #f in #candidates) && quant all #f in #loaded_registered: #f in #enveloped);",
    ] {
        assert_eq!(
            codes(&kafka_module(clause)),
            Vec::<String>::new(),
            "{clause}"
        );
    }
    for clause in [
        "    requires resource_premise(p, #admitted -> #faithful && #faithful);",
        "    requires resource_premise(p, quant all #r in #result: #r in #result \\ (#bound | #stored));",
        "    requires resource_premise(p,\n        quant all #r in #result: #r in #interior(#args)\n        && #r in #result \\ (#tracked | #claimed));",
    ] {
        assert_eq!(
            codes(&fluent_bit_module(clause)),
            Vec::<String>::new(),
            "{clause}"
        );
    }
}

/// Card SP5-GLOBAL-PROSE-PREMISE-BAN (C04.18): the retired string statement
/// is refused by the contract format before the operator gate reads the
/// module: a parse refusal (`Q000`) naming `QF001`, whether or not the string
/// holds an operator.
#[test]
fn a_string_premise_is_refused_by_name_at_parse() {
    for clause in [
        "    requires resource_premise(prose, \"an op callback takes over no reference\");",
        "    requires resource_premise(prose, \"#op_callbacks | #defaults |-> #inert\");",
    ] {
        let Err(diagnostics) = compile_files(
            &[SourceFile::new("premise.qv", kafka_module(clause))],
            &CompileOptions::default(),
        ) else {
            panic!("{clause}: a string premise compiles");
        };
        assert_eq!(
            diagnostics
                .iter()
                .map(|d| d.code.as_str())
                .collect::<Vec<_>>(),
            ["Q000"],
            "{clause}"
        );
        assert!(
            diagnostics.iter().any(|d| d.message.contains("QF001")),
            "{clause}: {diagnostics:?}"
        );
    }
}

/// Card SP6-SYMBOLIC-HYPOTHESIS-ATOMS (C04.18.7): the retired string
/// hypothesis atom is refused by the contract format on the compiler route
/// too: a parse refusal (`Q000`) naming `QF002`, under any introducer.
#[test]
fn a_string_hypothesis_is_refused_by_name_at_parse() {
    for clause in [
        "    requires resource_hypothesis(prose, \"an op callback takes over no reference\");",
        "    requires resource_hypothesis(prose, \"#op_callbacks | #defaults |-> #inert\");",
        "    ensures normal resource_hypothesis(prose, \"anything\");",
    ] {
        let Err(diagnostics) = compile_files(
            &[SourceFile::new("hypothesis.qv", kafka_module(clause))],
            &CompileOptions::default(),
        ) else {
            panic!("{clause}: a string hypothesis compiles");
        };
        assert_eq!(
            diagnostics
                .iter()
                .map(|d| d.code.as_str())
                .collect::<Vec<_>>(),
            ["Q000"],
            "{clause}"
        );
        assert!(
            diagnostics.iter().any(|d| d.message.contains("QF002")),
            "{clause}: {diagnostics:?}"
        );
    }
}

/// Card SP6-GUARDED-SYMBOL-PROSE (C04.18.6): the retired string forms of the
/// guarded source heads are refused on the compiler route too: a parse
/// refusal (`Q000`) naming `QF003`, under any introducer.
#[test]
fn a_string_guarded_symbol_or_document_is_refused_by_name_at_parse() {
    for clause in [
        "    requires resource_guarded_symbol_v1(environment, environment, \"Exact environment\");",
        "    requires resource_guarded_symbol_v1(environment, environment, \"#source_corresponds()\");",
        "    ensures normal resource_guarded_symbol_v1(detach, operation, \"Admission\");",
        "    requires resource_guarded_decomposition_v1(g, \"guarded_decomposition_v1;\");",
    ] {
        let Err(diagnostics) = compile_files(
            &[SourceFile::new("guarded.qv", kafka_module(clause))],
            &CompileOptions::default(),
        ) else {
            panic!("{clause}: a string guarded clause compiles");
        };
        assert_eq!(
            diagnostics
                .iter()
                .map(|d| d.code.as_str())
                .collect::<Vec<_>>(),
            ["Q000"],
            "{clause}"
        );
        assert!(
            diagnostics.iter().any(|d| d.message.contains("QF003")),
            "{clause}: {diagnostics:?}"
        );
    }
}

#[test]
fn a_union_or_conjunction_anywhere_else_in_a_contract_still_refuses() {
    for clause in [
        "    requires resource_borrowed(rkq) && resource_borrowed(do_lock);",
        "    requires resource_borrowed(rkq) | resource_borrowed(do_lock);",
    ] {
        assert!(
            codes(&kafka_module(clause)).contains(&"Q038".to_owned()),
            "{clause}"
        );
    }
    assert!(
        codes(&fluent_bit_module(
            "    requires resource_borrowed(ic | ic);"
        ))
        .contains(&"Q038".to_owned())
    );
    // A single ampersand inside a formula is no operator of the formula: the
    // premise reader refuses the module.
    assert!(
        !codes(&kafka_module(
            "    requires resource_premise(served_queue, #op_callbacks & #registered |-> #inert);"
        ))
        .is_empty()
    );
    // The logical-or connective is no formula operator.
    assert!(
        !codes(&fluent_bit_module(
            "    requires resource_premise(p, #faithful || #admitted);"
        ))
        .is_empty()
    );
    // A formula the reader refuses rejects the module.
    assert!(
        !codes(&fluent_bit_module(
            "    requires resource_premise(p, #unknown && #faithful);"
        ))
        .is_empty()
    );
}

/// Card SP4-SYMBOLIC-LIBEVENT-PREMISES: the libevent declaration shape.
fn libevent_module(clause: &str) -> String {
    format!(
        "module contract;
extern fn destroy(mth: ptr<mut, u8>) -> i32
contract {{
    requires resource_contract_v1;
{clause}
    ensures normal resource_borrowed(mth);
    ensures normal resource_return_none();
}};
"
    )
}

/// Card SP4-SYMBOLIC-LIBEVENT-PREMISES: the libevent formulas' shapes keep
/// their `&&` and `|`.
#[test]
fn a_libevent_premise_formula_keeps_its_operators() {
    for clause in [
        "    requires resource_premise(chain,\n        quant all #m in #args: #mk_session(#m) in #live_at_use\n        && #mk_parent(#m) in #live_at_use \\ (#released | #ended));",
        "    requires resource_premise(writers, #interleaved |-> #no_writes(#singleton(#libco_params_key) | #tracked));",
        "    requires resource_premise(slots, #stated |-> #inert && #slots(#args) \\ #stated |-> #inert);",
    ] {
        assert_eq!(
            codes(&libevent_module(clause)),
            Vec::<String>::new(),
            "{clause}"
        );
    }
}

/// Card SP4-SYMBOLIC-LIBEVENT-PREMISES: outside a premise the operators keep
/// their refusal, and a malformed libevent-shaped formula is refused.
#[test]
fn a_libevent_shaped_contract_still_refuses_elsewhere() {
    for clause in [
        "    requires resource_borrowed(mth && mth);",
        "    requires resource_premise_all(a | b);",
    ] {
        assert!(
            codes(&libevent_module(clause)).contains(&"Q038".to_owned()),
            "{clause}"
        );
    }
    for clause in [
        "    requires resource_premise(p, #stated |-> #inert || #stated |-> #inert);",
        "    requires resource_premise(p, #unknown |-> #inert && #stated |-> #inert);",
        "    requires resource_premise(p, #args | #stated |-> #inert);",
        "    requires resource_premise(p, #interleaved |-> #no_writes(#tracked) && #no_resumes(#tracked));",
    ] {
        assert!(!codes(&libevent_module(clause)).is_empty(), "{clause}");
    }
}

const FLUENT_BIT_OUTER: &str =
    include_str!("../../providers/quirin-provider-fluent-bit/contracts/fluent_bit/outer.qv");
const LIBRDKAFKA_OUTER: &str =
    include_str!("../../providers/qcore-provider-librdkafka/contracts/librdkafka/outer.qv");

/// Card SP8-OUTER-CONTRACT-SYMBOLIC-STATEMENTS (C04.20): an outer contract
/// definition's statement is a C04.18.9 formula, so the operator gate admits
/// its `&&` and `|` exactly as a premise formula's: the two shipped outer
/// modules raise no operator refusal on the compiler route.
#[test]
fn an_outer_statement_keeps_its_union_and_conjunction() {
    for (name, text) in [
        ("fluent_bit/outer.qv", FLUENT_BIT_OUTER),
        ("librdkafka/outer.qv", LIBRDKAFKA_OUTER),
    ] {
        assert!(text.contains(" && ") && text.contains(" | "), "{name}");
        let codes = codes(text);
        assert!(
            !codes.iter().any(|code| code == "Q038"),
            "{name}: {codes:?}"
        );
    }
}

/// Card SP8-OUTER-CONTRACT-SYMBOLIC-STATEMENTS: the retired string statement
/// of an outer contract definition is a parse refusal (`Q000`) naming
/// `QF004` on the compiler route too.
#[test]
fn a_string_outer_statement_is_refused_by_name_at_parse() {
    for statement in [
        "\"every loaded plugin stores into no core-owned member\"",
        "\"#outer_code | #interceptors |-> #balanced\"",
    ] {
        let text = format!(
            "module contract;
extern fn load(config: ptr<mut, u8>) -> i32
contract {{
    requires outer_contract_v1;
    quant #outer [[contract]] plugin, {statement};
    quant #outer where #config in #loaded [[runtime]] when #result == 0;
}};
"
        );
        let Err(diagnostics) = compile_files(
            &[SourceFile::new("outer.qv", text)],
            &CompileOptions::default(),
        ) else {
            panic!("{statement}: a string outer statement compiles");
        };
        assert_eq!(
            diagnostics
                .iter()
                .map(|d| d.code.as_str())
                .collect::<Vec<_>>(),
            ["Q000"],
            "{statement}"
        );
        assert!(
            diagnostics.iter().any(|d| d.message.contains("QF004")),
            "{statement}: {diagnostics:?}"
        );
    }
}
