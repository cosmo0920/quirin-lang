/* CT03 scenario 2: only the *second* `thrd_create` refuses, so the first child
   really runs and really returns its share. */
#include "track.h"
#include <threads.h>
static int attempts = 0;
static int refuse_second_create(thrd_t *thread, thrd_start_t worker, void *argument) {
    if (++attempts >= 2) return thrd_error;
    return thrd_create(thread, worker, argument);
}
#define QCORE_TASK_CREATE refuse_second_create
