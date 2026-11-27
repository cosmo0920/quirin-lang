# W47-SUMMARY-VALIDATION-PROTECTED-FORMALS — a W4 summary is validated on the premises its reported row reads (wave 47)

Soundness card. It amends gateway rule 4 as rule 4b; the reserved number 10p
is unused. Base `2d7970a4` (local `main`, round 2b). Lean:
`QCore.SummaryValidationPremises`, 20 theorems audited in
`proof/AuditSummaryValidationPremises.lean`.
Frozen forecast: [PREDICTION.md](PREDICTION.md).

## The gap

Every W4 route builds the CFG it infers on in the environment of the moment.
That environment does not yet hold the candidate, because the candidate is
what the route is deriving. The reported run (`analyze_defined_function`)
builds the same body with the candidate in force. Construction reads the
function's own contract in exactly two places:

- `protected_resource_formals`, which protects every formal the contract does
  not declare `Borrowed`;
- the descriptor reader, which names integer-kinded non-`Borrowed` formals.

The protected set seeds every typed-root premise:

- the lexical never-bound closure (`untracked_bases`, rule 10o);
- `may_bind` and the declared locals (`FunctionTypes::read`);
- the AST closure that rule 10l joins (`RootDeclarations::read`).

So a copy `const T *a = _a;` of a formal the candidate seeds as an entry
resource was a never-bound root in validation and a bound root in the
reported row. The typed reads rooted at the copy were admitted in one run and
refused in the other. A caller certified on a summary whose own row refused.
Rule 5b's premise, that a never-bound root reaches no tracked place, is false
for such a copy.

## Mechanism (rule 4b)

`build_function_cfg_with` takes an optional contract to read as the function's
own. A declaration in the environment still wins. That contract feeds the
protected set and the descriptor names, and nothing else. In particular, a call
to the function inside its own body still resolves through the environment,
so no recursive assumption enters (rule 4).
`legacy_summary_premises::validates_under_reported_premises` re-checks a
validated candidate with the unchanged `observations_match_contract` gate. The
check runs on the CFG built with the candidate as the function's own contract,
and a candidate that refuses there publishes nothing. It is hooked into the
live-entry route, the removed-wrapper route and the derived-result route
(which uses the derived-return CFG). The re-check is skipped in three cases:

- a declared function, which was already validated under its declaration's
  protection;
- an all-`Borrowed` candidate, which reads nothing new;
- a rebuilt CFG equal to the one the route validated on, since the analysis
  is a function of the CFG.

The contextual (ranked recursive) route rebuilds both context CFGs with the
candidate and stores the rebuilt combination. That combination is what its
reported run reads, so both runs read the protected premises. The rebuild
writes no parity rows (`typed_access_path::without_parity_rows`). Decisions
are recorded in the parity stream as `position: "summary-premises"`. Each
derivation's call graph and validated summaries are recorded as
`"summary-premises-graph"`.

### Audit of the other own-contract premises (brief item 4)

| Premise W4 validation reads | Depends on | Disposition |
|---|---|---|
| never-bound closure (rules 5b, 10k, 10o) | protected set | aligned: seeded from the same set |
| `may_bind`, declared locals (rule 10l lexical) | protected set (`bound_names`) | aligned |
| AST closure of rule 10l / 10o's join | protected set (`protected-formal` seeds) | aligned |
| descriptor names (`DescriptorNames::read`) | own contract's integer-kinded formals | aligned (read from the same own contract); inferred contracts are heap-kinded, so it adds no name |
| counted-user clauses | declared contract, checked after the reported analysis | not a validation premise; inferred summaries carry none |
| a call to the function itself | environment | deliberately different (rule 4: no assumed recursive summary) |

Nothing is left unaligned, so no residual is registered.

## Measurements

The arbiter is `QCORE_TYPED_PATH_PARITY=OUT/parity.jsonl
scripts/run_pinned_audit.py OUT <commit> --inputs-root
/home/cosmo/Documents/GitHub/qcore-c/build/cycle19 --jobs 8`. The system slice
is `docs/reports/w46-contract-gap-expansion/scripts/run_system.py OUT --jobs 8`
at `046accf6`. Both are driven by the worktree-private `measure.sh`. Every run
exits 0.

**BEFORE (`2d7970a4`)** reproduces the round-2b baseline
([artifacts/before/](artifacts/before/)):

- Corpus: `resource-diagnostics.json`, the chunk contracts and the Monkey
  source are byte-identical, and the parity stream and contexts are identical
  when sorted.
- System: verdicts and leaves are identical (`compose3.py` COMPOSES,
  0 mismatches). The 45 differing leaves of `resource-diagnostics.json` are 30
  contract-source digests and 15 `inventory_sha256`, which follow the
  worktree path embedded in the contract module paths.

**Record-only measurement (`adda51b8`)**, [artifacts/measure/](artifacts/measure/):

- The build re-checks and records every decision but refuses nothing.
- Its verdict-bearing artifacts are identical to the baseline.
- It also writes 125 parity rows from its rebuilds. The fix build mutes them.
- `scripts/census.py` produces [census-corpus.json](artifacts/measure/census-corpus.json)
  and [census-system.json](artifacts/measure/census-system.json).

**Step 1, the extent.** The corpus derives 1,166 summaries in 180 units, with
80 undeclared, non-all-`Borrowed` candidates, all on the live-entry route. Of
these, 9 CFGs change and 8 candidates stop validating. The summaries that
validate while their function's reported row is non-Safe are exactly those 8,
and each is Unknown. The two runs disagree on the protected set:

| Unit | Function | Candidate | Formals the reported run protects |
|---|---|---|---|
| `rdkafka_partition.c` | `rd_kafka_topic_partition_cmp` | Owned, Owned | `_a`, `_b` |
| `rdkafka_partition.c` | `rd_kafka_topic_partition_not_regex` | Owned, Borrowed | `elem` |
| `rdkafka_metadata.c` | `rd_kafka_metadata_broker_cmp` | Owned, Owned | `_a`, `_b` |
| `rdkafka_metadata.c` | `rd_kafka_metadata_broker_internal_cmp` | Owned, Owned | `_a`, `_b` |
| `rdkafka_metadata.c` | `rd_kafka_metadata_partition_internal_cmp` | Owned, Owned | `_a`, `_b` |
| `rdkafka_sasl_oauthbearer.c` | `check_oauthbearer_extension_value` | Owned, Borrowed, Borrowed | `value` |
| `rdkafka_cert.c` | `rd_kafka_conf_ssl_passwd_cb` | Borrowed ×3, Owned | `userdata` |
| `rdkafka_conf.c` | `rd_kafka_anyconf_is_modified` | Owned, Borrowed | `conf` |

The ninth changed CFG, `mk_config_sanity_check` (protecting `server`), still
validates, and its row is Safe. The other 497 summaries without a Safe row are
rule-4a helpers with no reported row (479 all-`Borrowed`; the other 18
validate on the reported premises). Safe functions that rest transitively on a
withdrawn summary: **one**, `rd_kafka_topic_partition_cmp_opaque`. The system
slice has 4 decisions (2 functions × 2 configurations), and none withdraws.

**AFTER (`0664bede`)** is archived as the new baseline under
[artifacts/after/corpus](artifacts/after/corpus/) and
[artifacts/after/system](artifacts/after/system/).
[`scripts/attribute.py`](scripts/attribute.py) exits 0
([attribution-corpus.json](artifacts/after/attribution-corpus.json)).

| Measure | Baseline | AFTER |
|---|---:|---:|
| Functions Safe / provisional Unsafe / Unknown | 527 / 10 / 4,297 | **526 / 10 / 4,298** |
| Units | 6 / 8 / 167 / 4 | unchanged |
| leak / summary_conflict | 36 / 1 | unchanged |
| unknown_alias | 7,927 | 7,936 (+9) |
| unsupported | 35,741 | 35,737 (−4) |
| System unique functions | 31 / 2 / 201 | unchanged |
| System unsupported / unknown_alias | 2,888 / 1,020 | unchanged |

The corpus changes 15 rows, and every one is attributed:

- **Correction:** `rd_kafka_topic_partition_cmp_opaque` moves Safe → Unknown,
  with one leaf, "call to uncontracted function
  `rd_kafka_topic_partition_cmp`". Its Safe rested on a summary validated on
  premises its callee's own row refused.
- **The 8 withdrawn functions** stay Unknown and exchange leaves. Refusals
  that came from protection are removed: 11 in all, of which 6 are
  return-family, 4 indirect access and one a branch condition. In their place
  16 `unknown_alias` leaves appear on the copies or on the locals read through
  them: `a` and `b` once each in the three metadata comparators and twice each
  in `…_partition_cmp`, plus `rktpar`, `c`, `confhdr`, and `conf` ×3 in
  `…_ssl_passwd_cb`.
- **Six already-Unknown callers** exchange an `unknown_alias` leaf for a
  refusal of the now-uncontracted call:
  - `rd_kafka_conf_is_modified` and `rd_kafka_topic_conf_is_modified`: "call
    to uncontracted function `rd_kafka_anyconf_is_modified`";
  - `rd_kafka_anyconf_copy` (−2 alias), `rd_kafka_anyconf_dump`,
    `rd_kafka_anyconf_warn_deprecated` and `rd_kafka_oauthbearer_set_token0`:
    one branch-condition refusal each, where the call sits in a condition.

The system slice is verdict and leaf identical to the baseline (COMPOSES,
0 mismatches). Its `resource-diagnostics.json` differs only by the same
45 digest leaves, and its sorted parity stream is identical.

**Corpus parity stream:** it adds 69 rows and removes 55, all at the 15
changed functions ([parity-attribution.json](artifacts/after/parity-attribution.json),
exit 0).

**Leaks and violations:** none added. There is no new leak, double release,
use after release, provisional Unsafe or unit move. Withdrawing a summary only
leaves a call uncontracted.

**After the fix**, no summary validates while its function's reported row is
non-Safe ([census-corpus.json](artifacts/after/census-corpus.json)). The 497
summaries without a Safe row are helpers with no reported row. The count of
validated summaries falls from 1,166 to 1,157: the 8 withdrawn summaries and
`cmp_opaque`'s own.

## Prediction against outcome

1. `cmp_opaque` Safe → Unknown with the one uncontracted-call leaf, a
   correction: **holds exactly**.
2. The 8 withdrawn functions stay Unknown and exchange leaves (the
   return-family leaf goes, alias leaves on copies stay or appear): **holds**.
3. Direct callers stay Unknown and may gain uncontracted-call leaves:
   **holds**. Two gain that leaf; four gain a branch-condition refusal
   instead, where the call is in a condition, and lose an alias leaf.
4. 526 / 10 / 4,298, units unchanged, nothing else moves: **holds exactly**.
5. No leak or violation added: **holds**.
6. System does not move: **holds**.
7. Parity differs only at the 8 withdrawn functions: **missed in part**. The
   7 callers' rows also change, because a caller's typed readers see the
   callee uncontracted. Every differing row belongs to the 15 changed
   functions, and the rebuild itself writes none.

## Tests and controls

`crates/qcore-c-gateway/tests/summary_validation_premises.rs` has 5 tests.
Their fixtures:

1. The cmp/cmp_opaque shape.
2. A one-formal copy read in a `Borrowed` actual.
3. A two-level cascade.
4. Copies whose scalar reads the reported run also admits: a `void *` copy
   and a typed copy, each keeping `Owned`.
5. A read-only formal (`Borrowed`), a released formal, a copy read then
   released, and a copy forwarded to a `Borrowed` callee.

They were written first and run on the record-only build, whose verdicts are
the base's ([controls-at-base.txt](artifacts/cause/controls-at-base.txt)).
There, fixtures 1–3 fail because their callers are `Safe`, and fixtures 4–5
pass. With the fix all five pass. The typed-const-pointer comparator pin moves
to the corrected answer: it is renamed
`a_comparator_summary_is_withdrawn_where_its_own_row_refuses` and now pins
the wrapper's uncontracted-call leaf and the comparator's four alias leaves.

## Lean

`QCore.SummaryValidationPremises` models a run as the body's binding events
(rule 10l's `BindingEvent`) plus one `protectedFormal` per formal that the own
contract in force does not declare `Borrowed`. It proves:

- **The gap, witnessed on the cmp shape:**
  `the_old_validation_disagreed_with_the_reported_run`.
- **The runs coincide, and so do their premises:** the installed contract is
  the own contract read at validation
  (`the_installed_contract_is_the_own_contract`,
  `the_validation_run_reads_the_reported_premises`), so never-bound and
  `may_bind` agree (`a_root_is_never_bound_in_both_runs_or_in_neither`,
  `may_bind_agrees`).
- **Fail-closed:** `every_old_binding_is_kept`;
  `a_published_summary_passed_on_the_reported_premises`;
  `a_published_summary_passed_the_old_gate`.
- **Exactly what protection adds:** `what_protection_adds` and
  `a_copy_of_a_protected_formal_is_bound`. A name the protected run binds is
  bound by the body alone or copy-reachable from a protected formal.
- **No change elsewhere:** `a_declaration_keeps_its_premises` and
  `an_all_borrowed_candidate_changes_no_premise`.
- **A read-only body binds only the formal:**
  `a_read_only_body_binds_only_the_formal`.

There are no placeholders and no project axioms. Stated, not proved: the Rust
construction reads the own contract nowhere else, and the Rust-to-Lean
correspondence (trust boundary 28).

## Gate

The standing gate ([gate.json](gate.json); exits in [gate/](gate/); logs in
the worktree's `private/w47-sv/gate-1/` and `gate-2/`) ran with every exit
captured to a file.

| Step | Exit | Count |
|---|---:|---|
| `git submodule update --init --recursive` | 0 | |
| `cargo fmt --all -- --check` | 0 | |
| `python3 scripts/run_test_binaries.py` | 0 | 345 units, 358 summaries; 6,177 passed / 0 failed / 11 ignored (`private/test-binaries/20260928T052333Z-2481390/`) |
| `cargo clippy --all-targets -- -D warnings` | 0 | |
| `reevaluation_probe` / `reevaluation_followup` | 0 / 0 | |
| `check_lean_audits.py` | 0 | 221 audits (the new one included; 20 theorems, standard axioms only) |
| `python3 -m unittest discover -s scripts/tests` | 0 | 365, after the matrix and index regeneration |
| `build_docs_index.py --check` | 0 | |
| `cargo clean --profile dev` | 0 | |
| `clang --version` | 0 | Ubuntu clang 21.1.8 |

The Rust and Lean steps ran on the code at `0664bede`; later commits change
only documents, artifacts, the ledger, the matrix and the index. No
`providers/` path is touched, so the provider gate does not apply.

## Ledger

[`scripts/update_ledger.py`](scripts/update_ledger.py) retires
`W47-SUMMARY-VALIDATION-PROTECTED-FORMALS` and registers nothing
([followups.json](followups.json) is empty; see the audit table), taking the
count from 538 to 537.

## Scripts

- `identity.py`: manifest-keyed identity against a gzipped archive.
- `json_diff.py`: leaf diff of two JSON files.
- `census.py`: the step-1 extent and the forecast.
- `attribute.py`: AFTER row attribution.
- `parity_attribution.py`: parity-row attribution.
- `archive.py`: the deterministic gzip archive and manifest.
- `update_ledger.py`: the ledger edits.
