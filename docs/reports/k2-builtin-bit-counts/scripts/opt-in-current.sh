#!/bin/sh
# K2-BUILTIN-BIT-COUNTS (from FB-RETRY-OR6-ELEMENT-IDENTITY, provider script path
# moved to its archive): the opt-in current run over Fluent Bit
# 046accf6, exactly as OPENSSL-PROVIDER-REMAINING-CALLEES' evidence/opt-in/
# opt-in-current.sh takes it (as `prepare --run` invokes the user's run,
# --monkey-linked, the contract selection moved to the side tree's generated
# user load path: the profile manifest's modules plus quirin-provider-openssl,
# OPENSSL-PROVIDER-PLUGIN's opt_in_providers.py). The side tree is a
# `git archive` of the revision measured; its checker is installed by that
# tree's own `providers/package.py checkers` step first.
#
#   sh opt-in-current.sh SIDE_TREE OUT QUIRIN_AUDIT PREPARED
TREE=$(cd "$1" && pwd)
OUT=$2
BIN=$3
P=$4
ENV=/home/cosmo/Documents/GitHub/quirin-lang/private/toolchain/env.sh
mkdir -p "$OUT"
OUT=$(cd "$OUT" && pwd)
cd "$TREE" || exit 2
sh "$ENV" python3 providers/package.py checkers > "$OUT/checkers.log" 2>&1
echo $? > "$OUT/checkers.exit"
python3 docs/reports/archive/contracts-and-premises/openssl-provider-plugin/evidence/opt_in_providers.py "$TREE" > "$OUT/providers.args"
set --
while IFS= read -r line; do set -- "$@" "$line"; done < "$OUT/providers.args"
export SOURCE_DATE_EPOCH=1767225600
export QCORE_LIBRARY_VERSION_COMPILE_DATABASE="$P/fluent-bit-clang-production/compile_commands.json"
date -u +%FT%TZ > "$OUT/started"
sh "$ENV" \
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
