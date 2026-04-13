"""
Validate Arcana data JSON files after AI writes.

Used as a PostToolUse hook — receives a file path, validates schema
rules for known data files, and exits non-zero on errors so Claude
can see and fix them immediately.

Usage:
    python scripts/validate_data.py <file_path>
"""

import json
import sys
from datetime import datetime, timezone
from pathlib import Path

DATA_DIR = (Path(__file__).parent / ".." / "data").resolve()

MISSION_STATUSES = {"proposed", "active", "completed", "archived", "rejected"}
ACHIEVEMENT_STATUSES = {"tracked", "achieved"}
DIFFICULTY_LEVELS = {"beginner", "intermediate", "advanced", "expert", "legendary"}
CHANGELOG_SKILLS = {"velvet-room", "phan-site", "agent"}
CHANGELOG_CHANGE_TYPES = {"add", "update", "delete"}
PRIORITY_LEVELS = {"high", "medium", "low"}

# Files whose modification should be accompanied by a changelog entry
CHANGELOG_TRACKED_FILES = {"missions.json", "achievement_progress.json", "status.json"}


def fail(msg: str) -> None:
    print(f"VALIDATION ERROR: {msg}", file=sys.stderr)
    sys.exit(1)


def warn(msg: str) -> None:
    print(f"VALIDATION WARNING: {msg}", file=sys.stderr)


# ─────────────────────────────────────────────────────────────────────────────
# Per-file validators
# ─────────────────────────────────────────────────────────────────────────────


def validate_missions(data: dict, path: Path) -> None:
    if "version" not in data:
        fail(f"{path.name}: missing 'version' field")

    missions = data.get("missions")
    if not isinstance(missions, list):
        fail(f"{path.name}: 'missions' must be an array")

    seen_ids: set[str] = set()
    for i, m in enumerate(missions):
        prefix = f"{path.name}: missions[{i}]"

        for field in ("id", "title", "status"):
            if field not in m:
                fail(f"{prefix}: missing required field '{field}'")

        mid = m["id"]
        if mid in seen_ids:
            fail(f"{prefix}: duplicate mission id '{mid}'")
        seen_ids.add(mid)

        if m["status"] not in MISSION_STATUSES:
            fail(f"{prefix}: invalid status '{m['status']}', must be one of {MISSION_STATUSES}")

        progress = m.get("progress")
        if progress is not None:
            if not isinstance(progress, (int, float)) or progress < 0 or progress > 100:
                fail(f"{prefix}: progress must be 0-100, got {progress}")

    # main_menu references
    main_menu = data.get("main_menu")
    if isinstance(main_menu, dict):
        for widget in ("countdown", "progress"):
            widget_data = main_menu.get(widget)
            if isinstance(widget_data, dict) and "mission_id" in widget_data:
                ref_id = widget_data["mission_id"]
                if ref_id not in seen_ids:
                    fail(f"{path.name}: main_menu.{widget}.mission_id '{ref_id}' not found in missions")


def validate_achievement_progress(data: dict, path: Path) -> None:
    if "version" not in data:
        fail(f"{path.name}: missing 'version' field")

    achievements = data.get("achievements")
    if not isinstance(achievements, dict):
        fail(f"{path.name}: 'achievements' must be an object")

    for aid, entry in achievements.items():
        if not isinstance(entry, dict):
            fail(f"{path.name}: achievements['{aid}'] must be an object")
        status = entry.get("status")
        if status not in ACHIEVEMENT_STATUSES:
            fail(f"{path.name}: achievements['{aid}'].status '{status}' must be one of {ACHIEVEMENT_STATUSES}")


def validate_changelog(data: dict, path: Path) -> None:
    if "version" not in data:
        fail(f"{path.name}: missing 'version' field")

    entries = data.get("entries")
    if not isinstance(entries, list):
        fail(f"{path.name}: 'entries' must be an array")

    if len(entries) > 200:
        fail(f"{path.name}: entries count {len(entries)} exceeds max 200")

    for i, entry in enumerate(entries):
        prefix = f"{path.name}: entries[{i}]"
        for field in ("timestamp", "skill", "changes"):
            if field not in entry:
                fail(f"{prefix}: missing required field '{field}'")

        if entry.get("skill") not in CHANGELOG_SKILLS:
            fail(f"{prefix}: invalid skill '{entry.get('skill')}', must be one of {CHANGELOG_SKILLS}")

        changes = entry.get("changes")
        if not isinstance(changes, list):
            fail(f"{prefix}: 'changes' must be an array")

        for j, change in enumerate(changes):
            cprefix = f"{prefix}.changes[{j}]"
            ctype = change.get("type")
            if ctype not in CHANGELOG_CHANGE_TYPES:
                fail(f"{cprefix}: invalid type '{ctype}', must be one of {CHANGELOG_CHANGE_TYPES}")
            if ctype == "update" and "old_value" not in change:
                fail(f"{cprefix}: 'update' type change must have 'old_value' for rollback")


def validate_mission_memory(data: dict, path: Path) -> None:
    if "version" not in data:
        fail(f"{path.name}: missing 'version' field")

    ctx = data.get("conversation_context")
    if isinstance(ctx, list) and len(ctx) > 20:
        fail(f"{path.name}: conversation_context has {len(ctx)} entries, max 20")

    log = data.get("completed_mission_log")
    if isinstance(log, list) and len(log) > 50:
        fail(f"{path.name}: completed_mission_log has {len(log)} entries, max 50")


def validate_status(data: dict, path: Path) -> None:
    if "version" not in data:
        fail(f"{path.name}: missing 'version' field")

    metrics = data.get("metrics")
    if not isinstance(metrics, dict):
        fail(f"{path.name}: 'metrics' must be an object")

    for key, val in metrics.items():
        if not isinstance(val, (int, float)):
            fail(f"{path.name}: metrics['{key}'] must be a number, got {type(val).__name__}")


def validate_loaded_packs(data: dict, path: Path) -> None:
    if "version" not in data:
        fail(f"{path.name}: missing 'version' field")

    packs = data.get("packs")
    if not isinstance(packs, list):
        fail(f"{path.name}: 'packs' must be an array")

    for i, p in enumerate(packs):
        if not isinstance(p, str):
            fail(f"{path.name}: packs[{i}] must be a string, got {type(p).__name__}")


def validate_pack_manifest(data: dict, path: Path) -> None:
    pack_id = path.parent.name
    for field in ("id", "name", "description", "version", "author"):
        if field not in data:
            fail(f"{path}: missing required field '{field}'")

    if data.get("id") != pack_id:
        fail(f"{path}: manifest.id '{data.get('id')}' must equal directory name '{pack_id}'")


def validate_pack_achievements(data: dict, path: Path) -> None:
    pack_id = path.parent.name
    if "version" not in data:
        fail(f"{path}: missing 'version' field")

    achievements = data.get("achievements")
    if not isinstance(achievements, list):
        fail(f"{path}: 'achievements' must be an array")

    expected_prefix = f"{pack_id}::"
    seen_ids: set[str] = set()

    for i, a in enumerate(achievements):
        prefix = f"{path}: achievements[{i}]"

        for field in ("id", "name", "description", "difficulty"):
            if field not in a:
                fail(f"{prefix}: missing required field '{field}'")

        aid = a["id"]
        if not aid.startswith(expected_prefix):
            fail(f"{prefix}: id '{aid}' must start with '{expected_prefix}'")
        if aid in seen_ids:
            fail(f"{prefix}: duplicate achievement id '{aid}'")
        seen_ids.add(aid)

        if a["difficulty"] not in DIFFICULTY_LEVELS:
            fail(f"{prefix}: invalid difficulty '{a['difficulty']}', must be one of {DIFFICULTY_LEVELS}")


def validate_pack_skills(data: dict, path: Path) -> None:
    pack_id = path.parent.name
    if "version" not in data:
        fail(f"{path}: missing 'version' field")

    skills = data.get("skills")
    if not isinstance(skills, list):
        fail(f"{path}: 'skills' must be an array")

    expected_prefix = f"{pack_id}::"

    for i, s in enumerate(skills):
        prefix = f"{path}: skills[{i}]"

        for field in ("id", "name", "max_level", "level_thresholds", "nodes"):
            if field not in s:
                fail(f"{prefix}: missing required field '{field}'")

        sid = s["id"]
        if not sid.startswith(expected_prefix):
            fail(f"{prefix}: id '{sid}' must start with '{expected_prefix}'")

        max_level = s["max_level"]
        thresholds = s["level_thresholds"]
        if not isinstance(thresholds, list):
            fail(f"{prefix}: 'level_thresholds' must be an array")
        if len(thresholds) != max_level:
            fail(f"{prefix}: level_thresholds length {len(thresholds)} != max_level {max_level}")

        prev_points = -1
        for j, t in enumerate(thresholds):
            pts = t.get("points_required", 0)
            if pts <= prev_points:
                fail(f"{prefix}: level_thresholds[{j}].points_required {pts} must be > previous {prev_points}")
            prev_points = pts

        nodes = s["nodes"]
        if not isinstance(nodes, list):
            fail(f"{prefix}: 'nodes' must be an array")

        seen_node_ids: set[str] = set()
        for j, n in enumerate(nodes):
            nid = n.get("node_id")
            if nid in seen_node_ids:
                fail(f"{prefix}: duplicate node_id '{nid}'")
            if nid:
                seen_node_ids.add(nid)


# ─────────────────────────────────────────────────────────────────────────────
# Changelog completeness check
# ─────────────────────────────────────────────────────────────────────────────


def check_changelog_freshness(file_name: str) -> None:
    """Warn if a tracked data file was modified but changelog wasn't updated recently."""
    if file_name not in CHANGELOG_TRACKED_FILES:
        return

    changelog_path = DATA_DIR / "ai_changelog.json"
    if not changelog_path.exists():
        warn(f"'{file_name}' was modified but ai_changelog.json does not exist yet. "
             "Remember to create it and log this change.")
        return

    try:
        changelog = json.loads(changelog_path.read_text(encoding="utf-8"))
    except (json.JSONDecodeError, OSError):
        return  # Don't warn if changelog itself is broken — the next write will catch it

    entries = changelog.get("entries")
    if not isinstance(entries, list) or len(entries) == 0:
        warn(f"'{file_name}' was modified but ai_changelog.json has no entries. "
             "Remember to log this change.")
        return

    latest = entries[-1]
    ts_str = latest.get("timestamp", "")
    try:
        ts = datetime.fromisoformat(ts_str)
        now = datetime.now(timezone.utc)
        if ts.tzinfo is None:
            ts = ts.replace(tzinfo=timezone.utc)
        age = (now - ts).total_seconds()
        if age > 60:
            warn(f"'{file_name}' was modified but the latest ai_changelog.json entry "
                 f"is {int(age)}s old. Did you forget to update the changelog?")
    except (ValueError, TypeError):
        pass  # Can't parse timestamp — don't warn


# ─────────────────────────────────────────────────────────────────────────────
# Main dispatch
# ─────────────────────────────────────────────────────────────────────────────

VALIDATORS: dict[str, callable] = {
    "missions.json": validate_missions,
    "achievement_progress.json": validate_achievement_progress,
    "ai_changelog.json": validate_changelog,
    "mission_memory.json": validate_mission_memory,
    "status.json": validate_status,
    "loaded_packs.json": validate_loaded_packs,
}


def main() -> None:
    if len(sys.argv) < 2:
        sys.exit(0)

    file_path = Path(sys.argv[1]).resolve()

    # Skip non-data files silently
    try:
        file_path.relative_to(DATA_DIR)
    except ValueError:
        sys.exit(0)

    # Must be a JSON file
    if file_path.suffix != ".json":
        sys.exit(0)

    # Parse JSON
    try:
        text = file_path.read_text(encoding="utf-8")
    except OSError as e:
        fail(f"Cannot read {file_path}: {e}")

    try:
        data = json.loads(text)
    except json.JSONDecodeError as e:
        fail(f"{file_path.name}: invalid JSON — {e}")

    if not isinstance(data, dict):
        fail(f"{file_path.name}: top-level value must be a JSON object")

    file_name = file_path.name

    # Dispatch to specific validator
    if file_name in VALIDATORS:
        VALIDATORS[file_name](data, file_path)
    elif file_path.parent.name == "packs" or (
        file_path.parent.parent.is_dir()
        and file_path.parent.parent.name == "packs"
    ):
        # Pack files: data/packs/<pack_id>/<file>.json
        # Check if we're inside a pack directory
        rel = file_path.relative_to(DATA_DIR)
        parts = rel.parts  # e.g. ("packs", "programmer", "achievements.json")
        if len(parts) == 3 and parts[0] == "packs":
            pack_validators = {
                "manifest.json": validate_pack_manifest,
                "achievements.json": validate_pack_achievements,
                "skills.json": validate_pack_skills,
            }
            validator = pack_validators.get(file_name)
            if validator:
                validator(data, file_path)

    # Changelog freshness check (non-blocking)
    check_changelog_freshness(file_name)


if __name__ == "__main__":
    main()
