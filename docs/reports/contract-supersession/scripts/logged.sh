#!/bin/sh
# Run one command with its output and exit in LOG_DIR/NAME.{log,exit}; print
# the exit and the log's tail. Usage: logged.sh LOG_DIR NAME COMMAND [ARG...]
set -u
logs=$1
name=$2
shift 2
mkdir -p "$logs"
"$@" >"$logs/$name.log" 2>&1
code=$?
echo $code >"$logs/$name.exit"
echo "$name exit $code"
tail -n 40 "$logs/$name.log"
exit $code
