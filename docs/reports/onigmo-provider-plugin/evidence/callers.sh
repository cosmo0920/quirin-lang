#!/bin/sh
# Card ONIGMO-PROVIDER-PLUGIN: the opt-in `current` run over Fluent Bit's
# Onigmo callers (src/flb_regex.c, src/flb_router.c; selection
# artifacts/onigmo-callers-selection.json) at Fluent Bit 046accf6, with the
# profile's default modules as a provider selection (the
# w48-libcurl-contract-plugin helper prints them): `without` on the shipped
# load path; each `with-*` side on the generated user load path with
# quirin-provider-onigmo added, under one probe configuration (rule 53):
#   with-selected  QCORE_LIBRARY_VERSION_COMPILE_DATABASE = the selected
#                  units' prepared compile commands
#   with-full      the whole prepared compile database
# Every step's exit goes to OUT/<step>.exit; nothing is piped. SIDES names the
# sides to run (default: all three).
#
#   sh callers.sh QUIRIN_AUDIT PREPARED SELECTED_DATABASE OUT [SIDES]
#
# PREPARED is the `quirin-audit prepare` output directory holding
# fluent-bit-clang-source and fluent-bit-clang-production.
QA=$1
S=$2
SELECTED=$3
OUT=$4
SIDES=${5:-without with-selected with-full}
cd "$(dirname "$0")/../../../.." || exit 2
W=$(pwd)
SEL=$W/docs/reports/onigmo-provider-plugin/artifacts/onigmo-callers-selection.json
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
for side in $SIDES; do
  case $side in
    without)
      unset QCORE_LIBRARY_VERSION_COMPILE_DATABASE
      # shellcheck disable=SC2046
      run without "$QA" $(common) --load-path "$W/providers/contract-load-path.json" "$@" \
        --report "$OUT/without/resource-diagnostics.json"
      ;;
    with-selected)
      export QCORE_LIBRARY_VERSION_COMPILE_DATABASE="$SELECTED"
      # shellcheck disable=SC2046
      run with-selected "$QA" $(common) \
        --load-path "$W/providers/user-crates-load-path.json" "$@" \
        --provider quirin-provider-onigmo --report "$OUT/with-selected/resource-diagnostics.json"
      ;;
    with-full)
      export QCORE_LIBRARY_VERSION_COMPILE_DATABASE="$DB"
      # shellcheck disable=SC2046
      run with-full "$QA" $(common) \
        --load-path "$W/providers/user-crates-load-path.json" "$@" \
        --provider quirin-provider-onigmo --report "$OUT/with-full/resource-diagnostics.json"
      ;;
    *)
      echo "unknown side $side" >&2
      exit 2
      ;;
  esac
done
for side in $SIDES; do
  printf '%s %s\n' "$side" "$(cat "$OUT/$side.exit")"
done
