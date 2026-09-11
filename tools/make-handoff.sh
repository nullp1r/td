#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  tools/make-handoff.sh [OUTPUT.tar.gz]

Create one chat handoff bundle containing:
  project.tar.gz   Git-aware working-tree snapshot
  diagnostics.txt  fresh tools/collect-diagnostics.sh output
  manifest.txt     branch/HEAD/status/checksum metadata

The bundle is created even when diagnostics contain failing checks. This makes
it suitable both for normal session bootstrap and for failure investigation.

By default:
  ../<repo>-handoff-<UTC timestamp>.tar.gz
EOF
}

case "${1:-}" in
  -h|--help)
    usage
    exit 0
    ;;
esac

root="$(git rev-parse --show-toplevel)"
repo="$(basename "$root")"
stamp="$(date -u +%Y%m%dT%H%M%SZ)"
output="${1:-"$root/../${repo}-handoff-${stamp}.tar.gz"}"

case "$output" in
  /*) ;;
  *) output="$PWD/$output" ;;
esac

[[ -x "$root/tools/package-project.sh" ]] ||
  { echo "error: tools/package-project.sh is missing or not executable" >&2; exit 2; }
[[ -f "$root/tools/collect-diagnostics.sh" ]] ||
  { echo "error: tools/collect-diagnostics.sh is missing" >&2; exit 2; }

mkdir -p "$(dirname "$output")"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

diagnostics="$tmp/diagnostics.txt"
project="$tmp/project.tar.gz"
manifest="$tmp/manifest.txt"

echo "Collecting diagnostics..."
diag_status=0
set +e
(
  cd "$root"
  bash tools/collect-diagnostics.sh
) >"$diagnostics" 2>&1
diag_status=$?
set -e

failures="$(
  sed -n 's/^Recorded failing\/missing checks: \([0-9][0-9]*\)$/\1/p' \
    "$diagnostics" | tail -n 1
)"
if [[ -z "$failures" ]]; then
  if (( diag_status == 0 )); then
    failures=0
  else
    failures=1
  fi
fi

echo "Packaging working tree..."
"$root/tools/package-project.sh" "$project" >/dev/null

if command -v sha256sum >/dev/null 2>&1; then
  project_sha="$(sha256sum "$project" | cut -d' ' -f1)"
elif command -v shasum >/dev/null 2>&1; then
  project_sha="$(shasum -a 256 "$project" | awk '{print $1}')"
else
  project_sha="unavailable"
fi

{
  echo "format=rustwater-handoff-v1"
  echo "created_utc=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "repository=$repo"
  echo "branch=$(git -C "$root" symbolic-ref --short -q HEAD || echo DETACHED)"
  echo "head=$(git -C "$root" rev-parse HEAD)"
  echo "diagnostics_exit_status=$diag_status"
  echo "diagnostic_failures=$failures"
  echo "project_sha256=$project_sha"
  echo
  echo "[git-status]"
  git -C "$root" status --short --branch
} >"$manifest"

tar \
  --create \
  --gzip \
  --file "$output" \
  --directory "$tmp" \
  project.tar.gz diagnostics.txt manifest.txt

echo
echo "Created:             $output"
echo "Diagnostic failures: $failures"
echo "Project SHA256:      $project_sha"
if command -v du >/dev/null 2>&1; then
  echo "Bundle size:         $(du -h "$output" | cut -f1)"
fi
if command -v sha256sum >/dev/null 2>&1; then
  echo "Bundle SHA256:       $(sha256sum "$output" | cut -d' ' -f1)"
fi

echo
echo "Attach this single handoff archive to the next chat."
