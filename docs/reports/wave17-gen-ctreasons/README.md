# GEN-CTREASONS — two tally-neutral tooling and ledger repairs

Card **GEN-CTREASONS**, wave 17. Base commit `8ec8402` ("Consolidate wave 16").
Two bounded follow-ons recorded by card HYGIENE-16, each measured before it was
touched:

| Item | Follow-on | Outcome |
|---|---|---|
| 1 | `D02-GEN-CTREASONS` | the dropped map is **derived**, not carried. HYGIENE-16 recorded it as "not derivable"; measured, it is the companion prose of a list the generator already derives. A second staleness class was found in the committed map while doing it. |
| 2 | `DIFF-TOOL-CONTRACTS-BACKFILL` | seven archived consolidation diff documents gain a `contracts` section, regenerated from their own committed artifact pairs, append-only, behind a provenance record. The card premise is corrected by measurement: the wave-16 document needed nothing. |

**Nothing is promoted, and no tally moves.** No verdict, no CT coverage number,
no corpus pin, no contract-corpus count and no audit input changed; §4 records
the verification. The only derived number that moves is the Lean audit count,
72 → 73, because this card lands one audit (HYGIENE-16's precedent, §4).

---

## 1. Item 1 — `D02-GEN-CTREASONS`

### 1.1 Measured before-state

Regenerating the committed matrix through its owned generator at `8ec8402`,
leaf by leaf against the committed file:

```
$ python3 -B docs/reports/remaining-d02/build_matrix.py . --out /scratch/base.json
REMOVED /ct_conformance/newly_decidable_not_mapped_reasons
CHANGED /validation/measurement_provenance
```

Exactly one section key is lost on every regeneration (the provenance label is
a `--gate` artifact, not a defect: it correctly says "carried, not measured for
this run"). The wave-16 consolidation carried the map forward with
`docs/reports/wave16-corpus/readd_ct_reasons.py`, which refuses to carry a row
describing a case the manifest maps — a guard at the carry, not a derivation,
which is why the follow-on stayed open.

The committed map held four keys:

| key | value |
|---|---|
| `_note` | its own commentary, saying it is hand-maintained |
| `CT11` | "Measured: ScopedReadSharesV1 refuses reconstruction while a share is retained (Q600 `LoanOutstanding { resource: 1, outstanding: 512, borrowers: [3] }`). …" |
| `CT24` | "Card T04's own proposal, unchanged and still unmapped; …" |
| `CT47` | "As recorded by T00; unchanged by this wave." |

### 1.2 The finding HYGIENE-16's guard could not see

The map's companion field, `newly_decidable_not_mapped`, **is** derived: card
T00's frozen `newly_decidable` judgment (`["CT24", "CT39", "CT47"]`) filtered by
the manifest's own unmapped set, which yields `["CT24", "CT47"]`. The committed
map therefore carried a **CT11** row explaining why a case its own published
list does not name is not mapped.

That is a second, distinct staleness class beside HYGIENE-16's CT39 row:

* HYGIENE-16's: *a paragraph about a case the manifest maps.* The wave-16 carry
  guard checks this one.
* this card's: *a paragraph belonging to no published claim.* The carry guard
  passes it, because it computes its "unmapped" set as the union of the list
  and every non-mapped `ct_rows` id, which includes CT11.

CT11's membership comes from `docs/reviews/r0_3/acceptance.json`'s own hand-kept
`newly_decidable_not_mapped`, which reads `["CT11", "CT24", "CT47"]` — the two
records have disagreed on that list since before this card, and card
T00-CT-SCHEMA's report recorded exactly that ("`acceptance.json`'s …
`newly_decidable_not_mapped` were updated by later waves with … `CT11` in the
decidable list, while `acceptance_matrix.json` derives both from T00's triage.
They already disagreed before this card"). That divergence is recorded as a
follow-on (§6), not silently reconciled: whether CT11 is "decidable now" is
analysis, and card T05's own record says mapping it would answer a different
question.

The same report left this card's decision open in as many words — the key was
one of "three keys that were hand-added to the committed matrix and are not
produced by the generator … Decide deliberately whether to re-add them to
`ct_section` in `build_matrix.py` or let them go". It is decided here in the
third way, which that card did not have: **derive it**.

**Nothing is lost by dropping the row.** The same section publishes CT11's
blocker, with more provenance, in its own `ct_rows` entry:

```
"current_bucket":        "blocked-on-scenario-mismatch (no local alias form)",
"current_bucket_source": "docs/reports/remaining-wave3-ct-integration/ct_stage_evidence.json",
"blocker_current":       "The scenario reads 'full permission reconstructed while local shared
                          aliases survive'. ScopedReadSharesV1 has no local alias form at all …",
"blocker_measured_by":   "…README.md section 1.2 (Q600 LoanOutstanding { resource: 1,
                          outstanding: 512, borrowers: [3] } …)"
```

### 1.3 The derivation

`scripts/ct_stage_summary.py` gains two functions and one checker, and
`build_matrix.py` consumes them:

* `newly_decidable_ids(root)` — the published list, moved out of the generator's
  body into the shared summarizer so the generator and the checker read one
  function.
* `newly_decidable_reasons(root)` — **one row per id of that list**, each row
  being that case's resolved label and reason: the newest CT lane record that
  measured it, else card T00's frozen archive, exactly as `unmapped_labels`
  resolves the buckets (card CT-LABELS' rule). Both halves are published, so
  "this is a current measurement" and "this is 2026-09-07 archive prose" are
  legible at a glance:

  ```json
  "CT24": {
    "label": "DECIDABLE-NOW (with a distinguishing fixture)",
    "label_source": "t00-archive",
    "reason": "resolved: joins exist, and scope-exit cleanup is a distinct mechanism
               from an explicit drop",
    "measured_by": "t00-archive"
  }
  ```

* `check_matrix_ct_reasons(root)` — the drift check, wired into
  `ct_stage_summary.py --check`.

Keying the map to the list is the whole repair: **both** staleness classes
become impossible rather than guarded. A mapped case leaves the list in the same
commit that maps it (HYGIENE-14's structural argument, one field over), and a
row belonging to no published claim cannot be produced at all.

Why the *values* are the records' text rather than the editor's: prose nothing
derives is prose nothing can refuse. The derived text is the same text the CT
lanes and card T00 already wrote, published beside the record that decided it.

### 1.4 Both refusal directions, kept

| direction | refusal | test |
|---|---|---|
| a derived row for a mapped case | impossible by construction (the key set is the manifest-filtered list) **and** asserted in the generator and in `newly_decidable_reasons`; a committed row for a mapped case is reported by name | `test_a_matrix_reason_row_for_a_mapped_case_is_refused` |
| a committed row the derivation cannot produce | reported by name: *"carries a CT11 row the derivation does not produce"* | `test_a_committed_row_the_derivation_cannot_produce_is_refused`, `test_the_historical_ct11_row_is_refused` |
| a derived row the file drops (the direction that lost the key for four waves) | reported: *"is missing the derived CT24 row"*; a missing map at all is reported too | `test_dropping_the_map_or_a_row_of_it_is_refused` |
| a rewritten paragraph | reported: *"row for CT24 is not what the evidence records derive"* | `test_a_rewritten_reason_paragraph_is_refused` |

Nine new tests in `scripts/tests/test_ct_stage_summary.py` and two in
`scripts/tests/test_build_matrix_sections.py`. **None of them names a case id or
a count that is not read out of the committed manifest at run time**, except the
CT11 regression, which computes its expected message from the manifest's current
mapping status of CT11 so that a future lane mapping it moves the test to the
other refusal message rather than breaking it. This is deliberate: a sibling CT
lane owns the manifest this wave and will move CT coverage to 33/50.

### 1.4.1 Why there is no second derivation of this key

HYGIENE-14 and HYGIENE-16 mirrored their acceptance-record checks in
`compiler/tests/ct_corpus.rs`, so the two derivations refuse each other's drift.
This key is deliberately not mirrored: the acceptance **matrix** is
`build_matrix.py`'s output, and the Rust derivation reads the acceptance
**record**, which it can check without ever opening the matrix. Both records'
copies of the map are now covered — the record's by
`check_newly_decidable` in both derivations (unchanged, HYGIENE-16), the
matrix's by `check_matrix_ct_reasons` beside the generator that writes it. A
Rust reader of the matrix would add a third consumer of a file no Rust code
reads today, for no refusal the generator does not already make. Recorded as a
decision, not an omission.

### 1.5 The fixed-point proof

```
$ python3 -B docs/reports/remaining-d02/build_matrix.py .          # in place
$ python3 -B docs/reports/remaining-d02/build_matrix.py .          # again
$ cmp pass1.json docs/reviews/r0_3/acceptance_matrix.json          # identical
$ python3 scripts/ct_stage_summary.py . --check                    # exit 0
```

The committed matrix is a fixed point of the new generator: after one
regeneration the file is byte-stable under further runs, and the whole-file diff
against `8ec8402` is three change groups and nothing else (§4).

`scripts/tests/test_build_matrix_sections.py::test_the_committed_reasons_map_is_a_fixed_point_of_the_generator`
pins it — deliberately scoped to this one derived key rather than the whole
`ct_conformance` section, because the CT lane moves that section's rows and a
test pinning them would fail on the lane's commit instead of on a defect.

### 1.6 The carry-guard, retired loudly

`docs/reports/wave16-corpus/readd_ct_reasons.py` is **not edited** — it is
archived evidence of another card — and it does not need to be: its author left
the retirement tripwire in it. Running it against the derived matrix now:

```
$ python3 -B docs/reports/wave16-corpus/readd_ct_reasons.py
AssertionError: the generator no longer drops the map; retire D02-GEN-CTREASONS
exit=1
```

`test_the_wave16_carry_step_retires_itself_once_the_map_is_derived` executes
that retirement in a scratch git repository (the committed matrix is never
written), asserting the exit status and the message by name. The derivation's
own test supersedes the carry step rather than the step disappearing quietly.

### 1.7 Deviations found by measurement

1. **HYGIENE-16 recorded the map as "not derivable".** Measured, it is: its keys
   are its companion list's ids and its values are the same resolved reasons the
   bucket labels already come from. What was missing was the observation that
   the map is the list's prose, not a free-standing note.
2. **The committed map and `acceptance.json`'s copy disagree on CT11**, and have
   for several waves. Recorded, not reconciled (§6).
3. **One stale claim in a HYGIENE-16 test docstring** — "the matrix copy is
   hand-carried commentary no derivation reads" — is corrected in place; it is
   the sentence this card falsifies.

---

## 2. Item 2 — `DIFF-TOOL-CONTRACTS-BACKFILL`

### 2.1 Measured before-state, and a corrected card premise

The card names three affected documents (wave13→14, wave14→15 and the wave-16
consolidation's diff). Measured over the whole consolidation chain:

| document | schema | artifacts |
|---|---|---|
| `wave9-corpus/diff-e01f1-to-wave9` … `wave15-corpus/diff-wave14-to-wave15` (7 documents) | 1 | four; **no `contracts`** |
| `wave16-corpus/diff-wave15-to-wave16` | 2 | five, `contracts` included |

So the wave-16 consolidation's diff **already carries the section** — it was
written after HYGIENE-16's itemizer landed — and needs no backfill, while four
documents older than the two the card names are in exactly the same state. The
scope taken is the whole schema-1 consolidation chain: seven documents, each
regenerated from its own committed artifact pair.

Two further measurements made the backfill safe to attempt at all:

* every one of the eight documents **round-trips textually** under the byte form
  `diff_artifacts.py` itself writes (`json.dumps(indent=2, sort_keys=True)`),
  so splicing a key is a textual insertion rather than a reformat; and
* every document **names its own before-side** in its `archived` field, and its
  after-side is the artifact set committed beside it. No pair is supplied by
  hand.

### 2.2 The tool and its three rules

`docs/reports/wave17-gen-ctreasons/backfill_contracts.py`:

1. **Self-describing inputs.** The before-side comes out of the document; the
   after-side is `<report dir>/artifacts`.
2. **The document is its own oracle.** The four entries it already carries must
   reproduce **byte-identically** from that pair under the current tool, or the
   tool refuses and writes nothing. That is what proves the pair is the recorded
   run's own input rather than a plausible substitute. Measured: all seven
   reproduce, as does wave 16's.
3. **Append-only, and said so.** Stripping the two added paths must give back
   the original document's exact bytes — checked inside the tool before it
   writes, and again by the tests afterwards.

`schema_version` is deliberately **not** bumped: changing it would not be
append-only, and a reader is told what they are holding by the record instead:

```json
"backfilled_sections": {
  "artifacts.contracts": {
    "card": "GEN-CTREASONS", "follow_on": "DIFF-TOOL-CONTRACTS-BACKFILL",
    "report": "docs/reports/wave17-gen-ctreasons/README.md", "date": "2026-09-10",
    "tool": "docs/reports/corpus-repin/diff_artifacts.py", "tool_section_schema": 2,
    "document_schema_version_unchanged": 1,
    "inputs": {"archived": "docs/reports/wave13-corpus/artifacts",
               "measured": "docs/reports/wave14-corpus/artifacts"},
    "original_commit": "8ec8402", "original_sha256": "…",
    "verification": "the four artifact entries this document already carried reproduce
                     byte-identically from the two archived artifact sets named in `inputs` …",
    "note": "… a regeneration from committed bytes, not a re-measurement of the corpus:
             no audit was re-run and no verdict, count or pin is affected."
  }
}
```

The tool is idempotent (`DATE` and `ORIGINAL_COMMIT` are constants, gzip is
written with `mtime=0`), so `--check` rebuilds every section from committed bytes
and requires the committed document to be exactly what it produces.

### 2.3 What the backfilled sections say

All seven, from committed bytes:

| document | `sources` movement | note |
|---|---|---|
| wave9 → e01f1 baseline | 5 → 5, 0 changed | no contract movement |
| wave9 → wave10 | 5 → 5, 0 changed | P04-B1's 18 `contract_source` leaves are **diagnostics** leaves; the audit's selected inventory did not move |
| wave10 → wave11 | 5 → 5, 0 changed | |
| wave11 → wave12 | 5 → 5, 0 changed | |
| wave12 → wave13 | **+1** `qcore/fluent_bit/hash_table_publication.qcore` | HASH-CONTRACTS' corpus 60 → 61, now itemized |
| wave13 → wave14 | **+3**, **1 changed** | the recorded blind spot, reproduced exactly |
| wave14 → wave15 | **+3**, 0 changed | the second recorded blind spot |

The two hand-verified cases the card asks about reproduce identically to the
consolidation READMEs' text:

* **wave13 → wave14**: added `input_chunk_name.qcore`, `logging.qcore`,
  `routes_mask.qcore`; **changed** `hash_table_publication.qcore` (HASH-CLAUSE's
  retirement of the comment carrier — a content change, not an inventory growth,
  which is the information the hand comparison did not have);
* **wave14 → wave15**: added `chunkio/chunk_residency.qcore`,
  `fluent_bit/input_chunk_helpers.qcore`, `fluent_bit/time.qcore`.

`manifest.modules` moves with the inventory in both, `audit_selection.libraries`
does not move anywhere, and no `keys` row appears in any document — every
top-level key of `contracts.json` is accounted for in all seven.

### 2.4 Verification, three ways

1. **Reproducible.** `backfill_contracts.py --check` exits 0 over the committed
   archives, rebuilding every section.
2. **Append-only against the tool's own digest.** Stripping the two paths
   reproduces the `original_sha256` each record carries, and the only new paths
   are `backfilled_sections` and `artifacts.contracts`.
3. **Append-only against git.** For all seven, the stripped document is
   byte-identical to `git show 8ec8402:<path>` — the independent oracle, which
   does not rely on the tool's own record.

Plus the oracle's refusal direction, exercised rather than described: a document
whose `diagnostics` entry has been tampered with is refused with *"does not
reproduce"*, and nothing is written.

Seven tests in `scripts/tests/test_backfilled_contract_sections.py`.

### 2.5 One deliberate pin update

`scripts/tests/test_diff_artifacts_contracts.py::test_the_four_pre_existing_artifact_entries_are_byte_stable`
asserted the archived documents are `schema_version: 1` **with exactly the four
artifacts**. That half moves: they now carry five. It is replaced by a stronger
pair of assertions — the document carries the four plus `contracts`, and a
`contracts` section in a schema-1 document **must** declare itself backfilled —
and the load-bearing half, the four entries reproducing byte-identically from
the archived pair, is untouched.

### 2.6 What changed in the archived bytes, exactly

The gzip container changes for all seven (level 9 and `mtime=0`, for
reproducibility; the originals were level 6 with an OS byte). The decompressed
document grows by the spliced section and the record — for example
`wave9-corpus/diff-e01f1-to-wave9.json.gz`, 9,006,068 → 9,008,493 bytes
decompressed — and every pre-existing byte of it is unchanged, as §2.4 measures.

---

## 3. Lean

**Item 1 — landed: `proof/QCore/CtReasons.lean` + `proof/AuditCtReasons.lean`,
15 audited theorems, no new axiom, no placeholder** (`propext` only, except one
theorem on `Classical.choice`/`Quot.sound` and five that depend on nothing). The
derivation admits a small decidable agreement relation, so the card's preference
applies. The module imports `QCore.BucketTally` rather than restating the
resolution rule — the reasons map *is* that resolution restricted to a published
key set, and saying so in the import is the honest statement.

| theorem | what it settles |
|---|---|
| `keys_are_the_published_list` | the keys are the companion list, in order — the sentence that makes both staleness classes impossible |
| `every_selected_case_is_unmapped`, `select_drops_a_mapped_case` | refusal direction one, in both polarities |
| `an_unclassified_case_refuses` | a hole is refused, not published as an empty paragraph |
| `a_lane_reason_wins`, `the_archive_reason_decides_an_unmeasured_case` | the text is the record's; both halves of the resolution rule |
| `a_row_outside_the_derivation_is_reported` | refusal direction two — one rule catching both HYGIENE-16's CT39 shape and this card's CT11 shape |
| `a_missing_derived_row_is_reported` | the direction that lost the key for four waves |
| `equal_key_lists_report_nothing`, `mem_iff` | the check is not vacuously loud |
| `the_derived_list_is_ct24_and_ct47`, `the_ct11_row_is_refused`, `the_ct39_row_would_be_refused`, `the_committed_map_after_this_card_reports_nothing`, `mapping_ct24_retires_its_row` | the transcribed instance, **decided** rather than argued, including this card's measured before-state and HYGIENE-16's finding under the same rule |

Stated premise, not proved: that `scripts/ct_stage_summary.py` and
`docs/reports/remaining-d02/build_matrix.py` compute these functions, and that
the transcribed rows are the committed ones. The generator writing the key and
the checker refusing a hand edit to it in both directions is where that premise
is checked. Whether a case is *decidable now* is card T00's analysis, which no
relation derives.

**Item 2 — Lean judged not applicable**, on HYGIENE-16's recorded reasoning,
which this card re-examined rather than inherited: `diff_artifacts.py` decides
nothing — it consumes two documents and describes their difference, admits
nothing and refuses nothing, and no verdict, certificate or gate reads its
output. The backfill adds one property a relation *could* state — append-only —
and it is settled by byte comparison against the pre-backfill blobs (§2.4),
which is stronger evidence than a model of the splice would be.

---

## 4. Tally neutrality, verified explicitly

| figure | before (`8ec8402`) | after | moved? |
|---|---|---|---|
| Raw Cycle 19 units | 2 Safe / 42 provisional Unsafe / 137 Unknown / 4 data-only | unchanged | no — no corpus artifact, audit input, checker, contract or profile file is touched |
| CT coverage | 32 / 50 mapped, 18 unmapped | 32 / 50, 18 | no — `ct_manifest.json` is untouched (the sibling lane owns it) |
| `unmapped_bucket_counts`, `unmapped_label_sources`, stage histogram, depth profile, observation counts, case depth | — | byte-identical | no |
| `docs/reviews/r0_3/acceptance.json` | — | **untouched**; `--write` leaves it byte-identical | no |
| Contract corpus | 67 modules | 67 | no |
| `qcore-schemas` / `qcore-compiler` | 13.0.0 / 0.4.0 | unchanged | no |
| Archived audit artifacts (`artifacts/*.json.gz`) | — | **untouched** | no |
| Archived diff documents | four artifacts | five, append-only, marked | **yes, by design** (item 2) |
| Lean audit files | 72 | **73** | yes — this card lands one audit; HYGIENE-16's precedent |

The complete change to `docs/reviews/r0_3/acceptance_matrix.json` against
`8ec8402` is three groups and nothing else:

1. `ct_conformance.newly_decidable_not_mapped_reasons` — hand-kept prose (four
   keys) replaced by the derived map (a `_derivation` note plus the two rows the
   companion list names);
2. `validation.lean_audits` 72 → 73 and `AuditCtReasons.lean` in
   `lean_audit_files` — both derived from the tree, and this card's own audit;
3. `validation.measurement_provenance` moves from wave 16's integrator gate seal
   to the generator's "carried, NOT measured for this regeneration" label,
   because a card agent's gate is not the wave's merged gate. The integrator
   replaces it with `--gate` (HYGIENE-16 §5 recorded the same movement).

No `ct_rows` mapping status, no `totals`, no `unmapped_bucket_counts`, no
`unmapped_label_sources`, no `corpus` row, no requirement row and no
`named_followups` entry appears in that diff.

**Orthogonality to the sibling CT40 lane**, checked rather than assumed: the
derived map's key set is card T00's newly-decidable judgment ∩ the manifest's
unmapped set. CT40 is in neither, so mapping it moves nothing this card
publishes and trips no check this card adds. No test added here pins a CT count,
a case id from the manifest, or the contents of `ct_rows`.

---

## 4.1 Standing gate, this worktree

| gate | result |
|---|---|
| `cargo test --workspace --no-fail-fast` | **2746 passed, 0 failed, 8 ignored** (171 test binaries, exit 0) |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean, exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `PATH="$HOME/.elan/bin:$PATH" python3 scripts/check_lean_audits.py` | **73 audits** passed (72 + `AuditCtReasons.lean`) |
| `python3 -m unittest discover -s scripts/tests` | **179 passed** (161 at base + 18 new: 9 CT-summary, 2 build-matrix, 7 backfill) |
| `python3 scripts/ct_stage_summary.py . --check` | exit 0, no DRIFT, 32 mapped / 18 unmapped |
| `python3 -B docs/reports/wave17-gen-ctreasons/backfill_contracts.py --check` | exit 0 over all eight documents |

Environment notes, recorded rather than smoothed over:

* `third_party/libevent` was absent in this worktree and its pin test failed
  closed exactly as designed; `git submodule update --init --recursive` (the
  documented once-per-checkout step) resolved it to the pinned `d82464a…`.
* The first workspace run died in `ld` with a bus error while linking
  `qcore-c-gateway`'s `reach_gate_positions` test — the machine's root
  filesystem was at 100% with 1.1 GB free, several sibling worktrees building
  at once. That is an environment failure, not a test failure: the same target
  links and passes (10 tests) on retry.
* A second run was cut short at 130 binaries (killed, exit 144) and recorded
  one failure on the way: `vp04_vp07_vp08_process_lifecycle_and_payload_budgets`
  at `compiler/tests/cycle20_verifier_plugins.rs:794`, the wall-clock assertion
  `elapsed < Duration::from_millis(500)`. It is the recorded load-flake of that
  name, and its dependence on load is measured rather than assumed: **failed in
  isolation at load average 50, passed in isolation at load average 37**, on
  the same binary, minutes apart. This card changes no Rust at all — its whole
  diff is Python, JSON, Lean and documentation — so it cannot reach that test.
  The assertion is deliberately not weakened. The clean run above is a third,
  complete run in which it passes.

---

## 5. Files changed

| file | change |
|---|---|
| `scripts/ct_stage_summary.py` | `newly_decidable_ids`, `newly_decidable_reasons`, `check_matrix_ct_reasons`; the last wired into `--check` |
| `docs/reports/remaining-d02/build_matrix.py` | emits the derived map; asserts its keys are the published list and that no key is a mapped case |
| `docs/reviews/r0_3/acceptance_matrix.json` | regenerated through its owned generator (§4) |
| `scripts/tests/test_ct_stage_summary.py` | 9 new tests; `MutableRoot` carries the matrix; one stale docstring claim corrected |
| `scripts/tests/test_build_matrix_sections.py` | 2 new tests (fixed point; the carry step's retirement) |
| `proof/QCore/CtReasons.lean`, `proof/AuditCtReasons.lean` | **new**, 15 audited theorems |
| `proof/QCore.lean` | import the new module |
| `docs/reports/wave17-gen-ctreasons/backfill_contracts.py` | **new**, the backfill tool |
| `docs/reports/wave{9,10,11,12,13,14,15}-corpus/diff-*.json.gz` | backfilled `contracts` section + `backfilled_sections` record, append-only |
| `scripts/tests/test_backfilled_contract_sections.py` | **new**, 7 tests |
| `scripts/tests/test_diff_artifacts_contracts.py` | one pin deliberately updated (§2.5) |
| `docs/reports/wave17-gen-ctreasons/README.md` | this record |

Not touched: `AGENTS.md`, `CLAUDE.md`, `README.md`, `DEVELOPMENT.md`,
`docs/reevaluation/ct_manifest.json`, `docs/reviews/r0_3/acceptance.json`,
`docs/reports/wave16-corpus/readd_ct_reasons.py`, and every archived
`artifacts/` set. §7 holds the increment paragraph for the integrator.

---

## 6. Follow-ons

1. **`D02-CTREASONS-ACCEPTANCE`** (new, §1.2) — `docs/reviews/r0_3/acceptance.json`'s
   hand-kept `newly_decidable_not_mapped` reads `["CT11", "CT24", "CT47"]` while
   the matrix derives `["CT24", "CT47"]` from card T00's frozen judgment. The
   two records have disagreed for several waves. Deriving the acceptance
   record's pair the same way would settle it, but whether CT11 belongs in a
   "decidable now" list is analysis: card T05's own record says mapping it would
   answer a different question. Recorded, deliberately not decided here.
2. **`D02-BUCKETS-STALE-RETIRE`** (carried from HYGIENE-16 §7.1, now with a third
   entry) — the `named_followups` entries for `D02-BUCKETS-STALE`,
   `CT-LABELS-T00-REVERT` and, after this card, `D02-GEN-CTREASONS` and
   `DIFF-TOOL-CONTRACTS-BACKFILL` are closed at their stated boundaries but
   `named_followups` is `hand_maintained` and carried by the generator, so
   retiring them is an integrator edit this card deliberately did not make.
3. **`DIFF-TOOL-CARD-DIFFS`** (new, §2.1) — the per-card artifact diffs under
   `docs/reports/remaining-*/artifacts/` (fourteen documents) are outside this
   card's chain and were not examined. They belong to their own cards' evidence;
   backfilling them would need each card's own before/after pair confirmed by
   the same oracle.
4. **`CT-LABELS-FLAKE` / `P02-FLAKE`** (carried, unchanged) — the wall-clock
   assertions in the provider-process tests remain load-sensitive; §4.1 records
   this card's measurement of `vp04_vp07_vp08` (fails at load 50, passes at
   load 37, same binary). Neither assertion is weakened here.

---

## 7. Increment paragraph (draft, for the integrator)

> Ledger and archive hygiene (GEN-CTREASONS): the CT ledger's
> `newly_decidable_not_mapped_reasons` map is **derived** rather than carried,
> closing `D02-GEN-CTREASONS`, which HYGIENE-16 had recorded as not derivable.
> Measured, the map is the companion prose of a list the generator already
> derives — card T00's frozen newly-decidable judgment filtered by the
> manifest's own unmapped set — so its keys are now that list's ids by
> construction and each value is that case's resolved label and reason with the
> record that decided it published beside them. Keying it to the list makes both
> staleness classes structurally impossible rather than guarded, and the second
> one was found by doing it: beside HYGIENE-16's CT39 row (a paragraph about a
> case the manifest maps) the committed map carried a **CT11** row explaining a
> case its own published list does not name — invisible to the wave-16 carry
> guard, which computed its unmapped set from `ct_rows`. Nothing is lost by
> dropping it: the same section publishes CT11's blocker with more provenance in
> its own row. Both refusal directions land in
> `scripts/ct_stage_summary.py --check`, the committed matrix is a fixed point of
> its own generator (regenerated and diffed; the whole-file movement is the map,
> the derived Lean audit count and the gate-provenance label), and the wave-16
> carry step `readd_ct_reasons.py` retires itself loudly through its own
> tripwire — asserted by name in a test, with its archived bytes untouched.
> 15 audited Lean theorems (`QCore.CtReasons`), built on `QCore.BucketTally`
> because the reasons map is that resolution restricted to a published key set.
> `DIFF-TOOL-CONTRACTS-BACKFILL` closes over the whole schema-1 consolidation
> chain: seven archived diff documents gain a `contracts` section regenerated
> from their own committed artifact pairs, append-only behind a
> `backfilled_sections` record naming the tool, its section schema, the date, the
> inputs and the original document's digest — verified three ways, including
> against the pre-backfill blobs in git. Each document is its own oracle: the
> four entries it already carried must reproduce byte-identically from the pair
> it names or nothing is written, and that refusal is exercised. The card premise
> is corrected by measurement — the wave-16 consolidation's diff already carried
> the section, while four documents older than the two named are in the same
> state as those — and the backfill supplies information the hand comparisons
> could not have: HASH-CLAUSE's *content change* to
> `hash_table_publication.qcore` in wave 13 → 14, and a measured zero contract
> movement in waves 9 → 13 where the audit's selected inventory did not move at
> all. One pin was deliberately updated, keeping its load-bearing half. Nothing
> is promoted and no tally moves: verdicts, CT coverage, the corpus pin, the
> contract count and `acceptance.json` are all untouched; the only derived number
> that moves is the Lean audit count, 72 → 73, for this card's own audit.
> Evidence: `docs/reports/wave17-gen-ctreasons/README.md`.
