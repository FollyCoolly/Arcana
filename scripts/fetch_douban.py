"""
Fetch movie, TV, and book data from Douban and generate gallery JSON.

Uses the Douban Rexxar mobile API (discovered via doufen-org/tofu) to
fetch a user's "done" interests, then splits them into three output files:
  - data/gallery/douban_movies.json
  - data/gallery/douban_tv.json
  - data/gallery/douban_books.json

Japanese anime entries (card_subtitle containing both "动画" and "日本")
are excluded, since anime is tracked via Bangumi instead.

Usage:
    python scripts/fetch_douban.py                # "done" only (default)
    python scripts/fetch_douban.py --status all   # all statuses (done + doing + mark)
"""

import argparse
import json
import re
import sys
import time
from pathlib import Path

SCRIPT_DIR = Path(__file__).parent
CONFIG_PATH = SCRIPT_DIR / "config.json"
OUTPUT_DIR = SCRIPT_DIR / ".." / "data" / "gallery"

REXXAR_API = "https://m.douban.com/rexxar/api/v2/user/{uid}/interests"
PAGE_SIZE = 50
REQUEST_INTERVAL = 1.0
MAX_RETRIES = 3
RETRY_WAIT = 5


# ─────────────────────────────────────────────────────────────────────────────
# HTTP client — prefer requests, fallback to urllib
# ─────────────────────────────────────────────────────────────────────────────

_session = None
_user_agent = "Arcana/gallery-fetcher"
_cookie = ""

try:
    import requests as _requests_lib

    def _init_session(user_agent: str, cookie: str = ""):
        global _session, _user_agent, _cookie
        _user_agent = user_agent
        _cookie = cookie
        _session = _requests_lib.Session()
        headers = {"User-Agent": user_agent}
        if cookie:
            headers["Cookie"] = cookie
        _session.headers.update(headers)

    def _http_get(url: str, params: dict | None = None, headers: dict | None = None) -> tuple[int, dict | list | None]:
        resp = _session.get(url, params=params, headers=headers, timeout=30)
        body = resp.json() if resp.headers.get("content-type", "").startswith("application/json") else None
        return resp.status_code, body

except ImportError:
    import urllib.request
    import urllib.parse
    import urllib.error

    def _init_session(user_agent: str, cookie: str = ""):
        global _user_agent, _cookie
        _user_agent = user_agent
        _cookie = cookie

    def _http_get(url: str, params: dict | None = None, headers: dict | None = None) -> tuple[int, dict | list | None]:
        if params:
            url = url + "?" + urllib.parse.urlencode(params)
        req_headers = {"User-Agent": _user_agent}
        if _cookie:
            req_headers["Cookie"] = _cookie
        if headers:
            req_headers.update(headers)
        req = urllib.request.Request(url, headers=req_headers)
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


def api_get(url: str, params: dict | None = None, headers: dict | None = None) -> tuple[int, dict | list | None]:
    """GET with retry on 429."""
    for attempt in range(1, MAX_RETRIES + 1):
        status, body = _http_get(url, params, headers)
        if status == 429:
            if attempt < MAX_RETRIES:
                print(f"  Rate limited (429), waiting {RETRY_WAIT}s... (attempt {attempt}/{MAX_RETRIES})")
                time.sleep(RETRY_WAIT)
                continue
            else:
                print(f"ERROR: Rate limited after {MAX_RETRIES} retries. Try again later.")
                sys.exit(1)
        return status, body
    return status, body  # unreachable


# ─────────────────────────────────────────────────────────────────────────────
# Config
# ─────────────────────────────────────────────────────────────────────────────

def load_config() -> dict:
    if not CONFIG_PATH.exists():
        print(f"ERROR: Config file not found: {CONFIG_PATH}")
        print(f"  Copy scripts/config.example.json to scripts/config.json")
        print(f"  and fill in your Douban ID.")
        sys.exit(1)

    config = json.loads(CONFIG_PATH.read_text(encoding="utf-8"))

    if not config.get("douban_id"):
        print("ERROR: douban_id is empty in config.json.")
        print("  Set it to your Douban user ID (numeric or custom URL ID).")
        sys.exit(1)

    return config


# ─────────────────────────────────────────────────────────────────────────────
# Fetch
# ─────────────────────────────────────────────────────────────────────────────

def fetch_interests(uid: str, interest_type: str, statuses: list[str]) -> list[dict]:
    """Fetch all interests of a given type and statuses."""
    all_items: list[dict] = []

    for status in statuses:
        offset = 0
        while True:
            params = {
                "type": interest_type,
                "status": status,
                "start": offset,
                "count": PAGE_SIZE,
                "for_mobile": 1,
            }
            headers = {
                "Referer": f"https://m.douban.com/mine/{interest_type}",
            }

            resp_status, body = api_get(REXXAR_API.replace("{uid}", uid), params, headers)

            if resp_status == 403 or resp_status == 401:
                print(f"ERROR: HTTP {resp_status} — authentication required.")
                print("  Your Douban profile may not be public.")
                sys.exit(1)

            if resp_status != 200:
                print(f"ERROR: HTTP {resp_status} from Douban API")
                if body:
                    print(f"  Response: {json.dumps(body, ensure_ascii=False)[:200]}")
                sys.exit(1)

            total = body.get("total", 0)
            interests = body.get("interests", [])
            all_items.extend(interests)

            print(f"  [{interest_type}/{status}] Fetched {len(all_items)}/{total}...")

            offset += PAGE_SIZE
            if offset >= total:
                break
            time.sleep(REQUEST_INTERVAL)

    return all_items


# ─────────────────────────────────────────────────────────────────────────────
# Parse & classify
# ─────────────────────────────────────────────────────────────────────────────

def parse_card_subtitle(subtitle: str) -> dict:
    """Parse card_subtitle like '2024 / 日本 / 动画 科幻' into structured data."""
    result: dict = {"year": None, "regions": [], "genres": [], "raw": subtitle}
    if not subtitle:
        return result

    parts = [p.strip() for p in subtitle.split("/")]

    for part in parts:
        # Year detection
        year_match = re.match(r"^(\d{4})$", part.strip())
        if year_match:
            result["year"] = year_match.group(1)
            continue

        tokens = part.strip().split()
        if not tokens:
            continue

        # Heuristic: if all tokens look like country/region names (CJK or short),
        # and this is the second segment, it's likely regions.
        # Otherwise treat as genres.
        # In practice: parts[0]=year, parts[1]=regions, parts[2+]=genres
        if not result["regions"] and result["year"]:
            result["regions"] = tokens
        else:
            result["genres"].extend(tokens)

    return result


def is_japanese_anime(interest: dict) -> bool:
    """Check if an entry is Japanese anime (to be excluded)."""
    subject = interest.get("subject", {})
    genres = subject.get("genres", [])
    card_subtitle = subject.get("card_subtitle", "")

    is_animation = "动画" in genres
    # Check regions from card_subtitle (second segment after year)
    parts = [p.strip() for p in card_subtitle.split("/")]
    regions_part = parts[1] if len(parts) > 1 else ""
    is_japan = "日本" in regions_part

    return is_japan and is_animation


def convert_douban_rating(value) -> float | None:
    """Convert Douban personal rating (2-10, step 2) to 0-10 scale."""
    if value is None:
        return None
    # Douban uses 2/4/6/8/10 for 1-5 stars
    # Convert to 0-10: keep as-is since it's already on a 10-point scale
    return float(value)


# ─────────────────────────────────────────────────────────────────────────────
# Transform
# ─────────────────────────────────────────────────────────────────────────────

def map_interest(interest: dict, top_tags_count: int) -> dict:
    """Convert a Douban interest to gallery format."""
    subject = interest.get("subject", {})
    subject_id = subject.get("id")

    title = subject.get("title", "")

    pics = subject.get("pic") or {}
    cover = pics.get("normal")

    # Community rating
    rating_obj = subject.get("rating")
    rating = None
    if rating_obj and isinstance(rating_obj, dict):
        val = rating_obj.get("value")
        if val:
            rating = float(val)

    # Personal rating
    my_rating_obj = interest.get("rating")
    my_rating = None
    if my_rating_obj and isinstance(my_rating_obj, dict):
        my_rating = convert_douban_rating(my_rating_obj.get("value"))

    # Tags: prefer subject genres, fall back to personal tags
    genres = subject.get("genres", [])
    personal_tags = interest.get("tags", [])
    tags = genres if genres else personal_tags
    if tags and top_tags_count:
        tags = tags[:top_tags_count]

    # Date finished = create_time of marking as "done"
    create_time = interest.get("create_time", "")
    date_finished = create_time.split(" ")[0] if create_time else None

    subtitle = subject.get("card_subtitle", "")

    extra: dict = {
        "douban_id": subject_id,
        "douban_url": subject.get("url", ""),
    }
    if subtitle:
        extra["card_subtitle"] = subtitle

    comment = interest.get("comment", "")
    if comment:
        extra["comment"] = comment

    return {
        "name": title,
        "name_original": None,
        "cover": cover,
        "rating": rating,
        "my_rating": my_rating,
        "date_started": None,
        "date_finished": date_finished,
        "tags": tags,
        "episodes": None,
        "extra": extra,
    }


# ─────────────────────────────────────────────────────────────────────────────
# Main
# ─────────────────────────────────────────────────────────────────────────────

def write_gallery_json(path: Path, items: list[dict]):
    """Write gallery JSON file."""
    output = {"version": 1, "items": items}
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(output, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )


def main():
    parser = argparse.ArgumentParser(
        description="Fetch Douban movies, TV shows, and books for Arcana gallery."
    )
    parser.add_argument(
        "--status", default="done",
        choices=["done", "doing", "mark", "all"],
        help="Which status to fetch (default: done)",
    )
    args = parser.parse_args()

    if args.status == "all":
        statuses = ["done", "doing", "mark"]
    else:
        statuses = [args.status]

    start_time = time.time()
    config = load_config()
    uid = config["douban_id"]
    cookie = config.get("douban_cookie", "")
    top_tags_count = config.get("top_tags_count", 5)
    user_agent = config.get("user_agent", "Arcana/gallery-fetcher")

    _init_session(user_agent, cookie)

    # ── Fetch movies (includes both movies and TV) ──
    print(f"Fetching movie interests for Douban user: {uid}")
    movie_interests = fetch_interests(uid, "movie", statuses)

    movies: list[dict] = []
    tv_shows: list[dict] = []
    anime_skipped = 0

    for interest in movie_interests:
        # Filter out Japanese anime
        if is_japanese_anime(interest):
            anime_skipped += 1
            continue

        item = map_interest(interest, top_tags_count)

        # Use subject.subtype to distinguish movies from TV shows
        subject = interest.get("subject", {})
        subtype = subject.get("subtype", "movie")

        if subtype == "tv":
            tv_shows.append(item)
        else:
            movies.append(item)

    # ── Fetch books ──
    print(f"\nFetching book interests for Douban user: {uid}")
    book_interests = fetch_interests(uid, "book", statuses)

    books: list[dict] = []
    for interest in book_interests:
        item = map_interest(interest, top_tags_count)
        books.append(item)

    # ── Write output ──
    write_gallery_json(OUTPUT_DIR / "douban_movies.json", movies)
    write_gallery_json(OUTPUT_DIR / "douban_tv.json", tv_shows)
    write_gallery_json(OUTPUT_DIR / "douban_books.json", books)

    elapsed = time.time() - start_time

    print()
    print(f"Done! Time: {elapsed:.1f}s")
    print(f"  Movies:  {len(movies)} → {(OUTPUT_DIR / 'douban_movies.json').resolve()}")
    print(f"  TV:      {len(tv_shows)} → {(OUTPUT_DIR / 'douban_tv.json').resolve()}")
    print(f"  Books:   {len(books)} → {(OUTPUT_DIR / 'douban_books.json').resolve()}")
    if anime_skipped:
        print(f"  Skipped: {anime_skipped} Japanese anime entries (tracked via Bangumi)")


if __name__ == "__main__":
    main()
