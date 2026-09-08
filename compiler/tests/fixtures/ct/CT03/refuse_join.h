/* CT03 scenario 3: a join that never observes its child returns no share, so
   nothing may be reclaimed. Every release is forbidden and exits 91, so a
   reclamation is distinguishable from the trapped abort (77). This header does
   not include `track.h`: it must own `QCORE_BUFFER_FREE` itself. */
#include <stdio.h>
#include <stdlib.h>
#include <threads.h>
#include <signal.h>
static void *share_alloc(size_t size) { void *p = malloc(size); fprintf(stderr, "SHARE_ALLOC %p\n", p); return p; }
static void caught_abort(int signal_number) { (void)signal_number; _Exit(77); }
static int refuse_join(thrd_t thread, int *result) {
    /* Flushed here, in ordinary control flow, so the trace written before the
       abort survives the `_Exit` that traps it. */
    (void)thread; (void)result; fflush(stdout); signal(SIGABRT, caught_abort); return thrd_error;
}
static void forbidden_free(void *p) { (void)p; _Exit(91); }
#define QCORE_BUFFER_MALLOC share_alloc
#define QCORE_TASK_JOIN refuse_join
#define QCORE_TASK_FREE forbidden_free
#define QCORE_BUFFER_FREE forbidden_free
