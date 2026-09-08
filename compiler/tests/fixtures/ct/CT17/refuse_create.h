/* CT17 scenario 4: creation refuses, so there is no child, no completion and no
   payload; the parent keeps and releases the Buffer it still owns. */
#include "track.h"
#include <threads.h>
static int refuse_create(thrd_t *thread, thrd_start_t worker, void *argument) {
    (void)thread; (void)worker; (void)argument; return thrd_error;
}
#define QCORE_TASK_CREATE refuse_create
