/* CT41 shape A — the PRODUCER, with the qualifier.
 *
 * `volatile int` used as a completion flag: the body acquires, sets the flag a
 * consumer would spin on, and releases. Card CT-CTWIN-LANE (wave 35) measured
 * this against shape B, which is this file with `volatile` removed and nothing
 * else, and the two answers are byte-identical.
 */

void *q_alloc(void);
void q_free(void *p);

volatile int g_done;

void subject(void) { void *p = q_alloc(); g_done = 1; q_free(p); }
