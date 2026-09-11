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

/// Card `UNSAFE-LINKAGE`: declaring an extern function that has **no** contract
/// at all, so that an external library can be linked without one. It grants the
/// *declaration* only; the call still needs [`EXTERN_MODULE`], so an
/// unchecked-linkage file names both modules. Holding this module changes
/// nothing about a contracted callee, and holding [`EXTERN_MODULE`] alone keeps
/// an uncontracted callee refused.
pub const UNCHECKED_EXTERN_MODULE: &str = "std.extern.unsafe";

/// Card `STD-MODULES`: the rendering vocabulary shared by `std.io` and
/// `std.logger`. It names how a scalar becomes the bytes of a line and nothing
/// else — it opens no stream, so importing it alone produces no output.
pub const FMT_MODULE: &str = "std.fmt";

/// Card `STD-MODULES`: writing a rendered line to the standard output stream.
pub const IO_MODULE: &str = "std.io";

/// Card `STD-EVENT`: the checked event vocabulary — one reactor context, a
/// registration on a program-created endpoint, a declared dispatch turn and
/// teardown — over the existing `event_reactor`/`os_endpoint` models
/// (`source.event.v1`, `docs/design/STD_EVENT_V1.md`).
///
/// A file-local opt-in exactly like `std.concurrency`: the context is
/// unavailable by default and importing another compilation input never grants
/// this file the vocabulary. The module grants no threading — v1's reactor
/// profile is single-threaded and a cross-thread activation is refused by the
/// reactor model itself.
pub const EVENT_MODULE: &str = "std.event";

/// Card `STD-MODULES`: writing a level-tagged rendered line to the standard
/// error stream, plus the `flush` sink-handoff barrier. Separate from
/// `std.io` because a diagnostic stream and a data stream are different
/// capabilities; neither grants the other.
pub const LOGGER_MODULE: &str = "std.logger";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ModuleImports {
    concurrency: bool,
    extern_calls: bool,
    unchecked_extern_calls: bool,
    fmt: bool,
    io: bool,
    logger: bool,
    event: bool,
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
                UNCHECKED_EXTERN_MODULE => imports.unchecked_extern_calls = true,
                FMT_MODULE => imports.fmt = true,
                IO_MODULE => imports.io = true,
                LOGGER_MODULE => imports.logger = true,
                EVENT_MODULE => imports.event = true,
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

    /// Whether this file may declare an extern function with no contract at
    /// all (card `UNSAFE-LINKAGE`). Never granted by [`EXTERN_MODULE`].
    pub fn unchecked_extern_calls(self) -> bool {
        self.unchecked_extern_calls
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

    /// Whether this file may open an event context and register interest on a
    /// descriptor (card `STD-EVENT`).
    pub fn event(self) -> bool {
        self.event
    }
}
