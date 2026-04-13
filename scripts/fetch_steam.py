"""
Fetch owned Steam games and generate gallery JSON.

Reads config from scripts/config.json, queries the Steam Web API
for owned games, and writes the result to data/gallery/steam_games.json.

Usage:
    python scripts/fetch_steam.py              # fast mode (owned games only)
    python scripts/fetch_steam.py --detailed   # also fetch achievements, genres, metacritic
"""

import argparse
import json
import sys
import time
from pathlib import Path

SCRIPT_DIR = Path(__file__).parent
CONFIG_PATH = SCRIPT_DIR / "config.json"
OUTPUT_PATH = SCRIPT_DIR / ".." / "data" / "gallery" / "steam_games.json"
CACHE_PATH = SCRIPT_DIR / ".steam_cache.json"

STEAM_API_BASE = "https://api.steampowered.com"
STORE_API_BASE = "https://store.steampowered.com/api"
STEAM_CDN = "https://cdn.akamai.steamstatic.com/steam/apps"
STEAM_STORE_URL = "https://store.steampowered.com/app/{}"

REQUEST_INTERVAL = 1.5  # seconds between store/achievement API calls
MAX_RETRIES = 3
RETRY_WAIT = 5


# ─────────────────────────────────────────────────────────────────────────────
# HTTP client — prefer requests, fallback to urllib
# ─────────────────────────────────────────────────────────────────────────────

_session = None
_user_agent = "Arcana/gallery-fetcher"

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


def api_get(url: str, params: dict | None = None) -> tuple[int, dict | list | None]:
    """GET with retry on 429."""
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
        return status, body
    return status, body  # unreachable


# ─────────────────────────────────────────────────────────────────────────────
# Config
# ─────────────────────────────────────────────────────────────────────────────

def load_config() -> dict:
    if not CONFIG_PATH.exists():
        print(f"ERROR: Config file not found: {CONFIG_PATH}")
        print(f"  Copy scripts/config.example.json to scripts/config.json")
        print(f"  and fill in your Steam API key and Steam ID.")
        sys.exit(1)

    config = json.loads(CONFIG_PATH.read_text(encoding="utf-8"))

    if not config.get("steam_api_key"):
        print("ERROR: steam_api_key is empty in config.json.")
        print("  Get one from https://steamcommunity.com/dev/apikey")
        sys.exit(1)

    if not config.get("steam_id"):
        print("ERROR: steam_id is empty in config.json.")
        print("  Your 64-bit Steam ID (numeric).")
        sys.exit(1)

    return config


# ─────────────────────────────────────────────────────────────────────────────
# Cache (for --detailed mode)
# ─────────────────────────────────────────────────────────────────────────────

def load_cache() -> dict:
    if CACHE_PATH.exists():
        try:
            return json.loads(CACHE_PATH.read_text(encoding="utf-8"))
        except (json.JSONDecodeError, OSError):
            pass
    return {}


def save_cache(cache: dict):
    CACHE_PATH.write_text(
        json.dumps(cache, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )


# ─────────────────────────────────────────────────────────────────────────────
# Fetch
# ─────────────────────────────────────────────────────────────────────────────

def fetch_owned_games(api_key: str, steam_id: str) -> list[dict]:
    """Fetch all owned games via IPlayerService/GetOwnedGames."""
    url = f"{STEAM_API_BASE}/IPlayerService/GetOwnedGames/v1/"
    params = {
        "key": api_key,
        "steamid": steam_id,
        "include_appinfo": 1,
        "include_played_free_games": 1,
        "format": "json",
    }
    status, body = api_get(url, params)

    if status == 403:
        print("ERROR: HTTP 403 — invalid Steam API key.")
        print("  Get a valid key from https://steamcommunity.com/dev/apikey")
        sys.exit(1)

    if status != 200:
        print(f"ERROR: HTTP {status} from GetOwnedGames")
        sys.exit(1)

    response = (body or {}).get("response", {})
    games = response.get("games", [])

    if not games:
        print("WARNING: No games returned.")
        print("  Ensure your Steam profile game details are set to Public:")
        print("  Steam > Profile > Edit Profile > Privacy Settings > Game details: Public")

    game_count = response.get("game_count", len(games))
    print(f"  Found {game_count} owned games.")
    return games


def fetch_achievements(api_key: str, steam_id: str, appid: int) -> dict | None:
    """Fetch achievement stats for a single game. Returns {unlocked, total} or None."""
    url = f"{STEAM_API_BASE}/ISteamUserStats/GetPlayerAchievements/v1/"
    params = {
        "key": api_key,
        "steamid": steam_id,
        "appid": appid,
    }
    status, body = api_get(url, params)

    # 400 = game has no achievements; other errors = skip silently
    if status != 200 or body is None:
        return None

    stats = (body or {}).get("playerstats", {})
    if not stats.get("success"):
        return None

    achievements = stats.get("achievements", [])
    if not achievements:
        return None

    total = len(achievements)
    unlocked = sum(1 for a in achievements if a.get("achieved"))
    return {"unlocked": unlocked, "total": total}


def fetch_store_details(appid: int) -> dict | None:
    """Fetch store details (genres, metacritic, release_date, type)."""
    url = f"{STORE_API_BASE}/appdetails"
    params = {"appids": str(appid)}
    status, body = api_get(url, params)

    if status != 200 or body is None:
        return None

    app_data = body.get(str(appid), {})
    if not app_data.get("success"):
        return None

    data = app_data.get("data", {})
    genres = [g["description"] for g in data.get("genres", [])]
    metacritic = data.get("metacritic", {}).get("score")
    release_date = data.get("release_date", {}).get("date")
    app_type = data.get("type", "game")

    return {
        "genres": genres,
        "metacritic": metacritic,
        "release_date": release_date,
        "type": app_type,
    }


def fetch_detailed(api_key: str, steam_id: str, games: list[dict], cache: dict) -> dict:
    """Fetch achievements + store details for each game, updating cache."""
    total = len(games)
    skipped = 0

    for idx, game in enumerate(games):
        appid = game["appid"]
        key = str(appid)
        name = game.get("name", f"App {appid}")

        if key in cache:
            print(f"  [{idx + 1}/{total}] Cached: {name}")
            continue

        print(f"  [{idx + 1}/{total}] Fetching: {name}")

        entry: dict = {}

        # Store details
        store = fetch_store_details(appid)
        if store:
            entry["genres"] = store["genres"]
            entry["metacritic"] = store["metacritic"]
            entry["release_date"] = store["release_date"]
            entry["type"] = store["type"]

            # Skip non-game items
            if store["type"] not in ("game",):
                entry["skip"] = True
                skipped += 1
        else:
            entry["genres"] = []
            entry["metacritic"] = None
            entry["release_date"] = None
            entry["type"] = None

        time.sleep(REQUEST_INTERVAL)

        # Achievements
        ach = fetch_achievements(api_key, steam_id, appid)
        if ach:
            entry["achievement_unlocked"] = ach["unlocked"]
            entry["achievement_total"] = ach["total"]

        time.sleep(REQUEST_INTERVAL)

        cache[key] = entry

        # Save cache periodically (every 20 games)
        if (idx + 1) % 20 == 0:
            save_cache(cache)

    if skipped:
        print(f"  Marked {skipped} non-game items for exclusion (DLC/demo/tool).")

    return cache


# ─────────────────────────────────────────────────────────────────────────────
# Transform
# ─────────────────────────────────────────────────────────────────────────────

def map_game(game: dict, details: dict | None, top_tags_count: int) -> dict | None:
    """Convert a Steam game to gallery format."""
    appid = game["appid"]
    name = game.get("name", f"App {appid}")

    # Skip non-game items in detailed mode
    if details and details.get("skip"):
        return None

    playtime_minutes = game.get("playtime_forever", 0)
    playtime_hours = round(playtime_minutes / 60, 1)

    cover = f"{STEAM_CDN}/{appid}/library_600x900.jpg"

    rating = None
    tags: list[str] = []
    release_date = None

    if details:
        tags = details.get("genres", [])[:top_tags_count]
        release_date = details.get("release_date")
        metacritic = details.get("metacritic")
        if metacritic is not None:
            rating = round(metacritic / 10.0, 1)

    extra: dict = {
        "steam_appid": appid,
        "steam_url": STEAM_STORE_URL.format(appid),
        "playtime_hours": playtime_hours,
    }

    if details:
        if "achievement_unlocked" in details:
            extra["achievement_unlocked"] = details["achievement_unlocked"]
            extra["achievement_total"] = details["achievement_total"]
        if release_date:
            extra["release_date"] = release_date

    return {
        "name": name,
        "name_original": None,
        "cover": cover,
        "rating": rating,
        "my_rating": None,
        "date_started": None,
        "date_finished": None,
        "tags": tags,
        "episodes": None,
        "extra": extra,
    }


# ─────────────────────────────────────────────────────────────────────────────
# Main
# ─────────────────────────────────────────────────────────────────────────────

def main():
    parser = argparse.ArgumentParser(
        description="Fetch Steam games for Arcana gallery."
    )
    parser.add_argument(
        "--detailed", action="store_true",
        help="Also fetch achievements, genres, and metacritic per game (slow)",
    )
    args = parser.parse_args()

    start_time = time.time()
    config = load_config()
    api_key = config["steam_api_key"]
    steam_id = config["steam_id"]
    top_tags_count = config.get("top_tags_count", 5)
    user_agent = config.get("user_agent", "Arcana/gallery-fetcher")

    _init_session(user_agent)

    print(f"Fetching owned games for Steam ID: {steam_id}")
    games = fetch_owned_games(api_key, steam_id)

    if not games:
        print("No games to process. Exiting.")
        sys.exit(0)

    # Sort by playtime descending
    games.sort(key=lambda g: g.get("playtime_forever", 0), reverse=True)

    cache: dict = {}
    if args.detailed:
        print(f"\nDetailed mode: fetching achievements + store data for {len(games)} games...")
        cache = load_cache()
        cache = fetch_detailed(api_key, steam_id, games, cache)
        save_cache(cache)

    warnings: list[str] = []
    gallery_items: list[dict] = []
    for game in games:
        details = cache.get(str(game["appid"])) if args.detailed else None
        item = map_game(game, details, top_tags_count)
        if item is None:
            warnings.append(f"Skipped non-game: {game.get('name', game['appid'])}")
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
    print(f"Done! {len(gallery_items)} games written to {OUTPUT_PATH.resolve()}")
    print(f"Time: {elapsed:.1f}s")
    if warnings:
        print(f"\nWarnings ({len(warnings)}):")
        for w in warnings:
            print(f"  - {w}")


if __name__ == "__main__":
    main()
