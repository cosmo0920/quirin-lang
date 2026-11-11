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

//! Format typed mutex events for the existing independent native/reference check.

pub fn instrument(source: &str) -> String {
    format!("{DECLARATION}\n{source}\n{SINK}")
}

const DECLARATION: &str = r#"
#include <inttypes.h>
#include <stdio.h>
#include <string.h>
struct qcore_diagnostic_event;
static void observe_mutex(const struct qcore_diagnostic_event *event);
#define QCORE_DIAGNOSTIC_SINK(event_pointer) observe_mutex(event_pointer)
#define QCORE_DIAGNOSTIC_THRESHOLD QCORE_LOG_TRACE
"#;

const SINK: &str = r#"
static void observe_mutex(const struct qcore_diagnostic_event *event)
{
    const char *label = NULL;
    switch (event->kind) {
    case QCORE_DIAG_RESOURCE_ACQUIRE:
        (void)printf("resource %" PRIu64 " %" PRIu64 "\n", event->subject, event->related); return;
    /* Card MUTEX-SHARE-CROSS-EMISSION: a fork/join trace's task structure.
       Both lines carry parent and child, so the native line is the reference
       interpreter's line and not a narrower rendering of it. */
    case QCORE_DIAG_TASK_SPAWN:
        (void)printf("spawn %" PRIu64 " %" PRIu64 "\n", event->subject, event->related); return;
    case QCORE_DIAG_TASK_JOIN:
        (void)printf("join %" PRIu64 " %" PRIu64 "\n", event->subject, event->related); return;
    case QCORE_DIAG_MUTEX_CREATE: label = "create"; break;
    case QCORE_DIAG_MUTEX_LOCK:
        (void)printf("lock %" PRIu64 " %" PRIu64 "\n", event->subject, event->related); return;
    case QCORE_DIAG_MUTEX_UNLOCK: label = "unlock"; break;
    case QCORE_DIAG_INVARIANT_OPEN: label = "open"; break;
    case QCORE_DIAG_INVARIANT_CLOSE: label = "close"; break;
    case QCORE_DIAG_RESOURCE_WRITE:
        (void)printf("write %" PRIu64 " %" PRIu64 " %" PRId64 "\n",
            event->subject, event->related, event->value); return;
    case QCORE_DIAG_RESOURCE_READ:
        if (event->name == NULL) abort();
        if (strcmp(event->name, "borrow") == 0) { label = "borrow"; break; }
        if (strcmp(event->name, "end-borrow") == 0) { label = "end-borrow"; break; }
        (void)printf("read %" PRIu64 " %" PRIu64 " %s %" PRId64 "\n",
            event->subject, event->related, event->name, event->value); return;
    case QCORE_DIAG_INVARIANT_FACT:
        if (event->name == NULL) abort();
        if (strcmp(event->name, "assert") == 0) { label = "assert"; break; }
        if (strcmp(event->name, "use") == 0) { label = "use-fact"; break; }
        if (strncmp(event->name, "grow ", 5) != 0) abort();
        (void)puts(event->name); return;
    case QCORE_DIAG_MUTEX_DESTROY: label = "destroy-mutex"; break;
    case QCORE_DIAG_RESOURCE_RELEASE: label = "destroy"; break;
    case QCORE_DIAG_RUN_OUTCOME:
        if (event->name == NULL || strcmp(event->name, "error-exit") != 0) abort();
        (void)printf("error-exit %" PRIu64 " %" PRId64 "\n", event->subject, event->value); return;
    default: abort();
    }
    (void)printf("%s %" PRIu64 "\n", label, event->subject);
}
"#;
