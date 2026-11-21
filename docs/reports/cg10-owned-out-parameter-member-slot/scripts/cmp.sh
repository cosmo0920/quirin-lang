#!/bin/sh
O=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-a43d32eda6d150c67/private/oom/fb-corpus
N=/home/cosmo/Documents/GitHub/quirin-lang/private/fb-046accf6/fluent-bit-clang-source
for f in lib/cmetrics/src/cmt_mpack_utils.c lib/ctraces/src/ctr_mpack_utils.c lib/librdkafka-2.15.0/src/rdkafka_msgset_writer.c lib/librdkafka-2.15.0/src/rdkafka_lz4.c lib/cmetrics/src/cmt_decode_msgpack.c lib/ctraces/src/ctr_decode_msgpack.c; do
  if cmp -s "$O/$f" "$N/$f"; then echo "same $f"; else echo "DIFF $f"; fi
done
for fn in "int cmt_mpack_consume_string_tag" "int ctr_mpack_consume_string_or_nil_tag" "int ctr_mpack_consume_string_tag" "rd_kafka_resp_err_t rd_kafka_gzip_compress" "rd_kafka_resp_err_t rd_kafka_snappy_compress_slice" "rd_kafka_resp_err_t rd_kafka_lz4_compress("; do
  for f in lib/cmetrics/src/cmt_mpack_utils.c lib/ctraces/src/ctr_mpack_utils.c lib/librdkafka-2.15.0/src/rdkafka_msgset_writer.c lib/librdkafka-2.15.0/src/rdkafka_lz4.c; do
    a=$(awk -v s="$fn" 'index($0,s)==1{p=1} p{print} p&&/^}/{exit}' "$O/$f" | sha256sum)
    b=$(awk -v s="$fn" 'index($0,s)==1{p=1} p{print} p&&/^}/{exit}' "$N/$f" | sha256sum)
    if [ "$(awk -v s="$fn" 'index($0,s)==1{print 1; exit}' "$N/$f")" = 1 ]; then
      if [ "$a" = "$b" ]; then echo "body same: $fn ($f)"; else echo "body DIFF: $fn ($f)"; fi
    fi
  done
done
