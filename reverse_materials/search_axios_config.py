import os
import re

dir_path = "hub_download"
js_files = [f for f in os.listdir(dir_path) if f.endswith(".js")]

print("=== Scanning for Axios baseURL, create, interceptors ===")

for fn in js_files:
    path = os.path.join(dir_path, fn)
    with open(path, "r", encoding="utf-8", errors="ignore") as f:
        content = f.read()

    # Search for create, baseURL, interceptors
    for keyword in ["baseURL", "interceptors", "headers", "token", "response", "request"]:
        matches = list(re.finditer(re.escape(keyword), content))
        if len(matches) > 0 and fn != "ui-vendor-DSHZq0I1.js" and fn != "vue-vendor-S9ArMmYT.js":
            print(f"[{fn}] Found '{keyword}' {len(matches)} times")
            # Print a few matches contexts for baseURL or interceptors
            if keyword in ["baseURL", "interceptors"]:
                for m in matches[:5]:
                    start = max(0, m.start() - 100)
                    end = min(len(content), m.end() + 100)
                    print(f"  Context around {keyword} at {m.start()}:\n  {content[start:end]}")
