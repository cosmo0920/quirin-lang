/* CT40 invariance control — the case with `_Atomic` removed and nothing else.
 *
 * This control does NOT move the verdict, and pinning it is the point. The
 * gateway derives no synchronization or atomicity fact, so the qualifier on the
 * destination cannot move a resource-projection verdict; card CTWIN-PUBLISH
 * measured exactly this (§7) and card CT-CTWIN asserted the same invariance for
 * `volatile` (CT-CTWIN-SYNC). The scenario's title names an atomic pointer; the
 * construct its `expected` text names — and the one this case's mapping rests
 * on — is the publication protocol.
 *
 * Recording the invariance as a fixture control keeps the bound structural: if
 * a future change ever made `_Atomic` load-bearing here, this pin moves and the
 * mapping's stated scope has to be re-argued.
 */

void *q_alloc(void);
void q_free(void *p);

void *g_published;

void subject(void) { void *p = q_alloc(); g_published = p; q_free(p); }
