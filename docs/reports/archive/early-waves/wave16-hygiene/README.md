# HYGIENE-16 — three tally-neutral ledger and tooling repairs

Card **HYGIENE-16**, wave 16. Base commit `8069a99` ("Consolidate wave 15").
Three bounded follow-ons, each measured before it was touched:

| Item | Follow-on | Outcome |
|---|---|---|
| 1 | `D02-BUCKETS-STALE` | **already repaired** by HYGIENE-14 + CT-LABELS; verified, and the missing regression landed. One live adjacent staleness found and repaired (`CT39`). |
| 2 | `CT-LABELS-T00-REVERT` | card T00's frozen archive restored to its pre-CTWIN bytes; current labels left to the dual-publication mechanism. |
| 3 | `DIFF-TOOL-CONTRACTS` | worse than recorded — `contracts.json` was not diffed **at all**. The fifth artifact now itemizes, with both recorded blind-spot cases pinned. |

**Nothing is promoted, and nothing moves.** No verdict, no CT coverage number,
no corpus pin, no contract-corpus count changed; §5 records the verification.

---

## 1. Item 1 — `D02-BUCKETS-STALE`

### 1.1 Measured before-state, and the deviation from the card premise

The card brief and the acceptance matrix both say the hand-kept unmapped bucket
counts sum **22** against the real **18**. On `8069a99` they do not:

```
$ python3 scripts/ct_stage_summary.py . --check
mapped 32 / unmapped 18; depth {...}
exit=0
```

The committed `docs/reviews/r0_3/acceptance.json` block sums to 18, and both
derivations — `scripts/ct_stage_summary.py::check_acceptance` and
`compiler/tests/ct_corpus.rs::ct_acceptance_summary_is_derived_from_the_manifest`
— agree with it field for field. **The finding was closed two waves ago and its
ledger entry was never retired:**

* card **HYGIENE-14** made the counted *set* the manifest's own `mapping_status`,
  moved `unmapped_bucket_counts` into `generated_fields`, and added the
  sum-refusal in both derivations;
* card **CT-LABELS** (wave 15) made the *label* derived as well and added
  `unmapped_label_sources`.

So the reconciliation Item 1 asks for was already a fixed point. Re-running the
owned writer proves it rather than asserting it:

```
$ python3 scripts/ct_stage_summary.py . --write --check
wrote .../docs/reviews/r0_3/acceptance.json
mapped 32 / unmapped 18
$ diff acceptance-before.json docs/reviews/r0_3/acceptance.json   # byte-identical
```

The `named_followups` entry for `D02-BUCKETS-STALE` still reads "Recorded by the
wave-13 CT lane, not repaired". That section is `hand_maintained` and carried by
the generator; retiring the entry is an integrator edit, named in §7.

### 1.2 What a tree-reading test cannot show, and the regression that closes it

Because the committed record and the derivation agree, every existing test that
reads the real tree passes whether or not the refusal works. Card HYGIENE-14's
scratch tests cover the *forward* direction (mapping a case makes the committed
record drift). The direction that was missing is the historical one: a hand edit
that **re-inflates** the tally.

`scripts/tests/test_ct_stage_summary.py::test_the_historical_stale_bucket_block_is_refused`
restores the literal shape of the finding — the four-case epoch bucket wave 8
mapped, still counted, summing exactly 22 against `ct_unmapped: 18` — into a
scratch acceptance record, asserts the precondition that the committed record is
currently reconciled, and requires the drift to be reported by field name.

### 1.3 A live staleness found next door, and repaired

`check_newly_decidable` checked the hand-kept `newly_decidable_not_mapped` id
list against the manifest but not its companion
`newly_decidable_not_mapped_reasons` map, which is the same claim in prose. It
is now checked in **both** derivations (`scripts/ct_stage_summary.py` and
`compiler/tests/ct_corpus.rs`), with a leading-underscore key treated as the
map's own commentary rather than a case id.

Measured immediately: `docs/reviews/r0_3/acceptance_matrix.json`'s copy of that
map still carried a **CT39** row —

> "Refused at model level by FlatEpochBoundV1 (card T08), but there is no epoch
> source syntax, hence no parser stage and no reference execution."

— four waves after card T08-SRC-CT mapped CT35/CT36/CT38/CT39. The acceptance
record's own copy was clean (it lists CT11/CT24/CT47), which is why the
`ct_unmapped`-side refusals never saw it. The matrix row is dropped in this
card's regeneration and the map's `_note` corrected; the reason the defect was
invisible is recorded as follow-on `D02-GEN-CTREASONS` (§7).

### 1.5 A second staleness, found by regenerating: the generator's byte form

Adopting the generator's output verbatim produced an **11,798-line** diff for a
14-leaf change. Measured cause: `build_matrix.py` writes
`json.dumps(matrix, indent=2, ensure_ascii=True)`, and the committed matrix has
been `indent=1, ensure_ascii=False` since the original D02 wave — verified by
round-tripping the committed file, which re-serialises byte-identically under
the second form and not the first. The committed file carries 22 literal em
dashes, so the ASCII escaping was plainly never applied to it.

The consequence is that every *"Adopt the regenerated matrix after …"* commit in
the log (e.g. `e307ff6`, 6 insertions / 5 deletions) re-serialised the
generator's output by hand. The file's **formatting** was silently
hand-maintained, in a file whose whole `section_ownership` block exists so that
nothing is.

Repaired in the direction that moves nothing: the writer now emits the committed
byte form. The matrix diff for this card is consequently **13 lines** and
reviewable as its own change. No test pinned either form — both derivations and
`test_build_matrix_sections.py` read the file through `json.loads` — and the
generator's semantic output is bit-for-bit the same object.

### 1.4 Deliberate pin update

`test_newly_decidable_ids_must_still_be_unmapped` asserted the exact
one-element problem list for a scratch tree in which CT11 is mapped. CT11 is
named in **both** hand-kept halves, so the widened check now reports it twice.
The pin was widened, not loosened: the original message is still asserted
verbatim and the new one is asserted beside it.

---

## 2. Item 2 — `CT-LABELS-T00-REVERT`

### 2.1 Measured before-state

`docs/reports/remaining-t00/coverage.json` is described everywhere as card T00's
audit *frozen at revision ea2af0f*. It has exactly two commits:

```
7da4f4e3a4e8e057c9f08714b5555480089c761e  Record T00 CT stage-coverage audit
97e69c082fe538c55dbb6121443c58d3fd50030e  Map CT43 and CT45 as C twins on a new legacy_c harness route
```

The second (card CT-CTWIN, wave 9) rewrote CT40/CT41/CT42's `current_bucket`,
`blocker`, `evidence` and `bucket_changed_since_batch2` in place, preserving the
prior text under `superseded_*` keys, and appended a trailing newline. Card
CT-LABELS measured this, built the dual-publication mechanism that makes the
in-place edit unnecessary, and named the restoration for a later card rather
than moving a pin it was not asked to move.

### 2.2 What changed

```
git checkout 7da4f4e -- docs/reports/remaining-t00/coverage.json
```

A byte-exact restoration of the blob at `7da4f4e` — 13 insertions, 22
deletions, including the original file's missing final newline, because "the
frozen archive's bytes" is the thing being restored. No other file in
`docs/reports/remaining-t00/` is touched.

The three cases' **current** labels do not come from that file any more: they
resolve from `docs/reports/wave9-ctwin/ct_stage_evidence.json`'s
`unmapped_reasons` rows under CT-LABELS' rule (*the newest CT lane record that
measured this case, else card T00's frozen archive*), and
`unmapped_label_sources` still names that lane for all three. What the archive
now holds beside them is T00's original 2026-09-07 text, published as
`archived_bucket`/`blocker` in the matrix — so the history is **visible** rather
than overwritten, which is the whole point of the mechanism.

### 2.3 Why it is tally-neutral, three ways

1. **Measured.** `ct_stage_summary.py . --check` exits 0 after the revert, and
   the writer leaves `acceptance.json` byte-identical (§1.1). Bucket counts,
   label sources, `ct_mapped`/`ct_unmapped` and every stage figure are unchanged.
2. **Tested generally.** A new test rewrites *all three* archive rows to an
   invented bucket in a scratch tree and requires the counts, the sources and
   `check_acceptance` to be unmoved — while asserting non-vacuity, that a case
   no lane measured (CT44) still reads its label out of the archive.
3. **Proved.** `QCore.BucketTally.archive_edits_under_lane_coverage_are_invisible`:
   if every case two archives disagree on is a case some lane measured, the two
   archives resolve identically; the tally corollary follows, and the transcribed
   CT40/CT41/CT42 instance is decided outright (§4).

### 2.4 Deliberate pin update

`test_a_measured_label_wins_over_the_archive_and_says_so` asserted, for
CT40/CT41/CT42, that `label == archived_label`. That equality was an *artifact
of the in-place edit* — the card's own comment said so. It is now inverted
deliberately, with both texts pinned by name:

| case | `label` (from the wave-9 lane) | `archived_label` (T00, restored) |
|---|---|---|
| CT40, CT41, CT42 | `blocked-on-gateway-expressiveness (C-twin measured)` | `blocked-on-harness-schema (C-twin stage)` |

The load-bearing halves of that test — which lanes measured which cases, CT11's
and CT46's moved labels, and the sum reconciling to `ct_unmapped` — are
untouched.

---

## 3. Item 3 — `DIFF-TOOL-CONTRACTS`

### 3.1 Measured before-state, and the deviation from the card premise

The follow-on says `diff_artifacts.py` "does not row-itemize `contracts.json`'s
top-level keys". Measured, it is stronger than that: `contracts` is **not in
`ARTIFACTS` at all**, so the file is never opened. Both archived diff documents
confirm it —

```
$ python3 -c 'read diff-wave13-to-wave14.json.gz'   → artifacts: chunk-contracts, diagnostics, source, source-effects
$ python3 -c 'read diff-wave14-to-wave15.json.gz'   → artifacts: chunk-contracts, diagnostics, source, source-effects
```

— while `artifacts/manifest.json` in every archive from CORPUS-REPIN onward
(`wave9`…`wave15`, eight sets) carries a fifth entry,
`contracts.json.gz` → `resource-diagnostics.contracts.json`. The wave-14
consolidation verified a deliberately grown inventory (corpus 61 → 64) by direct
comparison past the tool; wave 15 repeated that by hand for 64 → 67.

### 3.2 Why the four-way row diff could not have been reused

The four existing artifacts are documents holding row **lists** under known
field names (`units`, `results`, `backend_teardown`, `callbacks`, `wrappers`),
which is what `diff_rows` keys and itemizes. `contracts.json` has none of them.
Its six top-level keys are:

| key | shape |
|---|---|
| `sources` | the inventory: absolute path → contract module text |
| `manifest` | a JSON **document carried as a string**, listing the selected modules |
| `audit_selection` | likewise, listing the audited libraries |
| `source_profile` | a plain map (source set, profile digest, pins) |
| `schema_version`, `trust` | scalars |

Putting it through `ARTIFACTS` would have produced an entry with an empty
envelope and no rows — exactly the silence that was the finding.

### 3.3 What changed

A dedicated `diff_contracts` itemizer, plus the fifth artifact in `main`:

* **`sources`** — added, removed and per-module changed rows, keyed by the module
  path below `contract/`. The raw keys carry the checkout root and the
  unnormalised `compiler/../contract/profiles/<set>/../..` walk the loader
  spells, so two runs from different directories name the same module
  differently (the "path-literal provenance" residue class the wave-9
  consolidation recorded); normalising is what makes a module comparable across
  runs, and **both** keys are published so the normalisation stays auditable.
  A normalisation that collapsed two distinct keys would silently merge two
  modules into one row, so a collision demotes the whole section to raw keys and
  publishes `key_collisions` — it neither merges silently nor refuses to report.
* **`manifest` / `audit_selection`** — parsed and itemized by their own entries
  (modules; libraries by `library`), with every other field of the embedded
  document compared field by field. A value that does not parse is compared
  opaquely and says `parsed: false` rather than being dropped.
* **`keys`** — every remaining top-level key, so a future addition is visible
  rather than absent.
* Contract text travels only under `--full`; the default rows carry SHA-256 and
  byte length, because a module body is kilobytes.
* The document's `schema_version` moves **1 → 2**, so a reader can tell which
  shape they are holding.

### 3.4 The two recorded blind-spot cases, now visible

Both run on committed bytes (archived artifact set vs. the next wave's archived
set, decompressed under its produced file names):

**wave 13 → wave 14** (the case the finding records; corpus 61 → 64):

```
sources: {added: 3, removed: 0, changed: 1, before: 6, after: 9}
  added   qcore/fluent_bit/input_chunk_name.qcore
          qcore/fluent_bit/logging.qcore
          qcore/fluent_bit/routes_mask.qcore
  changed qcore/fluent_bit/hash_table_publication.qcore
manifest.modules: {added: 3, removed: 0}
```

The **changed** row is information the hand comparison did not have: the wave-14
report could only describe the artifact as "`sources`/`manifest` keys growing
13.5 → 30 KB", and HASH-CLAUSE's retirement of the comment carrier inside
`hash_table_publication.qcore` is not an inventory growth at all.

**wave 14 → wave 15** (corpus 64 → 67, card NEIGHBOURS):

```
sources: {added: 3, removed: 0, changed: 0, before: 9, after: 12}
  added   qcore/chunkio/chunk_residency.qcore        (6215 bytes)
          qcore/fluent_bit/input_chunk_helpers.qcore (6745 bytes)
          qcore/fluent_bit/time.qcore                (3506 bytes)
manifest.modules: {added: 3, removed: 0}
audit_selection.libraries: {added: 0, removed: 0, changed: 0}
keys: {}
```

### 3.5 Byte-stability of everything else

Both archived diff documents reproduce **exactly** from the two committed
artifact archives with the pre-change tool, which makes them a real oracle
rather than a re-implementation:

| pair | four pre-existing entries |
|---|---|
| wave13 → wave14 (`diff-wave13-to-wave14.json.gz`) | `chunk-contracts`, `diagnostics`, `source`, `source-effects` all byte-identical |
| wave14 → wave15 (`diff-wave14-to-wave15.json.gz`) | all four byte-identical |

`scripts/tests/test_diff_artifacts_contracts.py` pins this: it asserts the
archived documents are `schema_version: 1` with exactly those four artifacts,
regenerates with the current tool, requires `schema_version: 2`, requires each
of the four entries to serialize identically to the archived one, and requires
the artifact set to be exactly those four plus `contracts`. No other diff
semantics changed: `diff_rows`, `row_key`, `envelope`, `histogram` and the
`ARTIFACTS` loop are untouched.

### 3.6 The synthetic pair

Nine further tests drive a synthetic before/after `contracts.json` covering every
way the artifact can differ: a module added, a module removed, a module's text
changed (both digests and both raw paths published), the text travelling only
under `--full`, the embedded manifest itemized by module, the embedded audit
selection itemized by library, a plain key compared field by field, an
unrecognised top-level key surfacing under `keys`, an unparsable embedded
document reported rather than dropped, and a normalisation collision falling
back to raw keys. The two checkout roots in that fixture differ, and the
after-side spells one module through a `profiles/<set>/../..` walk, so the
normalisation is exercised rather than assumed.

---

## 4. Lean applicability, per item

Card T00-CT-SCHEMA's precedent is that a CT-ledger schema change may record
"Lean not applicable" with reasoning. That is taken for one item here and
declined for two, because items 1 and 2 share one decidable relation worth
stating.

**Landed: `proof/QCore/BucketTally.lean` + `proof/AuditBucketTally.lean`, 14
audited theorems, no new axiom, no placeholder** (`propext`/`Quot.sound` only;
one theorem depends on nothing). It models the label-resolution and bucket-tally
relation both derivations compute:

| theorem | what it settles |
|---|---|
| `total_bump`, `total_tally` | the tally accounts for every label it is given |
| `resolve_length`, `tally_total_is_the_case_count` | one row per unmapped case, and the published sum equals that count — **item 1**'s property, the one whose silent violation was the finding |
| `dropping_a_case_drops_the_total` | why making the counted set the manifest's own set is sufficient: mapping a case moves the total with nobody editing the tally |
| `an_unclassified_case_refuses` | a hole is refused, never a zero |
| `a_lane_row_wins`, `the_archive_decides_an_unmeasured_case` | the resolution rule, both halves |
| `archive_edits_under_lane_coverage_are_invisible`, `…_move_no_count` | **item 2**'s soundness argument in general form |
| `the_ctwin_restoration_publishes_the_same_rows` / `…_same_tally` | the same, *decided* on the transcribed CT40/CT41/CT42 instance |
| `the_archive_still_decides_ct44` | non-vacuity: the archive is not dead, it is out-ranked for measured cases only |
| `the_transcribed_instance_accounts_for_every_case` | non-vacuity of the sum on that instance |

Stated premise, not proved: that `scripts/ct_stage_summary.py` and
`compiler/tests/ct_corpus.rs` compute these functions, and that the transcribed
rows are the committed ones. Both derivations refuse each other's output against
the committed record, which is where that premise is checked. Which bucket a
case *belongs* in is analysis no relation derives.

**Item 3 — Lean judged not applicable.** Reasoning, recorded rather than
assumed: `diff_artifacts.py` decides nothing. It is a read-only reporting tool
that consumes two JSON documents and emits a description of their difference;
it admits nothing, refuses nothing, and no verdict, certificate or gate reads
its output — a consolidation author does. There is no relation whose soundness
a theorem could state that is not simply "this is what a diff is". The property
that *does* matter — that the four pre-existing entries did not move — is
settled by byte comparison against two committed pre-change outputs, which is
stronger evidence than a model of a re-implementation would be.

---

## 5. Tally neutrality, verified explicitly

Every number this card could have moved, measured before and after:

| figure | before (`8069a99`) | after | moved? |
|---|---|---|---|
| Raw Cycle 19 units | 2 Safe / 42 provisional Unsafe / 137 Unknown / 4 data-only | unchanged | no — no corpus artifact, audit input, checker or contract file is touched |
| CT coverage | 32 / 50 mapped, 18 unmapped | 32 / 50, 18 | no |
| `unmapped_bucket_counts` | 10 buckets, sum 18 | identical map | no |
| `unmapped_label_sources` | 5 measured / 13 `t00-archive` | identical map | no |
| stage histogram, depth profile, observation counts, case depth | — | byte-identical | no |
| `docs/reviews/r0_3/acceptance.json` | — | **byte-identical** after `--write` | no |
| Contract corpus | 67 modules | 67 | no |
| `qcore-schemas` / `qcore-compiler` | 12.0.0 / 0.4.0 | unchanged | no |
| Lean audit files | 67 | **68** | yes — this card adds one audit |

The complete change to `docs/reviews/r0_3/acceptance_matrix.json` — 13 changed
lines against `8069a99`, leaf by leaf (14 leaves; §1.5 is why the diff is
readable at all):

* 9 leaves — `archived_bucket`, `blocker`, `bucket_changed_since_batch2` for the
  three CT40/CT41/CT42 rows (item 2, by construction);
* 1 leaf — the stale `CT39` reason row dropped (§1.3);
* 1 leaf — that map's `_note` corrected;
* 2 leaves — `lean_audits` / `lean_audit_files` 67 → 68 (derived, this card's
  audit);
* 1 leaf — `measurement_provenance` moves from wave 15's integrator gate seal to
  the generator's "carried, NOT measured for this regeneration" label, because a
  card agent's gate is not the wave's merged gate. The integrator replaces it
  with `--gate`.

No `ct_rows` mapping status, no `totals`, no `unmapped_bucket_counts`, no
`unmapped_label_sources`, no `corpus` and no requirement row appears in that
diff.

---

## 6. Files changed

| file | change |
|---|---|
| `docs/reports/remaining-t00/coverage.json` | **restored** to the blob at `7da4f4e` (item 2) |
| `scripts/ct_stage_summary.py` | `check_newly_decidable` extended to the companion reasons map |
| `compiler/tests/ct_corpus.rs` | the same check, mirrored in the Rust derivation |
| `scripts/tests/test_ct_stage_summary.py` | 4 new tests; 2 pins deliberately updated (§1.4, §2.4) |
| `scripts/tests/test_diff_artifacts_contracts.py` | **new**, 11 tests |
| `docs/reports/corpus-repin/diff_artifacts.py` | fifth artifact + `diff_contracts` itemizer; document schema 1 → 2 |
| `proof/QCore/BucketTally.lean`, `proof/AuditBucketTally.lean` | **new**, 14 audited theorems |
| `proof/QCore.lean` | import the new module |
| `docs/reports/remaining-d02/build_matrix.py` | writer emits the committed byte form (§1.5); no semantic change |
| `docs/reviews/r0_3/acceptance_matrix.json` | regenerated through the owned generator; hand-maintained key re-added minus its stale row |
| `docs/reviews/r0_3/acceptance.json` | rewritten by its owned writer; **byte-identical** |
| `docs/reports/wave16-hygiene/README.md` | this record |

Not touched: `AGENTS.md`, `CLAUDE.md`, `README.md`, `DEVELOPMENT.md` — §8 holds
the increment paragraph for the integrator.

---

## 6.1 Standing gate, this worktree

| gate | result |
|---|---|
| `cargo test --workspace` | **2638 passed, 0 failed, 8 ignored** (162 test binaries, exit 0) — unchanged against the carried figure, because this card adds no workspace test; the widened `ct_corpus.rs` assertion lives inside an existing one |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `python3 scripts/check_lean_audits.py` | **68 audits** passed (67 + `AuditBucketTally.lean`) |
| `python3 -m unittest discover -s scripts/tests` | **161 passed** (146 at base + 4 CT-summary + 11 diff-tool) |
| `providers/`: `cargo test` | 108 passed, 0 failed — untouched by this card |
| `python3 scripts/ct_stage_summary.py . --check` | exit 0, no DRIFT, 32 mapped / 18 unmapped |

Environment note: `third_party/libevent` was absent in this worktree and its pin
test failed closed exactly as designed; `git submodule update --init
--recursive` (the documented once-per-checkout step) resolved it to the pinned
`d82464a…` before the run above.

---

## 7. Follow-ons

1. **`D02-BUCKETS-STALE-RETIRE`** (ledger) — the `named_followups` entries for
   `D02-BUCKETS-STALE` and `CT-LABELS-T00-REVERT` still read as open. Both are
   closed at their stated boundaries by this card and by HYGIENE-14/CT-LABELS
   before it; `named_followups` is `hand_maintained` and carried by the
   generator, so retiring them is an integrator edit this card deliberately did
   not make.
2. **`D02-GEN-CTREASONS`** (new, §1.3) — `build_matrix.py` owns `ct_conformance`
   wholly and drops `newly_decidable_not_mapped_reasons` on every regeneration,
   so that map is re-added by hand each time and no derivation reads it. That is
   why its CT39 row survived four waves. Making it structural means either
   labelling `ct_conformance` `mixed` with a field-level owner (which moves the
   pinned `DERIVED`/`MIXED` contract in
   `scripts/tests/test_build_matrix_sections.py`) or deriving the map, which it
   is not derivable. Recorded, not repaired; the acceptance record's copy of the
   same map **is** now checked in both derivations, which is where staleness can
   actually reach a published number.
3. **`DIFF-TOOL-CONTRACTS-BACKFILL`** (new) — eight archived artifact sets
   (`corpus-repin`, `wave9`…`wave15`) carry `contracts.json.gz`, and their
   committed diff documents predate this itemizer. Re-running the tool over the
   older pairs would show which contract-inventory movements earlier
   consolidations described only in prose. Deliberately not done here: it would
   rewrite archived evidence documents belonging to other cards.
4. **`CT-LABELS-FLAKE`** (carried, unchanged) — the `standalone_host`
   wall-clock observation CT-LABELS recorded as informational is untouched.

---

## 8. Increment paragraph (draft, for the integrator)

> Wave-16 hygiene (HYGIENE-16): three tally-neutral ledger and tooling
> follow-ons close, none of which moves a number. `D02-BUCKETS-STALE` was
> measured **already repaired** — HYGIENE-14 made the counted set structural and
> CT-LABELS made the label derived, so the committed acceptance record is a
> fixed point of its own writer — and the card supplied the regression a
> tree-reading test cannot show: the literal sum-22 shape re-inflated into a
> scratch record and refused by field name. Extending the same check to the
> companion `newly_decidable_not_mapped_reasons` map, in both derivations,
> immediately found one live staleness (the acceptance matrix still explained
> why CT39 was unmapped, four waves after wave 8 mapped it) which is dropped.
> `CT-LABELS-T00-REVERT` restores card T00's frozen `coverage.json` to its
> pre-CTWIN bytes (`7da4f4e`, byte-exact including the original missing final
> newline); CT40/CT41/CT42's current labels come from the wave-9 lane's own
> record through CT-LABELS' dual publication, and T00's original text is now
> visible beside them instead of overwritten — verified neutral three ways, one
> of them a proof. `DIFF-TOOL-CONTRACTS` was measured worse than recorded:
> `contracts.json` was not diffed at all, in any archived diff document. The
> fifth artifact now itemizes contract-module adds, removals and content
> changes, both embedded JSON documents, and every remaining top-level key; the
> two recorded blind-spot cases reproduce on committed bytes, and the four
> pre-existing artifact entries are byte-identical to the two archived diff
> documents the pre-change tool produced. 14 audited Lean theorems
> (`QCore.BucketTally`) cover the tally and the archive-invisibility argument;
> Lean is judged not applicable for the diff tool with the reasoning recorded.
> Two pins were deliberately widened, each keeping its load-bearing half. One
> further finding fell out of regenerating: `build_matrix.py`'s writer has never
> emitted the committed matrix's byte form (`indent=2, ensure_ascii=True`
> against a file that is `indent=1, ensure_ascii=False` and carries 22 literal em
> dashes), so every "adopt the regenerated matrix" commit re-serialised its
> output by hand and a verbatim adoption would have reformatted 11,798 lines
> around a 14-leaf change; the writer now emits the committed form and this
> card's matrix diff is 13 lines.
> Evidence: `docs/reports/wave16-hygiene/README.md`.
