#!/bin/sh
# V07 probe runner. Usage: run_probes.sh QCOREC_BIN [PLUGIN_CONFIG]
# PLUGIN_CONFIG is an absolute qcorec provider-selection file (see
# `qcorec plugins enable PACKAGE --config FILE`). When omitted, the
# --plugins rows are skipped.
set -u
BIN="$1"
CFG="${2:-}"
DIR="$(cd "$(dirname "$0")" && pwd)"
run() {
  label="$1"; shift
  printf '\n===== %s\n$ qcorec %s\n' "$label" "$*"
  "$BIN" "$@"
  printf 'EXIT=%s\n' "$?"
}
for f in a01_requires a02_ensures a03_quant a05_referenced_uncalled \
         a06_never_referenced a07_empty_contract_control a11_library_mode \
         a12_contract_mode a13_extern_contract_control a14_verify_scope \
         b01_outer_resolution b02_binder_equals_outer b03_header_unresolved \
         b04_binder_escape b05_sibling_leak b06_binder_masks_unresolved \
         b07_thread_scope_escape b08_duplicate_binding b09_binder_used_in_body \
         c01_scope_without_marker c02_nested_worker c03_spawn_outer_scope \
         c04_unjoined_scope_exit c05_contract_on_worker \
         c06_contract_on_uncalled_in_task_module \
         c07_empty_contract_in_task_module c08_nested_scope_control \
         c09_guard_in_task_module; do
  run "$f" check "$DIR/$f.qr"
done
run "a04_second_file" check "$DIR/a04_first_main.qr" "$DIR/a04_second_bound.qr"
run "a15_task_second_file" check "$DIR/a15_task_second_file_main.qr" "$DIR/a15_task_second_file_bound.qr"
run "b10_cross_file_binder_leak" check "$DIR/b10_cross_file_first.qr" "$DIR/b10_cross_file_second.qr"
run "d01_file_boundary_attribution" check "$DIR/d01_boundary_first.qr" "$DIR/d01_boundary_second.qr"
run "d02_file_boundary_parse" check "$DIR/a15_task_second_file_main.qr" "$DIR/d02_boundary_record.qr"
run "a08_emit_route" emit -o /dev/null "$DIR/a01_requires.qr"
run "a09_empty_plugin_selection" check --plugins "$DIR/plugins_empty.json" "$DIR/a01_requires.qr"
run "a09b_empty_plugin_selection_task" check --plugins "$DIR/plugins_empty.json" "$DIR/c05_contract_on_worker.qr"
if [ -n "$CFG" ]; then
  run "a10_enabled_provider_sequential" check --plugins "$CFG" "$DIR/a01_requires.qr"
  run "a10b_enabled_provider_uncalled" check --plugins "$CFG" "$DIR/a06_never_referenced.qr"
  run "a10c_enabled_provider_task" check --plugins "$CFG" "$DIR/c05_contract_on_worker.qr"
  run "a10d_enabled_provider_emit" emit --plugins "$CFG" -o /dev/null "$DIR/a01_requires.qr"
  run "a10e_enabled_provider_verify_scope" check --plugins "$CFG" "$DIR/a14_verify_scope.qr"
fi
