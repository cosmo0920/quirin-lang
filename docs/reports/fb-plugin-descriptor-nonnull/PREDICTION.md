# FB-PLUGIN-DESCRIPTOR-NONNULL-SCOPE: prediction

Frozen before any declaration is written. Base: local `main` `ca3d827fe`
(FB-QUANT-CONTRACT-CENSUS records over `d708641bc`; no code change since the
census). BEFORE figures are the census's (round 28): corpus 936 / 9 / 3,889,
null 1,113 / 1,435 / 2,355 / 18 Unsafe; system 93 / 4 / 351, null 84 / 125 /
245; slice 52 / 1 / 347 / 4 Pending, null 71 / 103 / 232. This card re-measures
its own BEFORE with main's binary and stops if it differs.

## 1. Every store of `p`, at both pinned revisions

Read at 66910c10 (corpus tree), 046accf6 (system tree) and the tail-pause
export of 90978ac6 (the slice tree; for every function below byte-identical to
046accf6):

| Store | Constructor | 66910c10 | 046accf6 | Value at the store |
| --- | --- | --- | --- | --- |
| `instance->p = plugin;` | `flb_input_new` | `flb_input.c:490` | `:490` | the loop's plugin; dereferenced at `plugin->name` / `plugin->flags` just before (moved question) |
| `instance->p = plugin;` | `flb_output_new` | `flb_output.c:779` | `:857` | the matched plugin; `plugin == NULL` returns earlier; dereferenced at `plugin->workers`, `plugin->name` before |
| `instance->p = plugin;` | `flb_filter_new` | `flb_filter.c:507` | `:507` | `if (!plugin) return NULL;` then `plugin->event_type`, `plugin->name` |
| `instance->p = plugin;` | `flb_processor_instance_create` | `flb_processor.c:2188` | `:2188` | `if (!plugin) return NULL;` then `plugin->name` |
| `instance->p = plugin;` | `flb_custom_new` | `flb_custom.c:214` | `:214` | (no reader on any route; not declared) |

No other source file under `src/`, `include/` or `plugins/` assigns the
member, and no file of either selection writes a whole instance (`memset`,
`memcpy`, `memmove`, `bzero`, struct assignment); the only other stores are in
`tests/internal/*.c` (`input->p = input_plugin`, `instance.p = &plugin`), which
no selection analyses. Each constructor's body is byte-identical at 046accf6
and the export; `flb_input_new` and `flb_output_new` differ at 66910c10 (one
TLS field, the alias lookup), not at the store.

**The fresh object's NULL.** Each instance is `flb_calloc`ed, so `p` is NULL
from the allocation to the store. Rule 14x examines assignments, compound
updates, struct assignments and typed fills/copies of an existing object; an
allocator call is none of them, so the calloc'd NULL is not an examined store.
That reading is the rule's own: the stable state is entered at a non-null
store or observation and a fresh object starts outside it, so the stable
declaration never makes a fresh instance's `p` non-null. A use of `ins->p` is
non-null only by a fact the walk saw (a test, a dereference, a scope entry),
and a scope entry is a checked obligation at every direct caller. Nothing
between the allocation and the store reads `instance->p` in any constructor.
Prediction: every examined store is `assignment` and proved (the plugin's
question moved or tested), none `whole-object`; no withdrawal on any route.

## 2. Declarations to write

* `requires nonnull_member_stable(TAG, p);` on `flb_input_new`
  (`flb_input_instance`), `flb_output_new` (`flb_output_instance`),
  `flb_filter_new` (`flb_filter_instance`), `flb_processor_instance_create`
  (`flb_processor_instance`). Standing at best `partly-validated` where a
  route analyses the store (input on the slice, output on the system route),
  else a declared assumption (rule 14x).
* `requires nonnull_member_scope(flb_input_instance, p, ins);` on the seven
  slice readers: `flb_input_plugin_pause`, `flb_input_plugin_resume`,
  `flb_input_pause`, `flb_input_resume`, `flb_input_instance_pre_run`,
  `flb_input_instance_exit`, `flb_input_thread_instance_pre_run`. None has its
  address taken.
* **No output scope entry.** The three output functions the census names
  (`flb_output_pre_run`, `flb_output_exit`, `flb_output_init_all`) take
  `config` and read `ins` from `config->outputs`; G05f's third argument must
  be a pointer parameter, so no scope fits them. Only the stable member acts
  there: a repeated `ins->p` read after a call keeps the first read's fact
  (rule 14y), so the first read per loop iteration stays a candidate and the
  later ones are moved. No filter or processor scope entry: no route reads
  those members.
* All pinned with `quirin-audit pin --bounds` at 66910c10 and 046accf6.

## 3. Per-function prediction

**Custody: no row moves on any route** (bounds v1 clauses; custody never
reads them, rule 14o). Report digests unchanged.

**Slice, null property.**

| Function | BEFORE | Predicted AFTER |
| --- | --- | --- |
| `flb_input_plugin_pause`, `flb_input_plugin_resume` | Unknown (only `ins->p` candidates) | Safe conditional (scope entry; `ins` itself inferred `quant null-deref`) |
| `flb_input_pause`, `flb_input_resume` | Unknown | Safe conditional; the entry fact survives `flb_input_buf_paused` / `flb_input_is_threaded` by stability, and the call to `flb_input_plugin_pause`/`_resume` is a discharged caller check |
| `flb_input_instance_pre_run` | Unknown | Safe conditional; its call to `flb_input_thread_instance_pre_run` discharged by the entry fact |
| `flb_input_instance_exit` | Unknown | Safe conditional (`p = ins->p` copies the entry fact) |
| `flb_input_thread_instance_pre_run` | Unknown | Safe conditional |

Up to 7 functions (both configurations each where present) Unknown → Safe
conditional; each depends on its own scope declaration and on
`stable:flb_input_instance.p` where a call intervenes.

**Callers (rule 14z's risk, each a named correction if a Safe row moves).**
New `call-precondition` checks of `ACTUAL->p`:

* `flb_input_pause_all`, `flb_input_test_pause_resume`,
  `flb_input_rate_gate_protect`, `flb_input_rate_gate_resume` (slice): the
  actual's `p` is never read first, so each check is a memory-field
  candidate; any of these that is null-Safe today becomes Unknown.
* `flb_input_pre_run_all`, `flb_input_exit_all` (slice): they read
  `p = ins->p; if (!p) continue;` before the call; the check is discharged
  only if the walk relates the tested copy to the path `ins->p`. Predicted:
  undecided either way; measured.
* `handle_input_event` (`flb_input_thread.c`, slice): reads
  `ins->p->cb_pause` right before the call; the check moves to that read
  (no new candidate; its row keeps its existing `ins->p` candidates).
* `flb_input_chunk.c` (system and corpus): `flb_input_resume(in)` in the
  limit-reset function and two `flb_input_pause(i)` in
  `flb_input_chunk_protect`: new candidates; both rows already carry other
  candidates (`storage->cio`, `in->storage`), so **no verdict moves on the
  corpus or system**, but the candidate counts grow by up to 3 per
  configuration.

**System, null property.** `flb_output_pre_run` reads `p = ins->p` once per
iteration (one candidate, unchanged); `flb_output_exit` and
`flb_output_init_all` lose the repeated `ins->p` candidates after calls but
keep the first read: no verdict moves. With the `flb_input_chunk.c` caller
checks: system null verdicts 84 / 125 / 245 unchanged.

**Corpus.** No verdict moves; up to 6 candidates added (3 sites × 2
configurations if `flb_input_chunk.c` is analysed twice), each a named
correction of a row already Unknown.

**Totals predicted.** Slice null 71 / 103 / 232 → about 71 / 110 / 225 minus
any caller correction; system 84 / 125 / 245 unchanged; corpus 1,113 / 1,435 /
2,355 / 18 unchanged.
