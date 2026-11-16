#!/bin/sh
# callers.sh HOST PLUGIN-DIR OUT: the libcurl callers of Fluent Bit 046accf6
# (this card's system preparation) through `quirin-audit current`, once
# without and once with the plugin's user load path; prepare, check and
# install run first through HOST's plugin route. Every step's exit goes to
# OUT/<step>.exit; nothing is piped.
W=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-a717a6f321d7ff5d9
H=$1
PLUGINS=$2
OUT=$3
P=$W/private/w48-libcurl
S=$P/prep-system
SEL=$W/docs/reports/w48-libcurl-contract-plugin/artifacts/libcurl-callers-selection.json
cd "$W" || exit 2
mkdir -p "$OUT" || exit 2
export SOURCE_DATE_EPOCH=1767225600
run() {
  step=$1
  shift
  "$@" > "$OUT/$step.log" 2>&1
  echo $? > "$OUT/$step.exit"
}
run prepare "$H" --plugin-path "$PLUGINS" --output "$OUT/plugin" \
  --report "$OUT/plugin/prepare.report.json" \
  libcurl prepare --tarball "$P/dl/curl-8.18.0.tar.gz"
run check "$H" --plugin-path "$PLUGINS" --output "$OUT/plugin" \
  --report "$OUT/plugin/check.report.json" libcurl check
run install "$H" --plugin-path "$PLUGINS" --output "$OUT/plugin" \
  --report "$OUT/plugin/install.report.json" libcurl install --repository "$W"
run without "$H" current --checkout "$S/fluent-bit-clang-source" \
  --profile-directory "$W/contract/profiles/fluent-bit" \
  --compile-database "$S/fluent-bit-clang-production/compile_commands.json" \
  --selection "$SEL" --jobs 2 --report "$OUT/without/resource-diagnostics.json"
run with "$H" current --checkout "$S/fluent-bit-clang-source" \
  --profile-directory "$W/contract/profiles/fluent-bit" \
  --compile-database "$S/fluent-bit-clang-production/compile_commands.json" \
  --load-path "$OUT/plugin/user/user-load-path.json" \
  --provider quirin-provider-libcurl \
  --selection "$SEL" --jobs 2 --report "$OUT/with/resource-diagnostics.json"
for step in prepare check install without with; do
  printf '%s %s\n' "$step" "$(cat "$OUT/$step.exit")"
done
