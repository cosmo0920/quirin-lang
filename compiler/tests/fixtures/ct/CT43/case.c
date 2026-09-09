/* CT43 — "Unresolved callback may run after API return".
 *
 * The C twin of the case: a reactor-style registration API records a
 * completion callback and returns immediately, so `on_complete` may run at any
 * later time, including after `subject` has returned. `q_register`'s body is
 * not in this translation unit and no contract declares what it does with
 * `arg`, so nothing here establishes when — or whether — `p` is released.
 *
 * Analyzed through the real unrestricted-C gateway
 * (`analyze_unrestricted_c11`) with `subject` as the entry.
 */

void *q_alloc(void);
void q_free(void *p);

typedef void (*q_completion)(void *);

extern void q_register(q_completion callback, void *arg);

void on_complete(void *p) { q_free(p); }

void subject(void *p) { q_register(on_complete, p); }
