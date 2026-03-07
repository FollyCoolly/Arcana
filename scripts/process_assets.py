"""
Asset preprocessing script for RealityMod UI images.

Usage:
    python scripts/process_assets.py

Requires:
    pip install Pillow numpy

What it does:
    0. (Optional) Remove outer background via flood-fill for images with a solid
       color background (e.g. AI-generated icons on black/white bg)
    1. Clean semi-transparent background pixels (AI-generated images often have
       near-transparent fringe pixels instead of true transparency)
    2. Trim to tight bounding box of visible content
    3. Resize to 2x display size (height capped at 320px)
    4. Optimize PNG compression -> static/ui/<name>.png
    5. Output a _scarlet.png with solid red background for visual QA

ASSETS dict format:
    "src_path": ("dest_name.png", {"remove_bg": True, "bg_threshold": 30})
    or simply:
    "src_path": "dest_name.png"   # no remove_bg
"""

from pathlib import Path
from PIL import Image, ImageDraw
from remove_bg import remove_black_bg, remove_color_bg

# Input -> output filename mapping.
# Value can be a plain string (dest name) or a (dest_name, options) tuple.
# Supported options:
#   remove_bg: bool          – strip outer background before other processing
#   bg_threshold: int        – brightness threshold for black-bg removal (default 30)
#   bg_color: (R, G, B)      – remove a non-black outer background instead
#   bg_tolerance: int        – tolerance when using bg_color (default 30)
ASSETS: dict[str, str | tuple[str, dict]] = {
    "D:/Pictures/RealityMod/back.png": "back.png",
    "D:/Downloads/Status.png": ("Status.png", {"remove_bg": True}),
}

OUTPUT_DIR = Path(__file__).parent.parent / "static" / "ui"
# Target height in CSS pixels; script outputs at 2x for HiDPI screens
TARGET_CSS_HEIGHT_PX = 160
OUTPUT_SCALE = 2

# Pixels with alpha below this value are forced to fully transparent.
# Raise if AI fringe pixels are still visible; lower if real content gets clipped.
ALPHA_CLEAN_THRESHOLD = 64

# Brand red used for the QA scarlet background
SCARLET = (128, 0, 26, 255)


def clean_alpha(img: Image.Image, threshold: int = ALPHA_CLEAN_THRESHOLD) -> Image.Image:
    """Force pixels below alpha threshold to fully transparent.

    AI-generated PNGs often leave semi-transparent noise in background areas
    that should be fully transparent. This pass zeroes them out before trimming.
    """
    r, g, b, a = img.split()
    cleaned_a = a.point(lambda v: 0 if v < threshold else v)
    return Image.merge("RGBA", (r, g, b, cleaned_a))


def trim(img: Image.Image) -> Image.Image:
    """Crop to the tight bounding box of non-transparent pixels."""
    bbox = img.split()[3].getbbox()
    return img.crop(bbox) if bbox else img


def resize_to_target(img: Image.Image) -> Image.Image:
    """Resize preserving aspect ratio, capping height at TARGET_CSS_HEIGHT_PX * OUTPUT_SCALE."""
    target_h = TARGET_CSS_HEIGHT_PX * OUTPUT_SCALE
    if img.height <= target_h:
        return img
    scale = target_h / img.height
    return img.resize((round(img.width * scale), target_h), Image.LANCZOS)


def make_scarlet(img: Image.Image) -> Image.Image:
    """Composite img over a solid scarlet background for visual QA."""
    bg = Image.new("RGBA", img.size, SCARLET)
    bg.paste(img, mask=img.split()[3])
    return bg.convert("RGB")


def process(src: str, dest_name: str, options: dict | None = None) -> None:
    opts = options or {}
    src_path = Path(src)
    if not src_path.exists():
        print(f"  SKIP  {src_path} not found")
        return

    original_kb = src_path.stat().st_size / 1024
    img = Image.open(src_path).convert("RGBA")
    original_size = img.size

    # 0. Remove outer background (flood-fill) if requested
    if opts.get("remove_bg"):
        if "bg_color" in opts:
            img = remove_color_bg(img, tuple(opts["bg_color"]), opts.get("bg_tolerance", 30))
        else:
            img = remove_black_bg(img, opts.get("bg_threshold", 30))

    # 1. Clean semi-transparent background noise
    img = clean_alpha(img)

    # 2. Trim to tight bounding box
    img = trim(img)
    trimmed_size = img.size

    # 3. Resize
    img = resize_to_target(img)

    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

    # 4. Save production PNG
    dest_path = OUTPUT_DIR / dest_name
    img.save(dest_path, format="PNG", optimize=True, compress_level=9)
    output_kb = dest_path.stat().st_size / 1024

    # 5. Save scarlet QA version
    stem = Path(dest_name).stem
    scarlet_path = OUTPUT_DIR / f"{stem}_scarlet.png"
    make_scarlet(img).save(scarlet_path, format="PNG", optimize=True, compress_level=9)
    scarlet_kb = scarlet_path.stat().st_size / 1024

    print(
        f"  OK    {dest_name}\n"
        f"        {original_size[0]}x{original_size[1]} -> "
        f"trim {trimmed_size[0]}x{trimmed_size[1]} -> "
        f"out {img.size[0]}x{img.size[1]}\n"
        f"        {original_kb:.0f} KB -> {output_kb:.0f} KB  "
        f"(scarlet QA: {scarlet_kb:.0f} KB -> {scarlet_path.name})"
    )


if __name__ == "__main__":
    print(f"Output dir: {OUTPUT_DIR}\n")
    for src, dest in ASSETS.items():
        if isinstance(dest, tuple):
            dest_name, options = dest
            process(src, dest_name, options)
        else:
            process(src, dest)
