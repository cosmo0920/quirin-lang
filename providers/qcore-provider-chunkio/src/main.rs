#[path = "../../common/runtime.rs"]
mod runtime;

#[path = "../../common/source_effect.rs"]
mod source_effect;

fn main() {
    if let Err(error) = runtime::run_with_v2(
        include_str!("../provider-template.json"),
        Some(source_effect::propose),
    ) {
        eprintln!("provider: {error}");
        std::process::exit(2);
    }
}
