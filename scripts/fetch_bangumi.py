"""
Fetch watched anime from Bangumi.tv and generate gallery JSON.

Reads config from scripts/config.json, queries the Bangumi v0 public API
for a user's "watched" anime collections, and writes the result to
data/gallery/bangumi_anime.json.

Usage:
    python scripts/fetch_bangumi.py
"""

import json
import sys
import time
from pathlib import Path

SCRIPT_DIR = Path(__file__).parent
CONFIG_PATH = SCRIPT_DIR / "config.json"
OUTPUT_PATH = SCRIPT_DIR / ".." / "data" / "gallery" / "bangumi_anime.json"

API_BASE = "https://api.bgm.tv/v0"
BANGUMI_SUBJECT_URL = "https://bgm.tv/subject/{}"
PAGE_LIMIT = 50
REQUEST_INTERVAL = 0.5
MAX_RETRIES = 3
RETRY_WAIT = 5


# ─────────────────────────────────────────────────────────────────────────────
# HTTP client — prefer requests, fallback to urllib
# ─────────────────────────────────────────────────────────────────────────────

_session = None
_user_agent = "RealityMod/gallery-fetcher"

try:
    import requests as _requests_lib

    def _init_session(user_agent: str):
        global _session, _user_agent
        _user_agent = user_agent
        _session = _requests_lib.Session()
        _session.headers.update({"User-Agent": user_agent})

    def _http_get(url: str, params: dict | None = None) -> tuple[int, dict | list | None]:
        resp = _session.get(url, params=params, timeout=30)
        body = resp.json() if resp.headers.get("content-type", "").startswith("application/json") else None
        return resp.status_code, body

except ImportError:
    import urllib.request
    import urllib.parse
    import urllib.error

    def _init_session(user_agent: str):
        global _user_agent
        _user_agent = user_agent

    def _http_get(url: str, params: dict | None = None) -> tuple[int, dict | list | None]:
        if params:
            url = url + "?" + urllib.parse.urlencode(params)
        req = urllib.request.Request(url, headers={"User-Agent": _user_agent})
        try:
            with urllib.request.urlopen(req, timeout=30) as resp:
                body = json.loads(resp.read().decode("utf-8"))
                return resp.status, body
        except urllib.error.HTTPError as e:
            body = None
            try:
                body = json.loads(e.read().decode("utf-8"))
            except Exception:
                pass
            return e.code, body


def api_get(path: str, params: dict | None = None) -> dict | list:
    """GET from the Bangumi API with retry on 429."""
    url = f"{API_BASE}{path}"
    for attempt in range(1, MAX_RETRIES + 1):
        status, body = _http_get(url, params)
        if status == 429:
            if attempt < MAX_RETRIES:
                print(f"  Rate limited (429), waiting {RETRY_WAIT}s... (attempt {attempt}/{MAX_RETRIES})")
                time.sleep(RETRY_WAIT)
                continue
            else:
                print(f"ERROR: Rate limited after {MAX_RETRIES} retries. Try again later.")
                sys.exit(1)
        if status == 404:
            print(f"ERROR: Not found (404) — {url}")
            sys.exit(1)
        if status != 200:
            print(f"ERROR: HTTP {status} from {url}")
            sys.exit(1)
        return body
    return body  # unreachable, but satisfies type checker


# ─────────────────────────────────────────────────────────────────────────────
# Config
# ─────────────────────────────────────────────────────────────────────────────

def load_config() -> dict:
    if not CONFIG_PATH.exists():
        print(f"ERROR: Config file not found: {CONFIG_PATH}")
        print(f"  Copy scripts/config.example.json to scripts/config.json")
        print(f"  and fill in your Bangumi UID.")
        sys.exit(1)

    config = json.loads(CONFIG_PATH.read_text(encoding="utf-8"))

    if not config.get("bangumi_uid"):
        print("ERROR: bangumi_uid is empty in config.json.")
        print("  Set it to your Bangumi.tv UID (numeric).")
        sys.exit(1)

    return config


# ─────────────────────────────────────────────────────────────────────────────
# Fetch & transform
# ─────────────────────────────────────────────────────────────────────────────

def fetch_collections(uid: str) -> list[dict]:
    """Fetch all 'watched' anime collections for a user by UID (paginated)."""
    items = []
    offset = 0

    while True:
        params = {
            "subject_type": 2,  # anime
            "type": 2,          # watched
            "limit": PAGE_LIMIT,
            "offset": offset,
        }
        data = api_get(f"/users/{uid}/collections", params)
        total = data.get("total", 0)
        page_items = data.get("data", [])
        items.extend(page_items)

        print(f"  Fetched {len(items)}/{total} collections...")

        offset += PAGE_LIMIT
        if offset >= total:
            break
        time.sleep(REQUEST_INTERVAL)

    return items


def map_item(entry: dict, top_tags_count: int) -> dict | None:
    """Convert a Bangumi collection entry to gallery format."""
    subject = entry.get("subject")
    if subject is None:
        return None

    subject_id = entry.get("subject_id", subject.get("id"))
    name_cn = subject.get("name_cn", "")
    name = subject.get("name", "")

    images = subject.get("images") or {}
    cover = images.get("large")

    score = subject.get("score", 0)
    rating = score if score else None

    my_rate = entry.get("rate", 0)
    my_rating = my_rate if my_rate else None

    tags_list = subject.get("tags") or []
    tags_sorted = sorted(tags_list, key=lambda t: t.get("count", 0), reverse=True)
    tags = [t["name"] for t in tags_sorted[:top_tags_count]]

    eps = subject.get("eps", 0)
    episodes = eps if eps else None

    return {
        "name": name_cn or name,
        "name_original": name,
        "cover": cover,
        "rating": rating,
        "my_rating": my_rating,
        "date_started": None,
        "date_finished": None,
        "tags": tags,
        "episodes": episodes,
        "extra": {
            "bangumi_id": subject_id,
            "bangumi_url": BANGUMI_SUBJECT_URL.format(subject_id),
            "air_date": subject.get("date"),
        },
    }


# ─────────────────────────────────────────────────────────────────────────────
# Main
# ─────────────────────────────────────────────────────────────────────────────

def main():
    start_time = time.time()

    config = load_config()
    uid = config["bangumi_uid"]
    top_tags_count = config.get("top_tags_count", 5)
    user_agent = config.get("user_agent", "RealityMod/gallery-fetcher")

    _init_session(user_agent)

    print(f"Fetching watched anime for UID: {uid}")
    collections = fetch_collections(uid)

    warnings = []
    gallery_items = []
    for entry in collections:
        item = map_item(entry, top_tags_count)
        if item is None:
            sid = entry.get("subject_id", "?")
            warnings.append(f"Skipped entry with subject_id={sid} (subject is None)")
            continue
        gallery_items.append(item)

    output = {
        "version": 1,
        "items": gallery_items,
    }

    OUTPUT_PATH.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT_PATH.write_text(
        json.dumps(output, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )

    elapsed = time.time() - start_time

    print()
    print(f"Done! {len(gallery_items)} items written to {OUTPUT_PATH.resolve()}")
    print(f"Time: {elapsed:.1f}s")
    if warnings:
        print(f"\nWarnings ({len(warnings)}):")
        for w in warnings:
            print(f"  - {w}")


if __name__ == "__main__":
    main()
