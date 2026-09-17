/* CT41 shape B — the PRODUCER, without the qualifier.
 *
 * Shape A with `volatile` removed and nothing else. Measured byte-identical to
 * A in verdict, verbatim reason, per-function status and ordered diagnostics.
 * That identity is card CTWIN-PUBLISH section 5's decision holding, not a
 * defect: a projection deriving no synchronization fact must answer the same
 * either way.
 */

void *q_alloc(void);
void q_free(void *p);

int g_done;

void subject(void) { void *p = q_alloc(); g_done = 1; q_free(p); }
