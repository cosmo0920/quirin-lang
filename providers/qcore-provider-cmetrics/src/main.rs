use quirin_provider_shared::{runtime, source_effect};

fn main() {
    if let Err(error) = runtime::run_with_v2_and_contracts(
        // The markers make the compiled-in template one contiguous, locatable
        // blob in the executable, so a host can bind an installed manifest's
        // declarations to it without executing this program.
        concat!(
            "<<<qcore.provider-template.v1:begin>>>\n",
            include_str!("../provider-template.json"),
            "<<<qcore.provider-template.v1:end>>>"
        ),
        Some(concat!(
            "<<<qcore.provider-contracts.v1:begin>>>\n",
            include_str!("../contracts.json"),
            "<<<qcore.provider-contracts.v1:end>>>"
        )),
        Some(source_effect::propose),
    ) {
        eprintln!("provider: {error}");
        std::process::exit(2);
    }
}
