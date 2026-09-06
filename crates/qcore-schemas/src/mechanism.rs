//! Generic library-mechanism vocabulary.
//!
//! Moved verbatim from `qcore-compiler`'s `external_integration` module (W0
//! of `docs/GENERALIZATION_PLAN.md`). The per-library `permits` tables stay
//! in the compiler crate pending their W2 replacement by data profiles.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LibraryMechanismSummary {
    ReferenceCount,
    IntrusiveList,
    TypedQueue,
    Arena,
    CascadingDestructor,
    AtomicPublication,
    ThreadHandoff,
    Reclamation,
}
