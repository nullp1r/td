#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  tools/apply-change.sh PATCH [--message MESSAGE] [--no-commit]

Apply one generated patch to a clean Git working tree, format the workspace,
collect the project's full diagnostics, and commit the resulting change even
when validation fails.

On validation success:
  commits with MESSAGE (or a filename-derived default)

On validation failure:
  commits as "WIP: MESSAGE (validation failed)"
  and exits non-zero after printing the diagnostics path

A patch that fails preflight/application is never committed because no trusted
source change was applied.

The working tree must be clean before use. Keep incoming .patch files outside
the repository (for example ~/Downloads or /tmp), or explicitly exclude local
transport artifacts with .git/info/exclude.
EOF
}

patch=""
message=""
commit=1

while (($#)); do
  case "$1" in
    -h|--help)
      usage
      exit 0
      ;;
    --message)
      [[ $# -ge 2 ]] || { echo "error: --message requires a value" >&2; exit 2; }
      message="$2"
      shift 2
      ;;
    --no-commit)
      commit=0
      shift
      ;;
    --*)
      echo "error: unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
    *)
      if [[ -n "$patch" ]]; then
        echo "error: only one patch may be supplied" >&2
        exit 2
      fi
      patch="$1"
      shift
      ;;
  esac
done

[[ -n "$patch" ]] || { usage >&2; exit 2; }
[[ -f "$patch" ]] || { echo "error: patch not found: $patch" >&2; exit 2; }

root="$(git rev-parse --show-toplevel)"
repo="$(basename "$root")"
patch="$(cd "$(dirname "$patch")" && pwd)/$(basename "$patch")"
patch_name="$(basename "$patch")"
patch_stem="${patch_name%.patch}"
stamp="$(date -u +%Y%m%dT%H%M%SZ)"
diagnostics="$root/../${repo}-diagnostics-${stamp}.txt"

if [[ -n "$(git -C "$root" status --porcelain=v1 --untracked-files=all)" ]]; then
  cat >&2 <<'EOF'
error: working tree is not clean.

apply-change.sh intentionally requires a clean baseline so cargo fmt, generated
files, and the final commit cannot accidentally absorb unrelated work.

Commit/stash the current work first. Keep transport .patch files outside the
repository, or add a local-only rule such as "*.patch" to .git/info/exclude.
EOF
  exit 2
fi

if (( commit )); then
  git -C "$root" config user.name >/dev/null ||
    { echo "error: git user.name is not configured" >&2; exit 2; }
  git -C "$root" config user.email >/dev/null ||
    { echo "error: git user.email is not configured" >&2; exit 2; }
fi

if [[ -z "$message" ]]; then
  message="apply ${patch_stem}"
fi

if command -v sha256sum >/dev/null 2>&1; then
  patch_sha="$(sha256sum "$patch" | cut -d' ' -f1)"
elif command -v shasum >/dev/null 2>&1; then
  patch_sha="$(shasum -a 256 "$patch" | awk '{print $1}')"
else
  patch_sha="unavailable"
fi

echo "Patch:       $patch"
echo "Patch SHA:   $patch_sha"
echo "Diagnostics: $diagnostics"

echo
echo "Preflight..."
git -C "$root" apply --stat "$patch"
git -C "$root" apply --check "$patch"

{
  echo "=== PATCH APPLICATION ==="
  echo "patch=$patch_name"
  echo "patch_sha256=$patch_sha"
  echo "message=$message"
  echo "started_utc=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo
} >"$diagnostics"

echo
echo "Applying patch..."
git -C "$root" apply "$patch"

fmt_status=0
echo "Formatting..."
{
  echo "=== CARGO FORMAT ==="
  echo '$ cargo fmt --all'
} >>"$diagnostics"
set +e
(
  cd "$root"
  cargo fmt --all
) >>"$diagnostics" 2>&1
fmt_status=$?
set -e
printf '\nformat_exit_status=%d\n\n' "$fmt_status" >>"$diagnostics"

diag_status=0
if [[ -f "$root/tools/collect-diagnostics.sh" ]]; then
  echo "Collecting full diagnostics..."
  set +e
  (
    cd "$root"
    bash tools/collect-diagnostics.sh
  ) >>"$diagnostics" 2>&1
  diag_status=$?
  set -e
else
  echo "warning: tools/collect-diagnostics.sh is missing" | tee -a "$diagnostics" >&2
  diag_status=1
fi

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
if (( fmt_status != 0 && failures == 0 )); then
  failures=1
fi

validation="PASS"
if (( failures != 0 )); then
  validation="FAIL"
fi

if (( commit )); then
  git -C "$root" add -A

  if git -C "$root" diff --cached --quiet; then
    echo "error: patch produced no staged changes" >&2
    exit 1
  fi

  subject="$message"
  if (( failures != 0 )); then
    subject="WIP: ${message} (validation failed)"
  fi

  body="$(
    cat <<EOF
Patch: $patch_name
Patch-SHA256: $patch_sha
Validation: $validation ($failures failing/missing checks)
Diagnostics: $(basename "$diagnostics")
EOF
  )"

  echo
  echo "Creating Git commit..."
  git -C "$root" commit -m "$subject" -m "$body"
fi

echo
echo "Validation:  $validation"
echo "Diagnostics: $diagnostics"
if (( commit )); then
  echo "Commit:      $(git -C "$root" rev-parse --short HEAD)"
fi

if (( failures != 0 )); then
  echo
  if (( commit )); then
    echo "Validation failed, but the applied state was preserved in a WIP commit."
  else
    echo "Validation failed; the applied state remains in the working tree."
  fi
  exit 1
fi
