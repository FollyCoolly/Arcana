"""
Extract skill tree data from MakerSkillTree SVG files.

Reads SVG files from the MakerSkillTree project, decodes the embedded
base64/URL-encoded JSON data, and saves individual JSON files.

Usage:
    python scripts/extract_maker_trees.py [--source DIR] [--output DIR]
"""

import argparse
import base64
import json
import os
import re
import sys
import urllib.parse
from pathlib import Path


DEFAULT_SOURCE = r"D:\projects\MakerSkillTree"
DEFAULT_OUTPUT = os.path.join(os.path.dirname(__file__), "..", "docs", "tmp", "decoded_trees")

# SVGs to skip (templates, duplicates, non-English, peer reviews)
SKIP_PATTERNS = [
    "Template",
    "Makerspace",
    "Wiki Building",
    "-FR.svg",
    "_fr.json",
    "_FR.json",
    "Peer Review",
    "Mini Coding",
    "with laser cutter",
]


def find_svg_files(source_dir: str) -> list[Path]:
    """Find all SVG files in MakerSkillTree, filtering out templates and duplicates."""
    svgs = []
    for root, _dirs, files in os.walk(source_dir):
        for fname in files:
            if not fname.endswith(".svg"):
                continue
            full_path = Path(root) / fname
            rel = str(full_path.relative_to(source_dir))
            if any(pat in rel for pat in SKIP_PATTERNS):
                continue
            svgs.append(full_path)
    return sorted(svgs)


def decode_svg_json(svg_path: Path) -> dict | None:
    """Extract and decode the embedded JSON from an SVG file."""
    content = svg_path.read_text(encoding="utf-8")
    match = re.search(r"<json>(.*?)</json>", content)
    if not match:
        return None

    b64_data = match.group(1)
    try:
        decoded = base64.b64decode(b64_data).decode("utf-8")
        unquoted = urllib.parse.unquote(decoded)
        return json.loads(unquoted)
    except Exception as e:
        print(f"  WARNING: Failed to decode {svg_path.name}: {e}", file=sys.stderr)
        return None


def also_check_json_files(source_dir: str) -> list[tuple[Path, dict]]:
    """Check for pre-decoded JSON files (like 3D_Modelling.json)."""
    results = []
    for root, _dirs, files in os.walk(source_dir):
        for fname in files:
            if not fname.endswith(".json"):
                continue
            full_path = Path(root) / fname
            rel = str(full_path.relative_to(source_dir))
            if any(pat in rel for pat in SKIP_PATTERNS):
                continue
            # Skip schema/template files at root
            if Path(root) == Path(source_dir):
                continue
            try:
                data = json.loads(full_path.read_text(encoding="utf-8"))
                # Only include if it has Skills array (pre-decoded format)
                if "Skills" in data and isinstance(data["Skills"], list):
                    results.append((full_path, data))
            except (json.JSONDecodeError, UnicodeDecodeError):
                pass
    return results


def slugify(title: str) -> str:
    """Convert a tree title to a filename-safe slug."""
    s = title.lower().strip()
    s = re.sub(r"[^a-z0-9]+", "_", s)
    s = s.strip("_")
    return s


def main():
    parser = argparse.ArgumentParser(description="Extract MakerSkillTree SVG data")
    parser.add_argument("--source", default=DEFAULT_SOURCE, help="MakerSkillTree directory")
    parser.add_argument("--output", default=DEFAULT_OUTPUT, help="Output directory for decoded JSON")
    args = parser.parse_args()

    source_dir = args.source
    output_dir = args.output
    os.makedirs(output_dir, exist_ok=True)

    print(f"Source: {source_dir}")
    print(f"Output: {output_dir}")
    print()

    svg_files = find_svg_files(source_dir)
    print(f"Found {len(svg_files)} SVG files to process")

    results = []
    for svg_path in svg_files:
        data = decode_svg_json(svg_path)
        if data is None:
            print(f"  SKIP (no data): {svg_path.name}")
            continue

        title = data.get("title", "unknown")
        items = data.get("items", {})
        item_count = len(items)
        slug = slugify(title)

        out_file = Path(output_dir) / f"{slug}.json"
        out_file.write_text(json.dumps(data, indent=2, ensure_ascii=False), encoding="utf-8")

        # Determine row range
        if items:
            max_idx = max(int(k) for k in items.keys())
            max_row = max_idx // 7
        else:
            max_row = 0

        results.append({
            "slug": slug,
            "title": title,
            "source_svg": str(svg_path.relative_to(source_dir)),
            "item_count": item_count,
            "max_row": max_row,
        })
        print(f"  OK: {title} ({item_count} items, rows 0-{max_row}) -> {slug}.json")

    # Write summary
    summary = {
        "total_trees": len(results),
        "total_items": sum(r["item_count"] for r in results),
        "trees": results,
    }
    summary_path = Path(output_dir) / "_summary.json"
    summary_path.write_text(json.dumps(summary, indent=2, ensure_ascii=False), encoding="utf-8")

    print()
    print(f"Extracted {len(results)} trees with {summary['total_items']} total items")
    print(f"Summary written to {summary_path}")


if __name__ == "__main__":
    main()
