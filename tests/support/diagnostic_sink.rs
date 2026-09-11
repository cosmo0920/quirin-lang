//! Opt a native verification harness into the maintained typed stdout sink.

use qcore_compiler::diagnostic_log_c_emit::{
    DIAGNOSTIC_STDOUT_ADAPTER_DEFINITION, diagnostic_stdout_adapter_declaration,
};

pub fn instrument(source: &str) -> String {
    format!(
        "{}\n{source}\n{DIAGNOSTIC_STDOUT_ADAPTER_DEFINITION}",
        diagnostic_stdout_adapter_declaration(qcore_compiler::std_io::Level::Trace)
    )
}
