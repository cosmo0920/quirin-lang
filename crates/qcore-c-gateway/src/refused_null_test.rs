//! Card **MONKEY-VHOST-PROPERTY-CUSTODY** (gateway rule 12e): a refused null
//! test never establishes the non-null-ness a leak accusation rests on.
//!
//! Rule 10b: a null guard on a binding that does not resolve to one identity
//! (unknown, or several targets) narrows only that binding, keeps every
//! resource, and refuses (`UnknownAlias`). Before this rule the null edge's
//! exits still accused every site the binding may denote of a leak, although
//! on that edge the one fact the accusation needs (the site is non-null
//! there) is exactly what the refused test could not decide. Monkey's
//! `mk_http_error_page` (`mk_server/mk_http.c:388-412`) is the shape:
//!
//! ```c
//! if (message) temp = mk_ptr_to_buf(*message);   /* refused: temp unbound */
//! else         temp = mk_string_dup("");          /* nullable copy */
//! if (!temp) return -1;                           /* refused test: leak "at :395" */
//! ```
//!
//! On the `return -1` the copy is NULL or was never made, yet it was reported
//! as a leak (provisional Unsafe).
//!
//! The rule ([`null_edge_step`]): on the **null** edge of a refused null
//! test, each site the tested binding may denote that is **nullable** and may
//! be live is withheld (an unresolved effect, as a refused statement's
//! hand-on does), so no exit reached through that edge accuses it; the path
//! is already unmodeled and the refusal already reported, so the function is
//! `Unknown`. Nothing else changes: the non-null edge keeps every
//! obligation, a site the binding cannot denote is untouched, a non-nullable
//! site is untouched (its non-null-ness does not come from the test), and a
//! decided test (one live identity) refines exactly as before.
//!
//! The Lean model is `QCore.RefusedNullTest`
//! (`proof/QCore/RefusedNullTest.lean`); [`NULL_EDGE_VECTORS`] is shared with
//! it.

/// What the null edge of a refused null test does to one tracked site.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NullEdgeStep {
    /// The accusation, if any, does not rest on the test: unchanged.
    Keep,
    /// The site's non-null-ness on this edge would come only through the
    /// refused test: withheld (an unresolved effect, no accusation).
    Withhold,
}

impl NullEdgeStep {
    #[cfg(test)]
    pub(crate) const fn code(self) -> &'static str {
        match self {
            Self::Keep => "keep",
            Self::Withhold => "withhold",
        }
    }
}

/// The decision for one site on the null edge of a refused null test
/// (`QCore.RefusedNullTest.nullEdgeStep`): `denoted`, the tested binding may
/// denote it; `nullable`, the site may be NULL; `possibly_live`, its status
/// has the live bit.
pub(crate) const fn null_edge_step(
    denoted: bool,
    nullable: bool,
    possibly_live: bool,
) -> NullEdgeStep {
    if denoted && nullable && possibly_live {
        NullEdgeStep::Withhold
    } else {
        NullEdgeStep::Keep
    }
}

/// `(denoted, nullable, possibly_live)` and the step, shared with
/// `QCore.RefusedNullTest.nullEdgeVectors`.
#[cfg(test)]
pub(crate) const NULL_EDGE_VECTORS: &[((bool, bool, bool), &str)] = &[
    ((false, false, false), "keep"),
    ((false, false, true), "keep"),
    ((false, true, false), "keep"),
    ((false, true, true), "keep"),
    ((true, false, false), "keep"),
    ((true, false, true), "keep"),
    ((true, true, false), "keep"),
    ((true, true, true), "withhold"),
];

#[cfg(test)]
#[path = "../tests/unit/refused_null_test.rs"]
mod tests;
