#[path = "../../common/runtime.rs"]
mod runtime;

fn main() {
    if let Err(error) = runtime::run(include_str!("../provider-template.json")) {
        eprintln!("provider: {error}");
        std::process::exit(2);
    }
}
