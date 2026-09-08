/* CT17 scenario 5: the child's first allocation fails, so its completion holds
   the tagged payload — and then the join refuses. Nothing may be reclaimed:
   every release exits 91, distinguishably from the trapped abort (77). This
   header does not include `track.h`: it must own `QCORE_BUFFER_FREE` itself. */
#include <stdio.h>
#include <stdlib.h>
#include <threads.h>
#include <signal.h>
static void *payload_alloc(size_t size, unsigned long long task, unsigned long long attempt) {
    if (task == 1 && attempt == 1) return NULL;
    void *p = malloc(size);
    fprintf(stderr, "PAYLOAD_ALLOC %p\n", p);
    return p;
}
static void caught_abort(int signal_number) { (void)signal_number; _Exit(77); }
static int refuse_join(thrd_t thread, int *result) {
    /* Flushed here, in ordinary control flow, so the trace written before the
       abort survives the `_Exit` that traps it. */
    (void)thread; (void)result; fflush(stdout); signal(SIGABRT, caught_abort); return thrd_error;
}
static void forbidden_free(void *p) { (void)p; _Exit(91); }
#define QCORE_BUFFER_MALLOC_FOR_TASK(size, task, attempt) payload_alloc(size, task, attempt)
#define QCORE_TASK_JOIN refuse_join
#define QCORE_TASK_FREE forbidden_free
#define QCORE_BUFFER_FREE forbidden_free
