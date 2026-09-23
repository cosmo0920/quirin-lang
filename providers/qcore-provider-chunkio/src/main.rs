use qcore_schemas::plugin::{PluginManifest, VerificationRequestV2, VerificationResponseV2};
use quirin_provider_shared::{
    call_use_effect, cleanup_replay_effect, composition_effect, counter_effect, runtime,
    source_effect, teardown_effect,
};

/// Each capability inspects the request itself and declines what it does not
/// implement; a declined request falls through to the runtime's Unknown reply.
fn propose(
    request: &VerificationRequestV2,
    manifest: &PluginManifest,
) -> Option<VerificationResponseV2> {
    source_effect::propose(request, manifest)
        .or_else(|| counter_effect::propose(request, manifest))
        .or_else(|| teardown_effect::propose(request, manifest))
        .or_else(|| call_use_effect::propose(request, manifest))
        .or_else(|| cleanup_replay_effect::propose(request, manifest))
        .or_else(|| composition_effect::propose(request, manifest))
}

fn main() {
    if let Err(error) =
        // The markers make the compiled-in template one contiguous, locatable
        // blob in the executable, so a host can bind an installed manifest's
        // declarations to it without executing this program.
        runtime::run_with_v2_and_contracts(
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
            Some(propose),
        )
    {
        eprintln!("provider: {error}");
        std::process::exit(2);
    }
}
