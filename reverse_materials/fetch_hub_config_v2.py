import os
import json
import urllib.request
import urllib.parse

OUTPUT_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "ajazz-hub-configs")
os.makedirs(OUTPUT_DIR, exist_ok=True)

models = [
    "ak980-led",
    "ak980-led-v2",
    "ak820-max",
    "ak35i"
]

print("=== Querying ajazz-hub.com API with verified device names ===")

def fetch_config(model_name):
    query = urllib.parse.urlencode({"device_name": model_name})
    url = f"https://www.ajazz-hub.com/api/device/configuration?{query}"
    
    req = urllib.request.Request(
        url,
        headers={
            "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64)",
            "Accept": "application/json"
        }
    )
    try:
        with urllib.request.urlopen(req, timeout=10) as response:
            data = json.loads(response.read().decode("utf-8"))
            return data
    except Exception as e:
        return {"error": str(e)}

for m in models:
    res = fetch_config(m)
    if res:
        safe_name = m.replace(" ", "_").replace("-", "_").lower()
        file_path = os.path.join(OUTPUT_DIR, f"{safe_name}.json")
        with open(file_path, "w", encoding="utf-8") as f:
            json.dump(res, f, ensure_ascii=False, indent=2)
        
        # Safe ASCII prints to avoid windows console crash
        code = res.get("code", "unknown")
        has_data = "yes" if res.get("data") is not None else "no"
        print(f"  Model: {m} -> Saved to {file_path} (Code: {code}, Has Data: {has_data})")

print("Fetch completed successfully.")
