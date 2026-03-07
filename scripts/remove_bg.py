"""
Remove background from images by replacing dark/black pixels with transparency.
Only the outermost background (flood-filled from image edges) is removed,
leaving enclosed interior regions intact.

Usage:
    python scripts/remove_bg.py <input> [output]
    python scripts/remove_bg.py <input_dir> [output_dir]

Options:
    --threshold INT   Brightness threshold (0-255, default: 30)
                      Pixels darker than this become transparent.
    --color R G B     Target color to remove instead of black (0-255 each)
    --tolerance INT   Color tolerance for non-black targets (default: 30)
"""

import argparse
import sys
from pathlib import Path
from collections import deque
from PIL import Image
import numpy as np


def flood_fill_mask(is_bg: np.ndarray) -> np.ndarray:
    """Return a boolean mask of pixels reachable from the image border that are background."""
    h, w = is_bg.shape
    visited = np.zeros((h, w), dtype=bool)
    queue = deque()

    # Seed from all border pixels that are background
    for y in range(h):
        for x in (0, w - 1):
            if is_bg[y, x] and not visited[y, x]:
                visited[y, x] = True
                queue.append((y, x))
    for x in range(w):
        for y in (0, h - 1):
            if is_bg[y, x] and not visited[y, x]:
                visited[y, x] = True
                queue.append((y, x))

    while queue:
        y, x = queue.popleft()
        for dy, dx in ((-1, 0), (1, 0), (0, -1), (0, 1)):
            ny, nx = y + dy, x + dx
            if 0 <= ny < h and 0 <= nx < w and not visited[ny, nx] and is_bg[ny, nx]:
                visited[ny, nx] = True
                queue.append((ny, nx))

    return visited


def remove_black_bg(img: Image.Image, threshold: int = 30) -> Image.Image:
    """Remove only the outer black background (flood-fill from edges)."""
    rgba = img.convert("RGBA")
    data = np.array(rgba, dtype=np.uint8)

    brightness = data[:, :, :3].max(axis=2)
    is_bg = brightness <= threshold

    outer_mask = flood_fill_mask(is_bg)
    data[:, :, 3] = np.where(outer_mask, 0, data[:, :, 3])

    return Image.fromarray(data, "RGBA")


def remove_color_bg(
    img: Image.Image, color: tuple[int, int, int], tolerance: int = 30
) -> Image.Image:
    """Remove only the outer region of the given color (flood-fill from edges)."""
    rgba = img.convert("RGBA")
    data = np.array(rgba, dtype=np.int32)

    target = np.array(color, dtype=np.int32)
    is_bg = np.abs(data[:, :, :3] - target).max(axis=2) <= tolerance

    outer_mask = flood_fill_mask(is_bg)
    data[:, :, 3] = np.where(outer_mask, 0, data[:, :, 3])

    return Image.fromarray(data.astype(np.uint8), "RGBA")


def process_file(src: Path, dst: Path, args: argparse.Namespace) -> None:
    img = Image.open(src)

    if args.color:
        result = remove_color_bg(img, tuple(args.color), args.tolerance)
    else:
        result = remove_black_bg(img, args.threshold)

    dst.parent.mkdir(parents=True, exist_ok=True)
    result.save(dst)
    print(f"  {src.name} -> {dst}")


def main() -> None:
    parser = argparse.ArgumentParser(description="Remove image background")
    parser.add_argument("input", help="Input image or directory")
    parser.add_argument("output", nargs="?", help="Output image or directory")
    parser.add_argument(
        "--threshold", type=int, default=30, metavar="INT",
        help="Brightness threshold for black-bg removal (default: 30)"
    )
    parser.add_argument(
        "--color", type=int, nargs=3, metavar=("R", "G", "B"),
        help="Target color to remove (e.g. --color 255 255 255)"
    )
    parser.add_argument(
        "--tolerance", type=int, default=30, metavar="INT",
        help="Color tolerance when using --color (default: 30)"
    )
    args = parser.parse_args()

    src = Path(args.input)

    if src.is_dir():
        out_dir = Path(args.output) if args.output else src.parent / (src.name + "_transparent")
        images = [
            p for ext in ("*.png", "*.jpg", "*.jpeg", "*.webp", "*.bmp")
            for p in src.glob(ext)
        ]
        if not images:
            print(f"No images found in {src}")
            sys.exit(1)
        print(f"Processing {len(images)} image(s) -> {out_dir}")
        for img_path in images:
            dst = out_dir / img_path.with_suffix(".png").name
            process_file(img_path, dst, args)
    else:
        if not src.exists():
            print(f"File not found: {src}")
            sys.exit(1)
        if args.output:
            dst = Path(args.output)
        else:
            dst = src.with_stem(src.stem + "_transparent").with_suffix(".png")
        process_file(src, dst, args)

    print("Done.")


if __name__ == "__main__":
    main()
