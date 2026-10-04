"""Validate the single-source skill layout without model calls or dependencies."""
from pathlib import Path
import re


def validate(root):
    sources = root / "skills"
    names = {p.name for p in sources.iterdir() if p.is_dir()}
    if not names:
        raise ValueError("No canonical skills found")
    for name in sorted(names):
        source = sources / name / "SKILL.md"
        if source.is_symlink() or not source.resolve(strict=True).is_relative_to(root.resolve()):
            raise ValueError(f"Canonical source must be a repository file: {source}")
        text = source.read_text(encoding="utf-8")
        header = re.match(r"\A---\n(.*?)\n---\n", text, re.DOTALL)
        if not header:
            raise ValueError(f"Missing skill frontmatter: {source}")
        if not re.search(rf"^name: {re.escape(name)}$", header[1], re.MULTILINE):
            raise ValueError(f"Skill name mismatch: {source}")
        if not re.search(r"^description: \S.+$", header[1], re.MULTILINE):
            raise ValueError(f"Missing description: {source}")
        for adapter in (".agents", ".claude"):
            entry = root / adapter / "skills" / name
            if (entry / "SKILL.md").resolve(strict=True) != source.resolve(strict=True):
                raise ValueError(f"Discovery entry is not the canonical source: {entry}")
        if not (root / ".agents" / "skills" / name / "SKILL.md").is_symlink():
            raise ValueError(f"Codex discovery must link to shared source: {name}")
        if not (root / ".claude" / "skills" / name).is_symlink():
            raise ValueError(f"Claude discovery must link to shared source: {name}")
    for adapter in (".agents", ".claude"):
        actual = {p.name for p in (root / adapter / "skills").iterdir()}
        if actual != names:
            raise ValueError(f"Unexpected or missing discovery entries: {adapter}")
    if "@AGENTS.md" not in (root / "CLAUDE.md").read_text().splitlines():
        raise ValueError("CLAUDE.md must import shared AGENTS.md")
    return len(names)


if __name__ == "__main__":
    root = Path(__file__).resolve().parents[2]
    print(f"Validated {validate(root)} shared skills and both discovery layouts.")
