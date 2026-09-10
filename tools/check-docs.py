#!/usr/bin/env python3
"""Validate the durable Rustwater documentation with only the standard library."""

from __future__ import annotations

import re
import sys
from pathlib import Path
from urllib.parse import unquote

ROOT = Path(__file__).resolve().parents[1]
DOCS = ROOT / "game" / "docs"
INDEX = DOCS / "README.md"
REQUIRED = {
    "README.md",
    "vision.md",
    "current-state.md",
    "decisions.md",
    "open-questions.md",
    "world.md",
    "narrative/characters-and-relationships.md",
    "narrative/mara-reed.md",
    "narrative/character-spec-template.md",
    "art/visual-direction.md",
    "art/generative-media.md",
    "ux/interaction-surfaces.md",
    "telegram/platform.md",
    "telegram/tdx.md",
    "architecture/overview.md",
    "architecture/persistence.md",
    "development/workflow.md",
    "development/diagnostics.md",
    "development/playtesting.md",
}
LINK = re.compile(r"(?<!!)\[[^\]]*\]\(([^)]+)\)")


def markdown_target(path: Path, raw: str) -> Path | None:
    target = raw.strip()
    if not target or target.startswith(("http://", "https://", "mailto:", "#")):
        return None
    target = target.split("#", 1)[0].split("?", 1)[0]
    if not target:
        return None
    return (path.parent / unquote(target)).resolve()


def main() -> int:
    errors: list[str] = []
    if not DOCS.is_dir():
        print(f"missing documentation root: {DOCS.relative_to(ROOT)}", file=sys.stderr)
        return 1

    files = sorted(DOCS.rglob("*.md"))
    present = {path.relative_to(DOCS).as_posix() for path in files}
    for missing in sorted(REQUIRED - present):
        errors.append(f"missing required doc: game/docs/{missing}")

    if not INDEX.is_file():
        errors.append("missing documentation index: game/docs/README.md")
        index_text = ""
    else:
        index_text = INDEX.read_text(encoding="utf-8")

    for path in files:
        text = path.read_text(encoding="utf-8")
        rel = path.relative_to(ROOT).as_posix()
        first_nonempty = next((line for line in text.splitlines() if line.strip()), "")
        if not first_nonempty.startswith("# "):
            errors.append(f"{rel}: first non-empty line must be an H1 title")

        for match in LINK.finditer(text):
            resolved = markdown_target(path, match.group(1))
            if resolved is None:
                continue
            try:
                resolved.relative_to(ROOT.resolve())
            except ValueError:
                errors.append(f"{rel}: link escapes project root: {match.group(1)}")
                continue
            if not resolved.exists():
                errors.append(f"{rel}: broken link: {match.group(1)}")

    # Every durable document must be reachable directly from the package map. This
    # keeps the knowledge base browsable without depending on search or old context.
    indexed: set[str] = {"README.md"}
    for match in LINK.finditer(index_text):
        resolved = markdown_target(INDEX, match.group(1))
        if resolved is None or resolved.suffix.lower() != ".md":
            continue
        try:
            indexed.add(resolved.relative_to(DOCS.resolve()).as_posix())
        except ValueError:
            pass
    for orphan in sorted(present - indexed):
        errors.append(f"game/docs/{orphan}: not linked from game/docs/README.md")

    if errors:
        print("documentation check failed:", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        return 1

    print(f"OK game/docs: {len(files)} Markdown files, titled, indexed, internal links resolved")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
