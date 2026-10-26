#!/bin/bash
# One game, bake or heavy build on this Mac at a time. Usage:
#   mac_lock.sh <who> <command...>   waits for the lock, runs the command holding it, releases it.
# A perf run takes it the same way; nothing else runs while it is held.
LOCK=$HOME/.cb-mac-lock
who=$1; shift
until mkdir "$LOCK" 2>/dev/null; do
  if [ -f "$LOCK/pid" ] && ! kill -0 "$(cat "$LOCK/pid")" 2>/dev/null; then rm -rf "$LOCK"; continue; fi
  sleep 10
done
echo $$ > "$LOCK/pid"; echo "$who $(date +%T) $*" > "$LOCK/who"
trap 'rm -rf "$LOCK"' EXIT
"$@"
