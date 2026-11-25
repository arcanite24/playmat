"""Prioritized Fictitious Self-Play (PFSP) League Manager for Phase 3 anti-echo-chamber self-play.

Tracks a heterogeneous opponent pool (1.3M soup, 5M soup, 10M, historical anchors, host@s6) in a persistent
`league_state.json`, computes PFSP sampling probabilities (`hard`: (1 - winrate)^p, `variance`: w*(1 - w),
or `uniform`), mixes Constructed + OOD deck suites, randomizes opening temperature/turn schedules, and
executes or plans `selfplay` rounds writing v3 shards.
"""

import argparse
import json
import math
import os
import re
import subprocess
import time

import numpy as np


def default_league(runs_dir="runs"):
    return {
        "version": 1,
        "agents": {
            "wavec-1.3m-soup": {
                "arch": "2.05m",
                "spec": f"cfg:policy=ismcts+net={runs_dir}/wavec-final-1.3m-soup/net.pmnet+hide=1+iters=256+samples=8+cpuct=1000+leaf_blend=1000+gumbel_root=1+multi_tree=1+mulligan_net=1+width=16+consider=12+extra_turns=2",
                "elo": 1110.0,
            },
            "wavec-5m-soup": {
                "arch": "5.07m",
                "spec": f"cfg:policy=ismcts+net={runs_dir}/wavec-final-5m-soup/net.pmnet+hide=1+iters=256+samples=8+cpuct=1000+leaf_blend=1000+gumbel_root=1+multi_tree=1+mulligan_net=1+width=16+consider=12+extra_turns=2",
                "elo": 1114.0,
            },
            "wavec-r12-5m": {
                "arch": "5.07m",
                "spec": f"cfg:policy=ismcts+net={runs_dir}/wavec-r12-5m-sem-oracle/net.pmnet+hide=1+iters=256+samples=8+cpuct=1000+leaf_blend=1000+gumbel_root=1+multi_tree=1+mulligan_net=1+width=16+consider=12+extra_turns=2",
                "elo": 1102.0,
            },
            "day1-r6-1.3m": {
                "arch": "1.3m",
                "spec": f"cfg:policy=ismcts+net={runs_dir}/obsv2-r6-1.3m-reg/net.pmnet+hide=1+iters=128+cpuct=1000+leaf_blend=1000+multi_tree=1+mulligan_sim=1+extra_turns=0",
                "elo": 1020.0,
            },
            "host@s6": {
                "arch": "heuristic",
                "spec": "host@s6",
                "elo": 1000.0,
            },
        },
        "history": {},
    }


def load_league(path, runs_dir="runs"):
    if path and os.path.exists(path):
        return json.load(open(path))
    return default_league(runs_dir)


def save_league(state, path):
    os.makedirs(os.path.dirname(os.path.abspath(path)), exist_ok=True)
    with open(path, "w") as f:
        json.dump(state, f, indent=2)


def winrate_against(state, learner, opp):
    key = f"{learner}__vs__{opp}"
    rec = state["history"].get(key)
    if not rec or rec["games"] == 0:
        # Prior from Elo difference if unplayed
        e_l = state["agents"].get(learner, {}).get("elo", 1100.0)
        e_o = state["agents"].get(opp, {}).get("elo", 1000.0)
        return 1.0 / (1.0 + 10.0 ** ((e_o - e_l) / 400.0))
    return (rec["w"] + 0.5 * rec["d"] + 1.0) / (rec["games"] + 2.0)


def pfsp_weights(state, learner, opponents, mode="hard", p=2.0, self_play_frac=0.35):
    """Returns normalized opponent sampling probabilities combining self-play + PFSP over diverse opponents."""
    weights = {}
    others = [o for o in opponents if o != learner]
    if not others:
        return {learner: 1.0}
    raw = []
    for o in others:
        wr = winrate_against(state, learner, o)
        if mode == "hard":
            f = (1.0 - wr) ** p
        elif mode == "variance":
            f = wr * (1.0 - wr)
        else:
            f = 1.0
        raw.append(max(f, 1e-3))
    s = sum(raw)
    other_share = 1.0 - (self_play_frac if learner in opponents else 0.0)
    if learner in opponents:
        weights[learner] = self_play_frac
    for o, r in zip(others, raw):
        weights[o] = other_share * (r / s)
    return weights


def plan_round(state, learner, total_games, suites, suite_weights, mode="hard", p=2.0, self_play_frac=0.35, seed=0):
    rng = np.random.default_rng(seed)
    opps = list(state["agents"].keys())
    if learner not in opps:
        opps.append(learner)
    probs = pfsp_weights(state, learner, opps, mode=mode, p=p, self_play_frac=self_play_frac)
    sw = np.array(suite_weights, dtype=np.float64)
    sw = sw / sw.sum()
    # Allocate games into chunks of >= 20 games across (opponent, suite, temp)
    plan = []
    for opp, prob in sorted(probs.items(), key=lambda kv: -kv[1]):
        opp_games = int(round(total_games * prob))
        if opp_games < 2:
            continue
        for si, suite in enumerate(suites):
            g = int(round(opp_games * sw[si]))
            if g < 2:
                continue
            temp = int(rng.choice([750, 1000, 1200]))
            temp_turn = int(rng.choice([6, 8, 10]))
            plan.append({
                "opponent": opp,
                "opponent_spec": state["agents"][opp]["spec"] if opp in state["agents"] else state["agents"][learner]["spec"],
                "suite": suite,
                "games": g,
                "temp": temp,
                "temp_turn": temp_turn,
                "pfsp_prob": round(prob, 4),
                "est_winrate": round(winrate_against(state, learner, opp), 4),
            })
    return plan


def with_temp(spec, temp, temp_turn):
    if not spec.startswith("cfg:"):
        return spec
    spec = re.sub(r"\+temp=\d+", "", spec)
    spec = re.sub(r"\+temp_turn=\d+", "", spec)
    return f"{spec}+temp={temp}+temp_turn={temp_turn}"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--state", default="league_state.json")
    ap.add_argument("--runs-dir", default="runs")
    ap.add_argument("--learner", default="wavec-1.3m-soup")
    ap.add_argument("--learner-spec", default=None)
    ap.add_argument("--games", type=int, default=2000)
    ap.add_argument("--jobs", type=int, default=32)
    ap.add_argument("--keep", type=int, default=160)
    ap.add_argument("--fast", default="ismcts:12")
    ap.add_argument("--suites", default="../../archbench/constructed-2,../../archbench/constructed-ood-1")
    ap.add_argument("--suite-weights", default="0.65,0.35")
    ap.add_argument("--mode", choices=["hard", "variance", "uniform"], default="hard")
    ap.add_argument("--power", type=float, default=2.0)
    ap.add_argument("--self-play-frac", type=float, default=0.35)
    ap.add_argument("--seed", type=int, default=0)
    ap.add_argument("--bin", default="../../rust/target/release/selfplay")
    ap.add_argument("--out", default=None, help="output shard directory when --run-round is set")
    ap.add_argument("--run-round", action="store_true")
    args = ap.parse_args()

    state = load_league(args.state, args.runs_dir)
    if args.learner_spec:
        state["agents"][args.learner] = {"arch": "custom", "spec": args.learner_spec, "elo": 1120.0}
    suites = [s.strip() for s in args.suites.split(",") if s.strip()]
    sw = [float(x) for x in args.suite_weights.split(",")]
    plan = plan_round(state, args.learner, args.games, suites, sw, mode=args.mode, p=args.power, self_play_frac=args.self_play_frac, seed=args.seed)

    print(f"PFSP PLAN learner={args.learner} total~{sum(p['games'] for p in plan)} games mode={args.mode} (p={args.power})")
    for i, item in enumerate(plan):
        print(f"  [{i:02d}] vs {item['opponent']:<18} suite={os.path.basename(item['suite']):<18} games={item['games']:>4} temp={item['temp']}@{item['temp_turn']} pfsp={item['pfsp_prob']:.3f} wr={item['est_winrate']:.3f}")

    if not args.run_round:
        return

    if not args.out:
        raise ValueError("--out is required with --run-round")
    os.makedirs(args.out, exist_ok=True)
    learner_base = state["agents"][args.learner]["spec"]
    pat = re.compile(r"SELFPLAY against .*?: (\d+)-(\d+)-(\d+)")
    for i, item in enumerate(plan):
        sub_out = os.path.join(args.out, f"slot_{i:02d}_{item['opponent'].replace('@', '_')}")
        os.makedirs(sub_out, exist_ok=True)
        cmd = [
            args.bin,
            "--suite", item["suite"],
            "--games", str(item["games"]),
            "--jobs", str(args.jobs),
            "--keep", str(args.keep),
            "--fast", args.fast,
            "--out", sub_out,
            "--learner", with_temp(learner_base, item["temp"], item["temp_turn"]),
            "--opponent", item["opponent_spec"],
        ]
        print(f"RUN slot {i:02d}: {' '.join(cmd)}", flush=True)
        proc = subprocess.run(cmd, capture_output=True, text=True, check=True)
        print(proc.stdout.strip(), flush=True)
        m = pat.search(proc.stdout)
        if m:
            w, d, l = int(m.group(1)), int(m.group(2)), int(m.group(3))
            key = f"{args.learner}__vs__{item['opponent']}"
            rec = state["history"].setdefault(key, {"w": 0, "d": 0, "l": 0, "games": 0})
            rec["w"] += w
            rec["d"] += d
            rec["l"] += l
            rec["games"] += w + d + l
    save_league(state, args.state)
    print(f"Updated league state -> {args.state}")


if __name__ == "__main__":
    main()
