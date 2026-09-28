# W47-SERVICE-PROPERTY-KEYS: the keys a property setter carries

Card W47-SERVICE-PROPERTY-KEYS (wave 47, card-high), amending gateway rule
**10p**. Base: local `main` at `ec4c9d4a` (the worktree's HEAD was already
there). The frozen prediction is [PREDICTION.md](PREDICTION.md), commit
`d3a1ab7f`, made after BEFORE was measured and before any census, typed
reading, contract or Lean change.

Nothing here is a verified race, a verified bug, a verified fix, an `Unsafe`
claim or an upstream defect claim. Every old-vs-fixed statement has the form
*discharged / not discharged / unresolved at this path under these derived
premises*. The Fluent Bit implementation and test are unchanged.

## 1. Result

At the `90978ac6` export (configured reading), the census now reads the fixed
test's `flb_service_set(ctx->flb, "Flush", …, "Grace", …, "Log_Level", …,
"Parsers_File", …, NULL)` (`tests/runtime/in_tail.c:218`) as reaching only
its four keys' branches: two scalar stores (`flush`, `grace`), the
`Log_Level` handler and the `Parsers_File` handler. None of them registers a
plugin, so the call is no longer a second hazard-reaching call on the
list-head path. The creation `flb_input(…, "tail", …)` at `:226` is the only
one.

The three sites (`cb_tail_pause_test`, `:2483`, `:2492`, `:2494`) stay
**unresolved**. One blocker remains: the path's closure has exactly 2
functions without a typed reading, and an escaped function reaching a hazard
may be called through a pointer from it. Both functions are Windows-only
definitions in compiled files, under groups guarded by compiler-predefined
macros that the configured reading does not own and so keeps undecided:

- `localtime_r` (`lib/monkey/mk_core/mk_utils.c:41`, `#elif defined
  (_WIN32)`), reached from the property function's `flb_warn` macro
  (`flb_log_print > log_render_message > log_prepare_header`);
- `dirname` (`src/config_format/flb_cf_yaml.c:550`, the `#else` of `#ifndef
  _WIN32`), reached through the `Parsers_File` handler.

A diagnostic reading (§5) that owns `_WIN32`-prefixed macros derives the
instance and discharges the three sites. The diagnostic is not a result; the
residual is registered as `W47-LOOP-OWNER-PREDEFINED-MACRO-ARMS`.

The raw reading stays held first by `src/flb_config.c` being unstructured
unconfigured, so the key reading refuses there. At `90978ac6~1` the old
test's paths stay **not discharged** (T), and both of its census readings are
byte-identical to BEFORE. No site moves at either revision.

## 2. Mechanism

**Typed facts** (`typed_dispatch_census.rs`, thin additions). Two new fact
kinds, recorded only for listed names:

- a call's argument list from a listed index on, one descriptor per argument:
  `s:TEXT` a string literal after macro expansion and concatenation, `null`
  the integer constant `0` under a `NullToPointer` cast, `i:N`, `v:NAME`,
  `offsetof`, else `?`. The listed calls are the four setters
  (`flb_service_set` from argument 1; `flb_input_set`, `flb_output_set`,
  `flb_filter_set` from 2) and `strncasecmp` inside
  `flb_config_set_property`;
- a listed file-scope array's initializer, one descriptor list per element
  (`service_configs`).

**The keyed setter reading** (`loop_owner_property_keys.rs`, a child module
of `thread_entry_census.rs`, one hook in `loop_owner_instance.rs`'s region
scan). A call of a listed setter on Step B's list-head path reaches only its
keys' effects when every part below is derived; otherwise the whole call is
kept, as before:

1. **Arguments.** The typed reading records exactly one list at the call's
   line, and the line holds exactly one lexical call. The list is
   NULL-terminated, has an even number of arguments before the null, and has
   no other null. Every key is a string literal of printable ASCII without a
   backslash or a quote. Values are not read. No key's reach depends on its
   value, because each branch is read whole: a file-path handler contributes
   its whole closure, whatever file the value names.
2. **The setter's body.** `va_start(VA, LAST)` names the last named
   parameter. There are exactly two `va_arg(VA, char *)`: the loop condition
   `while ((KEY = va_arg(…)))` and the loop's first statement
   `VALUE = va_arg(…);`. `KEY` and `VALUE` are locals assigned only there,
   with their addresses never taken. `KEY` is one whole argument of exactly
   one call, inside the loop: that call's callee is the property function.
   There is no jump and no `va_copy`.
3. **The key table.** The property function walks one table:
   `char *KV = TABLE[0].KF; while (KV != NULL) { if (CMP(KV, K, LEN) == 0)
   {…} KV = TABLE[++I].KF; }`, with `LEN = strnlen(K, N)`. `KV`, `I` and
   `LEN` are assigned nowhere else. The typed reading gives each element's
   key (after macro expansion) and type enumerator. The configured lexical
   reading gives each element's `offsetof(struct R, FIELD)`. The two must
   agree element for element (count and enumerator), and only the last key
   may be null.
4. **The comparison**, read from `CMP`'s body, which must be exactly
   `size_t L; L = strnlen(A, N1); if (strncasecmp(A, B, C) == 0 && L == C)
   { return 0; } return -1;`. A table key `e` matches a key `k` when, with
   `n = min(|k|, N)`, their first `n` bytes agree ignoring ASCII case and
   `min(|e|, N1) = n`: that is, the whole key, ignoring ASCII case. Keys must
   be shorter than both bounds. A key matched by no entry keeps every branch.
5. **The branch chain.** The matched block holds one
   `if (!strncasecmp(KV, C, M)) {…} else if … else {…}` chain. Every arm is a
   block, and each constant `C` and bound `M` are read by the typed reading.
   An entry takes the first arm whose comparison its own key satisfies, else
   the final `else`. Every entry a key matches contributes its arm (the union,
   so first-match order is not relied on). Everything outside the chain's
   arms is common to every key.
6. **The scalar store.** The final `else` is a
   `switch (TABLE[I].TYPEF)` whose cases store
   `*(T *)((char *)config + TABLE[I].OFF)`. An entry reaching it is admitted
   only when three things hold:
   - its `offsetof` names a field of `struct flb_config` other than
     `inputs` or `in_plugins`;
   - that field is declared once there with exactly the store's type
     (`unsigned`/`signed` ignored), so the store stays inside the field;
   - no such pointer arithmetic stands anywhere else in the function.

   This closes, for the setter, the one write the typed member-access reading
   cannot see: a store through an offset.

The call then reaches every function named in two places:

- the setter's body;
- the property function's body outside the arms no key selects.

A name counts when it is called, used as a value, or named by an invoked
macro's replacement list (through nested macros). Two functions are read in
place: their own hazards count, and their unmodelled calls join the
closure's. A field dispatch, an undecided conditional directive inside a read
body, or a hidden definition refuses. The census prints the derived table and
each key's reach per call (`typed.property_keys`).

`flb_input_set` (`:2546`) is also on the path. Its property function is not
table-driven, so its keys are not read. Its whole closure reaches no hazard,
as before, so nothing depends on reading it.

## 3. The derived key table

At the export, under the production build, `service_configs[]` has 53 keys
and the NULL terminator. `FLB_HAVE_HTTP_SERVER`, `FLB_HAVE_STREAM_PROCESSOR`
and `FLB_HAVE_CHUNK_TRACE` entries are in; `FLB_SYSTEM_WINDOWS` is out.
Archived in full: [artifacts/key-table-configured.txt](artifacts/key-table-configured.txt),
from `typed.property_keys` of the configured fixed reading.

| Effect | Keys |
|---|---|
| keyed arm 1, `Log_Level` (`strncasecmp` bound 256, `src/flb_config.c:869`): `set_log_level_from_env`, `flb_env_var_translate`, `set_log_level` | `Log_Level` |
| keyed arm 2, `Parsers_File` (bound 32, `:895`): `flb_env_var_translate`, `flb_parser_conf_file` | `Parsers_File` |
| keyed arm 3, `Plugins_File` (bound 32, `:903`): `flb_plugin_load_config_file`, which registers into `config->in_plugins` (the hazard) | `Plugins_File` |
| scalar store, `double` | `Flush`, `flush.adaptive.{min,max}_interval`, `flush.adaptive.{low,medium,high}_pressure` |
| scalar store, `int` (`INT` and `BOOL`; `unsigned int` fields `coro_stack_size`, `sched_cap`, `sched_base` included) | 31 keys, e.g. `Grace`, `Daemon`, `HTTP_Server`, `storage.max_chunks_up`, `Hot_Reload.Timeout` |
| scalar store, `char *` (the old string freed, the translated value duplicated) | 13 keys, e.g. `Log_File`, `HTTP_Listen`, `dns.mode`, `storage.path`, `Streams_File` |

Every entry is classified; none is refused. Keys compare as `prop_key_check`
compares them: the whole key, ignoring ASCII case (`flush`, `GRACE` and
`plugins_file` match). Each value matters only inside its branch, which is
read whole. Two points follow from the table:

- only `Plugins_File` reaches the registration hazard;
- `Streams_File` is only a stored string in this version.

## 4. Negative controls (fixtures first)

`crates/quirin-source-analysis/tests/loop_owner_property_keys.rs` has 12
tests. The fixtures are the instance-flow fixture with the export's shapes:
the variadic `flb_service_set` body, a `service_configs` table,
`prop_key_check`, `flb_config_set_property` with its chain and switch, and a
`Plugins_File` handler calling `mk_list_add(…, &config->in_plugins)`. Each
fixture is compiled by Clang and read by the census.

| Control | Outcome |
|---|---|
| `"Plugins_File"` passed literally | the key is read (keyed arm 3); **route kept**: `flb_plugin_load_config_file` is a second hazard-reaching call |
| `"PLUGINS_file"` (a case variant) | **route kept**, the same way |
| a key from a variable (`name`) | **every branch**: the key of pair 1 is not a string literal |
| an odd list (`"Flush", "1", "Grace", NULL`) | **every branch** |
| no terminating `NULL` | **every branch** |
| an unknown key (`"No_Such_Key"`) | **every branch**: it matches no entry |
| a prefix of a key (`"Flus"`) | **every branch**: the comparison is over the whole key |
| a table entry whose offset names `in_plugins` | **every branch**: the entry cannot be classified |
| a `double` entry on the `unsigned int` field `grace` | **every branch**: the store is wider than its field |
| the old test's shape (collector operations from the test thread) | **not discharged** (T) |
| the fixed call (`Flush`, `Grace`, `Log_Level`, `Parsers_File`) | read key by key; **discharged** on `(flb_lib_worker, NonThreaded)` and `(input_thread, Threaded)` |
| lower- and upper-case variants of scalar keys | matched as the comparison folds case; **discharged** |

## 5. Census: `90978ac6~1` against `90978ac6`

The census reads the same 1,685 files and the same four readings
([`census.py`](../w47-loop-owner-typed-dispatch/scripts/census.py)).

**BEFORE.** It used the base binaries (`loop_owner_census` `6966df12…`,
`typed_dispatch_facts` `7088ace3…`) and regenerated facts, and it reproduced
the baseline archive byte for byte on all four SHA-256s (`6ea187e4…`,
`d6475e38…`, `d05ebc0e…`, `a5edf1ed…`).

**AFTER.** Final binaries `c62530e2…` / `4e65c048…`. The facts gain the two
fact kinds (`library.json` `75f252d7…`, `fixed-test.json` `35dabc49…`); every
earlier kind is unchanged. The census, facts, compare log and derived tables
are archived in [artifacts/](artifacts/)
([`compare_census.py`](scripts/compare_census.py)).

| Reading | sites discharged / not / unresolved | edges | seeds | JSON |
|---|---|---|---|---|
| `90978ac6` configured | 8 / 130 / 10, unchanged | 33,573 | 2,854 | the fixed test's instance-flag reason and `typed.property_keys` change |
| `90978ac6` raw | 8 / 126 / 10, unchanged | 33,493 | 2,902 | only `typed.property_keys` is added (keys not read) |
| `~1` configured | 8 / 136 / 7, unchanged | 33,546 | 2,854 | byte-identical (`a5edf1ed…`) |
| `~1` raw | 8 / 132 / 7, unchanged | 33,466 | 2,902 | byte-identical (`d05ebc0e…`) |

No site changes verdict or first reason, so no site needs attribution and no
correction applies. The configured reason changes as follows:

- **before:** two blockers: an unmodelled call through `flb_service_set >
  … > dirname`, and *more than one call reaches a hazard: `flb_service_set`
  at :218 (… `flb_plugin_load`: mutates a `in_plugins` list), `flb_input` at
  :226*;
- **after:** the second blocker is gone. The first is unchanged in kind and
  now lists its two functions (`localtime_r`, `flb_cf_yaml.c::dirname`). The
  note *property keys: `flb_service_set` at :218 reaches only its keys'
  effects* joins the derived-nonetheless list.

Stated as *discharged / not discharged / unresolved at this path under these
derived premises*:

- **`90978ac6`, the fixed test's 3 sites: unresolved.** These premises are
  derived: the key reading of `:218`, the holder escape and
  P-CALLBACK-INSTANCE. The list-head instance is not derived: the
  unmodelled-call blocker stands over 2 predefined-macro arms.
- **`90978ac6~1`, the old test: not discharged.** Its NC1 path
  (`flb_input_collector_running` from T) is refused. It has no plugin copy,
  so Step B never applies.
- **Other sites.** in_tail's 16, NC3's 2 and the other-plugin refusals are
  unchanged at both revisions. No other derivation passes a property setter.

**Diagnostic reading** ([`diagnostic.py`](scripts/diagnostic.py), archived
in [artifacts/diagnostic/](artifacts/diagnostic/), never a result). It is
the configured fixed reading with `--owned-prefix=_WIN32` added, which drops
both Windows-only arms. The instance flag derives no coroutine, with the list
head, the holder escape, P-CALLBACK-INSTANCE and the property keys all named.
The three sites are **discharged** on `(flb_lib_worker, NonThreaded)` and
`(input_thread, Threaded)` (`b32ddd59…`). This only locates the remaining
blocker.

## 6. The observation as contracts

The census derives the key semantics from the bodies. The observation it
rests on is recorded in
[`fluent_bit/input_instance.qv`](../../../providers/quirin-provider-fluent-bit/contracts/fluent_bit/input_instance.qv),
so it is pinned with the bodies and reviewed, never trusted:

- **`flb_config_set_property(config, k, v)`** is new. `config` is held for
  review; its reason is the derived per-key effect (the matched entries' arms,
  the scalar store into the entry's own field, the unknown-key warning) and
  that `config`'s retention through a loaded plugin or parser was not read.
  `k` and `v` are borrowed: read by the comparisons and the warning, and
  through `flb_env_var_translate`'s copy. The scalar result is bare. The
  declaration is withheld (C06.1), so it discharges and accuses nothing.
- **`flb_service_set`'s** held variadic tail is reworded to the derived
  key-first pairing.

The bodies of `flb_service_set`, `flb_config_set_property`, `prop_key_check`,
`set_log_level`, `set_log_level_from_env` and `service_configs[]` are
byte-identical at `66910c10`, `046accf6` and the export. Both declarations are
pinned there with [`pin.py`](scripts/pin.py) (`65 pinned` at each tree).
`pin --check` reports 2 recorded, 0 not in force, 0 stale at each tree. The
contract-corpus pins move 251 → 252 v1 declarations and 9 → 10 held, and the
`qcore_contract_corpus.rs` table gains one row. The module and view counts are
unchanged.

## 7. Corpus, system and tail-pause slice

@ARBITERS@

## 8. Prediction against outcome

| Measure | Predicted | Measured |
|---|---|---|
| key table | 53 keys + NULL; case-insensitive whole key; 3 handler arms; only `Plugins_File` hazardous (90%) | **as predicted** (every entry classified) |
| fixed test ×3, configured | creation blocker gone (85%); unresolved with a narrower blocker (70%) | **as predicted**: unresolved; the blocker is the unmodelled-call one over `dirname` **and** `localtime_r` (the second is reached through the property function's `flb_warn` macro, which this reading expands) |
| fixed test ×3, raw | unresolved, first reason unchanged (90%) | **as predicted** |
| old test, both readings | not discharged, byte-identical (90%) | **as predicted** |
| other sites, edges, seeds | unchanged (95%) | **as predicted** |
| `flb_input_set` / others | reach no hazard (80%) | **as predicted** (`flb_input_set` read and refused as not table-driven; its closure reaches no hazard) |
| corpus / system / slice | @PREDICT_ARB@ | @MEASURED_ARB@ |
| follow-up | amended, not retired (70%) | **retired**: its DISCHARGE clause (a literal-key reading of `flb_config_set_property`'s comparisons, fail-closed, measured on the census) is met; the prediction conflated it with site discharge. The 3 sites move to the successor `W47-LOOP-OWNER-PREDEFINED-MACRO-ARMS` |

## 9. Lean

`proof/QCore/LoopOwner.lean` is audited in `proof/AuditLoopOwner.lean`. The
theorem count goes 53 → 66, with no new audit module, no placeholders and no
project axioms (`propext`, `Quot.sound`, `Classical.choice` only). The new
theorems:

- **`a_read_key_list_is_null_terminated_and_paired`**: `2n + 1` arguments,
  the last null; with `literalKeys_cons_str`.
- **`an_unreadable_argument_keeps_every_branch`**, with
  `a_non_literal_key_is_unreadable`, `an_odd_list_is_unreadable` and
  `an_unterminated_list_is_unreadable`.
- **`a_key_absent_from_the_table_keeps_every_branch`**.
- **`a_literal_key_sequence_reaches_only_its_keys_effects`**.
- **`the_key_reading_covers_the_run`**: whatever prefix of the keys the
  setter processes, its reach is read.
- **`a_matched_hazard_stays_read`**: the `Plugins_File` control.
- **`a_case_variant_matches_the_same_entries`**.
- **`a_key_reach_holding_a_hazard_keeps_the_route`** and
  `a_key_reach_without_a_hazard_leaves_the_one_creation`: the link to the
  list-head derivation.

The old test's T path is refused by the existing
`the_old_test_path_is_never_discharged`. It does not depend on the instance
flag this reading feeds.

## 10. Premises

**Derived** (at this path, the export): the setter's key-first pairing, the
key table, the comparison, the branch chain and every scalar store's field
and width; plus rule 10p's earlier derivations (the holder escape,
P-CALLBACK-INSTANCE).

**Stated, not proved** (trust boundary 28), added by this card:

- the typed reading's argument and initializer extraction;
- **P-KEY-FOLD**: the C library's `strncasecmp` folds printable-ASCII keys no
  more permissively than ASCII case folding;
- `strnlen` and `va_arg` as C specifies them.

Rule 10p's standing premises are unchanged (P-HEAD-INSTANCE, P-FRESH-CONTEXT,
P-REGISTERED-DESCRIPTORS, P-HOLDER-OPAQUE, P-INSTANCE-FIELDS-BY-MEMBER,
acutest's `TEST_CHECK`, the typed reading's premises). So is the unmodelled
premise that macro invocations on the region call no hazard
(`W47-LOOP-OWNER-REGION-MACRO-CALLS`).

## 11. Follow-ups

[`followups.json`](followups.json) is applied by the idempotent
[`scripts/update_ledger.py`](scripts/update_ledger.py): @LEDGER@.

- **Retired:** `W47-LOOP-OWNER-SERVICE-PROPERTY-KEYS` (§8).
- **Registered:**
  - `W47-LOOP-OWNER-PREDEFINED-MACRO-ARMS`: 2 definitions holding 3 sites;
  - `W47-LOOP-OWNER-REGION-MACRO-CALLS`: the region does not count calls a
    macro invocation makes. Expanding macros by name across the region was
    tried and refused the fixed test on a spurious match (the test's
    `TEST_CHECK` read with `lib/monkey`'s acutest definition), so the
    expansion is kept only in the two bodies this card reads. No site moves.
- **Amended:** `W47-INSTANCE-FLOW-CONTRACT-REVIEW` (5 declarations, 8 held
  clauses), `W47-LOOP-OWNER-COROUTINE-POST-YIELD` and
  `W47-LOOP-OWNER-INSTANCE-FLOW`.

## 12. Gate

@GATE@

**Files likely to conflict** with the parallel cards:

- `docs/agents/gateway.md` (the 10p amendment) and `docs/agents/README.md`
  (the 10p row);
- `docs/STATUS.md`, `docs/development/increments/wave-47.md`,
  `docs/reports/INDEX.md` and `docs/reviews/r0_3/acceptance_matrix.json`;
- `crates/qcore-c-gateway/src/thread_entry_census.rs` (the module
  declaration, `StepB` fields, `CensusReading::property_keys`, `Scan::directives`),
  `loop_owner_instance.rs` (helpers made `pub(super)`, the region hook, the
  unmodelled list) and `typed_dispatch_census.rs` (two fact kinds, the
  caller passed to `call`);
- `crates/quirin-source-analysis/examples/{loop_owner_census,typed_dispatch_facts}.rs`;
- `providers/quirin-provider-fluent-bit/contracts/fluent_bit/input_instance.qv`
  and `contracts.lock`;
- the pins in `compiler/tests/contract_loader.rs` (251 → 252, 9 → 10 held) and
  `compiler/tests/qcore_contract_corpus.rs` (one row). The module count and
  the resource-view count are unchanged.

## Reproduce

```sh
python3 docs/reports/w47-loop-owner-typed-dispatch/scripts/facts.py OUT FACTS_BINARY GENERATED_DIR
python3 docs/reports/w47-loop-owner-typed-dispatch/scripts/census.py OUT CENSUS_BINARY \
  --typed-library=FACTS/library.json --typed-fixed=FACTS/fixed-test.json --typed-previous=FACTS/previous-test.json
python3 docs/reports/w47-service-property-keys/scripts/compare_census.py BEFORE AFTER
python3 docs/reports/w47-service-property-keys/scripts/key_table.py OUT/fixed-configured.json
python3 docs/reports/w47-service-property-keys/scripts/diagnostic.py OUT CENSUS_BINARY FACTS
python3 docs/reports/w47-service-property-keys/scripts/pin.py OUT [--check]
python3 docs/reports/w47-loop-owner-typed-dispatch/scripts/arbiters.py OUT COMMIT SLICE_RUN_NAME
python3 docs/reports/w47-service-property-keys/scripts/identity.py RUN OUT
python3 docs/reports/w47-service-property-keys/scripts/before_after.py BEFORE.json AFTER.json
python3 docs/reports/w47-service-property-keys/scripts/gate.py LOGS rust|records
```
