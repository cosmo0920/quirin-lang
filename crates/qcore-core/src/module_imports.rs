//! Explicit selection of bundled semantic modules. No module is implicit.
//!
//! Imports expose vocabulary; existing checkers still discharge every operation.
//! Resolution is exact and total-or-reject, with no filesystem or provider fallback.

pub const CONCURRENCY_MODULE: &str = "std.concurrency";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ModuleImports {
    concurrency: bool,
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
                _ => return Err(UnknownModule(name.to_owned())),
            }
        }
        Ok(imports)
    }

    pub fn concurrency(self) -> bool {
        self.concurrency
    }
}
