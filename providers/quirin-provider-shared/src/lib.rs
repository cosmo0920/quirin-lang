//! Shared support for the first-party executable providers, and the bundle of
//! shared Quirin contracts.
//!
//! `runtime` is the bounded protocol transport every provider binary runs; the
//! other Rust modules are the schemas-only capability evaluators a provider may
//! chain into its v2 proposal. Each provider's `main` names exactly the
//! evaluators it offers, so linking this library adds no capability to a
//! binary that does not call it.
//!
//! The crate owns the provider-owner-less policies under `contracts/`
//! (`language/`, `library/`) and re-exports the five per-library contract
//! crates, so a consumer can take the whole shared set from one crate through
//! [`CONTRACT_SETS`] and [`shared_contracts`]. Every contract crate is data only:
//! nothing here loads, selects or installs a module implicitly.

pub mod call_use_effect;
pub mod cleanup_replay_effect;
pub mod composition_effect;
pub mod counter_effect;
pub mod reactor_callbacks;
pub mod runtime;
pub mod source_effect;
pub mod teardown_effect;

pub use quirin_provider_cfl as cfl;
pub use quirin_provider_fluent_bit as fluent_bit;
pub use quirin_provider_libc as libc;
pub use quirin_provider_mpack as mpack;
pub use quirin_provider_msgpack as msgpack;

/// The crate's contract-inheritance namespace (card CONTRACT-INHERIT-ADOPTION):
/// the package name without `quirin-provider-`, `-` read as `_`. A parent
/// this crate owns is written `shared::NAME`. Recorded here, checked
/// against the derivation rule by this crate's tests, and carried into
/// `providers/contract-load-path.json`, where the load path checks it again.
pub const NAMESPACE: &str = "shared";

/// The owned modules another contract module may `#include`, sorted: a
/// subset of [`CONTRACTS`]' logical paths. None yet; publishing a module is
/// a deliberate, reviewed act (card CONTRACT-INHERIT-ADOPTION).
pub const PUBLISHED: &[&str] = &[];

/// The contract modules this crate owns itself, as `(logical path, source
/// text)`, sorted by logical path relative to `contracts/`.
pub const CONTRACTS: &[(&str, &str)] = &[
    (
        "language/quantified_indices.qcore",
        include_str!("../contracts/language/quantified_indices.qcore"),
    ),
    (
        "library/custody_evidence.qcore",
        include_str!("../contracts/library/custody_evidence.qcore"),
    ),
    (
        "library/source_projection.qcore",
        include_str!("../contracts/library/source_projection.qcore"),
    ),
];

/// The complete shared contract set: this crate's own modules followed by the
/// five per-library crates' modules.
pub const CONTRACT_SETS: &[&[(&str, &str)]] = &[
    CONTRACTS,
    cfl::CONTRACTS,
    fluent_bit::CONTRACTS,
    libc::CONTRACTS,
    mpack::CONTRACTS,
    msgpack::CONTRACTS,
];

/// Every shared module as `(logical path, source text)`, in [`CONTRACT_SETS`]
/// order.
pub fn shared_contracts() -> impl Iterator<Item = (&'static str, &'static str)> {
    CONTRACT_SETS.iter().flat_map(|set| set.iter().copied())
}

/// The source text of one shared module, by logical path.
pub fn shared_contract(path: &str) -> Option<&'static str> {
    shared_contracts()
        .find(|(name, _)| *name == path)
        .map(|(_, text)| text)
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
    use std::collections::BTreeSet;

    /// This crate's own index is exactly its `contracts/` tree, byte for byte.
    #[test]
    fn own_index_is_exactly_the_contract_tree() {
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
    }

    /// Card CONTRACT-INHERIT-ADOPTION: the generated load-path manifest
    /// (`providers/contract-load-path.json`, which core consumers read as data)
    /// says exactly what the six linked crates record: namespace, owned
    /// modules, and each published module with the SHA-256 of its linked text.
    /// Versions and dependency edges come from Cargo metadata and are checked by
    /// `providers/contracts.py check`.
    #[test]
    fn load_path_manifest_matches_the_linked_crates() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("contract-load-path.json");
        let manifest: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        let crates = manifest["crates"].as_array().unwrap();
        let linked: [(&str, &str, &[&str], &[(&str, &str)]); 6] = [
            ("quirin-provider-cfl", cfl::NAMESPACE, cfl::PUBLISHED, cfl::CONTRACTS),
            (
                "quirin-provider-fluent-bit",
                fluent_bit::NAMESPACE,
                fluent_bit::PUBLISHED,
                fluent_bit::CONTRACTS,
            ),
            ("quirin-provider-libc", libc::NAMESPACE, libc::PUBLISHED, libc::CONTRACTS),
            ("quirin-provider-mpack", mpack::NAMESPACE, mpack::PUBLISHED, mpack::CONTRACTS),
            (
                "quirin-provider-msgpack",
                msgpack::NAMESPACE,
                msgpack::PUBLISHED,
                msgpack::CONTRACTS,
            ),
            ("quirin-provider-shared", NAMESPACE, PUBLISHED, CONTRACTS),
        ];
        assert_eq!(crates.len(), linked.len());
        for (record, (name, namespace, published, contracts)) in crates.iter().zip(linked) {
            assert_eq!(record["name"], name);
            assert_eq!(record["namespace"], namespace, "{name}");
            let modules: Vec<_> = contracts.iter().map(|(module, _)| *module).collect();
            assert_eq!(record["modules"], serde_json::json!(modules), "{name}");
            let pins: serde_json::Map<String, serde_json::Value> = published
                .iter()
                .map(|module| {
                    let text = contracts.iter().find(|(m, _)| m == module).unwrap().1;
                    let text = text.replace("\r\n", "\n");
                    (
                        (*module).to_owned(),
                        qcore_schemas::sha256_hex(text.as_bytes()).into(),
                    )
                })
                .collect();
            assert_eq!(record["published"], serde_json::Value::Object(pins), "{name}");
        }
    }

    /// The bundle names each shared module exactly once, and each member
    /// crate's modules sit under its own library directory only (so, for
    /// example, no libc module can hide in the Fluent Bit set).
    #[test]
    fn bundle_is_disjoint_and_each_set_stays_in_its_library() {
        let names: Vec<_> = shared_contracts().map(|(name, _)| name).collect();
        let unique: BTreeSet<_> = names.iter().copied().collect();
        assert_eq!(unique.len(), names.len(), "duplicate logical module");
        for (set, prefixes) in [
            (CONTRACTS, &["language/", "library/"][..]),
            (cfl::CONTRACTS, &["cfl/"][..]),
            (fluent_bit::CONTRACTS, &["fluent_bit/"][..]),
            (libc::CONTRACTS, &["libc/"][..]),
            (mpack::CONTRACTS, &["mpack/"][..]),
            (msgpack::CONTRACTS, &["msgpack/"][..]),
        ] {
            assert!(!set.is_empty());
            for (name, _) in set {
                assert!(
                    prefixes.iter().any(|prefix| name.starts_with(prefix)),
                    "{name}"
                );
            }
        }
        assert_eq!(
            shared_contract("libc/memory.qcore"),
            libc::contract("libc/memory.qcore")
        );
        assert_eq!(shared_contract("absent.qcore"), None);
    }
}
