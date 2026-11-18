#!/usr/bin/env python3
"""Fails when a relative link or image in the Markdown files points at something that does not exist, or at a heading
that is not there. Run it before a release: python3 tools/check-docs.py"""
import re, sys
from pathlib import Path

root = Path(__file__).resolve().parent.parent
files = [root / n for n in ("README.md", "CONTRIBUTING.md", "SECURITY.md", "CHANGELOG.md", "CODE_OF_CONDUCT.md")] + sorted((root / "docs").glob("*.md"))
link = re.compile(r"!?\[[^\]]*\]\(([^)\s]+)\)|<img[^>]+src=\"([^\"]+)\"")

def slug(heading: str) -> str:
    text = re.sub(r"[`*_]", "", heading.strip().lower())
    text = re.sub(r"[^\w\s-]", "", text)
    return re.sub(r"\s+", "-", text)

def anchors(path: Path) -> set[str]:
    return {slug(m.group(1)) for m in re.finditer(r"^#{1,6}\s+(.*)$", path.read_text(), re.M)}

problems = []
for file in files:
    text = file.read_text()
    for m in link.finditer(text):
        target = (m.group(1) or m.group(2)).strip()
        if re.match(r"^(https?:|mailto:)", target):
            continue
        path_part, _, fragment = target.partition("#")
        dest = (file.parent / path_part).resolve() if path_part else file
        if not dest.exists():
            problems.append(f"{file.relative_to(root)}: `{target}` does not exist")
        elif fragment and dest.suffix == ".md" and fragment not in anchors(dest):
            problems.append(f"{file.relative_to(root)}: `{target}` has no heading `{fragment}`")
print("\n".join(problems) if problems else f"ok: {len(files)} files, every relative link resolves")
sys.exit(1 if problems else 0)
