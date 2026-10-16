# FB-PROCESSOR-INSTANCE-ALLOCATION-SIZE: a processor instance allocated by the filter instance's size

Card FB-PROCESSOR-INSTANCE-ALLOCATION-SIZE (user request 2026-10-16: "We need
to check it via a small card."). Measurement and records only: no analysis
code, contract, rule or Lean file changed. Base: local `main` `1c7333db1`,
rebased onto `4285ff7cd` before the records. Fluent Bit sources were read
only, from the prepared trees the routes analyse and, for history, the
read-only checkout `/home/cosmo/Documents/GitHub/fluent-bit` (read-only `git
log`, no fetch).

**Verdict: benign but fragile.** It is a type mismatch, and nothing overflows.
In every configuration measured, `sizeof(struct flb_filter_instance)` (216
bytes on x86-64) is larger than `sizeof(struct flb_processor_instance)` (152
bytes). No member access to a processor instance reaches past byte 152.
Nothing views the object as any other record type. Under-allocation is
therefore not reachable, and no upstream note is drafted.

## The observation

`src/flb_processor.c:2171`, in `flb_processor_instance_create` (2144–2212):

```c
instance = flb_calloc(1, sizeof(struct flb_filter_instance));
```

`instance` is a `struct flb_processor_instance *`. The file is byte-identical
at 66910c10, at 046accf6 and in the slice export 34155955 (sha256
`015f68b9…`). Both headers are identical at 66910c10 and 046accf6:
`flb_processor.h` `654e2e73…` (also identical at 34155955) and `flb_filter.h`
`74b0c716…`.

## Sizes per configuration

The layouts were measured by [`scripts/layout.py`](scripts/layout.py). It
compiles `src/flb_processor.c` with the unit's own recorded command, once per
object (static and shared), using `-fsyntax-only`, so the trees are never
written. Each object gets two readings:

* Clang's `-fdump-record-layouts-complete` ([one dump](artifacts/record-layouts.txt));
  the dumps from all eight runs are byte-identical.
* An offsetof/sizeof probe ([probe.c](artifacts/probe.c)). It is compiled
  with the same command, and Clang's AST gives the probe's array extents. It
  also asserts `_Static_assert(sizeof(F) >= sizeof(P))`.

The two readings agree everywhere ([layouts.json](artifacts/layouts.json)).

| Configuration | Revision | Compile database | Objects | `sizeof` processor | `sizeof` filter | Filter's `#ifdef` members |
| --- | --- | --- | --- | --- | --- | --- |
| corpus (`build/cycle19/c-gateway-wave-pin`, `/usr/bin/clang` 21.1.8) | 66910c10 | `5e6868d9…` | static, shared | 152 | 216 | `match_regex`, `metrics` both present |
| system (`w46-external-callee-contracts/attempt-1`, llvm-21) | 046accf6 | `2475e637…` | static, shared | 152 | 216 | both |
| pin tree (`w48-callee-closure-r3/attempt-1`) | 046accf6 | `dcff56cf…` | static, shared | 152 | 216 | both |
| slice / tail-pause export (`w47-tail/attempt-1/prep`) | 34155955 | `faf44675…` | static, shared | 152 | 216 | both |

The `struct flb_processor_instance` members (offset, size, end) are the same
in every configuration:

| Member | Offset | Size | End |
| --- | --- | --- | --- |
| `id` | 0 | 4 | 4 |
| `log_level` | 4 | 4 | 8 |
| `event_type` | 8 | 4 | 12 |
| `name` (`char[32]`) | 12 | 32 | 44 |
| `alias` | 48 | 8 | 56 |
| `context` | 56 | 8 | 64 |
| `data` | 64 | 8 | 72 |
| `pu` | 72 | 8 | 80 |
| `p` | 80 | 8 | 88 |
| `properties` (`struct mk_list`) | 88 | 16 | 104 |
| `config_map` | 104 | 8 | 112 |
| `log_decoder` | 112 | 8 | 120 |
| `log_encoder` | 120 | 8 | 128 |
| `cmt` | 128 | 8 | 136 |
| `notification_channel` (`int` on Linux) | 136 | 4 | 140 |
| `config` | 144 | 8 | 152 |

**Derived readings** ([`scripts/variants.py`](scripts/variants.py),
[variants.json](artifacts/variants.json)). These are not recorded Fluent Bit
builds. Each uses the system static command, with the probe changed as
listed:

| Variant | Processor | Filter | Margin |
| --- | --- | --- | --- |
| Without `FLB_HAVE_REGEX` and `FLB_HAVE_METRICS`, the filter's two `#ifdef` members (the `-D`s are removed, and a private copy of the configured `flb_info.h` without its two `#define`s comes first on the include path) | 152 | 200 | 48 |
| `--target=i686-linux-gnu` | 96 | 132 | 36 |
| i686, without both members | 96 | 124 | 28 |
| `armv7a-linux-gnueabihf`, `aarch64-linux-gnu` | not measured | not measured | — |

The armv7 and aarch64 probes do not compile, because the host's x86 headers
refuse those targets. Their numbers are therefore not taken.

**Configurations not measured** (Windows/MSVC, macOS, 32-bit ARM). For these
the report gives a reasoning, not a measurement. It assumes standard C
layout: no packing pragma or attribute on either record (none exists), and
the same types in one translation unit. Both records begin with their `int`
members and `char name[32]`: 44 bytes for the processor, 48 for the filter.
Pointer-sized members follow:

* the processor has 11 pointer-sized slots, then `flb_pipefd_t`, then one
  pointer;
* the filter has at least 17 pointer-sized slots, then the same
  `flb_pipefd_t` and pointer.

The filter's first pointer offset is at least the processor's, and every
later offset adds the same pointer size. So the filter's
`notification_channel` sits at least six pointers later, and its size cannot
be smaller.

## Accesses

The census comes from [`scripts/census.py`](scripts/census.py) and
[`scripts/census-all.sh`](scripts/census-all.sh), summarised by
[`scripts/summarize.py`](scripts/summarize.py). Results are in
[access-census-summary.json](artifacts/access-census-summary.json) and
[system-046accf6.census.json](artifacts/system-046accf6.census.json). Its
method:

* It reads Clang's AST (`-ast-dump`), each unit with its own command.
* It identifies a member access by the `FieldDecl` the `MemberExpr` names, so a
  same-named member of another record is never counted.
* It classifies each use from the parent node.

It covers 50 units in each of the four configurations:

* `src/flb_processor.c`, `src/flb_help.c`, `src/fluent-bit.c`,
  `src/flb_notification.c` and `src/flb_metrics_exporter.c`;
* every `plugins/processor_*` unit;
* every `tests/` or `benchmarks/` unit whose text names `processor`.

Two units fail on generated headers that the prepared builds do not contain:

* `src/fluent-bit.c` lacks `backtrace.h`. Its AST was dumped only partially.
  Its processor lines, 504–511, were read by text: `p->p->config_map` at :509,
  then `flb_help_processor` and `flb_processor_instance_destroy`.
* `plugins/processor_sql/parser/sql_parser.c` lacks the bison header. Its text
  names no processor instance.

Four units never see the record definition.

Every configuration gives the same counts: **550 member accesses** over all
16 members, and the largest member end is 152. **No access lies outside 216**,
nor outside 200, the smallest filter size measured on LP64. By the reasoning
above, that holds on every ABI that was not measured too.

**The allocation's own path** (at 046accf6; the lines are identical at all
three revisions; 82 rows in the artifact):

| Site | Accesses (member, use) |
| --- | --- |
| `flb_processor_instance_create`, `flb_processor.c:2171` | `void *` → `struct flb_processor_instance *` (implicit `BitCast`) of `flb_calloc`'s result |
| :2176–2191 | writes `config` 144..152, `id` 0..4, `event_type` 8..12, `alias` 48..56, `p` 80..88, `data` 64..72, `log_level` 4..8, `pu` 72..80; `name` 12..44 as `snprintf`'s destination (bounded by `sizeof(instance->name) - 1`) |
| :2193 | `&instance->properties` (88..104) to `mk_list_init` |
| :2195–2209 | `log_encoder` (120..128) write and read; `log_level` and `p` read by `flb_plg_error`; `log_decoder` (112..120) write and reads |
| caller `flb_processor_unit_create`, :776, :792 | the result is stored as `pu->ctx = (void *) processor_instance`. `pu->unit_type` is `FLB_PROCESSOR_UNIT_NATIVE` (written only at :760 and :773) |
| `pu->ctx` readers in `flb_processor.c` | read back as `struct flb_processor_instance *` only on the native branch (:1143, :1178, :1182, :1216, :1228 `notification_channel` 136..140, :1537 then `p_ins->p` / `log_decoder` / `log_encoder` at :1543–1773). Read as `struct flb_filter_instance *` only under `unit_type == FLB_PROCESSOR_UNIT_FILTER` (:1099–1125, :1172–1174, :1203–1211, :1412–1414, :1956–1959) |
| outside `flb_processor.c` | `flb_metrics_exporter.c:334`, `:369` and `flb_notification.c:112–123` dispatch on `unit_type` in the same way. `flb_notification_deliver` reads its `union generic_plugin_instance` arm by `plugin_type` |
| caller `flb_help.c:665` | `flb_help_processor` reads `p` (80..88) and `config` (144..152) at :377–404, then destroys the instance |
| caller `fluent-bit.c:504` | `p->p->config_map` at :509, then help and destroy |
| `flb_processor_instance_init`, `_exit`, `_set_property`, `_check_properties`, `_get_name`, `_set_context` | members at :2069–2358 (`pu`, `config`, `p`, `context`, `alias`, `name`, `properties`, `config_map`, `log_level`, `cmt`), all within 0..152 |
| `flb_processor_instance_destroy`, :2361–2394 | reads `config_map`, `properties`, `cmt` (under `FLB_HAVE_METRICS`), `alias`, `log_encoder` and `log_decoder`, then `flb_free(ins)` (`BitCast` to `void *`). It frees the whole block and takes no size |

**Conversions.** These are counted over all 48 Clang-read units, and the four
configurations agree:

* 16 `void *` → `struct flb_processor_instance *`;
* 5 the other way;
* 7 null-pointer constants;
* one `(void) ins` discard (`processor_tda/tda.c:1116`).

None converts to or from another record type.

**Byte functions and sizes.** A text sweep of the processor units and callers
finds no `memcpy` / `memset` / `memmove` / `memcmp` on an instance. `sizeof(struct
flb_processor_instance)` appears nowhere in `src/`, `plugins/` or `tests/`.
`sizeof(struct flb_filter_instance)` appears only at :2171 and at
`flb_filter.c:480`, the filter's own constructor.

## Upstream history

The read-only checkout is at `046accf6`: `master` and `upstream/master`.

* `git log --all -G'flb_calloc\(1, sizeof\(struct flb_(filter|processor)_instance\)\)' -- src/flb_processor.c`
  finds two commits:
  * `f3a433c3` (2023-04-12, "processor: initial commit of the proof of
    concept for native processors"). It already allocated `struct
    flb_native_processor_instance` by the filter's size.
  * `7b50ee15` (2023-04-15, "processor: add support for processor plugins").
    It renamed the record to `struct flb_processor_instance` and kept the
    line.
* `git log -L` on the line in `master` names `7b50ee15` only.
* `-S'sizeof(struct flb_processor_instance)'` finds nothing in any ref.

No local revision changes the line. The checkout holds no revision newer than
046accf6 that touches it. The slice export 34155955 is not in this checkout,
but its file is byte-identical.

## What the existing analysis says

The unit is on no audited route: it is not in the profile's `audit.json`, nor
in the system or tail-pause selection. The analysis was run over
`src/flb_processor.c` alone:

* [`scripts/processor-selection.json`](scripts/processor-selection.json) and
  [`scripts/properties.sh`](scripts/properties.sh);
* `quirin-audit current` at 046accf6 (the system tree) and 34155955 (the
  slice export);
* the release binary `87c51c03…`, built from `1c7333db1` by
  [`scripts/build.sh`](scripts/build.sh);
* `QCORE_BOUNDS_PROPERTY` and `QCORE_NULL_PROPERTY` set.

Both runs exited 0, and the two revisions give identical rows
([properties.json](artifacts/properties.json), from
[`scripts/extract_properties.py`](scripts/extract_properties.py)). For
`flb_processor_instance_create` (both objects):

* **Custody: Unknown.** The diagnostics are 8 `unresolved resource identity
  instance` and 10 `indirect memory access is outside the resource
  projection`. The remaining diagnostics name uncontracted `mk_list_init`,
  `flb_processor_instance_destroy` (×2) and
  `flb_log_event_decoder_read_groups`, plus one `nullable resource instance
  used by a non-null contract`.
* **Bounds: Unknown.** All 49 accesses are `bounds-missing-extent`; 19 of them
  are `no extent established for instance`. The bounds vocabulary has no
  allocation byte size: rule [11u](../../agents/gateway.md#constraint-11u)
  says "no current contract establishes one", and `flb_calloc`'s contract in
  `quirin-provider-fluent-bit` states only `forwarded_size` / `forwarded_n`.
  Nothing compares an allocation's size with the pointee record's `sizeof`.
  The vocabulary therefore cannot express "allocation sized by another type".
  This is registered as `BOUNDS-ALLOCATION-SIZED-BY-ANOTHER-TYPE`; its
  prerequisite is `CG17-FLEXIBLE-EXTENT-UNESTABLISHED`.
* **Null: Unknown** (`null-deref-candidate`). The instance's checks are:
  * 13 proved (`tested:` the allocation check);
  * 3 moved;
  * 4 candidates: two `instance->p` reads in `flb_plg_error`, and two
    `instance` dereferences from the null literal at :2203 and :2209. The
    triage leaves these two unresolved.

`flb_processor_instance_destroy` is null Safe and custody Unknown (uncontracted
`flb_config_map_destroy` and `flb_kv_release`, among others).

**Outside the question, recorded and not validated**
(`FB-PROCESSOR-NULL-ROWS-OFF-ROUTE`):

1. **The post-destroy candidates.** After `flb_processor_instance_destroy(instance);
   instance = NULL;` (:2198–2199, encoder creation failed), the function
   continues to `instance->log_decoder = …` (:2203) and to
   `flb_log_event_decoder_read_groups(instance->log_decoder, …)` (:2209).
   Reading the code, this path is reachable only when
   `flb_log_event_encoder_create` or `flb_log_event_decoder_create` returns
   NULL. The null property leaves it a candidate. It is not a defect claim.
2. **A finding in `flb_processor_run`.** `flb_processor_run` carries the only
   null finding (`null-on-normal-path` at :1379). The list is NULL when `type`
   is none of the four `FLB_PROCESSOR_*` values. No caller passing another
   value has been established, so the finding stays a provisional `Unsafe`.

## Reasoning

The claim "under-allocation reachable" needs three things, in a configuration
Fluent Bit builds:

* a processor member at `offset + size > sizeof(struct flb_filter_instance)`;
* that member actually read or written;
* on a path from this allocation.

The largest processor end is 152. The smallest filter size measured is 200,
or 124 against 96 on i686. The reasoning above covers the ABIs that were not
measured. Every access is through the processor's own type. The only other
view of `pu->ctx` is guarded by a `unit_type` written once, at creation.
Nothing copies or clears the object by a size.

So the claim fails. The line is **benign but fragile**:

* it allocates 64 bytes too many on LP64, all zeroed;
* nothing ties the two records, such as a `_Static_assert` or allocating
  `sizeof(*instance)`;
* the margin is 64 bytes on LP64 (48 without the filter's `#ifdef` members)
  and 28 on a minimal i686 build. Growing `struct flb_processor_instance`
  past that, or shrinking the filter record, would make the allocation
  short.

No draft upstream note is written, because the claim did not validate.
Publishing an upstream report, including a hardening suggestion, is the
user's decision.

## Ledger

787 → 790. FB-PLUGIN-DESCRIPTOR-NONNULL-SCOPE, which registers
`FB-PROCESSOR-INSTANCE-ALLOCATION-SIZE`, was not on main when these records
were written. This card therefore registers a sibling for the integrator to
reconcile:

* **`FB-PROCESSOR-INSTANCE-ALLOCATION-SIZE-MEASURED`**: the outcome above.
  Once the original entry lands, retire both.
* **`BOUNDS-ALLOCATION-SIZED-BY-ANOTHER-TYPE`**: the vocabulary residual.
* **`FB-PROCESSOR-NULL-ROWS-OFF-ROUTE`**: the null rows above.

The script is [`scripts/update_ledger.py`](scripts/update_ledger.py), and
`dimensions.named_followups` = 790.

## Gate

Records only, as in the brief: the Python suite, the docs index and its
`--check`, and `build_matrix` (the ledger changed). No product code changed,
so the standing gate is not run. The steps and their exits are in
[`scripts/gate.sh`](scripts/gate.sh); its logs are in `private/fbpia/gate/`.
The results are below in "Gate results".

## Files

* Scripts: `layout.py`, `variants.py`, `census.py`, `census-all.sh`, `summarize.py`,
  `build.sh`, `properties.sh`, `processor-selection.json`, `extract_properties.py`,
  `update_ledger.py`, `gate.sh`.
* Artifacts: `layouts.json`, `variants.json`, `record-layouts.txt`, `probe.c`,
  `access-census-summary.json`, `system-046accf6.census.json`, `properties.json`.
* Private: `private/fbpia/`. This holds every layout and probe output, the
  four censuses, the property runs with their null and bounds streams, and the
  binary copy.
