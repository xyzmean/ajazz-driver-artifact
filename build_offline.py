#!/usr/bin/env python3
"""Scrape the upstream Ajazz WebHID app into a self-contained ./app snapshot.

This is the source of truth for the `artifact` pipeline. Unlike a fixed
download list, asset discovery is a **fixpoint crawl**: every downloaded JS/CSS
file is rescanned for further `assets/*` references and `import("./...")` chunks
until nothing new appears — so chunks added upstream are picked up automatically.

Outputs ./app (index.html + assets + langs + cached images), with the upstream
CDN URLs in the layout bundle patched to local relative paths.
"""
import os
import re
import sys
import urllib.request

BASE_URL = "https://ajazz.driveall.cn"
CONFIG_CDN = "https://config.driveall.cn"
STATIC_CDN = "https://static.driveall.cn"

APP_DIR = "app"
ASSETS_DIR = os.path.join(APP_DIR, "assets")
LANGS_DIR = os.path.join(APP_DIR, "langs")
CACHE_DIR = os.path.join(APP_DIR, "cache")

# Asset extensions we follow when crawling /assets/. Anything Vite emits as a
# hashed chunk lands here; broadening this list is how new file *types* survive.
ASSET_EXT = r"js|css|mjs|woff2?|ttf|otf|eot|png|jpe?g|gif|svg|webp|json|map"
ASSET_REF_RE = re.compile(rf'assets/([A-Za-z0-9_.\-]+\.(?:{ASSET_EXT}))')
DYNIMPORT_RE = re.compile(r'import\(\s*["\']\./([^"\']+)["\']\s*\)')

os.makedirs(ASSETS_DIR, exist_ok=True)
os.makedirs(LANGS_DIR, exist_ok=True)
os.makedirs(CACHE_DIR, exist_ok=True)


def request_url(url):
    req = urllib.request.Request(
        url, headers={"User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64)"}
    )
    with urllib.request.urlopen(req, timeout=30) as response:
        return response.read()


def download_file(url, local_path):
    os.makedirs(os.path.dirname(local_path), exist_ok=True)
    try:
        data = request_url(url)
        with open(local_path, "wb") as f:
            f.write(data)
        print(f"  [OK] {os.path.relpath(local_path, APP_DIR)}")
        return True
    except Exception as e:
        print(f"  [FAIL] {url} -> {e}")
        return False


def is_text_asset(name):
    return name.endswith((".js", ".mjs", ".css", ".map", ".json"))


# 1. Entry point ------------------------------------------------------------
print("1. Scraping entrypoint index.html ...")
try:
    html_str = request_url(BASE_URL + "/").decode("utf-8")
    with open(os.path.join(APP_DIR, "index.html"), "w", encoding="utf-8") as f:
        f.write(html_str)
    print("  [OK] index.html")
except Exception as e:
    sys.exit(f"Fatal: cannot fetch index.html: {e}")

# 2. Seed the crawl from every asset referenced by index.html ---------------
print("\n2. Seeding asset crawl from index.html ...")
seed = set(re.findall(r'(?:href|src)="/assets/([^"]+)"', html_str))
# Also catch bare assets/<name> references inside inlined scripts.
seed |= {m for m in ASSET_REF_RE.findall(html_str)}
print(f"  {len(seed)} seed asset(s)")

# 3. Fixpoint crawl: download, rescan text assets, repeat -------------------
print("\n3. Fixpoint asset crawl (picks up newly added chunks) ...")
downloaded = set()
queue = set(seed)
rounds = 0
while queue:
    rounds += 1
    batch = sorted(queue)
    queue = set()
    for name in batch:
        if name in downloaded:
            continue
        downloaded.add(name)
        local = os.path.join(ASSETS_DIR, name)
        if not download_file(f"{BASE_URL}/assets/{name}", local):
            continue
        if not is_text_asset(name):
            continue
        try:
            with open(local, "r", encoding="utf-8", errors="ignore") as f:
                content = f.read()
        except Exception:
            continue
        for ref in ASSET_REF_RE.findall(content):
            if ref not in downloaded:
                queue.add(ref)
        for imp in DYNIMPORT_RE.findall(content):
            ref = imp.split("/")[-1]
            if ref not in downloaded:
                queue.add(ref)
print(f"  Converged after {rounds} round(s): {len(downloaded)} asset file(s).")

# Locate the layout-default chunk (holds model table + CDN URL helpers).
layout_files = sorted(
    os.path.join(ASSETS_DIR, n)
    for n in downloaded
    if re.match(r"layout-default-.*\.js$", n)
)
layout_file = layout_files[0] if layout_files else None
print(f"  layout-default: {layout_file}")
layout_content = ""
if layout_file and os.path.exists(layout_file):
    with open(layout_file, "r", encoding="utf-8") as f:
        layout_content = f.read()

# 4. Localized language packs ----------------------------------------------
# Discover langs from the bundle when possible, fall back to the known set so
# newly added locales are still picked up.
print("\n4. Downloading language packs ...")
KNOWN_LANGS = ["ar", "de", "en", "es", "fr", "id", "it", "ja", "ko",
               "pt-BR", "pt", "ru", "th", "vi", "zh-TW", "zh"]
discovered_langs = set(re.findall(r'langs/([a-zA-Z\-]+)\.json', layout_content))
langs = sorted(set(KNOWN_LANGS) | discovered_langs)
print(f"  {len(langs)} locale(s): {', '.join(langs)}")
for lang in langs:
    download_file(f"{CONFIG_CDN}/langs/{lang}.json",
                  os.path.join(LANGS_DIR, f"{lang}.json"))

# 5. Keyboard render images -------------------------------------------------
print("\n5. Pre-caching keyboard images ...")
kb_images = sorted(set(re.findall(r'keyboardImg:\s*"([^"]+)"', layout_content)))
print(f"  {len(kb_images)} model image(s)")
for img in kb_images:
    download_file(f"{STATIC_CDN}/static/keyboards/{img}",
                  os.path.join(CACHE_DIR, "static/keyboards", img))

# 6. Standalone brand / switch graphics -------------------------------------
print("\n6. Downloading brand logos and switch graphics ...")
download_file(f"{CONFIG_CDN}/img/key_switch.png",
              os.path.join(CACHE_DIR, "config/img/key_switch.png"))
download_file(f"{STATIC_CDN}/static/keyboards/kb_bg.png",
              os.path.join(CACHE_DIR, "static/keyboards/kb_bg.png"))
download_file(f"{CONFIG_CDN}/logo/ajazz/logo.png",
              os.path.join(CACHE_DIR, "config/logo/ajazz/logo.png"))

# 7. Patch CDN URLs in the layout bundle to local relative paths ------------
print("\n7. Patching layout bundle for offline relative routing ...")
if layout_file and os.path.exists(layout_file):
    js = layout_content
    js = re.sub(
        r'function xl\s*\(\s*s\s*\)\s*\{\s*return\s*[`\']https://config\.driveall\.cn\$\{s\.startsWith\(\s*["\']\/["\']\s*\)\s*\?\s*s\s*:\s*["\']\/["\']\s*\+\s*s\}\s*[`\']\s*\}',
        'function xl(s){return`./cache/config${s.startsWith("/")?s:"/"+s}`}', js)
    js = re.sub(
        r'function ol\s*\(\s*s\s*\)\s*\{\s*return\s*[`\']https://static\.driveall\.cn/static/\$\{s\.startsWith\(\s*["\']\/["\']\s*\)\s*\?\s*s\s*:\s*["\']\/["\']\s*\+\s*s\}\s*[`\']\s*\}',
        'function ol(s){return`./cache/static${s.startsWith("/")?s:"/"+s}`}', js)
    js = re.sub(
        r'function fl\s*\(\s*s\s*\)\s*\{\s*return\s*[`\']https://config\.driveall\.cn\$\{s\.startsWith\(\s*["\']\/["\']\s*\)\s*\?\s*s\s*:\s*["\']\/["\']\s*\+\s*s\}\s*[`\']\s*\}',
        'function fl(s){return`./cache/config${s.startsWith("/")?s:"/"+s}`}', js)
    js = js.replace("https://static.driveall.cn/static/keyboards/", "./cache/static/keyboards/")
    js = js.replace("https://config.driveall.cn/logo/ajazz/logo.png", "./cache/config/logo/ajazz/logo.png")
    with open(layout_file, "w", encoding="utf-8") as f:
        f.write(js)
    print("  [OK] patched relative pathways")
else:
    print("  [WARN] layout-default not found; skipped URL patching")

print("\nOffline build generation completed.")
