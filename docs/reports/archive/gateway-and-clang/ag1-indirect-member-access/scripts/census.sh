#!/usr/bin/env bash
# census.sh OUT -- AG1-INDIRECT-MEMBER-ACCESS step 1 on the user's current run:
# the AST premise census (classify.py, premise.py) over the analysed texts
# prep_units.py rebuilt (sha256 checked). Exits to files; nothing piped.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
report=/media/Data/GitHub/quirin-lang/build/cycle19/clang/046accf6aded7906f66a1baf0202da349e521736/fluent-bit-clang-source/reports/resource-diagnostics.json
out="$repo/private/ag1/$1"
mkdir -p "$out"
python3 "$here/classify.py" "$report" "$repo/private/ag1/units" "$out" > "$out/classify.log" 2>&1
echo $? > "$out/classify.exit"
