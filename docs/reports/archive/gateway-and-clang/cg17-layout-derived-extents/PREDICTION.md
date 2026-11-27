# CG17-LAYOUT-DERIVED-EXTENTS: frozen prediction

Frozen before any AFTER measurement (corpus, system or tail-pause slice) and
before the route-G hooks were run on any input. Base: local `main` at
`b6a3dc42` (= origin/main). BEFORE is being measured with
`scripts/measure.sh before` (release binary built at the base).

## What the card changes

* **M12 (VLA).** A refused declaration of one automatic VLA of a builtin
  scalar element type, whose bound only reads integer parameters or locals and
  whose every use is an element access or a `sizeof` operand, is lowered to no
  custody event, carrying three unrecorded-as-discharged numeric obligations
  (bound defined, bound positive, size representable; CG20's). Any other VLA
  keeps its refusal (an address use, a bound with an effect, a non-scalar
  element).
* **E18 (`container_of`).** A refused declaration, assignment or `return`
  whose whole value is `(T *)((char *)p - offsetof(T, m))` is lowered as the
  respelled `… = x;` / `return x;` only when `p` is `&x->m` (or a pointer
  local whose only write is that initializer, neither it nor `x` otherwise
  written or addressed). Every other pointer keeps the refusal.
* **M13 / M19.** Facts and a separately published property
  (`layout.alignment.v1`, `layout_report`); no custody effect.

## Predicted corpus / system / slice movement

* **Corpus (`66910c10`): no function moves** (629 / 10 / 4,195 stays). CG00
  measured 0 Unknown functions freed by CG17 alone. VLAs are rare in the
  corpus libraries (librdkafka, LZ4, xxHash, Monkey/CFL, cmetrics); where one
  exists its function has other refusals. Every corpus `container_of` is a
  list lookup (`mk_list_entry`/`cfl_list_entry`/`rd_list`-style) over a cursor
  or `(H)->next`, never `&x->m` of the same body, so every one is
  `unestablished` and keeps its refusal.
* **Leaves:** at most a handful of same-status functions lose a VLA
  declaration leaf (expected 0–5 functions). No leak or violation added; no
  Safe → non-Safe move (the hooks are asked last, only for refusals, and only
  lower through the older routes).
* **System and tail-pause slice:** byte-identical (Fluent Bit builds with
  MSVC and declares no VLA on these paths; its `container_of` uses are list
  lookups).
* **Census (parity rows, `layout-extents-census`):** VLA declarations few
  (expected < 20); flexible-array accesses tens, every extent
  `size-not-established` (no contract establishes an allocation size);
  `container_of` hundreds, every one `unestablished`; derived-address
  conversions tens to low hundreds, mostly `undecided` (a `char *`/`void *`
  base guarantees one byte).

## Absorbed follow-up

`W45-FB-OR6-ELEMENT-IDENTITY` is predicted **not** discharged: OR-6's element
comes from `mk_list_entry_first(&queue->pending, …)`, a list lookup whose
identity is the container premise (rule 10f's P-CRR-MEMBER), not member
provenance; and the blocking piece, a callee summary stating that
`flb_output_task_singleplex_flush_next` frees the element, is
`W45-FOREIGN-UNLINK-SUMMARY`'s. It is amended with the re-measured state.
