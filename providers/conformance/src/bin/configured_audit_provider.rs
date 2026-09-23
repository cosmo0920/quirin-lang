//! Schemas-only fixture accepting concrete revisions pinned by the audit host.
use quirin_provider_shared::{runtime, source_effect};

fn main() {
    if let Err(error) = runtime::run_with_v2(
        concat!(
            "<<<qcore.provider-template.v1:begin>>>\n",
            include_str!(
                "../../tests/fixtures/configured_boolean_source/audit-provider-template.json"
            ),
            "<<<qcore.provider-template.v1:end>>>"
        ),
        Some(source_effect::propose),
    ) {
        eprintln!("configured audit provider: {error}");
        std::process::exit(2);
    }
}
