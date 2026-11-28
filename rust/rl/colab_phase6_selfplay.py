"""Phase 6 Deep-Search GPU PFSP Self-Play Runner on Colab A100/L4.

Runs 5.08M / 10.10M / 2.06M self-play on the expanded 452-deck `constructed-train-450` pool
(Constructed + Limited-curve + Brawl/Singleton decks, strictly disjoint from `constructed-ood-1`)
or rotating archetype hardening (`hardening-v2`), with `+threat_w=500 +playout=2` and Pre-Phase 6
MCTS-Solver + combat-pump fixes.
"""
import argparse, json, os, socket, subprocess, sys, threading, time


def wait_port(host, port, timeout=120.0):
    t0 = time.time()
    while time.time() - t0 < timeout:
        try:
            with socket.create_connection((host, port), timeout=1.0):
                return True
        except OSError:
            time.sleep(0.25)
    return False


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--round", required=True)
    ap.add_argument("--suite", default="constructed-train-450", choices=["constructed-train-450", "constructed-train-200", "constructed-2"])
    ap.add_argument("--learner-ckpt", default="p6pre-5m-carddrop")
    ap.add_argument("--sib-ckpt", default="phase5-final-10m-v4-soup")
    ap.add_argument("--ood-ckpt", default="phase5-final-1.3m-v4-soup")
    ap.add_argument("--games", type=int, default=1200)
    ap.add_argument("--iters", type=int, default=512)
    ap.add_argument("--samples", type=int, default=8)
    ap.add_argument("--vloss", type=int, default=16)
    ap.add_argument("--jobs", type=int, default=24)
    ap.add_argument("--keep", type=int, default=400)
    ap.add_argument("--fast-iters", type=int, default=64)
    ap.add_argument("--threat-w", type=int, default=500)
    ap.add_argument("--playout", type=int, default=2)
    ap.add_argument("--seed", type=int, default=25000)
    ap.add_argument("--tag", type=int, default=25)
    ap.add_argument("--timeout-secs", type=int, default=300)
    ap.add_argument("--hardening-v2", action="store_true")
    ap.add_argument("--models-repo", default="nerijs/playmat-ai-checkpoints")
    ap.add_argument("--dataset-repo", default="nerijs/playmat-ai-selfplay")
    args = ap.parse_args()

    t0 = time.time()
    subprocess.run([sys.executable, "-m", "pip", "install", "-q", "huggingface_hub", "hf_xet"], check=True)
    import torch
    from huggingface_hub import HfApi, snapshot_download

    tok = os.environ["HF_TOKEN"]
    print(
        f"PHASE6_SP round={args.round} suite={args.suite} learner={args.learner_ckpt} "
        f"hardening_v2={args.hardening_v2} threat_w={args.threat_w} playout={args.playout} "
        f"device={torch.cuda.get_device_name(0)} cpus={os.cpu_count()} games={args.games}",
        flush=True,
    )

    work = "/content/playmat_p6sp"
    os.makedirs(os.path.join(work, "rust", "crates", "playmat-ai"), exist_ok=True)
    for base_dir, link_path in [("/home/playmat", "/home/playmat/mage"), ("/home/brandonalamgarcianeri", "/home/brandonalamgarcianeri/playmat")]:
        os.makedirs(base_dir, exist_ok=True)
        if os.path.islink(link_path) or os.path.exists(link_path):
            try: os.unlink(link_path)
            except OSError: pass
        if not os.path.exists(link_path):
            os.symlink(work, link_path)

    ckpts = list(dict.fromkeys([args.learner_ckpt, args.sib_ckpt, args.ood_ckpt]))
    patterns = [
        "bin/linux-x86_64/selfplay",
        f"archbench/{args.suite}/*",
        "archbench/constructed-2/*",
        "Mage.Server/career-src/*",
        "code/rl/infer_server.py",
        "code/rl/model.py",
        "code/rl/cardtext.py",
        "code/rl/train.py",
        "code/rl/data.py",
        "cardtext/*",
    ] + [f"checkpoints/{c}/*" for c in ckpts]

    snapshot_download(repo_id=args.models_repo, repo_type="model", local_dir=work, allow_patterns=patterns, token=tok)
    rl_dir = os.path.join(work, "code", "rl")
    os.makedirs(os.path.join(rl_dir, "data"), exist_ok=True)
    for fn in ("cardtext.npy", "cardtext_sem.npy"):
        src = os.path.join(work, "cardtext", fn)
        dst = os.path.join(rl_dir, "data", fn)
        if os.path.exists(src) and not os.path.exists(dst):
            os.symlink(src, dst)

    sp_bin = os.path.join(work, "bin", "linux-x86_64", "selfplay")
    os.chmod(sp_bin, 0o755)
    out_dir = os.path.join(work, "shards", args.round)
    os.makedirs(out_dir, exist_ok=True)

    procs, ports = [], {}
    for idx, c in enumerate(ckpts):
        port = 18600 + idx
        ports[c] = port
        logp = os.path.join(work, f"srv_{idx}.log")
        lf = open(logp, "w")
        p = subprocess.Popen(
            [
                sys.executable, "-u", os.path.join(rl_dir, "infer_server.py"),
                "--dir", os.path.join(work, "checkpoints", c),
                "--port", str(port), "--amp", "1", "--max-batch", "512", "--wait-ms", "0.5",
            ],
            cwd=rl_dir, stdout=lf, stderr=subprocess.STDOUT,
        )
        procs.append((c, p, lf, logp))

    try:
        for c, port in ports.items():
            assert wait_port("127.0.0.1", port, 120.0), f"infer_server failed for {c}"
        print(f"ALL {len(ports)} GPU INFER SERVERS READY", flush=True)

        tw_flag = f"+threat_w={args.threat_w}" if args.threat_w > 0 else ""
        pl_flag = f"+playout={args.playout}" if args.playout > 0 else ""
        common = (
            f"+hide=1+iters={args.iters}+samples={args.samples}+cpuct=1000+leaf_blend=1000"
            f"+gumbel_root=1+multi_tree=1+mulligan_net=1+width=24+consider=16+extra_turns=2"
            f"+belief_ms=1+fpu_vnode=1+solver=1+forced_k=200+vloss={args.vloss}{tw_flag}{pl_flag}+temp=1000+temp_turn=6"
        )
        opp_common = (
            f"+hide=1+iters={max(256, args.iters // 2)}+samples={args.samples}+cpuct=1000+leaf_blend=1000"
            f"+gumbel_root=1+multi_tree=1+mulligan_net=1+width=24+consider=16+extra_turns=2"
            f"+belief_ms=1+fpu_vnode=1+solver=1+vloss={args.vloss}{tw_flag}{pl_flag}+temp=750+temp_turn=6"
        )
        learner = f"cfg:policy=ismcts+net=tcp://127.0.0.1:{ports[args.learner_ckpt]}{common}"
        sib = f"cfg:policy=ismcts+net=tcp://127.0.0.1:{ports[args.sib_ckpt]}{opp_common}"
        ood = f"cfg:policy=ismcts+net=tcp://127.0.0.1:{ports[args.ood_ckpt]}{opp_common}"

        suite_path = os.path.join(work, "archbench", args.suite)
        if args.hardening_v2:
            c2 = json.loads(open(os.path.join(work, "archbench", "constructed-2", "suite.json")).read())
            kws = (
                "control", "spirits", "dimir-midrange",
                "stompy", "landfall", "vigorbloom", "angels", "dragons", "greasefang",
                "humans", "company", "weenie", "sacrifice", "vampires", "lizards",
            )
            hdecks = [d for d in c2["decks"] if any(k in d["file"] for k in kws)]
            hdir = os.path.join(work, "archbench", "hardening-v2")
            os.makedirs(hdir, exist_ok=True)
            open(os.path.join(hdir, "suite.json"), "w").write(
                json.dumps(dict(c2, suite="hardening@2", decks=hdecks, deals=len(hdecks) * (len(hdecks) - 1)), indent=2)
            )
            suite_path = hdir
            print(f"HARDENING-V2 SUITE READY: {len(hdecks)} BigMana+GoWide+Control decks", flush=True)
            opponents = f"self*4;{sib}*3;{ood}*1;host@s6*2"
        else:
            opponents = f"self*4;{sib}*3;{ood}*2;host@s6*1"

        cmd = [
            sp_bin,
            "--out", out_dir,
            "--repo", work,
            "--suite", suite_path,
            "--learner", learner,
            "--opponents", opponents,
            "--games", str(args.games),
            "--jobs", str(args.jobs),
            "--keep", str(args.keep),
            "--fast", f"ismcts:{args.fast_iters}",
            "--seed", str(args.seed),
            "--tag", str(args.tag),
        ]
        env = dict(os.environ, PLAYMAT_REPO=work, RAYON_NUM_THREADS=str(args.jobs), PLAYMAT_GAME_TIMEOUT_SECS=str(args.timeout_secs))
        print("RUN $", " ".join(cmd[:10]), "...", flush=True)
        pr = subprocess.Popen(cmd, cwd=work, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, bufsize=1)
        wd = threading.Timer(7200.0, pr.kill)
        wd.daemon = True
        wd.start()
        try:
            for line in pr.stdout:
                print(line, end="", flush=True)
            pr.wait()
        finally:
            wd.cancel()
        if pr.returncode != 0:
            sys.exit(f"selfplay failed with rc={pr.returncode}")
    finally:
        for c, p, lf, logp in procs:
            p.terminate()
            try: p.wait(timeout=5)
            except Exception: p.kill()
            lf.close()
            try:
                lines = open(logp).read().strip().splitlines()
                print(f"SRV {c}: " + " | ".join(lines[-2:]), flush=True)
            except Exception:
                pass

    print(f"MERGING {out_dir}...", flush=True)
    subprocess.run([sys.executable, "-u", os.path.join(rl_dir, "data.py"), "--compact", out_dir], check=True)
    api = HfApi(token=tok)
    print(f"UPLOADING {out_dir}/merged.* -> {args.dataset_repo}/data/{args.round}", flush=True)
    api.upload_folder(
        folder_path=out_dir,
        path_in_repo=f"data/{args.round}",
        repo_id=args.dataset_repo,
        repo_type="dataset",
        allow_patterns=["merged.*"],
        commit_message=f"Phase 6 Obs-v4 PFSP shards on {args.suite}: {args.round}",
    )
    print(f"PHASE6_SP ALL DONE ({args.round}) in {time.time() - t0:.1f} s", flush=True)


if __name__ == "__main__":
    main()
