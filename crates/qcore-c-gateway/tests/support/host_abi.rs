//! Explicit host declarations for fixtures read by the lexical ABI checker.
//! Unlike Clang's AST, that reader does not expand `__SIZE_TYPE__`.

#[cfg(all(windows, target_pointer_width = "64"))]
macro_rules! size_t_declaration {
    () => {
        "typedef unsigned long long size_t;\n"
    };
}

#[cfg(all(not(windows), target_pointer_width = "64"))]
macro_rules! size_t_declaration {
    () => {
        "typedef unsigned long size_t;\n"
    };
}

#[cfg(target_pointer_width = "32")]
macro_rules! size_t_declaration {
    () => {
        "typedef unsigned int size_t;\n"
    };
}

pub(crate) use size_t_declaration;
