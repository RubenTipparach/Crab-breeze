#!/usr/bin/env bash
#
# butler-push.sh <source> <target> [butler push options...]
#
# Pale Blue Dot's .github/scripts/butler-push.sh (itself adapted from godot-sandbox's). Extra arguments
# after the target (`--userversion ...`) are passed to `butler push` as they
# are.
#
# Wraps `butler push` with a retry for one specific, transient itch.io failure:
#
#   itch.io API error (400): /wharf/builds: channel <name>'s latest build is
#   still processing, try again later
#
# itch refuses a new build on a channel while the previous one is still being
# processed server-side, so two deploys landing close together make the second
# fail even though nothing is wrong with the build.
#
# Only that error is retried. Anything else (bad credentials, missing file, a
# malformed channel name) fails immediately, so a genuinely broken deploy still
# reports fast instead of sitting in a retry loop.

set -uo pipefail

SOURCE="${1:?usage: butler-push.sh <source> <target> [butler options...]}"
TARGET="${2:?usage: butler-push.sh <source> <target> [butler options...]}"
shift 2

MAX_ATTEMPTS="${BUTLER_PUSH_ATTEMPTS:-5}"
BACKOFF="${BUTLER_PUSH_BACKOFF:-30}"

for attempt in $(seq 1 "$MAX_ATTEMPTS"); do
	# Tee so the butler output still shows up in the job log as it happens.
	output="$(butler push "$SOURCE" "$TARGET" "$@" 2>&1 | tee /dev/stderr)"
	status="${PIPESTATUS[0]}"

	if [ "$status" -eq 0 ]; then
		exit 0
	fi

	case "$output" in
		*"still processing"*) ;;
		*)
			echo "butler push failed for a non-transient reason, not retrying." >&2
			exit "$status"
			;;
	esac

	if [ "$attempt" -eq "$MAX_ATTEMPTS" ]; then
		break
	fi

	delay=$((attempt * BACKOFF))
	echo "itch is still processing the previous build on ${TARGET}; retrying in ${delay}s (attempt ${attempt}/${MAX_ATTEMPTS})." >&2
	sleep "$delay"
done

echo "butler push to ${TARGET} still blocked after ${MAX_ATTEMPTS} attempts." >&2
exit 1
