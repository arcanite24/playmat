"""Permanent Neural Anchor & Bradley-Terry Elo Gauntlet runner.

Pins fixed neural and heuristic anchors (host@s6, host@s6-fair, obsv2-r6-1.3m-reg, wavec-final-1.3m-soup,
wavec-final-5m-soup) so Phase 3 models are measured against strong neural peers instead of saturating host@s6 alone.

Computes exact Maximum-Likelihood Bradley-Terry Elo ratings with 95% confidence intervals via the observed
Fisher information matrix.
"""

import argparse
import json
import math
import os
import re
import subprocess
import time

import numpy as np
from scipy.optimize import minimize

FAIR_HOST = "cfg:policy=playouts+budget=5000+width=12+threads=4+hide=1+samples=4+playout=2+eval=2+plans=1+mulligan_sim=1"


def default_anchors(runs_dir="runs"):
    return {
        "host@s6": "host@s6",
        "host@s6-fair": FAIR_HOST,
        "day1-r6@128": (
            f"cfg:policy=ismcts+net={runs_dir}/obsv2-r6-1.3m-reg/net.pmnet"
            "+hide=1+samples=4+iters=128+cpuct=1000+leaf_blend=1000+multi_tree=1+mulligan_sim=1+extra_turns=0"
        ),
        "wavec-1.3m-soup@256": (
            f"cfg:policy=ismcts+net={runs_dir}/wavec-final-1.3m-soup/net.pmnet"
            "+hide=1+iters=256+samples=8+cpuct=1000+leaf_blend=1000+gumbel_root=1+multi_tree=1+mulligan_net=1+width=16+consider=12+extra_turns=2"
        ),
        "wavec-5m-soup@256": (
            f"cfg:policy=ismcts+net={runs_dir}/wavec-final-5m-soup/net.pmnet"
            "+hide=1+iters=256+samples=8+cpuct=1000+leaf_blend=1000+gumbel_root=1+multi_tree=1+mulligan_net=1+width=16+consider=12+extra_turns=2"
        ),
    }


def fit_bradley_terry_elo(names, records, anchor_name=None, prior_games=0.25):
    """Fits MLE Bradley-Terry ratings (Elo scale: 400/ln(10) * theta) with 95% CI via Fisher Hessian.

    records: list of (i, j, wins_i, draws, losses_i)
    anchor_name: entrant anchored to 1000.0 Elo (defaults to names[0]).
    """
    m = len(names)
    anchor = names.index(anchor_name) if (anchor_name in names) else 0
    free = [i for i in range(m) if i != anchor]
    scale = 400.0 / math.log(10.0)

    def unpack(x):
        th = np.zeros(m, dtype=np.float64)
        for k, idx in enumerate(free):
            th[idx] = x[k]
        return th

    def nll(x):
        th = unpack(x)
        loss = 0.5 * prior_games * float(np.sum(th * th))
        for i, j, w, d, l in records:
            s_i = w + 0.5 * d
            s_j = l + 0.5 * d
            diff = th[i] - th[j]
            # log(1 + exp(-diff)) and log(1 + exp(diff)) stably
            loss += s_i * np.logaddexp(0.0, -diff) + s_j * np.logaddexp(0.0, diff)
        return loss

    def grad(x):
        th = unpack(x)
        g = prior_games * th.copy()
        for i, j, w, d, l in records:
            n_g = w + d + l
            s_i = w + 0.5 * d
            p = 1.0 / (1.0 + math.exp(max(-40.0, min(40.0, -(th[i] - th[j])))))
            diff_g = n_g * p - s_i
            g[i] += diff_g
            g[j] -= diff_g
        return np.array([g[idx] for idx in free], dtype=np.float64)

    res = minimize(nll, np.zeros(len(free)), jac=grad, method="BFGS")
    th = unpack(res.x)

    # Observed Fisher Information Hessian on free parameters
    H = np.eye(len(free), dtype=np.float64) * prior_games
    pos = {idx: k for k, idx in enumerate(free)}
    for i, j, w, d, l in records:
        n_g = w + d + l
        p = 1.0 / (1.0 + math.exp(max(-40.0, min(40.0, -(th[i] - th[j])))))
        info = n_g * p * (1.0 - p)
        if i in pos:
            H[pos[i], pos[i]] += info
        if j in pos:
            H[pos[j], pos[j]] += info
        if i in pos and j in pos:
            H[pos[i], pos[j]] -= info
            H[pos[j], pos[i]] -= info
    cov = np.linalg.pinv(H)
    se = np.zeros(m, dtype=np.float64)
    for k, idx in enumerate(free):
        se[idx] = math.sqrt(max(cov[k, k], 0.0))

    out = []
    for i, name in enumerate(names):
        w_tot = sum(w for a, b, w, d, l in records if a == i) + sum(l for a, b, w, d, l in records if b == i)
        d_tot = sum(d for a, b, w, d, l in records if a == i or b == i)
        l_tot = sum(l for a, b, w, d, l in records if a == i) + sum(w for a, b, w, d, l in records if b == i)
        out.append({
            "name": name,
            "elo": round(1000.0 + scale * th[i], 1),
            "ci95": round(1.96 * scale * se[i], 1),
            "w": int(w_tot),
            "d": int(d_tot),
            "l": int(l_tot),
            "games": int(w_tot + d_tot + l_tot),
        })
    out.sort(key=lambda r: r["elo"], reverse=True)
    return out


def parse_archbench_h2h(text, spec_a, spec_b):
    pat = re.compile(r"^ARCHBENCH\s+(.+?)\s+vs\s+(.+?):\s+(\d+)-(\d+)-(\d+)\s*$")
    for line in text.splitlines():
        m = pat.match(line.strip())
        if not m:
            continue
        lhs, rhs = m.group(1).strip(), m.group(2).strip()
        w, d, l = int(m.group(3)), int(m.group(4)), int(m.group(5))
        if lhs == spec_a and rhs == spec_b:
            return w, d, l
        if lhs == spec_b and rhs == spec_a:
            return l, d, w
    raise RuntimeError(f"Could not parse archbench W-D-L for ({spec_a} vs {spec_b}) from output:\n{text[-1000:]}")


def run_match(bin_path, suite, deals, jobs, spec_a, spec_b):
    cmd = [
        bin_path,
        "--suite", suite,
        "--deals", str(deals),
        "--jobs", str(jobs),
        "--entrants", f"{spec_a},{spec_b}",
    ]
    t0 = time.time()
    proc = subprocess.run(cmd, capture_output=True, text=True, check=True)
    w, d, l = parse_archbench_h2h(proc.stdout, spec_a, spec_b)
    return w, d, l, time.time() - t0


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--bin", default="../../rust/target/release/archbench")
    ap.add_argument("--suite", default="../../archbench/constructed-2")
    ap.add_argument("--runs-dir", default="runs")
    ap.add_argument("--deals", type=int, default=100, help="duplicate deals per pairing (2*deals games)")
    ap.add_argument("--jobs", type=int, default=32)
    ap.add_argument("--anchors", default="host@s6,day1-r6@128,wavec-1.3m-soup@256,wavec-5m-soup@256")
    ap.add_argument("--candidate", action="append", default=[], help="label=entrant_spec")
    ap.add_argument("--round-robin", action="store_true", help="play all pairs instead of candidates vs anchors")
    ap.add_argument("--anchor-elo", default="host@s6", help="entrant anchored at 1000.0 Elo")
    ap.add_argument("--out-json", default=None)
    ap.add_argument("--self-test", action="store_true", help="run unit test of MLE Bradley-Terry solver")
    args = ap.parse_args()

    if args.self_test:
        names = ["host@s6", "day1", "soup13", "phase3"]
        recs = [
            (1, 0, 220, 16, 164),
            (2, 0, 267, 13, 120),
            (2, 1, 243, 18, 139),
            (3, 2, 230, 14, 156),
            (3, 0, 290, 10, 100),
        ]
        board = fit_bradley_terry_elo(names, recs, anchor_name="host@s6")
        assert board[0]["name"] == "phase3" and board[-1]["name"] == "host@s6"
        assert 1120 < [r for r in board if r["name"] == "soup13"][0]["elo"] < 1150
        print("BRADLEY_TERRY_SELF_TEST_OK", json.dumps(board, indent=2))
        return

    all_anchors = default_anchors(args.runs_dir)
    selected_anchors = [a.strip() for a in args.anchors.split(",") if a.strip()]
    entrants = {}
    for a in selected_anchors:
        if a not in all_anchors:
            raise ValueError(f"Unknown anchor {a}; choices: {list(all_anchors)}")
        entrants[a] = all_anchors[a]
    cand_names = []
    for c in args.candidate:
        k, v = c.split("=", 1)
        entrants[k] = v
        cand_names.append(k)

    names = list(entrants.keys())
    idx_of = {n: i for i, n in enumerate(names)}
    pairs = []
    if args.round_robin or not cand_names:
        for i in range(len(names)):
            for j in range(i + 1, len(names)):
                pairs.append((names[i], names[j]))
    else:
        for c in cand_names:
            for a in selected_anchors:
                if c != a:
                    pairs.append((c, a))
        for i in range(len(cand_names)):
            for j in range(i + 1, len(cand_names)):
                pairs.append((cand_names[i], cand_names[j]))

    records = []
    raw_matches = []
    for a, b in pairs:
        w, d, l, dt = run_match(args.bin, args.suite, args.deals, args.jobs, entrants[a], entrants[b])
        pct = 100.0 * (w + 0.5 * d) / max(w + d + l, 1)
        print(f"MATCH {a} vs {b}: {w}-{d}-{l} ({pct:.1f}%) in {dt:.1f}s", flush=True)
        records.append((idx_of[a], idx_of[b], w, d, l))
        raw_matches.append({"a": a, "b": b, "w": w, "d": d, "l": l, "seconds": round(dt, 1)})

    board = fit_bradley_terry_elo(names, records, anchor_name=args.anchor_elo)
    print(f"\nBRADLEY-TERRY LEADERBOARD (anchor {args.anchor_elo} = 1000.0 Elo):")
    print(f"{'Rank':>4}  {'Entrant':<24}  {'Elo':>7}  {'95% CI':>8}  {'W-D-L':>13}")
    for rank, r in enumerate(board, 1):
        wdl = f"{r['w']}-{r['d']}-{r['l']}"
        print(f"{rank:>4}  {r['name']:<24}  {r['elo']:>7.1f}  +/-{r['ci95']:>5.1f}  {wdl:>13}")

    if args.out_json:
        with open(args.out_json, "w") as f:
            json.dump({"suite": args.suite, "deals": args.deals, "anchor": args.anchor_elo, "leaderboard": board, "matches": raw_matches}, f, indent=2)
        print(f"Wrote {args.out_json}")


if __name__ == "__main__":
    main()
