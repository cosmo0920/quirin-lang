//! Schemas-only fixture accepting concrete revisions pinned by the audit host.
#[path = "../../../common/runtime.rs"]
mod runtime;
#[path = "../../../qcore-provider-chunkio/src/source_effect.rs"]
mod source_effect;

fn main() {
    if let Err(error) = runtime::run_with_v2(
        include_str!("../../tests/fixtures/configured_boolean_source/audit-provider-template.json"),
        Some(source_effect::propose),
    ) {
        eprintln!("configured audit provider: {error}");
        std::process::exit(2);
    }
}
