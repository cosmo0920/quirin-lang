//! Pinned Monkey syntax profile for the shared typed C path importer.
//! Calls remain opaque; source projection does not establish custody or safety.
use crate::external_integration::PINNED_FLUENT_BIT_REVISION;
use crate::legacy_c::C11TranslationUnit;
pub use crate::typed_source::{SourceExpression, SourceOperation, SourceRegion, SourceValue};
use crate::typed_source::{SourceImportProfile, import_source_functions};
pub use crate::typed_source::{
    SourcePath as MonkeySourcePath, SourceResult as MonkeySourceResult,
    SourceSummary as MonkeySourceSummary,
};

const CALLBACKS: [&str; 3] = ["cb_event", "cb_timeout", "cb_wait_2_timeout"];
pub(crate) const WRAPPERS: [&str; 15] = [
    "mk_event_libevent_socketpair",
    "_mk_event_init",
    "_mk_event_loop_create",
    "_mk_event_loop_destroy",
    "_mk_event_update",
    "_mk_event_add",
    "_mk_event_del",
    "_mk_event_timeout_create",
    "_mk_event_timeout_destroy",
    "_mk_event_channel_create",
    "_mk_event_channel_destroy",
    "_mk_event_inject",
    "_mk_event_wait_with_flags",
    "_mk_event_wait_2",
    "_mk_event_backend",
];

/// Import the three exact callbacks, even though mk_event.c includes their
/// definitions from its selected adapter. Revision is a caller assertion; the
/// pinned external audit separately checks Git/source/build identity.
pub fn import_monkey_callbacks(
    unit: &C11TranslationUnit<'_>,
    source_revision: &str,
) -> Result<Vec<MonkeySourceResult>, String> {
    import_functions(unit, source_revision, &CALLBACKS)
}

/// Import every selected adapter wrapper with an explicit Unknown for bodies
/// outside the bounded grammar. Calls remain opaque; returned values and every
/// imported failure guard are retained, without assuming cleanup or success.
pub fn import_monkey_wrappers(
    unit: &C11TranslationUnit<'_>,
    source_revision: &str,
) -> Result<Vec<MonkeySourceResult>, String> {
    import_functions(unit, source_revision, &WRAPPERS)
}

fn import_functions(
    unit: &C11TranslationUnit<'_>,
    source_revision: &str,
    functions: &[&str],
) -> Result<Vec<MonkeySourceResult>, String> {
    if source_revision != PINNED_FLUENT_BIT_REVISION {
        return Err("Monkey source revision does not match the pinned revision".into());
    }
    import_source_functions(
        unit,
        source_revision,
        functions,
        &SourceImportProfile {
            records: &[
                "mk_event",
                "mk_event_ctx",
                "ev_map",
                "mk_event_loop",
                "timeval",
            ],
            sizeof_types: &[
                "unsigned long",
                "unsigned long long",
                "int",
                "short",
                "struct mk_event_ctx",
                "struct mk_event",
                "struct ev_map",
            ],
            signature_constraints: &[
                ("cb_event", "void", &["int", "short", "void *"]),
                ("cb_timeout", "void", &["int", "short", "void *"]),
                ("cb_wait_2_timeout", "void", &["int", "short", "void *"]),
            ],
            restricted_calls: &[
                "send",
                "__errno_location",
                "evutil_socket_geterror",
                "perror",
                "evutil_closesocket",
                "event_del",
                "event_free",
                "mk_mem_free",
                "mk_mem_alloc_z",
                "event_base_new",
                "event_base_free",
                "event_new",
                "event_add",
                "event_base_loop",
                "evutil_socketpair",
                "_mk_event_init",
                "_mk_event_update",
                "_mk_event_add",
                "_mk_event_del",
                "mk_event_libevent_socketpair",
                "_mk_event_wait_with_flags",
            ],
        },
    )
}
