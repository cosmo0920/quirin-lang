# FB-AUDIT-01-COVERAGE — source/build and corpus census

Completed bounded measurement on 2026-09-26. This does not close the broader
semantic system coverage cards or claim a fresh Quirin audit of these files.

## Result

Eight selected system files all have compile records in the retained Clang
build: **15 records**, one backlog plugin object plus static/shared variants of
seven core objects. All 15 preprocess successfully (exit 0). All eight tracked
file hashes match between current checkout `864c3d97f3589515285a19396ed8dfc85cda08ae`
and build source `66910c10a4d7eafa810b84229cf2dcf7b0f26f97`. These equalities apply
to those files only: header/build configuration and whole-revision identity
remain different. No fresh configured build of 864 was created.

Only **one of eight** appears in the frozen 185-unit diagnostics baseline:
`flb_input_chunk.c`, with **64 functions: 9 Safe, 55 Unknown**, unit Unknown.
The seven others are absent, not seven Unknown units. This 64-function generic
projection is not the separate 66-function source-contract inventory or the
96-function combined chunk lifecycle denominator.

| File | Compile records | Historical generic audit |
|---|---:|---|
| plugins/in_storage_backlog/sb.c | 1 | absent |
| src/flb_input_chunk.c | 2 | 64 functions, unit Unknown |
| src/flb_storage.c | 2 | absent |
| src/flb_task.c | 2 | absent |
| src/flb_engine.c | 2 | absent |
| src/flb_engine_dispatch.c | 2 | absent |
| src/flb_scheduler.c | 2 | absent |
| src/flb_output.c | 2 | absent |

The baseline is W45-RETURN-EXPRESSION-FAMILIES integration at the declared Quirin
base, not whatever another agent later publishes on main. The scope gap is the
main result: improving verdicts within that existing corpus does not measure
most of the execution path this audit is investigating. Availability in a
compile database is not presence in an audit selection.

## Reproduce

From this Quirin branch, choose a fresh output directory:

```sh
python3 docs/reports/fb-system-audit-cards/census/measure.py \
  --source /home/cosmo/Documents/GitHub/fluent-bit \
  --database /home/cosmo/Documents/GitHub/qcore-c/build/cycle19/fluent-bit-clang-production/compile_commands.json \
  --build-source /home/cosmo/Documents/GitHub/qcore-c/build/cycle19/fluent-bit-clang-source \
  --archive docs/reports/w45-return-expression-families/integration/artifacts/diagnostics.json.gz \
  --out private/fb-system-audit-cards/census-rerun
```

The measured run exited 0; `private/fb-system-audit-cards/census.log` and
`census.exit` preserve it. `census-run/` contains per-record preprocessing text,
stderr, exits and evidence. Published `evidence.json` retains exact unmodified
compile entries, command identities, output digests and source/archive hashes.
The compiler is the recorded `/usr/bin/clang`; preprocessing strips only
compile/output switches and adds `-E -P`. Preprocessing success does not assert
successful type checking, linking, native execution or safety.

## Follow-on briefs

1. Select these eight paths in an explicit current-build audit profile, retaining
   static/shared variants or documenting a justified deduplication. Bind a fresh
   build to 864 before calling its results current. Preserve missing/unsupported
   files in the denominator. This is an analyzer/configuration implementation
   handoff, not an action performed by this investigation.
2. Start lifetime fault injection at retry request creation and singleplex enqueue
   (card 02); start overload/progress work at backlog chunk loading (card 03).
   Source hashes, exact symbols and control conditions live in those reports.
3. For a full FB-SYSTEM-CENSUS completion, derive function/callback inventories and
   synchronization edges from each selected build; this card measured file/build
   and archived generic-audit coverage only. No full callgraph was constructed.
