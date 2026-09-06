//! Compatibility facade for the extracted verifier host.
//!
//! Generic registry, transport, firewall and composition live in
//! `qcore-verifier-host`; configured first-party adapters live in `first_party`.

pub use crate::first_party::{
    first_party_manifests, first_party_registry, load_first_party_manifests,
    load_first_party_registry,
};
pub use qcore_verifier_host::*;
