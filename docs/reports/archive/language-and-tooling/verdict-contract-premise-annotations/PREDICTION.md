# VERDICT-CONTRACT-PREMISE-ANNOTATIONS: prediction

Frozen before any contract text changed. Base: local `main` at `9564eeb6d`.
BEFORE binary `private/vcpa/bin/base` `289ec7fb…` (built at the base; the card
changes no Rust, so the same binary measures AFTER, and nothing it embeds is
a module this card edits).

## BEFORE (measured)

| route | totals (Safe / Unsafe / Unknown, Pending) | report sha256 | Safe roll-up (none / only validated / declared assumption) |
| --- | --- | --- | --- |
| corpus | 881 / 9 / 3,944 | `96852f4e…` | 418 / 85 / 378 |
| system | 87 / 4 / 357 | `0985ced7…` | 47 / 3 / 37 |
| slice | 45 / 1 / 354, 4 Pending | `f0e8f358…` | 29 / 0 / 16 |

## What changes

Eleven declarations gain exactly one `requires resource_premise(NAME,
"STATEMENT");` each, after their last contract-level marker; no other clause
changes ([`scripts/annotate.py`](scripts/annotate.py) holds the texts).

| premise | declarations | named by |
| --- | --- | --- |
| `synchronous_output_reachability` | `flb_output_task_singleplex_enqueue` | the ledger |
| `ingestion_active` (C3) | `flb_input_collector_resume` | the ledger |
| `nested_call_admission` (N1–N10) | `__errno_location`; `strerror` (N3, N6, N7) | the ledger |
| `cjson_list_invariant` | `add_item_to_array` | the sweep (a comment written `PREMISE`) |
| `cjson_allocator_hooks` | `cJSON_New_Item` | the sweep ("a premise of whoever installed it") |
| `asserted_allocation` | `rd_kafka_toppar_new0`, `rd_kafka_toppar_desired_add`, `rd_kafka_q_new0`, `ut_create_mock_queue` | the sweep ("the non-null results stay declared assumptions") |
| `untracked_handback` (O04-TRACE N6/N8) | `flb_input_chunk_get_name` | the sweep (the route premise beside N1–N10's) |

`AOM-DESTROY-CORRUPTED-UNPAID-PREMISE` is **not** attached: it names
`flb_input_chunk_destroy_corrupted`, a function deliberately left undeclared
(payoff zero), not an assumption `flb_input_chunk_destroy` or
`flb_input_chunk_destroy_direct_routes` rests on. Attaching it would put a
false statement on every row resting on those two declarations. Neither
module changes; the ledger's wording is a correction in the report.

## Predictions

1. **Control.** After the edit and before the re-pin, `pin --check` reports
   each of the nine pinned changed declarations as `contract-changed` (not
   in force) at every revision it was reviewed at (99%).
2. **Re-pin.** `pin` at `66910c10` and `046accf6` for all nine and, for
   `flb_input_collector_resume` only, at the slice export `34155955`, where its
   lock already holds an identity: without it the declaration would leave
   force on the slice (it is `pinned-in-force` there, 2 rows). Then every
   changed crate's `--check` reports them recorded at each revision, and
   `--verify-contracts` exits 0 (95%). The libc pair is flexible: its lock
   entries take the new digest only.
3. **Identity.** No verdict moves on any route: verdict-row digests and the
   rows' `contract_dependencies` fields equal; parity multisets identical
   (in-order identity not expected across `--jobs 4` runs, as the previous
   card measured); totals unchanged (97%). The corpus report is
   byte-identical (`96852f4e…`), since the pinned route names no contract
   digest in a row or unit (90%). The system and slice reports differ only in
   the per-unit contract-source digests the current route records (90%). The
   sidecars differ only in the six modules' texts and digests, three locks'
   digests, the eleven declarations' `contract_sha256` and their `premises`
   (95%).
4. **Safe rows naming each premise** (= each declaration's Safe rows in the
   BEFORE section, since no row moves):

   | premise | corpus | system | slice |
   | --- | ---: | ---: | ---: |
   | `synchronous_output_reachability` | 0 (not consulted) | 0 (0 rows) | 0 (not consulted) |
   | `ingestion_active` | 0 (not consulted) | 0 (0 rows) | 0 (2 rows) |
   | `nested_call_admission` | 1 (`__errno_location` 1, `strerror` 0) | 6 | 3 |
   | `cjson_list_invariant` | 1 | 0 | 0 |
   | `cjson_allocator_hooks` | 7 | 0 | 0 |
   | `asserted_allocation` | 0 (51 rows) | 0 | 0 |
   | `untracked_handback` | 1 | 2 | 0 |

   A Safe row resting on two of the declarations is counted under each.
5. **Lean.** None: the clause changes no mode and no verdict reads it
   (C04.18); `QCore.ContractDependencies` already proves that publishing the
   dependency record changes no verdict.
6. **Pins.** `qcore_contract_corpus.rs`: eleven clause rows +1; no module or
   declaration count moves. The librdkafka package inventory changes
   (`contracts.py sync`); no published module changes, so
   `contract-load-path.json`'s module pins do not move (90%).
