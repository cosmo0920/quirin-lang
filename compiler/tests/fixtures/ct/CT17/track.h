/* CT17 heap witness. Records every Buffer allocation and release by address so
   single transfer and single release are checked by identity rather than by the
   trace alone. `PAYLOAD_FAIL_TASK`/`PAYLOAD_FAIL_ATTEMPT` select which worker
   allocation fails; attempt 0 never matches, so with no `-D` the header injects
   no failure at all. */
#include <stdio.h>
#include <stdlib.h>
#ifndef PAYLOAD_FAIL_TASK
#define PAYLOAD_FAIL_TASK 0
#endif
#ifndef PAYLOAD_FAIL_ATTEMPT
#define PAYLOAD_FAIL_ATTEMPT 0
#endif
static void *payload_alloc(size_t size, unsigned long long task, unsigned long long attempt) {
    if (task == PAYLOAD_FAIL_TASK && attempt == PAYLOAD_FAIL_ATTEMPT) return NULL;
    void *p = malloc(size);
    fprintf(stderr, "PAYLOAD_ALLOC %p\n", p);
    return p;
}
static void payload_free(void *p) { fprintf(stderr, "PAYLOAD_FREE %p\n", p); free(p); }
#define QCORE_BUFFER_MALLOC_FOR_TASK(size, task, attempt) payload_alloc(size, task, attempt)
#define QCORE_BUFFER_FREE payload_free
