//! Retained pointers (card **CALLEE-RETENTION-CLASS1**, stage (b); contract
//! spec §C04.14, grammar G05, gateway rule 12t).
//!
//! USER DECISION 2026-10-05 (`W48-CALLEE-DECISION-TABLE` class 1, "split by
//! shape"): a callee that keeps a pointer the caller passed — LZ4's stream
//! states keep pointers into `src`, a raw chunk keeps its input instance —
//! states it with
//!
//! ```text
//! ensures normal resource_retained_by(p, h);      // h a parameter, or `global`
//! ensures normal resource_holder_reset(h);        // h forgets what it held
//! ```
//!
//! The caller **keeps custody** of `p`'s object (its mode is
//! `resource_borrowed`), but the object must outlive the holder `h`, or the
//! holder must be reset or destroyed before the object is released. A
//! consumer that cannot establish that refuses the call: the result is
//! `Unknown` (or `Pending` with a named human-resolvable reason), never
//! `Safe`, and releasing `p` while a live holder retains it is a hazard the
//! consumer refuses, not a defect it reports.
//!
//! A declaration carrying either clause is **never** an entry of
//! [`crate::contract::ResourceContracts::functions`] (the §C06.1 precedent):
//! a consumer that knows nothing of retention sees an uncontracted callee, so
//! a retained pointer is never read as borrowed and forgotten.

use serde::{Deserialize, Serialize};

use crate::contract::ResourceFunctionContract;

/// What retains the pointer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "holder")]
pub enum RetentionHolder {
    /// The object the parameter at this index designates.
    Parameter { index: usize },
    /// Storage of static duration, or a holder no caller object denotes (a
    /// file-scope object, a queue another thread drains): it lives for the
    /// program as far as the caller can tell.
    Global,
}

/// `resource_retained_by(p, h)`: the parameter at `retained` is kept by `holder`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Retention {
    pub retained: usize,
    pub holder: RetentionHolder,
}

/// A declaration with retention clauses: its custody contract (every retained
/// pointer and every reset holder `resource_borrowed`), its retentions and
/// the parameters whose holders it resets.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RetainingContract {
    pub contract: ResourceFunctionContract,
    /// In clause order; at most one per retained parameter.
    pub retentions: Vec<Retention>,
    /// `resource_holder_reset(h)`, in clause order; at most one per parameter.
    pub resets: Vec<usize>,
}
