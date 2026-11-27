#!/bin/sh
# The census as committed: baseline:497bdeb90 corpus/system/slice reports, the
# integrator's linked `current` at binary 89fa31d21 (read-only evidence), the
# prepared Fluent Bit 046accf6 checkout and its two compile databases, and the
# host's libssl-dev headers and libraries the prepared build used.
set -eu
M=/media/Data/GitHub/quirin-lang
B=$M/private/baselines/497bdeb906e1ad29a94f54079991f0b62bd701df
E=$M/private/current-linked-556460e31
cd "$(dirname "$0")/../../../.."
exec python3 docs/reports/openssl-provider-plugin/census/census.py \
  --route corpus=$B/eight-home/corpus/corpus/resource-diagnostics.json \
  --route system=$B/fluent-bit-system/system/resource-diagnostics.json \
  --route slice=$B/fluent-bit-tail-pause/slice/resource-diagnostics.json \
  --route current=$E/reports-89fa31d21/resource-diagnostics.json \
  --tree $E/run/fluent-bit-clang-source \
  --headers /usr/include \
  --library /usr/lib/x86_64-linux-gnu/libssl.so.3 \
  --library /usr/lib/x86_64-linux-gnu/libcrypto.so.3 \
  --compile-database $E/run/fluent-bit-clang-production/compile_commands.json \
  --compile-database $E/run/monkey/monkey-build/compile_commands.json \
  --clang /usr/lib/llvm-21/bin/clang \
  --out docs/reports/openssl-provider-plugin/census
