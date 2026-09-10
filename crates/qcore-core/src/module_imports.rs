//! Explicit selection of bundled semantic modules. No module is implicit.
//!
//! Imports expose vocabulary; existing checkers still discharge every operation.
//! Resolution is exact and total-or-reject, with no filesystem or provider fallback.

pub const CONCURRENCY_MODULE: &str = "std.concurrency";

/// Card `source.extern-call.v1`: calling a declared extern function is a
/// file-local opt-in, exactly like concurrency. The module grants the *call*
/// vocabulary only; the callee's contract still has to be selected explicitly
/// and admitted by the resource-contract v1 loader, and every custody
/// conclusion drawn through it remains a declared assumption.
pub const EXTERN_MODULE: &str = "std.extern";

/// Card `STD-MODULES`: the rendering vocabulary shared by `std.io` and
/// `std.logger`. It names how a scalar becomes the bytes of a line and nothing
/// else — it opens no stream, so importing it alone produces no output.
pub const FMT_MODULE: &str = "std.fmt";

/// Card `STD-MODULES`: writing a rendered line to the standard output stream.
pub const IO_MODULE: &str = "std.io";

/// Card `STD-MODULES`: writing a level-tagged rendered line to the standard
/// error stream, plus the `flush` sink-handoff barrier. Separate from
/// `std.io` because a diagnostic stream and a data stream are different
/// capabilities; neither grants the other.
pub const LOGGER_MODULE: &str = "std.logger";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ModuleImports {
    concurrency: bool,
    extern_calls: bool,
    fmt: bool,
    io: bool,
    logger: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnknownModule(pub String);

impl std::fmt::Display for UnknownModule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown module `{}`", self.0)
    }
}

impl std::error::Error for UnknownModule {}

impl ModuleImports {
    pub fn resolve<'a>(names: impl IntoIterator<Item = &'a str>) -> Result<Self, UnknownModule> {
        let mut imports = Self::default();
        for name in names {
            match name {
                CONCURRENCY_MODULE => imports.concurrency = true,
                EXTERN_MODULE => imports.extern_calls = true,
                FMT_MODULE => imports.fmt = true,
                IO_MODULE => imports.io = true,
                LOGGER_MODULE => imports.logger = true,
                _ => return Err(UnknownModule(name.to_owned())),
            }
        }
        Ok(imports)
    }

    pub fn concurrency(self) -> bool {
        self.concurrency
    }

    /// Whether this file may call a declared extern function at all.
    pub fn extern_calls(self) -> bool {
        self.extern_calls
    }

    /// Whether this file may name a rendering (`dec`, `hex`).
    pub fn fmt(self) -> bool {
        self.fmt
    }

    /// Whether this file may write a rendered line to standard output.
    pub fn io(self) -> bool {
        self.io
    }

    /// Whether this file may write a level-tagged line to standard error, or
    /// hand the logger's stream to its sink with `flush`.
    pub fn logger(self) -> bool {
        self.logger
    }
}
