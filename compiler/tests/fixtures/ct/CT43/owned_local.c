/* CT43 second control — the registration is not read as a hand-off.
 *
 * Here `subject` owns what it registers. The gateway does not treat the
 * uncontracted registration as consuming `arg`, so it reports the acquisition
 * as a leak *beside* the unsupported call. That is a provisional `Unsafe` over
 * an unresolved effect, not a validated violation: if `q_register` takes
 * custody, no leak exists. This control exists so that reading is pinned and
 * visible rather than discovered later; card CT-CTWIN records it as
 * `CT-CTWIN-UNRESOLVED-LEAK`.
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
