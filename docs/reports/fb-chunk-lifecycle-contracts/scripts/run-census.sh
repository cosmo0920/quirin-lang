#!/bin/sh
# run-census.sh NAME REVISION OUT_DIR [BASE_CENSUS] -- census.py over
# private/fclc/run-NAME-{system,current} (side tree private/fclc/side-NAME),
# writing OUT_DIR/census.json and OUT_DIR/CENSUS.md.
W=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-a273dcc45e6a8fab7
R=$W/docs/reports/fb-chunk-lifecycle-contracts
F=$W/private/fclc
set -- "$1" "$2" "$3" "${4:-}"
EXTRA=
[ -n "$4" ] && EXTRA="--base-census $4"
mkdir -p "$3"
# shellcheck disable=SC2086
python3 "$R/scripts/census.py" --label "$1" --revision "$2" \
  --tree "$F/side-$1" \
  --checkout /home/cosmo/Documents/GitHub/quirin-lang/private/fb-046accf6/fluent-bit-clang-source \
  --system "$F/run-$1-system/resource-diagnostics.json" \
  --current "$F/run-$1-current/resource-diagnostics.json" \
  --classification "$R/classification.json" \
  --out-json "$3/census.json" --out-md "$3/CENSUS.md" $EXTRA
