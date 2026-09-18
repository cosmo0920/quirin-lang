#!/bin/sh
# Card W37-CORPUS-CONTRACTS-4: put the tree into its BEFORE state, run the
# audit, and put it back.
#
# BEFORE means: this card's six .qcore modules absent from disk and the manifest
# at its 33-minus-6 = 27-module form, with everything ELSE at the rebased tip -
# so the run measures the two wave-37 code arbiters beneath this card and
# nothing of this card's own.
#
# No `git stash` anywhere: the stash stack is shared across worktrees
# (CLAUDE.md). The six files are moved aside into the gitignored measure/
# directory and moved back; the manifest is taken from origin/main and restored
# from HEAD.
set -eu
WT=$(cd "$(dirname "$0")/../.." && pwd)
cd "$WT"
HELD="$WT/measure/w37/held"
MODULES="contract/qcore/mpack/writer.qcore
contract/qcore/cmetrics/metric_values.qcore
contract/qcore/cmetrics/mpack_utils.qcore
contract/qcore/cfl/time.qcore
contract/qcore/librdkafka/lz4_stream.qcore
contract/qcore/librdkafka/topic_partition.qcore"

mkdir -p "$HELD"
for m in $MODULES; do
    mkdir -p "$HELD/$(dirname "$m")"
    mv "$m" "$HELD/$m"
done
git checkout origin/main -- contract/cycle19-resource-contracts.json

echo "staged out: $(find contract/qcore -name '*.qcore' | wc -l) modules on disk"
python3 -c "import json;print('manifest selects', len(json.load(open('contract/cycle19-resource-contracts.json'))['modules']))"

set +e
sh "$WT/measure/w37/stage.sh" before2
code=$?
set -e

for m in $MODULES; do
    mv "$HELD/$m" "$m"
done
git checkout HEAD -- contract/cycle19-resource-contracts.json
rm -rf "$HELD"

echo "restored: $(find contract/qcore -name '*.qcore' | wc -l) modules on disk"
python3 -c "import json;print('manifest selects', len(json.load(open('contract/cycle19-resource-contracts.json'))['modules']))"
exit "$code"
