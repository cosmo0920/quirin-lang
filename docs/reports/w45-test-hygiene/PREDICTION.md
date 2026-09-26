# W45-TEST-HYGIENE prediction

Frozen before implementation, at base `37e1b82d`.

1. **No analysis moves.** The card changes test discovery only, so no corpus
   or slice figure can move; the arbiter is not run (brief).
2. **Counts.** Every existing test keeps its name and its pinned expectation.
   The Rust and Python totals grow only by the helper's own self-tests.
   No pin in the census counts untracked files on a clean checkout, so none
   changes.
3. **Before, with decoys.** With the `a-untracked` or `a-ignored` decoys of
   `scripts/decoys.py` placed, these tests fail at base: `std_socket_inet`
   (the 2026-09-26 incident), `examples_suite`'s family and index checks,
   `examples_cli`'s page census, `ct_corpus`'s fixture byte-identity check,
   both `proof_leg_digests` cross-checks, `composition_bounds`' vacuity
   tripwire, the gateway's `census_sink_inventory` (feature build),
   conformance `packages`' template census and `test_retired_scripts`. With
   the contract-root decoys (`b-*`), `qcore_contract_corpus`'s count pins,
   the owning provider crate's `index_is_exactly_the_contract_tree` and
   `test_contract_corpus` fail.
4. **After, with decoys.** Every converted test passes with every `a-*` decoy
   placed. With `b-*` decoys the converted test-side walks ignore them, but
   tests that go through the production contract load path still refuse
   the unlisted module: that loader reads the directory by design and is not
   test discovery, so those failures stay and are recorded as a residual.
5. **Fail-closed.** Outside a git checkout the helper panics with a message
   naming `git ls-files`; it never walks the filesystem instead. A file added
   to the index and not committed is seen.
