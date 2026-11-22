"""Search-then-learn rounds (#400 R4): the searched policy plays itself, the network learns the search's improved
policy and the games' results, and the new network replaces the old one only when an SPRT says it is stronger.

    python rounds.py --init runs/bc-tx --out runs/loop --rounds 5 --games 20000

Each round `r` writes `data/<out name>-r<r>` (the games) and `<out>/r<r>` (the network, the trainer's report and
the SPRT's log). `<out>/rounds.jsonl` gets one line per round. The data window is the last `--window` rounds.

Faster rounds: `--fast net` answers the decisions that won't be recorded with the network alone, so the search is
spent only on the recorded ones; `--rollout small --rollout-init runs/bc` plays the playouts with a small MLP trained
each round on the same games (`<out>/r<r>/rollout`), so the main network can grow without slowing the playouts.

More games: `--remote playmat-rl --remote-image ghcr.io/arcanite24/playmat-selfplay:sha-...@sha256:...` also has
the public server play the round while it is idle (ops/web/host/rl-worker.sh). The round's job goes there before
the local games; after them, its finished chunks are pulled into the round's data (`remote.py`).
"""

import argparse
import json
import os
import re
import subprocess
import sys
import time

import data as data_mod
import fleet
import remote

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
    # `cfg:` starts from skill-4 (`extra_turns=0, playout=1, budget=3000, width=8`), so set full search defaults
    # explicitly to match `host@s6` (`extra_turns=1, playout=2, budget=6000, width=12`)
    keys = [
        f"policy={policy}", "hide=1", f"samples={args.samples}", f"eval={args.eval}",
        f"extra_turns={args.extra_turns}", f"playout={args.playout}",
        f"budget={args.budget}", f"width={args.width}", f"net={net}",
    ]
    if args.rollout == "small":
        # the round's small network plays the playouts
        keys.append(f"rollout={os.path.join(roll, 'net.pmnet')}")
    elif args.rollout:
        # `self`: the entrant's own network plays the playouts too, so a better network also makes better playouts
        keys.append(f"rollout={net if args.rollout == 'self' else args.rollout}")
    keys += [kv for kv in args.cfg.split("+") if kv]
    return "cfg:" + "+".join(keys)


def elo_text(verdict):
    """`elo +5.2 [-2.6, +13.1]` out of an SPRT verdict line, or None"""
    m = re.search(r"elo [-+]?[0-9.]+ \[[^\]]*\]", verdict or "")
    return m.group(0) if m else None


def report(args, book, tag, r, phase, remote_games=None, lan_games=None):
    """The trainer's progress for the admin console's Self-play section (remote.progress); never fails the round."""
    if not args.remote:
        return
    history = []
    if os.path.exists(book):
        for raw in open(book).readlines()[-20:]:
            try:
                h = json.loads(raw)
            except ValueError:
                continue
            history.append({k: h.get(k) for k in ("round", "accepted", "remote_games", "lan_games", "selfplay_s",
                                                  "train_s", "sprt_s")}
                           | {"elo": elo_text(h.get("verdict"))})
    state = {"run": tag, "round": r, "rounds": args.start + args.rounds - 1, "phase": phase,
             "at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()), "local_games": args.games,
             "remote_games": remote_games, "lan_games": lan_games, "history": history}
    try:
        remote.progress(args.remote, state)
    except Exception as e:
        print(f"ROUNDS progress not sent: {e}", flush=True)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--init", required=True, help="a train.py run folder: the first network")
    ap.add_argument("--out", required=True)
    ap.add_argument("--rounds", type=int, default=5)
    ap.add_argument("--start", type=int, default=1, help="the round to start at (resume)")
    ap.add_argument("--games", type=int, default=20000)
    ap.add_argument("--keep", type=int, default=250, help="per thousand decisions recorded (KataGo playout cap)")
    ap.add_argument("--jobs", type=int, default=12)
    ap.add_argument("--policy", default="ismcts", help="the search that plays: ismcts, gumbel, or netsearch")
    ap.add_argument("--samples", type=int, default=4)
    ap.add_argument("--extra-turns", type=int, default=1)
    ap.add_argument("--playout", type=int, default=2)
    ap.add_argument("--budget", type=int, default=6000)
    ap.add_argument("--width", type=int, default=12)
    ap.add_argument("--rollout", default=None,
                    help="a network for inside the playouts: self, small (an MLP trained each round), builtin, or a .pmnet")
    ap.add_argument("--rollout-init", default=None, help="with --rollout small: the first small network's run folder")
    ap.add_argument("--fast", default=None, help="the policy for decisions that won't be recorded (net), or none")
    ap.add_argument("--cfg", default="", help="more config keys for the learner, k=v joined by + (gumbel_root=1+leaf_blend=1000+iters=48)")
    ap.add_argument("--eval", type=int, default=2,
                    help="the search's score: 2 the fitted score, 3 the network's value")
    ap.add_argument("--temp", type=int, default=1000, help="the search's temperature while recording, per thousand")
    ap.add_argument("--window", type=int, default=2, help="recent rounds used for policy targets")
    ap.add_argument("--value-window", type=int, default=0,
                    help="wider replay window for value/belief/aux (0 = same as --window; e.g. 8 keeps last 8 rounds)")
    ap.add_argument("--extra-data", action="append", default=[],
                    help="older data dirs included in the value/belief replay buffer (not policy if --value-window > 0)")
    ap.add_argument("--epochs", type=int, default=2)
    ap.add_argument("--lr", type=float, default=1e-3)
    ap.add_argument("--bs", type=int, default=2048)
    ap.add_argument("--samples-per-epoch", type=int, default=2000000)
    ap.add_argument("--max-deals", type=int, default=3444)
    ap.add_argument("--sprt-jobs", type=int, default=12)
    ap.add_argument("--gate", default="-5,5",
                    help="SPRT elo0,elo1 for keeping a round (-5,5), or 'none' for continuous updates without per-round SPRT")
    ap.add_argument("--anchor", default=None, help="periodic benchmark opponent (e.g. host@s6) run every --anchor-every rounds")
    ap.add_argument("--anchor-every", type=int, default=3)
    ap.add_argument("--anchor-deals", type=int, default=400)
    ap.add_argument("--bin", default=None, help="the folder with selfplay and archbench (default ../target/release)")
    ap.add_argument("--remote", default=None, help="an ssh alias for the public server's playmat-rl account")
    ap.add_argument("--remote-image", default=None, help="with --remote: the self-play image, by digest")
    ap.add_argument("--lan", default=None, help="idle LAN machines that play too, as alias:jobs (minivac:8,blindvac:4); "
                    "each provisioned with fleet.py from this commit")
    ap.add_argument("--train-args", default="", help="more train.py flags, e.g. \"--ema 0.995 --compile\"")
    ap.add_argument("--gpu-selfplay", action="store_true",
                    help="serve local selfplay evals from infer_server.py on GPU (use with --jobs 32..64)")
    ap.add_argument("--gpu-compile", action="store_true", help="with --gpu-selfplay: torch.compile the server model")
    ap.add_argument("--hf-models", default=None, help="auto-upload each round checkpoint to this HF model repo")
    ap.add_argument("--hf-dataset", default=None, help="auto-upload each round compacted data to this HF dataset repo")
    args = ap.parse_args()
    if args.lan and args.games > fleet.SEED_BLOCK:
        sys.exit(f"--lan needs --games <= {fleet.SEED_BLOCK} (the LAN hosts' seeds start there)")
    fleet.parse_hosts(args.lan)
    if args.remote and not args.remote_image:
        sys.exit("--remote needs --remote-image (ghcr.io/arcanite24/playmat-selfplay:...@sha256:...)")
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
        job = None
        if args.rollout == "small":
            roll_file = os.path.join(roll, "net.pmnet")
        elif args.rollout in (None, "builtin"):
            roll_file = net
        else:
            roll_file = net if args.rollout == "self" else args.rollout
        if args.remote:
            # the same learner with the networks where the worker mounts them; its seeds are the round's second half
            far = learner.replace(roll_file, "/job/rollout.pmnet").replace(net, "/job/net.pmnet")
            try:
                said, job = remote.put_job(args.remote, f"{tag}-r{r}", args.remote_image, far, net, roll_file,
                                           args.keep, args.fast if args.fast != "none" else "", 1_000_000 * r + 500_000, r)
                print("ROUNDS remote:", said, flush=True)
            except Exception as e:  # the round goes on with the local games alone
                print(f"ROUNDS remote job not sent: {e}", flush=True)
        report(args, book, tag, r, "selfplay")
        drivers = []
        for h in fleet.parse_hosts(args.lan):
            try:
                why = fleet.check(h)
            except Exception as e:
                why = f"no answer ({e})"
            if why:
                print(f"ROUNDS lan {h.alias} left out: {why}", flush=True)
                continue
            os.makedirs(data, exist_ok=True)
            d = fleet.Driver(h, r, learner, net, roll_file, args.keep, args.fast if args.fast != "none" else "", r, data,
                             log=lambda m: print(m, flush=True))
            d.start()
            drivers.append(d)
        local_learner = learner
        infer_proc = None
        if args.gpu_selfplay:
            srv_cmd = [sys.executable, "-u", "infer_server.py", "--dir", cur, "--port", "0"]
            if args.gpu_compile:
                srv_cmd.append("--compile")
            infer_proc = subprocess.Popen(srv_cmd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, cwd=HERE)
            gpu_url = None
            for line in infer_proc.stdout:
                print("ROUNDS infer:", line.strip(), flush=True)
                m = re.search(r"tcp://\S+", line)
                if m:
                    gpu_url = m.group(0)
                    break
            if not gpu_url:
                infer_proc.terminate()
                sys.exit("ROUNDS infer_server failed to start")
            local_learner = learner.replace(f"net={net}", f"net={gpu_url}")
        try:
            run([exe("selfplay"), "--out", data, "--games", str(args.games), "--jobs", str(args.jobs), "--keep",
                 str(args.keep), "--learner", local_learner, "--opponents", "self", "--seed", str(1_000_000 * r), "--tag", str(r)]
                + fast, os.path.join(rd, "selfplay.log"))
        finally:
            if infer_proc is not None:
                infer_proc.terminate()
                try:
                    infer_proc.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    infer_proc.kill()
        lan_games = None
        if drivers:
            # the chunks under way finish (a few minutes at most); then each driver stops
            for d in drivers:
                d.stop()
            for d in drivers:
                d.join(600)
                if d.is_alive():
                    d.h.kill()
            lan_games = {d.h.alias: d.games for d in drivers}
            print(f"ROUNDS lan: {lan_games}; thrown away {sum(d.stopped for d in drivers)} chunks", flush=True)
        remote_games = None
        if job is not None:
            report(args, book, tag, r, "pull")
            try:
                n, g, size, rejected = remote.pull(args.remote, job, data)
                remote_games = g
                print(f"ROUNDS remote: {n} chunks, {g} games, {size >> 20} MB; rejected {rejected}", flush=True)
            except Exception as e:
                print(f"ROUNDS remote pull failed: {e}", flush=True)
        t1 = time.time()
        # Compact hundreds of tiny fleet/remote chunks into one merged shard so train.py reads in ~2-4 s instead of 15 min
        if os.path.isdir(data):
            data_mod.compact_dir(data)
        report(args, book, tag, r, "train", remote_games, lan_games)
        vw = max(args.window, args.value_window)
        recent_dirs = [os.path.join(HERE, "data", f"{tag}-r{k}") for k in range(max(1, r - vw + 1), r + 1)]
        recent_dirs = [w for w in recent_dirs if os.path.isdir(w)]
        extra_dirs = [os.path.abspath(d) for d in args.extra_data if os.path.isdir(d)]
        window = extra_dirs + recent_dirs
        pol_recent = args.window if (args.value_window > args.window or extra_dirs) else 0
        base_train = [
            "--epochs", str(args.epochs),
            "--lr", str(args.lr),
            "--bs", str(args.bs),
            "--amp", "1",
            "--no-critic",
            "--compact",
        ]
        if args.samples_per_epoch > 0:
            base_train += ["--samples-per-epoch", str(args.samples_per_epoch)]
        if pol_recent > 0:
            base_train += ["--policy-recent", str(min(pol_recent, len(recent_dirs)))]
        cmd = [sys.executable, "train.py", "--init", cur, "--out", rd] + base_train + args.train_args.split()
        for w in window:
            cmd += ["--data", w]
        run(cmd, os.path.join(rd, "train.log"))
        new_roll = None
        if args.rollout == "small":
            new_roll = os.path.join(rd, "rollout")
            cmd = [sys.executable, "train.py", "--init", roll, "--out", new_roll] + base_train + args.train_args.split()
            for w in window:
                cmd += ["--data", w]
            run(cmd, os.path.join(rd, "train-rollout.log"))
        t2 = time.time()
        report(args, book, tag, r, "sprt", remote_games, lan_games)
        a = entrant(args.policy, os.path.join(rd, "net.pmnet"), args, new_roll)
        b = entrant(args.policy, net, args, roll)
        if args.gate.lower() == "none":
            accepted = True
            elo = ["continuous (no gate)"]
        else:
            out = run([exe("archbench"), "--entrants", f"{a},{b}", "--sprt", args.gate, "--max-deals", str(args.max_deals),
                       "--jobs", str(args.sprt_jobs)], os.path.join(rd, "sprt.log"))
            accepted = "H1 accepted" in out
            elo = re.findall(r"SPRT result[^\n]*", out)
        anchor_verdict = None
        if args.anchor and (r % max(1, args.anchor_every) == 0):
            a_out = run([exe("archbench"), "--entrants", f"{a},{args.anchor}", "--deals", str(args.anchor_deals),
                         "--jobs", str(args.sprt_jobs)], os.path.join(rd, "anchor.log"))
            lines = [ln for ln in a_out.splitlines() if "vs" in ln or "elo" in ln.lower() or "%" in ln]
            anchor_verdict = lines[-1] if lines else None
        line = {"round": r, "net_dir": rd, "from": cur, "rollout_dir": new_roll, "accepted": accepted,
                "verdict": elo[-1] if elo else None, "anchor": anchor_verdict,
                "remote_games": remote_games, "lan_games": lan_games, "selfplay_s": round(t1 - t0), "train_s": round(t2 - t1), "sprt_s": round(time.time() - t2)}
        with open(book, "a") as f:
            f.write(json.dumps(line) + "\n")
        print("ROUNDS", json.dumps(line), flush=True)
        report(args, book, tag, r, "done", remote_games, lan_games)
        if args.hf_models or args.hf_dataset:
            try:
                import hf_archive
                api = hf_archive.HfApi(token=hf_archive.load_token())
                if args.hf_models:
                    hf_archive.upload_checkpoint_dir(api, args.hf_models, rd, remote_name=f"{tag}/r{r}")
                if args.hf_dataset and os.path.isdir(data):
                    hf_archive.upload_data_dir(api, args.hf_dataset, data, remote_name=f"{tag}-r{r}")
            except Exception as e:
                print(f"ROUNDS hf archive failed (continuing): {e}", flush=True)
        if accepted:
            cur = rd
            roll = new_roll or roll
    if args.remote:
        try:
            remote.ssh(args.remote, "stop")
        except Exception as e:
            print(f"ROUNDS remote stop failed: {e}", flush=True)


if __name__ == "__main__":
    main()
