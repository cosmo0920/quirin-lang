#!/bin/sh
root=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-af93c329e85457c34
"$root/private/timed.sh" census-j1 --census --jobs 1; echo "census-j1 exit=$?"
"$root/private/timed.sh" census-j16 --census --jobs 16; echo "census-j16 exit=$?"
echo "census series done"
