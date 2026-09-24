//! Shared Quirin contract modules for Fluent Bit application code (input chunks, routing, hash tables, logging, memory, time).
//!
//! The `.qcore` sources under this crate's `contracts/` are canonical: the
//! corpus index `contract/corpus.json` names that directory as one of its
//! roots, and every consumer selects the modules it needs explicitly. This
//! crate is data only; it links no provider runtime and installs nothing.

/// The crate's contract-inheritance namespace (card CONTRACT-INHERIT-ADOPTION):
/// the package name without `quirin-provider-`, `-` read as `_`. A parent
/// this crate owns is written `fluent_bit::NAME`. Recorded here, checked
/// against the derivation rule by this crate's tests, and carried into
/// `providers/contract-load-path.json`, where the load path checks it again.
pub const NAMESPACE: &str = "fluent_bit";

/// The owned modules another contract module may `#include`, sorted: a
/// subset of [`CONTRACTS`]' logical paths. None yet; publishing a module is
/// a deliberate, reviewed act (card CONTRACT-INHERIT-ADOPTION).
pub const PUBLISHED: &[&str] = &[];

/// Every contract module this crate owns, as `(logical path, source text)`,
/// sorted by logical path. The logical path is relative to `contracts/`, which
/// is the name `contract/corpus.json` resolves (for example `fluent_bit/hash_table_publication.qcore`).
pub const CONTRACTS: &[(&str, &str)] = &[
    (
        "fluent_bit/hash_table_publication.qcore",
        include_str!("../contracts/fluent_bit/hash_table_publication.qcore"),
    ),
    (
        "fluent_bit/input_chunk_destroy.qcore",
        include_str!("../contracts/fluent_bit/input_chunk_destroy.qcore"),
    ),
    (
        "fluent_bit/input_chunk_destroy_direct_routes.qcore",
        include_str!("../contracts/fluent_bit/input_chunk_destroy_direct_routes.qcore"),
    ),
    (
        "fluent_bit/input_chunk_helpers.qcore",
        include_str!("../contracts/fluent_bit/input_chunk_helpers.qcore"),
    ),
    (
        "fluent_bit/input_chunk_name.qcore",
        include_str!("../contracts/fluent_bit/input_chunk_name.qcore"),
    ),
    (
        "fluent_bit/input_chunk_sizes.qcore",
        include_str!("../contracts/fluent_bit/input_chunk_sizes.qcore"),
    ),
    (
        "fluent_bit/input_chunk_total_size.qcore",
        include_str!("../contracts/fluent_bit/input_chunk_total_size.qcore"),
    ),
    (
        "fluent_bit/logging.qcore",
        include_str!("../contracts/fluent_bit/logging.qcore"),
    ),
    (
        "fluent_bit/memory.qcore",
        include_str!("../contracts/fluent_bit/memory.qcore"),
    ),
    (
        "fluent_bit/resource_memory.qcore",
        include_str!("../contracts/fluent_bit/resource_memory.qcore"),
    ),
    (
        "fluent_bit/routes_mask.qcore",
        include_str!("../contracts/fluent_bit/routes_mask.qcore"),
    ),
    (
        "fluent_bit/time.qcore",
        include_str!("../contracts/fluent_bit/time.qcore"),
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
            assert!(
                CONTRACTS.iter().any(|(name, _)| name == module),
                "{module}"
            );
        }
    }

    /// Card CONTRACT-INHERIT-ADOPTION: every `#include` this crate's contracts
    /// write names a module the libc crate publishes (the load path would
    /// refuse anything else with `QI004`/`QI005`), through the declared Cargo
    /// edge this test links, so the edge is load-bearing and not decorative.
    #[test]
    fn every_include_names_a_published_libc_module() {
        let mut includes = 0;
        for (name, text) in CONTRACTS {
            for line in text.lines() {
                let line = line.split("//").next().unwrap().trim();
                if !line.starts_with('#') {
                    continue;
                }
                let target = line
                    .strip_prefix("#include \"")
                    .and_then(|rest| rest.strip_suffix("\";"))
                    .unwrap_or_else(|| panic!("{name}: malformed directive `{line}`"));
                let (krate, module) = target.split_once(':').unwrap();
                assert_eq!(krate, "quirin-provider-libc", "{name}: {target}");
                assert!(
                    quirin_provider_libc::PUBLISHED.contains(&module),
                    "{name}: {target} is not published"
                );
                assert!(quirin_provider_libc::contract(module).is_some());
                includes += 1;
            }
        }
        assert!(includes > 0, "no contract includes a libc module");
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
