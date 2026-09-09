/* CT45 — "Unknown destructor accesses shared global state".
 *
 * The C twin of the case: `q_destroy` is the destructor for whatever `p`
 * refers to. Its body is not in this translation unit — that is what makes it
 * unknown — and it is documented to touch the file-scope registry below, so
 * its effect on `p` and on that shared state is exactly what an access
 * contract would have to declare. No contract in force declares it.
 *
 * Analyzed through the real unrestricted-C gateway
 * (`analyze_unrestricted_c11`) with `subject` as the entry.
 */

void *q_alloc(void);
void q_free(void *p);

/* Shared state the unknown destructor is documented to reach. */
static void *q_registry;

extern void q_destroy(void *p);

void subject(void *p) { q_destroy(p); }
