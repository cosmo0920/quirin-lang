#!/bin/sh
# Card LIBCURL-PROVIDER-PLUGIN: the opt-in `current` run over librdkafka's
# libcurl callers (W48-LIBCURL-CONTRACT-PLUGIN's selection: rdhttp.c and
# rdkafka_sasl_oauthbearer_oidc.c) at Fluent Bit 046accf6, with the
# profile's default modules as a provider selection: once on the shipped
# load path, then on the generated user load path with this crate added,
# under three probe configurations (rule 53):
#   with-selected  QCORE_LIBRARY_VERSION_COMPILE_DATABASE = the selected
#                  units' prepared compile commands
#   with-default   no database: Clang's default search path
#   with-full      the whole prepared compile database
# Every step's exit goes to OUT/<step>.exit; nothing is piped.
#
#   sh callers.sh QUIRIN_AUDIT PREPARED SELECTED_DATABASE OUT
#
# PREPARED is the `quirin-audit prepare` output (fluent-bit-clang-source,
# fluent-bit-clang-production).
QA=$1
S=$2
SELECTED=$3
OUT=$4
cd "$(dirname "$0")/../../../.." || exit 2
W=$(pwd)
SEL=$W/docs/reports/w48-libcurl-contract-plugin/artifacts/libcurl-callers-selection.json
DB=$S/fluent-bit-clang-production/compile_commands.json
mkdir -p "$OUT" || exit 2
export SOURCE_DATE_EPOCH=1767225600
run() {
  step=$1
  shift
  "$@" > "$OUT/$step.log" 2>&1
  echo $? > "$OUT/$step.exit"
}
python3 "$W/docs/reports/w48-libcurl-contract-plugin/scripts/provider_args.py" "$W" \
  > "$OUT/provider-args.txt"
set --
while IFS= read -r word; do
  set -- "$@" "$word"
done < "$OUT/provider-args.txt"
common() {
  echo current --checkout "$S/fluent-bit-clang-source" \
    --profile-directory "$W/contract/profiles/fluent-bit" --compile-database "$DB" \
    --selection "$SEL" --jobs 2
}
unset QCORE_LIBRARY_VERSION_COMPILE_DATABASE
# shellcheck disable=SC2046
run without "$QA" $(common) --load-path "$W/providers/contract-load-path.json" "$@" \
  --report "$OUT/without/resource-diagnostics.json"
export QCORE_LIBRARY_VERSION_COMPILE_DATABASE="$SELECTED"
# shellcheck disable=SC2046
run with-selected "$QA" $(common) \
  --load-path "$W/providers/user-crates-load-path.json" "$@" \
  --provider quirin-provider-libcurl --report "$OUT/with-selected/resource-diagnostics.json"
unset QCORE_LIBRARY_VERSION_COMPILE_DATABASE
# shellcheck disable=SC2046
run with-default "$QA" $(common) \
  --load-path "$W/providers/user-crates-load-path.json" "$@" \
  --provider quirin-provider-libcurl --report "$OUT/with-default/resource-diagnostics.json"
export QCORE_LIBRARY_VERSION_COMPILE_DATABASE="$DB"
# shellcheck disable=SC2046
run with-full "$QA" $(common) \
  --load-path "$W/providers/user-crates-load-path.json" "$@" \
  --provider quirin-provider-libcurl --report "$OUT/with-full/resource-diagnostics.json"
for step in without with-selected with-default with-full; do
  printf '%s %s\n' "$step" "$(cat "$OUT/$step.exit")"
done
