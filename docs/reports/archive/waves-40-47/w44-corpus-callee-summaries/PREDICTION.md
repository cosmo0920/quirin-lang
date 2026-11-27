# W44-CORPUS-CALLEE-SUMMARIES: prediction, frozen before implementation

Card W44-CORPUS-CALLEE-SUMMARIES, 2026-09-25, base `c1e69bb5`. This file is
written and committed before the analyzer's derivation scope changes. It is not
edited after that commit; the report compares it with the outcome.

The census behind it is measured, not estimated: a private capture run of the
base binary (diagnostics byte-identical to BEFORE) supplied every unit's
line-marked preprocessing, and a census build of the committed instrumentation
(`indirect-census` feature, `QCORE_CALLEE_SCOPE_CENSUS`; diagnostics
byte-identical to BEFORE) re-ran the W4 derivation over the widened scope on a
private copy and recorded what it decided for every callee, without installing
anything. Evidence is under [census/](census/).

## Census: why no summary reaches each uncontracted call

Population: the `call to uncontracted function` leaves of Unknown functions at
the baseline, **9,547 leaves / 2,591 functions / 407 blocked only by
uncontracted calls** (302 by exactly one callee). Each leaf is classified by
where the callee's body is, seen from the calling unit
([`scripts/callee_census.py`](scripts/callee_census.py)).

| Class | Leaves | Callees | Unknown fns | Sole (one callee) | Why no summary |
|---|---:|---:|---:|---:|---|
| same unit, selected, body `Unknown` | 3,594 | 1,566 | 1,633 | 219 | W4 already tries it; its own body does not validate |
| other pinned unit, body `Unknown` | 2,841 | 695 | 1,005 | 36 | no cross-unit route, and the body would not validate anyway |
| same unit, header `static` definition, body does not validate | 1,257 | 155 | 457 | 12 | **route gap** (outside derivation scope) *and* the body is not modeled |
| no body in any pinned unit | 950 | 217 | 361 | 23 | only a declaration can decide it (`cfl_sds_*` 170+, `__builtin_va_*`, `vsnprintf`, OpenSSL, `mpack_start_*` …) |
| same unit, system-header body (`__bswap_16/32/64`, `__uint64_identity`) | 586 | 4 | 97 | 3 | W44-LIBC-RESIDUALS; **untouched by this card** |
| other pinned unit, body `Safe` | 104 | 35 | 88 | 2 | **no cross-unit summary route** |
| same unit, header definition of external linkage (`mpack_*` C99 `inline`) | 86 | 16 | 39 | 0 | the call may run the external definition (C11 6.7.4p7) |
| same unit, selected, body `Unsafe` | 58 | 38 | 30 | 4 | a defective body validates nothing |
| **same unit, header `static` definition, body validates** | **41** | **16** | **38** | **2** | **route gap only: this card** |
| other pinned unit, body `Unsafe` | 30 | 19 | 17 | 1 | as above |

Of the 1,257 header leaves whose body does not validate, 378 (20 callees) are
blocked only by further uncontracted calls, and 879 (135 callees) carry some
other refusal ([`scripts/header_reasons.py`](scripts/header_reasons.py)).

**Destructor shape.** 230 callees whose names read as releases (`destroy`,
`free`, `release`, `unref`, `clear`, `fini`, …) own 1,175 of the same-unit and
other-unit leaves. Only 8 of them (53 leaves) have a body blocked by
uncontracted calls alone; every other one carries indirect access, a loop or
branch condition, or an unresolved identity in its own body
([`scripts/body_reasons.py`](scripts/body_reasons.py)). A "releases its formal
and nothing else" summary cannot be *proved* of such a body: an unmodeled
statement may release or publish anything. The card therefore does **not**
admit a destructor summary; it sizes the population and registers it.

**Cross-unit.** Only 104 leaves (35 callees, 2 sole) have a `Safe` body in
another unit. A cross-unit route needs certificate-bound identity (source
digests) for a summary validated in another unit's environment; the yield does
not justify it in this card. Registered with its size.

## What the card will add

The W4 **derivation scope** under main-source selection widens from the
selected definitions to the selected definitions **plus** every definition they
reach through direct calls that is (1) unselected, (2) of internal linkage —
`static` and not `extern` on the definition itself — and (3) not
implementation-reserved (`__x` or `_X`, C11 7.1.3). The reported rows, the
effective entry and the whole-unit `Safe` scope do not change. A newly admitted
definition gets a summary only through the unchanged derivation and
`observations_match_contract`; declared contracts still win. Lean:
`QCore.CalleeSummaryScope`.

## Expected movers

From [census/prediction.json](census/prediction.json)
([`scripts/predict.py`](scripts/predict.py)).

* **Validated header summaries at leaf sites: 16 callees**, deciding **42**
  leaves: 41 in Unknown functions and 1 in a provisional-Unsafe function
  (`mk_sched_add_connection`, `mk_sched_conn_timeout_add`,
  `Owned{nonnull}, Borrowed`).
  * 13 of the 42 call an **effectful** summary: `rd_kafkap_str_destroy` (8) and
    `rd_kafkap_bytes_destroy` (4), both `Released{nonnull}`, and the
    `mk_sched_conn_timeout_add` site. A modeled call still checks its
    arguments, so each of these 13 leaves is predicted to be **removed or
    replaced on the same line** (most likely by an `unresolved resource
    identity` leaf). The other 29 call all-`Borrowed`/`None` summaries and are
    predicted to be **removed**.
* **Function moves: exactly 2, Unknown → Safe**, both neutral:
  `mk_event_init` and `mk_event_backend` in `monkey-libevent/mk_core/mk_event.c`
  (each carries one leaf, naming `_mk_event_init` / `_mk_event_backend`, whose
  bodies validate `() -> None`). No other function's last blocker is decided.
* **Transitive movers: 0.** Neither mover is called by a function whose only
  remaining blockers they are.
* **Units: unchanged** (`mk_event.c` keeps 12 other Unknown functions).
* **Declared-mode gate: no withdrawal.** The widened derivation compares 95
  header rows against a declaration: 93 disagreements, every one the known
  nullability refinement of `rd_free` (66), `mk_mem_free` (26) and `flb_free`
  (1), and 2 agreements (`flb_malloc`, `flb_calloc`). No `mode-*-contradiction` leaf
  appears, and `summary_conflict` stays at **1** (a conflict for an unreported
  header definition emits no diagnostic).
* **`__bswap_*` unchanged:** 586 leaves remain.

## Expected totals

| Measure | Baseline | Predicted |
|---|---:|---:|
| Functions Safe / provisional Unsafe / Unknown | 294 / 85 / 4,455 | **296 / 85 / 4,453** |
| Units Safe / Unsafe / Unknown / data-only | 3 / 43 / 135 / 4 | unchanged |
| `call to uncontracted function` leaves (Unknown fns) | 9,547 | **9,506** (−41) |
| `unsupported` | 45,848 | 45,806 (−42), plus any same-line `unsupported` replacement |
| `unknown_alias` | 4,358 | 4,358 to 4,371 (+0 to +13 same-line replacements) |
| `leak` / `summary_conflict` | 271 / 1 | **271 / 1** |
| `double_release` / `invalid_release` / `use_after_release` | 0 / 0 / 0 | 0 / 0 / 0 |

The leak pin stays at **271**: the two movers acquire nothing, and a
`Released` summary at the other sites can only discharge or refuse.

## Stated risks

* A caller of an effectful summary could turn provisional `Unsafe` if the
  modeled release exposes a validated violation. Predicted: none, because every
  effectful site sits in a function with other refusals. Any such move is
  attributed, not absorbed, and is never an upstream defect claim.
* The certificate checker re-derives the widened scope. The three `Safe` units
  (`cmt_time.c`, `ctr_version.c`, `tls/tls_stub.c`) are predicted to
  re-certify unchanged.
