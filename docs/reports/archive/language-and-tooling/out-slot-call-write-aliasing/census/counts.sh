#!/bin/sh
# The census table's counts (counts.py over each route's report and census).
HERE=$(dirname "$0")
C=/media/Data/GitHub/quirin-lang/build/cycle19/clang
B=/media/Data/GitHub/quirin-lang/private/baselines/7b726587bdf6a8213f451fa1c78297bb5e476ef7
export CENSUS_ALSO_SAFE=rd_kafka_message_timestamp,rd_kafka_toppar_get_avail,cio_utils_read_file,cio_file_content_copy,cio_memfs_content_copy
python3 "$HERE/counts.py" $B/eight-home/corpus/corpus/resource-diagnostics.json "$HERE/corpus.json"
python3 "$HERE/counts.py" $B/fluent-bit-system/system/resource-diagnostics.json "$HERE/system.json"
python3 "$HERE/counts.py" $B/fluent-bit-tail-pause/slice/resource-diagnostics.json "$HERE/slice.json"
python3 "$HERE/counts.py" $C/046accf6aded7906f66a1baf0202da349e521736/fluent-bit-clang-source/reports/resource-diagnostics.json "$HERE/current.json"
