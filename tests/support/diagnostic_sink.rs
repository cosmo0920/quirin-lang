//! Opt a native verification harness into the maintained typed stdout sink.

use qcore_compiler::diagnostic_log_c_emit::{
    DIAGNOSTIC_STDOUT_ADAPTER_DEFINITION, diagnostic_stdout_adapter_declaration,
    diagnostic_stdout_adapter_definition_with_fixed_results,
};

pub fn instrument(source: &str) -> String {
    format!(
        "{}\n{source}\n{DIAGNOSTIC_STDOUT_ADAPTER_DEFINITION}",
        diagnostic_stdout_adapter_declaration(qcore_compiler::std_io::Level::Trace)
    )
}

/// [`instrument`] with the maintained sink's fixed-width task-result arm
/// (card `W48-QR-SCALAR-TYPES-R4`), for a program whose workers return a
/// fixed-width value. [`instrument`] keeps the historical adapter's bytes,
/// which the F02 evidence pins.
#[allow(dead_code)]
pub fn instrument_fixed(source: &str) -> String {
    format!(
        "{}\n{source}\n{}",
        diagnostic_stdout_adapter_declaration(qcore_compiler::std_io::Level::Trace),
        diagnostic_stdout_adapter_definition_with_fixed_results()
    )
}
