#!/bin/sh
# Reruns the four censuses of OUT-SLOT-CALL-WRITE-ALIASING into census/.
# The reports and compile databases are the main checkout's, read-only.
set -e
HERE=$(dirname "$0")
C=/media/Data/GitHub/quirin-lang/build/cycle19/clang
B=/media/Data/GitHub/quirin-lang/private/baselines/7b726587bdf6a8213f451fa1c78297bb5e476ef7
P=/media/Data/GitHub/quirin-lang/private
# The parent cards' predicted Unknown -> Safe rows, read as Safe.
export CENSUS_ALSO_SAFE=rd_kafka_message_timestamp,rd_kafka_toppar_get_avail,cio_utils_read_file,cio_file_content_copy,cio_memfs_content_copy
python3 "$HERE/ast_census.py" $C/046accf6aded7906f66a1baf0202da349e521736/fluent-bit-clang-source/reports/resource-diagnostics.json \
  current "$HERE/current.json" $C/046accf6aded7906f66a1baf0202da349e521736/fluent-bit-clang-production/compile_commands.json
python3 "$HERE/ast_census.py" $B/eight-home/corpus/corpus/resource-diagnostics.json \
  corpus "$HERE/corpus.json" $C/66910c10a4d7eafa810b84229cf2dcf7b0f26f97/fluent-bit-clang-production/compile_commands.json \
  $C/66910c10a4d7eafa810b84229cf2dcf7b0f26f97/monkey/monkey-build/compile_commands.json
python3 "$HERE/ast_census.py" $B/fluent-bit-system/system/resource-diagnostics.json \
  system "$HERE/system.json" $P/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-production/compile_commands.json
python3 "$HERE/ast_census.py" $B/fluent-bit-tail-pause/slice/resource-diagnostics.json \
  slice "$HERE/slice.json" $P/w47-tail/attempt-1/prep/fluent-bit-clang-production/compile_commands.json
