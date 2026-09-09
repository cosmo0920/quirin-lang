/* CT43 control — the same callback body, called directly.
 *
 * Everything except the deferral is unchanged: the same `on_complete`, the
 * same borrowed parameter, the same declared API in scope. Calling it here and
 * now is decidable, so this control checks. It is what makes the case's
 * `Unknown` attributable to the *registration*, and not to the callback, the
 * parameter or the surrounding declarations.
 */

void *q_alloc(void);
void q_free(void *p);

typedef void (*q_completion)(void *);

extern void q_register(q_completion callback, void *arg);

void on_complete(void *p) { q_free(p); }

void subject(void *p) { on_complete(p); }
