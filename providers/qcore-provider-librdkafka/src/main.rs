use quirin_provider_shared::runtime;

fn main() {
    // The markers make the compiled-in template one contiguous, locatable blob
    // in the executable, so a host can bind an installed manifest's declarations
    // to it without executing this program.
    if let Err(error) = runtime::run_with_contracts(
        concat!(
            "<<<qcore.provider-template.v1:begin>>>\n",
            include_str!("../provider-template.json"),
            "<<<qcore.provider-template.v1:end>>>"
        ),
        concat!(
            "<<<qcore.provider-contracts.v1:begin>>>\n",
            include_str!("../contracts.json"),
            "<<<qcore.provider-contracts.v1:end>>>"
        ),
    ) {
        eprintln!("provider: {error}");
        std::process::exit(2);
    }
}
