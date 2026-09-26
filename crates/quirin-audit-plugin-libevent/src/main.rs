//! `quirin-audit-libevent`: the `quirin-audit libevent` subcommand plugin.
//! See the library documentation.

use std::process::ExitCode;

fn main() -> ExitCode {
    quirin_audit_plugin::run(quirin_audit_plugin_libevent::answer)
}
