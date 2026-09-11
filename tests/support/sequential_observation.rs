//! Explicit diagnostic sink for native sequential tests with no owned result.
//!
//! Production emission stays quiet. These tests opt into the maintained typed
//! stdout adapter to compare actual native custody events and outcomes with the
//! reference interpreter, without changing the source fixture or its effects.

#[path = "diagnostic_sink.rs"]
mod diagnostic_sink;
pub use diagnostic_sink::instrument;
use qcore_core::{CEmitError, ErrorAbi, ExecutionObservation, parse_c_observation};

pub fn parse(stdout: &str, abi: &ErrorAbi) -> Result<ExecutionObservation, CEmitError> {
    // The live sink writes the outcome after the events. The historical
    // observation format requires it first; preserve every event in order and
    // let the strict parser reject missing, extra or malformed records.
    let mut lines: Vec<_> = stdout.lines().collect();
    let outcome = lines.pop().ok_or(CEmitError::MalformedOutput)?;
    let observation = std::iter::once(outcome)
        .chain(lines)
        .collect::<Vec<_>>()
        .join("\n");
    parse_c_observation(&observation, abi)
}
