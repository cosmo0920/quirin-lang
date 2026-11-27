# Event caller removal contracts

User direction (2026-10-27): extend caller lifetime and cross-function composition around `mk_event_add` and `mk_event_del`, after landing the 808d CMetrics getter work. Base: `b403cd4deddaab711f446f383270363aca39a2af`.

First concrete increment: declare the exact `mk_event_timeout_disable(loop, data)` wrapper as a borrowed pair plus the same registration removal as `mk_event_del`, in Monkey's owner provider. The complete wrapper is identical at the three audited revisions and returns `mk_event_del(loop, (struct mk_event *) data)` directly. It neither closes the descriptor nor destroys the loop. Preserve unmatched-removal poisoning and all existing holder-identity refusals. Add canonical-contract caller controls for exact removal, wrong-holder poisoning, poison persistence and release ordering, with audited Lean for the corresponding removal composition. Generate inventories and source pins with their owning tools; never hand-edit locks.

This increment carries an explicit removal effect across a declared function boundary. It does not implement retained/replayed lifetime imports across translation units, conditional host/record correlation, collector-by-id resolution, derived loop-path equality, or threaded teardown ordering. Keep `FB-EVENT-LOOP-CROSS-FUNCTION-LIFETIME`, `RETENTION-DERIVED-LOOP-PATH-EQUALITIES`, `RETENTION-PATH-LOOP-DESTROY-ADMISSION` and `MK-EVENT-DESCRIPTOR-CUSTODY` open. Body review and pins are declared assumptions, not a new automatic body proof.

Close with the standing scoped gate on the final card commit, source reviews and pin checks at all three revisions, provider/package checks, complete BASE/TIP resource/NULL/dependency attribution through the measure plugin, one integration amendment paragraph and current status. The integrator runs the full landing gate and publishes the live profiles. No new native safety gain is promised.

Documentation correction after semantic implementation: the statement above
that disable does not close a descriptor describes the selected epoll reading,
not every backend. The wrapper contains no extra close call, but its delegated
backend may close descriptors (the Libevent `_mk_event_del` closes pipe ends).
This card states no descriptor-lifecycle effect. The formal removal clauses
and their interpreted semantics are unchanged.
