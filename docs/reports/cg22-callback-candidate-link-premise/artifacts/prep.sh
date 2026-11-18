#!/bin/sh
# prep.sh: prepare the pinned-corpus inputs (66910c10) with the base binary,
# under private/corpus-inputs (c-gateway-wave-pin/, provider-packages-wave20/).
# Copied from docs/reports/cg22-callback-candidate-contracts/artifacts/prep.sh
# (base 974434463). `--link` also writes the quirin-link-resolution/1 artifact
# that link_premise.py reads as the link evidence.
W=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-a83f807263a0b65f2
B=$W/private/base-src/target/release/quirin-audit
E=/home/cosmo/Documents/GitHub/quirin-lang/private/toolchain/env.sh
O=$W/private/corpus-inputs/c-gateway-wave-pin
mkdir -p "$W/private/corpus-inputs" "$W/private/logs"
cd "$W" || exit 2
sh "$E" "$B" prepare clang-build \
  --fluent-bit /home/cosmo/Documents/GitHub/fluent-bit --revision 66910c10 \
  --output "$O" --clang /usr/lib/llvm-21/bin/clang \
  --clangxx /usr/lib/llvm-21/bin/clang++ --repository "$W/private/base-src" \
  --link --build-jobs 12 > "$W/private/logs/prep-corpus.log" 2>&1
e=$?
echo "clang-build exit $e" >> "$W/private/logs/prep-corpus.log"
[ $e -eq 0 ] || exit $e
mkdir -p "$O/fluent-bit-clang-source/build"
cp "$O/fluent-bit-clang-production/compile_commands.json" "$O/fluent-bit-clang-source/build/compile_commands.json"
sh "$E" "$B" prepare monkey --fluent-bit "$O/fluent-bit-clang-source" \
  --output "$O/monkey" --clang /usr/lib/llvm-21/bin/clang \
  --clangxx /usr/lib/llvm-21/bin/clang++ --repository "$W/private/base-src" \
  > "$W/private/logs/prep-monkey.log" 2>&1
e=$?
echo "monkey exit $e" >> "$W/private/logs/prep-monkey.log"
[ $e -eq 0 ] || exit $e
cd "$W/private/base-src" || exit 2
python3 providers/package.py build --output "$W/private/corpus-inputs/provider-packages-wave20" \
  > "$W/private/logs/prep-providers.log" 2>&1
e=$?
echo "providers exit $e" >> "$W/private/logs/prep-providers.log"
exit $e
