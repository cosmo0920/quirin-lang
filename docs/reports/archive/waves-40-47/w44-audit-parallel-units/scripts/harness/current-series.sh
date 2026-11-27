#!/bin/sh
root=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-af93c329e85457c34
"$root/private/current.sh" base 1 "$root/private/bin/base-audit"; echo "base exit=$?"
"$root/private/current.sh" j16 16; echo "j16 exit=$?"
"$root/private/current.sh" j1 1; echo "j1 exit=$?"
"$root/private/current.sh" j8 8; echo "j8 exit=$?"
"$root/private/current.sh" j4 4; echo "j4 exit=$?"
echo "current series done"
