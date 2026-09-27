# P06 — Define the first CTraces provider capability

**Outcome: designed, not implemented.** No CTraces function is accepted by any
implemented core checker, so the card's precondition for implementing
("only if an adequate core checker already exists") is not met. The deliverable
is the measured adapter inventory below, one capability contract
(`ctraces.span.dropped_counts.v1`), a machine-checked rejection boundary, and
four follow-on implementation units.

Nothing is promoted. Raw Cycle 19 units, CT coverage (11/50), the contract
corpus (59 modules) and `qcore-schemas` (4.0.0) are unchanged. The four pinned
185-unit artifacts were not touched and not re-measured.

- Design: [`docs/design/CTRACES_PROVIDER_CAPABILITY_V1.md`](../../design/CTRACES_PROVIDER_CAPABILITY_V1.md)
- Contract text: [`ctraces_span_dropped_counts.qv`](ctraces_span_dropped_counts.qv)
- Census tool: [`ctraces_census.py`](ctraces_census.py) → [`census.json`](census.json)
- Boundary test: `crates/qcore-verification/tests/ctraces_capability_boundary.rs`
- Excerpt fixture: [`ctraces_span_excerpt.c`](ctraces_span_excerpt.c)

## 1. What the CTraces route actually is today

### 1.1 Profile declaration

`contract/profiles/fluent-bit/profile.json`, library `ctraces`. Every inventory
is empty; the entry declares a source root and a provider id and nothing else.

| Field | `ctraces` | `chunkio` | `librdkafka` |
|---|---:|---:|---:|
| `source_roots` | 1 | 1 | 1 |
| `audited_files` | **0** | 3 | 3 |
| `source_witnesses` | **0** | 3 | 3 |
| `permitted_mechanisms` | **0** | 2 | 7 |
| `operation_summaries` | **0** | 14 | 0 |
| `api_dispositions` | **0** | 0 | 0 |
| `accepted_builds` | **0** | 0 | 0 |
| `required_evidence` | **0** | 0 | 0 |
| `typed_import` | **null** | null | null |
| `callback_summaries` | **0** | 0 | 0 |
| `boolean_field_models` | **0** | 1 | 0 |

So the CTraces adapter **imports nothing and asserts nothing**. It declares a
source root so the audit enumerates the 17 units, and a provider id so the
registry has an entry. There is no audited file, no digest, no mechanism, no
operation summary and no field model to bind source to a model.

(`cmetrics` is also all-zero in *this* profile; its one checked function contract
comes from the separate `contract/profiles/cmetrics-counter/` configuration,
which declares an audited file and one `boolean_field_models` entry. There is no
equivalent second configuration for CTraces.)

### 1.2 Provider package

`contract/profiles/fluent-bit/providers/qcore.ctraces.json` (built-in) and
`providers/qcore-provider-ctraces/provider-template.json` (executable). Both are
generic: `capabilities: ["resource_contracts"]`, all targets `"*"`, no required
foreign summary, no observation hook, no filesystem input.
`providers/qcore-provider-ctraces/src/main.rs` is four lines calling
`runtime::run(...)` with **no** v2 proposal function, so
`providers/common/runtime.rs` answers every request `Unknown` with
`qcore.ctraces.semantic_evidence_unavailable`. Compare
`qcore-provider-chunkio` / `qcore-provider-cmetrics`, which pass a proposal
function through `run_with_v2`.

Two dangling identities, recorded and not repaired here:

- Both manifests declare `certificate_checker: "qcore.unsupported_assertions.v1"`.
  Nothing in this repository implements that name. `crates/qcore-audit/src/source_contract.rs:434`
  admits only `qcore.boolean_field_contract.v1|v2`, so a configured
  source-contract route would refuse the CTraces package outright with
  "provider `qcore.ctraces` is incompatible with the Boolean source checker".
  This is currently harmless because the provider only ever answers `Unknown`.
- The built-in manifest declares `evidence_versions: [1]` while the executable
  package declares `[1, 2]`.

Follow-on unit P06-I4 owns both.

### 1.3 Contract corpus and Rust code

`contract/qcore/` has no `ctraces` directory and **no contract module mentions
any CTraces function**. No production Rust file special-cases CTraces: the only
occurrences under `crates/`, `compiler/` and `providers/` are registration and
discovery test expectations (`compiler/tests/first_party_discovery.rs:60`,
`compiler/tests/cycle20_verifier_plugins.rs:61,245`,
`providers/conformance/tests/packages.rs:439`) and a path literal in
`compiler/tests/cycle19_current_command.rs:26`. `compiler/src/source_profiles.rs`
mentions CTraces once, in a doc comment.

### 1.4 What the route produces

From the verified explicit-Clang run at fluent-bit `9f0fa5730fba1761ebaf14bcd31220a3a9be9d6e`
(`build/cycle19/explicit-clang-verified/reports/resource-diagnostics.json`, the
run recorded in `docs/reports/cycle19-explicit-clang/README.md`):

**17 units — 1 Safe / 2 provisional Unsafe / 13 Unknown / 1 data-only.**
**273 functions — 4 Safe / 3 provisional Unsafe / 266 Unknown.**

| Unit | Status | Fns | Unit | Status | Fns |
|---|---|---:|---|---|---:|
| `ctr_version.c` | Safe | 1 | `ctr_id.c` | Unknown | 10 |
| `ctr_encode_opentelemetry.c` | Unsafe | 70 | `ctr_link.c` | Unknown | 7 |
| `ctr_resource.c` | Unsafe | 9 | `ctr_log.c` | Unknown | 2 |
| `ctr_utils.c` | data-only | 0 | `ctr_mpack_utils.c` | Unknown | 16 |
| `ctr_attributes.c` | Unknown | 9 | `ctr_random.c` | Unknown | 1 |
| `ctr_decode_msgpack.c` | Unknown | 48 | `ctr_scope.c` | Unknown | 6 |
| `ctr_decode_opentelemetry.c` | Unknown | 16 | `ctr_span.c` | Unknown | 40 |
| `ctr_encode_msgpack.c` | Unknown | 18 | `ctraces.c` | Unknown | 5 |
| `ctr_encode_text.c` | Unknown | 15 | | | |

The seven non-`Unknown` functions are `ctr_version`, `ctr_opts_set`,
`ctr_opts_exit`, `ctr_encode_msgpack_destroy` (Safe) and `otel_span_set_links`,
`otel_span_set_status`, `ctr_resource_create` (provisional Unsafe — withdrawn
unsupported results, **not** proven upstream defects).

**1,578 diagnostics**, ranked:

| Count | Category | Description |
|---:|---|---|
| 612 | unsupported | indirect memory access is outside the resource projection |
| 420 | unsupported | call to uncontracted function |
| 205 | unsupported | unmodeled call, aggregate or indirect access in branch condition |
| 136 | unknown_alias | unresolved resource identity |
| 67 | unsupported | unmodeled call, aggregate or indirect access in return expression |
| 33 | unsupported | braced initializer, expression, or declaration is not modeled |
| 27 | unsupported | unsupported assignment expression |
| 24 | unsupported | unmodeled call, aggregate or indirect access in for-loop condition |
| 21 | unsupported | unrecognized assignment destination |
| 10 | unsupported | nullable resource used by a non-null contract |
| 9 | unsupported | store through pointer without an out-parameter contract |
| 6 | unsupported | local declaration may shadow a formal resource identity |
| 4 | leak | live obligation acquired at preprocessed line … |
| 3 | unsupported | (three singleton kinds) |

The 420 uncontracted calls reach **152 distinct callees**, dominated by
libraries CTraces does not own: 123 `mpack_*`, 52 `cfl_*`, 46 `sds_cat_safe`,
22 `opentelemetry__*`, 21 `snprintf`, 15 `otlp_*`; only 84 are `ctr_*`.

### 1.5 Recorded build-ingestion boundary

`docs/reports/fluent-bit-moving-revisions/README.md:57` records that at
`d1ab8abff` the full-corpus configuration rejects because "the fresh full Clang
database contains an unsupported CTraces option at `ctr_attributes.c`", leaving
CTraces the one library scope with no measurement in that table.
`docs/reports/cycle19-current-command/README.md:34` records "17 contain an
unexpanded CTraces option" — 17 being exactly the CTraces units. The option in
those databases is the CMake-recorded
`-D__FILENAME__="$$(subst <source-root>/,,$$(abspath $$<))"`, an unevaluated Make
expression. The pinned explicit-Clang preparation
(`scripts/prepare_cycle19_clang_audit.py`) handles it — the `9f0fa573` database
carries the same literal `-D__FILENAME__` on 70 of 1,415 entries and all 17
CTraces units analyze — so this is a build-ingestion boundary of specific
databases, not a permanent CTraces exclusion.

## 2. Census: can any CTraces function reach an implemented checker?

`ctraces_census.py` runs the recorded compile command for each of the 17 units
through the same Clang, walks the JSON AST, and records every function
**defined** in the unit with its signature, its direct and indirect callees, and
which implemented checker's *entry shape* it could match. Result: **273
definitions**, matching the audit's per-unit function counts exactly in all 17
units (see §1.4 table).

Entry-shape matches — a candidate is not an acceptance:

| Entry shape | Candidates | Why every candidate still fails |
|---|---:|---|
| Boolean field, unit result `void (struct R *)` | 26 | 25 are destructors with 1–7 calls; the Boolean statement grammar admits no call except a declared frame helper. The 26th, `ctr_opts_exit`, never touches a field. |
| Boolean field, i32 result `int (struct R *)` | 4 | `ctr_attributes_count`, `ctr_mpack_consume_nil_tag`, `ctr_mpack_peek_array_length` all call out; `decode_hex_digit` takes `char *`, not a declared record. |
| Unsigned unit counter | 2 | `ctr_id_get_len` and `get_attributes_count` perform **no** write; the counter route requires the `Read, Write, Read, ReturnValue` path of `p->f++; return p->f;`. The census finds zero `++`/`--`/`+= 1` on any member in all 17 units. |
| Field destructor | 26 | Every CTraces destructor releases through opaque callees (`cfl_sds_destroy`, `ctr_*_destroy`, `cfl_kvlist_destroy`) rather than libc `free`, and ends by freeing the **root**, which the checker explicitly excludes ("proves field release, not enclosing-object teardown"). |
| Cleanup outcome | 4 | Same four as the i32 row; none has the descriptor-sentinel/errno-report shape. |
| Chunk lifecycle, list unlink, reactor callback | 0 | CTraces has no chunk. Its list operations are CFL's `cfl_list_*`, a different library; the implemented unlink route targets Monkey's `mk_list_del` plus its helper. |

**Conclusion: zero of 273.** The census is a reachability measurement, not a
safety statement.

## 3. Machine-checked rejection boundary

`crates/qcore-verification/tests/ctraces_capability_boundary.rs` drives the real
`import_chunk_source_with_profile` over `ctraces_span_excerpt.c` — a reduced
excerpt whose six function bodies are the upstream bodies — with a hermetic
profile that declares the Boolean field models a CTraces adapter would have to
write today. Four tests, all refusals, no positive path:

| Test | Pins |
|---|---|
| `ctraces_span_setters_are_refused_by_the_boolean_field_abi` | all four dropped-count setters and `ctr_span_set_flags` refuse with exactly `outside single borrowed chunk / int-or-void-result source profile` |
| `declaring_the_written_count_field_changes_no_ctraces_outcome` | naming `dropped_links_count` in the profile changes nothing; all six definitions refuse at the same ABI selector |
| `ctraces_opts_exit_matches_the_abi_and_is_still_refused` | `ctr_opts_exit` is the one definition whose signature the importer accepts over an `int` member, and it still refuses with exactly `unmodeled expression DeclRefExpr` |
| `designed_ctraces_contract_parses_with_its_specified_vocabulary` | the designed contract parses and carries exactly its specified clause vocabulary, including the two setters that share one field |

`reason()` panics if any CTraces function ever yields a `Summary`, so the test
cannot silently become a Safe-asserting stub. When P06-I2 lands, these refusals
change deliberately and the test must be updated in the same change, not deleted.

The excerpt is not layout-compatible with the real `struct ctrace_span` (members
that would drag in the CFL and protobuf include trees are omitted) and its header
says so; it exists to exercise statement shapes, never to support an ABI claim.

## 4. The capability

`ctraces.span.dropped_counts.v1` — the four `ctr_span_set_dropped_*_count`
setters, contracted for a single unsigned scalar member write. Subject,
premises, obligation kinds, witness checker `qcore.scalar_field_contract.v1`,
the schema-only process fixture, a 19-row rejection matrix, six negative source
controls and the explicitly unsupported CTraces semantics are all in
[the design](../../design/CTRACES_PROVIDER_CAPABILITY_V1.md). The one new core
semantic it needs is a scalar-assignment path relation; everything else reuses
the existing sealed-import replay discipline.

Follow-on units: **P06-I1** (core checker + contract module + corpus re-pin
59 → 60 + Lean parity), **P06-I2** (gateway two-formal unsigned unit ABI),
**P06-I3** (provider package and fixtures), **P06-I4** (repair the dangling
`qcore.unsupported_assertions.v1` checker name and the `evidence_versions`
disagreement in the shipped CTraces manifests).

Even fully implemented, this would give checked contracts for 4 of 273 CTraces
functions, all in `ctr_span.c`, whose raw unit would stay `Unknown` on its other
36. It would not touch the two provisional `Unsafe` CTraces units.

## 5. Files changed

| Path | Kind |
|---|---|
| `docs/design/CTRACES_PROVIDER_CAPABILITY_V1.md` | new, design |
| `docs/reports/remaining-p06/README.md` | new, this record |
| `docs/reports/remaining-p06/ctraces_span_dropped_counts.qcore` | new, contract text (loaded by no production route) |
| `docs/reports/remaining-p06/ctraces_span_excerpt.c` | new, test fixture |
| `docs/reports/remaining-p06/ctraces_census.py` | new, read-only census tool |
| `docs/reports/remaining-p06/census.json` | new, census result |
| `crates/qcore-verification/tests/ctraces_capability_boundary.rs` | new, 4 refusal tests |

No production Rust, no profile, no manifest, no contract-corpus module, no
schema and no pinned expectation changed.

## 6. Gate

Run in this worktree after `git submodule update --init --recursive`
(the worktree started without the pinned Libevent submodule; the pin test
failed until it was initialized, which is the documented per-checkout step).

| Gate | Result |
|---|---|
| `cargo test --workspace --no-fail-fast` | **1084 passed, 0 failed** (1080 baseline + this card's 4) |
| `cargo fmt --all -- --check` | clean, exit 0 |
| `cargo clippy --all-targets -- -D warnings` | clean, exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `python3 -B -m unittest discover -s scripts/tests` | 43 tests, OK |
| `cd proof && lake build` | 64 jobs, build completed successfully |
| `python3 -B scripts/check_lean_audits.py` | Passed 21 Lean audits |
| `cargo test --manifest-path providers/Cargo.toml` | 48 passed, 0 failed |

Two honest notes:

- The first `cargo test --workspace` attempt failed
  `qcore-verifier-host::enforced_process_limits::a_declared_process_limit_stops_the_fixture_from_forking`.
  This is the pre-existing load-flaky `RLIMIT_NPROC` conformance test recorded
  in `docs/reports/remaining-p03/README.md` §9.1 (it counts every process of the
  real UID). It passes on its own and in the `--no-fail-fast` rerun. The
  assertion was **not** weakened.
- The providers workspace reports 48 passing tests here against the 51 recorded
  at the previous merged checkpoint. No file under `providers/` was touched by
  this card, so this is an environment difference in that workspace, not a
  regression from it; it was not investigated further.
- `lake`/`lean` are installed under `~/.elan/bin` and are not on the default
  PATH in this environment; both Lean gates were run with that directory
  prepended.

## 7. Open boundaries

- **P06-B1.** No CTraces function is checkable today, and this card does not make
  one checkable. The 17 units stay 1 Safe / 2 provisional Unsafe / 13 Unknown /
  1 data-only.
- **P06-B2.** The census's entry-shape filter is a *necessary*-condition sieve
  computed from Clang signatures and call sets. It proves no candidate exists;
  it does not prove any candidate would be accepted if one did.
- **P06-B3.** The measurements come from the `9f0fa573` explicit-Clang run, not
  from the four archived pinned 185-unit artifacts at `66910c10`; those were not
  re-measured (CORPUS-REPIN remains open). CTraces unit verdicts happen to be
  identical in both totals' CTraces slice only to the extent the totals agree,
  which was not verified here.
- **P06-B4.** The dangling `qcore.unsupported_assertions.v1` checker name in the
  shipped CTraces manifests is recorded, not fixed (P06-I4).
- **P06-B5.** The build-ingestion boundary of §1.5 is a property of specific
  compile databases. Nothing here establishes that an arbitrary CTraces CMake
  database is ingestible.
- **P06-B6.** The `ctr_span_set_dropped_link_count` / `..._links_count`
  same-field pair and the `ctr_span_set_flags` `uint32_t`-into-`int32_t` store
  are recorded observations about upstream source. Neither is a defect claim and
  neither was reported upstream.
