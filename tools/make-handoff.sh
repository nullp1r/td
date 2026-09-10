#!/usr/bin/env bash
# Run diagnostics, then package the project even when diagnostics fail.
# Usage: tools/make-handoff.sh [output.zip] [handoff.py pack flags...]

set -u
root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
output="$root/../$(basename "$root")-handoff.zip"
if (( $# > 0 )) && [[ $1 != -* ]]; then
  output=$1
  shift
fi

"$root/tools/collect-diagnostics.sh" "$root/diagnostics.txt"
diagnostic_status=$?
python3 "$root/tools/handoff.py" pack "$output" "$@"
pack_status=$?

printf 'diagnostics_status=%d pack_status=%d\n' "$diagnostic_status" "$pack_status"
exit "$pack_status"
