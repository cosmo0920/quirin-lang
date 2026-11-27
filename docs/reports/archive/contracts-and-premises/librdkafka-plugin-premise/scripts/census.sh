#!/usr/bin/env bash
# census.sh -- LIBRDKAFKA-PLUGIN-PREMISE-UNSTATED: preprocess librdkafka's C
# units at both audited revisions (preprocess.py), then build the census
# (reach.py) into the report's artifacts. Each step's output and exit go to
# files under private/lpp/; nothing is piped.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/lpp"
mkdir -p "$out"
cd "$repo" || exit 2
for tag in 66910c10 046accf6; do
  python3 "$here/preprocess.py" "$out/pp-$tag" "$tag" > "$out/pp-$tag.out" 2>&1
  echo $? > "$out/pp-$tag.exit"
  echo "preprocess $tag $(cat "$out/pp-$tag.exit")"
done
python3 "$here/reach.py" "$out" "$repo/docs/reports/librdkafka-plugin-premise/artifacts/census.json" > "$out/reach.out" 2>&1
echo $? > "$out/reach.exit"
echo "reach $(cat "$out/reach.exit")"
