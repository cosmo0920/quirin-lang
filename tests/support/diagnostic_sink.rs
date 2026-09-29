//! Opt a native verification harness into the maintained typed stdout sink.

use qcore_compiler::diagnostic_log_c_emit::{
    DIAGNOSTIC_STDOUT_ADAPTER_DEFINITION, diagnostic_stdout_adapter_declaration,
    diagnostic_stdout_adapter_definition_with_fixed_results,
};

#[allow(dead_code)]
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

/// [`instrument`] with the maintained sink's typed-payload arms (card
/// `W48-QR-SCALAR-TYPES-R6`), and the fixed-width result arm too when
/// `fixed_results`: for a program whose Buffers carry typed payloads.
#[allow(dead_code)]
pub fn instrument_typed(source: &str, fixed_results: bool) -> String {
    format!(
        "{}\n{source}\n{}",
        diagnostic_stdout_adapter_declaration(qcore_compiler::std_io::Level::Trace),
        qcore_compiler::diagnostic_log_c_emit::diagnostic_stdout_adapter_definition_for(
            fixed_results,
            true
        )
    )
}
