#!/usr/bin/env python3
"""Exports archbench/<suite>/suite.json + referenced decklists into single-file JSON bundles for
browser WASM workers and Live Watch (`<out>/suites/<suite>.json`)."""
import argparse
import json
import os
import re


def parse_deck(text):
    cards = []
    for raw in text.splitlines():
        line = raw.strip()
        if not line or line.startswith("#") or line.startswith("//") or line.upper().startswith("SB:"):
            continue
        m = re.match(r"^(\d+)\s+(.+)$", line)
        if m:
            cards.append([int(m.group(1)), m.group(2).strip()])
    return cards


def export_suites(repo, out_dir):
    suites_out = os.path.join(out_dir, "suites")
    os.makedirs(suites_out, exist_ok=True)
    arch = os.path.join(repo, "archbench")
    count = 0
    for name in sorted(os.listdir(arch)):
        sjson = os.path.join(arch, name, "suite.json")
        if not os.path.isfile(sjson):
            continue
        suite = json.load(open(sjson, encoding="utf-8"))
        decks = []
        for entry in suite.get("decks", []):
            rel = entry["file"]
            full = os.path.join(repo, rel)
            if not os.path.isfile(full):
                continue
            deck_name = os.path.splitext(os.path.basename(rel))[0]
            decks.append({"name": deck_name, "cards": parse_deck(open(full, encoding="utf-8").read())})
        if len(decks) < 2:
            continue
        payload = {
            "suite": name,
            "turn_cap": int(suite.get("turns", 30)),
            "mulligans": bool(suite.get("mulligans", True)),
            "decks": decks,
        }
        dst = os.path.join(suites_out, f"{name}.json")
        with open(dst, "w", encoding="utf-8") as f:
            json.dump(payload, f, separators=(",", ":"))
        count += 1
        print(f"exported {name}: {len(decks)} decks -> {dst}")
    print(f"exported {count} suites into {suites_out}")


if __name__ == "__main__":
    ap = argparse.ArgumentParser()
    ap.add_argument("--repo", default=os.path.abspath(os.path.join(os.path.dirname(__file__), "../..")))
    ap.add_argument("--out", required=True)
    args = ap.parse_args()
    export_suites(args.repo, args.out)
