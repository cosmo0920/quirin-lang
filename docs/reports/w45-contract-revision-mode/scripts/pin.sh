#!/usr/bin/env bash
# Run `quirin-audit pin` with the worktree's release binary, capturing its
# output and exit under private/rm-pin/NAME.{log,err,exit}.
# usage: pin.sh NAME [pin ARGUMENTS...]   (from the repository root)
set -u
name="$1"; shift
mkdir -p private/rm-pin
target/release/quirin-audit pin "$@" > "private/rm-pin/$name.log" 2> "private/rm-pin/$name.err"
echo $? > "private/rm-pin/$name.exit"
cat "private/rm-pin/$name.exit"
