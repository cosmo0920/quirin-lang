# Resume: W45-FB-ENGINE-PROGRESS

Read `README.md`, `census/census.json` (summary first) and
`native/evidence.json`. This card is measurement and native observation only.
It makes no analyzer, contract, provider or Lean change, and no verdict moves.

Before relying on anything, re-check:

* the base, `37e1b82d`;
* the Fluent Bit pin: `git -C /home/cosmo/Documents/GitHub/fluent-bit
  rev-parse HEAD` must print `046accf6aded7906f66a1baf0202da349e521736`.

## Private tree

`/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w45-fb-engine-progress/`:

* `bin/quirin-audit-37e1b82d`, the audit binary (sha256 `d7377944…`);
* `attempt-1/`:
  * `fluent-bit-clang-source/` and `fluent-bit-clang-production/` from
    `prepare`;
  * `native-bin/fluent-bit`, the Debug binary (sha256 `7e535c9e…`); its
    build tree `native-debug/` was deleted after the runs, so the run
    scripts need `native-build.sh` again (they read `native-debug/bin/`);
  * `run/`, the fixtures;
  * `native-logs/`, the full logs;
  * `slice-37e1b82d/` and `slice-37e1b82d-final/`, the slice runs (the
    census uses the second);
  * `census/`, the private inventory;
* `prepare-attempt-1*.{log,exit}`.

Intermediate builds may be deleted once the card is integrated. The published
hashes identify them.

## Re-run, from the repository root

1. Prepare:

   ```sh
   scripts/prepare-attempt.sh ATTEMPT BINARY
   ```

   Use a fresh ATTEMPT. Re-running it with a suffix, for example
   `ATTEMPT BINARY 046accf6… -recheck`, is the clean check after a native
   build.
2. Build the native binary and the fixture:

   ```sh
   scripts/native-build.sh ATTEMPT
   scripts/make-fixture.sh ATTEMPT
   ```

3. Run the backlog experiments, each with a fresh run name:

   ```sh
   scripts/run-native.sh ATTEMPT NAME {c1-valid-control,e1-persistent-error,e2-oneshot-fstat,c2-oneshot-open}.gdb SECONDS
   ```

   Then the retry experiments:

   ```sh
   scripts/run-native-retry.sh ATTEMPT NAME {c3-retry-control,e3-retry-setup-error}.gdb SECONDS
   ```

   Then bind the runs:

   ```sh
   scripts/bind_native.py ATTEMPT_DIR --out native/evidence.json --excerpts native/evidence
   ```

   Its run table names the canonical runs.
4. Run the slice, then regenerate the census:

   ```sh
   scripts/run-current.sh ATTEMPT RUN BINARY --selection contract/profiles/fluent-bit/system-selection.json --jobs 8
   scripts/regen-census.sh ATTEMPT RUN
   ```

## Rules for the runs

* Never run native builds or runs while the standing gate runs.
* Kill only captured PIDs. Every run has a `timeout -k` watchdog, and its PID
  is saved in `native-logs/NAME.pid`.
* The worktree guard refuses `git` commands aimed at `private/`. Clone and
  status checks go through `quirin-audit prepare`.

## Open

See `followups.json`. The implementation card is
`W45-FB-ENGINE-PROGRESS-MEASURE`.
