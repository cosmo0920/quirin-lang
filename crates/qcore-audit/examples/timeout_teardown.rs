use std::path::PathBuf;

fn main() -> Result<(), String> {
    let mut arguments = std::env::args_os().skip(1);
    let config = arguments
        .next()
        .ok_or("usage: timeout_teardown CONFIG.json")?;
    if arguments.next().is_some() {
        return Err("usage: timeout_teardown CONFIG.json".into());
    }
    qcore_audit::timeout_teardown::run_timeout_teardown_config(&PathBuf::from(config))
}
