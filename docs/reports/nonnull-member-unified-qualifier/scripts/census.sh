#!/usr/bin/env bash
# census.sh [BASE] -- NONNULL-MEMBER-UNIFIED-QUALIFIER: reproduce
# LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER's member census (its reachability rule,
# member_reach.py, unchanged but for the view it reads) on this card's base,
# before any contract changes: the preprocessed librdkafka units at 66910c10
# and 046accf6 (LIBRDKAFKA-PLUGIN-PREMISE-UNSTATED's preprocess.py) and the
# base's null_deref.qv (`git show BASE:...`, the old spellings the census
# parses). The preprocessing runs under the shared heavy lock. Outputs under
# private/nmuq/; the census JSON is copied into the report's artifacts.
# Every exit goes to a file; nothing is piped.
set -u
base="${1:-3f8f1a617}"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/nmuq"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
earlier="$repo/docs/reports/librdkafka-plugin-premise/scripts"
remainder="$repo/docs/reports/librdkafka-plugin-premise-remainder/scripts"
mkdir -p "$out/census"
cd "$repo" || exit 2
git show "$base:providers/qcore-provider-librdkafka/contracts/librdkafka/null_deref.qv" > "$out/census/null_deref-base.qv"
echo $? > "$out/census/base-view.exit"
# A copy of member_reach.py whose only change is the view it reads.
sed -e "s#^NULL_VIEW = .*#NULL_VIEW = pathlib.Path(\"$out/census/null_deref-base.qv\")#" \
    -e "s#^REPO = .*#REPO = pathlib.Path(\"$repo\")#" \
    "$remainder/member_reach.py" > "$out/census/member_reach.py"
for tag in 66910c10 046accf6; do
  flock "$lock" python3 "$earlier/preprocess.py" "$out/pp-$tag" "$tag" > "$out/pp-$tag.out" 2>&1
  echo $? > "$out/pp-$tag.exit"
  echo "preprocess $tag $(cat "$out/pp-$tag.exit")"
done
python3 "$out/census/member_reach.py" "$out" "$out/census/member-census.json" > "$out/census/member-reach.out" 2>&1
echo $? > "$out/census/member-reach.exit"
echo "member-reach $(cat "$out/census/member-reach.exit")"
cp "$out/census/member-census.json" "$here/../artifacts/member-census-reproduced.json"
