//! The verifier-host binaries' path to the shared fixture writer.
//!
//! The one implementation lives in the repository's
//! `tests/support/process_fixture.rs` (card
//! `PROCESS-FIXTURES-FS-WRITE-OUTSIDE-HOST-CRATE` moved it there from this
//! file, where card `EXECUTION-QUOTA-LOAD-INDEPENDENCE` wrote it, so every
//! crate that writes an executable fixture uses the same writer). This file
//! keeps the `#[path = "support/process_fixture.rs"]` includes of this
//! directory resolving, and re-exports everything they use.

#[path = "../../../../tests/support/process_fixture.rs"]
mod shared;
#[allow(unused_imports)]
pub use shared::{READ_REQUEST, copy_executable, write_executable, write_executable_mode};
