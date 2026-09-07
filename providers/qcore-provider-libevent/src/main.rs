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
        runtime::run_with_v2(include_str!("../provider-template.json"), Some(propose))
    {
        eprintln!("provider: {error}");
        std::process::exit(2);
    }
}
