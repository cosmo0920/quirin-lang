# Resume W45-FB-SYSTEM-SELECTION

The card is complete at its recorded tip. Read README.md, `binding.json` and
`followups.json` first.

* **Verify the source first.** Before reusing anything, recheck that the user's
  checkout is at `864c3d97f3589515285a19396ed8dfc85cda08ae` and that the private
  clone is clean. Never build in the user's checkout or in `build/cycle19`.
* **A rerun is a new attempt.** Run `scripts/prepare-attempt.sh attempt-N` with a
  fresh name under
  `/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w45-fb-system-selection/`,
  then `scripts/run-current.sh` with
  `--selection contract/profiles/fluent-bit/system-selection.json`.
* **Measure into a fresh directory.** `measure.py` refuses an existing output
  directory. Do not overwrite `measurement/`.
* **Compare a new revision against this one.** A new revision needs its own
  attempt and a comparison with this denominator. Never borrow this binding.
* **What stays open.** The residuals are listed one by one in `followups.json`
  and registered together as `W45-FB-SYSTEM-SLICE-RESIDUALS`.
* **The next semantic card.** Section 5 of README.md gives the recommended
  first card and its slice. That card needs its own Rust and audited Lean, and
  must use this report's denominator as its baseline.

After the final measurement, the private clone, build and Monkey copy of
`attempt-1` (about 450 MB) were deleted. Their digests are in `binding.json`.
The run outputs are kept under `attempt-1/<run>/` (41 MB). They include
`system-wip`, an exploratory run with a pre-commit binary that no published
figure uses.
