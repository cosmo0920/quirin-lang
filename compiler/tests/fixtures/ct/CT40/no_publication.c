/* CT40 control — the case with exactly the publication statement removed.
 *
 * Same declarations, same `_Atomic` file-scope object, same acquisition and
 * same release; only `g_published = p;` is gone. It checks, so the case's
 * `Unknown` is attributable to the publication rather than to the allocator
 * contracts, the local, the release or the presence of the object.
 */

void *q_alloc(void);
void q_free(void *p);

void *_Atomic g_published;

void subject(void) { void *p = q_alloc(); q_free(p); }
