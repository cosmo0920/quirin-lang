# LERI-HELD-AND-LINK-MEMBER-PREMISES: prediction

Frozen before any tip measurement. Base `8fd7c8d0d`.

**Disclosure.** The gateway code, the contract grammar, the Lean module and
the fixtures were drafted in the worktree before this file was committed; none
of it had been measured on a native route when this prediction was written.
The census below is of base `8fd7c8d0d`'s binary alone (`private/bin-base`,
the base built in this worktree), on this card's own preparations
(`artifacts/inputs.json`), with rule 43 on and with `QCORE_GATEWAY_RULES_OFF=43`
(`private/census/`).

## Census at base: where rule 43's premises carry a row

Rule 43 off against on (`scripts/compare.py`):

| Route | Rows moved by switching rule 43 off | Leaf trades |
|---|---|---|
| system (448 rows) | `flb_output_task_queue_enqueue` (×2 configurations) and `flb_sched_timer_cb_destroy` (×2) Safe → Unknown: each gains `unlink by DESTROYER of FORMAL, which this function has not published` beside `unresolved resource identity FORMAL` | `flb_output_task_singleplex_enqueue` (Unsafe, ×2) and `flb_sched_event_handler` (Unknown, ×2) trade `unresolved resource identity` for `call to uncontracted function` of the two above |
| slice (404 rows) | none | none |
| corpus | in flight when this file was committed (`private/census/base-corpus*`); see below | |

Every system row rule 43 carries rests on its **held arm with a formal**:
`flb_output_task_queue_enqueue(…, retry, …)` hands its formal `retry` to
`flb_task_retry_destroy` on its allocation-failure path, and
`flb_sched_timer_cb_destroy(timer)` hands `timer` to
`flb_sched_timer_destroy`. That arm rests on P-LERI-HELD-LINKS: the formal's
links are the caller's, and no statement of the body initialises them.

Bodies read at the three prepared trees (`scripts/unlinks.py`,
`artifacts/unlinks.json`): the ten Fluent Bit destroyers and
`mk_http_thread_destroy` each unlink `_head` unconditionally at their top
level (`flb_task_destroy` inside its orphan test), with the same body digests
LIST-ENTRY-RELEASE-IDENTITY read; `flb_tail_file_remove` also unlinks
`_rotate_head`, only while `file->rotated > 0`. The one caller iterating a
list through another member is `flb_tail_file_purge` (`files_rotated`
through `_rotate_head`), which hands each element to `flb_tail_file_remove`.

## Mechanism

1. Each element destroyer names its link member:
   `resource_unpublished(P, _head)` (eleven declarations, re-pinned).
2. P-LERI-LINK-MEMBER, checked: rule 34's destroyer takes a foreign element
   only when every `container_of` binding names the declared member.
3. P-LERI-HELD-LINKS, checked: rule 43's held arm admits an object only when
   this body initialised the declared member's links itself (a rule 36b
   self-initialisation covering `next` and `prev`), on every arriving path,
   with no statement since that may have written the object. A formal never
   has such a fact.

## Predicted moves

* system: `flb_output_task_queue_enqueue` and `flb_sched_timer_cb_destroy`
  Safe → Unknown in both configurations (custody 107 / 4 / 337 → 103 / 4 /
  341), each refused at the destroyer with the rule 46 held-arm refusal
  (`… which this function holds, but this function has not initialised its
  `_head` links`). Their callers `flb_output_task_singleplex_enqueue` and
  `flb_sched_event_handler` trade a leaf (the call becomes uncontracted, the
  callee's summary withheld), with no status move. Leaf trades where a
  destroyer's held arm admitted a body acquisition on a row that stays
  Unknown on other leaves. No foreign-arm move: every foreign element a
  destroyer takes on a Safe row is bound through `_head`.
* slice: no status move. `flb_tail_file_purge` (Unknown) trades the
  resolved `_rotate_head` element of `files_rotated` back to a refusal (the
  rule 46 member refusal), and may gain it where rule 43 resolved nothing.
* corpus: no status move. The corpus's Fluent Bit unit is
  `src/flb_input_chunk.c` alone: its destroyer calls take foreign elements
  bound through `_head` (`flb_input_chunk_release_space`,
  `input_chunk_release_space` style drains of `&in->chunks`) or a
  callee-constructed chunk (`input_chunk_get`, `flb_input_chunk_append_raw`)
  whose release was already refused at base, so the held refusal replaces an
  existing refusal there (leaf trades only). Monkey's two drains of
  `mk_http_thread` lists (`sched_thread_cleanup`,
  `mk_sched_threads_destroy_conn`) bind through `_head`, which
  `mk_http_thread_destroy` now names, so `mk_sched_threads_purge` and
  `mk_sched_threads_destroy_all` stay Safe. The corpus census of rule 43 off
  against on was still running when this prediction was committed; it is a
  base measurement, and its result is reported beside this prediction in the
  report, not folded into it.
* No Safe row moves except those named; no Unknown → Safe anywhere; no leak,
  double release or Unsafe added or removed; the null property is unchanged.

```measure-prediction
route system
move flb_output_task_queue_enqueue Safe -> Unknown
move flb_sched_timer_cb_destroy Safe -> Unknown
delta Safe -4
delta Unknown +4
route slice
none
route corpus
none
```
