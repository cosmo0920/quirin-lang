#!/bin/sh
# usage: dump.sh <checkout> <output-prefix>
# After `lake build QCore` in <checkout>/proof, writes <output-prefix>.audited
# (kind and name of every QCore./_private.QCore. constant, the AuditAll set)
# and <output-prefix>.all (every constant in the environment), both sorted.
here=$(dirname "$(readlink -f "$0")")
cd "$1/proof" || exit 2
DUMP_OUT="$2" lake env lean "$here/DumpDecls.lean"
echo "exit $?"
