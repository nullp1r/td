#!/usr/bin/env python3
"""Pack, unpack, and verify project handoffs using only the Python standard library."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
import zipfile
from datetime import datetime, timezone
from pathlib import Path, PurePosixPath

ROOT = Path(__file__).resolve().parent.parent
MANIFEST = "_handoff_manifest.json"
DEFAULT_EXCLUDED_DIRS = {".git", "target", "__pycache__"}
DEFAULT_EXCLUDED_FILES = {".DS_Store"}
MUTABLE_TREE_PATHS = {"diagnostics.txt", "diagnostics-sandbox.txt"}
MUTABLE_TREE_PREFIXES = (".tdx-session/", "game/game.sqlite3")
SESSION_PARTS = {".tdx-session"}
SECRET_NAMES = {".env", ".env.local", ".env.production", ".env.development"}


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def git_head() -> str | None:
    if not (ROOT / ".git").is_dir() or shutil.which("git") is None:
        return None
    result = subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True, capture_output=True)
    return result.stdout.strip() if result.returncode == 0 else None


def should_include(path: Path, *, include_session: bool, include_secrets: bool, output: Path | None) -> bool:
    relative = path.relative_to(ROOT)
    if relative.as_posix() == MANIFEST:
        return False
    if any(part in DEFAULT_EXCLUDED_DIRS for part in relative.parts):
        return False
    if not include_session and any(part in SESSION_PARTS for part in relative.parts):
        return False
    if not include_secrets and path.name in SECRET_NAMES:
        return False
    if path.name in DEFAULT_EXCLUDED_FILES:
        return False
    if output is not None:
        try:
            if path.resolve() == output.resolve():
                return False
        except FileNotFoundError:
            pass
    return path.is_file()


def pack(args: argparse.Namespace) -> int:
    output = Path(args.output).expanduser().resolve() if args.output else ROOT.parent / f"{ROOT.name}-handoff.zip"
    files = sorted(
        path
        for path in ROOT.rglob("*")
        if should_include(path, include_session=args.include_session, include_secrets=args.include_secrets, output=output)
    )
    manifest_files = []
    for path in files:
        relative = path.relative_to(ROOT).as_posix()
        manifest_files.append({"path": relative, "size": path.stat().st_size, "sha256": sha256_file(path)})

    manifest = {
        "format": 1,
        "created_at": datetime.now(timezone.utc).isoformat(),
        "project_root": ROOT.name,
        "git_head": git_head(),
        "include_session": args.include_session,
        "include_secrets": args.include_secrets,
        "files": manifest_files,
    }

    output.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(output, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=6) as archive:
        for path in files:
            archive.write(path, path.relative_to(ROOT).as_posix())
        archive.writestr(MANIFEST, json.dumps(manifest, indent=2, sort_keys=True) + "\n")

    print(output)
    print(f"files={len(files)} sha256={sha256_file(output)}")
    if not args.include_session:
        print("note: .tdx-session excluded; pass --include-session only when session state is intentionally needed")
    if not args.include_secrets:
        print("note: common .env secret files excluded; pass --include-secrets only when intentional")
    return 0


def safe_target(root: Path, member: str) -> Path:
    pure = PurePosixPath(member)
    if pure.is_absolute() or ".." in pure.parts:
        raise ValueError(f"unsafe archive path: {member}")
    target = (root / Path(*pure.parts)).resolve()
    if root.resolve() not in (target, *target.parents):
        raise ValueError(f"archive path escapes destination: {member}")
    return target


def unpack(args: argparse.Namespace) -> int:
    archive_path = Path(args.archive).expanduser().resolve()
    destination = Path(args.destination).expanduser().resolve()
    destination.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(archive_path) as archive:
        for info in archive.infolist():
            target = safe_target(destination, info.filename)
            if info.is_dir():
                target.mkdir(parents=True, exist_ok=True)
                continue
            target.parent.mkdir(parents=True, exist_ok=True)
            with archive.open(info) as source, target.open("wb") as sink:
                shutil.copyfileobj(source, sink)
            mode = (info.external_attr >> 16) & 0o777
            if mode:
                os.chmod(target, mode)
    print(destination)
    return 0


def verify(args: argparse.Namespace) -> int:
    archive_path = Path(args.archive).expanduser().resolve()
    with zipfile.ZipFile(archive_path) as archive:
        try:
            manifest = json.loads(archive.read(MANIFEST))
        except KeyError:
            print(f"missing {MANIFEST}", file=sys.stderr)
            return 2
        expected = {item["path"]: item for item in manifest["files"]}
        actual_names = {name for name in archive.namelist() if name != MANIFEST and not name.endswith("/")}
        if actual_names != set(expected):
            print("archive member set differs from manifest", file=sys.stderr)
            print(f"missing={sorted(set(expected) - actual_names)}", file=sys.stderr)
            print(f"extra={sorted(actual_names - set(expected))}", file=sys.stderr)
            return 1
        for name, item in expected.items():
            data = archive.read(name)
            digest = hashlib.sha256(data).hexdigest()
            if len(data) != item["size"] or digest != item["sha256"]:
                print(f"mismatch: {name}", file=sys.stderr)
                return 1
    print(f"OK {archive_path} sha256={sha256_file(archive_path)} files={len(expected)}")
    return 0



def tree_path_is_mutable(path: str) -> bool:
    return path in MUTABLE_TREE_PATHS or path.startswith(MUTABLE_TREE_PREFIXES)


def verify_tree(_: argparse.Namespace) -> int:
    manifest_path = ROOT / MANIFEST
    try:
        manifest = json.loads(manifest_path.read_text())
    except FileNotFoundError:
        print(f"missing {manifest_path}; unpack a handoff archive before verifying the tree", file=sys.stderr)
        return 2

    expected = {item["path"]: item for item in manifest["files"] if not tree_path_is_mutable(item["path"])}
    actual = {
        path.relative_to(ROOT).as_posix()
        for path in ROOT.rglob("*")
        if should_include(
            path,
            include_session=manifest.get("include_session", False),
            include_secrets=manifest.get("include_secrets", False),
            output=None,
        )
        and not tree_path_is_mutable(path.relative_to(ROOT).as_posix())
    }

    mismatches = [f"missing: {relative}" for relative in sorted(set(expected) - actual)]
    mismatches.extend(f"extra: {relative}" for relative in sorted(actual - set(expected)))
    for relative in sorted(set(expected) & actual):
        item = expected[relative]
        path = ROOT / relative
        if path.stat().st_size != item["size"] or sha256_file(path) != item["sha256"]:
            mismatches.append(f"mismatch: {relative}")

    if mismatches:
        print("tree differs from embedded handoff manifest:", file=sys.stderr)
        for mismatch in mismatches:
            print(mismatch, file=sys.stderr)
        return 1

    print(f"OK tree matches {MANIFEST} for {len(expected)} immutable files")
    return 0


def selftest(_: argparse.Namespace) -> int:
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        try:
            safe_target(root, "a/b.txt")
            try:
                safe_target(root, "../escape")
            except ValueError:
                pass
            else:
                raise AssertionError("zip-slip guard failed")
        except Exception as exc:
            print(f"selftest failed: {exc}", file=sys.stderr)
            return 1
    print("OK")
    return 0


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(description=__doc__)
    sub = result.add_subparsers(dest="command", required=True)

    pack_parser = sub.add_parser("pack", help="create a hashed project handoff ZIP")
    pack_parser.add_argument("output", nargs="?", help="output ZIP (default: ../<project>-handoff.zip)")
    pack_parser.add_argument("--include-session", action="store_true", help="include .tdx-session TDLib runtime/auth state")
    pack_parser.add_argument("--include-secrets", action="store_true", help="include common .env secret files")
    pack_parser.set_defaults(func=pack)

    unpack_parser = sub.add_parser("unpack", help="safely extract a handoff ZIP")
    unpack_parser.add_argument("archive")
    unpack_parser.add_argument("destination")
    unpack_parser.set_defaults(func=unpack)

    verify_parser = sub.add_parser("verify", help="verify a handoff ZIP against its embedded manifest")
    verify_parser.add_argument("archive")
    verify_parser.set_defaults(func=verify)

    tree_parser = sub.add_parser("verify-tree", help="verify extracted immutable files against the embedded manifest")
    tree_parser.set_defaults(func=verify_tree)

    test_parser = sub.add_parser("selftest", help="run local handoff-tool sanity checks")
    test_parser.set_defaults(func=selftest)
    return result


def main() -> int:
    args = parser().parse_args()
    return args.func(args)


if __name__ == "__main__":
    raise SystemExit(main())
