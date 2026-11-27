# Kafka callback contract placement

At the user's direction, the three Fluent Bit callbacks registered with
librdkafka now belong to
`providers/qcore-provider-librdkafka/contracts/librdkafka/logging.qv`:
`cb_kafka_msg`, `cb_kafka_logger`, and `oauthbearer_token_refresh_cb`.
They were removed from the Fluent Bit provider's logging module. The moved
card section is byte-identical, including its declared-assumption standing,
seven hypotheses, lifecycle clauses, decomposition and premise flow. Its three
binding-manifest comments move with it. The three reviewed lock entries move
to the Kafka provider with only their module paths changed; contract hashes,
reviewed revisions, declaration hashes and body hashes are preserved.

`providers/contracts.py sync` updates the package inventory and load path.
The default resource selection includes the new Kafka module, preserving the
callbacks for existing consumers. Its selected module count increases from
64 to 65 with unchanged declaration counts. The corpus pin intentionally increases from 136 to 137 modules; the three
per-function pins remain 22, 23 and 22 clauses (67 total). This is an ownership
and placement change, with no inference or Lean semantics change. No new
source review, native observation or verdict measurement is claimed. Module-count
pins in the loader, provider-selection and gateway tests increase accordingly;
the structured corpus table follows the new module order. The historical
symbolic-adoption test reconstructs its original logging layout from the two
current files, retaining its exact archived byte and metadata comparisons. The
historical `op-app-rk-callbacks/scripts/write_contracts.py` remains frozen as
evidence of its original checkpoint; it is not a current regeneration step.

Byte and pin preservation evidence: `private/rk-callback-logging-20261022/parity.json`.
Validation commands, exits and logs: `private/rk-callback-logging-20261022/commands.json`.
The checkout began with unrelated staged audit changes; validation ran on
the combined working tree. Concurrent audit work advanced HEAD and continued
editing other source files during this task. These results do not certify a
final, quiescent whole-tree snapshot. This checkout is not fresh, so the fresh-checkout
submodule initialization step is not due.

Validation ran on the combined working tree. `python3 providers/contracts.py
sync` and `python3 providers/contracts.py check` exited 0; the resource locks
now list 170 Kafka and 85 Fluent Bit pinned declarations. `cargo fmt --all --
--check`, `cargo clippy --all-targets -- -D warnings`, and both `cargo run -p
qcore-compiler --example reevaluation_probe` and `--example
reevaluation_followup` exited 0. Formatting and Clippy were checked again
successfully after the final test updates. `python3 scripts/check_lean_audits.py`
exited 0 with 362 audits; `python3 -m unittest discover -s scripts/tests` exited
0 with 374 tests.

The first `python3 scripts/run_test_binaries.py` exited 101: 537 units,
550 summaries, 8,376 passed, 116 failed, 15 ignored. It exposed module-count
and ordering pins, the adoption test's historical layout dependency,
LeakSanitizer's sandbox/ptrace restriction, and unavailable user/network
namespaces inside the sandbox. Its unit logs and summary are under
`private/test-binaries/20261022T003350Z-43/`.

After the pin and historical replay updates, the run outside the sandbox
exited 101: 537 units, 550 summaries, 8,491 passed, 1 failed, 15 ignored
(`private/test-binaries/20261022T005641Z-1526899/`). Its only failure was the
corpus table's ordering: the callback entries had been inserted before a
repeated comment heading in another module. Their position was corrected
explicitly after `librdkafka/kafka_mem.qv`. The final
`cargo test -p qcore-compiler --test qcore_contract_corpus` exited 0 with
5 passed and no failures (`private/rk-callback-logging-20261022/corpus-final.log`).
The exhaustive adoption replay passed in the full retry: 169 declarations,
1,179 introductions and 169 open conjunction flows; no archived bytes changed.

Provider formatting and Clippy exited 0:
`cargo fmt --manifest-path providers/Cargo.toml --all -- --check` and
`cargo clippy --manifest-path providers/Cargo.toml --workspace --all-targets
--locked --offline -- -D warnings`. `python3 providers/package.py build
--output /home/cosmo/Documents/GitHub/qcore-c/private/rk-callback-logging-20261022/packages`
exited 0, and `python3 providers/package.py verify` exited 0 for each of
`qcore.chunkio`, `qcore.cmetrics`, `qcore.ctraces`, `qcore.libevent` and
`qcore.librdkafka` in that directory.

The first `cargo test --manifest-path providers/Cargo.toml --workspace --locked
--offline` exited 101 at the packages binary: 36 passed, 1 failed, 0 ignored
across 18 summaries, an incomplete provider run. The confined-profile test
could not create user/network namespaces in the sandbox. The provider retry
outside the sandbox exited 0: 133 passed, 0 failed, 1 ignored
across 43 summaries, including the confined-profile package check. Its
command, exit and log are recorded in
`private/rk-callback-logging-20261022/final-checks/commands.json`.

Full command arguments and exits are retained in the main `commands.json`,
`retry/commands.json`, `corpus-final.json` and `final-checks/commands.json`
under `private/rk-callback-logging-20261022/`; each points to its own log.

Starting HEAD: `93eb17441c141f6b3272d2ed874d1b83db55bc37`; observed HEAD at
validation completion: `5adfcfe22a04ecc569c57a37b5235187ef0f70d0`. These validation runs preceded the placement commit.
