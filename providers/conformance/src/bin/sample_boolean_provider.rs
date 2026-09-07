//! Schemas-only process fixture for a separate declared source codebase.
//! The reused evaluator consumes imported statements, never expected paths.
#[path = "../../../common/runtime.rs"]
mod runtime;
#[path = "../../../common/source_effect.rs"]
mod source_effect;

fn main() {
    if let Err(error) = runtime::run_with_v2(
        include_str!("../../tests/fixtures/configured_boolean_source/provider-template.json"),
        Some(source_effect::propose),
    ) {
        eprintln!("sample provider: {error}");
        std::process::exit(2);
    }
}
