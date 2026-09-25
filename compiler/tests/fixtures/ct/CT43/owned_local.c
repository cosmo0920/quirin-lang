/* CT43 second control — the registration is not read as a hand-off.
 *
 * Here `subject` owns what it registers. The gateway does not treat the
 * uncontracted registration as consuming `arg`, so the acquisition is never
 * discharged. Card CT-CTWIN pinned the leak the gateway then reported *beside*
 * the unsupported call: a provisional `Unsafe` over an unresolved effect, not a
 * validated violation (if `q_register` takes custody, no leak exists), recorded
 * as `CT-CTWIN-UNRESOLVED-LEAK`. Card W44-LEAK-PIN-AUDIT's refused hand-on rule
 * withdraws that claim: the refused call names `p`, so the unit is `Unknown`,
 * never `Safe`.
 */

void *q_alloc(void);
void q_free(void *p);

typedef void (*q_completion)(void *);

extern void q_register(q_completion callback, void *arg);

void on_complete(void *p) { q_free(p); }

void subject(void) {
  void *p = q_alloc();
  q_register(on_complete, p);
}
