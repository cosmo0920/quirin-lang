# BODY-SUMMARY-REUSE

Card base `13337d0af` (origin/main). Brief: round 23 card B. The user's
`current` run at Fluent Bit 046accf6 refuses calls to callees whose own
bodies are `Safe` in every configuration as `call to uncontracted function`.
The card asked for a census of why, then a reading that lets such a body
supply a call summary.

Status: **the census is done, and the reading stopped on a semantic
choice** (see "Stop"). No analyzer, contract, provider or Lean file changes.
[PREDICTION.md](PREDICTION.md) predicts no move on any route.

## Re-measurement

The census was re-measured with the base binary (`cargo build --release
--locked -p qcore-audit` at `13337d0af`, `quirin-audit` SHA-256
`cecc9a8d…6a33c4`). The arguments were the user's (`resource-diagnostics.invocation.json`),
with this worktree's profile and the outputs written to a private directory
([`scripts/run_current.sh`](scripts/run_current.sh), audit slot at 4 GiB,
one worker). Exit 0 after 1 h 39 min, peak RSS 7.5 GiB, which is above the 4 GiB
admission. The report has SHA-256 `1b1d879c…c47c42f`, with units 10 Safe / 8
provisional Unsafe / 164 Unknown / 4 data-only and functions 1,066 / 10 /
3,836. It equals the user's own fresh run of the same code (reports
written 16:37 JST, 2026-11-06) row for row: 4,848 rows, no status and no
diagnostic-leaf difference. Against the 2026-11-05 run (main 945c0db1b
binary), 4 rows are newly Safe and 128 rows trade leaves. None of those
changes touches this population, so the census is byte-identical on all
three reports.

## Census

Population, computed by [`scripts/classify.py`](scripts/classify.py) and listed
per site in [`census.json`](census.json): every
`call to uncontracted function F` leaf on a non-Safe row where F has an
analysed body and every row of F is `Safe`. There are **165 callees, 456 sites,
307 functions, 13 sole-blocked**. Every callee has exactly one defining
configuration, so the brief's example question (whether a body proved Safe
in only some configurations may be used in another) does not arise in
this population.

| Reason the Safe body yields no call summary | Sites | Callees | Functions | Sole-blocked |
|---|---:|---:|---:|---:|
| Cross-unit: external-linkage body in another unit; rule 4c is off on `current` (no link-resolution premise) | 438 | 156 | 295 | 7 |
| Same unit, external linkage: rule 4a keeps it out of W4's derivation scope (C99 inline bypass, interposition) | 7 | 5 | 7 | 0 |
| Same unit, internal linkage: W4 publishes no summary whose effects the call reads (below) | 11 | 6 | 7 | 6 |
| **Total** | **456** | **165** | **307** | **13** |

The calls take these source forms: statement call 246; assignment or
initializer 168; `return CALL(...)` 24; nested in another call 7; a macro
line 8; other expression 3. Condition and loop calls are another family,
and this population has none. The cross-unit callees with the most sites are
`rd_kafka_error_destroy` 41, `cio_error_reset` 18, `rd_kafka_confval_get_int`
17, `ctr_attributes_destroy` 15, `rd_kafka_topic_partition_destroy` 14,
`mk_string_copy_substr` 11 and `cJSON_GetObjectItem` 10.

Seven of the population's callees, at 56 cross-unit sites, are already
named by the pinned corpus profile's eight-home premise
(`contract/evaluations/eight-home/link-map.json`): `rd_kafka_error_destroy`,
`rd_kafka_error_code`, `cio_file_hash`, `cio_file_is_up`, `mk_fifo_send`,
`LZ4_sizeofState` and `LZ4_sizeofStateHC`. They are Safe imports on that
route, but on `current` they reach no caller.

The sole-blocked functions are:

- Cross-unit: `ctxTypeID_to_size` (LZ4_sizeofState, LZ4_sizeofStateHC),
  `cio_chunk_hash` (cio_file_hash), `cio_chunk_is_up` (cio_file_is_up),
  `mk_mq_send` (mk_fifo_send), `rd_kafka_share_consumer_closed_err`
  (rd_kafka_error_code, rd_kafka_error_destroy), `net_plain_writev`
  (mk_iov_send) and `rd_kafka_transport_ssl_threadid_cb` (thrd_current_id).
  The first five rest on eight-home names.
- Same unit, internal linkage, read from the source (not confirmed by a parity
  probe, which was stopped at the integrator's request; see "Observed
  defect"):
  - `rd_kafka_CreateAcls_result_acls`, `rd_kafka_DescribeAcls_result_acls`,
    `rd_kafka_DeleteAcls_result_responses` and
    `rd_kafka_ElectLeaders_result_partitions` call static helpers that store a
    count through an out-parameter and return an interior pointer of a
    formal (`rko->rko_u.admin_result.results.rl_elems`). That is a derived
    result plus a scalar out store, effects a W4 summary does not carry
    (`W43-DESCRIPTOR-W4-SUMMARIES`, rule 12y's borrowed-result inference).
    They are `return CALL(...)` with a cast or nested actual.
  - `mk_sched_threads_purge` and `mk_sched_threads_destroy_all` call
    `sched_thread_cleanup`, which destroys every element of a formal's list.
    Rule 10f(3) says such a body publishes no summary.
  - Not sole-blocked: `resolve_data_point_start_timestamp` (4 sites) stores a
    scalar through its out-parameter and forwards it to
    `kvlist_fetch_uint64`.

## Existing routes (not duplicated)

- **Rule 4 / W4** (`legacy_summary.rs`) applies a validated in-unit
  summary at a same-unit call. This is the reading scope item 2 describes,
  limited to one translation unit and to effects W4 can state.
- **Rule 4a** admits only internal-linkage definitions to W4's
  derivation scope.
- **Rule 4c** (`legacy_cross_unit.rs`, `qcore-audit/src/cross_unit_links.rs`,
  `QCore.CrossUnitSummary`) lets a Safe home's validated W4 summary cross a
  unit only under an explicit linked-program resolution premise
  (`QCORE_CROSS_UNIT_LINK_MAP`). It also requires:
  - a unique home with no static or inline declaration and no declaration in
    force;
  - pinned home identities and two-sided certificate replay;
  - a separate fail-closed import channel.

  Callers list the import among their `contract_dependencies` (rule 13f), and
  the home's identities are bound in the caller's trust context, so a changed
  home re-audits its callers. This is scope item 2's evidence-identity
  requirement, and it is already built. Imported-home chains stay refused
  (one hop).
- **Rules 10f(3), 10v, 11f and 12t** and `W43-DESCRIPTOR-W4-SUMMARIES` cover
  bodies whose effects a W4 summary cannot state: they publish no summary.

The 445 sites that involve external linkage (cross-unit or same-unit)
therefore wait on one question: **on what standing may a call to an
external-linkage function be read through its analysed body?** Rule 4a
says no for every route except rule 4c's stated premise. Neither the brief
nor fail-closed settles which standing the `current` route should take.

## Stop: semantic choice for the user

Every reachable site needs a statement that the call runs the analysed body:
compatible type, calling convention and no interposition. Fail-closed rules
out reading the body without such a statement (rule 4a). Which statement
to accept is the user's call:

- **A. Derive the link resolution from the linked program** (new card). The
  prepare step links the Fluent Bit executable. The census then resolves a
  call only when:
  - one strong global definition from the home unit's object is the
    executable's single definition of the name;
  - the image is an executable, not a shared object, so its own definitions
    are not preempted;
  - Clang's declared type at the call matches the home definition.

  The result feeds rule 4c's existing import route, with the link
  artifact's digest in the assumptions in place of a human map. Lean extends
  `QCore.CrossUnitSummary` with a derived resolution. The residual premise
  is that the deployed binary is the built one. A shared-library build
  (`libfluent-bit.so`) resolves nothing. Cost: the production prepare
  currently links nothing (`fluent-bit-clang-production/bin/` has only
  `buildvm` and `minilua`), and the rule 4c route must first scale (see
  "Observed defect").
- **B. A stated link map on `current`** like eight-home, naming the 156
  cross-unit homes. It needs no analyzer change: every row resting on an
  import is Safe only conditionally on that premise. This conflicts with the
  standing decision that observed facts are recorded "never as new named
  premises" unless the user overrides it for link resolution, as the
  eight-home profile already did for measurement.
- **C. Declared, body-checked contracts** in the provider crates for the
  reachable callees, ranked by sole blockers and then sites. Rule 13g
  validates each declaration against its home body, and the declaration is
  recorded as a checked contract. This follows the precedent of
  CG10-INDIRECT-ACCESS-SOLE-RESIDUALS (five cross-unit Safe homes declared
  instead of a route) and MONKEY-VHOST's `mk_string_dup`. It also covers the
  7 same-unit external sites, which A and B cannot reach (rule 4a and the
  caller's local precedence in `for_caller`). It is out of this card's scope
  (new contracts) and is per-callee work.
- **D. Leave it.** The census stays as the record.

**Recommendation: A** as the route, with **C** for the seven cross-unit sole
blockers in the meantime. A turns the premise into a checked fact and reuses
rule 4c unchanged. It scales to every future Safe body without
per-callee declarations, and it keeps the standing decision. C gives the
immediate gain, but each declaration is a reviewed artefact whose
name-keyed application carries the same implicit link assumption A would
check. B is the cheapest option and moves the same rows as A, but only
conditionally.

Predicted effect on the user's `current` run (an upper bound read from the
census; the experiment that would measure it did not finish):

- **A or B:** at most 438 sites leave the uncontracted list (156 callees),
  and at most 7 functions lose their only blocker. Five of those 7 were Safe
  under the same imports on the corpus profile (W44/W48 reports). Callers
  can still expose deeper leaves: on the corpus, 38 rows traded leaves.
- **C:** the same, plus the 7 same-unit external sites.
- **No option** reaches the 11 same-unit internal sites or their 6
  sole-blocked functions. Those need summary vocabulary: a derived result
  with an out store, and a container-element release.
- No unit status is predicted to change. Every affected unit has other
  Unknown rows.

## Observed defect: the rule 4c route did not finish on 165 homes

The experiment (`link-map-experiment.json`, labelled EXPERIMENT ONLY, never
adopted) ran the same binary and arguments with
`QCORE_CROSS_UNIT_LINK_MAP` naming the population's 165 homes (audit slot 3,
7 GiB, three workers). The integrator observed at about 11:30 UTC:

- elapsed 10 h 18 min and 25.6 h CPU;
- console stopped after "member facts pre-pass: 1448 unit(s), 1095 read,
  2 unread";
- three worker threads at 100 % CPU for 7 to 8 h, with no `rchar`/`wchar`
  change over 30 s, no major faults and no swap;
- RSS 14.4 GiB against its 7 GiB admission.

It stopped the rust-suite gate's admission drain and every queued audit. The
card stopped it by its captured PID (exit 143, peak RSS 14.4 GiB;
`/proc/<pid>/io` at stop: rchar 230,970,537,276, wchar 217,222,863,036). The
same binary without a map completed in 1 h 39 min on one worker, as did the
user's run. The console prints nothing until every phase ends, so whether
this was a spin or the route's two-phase home replay at 165 homes (W44
already measured the route as costly at five homes) cannot be told from it.
The follow-up is `CURRENT-RUN-WORKER-SPIN`. Any of options A and B needs it
closed first.

## Records

- [`census.json`](census.json): totals, table, call shapes and every site.
- [`scripts/`](scripts/): `classify.py` (census), `mkmap.py` (the
  experiment map), `run_current.sh` (the invocation).
- [`integration-amendments.json`](integration-amendments.json).
- Private artefacts: the base run and the stopped map run under
  `private/census/` in the card worktree.
