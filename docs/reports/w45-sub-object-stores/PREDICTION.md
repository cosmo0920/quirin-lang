# W45-SUB-OBJECT-STORES: frozen prediction

Frozen before the rule is wired into the classifier. At this commit the
reader, the instruction, its transfer and the census exist, and
`sub_object_store::WIRED` is `false`, so no route calls the reader.

## Base

Local `main` `b160d765`. The BEFORE run (`before-run.json`) reproduces the
combined baseline (`docs/reports/w44-collection-allocation/integration/artifacts/`)
byte for byte in `diagnostics`, `chunk-contracts` and `source`; `contracts`
after the checkout-root alias; `source-effects` differs only in checkout-path
digests, as every previous card records.

* functions Safe / provisional Unsafe / Unknown: 306 / 12 / 4,516;
* units Safe / provisional Unsafe / Unknown / data-only: 3 / 10 / 168 / 4;
* `leak` 37, `unknown_alias` 3,507, `unsupported` 43,590, `summary_conflict` 1;
* indirect memory access: 17,710 leaves / 3,324 Unknown functions / 388 sole.

## Census (census run of `6bf77d1a`, rule unwired)

`census/store-census.txt`: 10,225 distinct sub-object store statements, 8,167
refused by the gate (6,778 in Unknown functions). Were every gate-refused store
admitted, 61 of the 388 sole-blocked functions would be free. The rule's reader
resolves 1,060 of the 6,778 (`read:*`); the rest keep the refusal for an
unresolved member (2,121: an incomplete record), a chain or nested aggregate
(1,088), an unreadable function (1,072), an unresolved root (472), a prefix
(cast, `(*p)`, 335), or a value shape (399).

## Forecast (`scripts/forecast.py`, `census/forecast.json`)

* **Leaves leaving the indirect family:** at least 106 (`clear`: every base and
  named value never bound), at most 437 (`clear` + `flow`); 2 of the upper
  bound in provisional-Unsafe functions, none in Safe ones. A leaf that the
  flow refuses keeps the gate's refusal byte for byte (the fallback).
* **Function moves Unknown → Safe:** at least the 6 `clear` movers, at most the
  12 upper-bound movers:
  `cJSON.c::suffix_object`, `ctr_encode_opentelemetry.c::destroy_id`,
  `::otel_span_set_name`, `::otel_span_set_trace_state`,
  `ctr_link.c::ctr_link_set_attributes`, `snappy.c::init_snappy_decompressor`
  (clear), and `cmt_decode_msgpack.c::create_{counter,exp_histogram,gauge,
  histogram,summary,untyped}_instance` (flow: a named value stored through a
  formal). Callers that gain a validated W4 summary from a mover may move too;
  that cascade is not forecast.
* **Units:** no unit moves.
* **Sole-blocked indirect family:** 388 → between 376 and 382.
* **Leak pin:** unchanged at 37. No store the rule admits withdraws a leak the
  refusal accused, because the refusal already handed the stored identity on
  (W44-LEAK-PIN-AUDIT); an escape removes an obligation only where the
  refusal had already withdrawn its accusation.
* **Violations:** 0 new. The one violation the rule can produce,
  `release_after_escape` (an identity escaped into the caller's record, then
  released), is forecast absent; any that appears is classified by reading.
* **`unknown_alias`:** may rise where an admitted stored call's own argument
  obligations or a base's C02-P-ALIAS obligation now report (they were masked
  by the refusal); each is attributed.
