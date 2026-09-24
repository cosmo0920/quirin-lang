//! Shared Quirin contract modules for the C standard library and POSIX: `errno`, heap
//! memory, `pthread_mutex`, and (card W43-LIBC-CONTRACTS) `<dlfcn.h>`, `<math.h>`,
//! process/regex/rlimit helpers, sockets, `<stdio.h>`, `<stdlib.h>`, `<string.h>`,
//! `<sys/stat.h>`, threads and signals, `<time.h>` and `<unistd.h>`, and (card
//! W43-LIBC-RESTRICT-PROTOTYPES) `<regex.h>`.
//!
//! The `.qcore` sources under this crate's `contracts/` are canonical: the
//! corpus index `contract/corpus.json` names that directory as one of its
//! roots, and every consumer selects the modules it needs explicitly. This
//! crate is data only; it links no provider runtime and installs nothing.
//!
//! This is the only home for libc contracts: a new libc module goes under
//! `contracts/libc/` here, with its entry in [`CONTRACTS`], and never under
//! `fluent_bit/` or another corpus module. The crate has no dependency and is
//! usable on its own, without the `quirin-provider-shared` bundle.

/// The crate's contract-inheritance namespace (card CONTRACT-INHERIT-ADOPTION):
/// the package name without `quirin-provider-`, `-` read as `_`. A parent
/// this crate owns is written `libc::NAME`. Recorded here, checked
/// against the derivation rule by this crate's tests, and carried into
/// `providers/contract-load-path.json`, where the load path checks it again.
pub const NAMESPACE: &str = "libc";

/// The owned modules another contract module may `#include`, sorted: a
/// subset of [`CONTRACTS`]' logical paths. `libc/memory.qcore` (`malloc`,
/// `calloc`, `free`) and `libc/string.qcore` (`strdup`, `strndup`) are
/// published: `quirin-provider-fluent-bit` inherits its allocator and
/// string-duplication wrappers from them. Publication makes a module's text
/// part of every inheriting contract's identity, so a published module's
/// SHA-256 is pinned in `providers/contract-load-path.json`.
pub const PUBLISHED: &[&str] = &["libc/memory.qcore", "libc/string.qcore"];

/// Every contract module this crate owns, as `(logical path, source text)`,
/// sorted by logical path. The logical path is relative to `contracts/`, which
/// is the name `contract/corpus.json` resolves (for example `libc/errno.qcore`).
pub const CONTRACTS: &[(&str, &str)] = &[
    (
        "libc/dlfcn.qcore",
        include_str!("../contracts/libc/dlfcn.qcore"),
    ),
    (
        "libc/errno.qcore",
        include_str!("../contracts/libc/errno.qcore"),
    ),
    (
        "libc/math.qcore",
        include_str!("../contracts/libc/math.qcore"),
    ),
    (
        "libc/memory.qcore",
        include_str!("../contracts/libc/memory.qcore"),
    ),
    (
        "libc/process.qcore",
        include_str!("../contracts/libc/process.qcore"),
    ),
    (
        "libc/pthread_mutex.qcore",
        include_str!("../contracts/libc/pthread_mutex.qcore"),
    ),
    (
        "libc/regex.qcore",
        include_str!("../contracts/libc/regex.qcore"),
    ),
    (
        "libc/socket.qcore",
        include_str!("../contracts/libc/socket.qcore"),
    ),
    (
        "libc/stdio.qcore",
        include_str!("../contracts/libc/stdio.qcore"),
    ),
    (
        "libc/stdlib.qcore",
        include_str!("../contracts/libc/stdlib.qcore"),
    ),
    (
        "libc/string.qcore",
        include_str!("../contracts/libc/string.qcore"),
    ),
    (
        "libc/sys_stat.qcore",
        include_str!("../contracts/libc/sys_stat.qcore"),
    ),
    (
        "libc/threads.qcore",
        include_str!("../contracts/libc/threads.qcore"),
    ),
    (
        "libc/time.qcore",
        include_str!("../contracts/libc/time.qcore"),
    ),
    (
        "libc/unistd.qcore",
        include_str!("../contracts/libc/unistd.qcore"),
    ),
];

/// The source text of one owned module, by logical path.
pub fn contract(path: &str) -> Option<&'static str> {
    CONTRACTS
        .iter()
        .find(|(name, _)| *name == path)
        .map(|(_, text)| *text)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Card CONTRACT-INHERIT-ADOPTION: the recorded namespace is the one the
    /// load path derives from the package name, and every published module
    /// is an owned one, listed once, in order.
    #[test]
    fn namespace_is_derived_and_published_modules_are_owned() {
        let derived = env!("CARGO_PKG_NAME")
            .strip_prefix("quirin-provider-")
            .expect("a contract crate is named quirin-provider-NAME")
            .replace('-', "_");
        assert_eq!(NAMESPACE, derived);
        assert!(PUBLISHED.windows(2).all(|pair| pair[0] < pair[1]));
        for module in PUBLISHED {
            assert!(CONTRACTS.iter().any(|(name, _)| name == module), "{module}");
        }
    }

    /// The index is exactly the `.qcore` tree on disk, byte for byte: a module
    /// added under `contracts/` without an index entry, or an entry without its
    /// file, fails here rather than being silently omitted.
    #[test]
    fn index_is_exactly_the_contract_tree() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("contracts");
        let mut on_disk = Vec::new();
        let mut pending = vec![root.clone()];
        while let Some(directory) = pending.pop() {
            for entry in std::fs::read_dir(&directory).unwrap() {
                let path = entry.unwrap().path();
                assert!(!path.is_symlink(), "symbolic link: {}", path.display());
                if path.is_dir() {
                    pending.push(path);
                } else {
                    let name = path.strip_prefix(&root).unwrap().to_str().unwrap();
                    let name = name.replace('\\', "/");
                    on_disk.push((name, std::fs::read_to_string(&path).unwrap()));
                }
            }
        }
        on_disk.sort();
        let indexed: Vec<_> = CONTRACTS
            .iter()
            .map(|(name, text)| (name.to_string(), text.to_string()))
            .collect();
        assert_eq!(indexed, on_disk);
        assert_eq!(contract(CONTRACTS[0].0), Some(CONTRACTS[0].1));
        assert_eq!(contract("absent.qcore"), None);
    }
}
