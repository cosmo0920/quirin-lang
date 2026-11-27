# NULL-MEMBER-INVARIANT-SCOPE: prediction

Frozen before any implementation code (base `c77efedbe`). Measured against
the BEFORE run of the base binary on the corpus arbiter (66910c10), the
system route and the tail-pause slice, custody and the null property both.

## The reading chosen

A global member declaration (rule 14t) fails when one store anywhere writes
NULL. The three named members fail for two different reasons, and only one of
them admits a sound scoped reading:

* **Fresh-object stores.** `rd_kafka_broker_s.rkb_rk` was withdrawn by four
  initializer lists (the lookup key `skel = {.rkb_nodeid = …}`, three
  `RD_ZERO_INIT` unit-test brokers). An initializer list writes an object
  that did not exist before it, so it cannot make an *existing* object's
  member NULL. Every other store is proved non-null. The member is therefore
  **stable once non-null**: a non-null value observed on an object stays
  non-null for that object's lifetime, whatever runs in between, on any
  thread, because every store that could change it is checked.
* **Exit stores on existing objects.** `rd_kafka_buf_s.rkbuf_rkb` is reset to
  NULL on the broker's receive buffer (`rdkafka_broker.c`, `rd_kafka_recv`),
  and `mk_list.next` (and `cfl_list.next`) is set to NULL on a node by
  `mk_list_del` (`cfl_list_del`). The invariant then holds only for objects
  in a state (a buffer handed to a parser, a linked node), and the state is
  left in place on an object other code may still hold. Showing that an exit
  store never reaches an object a scoped use is reading needs object identity
  across memory, threads and calls (which buffer is the broker's receive
  buffer; which nodes are linked); nothing the null property checks gives
  that, and a per-function scope does not either (the reset can run on
  another thread while a parse is active, and a fact kept across a call that
  synchronises could read that store). **Prediction: no sound reading; both
  recorded with measured sizes and left undeclared.**

Vocabulary (bounds contract v1, null property only, pinned; grammar G05f;
rules 14x–15b):

1. `requires null_member_stable(TAG, FIELD);` (object-state scope (b): the
   state "set" is entered at any non-null store or observation and is never
   left; a fresh object starts outside it). Checked at every store the
   analysed units make: an assignment or compound update not proved non-null
   withdraws it; an initializer list is a fresh-object store (exempt and
   counted); a whole-object store the walk can attribute (a struct assignment
   of `struct TAG`, a `memset`/`memcpy`/`memmove`/`bzero` whose destination is
   typed `struct TAG *`) is a store not proved. Whole-object writes it cannot
   attribute stay a listed premise, so the standing is at best
   `partly-validated`. Effect: a non-null fact on `V->FIELD`, `V` a tracked
   local or formal, survives calls and stores until `V` is reassigned (proof
   `stable:TAG.FIELD`, a row dependency; withdrawn ⇒ the check is a candidate
   again and the row is never Safe).
2. `requires null_member_scope(TAG, FIELD, P);` on a function (function
   scope (a)): on entry, `P->FIELD` is non-null; each caller's actual is a
   `call-precondition` check of `ACTUAL->FIELD` (a fact, or its own scope
   entry); the row is Safe conditional on it. With (1) in force the entry fact
   lasts the whole body; without it, until the first call.

## Predicted outcome

| Quantity | Prediction |
| --- | --- |
| Custody, all three routes | byte-identical (915 / 9 / 3,910 `ded14632…`; 91 / 4 / 353; 50 / 1 / 349 / 4 Pending) |
| Custody premise rows fully proved | 1 → 1 (the 31 premises read `s->iov`, `i->storage`, list elements; none is `rkb_rk`) |
| `rkb_rk` stable standing | `partly-validated`, 0 stores not proved, 4–8 fresh-object stores counted |
| `rkbuf_rkb`, `mk_list.next`, `cfl_list.next` | not declared (exit stores on existing objects); sizes recorded |
| Candidates sourced from `…->rkb_rk` (741 on the base, own + travelled) | −150 to −500 |
| Memory-field candidates on the functions' own checks (8,605) | −150 to −600 |
| Corpus null rows Unknown → Safe | +3 to +40; no Safe → non-Safe move except a named correction |
| Findings (33) | unchanged (persistence keeps only non-null facts; the path search is a separate model) |
| System, slice null rows | small or no movement (few librdkafka functions) |
| Parity streams | multiset-identical |
| Wall time | +0 to +5 % |

Scope entries (form 2) are declared for the functions the AFTER ranking of
the stable reading shows hold the most remaining first reads of
`rkb->rkb_rk` through a formal, each body read at both revisions and pinned;
their callers' checks are counted separately. If a stable declaration is
withdrawn by the run's store check, that is reported and the reading is
dropped, as rule 14t's withdrawal did for `rkb_rk` in NULL-CANDIDATE-TRIAGE.
