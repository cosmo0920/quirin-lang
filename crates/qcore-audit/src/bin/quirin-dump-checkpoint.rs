fn main() -> std::process::ExitCode {
    // Card RULE-SWITCH-ENTRY-POINTS: the dump records no rule set.
    qcore_c_gateway::gateway_rules::exit_if_unrecorded("quirin-dump-checkpoint");
    qcore_audit::dump_checkpoint_cli::main(std::env::args_os().skip(1).collect())
}
