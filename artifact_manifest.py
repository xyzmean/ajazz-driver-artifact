#!/usr/bin/env python3
"""Manifest tooling for the artifact pipeline.

  build <root> <out.json>      hash every file under <root> into a manifest
  diff  <old.json> <new.json>  compare two manifests, emit a markdown summary

`build` writes a manifest like:
  {"hash": "<sha256-of-table>", "count": N, "files": {"rel/path": "sha256", ...}}

`diff` prints a markdown report to stdout and, when running under GitHub
Actions, sets step outputs `changed`, `added`, `removed`, `modified` and writes
the report to $GITHUB_STEP_SUMMARY. Exit code is always 0; read `changed`.
"""
import hashlib
import json
import os
import sys


def sha256_file(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for block in iter(lambda: f.read(1 << 16), b""):
            h.update(block)
    return h.hexdigest()


def build_manifest(root):
    files = {}
    for dirpath, _, filenames in os.walk(root):
        for name in filenames:
            full = os.path.join(dirpath, name)
            rel = os.path.relpath(full, root).replace(os.sep, "/")
            files[rel] = sha256_file(full)
    table = json.dumps(files, sort_keys=True, separators=(",", ":"))
    return {
        "hash": hashlib.sha256(table.encode()).hexdigest(),
        "count": len(files),
        "files": dict(sorted(files.items())),
    }


def load_files(path):
    if not path or not os.path.exists(path):
        return {}
    with open(path, encoding="utf-8") as f:
        data = json.load(f)
    return data.get("files", {})


def set_output(key, value):
    out = os.environ.get("GITHUB_OUTPUT")
    if not out:
        return
    with open(out, "a", encoding="utf-8") as f:
        f.write(f"{key}={value}\n")


def write_summary(text):
    path = os.environ.get("GITHUB_STEP_SUMMARY")
    if not path:
        return
    with open(path, "a", encoding="utf-8") as f:
        f.write(text + "\n")


def cmd_build(root, out):
    manifest = build_manifest(root)
    with open(out, "w", encoding="utf-8") as f:
        json.dump(manifest, f, ensure_ascii=False, indent=2)
        f.write("\n")
    print(f"manifest: {manifest['count']} files, hash {manifest['hash'][:12]}")


def cmd_diff(old_path, new_path):
    old = load_files(old_path)
    new = load_files(new_path)

    added = sorted(k for k in new if k not in old)
    removed = sorted(k for k in old if k not in new)
    modified = sorted(k for k in new if k in old and new[k] != old[k])
    changed = bool(added or removed or modified)

    def section(title, items):
        if not items:
            return ""
        lines = "\n".join(f"- `{i}`" for i in items)
        return f"\n**{title} ({len(items)})**\n{lines}\n"

    if not changed:
        report = "No artifact changes since the last snapshot."
    else:
        report = (
            f"Artifact changed: **+{len(added)} / -{len(removed)} / "
            f"~{len(modified)}** files."
            + section("Added", added)
            + section("Removed", removed)
            + section("Modified", modified)
        )

    print(report)
    write_summary(report)
    with open("manifest_diff.md", "w", encoding="utf-8") as f:
        f.write(report + "\n")
    set_output("changed", "true" if changed else "false")
    set_output("added", str(len(added)))
    set_output("removed", str(len(removed)))
    set_output("modified", str(len(modified)))


def main(argv):
    if len(argv) >= 4 and argv[1] == "build":
        cmd_build(argv[2], argv[3])
    elif len(argv) >= 4 and argv[1] == "diff":
        cmd_diff(argv[2], argv[3])
    else:
        sys.exit("usage: artifact_manifest.py build <root> <out.json>\n"
                 "       artifact_manifest.py diff <old.json> <new.json>")


if __name__ == "__main__":
    main(sys.argv)
