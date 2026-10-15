#!/usr/bin/env bash
# census.sh -- LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER: preprocess librdkafka's C
# units at both audited revisions with LIBRDKAFKA-PLUGIN-PREMISE-UNSTATED's
# preprocess.py (each unit with its own recorded command), then run this
# card's member census (member_reach.py, which builds reach.py's graph) into
# the report's artifacts. Each step's output and exit go to files under
# private/lppr/; nothing is piped.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
earlier="$repo/docs/reports/librdkafka-plugin-premise/scripts"
out="$repo/private/lppr"
mkdir -p "$out"
cd "$repo" || exit 2
for tag in 66910c10 046accf6; do
  if [ ! -f "$out/pp-$tag/exits.json" ]; then
    python3 "$earlier/preprocess.py" "$out/pp-$tag" "$tag" > "$out/pp-$tag.out" 2>&1
    echo $? > "$out/pp-$tag.exit"
  fi
  echo "preprocess $tag $(cat "$out/pp-$tag.exit")"
done
python3 "$here/member_reach.py" "$out" "$repo/docs/reports/librdkafka-plugin-premise-remainder/artifacts/member-census.json" > "$out/member-reach.out" 2>&1
echo $? > "$out/member-reach.exit"
echo "member-reach $(cat "$out/member-reach.exit")"
