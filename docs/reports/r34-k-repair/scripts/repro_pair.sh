#!/usr/bin/env bash
# repro_pair.sh LABEL=BINARY=TIPDIR...: run_repro.sh for each, in turn.
here=/media/Data/GitHub/quirin-lang/.claude/worktrees/agent-a86a9bde776af1664
for spec in "$@"; do
  IFS='=' read -r label binary tip <<<"$spec"
  bash "$here/private/repair/run_repro.sh" "$binary" "$tip" "$here/private/repair/repro-$label" ${MONKEY:+monkey}
  echo "$label repro exit $?"
done
