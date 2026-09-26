# W45-CONTAINER-ROOT-REACH: frozen prediction

Frozen 2026-09-26 on base `12553fb3`, after the census and before any
implementation. The BEFORE arbiter run reproduces
`docs/reports/w45-fb-backlog-custody/integration/artifacts/`
(`resource-diagnostics.json` byte-identical; `verdict_bearing_identical:
true`). The slice BEFORE at `046accf6` (a fresh private clone, attempt-1) gives
units 0 / 2 / 13.

## Census (criterion 1)

Every `mk_list_add` / `mk_list_del` call site of the corpus units and of the
slice, classified by root shape (`scripts/census.py`, `scripts/census_rank.py`,
[census/ranked-before.json](census/ranked-before.json)). The first blocker is
the leaf the audit reports at the call's line.

| Scope | Shape | Sites | Functions | First blocker |
|---|---|---:|---:|---|
| corpus | C1 publication into a formal chain (already admitted) | 32 | 22 | identity 12, call refused 12, entry 4, none 4 |
| corpus | **U1** unlink of an element of a formal's container (safe iteration, first/last lookup) | 16 | 12 | **10d unlink 12**, call refused 4 |
| corpus | U6 unlink of a formal's own object, container unnamed | 12 | 11 | 10d unlink 9, call refused 3 |
| corpus | C4 publication into a container a callee returned | 10 | 6 | call refused 9, identity 1 |
| corpus | C3 publication into a body-acquired owner that is published or returned | 8 | 5 | identity 6, call refused 2 |
| corpus | U3 unlink of an element of a callee's (TLS) container | 2 | 2 | 10d unlink 2 |
| corpus | C5, C6, U0, U7, plain-iteration unlink | 1 each | 1 each | — |
| slice | C1 | 17 | 17 | none 7, identity 4, entry 3, call refused 3 |
| slice | U6 | 10 | 9 | 10d unlink 8, call refused 2 |
| slice | **U1** | 10 | 8 | **10d unlink 9**, call refused 1 |
| slice | C3 | 3 | 1 | call refused 3 |
| slice | C4, C5, U4, U5 | 1–2 | 1 | — |

**Finding.** No publication in the corpus or the slice is refused by rule 10d
for its container: the container-root refusal (`does not show outlives it`) has
**zero** leaves. Every publication 10d does not admit is blocked first by its
entry (an unresolved or nullable identity, an owner of linked objects) or by a
refusal of the whole call. The four `MK-LIST-ADD-BORROWED-PUBLICATION`
functions (`mk_rconf_read`, `mk_vhost_create`, `mk_vhost_set_property`,
`mk_vhost_set_single`) publish into C1 (`&conf->includes`, `&vh->server_names`,
`&server->hosts`) or C3 (`&h->server_names`) containers, and each is blocked by
its entry: an uncontracted `mk_string_dup` store into the object poisons its
binding before the link. Widening container roots reaches none of them.

The dominant 10d refusal is the **foreign unlink** (U1): 12 corpus leaves in 9
teardown functions and 9 slice leaves in 7 functions.

## Admitted shapes

* **U1** — an element reached through a container rooted at an unrebound
  formal (or a local initialised from one), bound by an admitted
  `container_of` over a safe iteration's current cursor, a plain cursor, or a
  `(H)->next`/`->prev` lookup; every write of the name is such a binding.
* **U2 / C2** — the same, and a publication, for a container that is a
  static-storage object (`&G`, `&G.F`). Zero sites in either census; admitted
  for the accepting control.

Everything else keeps its refusal: U3–U7 and C3–C6, and U6 (the formal's own
object) in particular, is registered as a residual with its size.

## Predicted movement

**Corpus (185 units)**, against the baseline:

* 10d unlink leaves removed: **10–12** of the 12 U1 leaves;
* the `unresolved resource identity E` leaves at the element's release after
  the unlink: **8–12** removed;
* leaves at member releases (`mk_mem_free(E->F)`) persist, possibly with a
  changed description: exchanges **0–15**;
* publication leaves (the 4 10d-entry leaves): **unchanged**;
* Unknown → Safe **0–1** (every U1 function has another refusal: member
  releases, uncontracted calls, a formal released without a summary);
  Safe → non-Safe **0**; units unchanged;
* leak pin 34 → **34–35**; the named risk is `mk_http_request_free_list`,
  whose embedded `sr_fixed` element is not released (expected withdrawn by the
  refused `mk_http_request_free` call that receives it).

**Slice at `046accf6`:** 7–9 of the 9 U1 unlink leaves removed per variant;
the U4 site (`schedule_request_promote`'s local list) keeps its refusal;
statuses unchanged, except a possible provisional leak in
`sb_remove_chunk_from_segregated_backlog` on its `destroy == 0` arm (leak
+0–1, a function-local candidate no caller reaches).

**Obligations.** OB-3, OB-7 and OB-10 lose the foreign-unlink blocker and stay
**Unknown** (OB-8's counter access, reachable release and shutdown ordering
remain). The OR table is unchanged: OR-1 and OR-7 refuted, OR-6 Unknown,
OR-2 not reached.
