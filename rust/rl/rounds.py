"""Search-then-learn rounds (#400 R4): the searched policy plays itself, the network learns the search's improved
policy and the games' results, and the new network replaces the old one only when an SPRT says it is stronger.

    python rounds.py --init runs/bc-tx --out runs/loop --rounds 5 --games 20000

Each round `r` writes `data/<out name>-r<r>` (the games) and `<out>/r<r>` (the network, the trainer's report and
the SPRT's log). `<out>/rounds.jsonl` gets one line per round. The data window is the last `--window` rounds.

Faster rounds: `--fast net` answers the decisions that won't be recorded with the network alone, so the search is
spent only on the recorded ones; `--rollout small --rollout-init runs/bc` plays the playouts with a small MLP trained
each round on the same games (`<out>/r<r>/rollout`), so the main network can grow without slowing the playouts.
"""

import argparse
import json
import os
import re
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
RUST = os.path.dirname(HERE)


BIN = os.path.join(RUST, "target", "release")


def exe(name):
    ext = ".exe" if os.name == "nt" else ""
    return os.path.join(BIN, name + ext)


def run(cmd, log):
    print("ROUNDS $", " ".join(cmd), flush=True)
    with open(log, "w") as f:
        p = subprocess.run(cmd, stdout=f, stderr=subprocess.STDOUT, cwd=HERE)
    if p.returncode != 0:
        sys.exit(f"ROUNDS failed ({p.returncode}): see {log}")
    return open(log).read()


def entrant(policy, net, args, roll=None):
    keys = [f"policy={policy}", "hide=1", f"samples={args.samples}", f"eval={args.eval}", f"net={net}"]
    if args.rollout == "small":
        # the round's small network plays the playouts
        keys.append(f"rollout={os.path.join(roll, 'net.pmnet')}")
    elif args.rollout:
        # `self`: the entrant's own network plays the playouts too, so a better network also makes better playouts
        keys.append(f"rollout={net if args.rollout == 'self' else args.rollout}")
    keys += [kv for kv in args.cfg.split("+") if kv]
    return "cfg:" + "+".join(keys)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--init", required=True, help="a train.py run folder: the first network")
    ap.add_argument("--out", required=True)
    ap.add_argument("--rounds", type=int, default=5)
    ap.add_argument("--start", type=int, default=1, help="the round to start at (resume)")
    ap.add_argument("--games", type=int, default=20000)
    ap.add_argument("--keep", type=int, default=500, help="per thousand decisions recorded")
    ap.add_argument("--jobs", type=int, default=12)
    ap.add_argument("--policy", default="gumbel", help="the search that plays: gumbel or netsearch")
    ap.add_argument("--samples", type=int, default=4)
    ap.add_argument("--rollout", default=None,
                    help="a network for inside the playouts: self, small (an MLP trained each round), builtin, or a .pmnet")
    ap.add_argument("--rollout-init", default=None, help="with --rollout small: the first small network's run folder")
    ap.add_argument("--fast", default=None, help="the policy for decisions that won't be recorded (net), or none")
    ap.add_argument("--cfg", default="", help="more config keys for the learner, k=v joined by + (cpuct=300+iters=32)")
    ap.add_argument("--eval", type=int, default=2,
                    help="the search's score: 2 the fitted score, 3 the network's value (lost to 2 in every test so far)")
    ap.add_argument("--temp", type=int, default=1000, help="the search's temperature while recording, per thousand")
    ap.add_argument("--window", type=int, default=2)
    ap.add_argument("--epochs", type=int, default=3)
    ap.add_argument("--lr", type=float, default=5e-4)
    ap.add_argument("--max-deals", type=int, default=3444)
    ap.add_argument("--sprt-jobs", type=int, default=12)
    ap.add_argument("--gate", default="-5,5",
                    help="the SPRT's elo0,elo1 for keeping a round's network (-5,5: keep it unless it got worse)")
    ap.add_argument("--bin", default=None, help="the folder with selfplay and archbench (default ../target/release)")
    ap.add_argument("--train-args", default="", help="more train.py flags, e.g. \"--bs 2048 --lr 1e-3\"")
    args = ap.parse_args()
    if args.bin:
        global BIN
        BIN = os.path.abspath(args.bin)
    os.makedirs(args.out, exist_ok=True)
    if args.rollout == "small" and not args.rollout_init:
        sys.exit("--rollout small needs --rollout-init")
    cur = os.path.abspath(args.init)
    roll = os.path.abspath(args.rollout_init) if args.rollout_init else None
    book = os.path.join(args.out, "rounds.jsonl")
    if args.start > 1 and os.path.exists(book):
        for line in open(book):
            r = json.loads(line)
            if r["round"] < args.start and r["accepted"]:
                cur = r["net_dir"]
                roll = r.get("rollout_dir") or roll
    tag = os.path.basename(os.path.normpath(args.out))
    for r in range(args.start, args.start + args.rounds):
        t0 = time.time()
        rd = os.path.abspath(os.path.join(args.out, f"r{r}"))
        os.makedirs(rd, exist_ok=True)
        data = os.path.join(HERE, "data", f"{tag}-r{r}")
        net = os.path.join(cur, "net.pmnet")
        learner = entrant(args.policy, net, args, roll) + f"+temp={args.temp}"
        fast = ["--fast", args.fast] if args.fast and args.fast != "none" else []
        run([exe("selfplay"), "--out", data, "--games", str(args.games), "--jobs", str(args.jobs), "--keep",
             str(args.keep), "--learner", learner, "--opponents", "self", "--seed", str(1000 * r), "--tag", str(r)]
            + fast, os.path.join(rd, "selfplay.log"))
        t1 = time.time()
        window = [os.path.join(HERE, "data", f"{tag}-r{k}") for k in range(max(1, r - args.window + 1), r + 1)]
        cmd = [sys.executable, "train.py", "--init", cur, "--out", rd, "--epochs", str(args.epochs), "--lr", str(args.lr)]
        cmd += args.train_args.split()
        for w in window:
            if os.path.isdir(w):
                cmd += ["--data", w]
        run(cmd, os.path.join(rd, "train.log"))
        new_roll = None
        if args.rollout == "small":
            new_roll = os.path.join(rd, "rollout")
            cmd = [sys.executable, "train.py", "--init", roll, "--out", new_roll, "--epochs", str(args.epochs), "--lr",
                   str(args.lr)] + args.train_args.split()
            for w in window:
                if os.path.isdir(w):
                    cmd += ["--data", w]
            run(cmd, os.path.join(rd, "train-rollout.log"))
        t2 = time.time()
        a = entrant(args.policy, os.path.join(rd, "net.pmnet"), args, new_roll)
        b = entrant(args.policy, net, args, roll)
        out = run([exe("archbench"), "--entrants", f"{a},{b}", "--sprt", args.gate, "--max-deals", str(args.max_deals),
                   "--jobs", str(args.sprt_jobs)], os.path.join(rd, "sprt.log"))
        accepted = "H1 accepted" in out
        elo = re.findall(r"Elo[^\n]*", out)
        line = {"round": r, "net_dir": rd, "from": cur, "rollout_dir": new_roll, "accepted": accepted,
                "verdict": elo[-1] if elo else None,
                "selfplay_s": round(t1 - t0), "train_s": round(t2 - t1), "sprt_s": round(time.time() - t2)}
        with open(book, "a") as f:
            f.write(json.dumps(line) + "\n")
        print("ROUNDS", json.dumps(line), flush=True)
        if accepted:
            cur = rd
            roll = new_roll or roll


if __name__ == "__main__":
    main()
