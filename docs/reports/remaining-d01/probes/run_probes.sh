#!/bin/sh
# D01 attribution probes. Usage: run_probes.sh path/to/qcorec
# Every row rejects with exit 1 both before and after the repair; only the
# reported artifact, line, column and byte offsets change.
set -u
QCOREC="${1:?usage: run_probes.sh path/to/qcorec}"
HERE=$(dirname "$0")

run() {
    label="$1"
    shift
    printf '=== %s\n' "$label"
    "$QCOREC" check "$@"
    printf 'exit=%s\n\n' "$?"
}

run "d01 file boundary (expect d01_boundary_second.qcore:1:1)" \
    "$HERE/d01_boundary_first.qcore" "$HERE/d01_boundary_second.qcore"
run "P21 in-file span (expect 4:1)" "$HERE/P21_span_line4.qcore"
run "P22 pass agreement (expect Q010 and Q016 both at 4:1)" "$HERE/P22_span_dupaxis.qcore"
run "P28 parse offsets (expect 3:18, bytes 48..55)" "$HERE/P28_byte_offsets.qcore"
run "P25+P26 multi-file (expect P26_multifile_badaxis.qcore:2:1)" \
    "$HERE/P25_multifile_dimension.qcore" "$HERE/P26_multifile_badaxis.qcore"
run "P26+P25 reversed (expect the same file, not argument order)" \
    "$HERE/P26_multifile_badaxis.qcore" "$HERE/P25_multifile_dimension.qcore"
run "P25+P27 control (expect acceptance)" \
    "$HERE/P25_multifile_dimension.qcore" "$HERE/P27_multifile_goodaxis.qcore"
