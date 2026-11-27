#include <inttypes.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

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

enum qcore_error_tag {
    QERR_NONE = 0,
    QERR_FAILURE = 1,
};

union qcore_error_payload {
    int32_t failure;
};

struct qcore_error {
    enum qcore_error_tag tag;
    union qcore_error_payload payload;
};

struct qcore_result {
    bool ok;
    int32_t value;
    struct qcore_error error;
    void *owned_1;
    void *owned_2;
};

static void q_trace(char kind, uint64_t identity)
{
    enum qcore_diagnostic_kind diagnostic_kind;
    switch (kind) {
    case 'A': diagnostic_kind = QCORE_DIAG_RESOURCE_ACQUIRE; break;
    case 'R': diagnostic_kind = QCORE_DIAG_RESOURCE_RELEASE; break;
    case 'M': diagnostic_kind = QCORE_DIAG_RESOURCE_MOVE; break;
    default: abort();
    }
    qcore_diagnostic_log(QCORE_LOG_TRACE, diagnostic_kind, identity,
        UINT64_C(0), INT64_C(0), (const char *)0);
}

static bool q_acquire(void **slot, bool succeeds, uint64_t identity)
{
    if (!succeeds) {
        qcore_diagnostic_log(QCORE_LOG_TRACE, QCORE_DIAG_RESOURCE_ACQUIRE_FAILED,
            identity, UINT64_C(0), INT64_C(0), (const char *)0);
        return false;
    }
    *slot = malloc(1);
    if (*slot == NULL) {
        qcore_diagnostic_log(QCORE_LOG_TRACE, QCORE_DIAG_RESOURCE_ACQUIRE_FAILED,
            identity, UINT64_C(0), INT64_C(0), (const char *)0);
        return false;
    }
    q_trace('A', identity);
    return true;
}

static void q_release(void **slot, uint64_t identity)
{
    if (*slot == NULL) {
        abort();
    }
    free(*slot);
    *slot = NULL;
    q_trace('R', identity);
}

#line 22 "compiler/tests/fixtures/region/discharged_pack.qcore"
static struct qcore_result qcore_fn_main(void)
{
    struct qcore_result q_result = {0};
    (void)&q_acquire;
    (void)&q_release;
    void *q_res_1 = NULL;
    void *q_res_2 = NULL;
    goto q_bb_0;
q_bb_0:
    if (q_acquire(&q_res_1, true, UINT64_C(1))) {
        goto q_bb_1;
    } else {
        goto q_cleanup_5;
    }
q_bb_1:
    if (q_acquire(&q_res_2, true, UINT64_C(2))) {
        goto q_bb_2;
    } else {
        goto q_cleanup_6;
    }
q_bb_2:
    q_release(&q_res_2, UINT64_C(2));
    goto q_bb_3;
q_bb_3:
    q_release(&q_res_1, UINT64_C(1));
    goto q_bb_4;
q_bb_4:
    q_result.ok = true;
    q_result.value = INT32_C(0);
    return q_result;
q_cleanup_5:
    q_result.ok = false;
    q_result.error.tag = QERR_FAILURE;
    q_result.error.payload.failure = INT32_C(0);
    return q_result;
q_cleanup_6:
    q_release(&q_res_1, UINT64_C(1));
    q_result.ok = false;
    q_result.error.tag = QERR_FAILURE;
    q_result.error.payload.failure = INT32_C(0);
    return q_result;
}

int main(void)
{
    struct qcore_result q_result = qcore_fn_main();
    (void) q_result.value;
    (void) q_result.error;
    qcore_diagnostic_log(QCORE_LOG_DEBUG, QCORE_DIAG_RUN_OUTCOME, UINT64_C(0), UINT64_C(0), q_result.ok ? (int64_t)q_result.value : (int64_t)q_result.error.tag, q_result.ok ? "ok" : "error");
    if (q_result.owned_1 != NULL) {
        free(q_result.owned_1);
        q_result.owned_1 = NULL;
    }
    if (q_result.owned_2 != NULL) {
        free(q_result.owned_2);
        q_result.owned_2 = NULL;
    }
    return 0;
}
