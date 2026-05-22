#!/usr/bin/env python3
"""Scrape the upstream Ajazz Hub app into a self-contained ./app-hub snapshot.

Specifically tailored for https://www.ajazz-hub.com/ which uses a different
structure (/js/ index modules and inlined layout definitions) than ajazz.driveall.cn.
"""
import os
import re
import sys
import urllib.request

BASE_URL = "https://www.ajazz-hub.com"
APP_DIR = "app-hub"
JS_DIR = os.path.join(APP_DIR, "js")
ASSETS_DIR = os.path.join(APP_DIR, "assets")

os.makedirs(JS_DIR, exist_ok=True)
os.makedirs(ASSETS_DIR, exist_ok=True)

# Asset extensions we follow when crawling
ASSET_EXT = r"js|css|mjs|woff2?|ttf|otf|eot|png|jpe?g|gif|svg|webp|json|map"
ASSET_REF_RE = re.compile(rf'([A-Za-z0-9_.\-/]+\.(?:{ASSET_EXT}))')

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

# 1. Entry point ------------------------------------------------------------
print("1. Scraping entrypoint index.html from ajazz-hub.com ...")
try:
    html_str = request_url(BASE_URL + "/").decode("utf-8")
    with open(os.path.join(APP_DIR, "index.html"), "w", encoding="utf-8") as f:
        f.write(html_str)
    print("  [OK] index.html")
except Exception as e:
    sys.exit(f"Fatal: cannot fetch index.html: {e}")

# 2. Extract scripts and link paths from index.html --------------------------
print("\n2. Discovering assets from index.html ...")
assets = []

# Find modulepreloads: href="/js/vue-vendor-*.js" or link stylesheets: href="/assets/index-*.css"
hrefs = re.findall(r'href="([^"]+)"', html_str)
# Find script srcs: src="/js/index-*.js"
srcs = re.findall(r'src="([^"]+)"', html_str)

all_links = sorted(set(hrefs + srcs))
seed = []
for link in all_links:
    if link.startswith("/") and not link.startswith("//"):
        seed.append(link.lstrip("/"))
    elif link.startswith(BASE_URL):
        seed.append(link[len(BASE_URL):].lstrip("/"))

print(f"  Found {len(seed)} seed asset(s): {seed}")

# 3. Download assets recursively --------------------------------------------
print("\n3. Downloading seed assets ...")
downloaded = set()
for path in seed:
    local = os.path.join(APP_DIR, path)
    url = f"{BASE_URL}/{path}"
    if download_file(url, local):
        downloaded.add(path)

# Let's inspect the downloaded text files to check if there are other files being referenced
print("\n4. Crawling downloaded JS/CSS for other assets ...")
text_files = [p for p in downloaded if p.endswith((".js", ".css"))]
queue = set()

for path in text_files:
    local_path = os.path.join(APP_DIR, path)
    try:
        content = open(local_path, "r", encoding="utf-8", errors="ignore").read()
        # Find any other relative paths like logo.svg or dynamic assets
        # Match strings like "/js/..." or "/assets/..." or "assets/..." or "js/..."
        for p in re.findall(r'"/((?:js|assets)/[A-Za-z0-9_.\-]+\.(?:js|css|png|svg|webp|json))"', content):
            queue.add(p)
        for p in re.findall(r'\'/((?:js|assets)/[A-Za-z0-9_.\-]+\.(?:js|css|png|svg|webp|json))\'', content):
            queue.add(p)
    except Exception as e:
        print(f"  Error reading {path}: {e}")

if queue:
    print(f"  Found {len(queue)} additional asset(s) in JS/CSS:")
    for path in sorted(queue):
        if path not in downloaded:
            local = os.path.join(APP_DIR, path)
            url = f"{BASE_URL}/{path}"
            if download_file(url, local):
                downloaded.add(path)

print(f"\nAjazz Hub offline build completed: {len(downloaded)} file(s) downloaded.")
