//! Shared Quirin contract modules for Fluent Bit application code (input chunks, routing, hash tables, logging, memory, strings, time).
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
/// is the name `contract/corpus.json` resolves (for example `fluent_bit/hash_table_publication.qv`).
pub const CONTRACTS: &[(&str, &str)] = &[
    (
        "fluent_bit/hash_table_publication.qv",
        include_str!("../contracts/fluent_bit/hash_table_publication.qv"),
    ),
    (
        "fluent_bit/input_chunk_destroy.qv",
        include_str!("../contracts/fluent_bit/input_chunk_destroy.qv"),
    ),
    (
        "fluent_bit/input_chunk_destroy_direct_routes.qv",
        include_str!("../contracts/fluent_bit/input_chunk_destroy_direct_routes.qv"),
    ),
    (
        "fluent_bit/input_chunk_helpers.qv",
        include_str!("../contracts/fluent_bit/input_chunk_helpers.qv"),
    ),
    (
        "fluent_bit/input_chunk_is_up.qv",
        include_str!("../contracts/fluent_bit/input_chunk_is_up.qv"),
    ),
    (
        "fluent_bit/input_chunk_name.qv",
        include_str!("../contracts/fluent_bit/input_chunk_name.qv"),
    ),
    (
        "fluent_bit/input_chunk_sizes.qv",
        include_str!("../contracts/fluent_bit/input_chunk_sizes.qv"),
    ),
    (
        "fluent_bit/input_chunk_total_size.qv",
        include_str!("../contracts/fluent_bit/input_chunk_total_size.qv"),
    ),
    (
        "fluent_bit/input_collector_registration.qv",
        include_str!("../contracts/fluent_bit/input_collector_registration.qv"),
    ),
    (
        "fluent_bit/input_instance.qv",
        include_str!("../contracts/fluent_bit/input_instance.qv"),
    ),
    (
        "fluent_bit/lib_engine_thread.qv",
        include_str!("../contracts/fluent_bit/lib_engine_thread.qv"),
    ),
    (
        "fluent_bit/logging.qv",
        include_str!("../contracts/fluent_bit/logging.qv"),
    ),
    (
        "fluent_bit/loop_owner.qv",
        include_str!("../contracts/fluent_bit/loop_owner.qv"),
    ),
    (
        "fluent_bit/memory.qv",
        include_str!("../contracts/fluent_bit/memory.qv"),
    ),
    (
        "fluent_bit/null_deref.qv",
        include_str!("../contracts/fluent_bit/null_deref.qv"),
    ),
    (
        "fluent_bit/outer.qv",
        include_str!("../contracts/fluent_bit/outer.qv"),
    ),
    (
        "fluent_bit/output_flush_id.qv",
        include_str!("../contracts/fluent_bit/output_flush_id.qv"),
    ),
    (
        "fluent_bit/output_singleplex.qv",
        include_str!("../contracts/fluent_bit/output_singleplex.qv"),
    ),
    (
        "fluent_bit/resource_memory.qv",
        include_str!("../contracts/fluent_bit/resource_memory.qv"),
    ),
    (
        "fluent_bit/resource_string.qv",
        include_str!("../contracts/fluent_bit/resource_string.qv"),
    ),
    (
        "fluent_bit/result_range.qv",
        include_str!("../contracts/fluent_bit/result_range.qv"),
    ),
    (
        "fluent_bit/routes_mask.qv",
        include_str!("../contracts/fluent_bit/routes_mask.qv"),
    ),
    (
        "fluent_bit/storage_backlog.qv",
        include_str!("../contracts/fluent_bit/storage_backlog.qv"),
    ),
    (
        "fluent_bit/task_route_queries.qv",
        include_str!("../contracts/fluent_bit/task_route_queries.qv"),
    ),
    (
        "fluent_bit/task_users.qv",
        include_str!("../contracts/fluent_bit/task_users.qv"),
    ),
    (
        "fluent_bit/teardown_destroy.qv",
        include_str!("../contracts/fluent_bit/teardown_destroy.qv"),
    ),
    (
        "fluent_bit/time.qv",
        include_str!("../contracts/fluent_bit/time.qv"),
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
#[path = "../../../tests/support/tracked_files.rs"]
mod tracked_files;

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
        // Card W45-TEST-HYGIENE: the tracked contract tree only; an
        // untracked or ignored file under `contracts/` is not a module.
        for path in super::tracked_files::tracked_files(&root) {
            assert!(!path.is_symlink(), "symbolic link: {}", path.display());
            let name = path.strip_prefix(&root).unwrap().to_str().unwrap();
            on_disk.push((
                name.replace('\\', "/"),
                std::fs::read_to_string(&path).unwrap(),
            ));
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
