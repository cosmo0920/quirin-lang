/* CT40 boundary control — the case with the release removed instead.
 *
 * Publishing without ever releasing is refused too, and with one diagnostic
 * rather than two: the alias the case's release reads is gone, but the
 * publication is not. The refusal is therefore the publication itself and not
 * the "prematurely freed" half of the scenario title, which this projection
 * does not decide.
 *
 * This body is measured `Unsafe` with a leak before card CTWIN-PUBLISH; the
 * leak claim was withdrawn there because it depended on an unresolved
 * publication protocol, which is a downgrade, never a promotion.
 */

void *q_alloc(void);
void q_free(void *p);

void *_Atomic g_published;

void subject(void) { void *p = q_alloc(); g_published = p; }
