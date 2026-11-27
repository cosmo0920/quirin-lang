#!/bin/sh
# run-system.sh SIDE RUN -- the system slice (contract/profiles/fluent-bit/system-selection.json)
# over the prepared Fluent Bit 046accf6 inputs, from side-SIDE's own tree and release binary,
# with the opt-in user load path (as opt-in-current.sh) and --monkey-linked.
W=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-aeba8645a8f540281
P=/home/cosmo/Documents/GitHub/quirin-lang/private/fb-046accf6
ENV=/home/cosmo/Documents/GitHub/quirin-lang/private/toolchain/env.sh
TREE=$W/private/fcl/side-$1
OUT=$W/private/fcl/run-$2
BIN=$TREE/target/release/quirin-audit
mkdir -p "$OUT"
cd "$TREE" || exit 2
sh "$ENV" python3 providers/package.py checkers > "$OUT/checkers.log" 2>&1
echo $? > "$OUT/checkers.exit"
python3 docs/reports/openssl-provider-plugin/evidence/opt_in_providers.py "$TREE" > "$OUT/providers.args"
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
  --selection "$TREE/contract/profiles/fluent-bit/system-selection.json" \
  --load-path "$TREE/providers/user-crates-load-path.json" \
  "$@" \
  --report "$OUT/resource-diagnostics.json" \
  --jobs 12 > "$OUT/console.log" 2> "$OUT/stderr.log"
echo $? > "$OUT/exit"
date -u +%FT%TZ > "$OUT/finished"
