import os
import re

path = "hub_download/keyboard-Bl_mDn24.js"
with open(path, "r", encoding="utf-8") as f:
    content = f.read()

# Search for the context around "KeyboardOpen" component definition
print("=== KeyboardOpen context ===")
for m in re.finditer(r'KeyboardOpen', content):
    start = max(0, m.start() - 100)
    end = min(len(content), m.end() + 400)
    clean = content[start:end].encode("ascii", "ignore").decode("ascii")
    print(f"Match at {m.start()}:\n{clean}\n{'-'*50}")
