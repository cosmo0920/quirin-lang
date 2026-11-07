# OUT-SLOT-CALL-WRITE-ALIASING

Soundness card registered by
[OUT-PARAMETER-SCALAR-FRAME-PREMISE](../out-parameter-scalar-frame-premise/README.md)'s
deviation. User decision, 2026-11-07, verbatim: "Launch it next". Base
`7c1a474bb` (OUT-PARAMETER-BODY-SUMMARY plus the effective-type premise, on
origin/main `532e03c97`; neither parent card is landed); measurement side
`rev:aa3999333`. [PREDICTION.md](PREDICTION.md) and the census were committed
before the semantic change (`753bf4448`); the change is `898fba855`.

**No currently published `Safe` row depends on the gap.** At baseline
`7b726587b` and in the user's `current` report, no `Safe` row passes a
pointer out slot across a call. The one row that does,
`cio_file_content_copy`, is `Safe` only on the unlanded parent branch (rule
11r(7)); this card returns it to `Unknown`.

## The gap

`int r(char **buf, size_t *size) { … *buf = data; memcpy(size, &n, sizeof n);
return 0; }` validated its out slot under rule 11r(6), and an ordinary W4
`Out` summary did the same: a contracted call with `Borrowed` positions wrote
no binding, so nothing touched `*buf`. `size` may designate the caller's
`char *` object or a byte of it, and `memcpy` copies through character
lvalues. The slot then holds `n`'s bytes, `data` leaks, and the caller frees
garbage. Every `Borrowed` actual that points at writable memory has the same
effect, not only `memcpy`.

## Census

[`census/ast_census.py`](census/ast_census.py) ([`run.sh`](census/run.sh)).
It covers every `Safe` row of the corpus (baseline `7b726587b`, Fluent Bit
`66910c10` and Monkey), system and slice reports, and of the user's
`current` report (Fluent Bit `046accf6`). It adds the parent cards'
predicted `Safe` rows (`CENSUS_ALSO_SAFE`) and every summary any row reads,
including header inline functions that have no row of their own.

For each function, the census reads the preprocessed unit (`clang -E -P`,
the analyzer's text) as a lexical prefilter: a store through a formal, or a
`T **` formal. It then reads Clang's JSON AST for that function. From the
AST it collects every store `*X = V` or `X[0] = V` through a formal, every
`T **` formal handed to a call, and every call after the first such store in
source order (or in a loop holding one). Each actual of those calls is
classified by its derivation. The table comes from
[`census/counts.sh`](census/counts.sh) and the lists from
[`census/summary.py`](census/summary.py).

| Route | `Safe` rows | parent-predicted `Safe` | summaries read | storing through a formal | pointer out slot | slot across a reaching call |
|---|---:|---:|---:|---:|---:|---:|
| corpus | 1,054 | 5 | 598 | 47 | 5 | 1 |
| system | 53 | 0 | 36 | 5 | 0 | 0 |
| slice | 38 | 0 | 41 | 1 | 0 | 0 |
| `current` | 1,052 | 5 | 560 | 47 | 5 | 1 |

The five pointer-out-slot rows are the same on the corpus and on `current`:

| Function | Slot | Calls after the slot store | Actual's derivation | May designate the slot? |
|---|---|---|---|---|
| `cio_file_content_copy` | `void **out_buf` (stored), `size_t *out_size` | `cio_chunk_down(ch)` when `set_down` | `ch`, another formal, `struct cio_chunk *`, declared `resource_borrowed(ch)` | **yes**: no rule or contract excludes it (the caller may pass `&ch->member`) |
| `cio_memfs_content_copy` | `void **out_buf` | none | | no call |
| `cio_utils_read_file` | `char **buf` | none (`fclose` precedes the store) | | no call |
| `rd_slice_reader` | `const void **p`, handed to `rd_slice_reader0`'s derived out slot | none | | no call, and no sanctioned slot (all-`Borrowed` summary) |
| `rd_slice_peeker` | the same | none | | the same |

The other 42 storing rows store only scalars (`size_t *`, `int *`, `double *`,
enumerations) through formals, and none of those is an out slot.
`rd_kafka_message_timestamp` and `rd_kafka_toppar_get_avail` are among them.

Some rows were not read from an AST. On the corpus, 38 summary names are
defined in another corpus unit (`cio_file_*`, `LZ4_*`, `rd_kafka_error_*`,
`mk_fifo_*`, `msg_write`). Their signatures have no `T **` formal, and where
a unit has their row it is read there. On `current`, the five
`lib/monkey/tls/tls_stub.c` stubs have no compile entry; their bodies store
nothing.

## The rule: 11r(8)

`docs/agents/rules/11r.md` (8), and an amended-by note in 12s.

Every analysis whose contract sanctions out slots reads its calls from the
Clang AST. Those analyses are:

* rule 4's W4 `Out` summaries and their probes;
* rule 11r(4)/(6)/(7)'s classes;
* rule 12s(2)'s rows;
* a declared `resource_out` body.

For each call, the AST reading gives the lines of the call's statement, its
callee, and the positions whose actual may designate caller memory. The code
is [`legacy_out_slot_call_write.rs`](../../../crates/qcore-c-gateway/src/legacy_out_slot_call_write.rs),
with one hook in `legacy_transfer.rs::transfer`.

At a node that applies a call on those lines, after the call's own effect,
every sanctioned slot that is still bound is poisoned the same way the typed
scalar store's refused-write frame poisons it. The node may be a
`ContractCall` with its nested calls, an element store or release, a
sub-object store of a call, or a self-initialiser. The slot is then
`Ambiguous` at the exits it reaches, so nothing that needed it validates.
The row returns to its uncontracted reading, and nothing is accused.

**Why fail-closed settles it.** The spec gives `Borrowed` positions no
read-only frame:

* `resource_borrowed` states no custody precondition and no transferred
  obligation (C04.1).
* The v1 vocabulary has no read/write distinction (C04.3.1 item 6).
* Rule 10k's W48 amendment already says "a `Borrowed` custody mode alone"
  establishes no read-only frame.

So no existing rule or spec text states that a `Borrowed` callee writes
nothing. The effective-type premise of 11r(7) never spares a call: a call's
write is untyped, and the `mem*`/`str*` copies are character-typed.

**Excluded actuals.** An actual is excluded only when it is positively read
as one of these:

* not a pointer (a record or union passed by value reaches);
* a function pointer, a null pointer constant or a string literal;
* a pointer to a `const` pointee at the position;
* `&NAME`, `&NAME.f`, `&NAME[i]` or the decay of an array `NAME`, where
  `NAME` is an automatic local or a formal's own variable whose type holds no
  pointer, record or union;
* a position the call's own contract declares `Out`, which names the object
  the call writes.

Everything else reaches, including another formal, a local pointer's value,
a member, `*X`, a call result, a `static` or file-scope object, and the
address of a pointer or record local.

**Slots and calls the rule does not touch.**

* A slot no store has bound stays unwritten. A `Borrowed` write passes no
  custody, and rule 11r(2)'s consumer already reads the caller's variable as
  untracked on that class.
* If there is no AST, or no AST call on a call node's lines, every position
  but an `Out` one reaches.
* A declared `resource_out` body's own row already refuses its store on
  this base (probed: `store through `*out` without an out-parameter
  contract`), so poisoning without a diagnostic cannot leave such a row
  `Safe`.

**Premises stated, not proved:**

* Clang's extraction (trust boundary 28);
* the callee does not write through a `const`-qualified pointee;
* an `Out` position writes only the object it names.

## Fixtures and Lean

[`tests/out_slot_call_write_aliasing.rs`](../../../crates/qcore-c-gateway/tests/out_slot_call_write_aliasing.rs)
has 13 tests. Each asserts every function's status, its complete diagnostic
vector and its carried premises.

These now refuse; the callee is `Unknown` with the uncontracted store, and
its testing caller is `Unknown`:

* `x_memcpy(size, &n, sizeof n)`;
* `x_memmove(size, …)`;
* `x_memset(size, 0, …)`;
* a bodied callee `put(size)` whose own summary is all-`Borrowed` and whose
  body writes `*p`;
* the `cio_file_content_copy` shape (`x_down(ch)` after both stores);
* a member actual (`ch->data`) and a local copy of one;
* an `Out` position beside a reaching `Borrowed` one;
* a call after the effective-type store.

These are unchanged and `Safe`:

* a `const` source position with a local array destination;
* distinct `&local` actuals;
* a call before the slot store;
* a call on the unwritten class's path;
* a callee whose `Out` contract states the written object (`x_make(&t)`
  after the slot);
* the read-file shape with its `effective-type:` premise.

With the hook disabled, the eight refusing fixtures are `Safe`, so every
one of them drives the new arm. The five unchanged ones pass either way.

The pinned fixture
`out_parameter_scalar_frame_premise::a_copy_into_the_second_formal_is_not_read_under_the_premise`
flips on purpose from `Safe` to `Unknown`, for the reason above. It is
re-pinned with that reason.

`QCore.OutSlotCallWrite` ([`proof/QCore/OutSlotCallWrite.lean`](../../../proof/QCore/OutSlotCallWrite.lean),
audited by `proof/AuditOutSlotCallWrite.lean`) proves:

* `transfer_sound`: every value the call can leave is admitted by the
  post-state, unwritten slots included (`unwritten_stays_sound`);
* `a_reaching_call_poisons_a_bound_slot`;
* `kept_when_no_actual_reaches`;
* `bound_kept_only_when_excluded`;
* `no_reaching_actual_designates`;
* `the_old_reading_is_unsound`: the identity transfer before this card
  admits a run where the slot claims a site and holds data;
* three decided instances: the `memcpy` shape, distinct locals, and a
  `const` source.

## Prediction

See [PREDICTION.md](PREDICTION.md). The integrator measures:

* **Corpus:** `cio_file_content_copy` goes `Safe -> Unknown` (`delta Safe -1`)
  against `rev:aa3999333`. The base row's `Safe` is itself the parent card's
  prediction.
* **System and slice:** no change.
* **`current`:** the same row is `Unknown` again, and no unit status
  changes.

```sh
target/release/quirin-audit --plugin-path target/release --output <dir> --timeout-ms 36000000 measure \
  --inputs crates/quirin-audit-plugin-measure/inputs/live.json --inputs-base /media/Data/GitHub/quirin-lang \
  --side base=rev:aa3999333 --side tip=rev:898fba855 \
  --declared docs/reports/out-slot-call-write-aliasing/PREDICTION.md
```

## Follow-ups

These are registered in [integration-amendments.json](integration-amendments.json):

* `OUT-SLOT-CALL-WRITE-FRESH-ACTUAL`: a local pointer bound only to an object
  this body allocated still reaches. Two common shapes are `free(tmp)` and
  `memcpy(buf, …)` after the slot store. Admitting them needs a reading of an
  owned result's freshness. No census row needs it.
* `OUT-SLOT-CALL-GLOBAL-REACH`: the rule reads only actuals. A callee may
  still write a file-scope object that is itself the caller's slot (`r(&g)`
  with the callee resetting `g`). This is P-BORROW-FRAME's premise, which is
  unstated for out slots.
