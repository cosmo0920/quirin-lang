/* CT40 — "Atomic pointer points to prematurely freed memory".
 *
 * The C twin of the case: `subject` acquires an owned pointer, publishes it
 * into a file-scope `_Atomic` object that outlives the call, and then releases
 * it. Whatever reads `g_published` afterwards holds a dangling reference; this
 * translation unit contains no such reader, and the projection does not model
 * the publication protocol that would decide the question either way.
 *
 * The scenario expects `Not certified; unsupported lifetime/publication
 * protocol`, and that is what the gateway reports (card CTWIN-PUBLISH): the
 * store of a tracked identity into a static-storage object is refused, so the
 * projection is withdrawn rather than completed.
 *
 * Analyzed through the real unrestricted-C gateway
 * (`analyze_unrestricted_c11`) with `subject` as the entry.
 */

void *q_alloc(void);
void q_free(void *p);

void *_Atomic g_published;

void subject(void) { void *p = q_alloc(); g_published = p; q_free(p); }
