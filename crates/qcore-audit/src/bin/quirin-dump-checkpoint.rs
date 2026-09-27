fn main() -> std::process::ExitCode {
    qcore_audit::dump_checkpoint_cli::main(std::env::args_os().skip(1).collect())
}
