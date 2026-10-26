//! `quirin-audit-measure`: the `quirin-audit measure` subcommand plugin.
//! See the library documentation.

use std::process::ExitCode;

fn main() -> ExitCode {
    quirin_audit_plugin::run(quirin_audit_plugin_measure::answer)
}
