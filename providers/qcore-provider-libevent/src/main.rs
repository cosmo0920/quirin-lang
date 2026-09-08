#[path = "../../common/runtime.rs"]
mod runtime;

#[path = "../../common/reactor_callbacks.rs"]
mod reactor_callbacks;

use qcore_schemas::plugin::{PluginManifest, VerificationRequestV2, VerificationResponseV2};

/// Each capability inspects the request itself and declines what it does not
/// implement; a declined request falls through to the runtime's Unknown reply.
fn propose(
    request: &VerificationRequestV2,
    manifest: &PluginManifest,
) -> Option<VerificationResponseV2> {
    reactor_callbacks::propose(request, manifest)
}

fn main() {
    if let Err(error) =
        // The markers make the compiled-in template one contiguous, locatable
        // blob in the executable, so a host can bind an installed manifest's
        // declarations to it without executing this program.
        runtime::run_with_v2(
            concat!(
                "<<<qcore.provider-template.v1:begin>>>\n",
                include_str!("../provider-template.json"),
                "<<<qcore.provider-template.v1:end>>>"
            ),
            Some(propose),
        )
    {
        eprintln!("provider: {error}");
        std::process::exit(2);
    }
}
