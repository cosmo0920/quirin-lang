/* CT41 shape E — the CONSUMER, without the qualifier.
 *
 * Shape D with `volatile` removed and nothing else. Measured byte-identical to
 * D: `safe`, no diagnostic. Removing the construct the scenario names moves
 * nothing, which is clause (b) of the wave-9 attribution rule failing, and it
 * fails by the project's own decision rather than by a gap.
 */

void *q_alloc(void);
void q_free(void *p);

int g_done;

void subject(void) { void *p = q_alloc(); while (!g_done) { } q_free(p); }
