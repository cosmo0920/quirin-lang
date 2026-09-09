/* CT45 source control — the same shape with a destructor that is declared.
 *
 * Identical to the case except that the destructor called is the one the
 * contract set describes. The registry, the borrowed parameter and every
 * declaration are unchanged, so this control is what makes the case's
 * `Unknown` attributable to the *unknown destructor* rather than to the
 * parameter, the registry or the shape of the call.
 */

void *q_alloc(void);
void q_free(void *p);

/* Shared state the unknown destructor is documented to reach. */
static void *q_registry;

extern void q_destroy(void *p);

void subject(void *p) { q_free(p); }
