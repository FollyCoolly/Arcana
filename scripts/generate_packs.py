"""
Generate RealityMod content packs from decoded MakerSkillTree data.

Reads decoded tree JSON files from docs/tmp/decoded_trees/ and generates
complete content packs (manifest.json, achievements.json, skills.json)
in data/packs/.

Usage:
    python scripts/generate_packs.py [--input DIR] [--output DIR] [--dry-run]
"""

import argparse
import json
import math
import os
import re
import sys
from pathlib import Path

SCRIPT_DIR = Path(__file__).parent
DEFAULT_INPUT = SCRIPT_DIR / ".." / "docs" / "tmp" / "decoded_trees"
DEFAULT_OUTPUT = SCRIPT_DIR / ".." / "data" / "packs"

# ─────────────────────────────────────────────────────────────────────────────
# Pack mapping: source tree slug → (pack_id, skill_name_hint)
# Trees mapping to the same pack_id will be merged into one pack.
# ─────────────────────────────────────────────────────────────────────────────

PACK_MAPPING = {
    # ── Tech ──
    "coding":                 ("programmer",          "programming"),
    "computing_basics":       ("computing",           "computing_basics"),
    "website_building":       ("webdev",              "web_development"),
    "mobile_app_dev":         ("mobile_dev",          "mobile_development"),
    "game_dev":               ("gamedev",             "game_development"),
    "linux":                  ("linux",               "linux"),
    "it_security":            ("cybersecurity",       "cybersecurity"),
    "kubernetes":             ("devops",              "devops"),
    "electronics":            ("electronics",         "electronics"),
    "embedded_systems":       ("electronics",         "embedded_systems"),
    "dev_boards":             ("electronics",         "dev_boards"),
    "microcontrollers":       ("electronics",         "microcontrollers"),
    "single_board_computers": ("electronics",         "single_board_computers"),
    "pcb_design":             ("electronics",         "pcb_design"),
    "robotics":               ("robotics",            "robotics"),
    "3d_modeling":            ("digital_fabrication", "3d_modeling"),
    "3d_printing":            ("digital_fabrication", "3d_printing"),
    "laser_cutting":          ("digital_fabrication", "laser_cutting"),
    "cnc_cam":                ("digital_fabrication", "cnc_cam"),
    "try_ai":                 ("ai",                  "artificial_intelligence"),

    # ── Life ──
    "cooking":                ("cooking",             "cooking"),
    "baking":                 ("cooking",             "baking"),
    "cocktails":              ("cooking",             "cocktails"),
    "cleaning":               ("life_skills",         "cleaning"),
    "finance":                ("finance",             "finance"),
    "entrepreneurship":       ("career",              "entrepreneurship"),
    "leadership":             ("career",              "leadership"),
    "language":               ("language",            "language_learning"),
    "gardening":              ("gardening",           "gardening"),
    "travel":                 ("travel",              "travel"),
    "self_care":              ("self_care",           "self_care"),
    "survivalist":            ("survival",            "survival"),

    # ── Crafts ──
    "woodworking":            ("woodworking",         "woodworking"),
    "metalworking":           ("metalworking",        "metalworking"),
    "sewing":                 ("crafting",            "sewing"),
    "knitting":               ("crafting",            "knitting"),
    "crochet":                ("crafting",            "crochet"),
    "embroidery":             ("crafting",            "embroidery"),
    "crafting":               ("crafting",            "general_crafting"),
    "visual_arts":            ("visual_arts",         "visual_arts"),
    "comic_artist":           ("visual_arts",         "comic_art"),
    "photography":            ("photography",         "photography"),
    "photo_video":            ("photography",         "photo_video"),
    "music":                  ("music",               "music"),
    "house_building":         ("build_repair",        "house_building"),
    "build_repair":           ("build_repair",        "build_repair"),
    "renovation_repair":      ("build_repair",        "renovation"),
    "automotive":             ("automotive",          "automotive"),

    # ── Sports/Outdoor ──
    "sports_fitness":         ("fitness",             "sports_fitness"),
    "climbing":               ("fitness",             "climbing"),
    "hiking":                 ("fitness",             "hiking"),
    "dance":                  ("dance",               "dance"),
    "boating_fishing":        ("outdoors",            "boating_fishing"),

    # ── Academic ──
    "astronomy":              ("science",             "astronomy"),
    "academic_research_skills": ("academic",          "research_skills"),

    # ── Entertainment ──
    "dungeons_dragons":       ("tabletop",            "dnd"),
    "reading":                ("reading",             "reading"),
    "reading_and_writing":    ("reading",             "writing"),
}

# Pack display metadata
PACK_META = {
    "programmer":          ("Programmer",           "Programming skills and software development achievements.",       ["tech", "career"]),
    "computing":           ("Computing Basics",     "Fundamental computing literacy and digital skills.",              ["tech"]),
    "webdev":              ("Web Development",      "Website building and web technologies.",                          ["tech"]),
    "mobile_dev":          ("Mobile Development",   "Mobile app development for iOS and Android.",                     ["tech"]),
    "gamedev":             ("Game Development",     "Game design and development skills.",                             ["tech", "creative"]),
    "linux":               ("Linux",                "Linux system administration and command line skills.",             ["tech"]),
    "cybersecurity":       ("Cybersecurity",        "IT security and ethical hacking skills.",                         ["tech"]),
    "devops":              ("DevOps",               "Container orchestration and infrastructure management.",          ["tech"]),
    "electronics":         ("Electronics",          "Electronics, embedded systems, and hardware engineering.",         ["tech", "maker"]),
    "robotics":            ("Robotics",             "Robotics design, programming, and building.",                     ["tech", "maker"]),
    "digital_fabrication": ("Digital Fabrication",  "3D printing, modeling, laser cutting, and CNC.",                  ["tech", "maker"]),
    "ai":                  ("AI & Machine Learning","Artificial intelligence and machine learning exploration.",       ["tech"]),
    "cooking":             ("Cooking",              "Cooking, baking, and cocktail making skills.",                    ["life"]),
    "life_skills":         ("Life Skills",          "Practical everyday life skills.",                                 ["life"]),
    "finance":             ("Finance",              "Personal finance and money management.",                          ["life", "career"]),
    "career":              ("Career",               "Entrepreneurship and leadership skills.",                         ["career"]),
    "language":            ("Language Learning",    "Foreign language acquisition milestones.",                         ["life", "academic"]),
    "gardening":           ("Gardening",            "Gardening and plant care skills.",                                ["life", "outdoor"]),
    "travel":              ("Travel",               "Travel experiences and adventure milestones.",                     ["life", "outdoor"]),
    "self_care":           ("Self Care",            "Self care, wellness, and mindfulness practices.",                 ["life"]),
    "survival":            ("Survival",             "Wilderness survival and preparedness skills.",                     ["outdoor"]),
    "woodworking":         ("Woodworking",          "Woodworking techniques and projects.",                            ["maker", "craft"]),
    "metalworking":        ("Metalworking",         "Metalworking and fabrication skills.",                            ["maker", "craft"]),
    "crafting":            ("Crafting",             "Textile and general crafting skills.",                             ["craft"]),
    "visual_arts":         ("Visual Arts",          "Drawing, painting, and visual art skills.",                        ["creative"]),
    "photography":         ("Photography",          "Photography and videography skills.",                              ["creative"]),
    "music":               ("Music",                "Musical instruments, theory, and performance.",                    ["creative"]),
    "build_repair":        ("Build & Repair",       "Home building, renovation, and repair skills.",                    ["maker", "life"]),
    "automotive":          ("Automotive",           "Automotive maintenance and repair skills.",                         ["maker"]),
    "fitness":             ("Fitness",              "Sports, fitness, and physical training achievements.",              ["health", "outdoor"]),
    "dance":               ("Dance",                "Dance styles and performance skills.",                              ["creative", "health"]),
    "outdoors":            ("Outdoors",             "Boating, fishing, and outdoor recreation.",                         ["outdoor"]),
    "science":             ("Science",              "Scientific exploration and astronomy.",                             ["academic"]),
    "academic":            ("Academic",             "Research methodology and academic skills.",                          ["academic"]),
    "tabletop":            ("Tabletop Gaming",      "Tabletop RPGs and board gaming.",                                  ["entertainment"]),
    "reading":             ("Reading & Writing",    "Reading habits and creative writing.",                              ["creative", "academic"]),
}


# ─────────────────────────────────────────────────────────────────────────────
# Conversion helpers
# ─────────────────────────────────────────────────────────────────────────────

def row_to_difficulty(row: int) -> str:
    if row <= 2:
        return "beginner"
    elif row <= 4:
        return "intermediate"
    elif row <= 6:
        return "advanced"
    elif row <= 8:
        return "expert"
    return "legendary"


DIFFICULTY_POINTS = {
    "beginner": 5,
    "intermediate": 10,
    "advanced": 15,
    "expert": 20,
    "legendary": 30,
}


def slugify_item(text: str) -> str:
    """Convert an item text into a snake_case ID segment."""
    # Collapse newlines/whitespace
    s = re.sub(r"\s+", " ", text).strip()
    # Remove parenthetical content
    s = re.sub(r"\(.*?\)", "", s).strip()
    # Keep only alphanumeric and spaces
    s = re.sub(r"[^a-zA-Z0-9 ]", "", s)
    # Convert to snake_case
    s = s.strip().lower()
    s = re.sub(r"\s+", "_", s)
    # Truncate to reasonable length
    if len(s) > 60:
        s = s[:60].rstrip("_")
    return s


def make_achievement_id(pack_id: str, item_text: str, index: int) -> str:
    slug = slugify_item(item_text)
    if not slug:
        slug = f"item_{index}"
    return f"{pack_id}::{slug}"


def deduplicate_ids(achievements: list[dict]) -> list[dict]:
    """Ensure all achievement IDs are unique within a pack."""
    seen = {}
    for ach in achievements:
        aid = ach["id"]
        if aid in seen:
            seen[aid] += 1
            ach["id"] = f"{aid}_{seen[aid]}"
        else:
            seen[aid] = 0
    return achievements


def clean_item_text(text: str) -> str:
    """Clean up item text: collapse newlines to spaces, strip."""
    return re.sub(r"\s+", " ", text).strip()


def build_achievements_for_tree(pack_id: str, skill_hint: str, tree_data: dict) -> list[dict]:
    """Convert a single decoded tree into a list of achievement dicts."""
    items = tree_data.get("items", {})
    achievements = []

    for idx_str, raw_text in sorted(items.items(), key=lambda x: int(x[0])):
        idx = int(idx_str)
        row = idx // 7
        text = clean_item_text(raw_text)
        if not text:
            continue

        difficulty = row_to_difficulty(row)
        ach_id = make_achievement_id(pack_id, text, idx)

        achievements.append({
            "id": ach_id,
            "name": text[:80],  # cap display name length
            "description": text,
            "difficulty": difficulty,
            "category": skill_hint,
            "tags": [skill_hint],
            "_row": row,  # internal, stripped later
            "_idx": idx,
        })

    return achievements


def assign_prerequisites(achievements: list[dict]) -> list[dict]:
    """
    Assign prerequisites based on difficulty progression within the same category.

    Strategy: Within each category, each achievement depends on the first
    achievement of the previous difficulty tier (if one exists).
    This creates a simple DAG: beginner ← intermediate ← advanced ← expert ← legendary.
    """
    return assign_prerequisites_generated_only(achievements, set())


def assign_prerequisites_generated_only(achievements: list[dict],
                                         preserve_ids: set[str]) -> list[dict]:
    """
    Assign prerequisites only to achievements NOT in preserve_ids.
    Achievements in preserve_ids keep their existing prerequisites unchanged.
    """
    # Group by category
    by_category: dict[str, list[dict]] = {}
    for ach in achievements:
        cat = ach["category"]
        by_category.setdefault(cat, []).append(ach)

    difficulty_order = ["beginner", "intermediate", "advanced", "expert", "legendary"]

    for cat, cat_achs in by_category.items():
        # Find the first achievement at each difficulty level
        first_at_level: dict[str, str] = {}
        for diff in difficulty_order:
            for ach in cat_achs:
                if ach["difficulty"] == diff:
                    first_at_level[diff] = ach["id"]
                    break

        # Assign prerequisites only to non-preserved achievements
        for ach in cat_achs:
            if ach["id"] in preserve_ids:
                continue  # don't touch existing hand-crafted prerequisites

            diff = ach["difficulty"]
            diff_idx = difficulty_order.index(diff)
            if diff_idx > 0:
                for prev_idx in range(diff_idx - 1, -1, -1):
                    prev_diff = difficulty_order[prev_idx]
                    if prev_diff in first_at_level:
                        ach["prerequisites"] = [first_at_level[prev_diff]]
                        break
                else:
                    ach["prerequisites"] = []
            else:
                ach["prerequisites"] = []

    return achievements


def build_skill(pack_id: str, skill_hint: str, tree_title: str,
                achievements: list[dict]) -> dict:
    """Build a skill tree definition from achievements belonging to one source tree."""
    skill_id = f"{pack_id}::{skill_hint}"

    # Filter achievements for this skill's category
    skill_achs = [a for a in achievements if a["category"] == skill_hint]
    if not skill_achs:
        return None

    # Build nodes
    nodes = []
    total_possible_points = 0
    for i, ach in enumerate(skill_achs):
        points = DIFFICULTY_POINTS[ach["difficulty"]]
        total_possible_points += points
        row = ach.get("_row", 0)
        col = i % 7
        nodes.append({
            "node_id": f"node_{i}",
            "achievement_id": ach["id"],
            "points": points,
            "position": {"x": col, "y": row},
        })

    # Build level thresholds (5 levels)
    max_level = 5
    # Distribute points across levels
    thresholds = []
    key_achievements_by_level = _pick_key_achievements(skill_achs, max_level)

    for lvl in range(1, max_level + 1):
        fraction = lvl / max_level
        pts = max(5, round(total_possible_points * fraction * 0.6))
        threshold = {"level": lvl, "points_required": pts}
        if key_achievements_by_level.get(lvl):
            threshold["required_key_achievements"] = key_achievements_by_level[lvl]
        thresholds.append(threshold)

    # Ensure points are monotonically increasing
    for i in range(1, len(thresholds)):
        if thresholds[i]["points_required"] <= thresholds[i - 1]["points_required"]:
            thresholds[i]["points_required"] = thresholds[i - 1]["points_required"] + 5

    return {
        "id": skill_id,
        "name": clean_item_text(tree_title).title(),
        "max_level": max_level,
        "level_thresholds": thresholds,
        "nodes": nodes,
    }


def _pick_key_achievements(achs: list[dict], max_level: int) -> dict[int, list[str]]:
    """Pick key achievements for higher levels from expert/legendary items."""
    result = {}
    experts = [a for a in achs if a["difficulty"] == "expert"]
    legendaries = [a for a in achs if a["difficulty"] == "legendary"]

    if experts:
        result[3] = [experts[0]["id"]]
    if experts and len(experts) > 1:
        result[4] = [experts[0]["id"], experts[1]["id"]]
    elif experts:
        result[4] = [experts[0]["id"]]
    if legendaries:
        keys = result.get(4, [])[:] if 4 in result else (result.get(3, [])[:] if 3 in result else [])
        keys.append(legendaries[0]["id"])
        result[5] = keys

    return result


# ─────────────────────────────────────────────────────────────────────────────
# Main
# ─────────────────────────────────────────────────────────────────────────────

def main():
    parser = argparse.ArgumentParser(description="Generate RealityMod content packs")
    parser.add_argument("--input", default=str(DEFAULT_INPUT), help="Decoded trees directory")
    parser.add_argument("--output", default=str(DEFAULT_OUTPUT), help="Output packs directory")
    parser.add_argument("--dry-run", action="store_true", help="Print plan without writing files")
    args = parser.parse_args()

    input_dir = Path(args.input)
    output_dir = Path(args.output)

    # Load decoded trees
    tree_files = sorted(input_dir.glob("*.json"))
    tree_files = [f for f in tree_files if f.name != "_summary.json"]

    # Group trees by target pack
    pack_trees: dict[str, list[tuple[str, str, dict]]] = {}  # pack_id -> [(slug, skill_hint, data)]
    skipped = []

    for tf in tree_files:
        slug = tf.stem
        if slug not in PACK_MAPPING:
            skipped.append(slug)
            continue
        pack_id, skill_hint = PACK_MAPPING[slug]
        data = json.loads(tf.read_text(encoding="utf-8"))
        pack_trees.setdefault(pack_id, []).append((slug, skill_hint, data))

    print(f"Loaded {len(tree_files)} decoded trees")
    print(f"Mapped to {len(pack_trees)} content packs")
    if skipped:
        print(f"Skipped (no mapping): {', '.join(skipped)}")
    print()

    if args.dry_run:
        for pack_id, trees in sorted(pack_trees.items()):
            meta = PACK_META.get(pack_id, (pack_id.title(), "", []))
            total_items = sum(len(d.get("items", {})) for _, _, d in trees)
            print(f"  {pack_id}: {meta[0]} ({len(trees)} trees, ~{total_items} achievements)")
            for slug, hint, _ in trees:
                print(f"    - {slug} -> skill: {hint}")
        return

    # Generate each pack
    stats = []
    for pack_id, trees in sorted(pack_trees.items()):
        meta = PACK_META.get(pack_id, (pack_id.replace("_", " ").title(), f"Achievements for {pack_id}.", []))
        display_name, description, tags = meta

        # Load existing pack data if present (preserve hand-crafted content)
        existing_pack_dir = output_dir / pack_id
        existing_achievements = []
        existing_skills = []
        existing_manifest = None
        existing_ach_ids = set()

        if (existing_pack_dir / "achievements.json").exists():
            try:
                ea = json.loads((existing_pack_dir / "achievements.json").read_text(encoding="utf-8"))
                existing_achievements = ea.get("achievements", [])
                existing_ach_ids = {a["id"] for a in existing_achievements}
                print(f"  Preserving {len(existing_achievements)} existing achievements for {pack_id}")
            except (json.JSONDecodeError, KeyError):
                pass

        if (existing_pack_dir / "skills.json").exists():
            try:
                es = json.loads((existing_pack_dir / "skills.json").read_text(encoding="utf-8"))
                existing_skills = es.get("skills", [])
            except (json.JSONDecodeError, KeyError):
                pass

        if (existing_pack_dir / "manifest.json").exists():
            try:
                existing_manifest = json.loads(
                    (existing_pack_dir / "manifest.json").read_text(encoding="utf-8")
                )
            except (json.JSONDecodeError, KeyError):
                pass

        # Build all achievements from all source trees in this pack
        generated_achievements = []

        for slug, skill_hint, data in trees:
            tree_achs = build_achievements_for_tree(pack_id, skill_hint, data)
            generated_achievements.extend(tree_achs)

        # Deduplicate IDs among generated achievements
        generated_achievements = deduplicate_ids(generated_achievements)

        # Remove generated achievements that clash with existing hand-crafted IDs
        generated_achievements = [a for a in generated_achievements if a["id"] not in existing_ach_ids]

        # Merge: existing first, then generated
        all_achievements = existing_achievements + generated_achievements

        # Strip all prerequisites (MakerSkillTree has no real dependency data)
        for ach in all_achievements:
            ach.pop("prerequisites", None)

        # Build skill trees (one per source tree)
        existing_skill_ids = {s["id"] for s in existing_skills}
        skills = list(existing_skills)  # preserve existing

        for slug, skill_hint, data in trees:
            tree_title = data.get("title", skill_hint)
            skill = build_skill(pack_id, skill_hint, tree_title, all_achievements)
            if skill and skill["id"] not in existing_skill_ids:
                skills.append(skill)

        # Strip internal fields from generated achievements
        for ach in all_achievements:
            ach.pop("_row", None)
            ach.pop("_idx", None)

        # Write pack files
        pack_dir = output_dir / pack_id
        pack_dir.mkdir(parents=True, exist_ok=True)

        # manifest.json — preserve existing if present, else generate
        if existing_manifest:
            manifest = existing_manifest
        else:
            manifest = {
                "id": pack_id,
                "name": display_name,
                "description": description,
                "version": "1.0.0",
                "author": "MakerSkillTree (converted)",
                "tags": tags,
            }
        (pack_dir / "manifest.json").write_text(
            json.dumps(manifest, indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
        )

        # achievements.json
        achievements_data = {
            "version": 1,
            "achievements": all_achievements,
        }
        (pack_dir / "achievements.json").write_text(
            json.dumps(achievements_data, indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
        )

        # skills.json
        skills_data = {
            "version": 1,
            "skills": skills,
        }
        (pack_dir / "skills.json").write_text(
            json.dumps(skills_data, indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
        )

        stats.append((pack_id, display_name, len(all_achievements), len(skills)))
        print(f"  Generated: {pack_id} ({len(all_achievements)} achievements, {len(skills)} skill trees)")

    # Summary
    print()
    print(f"Total: {len(stats)} packs, {sum(s[2] for s in stats)} achievements, {sum(s[3] for s in stats)} skill trees")

    # Update loaded_packs.json
    loaded_packs_path = output_dir.parent / "loaded_packs.json"
    if loaded_packs_path.exists():
        loaded = json.loads(loaded_packs_path.read_text(encoding="utf-8"))
    else:
        loaded = {"version": 1, "packs": []}

    existing_packs = set(loaded["packs"])
    new_packs = [s[0] for s in stats]
    for p in new_packs:
        if p not in existing_packs:
            loaded["packs"].append(p)

    loaded_packs_path.write_text(
        json.dumps(loaded, indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
    )
    print(f"\nUpdated loaded_packs.json: {len(loaded['packs'])} packs")


if __name__ == "__main__":
    main()
