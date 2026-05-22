import os
import json
import urllib.request
import urllib.parse

# Create a separate folder for these reverse engineering materials as requested by the user
OUTPUT_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "ajazz-hub-configs")
os.makedirs(OUTPUT_DIR, exist_ok=True)

models = [
    "AJazz-AK980",
    "AK980",
    "AK35I",
    "AK820",
    "AK820 MAX",
    "AK980 PRO",
    "AK35I PRO",
    "AK820 PRO",
    "AK980_PRO",
    "AK820_MAX"
]

print("=== Querying ajazz-hub.com API for device configurations ===")

def fetch_config(model_name):
    query = urllib.parse.urlencode({"device_name": model_name})
    url = f"https://www.ajazz-hub.com/api/device/configuration?{query}"
    print(f"Fetching: {url}")
    
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
        print(f"  Error fetching {model_name}: {e}")
        return None

for m in models:
    res = fetch_config(m)
    if res:
        # Save to the isolated output folder
        safe_name = m.replace(" ", "_").replace("-", "_").lower()
        file_path = os.path.join(OUTPUT_DIR, f"{safe_name}.json")
        with open(file_path, "w", encoding="utf-8") as f:
            json.dump(res, f, ensure_ascii=False, indent=2)
        print(f"  Saved to {file_path} (Code: {res.get('code')}, Message: {res.get('message')})")

print("Fetch completed.")
