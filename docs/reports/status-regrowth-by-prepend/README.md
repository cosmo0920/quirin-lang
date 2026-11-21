# `STATUS-REGROWTH-BY-PREPEND`: STATUS can no longer regrow by prepending

**Base:** `bdad6a04e`, the STATUS-COMPACT tip, not yet landed. The final merge
of `origin/main` is described in §5. **Scope:** tooling only. The card changed
`scripts/apply_amendments.py`, `scripts/land.py`, the new
`scripts/unpublished_landings.py`, the new
`docs/development/landing-publications.json`, their tests, the operations text
that describes them, and the regenerated unpublished-landings sentence of
`docs/STATUS.md`. No checker semantics changed, so no Rust or Lean parity was
needed.

[STATUS-COMPACT](../status-compact/README.md) found two mechanisms behind the
page's growth from 23,931 to 290,911 bytes. First, `apply_amendments.py`
prepended each card's `status_paragraph`. Second, the integrator lengthened the
unpublished-landings sentence with `sed` at every landing. It registered
`STATUS-REGROWTH-BY-PREPEND` with three proposals, which the user approved. All
three are implemented.

## 1. Section-targeted status edits replace the prepend (schema `quirin-amendments/2`)

**Design.** `status_paragraph` is gone from the current schema. In its place,
`status_edits` is a required list (it may be empty) of
`{"section", "old", "new"}`:

- `section` is a heading line of `docs/STATUS.md`, written exactly as on the
  page (for example `## Latest validated gate`), and it must occur exactly
  once. The section runs to the next heading of the same or a higher level.
  Headings inside code fences do not count.
- `old` must occur exactly once in the section body, and `new` replaces it.
  `new` may be empty, which removes a table row. It must differ from `old`.
- Edits apply in order, each to the result of the one before.
- An edit is refused when its heading is absent or repeated, when `old` is
  absent from the section or repeated in it, or when it touches the generated
  unpublished-landings block. An edit set that leaves the page over 32 KiB
  while growing it is also refused. When anything is refused, nothing is
  written.

Find-and-replace inside a named section is the smallest form that covers
"replace the row", "update the figure" and "restate the sentence" alike. It
cannot add a dated paragraph above the page without naming where that
paragraph goes. The authority is the module docstring of
`apply_amendments.py`.

**Version.** The schema is bumped to `/2` because the meaning of the
status key changed. `/1` is still read:

| Input | Without `--legacy` | With `--legacy` |
|---|---|---|
| `/2` with `status_edits` | applies the edits in place | same |
| `/1` with `"status_paragraph": null` (110 committed files, and every queued card) | applies exactly as before: no STATUS edit | same |
| `/1` with a non-null `status_paragraph` (29 committed, historical) | **refused**: "a non-null status_paragraph is retired … write schema 'quirin-amendments/2' with status_edits … --legacy replays a committed historical file" | replayed as it was (prepended) |
| pre-schema file (18 committed) | refused: unknown keys | normalised, as before, into a `/1` document |

**What the replay test needs.** `ParityTests` replays AUDIT-CANONICAL-PATHS
onto `c3d8ad3d4^1`, and STATUS must come out byte-identical. That file has a
non-null status paragraph. The legacy replay is unchanged. The schema replay
now uses the `/1` form: it asserts that the form is refused as new input, then
replays it with `--legacy`. Both stay byte-identical.
`test_every_committed_file_validates_or_normalises` now asserts that each of
the 29 non-null `/1` files is refused as new input and validates under
`--legacy`, and that every other schema file validates as it stands.

**Docs.** In the operations [card-brief §5](../../agents/operations.md#card-briefs-and-delegation),
new cards now write `/2` and state a changed fact as a `status_edits` entry. The
[integration bullet](../../agents/operations.md#integration-and-reconciliation)
says that `/1` with a null paragraph still applies, that a non-null one is
refused, and that the sentence is generated.

## 2. Size bound

`python3 scripts/land.py check` fails when `docs/STATUS.md` exceeds 32,768 bytes.
A page of exactly 32,768 bytes passes. The failure reads:

```
land: stale: docs/STATUS.md is 32,769 bytes, over the 32,768-byte bound; run first: compact it as docs/agents/operations.md#maintenance-cadence says (re-derive the figures from primary sources and move unique history verbatim into an indexed checkpoint; docs/reports/status-compact/README.md is the worked example), never by raising the bound
```

The page is 23,691 bytes after this card. Its sentence grew by 383 bytes,
because it now also names r8, r9 and the two side lanes (§3).

## 3. The generated unpublished-landings sentence

**Chosen mechanism.** An explicit command writes the sentence, and
`land.py check` fails while it is stale. This is the same pattern as the matrix,
the report index and the contributions page. The command is:

```sh
python3 scripts/land.py unpublished --write
```

Without `--write`, the command lists every landing and the records it could not
fully parse. It is step 5 of `land.py`'s sequence, after the contributions
render and before `land.py check`. The generator is
`scripts/unpublished_landings.py`. It rewrites everything from the
`<!-- unpublished-landings: … -->` line to `<!-- unpublished-landings:end -->`,
and the start line now says the block is generated and must never be edited by
hand.

**Inputs.**

- *Landing gate records.* These are `docs/reports/*/integration/gate.json`
  files whose `note` begins `Landing integrate/<label>`. They are listed with
  `git ls-files --cached --others --exclude-standard`, so a record written at
  step 1 counts before it is staged, and ignored `private/` content never
  counts. Records with an identical label and note are one landing (r8 left
  three copies). The cards come from the `NAME <revision>` pairs of the note's
  `merges …` clause, and parenthesised asides are skipped.
- *Publication records.* There are two kinds:
  - Every live profile's `baseline_history` entries whose reason begins
    `Landing <rev>`. These are the only committed publication evidence today:
    `556460e31` (eight-home, fluent-bit-system) and `777fba16e`.
  - A new committed file, `docs/development/landing-publications.json`
    (schema `quirin-landing-publications/1`, empty today). `land.py publish`
    checks this file before it measures, and appends `{revision, previous,
    published_at, baseline}` to it after it publishes. Before this card,
    `publish` wrote only `private/baselines/<sha>`. A publication that moved
    no baseline therefore left no committed trace, and a sentence generated
    from the tree could never have dropped that landing. The file closes that
    gap.
- A landing counts as published when one of its gate records is present in a
  published revision's tree (`git cat-file -e REV:path`). A published revision
  missing from the object store is an error, not a default.

**Output on the merged tree** (§5), compared with the hand-written sentence:

| | Hand-written (r50 on main) | Generated |
|---|---|---|
| Newest landing baseline record | `556460e31` (eight-home and fluent-bit-system, `497bdeb90`) | `556460e31` (eight-home and fluent-bit-system, [record](../landing-556460e31/README.md)) |
| Unpublished landings | "the 37 landings from integrate/r10 to r47" (in the checkpoint), r48, r49, r50 = 40 | **44**: r8 to r50, both r15s, integrate/ag-gaps, integrate/cg25-thread-key |
| Gaps | not stated | "r18 and r40 have no unpublished landing gate record" |
| Per-landing list | checkpoint part 1 (frozen) | `python3 scripts/land.py unpublished` (generated) |
| Deferral statement, `--previous` | as written | unchanged text, `--previous 556460e31` |

The meaning is reproduced: the same newest record, the same deferral, and the
same instruction. The differences are four landings the hand list had omitted
since `d7a497822`. These are r8 (52e3142d5: W45-FB-ENGINE-PROGRESS-MEASURE,
MEASURE-LOOKUP-RECORD-REUSE, LICENCE-HEADER-TEST-REPAIR), r9
(LIST-ENTRY-RELEASE-IDENTITY), and the side-lane landings integrate/ag-gaps
(AG1/AG2/AG3) and integrate/cg25-thread-key (CG25-VOID-CAST-GUARD-ESCAPE,
CG25-THREAD-KEY-AUTOMATIC-REVISIT). None of their gate records is in
`556460e31`'s tree, and no committed record shows any of them published, so
counting them as unpublished is the fail-closed reading. This is registered as
`UNPUBLISHED-LANDINGS-HAND-LIST-OMISSIONS`. The hand list also gave r16 as
W48-LIBCURL-CONTRACT-PLUGIN. The records name THREAD-LOCAL-CUSTODY-FREE-READ
and CURRENT-CONTEXT-NEW-TRIAGE D3 for r16, and W48-LIBCURL-CONTRACT-PLUGIN's
record is r17's, the merge of both lanes. The generated sentence spells out
only the first and last numbered landings and the side lanes. As landings
accrue, only the count and the last landing change, so the sentence no longer
grows.

**Records without a parsable rN or card** (listed by `land.py unpublished`):

| Record | Problem | Named as |
|---|---|---|
| `ag1-indirect-member-access/integration/gate.json` | label `integrate/ag-gaps`, no rN | ag-gaps |
| `cg25-void-cast-guard-escape/integration/gate.json` | label `integrate/cg25-thread-key`, no rN | cg25-thread-key |
| `w48-libcurl-contract-plugin/integration/gate.json` (r17) | "the merge of origin/main …", no `merges CARD <rev>` | W48-LIBCURL-CONTRACT-PLUGIN (directory) |
| `busy-state-fluent-bit-provider/integration/gate.json` (r21) | merges branch `move/busy-state-fluent-bit` | BUSY-STATE-FLUENT-BIT-PROVIDER (directory) |
| `windows-ci-plain-long-fixtures/integration/gate.json` (r42) | merges branch `fix/windows-plain-long-fixtures` | WINDOWS-CI-PLAIN-LONG-FIXTURES (directory) |

r18 and r40 have no landing gate record. Of the 176 `integration/gate.json`
files on the merged tree, 46 are landing records (44 landings) and 130 are card-integration gates that are not `Landing integrate/…`
records, and they are not landings. These are registered as
`LANDING-GATE-RECORD-UNSTRUCTURED`.

## 4. The integrator's new steps, replacing the `sed` step

After `apply_amendments.py` and the landing gate:

1. Write `docs/reports/<card>/integration/gate.json` with a note beginning
   `Landing integrate/rN (base main …; merges CARD <rev> …)`, as now.
2. `python3 docs/reports/remaining-d02/build_matrix.py . --gate <gate.json>`
3. `python3 scripts/build_docs_index.py`
4. `python3 scripts/contribution_ledgers.py render`
5. **`python3 scripts/land.py unpublished --write`** (replaces the `sed` edit of
   STATUS; the new gate record adds the landing)
6. `python3 scripts/land.py check` (must exit 0; also enforces the 32 KiB bound)
7. Commit the records. 8. Run the Python suite.
9. If `land.py publish` runs, it appends to
   `docs/development/landing-publications.json`. Then run
   `python3 scripts/land.py unpublished --write` again and commit both files.

When the integrator merges this card, it should resolve a conflict in
`docs/STATUS.md`'s sentence by rerunning step 5, not by hand.

## 5. Main revision folded in

`git fetch origin` found `origin/main` at `590f7d9f1`, the integrate/r50 landing
record (CG10-OWNED-OUT-PARAMETER-MEMBER-SLOT). It was merged in `0369504c0`.
The only conflict was in `docs/STATUS.md`. r50's only STATUS edit was the `sed`
extension `r49 (…) and r50 (CG10-OWNED-OUT-PARAMETER-MEMBER-SLOT)`, made on the
old long page. The conflict was resolved by keeping this branch's page and
rerunning `python3 scripts/land.py unpublished --write`, which picked r50 up
from its gate record: 44 landings, ending `r50
(CG10-OWNED-OUT-PARAMETER-MEMBER-SLOT)`. `docs/reports/INDEX.md` auto-merged
and was regenerated.

Two items are left to STATUS-COMPACT's landing (r51), per that report's §6.
STATUS's *Latest validated gate* section still names the r49 gate, and its
checkpoint is of `f77e97ae6`. Folding those in is that landing's job, and
compaction is out of scope here. STATUS-REGROWTH-BY-PREPEND is registered by
STATUS-COMPACT's amendments, so this card's retirement applies only after r51.
`apply_amendments.py … --check` on this tree refuses it ("no registered
follow-up has this id"). With STATUS-COMPACT's amendments applied first in the
working tree, the check exits 0 (retire 1, register 2). The simulation was then
reverted.

## 6. Gate

Recorded after the final merge; see below.

## 7. Ledger (`integration-amendments.json`, schema `/2`)

| id | operation | what |
|---|---|---|
| `STATUS-REGROWTH-BY-PREPEND` | retire | Registered by STATUS-COMPACT, so the retirement applies once r51 (STATUS-COMPACT) has landed. All three proposals are implemented. |
| `LANDING-GATE-RECORD-UNSTRUCTURED` | register | The landing identity lives only in free-text notes (§3 table). Discharge: a structured `landing` object in the gate record. |
| `UNPUBLISHED-LANDINGS-HAND-LIST-OMISSIONS` | register | r8, r9, ag-gaps and cg25-thread-key were never listed by hand. Discharge: confirm any publication, or let the generated sentence stand. |

`status_edits` is empty. The only STATUS change is the regenerated sentence,
which is committed in the card.
