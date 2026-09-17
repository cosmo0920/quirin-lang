/* CT41 shape C — the ISOLATING control: no flag at all.
 *
 * Shapes A and B with the flag store removed. This one is `safe`, which is what
 * shows the `unknown` A and B receive is bought by the unsupported assignment
 * expression and not by the qualifier, the acquisition or the release. An
 * Unknown attributable to something else is exactly what the wave-9 attribution
 * rule's clause (b) refuses to accept as a mapping.
 */

void *q_alloc(void);
void q_free(void *p);

void subject(void) { void *p = q_alloc(); q_free(p); }
