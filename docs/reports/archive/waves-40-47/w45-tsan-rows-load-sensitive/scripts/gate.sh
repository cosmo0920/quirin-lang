#!/bin/sh
# The standing gate (docs/agents/validation.md), with the Rust suite run three
# times through scripts/run_test_binaries.py as the card's brief asks. Every
# step's output and exit go to OUT; no step is piped.
# usage: scripts/gate.sh OUT_DIR   (from the repository root)
OUT="$1"
mkdir -p "$OUT"
export CARGO_INCREMENTAL=0
git submodule update --init --recursive > "$OUT/01-submodule.log" 2>&1; echo $? > "$OUT/01-submodule.exit"
for i in 1 2 3; do
    python3 scripts/run_test_binaries.py --log-dir "$OUT/test-binaries-$i" > "$OUT/02-test-$i.log" 2>&1
    echo $? > "$OUT/02-test-$i.exit"
done
cargo fmt --all -- --check > "$OUT/03-fmt.log" 2>&1; echo $? > "$OUT/03-fmt.exit"
cargo clippy --all-targets -- -D warnings > "$OUT/04-clippy.log" 2>&1; echo $? > "$OUT/04-clippy.exit"
cargo run -p qcore-compiler --example reevaluation_probe > "$OUT/05-probe.log" 2>&1; echo $? > "$OUT/05-probe.exit"
cargo run -p qcore-compiler --example reevaluation_followup > "$OUT/06-followup.log" 2>&1; echo $? > "$OUT/06-followup.exit"
python3 scripts/check_lean_audits.py > "$OUT/07-lean.log" 2>&1; echo $? > "$OUT/07-lean.exit"
python3 -m unittest discover -s scripts/tests > "$OUT/08-python.log" 2>&1; echo $? > "$OUT/08-python.exit"
python3 scripts/build_docs_index.py --check > "$OUT/09-docs-index.log" 2>&1; echo $? > "$OUT/09-docs-index.exit"
clang --version > "$OUT/clang-version.txt" 2>&1; echo $? > "$OUT/clang-version.exit"
cargo clean --profile dev > "$OUT/10-clean.log" 2>&1; echo $? > "$OUT/10-clean.exit"
echo done > "$OUT/done"
