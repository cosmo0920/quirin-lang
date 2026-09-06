//! Source identity and pinned-revision records.
//!
//! Moved verbatim from `qcore-compiler`'s `external_source` module (W0 of
//! `docs/GENERALIZATION_PLAN.md`). The pinned per-library path tables and
//! the profile derivation stay in the compiler crate pending W2.
//! Unit/build identity for verification requests lives in
//! [`crate::plugin::UnitIdentity`].

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalSourceBundle {
    pub source_revision: String,
    pub files: BTreeMap<String, String>,
}
