#!/usr/bin/env python3
"""Re-reverse the Ajazz protocol bundle with Vertex AI (Gemini).

When the upstream *core* bundle changes, the deterministic model-table
extractor (`extract_models.py`) is not enough — the hand-written protocol
(`core.ts` / `commands.ts`) was reverse-engineered from minified JS and must be
re-derived. This sends Gemini the OLD minified bundle + the OLD hand-written TS
(the ground-truth mapping) + the NEW minified bundle, and asks for an updated
protocol plus a change report. The output is written into a `reverse` worktree
for a human-reviewed PR; it is never merged blindly.

Env:
  VERTEX_PROJECT, VERTEX_REGION   GCP project / location
  VERTEX_MODEL                    e.g. gemini-3.5-flash
  REVERSE_DIR                     path to the reverse worktree to write into
  OLD_MIN, NEW_MIN                paths to old/new minified core bundles
Auth: Application Default Credentials (GOOGLE_APPLICATION_CREDENTIALS), set by
google-github-actions/auth in CI.

Outputs (under REVERSE_DIR): updated src/protocol/core.ts, src/protocol/commands.ts,
index.core.min.js (= NEW_MIN), and vertex_report.md. Prints PROTOCOL_CHANGED=...
to stdout and to $GITHUB_OUTPUT for the workflow to gate the PR.
"""
import json
import os
import sys

import vertexai
from vertexai.generative_models import GenerationConfig, GenerativeModel, Part


def read(path):
    with open(path, "r", encoding="utf-8", errors="ignore") as f:
        return f.read()


def write(path, text):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8") as f:
        f.write(text if text.endswith("\n") else text + "\n")


def set_output(key, value):
    out = os.environ.get("GITHUB_OUTPUT")
    if out:
        with open(out, "a", encoding="utf-8") as f:
            f.write(f"{key}={value}\n")


SYSTEM = """You reverse-engineer minified Vite/JS bundles into a clean, typed \
TypeScript protocol layer for the Ajazz keyboard WebHID driver.

You are given, for an EARLIER upstream build:
  - OLD_MINIFIED: the minified core bundle
  - OLD_CORE_TS and OLD_COMMANDS_TS: the human-maintained TypeScript port of it
    (the ground-truth mapping from minified symbols to meaning)
and for the CURRENT upstream build:
  - NEW_MINIFIED: the new minified core bundle (symbol names will differ).

Task: produce the UPDATED core.ts and commands.ts that correctly describe
NEW_MINIFIED, by re-deriving the symbol mapping the same way the OLD pair shows.

Rules:
  - Preserve the existing file structure, exported names, comments style, and
    public API of the OLD TS unless the new bundle genuinely changed semantics.
  - Only change what the new bundle requires (new/removed/renamed commands,
    changed opcodes, changed framing constants, changed parse offsets).
  - Keep it compiling TypeScript; do not invent APIs you cannot justify from the
    bundle. If unsure about a detail, keep the OLD behavior and note it.
  - protocol_changed = true ONLY if the wire protocol / command set / parsing
    actually changed; false if the bundle was merely re-minified with the same
    semantics (in that case return the OLD TS unchanged).
  - report: concise Markdown — what changed at the protocol level and your
    confidence, plus anything a human must verify against hardware."""


def main():
    project = os.environ["VERTEX_PROJECT"]
    region = os.environ.get("VERTEX_REGION") or "us-central1"
    model_id = os.environ.get("VERTEX_MODEL") or "gemini-3.5-flash"
    reverse_dir = os.environ["REVERSE_DIR"]
    old_min_path = os.environ["OLD_MIN"]
    new_min_path = os.environ["NEW_MIN"]

    old_min = read(old_min_path)
    new_min = read(new_min_path)
    old_core = read(os.path.join(reverse_dir, "src/protocol/core.ts"))
    old_commands = read(os.path.join(reverse_dir, "src/protocol/commands.ts"))

    vertexai.init(project=project, location=region)
    model = GenerativeModel(model_id, system_instruction=SYSTEM)

    prompt = "\n\n".join([
        "=== OLD_MINIFIED ===", old_min,
        "=== OLD_CORE_TS ===", old_core,
        "=== OLD_COMMANDS_TS ===", old_commands,
        "=== NEW_MINIFIED ===", new_min,
        "Return the updated core.ts, commands.ts, a report, and protocol_changed.",
    ])

    config = GenerationConfig(
        temperature=0.1,
        response_mime_type="application/json",
        response_schema={
            "type": "object",
            "properties": {
                "protocol_changed": {"type": "boolean"},
                "core_ts": {"type": "string"},
                "commands_ts": {"type": "string"},
                "report": {"type": "string"},
            },
            "required": ["protocol_changed", "core_ts", "commands_ts", "report"],
        },
    )

    resp = model.generate_content([Part.from_text(prompt)], generation_config=config)
    data = json.loads(resp.text)

    changed = bool(data.get("protocol_changed"))
    report = data.get("report", "").strip() or "(no report returned)"

    # Always refresh the committed bundle baseline to the new core.
    write(os.path.join(reverse_dir, "index.core.min.js"), new_min)

    if changed:
        write(os.path.join(reverse_dir, "src/protocol/core.ts"), data["core_ts"])
        write(os.path.join(reverse_dir, "src/protocol/commands.ts"), data["commands_ts"])

    header = (
        f"# Vertex AI protocol re-derivation\n\n"
        f"- model: `{model_id}`\n"
        f"- protocol_changed: **{changed}**\n\n"
        f"> Auto-generated by Vertex AI. **Review carefully** before merging — "
        f"this is a proposal, not verified against hardware.\n\n"
    )
    write(os.path.join(reverse_dir, "vertex_report.md"), header + report)

    print(f"PROTOCOL_CHANGED={'true' if changed else 'false'}")
    set_output("protocol_changed", "true" if changed else "false")


if __name__ == "__main__":
    try:
        main()
    except Exception as e:  # surface a clear CI error
        print(f"::error::Vertex reverse failed: {e}", file=sys.stderr)
        sys.exit(1)
