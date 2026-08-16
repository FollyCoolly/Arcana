#!/usr/bin/env python3
"""Generate or verify Claude-compatible mirrors of canonical Arcana Skills."""

from __future__ import annotations

import argparse
import shutil
import sys
from pathlib import Path


REPOSITORY_ROOT = Path(__file__).resolve().parents[1]
PLUGIN_ROOT = REPOSITORY_ROOT / "plugins" / "arcana"
MIRROR_ROOT = REPOSITORY_ROOT / ".claude"


def collect_files(root: Path, *, omit_agents: bool = False) -> dict[Path, bytes]:
    if not root.is_dir():
        raise RuntimeError(f"source directory does not exist: {root}")
    files: dict[Path, bytes] = {}
    for path in sorted(root.rglob("*")):
        if not path.is_file():
            continue
        relative = path.relative_to(root)
        if omit_agents and "agents" in relative.parts:
            continue
        files[relative] = path.read_bytes()
    return files


def source_groups() -> list[tuple[Path, Path, dict[Path, bytes]]]:
    return [
        (
            PLUGIN_ROOT / "skills",
            MIRROR_ROOT / "skills",
            collect_files(PLUGIN_ROOT / "skills", omit_agents=True),
        ),
        (
            PLUGIN_ROOT / "fixtures",
            MIRROR_ROOT / "fixtures",
            collect_files(PLUGIN_ROOT / "fixtures"),
        ),
    ]


def ensure_safe_target(target: Path) -> None:
    resolved = target.resolve()
    mirror = MIRROR_ROOT.resolve()
    if resolved == mirror or mirror not in resolved.parents:
        raise RuntimeError(f"refusing to replace unsafe mirror target: {resolved}")


def generate() -> None:
    for source, target, files in source_groups():
        ensure_safe_target(target)
        if target.exists():
            shutil.rmtree(target)
        for relative, content in files.items():
            destination = target / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_bytes(content)
        print(f"generated {target.relative_to(REPOSITORY_ROOT)} from {source.relative_to(REPOSITORY_ROOT)}")


def check() -> bool:
    clean = True
    for source, target, expected in source_groups():
        actual = collect_files(target) if target.is_dir() else {}
        missing = sorted(expected.keys() - actual.keys())
        extra = sorted(actual.keys() - expected.keys())
        changed = sorted(
            relative
            for relative in expected.keys() & actual.keys()
            if expected[relative] != actual[relative]
        )
        if missing or extra or changed:
            clean = False
            print(
                f"mirror drift: {target.relative_to(REPOSITORY_ROOT)} "
                f"(source {source.relative_to(REPOSITORY_ROOT)})",
                file=sys.stderr,
            )
            for label, paths in [
                ("missing", missing),
                ("extra", extra),
                ("changed", changed),
            ]:
                for path in paths:
                    print(f"  {label}: {path.as_posix()}", file=sys.stderr)
    if clean:
        print("Arcana Agent Skill mirrors are up to date")
    return clean


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Generate or verify .claude mirrors from plugins/arcana"
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="report mirror drift without writing files",
    )
    args = parser.parse_args()
    try:
        if args.check:
            return 0 if check() else 1
        generate()
        return 0
    except (OSError, RuntimeError) as error:
        print(error, file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
