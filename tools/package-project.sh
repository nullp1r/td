#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  tools/package-project.sh [OUTPUT.tar.gz]

Create a snapshot of the current Git working tree containing:
  - every tracked file that still exists
  - every untracked file not ignored by Git

Ignored files, .git/, and deleted tracked files are excluded.

By default the archive is written next to the repository:
  ../<repo>-snapshot-<UTC timestamp>.tar.gz
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
output="${1:-"$root/../${repo}-snapshot-${stamp}.tar.gz"}"

case "$output" in
  /*) ;;
  *) output="$PWD/$output" ;;
esac

mkdir -p "$(dirname "$output")"

manifest="$(mktemp)"
trap 'rm -f "$manifest"' EXIT

count=0
while IFS= read -r -d '' path; do
  # A tracked file may be deleted in the working tree. Snapshot what actually
  # exists now, including symlinks.
  if [[ -e "$root/$path" || -L "$root/$path" ]]; then
    printf '%s\0' "$path" >>"$manifest"
    ((count += 1))
  fi
done < <(
  git -C "$root" ls-files \
    --cached \
    --others \
    --exclude-standard \
    -z
)

if (( count == 0 )); then
  echo "error: no files selected for archive" >&2
  exit 1
fi

# GNU tar's NUL-delimited file list safely handles spaces, newlines, leading
# dashes, and other unusual path names. --no-recursion prevents a gitlink or
# other directory entry from accidentally pulling in files Git did not select.
tar \
  --create \
  --gzip \
  --file "$output" \
  --directory "$root" \
  --null \
  --verbatim-files-from \
  --no-recursion \
  --files-from "$manifest"

printf 'Created: %s\n' "$output"
printf 'Files:   %d\n' "$count"
if command -v du >/dev/null 2>&1; then
  printf 'Size:    %s\n' "$(du -h "$output" | cut -f1)"
fi
if command -v sha256sum >/dev/null 2>&1; then
  printf 'SHA256:  %s\n' "$(sha256sum "$output" | cut -d' ' -f1)"
fi
