#!/usr/bin/env bash
# gate.sh RUN -- full standing gate into one fresh worktree-private directory.
# The coordinator grants the heavy-run slot before this script starts.
set -u
run="${1:?fresh run name}"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/w48-pointer-return/gate-$run"
if [ -e "$out" ]; then
  echo "gate directory already exists: $out" >&2
  exit 2
fi
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
git rev-parse HEAD > "$out/head"
step() {
  local label="$1"
  shift
  "$@" > "$out/$label.log" 2>&1
  echo "$?" > "$out/$label.exit"
}
step 00-submodule git submodule update --init --recursive
step 01-tests python3 scripts/run_test_binaries.py --log-dir "$out/test-binaries"
step 02-fmt cargo fmt --all -- --check
step 03-clippy cargo clippy --all-targets -- -D warnings
step 04-probe cargo run -p qcore-compiler --example reevaluation_probe
step 05-followup cargo run -p qcore-compiler --example reevaluation_followup
step 06-lean python3 scripts/check_lean_audits.py
step 07-matrix python3 docs/reports/remaining-d02/build_matrix.py .
step 08-python python3 -m unittest discover -s scripts/tests
step 09-index python3 scripts/build_docs_index.py --check
step 10-clang clang --version
python3 - "$out" <<'PY'
from pathlib import Path
import sys

root = Path(sys.argv[1])
results = {p.stem: int(p.read_text().strip()) for p in sorted(root.glob('*.exit'))}
(root / 'summary.txt').write_text(''.join(f'{name} {status}\n' for name, status in results.items()))
print((root / 'summary.txt').read_text(), end='')
sys.exit(0 if all(status == 0 for status in results.values()) else 1)
PY
