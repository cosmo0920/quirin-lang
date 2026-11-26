// SPDX-License-Identifier: Apache-2.0

use std::collections::BTreeMap;

use qcore_compiler::{
    CompileOptions, CompiledModule, FileDiagnostic, SourceExecutionProfile, SourceFile,
    compile_files, extern_call::ExternAssumption,
};

#[path = "../../tests/support/windows_c.rs"]
mod native_c;

const SOURCE: &str = include_str!("fixtures/quant_variant/delayed.qr");
const CONTRACT: &str = include_str!("fixtures/quant_variant/take.qv");
const CALLER: &str = include_str!("fixtures/quant_variant/caller.c");

fn compile(source: &str, contract: Option<&str>) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    let extern_contracts = contract.map(|text| BTreeMap::from([("take.qv".into(), text.into())]));
    compile_files(
        &[SourceFile::new("delayed.qr", source)],
        &CompileOptions {
            extern_contracts,
            ..Default::default()
        },
    )
}

fn refuses(source: &str, contract: Option<&str>, code: &str, reason: &str) {
    let diagnostics = compile(source, contract).expect_err("source must be refused");
    assert!(
        diagnostics
            .iter()
            .any(|d| d.code == code && d.message.contains(reason)),
        "expected {code} containing {reason:?}, got {diagnostics:?}"
    );
}

#[test]
fn the_caller_and_delayed_match_agree_on_identity_and_the_exact_return_code() {
    let module = compile(SOURCE, Some(CONTRACT)).unwrap();
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::QuantVariantV1
    );
    assert!(
        module
            .declared_assumptions()
            .contains(&ExternAssumption::DeclaredContract("take".into()))
    );
    let emitted = module.c_source.expect("the typed library must emit C");
    // Only generated releases use the observing allocator. The external
    // callee and native caller are compiled as another translation unit.
    let instrumented = format!(
        "#include <stdlib.h>\nvoid observed_free(void *);\n#define free observed_free\n{emitted}"
    );
    let run = native_c::run_with_sources(&instrumented, None, &[], &[("caller.c", CALLER)]);
    assert!(
        run.status.success(),
        "native custody or payload mismatch: {run:?}"
    );
}

#[test]
fn double_open_and_alias_release_while_sealed_are_rejected() {
    let double_open = SOURCE.replace("return 0;", "match result {} return 0;");
    refuses(
        &double_open,
        Some(CONTRACT),
        "Q020",
        "use after move or opening of result",
    );
    let alias_release = SOURCE.replace("drop other;", "drop alias; drop other;");
    refuses(&alias_release, Some(CONTRACT), "Q020", "custody is sealed");
    let reuse_moved = SOURCE.replace("move packed)", "move receipt)");
    refuses(
        &reuse_moved,
        Some(CONTRACT),
        "Q020",
        "use after move or opening of receipt",
    );
    let wrong_generation = SOURCE
        .replace("drop other;", "")
        .replace("finish(alias, move packed)", "finish(other, move packed)");
    refuses(
        &wrong_generation,
        Some(CONTRACT),
        "Q020",
        "different generation",
    );
}

#[test]
fn the_adapter_requires_the_selected_abi_and_all_declared_dispositions() {
    refuses(SOURCE, None, "Q023", "selection");
    let wrong_abi = CONTRACT.replace("-> i32", "-> i64");
    refuses(
        SOURCE,
        Some(&wrong_abi),
        "Q027",
        "differ from the selected contract",
    );
    let reversed = CONTRACT.replace("eq, 0", "eq, 1").replace("ne, 0", "ne, 1");
    refuses(
        SOURCE,
        Some(&reversed),
        "Q027",
        "differ from the selected contract",
    );
}

#[test]
fn source_calls_cannot_cycle_while_transferring_custody() {
    let self_call = "fn again(p: own<Buffer>) -> i32 { return again(move p); }";
    let mutual = "fn next(p: own<Buffer>) -> i32 { return back(move p); }
                  fn back(q: own<Buffer>) -> i32 { return next(move q); }";
    for cycle in [self_call, mutual] {
        refuses(
            &format!("{SOURCE}\n{cycle}"),
            Some(CONTRACT),
            "Q020",
            "recursive quant variant call graph",
        );
    }
}
