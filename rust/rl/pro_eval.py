"""Evaluates one or more entrants on categorized competitive MTG decision suites (e.g. archbench/pro-decisions-1).

Example:
    python pro_eval.py --suite ../../archbench/pro-decisions-1 --seeds 4 \
        --entrant host=host@s6 \
        --entrant soup13=cfg:policy=ismcts+net=runs/wavec-final-1.3m-soup/net.pmnet+iters=256+extra_turns=3+gumbel_root=1+leaf_blend=1000+multi_tree=1+samples=8+width=24+consider=16
"""

import argparse
import collections
import json
import os
import re
import subprocess
import sys


def parse_tactics_output(text, labels):
    rows = {}
    pat = re.compile(r"^TACTICS\s+(\S+)\s+((?:\d+/\d+\s*)+)$")
    for line in text.splitlines():
        m = pat.match(line.strip())
        if not m:
            continue
        name = m.group(1)
        scores = m.group(2).split()
        if len(scores) != len(labels):
            continue
        parsed = []
        for s in scores:
            a, b = s.split("/")
            parsed.append((int(a), int(b)))
        rows[name] = parsed
    return rows


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--bin", default="../../rust/target/release/tactics")
    ap.add_argument("--suite", default="../../archbench/pro-decisions-1")
    ap.add_argument("--seeds", type=int, default=4)
    ap.add_argument("--entrant", action="append", required=True, help="label=entrant_spec")
    ap.add_argument("--out-json", default=None)
    args = ap.parse_args()

    labels, specs = [], []
    for item in args.entrant:
        if "=" in item and not item.startswith("cfg:"):
            k, v = item.split("=", 1)
        else:
            k, v = item, item
        labels.append(k)
        specs.append(v)

    suite_data = json.load(open(os.path.join(args.suite, "puzzles.json")))
    cat_of = {p["name"]: p.get("category", "uncategorized") for p in suite_data["puzzles"]}
    categories = sorted(set(cat_of.values()))

    cmd = [args.bin, "--suite", args.suite, "--seeds", str(args.seeds), "--entrants", ",".join(specs)]
    proc = subprocess.run(cmd, capture_output=True, text=True, check=True)
    rows = parse_tactics_output(proc.stdout, labels)

    cat_totals = {c: [[0, 0] for _ in labels] for c in categories}
    overall = [[0, 0] for _ in labels]
    for pname, scores in rows.items():
        c = cat_of.get(pname, "uncategorized")
        for i, (s, n) in enumerate(scores):
            cat_totals[c][i][0] += s
            cat_totals[c][i][1] += n
            overall[i][0] += s
            overall[i][1] += n

    print(f"PRO-EVAL suite={suite_data.get('suite')} puzzles={len(rows)} seeds={args.seeds}")
    hdr = f"{'Category':<20}" + "".join(f"  {lb:>16}" for lb in labels)
    print(hdr)
    print("-" * len(hdr))
    summary = {"suite": suite_data.get("suite"), "seeds": args.seeds, "entrants": labels, "categories": {}, "overall": {}}
    for c in categories:
        parts = []
        summary["categories"][c] = {}
        for i, lb in enumerate(labels):
            s, n = cat_totals[c][i]
            pct = 100.0 * s / max(n, 1)
            parts.append(f"  {s:>3}/{n:<3} ({pct:5.1f}%)")
            summary["categories"][c][lb] = {"solved": s, "total": n, "pct": round(pct, 2)}
        print(f"{c:<20}" + "".join(parts))
    print("-" * len(hdr))
    parts = []
    for i, lb in enumerate(labels):
        s, n = overall[i]
        pct = 100.0 * s / max(n, 1)
        parts.append(f"  {s:>3}/{n:<3} ({pct:5.1f}%)")
        summary["overall"][lb] = {"solved": s, "total": n, "pct": round(pct, 2)}
    print(f"{'OVERALL':<20}" + "".join(parts))

    if args.out_json:
        with open(args.out_json, "w") as f:
            json.dump(summary, f, indent=2)
        print(f"Wrote {args.out_json}")


if __name__ == "__main__":
    main()
