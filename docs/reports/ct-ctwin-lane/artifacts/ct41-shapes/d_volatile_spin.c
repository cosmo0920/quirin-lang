/* CT41 shape D — the CONSUMER, with the qualifier. The scenario's own shape.
 *
 * "`volatile` used as a completion flag" in its proper sense: a consumer spins
 * on the flag and then releases, the release being ordered only by the flag.
 * Card CT-CTWIN-LANE (wave 35) measured this `safe` with NO diagnostic, and
 * measured shape E -- this file without the qualifier -- byte-identically so.
 *
 * The `safe` is correct within the gateway's own scope: one translation unit,
 * one acquisition, one release, resource projection only. It is simply not
 * evidence about a synchronization proof, which is why CT41 cannot map here --
 * a mapping would record a certification against a case whose expected outcome
 * is a non-certification.
 */

void *q_alloc(void);
void q_free(void *p);

volatile int g_done;

void subject(void) { void *p = q_alloc(); while (!g_done) { } q_free(p); }
