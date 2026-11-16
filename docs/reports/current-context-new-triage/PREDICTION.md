# CURRENT-CONTEXT-NEW-TRIAGE: prediction

Frozen before the implementation commit. Base `0f6e17118`; measurement side
`baseline:497bdeb90`.

**Disclosure.** The gateway change (`legacy_transfer.rs`: the B08 owner
refusal hands on its stored value through
`refused_operands::withdraw_refused_call`), the fixtures
(`crates/qcore-c-gateway/tests/refused_store_hand_on.rs`) and the Lean module
`QCore.RefusedStoreHandOn` were drafted in the worktree before this file was
committed, and the unit-scoped reproductions below ran a binary built from
that draft (`private/repro/quirin-audit-tip`) beside the base binary
(`private/repro/quirin-audit-base`). No live-route measurement (corpus,
system, slice) was run.

## Mechanism and its reach

The change runs only inside B08's owner-refusal branch, which reports
`UnknownAlias OWNER` and returns. It only adds sites to
`unresolved_effect_sites` (withdraws), and the refusal stays on the node.
Hence, for any route:

* a `Safe` row carries no `UnknownAlias`, so it never reaches the branch:
  unchanged;
* an `Unknown` row has no leak leaf to withdraw and keeps its refusal: stays
  `Unknown`;
* only an `Unsafe` row that carries an `UnknownAlias` can move, to `Unknown`,
  and only if the withdrawn site carried its every violation;
* a function with a refusal derives no summary, so no caller moves.

## Do the live routes read `tls_openssl.c`?

No. The corpus's Monkey is the standalone `monkey-build` database
(`build/cycle19/c-gateway-wave-pin/monkey`, no `tls_openssl.c` entry); the
system selection (`system-selection.json`) and the tail-pause slice select no
`lib/monkey` file.

## Census of the rows that can move, at `baseline:497bdeb90`

`Unsafe` rows carrying an unresolved identity (`private/repro/candidates.py`):

| Route | Rows | Their violations | B08 owner-refused store of a tracked value? |
|---|---|---|---|
| corpus | `decode_statsd_message`, `ctr_random_get`, `unittest_http_get_params_append`, three `ut_sasl_oauthbearer_oidc_*`, `rd_kafka_snappy_java_uncompress`, `mk_http_error`, `mk_net_conn_create` | leaks / one double release | no (unresolved identities are reads, literals and scalar member stores) |
| system | `flb_output_task_flush`, `flb_output_task_singleplex_enqueue` (both configurations) | `counted_user` | no (`config->flush_in_flight++`, reads of `retry`, `out_flush`) |
| slice | `flb_tail_file_budget_destroy` | `counted_user` | no (`budget->users--`) |

The same files (Fluent Bit `046accf6` revisions) through the unit-scoped
`current --monkey-linked` reproduction, base binary vs draft: every row equal
except `context_new` (`private/repro/cand-{base,tip}`, 181 rows, 35 units;
`statsd-{base,tip}` for `cmt_decode_statsd.c`).

## Predicted moves

* corpus, system, slice: **none**.
* The user's `current` with `--monkey-linked` (integrator rerun after
  landing): exactly one function row moves,
  `monkey::lib/monkey/tls/tls_openssl.c::context_new` `Unsafe -> Unknown`
  (its two leak leaves, `:533` and `:541`, acquired at `:521`, withdrawn; its
  `UnknownAlias thctx`, `UnknownAlias ctx_head` and uncontracted-call
  refusals remain). The unit `lib/monkey/tls/tls_openssl.c` moves
  `Unsafe -> Unknown` (`leak` 2 -> 0). No other row of the evidence run's
  nine `Unsafe` rows with unresolved identities moves; the other 26 rows of
  the unit are unchanged (census table in the README). No callee leaves the
  uncontracted list and no function leaves the "only uncontracted" set: the
  change touches no contract. Run-level: `Unsafe` -1, `Unknown` +1, `leak`
  -2.
* No `Safe` row moves anywhere.

```measure-prediction
route corpus
none
route system
none
route slice
none
```
