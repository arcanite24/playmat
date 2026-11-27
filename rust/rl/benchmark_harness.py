#!/usr/bin/env python3
"""Unified multi-AI benchmark harness for Playmat Archbench, Tactics, and Pro-Decisions.

Benchmarks any mix of Playmat's native Rust models and external AI adapters (Jev, frontier & open-weight LLMs,
Forge-style heuristics, ONNX models, Python plugins, HTTP agents, and MCP bridges) on the exact same locked suites,
automatically managing adapter server lifecycles and collecting Elo, win rates, latency, token usage, and USD cost.

Examples:
    # Probe available engines on this machine
    python3 rust/rl/benchmark_harness.py --mode probe

    # Benchmark Forge-style heuristic & Jev mock vs Playmat's rust-quick and random on constructed-2
    python3 rust/rl/benchmark_harness.py --mode duels --suite archbench/constructed-2 --deals 10 --jobs 4 \
        --entrant quick=rust-quick --entrant random=random --entrant forge=open:forge-heuristic --entrant jev=jev:mock

    # Benchmark Jev & Claude Opus 4.6 on pro-decisions-1
    OPENROUTER_API_KEY=... python3 rust/rl/benchmark_harness.py --mode puzzles --suite archbench/pro-decisions-1 --seeds 2 \
        --entrant host=host@s6 --entrant jev=jev:typesafe/jev-1.13 \
        --entrant opus=llm:openrouter/anthropic/claude-opus-4.6 --out-json pro_results.json --publish-to-site
"""

from __future__ import annotations

import argparse
import datetime
import json
import math
import os
import re
import subprocess
import sys
import time
from typing import Any, Dict, List, Optional, Tuple

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", ".."))
if HERE not in sys.path:
    sys.path.insert(0, HERE)

from adapters import build_adapter, is_adapter_spec  # noqa: E402
from adapters.base import Adapter, start_tcp_server  # noqa: E402
from adapters.external_engines import probe_engines  # noqa: E402


def resolve_binary(name: str, explicit: Optional[str] = None) -> str:
    if explicit:
        return explicit
    ext = ".exe" if os.name == "nt" else ""
    candidates = [
        os.path.join(REPO, "rust", "target", "release", name + ext),
        os.path.join(REPO, "rust", "target", "debug", name + ext),
    ]
    for path in candidates:
        if os.path.exists(path):
            return path
    raise FileNotFoundError(
        f"Playmat Rust binary `{name}` not found. Build it with: cd rust && cargo build --release -p playmat-ai --bins"
    )


def resolve_suite(path: str) -> str:
    if os.path.isabs(path) and os.path.exists(path):
        return path
    cand = os.path.join(REPO, path)
    if os.path.exists(cand):
        return cand
    cand2 = os.path.join(REPO, "archbench", path)
    if os.path.exists(cand2):
        return cand2
    return path


def bradley_terry_elo(
    names: List[str],
    pairs: List[Tuple[int, int, int, int, int]],
    anchor_name: Optional[str] = None,
) -> List[Dict[str, Any]]:
    """Pure-Python MM Bradley-Terry Elo solver matching archbench.rs (half-game Laplace prior)."""
    k = len(names)
    w_mat = [[0.0] * k for _ in range(k)]
    totals = [[0, 0, 0] for _ in range(k)]
    for i, j, w, d, l in pairs:
        w_mat[i][j] += w + 0.5 * d
        w_mat[j][i] += l + 0.5 * d
        totals[i][0] += w
        totals[i][1] += d
        totals[i][2] += l
        totals[j][0] += l
        totals[j][1] += d
        totals[j][2] += w

    p = [1.0] * k
    for _ in range(2000):
        q = p[:]
        for x in range(k):
            wins = sum(w_mat[x][y] + 0.5 for y in range(k) if y != x)
            denom = sum((w_mat[x][y] + w_mat[y][x] + 1.0) / (p[x] + p[y]) for y in range(k) if y != x)
            q[x] = wins / max(denom, 1e-12)
        g = sum(math.log(max(v, 1e-12)) for v in q) / max(k, 1)
        p = [math.exp(math.log(max(v, 1e-12)) - g) for v in q]

    elos = [400.0 * math.log10(max(v, 1e-12)) for v in p]
    if anchor_name and anchor_name in names:
        zero = elos[names.index(anchor_name)]
    elif "random" in names:
        zero = elos[names.index("random")]
    else:
        zero = sum(elos) / max(k, 1)

    rows = []
    for i, name in enumerate(names):
        w, d, l = totals[i]
        games = w + d + l
        pct = 100.0 * (w + 0.5 * d) / max(games, 1)
        rows.append(
            {
                "name": name,
                "elo": round(elos[i] - zero + 1000.0, 1),
                "won": w,
                "drew": d,
                "lost": l,
                "games": games,
                "win_pct": round(pct, 2),
            }
        )
    rows.sort(key=lambda r: r["elo"], reverse=True)
    return rows


def parse_archbench_output(text: str, spec_to_label: Dict[str, str]) -> Tuple[List[Dict[str, Any]], Dict[str, int]]:
    """Parses `ARCHBENCH   A vs B: W-D-L` and per-entrant worst decision ms from archbench stdout."""
    pair_pat = re.compile(r"^ARCHBENCH\s+(.+?)\s+vs\s+(.+?):\s+(\d+)-(\d+)-(\d+)\s*$")
    stat_pat = re.compile(r"^ARCHBENCH\s+(\S+)\s+rating\s+\S+\s+won\s+\d+\s+drew\s+\d+\s+lost\s+\d+\s+worst decision\s+(\d+)\s+ms")
    seen_pairs = set()
    pairs: List[Dict[str, Any]] = []
    worst_ms: Dict[str, int] = {}

    for raw in text.splitlines():
        line = raw.strip()
        ms = stat_pat.match(line)
        if ms:
            lbl = spec_to_label.get(ms.group(1), ms.group(1))
            worst_ms[lbl] = int(ms.group(2))
            continue
        mp = pair_pat.match(line)
        if mp:
            a_spec, b_spec = mp.group(1).strip(), mp.group(2).strip()
            a = spec_to_label.get(a_spec, a_spec)
            b = spec_to_label.get(b_spec, b_spec)
            key = tuple(sorted((a, b)))
            if key in seen_pairs:
                continue
            seen_pairs.add(key)
            pairs.append(
                {
                    "a": a,
                    "b": b,
                    "a_won": int(mp.group(3)),
                    "drew": int(mp.group(4)),
                    "b_won": int(mp.group(5)),
                }
            )
    return pairs, worst_ms


def parse_tactics_output(text: str, labels: List[str]) -> Dict[str, List[Tuple[int, int]]]:
    pat = re.compile(r"^TACTICS\s+(\S+)\s+((?:\d+/\d+\s*)+)$")
    rows: Dict[str, List[Tuple[int, int]]] = {}
    for raw in text.splitlines():
        m = pat.match(raw.strip())
        if not m:
            continue
        scores = m.group(2).split()
        if len(scores) != len(labels):
            continue
        parsed = []
        for s in scores:
            a, b = s.split("/")
            parsed.append((int(a), int(b)))
        rows[m.group(1)] = parsed
    return rows


def publish_run_to_site(summary: Dict[str, Any]) -> None:
    """Appends this harness run into rust/tools/benchmarks-data.json and regenerates site/benchmarks/benchmarks.json."""
    data_path = os.path.join(REPO, "rust", "tools", "benchmarks-data.json")
    gen_path = os.path.join(REPO, "rust", "tools", "benchmarks.py")
    with open(data_path, "r", encoding="utf-8") as f:
        data = json.load(f)
    runs = data.setdefault("adapter_runs", [])
    runs.append(summary)
    data["updated"] = datetime.date.today().isoformat()
    with open(data_path, "w", encoding="utf-8") as f:
        json.dump(data, f, indent=2, ensure_ascii=False)
        f.write("\n")
    subprocess.run([sys.executable, gen_path], check=True)
    print(f"Published run into {data_path} and regenerated site/benchmarks/benchmarks.json")


def main() -> None:
    ap = argparse.ArgumentParser(description="Unified Playmat multi-AI benchmark harness (native + Jev + LLMs + open AIs).")
    ap.add_argument("--mode", choices=["duels", "puzzles", "probe"], default="duels")
    ap.add_argument("--suite", default="archbench/constructed-2", help="Archbench duel or puzzle suite path")
    ap.add_argument("--entrant", action="append", default=[], help="label=spec (built-in or jev:/llm:/open:/onnx:/python:/http:/mcp:)")
    ap.add_argument("--deals", type=int, default=10, help="Duplicate deals per pair in duels mode (2*deals games/pair)")
    ap.add_argument("--jobs", type=int, default=2, help="Parallel worker threads in duels mode")
    ap.add_argument("--seeds", type=int, default=2, help="Seeds per puzzle in puzzles mode")
    ap.add_argument("--only", default=None, help="Comma-separated puzzle names to run (puzzles mode)")
    ap.add_argument("--anchor", default=None, help="Entrant label anchored at 1000.0 Elo (defaults to random or mean)")
    ap.add_argument("--bin", default=None, help="Explicit path to archbench or tactics binary")
    ap.add_argument("--base-url", default=None, help="Optional endpoint override for Jev/LLM adapters")
    ap.add_argument("--reasoning-effort", default=None, help="Optional reasoning effort for LLM adapters")
    ap.add_argument("--with-value", action="store_true", help="Enable Jev secondary board-value scoring")
    ap.add_argument("--timeout-ms", type=int, default=60000, help="Per-decision external timeout in ms")
    ap.add_argument("--out-json", default=None, help="Write structured benchmark report JSON")
    ap.add_argument("--publish-to-site", action="store_true", help="Record run in rust/tools/benchmarks-data.json and rebuild site JSON")
    args = ap.parse_args()

    if args.mode == "probe":
        print(json.dumps(probe_engines(), indent=2))
        return

    if not args.entrant:
        ap.error("--entrant label=spec is required for duels and puzzles modes")

    suite_dir = resolve_suite(args.suite)
    servers: List[Tuple[Any, Adapter]] = []
    labels: List[str] = []
    rust_specs: List[str] = []
    orig_specs: Dict[str, str] = {}
    adapter_of: Dict[str, Adapter] = {}
    spec_to_label: Dict[str, str] = {}

    try:
        for item in args.entrant:
            if "=" in item and not item.startswith(("cfg:", "ext:", "http://", "https://")):
                lbl, spec = item.split("=", 1)
            else:
                lbl, spec = item, item
            lbl = lbl.strip()
            spec = spec.strip()
            labels.append(lbl)
            orig_specs[lbl] = spec

            if is_adapter_spec(spec):
                adapter = build_adapter(
                    spec,
                    base_url=args.base_url,
                    reasoning_effort=args.reasoning_effort,
                    with_value=args.with_value,
                )
                srv, _, port = start_tcp_server(adapter, host="127.0.0.1", port=0)
                servers.append((srv, adapter))
                adapter_of[lbl] = adapter
                suffix = "?obs=1" if adapter.needs_obs else ""
                rspec = f"ext:tcp://127.0.0.1:{port}{suffix}"
            else:
                rspec = spec
            rust_specs.append(rspec)
            spec_to_label[rspec] = lbl

        env = dict(os.environ, PLAYMAT_EXT_TIMEOUT_MS=str(args.timeout_ms))
        t0 = time.time()

        if args.mode == "duels":
            bin_path = resolve_binary("archbench", args.bin)
            cmd = [
                bin_path,
                "--suite",
                suite_dir,
                "--deals",
                str(args.deals),
                "--jobs",
                str(args.jobs),
                "--entrants",
                ",".join(rust_specs),
            ]
            proc = subprocess.run(cmd, env=env, capture_output=True, text=True)
            if proc.returncode != 0:
                print(proc.stdout)
                print(proc.stderr, file=sys.stderr)
                sys.exit(proc.returncode)
            elapsed = round(time.time() - t0, 2)
            pairs_list, worst_ms = parse_archbench_output(proc.stdout, spec_to_label)
            idx_of = {lbl: i for i, lbl in enumerate(labels)}
            bt_records = [
                (idx_of[p["a"]], idx_of[p["b"]], p["a_won"], p["drew"], p["b_won"])
                for p in pairs_list
                if p["a"] in idx_of and p["b"] in idx_of
            ]
            board = bradley_terry_elo(labels, bt_records, anchor_name=args.anchor)
            for row in board:
                lbl = row["name"]
                row["spec"] = orig_specs[lbl]
                row["worst_decision_ms"] = worst_ms.get(lbl)
                if lbl in adapter_of:
                    row["telemetry"] = adapter_of[lbl].telemetry.summary()

            suite_meta = json.load(open(os.path.join(suite_dir, "suite.json"), encoding="utf-8"))
            summary = {
                "mode": "duels",
                "suite": suite_meta.get("suite", os.path.basename(suite_dir)),
                "date": datetime.date.today().isoformat(),
                "deals": args.deals,
                "elapsed_s": elapsed,
                "leaderboard": board,
                "pairs": pairs_list,
            }

            print(f"\nPLAYMAT HARNESS ({summary['suite']}, {args.deals} deals/pair, {elapsed:.1f}s):")
            print(f"{'Rank':>4}  {'Entrant':<22}  {'Elo':>7}  {'W-D-L':>11}  {'Win%':>6}  {'Avg ms':>8}  {'Cost USD':>9}")
            for rk, r in enumerate(board, 1):
                tel = r.get("telemetry") or {}
                avg_ms = f"{tel['avg_latency_ms']:.1f}" if "avg_latency_ms" in tel else "-"
                cost = f"${tel['cost_usd']:.4f}" if "cost_usd" in tel else "$0.0000"
                wdl = f"{r['won']}-{r['drew']}-{r['lost']}"
                print(f"{rk:>4}  {r['name']:<22}  {r['elo']:>7.1f}  {wdl:>11}  {r['win_pct']:>5.1f}%  {avg_ms:>8}  {cost:>9}")
        else:
            bin_path = resolve_binary("tactics", args.bin)
            cmd = [
                bin_path,
                "--suite",
                suite_dir,
                "--seeds",
                str(args.seeds),
                "--entrants",
                ",".join(rust_specs),
            ]
            if args.only:
                cmd += ["--only", args.only]
            proc = subprocess.run(cmd, env=env, capture_output=True, text=True)
            if proc.returncode != 0:
                print(proc.stdout)
                print(proc.stderr, file=sys.stderr)
                sys.exit(proc.returncode)
            elapsed = round(time.time() - t0, 2)
            suite_meta = json.load(open(os.path.join(suite_dir, "puzzles.json"), encoding="utf-8"))
            cat_of = {p["name"]: p.get("category", "all") for p in suite_meta.get("puzzles", [])}
            rows = parse_tactics_output(proc.stdout, labels)

            categories = sorted(set(cat_of.values()))
            cat_totals = {c: [[0, 0] for _ in labels] for c in categories}
            overall = [[0, 0] for _ in labels]
            for pname, scores in rows.items():
                c = cat_of.get(pname, "all")
                for i, (s, n) in enumerate(scores):
                    cat_totals[c][i][0] += s
                    cat_totals[c][i][1] += n
                    overall[i][0] += s
                    overall[i][1] += n

            standings = []
            for i, lbl in enumerate(labels):
                s, n = overall[i]
                entry: Dict[str, Any] = {
                    "name": lbl,
                    "spec": orig_specs[lbl],
                    "solved": s,
                    "total": n,
                    "pct": round(100.0 * s / max(n, 1), 2),
                    "categories": {
                        c: {
                            "solved": cat_totals[c][i][0],
                            "total": cat_totals[c][i][1],
                            "pct": round(100.0 * cat_totals[c][i][0] / max(cat_totals[c][i][1], 1), 2),
                        }
                        for c in categories
                    },
                }
                if lbl in adapter_of:
                    entry["telemetry"] = adapter_of[lbl].telemetry.summary()
                standings.append(entry)
            standings.sort(key=lambda r: r["pct"], reverse=True)

            summary = {
                "mode": "puzzles",
                "suite": suite_meta.get("suite", os.path.basename(suite_dir)),
                "date": datetime.date.today().isoformat(),
                "seeds": args.seeds,
                "puzzles": len(rows),
                "elapsed_s": elapsed,
                "standings": standings,
            }
            print(f"\nPLAYMAT HARNESS ({summary['suite']}, {len(rows)} puzzles x {args.seeds} seeds, {elapsed:.1f}s):")
            print(f"{'Rank':>4}  {'Entrant':<22}  {'Solved':>9}  {'Score':>7}  {'Avg ms':>8}  {'Cost USD':>9}")
            for rk, r in enumerate(standings, 1):
                tel = r.get("telemetry") or {}
                avg_ms = f"{tel['avg_latency_ms']:.1f}" if "avg_latency_ms" in tel else "-"
                cost = f"${tel['cost_usd']:.4f}" if "cost_usd" in tel else "$0.0000"
                print(f"{rk:>4}  {r['name']:<22}  {r['solved']:>4}/{r['total']:<4}  {r['pct']:>6.1f}%  {avg_ms:>8}  {cost:>9}")

        if args.out_json:
            with open(args.out_json, "w", encoding="utf-8") as f:
                json.dump(summary, f, indent=2)
            print(f"Wrote {args.out_json}")

        if args.publish_to_site:
            publish_run_to_site(summary)
    finally:
        for srv, adapter in servers:
            try:
                srv.shutdown()
                srv.server_close()
            except Exception:
                pass
            if hasattr(adapter, "close"):
                try:
                    adapter.close()
                except Exception:
                    pass


if __name__ == "__main__":
    main()
