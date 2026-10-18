#!/usr/bin/env bash
set -u
repo="$(cd "$(dirname "$0")/../../../.." && pwd)"
out="$repo/private/w48-bottom-up-batch2/gate-${1:?fresh run name}"
[ ! -e "$out" ] || exit 2
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4
export CARGO_TARGET_DIR=/home/cosmo/Documents/GitHub/qcore-c/private/w48-bottom-up-batch2-target
git rev-parse HEAD > "$out/head"
step() {
  local label="$1"
  shift
  "$@" > "$out/$label.log" 2>&1
  printf '%s\n' "$?" > "$out/$label.exit"
}
step 00-submodule git submodule update --init --recursive
step 01-tests python3 scripts/run_test_binaries.py --log-dir "$out/test-binaries"
step 02-fmt cargo fmt --all -- --check
step 03-clippy cargo clippy --all-targets -- -D warnings
step 04-probe cargo run -p qcore-compiler --example reevaluation_probe
step 05-followup cargo run -p qcore-compiler --example reevaluation_followup
step 06-lean python3 scripts/check_lean_audits.py
step 07-python python3 -m unittest discover -s scripts/tests
step 08-index python3 scripts/build_docs_index.py --check
step 09-clang clang --version
python3 - "$out" <<'PY'
from pathlib import Path
import sys
root = Path(sys.argv[1])
results = {p.stem: int(p.read_text()) for p in sorted(root.glob('*.exit'))}
(root/'summary.txt').write_text(''.join(f'{k} {v}\n' for k,v in results.items()))
print((root/'summary.txt').read_text(),end='')
sys.exit(0 if all(v==0 for v in results.values()) else 1)
PY
