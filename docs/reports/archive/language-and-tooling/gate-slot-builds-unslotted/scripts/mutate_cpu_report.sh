#!/bin/sh
# SPDX-License-Identifier: Apache-2.0
#
# Mutation check of the bracketed CPU assertions (card GATE-SLOT-BUILDS-UNSLOTTED).
# Copies scripts/ into a fresh scratch directory three times, mutates
# sample_delta's CPU figure in two copies (reported as 0; reported cumulative
# instead of over the window), and runs the two bracketed tests against each.
#
#   mutate_cpu_report.sh <fresh scratch directory>
set -u
S=$1
W=$(cd "$(dirname "$0")/../../../.." && pwd)
mkdir "$S" || exit 2
mkdir -p "$S/zero" "$S/cumulative" "$S/none"
for m in zero cumulative none; do cp -r "$W/scripts" "$S/$m/"; done
python3 - "$S" <<'EOF'
import sys
from pathlib import Path
root = Path(sys.argv[1])
old = '"cpu_seconds": round(after["cpu_seconds"] - before["cpu_seconds"], 2),'
for name, new in (("zero", '"cpu_seconds": 0.0,'),
                  ("cumulative", '"cpu_seconds": round(after["cpu_seconds"], 2),')):
    path = root / name / "scripts" / "gate_slot.py"
    text = path.read_text()
    assert text.count(old) == 1
    path.write_text(text.replace(old, new))
EOF
for m in none zero cumulative; do
  echo "== $m"
  (cd "$S/$m/scripts/tests" && timeout 600 python3 -m unittest \
    test_gate_slot_starvation.Drain.test_a_drain_gives_up_reopens_admission_releases_and_names_the_holder \
    test_gate_slot_starvation.Status.test_holders_a_draining_suite_and_a_waiter 2>&1 | grep -E '^(FAIL|ERROR|OK|Ran|AssertionError)')
done
