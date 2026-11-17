#!/bin/sh
# The opt-in current run over Fluent Bit 046accf6, as `prepare --run` invokes
# the user's run (--monkey-linked, the prepared Monkey database accounted
# for), with the contract selection moved to the side tree's generated user
# load path: the profile manifest's modules plus quirin-provider-openssl
# (docs/reports/openssl-provider-plugin/evidence/opt_in_providers.py).
#
#   sh opt-in-current.sh SIDE_TREE OUT QUIRIN_AUDIT
W=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-a95aa6ed324c823fb
TREE=$(cd "$1" && pwd)
OUT=$2
BIN=$3
P=$W/private/fb-046accf6
mkdir -p "$OUT"
cd "$TREE" || exit 2
python3 "$W/docs/reports/openssl-provider-plugin/evidence/opt_in_providers.py" "$TREE" > "$OUT/providers.args"
set --
while IFS= read -r line; do set -- "$@" "$line"; done < "$OUT/providers.args"
export SOURCE_DATE_EPOCH=1767225600
export QCORE_LIBRARY_VERSION_COMPILE_DATABASE="$P/fluent-bit-clang-production/compile_commands.json"
date -u +%FT%TZ > "$OUT/started"
sh /home/cosmo/Documents/GitHub/quirin-lang/private/toolchain/env.sh \
  "$BIN" current \
  --checkout "$P/fluent-bit-clang-source" \
  --profile-directory "$TREE/contract/profiles/fluent-bit" \
  --compile-database "$P/fluent-bit-clang-production/compile_commands.json" \
  --monkey-database "$P/monkey/monkey-build/compile_commands.json" \
  --monkey-source-report "$OUT/monkey-source.json" \
  --monkey-linked \
  --load-path "$TREE/providers/user-crates-load-path.json" \
  "$@" \
  --report "$OUT/resource-diagnostics.json" \
  --jobs 12 > "$OUT/console.log" 2> "$OUT/stderr.log"
echo $? > "$OUT/exit"
date -u +%FT%TZ > "$OUT/finished"
