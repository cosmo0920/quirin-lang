//! Typed diagnostic logging boundary shared by the C11 backends.
//!
//! Backends retain their trace-event routes but submit events here instead of
//! formatting writes at each event site. Ordinary emission selects the null
//! sink and therefore needs no stdio. A verification or debug build may define
//! `QCORE_DIAGNOSTIC_SINK` to the name of a typed event consumer supplied by
//! its harness.

use crate::c_program::{Fragment, Section};

/// The deduplication key of [`DIAGNOSTIC_LOG_PRELUDE`] in an assembled unit.
pub const DIAGNOSTIC_LOG_KEY: &str = "qcore.diagnostic_log";

/// The deduplication key of the hosted stdout adapter's declaration.
pub const ADAPTER_DECLARATION_KEY: &str = "qcore.diagnostic_stdout.declaration";

/// The deduplication key of the hosted stdout adapter's definition.
pub const ADAPTER_DEFINITION_KEY: &str = "qcore.diagnostic_stdout.definition";

/// [`DIAGNOSTIC_LOG_PRELUDE`] as an assembler fragment (card
/// `C-EMIT-MAIN-UNIFY`). Every backend that installs the typed logger names it
/// through this constructor, so a composed program defines it once.
#[must_use]
pub fn diagnostic_log_fragment() -> Fragment {
    Fragment::new(Section::Prelude, DIAGNOSTIC_LOG_KEY, DIAGNOSTIC_LOG_PRELUDE)
}

/// The hosted stdout adapter's declaration as a fragment. A declaration, so it
/// sits in [`Section::Declarations`] — ahead of the prelude that defines the
/// event structure it names.
#[must_use]
pub fn adapter_declaration_fragment(level: crate::std_io::Level) -> Fragment {
    Fragment::new(
        Section::Declarations,
        ADAPTER_DECLARATION_KEY,
        diagnostic_stdout_adapter_declaration(level),
    )
}

/// The hosted stdout adapter's definition as a fragment.
#[must_use]
pub fn adapter_definition_fragment() -> Fragment {
    Fragment::new(
        Section::Prelude,
        ADAPTER_DEFINITION_KEY,
        DIAGNOSTIC_STDOUT_ADAPTER_DEFINITION,
    )
}

/// C11 definitions for the typed diagnostic logger.
pub const DIAGNOSTIC_LOG_PRELUDE: &str = r#"
#include <stdint.h>

enum qcore_diagnostic_level {
    QCORE_LOG_TRACE = 0,
    QCORE_LOG_DEBUG = 1,
    QCORE_LOG_INFO = 2,
    QCORE_LOG_WARN = 3,
    QCORE_LOG_ERROR = 4,
    QCORE_LOG_FATAL = 5
};

enum qcore_diagnostic_kind {
    QCORE_DIAG_RESOURCE_ACQUIRE = 0,
    QCORE_DIAG_RESOURCE_ACQUIRE_FAILED,
    QCORE_DIAG_RESOURCE_READ,
    QCORE_DIAG_RESOURCE_WRITE,
    QCORE_DIAG_RESOURCE_RELEASE,
    QCORE_DIAG_RESOURCE_MOVE,
    QCORE_DIAG_TASK_CREATE_FAILED,
    QCORE_DIAG_TASK_SPAWN,
    QCORE_DIAG_TASK_JOIN,
    QCORE_DIAG_TASK_RESULT,
    QCORE_DIAG_TASK_LOAN,
    QCORE_DIAG_TASK_RETURN,
    QCORE_DIAG_MUTEX_CREATE,
    QCORE_DIAG_MUTEX_LOCK,
    QCORE_DIAG_MUTEX_UNLOCK,
    QCORE_DIAG_MUTEX_DESTROY,
    QCORE_DIAG_INVARIANT_OPEN,
    QCORE_DIAG_INVARIANT_CLOSE,
    QCORE_DIAG_INVARIANT_FACT,
    QCORE_DIAG_EXTERN_CALL,
    QCORE_DIAG_EXTERN_OUTCOME,
    QCORE_DIAG_EVENT_STEP,
    QCORE_DIAG_RUN_OUTCOME
};

struct qcore_diagnostic_event {
    enum qcore_diagnostic_level level;
    enum qcore_diagnostic_kind kind;
    uint64_t subject;
    uint64_t related;
    int64_t value;
    const char *name;
};

#ifndef QCORE_DIAGNOSTIC_SINK
#define QCORE_DIAGNOSTIC_SINK(event_pointer) ((void)(event_pointer))
#endif

/* Levels at or above the threshold are admitted; 6 is the ordinary quiet
 * configuration. A harness may override both macros before this prelude. */
#ifndef QCORE_DIAGNOSTIC_THRESHOLD
#define QCORE_DIAGNOSTIC_THRESHOLD 6
#endif

static void qcore_diagnostic_log(enum qcore_diagnostic_level level,
        enum qcore_diagnostic_kind kind, uint64_t subject, uint64_t related,
        int64_t value, const char *name)
{
    const struct qcore_diagnostic_event event = {
        level, kind, subject, related, value, name
    };
    if ((int)level >= QCORE_DIAGNOSTIC_THRESHOLD) {
        QCORE_DIAGNOSTIC_SINK(&event);
    }
}
"#;

/// Install the hosted stdout adapter before [`DIAGNOSTIC_LOG_PRELUDE`]. The
/// source-level `log_level` declaration is the only production route that uses
/// it; ordinary emission never includes stdio through this module.
pub fn diagnostic_stdout_adapter_declaration(level: crate::std_io::Level) -> String {
    let threshold = match level {
        crate::std_io::Level::Trace => "QCORE_LOG_TRACE",
        crate::std_io::Level::Debug => "QCORE_LOG_DEBUG",
        crate::std_io::Level::Info => "QCORE_LOG_INFO",
        crate::std_io::Level::Warn => "QCORE_LOG_WARN",
        crate::std_io::Level::Error => "QCORE_LOG_ERROR",
        crate::std_io::Level::Fatal => "QCORE_LOG_FATAL",
    };
    format!(
        r#"#include <inttypes.h>
#include <stdio.h>
struct qcore_diagnostic_event;
static void qcore_diagnostic_stdout(const struct qcore_diagnostic_event *event);
#define QCORE_DIAGNOSTIC_SINK(event_pointer) qcore_diagnostic_stdout(event_pointer)
#define QCORE_DIAGNOSTIC_THRESHOLD {threshold}
"#
    )
}

/// Definition paired with [`diagnostic_stdout_adapter_declaration`], emitted
/// after the typed event structure is complete.
pub const DIAGNOSTIC_STDOUT_ADAPTER_DEFINITION: &str = r#"
static void qcore_diagnostic_stdout(const struct qcore_diagnostic_event *event)
{
    switch (event->kind) {
    case QCORE_DIAG_RESOURCE_ACQUIRE:
        if (event->related == 0) {
            (void)fprintf(stdout, "A %" PRIu64 "\n", event->subject); return;
        }
        (void)fprintf(stdout, "QCORE_BUFFER_ALLOC %" PRIu64 " %" PRIu64 " %" PRId64 "\n",
            event->subject, event->related, event->value); return;
    case QCORE_DIAG_RESOURCE_ACQUIRE_FAILED:
        (void)fprintf(stdout, "QCORE_BUFFER_ALLOC_FAILED %" PRIu64 "\n", event->subject); return;
    case QCORE_DIAG_RESOURCE_READ:
        (void)fprintf(stdout, "QCORE_BUFFER_READ %" PRIu64 " %" PRIu64 " %" PRId64 "\n",
            event->subject, event->related, event->value); return;
    case QCORE_DIAG_RESOURCE_WRITE:
        (void)fprintf(stdout, "QCORE_BUFFER_WRITE %" PRIu64 " %" PRIu64 " %" PRId64 "\n",
            event->subject, event->related, event->value); return;
    case QCORE_DIAG_RESOURCE_RELEASE:
        if (event->related == 0) {
            (void)fprintf(stdout, "R %" PRIu64 "\n", event->subject); return;
        }
        (void)fprintf(stdout, "QCORE_BUFFER_DROP %" PRIu64 " %" PRIu64 "\n",
            event->subject, event->related); return;
    case QCORE_DIAG_RESOURCE_MOVE:
        (void)fprintf(stdout, "M %" PRIu64 "\n", event->subject); return;
    case QCORE_DIAG_TASK_CREATE_FAILED:
        (void)fprintf(stdout, "QCORE_TASK_CREATE_FAILED %" PRIu64 "\n", event->subject); return;
    case QCORE_DIAG_TASK_SPAWN:
        (void)fprintf(stdout, "QCORE_TASK_SPAWN %" PRIu64 " %" PRIu64 "\n",
            event->subject, event->related); return;
    case QCORE_DIAG_TASK_JOIN:
        (void)fprintf(stdout, "QCORE_TASK_JOIN %" PRIu64 "\n", event->subject); return;
    case QCORE_DIAG_TASK_LOAN:
        (void)fprintf(stdout, "QCORE_BUFFER_LOAN %" PRIu64 " %" PRIu64 "\n",
            event->subject, event->related); return;
    case QCORE_DIAG_TASK_RETURN:
        (void)fprintf(stdout, "QCORE_BUFFER_RETURN %" PRIu64 " %" PRIu64 "\n",
            event->subject, event->related); return;
    case QCORE_DIAG_TASK_RESULT:
        if (event->name != NULL && event->name[0] == 'i')
            (void)fprintf(stdout, "QCORE_TASK_RESULT %" PRIu64 " I32 %" PRId64 "\n",
                event->subject, event->value);
        else if (event->name != NULL && event->name[0] == 'u')
            (void)fprintf(stdout, "QCORE_TASK_RESULT %" PRIu64 " UNIT\n", event->subject);
        else if (event->name != NULL && event->name[0] == 'b')
            (void)fprintf(stdout, "QCORE_TASK_RESULT %" PRIu64 " BUFFER %" PRIu64 "\n",
                event->subject, event->related);
        else if (event->related != 0)
            (void)fprintf(stdout, "QCORE_TASK_RESULT %" PRIu64 " ERROR_BUFFER %s %" PRIu64 "\n",
                event->subject, event->name, event->related);
        else
            (void)fprintf(stdout, "QCORE_TASK_RESULT %" PRIu64 " ERROR %s\n",
                event->subject, event->name);
        return;
    case QCORE_DIAG_RUN_OUTCOME:
        if (event->name != NULL && event->name[0] == 'o') {
            (void)fprintf(stdout, "OK %" PRId64 "\n", event->value); return;
        }
        if (event->name != NULL && event->name[0] == 'e') {
            (void)fprintf(stdout, "ERR %" PRId64 "\n", event->value); return;
        }
        break;
    default:
        break;
    }
    if (event->name != NULL)
        (void)fprintf(stdout, "%s\n", event->name);
}
"#;
