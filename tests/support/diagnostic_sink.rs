//! Opt a native verification harness into the maintained typed stdout sink.
//!
//! The sink is the maintained adapter with its fixed-width task-result arm
//! (card `W48-QR-SCALAR-TYPES-R4`). That arm fires only for a result logged
//! under a `=TYPE` name, which no program without a fixed-width worker result
//! logs, so every other harness prints exactly what it printed before.

use qcore_compiler::diagnostic_log_c_emit::{
    diagnostic_stdout_adapter_declaration, diagnostic_stdout_adapter_definition_with_fixed_results,
};

pub fn instrument(source: &str) -> String {
    format!(
        "{}\n{source}\n{}",
        diagnostic_stdout_adapter_declaration(qcore_compiler::std_io::Level::Trace),
        diagnostic_stdout_adapter_definition_with_fixed_results()
    )
}
