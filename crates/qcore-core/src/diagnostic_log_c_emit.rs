// SPDX-License-Identifier: Apache-2.0
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

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
    QCORE_DIAG_RUN_OUTCOME,
    QCORE_DIAG_TASK_LEASE,
    QCORE_DIAG_TASK_LEASE_RETURN,
    QCORE_DIAG_BUFFER_BYTE_WRITE,
    QCORE_DIAG_BUFFER_BYTE_READ
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

/// The fixed-width result arm of the hosted stdout adapter (card
/// `W48-QR-SCALAR-TYPES-R4`). A task worker's result of a fixed-width type other
/// than `i32` is logged under a name `=TYPE` that no other event uses — every
/// other result name is `i32`, `unit`, `buffer` or an error tag, which is an
/// identifier — with an unsigned number in `related` and a signed one (and
/// `bool`) in `value`, each lossless. The line is `QCORE_TASK_RESULT TASK TYPE
/// NUMBER`, `true`/`false` for `bool`.
const FIXED_RESULT_ARM: &str = r#"        if (event->name != NULL && event->name[0] == '=') {
            if (event->name[1] == 'U')
                (void)fprintf(stdout, "QCORE_TASK_RESULT %" PRIu64 " %s %" PRIu64 "\n",
                    event->subject, event->name + 1, event->related);
            else if (event->name[1] == 'B')
                (void)fprintf(stdout, "QCORE_TASK_RESULT %" PRIu64 " BOOL %s\n",
                    event->subject, event->value != 0 ? "true" : "false");
            else
                (void)fprintf(stdout, "QCORE_TASK_RESULT %" PRIu64 " %s %" PRId64 "\n",
                    event->subject, event->name + 1, event->value);
            return;
        }
"#;

/// The adapter `text` (one with [`FIXED_RESULT_ARM`]) with a float result's
/// branch (card `W48-QR-FLOATING-POINT`): a result named `=F32`/`=F64` carries
/// its bit pattern in `related`, printed as that unsigned number, and every NaN
/// as the word `nan` — a NaN's payload and sign are not specified, so a native
/// run and the reference agree on "a NaN", never on its bits. Installed only
/// in a program that logs a float result; every other adapter keeps its bytes.
#[must_use]
pub fn with_float_results(text: &str) -> String {
    let anchor = "            if (event->name[1] == 'U')\n                (void)fprintf(stdout, \"QCORE_TASK_RESULT";
    assert_eq!(
        text.matches(anchor).count(),
        1,
        "the fixed result arm is installed once"
    );
    text.replace(
        anchor,
        &format!(
            "{}{}",
            r#"            if (event->name[1] == 'F') {
                int q_nan = event->name[2] == '3'
                    ? ((event->related & UINT64_C(0x7F800000)) == UINT64_C(0x7F800000)
                        && (event->related & UINT64_C(0x007FFFFF)) != 0)
                    : ((event->related & UINT64_C(0x7FF0000000000000)) == UINT64_C(0x7FF0000000000000)
                        && (event->related & UINT64_C(0x000FFFFFFFFFFFFF)) != 0);
                if (q_nan)
                    (void)fprintf(stdout, "QCORE_TASK_RESULT %" PRIu64 " %s nan\n",
                        event->subject, event->name + 1);
                else
                    (void)fprintf(stdout, "QCORE_TASK_RESULT %" PRIu64 " %s %" PRIu64 "\n",
                        event->subject, event->name + 1, event->related);
            } else
"#,
            anchor
        ),
    )
}

/// The adapter `text` (one with the typed-payload arms) with a float payload's
/// branch in each of the allocation, read and write cases (card
/// `W48-QR-FLOAT-BUFFER-PAYLOADS`): an event named `=F32`/`=F64` carries the
/// payload's bit pattern in `value`, printed as that unsigned number — read
/// back by the always-defined signed-to-unsigned conversion — and every NaN
/// as the word `nan`, the float result's rule. Installed only in a program
/// holding a float payload; every other adapter keeps its bytes.
#[must_use]
pub fn with_float_payloads(text: &str) -> String {
    let mut text = text.to_owned();
    for tag in [
        "QCORE_BUFFER_ALLOC",
        "QCORE_BUFFER_READ",
        "QCORE_BUFFER_WRITE",
    ] {
        let anchor = format!(
            "            if (event->name[1] == 'U')\n                (void)fprintf(stdout, \"{tag} %"
        );
        assert_eq!(
            text.matches(&anchor).count(),
            1,
            "the {tag} typed-payload arm is installed once"
        );
        let branch = format!(
            r#"            if (event->name[1] == 'F') {{
                uint64_t q_bits = (uint64_t)event->value;
                int q_nan = event->name[2] == '3'
                    ? ((q_bits & UINT64_C(0x7F800000)) == UINT64_C(0x7F800000)
                        && (q_bits & UINT64_C(0x007FFFFF)) != 0)
                    : ((q_bits & UINT64_C(0x7FF0000000000000)) == UINT64_C(0x7FF0000000000000)
                        && (q_bits & UINT64_C(0x000FFFFFFFFFFFFF)) != 0);
                if (q_nan)
                    (void)fprintf(stdout, "{tag} %" PRIu64 " %" PRIu64 " %s nan\n",
                        event->subject, event->related, event->name + 1);
                else
                    (void)fprintf(stdout, "{tag} %" PRIu64 " %" PRIu64 " %s %" PRIu64 "\n",
                        event->subject, event->related, event->name + 1, q_bits);
            }} else
"#
        );
        text = text.replace(&anchor, &format!("{branch}{anchor}"));
    }
    text
}

/// [`DIAGNOSTIC_STDOUT_ADAPTER_DEFINITION`] with [`FIXED_RESULT_ARM`] placed
/// first in its task-result case (card `W48-QR-SCALAR-TYPES-R4`). A program
/// whose workers return no fixed-width value never logs such a name, so the
/// arm is unreachable there; it is installed only in a program that needs it,
/// and every other program keeps the historical adapter's bytes.
#[must_use]
pub fn diagnostic_stdout_adapter_definition_with_fixed_results() -> String {
    let anchor = "    case QCORE_DIAG_TASK_RESULT:\n";
    assert_eq!(
        DIAGNOSTIC_STDOUT_ADAPTER_DEFINITION.matches(anchor).count(),
        1,
        "the adapter has one task-result case"
    );
    DIAGNOSTIC_STDOUT_ADAPTER_DEFINITION.replace(anchor, &format!("{anchor}{FIXED_RESULT_ARM}"))
}

/// The typed-payload arm of one Buffer case of the hosted stdout adapter (card
/// `W48-QR-SCALAR-TYPES-R6`). A Buffer event whose payload has a §G02t type
/// other than `i32` is logged under the name `=TYPE` — no other Buffer event
/// carries a name — with the payload's bit pattern in `value`: an unsigned
/// number's `uint64_t` bits (read back by the always-defined signed-to-unsigned
/// conversion), a signed number or a `bool` itself. The line is the historical
/// one with `TYPE NUMBER` in place of the `i32` (`true`/`false` for `bool`).
fn typed_payload_arm(tag: &str) -> String {
    format!(
        r#"        if (event->name != NULL && event->name[0] == '=') {{
            if (event->name[1] == 'U')
                (void)fprintf(stdout, "{tag} %" PRIu64 " %" PRIu64 " %s %" PRIu64 "\n",
                    event->subject, event->related, event->name + 1, (uint64_t)event->value);
            else if (event->name[1] == 'B')
                (void)fprintf(stdout, "{tag} %" PRIu64 " %" PRIu64 " BOOL %s\n",
                    event->subject, event->related, event->value != 0 ? "true" : "false");
            else
                (void)fprintf(stdout, "{tag} %" PRIu64 " %" PRIu64 " %s %" PRId64 "\n",
                    event->subject, event->related, event->name + 1, event->value);
            return;
        }}
"#
    )
}

/// The hosted stdout adapter for a task program, with the fixed-width
/// result arm when `fixed_results` (card `W48-QR-SCALAR-TYPES-R4`) and the
/// typed-payload arms of the allocation, read and write cases when
/// `typed_payloads` (card `W48-QR-SCALAR-TYPES-R6`). With neither it is
/// [`DIAGNOSTIC_STDOUT_ADAPTER_DEFINITION`] byte for byte: an arm is installed
/// only in a program that can reach it.
#[must_use]
pub fn diagnostic_stdout_adapter_definition_for(
    fixed_results: bool,
    typed_payloads: bool,
) -> String {
    let mut text = if fixed_results {
        diagnostic_stdout_adapter_definition_with_fixed_results()
    } else {
        DIAGNOSTIC_STDOUT_ADAPTER_DEFINITION.to_owned()
    };
    if typed_payloads {
        for (case, tag) in [
            (
                "    case QCORE_DIAG_RESOURCE_ACQUIRE:\n",
                "QCORE_BUFFER_ALLOC",
            ),
            ("    case QCORE_DIAG_RESOURCE_READ:\n", "QCORE_BUFFER_READ"),
            (
                "    case QCORE_DIAG_RESOURCE_WRITE:\n",
                "QCORE_BUFFER_WRITE",
            ),
        ] {
            assert_eq!(
                text.matches(case).count(),
                1,
                "the adapter has one {tag} case"
            );
            text = text.replace(case, &format!("{case}{}", typed_payload_arm(tag)));
        }
    }
    text
}

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
    case QCORE_DIAG_TASK_LEASE:
        /* The lease's byte range travels packed in `value`: low bound in the
           high 32 bits, high bound in the low 32 (card WRITE-LEND-TASKS). */
        (void)fprintf(stdout, "QCORE_BUFFER_LEASE %" PRIu64 " %" PRIu64 " %" PRIu64 " %" PRIu64 "\n",
            event->subject, event->related,
            (uint64_t)event->value >> 32, (uint64_t)event->value & 0xffffffffu); return;
    case QCORE_DIAG_TASK_LEASE_RETURN:
        (void)fprintf(stdout, "QCORE_BUFFER_LEASE_RETURN %" PRIu64 " %" PRIu64 "\n",
            event->subject, event->related); return;
    case QCORE_DIAG_BUFFER_BYTE_WRITE:
        /* Index and byte travel packed in `value`: the payload's extent is
           four and a byte is eight bits, so both fit the int32 the worker's
           private event array carries (card WRITE-LEND-TASKS-EXERCISE). */
        (void)fprintf(stdout, "QCORE_BUFFER_BYTE_WRITE %" PRIu64 " %" PRIu64 " %" PRId64 " %" PRId64 "\n",
            event->subject, event->related,
            event->value >> 8, event->value & INT64_C(255)); return;
    case QCORE_DIAG_BUFFER_BYTE_READ:
        (void)fprintf(stdout, "QCORE_BUFFER_BYTE_READ %" PRIu64 " %" PRIu64 " %" PRId64 " %" PRId64 "\n",
            event->subject, event->related,
            event->value >> 8, event->value & INT64_C(255)); return;
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
